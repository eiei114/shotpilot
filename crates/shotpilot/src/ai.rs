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

fn script_line_schema() -> Value {
    let mut kinds: Vec<&str> = crate::script::SLOT_KINDS.to_vec();
    kinds.extend(crate::script::OVERLAY_KINDS);
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "string", "description": "short lowercase id, unique"},
            "kind": {"type": "string", "enum": kinds},
            "ja": {"type": "string", "description": "the line (in the pitch's language); empty only for black"},
            "en": {"type": "string", "description": "a quiet English line under it, or empty"},
            "head": {"type": "string", "description": "rule: its small heading; message: the chat room name; else empty"},
            "from": {"type": "string", "description": "message: who sent it; else empty"},
            "dur": {"type": "number", "description": "seconds"},
            "after": {"type": "string", "description": "slots: the shot id or slot id it follows, or start; overlays: empty"},
            "on": {"type": "string", "description": "overlays: the shot id it sits on; slots: empty"},
            "at": {"type": "number", "description": "overlays: seconds into that shot; slots: 0"},
        },
        "required": ["id", "kind", "ja", "en", "head", "from", "dur", "after", "on", "at"],
        "additionalProperties": false,
    })
}

pub fn plan_schema() -> Value {
    let mut shot = shot_schema();
    shot["properties"]["transition"] = json!({"type": "string", "enum": crate::finish::TRANSITIONS,
                                              "description": "how this shot hands over to the next one"});
    if let Some(req) = shot["required"].as_array_mut() {
        req.push(json!("transition"));
    }
    let grades: Vec<&str> = std::iter::once("none").chain(crate::finish::GRADES.iter().map(|(n, _)| *n)).collect();
    json!({
        "type": "object",
        "properties": {
            "rationale": {"type": "string"},
            "script": {"type": "array", "items": script_line_schema(),
                       "description": "the teaser's text, written first: cards between shots and lines over them"},
            "shots": {"type": "array", "items": shot},
            "grade": {"type": "string", "enum": grades},
            "letterbox": {"type": "number", "description": "picture aspect inside black bars (2.39 = scope), or 0 for none"},
        },
        "required": ["rationale", "script", "shots", "grade", "letterbox"],
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
                      "description": "the complete replacement YAML mapping for the top-level `audio` (music, bgm, gain, loudness), or empty to keep it"},
            "lines": {"type": "array", "description": "script lines whose `sfx` changes (sound notes only), or empty",
                      "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "sfx": {"type": "string", "description": "the complete replacement YAML list for the line's `sfx` (empty = none)"},
                },
                "required": ["id", "sfx"],
                "additionalProperties": false,
            }},
            "texts": {"type": "array", "description": "script lines whose words, kind or timing change (only the lines this note names), or empty",
                      "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "yaml": {"type": "string", "description": "a YAML mapping of the line's fields that change (ja, en, head, from, kind, at, dur); an empty string removes en, head or from"},
                },
                "required": ["id", "yaml"],
                "additionalProperties": false,
            }},
            "finish": {"type": "string",
                       "description": "a YAML mapping of the top-level `finish` keys that change (grade, letterbox, typeface), or empty to keep them"},
        },
        "required": ["summary", "edits", "order", "audio", "lines", "texts", "finish"],
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

