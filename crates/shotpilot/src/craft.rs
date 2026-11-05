//! Trailer craft handed to the planner before it writes anything.
//!
//! Quality comes from what the planner is given, not from asking it to look at its own
//! output afterwards (no self-review, no checklist, no repair rounds; see D-22). This
//! module turns the brief and the craft rules into a write-time spec:
//! - a skeleton: how many shots, which beat each has, a time budget and a shot size
//!   (hook and title lengths measured from published teasers of the genre, see refs.rs);
//! - the craft rules (the same table the shot check lints with, so they cannot drift);
//! - the measured references: the all-genre craft and the brief's genre package (refs.rs);
//! - a framing table: camera distance per shot size and lens, from the sensor geometry.
//!
//! Code then normalizes what comes back (beat order, duration range, runtime); the shot
//! check results are shown to the human, who decides what to send back.

use crate::geometry::round_to;
use crate::model::{fmt_g, BriefCfg, Definition};
use crate::shotcheck::{Severity, RULES};

/// One slot of the skeleton.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub beat: &'static str,
    pub duration: f64,
    pub size: &'static str,
}

/// Default shot count for a runtime: about one shot per 4.5 s, 5..10 shots.
pub fn default_shots(target: f64) -> usize {
    ((target / 4.5).round() as usize).clamp(5, 10)
}

fn half(v: f64) -> f64 {
    (v * 2.0).round() / 2.0
}

/// The teaser skeleton: hook, establishing shots, escalating shots that cut faster and
/// move closer, and a held title shot. Durations sum to the target.
pub fn skeleton(brief: &BriefCfg, target: f64, n_shots: Option<usize>) -> Vec<Slot> {
    let target = if target > 0.0 { target } else { 45.0 };
    let n = n_shots.unwrap_or_else(|| default_shots(target)).max(4);
    // hook and title lengths come from published teasers of the brief's genre (refs.rs)
    let pace = crate::refs::pace(brief);
    let (hook_lo, hook_hi) = if brief.is_gameplay() { (1.5, 3.0) } else { (2.0, 5.0) };
    let hook = half(pace.hook.clamp(hook_lo, hook_hi));
    let title = half(pace.title.clamp(3.0, 6.0).min((0.2 * target).max(3.0)));
    let middle = n - 2;
    let establish = ((middle as f64 * 0.4).round() as usize).clamp(1, middle - 1);
    let escalate = middle - establish;
    // establishing shots breathe; escalation accelerates (each shot a little shorter)
    let mut beat_weights: Vec<f64> = vec![1.3; establish];
    for i in 0..escalate {
        beat_weights.push(1.0 - 0.3 * i as f64 / (escalate.max(2) - 1) as f64);
    }
    let rest = (target - hook - title).max(middle as f64);
    // then follow the measured cutting along the teaser (the style's, else the genre's): a
    // shot in a busy quarter gets shorter, one in a calm quarter longer (D-29, D-39)
    let density_at = |frac: f64| {
        let c = [0.125, 0.375, 0.625, 0.875];
        let d = pace.density;
        let v = if frac <= c[0] {
            d[0]
        } else if frac >= c[3] {
            d[3]
        } else {
            let i = c.iter().rposition(|x| *x <= frac).unwrap_or(0).min(2);
            d[i] + (d[i + 1] - d[i]) * (frac - c[i]) / (c[i + 1] - c[i])
        };
        v.clamp(0.5, 2.0)
    };
    let mut weights = beat_weights.clone();
    for _ in 0..2 {
        let sum: f64 = weights.iter().sum();
        let mut t = hook;
        weights = beat_weights
            .iter()
            .zip(&weights)
            .map(|(b, w)| {
                let d = rest * w / sum;
                let mid = (t + d / 2.0) / target;
                t += d;
                b / density_at(mid)
            })
            .collect();
    }
    let sum: f64 = weights.iter().sum();
    let mut mids: Vec<f64> = weights.iter().map(|w| half(rest * w / sum).clamp(1.5, 8.0)).collect();
    // rounding and the 1.5-8 s bounds leave some time over: hand it out half a second at a
    // time to the shot furthest from its share that still has room
    let mut drift = rest - mids.iter().sum::<f64>();
    while drift.abs() >= 0.5 {
        let step: f64 = if drift > 0.0 { 0.5 } else { -0.5 };
        let want = |i: usize| (rest * weights[i] / sum - mids[i]) * step.signum();
        let room = |i: &usize| (1.5..=8.0).contains(&(mids[*i] + step));
        let Some(i) = (0..mids.len()).filter(room).max_by(|a, b| want(*a).total_cmp(&want(*b))) else { break };
        mids[i] += step;
        drift -= step;
    }

    let mut out = vec![Slot { beat: "hook", duration: hook, size: "one striking image: an extreme close-up or a strange wide" }];
    for (i, d) in mids.into_iter().enumerate() {
        let (beat, size) = if i < establish {
            ("establish", if i == 0 { "wide / establishing: where we are" } else { "medium: what kind of game this is" })
        } else {
            let k = i - establish;
            ("escalate", if k + 1 == escalate { "close-up: the peak" } else if k % 2 == 0 { "medium, moving closer" } else { "close" })
        };
        out.push(Slot { beat, duration: d, size });
    }
    out.push(Slot { beat: "title", duration: title, size: "wide and calm, room in the upper third for the title card" });
    out
}

