//! Chip suggestions for the app's stack composer (D-62).
//!
//! In the app a note is built by tapping short chips and stacking them, not by typing it; only
//! the first planning prompt is typed. This module asks Claude once, quickly (low effort), for
//! the chips a developer is most likely to tap next for one target: a cut and the part of it
//! pointed at, a card, the whole teaser, the finishing, the sound, the mood board, the plan, or
//! the home's first prompt. It writes nothing. A stacked note goes to `revise` (or the mood
//! board, or the pitch) exactly like a typed one, and the scope is enforced there, in code.
//! Code tidies the answer: short chips only, none twice, none already stacked.

use serde_json::{json, Map, Value};

use crate::ai::{self, Claude};
use crate::bail;
use crate::error::{Error, Result};
use crate::model::{self, Definition};

/// The effort of a suggestion call: chips have to arrive while the row is still selected.
pub const EFFORT: &str = "low";
/// Chips returned at most.
pub const MAX_CHIPS: usize = 6;
/// The longest chip kept, in characters (an instruction; a mood word is shorter).
const MAX_CHARS: usize = 40;
const MAX_WORD_CHARS: usize = 18;

/// What a row of chips can be for.
pub const TARGETS: &[&str] = &["row", "card", "whole", "plan", "design", "sound", "mood", "pitch"];
/// The parts of a cut a row can point at (the app's 直す範囲).
pub const ASPECTS: &[&str] = &["shot", "camera", "light", "sound", "finish"];

/// One request for chips, as the app sends it.
#[derive(Debug, Clone, Default)]
pub struct Ask {
    pub target: String,
    /// The cut (row, sound) or the script line (card) the chips are for.
    pub shot: String,
    pub line: String,
    /// Which part of the cut (`ASPECTS`); a row only.
    pub aspect: String,
    /// The chips already stacked, in order.
    pub stack: Vec<String>,
    /// The home's prompt as typed so far (`pitch` only: there is no project yet).
    pub pitch: String,
    pub genre: String,
    pub kind: String,
    /// The language to write the chips in: "ja" or "en".
    pub lang: String,
}

impl Ask {
    pub fn from_json(v: &Map<String, Value>) -> Ask {
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
        let stack = v
            .get("stack")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .take(12)
            .collect();
        Ask {
            target: s("target"),
            shot: s("shot"),
            line: s("line"),
            aspect: s("aspect"),
            stack,
            pitch: s("pitch"),
            genre: s("genre"),
            kind: s("kind"),
            lang: s("lang"),
        }
    }

    /// Mood words (the mood board, the home's prompt) rather than instructions.
    fn words(&self) -> bool {
        matches!(self.target.as_str(), "mood" | "pitch")
    }

    fn english(&self) -> bool {
        self.lang == "en"
    }
}

pub fn system() -> String {
    "You help a solo indie developer direct a game teaser without typing. The app shows tappable chips; each \
     chip is one short instruction the developer can stack with others and send on. The stacked chips are joined \
     into a note that another step carries out, within a scope that code enforces. Predict the chips this \
     developer most likely wants to tap next, for exactly the target described.\n\n\
     - Return 6 chips, the most useful first.\n\
     - One chip is one concrete change, a few words long, written as a request without a final period. Aim for \
     6 to 16 Japanese characters, or 2 to 5 English words. Mood words (for a mood board or a first prompt) are \
     nouns or short noun phrases, not sentences.\n\
     - Stay inside the target's scope. Never offer a chip the scope cannot carry out.\n\
     - The developer has already stacked some chips: offer what goes well with them as the next link of the same \
     chain. Never repeat a stacked chip or contradict it.\n\
     - Ground the chips in the level as the location scout read it (the stage): name what is really there, a \
     lamp, a window, the fog, a doorway, the way the light falls, rather than generic film words. A chip that asks \
     for something the level does not have cannot be filmed.\n\
     - Prefer chips that serve the cut's purpose and the teaser's direction, and chips that fix what the shot \
     check names. Vary them: a camera chip, a timing chip and a bolder idea read better than six near-copies.\n\
     - Never invent facts about the game (a release date, a platform, a price, a feature the project does not \
     state). Words to burn in are short and in the voice of the script."
        .to_string()
}

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {"chips": {"type": "array", "items": {"type": "string"}}},
        "required": ["chips"],
        "additionalProperties": false,
    })
}

