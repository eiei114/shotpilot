//! The global sound library shared by every project (D-26).
//!
//! It lives outside any project, at `~/Documents/ShotpilotLibrary` (or `$SHOTPILOT_LIBRARY`):
//!
//! ```text
//! sounds/library.json        catalogue: id, path, kind, duration_s, bpm, key, tags, pack, source
//! sounds/<path>              the files (e.g. splice/<pack>/<file>.wav)
//! ```
//!
//! A project never points into the library. Using a sound copies it into the project's
//! `audio/lib/`, so shots.yaml stays the single source of truth and the project still mixes
//! on a machine without the library.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::bail;
use crate::error::Result;
use crate::sound::AUDIO_EXTS;

/// Where imported sounds land inside a project.
pub const IMPORT_DIR: &str = "audio/lib";

#[derive(Debug, Clone)]
pub struct Sound {
    pub id: String,
    pub path: String,
    pub name: String,
    /// `loop` or `oneshot`
    pub kind: String,
    pub duration_s: Option<f64>,
    pub bpm: Option<f64>,
    pub key: Option<String>,
    pub tags: Vec<String>,
    pub pack: Option<String>,
}

impl Sound {
    /// Long loops are what a teaser can carry as background music.
    pub fn is_bgm(&self) -> bool {
        self.kind == "loop" && self.duration_s.is_some_and(|d| d >= 20.0)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id, "name": self.name, "kind": self.kind, "bgm": self.is_bgm(),
            "duration_s": self.duration_s, "bpm": self.bpm, "key": self.key,
            "tags": self.tags, "pack": self.pack,
        })
    }

    /// The text a query is matched against.
    fn haystack(&self) -> String {
        format!("{} {} {} {}", self.name, self.tags.join(" "), self.pack.as_deref().unwrap_or(""), self.kind)
            .to_lowercase()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    /// The `sounds/` folder. `None` when there is no library on this machine.
    pub dir: Option<PathBuf>,
    pub sounds: Vec<Sound>,
}

/// `$SHOTPILOT_LIBRARY`, else `~/Documents/ShotpilotLibrary`.
pub fn default_root() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SHOTPILOT_LIBRARY").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join("Documents").join("ShotpilotLibrary"))
}

fn str_of(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
}

/// A catalogue path must stay inside `sounds/` and name an audio file.
fn safe_rel(p: &str) -> bool {
    let segs: Vec<&str> = p.split('/').collect();
    !segs.iter().any(|s| s.is_empty() || *s == "." || *s == ".." || s.contains(['\\', ':']))
        && p.rsplit('.').next().is_some_and(|x| AUDIO_EXTS.contains(&x.to_ascii_lowercase().as_str()))
}

