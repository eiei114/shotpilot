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

use std::collections::{BTreeSet, HashMap};
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

    /// The pack it came in (its folder when the catalogue names none).
    fn pack_key(&self) -> &str {
        self.pack.as_deref().unwrap_or_else(|| self.path.rsplit_once('/').map_or("", |(d, _)| d))
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
        let [file, renamed] = copy_names(s);
        let dir = project_root.join(IMPORT_DIR);
        std::fs::create_dir_all(&dir)?;
        let size = std::fs::metadata(&src)?.len();
        let same = |p: &Path| std::fs::metadata(p).is_ok_and(|m| m.len() == size);
        let name = if dir.join(&file).exists() && !same(&dir.join(&file)) { renamed } else { file };
        let dest = dir.join(&name);
        if !(dest.exists() && same(&dest)) {
            let tmp = dir.join(format!(".{name}.partial"));
            std::fs::copy(&src, &tmp)?;
            std::fs::rename(&tmp, &dest)?;
        }
        self.record_use(id, project_root);
        Ok(format!("{IMPORT_DIR}/{name}"))
    }

    /// `sounds/usage.json`: which projects each sound was copied into, so a later teaser can
    /// sound like itself rather than like the last one.
    fn usage_path(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join("usage.json"))
    }

    fn usage(&self) -> serde_json::Map<String, Value> {
        self.usage_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default()
    }

    /// Note that `id` went into the project at `project_root` (best effort: a library on a
    /// read-only drive just does not remember).
    fn record_use(&self, id: &str, project_root: &Path) {
        let Some(path) = self.usage_path() else { return };
        let root = plain(project_root);
        let mut usage = self.usage();
        let list = usage.entry(id.to_string()).or_insert_with(|| json!([]));
        if let Some(a) = list.as_array_mut() {
            if !a.iter().any(|v| v.as_str().is_some_and(|r| plain(Path::new(r)) == root)) {
                a.push(json!(root.display().to_string()));
                let _ = std::fs::write(&path, serde_json::to_string_pretty(&Value::Object(usage)).unwrap_or_default());
            }
        }
    }

    /// For each sound, how many teasers of other games use it now: the projects the usage
    /// record names whose shots.yaml still points at the sound's copy. This project and the
    /// other videos of its game (projects on one promo plan, D-47) do not count: a game may
    /// keep its own sound.
    pub fn worn(&self, project_root: &Path) -> HashMap<String, usize> {
        let here = plain(project_root);
        let game = crate::promo::plan_of(&here);
        let mut texts: HashMap<PathBuf, Option<String>> = HashMap::new();
        let mut out = HashMap::new();
        for (id, roots) in self.usage() {
            let Some(s) = self.get(&id) else { continue };
            let names = copy_names(s).map(|n| format!("{IMPORT_DIR}/{n}"));
            let users: BTreeSet<PathBuf> = roots
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|r| plain(Path::new(r)))
                .filter(|r| *r != here)
                .filter(|r| {
                    let text = texts.entry(r.clone()).or_insert_with(|| {
                        (crate::promo::plan_of(r) != game).then(|| std::fs::read_to_string(r.join("shots.yaml")).ok()).flatten()
                    });
                    text.as_deref().is_some_and(|t| names.iter().any(|n| t.contains(n.as_str())))
                })
                .collect();
            if !users.is_empty() {
                out.insert(id, users.len());
            }
        }
        out
    }

    /// How many teasers of other games use this sound now (see `worn`).
    pub fn other_uses(&self, s: &Sound, project_root: &Path) -> usize {
        self.worn(project_root).get(&s.id).copied().unwrap_or(0)
    }

    /// Whether a sound has already been copied into this project.
    pub fn imported(&self, s: &Sound, project_root: &Path) -> bool {
        copy_names(s).iter().any(|n| project_root.join(IMPORT_DIR).join(n).is_file())
    }

    /// The list Claude picks sounds from, for one project (or none): music first, then the
    /// effects. Within each, sounds no other teaser uses come before the worn ones (marked
    /// with how many), and the packs take turns, so a limit never cuts a stretch of the
    /// alphabet or a whole pack. Returns the list and how many sounds did not fit.
    pub fn prompt_list_for(&self, limit: usize, project_root: Option<&Path>) -> (String, usize) {
        let worn = project_root.map(|r| self.worn(r)).unwrap_or_default();
        let order = |mut group: Vec<&Sound>| -> Vec<(usize, Sound)> {
            group.sort_by(|a, b| a.name.cmp(&b.name));
            let mut turn: HashMap<String, usize> = HashMap::new();
            let mut ranked: Vec<(usize, usize, &Sound)> = group
                .into_iter()
                .map(|s| {
                    let t = turn.entry(s.pack_key().to_string()).or_insert(0);
                    *t += 1;
                    (worn.get(&s.id).copied().unwrap_or(0), *t, s)
                })
                .collect();
            ranked.sort_by(|a, b| {
                a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.pack_key().cmp(b.2.pack_key())).then(a.2.name.cmp(&b.2.name))
            });
            ranked.into_iter().map(|(n, _, s)| (n, s.clone())).collect()
        };
        let all: Vec<(usize, Sound)> = order(self.sounds.iter().filter(|s| s.is_bgm()).collect())
            .into_iter()
            .chain(order(self.sounds.iter().filter(|s| !s.is_bgm()).collect()))
            .collect();
        let line = |(n, s): &(usize, Sound)| {
            let d = s.duration_s.map(|d| format!("{d:.1}s")).unwrap_or_default();
            let worn = if *n > 0 { format!(" [used in {n} other teaser{}]", if *n == 1 { "" } else { "s" }) } else { String::new() };
            format!("- {} [{}{}] {} ({}){worn}", s.id, if s.is_bgm() { "bgm " } else { "" }, d, s.name, s.tags.join(", "))
        };
        let hidden = all.len().saturating_sub(limit);
        (all.iter().take(limit).map(line).collect::<Vec<_>>().join("\n"), hidden)
    }
}

