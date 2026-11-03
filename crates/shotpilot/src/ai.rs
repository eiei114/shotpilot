//! Claude-assisted shot planning and partial revision.
//!
//! `plan`   : pitch + anchors -> a first shot list (high-level camera intent only).
//! `revise` : natural-language note -> edits to the named shots only, plus an optional
//!            reorder and sound (a shot's `sfx`, the teaser's `audio`). Everything Claude
//!            returns is validated by resolving it before it touches shots.yaml, and the
//!            edit scope is enforced in code, not by prompt.
//!
//! There is no official Anthropic SDK for Rust, so this speaks the Messages API over
//! HTTPS directly (streaming, structured outputs, server-side refusal fallback).

use std::io::{BufRead, BufReader};
use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::bail;
use crate::craft;
use crate::error::{Error, Result};
use crate::library::Library;
use crate::model::{self, beat_guide, num, tod_names_sorted, Definition, BEATS, MOVES};
use crate::sound;
use crate::yamlio;

pub const DEFAULT_MODEL: &str = "claude-opus-5-5";

/// Effort levels, lowest first. The same names work for the Messages API
/// (`output_config.effort`) and for Claude Code (`claude -p --effort`).
pub const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];
/// The default when nothing is set (also Claude Opus 5.5's own default).
pub const DEFAULT_EFFORT: &str = "medium";

pub fn valid_effort(e: &str) -> bool {
    EFFORTS.contains(&e)
}
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
orbit_left/orbit_right (degrees around the subject).
Props (listed under `props`) can pop into a shot with no animation: a shot's `appear` list
holds {prop, at: seconds from the shot start, anchor, offset [x, y, z] cm, optional yaw}.
The prop stands on anchor+offset and faces the camera unless yaw is given. Use it for a
sudden reveal (an enemy that is simply there on the cut or a beat later); one or two per
teaser land harder than many.";

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
            "appear": {"type": "array", "description": "props that pop in during this shot (empty for none)", "items": {
                "type": "object",
                "properties": {
                    "prop": {"type": "string"},
                    "at": {"type": "number", "description": "seconds from the shot start"},
                    "anchor": {"type": "string"},
                    "offset": {"type": "array", "items": {"type": "number"}},
                },
                "required": ["prop", "at", "anchor", "offset"],
                "additionalProperties": false,
            }},
        },
        "required": ["id", "beat", "purpose", "duration", "anchor", "offset", "look_at", "focal_length", "move",
                     "move_amount", "time_of_day", "notes", "appear"],
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
            "audio": {"type": "string",
                      "description": "the complete replacement YAML mapping for the top-level `audio` (bgm, gain), or empty to keep it"},
        },
        "required": ["summary", "edits", "order", "audio"],
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
        "output_config": {"effort": DEFAULT_EFFORT, "format": {"type": "json_schema", "schema": schema}},
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

fn props_list(defn: &Definition) -> String {
    if defn.props.is_empty() {
        return "(none: leave `appear` empty)".into();
    }
    defn.props
        .iter()
        .map(|(k, p)| format!("- {k}: {} ({}), {} x {} x {} cm", p.asset, if p.kind == "skeletal" { "a character, no animation" } else { "an object" },
                              model::fmt_g(p.size[0]), model::fmt_g(p.size[1]), model::fmt_g(p.size[2])))
        .collect::<Vec<_>>()
        .join("\n")
}

