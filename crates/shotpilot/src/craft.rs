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

/// Default shot count for a runtime: about one shot per 4.5 s, 5..10 shots. Only used when
/// no measured beats are at hand (refs::beats always has some with the embedded styles).
pub fn default_shots(target: f64) -> usize {
    ((target / 4.5).round() as usize).clamp(5, 10)
}

/// Most shots a skeleton asks for: the plan has to stay writable in one reply.
pub const MAX_SHOTS: usize = 18;

/// How far under the shot check's `hook.long` a skeleton's hook stays.
const HOOK_MARGIN: f64 = 0.5;

fn half(v: f64) -> f64 {
    (v * 2.0).round() / 2.0
}

fn quarter(v: f64) -> f64 {
    (v * 4.0).round() / 4.0
}

/// How long the `title` card the planner writes after the last shot should hold: the final
/// shot of the measured teaser (a published teaser's last cut is usually its title or end
/// card), kept between 2.5 and 5 s.
pub fn title_card_secs(brief: &BriefCfg) -> f64 {
    quarter(crate::refs::pace(brief).title.clamp(2.5, 5.0))
}

/// The time a skeleton leaves for the cards after its last shot: a `black` gap and the
/// `title` card (other cards come out of the shots' time when the planner writes them).
pub fn card_reserve(brief: &BriefCfg, target: f64) -> f64 {
    (0.5 + title_card_secs(brief)).min(0.2 * target)
}

/// Split `n` into parts in proportion to `want`, each at least `min[i]` (largest remainder).
fn apportion(n: usize, want: &[f64], min: &[usize]) -> Vec<usize> {
    let floor: usize = min.iter().sum();
    let n = n.max(floor);
    let total: f64 = want.iter().sum::<f64>().max(1e-9);
    let spare = (n - floor) as f64;
    let exact: Vec<f64> = want.iter().map(|w| spare * w / total).collect();
    let mut out: Vec<usize> = exact.iter().zip(min).map(|(e, m)| m + e.floor() as usize).collect();
    let mut left = n - out.iter().sum::<usize>();
    let mut order: Vec<usize> = (0..want.len()).collect();
    order.sort_by(|a, b| (exact[*b] - exact[*b].floor()).total_cmp(&(exact[*a] - exact[*a].floor())));
    for i in order.into_iter().cycle() {
        if left == 0 {
            break;
        }
        out[i] += 1;
        left -= 1;
    }
    out
}

