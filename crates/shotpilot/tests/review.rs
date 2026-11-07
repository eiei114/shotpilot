//! The first pass stops at the review (D-38): the script and the cuts are checked, fixed and
//! approved on the cut sheet before the sound, the takes and the cut. The script's approval
//! follows its text and the cut it is the spine of, not the cameras.

mod common;

use std::cell::RefCell;

use common::{ids, set, ws};
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude, Pointer};
use shotpilot::produce::{self, ProduceOpts};
use shotpilot::Result;

/// Answers the stage reading and the plan, and records which was asked.
struct Director(RefCell<Vec<String>>);

impl Claude for Director {
    fn ask(&self, request: &Value) -> Result<String> {
        let props = &request["output_config"]["format"]["schema"]["properties"];
        if common::is_discover(request) {
            self.0.borrow_mut().push("discover".into());
            return Ok(common::discovery_reply().to_string());
        }
        if props.get("summary").is_some() && props.get("anchors").is_some() {
            self.0.borrow_mut().push("stage".into());
            return Ok(json!({"summary": "中庭", "anchors": [{"name": "well", "description": "井戸"}],
                             "mood": {"place": "gate", "detail": "lantern", "threat": "window", "title": "well"}}).to_string());
        }
        self.0.borrow_mut().push("plan".into());
        let shot = |id: &str, beat: &str, d: f64, anchor: &str| json!({
            "id": id, "beat": beat, "purpose": "p", "duration": d, "anchor": anchor, "offset": [-400, -300, 100],
            "look_at": "", "focal_length": 35, "move": "static", "move_amount": 0, "time_of_day": "default",
            "notes": "", "appear": [], "caption": "", "transition": "cut"});
        let line = |id: &str, kind: &str, ja: &str, after: &str| json!({"id": id, "kind": kind, "ja": ja, "en": "",
            "head": "", "from": "", "dur": 2.0, "after": after, "on": "", "at": 0.0});
        Ok(json!({"rationale": "静かに始める", "grade": "none", "letterbox": 0,
                  "script": [line("intro", "card", "今日から、ここに住む。", "start"), line("ttl", "title", "夜の中庭", "x")],
                  "shots": [shot("h", "hook", 4.0, "lantern"), shot("e", "establish", 8.0, "gate"),
                            shot("x", "escalate", 6.0, "window"), shot("t", "title", 6.0, "well")]}).to_string())
    }
}

fn steps(events: &[(String, Value)]) -> Vec<String> {
    events
        .iter()
        .filter(|(e, d)| e == "step" && d["status"] != "start")
        .map(|(_, d)| format!("{}:{}", d["key"].as_str().unwrap(), d["status"].as_str().unwrap()))
        .collect()
}

