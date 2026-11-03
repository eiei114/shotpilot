//! The brief's mood board: a few reference images that settle the teaser's palette,
//! light and tone before any shot is planned.
//!
//! Images live in `moodboard/` next to shots.yaml and are listed in `brief.images`. They
//! come from the developer (their own screenshots or concept art) or from an image model
//! (OpenAI's Images API). The planner sees them as mood, not as shot designs: every frame of
//! the teaser is still rendered from the game itself.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use crate::bail;
use crate::error::{Error, Result};
use crate::model::Definition;

pub const DIR: &str = "moodboard";
pub const DEFAULT_IMAGE_MODEL: &str = "gpt-image-2";
/// Uploads and generated files above this are refused.
pub const MAX_BYTES: usize = 12_000_000;

pub fn image_model() -> String {
    std::env::var("SHOTPILOT_IMAGE_MODEL").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_IMAGE_MODEL.into())
}

/// Makes images from a prompt. Tests swap in a fake; the real one is [`OpenAiImages`].
pub trait ImageGen {
    fn generate(&self, prompt: &str) -> Result<Vec<u8>>;
}

pub struct OpenAiImages {
    key: String,
    model: String,
}

impl OpenAiImages {
    pub fn new(key: String) -> OpenAiImages {
        OpenAiImages { key, model: image_model() }
    }
}

impl ImageGen for OpenAiImages {
    fn generate(&self, prompt: &str) -> Result<Vec<u8>> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(30))
            .timeout_read(Duration::from_secs(300))
            .build();
        let body = json!({"model": self.model, "prompt": prompt, "size": "1536x1024", "n": 1, "quality": "medium"});
        let resp = agent
            .post("https://api.openai.com/v1/images/generations")
            .set("content-type", "application/json")
            .set("authorization", &format!("Bearer {}", self.key))
            .send_string(&body.to_string());
        let text = match resp {
            Ok(r) => r.into_string().map_err(|e| Error::user(format!("OpenAI: could not read the reply: {e}")))?,
            Err(ureq::Error::Status(code, r)) => {
                let detail = r
                    .into_string()
                    .ok()
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
                    .unwrap_or_default();
                match code {
                    401 => bail!("OpenAI rejected the API key"),
                    429 => bail!("OpenAI rate limited the request; retry shortly ({detail})"),
                    _ => bail!("OpenAI image error {code}: {detail}"),
                }
            }
            Err(e) => bail!("could not reach OpenAI: {e}"),
        };
        let v: Value = serde_json::from_str(&text).map_err(|e| Error::user(format!("OpenAI returned malformed JSON: {e}")))?;
        let b64 = v["data"][0]["b64_json"].as_str().ok_or_else(|| Error::user("OpenAI returned no image"))?;
        let bytes = base64_decode(b64).ok_or_else(|| Error::user("OpenAI returned an unreadable image"))?;
        image_type(&bytes).ok_or_else(|| Error::user("OpenAI returned something that is not an image"))?;
        Ok(bytes)
    }
}

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        match chunk.len() {
            4 => out.extend_from_slice(&bytes),
            3 => out.extend_from_slice(&bytes[..2]),
            2 => out.push(bytes[0]),
            _ => return None,
        }
    }
    Some(out)
}

/// (extension, media type) of a PNG / JPEG / WebP, from its first bytes.
pub fn image_type(b: &[u8]) -> Option<(&'static str, &'static str)> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("jpg", "image/jpeg"))
    } else if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else {
        None
    }
}

/// A safe file stem from a user's file name.
fn stem(name: &str) -> String {
    let base = Path::new(name).file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let s: String = base.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).take(40).collect();
    if s.is_empty() { "image".into() } else { s }
}

/// Write image bytes into moodboard/ under a fresh name; returns the path relative to the root.
pub fn store(root: &Path, name: &str, bytes: &[u8]) -> Result<String> {
    if bytes.len() > MAX_BYTES {
        bail!("image is larger than {} MB", MAX_BYTES / 1_000_000);
    }
    let Some((ext, _)) = image_type(bytes) else { bail!("not a PNG, JPEG or WebP image") };
    let dir = root.join(DIR);
    std::fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let s = stem(name);
    let mut k = 0;
    let file = loop {
        let f = if k == 0 { format!("{s}-{stamp}.{ext}") } else { format!("{s}-{stamp}-{k}.{ext}") };
        if !dir.join(&f).exists() {
            break f;
        }
        k += 1;
    };
    std::fs::write(dir.join(&file), bytes)?;
    Ok(format!("{DIR}/{file}"))
}

/// A mood-board path from brief.images, resolved inside moodboard/ only.
pub fn resolve(root: &Path, rel: &str) -> Option<PathBuf> {
    let rest = rel.strip_prefix(&format!("{DIR}/"))?;
    if rest.is_empty() || rest.contains(['/', '\\', ':']) || rest.starts_with('.') {
        return None;
    }
    Some(root.join(DIR).join(rest))
}

/// The frames an image model is asked for: one per kind of teaser image, all in the brief's tone.
pub const FRAMES: &[(&str, &str)] = &[
    ("place", "a wide establishing view of the main location, where the story happens"),
    ("detail", "a close detail that carries the mood (an object, a texture, a light source)"),
    ("threat", "the moment of unease or tension, the thing the viewer should fear or want"),
    ("title", "a calm, wide closing image with empty space in the upper third for a title card"),
];

/// Image prompts for the mood board, from the pitch and the brief.
pub fn prompts(defn: &Definition, direction: &str) -> Vec<(String, String)> {
    let b = &defn.brief;
    let mut tone = Vec::new();
    for (label, v) in [("Game", defn.teaser.pitch.as_str()), ("Genre", b.genre.as_str()), ("Core experience", b.core.as_str()),
                       ("Notes", b.notes.as_str())] {
        if !v.is_empty() {
            tone.push(format!("{label}: {v}"));
        }
    }
    if !b.must_show.is_empty() {
        tone.push(format!("Elements of the world: {}", b.must_show.join(", ")));
    }
    if !direction.trim().is_empty() {
        tone.push(format!("Art direction: {}", direction.trim()));
    }
    FRAMES
        .iter()
        .map(|(key, what)| {
            (key.to_string(), format!(
                "A cinematic still frame for a video game teaser, 16:9, shot as if inside the game engine. \
                 {what}. Consistent palette, lighting and atmosphere across the set. \
                 No text, no logos, no user interface, no watermark.\n{}",
                tone.join("\n")
            ))
        })
        .collect()
}

/// Mood-board images for the planner (image blocks), newest last, within the size limit.
pub fn image_blocks(defn: &Definition, limit: usize) -> Vec<Value> {
    let root = defn.root();
    defn.brief
        .images
        .iter()
        .filter_map(|rel| {
            let p = resolve(&root, rel)?;
            let bytes = std::fs::read(p).ok()?;
            let (_, media) = image_type(&bytes)?;
            (bytes.len() <= crate::ai::MAX_IMAGE_BYTES).then(|| {
                json!({"type": "image", "source": {"type": "base64", "media_type": media, "data": crate::ai::base64(&bytes)}})
            })
        })
        .take(limit)
        .collect()
}