/// The names a sound's copy can have in a project: its file name, or that name with the id's
/// tail when another file of that name got there first.
fn copy_names(s: &Sound) -> [String; 2] {
    let file = s.path.rsplit('/').next().unwrap_or("sound.wav").to_string();
    let tail: String =
        s.id.chars().rev().filter(char::is_ascii_alphanumeric).take(6).collect::<Vec<_>>().into_iter().rev().collect();
    let renamed = match file.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}-{tail}.{ext}"),
        None => format!("{file}-{tail}"),
    };
    [file, renamed]
}

/// A project folder as one comparable path: resolved, without Windows' `\\?\` prefix (so a
/// relative promo link still joins onto it).
fn plain(p: &Path) -> PathBuf {
    let c = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    match c.to_str().and_then(|t| t.strip_prefix(r"\\?\")) {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_remembers_which_teasers_use_a_sound_and_lists_the_worn_ones_last() {
        let d = tempfile::tempdir().unwrap();
        let l = lib(d.path());
        let (a, b, c) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        for p in [&a, &b] {
            let f = l.import("splice:a1", p.path()).unwrap();
            std::fs::write(p.path().join("shots.yaml"), format!("shots:\n  - {{id: s1, sfx: [{{file: {f}}}]}}\n")).unwrap();
        }
        l.import("splice:a1", b.path()).unwrap(); // once per project
        let hit = l.get("splice:a1").unwrap();
        assert_eq!(l.other_uses(hit, c.path()), 2);
        assert_eq!(l.other_uses(hit, a.path()), 1, "the project itself does not count");
        let (list, hidden) = l.prompt_list_for(10, Some(c.path()));
        assert_eq!(hidden, 0);
        assert!(list.contains("[used in 2 other teasers]"), "{list}");
        let fx: Vec<&str> = list.lines().filter(|x| !x.contains("[bgm")).collect();
        assert!(fx.last().unwrap().contains("splice:a1"), "the worn hit goes last: {list}");
        // a project that dropped the sound no longer counts (its copy may stay behind)
        std::fs::write(b.path().join("shots.yaml"), "shots: []\n").unwrap();
        assert_eq!(l.other_uses(hit, c.path()), 1);
    }

    #[test]
    fn a_renamed_copy_is_still_a_use_and_videos_of_the_same_game_are_not_other_teasers() {
        let d = tempfile::tempdir().unwrap();
        let l = lib(d.path());
        let games = tempfile::tempdir().unwrap();
        let (a, b, c) = (games.path().join("A"), games.path().join("B"), games.path().join("C"));
        for p in [&a, &b, &c] {
            std::fs::create_dir_all(p).unwrap();
        }
        // A already had another pack's hit.wav, so the Horror one lands as hit-<tail>.wav
        l.import("splice:b2", &a).unwrap();
        let f = l.import("splice:a1", &a).unwrap();
        assert_ne!(f, "audio/lib/hit.wav");
        std::fs::write(a.join("shots.yaml"), format!("shots:\n  - {{id: s1, sfx: [{{file: {f}}}]}}\n")).unwrap();
        let hit = l.get("splice:a1").unwrap();
        assert_eq!(l.other_uses(hit, &c), 1, "the renamed copy counts");
        assert!(l.imported(hit, &a));
        // B is A's update trailer (one promo plan): the game keeps its sound
        std::fs::write(b.join("promo.json"), r#"{"link": "../A/promo.json"}"#).unwrap();
        assert_eq!(l.other_uses(hit, &b), 0);
        assert!(l.worn(&b).is_empty());
        assert_eq!(l.other_uses(hit, &c), 1);
    }

    #[test]
    fn a_list_that_must_be_cut_takes_the_packs_in_turn_and_says_how_much_it_left() {
        let d = tempfile::tempdir().unwrap();
        let sounds = d.path().join("sounds");
        let mut index = Vec::new();
        // pack A sorts first and is large; a cut by name would show only pack A
        for (pack, n) in [("A", 6), ("B", 2), ("C", 2)] {
            std::fs::create_dir_all(sounds.join(pack)).unwrap();
            for i in 0..n {
                let path = format!("{pack}/{pack}_{i}.wav");
                std::fs::write(sounds.join(&path), b"RIFF").unwrap();
                index.push(json!({"id": format!("x:{pack}{i}"), "path": path, "kind": "oneshot", "duration_s": 1.0,
                                  "pack": pack, "status": "ok"}));
            }
        }
        std::fs::write(sounds.join("library.json"), json!({"sounds": index}).to_string()).unwrap();
        let l = Library::open(Some(d.path())).unwrap();
        let (list, hidden) = l.prompt_list_for(6, None);
        assert_eq!(hidden, 4);
        for pack in ["x:A", "x:B", "x:C"] {
            assert_eq!(list.lines().filter(|x| x.starts_with(&format!("- {pack}"))).count(), 2, "{list}");
        }
    }

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