#[test]
fn the_first_pass_stops_at_the_review_and_resumes_after_it() {
    let mut f = ws();
    let claude = Director(Default::default());
    let opts = ProduceOpts { review: true, mood: false, ..Default::default() };
    let mut events = Vec::new();
    let mut on = |e: &str, d: &Value| events.push((e.to_string(), d.clone()));
    let out = produce::produce(&mut f.ws, &claude, None, &opts, &mut on).unwrap();
    assert_eq!(out["stopped"], "review");
    assert_eq!(steps(&events), ["discover:done", "scout:done", "stage:done", "mood:skipped", "plan:done", "review:done"]);
    // the cuts can be looked at (previz), and nothing past the review ran
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
    assert!(!f.ws.root.join("cut").exists());
    assert_eq!(f.ws.state.history().last().unwrap()["stopped"], "review");

    // the script is approved, then its text changes: the approval goes
    assert_eq!(f.ws.script_approval(), "none");
    f.ws.approve_script(true).unwrap();
    assert_eq!(f.ws.script_approval(), "approved");
    f.ws.set_line("intro", &set(&[("ja", json!("今日から、この家に住む。"))])).unwrap();
    assert_eq!(f.ws.script_approval(), "outdated");
    f.ws.approve_script(true).unwrap();
    // cameras and light are not the script: rebuilding them keeps the approval
    f.ws.nudge("e", "closer").unwrap();
    f.ws.nudge("e", "tighter").unwrap();
    assert_eq!(f.ws.script_approval(), "approved");
    // a shot's purpose or length is the script's spine: they drop it
    f.ws.set_fields("x", &set(&[("purpose", json!("窓に人影"))])).unwrap();
    assert_eq!(f.ws.script_approval(), "outdated");
    f.ws.approve_script(true).unwrap();
    f.ws.nudge("x", "longer").unwrap();
    assert_eq!(f.ws.script_approval(), "outdated");
    f.ws.approve_script(true).unwrap();

    // continue: sound (no library here), render, cut; the plan is not asked again
    let mut events = Vec::new();
    let mut on = |e: &str, d: &Value| events.push((e.to_string(), d.clone()));
    let out = produce::resume(&mut f.ws, &claude, &ProduceOpts::default(), &mut on).unwrap();
    assert_eq!(steps(&events), ["sound:skipped", "render:done", "cut:done"]);
    assert_eq!(out["resumed"], true);
    assert_eq!(*claude.0.borrow(), ["discover", "stage", "plan"]);
    assert!(f.ws.root.join("cut").exists());
    assert_eq!(f.ws.defn.script[0].ja, "今日から、この家に住む。"); // the human's fix survived
}

#[test]
fn without_review_the_pass_goes_all_the_way_as_before() {
    let mut f = ws();
    let claude = Director(Default::default());
    let mut events = Vec::new();
    let mut on = |e: &str, d: &Value| events.push((e.to_string(), d.clone()));
    produce::produce(&mut f.ws, &claude, None, &ProduceOpts { mood: false, ..Default::default() }, &mut on).unwrap();
    assert_eq!(steps(&events), ["discover:done", "scout:done", "stage:done", "mood:skipped", "plan:done", "sound:skipped", "render:done", "cut:done"]);
    let mut f2 = ws();
    let err = produce::resume(&mut f2.ws, &claude, &ProduceOpts::default(), &mut |_, _| {});
    assert!(err.is_ok(), "the demo project has shots, so it can continue");
    let mut empty = ws();
    let text = empty.text();
    let cut = text.find("\nshots:").unwrap();
    empty.write(&format!("{}\nshots: []\n", &text[..cut]));
    empty.ws.reload().unwrap();
    assert!(produce::resume(&mut empty.ws, &claude, &ProduceOpts::default(), &mut |_, _| {}).is_err());
}

struct Fake(Value, RefCell<Vec<Value>>);

impl Claude for Fake {
    fn ask(&self, request: &Value) -> Result<String> {
        self.1.borrow_mut().push(request.clone());
        Ok(self.0.to_string())
    }
}

#[test]
fn the_cameras_can_be_rebuilt_for_every_shot_without_touching_the_script() {
    let f = ws();
    let mut s01 = f.ws.defn.find("s01_gate").unwrap().clone();
    let mut s03 = f.ws.defn.find("s03_window").unwrap().clone();
    s01["camera"]["lens"] = json!({"focal_length": 50});
    s01["duration"] = json!(9);
    s01["purpose"] = json!("書き換えられた目的");
    s03["camera"]["offset"] = json!([-300, -200, -100]);
    s03["beat"] = json!("hook");
    let fake = Fake(json!({"summary": "全カットのカメラ", "order": [], "edits": [
        {"id": "s01_gate", "yaml": shotpilot::yamlio::dumps(&s01), "reason": "r"},
        {"id": "s03_window", "yaml": shotpilot::yamlio::dumps(&s03), "reason": "r"}]}), RefCell::new(Vec::new()));
    let pointer = Pointer { fields: ai::aspect_fields("camera"), ..Default::default() };
    let all = f.ws.defn.shot_ids();
    let rev = ai::revise_at(&f.ws.defn, "台本はそのまま、カメラを組み直して", Some(&all), &pointer, &fake).unwrap();
    let edited: Vec<String> = rev.edits.iter().map(|(k, _)| k.clone()).collect();
    assert_eq!(edited, ids(&["s01_gate", "s03_window"]));
    for (id, new) in &rev.edits {
        let old = f.ws.defn.find(id).unwrap();
        for k in ["duration", "purpose", "beat", "sfx", "environment"] {
            assert_eq!(new[k], old[k], "{id}.{k} must stay");
        }
        assert_ne!(new["camera"], old["camera"], "{id}.camera should change");
    }
    let user = fake.1.borrow()[0]["messages"][0]["content"].to_string();
    assert!(user.contains("## Framing") && user.contains("the shot's camera only"), "{user}");
    // still never without naming the shots
    assert!(ai::revise_at(&f.ws.defn, "x", None, &pointer, &fake).is_err());
}