/// What a target lets the chips change.
fn scope_text(ask: &Ask) -> &'static str {
    match (ask.target.as_str(), ask.aspect.as_str()) {
        ("row", "camera") => "the camera of this cut only: its position, height, direction, lens, framing and movement",
        ("row", "light") => "the light of this cut only: the time of day and where the light comes from",
        ("row", "sound") => "the sound effects of this cut only: which sound, when it hits, how loud, how bright or muffled",
        ("row", "finish") => {
            "the finishing of this cut only: the words burned in on it, when they show and how they enter, the \
             hand-over to the next cut (a black or white frame, a whiteout, a glitch), its brightness, and camera \
             effects (chroma, glitch, flicker, static, vhs, shake, redflash). Nothing is filmed again"
        }
        ("row", _) => "anything about this cut: what it shows, the camera, the light, its length and timing",
        ("card", _) => "this card between cuts only: its words, their form and how long it holds",
        ("whole", _) => "the whole teaser: its length, the order of the cuts, the pacing, the cards between cuts, where the title lands",
        ("plan", _) => {
            "how to plan this teaser before any cut exists: the tone, the pace, the structure, what to show first \
             and last, what to avoid"
        }
        ("design", _) => {
            "the finishing of the whole teaser only: the words and their timing, the hand-overs between cuts, the \
             colour grade, the letterbox bars, the typefaces, the energy of the motion. Cameras, light and sound stay"
        }
        ("sound", _) => "the sound only: the music, where it drops out, the sound effects and their levels. The picture stays",
        ("mood", _) => {
            "the mood board: atmosphere words for the light, the colour, the air and weather, the textures. They \
             steer generated reference pictures, not the cuts"
        }
        _ => {
            "the developer's first prompt for a new teaser: atmosphere and direction words to stack onto what they \
             typed (the light, the colour, the air, the pace, the camera's temperament, how it ends)"
        }
    }
}

fn line_text(l: &crate::script::Line) -> String {
    let words = [l.head.as_str(), l.ja.as_str(), l.en.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" / ");
    let at = match (&l.after, &l.on) {
        (_, Some(on)) => format!("on {on} at +{}s", l.at),
        (Some(after), _) => format!("after {after}"),
        _ => String::new(),
    };
    format!("- {} {} ({}, {}s): {}", l.kind, l.id, at, l.dur, words)
}

fn project_text(defn: &Definition) -> String {
    let b = &defn.brief;
    let mut out = String::from("## Project\n");
    let mut put = |k: &str, v: &str| {
        if !v.trim().is_empty() {
            out.push_str(&format!("{k}: {}\n", v.trim()));
        }
    };
    put("Title", &defn.teaser.title);
    put("Pitch", &defn.teaser.pitch);
    put("Kind", &b.kind);
    put("Genre", &b.genre);
    put("Direction", &b.direction);
    put("Core experience", &b.core);
    put("Must show", &b.must_show.join(", "));
    put("Look", &b.design);
    put("Stage (what the location scout found)", &defn.stage.summary.chars().take(600).collect::<String>());
    out.push_str(&format!("Target length: {}s\n", defn.teaser.target_duration));
    if !defn.shots.is_empty() {
        out.push_str("\n## The cuts, in order\n");
        for (i, s) in defn.shots.iter().enumerate() {
            out.push_str(&format!(
                "{}. {} [{}] {}s: {}\n",
                i + 1,
                s["id"].as_str().unwrap_or("?"),
                s["beat"].as_str().unwrap_or("-"),
                model::num(&s["duration"]).unwrap_or(0.0),
                s["purpose"].as_str().unwrap_or("")
            ));
        }
    }
    if !defn.script.is_empty() {
        out.push_str("\n## Script\n");
        for l in &defn.script {
            out.push_str(&line_text(l));
            out.push('\n');
        }
    }
    out
}