/// Camera distance (cm) at which `visible_cm` of subject fills the frame height.
pub fn distance_for(visible_cm: f64, focal_mm: f64, sensor_width_mm: f64, aspect: f64) -> f64 {
    let sensor_h = sensor_width_mm / aspect.max(0.1);
    focal_mm * visible_cm / sensor_h
}

const SIZES: &[(&str, f64)] = &[
    ("close-up (a face, a hand)", 40.0),
    ("medium (a person waist-up, a door)", 110.0),
    ("full (a whole person, a car)", 220.0),
    ("wide (a room, a yard)", 700.0),
    ("establishing (a building, a street)", 2500.0),
];
const LENSES: &[f64] = &[18.0, 24.0, 35.0, 50.0, 85.0];

/// The framing table for the planner: distance per shot size and lens.
pub fn framing_table(defn: &Definition) -> String {
    let sensor = defn.defaults.get("lens").and_then(|l| l.get("sensor_width")).and_then(crate::model::num).unwrap_or(36.0);
    let (w, h) = defn.teaser.resolution;
    let aspect = if h > 0 { w as f64 / h as f64 } else { 16.0 / 9.0 };
    let mut lines = vec![format!(
        "Camera distance to the subject (cm) so the subject fills the frame height \
         ({}mm sensor, {}:{} frame). Pick the size first, then a lens, then the distance:",
        fmt_g(sensor), w, h
    )];
    lines.push(format!("  {:<40}{}", "size", LENSES.iter().map(|f| format!("{:>8}", format!("{}mm", fmt_g(*f)))).collect::<String>()));
    for (name, visible) in SIZES {
        let row: String = LENSES
            .iter()
            .map(|f| format!("{:>8}", fmt_g(round_to(distance_for(*visible, *f, sensor, aspect) / 10.0, 0) * 10.0)))
            .collect();
        lines.push(format!("  {name:<40}{row}"));
    }
    lines.push("Wide lenses (18-24mm) exaggerate space and make rooms loom; long lenses (85mm) flatten and isolate.".into());
    lines.join("\n")
}

/// The craft rules as write-time instructions, most important first.
pub fn rules_text() -> String {
    let mut out = Vec::new();
    for sev in [Severity::P0, Severity::P1, Severity::P2] {
        for r in RULES.iter().filter(|r| r.severity == sev && r.basis != "observed") {
            out.push(format!("- {}", r.summary));
        }
    }
    out.push("- One idea per shot, one motivated move per shot; a static shot is a valid choice.".into());
    out.push("- Every item the brief says must be shown appears in at least one shot.".into());
    out.join("\n")
}

fn brief_text(b: &BriefCfg) -> String {
    let mut lines = vec![format!(
        "Kind: {}",
        if b.is_gameplay() {
            "gameplay trailer: real play from the very first seconds; the hook is gameplay"
        } else {
            "teaser: sell the title and the world; cinematic, built in the engine"
        }
    )];
    for (label, v) in [("Genre", &b.genre), ("Core experience", &b.core), ("Audience", &b.audience), ("Notes", &b.notes)] {
        if !v.is_empty() {
            lines.push(format!("{label}: {v}"));
        }
    }
    if !b.must_show.is_empty() {
        lines.push(format!("Must show: {}", b.must_show.join(", ")));
    }
    if !b.references.is_empty() {
        lines.push(format!("Reference trailers (match their pacing and restraint): {}", b.references.join(" / ")));
    }
    lines.join("\n")
}

