//! Workspace: ties definition, state, engine and outputs together.
//!
//! Every operation is incremental by construction: it computes fingerprints from the
//! current definition, compares them with what exists, and only does the missing work.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Map, Value};

use crate::bail;
use crate::cut::{self, CutInfo, Pick};
use crate::engines::{make_engine, master_asset, readback_subset, shot_asset, MockEngine};
use crate::error::{Error, Result};
use crate::fingerprint as fp;
use crate::geometry::round_to;
use crate::model::{self, num, Definition, ResolvedShot};
use crate::previz;
use crate::shotcheck::{self, Finding, Severity};
use crate::sound::{self, Mixed};
use crate::state::{State, Take};
use crate::sync::{self, PullDecision, PullKind};
use crate::yamlio::Doc;

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "exr", "bmp"];
pub const QUALITY_RANK: &[&str] = &["final", "preview", "still", "previz"];

/// (shot, its take if any, the quality that take is in).
pub type TakePick = (ResolvedShot, Option<Take>, Option<String>);

/// Progress callback: ("plan", {...}) once, then ("shot", outcome) per shot.
pub type Progress<'a> = &'a mut dyn FnMut(&str, &Value);

#[derive(Debug, Clone)]
pub struct RenderOutcome {
    pub shot_id: String,
    pub quality: String,
    pub action: String, // reused | rendered | failed
    pub fingerprint: String,
    pub seconds: f64,
    pub detail: String,
    /// Middle frame of the take, when there is one.
    pub thumb: Option<PathBuf>,
}

