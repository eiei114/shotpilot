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
    let shots = ai::plan(&f.ws.defn, Some("test"), None, None, None, &fake).unwrap().shots;
    assert_eq!(shots[0]["id"], "s01_hook");
    assert_eq!(shots[0]["beat"], "hook");
    assert_eq!(shots[0]["camera"]["move"], json!({"type": "orbit_left", "degrees": 20.0}));
    assert_eq!(shots[0]["environment"], json!({"time_of_day": "night"}));
    // writing a plan keeps the file's header comments
    f.ws.replace_all_shots(&shots).unwrap();
    assert_eq!(f.ws.defn.shot_ids(), ids(&["s01_hook"]));
    assert!(f.text().contains("# Order here is the order of the cut"));
}

fn plan_shot(id: &str, beat: &str, duration: f64, anchor: &str) -> Value {
    json!({"id": id, "beat": beat, "purpose": "p", "duration": duration, "anchor": anchor,
           "offset": [-300, 0, 120], "look_at": "", "focal_length": 35, "move": "static",
           "move_amount": 0, "time_of_day": "default", "notes": "", "appear": []})
}

#[test]
fn plan_gets_the_brief_skeleton_and_craft_before_writing() {
    let mut f = ws();
    f.ws.set_brief(&common::set(&[("genre", json!("一人称ホラー")), ("must_show", json!(["井戸"]))])).unwrap();
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("a", "hook", 4.0, "well")]}));
    ai::plan(&f.ws.defn, None, None, None, Some("ジャンプスケアなしで"), &fake).unwrap();
    let req = &fake.calls.borrow()[0];
    let user = req["messages"][0]["content"].as_str().unwrap();
    for part in ["well: [0.0, 0.0, 80.0], 140 x 140 x 90 cm", "fills the frame height from about", "## Brief", "Genre: 一人称ホラー", "Must show: 井戸", "## Skeleton", "## Craft rules",
                 "Do not open on the title", "## Framing", "ジャンプスケアなしで"] {
        assert!(user.contains(part), "missing {part}:
{user}");
    }
}

#[test]
fn prompts_never_ask_claude_to_inspect_its_own_output() {
    // D-22: quality comes from the spec given before writing, not from self-review
    let f = ws();
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("a", "hook", 4.0, "well")]}));
    ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let rev = FakeClaude::new(json!({"summary": "x", "edits": [], "order": [], "audio": ""}));
    ai::revise(&f.ws.defn, "3番目はもっと寄って", Some(&ids(&["s03_window"])), &rev).unwrap();
    for req in [&fake.calls.borrow()[0], &rev.calls.borrow()[0]] {
        let system = req["system"].as_str().unwrap();
        assert!(shotpilot::craft::self_review_words(system).is_empty(), "{:?}", shotpilot::craft::self_review_words(system));
    }
    let plan_user = fake.calls.borrow()[0]["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(shotpilot::craft::self_review_words(&plan_user).is_empty(), "{:?}", shotpilot::craft::self_review_words(&plan_user));
}

#[test]
fn code_puts_beats_in_order_and_fits_the_runtime() {
    let f = ws(); // target 30s
    let shots = vec![
        plan_shot("t", "title", 4.0, "well"),
        plan_shot("h", "hook", 12.0, "gate"),
        plan_shot("e", "establish", 10.0, "well"),
        plan_shot("x", "escalate", 20.0, "door"),
    ];
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": shots}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let order: Vec<&str> = p.shots.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert_eq!(order, ["h", "e", "x", "t"]);
    let total: f64 = p.shots.iter().map(|s| s["duration"].as_f64().unwrap()).sum();
    assert!((total - 30.0).abs() <= 3.0, "{total}");
    assert!(p.shots.iter().all(|s| (1.0..=8.0).contains(&s["duration"].as_f64().unwrap())));
    // 12/10/20 s clamp to 8 s; 28 s is within 10% of 30 s, so nothing is rescaled
    assert_eq!(p.adjustments.len(), 2, "{:?}", p.adjustments);

    // far off the target: durations are rescaled to fit
    let short = vec![plan_shot("h", "hook", 1.0, "gate"), plan_shot("e", "establish", 2.0, "well"),
                     plan_shot("x", "escalate", 2.0, "door"), plan_shot("t", "title", 2.0, "well")];
    let p = ai::plan(&f.ws.defn, None, None, None, None, &FakeClaude::new(json!({"rationale": "r", "shots": short}))).unwrap();
    let total: f64 = p.shots.iter().map(|s| s["duration"].as_f64().unwrap()).sum();
    assert!((total - 30.0).abs() <= 3.0, "{total}");
    assert!(p.adjustments.iter().any(|a| a.starts_with("scaled durations")), "{:?}", p.adjustments);
}

#[test]
fn skeleton_follows_the_craft() {
    use shotpilot::craft::skeleton;
    use shotpilot::model::BriefCfg;
    for target in [30.0, 45.0, 60.0] {
        let slots = skeleton(&BriefCfg::default(), target, None);
        let total: f64 = slots.iter().map(|s| s.duration).sum();
        assert!((total - target).abs() < 0.6, "{target}: {total}");
        assert_eq!(slots.first().unwrap().beat, "hook");
        assert_eq!(slots.last().unwrap().beat, "title");
        let mean = |b: &str| {
            let d: Vec<f64> = slots.iter().filter(|s| s.beat == b).map(|s| s.duration).collect();
            d.iter().sum::<f64>() / d.len() as f64
        };
        assert!(mean("escalate") < mean("establish"), "{slots:?}");
    }
    let gameplay = BriefCfg { kind: "gameplay".into(), ..Default::default() };
    assert!(skeleton(&gameplay, 60.0, None)[0].duration <= 3.0);
}

#[test]
fn plan_can_make_a_prop_appear() {
    let f = ws();
    let mut hook = plan_shot("h", "hook", 4.0, "door");
    hook["appear"] = json!([{"prop": "stranger", "at": 1.2, "anchor": "door", "offset": [150, 0, -120]}]);
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [hook]}));
    let p = ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    assert_eq!(p.shots[0]["appear"][0]["prop"], "stranger");
    let user = fake.calls.borrow()[0]["messages"][0]["content"].as_str().unwrap().to_string();
    assert!(user.contains("- stranger: /Game/Characters/Mannequins/Meshes/SKM_Manny (a character, no animation)"), "{user}");
}

#[test]
fn plan_sees_the_mood_board_as_images() {
    let mut f = ws();
    let png = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
    f.ws.add_moodboard_image("mood.png", &png).unwrap();
    let fake = FakeClaude::new(json!({"rationale": "r", "shots": [plan_shot("h", "hook", 4.0, "well")]}));
    ai::plan(&f.ws.defn, None, None, None, None, &fake).unwrap();
    let content = fake.calls.borrow()[0]["messages"][0]["content"].clone();
    let blocks = content.as_array().unwrap();
    assert_eq!(blocks[0]["type"], "image");
    let text = blocks.last().unwrap()["text"].as_str().unwrap();
    assert!(text.contains("## Mood board") && text.contains("not shot designs"), "{text}");
    assert!(shotpilot::craft::self_review_words(text).is_empty());
}