/// The skeleton as a table for the prompt.
pub fn skeleton_text(slots: &[Slot]) -> String {
    let total: f64 = slots.iter().map(|s| s.duration).sum();
    let mut lines = vec![format!("{} shots, {}s in total. Follow this skeleton; adjust a duration by up to ~1s when a shot needs it:", slots.len(), fmt_g(total))];
    for (i, s) in slots.iter().enumerate() {
        lines.push(format!("  {:>2}. {:<10} {:>4}s  {}", i + 1, s.beat, fmt_g(s.duration), s.size));
    }
    lines.join("\n")
}

/// Everything the planner is told about the brief and the craft, in the order it reads it.
pub fn plan_spec(defn: &Definition, slots: &[Slot]) -> String {
    format!(
        "## Brief\n{}\n{}\n## Skeleton\n{}\n\n## Craft rules\n{}\n{}\n{}{}{}\n## Framing\n{}",
        brief_text(&defn.brief),
        plan_text(&defn.brief),
        skeleton_text(slots),
        rules_text(),
        crate::refs::baseline_text(),
        crate::refs::planner_text(&defn.brief),
        crate::designsys::planner_text(defn),
        crate::skills::planner_text(&defn.brief),
        framing_table(defn)
    )
}

/// What a note can change, which decides how much of the frozen spec an edit gets (D-48).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The picture may change (the camera, the light, or a free note on a shot or the teaser).
    Picture,
    /// Only text, hand-overs and the finish burned in at the cut.
    Text,
    /// Only the sound.
    Sound,
}

/// The spec an edit is made against: the same pieces the planner wrote the cut from, so a
/// revised shot keeps the brief, the direction, the look and the craft (open-design's frozen
/// Design Spec, shared by every path that writes the artifact).
pub fn edit_spec(defn: &Definition, reach: Reach) -> String {
    let b = &defn.brief;
    let mut out = format!(
        "## The teaser's spec\nThe cut was written from this; keep every edit inside it.\n\n## Brief\n{}\n{}",
        brief_text(b),
        plan_text(b)
    );
    if reach == Reach::Sound {
        return out;
    }
    if reach == Reach::Picture {
        out.push_str(&format!("\n## Craft rules\n{}\n", rules_text()));
    }
    out.push_str(&crate::refs::baseline_text());
    out.push_str(&crate::designsys::planner_text(defn));
    out.push_str(&crate::skills::planner_text(b));
    if reach == Reach::Picture {
        out.push_str(&crate::ai::stage_section(defn));
        out.push_str(&format!("\n## Framing\n{}\n", framing_table(defn)));
    }
    out
}

/// The planning turn's plan (D-45): production follows it; the latest direction wins.
fn plan_text(b: &BriefCfg) -> String {
    if b.plan.trim().is_empty() && b.direction.trim().is_empty() {
        return String::new();
    }
    let mut out =
        String::from("\n## The plan\nAgreed before production. Carry it out; the director's latest direction outranks it.\n");
    if !b.direction.trim().is_empty() {
        out.push_str(&format!("Direction: {}\n", b.direction.trim()));
    }
    if !b.plan.trim().is_empty() {
        out.push_str(&format!("\n{}\n", b.plan.trim()));
    }
    out
}

/// What the planner is told about writing the script first (the text is the spine of the
/// cut, D-25) and about the hand-overs between shots.
pub fn script_text(defn: &Definition) -> String {
    format!(
        "\n## Script (write it first: it is the spine of the cut)\n\
         Write `script` before choosing shots: the lines the viewer reads, in order. Then pick each shot to show one \
         line, as evidence of it. A teaser told in text is read with the sound off.\n\
         - Slots take their own time on black between shots (`after`: the shot or slot they follow, or `start`): \
           `card` a centred line, with a quiet English line under it; `rule` a rule of the world with a small `head`; \
           `black` a silent gap (0.5-1 s) before a reveal or the title; `title` the game's title{} (3-4 s, near the end); \
           `message` a last line from inside the world (`head` = where it is posted, `from` = who sends it).\n\
         - Overlays sit on a shot (`on`, `at` seconds into it): `caption` lower centre, `count` lower left like a note \
           under a photograph, for a run of evidence shots. One line at a time: two overlays on the same shot must \
           not overlap in time (a count reads best on its own short shot).\n\
         - Devices that hold a teaser together (use the ones the genre and style references use): a first-person \
           voice (\"Today, I move in.\"); a count that keeps going over short evidence shots of 1-1.5 s; a rule of the \
           world that is about to be broken; a final message that turns the premise back on the viewer; verbs one at a \
           time over the play they name (DIG / GROW / BUILD); features named one line at a time; a maker's short note.\n\
         - Lines are short: Japanese up to ~15 characters, English up to ~6 words. Never invent a release date, a \
           platform or a store claim; use only facts the brief states.\n\
         - `target` length is the whole cut, cards included.\n\
         - `transition`: mostly `cut`; `dip` through black for a jump in time; `flash` (from white) on each short \
           evidence shot of a count, or for one jolt.\n\
         - `grade`: follow the genre and style references: `horror` (cold, desaturated, grain), `red` (only for a \
           short jolt teaser), `warm` (cozy, bright, sunny), `vivid` (action, punchy colour), `cool` (sci-fi, mystery, \
           night), or `none` to keep the game's own look. \
           `letterbox`: 2.39 for a cinematic teaser, 0 for none.\n",
        if defn.teaser.title.is_empty() { String::new() } else { format!(" exactly as written: {}", defn.teaser.title) }
    )
}