/// What the location scout read at these anchors (`stage.anchors`), within a budget of characters.
fn stage_notes(defn: &Definition, only: &[&str], budget: usize) -> String {
    let mut out = String::new();
    for (name, note) in &defn.stage.anchors {
        if !only.is_empty() && !only.contains(&name.as_str()) {
            continue;
        }
        let line = format!("- {name}: {}\n", note.trim());
        if out.chars().count() + line.chars().count() > budget {
            break;
        }
        out.push_str(&line);
    }
    if out.is_empty() {
        out
    } else {
        format!("\n## The location scout's notes\n{out}")
    }
}

/// The anchors a cut films: where its camera stands and what it looks at.
fn shot_anchors(shot: &Value) -> Vec<&str> {
    let cam = &shot["camera"];
    let mut v: Vec<&str> = [&cam["anchor"], &cam["look_at"], &cam["end"]["anchor"]].iter().filter_map(|x| x.as_str()).collect();
    v.dedup();
    v
}

/// The shot check's findings on one cut (or on the whole teaser), as lines.
fn findings_text(defn: &Definition, shot: Option<&str>) -> String {
    let Ok(resolved) = model::resolve_all(defn) else { return String::new() };
    let lines: Vec<String> = crate::shotcheck::check(defn, &resolved)
        .into_iter()
        .filter(|f| f.shot.as_deref() == shot)
        .map(|f| format!("- {} {}: {} ({})", f.severity.as_str(), f.rule, f.message, f.fix))
        .collect();
    if lines.is_empty() {
        String::new()
    } else {
        format!("\n## Shot check\n{}\n", lines.join("\n"))
    }
}

/// The user turn: the project, the target and what is stacked.
pub fn prompt(defn: Option<&Definition>, ask: &Ask) -> String {
    let mut out = String::new();
    match defn {
        Some(d) if ask.target != "pitch" => out.push_str(&project_text(d)),
        _ => {
            out.push_str("## A new teaser (no project yet)\n");
            if !ask.kind.is_empty() {
                out.push_str(&format!("Kind: {}\n", ask.kind));
            }
            if !ask.genre.is_empty() {
                out.push_str(&format!("Genre: {}\n", ask.genre));
            }
            out.push_str(&format!("Prompt so far: {}\n", if ask.pitch.is_empty() { "(nothing typed yet)" } else { &ask.pitch }));
        }
    }
    out.push_str(&format!("\n## Target\n{}\n", scope_text(ask)));
    if let Some(d) = defn.filter(|_| ask.target != "pitch") {
        if let Ok(shot) = d.find(&ask.shot) {
            let i = d.index_of(&ask.shot).unwrap_or(0);
            out.push_str(&format!("\nThe cut: {} ({} of {})\n{}\n", ask.shot, i + 1, d.shots.len(), shot));
            let on: Vec<String> = d.script.iter().filter(|l| l.on.as_deref() == Some(ask.shot.as_str())).map(line_text).collect();
            if !on.is_empty() {
                out.push_str(&format!("Words on it:\n{}\n", on.join("\n")));
            }
            out.push_str(&findings_text(d, Some(&ask.shot)));
            let at = shot_anchors(shot);
            if !at.is_empty() {
                out.push_str(&stage_notes(d, &at, 1200));
            }
        } else if let Some(l) = d.script.iter().find(|l| l.id == ask.line) {
            out.push_str(&format!("\nThe card:\n{}\n", line_text(l)));
        } else {
            if ask.target == "whole" {
                out.push_str(&findings_text(d, None));
            }
            // the teaser as a whole, its mood and its plan draw on every place the scout saw
            out.push_str(&stage_notes(d, &[], 1800));
        }
    }
    if ask.stack.is_empty() {
        out.push_str("\n## Stacked so far\n(nothing yet: suggest good first chips)\n");
    } else {
        out.push_str("\n## Stacked so far, in order\n");
        for (i, s) in ask.stack.iter().enumerate() {
            out.push_str(&format!("{}. {s}\n", i + 1));
        }
    }
    out.push_str(if ask.english() { "\nWrite the chips in English.\n" } else { "\nWrite the chips in Japanese.\n" });
    out
}

