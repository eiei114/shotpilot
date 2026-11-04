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

/// One image to make: a prompt, and optionally a picture to start from (a scout photo:
/// keep its geometry and framing, change the light and atmosphere).
#[derive(Debug, Clone, Default)]
pub struct ImageJob {
    pub prompt: String,
    pub base: Option<PathBuf>,
    /// The base is a wireframe layout (a previz frame), not a frame of the game: keep its
    /// camera and the placement of every box, replace the drawing with a finished picture.
    pub layout: bool,
    /// Frames of the actual game to match for materials, architecture and light (not framing).
    pub refs: Vec<PathBuf>,
}

/// Said of a layout base (after "Attached image #k" / "The first attached image").
const LAYOUT_NOTE: &str = "is a wireframe layout of this exact shot: keep its camera position, lens, horizon line, and \
                           the position and size on screen of every box (each labelled box is an object or place in the \
                           scene; the red cross is where the camera looks). Replace the wireframe, grid and labels with a \
                           finished picture; draw no boxes, lines or labels.";
/// Said of reference images (after "Attached images #a, #b" / "The other attached images").
const REFS_NOTE: &str = "are frames from the actual game: match their materials, architecture, props and lighting, not \
                         their framing.";

impl ImageJob {
    /// What the attached images are, for a backend that sends them with one prompt
    /// (the base first, then the references).
    pub fn attachment_note(&self) -> String {
        let mut s = String::new();
        if self.base.is_some() {
            if self.layout {
                s.push_str(&format!("The first attached image {LAYOUT_NOTE} "));
            } else {
                s.push_str("Start from the first attached image (a frame from the actual game): keep its architecture, \
                            layout, camera position and framing; change only the lighting, weather, colour grade and \
                            atmosphere. ");
            }
        }
        if !self.refs.is_empty() {
            let who = if self.base.is_some() { "The other attached images" } else { "The attached images" };
            s.push_str(&format!("{who} {REFS_NOTE} "));
        }
        s
    }
}

/// Makes images from prompts. Tests swap in a fake; the real ones are [`CodexImages`]
/// (the user's logged-in Codex CLI) and [`OpenAiImages`] (an API key).
pub trait ImageGen {
    fn generate(&self, job: &ImageJob) -> Result<Vec<u8>>;

    /// Several images; backends with a high per-call cost make them in one go.
    /// Returns the images made so far if one fails after the first.
    fn generate_many(&self, jobs: &[ImageJob]) -> Result<Vec<Vec<u8>>> {
        let mut out = Vec::new();
        for j in jobs {
            match self.generate(j) {
                Ok(b) => out.push(b),
                Err(e) if out.is_empty() => return Err(e),
                Err(_) => break,
            }
        }
        Ok(out)
    }
}

/// Generates through the user's Codex CLI (`codex exec`), which has an image generation
/// tool and uses the account already logged in to Codex: no API key. Codex keeps what it
/// makes in `$CODEX_HOME/generated_images/<thread id>/`; we read the files from there, so
/// Codex runs read-only and never writes into the project.
pub struct CodexImages {
    exe: PathBuf,
    home: PathBuf,
}

