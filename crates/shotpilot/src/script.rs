//! The script: the teaser's text, written before the shots (the spine of the cut).
//!
//! A trailer that tells its story in text (first-person lines, a count, a house rule, a
//! last message) is written text-first: each shot then exists to show one line. The script
//! lives in shots.yaml (`script:`) and is editorial: it never re-renders a shot, only the
//! cut (D-25).
//!
//! Two kinds of lines:
//! - slots take their own time between shots, on black: `card`, `rule`, `title`,
//!   `message`, `black` (a silent gap). Placed with `after: <shot id | line id | start>`.
//! - overlays sit on a shot: `caption` (lower centre on a band) and `count` (lower left,
//!   like a note under a photograph). Placed with `on: <shot id>`, `at` seconds into it.

use serde_json::Value;

use crate::model::{fmt_g, num, str_of, ResolvedShot};

pub const SLOT_KINDS: &[&str] = &["card", "rule", "title", "message", "black"];
pub const OVERLAY_KINDS: &[&str] = &["caption", "count"];

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub id: String,
    pub kind: String,
    /// The main (Japanese) text.
    pub ja: String,
    /// A quiet English line under it (optional).
    pub en: String,
    /// `rule`: the small heading above the rule; `message`: the chat room name.
    pub head: String,
    /// `message`: who sent it.
    pub from: String,
    /// Seconds (slots: their length; overlays: how long they show).
    pub dur: f64,
    /// Slots: what they follow (a shot id, a line id, or `start`).
    pub after: Option<String>,
    /// Overlays: the shot they sit on, and when (seconds into it).
    pub on: Option<String>,
    pub at: f64,
    /// Sound effects on this line (`at` from the line's start), mixed at the cut.
    pub sfx: Vec<Value>,
}

impl Line {
    pub fn is_slot(&self) -> bool {
        SLOT_KINDS.contains(&self.kind.as_str())
    }
}

fn default_dur(kind: &str) -> f64 {
    match kind {
        "black" => 0.8,
        "title" => 4.0,
        "message" => 3.0,
        "count" => 1.4,
        "rule" => 2.0,
        "caption" => 2.4,
        _ => 2.6,
    }
}

/// `script:` from shots.yaml. Problems go to `errs`; ids of shots let lines refer to them.
pub fn parse(v: Option<&Value>, shot_ids: &[String], errs: &mut Vec<String>) -> Vec<Line> {
    let items = match v {
        None | Some(Value::Null) => return Vec::new(),
        Some(Value::Array(a)) => a.clone(),
        Some(_) => {
            errs.push("script: must be a list of lines".into());
            return Vec::new();
        }
    };
    let mut out: Vec<Line> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let at = format!("script[{i}]");
        let Some(m) = it.as_object() else {
            errs.push(format!("{at}: must be a mapping"));
            continue;
        };
        let kind = m.get("kind").map(str_of).unwrap_or_else(|| "caption".into());
        if !SLOT_KINDS.contains(&kind.as_str()) && !OVERLAY_KINDS.contains(&kind.as_str()) {
            errs.push(format!("{at}.kind: one of {}, {} (got '{kind}')", SLOT_KINDS.join(", "), OVERLAY_KINDS.join(", ")));
            continue;
        }
        let id = m.get("id").map(str_of).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("l{:02}", i + 1));
        if out.iter().any(|l| l.id == id) || shot_ids.contains(&id) {
            errs.push(format!("{at}.id: '{id}' is already used"));
        }
        let ja = m.get("ja").or(m.get("text")).map(str_of).unwrap_or_default();
        if ja.trim().is_empty() && kind != "black" {
            errs.push(format!("{at}.ja: the line's text is empty"));
        }
        let dur = m.get("dur").or(m.get("duration")).and_then(num).unwrap_or_else(|| default_dur(&kind));
        if !(0.2..=10.0).contains(&dur) {
            errs.push(format!("{at}.dur: 0.2..10 seconds (got {})", fmt_g(dur)));
        }
        let line = Line {
            id,
            ja,
            en: m.get("en").map(str_of).unwrap_or_default(),
            head: m.get("head").or(m.get("room")).map(str_of).unwrap_or_default(),
            from: m.get("from").map(str_of).unwrap_or_default(),
            dur,
            after: m.get("after").map(str_of).filter(|s| !s.is_empty()),
            on: m.get("on").map(str_of).filter(|s| !s.is_empty()),
            at: m.get("at").and_then(num).unwrap_or(0.3),
            sfx: m.get("sfx").and_then(Value::as_array).cloned().unwrap_or_default(),
            kind,
        };
        crate::sound::validate_line_sfx(&at, m.get("sfx"), errs);
        if line.is_slot() && line.after.is_none() {
            errs.push(format!("{at}.after: a {} needs `after: <shot id | line id | start>`", line.kind));
        }
        if !line.is_slot() && line.on.is_none() {
            errs.push(format!("{at}.on: a {} needs `on: <shot id>`", line.kind));
        }
        out.push(line);
    }
    // references
    for (i, l) in out.iter().enumerate() {
        if let Some(a) = &l.after {
            let ok = a == "start" || shot_ids.contains(a) || out.iter().any(|o| &o.id == a && o.is_slot() && o.id != l.id);
            if !ok {
                errs.push(format!("script[{i}].after: '{a}' is not a shot, a slot line or `start`"));
            }
        }
        if let Some(s) = &l.on {
            if !shot_ids.contains(s) {
                errs.push(format!("script[{i}].on: unknown shot '{s}'"));
            }
        }
    }
    out
}

/// One piece of the cut, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    /// A shot: its index in the resolved list.
    Shot(usize),
    /// A slot line: its index in the script.
    Slot(usize),
}

#[derive(Debug, Clone)]
pub struct Item {
    pub piece: Piece,
    pub id: String,
    /// Frames from the start of the cut.
    pub start: i64,
    pub frames: i64,
}

/// The cut in order: slots after `start`, then each shot followed by the slots after it
/// (and the slots after those). Frame-exact at `fps`.
pub fn layout(script: &[Line], shots: &[ResolvedShot], fps: i64) -> Vec<Item> {
    let fps = fps.max(1);
    let mut out: Vec<Item> = Vec::new();
    let mut t = 0i64;
    let mut placed = vec![false; script.len()];
    fn follow(target: &str, script: &[Line], placed: &mut Vec<bool>, fps: i64, t: &mut i64, out: &mut Vec<Item>) {
        for (i, l) in script.iter().enumerate() {
            if placed[i] || !l.is_slot() || l.after.as_deref() != Some(target) {
                continue;
            }
            placed[i] = true;
            let frames = ((l.dur * fps as f64).round() as i64).max(1);
            out.push(Item { piece: Piece::Slot(i), id: l.id.clone(), start: *t, frames });
            *t += frames;
            follow(&l.id.clone(), script, placed, fps, t, out);
        }
    }
    follow("start", script, &mut placed, fps, &mut t, &mut out);
    for (k, s) in shots.iter().enumerate() {
        out.push(Item { piece: Piece::Shot(k), id: s.id.clone(), start: t, frames: s.frames });
        t += s.frames;
        follow(&s.id, script, &mut placed, fps, &mut t, &mut out);
    }
    out
}

/// The overlay lines that sit on a shot.
pub fn overlays<'a>(script: &'a [Line], shot_id: &str) -> Vec<&'a Line> {
    script.iter().filter(|l| !l.is_slot() && l.on.as_deref() == Some(shot_id)).collect()
}