/// The teaser skeleton: hook, establishing shots, escalating shots that cut faster and
/// move closer, and a calm title shot, followed by the cards (`card_reserve`). The beats take
/// the share of the runtime, and cut as often, as in the measured teaser the brief follows
/// (its style, else the average of its genre's styles, else of every style; refs::beats).
/// Durations sum to the target less the reserve for the cards.
pub fn skeleton(brief: &BriefCfg, target: f64, n_shots: Option<usize>) -> Vec<Slot> {
    let target = if target > 0.0 { target } else { 45.0 };
    let pace = crate::refs::pace(brief);
    let (share, rate) = match crate::refs::beats(brief) {
        Some(b) => (b.share, b.rate),
        None => ([0.12, 0.3, 0.42, 0.16], [1.0 / 4.5; 4]),
    };
    let shots_time = target - card_reserve(brief, target);
    // the first cut lands where the measured teaser's does
    let (hook_lo, hook_hi) = if brief.is_gameplay() { (1.0, 3.0) } else { (1.0, 5.0) };
    let first = quarter(pace.hook.clamp(hook_lo, hook_hi));
    // the title shot holds what is left of the title beat once its card has had its time
    let title = half((share[3] * target - title_card_secs(brief)).clamp(2.0, 6.0));
    let hook_max = if brief.is_gameplay() { crate::shotcheck::HOOK_LONG_GAMEPLAY } else { crate::shotcheck::HOOK_LONG_TEASER } - HOOK_MARGIN;
    let hook_beat = (share[0] * target).clamp(first, hook_max.max(first));
    // pieces (shots and cards) each beat cuts to in the measured teaser, at this length
    let pieces: Vec<f64> = (0..4).map(|i| rate[i] * share[i] * target).collect();
    // the hook: the first shot, then short ones (1 s or more) as long as the hook allows
    let hook_room = (hook_beat - first).floor().max(0.0) as usize;
    let mut hook_n = 1 + (pieces[0].round() as usize).saturating_sub(1).min(hook_room);
    let hook_secs = |n: usize| if n > 1 { hook_beat } else { first };
    // the middle: the measured pieces less one card written there, at least 1 s a shot
    let middle_room = |n: usize| (shots_time - title - hook_secs(n)).floor().max(2.0) as usize;
    let mut middle_n = ((pieces[1] + pieces[2] - 1.0).round().max(2.0) as usize).min(middle_room(hook_n));
    match n_shots {
        Some(n) => {
            let n = n.max(4);
            let split = apportion(n - 1, &[pieces[0].max(0.5), pieces[1] + pieces[2]], &[1, 2]);
            hook_n = split[0].min(1 + hook_room);
            middle_n = n - 1 - hook_n;
        }
        None => {
            let over = (hook_n + middle_n + 1).saturating_sub(MAX_SHOTS);
            middle_n -= over.min(middle_n - 2);
            // a shot holds 8 s at most, so a slow teaser still needs enough of them
            middle_n = middle_n.max(((shots_time - title - hook_secs(hook_n)) / 8.0).ceil() as usize);
        }
    }
    let hook_time = hook_secs(hook_n);
    let middle = (shots_time - title - hook_time).max(middle_n as f64);
    let split = apportion(middle_n, &[pieces[1], pieces[2]], &[1, 1]);
    let (establish, escalate) = (split[0], split[1]);
    // the middle's time goes as in the measured teaser. Where that teaser escalates faster
    // than it establishes (most do; the shot check's tempo.escalate), rounding must not turn
    // it around; where it escalates slower (a third of the styles), the skeleton does too
    let faster = rate[2] >= rate[1];
    let mut est_time = middle * share[1] / (share[1] + share[2]);
    if faster && (middle - est_time) / escalate as f64 > est_time / establish as f64 / 1.1 {
        // establishing shots 10% longer than the escalating ones
        let (k1, k2) = (1.1 * establish as f64, escalate as f64);
        est_time = middle * k1 / (k1 + k2);
    }
    let esc_time = middle - est_time;
    // escalation accelerates: each shot a little shorter than the one before
    let esc_w: Vec<f64> = (0..escalate).map(|i| 1.15 - 0.3 * i as f64 / (escalate.max(2) - 1) as f64).collect();
    let esc_sum: f64 = esc_w.iter().sum();
    let mut want: Vec<f64> = vec![first];
    want.extend((1..hook_n).map(|_| (hook_time - first) / (hook_n - 1) as f64));
    want.extend((0..establish).map(|_| est_time / establish as f64));
    want.extend(esc_w.iter().map(|w| esc_time * w / esc_sum));
    want.push(title);
    let mut lens: Vec<f64> = want.iter().map(|d| quarter(*d).clamp(1.0, 8.0)).collect();
    // rounding and the 1-8 s bounds leave some time over: hand it out a quarter second at a
    // time to the middle shot furthest from its share that still has room
    let mid = hook_n..hook_n + middle_n;
    let mut drift = shots_time - lens.iter().sum::<f64>();
    while drift.abs() >= 0.25 {
        let step: f64 = if drift > 0.0 { 0.25 } else { -0.25 };
        let room = |i: &usize| (1.0..=8.0).contains(&(lens[*i] + step));
        let gap = |i: usize| (want[i] - lens[i]) * step.signum();
        let Some(i) = mid.clone().filter(room).max_by(|a, b| gap(*a).total_cmp(&gap(*b))) else { break };
        lens[i] += step;
        drift -= step;
    }
    // rounding can even out the two middle beats: keep escalation the faster one
    let (est, esc) = (hook_n..hook_n + establish, hook_n + establish..hook_n + middle_n);
    let mean = |r: &std::ops::Range<usize>, l: &[f64]| l[r.clone()].iter().sum::<f64>() / r.len() as f64;
    for _ in 0..4 * middle_n {
        if !faster || mean(&esc, &lens) < mean(&est, &lens) {
            break;
        }
        let from = esc.clone().filter(|i| lens[*i] > 1.0).max_by(|a, b| lens[*a].total_cmp(&lens[*b]));
        let to = est.clone().filter(|i| lens[*i] < 8.0).min_by(|a, b| lens[*a].total_cmp(&lens[*b]));
        let (Some(from), Some(to)) = (from, to) else { break };
        lens[from] -= 0.25;
        lens[to] += 0.25;
    }

    let mut out = vec![Slot { beat: "hook", duration: lens[0], size: "one striking image: an extreme close-up or a strange wide" }];
    for (k, d) in lens[1..hook_n].iter().enumerate() {
        out.push(Slot { beat: "hook", duration: *d, size: if k % 2 == 0 { "a flash of what is coming: close" } else { "a flash of what is coming: wide" } });
    }
    for (i, d) in lens[mid].iter().enumerate() {
        let (beat, size) = if i < establish {
            ("establish", if i == 0 { "wide / establishing: where we are" } else if i % 2 == 1 { "medium: what kind of game this is" } else { "detail: something that belongs here" })
        } else {
            let k = i - establish;
            ("escalate", if k + 1 == escalate { "close-up: the peak" } else if k % 2 == 0 { "medium, moving closer" } else { "close" })
        };
        out.push(Slot { beat, duration: *d, size });
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
    skeleton_text_with(slots, None)
}

/// The skeleton as a table, with what follows the last shot: the time left for the cards
/// and how long the `title` card holds.
pub fn skeleton_text_with(slots: &[Slot], cards: Option<(f64, f64)>) -> String {
    let total: f64 = slots.iter().map(|s| s.duration).sum();
    let mut lines = vec![format!("{} shots, {}s in total. Follow this skeleton; adjust a duration by up to ~1s when a shot needs it:", slots.len(), fmt_g(total))];
    for (i, s) in slots.iter().enumerate() {
        lines.push(format!("  {:>2}. {:<10} {:>5}s  {}", i + 1, s.beat, fmt_g(s.duration), s.size));
    }
    if let (Some(first), Some(_)) = (slots.first(), cards) {
        lines.push(format!("Shot 1 holds {}s exactly: the measured teaser makes its first cut there.", fmt_g(first.duration)));
    }
    if let Some((reserve, title_card)) = cards {
        lines.push(format!(
            "After the last shot: a `black` slot of about 0.5s, then the `title` card for about {}s ({}s with the shots). \
             Any other card comes out of the shots' time.",
            fmt_g(title_card),
            fmt_g(total + reserve)
        ));
    }
    lines.join("\n")
}

/// What follows a skeleton's last shot: the time it left for the cards (`card_reserve` at the
/// target the slots were cut for) and how long the `title` card holds.
fn cards_after(brief: &BriefCfg, slots: &[Slot]) -> (f64, f64) {
    let total: f64 = slots.iter().map(|s| s.duration).sum();
    let reserve = 0.5 + title_card_secs(brief);
    // the target t had t - min(reserve, 0.2 t) = total
    let target = if total + reserve >= 5.0 * reserve { total + reserve } else { total / 0.8 };
    (target - total, title_card_secs(brief))
}

/// Everything the planner is told about the brief and the craft, in the order it reads it.
pub fn plan_spec(defn: &Definition, slots: &[Slot]) -> String {
    format!(
        "## Brief\n{}\n{}\n## Skeleton\n{}\n\n## Craft rules\n{}\n{}{}{}{}\n{}{}{}\n## Framing\n{}",
        brief_text(&defn.brief),
        plan_text(&defn.brief),
        skeleton_text_with(slots, Some(cards_after(&defn.brief, slots))),
        rules_text(),
        crate::refs::baseline_text(),
        crate::refs::type_text(),
        crate::fonts::title_text(&defn.brief.design),
        crate::refs::motion_text(),
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
        out.push_str(&crate::designsys::sound_text(defn));
        return out;
    }
    if reach == Reach::Picture {
        out.push_str(&format!("\n## Craft rules\n{}\n", rules_text()));
    }
    out.push_str(&crate::refs::baseline_text());
    out.push_str(&crate::refs::type_text());
    out.push_str(&crate::fonts::title_text(&b.design));
    out.push_str(&crate::refs::motion_text());
    out.push_str(&crate::designsys::planner_text(defn));
    out.push_str(&crate::skills::planner_text(b));
    // the place as the scout read it: the camera works in it, and the camera effects follow
    // from it (a failing lamp flickers, D-57)
    out.push_str(&crate::ai::stage_section(defn));
    if reach == Reach::Picture {
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
           `card` a centred line, optionally with a quiet English line under it; `rule` a rule of the world with a small `head`; \
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
        notes.push("カットをビートの順（hook → establish → escalate → title）に並べ替えた".into());
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
        notes.push(format!("カットの長さを 1〜8 秒に収めた: {}", clamped.join(", ")));
    }
    let total: f64 = shots.iter().map(dur).sum();
    if target > 0.0 && total > 0.0 && (total - target).abs() > 0.1 * target {
        let k = target / total;
        for s in shots.iter_mut() {
            let d = round_to((dur(s) * k * 4.0).round() / 4.0, 2).clamp(1.0, 8.0);
            s["duration"] = serde_json::json!(d);
        }
        let now: f64 = shots.iter().map(dur).sum();
        notes.push(format!("カットの合計を {}秒 から {}秒 に合わせた（台本のカードを除いた持ち時間 {}秒）", fmt_g(round_to(total, 2)), fmt_g(round_to(now, 2)), fmt_g(target)));
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
