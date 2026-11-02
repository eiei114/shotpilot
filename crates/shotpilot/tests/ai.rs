//! Claude integration, with a fake client: scope enforcement and validation are code, not prompt.

mod common;

use std::cell::RefCell;

use common::{ids, ws};
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude};
use shotpilot::Result;

struct FakeClaude {
    payload: Value,
    calls: RefCell<Vec<Value>>,
}

impl FakeClaude {
    fn new(payload: Value) -> Self {
        FakeClaude { payload, calls: RefCell::new(Vec::new()) }
    }
}

impl Claude for FakeClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        self.calls.borrow_mut().push(request.clone());
        Ok(self.payload.to_string())
    }
}

const S03_CLOSER: &str = "id: s03_window
purpose: 2階の窓を見上げる。人影が一瞬だけ映る
duration: 4
camera:
  anchor: window
  offset: [-400, -300, -280]
  move: {type: crane_up, distance: 80}
  lens: {focal_length: 85}
environment: {time_of_day: night}
";

#[test]
fn revise_edits_only_named_shot() {
    let mut f = ws();
    let fake = FakeClaude::new(json!({"summary": "closer", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}],
                                      "order": []}));
    let rev = ai::revise(&f.ws.defn, "3番目はもっと寄って", None, &fake).unwrap();
    assert_eq!(rev.edits.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(), ids(&["s03_window"]));
    assert!(rev.order.is_none());
    let rep = f.ws.edit_definition(&rev.edits, rev.order.as_deref(), "revise", "note").unwrap();
    assert_eq!(rep.stale, ids(&["s03_window"]));
    let req = &fake.calls.borrow()[0];
    assert_eq!(req["model"], "claude-opus-5-5");
    assert_eq!(req["output_config"]["format"]["type"], "json_schema");
    assert_eq!(req["thinking"]["type"], "adaptive");
    assert_eq!(req["fallbacks"], "default");
}

#[test]
fn revise_scope_is_enforced() {
    let f = ws();
    let fake = FakeClaude::new(json!({"summary": "x", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}],
                                      "order": []}));
    let err = ai::revise(&f.ws.defn, "note", Some(&ids(&["s01_gate"])), &fake).err().unwrap();
    assert!(err.msg.contains("outside the allowed scope"), "{}", err.msg);
}

#[test]
fn revise_rejects_invalid_shot() {
    let f = ws();
    let bad = S03_CLOSER.replace("anchor: window", "anchor: moon");
    let fake = FakeClaude::new(json!({"summary": "x", "edits": [{"id": "s03_window", "yaml": bad, "reason": "r"}], "order": []}));
    let err = ai::revise(&f.ws.defn, "note", None, &fake).err().unwrap();
    assert!(err.msg.contains("unknown anchor"), "{}", err.msg);
}

#[test]
fn revise_reorder_must_be_permutation() {
    let f = ws();
    let fake = FakeClaude::new(json!({"summary": "x", "edits": [], "order": ["s06_title", "s01_gate"]}));
    let err = ai::revise(&f.ws.defn, "タイトルを先に", None, &fake).err().unwrap();
    assert!(err.msg.contains("permutation"), "{}", err.msg);
}

#[test]
fn plan_builds_valid_shots() {
    let mut f = ws();
    let shot = json!({"id": "S01 Hook!", "beat": "hook", "purpose": "p", "duration": 4, "anchor": "well",
                      "offset": [-300, 0, 120], "look_at": "", "focal_length": 35, "move": "orbit_left",
                      "move_amount": 20, "time_of_day": "night", "notes": ""});
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [shot]}));
    let (shots, _) = ai::plan(&f.ws.defn, Some("test"), None, None, &fake).unwrap();
    assert_eq!(shots[0]["id"], "s01_hook");
    assert_eq!(shots[0]["beat"], "hook");
    assert_eq!(shots[0]["camera"]["move"], json!({"type": "orbit_left", "degrees": 20.0}));
    assert_eq!(shots[0]["environment"], json!({"time_of_day": "night"}));
    // writing a plan keeps the file's header comments
    f.ws.replace_all_shots(&shots).unwrap();
    assert_eq!(f.ws.defn.shot_ids(), ids(&["s01_hook"]));
    assert!(f.text().contains("# Order here is the order of the cut"));
}