/// One structured call (for other modules: sound design).
pub fn ask_structured(claude: &dyn Claude, system: &str, user: Value, schema: Value) -> Result<Value> {
    ask_json(claude, system, user, schema)
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
    /// `finish:` entries Claude proposed (grade, letterbox); transitions are on the shots.
    pub finish: Vec<(String, Value)>,
    /// `script:` lines (cards between shots, lines over them), ids already mapped to the shots.
    pub script: Vec<Value>,
    /// The measured references the planner read, e.g. "horror (158 teasers)" (refs.rs).
    pub references: Vec<String>,
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
    user.push_str(&craft::script_text(defn));
    if let Some(d) = direction.map(str::trim).filter(|d| !d.is_empty()) {
        user.push_str(&format!("\n## Direction from the director\n{d}\n"));
    }
    user.push_str(&stage_section(defn));
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
    // the script refers to Claude's shot ids; ids are cleaned, so map them through
    let raw_ids: Vec<String> = out["shots"].as_array().into_iter().flatten().map(|s| model::str_of(&s["id"])).collect();
    let id_map: std::collections::HashMap<String, String> =
        raw_ids.iter().cloned().zip(shots.iter().map(|s| model::str_of(&s["id"]))).collect();
    let (script, mut script_notes) = script_from_plan(&out["script"], &id_map, &shots);
    // the target is the whole cut: shots get what the cards leave
    let slot_secs: f64 = script.iter().filter(|l| crate::script::SLOT_KINDS.contains(&l["kind"].as_str().unwrap_or(""))).filter_map(|l| num(&l["dur"])).sum();
    let mut adjustments = craft::normalize(&mut shots, (duration - slot_secs).max(duration * 0.5));
    adjustments.append(&mut script_notes);
    validate_shots(defn, &shots)?;
    let finish = vec![
        ("grade".to_string(), json!(out["grade"].as_str().filter(|g| *g != "none"))),
        ("letterbox".to_string(), json!(num(&out["letterbox"]).filter(|r| (1.0..=3.0).contains(r)))),
        // the title and end card now live in the script
        ("title".to_string(), Value::Null),
        ("end_card".to_string(), Value::Null),
    ];
    let mut references = vec![format!("all genres ({} teasers)", crate::refs::craft().n)];
    if let Some(r) = crate::refs::genre_for(&defn.brief.genre) {
        references.push(format!("{} ({} teasers)", r.id, r.n));
    }
    if let Some(s) = crate::refs::style(&defn.brief.style) {
        references.push(format!("style: {}", s.name));
    }
    Ok(Planned { shots, rationale: out["rationale"].as_str().unwrap_or("").to_string(), adjustments, skeleton: slots, finish, references, script })
}

