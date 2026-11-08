//! Fingerprints decide what is stale.
//!
//! A shot's render fingerprint covers only what changes pixels for that shot. Order,
//! purpose and notes are deliberately excluded, so reordering the cut or rewording a
//! note never triggers a re-render.
//!
//! The digest is SHA-256 over canonical JSON (sorted keys, compact separators), the
//! same bytes the Python version hashed, so engine takes made before the Rust port
//! are still recognised.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::bail;
use crate::error::Result;
use crate::geometry::round_half_even;
use crate::model::{num, Definition, ResolvedShot};

/// Bump when the engine-side script changes output for the same input.
pub const RENDERER_VERSION: i64 = 1;
/// Bump when the local previz drawing changes. Kept separate so a previz change
/// never invalidates expensive engine takes.
pub const PREVIZ_VERSION: i64 = 5;

pub const BUILTIN_ORDER: &[&str] = &["previz", "still", "preview", "final"];

fn builtin(name: &str) -> Option<Value> {
    Some(match name {
        // Local previz: the level drawn and lit by the game, no engine launch: seconds per shot.
        "previz" => json!({"runner": "previz", "scale": 1.0 / 6.0, "frame_step": 3, "format": "png"}),
        // Engine, three frames per shot (first/mid/last) to check framing and light.
        "still" => json!({"runner": "engine", "scale": 0.5, "frames": "keys", "spatial_samples": 1,
                          "temporal_samples": 1, "screen_percentage": 50, "warmup_frames": 8, "format": "jpg"}),
        // Engine, every frame at half res, no AA accumulation.
        "preview" => json!({"runner": "engine", "scale": 0.5, "frames": "all", "spatial_samples": 1,
                            "temporal_samples": 1, "screen_percentage": 50, "warmup_frames": 8, "format": "jpg"}),
        // Delivery frames.
        "final" => json!({"runner": "engine", "scale": 1.0, "frames": "all", "spatial_samples": 1,
                          "temporal_samples": 8, "screen_percentage": 100, "warmup_frames": 32, "format": "png"}),
        _ => return None,
    })
}

pub fn quality(defn: &Definition, name: &str) -> Result<Map<String, Value>> {
    let mut base = match builtin(name) {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    if let Some(Value::Object(over)) = defn.qualities.get(name) {
        for (k, v) in over {
            base.insert(k.clone(), v.clone());
        }
    }
    if base.is_empty() {
        let mut known: Vec<String> = BUILTIN_ORDER.iter().map(|s| s.to_string()).collect();
        known.extend(defn.qualities.keys().cloned());
        known.sort();
        known.dedup();
        bail!("unknown quality '{name}'. known: {}", known.join(", "));
    }
    for (k, v) in [("runner", json!("engine")), ("frames", json!("all")), ("format", json!("png")), ("scale", json!(1.0))] {
        base.entry(k).or_insert(v);
    }
    // a streamed level warms up on the shot's own camera (the lead-in of its camera cut)
    if base.get("runner").and_then(Value::as_str) == Some("engine") && crate::engines::streaming_lead(defn) > 0 {
        base.entry("warmup_camera_cut").or_insert(json!(true));
    }
    Ok(base)
}

pub fn is_previz(q: &Map<String, Value>) -> bool {
    q.get("runner").and_then(Value::as_str) == Some("previz")
}

pub fn quality_names(defn: &Definition) -> Vec<String> {
    let mut out: Vec<String> = BUILTIN_ORDER.iter().map(|s| s.to_string()).collect();
    for q in defn.qualities.keys() {
        if !out.contains(q) {
            out.push(q.clone());
        }
    }
    out
}

pub fn resolution(defn: &Definition, q: &Map<String, Value>) -> (i64, i64) {
    let (w, h) = defn.teaser.resolution;
    let s = q.get("scale").and_then(num).unwrap_or(1.0);
    // even numbers keep video encoders happy
    let even = |v: i64| (round_half_even(v as f64 * s / 2.0) * 2).max(2);
    (even(w), even(h))
}

pub fn frame_list(shot: &ResolvedShot, q: &Map<String, Value>) -> Vec<i64> {
    let n = shot.frames;
    match q.get("frames").and_then(Value::as_str) {
        Some("keys") => {
            let mut v = vec![0, (n - 1) / 2, n - 1];
            v.sort_unstable();
            v.dedup();
            v
        }
        Some("mid") => vec![(n - 1) / 2],
        _ => {
            let step = q.get("frame_step").and_then(num).map(|f| f as i64).unwrap_or(1).max(1);
            let mut frames: Vec<i64> = (0..n).step_by(step as usize).collect();
            if frames.last() != Some(&(n - 1)) {
                frames.push(n - 1);
            }
            frames
        }
    }
}

/// Canonical JSON: sorted keys, `,`/`:` separators, non-ASCII kept as-is.
pub fn canonical(v: &Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out);
    out
}

