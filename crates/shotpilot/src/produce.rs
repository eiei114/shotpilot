//! The first pass: everything constrained up front, then one go to a teaser you can watch.
//!
//! Like open-design's production turn: gather what is missing (scout photos, the stage,
//! a mood board), hand the planner every constraint at once (brief, stage, craft rules,
//! skeleton, scout photos, mood board, finish), write the shot list and the finish, render
//! at the cheapest quality, and cut with the finish burned in. No step looks back at its
//! own output (D-22). Then the human verifies and trims shot by shot: approvals and pointed
//! notes, where only the changed shot re-renders (D-13).

use serde_json::{json, Map, Value};

use crate::ai::{self, Claude};
use crate::error::Result;
use crate::moodboard::ImageGen;
use crate::workspace::Workspace;

#[derive(Debug, Clone)]
pub struct ProduceOpts {
    /// Quality of the first render (previz needs no engine).
    pub quality: String,
    /// Extra direction for the cut (the note typed with the request).
    pub direction: String,
    /// Make a mood board when there is none and an image model is available.
    pub mood: bool,
    /// The sound library to design the sound from (None: no sound design).
    pub library: Option<std::path::PathBuf>,
    /// Stop at the review (D-38): after the script, the shots and their previz, for the
    /// human to check them on the cut sheet. [`resume`] then designs the sound, renders and
    /// cuts. Without it the pass goes all the way, as before.
    pub review: bool,
}

impl Default for ProduceOpts {
    fn default() -> Self {
        ProduceOpts { quality: "previz".into(), direction: String::new(), mood: true, library: None, review: false }
    }
}

impl ProduceOpts {
    /// The app's and the CLI's defaults: the user's library (D-26) when there is one.
    pub fn with_user_library(mut self) -> Self {
        self.library = crate::library::default_root();
        self
    }
}

pub const STEPS: &[(&str, &str)] = &[
    ("scout", "ロケハン写真"),
    ("stage", "ステージの理解"),
    ("mood", "ムードボード"),
    ("plan", "台本と構成"),
    ("review", "確認（台本とカット）"),
    ("sound", "音の設計"),
    ("render", "撮影"),
    ("cut", "Cut（仕上げを焼き込む）"),
];

fn label(key: &str) -> &'static str {
    STEPS.iter().find(|(k, _)| *k == key).map(|(_, l)| *l).unwrap_or("")
}

