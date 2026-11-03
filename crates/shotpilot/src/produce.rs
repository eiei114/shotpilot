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
}

impl Default for ProduceOpts {
    fn default() -> Self {
        ProduceOpts { quality: "previz".into(), direction: String::new(), mood: true }
    }
}

pub const STEPS: &[(&str, &str)] = &[
    ("scout", "ロケハン写真"),
    ("stage", "ステージの理解"),
    ("mood", "ムードボード"),
    ("plan", "構成と仕上げ"),
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
    ws.replace_all_shots(&p.shots)?;
    ws.set_finish(&p.finish)?;
    let found = ws.check()?;
    let count = |sev: &str| found.iter().filter(|f| f.severity.as_str() == sev).count();
    let checks = json!({"P0": count("P0"), "P1": count("P1"), "P2": count("P2")});
    step("plan", "done", format!("{} ショット", p.shots.len()), emit, &mut steps);

    // 5. render at the cheapest quality asked for
    step("render", "start", opts.quality.clone(), emit, &mut steps);
    let mut fwd = |e: &str, d: &Value| emit(&format!("render.{e}"), d);
    let outcomes = ws.render(&[], &opts.quality, false, Some(&mut fwd))?;
    let failed: Vec<&str> = outcomes.iter().filter(|o| o.action == "failed").map(|o| o.shot_id.as_str()).collect();
    if failed.is_empty() {
        step("render", "done", format!("{} {} 本", opts.quality, outcomes.len()), emit, &mut steps);
    } else {
        notes.push(format!("撮れなかったショット: {}", failed.join(", ")));
        step("render", "failed", format!("{} 本失敗", failed.len()), emit, &mut steps);
    }

    // 6. cut, finish burned in
    step("cut", "start", String::new(), emit, &mut steps);
    let info = ws.cut("best", true)?;
    notes.extend(info.notes.iter().cloned());
    step("cut", "done", format!("{}s", info.seconds), emit, &mut steps);

    let mut entry = Map::new();
    entry.insert("note".into(), json!(opts.direction));
    entry.insert("rationale".into(), json!(p.rationale));
    entry.insert("adjustments".into(), json!(p.adjustments));
    entry.insert("references".into(), json!(p.references));
    entry.insert("checks".into(), checks.clone());
    entry.insert("steps".into(), json!(steps));
    entry.insert("shots".into(), json!(p.shots.iter().map(|s| &s["id"]).collect::<Vec<_>>()));
    entry.insert("quality".into(), json!(opts.quality));
    entry.insert("notes".into(), json!(notes));
    ws.state.log("produce", Value::Object(entry.clone()));
    ws.state.save()?;
    let mut out = Value::Object(entry);
    out["video"] = json!(info.video.map(|v| v.display().to_string()));
    out["seconds"] = json!(info.seconds);
    Ok(out)
}