fn write_canonical(v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_canonical(&m[k.as_str()], out);
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(x, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

pub fn digest(v: &Value) -> String {
    let h = Sha256::digest(canonical(v).as_bytes());
    h.iter().map(|b| format!("{b:02x}")).collect()
}

/// What the engine's shot sequence must contain.
pub fn build_fingerprint(defn: &Definition, shot: &ResolvedShot) -> String {
    let mut body = json!({
        "v": RENDERER_VERSION,
        "map": defn.project.map,
        "content_root": defn.project.content_root,
        "sun_actor": defn.project.sun_actor,
        "shot": shot.engine_payload(),
    });
    // a streamed level builds its sequences with a lead-in (only there, so others keep their builds)
    let lead = crate::engines::streaming_lead(defn);
    if lead > 0 {
        body["lead"] = json!(lead);
    }
    digest(&body)
}

/// What a human approves when signing off the script (D-38): the text and where it sits,
/// and the cut it is the spine of (each shot's id, beat, purpose and length, in order).
/// Cameras, light, sound and the finish are not in it: rebuilding the cameras for an
/// approved script keeps the approval. Editorial only; it never decides a re-render.
pub fn script_fingerprint(defn: &Definition) -> String {
    let lines: Vec<Value> = defn
        .script
        .iter()
        .map(|l| json!({"id": l.id, "kind": l.kind, "ja": l.ja, "en": l.en, "head": l.head, "from": l.from,
                        "dur": l.dur, "after": l.after, "on": l.on, "at": l.at}))
        .collect();
    let shots: Vec<Value> = defn
        .shots
        .iter()
        .map(|s| json!({"id": s.get("id"), "beat": s.get("beat"), "purpose": s.get("purpose"),
                        "duration": s.get("duration").and_then(num)}))
        .collect();
    digest(&json!({"script": lines, "shots": shots}))
}

/// What a human approves when signing off a shot's framing: the shot's picture, independent
/// of quality presets. Any edit that changes pixels changes it (and so drops the approval);
/// beats, purpose, notes, markers and sound do not.
pub fn look_fingerprint(defn: &Definition, shot: &ResolvedShot) -> String {
    let mut payload = shot.engine_payload();
    if let Some(m) = payload.as_object_mut() {
        m.remove("markers");
    }
    digest(&json!({"shot": payload, "map": defn.project.map}))
}

pub fn render_fingerprint(defn: &Definition, shot: &ResolvedShot, quality_name: &str) -> Result<String> {
    let q = quality(defn, quality_name)?;
    let mut payload = shot.engine_payload();
    if let Some(m) = payload.as_object_mut() {
        m.remove("markers"); // markers are editorial metadata, not pixels
    }
    let (w, h) = resolution(defn, &q);
    let mut body = json!({
        "v": RENDERER_VERSION,
        "shot": payload,
        "quality": q,
        "resolution": [w, h],
    });
    if is_previz(&q) {
        let anchors: Map<String, Value> = {
            let mut a = defn.anchors.clone();
            a.sort_by(|x, y| x.0.cmp(&y.0));
            a.into_iter().map(|(k, v)| (k, json!(v))).collect()
        };
        body["anchors"] = Value::Object(anchors);
        // only when sizes are known, so previz takes of projects without them stay valid
        if !defn.boxes.is_empty() {
            let mut b = defn.boxes.clone();
            b.sort_by(|x, y| x.0.cmp(&y.0));
            body["anchor_boxes"] = Value::Object(b.into_iter().map(|(k, v)| (k, v.to_json())).collect());
        }
        body["previz"] = json!(PREVIZ_VERSION);
        // whether a level is drawn around the anchors; what it holds is in the take's deps
        if let Some(kind) = crate::previz::set_kind(defn) {
            body["set"] = json!(kind);
        }
    } else {
        body["map"] = json!(defn.project.map);
        body["sun_actor"] = json!(defn.project.sun_actor);
        body["engine"] = json!(defn.project.engine);
        // launch flags and the game mode can change what the game builds; omitted when unset so
        // fingerprints of projects without them stay as they were
        if !defn.project.ue_args.is_empty() {
            body["ue_args"] = json!(defn.project.ue_args);
        }
        if let Some(g) = &defn.project.game_mode {
            body["game_mode"] = json!(g);
        }
        // the page that draws three takes has its own version, so it never moves UE or mock takes
        if defn.project.engine == "three" {
            body["three"] = json!(crate::three::RENDERER_VERSION);
        }
    }
    Ok(digest(&body))
}