impl Library {
    /// Open the library under `root`. A missing library is empty, not an error: the app
    /// works without one.
    pub fn open(root: Option<&Path>) -> Result<Library> {
        let Some(dir) = root.map(|r| r.join("sounds")) else { return Ok(Library::default()) };
        let index = dir.join("library.json");
        let Ok(text) = std::fs::read_to_string(&index) else { return Ok(Library::default()) };
        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => bail!("{}: not valid JSON ({e})", index.display()),
        };
        let mut sounds = Vec::new();
        for s in v.get("sounds").and_then(Value::as_array).into_iter().flatten() {
            if s.get("status").and_then(Value::as_str).is_some_and(|st| st != "ok") {
                continue;
            }
            let (Some(id), Some(path)) = (str_of(s, "id"), str_of(s, "path")) else { continue };
            if !safe_rel(&path) {
                continue;
            }
            let name = str_of(s, "name").unwrap_or_else(|| path.rsplit('/').next().unwrap_or("").to_string());
            sounds.push(Sound {
                id,
                name,
                kind: str_of(s, "kind").unwrap_or_else(|| "oneshot".into()),
                duration_s: s.get("duration_s").and_then(Value::as_f64),
                bpm: s.get("bpm").and_then(Value::as_f64),
                key: str_of(s, "key"),
                tags: s.get("tags").and_then(Value::as_array).into_iter().flatten()
                    .filter_map(|t| t.as_str().map(str::to_string)).collect(),
                pack: str_of(s, "pack"),
                path,
            });
        }
        Ok(Library { dir: Some(dir), sounds })
    }

    pub fn get(&self, id: &str) -> Option<&Sound> {
        self.sounds.iter().find(|s| s.id == id)
    }

    /// The file on disk, if it is there.
    pub fn file(&self, s: &Sound) -> Option<PathBuf> {
        let p = s.path.split('/').fold(self.dir.clone()?, |acc, seg| acc.join(seg));
        p.is_file().then_some(p)
    }

    /// Every word of `query` must appear in the name, tags or pack. `kind` is `bgm`
    /// (long loops), `loop`, `oneshot` or empty for all. Sounds whose tags hold a word
    /// come first, then shorter names.
    pub fn search(&self, query: &str, kind: &str, limit: usize) -> Vec<&Sound> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let mut hits: Vec<(usize, &Sound)> = self
            .sounds
            .iter()
            .filter(|s| match kind {
                "bgm" => s.is_bgm(),
                "loop" | "oneshot" => s.kind == kind,
                _ => true,
            })
            .filter_map(|s| {
                let hay = s.haystack();
                if !words.iter().all(|w| hay.contains(w.as_str())) {
                    return None;
                }
                let tag_hits = words.iter().filter(|w| s.tags.iter().any(|t| t.to_lowercase() == **w)).count();
                Some((tag_hits, s))
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.name.len().cmp(&b.1.name.len())).then(a.1.name.cmp(&b.1.name)));
        hits.into_iter().take(limit).map(|(_, s)| s).collect()
    }

    /// Copy a sound into `<project>/audio/lib/` and return its project path
    /// (`audio/lib/<file>`) for shots.yaml. Importing twice reuses the copy; a different
    /// file with the same name gets the id's tail appended.
    pub fn import(&self, id: &str, project_root: &Path) -> Result<String> {
        let Some(s) = self.get(id) else { bail!("no sound '{id}' in the library") };
        let Some(src) = self.file(s) else { bail!("'{}' is in the catalogue but its file is missing", s.name) };
        let file = s.path.rsplit('/').next().unwrap_or("sound.wav").to_string();
        let dir = project_root.join(IMPORT_DIR);
        std::fs::create_dir_all(&dir)?;
        let size = std::fs::metadata(&src)?.len();
        let same = |p: &Path| std::fs::metadata(p).is_ok_and(|m| m.len() == size);
        let mut name = file.clone();
        if dir.join(&name).exists() && !same(&dir.join(&name)) {
            let tail: String = id.chars().rev().filter(char::is_ascii_alphanumeric).take(6).collect::<Vec<_>>()
                .into_iter().rev().collect();
            name = match file.rsplit_once('.') {
                Some((stem, ext)) => format!("{stem}-{tail}.{ext}"),
                None => format!("{file}-{tail}"),
            };
        }
        let dest = dir.join(&name);
        if !(dest.exists() && same(&dest)) {
            let tmp = dir.join(format!(".{name}.partial"));
            std::fs::copy(&src, &tmp)?;
            std::fs::rename(&tmp, &dest)?;
        }
        Ok(format!("{IMPORT_DIR}/{name}"))
    }

    /// Whether a sound has already been copied into this project.
    pub fn imported(&self, s: &Sound, project_root: &Path) -> bool {
        let file = s.path.rsplit('/').next().unwrap_or("");
        project_root.join(IMPORT_DIR).join(file).is_file()
    }

    /// A short list for Claude: what it may pick from when a note asks for a sound
    /// the project does not have yet.
    pub fn prompt_list(&self, limit: usize) -> String {
        let mut bgm: Vec<&Sound> = self.sounds.iter().filter(|s| s.is_bgm()).collect();
        bgm.sort_by(|a, b| a.name.cmp(&b.name));
        let mut fx: Vec<&Sound> = self.sounds.iter().filter(|s| !s.is_bgm()).collect();
        fx.sort_by(|a, b| a.name.cmp(&b.name));
        let line = |s: &&Sound| {
            let d = s.duration_s.map(|d| format!("{d:.1}s")).unwrap_or_default();
            format!("- {} [{}{}] {} ({})", s.id, if s.is_bgm() { "bgm " } else { "" }, d, s.name, s.tags.join(", "))
        };
        bgm.iter().chain(fx.iter()).take(limit).map(line).collect::<Vec<_>>().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(dir: &Path) -> Library {
        let sounds = dir.join("sounds");
        std::fs::create_dir_all(sounds.join("splice/Horror")).unwrap();
        std::fs::create_dir_all(sounds.join("splice/Other")).unwrap();
        std::fs::write(sounds.join("splice/Horror/hit.wav"), b"RIFF-hit").unwrap();
        std::fs::write(sounds.join("splice/Other/hit.wav"), b"RIFF-other-hit").unwrap();
        std::fs::write(sounds.join("splice/Horror/drone.wav"), b"RIFF-drone").unwrap();
        let index = json!({"sounds": [
            {"id": "splice:a1", "path": "splice/Horror/hit.wav", "kind": "oneshot", "duration_s": 1.2,
             "tags": ["fx", "impact", "horror"], "pack": "Horror", "status": "ok"},
            {"id": "splice:b2", "path": "splice/Other/hit.wav", "kind": "oneshot", "duration_s": 0.8,
             "tags": ["fx", "impact"], "pack": "Other", "status": "ok"},
            {"id": "splice:c3", "path": "splice/Horror/drone.wav", "kind": "loop", "duration_s": 64.0,
             "tags": ["drones", "horror"], "pack": "Horror", "status": "ok"},
            {"id": "splice:d4", "path": "splice/Horror/missing.wav", "kind": "oneshot", "status": "failed"},
            {"id": "evil", "path": "../outside.wav", "kind": "oneshot"},
        ]});
        std::fs::write(sounds.join("library.json"), index.to_string()).unwrap();
        Library::open(Some(dir)).unwrap()
    }

    #[test]
    fn missing_library_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let l = Library::open(Some(dir.path())).unwrap();
        assert!(l.sounds.is_empty());
        assert!(Library::open(None).unwrap().sounds.is_empty());
    }

    #[test]
    fn skips_failed_and_escaping_entries() {
        let dir = tempfile::tempdir().unwrap();
        let l = lib(dir.path());
        let ids: Vec<&str> = l.sounds.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["splice:a1", "splice:b2", "splice:c3"]);
    }

    #[test]
    fn search_needs_every_word_and_filters_kind() {
        let dir = tempfile::tempdir().unwrap();
        let l = lib(dir.path());
        let ids = |q: &str, k: &str| l.search(q, k, 10).iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("horror", "bgm"), ["splice:c3"]);
        assert_eq!(ids("impact horror", ""), ["splice:a1"]);
        assert_eq!(ids("", "oneshot").len(), 2);
        assert!(ids("whoosh", "").is_empty());
    }

    #[test]
    fn import_copies_once_and_keeps_names_apart() {
        let dir = tempfile::tempdir().unwrap();
        let l = lib(dir.path());
        let proj = dir.path().join("proj");
        assert_eq!(l.import("splice:a1", &proj).unwrap(), "audio/lib/hit.wav");
        assert_eq!(l.import("splice:a1", &proj).unwrap(), "audio/lib/hit.wav");
        // same file name, different sound
        assert_eq!(l.import("splice:b2", &proj).unwrap(), "audio/lib/hit-liceb2.wav");
        assert_eq!(std::fs::read(proj.join("audio/lib/hit.wav")).unwrap(), b"RIFF-hit");
        assert!(l.imported(l.get("splice:a1").unwrap(), &proj));
        assert!(l.import("nope", &proj).is_err());
        // the imported path is one shots.yaml accepts
        assert!(crate::sound::valid_file("audio/lib/hit.wav").is_ok());
    }
}