/// What the planner is told about the text and cuts burned in at the cut (D-25).
pub fn finish_text(defn: &Definition) -> String {
    format!(
        "\n## Finish (burned in when the cut is assembled; never re-renders)\n\
         - `caption`: at most 3 in the whole teaser, each short (Japanese up to ~15 characters, English up to ~6 words), \
           so the teaser reads with the sound off. Leave it empty on the hook and the title shot.\n\
         - `transition`: mostly `cut`. `dip` (through black) before the title or for a jump in time; `flash` (through \
           white) only for one jolt at most.\n\
         - `title_card`: the game's title{}.\n\
         - `end_card`: only facts the brief states (platforms, release window, a call to action written there). \
           Never invent a date or a platform; with none given, leave it empty.\n",
        if defn.teaser.title.is_empty() { String::new() } else { format!(" exactly as written: {}", defn.teaser.title) }
    )
}

/// Code-side shaping of a planned list: beats in order, durations in 1..8 s, runtime
/// within 10% of the target. Returns what was changed, for the human to see.
pub fn normalize(shots: &mut [serde_json::Value], target: f64) -> Vec<String> {
    use crate::model::{num, BEATS};
    let mut notes = Vec::new();
    let rank = |s: &serde_json::Value| s["beat"].as_str().and_then(|b| BEATS.iter().position(|x| *x == b)).unwrap_or(BEATS.len());
    let before: Vec<String> = shots.iter().map(|s| crate::model::str_of(&s["id"])).collect();
    shots.sort_by_key(rank); // stable: keeps the planner's order within a beat
    let after: Vec<String> = shots.iter().map(|s| crate::model::str_of(&s["id"])).collect();
    if before != after {
        notes.push("put the shots in beat order (hook -> establish -> escalate -> title)".into());
    }
    let dur = |s: &serde_json::Value| num(&s["duration"]).unwrap_or(3.0);
    let mut clamped = Vec::new();
    for s in shots.iter_mut() {
        let d = dur(s);
        let c = d.clamp(1.0, 8.0);
        if c != d {
            clamped.push(crate::model::str_of(&s["id"]));
            s["duration"] = serde_json::json!(c);
        }
    }
    if !clamped.is_empty() {
        notes.push(format!("kept durations within 1-8s: {}", clamped.join(", ")));
    }
    let total: f64 = shots.iter().map(dur).sum();
    if target > 0.0 && total > 0.0 && (total - target).abs() > 0.1 * target {
        let k = target / total;
        for s in shots.iter_mut() {
            let d = round_to((dur(s) * k * 4.0).round() / 4.0, 2).clamp(1.0, 8.0);
            s["duration"] = serde_json::json!(d);
        }
        let now: f64 = shots.iter().map(dur).sum();
        notes.push(format!("scaled durations from {}s to {}s to fit the {}s target", fmt_g(round_to(total, 2)), fmt_g(round_to(now, 2)), fmt_g(target)));
    }
    notes
}

/// Words that ask a model to inspect its own output. The planner and reviser prompts
/// must not contain them (D-22); a test keeps it that way.
pub const SELF_REVIEW_WORDS: &[&str] = &[
    "verify", "verification", "checklist", "critique", "judge", "self-check", "double-check", "re-check",
    "review your", "validate your", "check your", "score your", "rate your",
];

pub fn self_review_words(text: &str) -> Vec<&'static str> {
    let low = text.to_lowercase();
    SELF_REVIEW_WORDS.iter().copied().filter(|w| low.contains(w)).collect()
}