/// Run the first pass. `emit(event, data)`: "step" {key, label, status: start|done|skipped|failed, detail}.
pub fn produce(ws: &mut Workspace, claude: &dyn Claude, images: Option<&dyn ImageGen>, opts: &ProduceOpts,
               emit: &mut dyn FnMut(&str, &Value)) -> Result<Value> {
    let mut steps: Vec<Value> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let step = |key: &str, status: &str, detail: String, emit: &mut dyn FnMut(&str, &Value), steps: &mut Vec<Value>| {
        let v = json!({"key": key, "label": label(key), "status": status, "detail": detail});
        emit("step", &v);
        if status != "start" {
            steps.push(v);
        }
    };

    // 1. scout photos
    if crate::scout::summary(&ws.root).is_empty() {
        step("scout", "start", String::new(), emit, &mut steps);
        let idx = ws.take_scout(&[])?;
        let n = idx["anchors"].as_object().map(|m| m.len()).unwrap_or(0);
        step("scout", "done", format!("{n} か所"), emit, &mut steps);
    } else {
        step("scout", "skipped", "撮影済み".into(), emit, &mut steps);
    }

    // 2. the stage, read from the photos
    if ws.defn.stage.is_empty() {
        step("stage", "start", String::new(), emit, &mut steps);
        let st = ai::understand_stage(&ws.defn, claude)?;
        ws.set_stage(&st)?;
        step("stage", "done", format!("{} か所を記述", st.anchors.len()), emit, &mut steps);
    } else {
        step("stage", "skipped", "記述済み".into(), emit, &mut steps);
    }

    // 3. mood board (optional: a failure is noted, the pass goes on)
    match (opts.mood, images) {
        (true, Some(gen)) if ws.defn.brief.images.is_empty() => {
            step("mood", "start", String::new(), emit, &mut steps);
            match ws.generate_moodboard(gen, &opts.direction, 4) {
                Ok(made) => step("mood", "done", format!("{} 枚", made.len()), emit, &mut steps),
                Err(e) => {
                    notes.push(format!("ムードボードは作れませんでした: {}", e.msg));
                    step("mood", "failed", e.msg, emit, &mut steps);
                }
            }
        }
        (true, None) if ws.defn.brief.images.is_empty() => {
            step("mood", "skipped", "画像の生成先が未設定".into(), emit, &mut steps)
        }
        (false, _) => step("mood", "skipped", "なしで進める".into(), emit, &mut steps),
        _ => step("mood", "skipped", "作成済み".into(), emit, &mut steps),
    }

    // 4. the shot list and the finish, in one call with every constraint
    step("plan", "start", String::new(), emit, &mut steps);
    let direction = Some(opts.direction.as_str()).filter(|d| !d.trim().is_empty());
    let p = ai::plan(&ws.defn, None, None, None, direction, claude)?;
    ws.replace_plan(&p.shots, Some(&p.script))?;
    ws.set_finish(&p.finish)?;
    let found = ws.check()?;
    let count = |sev: &str| found.iter().filter(|f| f.severity.as_str() == sev).count();
    let checks = json!({"P0": count("P0"), "P1": count("P1"), "P2": count("P2")});
    step("plan", "done", format!("{} カット", p.shots.len()), emit, &mut steps);

    let mut entry = Map::new();
    entry.insert("note".into(), json!(opts.direction));
    entry.insert("rationale".into(), json!(p.rationale));
    entry.insert("adjustments".into(), json!(p.adjustments));
    entry.insert("references".into(), json!(p.references));
    entry.insert("checks".into(), checks);
    entry.insert("shots".into(), json!(p.shots.iter().map(|s| &s["id"]).collect::<Vec<_>>()));

    if opts.review {
        // the review (D-38): the script and the cuts at the cheapest look, for the human to
        // check, fix and approve on the cut sheet before the sound, the takes and the cut
        let base = crate::fingerprint::quality_names(&ws.defn).into_iter().next().unwrap_or_else(|| "previz".into());
        let mut fwd = |e: &str, d: &Value| emit(&format!("render.{e}"), d);
        let outcomes = ws.render(&[], &base, false, Some(&mut fwd))?;
        let failed = outcomes.iter().filter(|o| o.action == "failed").count();
        if failed > 0 {
            notes.push(format!("{base} で撮れなかったカット: {failed} 本"));
        }
        step("review", "done", format!("{base} で {} 本。カット表で確かめて「続ける」", outcomes.len()), emit, &mut steps);
        entry.insert("stopped".into(), json!("review"));
        entry.insert("quality".into(), json!(base));
        entry.insert("steps".into(), json!(steps));
        entry.insert("notes".into(), json!(notes));
        ws.state.log("produce", Value::Object(entry.clone()));
        ws.state.save()?;
        return Ok(Value::Object(entry));
    }
    finish(ws, claude, opts, emit, steps, notes, entry)
}

/// After the review: design the sound (when none was designed yet), render, cut. The
/// human has checked and fixed the script and the cuts on the cut sheet in between.
pub fn resume(ws: &mut Workspace, claude: &dyn Claude, opts: &ProduceOpts, emit: &mut dyn FnMut(&str, &Value))
    -> Result<Value> {
    if ws.defn.shots.is_empty() {
        crate::bail!("nothing to continue: there are no shots yet (make the first pass)");
    }
    let mut entry = Map::new();
    entry.insert("note".into(), json!(opts.direction));
    entry.insert("resumed".into(), json!(true));
    entry.insert("shots".into(), json!(ws.defn.shot_ids()));
    finish(ws, claude, opts, emit, Vec::new(), Vec::new(), entry)
}

