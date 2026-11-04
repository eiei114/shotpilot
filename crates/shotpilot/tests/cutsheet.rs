//! The cut sheet's edits: one-tap nudges, script lines typed in place, notes pointed at one
//! part of a shot, and undo. Each one re-renders only the shot it changes (D-13), and undo
//! puts the old take back without rendering it again.

mod common;

use std::cell::RefCell;

use common::{actions, ids, launches, set, ws, Fixture};
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude, Pointer};
use shotpilot::Result;

fn rendered() -> Fixture {
    let mut f = ws();
    f.ws.render(&[], "still", false, None).unwrap();
    f
}

fn node(f: &Fixture, id: &str) -> Value {
    f.ws.defn.find(id).unwrap().clone()
}

fn with_script(f: &mut Fixture) {
    let mut text = f.text();
    text.push_str(
        "\nscript:\n  # the count runs over the evidence shots\n  - {id: seven, kind: card, ja: 自分を入れて、七人。, en: SEVEN OF US, dur: 2, after: s02_well}\n  - {id: cups, kind: count, ja: コップ、七つ。, on: s03_window, at: 0.5}\n",
    );
    f.write(&text);
    f.ws.reload().unwrap();
}

#[test]
fn a_nudge_rerenders_only_its_shot_and_undo_reuses_the_old_take() {
    let mut f = rendered();
    let before = node(&f, "s03_window");
    let launched = launches(&f.ws).len();

    let rep = f.ws.nudge("s03_window", "closer").unwrap();
    assert_eq!(rep.stale, ids(&["s03_window"]));
    // s03 looks at its own anchor, so moving closer scales the offset
    assert_eq!(node(&f, "s03_window")["camera"]["offset"], json!([-560.0, -400.0, -224.0]));
    // the rest of the shot (its marker, its sound) is untouched
    assert_eq!(node(&f, "s03_window")["sfx"], before["sfx"]);
    assert_eq!(node(&f, "s03_window")["markers"], before["markers"]);

    let out = f.ws.render(&[], "still", false, None).unwrap();
    let acts = actions(&out);
    assert_eq!(acts["s03_window"], "rendered");
    assert!(acts.iter().filter(|(k, _)| *k != "s03_window").all(|(_, a)| a == "reused"), "{acts:?}");
    assert_eq!(launches(&f.ws).len(), launched + 1);

    // undo: the definition goes back, and so does the old take (nothing renders)
    let (rep, what) = f.ws.undo().unwrap();
    assert_eq!(what["action"], "nudge");
    assert_eq!(rep.stale, ids(&["s03_window"]));
    assert_eq!(node(&f, "s03_window"), before);
    let out = f.ws.render(&[], "still", false, None).unwrap();
    assert!(actions(&out).values().all(|a| a == "reused"), "{:?}", actions(&out));
    assert_eq!(launches(&f.ws).len(), launched + 1);
}

#[test]
fn nudges_step_the_lens_and_the_length() {
    let mut f = ws();
    f.ws.nudge("s03_window", "tighter").unwrap();
    assert_eq!(node(&f, "s03_window")["camera"]["lens"]["focal_length"], json!(65.0));
    f.ws.nudge("s03_window", "wider").unwrap();
    f.ws.nudge("s03_window", "wider").unwrap();
    assert_eq!(node(&f, "s03_window")["camera"]["lens"]["focal_length"], json!(35.0));
    f.ws.nudge("s03_window", "longer").unwrap();
    assert_eq!(node(&f, "s03_window")["duration"], json!(4.5));
    // a dolly keeps its share of the distance when the camera moves in
    let d = node(&f, "s01_gate")["camera"]["move"]["distance"].as_f64().unwrap();
    f.ws.nudge("s01_gate", "closer").unwrap();
    assert_eq!(node(&f, "s01_gate")["camera"]["move"]["distance"].as_f64().unwrap(), (d * 0.8).round());
    let err = f.ws.nudge("s03_window", "sideways").err().unwrap();
    assert!(err.msg.contains("closer"), "{}", err.msg);
}

#[test]
fn undo_goes_back_one_edit_at_a_time_and_stops_at_a_plan() {
    let mut f = ws();
    let (s01, s02) = (node(&f, "s01_gate"), node(&f, "s02_well"));
    f.ws.nudge("s01_gate", "longer").unwrap();
    f.ws.set_fields("s02_well", &set(&[("purpose", json!("井戸の縁"))])).unwrap();
    assert_eq!(f.ws.undo_target().unwrap().1["action"], "set");
    f.ws.undo().unwrap();
    assert_eq!(node(&f, "s02_well"), s02);
    f.ws.undo().unwrap();
    assert_eq!(node(&f, "s01_gate"), s01);
    assert!(f.ws.undo_target().is_none());
    assert!(f.ws.undo().err().unwrap().msg.contains("nothing to undo"));

    // a fresh plan rewrites everything: nothing before it can be undone
    f.ws.nudge("s01_gate", "longer").unwrap();
    let shots: Vec<Value> = f.ws.defn.shots.clone();
    f.ws.replace_plan(&shots, None).unwrap();
    assert!(f.ws.undo_target().is_none());
}

