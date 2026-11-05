//! Concept frames (D-44): a rough picture per beat of the plan, made by an image model when the
//! human asks, then matched against the level by the location scout.
//!
//! The planning turn describes what each beat should look like (`frames`). The human presses
//! "make concept frames"; each beat gets one generated picture in `concept/`. The scout then
//! looks for places in the level whose photos feel like those pictures, photographing other
//! candidates when nothing fits, and records which scout photo answers which frame. The planner
//! gets each frame with the place found for it.
//!
//! Like the mood board, these images are references only: they never become takes, never reach
//! the cut, and never enter the fingerprint (the teaser is filmed from the game).

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::Result;
use crate::model::Definition;
use crate::moodboard::{image_type, ImageJob};

pub const DIR: &str = "concept";
/// How many times the scout adds new candidates for frames that found no place.
pub const MATCH_ROUNDS: usize = 3;
/// Candidates added per round for the frames still open.
pub const PICKS_PER_ROUND: usize = 4;

/// Where a frame was found in the level: one scout photo.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub anchor: String,
    /// front | right | back | left
    pub side: String,
    /// The scout photo (relative to the project root).
    pub file: String,
    /// good | rough
    pub fit: String,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub id: String,
    /// The beat it shows (hook, establish, escalate, title).
    pub beat: String,
    /// What the picture should look like, in the plan's words.
    pub look: String,
    /// The generated picture (relative to the root), once made.
    pub file: Option<String>,
    pub found: Option<Found>,
    /// Anchors already compared with it that did not fit.
    pub tried: Vec<String>,
}

impl Frame {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id, "beat": self.beat, "look": self.look, "file": self.file, "tried": self.tried,
            "found": self.found.as_ref().map(|f| json!({"anchor": f.anchor, "side": f.side, "file": f.file, "fit": f.fit, "why": f.why})),
        })
    }

    pub fn from_json(v: &Value) -> Option<Frame> {
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        let found = v.get("found").filter(|f| f.is_object()).and_then(|f| {
            let g = |k: &str| f.get(k).and_then(Value::as_str).map(str::to_string);
            Some(Found { anchor: g("anchor")?, side: g("side")?, file: g("file")?, fit: g("fit").unwrap_or_else(|| "rough".into()), why: g("why").unwrap_or_default() })
        });
        Some(Frame {
            id: s("id")?,
            beat: s("beat").unwrap_or_default(),
            look: s("look")?,
            file: s("file"),
            found,
            tried: v.get("tried").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect(),
        })
    }

    /// The picture is made and no place answers it yet.
    pub fn open(&self) -> bool {
        self.file.is_some() && self.found.is_none()
    }
}

fn index_path(root: &Path) -> PathBuf {
    root.join(DIR).join("index.json")
}

pub fn load(root: &Path) -> Vec<Frame> {
    std::fs::read_to_string(index_path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["frames"].as_array().cloned())
        .map(|a| a.iter().filter_map(Frame::from_json).collect())
        .unwrap_or_default()
}

