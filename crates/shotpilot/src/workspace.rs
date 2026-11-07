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
/// Script line fields the app edits in place (the text and its timing; never where it sits).
pub const LINE_FIELDS: &[&str] = &["ja", "en", "head", "from", "dur", "at"];
/// One-tap shot changes worked out in code ([`Workspace::nudge`]).
pub const NUDGES: &[&str] = &[
    "closer", "farther", "wider", "tighter", "shorter", "longer", "lower", "higher", "orbit_left", "orbit_right",
    "aim_up", "aim_down", "move_longer", "move_shorter", "tod_earlier", "tod_later", "brighter", "darker",
];
/// How big a nudge is: small / medium (the default) / large, as a multiplier on the step.
pub const NUDGE_AMOUNTS: &[&str] = &["s", "m", "l"];
/// Lens steps for `wider` / `tighter` (mm).
pub const LENS_STEPS: &[f64] = &[12.0, 14.0, 18.0, 24.0, 28.0, 35.0, 50.0, 65.0, 85.0, 105.0, 135.0, 200.0];

/// (shot, its take if any, the quality that take is in).
pub type TakePick = (ResolvedShot, Option<Take>, Option<String>);

/// Progress callback: ("plan", {...}) once, then ("shot", outcome) per shot.
pub type Progress<'a> = &'a mut dyn FnMut(&str, &Value);

/// What happened to a shot's storyboard frame.
#[derive(Debug, Clone)]
pub struct StoryboardOutcome {
    pub shot_id: String,
    /// "made" or "kept" (its frame already matched the shot's picture).
    pub action: String,
    pub file: PathBuf,
}

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
    /// Qualities whose take went out of date because the game changed under it, with the
    /// packages that changed (content.rs).
    pub changed: Vec<(String, Vec<String>)>,
}

impl ShotStatus {
    pub fn quality(&self, q: &str) -> &str {
        self.qualities.iter().find(|(k, _)| k == q).map(|(_, v)| v.as_str()).unwrap_or("none")
    }

    pub fn to_json(&self) -> Value {
        let qs: Map<String, Value> = self.qualities.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        let changed: Map<String, Value> = self.changed.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        json!({
            "index": self.index, "id": self.id, "duration": self.duration, "purpose": self.purpose,
            "beat": self.beat, "build": self.build, "qualities": qs, "approval": self.approval, "changed": changed,
        })
    }
}

/// One quality of a shot as the app shows it (see [`Workspace::status_detail`]).
#[derive(Debug, Clone)]
pub struct QualityTake {
    pub quality: String,
    /// The render fingerprint the shot has now at this quality.
    pub fingerprint: String,
    /// `take` matches `fingerprint`.
    pub fresh: bool,
    pub take: Option<Take>,
    /// The game content that changed under this definition's take (empty when fresh).
    pub changed: Vec<String>,
}

/// What there is of a shot's definition at one quality (see [`Workspace::current_take`]).
#[derive(Debug, Clone)]
pub enum TakeMatch {
    /// A take of this definition whose game content is unchanged.
    Fresh(Take),
    /// Only takes whose game content changed since they were rendered: the newest one, and
    /// the packages that changed under it.
    GameChanged(Take, Vec<String>),
    None,
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
    /// The anchors were picked among the level's landmarks (nothing was tagged).
    pub auto: bool,
    /// Who picked them: "claude" (by the plan) or "spread" (the fallback); empty for tags.
    pub picked_by: String,
    /// Why the fallback was used, when Claude failed.
    pub note: Option<String>,
}

/// How many places the location scout picks when nothing in the level is tagged.
pub const SCOUT_PICKS: usize = 6;

/// What [`Workspace::match_concept`] did.
#[derive(Debug, Default, Clone)]
pub struct MatchReport {
    /// Comparison rounds run.
    pub rounds: usize,
    /// Candidates photographed as new anchors to find a place for a frame.
    pub added: Vec<String>,
    /// Frames with a place.
    pub found: usize,
    /// Frames with a picture and still no place.
    pub open: Vec<String>,
}

/// The pictures still looking for a place, as text for the place pick.
fn concept_wants(frames: &[crate::concept::Frame]) -> String {
    frames
        .iter()
        .filter(|f| f.found.is_none())
        .map(|f| format!("- `{}` ({}): {}", f.id, f.beat, f.look))
        .collect::<Vec<_>>()
        .join("\n")
}

