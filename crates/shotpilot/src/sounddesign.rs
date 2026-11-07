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
            "ambience": {"type": "array", "items": {
                "type": "object",
                "properties": {
                    "place": {"type": "string", "description": "one of the places listed with the cut"},
                    "lib": {"type": "string", "description": "library sound id (a loop or a long bed)"},
                    "gain": {"type": "number"},
                    "label": {"type": "string"},
                },
                "required": ["place", "lib", "gain", "label"],
                "additionalProperties": false,
            }},
            "missing": {"type": "array", "items": {"type": "string"},
                        "description": "sounds the look calls for that the list lacks, as short search words for a sample store (empty when the list has them)"},
        },
        "required": ["rationale", "music", "hits", "ambience", "missing"],
        "additionalProperties": false,
    })
}

const CRAFT: &str = "\
The look's sound character (above) and the genre come first: they say which instruments, which ambience and \
which hits this game has. The habits below are a cinematic trailer's defaults and bend to them (a cozy look has \
no braam, a strategy game's title takes a horn, not a deep boom).
Each game should sound like itself: prefer sounds no other teaser uses. A sound marked `used in N other teasers` \
only when nothing else in the list fits the moment. When the look calls for sounds the list lacks, use the nearest \
ones you have and name the missing ones in `missing`.
How a cinematic teaser's sound is usually built (half of what the cut makes you feel):
- The air of each place goes in `ambience`, one sound per place: the code lays it under the shots that film that \
place and switches where the place changes, so keep room tone, hum and wind out of `music`. Music changes by section \
(a low drone to open, tension through the escalation, a cut to silence before the title), not one track wall to wall.
- Each text card lands on an impact (a boom or braam) or on silence; a count of short evidence shots gets the same \
small sound on every cut (a shutter, a click).
- A riser builds into the reveal; a stinger stops it dead before a black gap; the title arrives on a deep boom; the \
last message gets one small real sound (a knock, a phone buzz).
- Levels: music -9 to -14 dB, ambience -6 to -12 dB, hits -1 to -8 dB, small repeated sounds -4 to -8 dB.
- Use only sounds from the library list, by id. Fewer, deliberate sounds beat many.";

const SYSTEM: &str = "You are the sound designer on a game teaser. You place music and sound effects on a finished \
                      edit, by section and on the text cards, so the cut is felt with the sound on and read with it off.";

/// How many library sounds the designer sees (all of a library of the size D-26 collects).
const LIST_MAX: usize = 600;

/// What Claude decided, already mapped to files in the project.
pub struct Design {
    pub rationale: String,
    pub music: Vec<Value>,
    /// The air of each place: place -> {file, gain, label} (D-76).
    pub ambience: Map<String, Value>,
    pub shot_sfx: Vec<(String, Vec<Value>)>,
    pub line_sfx: Vec<(String, Vec<Value>)>,
    pub notes: Vec<String>,
    /// What the look asked for that the library lacks (search words), for the human to fetch.
    pub missing: Vec<String>,
}

