//! State: what was pushed to the engine, which takes exist, and what changed when.
//!
//! Layout (relative to the folder holding shots.yaml):
//!
//!   .shotpilot/state.json            pushed snapshots (base for pull), current takes, history
//!   .shotpilot/jobs/                 job/result JSON exchanged with the engine
//!   render/<shot>/<quality>/<fp12>/  one immutable take per fingerprint, with take.json
//!   render/<shot>/<quality>/<fp12>~N/  the same definition rendered again because the game
//!                                    changed under it (content.rs)
//!
//! Takes are content-addressed by fingerprint and never overwritten. Reverting a shot's
//! definition (or the game content it showed) therefore reuses the old take instead of
//! rendering again.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::error::Result;

pub const STATE_VERSION: i64 = 1;
const HISTORY_KEEP: usize = 500;

pub fn now_iso() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string()
}

/// One take on disk: its take.json plus where it lives.
#[derive(Debug, Clone)]
pub struct Take {
    pub info: Map<String, Value>,
    pub dir: PathBuf,
}

impl Take {
    pub fn fingerprint(&self) -> &str {
        self.info.get("fingerprint").and_then(Value::as_str).unwrap_or("")
    }

    pub fn files(&self) -> Vec<String> {
        self.info
            .get("files")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    pub fn frames(&self) -> Vec<i64> {
        self.info
            .get("frames")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default()
    }

    pub fn created(&self) -> &str {
        self.info.get("created").and_then(Value::as_str).unwrap_or("")
    }

    /// Middle frame, the one the storyboard and the app show.
    pub fn thumb(&self) -> Option<PathBuf> {
        let files = self.files();
        files.get(files.len() / 2).map(|f| self.dir.join(f))
    }
}

pub struct State {
    pub dir: PathBuf,
    pub path: PathBuf,
    pub render_dir: PathBuf,
    pub data: Value,
}

impl State {
    pub fn open(root: &Path) -> Result<State> {
        let dir = root.join(".shotpilot");
        let path = dir.join("state.json");
        let data = if path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&path)?)?
        } else {
            json!({"version": STATE_VERSION, "shots": {}, "history": []})
        };
        Ok(State { dir, path, render_dir: root.join("render"), data })
    }

    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.data)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn shots(&mut self) -> &mut Map<String, Value> {
        if !self.data["shots"].is_object() {
            self.data["shots"] = json!({});
        }
        self.data["shots"].as_object_mut().expect("object")
    }

    fn shot(&mut self, shot_id: &str) -> &mut Map<String, Value> {
        let s = self.shots().entry(shot_id).or_insert_with(|| json!({"current": {}}));
        if !s["current"].is_object() {
            s["current"] = json!({});
        }
        s.as_object_mut().expect("object")
    }

    // ----------------------------------------------------------------- takes

    pub fn take_dir(&self, shot_id: &str, quality: &str, fp: &str) -> PathBuf {
        self.render_dir.join(shot_id).join(quality).join(&fp[..fp.len().min(12)])
    }

    pub fn find_take(&self, shot_id: &str, quality: &str, fp: &str) -> Option<Take> {
        let dir = self.take_dir(shot_id, quality, fp);
        let text = std::fs::read_to_string(dir.join("take.json")).ok()?;
        let info: Map<String, Value> = serde_json::from_str(&text).ok()?;
        let complete = info.get("complete").and_then(Value::as_bool).unwrap_or(false);
        if info.get("fingerprint").and_then(Value::as_str) != Some(fp) || !complete {
            return None;
        }
        Some(Take { info, dir })
    }

    /// Every complete take of this definition (fingerprint), newest first. There is more than
    /// one when the game changed under a shot and it was rendered again: the first lives in
    /// `<fp12>/`, later ones in `<fp12>~2/`, `<fp12>~3/`, ... (see content.rs).
    pub fn takes_for(&self, shot_id: &str, quality: &str, fp: &str) -> Vec<Take> {
        let base = self.take_dir(shot_id, quality, fp);
        let name = base.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let Some(parent) = base.parent() else { return Vec::new() };
        let Ok(rd) = std::fs::read_dir(parent) else { return Vec::new() };
        let mut out: Vec<(u32, Take)> = rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                let gen = if n == name { 1 } else { n.strip_prefix(&format!("{name}~"))?.parse::<u32>().ok()? };
                let dir = e.path();
                let info: Map<String, Value> = serde_json::from_str(&std::fs::read_to_string(dir.join("take.json")).ok()?).ok()?;
                let complete = info.get("complete").and_then(Value::as_bool).unwrap_or(false);
                (complete && info.get("fingerprint").and_then(Value::as_str) == Some(fp)).then_some((gen, Take { info, dir }))
            })
            .collect();
        out.sort_by_key(|(gen, _)| std::cmp::Reverse(*gen));
        out.into_iter().map(|(_, t)| t).collect()
    }

    /// Where a new take of this definition goes without touching a finished one: the plain
    /// `<fp12>/` while it holds no complete take, else the next free `<fp12>~N/`.
    pub fn new_take_dir(&self, shot_id: &str, quality: &str, fp: &str) -> PathBuf {
        let base = self.take_dir(shot_id, quality, fp);
        let complete = |d: &Path| {
            std::fs::read_to_string(d.join("take.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .is_some_and(|v| v["complete"].as_bool().unwrap_or(false))
        };
        if !complete(&base) {
            return base;
        }
        let name = base.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        (2..).map(|n| base.with_file_name(format!("{name}~{n}"))).find(|d| !complete(d)).expect("unbounded")
    }

    pub fn write_take(&mut self, shot_id: &str, quality: &str, fp: &str, extra: Map<String, Value>) -> Result<PathBuf> {
        let dir = self.take_dir(shot_id, quality, fp);
        self.write_take_at(&dir, shot_id, quality, fp, extra)
    }

    /// [`Self::write_take`] into a given take folder (see [`Self::new_take_dir`]).
    pub fn write_take_at(&mut self, dir: &Path, shot_id: &str, quality: &str, fp: &str, extra: Map<String, Value>)
        -> Result<PathBuf> {
        let dir = dir.to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let mut meta = Map::new();
        meta.insert("shot".into(), json!(shot_id));
        meta.insert("quality".into(), json!(quality));
        meta.insert("fingerprint".into(), json!(fp));
        meta.insert("created".into(), json!(now_iso()));
        meta.extend(extra);
        std::fs::write(dir.join("take.json"), serde_json::to_string_pretty(&meta)?)?;
        self.mark_current(shot_id, quality, fp);
        Ok(dir)
    }

    pub fn mark_current(&mut self, shot_id: &str, quality: &str, fp: &str) {
        self.shot(shot_id)["current"][quality] = json!(fp);
    }

    pub fn current(&self, shot_id: &str, quality: &str) -> Option<String> {
        self.data["shots"][shot_id]["current"][quality].as_str().map(str::to_string)
    }

    pub fn list_takes(&self, shot_id: &str, quality: &str) -> Vec<Take> {
        let base = self.render_dir.join(shot_id).join(quality);
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&base) {
            for entry in rd.flatten() {
                let dir = entry.path();
                if let Ok(text) = std::fs::read_to_string(dir.join("take.json")) {
                    if let Ok(info) = serde_json::from_str::<Map<String, Value>>(&text) {
                        out.push(Take { info, dir });
                    }
                }
            }
        }
        // `created` has one-second resolution; the file time breaks ties (read once per take)
        out.sort_by_cached_key(|t| {
            let mtime = std::fs::metadata(t.dir.join("take.json")).and_then(|m| m.modified()).ok();
            (t.created().to_string(), mtime)
        });
        out
    }

    /// The newest take of this shot and quality ([`Self::list_takes`]'s last), reading only
    /// that one take.json: takes are written once, so the newest file is the newest take.
    pub fn latest_take(&self, shot_id: &str, quality: &str) -> Option<Take> {
        let rd = std::fs::read_dir(self.render_dir.join(shot_id).join(quality)).ok()?;
        let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = rd
            .flatten()
            .filter_map(|e| {
                let dir = e.path();
                let t = std::fs::metadata(dir.join("take.json")).and_then(|m| m.modified()).ok()?;
                Some((t, dir))
            })
            .collect();
        dirs.sort();
        // newest first; one that does not parse is skipped, as list_takes skips it
        dirs.into_iter().rev().find_map(|(_, dir)| {
            let text = std::fs::read_to_string(dir.join("take.json")).ok()?;
            let info = serde_json::from_str::<Map<String, Value>>(&text).ok()?;
            Some(Take { info, dir })
        })
    }

    /// Whether any take of this shot and quality exists, without reading them.
    pub fn has_takes(&self, shot_id: &str, quality: &str) -> bool {
        std::fs::read_dir(self.render_dir.join(shot_id).join(quality))
            .map(|rd| rd.flatten().any(|e| e.path().join("take.json").is_file()))
            .unwrap_or(false)
    }

    // ----------------------------------------------------------------- engine sync

    pub fn pushed(&self, shot_id: &str) -> Option<&Value> {
        self.data["shots"].get(shot_id)?.get("pushed").filter(|v| !v.is_null())
    }

    pub fn pushed_fp(&self, shot_id: &str) -> Option<String> {
        self.pushed(shot_id)?.get("fingerprint")?.as_str().map(str::to_string)
    }

    pub fn pushed_payload(&self, shot_id: &str) -> Option<Value> {
        self.pushed(shot_id)?.get("payload").cloned()
    }

    pub fn set_pushed(&mut self, shot_id: &str, fp: &str, payload: Value) {
        self.shot(shot_id).insert("pushed".into(), json!({"fingerprint": fp, "payload": payload, "at": now_iso()}));
    }

    // ----------------------------------------------------------------- approvals

    /// The look fingerprint a human approved for this shot, if any.
    pub fn approved_fp(&self, shot_id: &str) -> Option<String> {
        self.data["shots"].get(shot_id)?.get("approved")?.get("fingerprint")?.as_str().map(str::to_string)
    }

    pub fn set_approved(&mut self, shot_id: &str, fp: Option<&str>) {
        match fp {
            Some(fp) => {
                self.shot(shot_id).insert("approved".into(), json!({"fingerprint": fp, "at": now_iso()}));
            }
            None => {
                self.shot(shot_id).remove("approved");
            }
        }
    }

    pub fn forget(&mut self, shot_id: &str) {
        self.shots().remove(shot_id);
    }

    // ----------------------------------------------------------------- history

    pub fn log(&mut self, action: &str, detail: Value) {
        let mut entry = Map::new();
        entry.insert("at".into(), json!(now_iso()));
        entry.insert("action".into(), json!(action));
        if let Value::Object(d) = detail {
            entry.extend(d);
        }
        if !self.data["history"].is_array() {
            self.data["history"] = json!([]);
        }
        let h = self.data["history"].as_array_mut().expect("array");
        h.push(Value::Object(entry));
        // keep the file small; the take.json files keep the long-term record
        if h.len() > HISTORY_KEEP {
            let drop = h.len() - HISTORY_KEEP;
            h.drain(..drop);
        }
    }

    /// Merge more detail into the newest history entry (e.g. Claude's summary of an edit).
    pub fn annotate_last(&mut self, extra: Map<String, Value>) {
        if let Some(Value::Object(last)) = self.data["history"].as_array_mut().and_then(|h| h.last_mut()) {
            last.extend(extra);
        }
    }

    pub fn history(&self) -> Vec<Value> {
        self.data["history"].as_array().cloned().unwrap_or_default()
    }
}