/// The script lines Claude wrote, as shots.yaml values: ids mapped to the cleaned shot ids,
/// empty fields dropped, lines that point nowhere left out (and reported).
fn script_from_plan(v: &Value, id_map: &std::collections::HashMap<String, String>, shots: &[Value]) -> (Vec<Value>, Vec<String>) {
    let shot_ids: Vec<String> = shots.iter().map(|s| model::str_of(&s["id"])).collect();
    let mut out: Vec<Value> = Vec::new();
    let mut notes = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (i, l) in v.as_array().into_iter().flatten().enumerate() {
        let kind = l["kind"].as_str().unwrap_or("caption").to_string();
        let slot = crate::script::SLOT_KINDS.contains(&kind.as_str());
        let mut id = clean_id(l["id"].as_str().unwrap_or(""), i).replace('-', "_");
        if id.is_empty() || seen.contains(&id) || shot_ids.contains(&id) {
            id = format!("l{:02}", i + 1);
        }
        let map = |s: &str| id_map.get(s).cloned().unwrap_or_else(|| s.to_string());
        let mut m = serde_json::Map::new();
        m.insert("id".into(), json!(id));
        m.insert("kind".into(), json!(kind));
        for k in ["ja", "en", "head", "from"] {
            if let Some(t) = l[k].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                m.insert(k.into(), json!(t));
            }
        }
        m.insert("dur".into(), json!(r1(num(&l["dur"]).unwrap_or(2.5).clamp(0.3, 8.0))));
        if slot {
            let after = map(l["after"].as_str().unwrap_or("start"));
            let after = if after.is_empty() { "start".to_string() } else { after };
            if after != "start" && !shot_ids.contains(&after) && !seen.contains(&after) && !v.as_array().into_iter().flatten().any(|o| o["id"] == after.as_str()) {
                notes.push(format!("left out script line `{id}`: it follows `{after}`, which is not in the cut"));
                continue;
            }
            m.insert("after".into(), json!(after));
        } else {
            let on = map(l["on"].as_str().unwrap_or(""));
            if !shot_ids.contains(&on) {
                notes.push(format!("left out script line `{id}`: it sits on `{on}`, which is not a shot"));
                continue;
            }
            m.insert("on".into(), json!(on));
            m.insert("at".into(), json!(r1(num(&l["at"]).unwrap_or(0.3).max(0.0))));
        }
        seen.push(id);
        out.push(Value::Object(m));
    }
    // one line at a time on a shot: later overlays wait for the one before (0.1 s gap)
    let dur_of = |id: &str| shots.iter().find(|s| model::str_of(&s["id"]) == id).and_then(|s| num(&s["duration"])).unwrap_or(0.0);
    let mut free_at: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |l: &Value| (model::str_of(&l["on"]), num(&l["at"]).unwrap_or(0.0));
        let (ka, kb) = (key(&out[a]), key(&out[b]));
        ka.0.cmp(&kb.0).then(ka.1.partial_cmp(&kb.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    let mut drop = Vec::new();
    for i in order {
        let Some(on) = out[i].get("on").and_then(Value::as_str).map(str::to_string) else { continue };
        let id = model::str_of(&out[i]["id"]);
        let d = dur_of(&on);
        let asked = num(&out[i]["at"]).unwrap_or(0.0);
        let at = asked.max(free_at.get(&on).copied().unwrap_or(0.0));
        if at > d - 0.6 {
            notes.push(format!("left out script line `{id}`: no room left on `{on}` after the line before it"));
            drop.push(i);
            continue;
        }
        if (at - asked).abs() > 1e-6 {
            notes.push(format!("moved script line `{id}` to {}s on `{on}` so it does not overlap the line before", model::fmt_g(r1(at))));
        }
        let len = num(&out[i]["dur"]).unwrap_or(2.0).min(d - at);
        out[i]["at"] = json!(r1(at));
        out[i]["dur"] = json!(r1(len));
        free_at.insert(on, at + len + 0.1);
    }
    drop.sort_unstable();
    for i in drop.into_iter().rev() {
        out.remove(i);
    }
    // a slot that follows a dropped or renamed line falls back to following the start
    let ids: Vec<String> = out.iter().map(|l| model::str_of(&l["id"])).collect();
    for l in out.iter_mut() {
        if let Some(a) = l.get("after").and_then(Value::as_str).map(str::to_string) {
            if a != "start" && !shot_ids.contains(&a) && !ids.contains(&a) {
                l["after"] = json!("start");
            }
        }
    }
    (out, notes)
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
    if let Some(c) = s["caption"].as_str().map(str::trim).filter(|c| !c.is_empty()) {
        shot.insert("caption".into(), json!(c));
    }
    if let Some(t) = s["transition"].as_str().filter(|t| *t != "cut" && crate::finish::TRANSITIONS.contains(t)) {
        shot.insert("transition".into(), json!(t));
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
    /// A sound note about one script line's effects: only that line's `sfx` may change.
    pub line: Option<String>,
    /// The director pointed at one part of the shot (`camera`, `environment`): only these
    /// top-level keys of the shots in scope may change. Anything else Claude returns is dropped.
    pub fields: Option<Vec<String>>,
    /// Script lines whose words, kind and timing ([`LINE_TEXT_FIELDS`]) may change. None:
    /// no script text may change.
    pub texts: Option<Vec<String>>,
    /// The teaser's `finish:` look ([`FINISH_FIELDS`]) may change (notes on the whole teaser).
    pub finish: bool,
}

/// Parts of a shot a note can be pointed at, with the top-level keys each one covers and
/// how the planner is told about it.
pub const ASPECTS: &[(&str, &[&str], &str)] = &[
    ("camera", &["camera"], "the shot's camera only (where it stands, where it looks, the lens and the move)"),
    ("light", &["environment"], "the shot's light only (`environment`: time of day and the like)"),
    ("finish", &["caption", "transition", "exposure"],
     "the finishing burned in at the cut only (the text over the shot, how it hands over to the next shot, its brightness at the cut)"),
];

/// Top-level keys of a shot it may go without: when a note pointed at them gets a shot back
/// without one, it is taken off ("no caption here").
const OPTIONAL_FIELDS: &[&str] = &["caption", "transition", "exposure"];

/// Fields of a script line a note may change: its words, its kind (a slot stays a slot, an
/// overlay an overlay) and its timing. Where it sits (`after`, `on`) and its sound stay.
pub const LINE_TEXT_FIELDS: &[&str] = &["ja", "en", "head", "from", "kind", "at", "dur"];

/// Top-level `finish:` keys a note on the whole teaser may change. The title and the end card
/// are left to the director (the end card holds facts only they know, D-28).
pub const FINISH_FIELDS: &[&str] = &["grade", "letterbox", "typeface"];

const FINISH_GUIDE: &str = "\
Finishing is burned in when the cut is assembled; it never re-renders a shot.
- Script lines are the teaser's text. Slots take their own time on black between shots: card (a centred \
line fading in, a quiet English line under it), rule (a small heading over a line), title (large, \
flickering in), message (a chat bubble with the room name and who sent it). Overlays sit on a shot: \
caption (lower centre on a dark band, a slow fade in and out) and count (lower left, like a note under \
a photograph, a quick fade). The motion of a line comes from its kind and its timing: `at` (overlays: \
seconds into the shot it sits on) and `dur` (seconds it shows; for a slot, its length in the cut). \
A slot can become another slot kind and an overlay another overlay kind; where a line sits stays.
- On a shot: `caption` {text, at, duration} is one more lower-centre line; `transition` is how it hands \
over to the next shot: cut, dip (through black, 0.25 s each side) or flash (from white); `exposure` \
shifts its brightness at the cut (-0.5..0.5; leave it out for as shot).
- The whole teaser: `grade` (the colour of every shot), `letterbox` (picture aspect inside black bars, \
e.g. 2.39, or null for none), `typeface` (mincho or gothic).
- Images, logos and other motion cannot be added here; when the note asks for one, do what these allow \
and say so in `summary`.";

/// The keys a pointed-at part covers (`camera` -> [camera]).
pub fn aspect_fields(aspect: &str) -> Option<Vec<String>> {
    ASPECTS.iter().find(|(a, _, _)| *a == aspect).map(|(_, keys, _)| keys.iter().map(|k| k.to_string()).collect())
}

/// The stage as the location scout read it (empty when it has not been read).
fn stage_section(defn: &Definition) -> String {
    if defn.stage.is_empty() {
        return String::new();
    }
    let mut st = format!("\n## Stage (from the location scout)\n{}\n", defn.stage.summary);
    for (n, d) in &defn.stage.anchors {
        st.push_str(&format!("- {n}: {d}\n"));
    }
    st
}

/// How many library sounds a sound note lists for Claude.
const LIBRARY_PROMPT_MAX: usize = 300;

/// Replace `lib:<id>` files in the shots' and script lines' `sfx`, the BGM and the music
/// cues with copies in `audio/lib/`.
fn take_from_library(edits: &mut [(String, Value)], lines: &mut [(String, Value)], audio: Option<&mut Map<String, Value>>,
                     lib: Option<&Library>, root: &std::path::Path) -> Result<()> {
    let fix = |f: &mut Value| -> Result<()> {
        let Some(id) = f.as_str().and_then(|s| s.strip_prefix("lib:")).map(str::to_string) else { return Ok(()) };
        let Some(lib) = lib else { bail!("Claude used library sound '{id}' but no library is open; nothing applied") };
        *f = json!(lib.import(&id, root)?);
        Ok(())
    };
    let lists = edits.iter_mut().filter_map(|(_, shot)| shot.get_mut("sfx")).chain(lines.iter_mut().map(|(_, l)| l));
    for list in lists {
        for it in list.as_array_mut().into_iter().flatten() {
            if let Some(f) = it.get_mut("file") {
                fix(f)?;
            }
        }
    }
    if let Some(a) = audio {
        if let Some(f) = a.get_mut("bgm").and_then(|b| b.get_mut("file")) {
            fix(f)?;
        }
        for c in a.get_mut("music").and_then(Value::as_array_mut).into_iter().flatten() {
            if let Some(f) = c.get_mut("file") {
                fix(f)?;
            }
        }
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
    /// New `sfx` of script lines (a list, or null for none), sound notes only.
    pub lines: Vec<(String, Value)>,
    /// Words, kind and timing of script lines that change (field, value; null removes it).
    pub texts: Vec<(String, Vec<(String, Value)>)>,
    /// `finish:` entries that change (grade, letterbox, typeface).
    pub finish: Vec<(String, Value)>,
    pub summary: String,
}

const SOUND_GUIDE: &str = "\
Sound never re-renders a shot: it is mixed when the cut is assembled.
- The top-level `audio` mapping holds the music and the level:
  {loudness: LUFS the whole cut is brought to (-30..-8, default -16; this, not gain, sets how loud
  the teaser is), gain: master dB, music: [cues], bgm: {file, gain, start, offset, fade_in, fade_out}}.
  A music cue is {file, from: the shot or script line id it starts on (or start), until: the id it
  stops after (or end), at: seconds after that start, offset: seconds into the file, gain dB,
  fade_in, fade_out, label}. `from` and `until` must name pieces that are in the cut now.
- A sound effect is {file, at, gain dB, label} in the `sfx` list of a shot (at: seconds from the
  shot's start, negative up to -3 leads into the cut, e.g. a whoosh) or of a script line (at:
  seconds from when the card or text appears). It moves with its shot or line when the cut is reordered.
- Hits, whooshes and risers usually land on a cut (at: 0), a card, or a marker inside the shot.
  Typical gains: music -14..-9 dB, effects -8..-1 dB.";

/// Every audio file the definition refers to (BGM, music cues, and the `sfx` lists of
/// shots and script lines).
fn audio_files<'a>(audio: &'a Map<String, Value>, sfx_lists: impl Iterator<Item = &'a Value>) -> Vec<String> {
    let files = audio
        .get("bgm")
        .into_iter()
        .chain(audio.get("music").and_then(Value::as_array).into_iter().flatten())
        .chain(sfx_lists.filter_map(Value::as_array).flatten());
    files.filter_map(|it| it.get("file")).filter_map(Value::as_str).map(|f| f.replace('\\', "/")).collect()
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
    let audio_allowed = (scope.is_none() || pointer.sound) && pointer.line.is_none();
    if let Some(l) = &pointer.line {
        if !pointer.sound || !defn.script.iter().any(|x| &x.id == l) {
            bail!("unknown script line '{l}'");
        }
    }
    // the script, with where each line sits in the cut, so effects can go on cards
    let script_now = if pointer.sound && !defn.script.is_empty() {
        let (spans, _) = sound::spans(defn, &resolved);
        let rows: Vec<String> = defn
            .script
            .iter()
            .map(|l| {
                let at = spans.get(&l.id).map(|(a, b)| format!("{a:.2}-{b:.2}s")).unwrap_or_else(|| "not in the cut".into());
                let place = l.on.as_ref().map(|o| format!("over {o}")).or(l.after.as_ref().map(|a| format!("after {a}"))).unwrap_or_default();
                let sfx = if l.sfx.is_empty() { String::new() } else { format!("\n    sfx: {}", yamlio::emit_flow(&Value::Array(l.sfx.clone()))) };
                format!("- {} ({}, {place}, {at}): {}{sfx}", l.id, l.kind, l.ja)
            })
            .collect();
        format!("\n\nScript lines (cards between shots and text over shots; a line's `sfx` plays from when it appears):\n{}", rows.join("\n"))
    } else {
        String::new()
    };
    // the script text and the finishing this note may change (the server decides; code holds it)
    let text_ids: &[String] = pointer.texts.as_deref().unwrap_or(&[]);
    if pointer.sound && (!text_ids.is_empty() || pointer.finish) {
        bail!("a note on the sound does not change the script text or the finishing");
    }
    if let Some(l) = text_ids.iter().find(|l| !defn.script.iter().any(|x| &x.id == *l)) {
        bail!("unknown script line '{l}'");
    }
    let finishing = !text_ids.is_empty()
        || pointer.finish
        || pointer.fields.as_ref().is_some_and(|f| f.iter().any(|k| OPTIONAL_FIELDS.contains(&k.as_str())));
    let texts_now = if text_ids.is_empty() {
        String::new()
    } else {
        let rows: Vec<String> = defn.script.iter().filter(|l| text_ids.contains(&l.id)).map(text_row).collect();
        format!("\n\nScript lines this note may change (in `texts`):\n{}", rows.join("\n"))
    };
    let finish_now = if pointer.finish {
        let grades: Vec<&str> = crate::finish::GRADES.iter().map(|(n, _)| *n).collect();
        format!(
            "\n\nCurrent finish: grade {}, letterbox {}, typeface {} (grades: none, {})",
            defn.finish.grade,
            defn.finish.letterbox.map(model::fmt_g).unwrap_or_else(|| "none".into()),
            defn.finish.typeface,
            grades.join(", ")
        )
    } else {
        String::new()
    };
    let mut system = format!(
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
    if finishing {
        system.push_str("\n\n");
        system.push_str(FINISH_GUIDE);
    }
    let mut scope_line = match scope {
        Some([]) => "Change no shot: leave `edits` and `order` empty.\n".to_string(),
        Some(s) => format!("You may only edit these shots: {}. Leave `order` empty.\n", s.join(", ")),
        None => "Only reorder if the note asks for it.\n".to_string(),
    };
    if let Some(l) = &pointer.line {
        scope_line = format!("This note is about the sound effects of script line {l} only: return its new `sfx` in \
                              `lines`; leave `edits`, `order` and `audio` empty.\n");
    } else if pointer.sound && scope.is_some() {
        scope_line.push_str("This note is about the sound: change only `sfx` of those shots and the top-level `audio`; \
                             leave cameras, durations and everything else as they are. Leave `lines` empty.\n");
    } else if pointer.sound {
        scope_line.push_str("This note is about the sound: change only `sfx` of shots (in `edits`), `sfx` of script \
                             lines (in `lines`) and the top-level `audio`; leave cameras, durations and everything \
                             else as they are.\n");
    } else {
        scope_line.push_str("Leave `lines` empty.\n");
    }
    if let Some(fields) = &pointer.fields {
        if pointer.sound || pointer.line.is_some() || scope.map_or(true, |s| s.is_empty()) {
            bail!("a note on one part of shots names the shots it is about and is not a sound note");
        }
        let what = ASPECTS
            .iter()
            .find(|(_, keys, _)| keys.len() == fields.len() && keys.iter().zip(fields).all(|(k, f)| k == f))
            .map(|(_, _, t)| t.to_string())
            .unwrap_or_else(|| format!("`{}` of the shot only", fields.join("`, `")));
        let keep = if fields.iter().any(|f| OPTIONAL_FIELDS.contains(&f.as_str())) {
            "the camera, the light, the duration, beat, purpose, sound"
        } else {
            "the duration, beat, purpose, sound, caption"
        };
        scope_line.push_str(&format!(
            "This note is about {what}: change only `{}`; keep {keep} and every other field exactly as they are.\n",
            fields.join("`, `")
        ));
    }
    if text_ids.is_empty() {
        scope_line.push_str("Leave `texts` empty.\n");
    } else {
        scope_line.push_str(&format!(
            "The words, kind and timing of script line(s) {} may change too (in `texts`, only the fields that change); \
             no other line.\n",
            text_ids.join(", ")
        ));
    }
    if pointer.finish {
        scope_line.push_str("The teaser's `grade`, `letterbox` and `typeface` may change too (in `finish`).\n");
    } else {
        scope_line.push_str("Leave `finish` empty.\n");
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
    // a note on the cameras gets what the planner places cameras with: the stage and the framing table
    let camera_help = if pointer.fields.as_ref().is_some_and(|f| f.iter().any(|k| k == "camera")) {
        format!("{}\n## Framing\n{}\n\n", stage_section(defn), craft::framing_table(defn))
    } else {
        String::new()
    };
    let user = format!(
        "{camera_help}Anchors:\n{anchors}\n\nCurrent shots:\n{}{script_now}{texts_now}{finish_now}\n\nCurrent audio:\n{audio_now}\n\n\
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
        if let Some(fields) = &pointer.fields {
            // a note pointed at one part keeps the rest of the shot as it is
            let mut kept = defn.find(&sid)?.clone();
            if let Some(k) = kept.as_object_mut() {
                for f in fields {
                    let optional = OPTIONAL_FIELDS.contains(&f.as_str());
                    // a plain cut and no brightness shift are the defaults: written as nothing
                    let v = m.get(f).filter(|v| {
                        let default = (f == "transition" && v.as_str() == Some("cut")) || (f == "exposure" && num(v) == Some(0.0));
                        !v.is_null() && !default
                    });
                    match v {
                        Some(v) => {
                            k.insert(f.clone(), v.clone());
                        }
                        None if optional => {
                            k.remove(f);
                        }
                        None => {}
                    }
                }
            }
            new = kept;
            if &new == defn.find(&sid)? {
                continue; // the part came back as it was
            }
        } else if pointer.sound {
            // a sound note keeps the shot as it is and takes only Claude's `sfx`
            let mut kept = defn.find(&sid)?.clone();
            if let Some(k) = kept.as_object_mut() {
                match m.get("sfx").filter(|v| !v.is_null()) {
                    Some(v) => k.insert("sfx".into(), v.clone()),
                    None => k.remove("sfx"),
                };
            }
            new = kept;
            if &new == defn.find(&sid)? {
                continue; // the shot came back as it was
            }
        }
        if let Some(l) = &pointer.line {
            bail!("Claude edited shot '{sid}' in a note about the sound of script line '{l}'; nothing applied");
        }
        edits.retain(|(k, _)| k != &sid);
        edits.push((sid, new));
    }
    let mut line_edits: Vec<(String, Value)> = Vec::new();
    for e in out["lines"].as_array().into_iter().flatten() {
        let lid = e["id"].as_str().unwrap_or("").to_string();
        let Some(cur) = defn.script.iter().find(|l| l.id == lid) else {
            bail!("Claude edited an unknown script line '{lid}'");
        };
        if !pointer.sound || scope.is_some() || pointer.line.as_ref().is_some_and(|l| *l != lid) {
            bail!("Claude tried to change the sound of script line '{lid}' outside this note's scope; nothing applied");
        }
        let yaml = e["sfx"].as_str().unwrap_or("").trim();
        let list = if yaml.is_empty() { Value::Null } else { yamlio::parse_str(yaml, &format!("Claude's sfx for '{lid}'"))? };
        let list = match list {
            Value::Array(a) if !a.is_empty() => Value::Array(a),
            Value::Object(m) => Value::Array(vec![Value::Object(m)]),
            Value::Null | Value::Array(_) => Value::Null,
            _ => bail!("Claude's sfx for script line '{lid}' is not a list"),
        };
        let before = if cur.sfx.is_empty() { Value::Null } else { Value::Array(cur.sfx.clone()) };
        if list != before {
            line_edits.retain(|(k, _)| *k != lid);
            line_edits.push((lid, list));
        }
    }
    let mut text_edits: Vec<(String, Vec<(String, Value)>)> = Vec::new();
    for e in out["texts"].as_array().into_iter().flatten() {
        let lid = e["id"].as_str().unwrap_or("").to_string();
        let Some(cur) = defn.script.iter().find(|l| l.id == lid) else {
            bail!("Claude edited an unknown script line '{lid}'");
        };
        let m = match yamlio::parse_str(e["yaml"].as_str().unwrap_or(""), &format!("Claude's text for '{lid}'"))? {
            Value::Object(m) => m,
            Value::Null => continue,
            _ => bail!("Claude's text for script line '{lid}' is not a mapping"),
        };
        let set = line_text_changes(cur, &m)?;
        if set.is_empty() {
            continue; // the line came back as it was
        }
        if !text_ids.contains(&lid) {
            bail!("Claude tried to change the text of script line '{lid}' outside this note's scope; nothing applied");
        }
        text_edits.retain(|(k, _)| *k != lid);
        text_edits.push((lid, set));
    }
    let finish = finish_changes(defn, out["finish"].as_str().unwrap_or(""))?;
    if !finish.is_empty() && !pointer.finish {
        bail!("Claude tried to change the teaser's finishing (`finish:`) outside this note's scope; nothing applied");
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
    // a music cue Claude points somewhere new must point at a piece of the cut
    if let Some(Value::Array(cues)) = audio.as_ref().and_then(|a| a.get("music")) {
        let refs = |a: &Map<String, Value>| -> Vec<String> {
            a.get("music").and_then(Value::as_array).into_iter().flatten()
                .flat_map(|c| ["from", "until"].map(|k| c.get(k).map(model::str_of).unwrap_or_default()))
                .collect()
        };
        let old_refs = refs(&defn.audio);
        for c in cues {
            for k in ["from", "until"] {
                let r = c.get(k).map(model::str_of).unwrap_or_default();
                let known = r.is_empty() || r == "start" || r == "end" || ids.contains(&r) || defn.script.iter().any(|l| l.id == r);
                if !known && !old_refs.contains(&r) {
                    bail!("Claude's music cue has {k}: '{r}', which is not a shot or script line; nothing applied");
                }
            }
        }
    }
    take_from_library(&mut edits, &mut line_edits, audio.as_mut(), pointer.library.as_ref(), &defn.root())?;
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
    for (lid, list) in &line_edits {
        sound::validate_line_sfx(&format!("script line {lid}"), Some(list), &mut errs);
    }
    if !errs.is_empty() {
        bail!("Claude's sound edit is invalid: {}", errs.join("; "));
    }
    // a file Claude brings in must exist; files already in the definition are the director's business
    let old_lines: Vec<Value> = defn.script.iter().map(|l| Value::Array(l.sfx.clone())).collect();
    let known = audio_files(&defn.audio, defn.shots.iter().filter_map(|s| s.get("sfx")).chain(&old_lines));
    let root = defn.root();
    let missing: Vec<String> = audio_files(&new_audio, trial.iter().filter_map(|s| s.get("sfx")).chain(line_edits.iter().map(|(_, l)| l)))
        .into_iter()
        .filter(|f| !known.contains(f) && !root.join(f).is_file())
        .collect();
    if !missing.is_empty() {
        bail!("Claude used audio file(s) that do not exist: {}; nothing applied", missing.join(", "));
    }
    Ok(Revision {
        edits,
        order,
        audio,
        lines: line_edits,
        texts: text_edits,
        finish,
        summary: out["summary"].as_str().unwrap_or("").to_string(),
    })
}

/// One script line as a note on the text sees it: where it sits, its timing and its words.
fn text_row(l: &crate::script::Line) -> String {
    let g = model::fmt_g;
    let place = match (&l.on, &l.after) {
        (Some(o), _) => format!("over {o} from {}s for {}s", g(l.at), g(l.dur)),
        (_, Some(a)) => format!("after {a}, {}s", g(l.dur)),
        _ => format!("{}s", g(l.dur)),
    };
    let mut row = format!("- {} ({}, {place}): ja: {}", l.id, l.kind, l.ja);
    for (k, v) in [("en", &l.en), ("head", &l.head), ("from", &l.from)] {
        if !v.is_empty() {
            row.push_str(&format!(" | {k}: {v}"));
        }
    }
    row
}

/// A whole number of seconds as an integer, so the YAML reads `dur: 2`, not `2.0`.
fn seconds(x: f64) -> Value {
    if x.fract() == 0.0 && x.abs() < 1e9 { json!(x as i64) } else { json!(x) }
}

/// The fields of a script line that Claude's mapping changes ([`LINE_TEXT_FIELDS`]; null
/// takes a field off). A line keeps its place: a slot stays a slot, an overlay an overlay.
fn line_text_changes(cur: &crate::script::Line, m: &Map<String, Value>) -> Result<Vec<(String, Value)>> {
    use crate::script::{OVERLAY_KINDS, SLOT_KINDS};
    let mut set = Vec::new();
    for f in LINE_TEXT_FIELDS {
        let Some(v) = m.get(*f) else { continue };
        let new = match *f {
            "at" | "dur" => {
                if *f == "at" && cur.is_slot() {
                    continue; // a slot's time is its place in the cut
                }
                let Some(x) = num(v) else { bail!("Claude's `{f}` for script line '{}' is not a number", cur.id) };
                if (x - if *f == "at" { cur.at } else { cur.dur }).abs() < 1e-9 {
                    continue;
                }
                seconds(x)
            }
            _ => {
                let now = match *f {
                    "ja" => cur.ja.as_str(),
                    "en" => cur.en.as_str(),
                    "head" => cur.head.as_str(),
                    "from" => cur.from.as_str(),
                    _ => cur.kind.as_str(),
                };
                let s = model::str_of(v).trim().to_string();
                if s == now.trim() {
                    continue;
                }
                if *f == "kind" {
                    let same_family = (cur.is_slot() && SLOT_KINDS.contains(&s.as_str()))
                        || (!cur.is_slot() && OVERLAY_KINDS.contains(&s.as_str()));
                    if !same_family || s == "black" || cur.kind == "black" {
                        bail!("Claude tried to turn script line '{}' from {} into '{s}' (a line keeps its place: \
                               cards stay cards, text over a shot stays over it); nothing applied", cur.id, cur.kind);
                    }
                }
                if s.is_empty() { Value::Null } else { json!(s) }
            }
        };
        set.push((f.to_string(), new));
    }
    Ok(set)
}

/// The `finish:` entries ([`FINISH_FIELDS`]) Claude's mapping changes, checked as `finish:`
/// would be. Other keys are dropped (the title and end card are the director's).
fn finish_changes(defn: &Definition, yaml: &str) -> Result<Vec<(String, Value)>> {
    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }
    let m = match yamlio::parse_str(yaml, "Claude's finish")? {
        Value::Object(m) => m,
        Value::Null => return Ok(Vec::new()),
        _ => bail!("Claude's finish is not a mapping"),
    };
    let f = &defn.finish;
    let mut now = Map::new();
    now.insert("grade".into(), json!(f.grade));
    now.insert("letterbox".into(), f.letterbox.map_or(Value::Null, |x| json!(x)));
    now.insert("typeface".into(), json!(f.typeface));
    let mut out = Vec::new();
    for k in FINISH_FIELDS {
        let Some(v) = m.get(*k) else { continue };
        let v = match v {
            // no bars: null (0 and false are how people say it)
            Value::Bool(false) => Value::Null,
            v if *k == "letterbox" && num(v) == Some(0.0) => Value::Null,
            Value::String(s) => json!(s.trim()),
            v => v.clone(),
        };
        let same = if *k == "letterbox" { num(&v) == f.letterbox } else { model::str_of(&v) == model::str_of(&now[*k]) };
        if !same {
            out.push((k.to_string(), v));
        }
    }
    if !out.is_empty() {
        let mut trial = now;
        for (k, v) in &out {
            trial.insert(k.clone(), v.clone());
        }
        let mut errs = Vec::new();
        crate::finish::parse(Some(&Value::Object(trial)), &mut errs);
        if !errs.is_empty() {
            bail!("Claude's finish is invalid: {}", errs.join("; "));
        }
    }
    Ok(out)
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
