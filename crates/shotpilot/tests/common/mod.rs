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
