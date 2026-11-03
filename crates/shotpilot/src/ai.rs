//! Claude-assisted shot planning and partial revision.
//!
//! `plan`   : pitch + anchors -> a first shot list (high-level camera intent only).
//! `revise` : natural-language note -> edits to the named shots only, plus an optional
//!            reorder. Everything Claude returns is validated by resolving it before it
//!            touches shots.yaml, and the edit scope is enforced in code, not by prompt.
//!
//! There is no official Anthropic SDK for Rust, so this speaks the Messages API over
//! HTTPS directly (streaming, structured outputs, server-side refusal fallback).

use std::io::{BufRead, BufReader};
use std::time::Duration;

use serde_json::{json, Value};

use crate::bail;
use crate::error::{Error, Result};
use crate::model::{self, beat_guide, num, tod_names_sorted, Definition, BEATS, MOVES};
use crate::yamlio;

pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_VERSION: &str = "2026-10-21";
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

pub fn model_name() -> String {
    std::env::var("SHOTPILOT_MODEL").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_MODEL.into())
}

const CONVENTIONS: &str = "\
Unreal Engine conventions: units are centimeters, Z is up, X forward, Y right.
A camera is placed relative to an anchor (a named point in the level) with an
offset [x, y, z] in world space, and looks at that anchor unless look_at names
another anchor. 1.7 m eye height is z=170 relative to the ground. Typical framing
distances: close-up 80-200 cm at 50-85 mm, medium 250-500 cm at 35-50 mm,
wide 800-3000 cm at 18-28 mm.
Moves: static, dolly_in/dolly_out (distance cm along view), truck_left/truck_right
(distance cm sideways, keeps aim on the subject), crane_up/crane_down (distance cm),
orbit_left/orbit_right (degrees around the subject).";

fn shot_schema() -> Value {
    let mut tod = vec!["default"];
    tod.extend(tod_names_sorted());
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "string", "description": "lowercase id like s01_courtyard"},
            "beat": {"type": "string", "enum": BEATS},
            "purpose": {"type": "string", "description": "what this shot must communicate, one sentence"},
            "duration": {"type": "number"},
            "anchor": {"type": "string"},
            "offset": {"type": "array", "items": {"type": "number"}},
            "look_at": {"type": "string", "description": "anchor name, or empty string to look at `anchor`"},
            "focal_length": {"type": "number"},
            "move": {"type": "string", "enum": MOVES},
            "move_amount": {"type": "number", "description": "cm for dolly/truck/crane, degrees for orbit, 0 for static"},
            "time_of_day": {"type": "string", "enum": tod},
            "notes": {"type": "string"},
        },
        "required": ["id", "beat", "purpose", "duration", "anchor", "offset", "look_at", "focal_length", "move",
                     "move_amount", "time_of_day", "notes"],
        "additionalProperties": false,
    })
}

pub fn plan_schema() -> Value {
    json!({
        "type": "object",
        "properties": {"rationale": {"type": "string"}, "shots": {"type": "array", "items": shot_schema()}},
        "required": ["rationale", "shots"],
        "additionalProperties": false,
    })
}

pub fn revise_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "edits": {"type": "array", "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "yaml": {"type": "string", "description": "the complete replacement YAML mapping for this shot"},
                    "reason": {"type": "string"},
                },
                "required": ["id", "yaml", "reason"],
                "additionalProperties": false,
            }},
            "order": {"type": "array", "items": {"type": "string"},
                      "description": "full list of shot ids in the new order, or empty to keep the order"},
        },
        "required": ["summary", "edits", "order"],
        "additionalProperties": false,
    })
}

/// Sends one Messages API request body and returns the final answer text.
/// Tests swap in a fake; the real one is [`HttpClaude`].
pub trait Claude {
    fn ask(&self, request: &Value) -> Result<String>;
}

pub fn request(system: &str, user: &str, schema: Value) -> Value {
    request_with(system, json!(user), schema)
}

/// Like [`request`], with the user turn given as content (a string or content blocks).
fn request_with(system: &str, user: Value, schema: Value) -> Value {
    json!({
        "model": model_name(),
        "max_tokens": 64000,
        "stream": true,
        "thinking": {"type": "adaptive"},
        "output_config": {"effort": "medium", "format": {"type": "json_schema", "schema": schema}},
        // on a policy decline, the API re-runs the request on Anthropic's recommended fallback model
        "fallbacks": "default",
        "system": system,
        "messages": [{"role": "user", "content": user}],
    })
}

