#![allow(dead_code)]

use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::Value;
use shotpilot::templates::{self, DEMO};
use shotpilot::workspace::RenderOutcome;
use shotpilot::Workspace;

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub ws: Workspace,
}

impl Fixture {
    pub fn path(&self) -> PathBuf {
        self.ws.path.clone()
    }

    pub fn text(&self) -> String {
        std::fs::read_to_string(&self.ws.path).unwrap()
    }

    pub fn write(&self, text: &str) {
        std::fs::write(&self.ws.path, text).unwrap();
    }
}

/// A demo project (6 shots, mock engine) in a fresh temp dir.
pub fn ws() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("shots.yaml");
    std::fs::write(&p, templates::render(DEMO, "t", "mock", "null", "/Game/Maps/Courtyard")).unwrap();
    let ws = Workspace::open(&p).unwrap();
    Fixture { dir, ws }
}

/// Mock engine launch records (`.shotpilot/mock_engine/launches.jsonl`).
pub fn launches(ws: &Workspace) -> Vec<Value> {
    let f = ws.root.join(".shotpilot").join("mock_engine").join("launches.jsonl");
    std::fs::read_to_string(f)
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).unwrap()).collect())
        .unwrap_or_default()
}

pub fn actions(outcomes: &[RenderOutcome]) -> HashMap<String, String> {
    outcomes.iter().map(|o| (o.shot_id.clone(), o.action.clone())).collect()
}

pub fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

pub fn set(pairs: &[(&str, Value)]) -> Vec<(String, Value)> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

/// Whether a fake Claude is being asked the planning turn (D-45).
pub fn is_discover(request: &Value) -> bool {
    request["output_config"]["format"]["schema"]["properties"].get("route").is_some()
}

/// A planning turn's answer: a plan in the quiet-dread look with the rules skill.
pub fn discovery_reply() -> Value {
    serde_json::json!({
        "route": "plan", "say": "規則で見せる静かなホラーにします。", "form": {"title": "", "questions": []},
        "plan": "目的: 中庭の規則を3つ見せ、最後に破られる。", "direction": "動かないカメラと規則のカードで、静かに怖がらせる。",
        "brief": {"kind": "teaser", "genre": "ホラー", "audience": "", "core": "見てはいけないものを見てしまう", "must_show": []},
        "design": "quiet-dread", "skills": ["rules-teaser", "title-drop"], "style": "",
        "scout_targets": [{"look_for": "井戸", "why": "規則が指す場所"}], "assumptions": ["発売日は書かない"]
    })
}

/// A measured reference teaser (D-59) as `shotpilot reference` leaves it, without ffmpeg:
/// 40 s, two blacks (one reached by a fade), two white flashes, a whiteout, two lines on black.
pub fn measured_reference(ws: &Workspace, id: &str) {
    let dir = ws.root.join("reference").join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let facts = serde_json::json!({
        "id": id, "source": "demo.mp4", "bytes": 1, "secs": 40.0,
        "cuts": [2.0, 4.1, 6.0, 9.5, 12.0, 15.0, 18.0, 22.0, 26.0, 31.0, 35.0],
        "blacks": [{"at": 10.0, "gap": 0.6, "fade_out": 0.0, "fade_in": 0.0}, {"at": 30.0, "gap": 0.5, "fade_out": 0.33, "fade_in": 0.0}],
        "flashes": [{"at": 20.0, "len": 0.17}, {"at": 21.2, "len": 0.083}],
        "whiteouts": [{"at": 36.0, "rise": 0.33, "hold": 0.42, "decay": 0.92}],
        "texts": [
            {"at": 1.2, "in": 0.5, "hold": 2.0, "out": 0.0, "in_dark": true, "out_dark": false, "contrast": 80, "steady": 0.02},
            {"at": 24.0, "in": 0.58, "hold": 2.5, "out": 0.0, "in_dark": true, "out_dark": false, "contrast": 90, "steady": 0.03}
        ]
    });
    std::fs::write(dir.join("facts.json"), serde_json::to_string_pretty(&facts).unwrap()).unwrap();
}