/// The places the cut films, each with the shots that film it (where its air will be laid).
fn places_text(ws: &Workspace) -> String {
    let mut order: Vec<(String, Vec<String>)> = Vec::new();
    for s in &ws.defn.shots {
        let Some(p) = crate::sound::place_of(s) else { continue };
        let id = str_of(&s["id"]);
        match order.iter_mut().find(|(q, _)| *q == p) {
            Some((_, ids)) => ids.push(id),
            None => order.push((p, vec![id])),
        }
    }
    if order.is_empty() {
        return "Places: none named (leave `ambience` empty).".into();
    }
    let rows: Vec<String> = order.iter().map(|(p, ids)| format!("- `{p}`: {}", ids.join(", "))).collect();
    format!("Places (one air each in `ambience`):\n{}", rows.join("\n"))
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
    // the same brief, direction and look the cut was written from (D-48), with the look's sound
    let (list, hidden) = lib.prompt_list_for(LIST_MAX, Some(&ws.root));
    let user = format!(
        "{}\n\n{}\n\n{}\n\n{CRAFT}\n\nLibrary sounds (id, kind, length, tags; bgm = a loop long enough for music):\n{}\n\n\
         Write `rationale` in the pitch's language: {}",
        crate::craft::edit_spec(&ws.defn, crate::craft::Reach::Sound),
        cut_text(ws)?,
        places_text(ws),
        list,
        ws.defn.teaser.pitch
    );
    let out = ai::ask_structured(claude, SYSTEM, json!(user), schema())?;
    let shots = ws.resolved()?;
    let (spans, _) = crate::sound::spans(&ws.defn, &shots);
    let shot_ids: Vec<String> = shots.iter().map(|s| s.id.clone()).collect();
    let mut notes = Vec::new();
    if hidden > 0 {
        notes.push(format!("library: {hidden} of {} sounds did not fit the list (the packs took turns)", lib.sounds.len()));
    }
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
    // a piece named by the head of its id (`s02` for `s02_gate`) is that piece, when only one
    // piece of the cut starts that way
    let piece = |id: String| -> String {
        if id == "start" || id == "end" || spans.contains_key(&id) {
            return id;
        }
        let head = format!("{id}_");
        let mut hits = spans.keys().filter(|k| k.starts_with(&head));
        match (hits.next(), hits.next()) {
            (Some(k), None) => k.clone(),
            _ => id,
        }
    };
    let mut music = Vec::new();
    for c in out["music"].as_array().into_iter().flatten() {
        let from = piece(str_of(&c["from"]));
        let until = piece(str_of(&c["until"]));
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
        let on = piece(str_of(&h["on"]));
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
    // the air of each place the cut films (a place no shot films is dropped)
    let places: Vec<String> = ws.defn.shots.iter().filter_map(crate::sound::place_of).collect();
    let mut ambience = Map::new();
    for a in out["ambience"].as_array().into_iter().flatten() {
        let place = str_of(&a["place"]);
        if !places.contains(&place) {
            notes.push(format!("left out the air of `{place}`: no shot films that place"));
            continue;
        }
        let Some(file) = take(&str_of(&a["lib"]), &mut notes) else { continue };
        let mut m = Map::new();
        m.insert("file".into(), json!(file));
        if let Some(g) = num(&a["gain"]).filter(|g| *g != 0.0) {
            m.insert("gain".into(), json!(crate::geometry::round_to(g, 1)));
        }
        if let Some(l) = a["label"].as_str().filter(|l| !l.is_empty()) {
            m.insert("label".into(), json!(l));
        }
        ambience.insert(place, Value::Object(m));
    }
    // what this teaser shares with teasers of other games, for the human to hear
    let worn = lib.worn(&ws.root);
    let mut shared: Vec<String> = files
        .keys()
        .filter_map(|id| worn.get(id).map(|n| (id, *n)))
        .map(|(id, n)| format!("{} ({n})", lib.get(id).map_or(id.as_str(), |s| s.name.as_str())))
        .collect();
    shared.sort();
    if !shared.is_empty() {
        notes.push(format!("ほかのティザーと同じ音: {}", shared.join(" / ")));
    }
    let missing: Vec<String> = out["missing"].as_array().into_iter().flatten().filter_map(Value::as_str)
        .map(str::trim).filter(|m| !m.is_empty()).map(str::to_string).collect();
    if !missing.is_empty() {
        notes.push(format!("ライブラリに無い音（探すなら）: {}", missing.join(" / ")));
    }
    Ok(Design {
        rationale: out["rationale"].as_str().unwrap_or("").to_string(),
        music,
        ambience,
        shot_sfx: shot_sfx.into_iter().collect(),
        line_sfx: line_sfx.into_iter().collect(),
        notes,
        missing,
    })
}

/// Write a design: music cues into `audio:`, effects onto their shots and script lines.
/// Shots and lines the design does not mention lose their effects (it is a fresh design).
pub fn apply(ws: &mut Workspace, d: &Design) -> Result<()> {
    let mut audio = ws.defn.audio.clone();
    audio.insert("music".into(), if d.music.is_empty() { Value::Null } else { Value::Array(d.music.clone()) });
    audio.insert("ambience".into(), if d.ambience.is_empty() { Value::Null } else { Value::Object(d.ambience.clone()) });
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Words that would tie the designer's own rules to one genre. The rules hold for every
    /// game; what a genre or a look sounds like comes from the look and the brief (D-64).
    /// The kanji for fear are escaped so the i18n extractor does not take them for UI text.
    const GENRE_WORDS: &[&str] =
        &["fear", "horror", "scare", "scary", "dread", "terror", "creepy", "spooky", "\u{6016}", "\u{6050}", "ホラー"];

    #[test]
    fn the_designers_own_rules_belong_to_no_genre() {
        // the first rules were written from one horror teaser, and every game got its booms
        let own = format!("{SYSTEM}\n{CRAFT}").to_lowercase();
        let found: Vec<&&str> = GENRE_WORDS.iter().filter(|w| own.contains(&w.to_lowercase())).collect();
        assert!(found.is_empty(), "genre words in the sound designer's rules: {found:?}");
    }
}
