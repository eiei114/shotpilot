//! Reverse sync: bring edits made in Sequencer back into shots.yaml without losing them.
//!
//! Three-way comparison per shot:
//!   base   = what Shotpilot last pushed (state.json)
//!   engine = what the sequence asset holds now
//!   ours   = what shots.yaml resolves to now
//!
//!   engine == base                 -> nobody touched the asset; nothing to pull
//!   engine != base, ours == base   -> human edited in the editor: write it into shots.yaml
//!   engine != base, ours == engine -> both sides agree already; just record it
//!   engine != base, ours != base   -> conflict: both changed. user picks with --prefer

use serde_json::{json, Map, Value};

use crate::engines::readback_subset;
use crate::geometry::{angle_delta, round_to};
use crate::model::{num, ResolvedShot};
use crate::state::now_iso;

pub const TOL_LOC: f64 = 0.5;
pub const TOL_ROT: f64 = 0.05;
pub const TOL_LENS: f64 = 0.01;

fn f(v: &Value) -> f64 {
    num(v).unwrap_or(0.0)
}

pub fn same_payload(a: Option<&Value>, b: Option<&Value>) -> bool {
    let (a, b) = match (a.filter(|v| !v.is_null()), b.filter(|v| !v.is_null())) {
        (None, None) => return true,
        (Some(a), Some(b)) => (a, b),
        _ => return false,
    };
    if a["frames"].as_i64().unwrap_or(-1) != b["frames"].as_i64().unwrap_or(-2) {
        return false;
    }
    for k in ["start", "end"] {
        if a[k].is_null() || b[k].is_null() {
            return false;
        }
        for i in 0..3 {
            if (f(&a[k]["location"][i]) - f(&b[k]["location"][i])).abs() > TOL_LOC {
                return false;
            }
            if angle_delta(f(&b[k]["rotation"][i]), f(&a[k]["rotation"][i])).abs() > TOL_ROT {
                return false;
            }
        }
    }
    for k in ["focal_length", "aperture"] {
        if (f(&a["lens"][k]) - f(&b["lens"][k])).abs() > TOL_LENS {
            return false;
        }
    }
    true
}

fn fmt_vec(v: &Value) -> String {
    let parts: Vec<String> = v.as_array().map(|a| a.iter().map(|x| format!("{:.1}", f(x))).collect()).unwrap_or_default();
    format!("[{}]", parts.join(", "))
}