impl CodexImages {
    pub fn new(exe: PathBuf) -> Option<CodexImages> {
        let home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| {
            std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(|h| PathBuf::from(h).join(".codex"))
        })?;
        Some(CodexImages { exe, home })
    }

    /// The instruction Codex gets (on stdin), and the base images to attach (in order).
    pub fn instruction(jobs: &[ImageJob]) -> (String, Vec<PathBuf>) {
        let mut attach: Vec<PathBuf> = Vec::new();
        let mut s = format!(
            "Use your image generation tool to generate exactly {} image(s), one per prompt below, in this order. \
             Landscape 16:9. Do not write, copy or save any files and do not run commands: generating the images is the \
             whole task. When done, reply with just the number of images generated.\n",
            jobs.len()
        );
        for (i, j) in jobs.iter().enumerate() {
            s.push_str(&format!("\n--- Image {} ---\n", i + 1));
            let mut slot = |p: &PathBuf| match attach.iter().position(|a| a == p) {
                Some(k) => k + 1,
                None => {
                    attach.push(p.clone());
                    attach.len()
                }
            };
            if let Some(b) = &j.base {
                let k = slot(b);
                if j.layout {
                    s.push_str(&format!("Attached image #{k} {LAYOUT_NOTE}\n"));
                } else {
                    s.push_str(&format!(
                        "Start from attached image #{k} (a frame from the actual game): keep its architecture, layout, \
                         camera position and framing; change only the lighting, weather, colour grade and atmosphere.\n"
                    ));
                }
            }
            if !j.refs.is_empty() {
                let ks: Vec<String> = j.refs.iter().map(|r| format!("#{}", slot(r))).collect();
                s.push_str(&format!("Attached images {} {REFS_NOTE}\n", ks.join(", ")));
            }
            s.push_str(&j.prompt);
            s.push('\n');
        }
        (s, attach)
    }
}

/// The thread id from `codex exec --json` output.
pub fn codex_thread_id(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .find(|v| v["type"] == "thread.started")
        .and_then(|v| v["thread_id"].as_str().map(str::to_string))
}

/// Images in a Codex generated_images folder, oldest first.
pub fn codex_images(dir: &Path) -> Vec<Vec<u8>> {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .map(|p| (std::fs::metadata(&p).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), p))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files.into_iter().filter_map(|(_, p)| std::fs::read(p).ok()).filter(|b| image_type(b).is_some()).collect()
}

impl ImageGen for CodexImages {
    fn generate(&self, job: &ImageJob) -> Result<Vec<u8>> {
        self.generate_many(std::slice::from_ref(job))?.into_iter().next().ok_or_else(|| Error::user("Codex made no image"))
    }