/// Whether the project has any sound placed yet (music, effects on shots or on lines).
fn has_sound(ws: &Workspace) -> bool {
    let music = ws.defn.audio.get("music").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
    music || ws.defn.audio.get("bgm").is_some()
        || ws.defn.shots.iter().any(|s| s.get("sfx").and_then(Value::as_array).is_some_and(|a| !a.is_empty()))
        || ws.defn.script.iter().any(|l| !l.sfx.is_empty())
}

/// Sound, render, cut, and the log entry for the pass.
fn finish(ws: &mut Workspace, claude: &dyn Claude, opts: &ProduceOpts, emit: &mut dyn FnMut(&str, &Value),
          mut steps: Vec<Value>, mut notes: Vec<String>, mut entry: Map<String, Value>) -> Result<Value> {
    let step = |key: &str, status: &str, detail: String, emit: &mut dyn FnMut(&str, &Value), steps: &mut Vec<Value>| {
        let v = json!({"key": key, "label": label(key), "status": status, "detail": detail});
        emit("step", &v);
        if status != "start" {
            steps.push(v);
        }
    };

    // 5. sound, from the library, against the cut's pieces (skipped without a library, and
    // after a review when the human already placed sound)
    let lib = crate::library::Library::open(opts.library.as_deref()).unwrap_or_default();
    if lib.sounds.is_empty() {
        step("sound", "skipped", "音源ライブラリが無い".into(), emit, &mut steps);
    } else if entry.contains_key("resumed") && has_sound(ws) {
        step("sound", "skipped", "設計済み".into(), emit, &mut steps);
    } else {
        step("sound", "start", String::new(), emit, &mut steps);
        match crate::sounddesign::design(ws, &lib, claude).and_then(|d| crate::sounddesign::apply(ws, &d).map(|_| d)) {
            Ok(d) => {
                notes.extend(d.notes.iter().cloned());
                let hits: usize = d.shot_sfx.iter().chain(&d.line_sfx).map(|(_, l)| l.len()).sum();
                step("sound", "done", format!("曲 {} · SE {}", d.music.len(), hits), emit, &mut steps);
            }
            Err(e) => {
                notes.push(format!("音の設計はできませんでした: {}", e.msg));
                step("sound", "failed", e.msg, emit, &mut steps);
            }
        }
    }

    // 6. render at the quality asked for
    step("render", "start", opts.quality.clone(), emit, &mut steps);
    let mut fwd = |e: &str, d: &Value| emit(&format!("render.{e}"), d);
    let outcomes = ws.render(&[], &opts.quality, false, Some(&mut fwd))?;
    let failed: Vec<&str> = outcomes.iter().filter(|o| o.action == "failed").map(|o| o.shot_id.as_str()).collect();
    if failed.is_empty() {
        step("render", "done", format!("{} {} 本", opts.quality, outcomes.len()), emit, &mut steps);
    } else {
        notes.push(format!("撮れなかったカット: {}", failed.join(", ")));
        step("render", "failed", format!("{} 本失敗", failed.len()), emit, &mut steps);
    }

    // 7. cut, finish burned in
    step("cut", "start", String::new(), emit, &mut steps);
    let info = ws.cut("best", true)?;
    notes.extend(info.notes.iter().cloned());
    step("cut", "done", format!("{}s", info.seconds), emit, &mut steps);

    entry.insert("steps".into(), json!(steps));
    entry.insert("quality".into(), json!(opts.quality));
    entry.insert("notes".into(), json!(notes));
    ws.state.log("produce", Value::Object(entry.clone()));
    ws.state.save()?;
    let mut out = Value::Object(entry);
    out["video"] = json!(info.video.map(|v| v.display().to_string()));
    out["seconds"] = json!(info.seconds);
    Ok(out)
}