pub fn save(root: &Path, frames: &[Frame]) -> Result<()> {
    std::fs::create_dir_all(root.join(DIR))?;
    let v = json!({"frames": frames.iter().map(Frame::to_json).collect::<Vec<_>>(), "saved": chrono::Local::now().to_rfc3339()});
    std::fs::write(index_path(root), serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

/// New frames from the planning turn: the descriptions only. A frame whose beat and look are
/// unchanged keeps its picture and its place; the others start over.
pub fn replace(root: &Path, looks: &[(String, String)]) -> Result<Vec<Frame>> {
    let old = load(root);
    let frames: Vec<Frame> = looks
        .iter()
        .enumerate()
        .map(|(i, (beat, look))| {
            old.iter().find(|f| &f.beat == beat && &f.look == look).cloned().map(|mut f| {
                f.id = format!("c{}", i + 1);
                f
            }).unwrap_or(Frame { id: format!("c{}", i + 1), beat: beat.clone(), look: look.clone(), file: None, found: None, tried: Vec::new() })
        })
        .collect();
    save(root, &frames)?;
    Ok(frames)
}

/// The image prompt for one frame: the beat's look in the game's world and the active look.
pub fn job(defn: &Definition, f: &Frame) -> ImageJob {
    let mut lines = crate::moodboard::tone(defn, "");
    if let Some(d) = crate::designsys::get(&defn.brief.design) {
        let palette: Vec<String> = d.tokens["palette"].as_object().into_iter().flatten().map(|(k, v)| format!("{k} {}", v.as_str().unwrap_or(""))).collect();
        lines.push(format!("Look: {} — {} Palette: {}.", d.title(), d.description, palette.join(", ")));
    }
    if !defn.brief.direction.is_empty() {
        lines.push(format!("Direction: {}", defn.brief.direction));
    }
    ImageJob {
        prompt: format!(
            "A rough concept frame for one beat ({}) of a video game teaser, 16:9, as if filmed inside the game engine. \
             {}. Show the composition clearly: what fills the frame, where the light comes from, how deep the space is. \
             No text, no logos, no user interface, no watermark.\n{}",
            f.beat,
            f.look.trim_end_matches('.'),
            lines.join("\n")
        ),
        ..Default::default()
    }
}

/// Store a generated picture for a frame; returns the path relative to the root.
pub fn store(root: &Path, f: &Frame, bytes: &[u8]) -> Result<String> {
    let Some((ext, _)) = image_type(bytes) else { crate::bail!("not a PNG, JPEG or WebP image") };
    let dir = root.join(DIR);
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let file = format!("{}-{}-{stamp}.{ext}", f.id, f.beat.replace(|c: char| !c.is_ascii_alphanumeric(), ""));
    std::fs::write(dir.join(&file), bytes)?;
    Ok(format!("{DIR}/{file}"))
}

/// An image block for Claude, when the file is readable and small enough.
pub fn image_block(root: &Path, rel: &str) -> Option<(Value, usize)> {
    let bytes = std::fs::read(root.join(rel)).ok()?;
    let (_, media) = image_type(&bytes)?;
    (bytes.len() <= crate::ai::MAX_IMAGE_BYTES)
        .then(|| (json!({"type": "image", "source": {"type": "base64", "media_type": media, "data": crate::ai::base64(&bytes)}}), bytes.len()))
}

/// What the planner gets: each frame's picture and, when the scout found it, the photo of
/// the place, each preceded by a line naming it.
pub fn planner_blocks(defn: &Definition) -> (Vec<Value>, String) {
    let root = defn.root();
    let frames = load(&root);
    let mut blocks = Vec::new();
    let mut text = Vec::new();
    for f in frames.iter().filter(|f| f.file.is_some()) {
        let Some((img, _)) = f.file.as_deref().and_then(|p| image_block(&root, p)) else { continue };
        blocks.push(json!({"type": "text", "text": format!("Concept frame `{}` ({}): {}", f.id, f.beat, f.look)}));
        blocks.push(img);
        match &f.found {
            Some(p) if defn.anchor(&p.anchor).is_some() => {
                if let Some((photo, _)) = image_block(&root, &p.file) {
                    blocks.push(json!({"type": "text", "text": format!("Found in the level for `{}`: `{}` seen from its {} side ({} fit)", f.id, p.anchor, p.side, p.fit)}));
                    blocks.push(photo);
                }
                text.push(format!("- `{}` ({}): film it at anchor `{}`, framed like its {} scout photo ({} fit: {})", f.id, f.beat, p.anchor, p.side, p.fit, p.why));
            }
            _ => text.push(format!("- `{}` ({}): no place in the level looks like it; take its mood, not its composition", f.id, f.beat)),
        }
    }
    let text = if text.is_empty() {
        String::new()
    } else {
        format!(
            "\n## Concept frames and where the level matches them\nThe images labelled \"Concept frame\" are generated sketches of what each beat should \
             feel like (never footage). The photo after a frame is the place in the level that looks most like it. Put that \
             beat's shot at that anchor, framed like that photo:\n{}\n",
            text.join("\n")
        )
    };
    (blocks, text)
}
