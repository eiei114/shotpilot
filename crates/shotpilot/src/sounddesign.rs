//! Sound design for a planned cut: Claude picks music cues and hits from the sound library
//! against the cut's pieces (shots, script cards, captions), code imports the files and
//! writes them to shots.yaml (`audio.music`, shot `sfx`, script line `sfx`). One pass, no
//! self-review (D-22); the sound check (soundcheck.rs) measures the mix afterwards.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::ai::{self, Claude};
use crate::bail;
use crate::error::Result;
use crate::library::Library;
use crate::model::{fmt_g, num, str_of};
use crate::workspace::Workspace;
use crate::yamlio::Doc;

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "rationale": {"type": "string"},
            "music": {"type": "array", "items": {
                "type": "object",
                "properties": {
                    "lib": {"type": "string", "description": "library sound id"},
                    "from": {"type": "string", "description": "id of the piece the cue starts on, or start"},
                    "until": {"type": "string", "description": "id of the piece it runs to the end of, or end"},
                    "at": {"type": "number", "description": "seconds after the start of `from`"},
                    "gain": {"type": "number"},
                    "fade_in": {"type": "number"},
                    "fade_out": {"type": "number"},
                    "label": {"type": "string"},
                },
                "required": ["lib", "from", "until", "at", "gain", "fade_in", "fade_out", "label"],
                "additionalProperties": false,
            }},
            "hits": {"type": "array", "items": {
                "type": "object",
                "properties": {
                    "lib": {"type": "string"},
                    "on": {"type": "string", "description": "id of the shot or script line it belongs to"},
                    "at": {"type": "number", "description": "seconds from the start of `on` (negative leads into it)"},
                    "gain": {"type": "number"},
                    "label": {"type": "string"},
                },
                "required": ["lib", "on", "at", "gain", "label"],
                "additionalProperties": false,
            }},
        },
        "required": ["rationale", "music", "hits"],
        "additionalProperties": false,
    })
}

const CRAFT: &str = "\
How a teaser's sound is built (it is half of the fear):
- A quiet ambience bed under most of the cut; music that changes by section (a low drone to open, tension through \
the escalation, a cut to silence before the title), not one track wall to wall.
- Each text card lands on an impact (a boom or braam) or on silence; a count of short evidence shots gets the same \
small sound on every cut (a shutter, a click).
- A riser builds into the reveal; a stinger stops it dead before a black gap; the title arrives on a deep boom; the \
last message gets one small real sound (a knock, a phone buzz).
- Levels: music -9 to -14 dB, ambience -6 to -12 dB, hits -1 to -8 dB, small repeated sounds -4 to -8 dB.
- Use only sounds from the library list, by id. Fewer, deliberate sounds beat many.";

/// What Claude decided, already mapped to files in the project.
pub struct Design {
    pub rationale: String,
    pub music: Vec<Value>,
    pub shot_sfx: Vec<(String, Vec<Value>)>,
    pub line_sfx: Vec<(String, Vec<Value>)>,
    pub notes: Vec<String>,
}

/// The cut in order for the prompt: id, kind, seconds, text or purpose.
fn cut_text(ws: &Workspace) -> Result<String> {
    let shots = ws.resolved()?;
    let (spans, total) = crate::sound::spans(&ws.defn, &shots);
    let mut rows: Vec<(f64, String)> = Vec::new();
    for s in &shots {
        if let Some(&(a, b)) = spans.get(&s.id) {
            rows.push((a, format!("{a:>6.2}-{b:<6.2} shot `{}` ({}): {}", s.id, s.beat.as_deref().unwrap_or("-"), s.purpose)));
        }
    }
    for l in &ws.defn.script {
        if let Some(&(a, b)) = spans.get(&l.id) {
            rows.push((a + 0.001, format!("{a:>6.2}-{b:<6.2} {} `{}`: {}", l.kind, l.id, l.ja)));
        }
    }
    rows.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    Ok(format!("The cut ({}s):\n{}", fmt_g(crate::geometry::round_to(total, 2)),
               rows.into_iter().map(|(_, r)| r).collect::<Vec<_>>().join("\n")))
}

