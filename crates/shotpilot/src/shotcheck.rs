//! Shot check: code-level lint of a shot list against trailer craft rules.
//!
//! No model and no render: it reads the resolved shots only, so it runs on every save.
//! Severity follows how well a rule is supported:
//! - P0: sourced and unambiguous (e.g. Steam: don't open on a logo/title card). Blocks approval.
//! - P1: sourced rule with a judgement call, or a strong convention (30-degree rule, beat order).
//! - P2: inferred threshold, to be calibrated against measured trailers. Advice only.
//!
//! Findings carry a fix sentence so they can be handed back to the planner as-is.

use serde_json::{json, Value};

use crate::geometry::{self as g, round_to, Vec3};
use crate::model::{beat_guide, fmt_g, str_of, AnchorBox, Definition, ResolvedShot, BEATS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    P0,
    P1,
    P2,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::P0 => "P0",
            Severity::P1 => "P1",
            Severity::P2 => "P2",
        }
    }
}

/// A rule: id, severity, and where it comes from ("sourced" or "inferred").
pub struct Rule {
    pub id: &'static str,
    pub severity: Severity,
    pub basis: &'static str,
    pub summary: &'static str,
}

pub const RULES: &[Rule] = &[
    Rule { id: "open.title", severity: Severity::P0, basis: "sourced",
           summary: "Do not open on the title/logo; viewers decide in the first seconds" },
    Rule { id: "open.hook", severity: Severity::P1, basis: "sourced",
           summary: "Open on the hook" },
    Rule { id: "end.title", severity: Severity::P1, basis: "sourced",
           summary: "End on the title beat (title card, platforms, call to action)" },
    Rule { id: "beats.untagged", severity: Severity::P1, basis: "sourced",
           summary: "Every shot carries a beat (hook, establish, escalate, title)" },
    Rule { id: "beats.missing", severity: Severity::P1, basis: "sourced",
           summary: "The teaser has every beat" },
    Rule { id: "beats.order", severity: Severity::P1, basis: "sourced",
           summary: "Beats run hook -> establish -> escalate -> title" },
    Rule { id: "cut.jump", severity: Severity::P1, basis: "sourced",
           summary: "30-degree rule: consecutive shots of one subject change angle >= 30 deg or shot size" },
    Rule { id: "runtime.target", severity: Severity::P1, basis: "inferred",
           summary: "Runtime within 10% of the target" },
    Rule { id: "hook.long", severity: Severity::P1, basis: "inferred",
           summary: "The hook is short (teaser <= 6 s, gameplay trailer <= 3 s)" },
    Rule { id: "camera.inside", severity: Severity::P1, basis: "inferred",
           summary: "The camera is never inside its subject (outside its box, or >= 40 cm away when its size is unknown)" },
    Rule { id: "frame.lost", severity: Severity::P2, basis: "inferred",
           summary: "The subject reads: it fills >= 5% of the frame height (wide establishing and title shots excepted)" },
    Rule { id: "frame.cropped", severity: Severity::P2, basis: "inferred",
           summary: "A close-up shows a chosen part: the subject is at most 4x the frame height" },
    Rule { id: "shot.length", severity: Severity::P2, basis: "inferred",
           summary: "Shots run 1-8 s" },
    Rule { id: "tempo.flat", severity: Severity::P2, basis: "inferred",
           summary: "Shot lengths vary (longest >= 1.5x shortest)" },
    Rule { id: "tempo.escalate", severity: Severity::P2, basis: "inferred",
           summary: "Escalation cuts faster than the establishing shots" },
    Rule { id: "repeat.move", severity: Severity::P2, basis: "inferred",
           summary: "No move used on 3 shots in a row" },
    Rule { id: "repeat.lens", severity: Severity::P2, basis: "inferred",
           summary: "No focal length used on 3 shots in a row" },
    Rule { id: "title.short", severity: Severity::P2, basis: "inferred",
           summary: "The title beat holds >= 2 s so the card can be read" },
];

pub fn rule(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    /// The shot it is about; None for the whole teaser.
    pub shot: Option<String>,
    pub message: String,
    pub fix: String,
}

impl Finding {
    pub fn to_json(&self) -> Value {
        json!({
            "rule": self.rule, "severity": self.severity.as_str(), "shot": self.shot,
            "message": self.message, "fix": self.fix,
            "basis": rule(self.rule).map(|r| r.basis).unwrap_or("inferred"),
        })
    }