fn ask_json(claude: &dyn Claude, system: &str, user: Value, schema: Value) -> Result<Value> {
    let text = claude.ask(&request_with(system, user, schema))?;
    serde_json::from_str(&text).map_err(|e| Error::user(format!("Claude returned malformed JSON: {e}")))
}

// --------------------------------------------------------------------------- HTTP

pub struct HttpClaude {
    base_url: String,
    auth: (String, String),
    oauth: bool,
}

impl HttpClaude {
    /// A client with this API key (base URL still follows ANTHROPIC_BASE_URL).
    pub fn with_key(key: String) -> HttpClaude {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.anthropic.com".into());
        HttpClaude { base_url, auth: ("x-api-key".into(), key), oauth: false }
    }

    pub fn from_env() -> Result<HttpClaude> {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.anthropic.com".into());
        if let Some(key) = std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()) {
            return Ok(HttpClaude { base_url, auth: ("x-api-key".into(), key), oauth: false });
        }
        if let Some(tok) = std::env::var("ANTHROPIC_AUTH_TOKEN").ok().filter(|s| !s.is_empty()) {
            return Ok(HttpClaude { base_url, auth: ("authorization".into(), format!("Bearer {tok}")), oauth: true });
        }
        bail!("set ANTHROPIC_API_KEY to use plan / revise")
    }

    fn send_once(&self, body: &str) -> std::result::Result<String, (bool, Error)> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(30))
            .timeout_read(Duration::from_secs(600))
            .build();
        let mut betas = vec![FALLBACK_BETA];
        if self.oauth {
            betas.push("oauth-2025-04-20");
        }
        let resp = agent
            .post(&format!("{}/v1/messages", self.base_url.trim_end_matches('/')))
            .set("content-type", "application/json")
            .set("anthropic-version", API_VERSION)
            .set("anthropic-beta", &betas.join(","))
            .set(&self.auth.0, &self.auth.1)
            .send_string(body);
        let resp = match resp {
            Ok(r) => r,
            Err(ureq::Error::Status(code, r)) => {
                let detail = r
                    .into_string()
                    .ok()
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
                    .unwrap_or_default();
                let retry = code == 429 || code >= 500;
                let msg = match code {
                    401 => "Claude API authentication failed. set ANTHROPIC_API_KEY".to_string(),
                    429 => "Claude API rate limited; retry shortly".to_string(),
                    _ => format!("Claude API error {code}: {detail}"),
                };
                return Err((retry, Error::user(msg)));
            }
            Err(e) => return Err((true, Error::user(format!("could not reach the Claude API: {e}")))),
        };
        read_stream(BufReader::new(resp.into_reader())).map_err(|e| (false, e))
    }
}

impl Claude for HttpClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        let body = request.to_string();
        let mut wait = 2;
        for attempt in 0..3 {
            match self.send_once(&body) {
                Ok(text) => return Ok(text),
                Err((true, e)) if attempt < 2 => {
                    eprintln!("shotpilot: {e}; retrying in {wait}s");
                    std::thread::sleep(Duration::from_secs(wait));
                    wait *= 2;
                }
                Err((_, e)) => return Err(e),
            }
        }
        unreachable!("the last attempt returns")
    }
}

/// Collect the answer text from a Messages API SSE stream.
///
/// With server-side fallback, a declined attempt's partial output is followed by a
/// `fallback` content block and the fallback model's answer on the same stream; only
/// text after the last `fallback` block is the answer.
pub fn read_stream(reader: impl BufRead) -> Result<String> {
    let mut text = String::new();
    let mut stop_reason = String::new();
    for line in reader.lines() {
        let line = line.map_err(|e| Error::user(format!("Claude API stream broke: {e}")))?;
        let Some(data) = line.strip_prefix("data:") else { continue };
        let Ok(ev) = serde_json::from_str::<Value>(data.trim()) else { continue };
        match ev["type"].as_str().unwrap_or("") {
            "content_block_start" => {
                if ev["content_block"]["type"] == "fallback" {
                    text.clear();
                } else if let Some(t) = ev["content_block"]["text"].as_str() {
                    text.push_str(t);
                }
            }
            "content_block_delta" => {
                if ev["delta"]["type"] == "text_delta" {
                    text.push_str(ev["delta"]["text"].as_str().unwrap_or(""));
                }
            }
            "message_delta" => {
                if let Some(s) = ev["delta"]["stop_reason"].as_str() {
                    stop_reason = s.to_string();
                }
            }
            "error" => bail!("Claude API error: {}", ev["error"]["message"].as_str().unwrap_or("unknown")),
            _ => {}
        }
    }
    match stop_reason.as_str() {
        "refusal" => bail!("Claude declined this request"),
        "max_tokens" => bail!("Claude's answer was cut off (max_tokens)"),
        _ => {}
    }
    if text.trim().is_empty() {
        bail!("Claude returned no text");
    }
    Ok(text)
}