fn anchors_list(defn: &Definition) -> String {
    let sensor = defn.defaults.get("lens").and_then(|l| l.get("sensor_width")).and_then(num).unwrap_or(36.0);
    let (w, h) = defn.teaser.resolution;
    let aspect = if h > 0 { w as f64 / h as f64 } else { 16.0 / 9.0 };
    let xyz = |v: &[f64; 3]| v.iter().map(|x| yamlio::fmt_float(*x)).collect::<Vec<_>>().join(", ");
    defn.anchors
        .iter()
        .map(|(k, v)| match defn.known_box(k) {
            Some(b) => format!(
                "- {k}: [{}], {} x {} x {} cm (W x D x H, box centre [{}]); fills the frame height from about {} cm at 35mm",
                xyz(v), model::fmt_g(b.size[0]), model::fmt_g(b.size[1]), model::fmt_g(b.size[2]), xyz(&b.center),
                model::fmt_g((craft::distance_for(b.size[2], 35.0, sensor, aspect) / 10.0).round() * 10.0)
            ),
            None => format!("- {k}: [{}] (size unknown)", xyz(v)),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A planned shot list, before it is written to shots.yaml.
#[derive(Debug, Clone)]
pub struct Planned {
    pub shots: Vec<Value>,
    pub rationale: String,
    /// What code changed in Claude's list (beat order, duration range, runtime).
    pub adjustments: Vec<String>,
    /// The skeleton Claude was given.
    pub skeleton: Vec<craft::Slot>,
}

pub fn stage_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string", "description": "the whole set in 3-5 sentences: layout, scale, light, materials, mood"},
            "anchors": {"type": "array", "items": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "description": {"type": "string", "description": "what is there, what it is made of, the light on it, where a camera has room and what blocks the view"},
                },
                "required": ["name", "description"],
                "additionalProperties": false,
            }},
            "mood": {"type": "object", "description": "for each mood-board frame, the anchor whose scout photo suits it best",
                     "properties": {"place": {"type": "string"}, "detail": {"type": "string"}, "threat": {"type": "string"}, "title": {"type": "string"}},
                     "required": ["place", "detail", "threat", "title"], "additionalProperties": false},
        },
        "required": ["summary", "anchors", "mood"],
        "additionalProperties": false,
    })
}

/// Read the level from the scout photos: a description of the set and of each anchor, and
/// which anchor each mood-board frame should start from. One pass, written to `stage:`.
pub fn understand_stage(defn: &Definition, claude: &dyn Claude) -> Result<model::StageCfg> {
    let photos = crate::scout::planner_blocks(defn, 12);
    if photos.is_empty() {
        bail!("no scout photos yet: take them first (shotpilot scout, or the ロケハン stage in the app)");
    }
    let system = "You are the location scout on a game trailer shoot. From photos taken inside the game engine and the \
                  list of named anchors, describe the set the way a cinematographer needs it before planning shots. \
                  Describe only what the photos and data show; where something is unclear, say so plainly.";
    let mut text = format!(
        "Game pitch: {}\nAnchors (name: position, size):\n{}\n\nProps that can be made to appear:\n{}\n\n\
         The photos are labelled with their anchor; each looks at that anchor from the side facing the middle of the set.\n\
         Write `summary` and each anchor's `description` in the same language as the pitch. For `mood`, pick from the \
         anchor names above: `place` = the widest establishing view, `detail` = a small object with texture, \
         `threat` = where something unsettling could stand or watch, `title` = a calm wide view with sky or open space.",
        if defn.teaser.pitch.is_empty() { defn.brief.core.as_str() } else { defn.teaser.pitch.as_str() },
        anchors_list(defn),
        props_list(defn)
    );
    if !defn.brief.genre.is_empty() {
        text.push_str(&format!("\nGenre: {}", defn.brief.genre));
    }
    let mut content = photos;
    content.push(json!({"type": "text", "text": text}));
    let out = ask_json(claude, system, Value::Array(content), stage_schema())?;
    let names: Vec<String> = defn.anchors.iter().map(|(n, _)| n.clone()).collect();
    let anchors: Vec<(String, String)> = out["anchors"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let n = a["name"].as_str()?.to_string();
            names.contains(&n).then(|| (n, a["description"].as_str().unwrap_or("").to_string()))
        })
        .collect();
    let mood: Vec<(String, String)> = ["place", "detail", "threat", "title"]
        .iter()
        .filter_map(|k| out["mood"][k].as_str().filter(|a| names.iter().any(|n| n == a)).map(|a| (k.to_string(), a.to_string())))
        .collect();
    Ok(model::StageCfg { summary: out["summary"].as_str().unwrap_or("").to_string(), anchors, mood })
}

/// The planner's system prompt: role, teaser shape and engine conventions. The brief,
/// skeleton and craft rules go in the user turn (they change per project).
pub fn plan_system() -> String {
    let shape: Vec<String> = BEATS.iter().map(|b| format!("- {b}: {}", beat_guide(b))).collect();
    format!(
        "You are a trailer director planning a game teaser shot list for an indie developer. \
         You plan what to film inside the engine before anything is recorded. Each shot exists \
         to make the viewer feel one specific thing. Prefer fewer, deliberate shots over many busy ones.\n\n\
         Write the whole list in one pass. The brief, the skeleton and the craft rules you are given \
         come from how professional game trailers are cut; follow them as you write each shot.\n\n\
         Every teaser follows this shape, in order, and each shot carries its beat:\n{}\n\n{CONVENTIONS}",
        shape.join("\n")
    )
}