    /// One line for the CLI.
    pub fn line(&self) -> String {
        match &self.shot {
            Some(s) => format!("[{}] {} {s}: {}", self.severity.as_str(), self.rule, self.message),
            None => format!("[{}] {}: {}", self.severity.as_str(), self.rule, self.message),
        }
    }
}

struct Out(Vec<Finding>);

impl Out {
    fn push(&mut self, id: &'static str, shot: Option<&str>, message: String, fix: impl Into<String>) {
        let severity = rule(id).map(|r| r.severity).unwrap_or(Severity::P2);
        self.0.push(Finding { rule: id, severity, shot: shot.map(str::to_string), message, fix: fix.into() });
    }
}

/// The move a shot makes: `static`, a move type, or `keyframed` for an explicit end pose.
fn move_kind(defn: &Definition, shot: &ResolvedShot) -> String {
    let cam = &defn.shots.get(shot.index).map(|s| s["camera"].clone()).unwrap_or(Value::Null);
    if cam.get("end").is_some_and(|e| !e.is_null()) {
        return "keyframed".into();
    }
    match cam.get("move").and_then(|m| m.get("type")) {
        Some(t) => str_of(t),
        None => "static".into(),
    }
}

fn dist(a: Vec3, b: Vec3) -> f64 {
    g::length(g::sub(a, b))
}

/// Angle in degrees between two directions.
fn angle_between(a: Vec3, b: Vec3) -> f64 {
    let (la, lb) = (g::length(a), g::length(b));
    if la < 1e-6 || lb < 1e-6 {
        return 0.0;
    }
    let d = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]) / (la * lb);
    d.clamp(-1.0, 1.0).acos().to_degrees()
}

/// How a shot frames its subject (the look_at anchor, else the anchor it is placed from).
#[derive(Debug, Clone)]
pub struct Framing {
    pub subject: String,
    /// The subject's size is known (else the person-sized default box was used).
    pub known: bool,
    /// Projected height of the subject's box / frame height, at the first and last frame.
    /// Infinite when the camera is inside or right against the box.
    pub fill: (f64, f64),
    pub inside: bool,
}

impl Framing {
    pub fn to_json(&self) -> Value {
        let f = |v: f64| if v.is_finite() { json!(round_to(v, 2)) } else { Value::Null };
        json!({"subject": self.subject, "known": self.known, "fill": [f(self.fill.0), f(self.fill.1)],
               "inside": self.inside, "size": size_name(self.fill.0.min(self.fill.1))})
    }
}

/// A plain shot-size word for how much of the frame the subject fills.
pub fn size_name(fill: f64) -> &'static str {
    match fill {
        f if f > 1.6 => "detail",
        f if f > 0.75 => "close",
        f if f > 0.35 => "medium",
        f if f > 0.12 => "full",
        _ => "wide",
    }
}

fn subject_name(defn: &Definition, shot: &ResolvedShot) -> Option<String> {
    let cam = &defn.shots.get(shot.index)?["camera"];
    let look = cam.get("look_at").and_then(Value::as_str).filter(|n| defn.anchor(n).is_some());
    look.or_else(|| cam.get("anchor").and_then(Value::as_str)).map(str::to_string)
}

fn fill_at(b: &AnchorBox, cam: &crate::geometry::Xform, focal: f64, sensor: f64, w: u32, h: u32) -> f64 {
    let pts: Vec<Option<(f64, f64, f64)>> = b.corners().iter().map(|p| g::project(*p, cam, focal, w, h, sensor)).collect();
    if pts.iter().any(Option::is_none) {
        return f64::INFINITY; // a corner behind the lens: the camera is inside or against it
    }
    let ys: Vec<f64> = pts.iter().flatten().map(|p| p.1).collect();
    let span = ys.iter().cloned().fold(f64::MIN, f64::max) - ys.iter().cloned().fold(f64::MAX, f64::min);
    span / h as f64
}

pub fn framing(defn: &Definition, shot: &ResolvedShot) -> Option<Framing> {
    let subject = subject_name(defn, shot)?;
    let b = defn.anchor_box(&subject)?;
    let (w, h) = defn.teaser.resolution;
    let (w, h) = (w.max(1) as u32, h.max(1) as u32);
    let (focal, sensor) = (shot.focal_length(), shot.sensor_width());
    let fill = (fill_at(&b, &shot.start, focal, sensor, w, h), fill_at(&b, &shot.end, focal, sensor, w, h));
    let known = defn.known_box(&subject).is_some();
    let inside = known && (b.contains(shot.start.location, 10.0) || b.contains(shot.end.location, 10.0));
    Some(Framing { subject, known, fill, inside })
}

