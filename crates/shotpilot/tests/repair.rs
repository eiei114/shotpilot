//! Weak-model robustness of the planning call: JSON rescue, vocabulary fixes, contract retry.
//! Every repair is code; none asks the model to review anything.

mod common;

use std::cell::RefCell;

use common::ws;
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude};
use shotpilot::craft;
use shotpilot::Result;

/// Answers with a scripted sequence of replies (the last one repeats) and records requests.
struct Scripted {
    replies: Vec<String>,
    calls: RefCell<Vec<Value>>,
}

impl Scripted {
    fn new(replies: Vec<String>) -> Self {
        Scripted { replies, calls: RefCell::new(Vec::new()) }
    }
    fn n(&self) -> usize {
        self.calls.borrow().len()
    }
}

impl Claude for Scripted {
    fn ask(&self, request: &Value) -> Result<String> {
        let mut c = self.calls.borrow_mut();
        let i = c.len().min(self.replies.len() - 1);
        c.push(request.clone());
        Ok(self.replies[i].clone())
    }
}

fn shot(id: &str, beat: &str, anchor: &str) -> Value {
    json!({"id": id, "beat": beat, "purpose": "p", "duration": 4, "anchor": anchor,
           "offset": [-300, 0, 120], "look_at": "", "focal_length": 35, "move": "static",
           "move_amount": 0, "time_of_day": "default", "notes": "", "appear": []})
}

fn plan_json(shots: Vec<Value>) -> String {
    json!({"rationale": "r", "shots": shots}).to_string()
}

fn good() -> String {
    plan_json(vec![shot("a", "hook", "well")])
}

fn run(c: &Scripted, repair: bool) -> Result<ai::Planned> {
    let f = ws();
    ai::plan_with(&f.ws.defn, Some("test"), None, None, None, c, repair)
}

#[test]
fn fenced_json_is_rescued() {
    let c = Scripted::new(vec![format!("Here you go:\n```json\n{}\n```\nDone.", good())]);
    let p = run(&c, true).unwrap();
    assert_eq!(p.shots.len(), 1);
    assert!(p.adjustments.iter().any(|a| a.contains("返答の JSON を取り出した")), "{:?}", p.adjustments);
    assert_eq!(c.n(), 1);
}

#[test]
fn prose_around_json_is_rescued() {
    let c = Scripted::new(vec![format!("Sure! {{not json}} then {} -- hope it helps {{", good())]);
    let p = run(&c, true).unwrap();
    assert_eq!(p.shots.len(), 1);
    assert!(p.adjustments.iter().any(|a| a.contains("取り出した")));
}

#[test]
fn vocabulary_wobble_is_normalised_and_noted() {
    let mut s = shot("a", "Hook", " Well ");
    s["move"] = json!("DOLLY_IN");
    s["move_amount"] = json!(50);
    s["look_at"] = json!("SP_Gate");
    let c = Scripted::new(vec![plan_json(vec![s])]);
    let p = run(&c, true).unwrap();
    assert_eq!(p.shots[0]["beat"], "hook");
    assert_eq!(p.shots[0]["camera"]["anchor"], "well");
    assert_eq!(p.shots[0]["camera"]["look_at"], "gate");
    assert_eq!(p.shots[0]["camera"]["move"]["type"], "dolly_in");
    let notes = p.adjustments.join("\n");
    for part in ["beat 'Hook' → hook", "move 'DOLLY_IN' → dolly_in", "anchor ' Well ' → well", "look_at 'SP_Gate' → gate"] {
        assert!(notes.contains(part), "missing {part}: {notes}");
    }
    assert_eq!(c.n(), 1);
}