#[test]
fn undo_never_overwrites_a_later_hand_edit() {
    let mut f = ws();
    f.ws.nudge("s01_gate", "longer").unwrap();
    // the same shot changed again in a text editor
    let text = f.text().replacen("purpose: ", "purpose: 手で直した。", 1);
    f.write(&text);
    f.ws.reload().unwrap();
    let err = f.ws.undo().err().unwrap();
    assert!(err.msg.contains("changed again"), "{}", err.msg);
    assert!(f.text().contains("手で直した。"));
}

#[test]
fn script_lines_are_typed_in_place_and_never_rerender() {
    let mut f = rendered();
    with_script(&mut f);
    let launched = launches(&f.ws).len();
    let rep = f.ws.set_line("seven", &set(&[("ja", json!(" 七人目がいる。 ")), ("dur", json!(2.5))])).unwrap();
    assert!(rep.stale.is_empty());
    let line = f.ws.defn.script.iter().find(|l| l.id == "seven").unwrap();
    assert_eq!((line.ja.as_str(), line.dur), ("七人目がいる。", 2.5));
    // the hand-written comment and the other line stay as written
    assert!(f.text().contains("# the count runs over the evidence shots"));
    assert!(f.text().contains("ja: コップ、七つ。"));
    let out = f.ws.render(&[], "still", false, None).unwrap();
    assert!(actions(&out).values().all(|a| a == "reused"));
    assert_eq!(launches(&f.ws).len(), launched);

    // where a line sits is not typed here
    let err = f.ws.set_line("seven", &set(&[("after", json!("s01_gate"))])).err().unwrap();
    assert!(err.msg.contains("cannot be set here"), "{}", err.msg);
    assert!(f.ws.set_line("nope", &set(&[("ja", json!("x"))])).is_err());

    f.ws.undo().unwrap();
    let line = f.ws.defn.script.iter().find(|l| l.id == "seven").unwrap();
    assert_eq!((line.ja.as_str(), line.dur), ("自分を入れて、七人。", 2.0));
}

struct FakeClaude {
    payload: Value,
    calls: RefCell<Vec<Value>>,
}

impl Claude for FakeClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        self.calls.borrow_mut().push(request.clone());
        Ok(self.payload.to_string())
    }
}

/// s03 as Claude might send it back: a new camera, but also a new length and purpose, and
/// without its marker and sound.
const S03_EVERYTHING: &str = "id: s03_window
purpose: 窓だけを見せる
duration: 6
camera:
  anchor: window
  offset: [-400, -300, -280]
  lens: {focal_length: 85}
environment: {time_of_day: noon}
";

#[test]
fn a_note_on_the_camera_changes_only_the_camera() {
    let f = ws();
    let before = node(&f, "s03_window");
    let fake = FakeClaude {
        payload: json!({"summary": "closer", "edits": [{"id": "s03_window", "yaml": S03_EVERYTHING, "reason": "r"}], "order": []}),
        calls: RefCell::new(Vec::new()),
    };
    let pointer = Pointer { fields: ai::aspect_fields("camera"), ..Default::default() };
    let rev = ai::revise_at(&f.ws.defn, "もっと寄って", Some(&ids(&["s03_window"])), &pointer, &fake).unwrap();
    let (_, new) = &rev.edits[0];
    assert_eq!(new["camera"]["offset"], json!([-400, -300, -280]));
    assert!(new["camera"].get("move").is_none(), "the camera is taken whole");
    for k in ["purpose", "duration", "environment", "markers", "sfx", "beat"] {
        assert_eq!(new[k], before[k], "{k} must stay as it was");
    }
    let user = fake.calls.borrow()[0]["messages"][0]["content"].to_string();
    assert!(user.contains("the shot's camera only"), "{user}");

    // a camera that comes back unchanged is no edit at all, whatever else came back
    let mut same = before.clone();
    same["duration"] = json!(9);
    let fake = FakeClaude {
        payload: json!({"summary": "x", "edits": [{"id": "s03_window", "yaml": shotpilot::yamlio::dumps(&same), "reason": "r"}], "order": []}),
        calls: RefCell::new(Vec::new()),
    };
    let rev = ai::revise_at(&f.ws.defn, "そのまま", Some(&ids(&["s03_window"])), &pointer, &fake).unwrap();
    assert!(rev.edits.is_empty());

    // a pointed note needs exactly one shot
    let err = ai::revise_at(&f.ws.defn, "x", None, &pointer, &fake).err().unwrap();
    assert!(err.msg.contains("exactly one shot"), "{}", err.msg);
    assert!(ai::aspect_fields("nope").is_none());
}