/// What [`Workspace::scout`] gathered before taking the photos.
#[derive(Debug, Default)]
pub struct ScoutReport {
    /// Anchors brought in from the level (None: shots.yaml already had anchors).
    pub scan: Option<ScanReport>,
    /// Meshes inventoried in the same launch (None: the inventory was already there).
    pub assets: Option<usize>,
    /// scout/index.json after the photos.
    pub index: Value,
    /// The level's landmark candidates, when the survey listed them.
    pub candidates: Vec<crate::scout::Candidate>,
    /// The concept frames' search, when frames were waiting for a place (D-46).
    pub matched: Option<MatchReport>,
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
        self.check_resolved(&self.resolved()?)
    }

    /// [`Self::check`] for shots already resolved. Every caller (the CLI, the app's status,
    /// produce) goes through here, so what the look after the shoot found shows everywhere.
    pub fn check_resolved(&self, shots: &[ResolvedShot]) -> Result<Vec<Finding>> {
        let mut found = shotcheck::check(&self.defn, shots);
        let seen = self.seen_findings(shots)?;
        if seen.is_empty() {
            return Ok(found);
        }
        // the look's findings go in with the rest, most severe first (a stable sort keeps the order within)
        found.extend(seen);
        found.sort_by_key(|f| (f.severity, f.shot.as_ref().and_then(|id| shots.iter().position(|s| &s.id == id))));
        Ok(found)
    }

    // ------------------------------------------------------------------ after the shoot (D-50)

    /// The fresh take of a shot at a quality, if there is one.
    fn fresh_take(&self, s: &ResolvedShot, quality: &str) -> Result<Option<Take>> {
        let rfp = fp::render_fingerprint(&self.defn, s, quality)?;
        Ok(match self.current_take(&s.id, quality, &rfp) {
            TakeMatch::Fresh(t) => Some(t),
            _ => None,
        })
    }

    /// Look once at the middle frame of each shot's fresh take at `quality` (empty `ids`: every
    /// shot) and record what it shows, tied to that take: a new take of the shot drops it.
    pub fn see(&mut self, quality: &str, ids: &[String], claude: &dyn crate::ai::Claude) -> Result<Vec<crate::ai::Seen>> {
        let shots = self.select(ids)?;
        let mut frames = Vec::new();
        let mut takes = HashMap::new();
        for s in &shots {
            let Some(t) = self.fresh_take(s, quality)? else { continue };
            let Some(thumb) = t.thumb() else { continue };
            let subject = shotcheck::subject_name(&self.defn, s).unwrap_or_else(|| "the shot's subject".into());
            frames.push((s.id.clone(), subject, s.purpose.clone(), thumb));
            takes.insert(s.id.clone(), t.dir.to_string_lossy().replace('\\', "/"));
        }
        let seen = crate::ai::see_frames(&frames, claude)?;
        for x in &seen {
            let Some(take) = takes.get(&x.shot) else { continue };
            let rec = json!({"quality": quality, "take": take, "visible": x.visible, "blocked_by": x.blocked_by, "note": x.note});
            self.state.set_seen(&x.shot, rec);
        }
        self.state.save()?;
        Ok(seen)
    }

    /// What the look after the shoot found, for shots whose recorded take is still the one in use.
    fn seen_findings(&self, shots: &[ResolvedShot]) -> Result<Vec<Finding>> {
        let mut out = Vec::new();
        for s in shots {
            let Some(rec) = self.state.seen(&s.id) else { continue };
            let visible = rec["visible"].as_str().unwrap_or("yes");
            if visible == "yes" {
                continue;
            }
            let quality = rec["quality"].as_str().unwrap_or("");
            let Some(t) = self.fresh_take(s, quality)? else { continue };
            if t.dir.to_string_lossy().replace('\\', "/") != rec["take"].as_str().unwrap_or("") {
                continue; // filmed again since
            }
            let subject = shotcheck::subject_name(&self.defn, s).unwrap_or_else(|| "the subject".into());
            let by = rec["blocked_by"].as_str().filter(|b| !b.is_empty()).map(|b| format!(" ({b})")).unwrap_or_default();
            let what = if visible == "no" { "is not in the rendered frame" } else { "is only partly in the rendered frame" };
            out.push(Finding {
                rule: "frame.blocked",
                severity: Severity::P1,
                shot: Some(s.id.clone()),
                message: format!("`{subject}` {what}{by}"),
                fix: "Move the camera so nothing stands between it and the subject (point at the camera and ask Claude).".into(),
            });
        }
        Ok(out)
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

    /// The take to use for this definition: the newest one whose game content is unchanged
    /// (a take that recorded no content, such as previz, always is).
    pub fn current_take(&self, shot_id: &str, quality: &str, fp: &str) -> TakeMatch {
        let mut first_changed = None;
        for t in self.state.takes_for(shot_id, quality, fp) {
            let gone = crate::content::changed(&self.root, &t);
            if gone.is_empty() {
                return TakeMatch::Fresh(t);
            }
            first_changed.get_or_insert((t, gone));
        }
        match first_changed {
            Some((t, gone)) => TakeMatch::GameChanged(t, gone),
            None => TakeMatch::None,
        }
    }

    pub fn status(&self) -> Result<Vec<ShotStatus>> {
        Ok(self.status_detail(&self.resolved()?, false)?.into_iter().map(|(row, _)| row).collect())
    }

    /// [`Self::status`] for these shots, with each quality's render fingerprint and the take
    /// to show: the fresh one, else (with `latest`) the newest stale one. Without `latest`
    /// stale takes are only counted, not read.
    pub fn status_detail(&self, shots: &[ResolvedShot], latest: bool) -> Result<Vec<(ShotStatus, Vec<QualityTake>)>> {
        let names = fp::quality_names(&self.defn);
        let mut rows = Vec::new();
        for s in shots {
            let bfp = fp::build_fingerprint(&self.defn, s);
            let build = match self.state.pushed_fp(&s.id) {
                None => "new",
                Some(p) if p == bfp => "synced",
                Some(_) => "dirty",
            };
            let mut qualities = Vec::new();
            let mut takes = Vec::new();
            let mut changed = Vec::new();
            for q in &names {
                let rfp = fp::render_fingerprint(&self.defn, s, q)?;
                let mut gone = Vec::new();
                let (st, take) = match self.current_take(&s.id, q, &rfp) {
                    TakeMatch::Fresh(t) => ("fresh", Some(t)),
                    TakeMatch::GameChanged(t, pkgs) => {
                        gone = pkgs;
                        ("stale", latest.then_some(t))
                    }
                    TakeMatch::None if latest => match self.state.latest_take(&s.id, q) {
                        Some(t) => ("stale", Some(t)),
                        None => ("none", None),
                    },
                    TakeMatch::None if self.state.has_takes(&s.id, q) => ("stale", None),
                    TakeMatch::None => ("none", None),
                };
                if !gone.is_empty() {
                    changed.push((q.clone(), gone.clone()));
                }
                qualities.push((q.clone(), st.to_string()));
                takes.push(QualityTake { quality: q.clone(), fingerprint: rfp, fresh: st == "fresh", take, changed: gone });
            }
            let approval = match self.state.approved_fp(&s.id) {
                None => "none",
                Some(a) if a == fp::look_fingerprint(&self.defn, s) => "approved",
                Some(_) => "outdated",
            };
            let row = ShotStatus {
                approval: approval.into(),
                index: s.index,
                id: s.id.clone(),
                duration: s.duration,
                purpose: s.purpose.clone(),
                beat: s.beat.clone(),
                build: build.into(),
                qualities,
                changed,
            };
            rows.push((row, takes));
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
            let mut it = json!({
                "id": s.id, "asset": shot_asset(&self.defn, &s.id), "fingerprint": bfp,
                "payload": s.engine_payload(),
                "base": self.state.pushed_payload(&s.id).map(|p| readback_subset(&p)),
            });
            let lead = crate::engines::streaming_lead(&self.defn);
            if lead > 0 {
                it["lead"] = json!(lead);
            }
            items.push(it);
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
        let mut game_changed = Map::new();
        for s in &shots {
            let rfp = fp::render_fingerprint(&self.defn, s, quality)?;
            let found = if force { TakeMatch::None } else { self.current_take(&s.id, quality, &rfp) };
            match found {
                TakeMatch::Fresh(t) => {
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
                TakeMatch::GameChanged(_, gone) => {
                    game_changed.insert(s.id.clone(), json!(gone));
                    todo.push((s.clone(), rfp));
                }
                TakeMatch::None => todo.push((s.clone(), rfp)),
            }
        }

        emit("plan", &json!({
            "quality": quality,
            "render": todo.iter().map(|(s, _)| &s.id).collect::<Vec<_>>(),
            "reused": outcomes.iter().map(|o| &o.shot_id).collect::<Vec<_>>(),
            "game_changed": game_changed,
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
            let mut entry = json!({
                "quality": quality, "rendered": by("rendered"), "reused": by("reused"), "failed": by("failed"),
            });
            if !game_changed.is_empty() {
                entry["game_changed"] = Value::Object(game_changed.clone());
            }
            self.state.log("render", entry);
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
            // a forced take replaces this definition's take; otherwise a finished take (whose
            // game content changed since) stays, and the new one goes next to it
            let d = if force { self.state.take_dir(&s.id, quality, rfp) } else { self.state.new_take_dir(&s.id, quality, rfp) };
            if d.exists() {
                std::fs::remove_dir_all(&d)?; // incomplete leftovers from a failed attempt, or a forced re-take
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
            let mut info = self.take_info(s, q, &frames, &files, share, engine.name());
            // what was on screen, so a later change to the game re-renders only this shot
            if let Some(reported) = r["deps"].as_array() {
                info.insert("deps".into(), json!(crate::content::record(&self.root, reported)));
            }
            if let Some(e) = r["deps_error"].as_str() {
                info.insert("deps_error".into(), json!(e));
            }
            self.state.write_take_at(&d, &s.id, quality, rfp, info)?;
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
        self.apply_scan(&found)
    }

    /// Write the anchors an engine found into shots.yaml (new ones added, moved ones updated).
    fn apply_scan(&mut self, found: &Value) -> Result<ScanReport> {
        let mut names: Vec<(String, Value)> =
            found["anchors"].as_object().map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
        names.sort_by(|a, b| a.0.cmp(&b.0));
        let mut report = ScanReport { auto: found["auto"].as_bool().unwrap_or(false), ..Default::default() };
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
            self.state.log("scan", json!({"added": report.added, "moved": report.moved, "auto": report.auto}));
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
                if let TakeMatch::Fresh(t) = self.current_take(&s.id, qn, &rfp) {
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
        self.edit_definition_full(edits, &[], order, audio, action, note)
    }

    /// [`Self::edit_definition_with`], also setting the `sfx` of script lines (`line_sfx`:
    /// line id -> a list, or null for none). The rest of each line stays as written.
    pub fn edit_definition_full(&mut self, edits: &[(String, Value)], line_sfx: &[(String, Value)],
                                order: Option<&[String]>, audio: Option<&Map<String, Value>>, action: &str,
                                note: &str) -> Result<EditReport> {
        let rev = crate::ai::Revision {
            edits: edits.to_vec(),
            order: order.map(<[String]>::to_vec),
            audio: audio.cloned(),
            lines: line_sfx.to_vec(),
            texts: Vec::new(),
            finish: Vec::new(),
            summary: String::new(),
        };
        self.apply_revision(&rev, action, note)
    }

    /// Write what a note changed ([`crate::ai::Revision`]) as one edit that can be undone:
    /// whole shots, the sound and the words of script lines (the rest of each line stays as
    /// written), the order, `audio:` and `finish:` entries.
    pub fn apply_revision(&mut self, rev: &crate::ai::Revision, action: &str, note: &str) -> Result<EditReport> {
        let touched: Vec<&String> = rev.lines.iter().map(|(id, _)| id).chain(rev.texts.iter().map(|(id, _)| id)).collect();
        let lines = if touched.is_empty() {
            Vec::new()
        } else {
            Doc::load(&self.path)?.value()?["script"].as_array().cloned().unwrap_or_default()
        };
        let mut line_nodes: Vec<(String, Value)> = Vec::new();
        for id in &touched {
            if line_nodes.iter().any(|(k, _)| k == *id) {
                continue;
            }
            let Some(cur) = lines.iter().find(|l| model::str_of(&l["id"]) == **id) else {
                bail!("unknown script line '{id}'");
            };
            let mut node = cur.clone();
            let Some(m) = node.as_object_mut() else { bail!("script line '{id}' is not a mapping") };
            for (_, list) in rev.lines.iter().filter(|(k, _)| k == *id) {
                match list {
                    Value::Array(a) if !a.is_empty() => {
                        m.insert("sfx".into(), list.clone());
                    }
                    Value::Null | Value::Array(_) => {
                        m.remove("sfx");
                    }
                    _ => bail!("sfx for script line '{id}' must be a list"),
                }
            }
            for (_, set) in rev.texts.iter().filter(|(k, _)| k == *id) {
                for (k, v) in set {
                    if !crate::ai::LINE_TEXT_FIELDS.contains(&k.as_str()) {
                        bail!("script line field '{k}' cannot be set here (one of {})", crate::ai::LINE_TEXT_FIELDS.join(", "));
                    }
                    if v.is_null() || v.as_str() == Some("") {
                        m.remove(k);
                    } else {
                        m.insert(k.clone(), v.clone());
                    }
                }
            }
            line_nodes.push(((*id).clone(), node));
        }
        let rep = self.write_definition(&rev.edits, &line_nodes, rev.order.as_deref(), rev.audio.as_ref(), &rev.finish,
                                        action, note)?;
        if !line_nodes.is_empty() {
            let mut extra = Map::new();
            extra.insert("lines".into(), json!(line_nodes.iter().map(|(id, _)| id).collect::<Vec<_>>()));
            self.state.annotate_last(extra);
            self.state.save()?;
        }
        Ok(rep)
    }

    /// The one write path for edits: whole shots, whole script lines, the shot order,
    /// `audio:` and `finish:` entries, checked as a whole before anything is written. The log
    /// entry keeps each changed piece as it was before and after (`undo`), so the edit can be
    /// taken back.
    #[allow(clippy::too_many_arguments)]
    fn write_definition(&mut self, edits: &[(String, Value)], line_nodes: &[(String, Value)],
                        order: Option<&[String]>, audio: Option<&Map<String, Value>>, finish: &[(String, Value)],
                        action: &str, note: &str) -> Result<EditReport> {
        let before = self.render_fps()?;
        let audio_before = self.defn.audio.clone();
        let mut doc = Doc::load(&self.path)?;
        let current = doc.value()?;
        let finish_before = current.get("finish").cloned().unwrap_or(Value::Null);
        let ids: Vec<String> = current["shots"]
            .as_array()
            .map(|a| a.iter().map(|s| model::str_of(&s["id"])).collect())
            .unwrap_or_default();
        let mut undo_shots = Map::new();
        let mut replace = Vec::new();
        for (id, node) in edits {
            let Some(i) = ids.iter().position(|x| x == id) else {
                bail!("unknown shot '{id}'. known: {}", ids.join(", "));
            };
            let mut node = node.clone();
            if let Some(m) = node.as_object_mut() {
                m.insert("id".into(), json!(id));
            }
            undo_shots.insert(id.clone(), current["shots"][i].clone());
            replace.push((i, node, Vec::new()));
        }
        if !replace.is_empty() {
            doc.replace_items("shots", &replace)?;
        }
        let mut undo_lines = Map::new();
        if !line_nodes.is_empty() {
            let lines = current["script"].as_array().cloned().unwrap_or_default();
            let mut replace = Vec::new();
            for (id, node) in line_nodes {
                let Some(i) = lines.iter().position(|l| model::str_of(&l["id"]) == *id) else {
                    bail!("unknown script line '{id}'");
                };
                let mut node = node.clone();
                let Some(m) = node.as_object_mut() else { bail!("script line '{id}' is not a mapping") };
                m.insert("id".into(), json!(id));
                undo_lines.insert(id.clone(), lines[i].clone());
                replace.push((i, node, Vec::new()));
            }
            doc.replace_items("script", &replace)?;
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
        if !finish.is_empty() {
            doc.upsert_entries("finish", finish)?;
        }
        // validate before writing anything
        let written = doc.value()?;
        model::parse(&written, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        let finish_changed = written.get("finish").cloned().unwrap_or(Value::Null) != finish_before;

        // each changed piece as it is now, next to what it was, so the edit can be undone
        let find = |list: &str, id: &str| -> Value {
            written[list].as_array().into_iter().flatten().find(|x| model::str_of(&x["id"]) == id).cloned().unwrap_or(Value::Null)
        };
        let pair = |b: Value, a: Value| json!({"before": b, "after": a});
        let mut undo = Map::new();
        if !undo_shots.is_empty() {
            let m: Map<String, Value> = undo_shots.into_iter().map(|(id, b)| { let a = find("shots", &id); (id, pair(b, a)) }).collect();
            undo.insert("shots".into(), Value::Object(m));
        }
        if !undo_lines.is_empty() {
            let m: Map<String, Value> = undo_lines.into_iter().map(|(id, b)| { let a = find("script", &id); (id, pair(b, a)) }).collect();
            undo.insert("lines".into(), Value::Object(m));
        }
        if order.is_some() {
            undo.insert("order".into(), pair(json!(ids), json!(self.defn.shot_ids())));
        }
        if audio_changed {
            undo.insert("audio".into(), pair(Value::Object(audio_before), written["audio"].clone()));
        }
        if finish_changed {
            undo.insert("finish".into(), pair(finish_before, written["finish"].clone()));
        }

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
        if finish_changed {
            entry["finish"] = json!(true);
        }
        if action != "undo" {
            entry["undo"] = Value::Object(undo);
        }
        self.state.log(action, entry);
        self.state.save()?;
        Ok(EditReport { edited, stale, order: order_v, audio: audio_changed })
    }

    // ------------------------------------------------------------------ script lines, nudges, undo

    /// Set text and timing fields of one script line (see [`LINE_FIELDS`]; null or an empty
    /// string removes the field). The script is editorial: nothing re-renders.
    pub fn set_line(&mut self, line_id: &str, assignments: &[(String, Value)]) -> Result<EditReport> {
        let lines = Doc::load(&self.path)?.value()?["script"].as_array().cloned().unwrap_or_default();
        let Some(cur) = lines.iter().find(|l| model::str_of(&l["id"]) == line_id) else {
            bail!("unknown script line '{line_id}'");
        };
        let mut node = cur.clone();
        let Some(m) = node.as_object_mut() else { bail!("script line '{line_id}' is not a mapping") };
        for (k, v) in assignments {
            if !LINE_FIELDS.contains(&k.as_str()) {
                bail!("script line field '{k}' cannot be set here (one of {})", LINE_FIELDS.join(", "));
            }
            let v = match v {
                Value::String(s) => json!(s.trim()),
                v => v.clone(),
            };
            if v.is_null() || v.as_str() == Some("") {
                m.remove(k);
            } else {
                m.insert(k.clone(), v);
            }
        }
        if &node == cur {
            return Ok(EditReport::default());
        }
        let changes = field_changes(cur, &node);
        let note: Vec<&str> = assignments.iter().map(|(k, _)| k.as_str()).collect();
        let rep = self.write_definition(&[], &[(line_id.to_string(), node)], None, None, &[], "script", &note.join(", "))?;
        let mut extra = Map::new();
        extra.insert("line".into(), json!(line_id));
        extra.insert("changes".into(), json!({ line_id: changes }));
        self.state.annotate_last(extra);
        self.state.save()?;
        Ok(rep)
    }

    /// A one-tap change to a shot, worked out in code (no Claude), at the medium amount.
    pub fn nudge(&mut self, shot_id: &str, kind: &str) -> Result<EditReport> {
        self.nudge_by(shot_id, kind, "m")
    }

    /// A one-tap change to a shot, worked out in code (no Claude). `amount` is `s` / `m` / `l`
    /// (x0.5 / x1 / x2 of the step). `closer` / `farther` move the camera (and its whole path)
    /// toward or away from what it looks at, `lower` / `higher` change its height, `orbit_left` /
    /// `orbit_right` swing it around what it looks at, `wider` / `tighter` step the lens,
    /// `aim_up` / `aim_down` step where on the subject it looks, `move_longer` / `move_shorter`
    /// scale the camera move, `tod_earlier` / `tod_later` step the time of day, `shorter` /
    /// `longer` change the length by half a second, and `brighter` / `darker` shift the
    /// brightness at the cut (editorial: nothing re-renders).
    pub fn nudge_by(&mut self, shot_id: &str, kind: &str, amount: &str) -> Result<EditReport> {
        use crate::geometry as g;
        if !NUDGES.contains(&kind) {
            bail!("nudge: one of {}", NUDGES.join(", "));
        }
        let mult = match amount {
            "s" => 0.5,
            "m" => 1.0,
            "l" => 2.0,
            _ => bail!("nudge amount: one of {}", NUDGE_AMOUNTS.join(", ")),
        };
        // discrete axes (lens, aim, time of day) move one step, or two for a large nudge
        let steps: i64 = if mult > 1.0 { 2 } else { 1 };
        let s = self.select_one(shot_id)?;
        let old = self.defn.find(&s.id)?.clone();
        let mut node = old.clone();
        let round = |v: g::Vec3| json!(v.map(f64::round));
        // Move the camera (its start, and the end of its path if it has one). `f` gets the
        // stored position and whether it is an offset from an anchor.
        let shift = |node: &mut Value, id: &str, f: &dyn Fn(g::Vec3, bool) -> g::Vec3| -> Result<()> {
            let Some(cam) = node.get_mut("camera").and_then(Value::as_object_mut) else {
                bail!("{id} has no camera mapping");
            };
            if cam.contains_key("anchor") {
                let off = cam.get("offset").and_then(model::vec3).unwrap_or([-600.0, 0.0, 150.0]);
                cam.insert("offset".into(), round(f(off, true)));
            } else if let Some(loc) = cam.get("location").and_then(model::vec3) {
                cam.insert("location".into(), round(f(loc, false)));
            }
            if let Some(end) = cam.get_mut("end").and_then(Value::as_object_mut) {
                if let Some(l) = end.get("location").and_then(model::vec3) {
                    end.insert("location".into(), round(f(l, false)));
                }
            }
            Ok(())
        };
        match kind {
            "closer" | "farther" => {
                let k = if kind == "closer" { 1.0 - 0.2 * mult } else { 1.0 + 0.25 * mult };
                let Some(target) = s.target else {
                    bail!("{} is aimed by rotation, not at a point; ask Claude to move it instead", s.id);
                };
                let d = g::sub(s.start.location, target);
                let dist = g::length(d);
                if dist * k < 30.0 {
                    bail!("{} is already {:.0} cm from what it looks at", s.id, dist);
                }
                let delta = g::scale(d, k - 1.0);
                shift(&mut node, &s.id, &|v, _| g::add(v, delta))?;
                // a dolly keeps its share of the distance, so it does not run through the subject
                if let Some(mv) = node.pointer_mut("/camera/move").and_then(Value::as_object_mut) {
                    let dolly = mv.get("type").and_then(Value::as_str).is_some_and(|t| t.starts_with("dolly"));
                    if let (true, Some(dd)) = (dolly, mv.get("distance").and_then(num)) {
                        mv.insert("distance".into(), json!((dd * k).round()));
                    }
                }
            }
            "lower" | "higher" => {
                let dz = (if kind == "higher" { 25.0 } else { -25.0 }) * mult;
                shift(&mut node, &s.id, &|v, _| g::add(v, [0.0, 0.0, dz]))?;
            }
            "orbit_left" | "orbit_right" => {
                let Some(target) = s.target else {
                    bail!("{} is aimed by rotation, not at a point; ask Claude to move it instead", s.id);
                };
                let deg = 15.0 * mult * if kind == "orbit_left" { 1.0 } else { -1.0 };
                let eye = s.start.location;
                // an anchored camera stores an offset: move it by the swing of the world position
                let delta = g::sub(g::orbit(eye, target, deg), eye);
                shift(&mut node, &s.id, &|v, rel| if rel { g::add(v, delta) } else { g::orbit(v, target, deg) })?;
            }
            "wider" | "tighter" => {
                let f = s.focal_length();
                let mut cur = f;
                for _ in 0..steps {
                    let next = if kind == "wider" {
                        LENS_STEPS.iter().rev().find(|x| **x < cur - 0.5)
                    } else {
                        LENS_STEPS.iter().find(|x| **x > cur + 0.5)
                    };
                    match next {
                        Some(n) => cur = *n,
                        None => break,
                    }
                }
                if cur == f {
                    bail!("{} is already at {}mm", s.id, model::fmt_g(f));
                }
                let Some(cam) = node.get_mut("camera").and_then(Value::as_object_mut) else {
                    bail!("{} has no camera mapping", s.id);
                };
                if !cam.get("lens").is_some_and(Value::is_object) {
                    cam.insert("lens".into(), json!({}));
                }
                cam["lens"]["focal_length"] = json!(cur);
            }
            "aim_up" | "aim_down" => {
                if s.target.is_none() {
                    bail!("{} is aimed by rotation, not at a point; ask Claude to move it instead", s.id);
                }
                let cur = node.pointer("/camera/aim").and_then(Value::as_str).unwrap_or("base").to_string();
                let at = model::AIMS.iter().position(|a| *a == cur).unwrap_or(0) as i64;
                let to = (at + if kind == "aim_up" { steps } else { -steps }).clamp(0, model::AIMS.len() as i64 - 1);
                if to == at {
                    bail!("{} already looks at its {} point", s.id, cur);
                }
                let Some(cam) = node.get_mut("camera").and_then(Value::as_object_mut) else {
                    bail!("{} has no camera mapping", s.id);
                };
                cam.insert("aim".into(), json!(model::AIMS[to as usize]));
            }
            "move_longer" | "move_shorter" => {
                let Some(mv) = node.pointer_mut("/camera/move").and_then(Value::as_object_mut) else {
                    bail!("{} has no camera move", s.id);
                };
                let ty = mv.get("type").and_then(Value::as_str).unwrap_or("static").to_string();
                if ty == "static" {
                    bail!("{} has no camera move", s.id);
                }
                // an orbit is sized in degrees, the other moves in cm
                let (key, dflt, lo, hi) =
                    if ty.starts_with("orbit") { ("degrees", 30.0, 5.0, 360.0) } else { ("distance", 150.0, 10.0, 3000.0) };
                let k = if kind == "move_longer" { 1.0 + 0.25 * mult } else { 1.0 - 0.2 * mult };
                let n = (mv.get(key).and_then(num).unwrap_or(dflt) * k).round();
                if !(lo..=hi).contains(&n) {
                    bail!("{} would move {} {}; a move stays within {}-{}", s.id, n, key, lo, hi);
                }
                mv.insert(key.into(), json!(n));
            }
            "tod_earlier" | "tod_later" => {
                let Some(cur) = s.environment.get("time_of_day").and_then(Value::as_str).map(str::to_string) else {
                    bail!("{} has no time of day; set one first", s.id);
                };
                let at = model::TIME_OF_DAY.iter().position(|(n, _)| *n == cur).unwrap_or(0) as i64;
                let to = (at + if kind == "tod_later" { steps } else { -steps }).clamp(0, model::TIME_OF_DAY.len() as i64 - 1);
                if to == at {
                    bail!("{} is already at {}", s.id, cur);
                }
                if !node.get("environment").is_some_and(Value::is_object) {
                    node["environment"] = json!({});
                }
                let Some(env) = node["environment"].as_object_mut() else { bail!("{} has no environment mapping", s.id) };
                env.insert("time_of_day".into(), json!(model::TIME_OF_DAY[to as usize].0));
                // an explicit sun height would hide the change
                env.remove("sun_elevation");
            }
            "shorter" | "longer" => {
                let grid = if mult < 1.0 { 4.0 } else { 2.0 };
                let step = 0.5 * mult;
                let d = round_to(((s.duration + if kind == "longer" { step } else { -step }) * grid).round() / grid, 2);
                if !(1.0..=8.0).contains(&d) {
                    bail!("{} would be {}s; a shot stays within 1-8 s", s.id, model::fmt_g(d));
                }
                node["duration"] = json!(d);
            }
            _ => {
                // brighter / darker: the shot's brightness shift at the cut (-0.5..0.5)
                let cur = node.get("exposure").and_then(num).unwrap_or(0.0);
                let step = 0.25 * mult * if kind == "brighter" { 1.0 } else { -1.0 };
                let to = ((cur + step).clamp(-0.5, 0.5) * 1000.0).round() / 1000.0;
                if to == cur {
                    bail!("{} is already at the {} limit", s.id, if kind == "brighter" { "brightest" } else { "darkest" });
                }
                let Some(m) = node.as_object_mut() else { bail!("{} is not a mapping", s.id) };
                if to == 0.0 {
                    m.remove("exposure");
                } else {
                    m.insert("exposure".into(), json!(to));
                }
            }
        }
        let changes = field_changes(&old, &node);
        let rep = self.edit_definition(&[(s.id.clone(), node)], None, "nudge", kind)?;
        let mut extra = Map::new();
        extra.insert("shot".into(), json!(s.id));
        extra.insert("kind".into(), json!(kind));
        extra.insert("amount".into(), json!(amount));
        extra.insert("changes".into(), json!({ s.id.clone(): changes }));
        self.state.annotate_last(extra);
        self.state.save()?;
        Ok(rep)
    }

    /// The newest edit that can still be taken back: (its index in the history, the entry).
    /// A fresh plan rewrites everything, so nothing before one can be undone.
    pub fn undo_target(&self) -> Option<(usize, Value)> {
        let hist = self.state.history();
        let undone: Vec<u64> = hist.iter().filter(|e| e["action"] == "undo").filter_map(|e| e["of"].as_u64()).collect();
        for (i, e) in hist.iter().enumerate().rev() {
            let action = e["action"].as_str().unwrap_or("");
            if action == "plan" || action == "produce" {
                return None;
            }
            if action != "undo" && e.get("undo").is_some_and(Value::is_object) && !undone.contains(&(i as u64)) {
                return Some((i, e.clone()));
            }
        }
        None
    }

    /// Take back the newest edit: its shots, script lines, order, `audio:` and `finish:` go back to
    /// what they were. Takes are kept by fingerprint, so a shot put back reuses the take it
    /// had (nothing re-renders). Refuses when a piece changed again since (a hand edit, the
    /// CLI), so nothing written later is lost.
    pub fn undo(&mut self) -> Result<(EditReport, Value)> {
        let Some((idx, entry)) = self.undo_target() else { bail!("nothing to undo") };
        let u = &entry["undo"];
        let now = Doc::load(&self.path)?.value()?;
        let find = |list: &str, id: &str| -> Value {
            now[list].as_array().into_iter().flatten().find(|x| model::str_of(&x["id"]) == id).cloned().unwrap_or(Value::Null)
        };
        let mut shots = Vec::new();
        for (id, p) in u["shots"].as_object().into_iter().flatten() {
            if find("shots", id) != p["after"] {
                bail!("{id} changed again after that edit; nothing undone");
            }
            shots.push((id.clone(), p["before"].clone()));
        }
        let mut lines = Vec::new();
        for (id, p) in u["lines"].as_object().into_iter().flatten() {
            if find("script", id) != p["after"] {
                bail!("script line {id} changed again after that edit; nothing undone");
            }
            lines.push((id.clone(), p["before"].clone()));
        }
        let order: Option<Vec<String>> = match u.get("order") {
            Some(p) => {
                if json!(self.defn.shot_ids()) != p["after"] {
                    bail!("the shot order changed again after that edit; nothing undone");
                }
                Some(p["before"].as_array().into_iter().flatten().map(model::str_of).collect())
            }
            None => None,
        };
        let audio: Option<Map<String, Value>> = match u.get("audio") {
            Some(p) => {
                if now["audio"] != p["after"] {
                    bail!("the audio changed again after that edit; nothing undone");
                }
                Some(p["before"].as_object().cloned().unwrap_or_default())
            }
            None => None,
        };
        // `finish:` goes back key by key; a key the edit added is written as null
        let finish: Vec<(String, Value)> = match u.get("finish") {
            Some(p) => {
                if now["finish"] != p["after"] {
                    bail!("the finishing (finish:) changed again after that edit; nothing undone");
                }
                let before = p["before"].as_object().cloned().unwrap_or_default();
                let added = p["after"].as_object().into_iter().flatten().filter(|(k, _)| !before.contains_key(*k));
                let gone: Vec<(String, Value)> = added.map(|(k, _)| (k.clone(), Value::Null)).collect();
                before.into_iter().chain(gone).collect()
            }
            None => Vec::new(),
        };
        let what = entry["note"].as_str().unwrap_or("").to_string();
        let action = entry["action"].as_str().unwrap_or("").to_string();
        let line_ids: Vec<String> = lines.iter().map(|(id, _)| id.clone()).collect();
        let rep = self.write_definition(&shots, &lines, order.as_deref(), audio.as_ref(), &finish, "undo", &what)?;
        let mut extra = Map::new();
        extra.insert("of".into(), json!(idx));
        extra.insert("undid".into(), json!(action));
        extra.insert("lines".into(), json!(line_ids));
        self.state.annotate_last(extra);
        self.state.save()?;
        Ok((rep, json!({"action": action, "note": what, "lines": line_ids})))
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

    /// Sign off (or withdraw) the script: its text and the cut it is the spine of (D-38).
    /// A change to a line, a shot's purpose, beat or length, or the order drops it; cameras,
    /// light and sound do not.
    pub fn approve_script(&mut self, approved: bool) -> Result<()> {
        if approved && self.defn.shots.is_empty() {
            bail!("there is no script to approve yet");
        }
        let fp = fp::script_fingerprint(&self.defn);
        self.state.set_approved_script(approved.then_some(fp.as_str()));
        self.state.log("approve-script", json!({"approved": approved}));
        self.state.save()
    }

    /// approved | outdated (approved, then the script or the cut changed) | none
    pub fn script_approval(&self) -> &'static str {
        match self.state.approved_script() {
            None => "none",
            Some(a) if a == fp::script_fingerprint(&self.defn) => "approved",
            Some(_) => "outdated",
        }
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

    /// Stage 0 from one button: gather what is missing, then photograph. With no anchors yet,
    /// one engine launch brings them in from the level (tagged actors, else landmarks the
    /// engine picks) together with the mesh inventory when there is none; then the photos.
    pub fn scout(&mut self, only: &[String], claude: Option<&dyn crate::ai::Claude>) -> Result<ScoutReport> {
        let mut report = ScoutReport::default();
        if self.defn.anchors.is_empty() {
            let want_assets = crate::scout::load_assets(&self.root).is_empty();
            let engine = make_engine(&self.defn)?;
            let mut found = engine.survey("shotpilot", "SP_", want_assets, false)?;
            if let Some(list) = found["assets"].as_array() {
                let mut list: Vec<crate::scout::AssetInfo> = list.iter().filter_map(crate::scout::AssetInfo::from_json).collect();
                list.sort_by(|a, b| a.asset.cmp(&b.asset));
                list.dedup_by(|a, b| a.asset == b.asset);
                crate::scout::save_assets(&self.root, &list)?;
                self.state.log("assets", json!({"count": list.len()}));
                self.state.save()?;
                report.assets = Some(list.len());
            }
            // nothing tagged: pick among the landmarks (Claude, by the plan; else spread out)
            let mut picked_by = String::new();
            let mut note = None;
            if !found["anchors"].as_object().is_some_and(|m| !m.is_empty()) {
                let cands: Vec<crate::scout::Candidate> =
                    found["candidates"].as_array().into_iter().flatten().filter_map(crate::scout::Candidate::from_json).collect();
                // the concept frames, when made, say what the places should look like (D-46)
                let want = concept_wants(&crate::concept::load(&self.root));
                let by_claude = match claude.filter(|_| !cands.is_empty()) {
                    Some(c) => match crate::ai::pick_locations(&self.defn, &cands, SCOUT_PICKS, Some(&want), c) {
                        Ok(p) if !p.is_empty() => Some(p),
                        Ok(_) => None,
                        Err(e) => {
                            note = Some(format!("Claude could not pick the places ({}); picked the most spread-out ones", e.msg));
                            None
                        }
                    },
                    None => None,
                };
                picked_by = if by_claude.is_some() { "claude" } else { "spread" }.to_string();
                let names = by_claude.unwrap_or_else(|| {
                    crate::scout::spread(&cands, SCOUT_PICKS).into_iter().map(|c| c.name).collect()
                });
                let picks: Map<String, Value> = names
                    .iter()
                    .filter_map(|n| cands.iter().find(|c| &c.name == n))
                    .map(|c| (c.name.clone(), c.entry()))
                    .collect();
                found["anchors"] = Value::Object(picks);
                found["auto"] = json!(true);
                report.candidates = cands;
            }
            let mut rep = self.apply_scan(&found)?;
            rep.picked_by = picked_by;
            rep.note = note;
            if self.defn.anchors.is_empty() {
                bail!(
                    "found nothing in the level to scout: tag actors `shotpilot` (or name them SP_...) in UE, or write \
                     `anchors:` in shots.yaml. A level built at runtime has nothing in the editor to find"
                );
            }
            report.scan = Some(rep);
        }
        report.index = self.take_scout(only)?;
        // the concept frames look for their places among the photos, and further afield (D-46)
        if let Some(c) = claude.filter(|_| crate::concept::load(&self.root).iter().any(crate::concept::Frame::open)) {
            let cands = std::mem::take(&mut report.candidates);
            report.matched = Some(self.match_concept(c, Some(cands))?);
            report.index = crate::scout::load_index(&self.root);
        }
        Ok(report)
    }

    /// Write a planning turn's plan (D-45): the brief fields, the concept frames it described
    /// (D-46; unchanged ones keep their picture and place), and the log entry the chat shows.
    pub fn apply_discovery(&mut self, d: &crate::ai::Discovery, note: &str, answers: &Value) -> Result<Value> {
        if !d.is_question() {
            self.set_brief(&d.brief)?;
            if !d.frames.is_empty() {
                crate::concept::replace(&self.root, &d.frames)?;
            }
        }
        let mut entry = d.to_json();
        entry["note"] = json!(note);
        entry["answers"] = answers.clone();
        self.state.log("discover", entry.clone());
        self.state.save()?;
        Ok(entry)
    }

    /// Make the concept frames' pictures (D-46): one per frame that has none. Only when the
    /// human asks; the pictures are references in concept/, never takes.
    pub fn generate_concept(&mut self, gen: &dyn crate::moodboard::ImageGen) -> Result<Vec<String>> {
        let mut frames = crate::concept::load(&self.root);
        if frames.is_empty() {
            bail!("no concept frames yet: the plan describes them (shotpilot discover, or 企画 in the app)");
        }
        let todo: Vec<usize> = (0..frames.len()).filter(|&i| frames[i].file.is_none()).collect();
        if todo.is_empty() {
            return Ok(Vec::new());
        }
        let jobs: Vec<crate::moodboard::ImageJob> = todo.iter().map(|&i| crate::concept::job(&self.defn, &frames[i])).collect();
        let images = gen.generate_many(&jobs)?;
        let mut made = Vec::new();
        for (k, bytes) in images.iter().enumerate() {
            let i = todo[k];
            let rel = crate::concept::store(&self.root, &frames[i], bytes)?;
            frames[i].file = Some(rel.clone());
            frames[i].found = None;
            frames[i].tried.clear();
            made.push(rel);
        }
        crate::concept::save(&self.root, &frames)?;
        self.state.log("concept", json!({"made": made, "frames": frames.iter().map(crate::concept::Frame::to_json).collect::<Vec<_>>()}));
        self.state.save()?;
        Ok(made)
    }

    /// Find the concept frames' places (D-46): compare the frames still open with the scout
    /// photos; for the ones nothing fits, photograph other candidates of the level (ones the
    /// plan's pictures suggest) and compare again, up to `concept::MATCH_ROUNDS` times.
    /// `cands`: the level's candidates when already known (else one survey asks for them).
    pub fn match_concept(&mut self, claude: &dyn crate::ai::Claude, cands: Option<Vec<crate::scout::Candidate>>) -> Result<MatchReport> {
        use crate::concept;
        let mut frames = concept::load(&self.root);
        let mut rep = MatchReport::default();
        let mut cands = cands.filter(|c| !c.is_empty());
        let mut fresh: Option<Vec<String>> = None; // anchors added in the last round (None: compare with all)
        for round in 0..concept::MATCH_ROUNDS {
            let open: Vec<usize> = (0..frames.len()).filter(|&i| frames[i].open()).collect();
            if open.is_empty() {
                break;
            }
            let idx = crate::scout::load_index(&self.root);
            let mut photos = Vec::new();
            for (anchor, views) in idx["anchors"].as_object().into_iter().flatten() {
                if self.defn.anchor(anchor).is_none() || fresh.as_ref().is_some_and(|f| !f.contains(anchor)) {
                    continue;
                }
                for v in views.as_array().into_iter().flatten() {
                    if let (Some(side), Some(file)) = (v["side"].as_str(), v["file"].as_str()) {
                        photos.push((anchor.clone(), side.to_string(), self.root.join(file)));
                    }
                }
            }
            let asked: Vec<(String, String, String, PathBuf)> = open
                .iter()
                .map(|&i| {
                    let f = &frames[i];
                    (f.id.clone(), f.beat.clone(), f.look.clone(), self.root.join(f.file.as_deref().unwrap_or_default()))
                })
                .collect();
            let seen: Vec<String> = photos.iter().map(|(a, _, _)| a.clone()).fold(Vec::new(), |mut v, a| {
                if !v.contains(&a) {
                    v.push(a);
                }
                v
            });
            for m in crate::ai::match_frames(&self.defn, &asked, &photos, claude)? {
                let Some(f) = frames.iter_mut().find(|f| f.id == m.frame) else { continue };
                match m.photo {
                    Some((anchor, side)) => {
                        let file = idx["anchors"][&anchor]
                            .as_array()
                            .and_then(|a| a.iter().find(|v| v["side"] == side.as_str()))
                            .and_then(|v| v["file"].as_str())
                            .unwrap_or_default()
                            .to_string();
                        f.found = Some(concept::Found { anchor, side, file, fit: m.fit, why: m.why });
                    }
                    None => {
                        for a in &seen {
                            if !f.tried.contains(a) {
                                f.tried.push(a.clone());
                            }
                        }
                    }
                }
            }
            rep.rounds = round + 1;
            concept::save(&self.root, &frames)?;
            let open: Vec<&concept::Frame> = frames.iter().filter(|f| f.open()).collect();
            if open.is_empty() || round + 1 == concept::MATCH_ROUNDS {
                break;
            }
            // other places for the frames nothing fits: the level's candidates the plan's pictures suggest
            if cands.is_none() {
                let engine = make_engine(&self.defn)?;
                let found = engine.survey("shotpilot", "SP_", false, true)?;
                cands = Some(found["candidates"].as_array().into_iter().flatten().filter_map(crate::scout::Candidate::from_json).collect());
            }
            let pool: Vec<crate::scout::Candidate> = cands
                .as_ref()
                .map(|c| c.iter().filter(|c| self.defn.anchor(&c.name).is_none()).cloned().collect())
                .unwrap_or_default();
            if pool.is_empty() {
                break;
            }
            let want = concept_wants(&frames);
            let picks = crate::ai::pick_locations(&self.defn, &pool, concept::PICKS_PER_ROUND, Some(&want), claude)?;
            if picks.is_empty() {
                break;
            }
            let entries: Map<String, Value> =
                picks.iter().filter_map(|n| pool.iter().find(|c| &c.name == n)).map(|c| (c.name.clone(), c.entry())).collect();
            let added = self.apply_scan(&json!({"anchors": entries, "auto": true}))?.added;
            if added.is_empty() {
                break;
            }
            self.take_scout(&added)?;
            rep.added.extend(added.iter().cloned());
            fresh = Some(added);
        }
        rep.found = frames.iter().filter(|f| f.found.is_some()).count();
        rep.open = frames.iter().filter(|f| f.open()).map(|f| f.id.clone()).collect();
        self.state.log("concept-match", json!({"rounds": rep.rounds, "added": rep.added, "found": rep.found, "open": rep.open,
                                               "frames": frames.iter().map(concept::Frame::to_json).collect::<Vec<_>>()}));
        self.state.save()?;
        Ok(rep)
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
        // where each camera really stood: the engine pulls a camera in front of a wall it would
        // otherwise see through (`clear_view`), and says so
        let mut stood: Vec<(crate::geometry::Vec3, f64)> = cams.iter().map(|c| (c.location, c.focal_length)).collect();
        // an engine that draws (UE, three) photographs; mock draws the previz lines
        if self.defn.project.engine != "mock" {
            let raw = dir.join(".raw");
            let q = fp::quality(&self.defn, "still")?;
            let mut build = Vec::new();
            let mut items = Vec::new();
            for (s, c) in shots.iter().zip(&cams) {
                let asset = format!("{}/Scout/SEQ_{}", self.defn.project.content_root, s.id);
                let mut b = json!({"id": s.id, "asset": asset, "fingerprint": fp::build_fingerprint(&self.defn, s),
                                   "payload": s.engine_payload(), "base": Value::Null,
                                   "clear_view": {"target": c.look_at}});
                let lead = crate::engines::streaming_lead(&self.defn);
                if lead > 0 {
                    b["lead"] = json!(lead);
                }
                build.push(b);
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
            for (i, s) in shots.iter().enumerate() {
                if let Some(m) = crate::scout::moved_camera(&result["build"]["shots"][&s.id]) {
                    stood[i] = m;
                }
            }
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
        for ((c, f), (loc, focal)) in cams.iter().zip(&files).zip(&stood) {
            let list = idx["anchors"].as_object_mut().expect("object").entry(c.anchor.clone()).or_insert_with(|| json!([]));
            let arr = list.as_array_mut().expect("array");
            arr.retain(|v| v["side"] != c.side.as_str());
            if let Some(f) = f {
                arr.push(json!({"side": c.side, "file": format!("{}/{f}", crate::scout::SCOUT_DIR), "location": loc,
                                "focal_length": focal}));
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

    /// Set fields under `finish:` (letterbox, title, end_card, font, fonts, logo, ...; null
    /// clears one). Editorial: it is burned in at the next cut, nothing re-renders.
    pub fn set_finish(&mut self, entries: &[(String, Value)]) -> Result<()> {
        const KEYS: &[&str] = &["letterbox", "title", "end_card", "font", "grade", "typeface", "fonts", "title_size", "logo", "motion"];
        for (k, _) in entries {
            if !KEYS.contains(&k.as_str()) {
                bail!("unknown finish field '{k}'. known: {}", KEYS.join(", "));
            }
        }
        self.check_faces(entries)?;
        let mut doc = Doc::load(&self.path)?;
        doc.upsert_entries("finish", entries)?;
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        let keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
        self.state.log("finish", json!({"fields": keys}));
        self.state.save()
    }

    /// The faces a `fonts` write changes must be fit for their role (D-52, D-59): not
    /// overused, kanji for the cards and lines, and for the title a title face that has the
    /// glyphs of the title it will set. Faces left as they are are not looked at again.
    fn check_faces(&self, entries: &[(String, Value)]) -> Result<()> {
        let Some((_, Value::Object(fonts))) = entries.iter().find(|(k, _)| k == "fonts") else { return Ok(()) };
        let cur = &self.defn.finish.fonts;
        let slots = [("display", &cur.display), ("body", &cur.body), ("latin", &cur.latin), ("title", &cur.title)];
        for (role, now) in slots {
            let Some(Value::String(s)) = fonts.get(role) else { continue };
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            let Ok(spec) = crate::fonts::Spec::parse(s) else { continue }; // the parse below says why
            if now.as_ref() == Some(&spec) {
                continue;
            }
            if let Some(o) = crate::fonts::overused(&spec.family) {
                bail!("finish.fonts.{role}: {} is overused (instead: {})", o.family, o.instead.join(", "));
            }
            if role == "title" {
                let tt = entries.iter().find(|(k, _)| k == "title").map(|(_, v)| v.clone());
                let typed = match &tt {
                    Some(Value::Object(m)) => m.get("text").map(model::str_of),
                    Some(Value::Null) => None,
                    _ => self.defn.finish.title.as_ref().map(|t| t.text.clone()),
                };
                let titles: Vec<&str> = self.defn.script.iter().filter(|l| l.kind == "title").map(|l| l.ja.as_str()).chain(typed.as_deref()).collect();
                crate::fonts::title_spec(&spec.family, &titles).map_err(|e| Error::user(format!("finish.fonts.title: {e}")))?;
            } else if role != "latin" {
                if let Some(f) = crate::fonts::family(&spec.family) {
                    if f.kanji == "none" {
                        bail!("finish.fonts.{role}: {} has no kanji for the cards and lines", f.family);
                    }
                }
            }
        }
        Ok(())
    }

    /// Write the active look's typefaces, title size and motion into `finish:` (D-52), and
    /// start fetching the faces. Nothing when `brief.design` names no bundled look.
    /// Editorial: nothing re-renders.
    pub fn apply_look(&mut self) -> Result<bool> {
        let Some(d) = crate::designsys::get(&self.defn.brief.design) else { return Ok(false) };
        // a measured reference's lengths stay on top of the look's motion (D-71)
        let entries: Vec<(String, Value)> = crate::designsys::look_entries(d)
            .into_iter()
            .map(|(k, v)| if k == "motion" { (k, crate::reference::motion_for(&self.defn, v)) } else { (k, v) })
            .collect();
        self.set_finish(&entries)?;
        crate::fonts::prefetch(&self.defn.finish.fonts.specs());
        Ok(true)
    }

    // ------------------------------------------------------------------ a reference teaser (D-71)

    /// Measure a reference teaser into `reference/<id>/`; with `borrow`, the brief names it and
    /// `finish.motion` takes its lengths. Editorial: nothing re-renders.
    pub fn add_reference(&mut self, video: &Path, borrow: bool) -> Result<(crate::reference::Reference, Vec<String>)> {
        let r = crate::reference::add(&self.root, video)?;
        self.state.log("reference", json!({"id": r.id, "source": r.source}));
        self.state.save()?;
        let notes = if borrow { self.use_reference(&r.id)? } else { Vec::new() };
        Ok((r, notes))
    }

    /// Borrow a measured reference: `brief.reference_teaser` names it, and `finish.motion` takes
    /// what it measured on top of the cut's motion (or the look's, or the genre's, when none is
    /// written yet). Returns where each length came from.
    pub fn use_reference(&mut self, id: &str) -> Result<Vec<String>> {
        let Some(r) = crate::reference::load(&self.root, id) else {
            bail!("no measured reference `{id}` (reference/{id}/facts.json): measure one with `shotpilot reference <video>`");
        };
        let base = if self.defn.finish.motion.is_set() {
            // switching references: what the one before set, and nobody changed since, goes
            match crate::reference::active(&self.defn).filter(|old| old.id != r.id) {
                Some(old) => crate::reference::without(&self.defn.finish.motion.to_value(), &old.measure),
                None => self.defn.finish.motion.to_value(),
            }
        } else if let Some(d) = crate::designsys::get(&self.defn.brief.design) {
            crate::designsys::look_entries(d).into_iter().find(|(k, _)| k == "motion").map_or(Value::Null, |(_, v)| v)
        } else {
            crate::refs::genre_for(&self.defn.brief.genre)
                .filter(|g| crate::motion::profile(g.id).is_some())
                .map_or(Value::Null, |g| json!({"profile": g.id}))
        };
        self.set_brief(&[("reference_teaser".into(), json!(r.id))])?;
        self.set_finish(&[("motion".into(), crate::reference::merged_motion(&base, &r.measure))])?;
        Ok(crate::reference::motion_entries(&r.measure).1)
    }

    /// The newest cut video (`cut/<quality>/cut.mp4`).
    pub fn latest_cut_video(&self) -> Option<PathBuf> {
        std::fs::read_dir(self.root.join("cut"))
            .ok()?
            .flatten()
            .map(|e| e.path().join("cut.mp4"))
            .filter(|p| p.is_file())
            .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
    }

    /// Measure the newest cut the same way as the reference (the brief's, unless `id` names
    /// another) and pair their frames at the same shares of their runtimes, for the director.
    pub fn compare_reference(&self, id: Option<&str>) -> Result<crate::reference::Comparison> {
        let id = id.map(str::to_string).unwrap_or_else(|| self.defn.brief.reference_teaser.clone());
        if id.is_empty() {
            bail!("no reference teaser yet: measure one with `shotpilot reference <video>`");
        }
        let Some(r) = crate::reference::load(&self.root, &id) else {
            bail!("no measured reference `{id}` (reference/{id}/facts.json)");
        };
        let Some(video) = self.latest_cut_video() else { bail!("no cut video yet: run `shotpilot cut` first") };
        crate::reference::compare(&self.root, &r, &video)
    }

    /// Bring the game's logo into the project (an image picked anywhere is copied to `art/`)
    /// and return its path relative to shots.yaml, for `finish.logo`.
    pub fn import_logo(&self, src: &Path) -> Result<String> {
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) || !src.is_file() {
            bail!("the logo must be a png, jpg or webp image ({})", src.display());
        }
        let root = std::fs::canonicalize(&self.root).unwrap_or_else(|_| self.root.clone());
        let full = std::fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
        if let Ok(rel) = full.strip_prefix(&root) {
            return Ok(rel.to_string_lossy().replace('\\', "/"));
        }
        let dir = self.root.join("art");
        std::fs::create_dir_all(&dir)?;
        let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("logo");
        let bytes = std::fs::read(src)?;
        let mut name = format!("{stem}.{ext}");
        let mut n = 1;
        // never overwrite a different image of the same name
        while dir.join(&name).exists() && std::fs::read(dir.join(&name)).ok().as_deref() != Some(bytes.as_slice()) {
            n += 1;
            name = format!("{stem}-{n}.{ext}");
        }
        std::fs::write(dir.join(&name), &bytes)?;
        Ok(format!("art/{name}"))
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

    /// Paint storyboard frames (a near-real look over the previz layout) for the given shots,
    /// all when empty. Shots whose frame matches their current picture are kept unless
    /// `force`. Frames live in storyboard/, outside the takes: nothing here re-renders or
    /// reaches the cut. A failed batch stops the run; the frames already made are kept.
    pub fn generate_storyboard(&mut self, gen: &dyn crate::moodboard::ImageGen, ids: &[String], direction: &str,
                               force: bool) -> Result<Vec<StoryboardOutcome>> {
        use crate::storyboard as sb;
        let shots = self.select(ids)?;
        let mut out = Vec::new();
        let mut todo = Vec::new();
        for s in shots {
            match sb::current(&self.defn, &s) {
                Some(b) if b.fresh && !force => out.push(StoryboardOutcome { shot_id: s.id.clone(), action: "kept".into(), file: b.file }),
                _ => todo.push(s),
            }
        }
        let mut err = None;
        for batch in todo.chunks(4) {
            let jobs = batch.iter().map(|s| sb::job(&self.defn, s, direction)).collect::<Result<Vec<_>>>()?;
            match gen.generate_many(&jobs) {
                Ok(images) => {
                    for (s, bytes) in batch.iter().zip(images.iter()) {
                        let file = sb::store(&self.defn, s, bytes)?;
                        out.push(StoryboardOutcome { shot_id: s.id.clone(), action: "made".into(), file });
                    }
                    if images.len() < batch.len() {
                        break;
                    }
                }
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }
        let made: Vec<&str> = out.iter().filter(|o| o.action == "made").map(|o| o.shot_id.as_str()).collect();
        if !made.is_empty() {
            self.state.log("storyboard", json!({"made": made, "direction": direction}));
            self.state.save()?;
        }
        match err {
            Some(e) if made.is_empty() => Err(e),
            _ => Ok(out),
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
        self.edit_sound_full(sfx, &[], audio, note)
    }

    /// [`Self::edit_sound`], also replacing the `sfx` of script lines (line id -> a list,
    /// or null for none): fine-tuning a card's hit is a sound edit like any other.
    pub fn edit_sound_full(&mut self, sfx: &[(String, Option<Value>)], line_sfx: &[(String, Value)],
                           audio: Option<&Map<String, Value>>, note: &str) -> Result<EditReport> {
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
        let mut lines = Vec::new();
        for (id, list) in line_sfx {
            let Some(cur) = self.defn.script.iter().find(|l| l.id == *id) else { bail!("unknown script line '{id}'") };
            let old = json!({"sfx": cur.sfx});
            let new = json!({"sfx": if list.is_null() { json!([]) } else { list.clone() }});
            if old != new {
                changes.insert(id.clone(), json!(field_changes(&old, &new)));
                lines.push((id.clone(), list.clone()));
            }
        }
        if edits.is_empty() && lines.is_empty() && !changes.contains_key("audio") {
            return Ok(EditReport::default()); // nothing to write, nothing to log
        }
        let rep = self.edit_definition_full(&edits, &lines, None, audio, "sound", note)?;
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
        self.replace_plan(shots, None)
    }

    /// Write a fresh plan: the shots and (when given) the script together, since script lines
    /// refer to shot ids. Checked as a whole before anything is written.
    pub fn replace_plan(&mut self, shots: &[Value], script: Option<&[Value]>) -> Result<()> {
        let mut shots = shots.to_vec();
        let mut lines = script.map(<[Value]>::to_vec);
        let music = carry_sound(&self.defn, &mut shots, lines.as_deref_mut().unwrap_or(&mut []));
        let mut doc = Doc::load(&self.path)?;
        doc.set_items("shots", &shots)?;
        match &lines {
            Some(lines) => doc.set_items("script", lines)?,
            // a script that points at shots that are gone would not parse: drop it with them
            None if !self.defn.script.is_empty() => doc.set_items("script", &[])?,
            None => {}
        }
        if let Some(music) = music {
            doc.upsert_entries("audio", &[("music".to_string(), music)])?;
        }
        model::parse(&doc.value()?, &self.path)?;
        doc.save(&self.path)?;
        self.reload()?;
        self.state.log("plan", json!({"shots": shots.iter().map(|s| &s["id"]).collect::<Vec<_>>()}));
        self.state.save()
    }
}

/// Keep the sound of the old plan on a fresh one: a re-plan rewrites shots and script
/// lines (often under new ids), and the effects and music cues placed on them would
/// otherwise be dropped or left pointing at nothing. A shot is matched by id, else by its
/// position; a script line by id (same kind), else by kind and the matched shot it sits on
/// or follows. Items the new plan gives sound of its own keep theirs. Returns the music
/// cues with their `from`/`until` moved, when any moved.
fn carry_sound(old: &Definition, shots: &mut [Value], lines: &mut [Value]) -> Option<Value> {
    let new_shots: Vec<String> = shots.iter().map(|s| model::str_of(&s["id"])).collect();
    let old_shots = old.shot_ids();
    let mut shot_map: HashMap<String, String> = HashMap::new();
    for (i, oid) in old_shots.iter().enumerate() {
        if new_shots.contains(oid) {
            shot_map.insert(oid.clone(), oid.clone());
        } else if let Some(nid) = new_shots.get(i).filter(|n| !old_shots.contains(n)) {
            shot_map.insert(oid.clone(), nid.clone());
        }
    }
    let no_sfx = |v: &Value| v.get("sfx").and_then(Value::as_array).map_or(true, Vec::is_empty);

    // effects on shots (an effect past the new shot's end is dropped)
    for (oid, nid) in &shot_map {
        let Some(sfx) = old.find(oid).ok().and_then(|s| s.get("sfx")).and_then(Value::as_array) else { continue };
        let Some(shot) = shots.iter_mut().find(|s| model::str_of(&s["id"]) == *nid) else { continue };
        if !no_sfx(shot) {
            continue;
        }
        let dur = shot.get("duration").and_then(num).unwrap_or(0.0);
        let kept: Vec<Value> = sfx.iter().filter(|it| it.get("at").and_then(num).unwrap_or(0.0) < dur).cloned().collect();
        if let (false, Some(m)) = (kept.is_empty(), shot.as_object_mut()) {
            m.insert("sfx".into(), Value::Array(kept));
        }
    }

    // effects on script lines
    let anchor = |kind: &str, on: Option<&String>, after: Option<&String>| -> String {
        let a = on.or(after).cloned().unwrap_or_default();
        format!("{kind}@{}", shot_map.get(&a).cloned().unwrap_or(a))
    };
    let new_lines: Vec<(String, String, String)> = lines
        .iter()
        .map(|l| {
            let kind = l.get("kind").map(model::str_of).unwrap_or_else(|| "caption".into());
            let key = format!("{kind}@{}", model::str_of(l.get("on").or(l.get("after")).unwrap_or(&Value::Null)));
            (model::str_of(&l["id"]), kind, key)
        })
        .collect();
    let mut line_map: HashMap<String, String> = HashMap::new();
    let mut taken: Vec<String> = Vec::new();
    for ol in &old.script {
        if new_lines.iter().any(|(id, kind, _)| *id == ol.id && *kind == ol.kind) {
            line_map.insert(ol.id.clone(), ol.id.clone());
            taken.push(ol.id.clone());
        }
    }
    for ol in &old.script {
        if line_map.contains_key(&ol.id) {
            continue;
        }
        let key = anchor(&ol.kind, ol.on.as_ref(), ol.after.as_ref());
        let free = new_lines.iter().find(|(id, _, k)| *k == key && !taken.contains(id) && !old.script.iter().any(|o| o.id == *id));
        if let Some((id, _, _)) = free {
            line_map.insert(ol.id.clone(), id.clone());
            taken.push(id.clone());
        }
    }
    for ol in old.script.iter().filter(|l| !l.sfx.is_empty()) {
        let Some(nid) = line_map.get(&ol.id) else { continue };
        let Some(line) = lines.iter_mut().find(|l| model::str_of(&l["id"]) == *nid) else { continue };
        if let (true, Some(m)) = (no_sfx(line), line.as_object_mut()) {
            m.insert("sfx".into(), Value::Array(ol.sfx.clone()));
        }
    }

    // music cues follow the pieces they start and end on
    let Some(Value::Array(cues)) = old.audio.get("music") else { return None };
    let known = |r: &str| r == "start" || r == "end" || new_shots.contains(&r.to_string()) || new_lines.iter().any(|(id, _, _)| id == r);
    let mut moved = false;
    let cues: Vec<Value> = cues
        .iter()
        .map(|c| {
            let mut c = c.clone();
            for key in ["from", "until"] {
                let r = c.get(key).map(model::str_of).unwrap_or_default();
                if r.is_empty() || known(&r) {
                    continue;
                }
                if let Some(to) = shot_map.get(&r).or(line_map.get(&r)) {
                    c[key] = json!(to);
                    moved = true;
                }
            }
            c
        })
        .collect();
    moved.then_some(Value::Array(cues))
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