pub fn describe_diff(old: &Value, new: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if old["frames"] != new["frames"] {
        out.push(format!("frames {} -> {}", old["frames"], new["frames"]));
    }
    for k in ["start", "end"] {
        for (part, tol) in [("location", TOL_LOC), ("rotation", TOL_ROT)] {
            let (a, b) = (&old[k][part], &new[k][part]);
            if let (Some(x), Some(y)) = (a.as_array(), b.as_array()) {
                if x.iter().zip(y).any(|(p, q)| (f(p) - f(q)).abs() > tol) {
                    out.push(format!("{k}.{part} {} -> {}", fmt_vec(a), fmt_vec(b)));
                }
            }
        }
    }
    for k in ["focal_length", "aperture"] {
        let (a, b) = (&old["lens"][k], &new["lens"][k]);
        if !a.is_null() && !b.is_null() && (f(a) - f(b)).abs() > TOL_LENS {
            out.push(format!("lens.{k} {} -> {}", crate::model::fmt_g(f(a)), crate::model::fmt_g(f(b))));
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullKind {
    Clean,
    EngineEdit,
    Agree,
    Conflict,
    Missing,
    NeverPushed,
}

impl PullKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PullKind::Clean => "clean",
            PullKind::EngineEdit => "engine_edit",
            PullKind::Agree => "agree",
            PullKind::Conflict => "conflict",
            PullKind::Missing => "missing",
            PullKind::NeverPushed => "never_pushed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PullDecision {
    pub shot_id: String,
    pub kind: PullKind,
    pub engine: Option<Value>,
    pub changes: Vec<String>,
}

pub fn decide(shot_id: &str, engine: Option<&Value>, base: Option<&Value>, ours: &ResolvedShot) -> PullDecision {
    let d = |kind, engine: Option<&Value>, changes| PullDecision {
        shot_id: shot_id.to_string(),
        kind,
        engine: engine.cloned(),
        changes,
    };
    let ours_rb = readback_subset(&ours.engine_payload());
    let Some(engine) = engine.filter(|v| !v.is_null()) else {
        return d(PullKind::Missing, None, vec![]);
    };
    let Some(base) = base else {
        return d(PullKind::NeverPushed, Some(engine), vec![]);
    };
    let base_rb = readback_subset(base);
    if same_payload(Some(engine), Some(&base_rb)) {
        return d(PullKind::Clean, Some(engine), vec![]);
    }
    let changes = describe_diff(&base_rb, engine);
    if same_payload(Some(&ours_rb), Some(&base_rb)) {
        return d(PullKind::EngineEdit, Some(engine), changes);
    }
    if same_payload(Some(&ours_rb), Some(engine)) {
        return d(PullKind::Agree, Some(engine), changes);
    }
    d(PullKind::Conflict, Some(engine), changes)
}

/// Rewrite one shot node so it resolves to exactly what the engine holds.
///
/// High-level camera intent (anchor/offset/move/look_at) can't express an arbitrary
/// hand-placed camera, so the camera becomes explicit start/end transforms. Returns the
/// new node and a comment for the `camera:` line that keeps the old intent visible.
pub fn apply_engine_to_shot(node: &Value, engine: &Value, ours: &ResolvedShot) -> (Value, Option<String>) {
    let mut node = node.clone();
    let cam = node["camera"].as_object().cloned().unwrap_or_default();
    let was: Vec<(String, Value)> = ["anchor", "offset", "look_at", "move"]
        .iter()
        .filter_map(|k| cam.get(*k).map(|v| (k.to_string(), v.clone())))
        .collect();
    let lens_old = cam.get("lens").and_then(Value::as_object).cloned().unwrap_or_default();

    let r = |v: &Value, n: usize| -> Value {
        Value::Array(v.as_array().map(|a| a.iter().map(|x| json!(round_to(f(x), n))).collect()).unwrap_or_default())
    };
    let mut new_cam = Map::new();
    new_cam.insert("location".into(), r(&engine["start"]["location"], 2));
    new_cam.insert("rotation".into(), r(&engine["start"]["rotation"], 3));
    let mut still = engine.clone();
    still["end"] = engine["start"].clone();
    if !same_payload(Some(&still), Some(engine)) {
        new_cam.insert("end".into(), json!({
            "location": r(&engine["end"]["location"], 2),
            "rotation": r(&engine["end"]["rotation"], 3),
            "curve": ours.curve,
        }));
    }
    let mut lens = lens_old.clone();
    let elens = &engine["lens"];
    for k in ["focal_length", "aperture"] {
        if let Some(e) = elens.get(k).and_then(num) {
            if (e - ours.lens.get(k).and_then(num).unwrap_or(0.0)).abs() > TOL_LENS {
                lens.insert(k.into(), json!(round_to(e, 2)));
            }
        }
    }
    if let Some(fd) = elens.get("focus_distance").and_then(num) {
        if !was.is_empty() || lens_old.contains_key("focus_distance") {
            lens.insert("focus_distance".into(), json!(round_to(fd, 1)));
        }
    }
    if !lens.is_empty() {
        new_cam.insert("lens".into(), Value::Object(lens));
    }
    node["camera"] = Value::Object(new_cam);
    let comment = (!was.is_empty()).then(|| {
        let summary: Vec<String> =
            was.iter().map(|(k, v)| format!("{k}={}", crate::yamlio::emit_flow(v))).collect();
        format!("pulled from engine {}; was {}", &now_iso()[..16], summary.join(", "))
    });
    let frames = engine["frames"].as_i64().unwrap_or(ours.frames);
    if frames != ours.frames {
        let fps = engine["fps"].as_f64().filter(|v| *v > 0.0).unwrap_or(ours.fps as f64);
        node["duration"] = json!(round_to(frames as f64 / fps, 3));
    }
    (node, comment)
}