impl RenderOutcome {
    pub fn to_json(&self) -> Value {
        json!({
            "shot_id": self.shot_id, "quality": self.quality, "action": self.action,
            "fingerprint": self.fingerprint, "seconds": self.seconds, "detail": self.detail,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ShotStatus {
    pub index: usize,
    pub id: String,
    pub duration: f64,
    pub purpose: String,
    pub beat: Option<String>,
    pub build: String,                    // synced | dirty | new
    pub qualities: Vec<(String, String)>, // fresh | stale | none
    /// approved | outdated (approved, then the picture changed) | none
    pub approval: String,
}

impl ShotStatus {
    pub fn quality(&self, q: &str) -> &str {
        self.qualities.iter().find(|(k, _)| k == q).map(|(_, v)| v.as_str()).unwrap_or("none")
    }

    pub fn to_json(&self) -> Value {
        let qs: Map<String, Value> = self.qualities.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        json!({
            "index": self.index, "id": self.id, "duration": self.duration, "purpose": self.purpose,
            "beat": self.beat, "build": self.build, "qualities": qs, "approval": self.approval,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuildReport {
    pub built: Vec<String>,
    pub diverged: Vec<String>,
    pub error: Vec<String>,
    /// Set when nothing needed building.
    pub skipped: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct EditReport {
    pub edited: Vec<String>,
    pub stale: Vec<String>,
    pub order: Vec<String>,
    /// The top-level `audio:` (BGM, master gain) changed.
    pub audio: bool,
}

impl EditReport {
    pub fn to_json(&self) -> Value {
        json!({"edited": self.edited, "stale": self.stale, "order": self.order, "audio": self.audio})
    }
}

#[derive(Debug, Clone, Default)]
pub struct ScanReport {
    pub added: Vec<String>,
    pub moved: Vec<String>,
    pub same: Vec<String>,
}

pub struct Workspace {
    pub path: PathBuf,
    pub root: PathBuf,
    pub defn: Definition,
    pub state: State,
}

impl Workspace {
    pub fn open(shots_path: impl AsRef<Path>) -> Result<Workspace> {
        let path = std::path::absolute(shots_path.as_ref())?;
        let defn = model::load(&path)?;
        let root = defn.root();
        let state = State::open(&root)?;
        Ok(Workspace { path, root, defn, state })
    }

    pub fn reload(&mut self) -> Result<()> {
        self.defn = model::load(&self.path)?;
        Ok(())
    }

    /// The mock engine, for tests and demos that simulate Sequencer edits.
    pub fn mock_engine(&self) -> Result<MockEngine> {
        MockEngine::new(&self.defn)
    }

    pub fn resolved(&self) -> Result<Vec<ResolvedShot>> {
        model::resolve_all(&self.defn)
    }

    /// The shot check (see shotcheck.rs), most severe first.
    pub fn check(&self) -> Result<Vec<Finding>> {
        Ok(shotcheck::check(&self.defn, &self.resolved()?))
    }

    /// P0 and P1 findings as one-line notes (the CLI's `note:` lines).
    pub fn structure_warnings(&self) -> Result<Vec<String>> {
        Ok(self.check()?.iter().filter(|f| f.severity <= Severity::P1).map(Finding::line).collect())
    }

    /// Shots by id, or by 1-based position ("3" = third shot). Empty = all.
    pub fn select(&self, ids: &[String]) -> Result<Vec<ResolvedShot>> {
        let shots = self.resolved()?;
        if ids.is_empty() {
            return Ok(shots);
        }
        let mut out = Vec::new();
        for i in ids {
            if let Some(s) = shots.iter().find(|s| &s.id == i) {
                out.push(s.clone());
            } else if let Some(n) = i.parse::<usize>().ok().filter(|n| (1..=shots.len()).contains(n)) {
                out.push(shots[n - 1].clone());
            } else {
                let known: Vec<&str> = shots.iter().map(|s| s.id.as_str()).collect();
                bail!("unknown shot '{i}'. known: {}", known.join(", "));
            }
        }
        Ok(out)
    }

    pub fn select_one(&self, id: &str) -> Result<ResolvedShot> {
        Ok(self.select(&[id.to_string()])?.remove(0))
    }

    // ------------------------------------------------------------------ status

    pub fn status(&self) -> Result<Vec<ShotStatus>> {
        let mut rows = Vec::new();
        for s in self.resolved()? {
            let bfp = fp::build_fingerprint(&self.defn, &s);
            let build = match self.state.pushed_fp(&s.id) {
                None => "new",
                Some(p) if p == bfp => "synced",
                Some(_) => "dirty",
            };
            let mut qualities = Vec::new();
            for q in fp::quality_names(&self.defn) {
                let rfp = fp::render_fingerprint(&self.defn, &s, &q)?;
                let st = if self.state.find_take(&s.id, &q, &rfp).is_some() {
                    "fresh"
                } else if !self.state.list_takes(&s.id, &q).is_empty() {
                    "stale"
                } else {
                    "none"
                };
                qualities.push((q, st.to_string()));
            }
            let approval = match self.state.approved_fp(&s.id) {
                None => "none",
                Some(a) if a == fp::look_fingerprint(&self.defn, &s) => "approved",
                Some(_) => "outdated",
            };
            rows.push(ShotStatus {
                approval: approval.into(),
                index: s.index,
                id: s.id.clone(),
                duration: s.duration,
                purpose: s.purpose.clone(),
                beat: s.beat.clone(),
                build: build.into(),
                qualities,
            });
        }
        Ok(rows)
    }

    // ------------------------------------------------------------------ build

    fn master(&self, shots: &[ResolvedShot]) -> Value {
        json!({
            "asset": master_asset(&self.defn), "fps": self.defn.teaser.fps,
            "shots": shots.iter().map(|s| json!({"id": s.id, "asset": shot_asset(&self.defn, &s.id), "frames": s.frames}))
                .collect::<Vec<_>>(),
        })
    }

    fn build_items(&self, shots: &[ResolvedShot], force: bool) -> Vec<Value> {
        let mut items = Vec::new();
        for s in shots {
            let bfp = fp::build_fingerprint(&self.defn, s);
            let pushed_fp = self.state.pushed_fp(&s.id);
            if !force && pushed_fp.as_deref() == Some(bfp.as_str()) {
                continue;
            }
            items.push(json!({
                "id": s.id, "asset": shot_asset(&self.defn, &s.id), "fingerprint": bfp,
                "payload": s.engine_payload(),
                "base": self.state.pushed_payload(&s.id).map(|p| readback_subset(&p)),
            }));
        }
        items
    }

    fn master_if_changed(&self, force: bool) -> Result<Option<Value>> {
        let master = self.master(&self.resolved()?);
        let changed = self.state.data.get("master_fp").and_then(Value::as_str) != Some(fp::digest(&master).as_str());
        Ok((force || changed).then_some(master))
    }

    fn absorb_build(&mut self, items: &[Value], result: &Value, master: Option<&Value>) -> BuildReport {
        let mut report = BuildReport::default();
        for it in items {
            let id = it["id"].as_str().unwrap_or_default().to_string();
            let status = result["shots"][&id]["status"].as_str().unwrap_or("error");
            match status {
                "built" => {
                    self.state.set_pushed(&id, it["fingerprint"].as_str().unwrap_or_default(), it["payload"].clone());
                    report.built.push(id);
                }
                "diverged" => report.diverged.push(id),
                _ => report.error.push(id),
            }
        }
        if let Some(m) = master {
            let mres = &result["master"];
            if mres.is_object() && mres.get("error").map_or(true, Value::is_null) {
                self.state.data["master_fp"] = json!(fp::digest(m));
            }
        }
        if !report.built.is_empty() || !report.diverged.is_empty() {
            self.state.log("build", json!({"built": report.built, "diverged": report.diverged}));
        }
        report
    }

    pub fn build(&mut self, ids: &[String], force: bool) -> Result<BuildReport> {
        let shots = self.select(ids)?;
        let items = self.build_items(&shots, force);
        let master = self.master_if_changed(force)?;
        if items.is_empty() && master.is_none() {
            return Ok(BuildReport { skipped: Some(shots.iter().map(|s| s.id.clone()).collect()), ..Default::default() });
        }
        let engine = make_engine(&self.defn)?;
        let result = engine.build(&items, master.as_ref(), force)?;
        let report = self.absorb_build(&items, &result, master.as_ref());
        self.state.save()?;
        Ok(report)
    }

    // ------------------------------------------------------------------ render

    pub fn render(&mut self, ids: &[String], quality: &str, force: bool, on_progress: Option<Progress>)
        -> Result<Vec<RenderOutcome>> {
        let mut noop = |_: &str, _: &Value| {};
        let emit: Progress = match on_progress {
            Some(f) => f,
            None => &mut noop,
        };
        let q = fp::quality(&self.defn, quality)?;
        let shots = self.select(ids)?;
        let mut outcomes: Vec<RenderOutcome> = Vec::new();
        let mut todo: Vec<(ResolvedShot, String)> = Vec::new();
        for s in &shots {
            let rfp = fp::render_fingerprint(&self.defn, s, quality)?;
            let take = if force { None } else { self.state.find_take(&s.id, quality, &rfp) };
            match take {
                Some(t) => {
                    self.state.mark_current(&s.id, quality, &rfp);
                    outcomes.push(RenderOutcome {
                        shot_id: s.id.clone(),
                        quality: quality.into(),
                        action: "reused".into(),
                        fingerprint: rfp,
                        seconds: 0.0,
                        detail: t.dir.display().to_string(),
                        thumb: t.thumb(),
                    });
                }
                None => todo.push((s.clone(), rfp)),
            }
        }

        emit("plan", &json!({
            "quality": quality,
            "render": todo.iter().map(|(s, _)| &s.id).collect::<Vec<_>>(),
            "reused": outcomes.iter().map(|o| &o.shot_id).collect::<Vec<_>>(),
        }));
        for o in &outcomes {
            emit("shot", &outcome_event(o));
        }

        if !todo.is_empty() && fp::is_previz(&q) {
            let done = self.render_previz(&todo, quality, &q, emit)?;
            outcomes.extend(done);
        } else if !todo.is_empty() {
            let done = self.render_engine(&todo, quality, &q, force)?;
            for o in &done {
                emit("shot", &outcome_event(o)); // one engine launch renders the whole batch
            }
            outcomes.extend(done);
        }

        if outcomes.iter().any(|o| o.action != "reused") {
            let by = |a: &str| outcomes.iter().filter(|o| o.action == a).map(|o| o.shot_id.clone()).collect::<Vec<_>>();
            self.state.log("render", json!({
                "quality": quality, "rendered": by("rendered"), "reused": by("reused"), "failed": by("failed"),
            }));
        }
        self.state.save()?;
        let order: HashMap<&str, usize> = shots.iter().enumerate().map(|(i, s)| (s.id.as_str(), i)).collect();
        outcomes.sort_by_key(|o| order.get(o.shot_id.as_str()).copied().unwrap_or(usize::MAX));
        Ok(outcomes)
    }

    fn take_info(&self, s: &ResolvedShot, q: &Map<String, Value>, frames: &[i64], files: &[String], seconds: f64,
                 engine: &str) -> Map<String, Value> {
        let (w, h) = fp::resolution(&self.defn, q);
        let v = json!({
            "complete": true, "engine": engine, "frames": frames, "files": files,
            "fps": s.fps, "shot_frames": s.frames, "resolution": [w, h],
            "seconds": round_to(seconds, 2), "payload": s.engine_payload(), "quality_preset": q,
        });
        v.as_object().cloned().unwrap_or_default()
    }

    fn render_previz(&mut self, todo: &[(ResolvedShot, String)], quality: &str, q: &Map<String, Value>,
                     emit: Progress) -> Result<Vec<RenderOutcome>> {
        let (w, h) = fp::resolution(&self.defn, q);
        let ground_z = self.defn.defaults.get("previz").and_then(|p| p.get("ground_z")).and_then(num).unwrap_or(0.0);
        let mut out = Vec::new();
        for (s, rfp) in todo {
            let t0 = Instant::now();
            let d = self.state.take_dir(&s.id, quality, rfp);
            if d.exists() {
                std::fs::remove_dir_all(&d)?; // forced re-render: drop old frames and cached segments
            }
            let frames = fp::frame_list(s, q);
            let files = previz::render_shot(s, &frames, &self.defn.anchor_boxes(), &d, w as u32, h as u32, ground_z, None)?;
            let dt = t0.elapsed().as_secs_f64();
            let info = self.take_info(s, q, &frames, &files, dt, "previz");
            self.state.write_take(&s.id, quality, rfp, info)?;
            let o = RenderOutcome {
                shot_id: s.id.clone(),
                quality: quality.into(),
                action: "rendered".into(),
                fingerprint: rfp.clone(),
                seconds: dt,
                detail: d.display().to_string(),
                thumb: files.get(files.len() / 2).map(|f| d.join(f)),
            };
            emit("shot", &outcome_event(&o));
            out.push(o);
        }
        Ok(out)
    }

    fn render_engine(&mut self, todo: &[(ResolvedShot, String)], quality: &str, q: &Map<String, Value>, force: bool)
        -> Result<Vec<RenderOutcome>> {
        let shots: Vec<ResolvedShot> = todo.iter().map(|(s, _)| s.clone()).collect();
        let build_items = self.build_items(&shots, false);
        let master = self.master_if_changed(false)?;
        let (w, h) = fp::resolution(&self.defn, q);
        let mut items = Vec::new();
        for (s, rfp) in todo {
            let d = self.state.take_dir(&s.id, quality, rfp);
            if d.exists() {
                std::fs::remove_dir_all(&d)?; // incomplete leftovers from a failed attempt
            }
            std::fs::create_dir_all(&d)?;
            items.push(json!({
                "key": format!("{}:{quality}", s.id), "id": s.id, "asset": shot_asset(&self.defn, &s.id),
                "quality_name": quality, "quality": q, "resolution": [w, h],
                "frames": fp::frame_list(s, q), "out_dir": d.display().to_string(),
            }));
        }
        let engine = make_engine(&self.defn)?;
        let t0 = Instant::now();
        let result = engine.render(&build_items, master.as_ref(), &items, force)?;
        let build_report = self.absorb_build(&build_items, &result["build"], master.as_ref());
        let total = t0.elapsed().as_secs_f64();
        let share = total / items.len().max(1) as f64;
        let mut out = Vec::new();
        for ((s, rfp), it) in todo.iter().zip(&items) {
            let key = it["key"].as_str().unwrap_or_default();
            let r = &result["renders"][key];
            let d = PathBuf::from(it["out_dir"].as_str().unwrap_or_default());
            let failed = |detail: String| RenderOutcome {
                shot_id: s.id.clone(),
                quality: quality.into(),
                action: "failed".into(),
                fingerprint: rfp.clone(),
                seconds: 0.0,
                detail,
                thumb: None,
            };
            if r["status"].as_str() != Some("ok") {
                let mut why = r["error"].as_str().unwrap_or("no result").to_string();
                if build_report.diverged.contains(&s.id) {
                    why = "sequence was edited in the editor; run `shotpilot pull` first (or build --force)".into();
                }
                out.push(failed(why));
                continue;
            }
            let frames: Vec<i64> = it["frames"].as_array().map(|a| a.iter().filter_map(Value::as_i64).collect()).unwrap_or_default();
            let files = match normalize_frames(&d, &frames) {
                Ok(f) => f,
                Err(e) => {
                    out.push(failed(e.msg));
                    continue;
                }
            };
            let info = self.take_info(s, q, &frames, &files, share, engine.name());
            self.state.write_take(&s.id, quality, rfp, info)?;
            out.push(RenderOutcome {
                shot_id: s.id.clone(),
                quality: quality.into(),
                action: "rendered".into(),
                fingerprint: rfp.clone(),
                seconds: share,
                detail: d.display().to_string(),
                thumb: files.get(files.len() / 2).map(|f| d.join(f)),
            });
        }
        Ok(out)
    }

    // ------------------------------------------------------------------ pull

    /// Bring Sequencer edits back into shots.yaml. prefer: None | "engine" | "yaml" for conflicts.
    pub fn pull(&mut self, prefer: Option<&str>, dry_run: bool) -> Result<Vec<PullDecision>> {
        let shots: Vec<ResolvedShot> =
            self.resolved()?.into_iter().filter(|s| self.state.pushed(&s.id).is_some()).collect();
        if shots.is_empty() {
            return Ok(Vec::new());
        }
        let engine = make_engine(&self.defn)?;
        let items: Vec<Value> = shots.iter().map(|s| json!({"id": s.id, "asset": shot_asset(&self.defn, &s.id)})).collect();
        let result = engine.pull(&items)?;
        let decisions: Vec<PullDecision> = shots
            .iter()
            .map(|s| {
                let base = self.state.pushed_payload(&s.id);
                sync::decide(&s.id, result["shots"].get(&s.id), base.as_ref(), s)
            })
            .collect();
        if dry_run {
            return Ok(decisions);
        }

        let take_engine: Vec<&PullDecision> = decisions
            .iter()
            .filter(|d| d.kind == PullKind::EngineEdit || (d.kind == PullKind::Conflict && prefer == Some("engine")))
            .collect();
        if !take_engine.is_empty() {
            let mut doc = Doc::load(&self.path)?;
            let mut edits = Vec::new();
            for d in &take_engine {
                let idx = self.defn.index_of(&d.shot_id)?;
                let ours = shots.iter().find(|s| s.id == d.shot_id).expect("pulled shot");
                let engine_v = d.engine.clone().unwrap_or(Value::Null);
                let (node, comment) = sync::apply_engine_to_shot(&self.defn.shots[idx], &engine_v, ours);
                let comments = comment.map(|c| vec![("camera".to_string(), c)]).unwrap_or_default();
                edits.push((idx, node, comments));
            }
            doc.replace_items("shots", &edits)?;
            model::parse(&doc.value()?, &self.path)?;
            doc.save(&self.path)?;
            self.reload()?;
        }

        for d in &decisions {
            let pulled = take_engine.iter().any(|t| t.shot_id == d.shot_id);
            if pulled || d.kind == PullKind::Agree {
                // yaml now resolves to what the engine holds: that is the new base
                let idx = self.defn.index_of(&d.shot_id)?;
                let s = model::resolve_shot(&self.defn, &self.defn.shots[idx], idx)?;
                self.state.set_pushed(&s.id, &fp::build_fingerprint(&self.defn, &s), s.engine_payload());
            } else if d.kind == PullKind::Conflict && prefer == Some("yaml") {
                if let (Some(e), Some(mut payload)) = (&d.engine, self.state.pushed_payload(&d.shot_id)) {
                    // accept the engine asset as the base so the next build overwrites it with yaml
                    if let (Some(p), Some(e)) = (payload.as_object_mut(), e.as_object()) {
                        p.extend(e.clone());
                    }
                    self.state.set_pushed(&d.shot_id, "superseded", payload);
                }
            }
        }

        let conflicts: Vec<&str> =
            decisions.iter().filter(|d| d.kind == PullKind::Conflict).map(|d| d.shot_id.as_str()).collect();
        if !take_engine.is_empty() || !conflicts.is_empty() {
            self.state.log("pull", json!({
                "pulled": take_engine.iter().map(|d| &d.shot_id).collect::<Vec<_>>(),
                "conflicts": conflicts, "prefer": prefer,
            }));
        }
        self.state.save()?;
        Ok(decisions)
    }

    // ------------------------------------------------------------------ scan

    pub fn scan(&mut self, tag: &str, prefix: &str) -> Result<ScanReport> {
        let engine = make_engine(&self.defn)?;
        let found = engine.scan(tag, prefix)?;
        let mut names: Vec<(String, Value)> =
            found["anchors"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        let mut report = ScanReport::default();
        let mut updates = Vec::new();
        for (name, found) in names {
            let Ok((at, b)) = model::parse_anchor(&found) else { continue };
            let r = |v: crate::geometry::Vec3| v.map(|x| round_to(x, 1));
            let (at, b) = (r(at), b.map(|b| model::AnchorBox { center: r(b.center), size: r(b.size) }));
            let new = match b {
                Some(b) if b.center == at => json!({"at": at, "size": b.size}),
                Some(b) => json!({"at": at, "size": b.size, "center": b.center}),
                None => json!(at),
            };
            match self.defn.anchor(&name) {
                None => {
                    report.added.push(name.clone());
                    updates.push((name, new));
                }
                // a size found in the level fills in; a hand-written one is kept when the level has none
                Some(old) if old != at || (b.is_some() && self.defn.known_box(&name) != b) => {
                    report.moved.push(name.clone());
                    updates.push((name, new));
                }
                Some(_) => report.same.push(name),
            }
        }
        if !updates.is_empty() {
            let mut doc = Doc::load(&self.path)?;
            doc.upsert_entries("anchors", &updates)?;
            model::parse(&doc.value()?, &self.path)?;
            doc.save(&self.path)?;
            self.reload()?;
            self.state.log("scan", json!({"added": report.added, "moved": report.moved}));
            self.state.save()?;
        }
        Ok(report)
    }

    // ------------------------------------------------------------------ cut / export

    /// Per shot in definition order: (shot, take, quality used). quality "best" mixes.
    pub fn pick_takes(&self, quality: &str) -> Result<Vec<TakePick>> {
        let mut out = Vec::new();
        for s in self.resolved()? {
            let ladder: Vec<&str> = if quality == "best" { QUALITY_RANK.to_vec() } else { vec![quality] };
            let mut chosen = (None, None);
            for qn in ladder {
                let rfp = fp::render_fingerprint(&self.defn, &s, qn)?;
                if let Some(t) = self.state.find_take(&s.id, qn, &rfp) {
                    chosen = (Some(t), Some(qn.to_string()));
                    break;
                }
            }
            out.push((s, chosen.0, chosen.1));
        }
        Ok(out)
    }

    pub fn cut(&mut self, quality: &str, video: bool) -> Result<CutInfo> {
        let picks = self.pick_takes(quality)?;
        let missing: Vec<&str> = picks.iter().filter(|(_, t, _)| t.is_none()).map(|(s, _, _)| s.id.as_str()).collect();
        if !missing.is_empty() {
            let hint = if quality == "best" { "previz" } else { quality };
            bail!("no fresh {quality} take for: {}. run `shotpilot render {hint}` first", missing.join(", "));
        }
        let picks: Vec<Pick> = picks
            .into_iter()
            .map(|(shot, take, q)| Pick { shot, take: take.expect("checked"), quality: q.expect("checked") })
            .collect();
        let out_dir = self.root.join("cut").join(quality);
        // sound never blocks the cut: a missing file or decoder is reported and the cut goes on silent
        let mut notes = Vec::new();
        let mixed = match self.mix() {
            Ok(m) => m,
            Err(e) => {
                notes.push(format!("sound skipped: {}", e.msg));
                None
            }
        };
        let mut info = cut::write_cut(&self.defn, &picks, &out_dir, video, mixed.as_ref())?;
        info.notes.extend(notes);
        let takes: Map<String, Value> = picks
            .iter()
            .map(|p| (p.shot.id.clone(), json!(format!("{}:{}", p.quality, &p.take.fingerprint()[..12.min(p.take.fingerprint().len())]))))
            .collect();
        self.state.log("cut", json!({
            "quality": quality, "order": picks.iter().map(|p| &p.shot.id).collect::<Vec<_>>(), "takes": takes,
            "sound": mixed.as_ref().map(|m| m.check["verdict"].clone()),
        }));
        self.state.save()?;
        Ok(info)
    }

    // ------------------------------------------------------------------ definition edits

    fn render_fps(&self) -> Result<HashMap<String, Vec<(String, String)>>> {
        let mut out = HashMap::new();
        for s in self.resolved()? {
            let mut v = Vec::new();
            for q in fp::quality_names(&self.defn) {
                let rfp = fp::render_fingerprint(&self.defn, &s, &q)?;
                v.push((q, rfp));
            }
            out.insert(s.id.clone(), v);
        }
        Ok(out)
    }

    /// Replace whole shots and/or reorder, then report which renders went stale.
    /// Untouched shots keep their lines (and comments) exactly as they are in the file.
    pub fn edit_definition(&mut self, edits: &[(String, Value)], order: Option<&[String]>, action: &str, note: &str)
        -> Result<EditReport> {
        self.edit_definition_with(edits, order, None, action, note)
    }

    /// [`Self::edit_definition`], optionally replacing the top-level `audio:` mapping too.
    pub fn edit_definition_with(&mut self, edits: &[(String, Value)], order: Option<&[String]>,
                                audio: Option<&Map<String, Value>>, action: &str, note: &str) -> Result<EditReport> {
        let before = self.render_fps()?;
        let audio_before = self.defn.audio.clone();
        let mut doc = Doc::load(&self.path)?;
        let current = doc.value()?;
        let ids: Vec<String> = current["shots"]
            .as_array()
            .map(|a| a.iter().map(|s| model::str_of(&s["id"])).collect())
            .unwrap_or_default();
        let mut replace = Vec::new();
        for (id, node) in edits {
            let Some(i) = ids.iter().position(|x| x == id) else {
                bail!("unknown shot '{id}'. known: {}", ids.join(", "));
            };
            let mut node = node.clone();
            if let Some(m) = node.as_object_mut() {
                m.insert("id".into(), json!(id));
            }
            replace.push((i, node, Vec::new()));
        }
        if !replace.is_empty() {
            doc.replace_items("shots", &replace)?;
        }
        if let Some(order) = order {
            let mut a = order.to_vec();
            let mut b = ids.clone();
            a.sort();
            b.sort();
            if a != b {
                bail!("order must list every shot exactly once: {}", ids.join(", "));
            }
            let perm: Vec<usize> = order.iter().map(|id| ids.iter().position(|x| x == id).expect("checked")).collect();
            doc.reorder_items("shots", &perm)?;
        }
        let mut audio_changed = false;
        if let Some(new) = audio.filter(|a| **a != audio_before) {
            // keys that went away are written as null: the line stays, the value is gone
            let mut entries: Vec<(String, Value)> = new.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            entries.extend(audio_before.keys().filter(|k| !new.contains_key(*k)).map(|k| (k.clone(), Value::Null)));
            doc.upsert_entries("audio", &entries)?;
            audio_changed = true;
        }
        // validate before writing anything
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;

        let mut stale = Vec::new();
        for s in self.resolved()? {
            if let Some(old) = before.get(&s.id) {
                for (q, f) in old {
                    if &fp::render_fingerprint(&self.defn, &s, q)? != f {
                        stale.push(s.id.clone());
                        break;
                    }
                }
            }
        }
        let mut edited: Vec<String> = edits.iter().map(|(id, _)| id.clone()).collect();
        edited.sort();
        let order_v: Vec<String> = order.map(<[String]>::to_vec).unwrap_or_default();
        let mut entry = json!({"edited": edited, "reordered": order.is_some(), "stale": stale, "note": note});
        if audio_changed {
            entry["audio"] = json!(true);
        }
        self.state.log(action, entry);
        self.state.save()?;
        Ok(EditReport { edited, stale, order: order_v, audio: audio_changed })
    }

    pub fn set_fields(&mut self, shot_id: &str, assignments: &[(String, Value)]) -> Result<EditReport> {
        let mut node = self.defn.find(shot_id)?.clone();
        for (path, value) in assignments {
            let parts: Vec<&str> = path.split('.').collect();
            let mut cur = &mut node;
            for p in &parts[..parts.len() - 1] {
                if !cur.get(*p).is_some_and(Value::is_object) {
                    cur[*p] = json!({});
                }
                cur = cur.get_mut(*p).expect("just inserted");
            }
            let last = parts[parts.len() - 1];
            let m = cur.as_object_mut().ok_or_else(|| Error::user(format!("cannot set {path}")))?;
            if value.is_null() {
                m.remove(last);
            } else {
                m.insert(last.into(), value.clone());
            }
        }
        // switching camera form: an explicit location drops the anchor form
        let sets_location = assignments.iter().any(|(k, v)| {
            k == "camera.location" || (k == "camera" && v.get("location").is_some())
        });
        if sets_location {
            if let Some(cam) = node.get_mut("camera").and_then(Value::as_object_mut) {
                cam.remove("anchor");
                cam.remove("offset");
            }
        }
        let note: Vec<&str> = assignments.iter().map(|(k, _)| k.as_str()).collect();
        self.edit_definition(&[(shot_id.to_string(), node)], None, "set", &note.join(", "))
    }

    // ------------------------------------------------------------------ approval

    /// Sign off (or withdraw) a shot's framing. Approval is tied to the shot's picture:
    /// any edit that changes pixels drops it, editorial edits keep it. A shot with an
    /// open P0 finding cannot be approved.
    pub fn approve(&mut self, shot_id: &str, approved: bool) -> Result<()> {
        let s = self.select_one(shot_id)?;
        if approved {
            let blocking: Vec<String> = self
                .check()?
                .into_iter()
                .filter(|f| f.severity == Severity::P0 && f.shot.as_deref().map_or(true, |x| x == s.id))
                .map(|f| f.line())
                .collect();
            if !blocking.is_empty() {
                bail!("cannot approve {} while P0 problems are open:
  - {}", s.id, blocking.join("
  - "));
            }
            let look = fp::look_fingerprint(&self.defn, &s);
            self.state.set_approved(&s.id, Some(&look));
        } else {
            self.state.set_approved(&s.id, None);
        }
        self.state.log("approve", json!({"shot": s.id, "approved": approved}));
        self.state.save()
    }

    // ------------------------------------------------------------------ brief

    /// Set fields under `brief:` (null removes the value). Editorial: nothing goes stale.
    pub fn set_brief(&mut self, entries: &[(String, Value)]) -> Result<()> {
        for (k, _) in entries {
            if !model::BRIEF_FIELDS.contains(&k.as_str()) {
                bail!("unknown brief field '{k}'. known: {}", model::BRIEF_FIELDS.join(", "));
            }
        }
        let mut doc = Doc::load(&self.path)?;
        doc.upsert_entries("brief", entries)?;
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        let keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
        self.state.log("brief", json!({"fields": keys}));
        self.state.save()
    }

    // ------------------------------------------------------------------ location scouting (stage 0)

    /// Ask the engine which meshes the project has, and keep the list (derived, not shots.yaml).
    pub fn scan_assets(&mut self) -> Result<Vec<crate::scout::AssetInfo>> {
        let engine = make_engine(&self.defn)?;
        let found = engine.assets()?;
        let mut list: Vec<crate::scout::AssetInfo> =
            found["assets"].as_array().into_iter().flatten().filter_map(crate::scout::AssetInfo::from_json).collect();
        list.sort_by(|a, b| a.asset.cmp(&b.asset));
        list.dedup_by(|a, b| a.asset == b.asset);
        crate::scout::save_assets(&self.root, &list)?;
        self.state.log("assets", json!({"count": list.len()}));
        self.state.save()?;
        Ok(list)
    }

    /// Put a mesh in `props:` so shots may make it appear. Kind and size come from the
    /// inventory when the asset is in it.
    pub fn add_prop(&mut self, name: &str, asset: &str, kind: Option<&str>, size: Option<crate::geometry::Vec3>)
        -> Result<()> {
        if !model::valid_id(name) {
            bail!("prop name: lowercase letters, digits, _ or - (got '{name}')");
        }
        let known = crate::scout::load_assets(&self.root).into_iter().find(|a| a.asset == asset);
        let kind = kind.map(str::to_string).or_else(|| known.as_ref().map(|a| a.kind.clone())).unwrap_or_else(|| "static".into());
        let mut entry = Map::new();
        entry.insert("asset".into(), json!(asset));
        entry.insert("kind".into(), json!(kind));
        if let Some(sz) = size.or_else(|| known.and_then(|a| a.size)) {
            entry.insert("size".into(), json!(sz.map(|v| round_to(v, 1))));
        }
        let mut doc = Doc::load(&self.path)?;
        doc.upsert_entries("props", &[(name.to_string(), Value::Object(entry))])?;
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        self.state.log("prop", json!({"added": name, "asset": asset}));
        self.state.save()
    }

    /// Take a prop out of `props:` (refused while a shot still makes it appear).
    pub fn remove_prop(&mut self, name: &str) -> Result<()> {
        if self.defn.prop(name).is_none() {
            bail!("unknown prop '{name}'");
        }
        let users: Vec<String> = self.resolved()?.iter().filter(|s| s.appear.iter().any(|a| a.prop == name)).map(|s| s.id.clone()).collect();
        if !users.is_empty() {
            bail!("prop '{name}' still appears in: {}", users.join(", "));
        }
        let mut doc = Doc::load(&self.path)?;
        doc.upsert_entries("props", &[(name.to_string(), Value::Null)])?;
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        self.state.log("prop", json!({"removed": name}));
        self.state.save()
    }

    /// Photograph anchors from four sides (all anchors when `only` is empty) into scout/.
    /// previz for the mock engine; one still per camera in the real engine. Scout cameras are
    /// not shots: nothing here touches shots.yaml or the take state.
    pub fn take_scout(&mut self, only: &[String]) -> Result<Value> {
        let cams = crate::scout::cameras(&self.defn, only);
        if cams.is_empty() {
            bail!("no anchors to scout. add `anchors:` (or run `shotpilot scan`) first");
        }
        let dir = self.root.join(crate::scout::SCOUT_DIR);
        std::fs::create_dir_all(&dir)?;
        let fps = self.defn.teaser.fps;
        let (w, h) = (960u32, 540u32);
        let mut shots = Vec::new();
        for (i, c) in cams.iter().enumerate() {
            shots.push(model::resolve_shot(&self.defn, &c.shot(fps), i)?);
        }
        let mut files: Vec<Option<String>> = vec![None; cams.len()];
        if self.defn.project.engine == "ue5" {
            let raw = dir.join(".raw");
            let q = fp::quality(&self.defn, "still")?;
            let mut build = Vec::new();
            let mut items = Vec::new();
            for s in &shots {
                let asset = format!("{}/Scout/SEQ_{}", self.defn.project.content_root, s.id);
                build.push(json!({"id": s.id, "asset": asset, "fingerprint": fp::build_fingerprint(&self.defn, s),
                                  "payload": s.engine_payload(), "base": Value::Null}));
                let d = raw.join(&s.id);
                if d.exists() {
                    std::fs::remove_dir_all(&d)?;
                }
                std::fs::create_dir_all(&d)?;
                items.push(json!({"key": format!("{}:scout", s.id), "id": s.id, "asset": asset, "quality_name": "still",
                                  "quality": q, "resolution": [w, h], "frames": [0], "out_dir": d.display().to_string()}));
            }
            let engine = make_engine(&self.defn)?;
            let result = engine.render(&build, None, &items, true)?;
            for (i, it) in items.iter().enumerate() {
                let key = it["key"].as_str().unwrap_or_default();
                if result["renders"][key]["status"].as_str() != Some("ok") {
                    continue;
                }
                let d = PathBuf::from(it["out_dir"].as_str().unwrap_or_default());
                if let Some(f) = normalize_frames(&d, &[0]).ok().and_then(|v| v.into_iter().next()) {
                    let ext = Path::new(&f).extension().and_then(|e| e.to_str()).unwrap_or("png").to_string();
                    let name = format!("{}-{}.{ext}", cams[i].anchor, cams[i].side);
                    std::fs::copy(d.join(&f), dir.join(&name))?;
                    files[i] = Some(name);
                }
            }
            let _ = std::fs::remove_dir_all(&raw);
        } else {
            let boxes = self.defn.anchor_boxes();
            let ground_z = self.defn.defaults.get("previz").and_then(|p| p.get("ground_z")).and_then(num).unwrap_or(0.0);
            for (i, s) in shots.iter().enumerate() {
                let name = format!("{}-{}.png", cams[i].anchor, cams[i].side);
                std::fs::write(dir.join(&name), previz::render_frame(s, 0, &boxes, w, h, ground_z, None).png())?;
                files[i] = Some(name);
            }
        }
        let mut idx = crate::scout::load_index(&self.root);
        if !idx["anchors"].is_object() {
            idx["anchors"] = json!({});
        }
        let mut taken = 0;
        for (c, f) in cams.iter().zip(&files) {
            let list = idx["anchors"].as_object_mut().expect("object").entry(c.anchor.clone()).or_insert_with(|| json!([]));
            let arr = list.as_array_mut().expect("array");
            arr.retain(|v| v["side"] != c.side.as_str());
            if let Some(f) = f {
                arr.push(json!({"side": c.side, "file": format!("{}/{f}", crate::scout::SCOUT_DIR), "location": c.location}));
                taken += 1;
            }
        }
        // anchors that no longer exist drop out of the index
        let live: Vec<String> = self.defn.anchors.iter().map(|(n, _)| n.clone()).collect();
        if let Some(m) = idx["anchors"].as_object_mut() {
            m.retain(|k, _| live.contains(k));
        }
        idx["taken"] = json!(chrono::Local::now().to_rfc3339());
        crate::scout::save_index(&self.root, &idx)?;
        self.state.log("scout", json!({"photos": taken, "cameras": cams.len()}));
        self.state.save()?;
        if taken == 0 {
            bail!("no scout photo came back from the engine (see .shotpilot/logs)");
        }
        Ok(idx)
    }

    /// Write `stage:` (the set as read from the scout photos). Editorial: nothing re-renders.
    pub fn set_stage(&mut self, stage: &model::StageCfg) -> Result<()> {
        let to_map = |v: &[(String, String)]| Value::Object(v.iter().map(|(k, d)| (k.clone(), json!(d))).collect());
        let old: Vec<String> = self.defn.stage.anchors.iter().map(|(n, _)| n.clone()).collect();
        let mut anchors = to_map(&stage.anchors);
        // anchors that were described before but not now are dropped (null keeps the line, loses the value)
        if let Some(m) = anchors.as_object_mut() {
            for n in old {
                m.entry(n).or_insert(Value::Null);
            }
        }
        let mut doc = Doc::load(&self.path)?;
        doc.upsert_entries("stage", &[
            ("summary".to_string(), json!(stage.summary)),
            ("anchors".to_string(), anchors),
            ("mood".to_string(), to_map(&stage.mood)),
        ])?;
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        self.state.log("stage", json!({"anchors": stage.anchors.len()}));
        self.state.save()
    }

    // ------------------------------------------------------------------ mood board

    fn set_images(&mut self, images: &[String]) -> Result<()> {
        self.set_brief(&[("images".to_string(), json!(images))])
    }

    /// Generate mood-board frames (place, detail, threat, title; `n` of them) and add them to
    /// `brief.images`. A failed frame stops the run; the ones already made are kept.
    pub fn generate_moodboard(&mut self, gen: &dyn crate::moodboard::ImageGen, direction: &str, n: usize)
        -> Result<Vec<String>> {
        let frames: Vec<(String, crate::moodboard::ImageJob)> =
            crate::moodboard::jobs(&self.defn, direction).into_iter().take(n.max(1)).collect();
        let jobs: Vec<crate::moodboard::ImageJob> = frames.iter().map(|(_, j)| j.clone()).collect();
        let mut made = Vec::new();
        let mut err = None;
        match gen.generate_many(&jobs) {
            Ok(images) => {
                for (i, bytes) in images.iter().enumerate() {
                    let key = frames.get(i).map(|(k, _)| k.as_str()).unwrap_or("extra");
                    made.push(crate::moodboard::store(&self.root, &format!("gen-{key}"), bytes)?);
                }
            }
            Err(e) => err = Some(e),
        }
        if !made.is_empty() {
            let mut all = self.defn.brief.images.clone();
            all.extend(made.iter().cloned());
            self.set_images(&all)?;
            self.state.log("moodboard", json!({"added": made, "direction": direction}));
            self.state.save()?;
        }
        match err {
            Some(e) if made.is_empty() => Err(e),
            _ => Ok(made),
        }
    }

    /// Add an image of the developer's own (a screenshot, concept art) to the mood board.
    pub fn add_moodboard_image(&mut self, name: &str, bytes: &[u8]) -> Result<String> {
        let rel = crate::moodboard::store(&self.root, name, bytes)?;
        let mut all = self.defn.brief.images.clone();
        all.push(rel.clone());
        self.set_images(&all)?;
        Ok(rel)
    }

    /// Take an image off the mood board (and delete its file from moodboard/).
    pub fn remove_moodboard_image(&mut self, rel: &str) -> Result<()> {
        if !self.defn.brief.images.iter().any(|i| i == rel) {
            bail!("'{rel}' is not on the mood board");
        }
        let all: Vec<String> = self.defn.brief.images.iter().filter(|i| *i != rel).cloned().collect();
        self.set_images(&all)?;
        if let Some(p) = crate::moodboard::resolve(&self.root, rel) {
            let _ = std::fs::remove_file(p);
        }
        Ok(())
    }

    // ------------------------------------------------------------------ sound

    /// The teaser's mix (cached under cut/sound/), or None when nothing is set to play.
    /// Mixing never touches a take: sound is not part of any fingerprint.
    pub fn mix(&self) -> Result<Option<Mixed>> {
        sound::mix_cached(&self.defn, &self.root, &self.resolved()?)
    }

    /// Edit sound only: replace the `sfx:` list of some shots (None or an empty list
    /// removes it) and/or the whole `audio:` mapping. Logged as "sound"; shots only change
    /// in their `sfx`, so nothing goes stale.
    pub fn edit_sound(&mut self, sfx: &[(String, Option<Value>)], audio: Option<&Map<String, Value>>, note: &str)
        -> Result<EditReport> {
        let mut edits = Vec::new();
        let mut changes = Map::new();
        for (id, list) in sfx {
            let old = self.defn.find(id)?.clone();
            let mut node = old.clone();
            let m = node.as_object_mut().ok_or_else(|| Error::user(format!("shot '{id}' is not a mapping")))?;
            match list {
                Some(Value::Array(a)) if !a.is_empty() => {
                    m.insert("sfx".into(), Value::Array(a.clone()));
                }
                None | Some(Value::Null) | Some(Value::Array(_)) => {
                    m.remove("sfx");
                }
                Some(_) => bail!("sfx for '{id}' must be a list"),
            }
            if node != old {
                changes.insert(id.clone(), json!(field_changes(&old, &node)));
                edits.push((id.clone(), node));
            }
        }
        if let Some(a) = audio {
            let c = field_changes(&Value::Object(self.defn.audio.clone()), &Value::Object(a.clone()));
            if !c.is_empty() {
                changes.insert("audio".into(), json!(c));
            }
        }
        if edits.is_empty() && !changes.contains_key("audio") {
            return Ok(EditReport::default()); // nothing to write, nothing to log
        }
        let rep = self.edit_definition_with(&edits, None, audio, "sound", note)?;
        let mut extra = Map::new();
        extra.insert("changes".into(), Value::Object(changes));
        self.state.annotate_last(extra);
        self.state.save()?;
        Ok(rep)
    }

    /// Set fields under `audio:` by dotted path (`gain`, `bgm.file`, `bgm` = a mapping;
    /// null removes).
    pub fn set_audio(&mut self, assignments: &[(String, Value)]) -> Result<EditReport> {
        let mut node = Value::Object(self.defn.audio.clone());
        for (path, value) in assignments {
            let path = path.strip_prefix("audio.").unwrap_or(path);
            let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
            let Some((last, head)) = parts.split_last() else { bail!("empty audio field") };
            let mut cur = &mut node;
            for p in head {
                if !cur.get(*p).is_some_and(Value::is_object) {
                    cur[*p] = json!({});
                }
                cur = cur.get_mut(*p).expect("just inserted");
            }
            let m = cur.as_object_mut().ok_or_else(|| Error::user(format!("cannot set audio.{path}")))?;
            if value.is_null() {
                m.remove(*last);
            } else {
                m.insert(last.to_string(), value.clone());
            }
        }
        let note: Vec<&str> = assignments.iter().map(|(k, _)| k.as_str()).collect();
        let audio = node.as_object().cloned().unwrap_or_default();
        self.edit_sound(&[], Some(&audio), &note.join(", "))
    }

    // ------------------------------------------------------------------ gc

    /// Delete old takes, keeping the current one plus `keep` most recent per shot/quality.
    pub fn gc(&mut self, keep: usize, dry_run: bool) -> Result<Vec<PathBuf>> {
        let mut removed = Vec::new();
        let live: Vec<String> = self.defn.shot_ids();
        let Ok(rd) = std::fs::read_dir(&self.state.render_dir) else { return Ok(removed) };
        for shot_dir in rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            let shot = shot_dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            for qdir in std::fs::read_dir(&shot_dir)?.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
                let q = qdir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let cur = self.state.current(&shot, &q);
                let old: Vec<Take> =
                    self.state.list_takes(&shot, &q).into_iter().filter(|t| Some(t.fingerprint()) != cur.as_deref()).collect();
                let victims: &[Take] = if !live.contains(&shot) {
                    &old
                } else {
                    &old[..old.len().saturating_sub(keep)]
                };
                for t in victims {
                    removed.push(t.dir.clone());
                    if !dry_run {
                        std::fs::remove_dir_all(&t.dir)?;
                    }
                }
            }
        }
        if !removed.is_empty() && !dry_run {
            self.state.log("gc", json!({"removed": removed.len()}));
            self.state.save()?;
        }
        Ok(removed)
    }

    /// Write a fresh shot list (from `plan`), keeping the rest of the file as it is.
    pub fn replace_all_shots(&mut self, shots: &[Value]) -> Result<()> {
        let mut doc = Doc::load(&self.path)?;
        doc.set_items("shots", shots)?;
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        self.state.log("plan", json!({"shots": shots.iter().map(|s| &s["id"]).collect::<Vec<_>>()}));
        self.state.save()
    }
}

/// Leaf-level differences between two nodes, as dotted paths. Lists are leaves, except
/// lists of mappings of the same length (`sfx`, `markers`), which diff per item: `sfx[0].at`.
pub fn field_changes(before: &Value, after: &Value) -> Vec<Value> {
    fn walk(path: &str, a: Option<&Value>, b: Option<&Value>, out: &mut Vec<Value>) {
        match (a, b) {
            (Some(Value::Object(x)), Some(Value::Object(y))) => {
                let mut keys: Vec<&String> = x.keys().collect();
                keys.extend(y.keys().filter(|k| !x.contains_key(*k)));
                for k in keys {
                    let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                    walk(&p, x.get(k), y.get(k), out);
                }
            }
            (Some(Value::Array(x)), Some(Value::Array(y)))
                if x.len() == y.len() && x.iter().chain(y).all(Value::is_object) =>
            {
                for (i, (u, v)) in x.iter().zip(y).enumerate() {
                    walk(&format!("{path}[{i}]"), Some(u), Some(v), out);
                }
            }
            _ if a == b => {}
            _ => out.push(json!({"path": path, "before": a, "after": b})),
        }
    }
    let mut out = Vec::new();
    walk("", Some(before), Some(after), &mut out);
    out
}

fn outcome_event(o: &RenderOutcome) -> Value {
    let mut v = o.to_json();
    v["thumb_path"] = json!(o.thumb.as_ref().map(|p| p.display().to_string()));
    v
}

/// Engines name files their own way; rename to frame.NNNN.ext matching the planned frame list.
fn normalize_frames(d: &Path, frames: &[i64]) -> Result<Vec<String>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(d)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).is_some_and(|e| IMAGE_EXTS.contains(&e.as_str()))
        })
        .collect();
    files.sort();
    if files.len() != frames.len() {
        bail!("expected {} frames in {}, found {}", frames.len(), d.display(), files.len());
    }
    let mut tmp = Vec::new();
    for (p, f) in files.iter().zip(frames) {
        let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let t = d.join(format!("__tmp_{f:04}.{ext}"));
        std::fs::rename(p, &t)?;
        tmp.push((t, *f, ext));
    }
    let mut names = Vec::new();
    for (t, f, ext) in tmp {
        let name = format!("frame.{f:04}.{ext}");
        std::fs::rename(&t, d.join(&name))?;
        names.push(name);
    }
    Ok(names)
}