/// Tidy what came back: one chip per idea, short, nothing already stacked, at most `MAX_CHIPS`.
pub fn tidy(raw: &Value, ask: &Ask) -> Vec<String> {
    let max = if ask.words() { MAX_WORD_CHARS } else { MAX_CHARS };
    let mut seen: Vec<String> = ask.stack.iter().map(|s| norm(s)).collect();
    let mut out = Vec::new();
    for v in raw["chips"].as_array().into_iter().flatten() {
        let Some(c) = v.as_str().map(clean) else { continue };
        let key = norm(&c);
        if key.is_empty() || c.chars().count() > max || seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push(c);
        if out.len() == MAX_CHIPS {
            break;
        }
    }
    out
}

/// One chip as it should read: no bullet or number in front, no full stop at the end, no quotes around it.
fn clean(s: &str) -> String {
    let mut t = s.trim().trim_start_matches(['-', '*', '•', '・', '✦', ' ', '\u{3000}']).trim();
    // "1. " / "2) " in front of a chip (but keep "30秒に収める")
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let rest = &t[digits..];
        if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")).or_else(|| rest.strip_prefix("．")) {
            t = r.trim_start();
        }
    }
    let t = t.trim_end_matches(['。', '.', '、', ',', ' ', '\u{3000}']);
    let quoted = [('「', '」'), ('"', '"'), ('“', '”'), ('\'', '\'')]
        .iter()
        .find_map(|(a, b)| t.strip_prefix(*a).and_then(|x| x.strip_suffix(*b)))
        .filter(|inner| !inner.contains(['「', '」', '"']));
    quoted.unwrap_or(t).trim().to_string()
}

/// Two chips are the same idea when they read the same without spaces, case and punctuation.
fn norm(s: &str) -> String {
    let punct = |c: char| matches!(c, '。' | '、' | '.' | ',' | '!' | '！' | '?' | '？' | '・');
    s.chars().filter(|c| !c.is_whitespace() && !punct(*c)).flat_map(char::to_lowercase).collect()
}

/// Ask Claude for the next chips (one quick call). `defn` is None on the home, before a project exists.
pub fn suggest(defn: Option<&Definition>, ask: &Ask, claude: &dyn Claude) -> Result<Vec<String>> {
    if !TARGETS.contains(&ask.target.as_str()) {
        bail!("unknown chip target '{}' (known: {})", ask.target, TARGETS.join(", "));
    }
    if ask.target == "row" && !ask.aspect.is_empty() && !ASPECTS.contains(&ask.aspect.as_str()) {
        bail!("unknown aspect '{}' (known: {})", ask.aspect, ASPECTS.join(", "));
    }
    let mut req = ai::request(&system(), &prompt(defn, ask), schema());
    req["output_config"]["effort"] = json!(EFFORT);
    req["max_tokens"] = json!(8000);
    let text = claude.ask(&req)?;
    let v: Value = serde_json::from_str(&text).map_err(|e| Error::user(format!("Claude returned malformed JSON: {e}")))?;
    Ok(tidy(&v, ask))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_prompt_never_asks_the_model_to_look_back_at_itself() {
        assert!(crate::craft::self_review_words(&system()).is_empty(), "{:?}", crate::craft::self_review_words(&system()));
    }
}