/// Lint a shot list. Findings are sorted most severe first, then by shot order.
pub fn check(defn: &Definition, shots: &[ResolvedShot]) -> Vec<Finding> {
    let mut out = Out(Vec::new());
    if shots.is_empty() {
        return out.0;
    }
    let gameplay = defn.brief.is_gameplay();
    let beats: Vec<Option<&str>> = shots.iter().map(|s| s.beat.as_deref()).collect();
    let tagged = beats.iter().any(Option::is_some);

    // ---- structure (beats)
    if !tagged {
        out.push("beats.untagged", None,
                 "no shot has a `beat`; tag shots with hook / establish / escalate / title to check the teaser shape".into(),
                 "Give every shot a beat.");
    } else {
        for s in shots.iter().filter(|s| s.beat.is_none()) {
            out.push("beats.untagged", Some(&s.id), "no beat".into(), "Give this shot a beat.");
        }
        for b in BEATS {
            if !beats.contains(&Some(*b)) {
                out.push("beats.missing", None, format!("no `{b}` shot ({})", beat_guide(b)),
                         format!("Add a `{b}` shot or retag one."));
            }
        }
        match beats.first() {
            Some(Some("title")) => out.push("open.title", Some(&shots[0].id),
                "the cut opens on `title`; the first seconds decide whether anyone keeps watching".into(),
                "Open on the hook; move the title shot to the end."),
            Some(Some(first)) if *first != "hook" => out.push("open.hook", Some(&shots[0].id),
                format!("the cut opens on `{first}`; teasers usually open on the hook"),
                "Open on the hook shot."),
            _ => {}
        }
        if let Some(Some(last)) = beats.last() {
            if *last != "title" {
                out.push("end.title", Some(&shots[shots.len() - 1].id),
                         format!("the cut ends on `{last}`; end on the title beat"),
                         "End on a calm title shot with room for the card.");
            }
        }
        let ranked: Vec<(usize, usize)> = beats
            .iter()
            .enumerate()
            .filter_map(|(i, b)| b.and_then(|b| BEATS.iter().position(|x| *x == b)).map(|r| (i, r)))
            .collect();
        if let Some(w) = ranked.windows(2).find(|w| w[1].1 < w[0].1) {
            out.push("beats.order", Some(&shots[w[1].0].id),
                     format!("`{}` comes after `{}`; beats go backwards", BEATS[w[1].1], BEATS[w[0].1]),
                     "Reorder so beats run hook -> establish -> escalate -> title, or retag.");
        }
    }

    // ---- timing
    let total: f64 = shots.iter().map(|s| s.duration).sum();
    let target = defn.teaser.target_duration;
    if target != 0.0 && (total - target).abs() > 0.1 * target {
        out.push("runtime.target", None,
                 format!("runtime {}s is more than 10% off the {}s target", fmt_g(round_to(total, 2)), fmt_g(target)),
                 format!("Bring the shot durations to about {}s in total.", fmt_g(target)));
    }
    let hook: f64 = shots.iter().filter(|s| s.beat.as_deref() == Some("hook")).map(|s| s.duration).sum();
    let hook_max = if gameplay { 3.0 } else { 6.0 };
    if hook > hook_max {
        out.push("hook.long", None,
                 format!("hook runs {}s; keep it under ~{}s", fmt_g(round_to(hook, 2)), fmt_g(hook_max - 1.0)),
                 "Shorten the hook to one striking image.");
    }
    for s in shots {
        if s.duration < 1.0 || s.duration > 8.0 {
            out.push("shot.length", Some(&s.id), format!("{}s is outside 1-8s", fmt_g(s.duration)),
                     if s.duration < 1.0 { "Hold it at least 1s." } else { "Split it or shorten it below 8s." });
        }
    }
    if shots.len() >= 4 {
        let (lo, hi) = shots.iter().fold((f64::MAX, 0.0_f64), |(lo, hi), s| (lo.min(s.duration), hi.max(s.duration)));
        if hi < 1.5 * lo {
            out.push("tempo.flat", None,
                     format!("every shot runs {}-{}s; the tempo never changes", fmt_g(lo), fmt_g(hi)),
                     "Let establishing shots breathe and cut escalation shorter.");
        }
    }
    let mean = |beat: &str| {
        let d: Vec<f64> = shots.iter().filter(|s| s.beat.as_deref() == Some(beat)).map(|s| s.duration).collect();
        (!d.is_empty()).then(|| d.iter().sum::<f64>() / d.len() as f64)
    };
    if let (Some(est), Some(esc)) = (mean("establish"), mean("escalate")) {
        if esc > est {
            out.push("tempo.escalate", None,
                     format!("escalate shots average {}s, longer than establish ({}s)", fmt_g(round_to(esc, 1)), fmt_g(round_to(est, 1))),
                     "Cut escalation shots shorter so tension rises.");
        }
    }
    let title: f64 = shots.iter().filter(|s| s.beat.as_deref() == Some("title")).map(|s| s.duration).sum();
    if beats.contains(&Some("title")) && title < 2.0 {
        out.push("title.short", None, format!("title beat holds {}s", fmt_g(title)), "Hold the title shot at least 2s.");
    }

    // ---- camera and framing
    for s in shots {
        let fr = framing(defn, s);
        match &fr {
            Some(f) if f.known => {
                if f.inside {
                    out.push("camera.inside", Some(&s.id), format!("the camera is inside `{}`", f.subject),
                             "Pull the camera back out of the subject, or frame a part of it with a longer lens.");
                }
                let (lo, hi) = (f.fill.0.min(f.fill.1), f.fill.0.max(f.fill.1));
                let wide_ok = matches!(s.beat.as_deref(), Some("establish") | Some("title"));
                if !f.inside && hi < 0.05 && !wide_ok {
                    out.push("frame.lost", Some(&s.id),
                             format!("`{}` fills only {}% of the frame height", f.subject, fmt_g((hi * 100.0).round())),
                             "Move closer or use a longer lens so the subject reads.");
                }
                if !f.inside && lo.is_finite() && lo > 4.0 {
                    out.push("frame.cropped", Some(&s.id),
                             format!("`{}` is {}x the frame height; only a sliver of it shows", f.subject, fmt_g(round_to(lo, 1))),
                             "Pull back or use a wider lens, unless this detail is the point.");
                }
            }
            _ => {
                if let Some(t) = s.target {
                    let near = dist(s.start.location, t).min(dist(s.end.location, t));
                    if near < 40.0 {
                        out.push("camera.inside", Some(&s.id),
                                 format!("camera comes within {}cm of what it looks at", fmt_g(near.round())),
                                 "Pull the camera back (larger offset) or use a longer lens instead.");
                    }
                }
            }
        }
    }
    let moves: Vec<String> = shots.iter().map(|s| move_kind(defn, s)).collect();
    for w in 0..shots.len().saturating_sub(2) {
        if moves[w] == moves[w + 1] && moves[w] == moves[w + 2] && moves[w] != "keyframed" {
            out.push("repeat.move", Some(&shots[w + 2].id),
                     format!("third `{}` in a row", moves[w]), "Vary the move (or hold one shot static).");
        }
        let f = |i: usize| shots[i].focal_length();
        if (f(w) - f(w + 1)).abs() < 0.5 && (f(w) - f(w + 2)).abs() < 0.5 {
            out.push("repeat.lens", Some(&shots[w + 2].id),
                     format!("third {}mm shot in a row", fmt_g(f(w))), "Change the focal length (shot size).");
        }
    }
    for w in shots.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let (Some(ta), Some(tb)) = (a.target, b.target) else { continue };
        if dist(ta, tb) > 150.0 {
            continue; // a different subject: no jump cut
        }
        let va = g::sub(a.end.location, ta);
        let vb = g::sub(b.start.location, tb);
        let angle = angle_between(va, vb);
        // visible height ~ distance / focal length: similar ratio = similar shot size
        let size = (g::length(va) / a.focal_length()) / (g::length(vb) / b.focal_length()).max(1e-6);
        if angle < 30.0 && (0.75..1.33).contains(&size) {
            out.push("cut.jump", Some(&b.id),
                     format!("cuts from `{}` to the same subject {}° apart at a similar size; reads as a jump cut",
                             a.id, fmt_g(angle.round())),
                     "Move the camera >= 30° around the subject, or change the shot size (closer/longer lens).");
        }
    }

    out.0.sort_by(|x, y| {
        let pos = |f: &Finding| f.shot.as_ref().and_then(|id| shots.iter().position(|s| &s.id == id)).unwrap_or(usize::MAX);
        x.severity.cmp(&y.severity).then(pos(x).cmp(&pos(y)))
    });
    out.0
}

/// Findings as a block to hand back to the planner (one retry).
pub fn for_planner(findings: &[Finding]) -> String {
    let mut lines = vec![
        "The shot list has these problems (P0 must fix, P1 should fix, P2 nice to have).".to_string(),
        "Return the whole corrected shot list; keep what is not mentioned.".to_string(),
    ];
    for f in findings {
        lines.push(format!("- {}  Fix: {}", f.line(), f.fix));
    }
    lines.join("\n")
}