/// Plan a shot list from the pitch, the brief and the anchors. `direction` is an extra
/// note from the director (e.g. typed in the app's structure stage).
pub fn plan(defn: &Definition, pitch: Option<&str>, duration: Option<f64>, n_shots: Option<usize>,
            direction: Option<&str>, claude: &dyn Claude) -> Result<Planned> {
    if defn.anchors.is_empty() {
        bail!("no anchors. run `shotpilot scan` (or add `anchors:`) so shots can be placed");
    }
    let pitch = pitch.filter(|p| !p.is_empty()).unwrap_or(&defn.teaser.pitch);
    let pitch = if pitch.is_empty() { defn.brief.core.as_str() } else { pitch };
    if pitch.is_empty() {
        bail!("give a pitch: --pitch '...', teaser.pitch in shots.yaml, or brief.core");
    }
    let duration = duration.unwrap_or(defn.teaser.target_duration);
    let slots = craft::skeleton(&defn.brief, duration, n_shots);
    let mut user = format!(
        "Game pitch: {pitch}\nTarget teaser length: {} seconds.\n\
         Available anchors (only use these names):\n{}\n\nProps that can appear (only these names):\n{}\n\n{}\n",
        model::fmt_g(duration),
        anchors_list(defn),
        props_list(defn),
        craft::plan_spec(defn, &slots)
    );
    if let Some(d) = direction.map(str::trim).filter(|d| !d.is_empty()) {
        user.push_str(&format!("\n## Direction from the director\n{d}\n"));
    }
    if !defn.stage.is_empty() {
        let mut st = format!("\n## Stage (from the location scout)\n{}\n", defn.stage.summary);
        for (n, d) in &defn.stage.anchors {
            st.push_str(&format!("- {n}: {d}\n"));
        }
        user.push_str(&st);
    }
    let scout = crate::scout::planner_blocks(defn, 8);
    if !scout.is_empty() {
        user.push_str(&format!(
            "\n## Location scout\nThe attached photos labelled \"Location scout\" show {} anchor(s) as they really look in \
             the level, one view each. Place cameras where the photos show open space and a clear view; they are the \
             ground truth for what each anchor is.\n",
            scout.len() / 2
        ));
    }
    let mood = crate::moodboard::image_blocks(defn, 4);
    let mut content = scout;
    let n_mood = mood.len();
    content.extend(mood);
    if n_mood > 0 {
        user.push_str(&format!(
            "\n## Mood board\nThe {} unlabelled image(s) after the scout photos are the director's mood board: match \
             their palette, light and atmosphere (time of day, haze, contrast). They are mood, not shot designs: every \
             shot still has to be filmed from the anchors above.\n",
            n_mood
        ));
    }
    user.push_str(
        "\nWrite the shot list. Write `purpose` and `notes` in the same language as the pitch. \
         `rationale`: two or three sentences on the idea of the cut, in that language too.",
    );
    let user = if content.is_empty() {
        json!(user)
    } else {
        content.push(json!({"type": "text", "text": user}));
        Value::Array(content)
    };
    let out = ask_json(claude, &plan_system(), user, plan_schema())?;
    let mut shots: Vec<Value> = out["shots"]
        .as_array()
        .ok_or_else(|| Error::user("Claude returned no shots"))?
        .iter()
        .enumerate()
        .map(|(i, s)| shot_from_plan(s, i))
        .collect();
    if shots.is_empty() {
        bail!("Claude returned no shots");
    }
    let adjustments = craft::normalize(&mut shots, duration);
    validate_shots(defn, &shots)?;
    Ok(Planned { shots, rationale: out["rationale"].as_str().unwrap_or("").to_string(), adjustments, skeleton: slots })
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
    let appear: Vec<Value> = s["appear"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| {
            let mut off: Vec<f64> = a["offset"].as_array().map(|o| o.iter().filter_map(num).collect()).unwrap_or_default();
            off.resize(3, 0.0);
            json!({"prop": a["prop"], "at": crate::geometry::round_to(num(&a["at"]).unwrap_or(0.0), 2),
                   "anchor": a["anchor"], "offset": off.iter().take(3).map(|v| r1(*v)).collect::<Vec<_>>()})
        })
        .collect();
    if !appear.is_empty() {
        shot.insert("appear".into(), Value::Array(appear));
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
    /// The director is working on the sound: only `sfx` of the shots in scope and the
    /// teaser's `audio` may change. Anything else Claude returns is dropped.
    pub sound: bool,
    /// The global sound library. With a sound note, Claude may pick from it by writing
    /// `file: lib:<id>`; the file is copied into the project before the edit is checked.
    pub library: Option<Library>,
}

/// How many library sounds a sound note lists for Claude.
const LIBRARY_PROMPT_MAX: usize = 300;

/// Replace `lib:<id>` files in the shots' `sfx` and the BGM with copies in `audio/lib/`.
fn take_from_library(edits: &mut [(String, Value)], audio: Option<&mut Map<String, Value>>, lib: Option<&Library>,
                     root: &std::path::Path) -> Result<()> {
    let fix = |f: &mut Value| -> Result<()> {
        let Some(id) = f.as_str().and_then(|s| s.strip_prefix("lib:")).map(str::to_string) else { return Ok(()) };
        let Some(lib) = lib else { bail!("Claude used library sound '{id}' but no library is open; nothing applied") };
        *f = json!(lib.import(&id, root)?);
        Ok(())
    };
    for (_, shot) in edits.iter_mut() {
        for it in shot.get_mut("sfx").and_then(Value::as_array_mut).into_iter().flatten() {
            if let Some(f) = it.get_mut("file") {
                fix(f)?;
            }
        }
    }
    if let Some(f) = audio.and_then(|a| a.get_mut("bgm")).and_then(|b| b.get_mut("file")) {
        fix(f)?;
    }
    Ok(())
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

#[derive(Debug)]
pub struct Revision {
    /// Replacement shots by id, in the order Claude returned them.
    pub edits: Vec<(String, Value)>,
    pub order: Option<Vec<String>>,
    /// Replacement for the top-level `audio:` mapping, when it changed.
    pub audio: Option<Map<String, Value>>,
    pub summary: String,
}

const SOUND_GUIDE: &str = "\
Sound never re-renders a shot: it is mixed when the cut is assembled.
- The teaser's background music lives in the top-level `audio` mapping:
  {gain: master dB, bgm: {file, gain dB, start: teaser seconds, offset: seconds into the file,
  fade_in, fade_out}}.
- A sound effect belongs to a shot, in that shot's `sfx` list:
  {file, at: seconds from the shot's start (negative up to -3 leads into the cut, e.g. a whoosh),
  gain dB, label}. It moves with its shot when the cut is reordered.
- Hits, whooshes and risers usually land on a cut (at: 0) or on a marker inside the shot.
  Typical gains: BGM -12..-4 dB, effects -6..0 dB.";

/// Every audio file the definition refers to (sfx and BGM).
fn audio_files(audio: &Map<String, Value>, shots: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(f) = audio.get("bgm").and_then(|b| b.get("file")).and_then(Value::as_str) {
        out.push(f.replace('\\', "/"));
    }
    for s in shots {
        for it in s.get("sfx").and_then(Value::as_array).into_iter().flatten() {
            if let Some(f) = it.get("file").and_then(Value::as_str) {
                out.push(f.replace('\\', "/"));
            }
        }
    }
    out
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
    let files = sound::list_files(&defn.root());
    let audio_now = if defn.audio.is_empty() {
        "(none)".to_string()
    } else {
        yamlio::dumps(&Value::Object(defn.audio.clone())).trim_end().to_string()
    };
    let audio_allowed = scope.is_none() || pointer.sound;
    let system = format!(
        "You revise a game teaser shot list defined in YAML. You change as little as possible: \
         only the shots the note is about, only the fields that need to change. You never rename ids. \
         Shots are numbered from 1 in the order listed (\"the third shot\" = shot 3). Each shot has an \
         editorial `beat` ({}); keep it unless the note changes the shot's role.\n\n{CONVENTIONS}\n\
         A shot's camera can be either `anchor`+`offset` (+ optional `move`, `look_at`, `lens`), or \
         explicit `location` + `rotation` [pitch, yaw, roll] / `look_at`, with optional `end` \
         {{location, rotation|look_at, curve}}. `environment.time_of_day` is one of {}. \
         A shot may carry on-screen text burned in at the cut: `caption` {{text, at, duration}} (short, one line), \
         and `transition` into the next shot: cut, dip (through black) or flash (through white). They never re-render.

         When a rendered frame is attached, it is what the camera currently sees at the moment the          director points to. Read the composition from it (what blocks the view, what is cut off,          where the subject sits) and move the camera so the note is satisfied. Positions in the frame          are given as percentages from the top-left corner. Write `summary` in the language of the note.\n\n{SOUND_GUIDE}",
        BEATS.join(" -> "),
        tod_names_sorted().join(", ")
    );
    let mut scope_line = match scope {
        Some(s) => format!("You may only edit these shots: {}. Leave `order` empty.\n", s.join(", ")),
        None => "Only reorder if the note asks for it.\n".to_string(),
    };
    if pointer.sound {
        scope_line.push_str("This note is about the sound: change only `sfx` of shots and the top-level `audio`; \
                             leave cameras, durations and everything else as they are.\n");
    }
    if !audio_allowed {
        scope_line.push_str("Leave `audio` empty (the background music is out of scope for this note).\n");
    }
    let mut files_line = if files.is_empty() {
        "(none yet: sound files go in the audio/ folder next to shots.yaml)".to_string()
    } else {
        files.join(", ")
    };
    if let Some(lib) = pointer.library.as_ref().filter(|l| pointer.sound && !l.sounds.is_empty()) {
        files_line.push_str(&format!(
            "

Library sounds you may also use: write `file: lib:<id>` and it is copied into the project              (bgm = a loop long enough for background music):
{}",
            lib.prompt_list(LIBRARY_PROMPT_MAX)
        ));
    }
    let anchors = format!("{anchors}\n\nProps that can appear (shot `appear` list):\n{}", props_list(defn));
    let user = format!(
        "Anchors:\n{anchors}\n\nCurrent shots:\n{}\n\nCurrent audio:\n{audio_now}\n\n\
         Audio files you may use (only these paths): {files_line}\n\n\
         Note from the director: {instruction}\n{scope_line}\
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
        if pointer.sound {
            // a sound note keeps the shot as it is and takes only Claude's `sfx`
            let mut kept = defn.find(&sid)?.clone();
            if let Some(k) = kept.as_object_mut() {
                match m.get("sfx").filter(|v| !v.is_null()) {
                    Some(v) => k.insert("sfx".into(), v.clone()),
                    None => k.remove("sfx"),
                };
            }
            new = kept;
        }
        edits.retain(|(k, _)| k != &sid);
        edits.push((sid, new));
    }
    let audio_yaml = out["audio"].as_str().unwrap_or("").trim();
    let mut audio: Option<Map<String, Value>> = None;
    if !audio_yaml.is_empty() {
        let parsed = match yamlio::parse_str(audio_yaml, "Claude's audio")? {
            Value::Null => Map::new(),
            Value::Object(m) => m,
            _ => bail!("Claude's audio is not a mapping"),
        };
        if parsed != defn.audio {
            if !audio_allowed {
                bail!("Claude tried to change the background music while scoped to {scope:?}; nothing applied");
            }
            audio = Some(parsed);
        }
    }
    let mut order: Option<Vec<String>> = out["order"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect::<Vec<_>>())
        .filter(|o| !o.is_empty());
    if let Some(o) = &order {
        if scope.is_some() {
            bail!("Claude tried to reorder while scoped to specific shots; nothing applied");
        }
        if pointer.sound && *o != ids {
            bail!("Claude tried to reorder shots in a note about the sound; nothing applied");
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
    take_from_library(&mut edits, audio.as_mut(), pointer.library.as_ref(), &defn.root())?;
    let trial: Vec<Value> = defn
        .shots
        .iter()
        .map(|s| {
            let id = model::str_of(&s["id"]);
            edits.iter().find(|(k, _)| *k == id).map(|(_, v)| v.clone()).unwrap_or_else(|| s.clone())
        })
        .collect();
    validate_shots(defn, &trial)?;
    let new_audio = audio.clone().unwrap_or_else(|| defn.audio.clone());
    let mut top = Map::new();
    top.insert("audio".into(), Value::Object(new_audio.clone()));
    let mut errs = Vec::new();
    sound::validate(&top, &trial, &mut errs);
    if !errs.is_empty() {
        bail!("Claude's sound edit is invalid: {}", errs.join("; "));
    }
    // a file Claude brings in must exist; files already in the definition are the director's business
    let known = audio_files(&defn.audio, &defn.shots);
    let root = defn.root();
    let missing: Vec<String> = audio_files(&new_audio, &trial)
        .into_iter()
        .filter(|f| !known.contains(f) && !root.join(f).is_file())
        .collect();
    if !missing.is_empty() {
        bail!("Claude used audio file(s) that do not exist: {}; nothing applied", missing.join(", "));
    }
    Ok(Revision { edits, order, audio, summary: out["summary"].as_str().unwrap_or("").to_string() })
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