#[test]
fn the_schema_names_the_projects_anchors_and_props() {
    let c = Scripted::new(vec![good()]);
    run(&c, true).unwrap();
    let shot = &c.calls.borrow()[0]["output_config"]["format"]["schema"]["properties"]["shots"]["items"]["properties"];
    let anchors: Vec<&str> = shot["anchor"]["enum"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert!(anchors.contains(&"well") && anchors.contains(&"gate"));
    assert!(shot["look_at"]["enum"].as_array().unwrap().contains(&json!("")));
    assert_eq!(shot["appear"]["items"]["properties"]["prop"]["enum"], json!(["stranger"]));
}

#[test]
fn an_unknown_anchor_is_asked_again_once_with_the_facts() {
    let c = Scripted::new(vec![plan_json(vec![shot("a", "hook", "moon")]), good()]);
    let p = run(&c, true).unwrap();
    assert_eq!(c.n(), 2);
    assert_eq!(p.shots[0]["camera"]["anchor"], "well");
    assert!(p.adjustments.iter().any(|a| a.contains("頼み直した（1回）")), "{:?}", p.adjustments);
    let calls = c.calls.borrow();
    let blocks = calls[1]["messages"][0]["content"].as_array().expect("blocks");
    let extra = blocks.last().unwrap()["text"].as_str().unwrap();
    for name in ["well", "gate", "window", "door", "stranger"] {
        assert!(extra.contains(name), "{name} missing: {extra}");
    }
    assert!(extra.contains("moon"), "{extra}");
    assert!(craft::self_review_words(extra).is_empty(), "{:?}", craft::self_review_words(extra));
}

#[test]
fn three_bad_replies_end_in_the_error_after_three_calls() {
    let c = Scripted::new(vec![plan_json(vec![shot("a", "hook", "moon")])]);
    let err = run(&c, true).err().unwrap();
    assert_eq!(c.n(), 3);
    assert!(err.msg.contains("unknown anchor"), "{}", err.msg);
}

#[test]
fn a_duplicate_id_and_garbage_are_asked_again_too() {
    let dup = plan_json(vec![shot("a", "hook", "well"), shot("a", "title", "gate")]);
    let c = Scripted::new(vec!["I cannot do that".into(), dup, good()]);
    let p = run(&c, true).unwrap();
    assert_eq!(c.n(), 3);
    assert_eq!(p.adjustments.iter().filter(|a| a.contains("頼み直した")).count(), 2);
}

#[test]
fn with_repair_off_every_bad_reply_fails_on_the_first_call() {
    let cases = [
        format!("```json\n{}\n```", good()),
        format!("Sure! {}", good()),
        plan_json(vec![shot("a", "hook", "moon")]),
    ];
    for reply in cases {
        let c = Scripted::new(vec![reply.clone(), good()]);
        assert!(run(&c, false).is_err(), "{reply}");
        assert_eq!(c.n(), 1, "{reply}");
    }
    // the plain schema is the old one: anchors are free text
    let c = Scripted::new(vec![good()]);
    run(&c, false).unwrap();
    let shot = &c.calls.borrow()[0]["output_config"]["format"]["schema"]["properties"]["shots"]["items"]["properties"];
    assert!(shot["anchor"].get("enum").is_none());
}

#[test]
fn a_script_line_with_no_words_is_left_out_and_the_plan_still_writes() {
    // a weak model wrote its first shot again as an empty card, and a card after that card
    let reply = json!({"rationale": "r", "shots": [shot("a", "hook", "well"), shot("b", "title", "well")], "script": [
        {"id": "a", "kind": "card", "ja": "", "after": "start", "dur": 3},
        {"id": "x", "kind": "card", "ja": "", "after": "start", "dur": 2},
        {"id": "y", "kind": "card", "ja": "ここから", "after": "x", "dur": 2},
        {"id": "z", "kind": "caption", "ja": " ", "on": "a", "at": 0.5, "dur": 2},
        {"id": "k", "kind": "black", "after": "b", "dur": 0.5}
    ]});
    let c = Scripted::new(vec![reply.to_string()]);
    let mut f = ws();
    let p = ai::plan_with(&f.ws.defn, Some("test"), None, None, None, &c, true).unwrap();
    assert_eq!(c.n(), 1);
    let ids: Vec<&str> = p.script.iter().map(|l| l["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["y", "k"], "{:?}", p.script);
    // the card that followed the dropped one now follows what that one followed
    assert_eq!(p.script[0]["after"], "start");
    assert_eq!(p.adjustments.iter().filter(|a| a.contains("（文字が空）")).count(), 3, "{:?}", p.adjustments);
    // and it goes into shots.yaml (an empty line would not resolve)
    f.ws.replace_plan(&p.shots, Some(&p.script)).unwrap();
}