    fn generate_many(&self, jobs: &[ImageJob]) -> Result<Vec<Vec<u8>>> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let cwd = std::env::temp_dir();
        let mut cmd = Command::new(&self.exe);
        let (text, attach) = CodexImages::instruction(jobs);
        cmd.args(["exec", "--json", "--ephemeral", "--skip-git-repo-check", "-s", "read-only", "-C"]).arg(&cwd);
        for a in &attach {
            cmd.arg("-i").arg(a);
        }
        cmd.arg("-")
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let mut child = cmd.spawn().map_err(|e| Error::user(format!("cannot start Codex ({}): {e}", self.exe.display())))?;
        let mut stdin = child.stdin.take().expect("piped");
        let writer = std::thread::spawn(move || {
            let r = stdin.write_all(text.as_bytes());
            drop(stdin); // EOF: codex reads the prompt until stdin closes
            r
        });
        let out = child.wait_with_output().map_err(|e| Error::user(format!("Codex: {e}")))?;
        let _ = writer.join();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let Some(thread) = codex_thread_id(&stdout) else {
            let err = String::from_utf8_lossy(&out.stderr);
            let tail = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
            bail!("Codex did not start a session{}", if tail.is_empty() { String::new() } else { format!(": {tail}") });
        };
        let images = codex_images(&self.home.join("generated_images").join(&thread));
        if images.is_empty() {
            let said = stdout
                .lines()
                .rev()
                .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
                .filter(|v| v["item"]["type"] == "agent_message")
                .find_map(|v| v["item"]["text"].as_str().map(str::to_string))
                .unwrap_or_default();
            bail!("Codex made no image{}", if said.is_empty() { String::new() } else { format!(" ({})", said.trim()) });
        }
        Ok(images)
    }
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
    /// One request per image, all at once (each takes tens of seconds). Same contract as
    /// the default: the first failure is an error, a later one keeps the images before it.
    fn generate_many(&self, jobs: &[ImageJob]) -> Result<Vec<Vec<u8>>> {
        let results: Vec<Result<Vec<u8>>> = std::thread::scope(|sc| {
            let handles: Vec<_> = jobs.iter().map(|j| sc.spawn(move || self.generate(j))).collect();
            handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err(Error::user("image request panicked")))).collect()
        });
        let mut out = Vec::new();
        for r in results {
            match r {
                Ok(b) => out.push(b),
                Err(e) if out.is_empty() => return Err(e),
                Err(_) => break,
            }
        }
        Ok(out)
    }

    fn generate(&self, job: &ImageJob) -> Result<Vec<u8>> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(30))
            .timeout_read(Duration::from_secs(300))
            .build();
        let auth = format!("Bearer {}", self.key);
        let prompt = format!("{}{}", job.attachment_note(), job.prompt);
        let images: Vec<&PathBuf> = job.base.iter().chain(job.refs.iter()).collect();
        let resp = match images.first() {
            None => {
                let body = json!({"model": self.model, "prompt": prompt, "size": "1536x1024", "n": 1, "quality": "medium"});
                agent
                    .post("https://api.openai.com/v1/images/generations")
                    .set("content-type", "application/json")
                    .set("authorization", &auth)
                    .send_string(&body.to_string())
            }
            Some(_) => {
                let boundary = format!("shotpilot{}", chrono::Local::now().timestamp_nanos_opt().unwrap_or(0));
                let mut body: Vec<u8> = Vec::new();
                for (k, v) in [("model", self.model.as_str()), ("prompt", prompt.as_str()), ("size", "1536x1024"), ("quality", "medium")] {
                    body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").as_bytes());
                }
                for (i, path) in images.iter().enumerate() {
                    let bytes = std::fs::read(path)?;
                    let (ext, media) = image_type(&bytes).ok_or_else(|| Error::user("an attached image is not PNG / JPEG / WebP"))?;
                    body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"image[]\"; filename=\"image{i}.{ext}\"\r\nContent-Type: {media}\r\n\r\n").as_bytes());
                    body.extend(&bytes);
                    body.extend(b"\r\n");
                }
                body.extend(format!("--{boundary}--\r\n").as_bytes());
                agent
                    .post("https://api.openai.com/v1/images/edits")
                    .set("content-type", &format!("multipart/form-data; boundary={boundary}"))
                    .set("authorization", &auth)
                    .send_bytes(&body)
            }
        };
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

/// The scout photo a frame starts from (stage.mood picks the anchor; its front view).
pub fn base_for(defn: &Definition, frame: &str) -> Option<PathBuf> {
    let anchor = defn.stage.mood_base(frame)?;
    let root = defn.root();
    let idx = crate::scout::load_index(&root);
    let file = idx["anchors"][anchor].as_array()?.iter().find(|v| v["side"] == "front")?["file"].as_str()?.to_string();
    let p = root.join(file);
    p.is_file().then_some(p)
}

/// Image jobs for the mood board: frame key, prompt, and the scout photo it starts from.
pub fn jobs(defn: &Definition, direction: &str) -> Vec<(String, ImageJob)> {
    prompts(defn, direction)
        .into_iter()
        .map(|(k, prompt)| {
            let base = base_for(defn, &k);
            (k, ImageJob { prompt, base, ..Default::default() })
        })
        .collect()
}

/// Image prompts for the mood board, from the pitch, the brief and the stage.
pub fn prompts(defn: &Definition, direction: &str) -> Vec<(String, String)> {
    let tone = tone(defn, direction);
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

/// The world and tone an image model is told about: the set, the pitch, the brief and an
/// optional art direction.
pub fn tone(defn: &Definition, direction: &str) -> Vec<String> {
    let b = &defn.brief;
    let mut tone = Vec::new();
    if !defn.stage.summary.is_empty() {
        tone.push(format!("The set: {}", defn.stage.summary));
    }
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
    tone
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
