//! The first pass: everything constrained up front, then one go to a teaser you can watch.
//!
//! Plan first when there is no plan (D-45), gather what
//! is missing (scout photos at the places the plan needs, the stage,
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
    /// Where the pass stops before the review, for a human who wants to look first (D-84):
    /// "discover" after the plan, "scout" after the photos, the stage and the mood board. A pass
    /// stops there only when that part ran this time, so pressing on goes past what is done.
    pub stop: Vec<String>,
}

impl Default for ProduceOpts {
    fn default() -> Self {
        ProduceOpts { quality: "previz".into(), direction: String::new(), mood: true, library: None, review: false, stop: Vec::new() }
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
    ("discover", "企画"),
    ("scout", "ロケハン写真"),
    ("stage", "ステージの理解"),
    ("mood", "ムードボード"),
    ("plan", "台本と構成"),
    ("review", "確認（台本とカット）"),
    ("sound", "音の設計"),
    ("render", "撮影"),
    ("see", "撮れた画の確認"),
    ("repair", "写っていないカットの撮り直し"),
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

    // 0. the plan (D-45): the app runs the planning turn first, where it may ask; here it
    // plans with stated assumptions when there is no plan yet
    if ws.defn.brief.plan.trim().is_empty() {
        step("discover", "start", String::new(), emit, &mut steps);
        let d = ai::discover(&ws.defn, &opts.direction, "", false, claude)?;
        ws.apply_discovery(&d, &opts.direction, &Value::Null)?;
        step("discover", "done", d.direction.clone(), emit, &mut steps);
        if opts.stop.iter().any(|s| s == "discover") {
            return stopped(ws, opts, "discover", steps, notes);
        }
    } else {
        step("discover", "skipped", "企画済み".into(), emit, &mut steps);
    }

    // 1. scout photos (the places picked by the plan when nothing in the level is tagged)
    if crate::scout::summary(&ws.root).is_empty() {
        step("scout", "start", String::new(), emit, &mut steps);
        let rep = ws.scout(&[], Some(claude))?;
        if let Some(n) = rep.scan.as_ref().and_then(|s| s.note.clone()) {
            notes.push(n);
        }
        if let Some(m) = &rep.matched {
            if !m.open.is_empty() {
                notes.push(format!("構成画に合う場所が見つからなかった: {}（雰囲気だけを使う）", m.open.join(", ")));
            }
        }
        let n = rep.index["anchors"].as_object().map(|m| m.len()).unwrap_or(0);
        let from = match &rep.scan {
            Some(s) if s.picked_by == "claude" => "（企画に合わせて Claude が選んだ場所）",
            Some(s) if s.auto => "（レベルから自動で選んだ場所）",
            Some(_) => "（レベルのタグから取り込み）",
            None => "",
        };
        step("scout", "done", format!("{n} か所{from}"), emit, &mut steps);
    } else if crate::concept::load(&ws.root).iter().any(crate::concept::Frame::open) {
        // the photos exist; the concept frames made since still look for their places (D-46)
        step("scout", "start", "構成画に合う場所".into(), emit, &mut steps);
        let m = ws.match_concept(claude, None)?;
        if !m.open.is_empty() {
            notes.push(format!("構成画に合う場所が見つからなかった: {}（雰囲気だけを使う）", m.open.join(", ")));
        }
        step("scout", "done", format!("構成画 {} 枚に場所（{} 周、足した場所 {}）", m.found, m.rounds, m.added.len()), emit, &mut steps);
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
    let concept_made = crate::concept::load(&ws.root).iter().any(|f| f.file.is_some());
    match (opts.mood, images) {
        _ if concept_made && ws.defn.brief.images.is_empty() => {
            step("mood", "skipped", "構成画で代用".into(), emit, &mut steps)
        }
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

    // a stop after the scouting: only when something of it ran now (photos, stage or mood board)
    let scouted = steps.iter().any(|s| ["scout", "stage", "mood"].contains(&s["key"].as_str().unwrap_or("")) && s["status"] == "done");
    if scouted && opts.stop.iter().any(|s| s == "scout") {
        return stopped(ws, opts, "scout", steps, notes);
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
    if !p.candidates.is_empty() {
        entry.insert("candidates".into(), json!(p.candidates));
    }
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

/// End a pass at a stop the human asked for (D-84): logged like a pass, so the next press goes on.
fn stopped(ws: &mut Workspace, opts: &ProduceOpts, at: &str, steps: Vec<Value>, notes: Vec<String>) -> Result<Value> {
    let mut entry = Map::new();
    entry.insert("note".into(), json!(opts.direction));
    entry.insert("stopped".into(), json!(at));
    entry.insert("steps".into(), json!(steps));
    entry.insert("notes".into(), json!(notes));
    ws.state.log("produce", Value::Object(entry.clone()));
    ws.state.save()?;
    Ok(Value::Object(entry))
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

/// After the shoot (D-50): look once at each shot's frame; a shot whose subject is not in the
/// picture gets its camera fixed once and is filmed again, and only that shot is looked at
/// again. Never a second round: what is still wrong stays on the cut sheet for the human.
fn see_and_repair(ws: &mut Workspace, claude: &dyn Claude, quality: &str, emit: &mut dyn FnMut(&str, &Value),
                  steps: &mut Vec<Value>, notes: &mut Vec<String>) -> Result<()> {
    let mut step = |key: &str, status: &str, detail: String, emit: &mut dyn FnMut(&str, &Value)| {
        let v = json!({"key": key, "label": label(key), "status": status, "detail": detail});
        emit("step", &v);
        if status != "start" {
            steps.push(v);
        }
    };
    step("see", "start", String::new(), emit);
    let seen = match ws.see(quality, &[], claude) {
        Ok(s) => s,
        Err(e) => {
            notes.push(format!("撮れた画の確認はできませんでした: {}", e.msg));
            step("see", "failed", e.msg, emit);
            return Ok(());
        }
    };
    let blocked: Vec<&ai::Seen> = seen.iter().filter(|s| s.visible == "no").collect();
    let partly = seen.iter().filter(|s| s.visible == "partly").count();
    let detail = match (blocked.len(), partly) {
        (0, 0) => format!("{} カットとも被写体が写っている", seen.len()),
        (b, p) => format!("写っていない {b} · 一部だけ {p}（{} カット中）", seen.len()),
    };
    step("see", "done", detail, emit);
    if blocked.is_empty() {
        step("repair", "skipped", "直すカットなし".into(), emit);
        return Ok(());
    }

    step("repair", "start", blocked.iter().map(|s| s.shot.as_str()).collect::<Vec<_>>().join(", "), emit);
    let mut redo: Vec<String> = Vec::new();
    for s in &blocked {
        let shot = ws.select_one(&s.shot)?;
        let subject = crate::shotcheck::subject_name(&ws.defn, &shot).unwrap_or_else(|| "被写体".into());
        let by = if s.blocked_by.is_empty() { String::new() } else { format!("（{}が遮っている）", s.blocked_by) };
        let note = format!("撮った画に `{subject}` が写っていない{by}。被写体がはっきり見えるようにカメラを直す。");
        let mut pointer = ai::Pointer {
            fields: ai::aspect_fields("camera"),
            context: Some(format!("shot {}; the rendered frame: {}", s.shot, s.note)),
            ..Default::default()
        };
        if let Some(thumb) = crate::fingerprint::render_fingerprint(&ws.defn, &shot, quality)
            .ok()
            .and_then(|rfp| ws.state.takes_for(&shot.id, quality, &rfp).into_iter().next())
            .and_then(|t| t.thumb())
        {
            if let Ok(bytes) = std::fs::read(&thumb) {
                if bytes.len() <= ai::MAX_IMAGE_BYTES {
                    if let Some((_, media)) = crate::moodboard::image_type(&bytes) {
                        pointer.image = Some((media.to_string(), bytes));
                    }
                }
            }
        }
        let rev = match ai::revise_at(&ws.defn, &note, Some(std::slice::from_ref(&s.shot)), &pointer, claude) {
            Ok(r) => r,
            Err(e) => {
                notes.push(format!("{} のカメラは直せませんでした: {}", s.shot, e.msg));
                continue;
            }
        };
        if rev.edits.is_empty() {
            notes.push(format!("{} のカメラは変わりませんでした", s.shot));
            continue;
        }
        ws.edit_definition(&rev.edits, None, "repair", &note)?;
        redo.push(s.shot.clone());
    }
    if redo.is_empty() {
        step("repair", "failed", "直せたカットなし（カット表に残します）".into(), emit);
        return Ok(());
    }
    let mut fwd = |e: &str, d: &Value| emit(&format!("render.{e}"), d);
    ws.render(&redo, quality, false, Some(&mut fwd))?;
    // the repaired shots only, once
    let again = ws.see(quality, &redo, claude).unwrap_or_default();
    let still: Vec<&str> = again.iter().filter(|s| s.visible != "yes").map(|s| s.shot.as_str()).collect();
    if !still.is_empty() {
        notes.push(format!("直しても被写体がはっきり写らないカット: {}（カット表で確かめてください）", still.join(", ")));
    }
    step("repair", "done", format!("{} カットを撮り直した{}", redo.len(),
                                   if still.is_empty() { String::new() } else { format!("（まだ {} カット）", still.len()) }), emit);
    Ok(())
}

/// Facts about the cut that code can read off it (the clock and the title), as notes.
fn cut_facts(ws: &Workspace, info: &crate::cut::CutInfo) -> Vec<String> {
    let mut out = Vec::new();
    let target = ws.defn.teaser.target_duration;
    if target > 0.0 && (info.seconds - target).abs() > 0.1 * target {
        out.push(format!("Cut は {}s で、目標の {}s から 1 割以上ずれています", info.seconds, crate::model::fmt_g(target)));
    }
    let title = ws.defn.teaser.title.trim();
    if !title.is_empty() && !ws.defn.script.iter().any(|l| l.kind == "title" && l.ja.trim() == title) {
        if let Some(l) = ws.defn.script.iter().find(|l| l.kind == "title") {
            out.push(format!("タイトルのカード「{}」が、タイトル「{title}」と一致していません", l.ja));
        }
    }
    out
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

    // 7. one look at what the engine filmed, and at most one repair (D-50, the one
    // bounded exception to D-22). Previz draws boxes, so there is nothing to look at there.
    if !crate::fingerprint::is_previz(&crate::fingerprint::quality(&ws.defn, &opts.quality)?) {
        see_and_repair(ws, claude, &opts.quality, emit, &mut steps, &mut notes)?;
    }

    // 8. cut, finish burned in
    step("cut", "start", String::new(), emit, &mut steps);
    let info = ws.cut("best", true)?;
    notes.extend(info.notes.iter().cloned());
    step("cut", "done", format!("{}s", info.seconds), emit, &mut steps);
    notes.extend(cut_facts(ws, &info));

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