/// Ask Claude for the sound of the cut, and import what it chose.
pub fn design(ws: &Workspace, lib: &Library, claude: &dyn Claude) -> Result<Design> {
    if lib.sounds.is_empty() {
        bail!("the sound library is empty or missing (see D-26)");
    }
    let system = "You are the sound designer on a game teaser. You place music and sound effects on a finished \
                  edit, by section and on the text cards, so the cut is felt with the sound on and read with it off.";
    let user = format!(
        "{}\n\n{CRAFT}\n\nLibrary sounds (id, kind, length, tags; bgm = a loop long enough for music):\n{}\n\n\
         Write `rationale` in the pitch's language: {}",
        cut_text(ws)?,
        lib.prompt_list(160),
        ws.defn.teaser.pitch
    );
    let out = ai::ask_structured(claude, system, json!(user), schema())?;
    let shots = ws.resolved()?;
    let (spans, _) = crate::sound::spans(&ws.defn, &shots);
    let shot_ids: Vec<String> = shots.iter().map(|s| s.id.clone()).collect();
    let mut notes = Vec::new();
    let mut files: HashMap<String, String> = HashMap::new();
    let mut take = |id: &str, notes: &mut Vec<String>| -> Option<String> {
        if let Some(f) = files.get(id) {
            return Some(f.clone());
        }
        match lib.import(id, &ws.root) {
            Ok(f) => {
                files.insert(id.to_string(), f.clone());
                Some(f)
            }
            Err(e) => {
                notes.push(format!("left out library sound `{id}`: {}", e.msg));
                None
            }
        }
    };
    let mut music = Vec::new();
    for c in out["music"].as_array().into_iter().flatten() {
        let from = str_of(&c["from"]);
        let until = str_of(&c["until"]);
        if from != "start" && !spans.contains_key(&from) {
            notes.push(format!("left out a music cue: `{from}` is not in the cut"));
            continue;
        }
        let Some(file) = take(&str_of(&c["lib"]), &mut notes) else { continue };
        let mut m = Map::new();
        m.insert("file".into(), json!(file));
        m.insert("from".into(), json!(from));
        if until != "end" && spans.contains_key(&until) {
            m.insert("until".into(), json!(until));
        }
        for k in ["at", "gain", "fade_in", "fade_out"] {
            if let Some(v) = num(&c[k]).filter(|v| *v != 0.0) {
                m.insert(k.into(), json!(crate::geometry::round_to(v, 2)));
            }
        }
        if let Some(l) = c["label"].as_str().filter(|l| !l.is_empty()) {
            m.insert("label".into(), json!(l));
        }
        music.push(Value::Object(m));
    }
    let mut shot_sfx: HashMap<String, Vec<Value>> = HashMap::new();
    let mut line_sfx: HashMap<String, Vec<Value>> = HashMap::new();
    for h in out["hits"].as_array().into_iter().flatten() {
        let on = str_of(&h["on"]);
        let is_shot = shot_ids.contains(&on);
        let is_line = ws.defn.script.iter().any(|l| l.id == on);
        if !is_shot && !is_line {
            notes.push(format!("left out a hit: `{on}` is not a shot or a script line"));
            continue;
        }
        let Some(file) = take(&str_of(&h["lib"]), &mut notes) else { continue };
        let at = num(&h["at"]).unwrap_or(0.0).max(-crate::sound::LEAD_MAX);
        let at = if is_shot { at.min(spans[&on].1 - spans[&on].0 - 0.05) } else { at };
        let mut m = Map::new();
        m.insert("file".into(), json!(file));
        m.insert("at".into(), json!(crate::geometry::round_to(at, 2)));
        if let Some(g) = num(&h["gain"]).filter(|g| *g != 0.0) {
            m.insert("gain".into(), json!(crate::geometry::round_to(g, 1)));
        }
        if let Some(l) = h["label"].as_str().filter(|l| !l.is_empty()) {
            m.insert("label".into(), json!(l));
        }
        let list = if is_shot { shot_sfx.entry(on).or_default() } else { line_sfx.entry(on).or_default() };
        list.push(Value::Object(m));
    }
    Ok(Design {
        rationale: out["rationale"].as_str().unwrap_or("").to_string(),
        music,
        shot_sfx: shot_sfx.into_iter().collect(),
        line_sfx: line_sfx.into_iter().collect(),
        notes,
    })
}

/// Write a design: music cues into `audio:`, effects onto their shots and script lines.
/// Shots and lines the design does not mention lose their effects (it is a fresh design).
pub fn apply(ws: &mut Workspace, d: &Design) -> Result<()> {
    let mut audio = ws.defn.audio.clone();
    audio.insert("music".into(), if d.music.is_empty() { Value::Null } else { Value::Array(d.music.clone()) });
    if !audio.contains_key("loudness") {
        audio.insert("loudness".into(), json!(crate::sound::DEFAULT_LOUDNESS));
    }
    let sfx: Vec<(String, Option<Value>)> = ws
        .defn
        .shot_ids()
        .into_iter()
        .map(|id| {
            let list = d.shot_sfx.iter().find(|(s, _)| *s == id).map(|(_, l)| Value::Array(l.clone()));
            (id, list)
        })
        .collect();
    ws.edit_sound(&sfx, Some(&audio), "sound design")?;
    // script line effects
    if !ws.defn.script.is_empty() {
        let mut doc = Doc::load(&ws.path)?;
        let mut lines: Vec<Value> = doc.value()?["script"].as_array().cloned().unwrap_or_default();
        for l in lines.iter_mut() {
            let id = str_of(&l["id"]);
            if let Some(m) = l.as_object_mut() {
                match d.line_sfx.iter().find(|(x, _)| *x == id) {
                    Some((_, list)) => {
                        m.insert("sfx".into(), Value::Array(list.clone()));
                    }
                    None => {
                        m.remove("sfx");
                    }
                }
            }
        }
        doc.set_items("script", &lines)?;
        crate::model::parse(&doc.value()?, &ws.path)?;
        doc.save(&ws.path)?;
        ws.reload()?;
    }
    Ok(())
}