/// D-38 / invariant 9: the script approval follows the script's text and position, each cut's
/// id / beat / purpose / duration and the order. Camera, light and sound never drop it.
#[test]
fn script_approval_follows_the_script_and_never_the_camera_light_or_sound() {
    let mut f = ws();
    shotpilot::sound::write_demo_audio(&f.ws.root).unwrap();
    let mut text = f.text();
    text.push_str("\nscript:\n  - {id: intro, kind: card, ja: 今日から、ここに住む。, dur: 3, after: s01_gate}\n");
    f.write(&text);
    f.ws.reload().unwrap();
    assert_eq!(f.ws.script_approval(), "none");
    f.ws.approve_script(true).unwrap();
    assert_eq!(f.ws.script_approval(), "approved");

    // sound, light and camera edits keep it
    f.ws.edit_sound(&[("s04_lantern".into(), Some(json!([{"file": "audio/hit.wav", "at": 0, "gain": -2}])))], None, "").unwrap();
    assert_eq!(f.ws.script_approval(), "approved", "an sfx edit dropped the script approval");
    f.ws.set_audio(&[("bgm.gain".into(), json!(-10)), ("gain".into(), json!(2))]).unwrap();
    assert_eq!(f.ws.script_approval(), "approved", "an audio gain edit dropped the script approval");
    f.ws.set_fields("s02_well", &set(&[("environment.time_of_day", json!("dawn")), ("exposure", json!(0.5))])).unwrap();
    assert_eq!(f.ws.script_approval(), "approved", "light dropped the script approval");
    f.ws.set_fields("s02_well", &set(&[("camera.lens.focal_length", json!(50))])).unwrap();
    assert_eq!(f.ws.script_approval(), "approved", "a focal change dropped the script approval");

    // a beat change drops it
    f.ws.set_fields("s03_window", &set(&[("beat", json!("hook"))])).unwrap();
    assert_eq!(f.ws.script_approval(), "outdated", "a beat change kept the approval");
    f.ws.approve_script(true).unwrap();

    // a reorder drops it
    let mut order = f.ws.defn.shot_ids();
    order.swap(1, 2);
    f.ws.edit_definition(&[], Some(&order), "reorder", "").unwrap();
    assert_eq!(f.ws.script_approval(), "outdated", "a reorder kept the approval");
    f.ws.approve_script(true).unwrap();

    // a line's timing drops it
    f.ws.set_line("intro", &set(&[("at", json!(1.5))])).unwrap();
    assert_eq!(f.ws.script_approval(), "outdated", "a line's `at` change kept the approval");
    f.ws.approve_script(true).unwrap();

    // moving the line to follow another shot drops it
    let moved = f.text().replace("after: s01_gate", "after: s02_well");
    assert_ne!(moved, f.text());
    f.write(&moved);
    f.ws.reload().unwrap();
    assert_eq!(f.ws.script_approval(), "outdated", "moving a line's `after` kept the approval");
    f.ws.approve_script(true).unwrap();
    assert_eq!(f.ws.script_approval(), "approved");

    // the approval lives in state.json: a fresh Workspace sees it
    let reopened = shotpilot::Workspace::open(f.path()).unwrap();
    assert_eq!(reopened.script_approval(), "approved");
}