// --------------------------------------------------------------------------- plan

fn anchors_list(defn: &Definition) -> String {
    defn.anchors
        .iter()
        .map(|(k, v)| format!("- {k}: [{}]", v.iter().map(|x| yamlio::fmt_float(*x)).collect::<Vec<_>>().join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn plan(defn: &Definition, pitch: Option<&str>, duration: Option<f64>, n_shots: Option<usize>, claude: &dyn Claude)
    -> Result<(Vec<Value>, String)> {
    if defn.anchors.is_empty() {
        bail!("no anchors. run `shotpilot scan` (or add `anchors:`) so shots can be placed");
    }
    let pitch = pitch.filter(|p| !p.is_empty()).unwrap_or(&defn.teaser.pitch);
    if pitch.is_empty() {
        bail!("give a pitch: --pitch '...' or teaser.pitch in shots.yaml");
    }
    let duration = duration.unwrap_or(defn.teaser.target_duration);
    let shape: Vec<String> = BEATS.iter().map(|b| format!("- {b}: {}", beat_guide(b))).collect();
    let system = format!(
        "You are a trailer director planning a game teaser shot list for an indie developer. \
         You plan what to film inside the engine before anything is recorded. Each shot exists \
         to make the viewer feel one specific thing. Prefer fewer, deliberate shots over many busy ones.\n\n\
         Every teaser follows this shape, in order, and each shot carries its beat:\n{}\n\n{CONVENTIONS}",
        shape.join("\n")
    );
    let mut user = format!(
        "Game pitch: {pitch}\nTarget teaser length: {} seconds (shot durations must sum to within 10% of this).\n",
        model::fmt_g(duration)
    );
    if let Some(n) = n_shots {
        user.push_str(&format!("Number of shots: {n}\n"));
    }
    user.push_str(&format!(
        "Available anchors (only use these names):\n{}\n\n\
         Write the shot list. Write `purpose` and `notes` in the same language as the pitch.",
        anchors_list(defn)
    ));
    let out = ask_json(claude, &system, json!(user), plan_schema())?;
    let shots: Vec<Value> = out["shots"]
        .as_array()
        .ok_or_else(|| Error::user("Claude returned no shots"))?
        .iter()
        .enumerate()
        .map(|(i, s)| shot_from_plan(s, i))
        .collect();
    validate_shots(defn, &shots)?;
    Ok((shots, out["rationale"].as_str().unwrap_or("").to_string()))
}

fn r1(v: f64) -> f64 {
    crate::geometry::round_to(v, 1)
}

fn shot_from_plan(s: &Value, i: usize) -> Value {
    let anchor = s["anchor"].as_str().unwrap_or("").to_string();
    let mut off: Vec<f64> = s["offset"].as_array().map(|a| a.iter().filter_map(num).collect()).unwrap_or_default();
    off.resize(3, 0.0);
    let mut cam = serde_json::Map::new();
    cam.insert("anchor".into(), json!(anchor));
    cam.insert("offset".into(), json!(off.iter().take(3).map(|v| r1(*v)).collect::<Vec<_>>()));
    if let Some(la) = s["look_at"].as_str().filter(|l| !l.is_empty() && *l != anchor) {
        cam.insert("look_at".into(), json!(la));
    }
    let mv = s["move"].as_str().unwrap_or("static");
    if mv != "static" {
        let key = if mv.starts_with("orbit") { "degrees" } else { "distance" };
        cam.insert("move".into(), json!({"type": mv, key: r1(num(&s["move_amount"]).unwrap_or(0.0))}));
    }
    cam.insert("lens".into(), json!({"focal_length": r1(num(&s["focal_length"]).unwrap_or(35.0))}));
    let mut shot = serde_json::Map::new();
    shot.insert("id".into(), json!(clean_id(s["id"].as_str().unwrap_or(""), i)));
    shot.insert("beat".into(), s["beat"].clone());
    shot.insert("purpose".into(), s["purpose"].clone());
    shot.insert("duration".into(), json!(crate::geometry::round_to(num(&s["duration"]).unwrap_or(3.0), 2)));
    shot.insert("camera".into(), Value::Object(cam));
    if let Some(t) = s["time_of_day"].as_str().filter(|t| *t != "default") {
        shot.insert("environment".into(), json!({"time_of_day": t}));
    }
    if let Some(n) = s["notes"].as_str().filter(|n| !n.is_empty()) {
        shot.insert("notes".into(), json!(n));
    }
    Value::Object(shot)
}

pub fn clean_id(raw: &str, i: usize) -> String {
    let lowered: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let sid: String = lowered.trim_matches(['_', '-']).chars().take(48).collect();
    if model::valid_id(&sid) {
        sid
    } else {
        format!("s{:02}", i + 1)
    }
}

fn validate_shots(defn: &Definition, shots: &[Value]) -> Result<()> {
    let mut trial = defn.clone();
    trial.shots = shots.to_vec();
    for (i, s) in shots.iter().enumerate() {
        model::resolve_shot(&trial, s, i)?;
    }
    let ids: Vec<String> = shots.iter().map(|s| model::str_of(&s["id"])).collect();
    let mut uniq = ids.clone();
    uniq.sort();
    uniq.dedup();
    if uniq.len() != ids.len() {
        bail!("duplicate shot ids from Claude: {ids:?}");
    }
    Ok(())
}

// --------------------------------------------------------------------------- revise

/// What the director is pointing at when giving a note from the app: a line of context
/// (shot, frame, marked region) and optionally the rendered frame itself.
#[derive(Debug, Clone, Default)]
pub struct Pointer {
    pub context: Option<String>,
    /// (media type, file bytes) of the frame on screen.
    pub image: Option<(String, Vec<u8>)>,
}

/// Images above this are sent as text only (the API caps one image at 5 MB after base64).
pub const MAX_IMAGE_BYTES: usize = 3_700_000;

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub struct Revision {
    /// Replacement shots by id, in the order Claude returned them.
    pub edits: Vec<(String, Value)>,
    pub order: Option<Vec<String>>,
    pub summary: String,
}

/// Nothing is written here; the caller applies the revision through the workspace.
pub fn revise(defn: &Definition, instruction: &str, scope: Option<&[String]>, claude: &dyn Claude) -> Result<Revision> {
    revise_at(defn, instruction, scope, &Pointer::default(), claude)
}

/// [`revise`] with what the director is looking at in the app.
pub fn revise_at(defn: &Definition, instruction: &str, scope: Option<&[String]>, pointer: &Pointer,
                 claude: &dyn Claude) -> Result<Revision> {
    let ids = defn.shot_ids();
    if let Some(scope) = scope {
        let unknown: Vec<&String> = scope.iter().filter(|s| !ids.contains(s)).collect();
        if !unknown.is_empty() {
            bail!("unknown shot(s): {}", unknown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
        }
    }
    let resolved = model::resolve_all(defn)?;
    let mut lines = Vec::new();
    for (i, (s, r)) in defn.shots.iter().zip(&resolved).enumerate() {
        lines.push(format!(
            "# shot {}: resolves to start {} end {} lens {}mm",
            i + 1, r.start.to_json(), r.end.to_json(), model::fmt_g(r.focal_length())
        ));
        lines.push(yamlio::dumps(&json!([s])).trim_end().to_string());
    }
    let anchors = if defn.anchors.is_empty() { "(none)".to_string() } else { anchors_list(defn) };
    let system = format!(
        "You revise a game teaser shot list defined in YAML. You change as little as possible: \
         only the shots the note is about, only the fields that need to change. You never rename ids. \
         Shots are numbered from 1 in the order listed (\"the third shot\" = shot 3). Each shot has an \
         editorial `beat` ({}); keep it unless the note changes the shot's role.\n\n{CONVENTIONS}\n\
         A shot's camera can be either `anchor`+`offset` (+ optional `move`, `look_at`, `lens`), or \
         explicit `location` + `rotation` [pitch, yaw, roll] / `look_at`, with optional `end` \
         {{location, rotation|look_at, curve}}. `environment.time_of_day` is one of {}.

         When a rendered frame is attached, it is what the camera currently sees at the moment the          director points to. Judge the composition from it (what blocks the view, what is cut off,          where the subject sits) and move the camera so the note is satisfied. Positions in the frame          are given as percentages from the top-left corner. Write `summary` in the language of the note.",
        BEATS.join(" -> "),
        tod_names_sorted().join(", ")
    );
    let scope_line = match scope {
        Some(s) => format!("You may only edit these shots: {}. Leave `order` empty.\n", s.join(", ")),
        None => "Only reorder if the note asks for it.\n".to_string(),
    };
    let user = format!(
        "Anchors:\n{anchors}\n\nCurrent shots:\n{}\n\nNote from the director: {instruction}\n{scope_line}\
         Return each changed shot as a complete YAML mapping (without the leading '- ').",
        lines.join("\n")
    );
    let context = pointer.context.as_deref().map(|c| format!("

Where the director is pointing: {c}")).unwrap_or_default();
    let mut content = Vec::new();
    if let Some((media_type, bytes)) = &pointer.image {
        content.push(json!({"type": "image", "source": {"type": "base64", "media_type": media_type, "data": base64(bytes)}}));
    }
    content.push(json!({"type": "text", "text": format!("{user}{context}")}));
    let out = ask_json(claude, &system, Value::Array(content), revise_schema())?;

    let mut edits: Vec<(String, Value)> = Vec::new();
    for e in out["edits"].as_array().into_iter().flatten() {
        let sid = e["id"].as_str().unwrap_or("").to_string();
        if !ids.contains(&sid) {
            bail!("Claude edited an unknown shot '{sid}'");
        }
        if let Some(scope) = scope {
            if !scope.contains(&sid) {
                bail!("Claude tried to edit '{sid}' outside the allowed scope {scope:?}; nothing applied");
            }
        }
        let mut new = yamlio::parse_str(e["yaml"].as_str().unwrap_or(""), &format!("Claude's edit for '{sid}'"))?;
        if let Value::Array(a) = &new {
            if a.len() == 1 {
                new = a[0].clone();
            }
        }
        let Some(m) = new.as_object_mut() else {
            bail!("edit for '{sid}' is not a mapping");
        };
        m.insert("id".into(), json!(sid));
        edits.retain(|(k, _)| k != &sid);
        edits.push((sid, new));
    }
    let mut order: Option<Vec<String>> = out["order"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect::<Vec<_>>())
        .filter(|o| !o.is_empty());
    if let Some(o) = &order {
        if scope.is_some() {
            bail!("Claude tried to reorder while scoped to specific shots; nothing applied");
        }
        let (mut a, mut b) = (o.clone(), ids.clone());
        a.sort();
        b.sort();
        if a != b {
            bail!("Claude's new order {o:?} is not a permutation of {ids:?}");
        }
        if *o == ids {
            order = None;
        }
    }
    let trial: Vec<Value> = defn
        .shots
        .iter()
        .map(|s| {
            let id = model::str_of(&s["id"]);
            edits.iter().find(|(k, _)| *k == id).map(|(_, v)| v.clone()).unwrap_or_else(|| s.clone())
        })
        .collect();
    validate_shots(defn, &trial)?;
    Ok(Revision { edits, order, summary: out["summary"].as_str().unwrap_or("").to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648() {
        for (raw, enc) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("foobar", "Zm9vYmFy")] {
            assert_eq!(base64(raw.as_bytes()), enc);
        }
    }

    #[test]
    fn stream_keeps_only_text_after_fallback() {
        let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{}}\n\n\
event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"{\\\"partial\"}}\n\n\
event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"fallback\"}}\n\n\
event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"text_delta\",\"text\":\"{\\\"ok\\\": 1}\"}}\n\n\
event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n";
        assert_eq!(read_stream(sse.as_bytes()).unwrap(), "{\"ok\": 1}");
    }

    #[test]
    fn stream_refusal_is_an_error() {
        let sse = "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"refusal\"}}\n";
        assert!(read_stream(sse.as_bytes()).unwrap_err().msg.contains("declined"));
    }

    #[test]
    fn ids_are_cleaned() {
        assert_eq!(clean_id("S01 Hook!", 0), "s01_hook");
        assert_eq!(clean_id("!!!", 2), "s03");
    }
}
