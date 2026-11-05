//! The planning turn (D-43): a question only for an essential gap, otherwise a plan with a
//! one-sentence direction, a design system (the look) and skills (playbooks) the planner then
//! reads in full. The location scout picks the places the plan needs. Ids are enforced by code.

mod common;

use std::cell::RefCell;

use common::{ids, launches, ws};
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude};
use shotpilot::{designsys, skills, Result};

/// Replies in order and records every request.
struct Script(RefCell<Vec<Value>>, RefCell<Vec<Value>>);

impl Script {
    fn new(replies: Vec<Value>) -> Self {
        Script(RefCell::new(replies), RefCell::new(Vec::new()))
    }

    fn user(&self, i: usize) -> String {
        let req = &self.1.borrow()[i];
        match &req["messages"][0]["content"] {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

impl Claude for Script {
    fn ask(&self, request: &Value) -> Result<String> {
        self.1.borrow_mut().push(request.clone());
        let mut r = self.0.borrow_mut();
        if r.is_empty() {
            return Err(shotpilot::Error::user("no reply left"));
        }
        Ok(r.remove(0).to_string())
    }
}

fn question_reply(n: usize) -> Value {
    let q = |i: usize| json!({"id": format!("q{i}"), "label": "どんなゲーム？", "type": "radio", "default": "horror",
                              "placeholder": "", "options": [{"label": "ホラー", "value": "horror"}, {"label": "コージー", "value": "cozy"}]});
    json!({"route": "question", "say": "1つだけ確認させてください。", "form": {"title": "かんたん確認", "questions": (0..n).map(q).collect::<Vec<_>>()},
           "plan": "", "direction": "", "brief": {"kind": "teaser", "genre": "", "audience": "", "core": "", "must_show": []},
           "design": "", "skills": [], "style": "", "scout_targets": [], "assumptions": []})
}

#[test]
fn the_catalogues_load_and_read_cleanly() {
    assert_eq!(designsys::all().len(), 12);
    for d in designsys::all() {
        assert!(!d.name.is_empty() && !d.description.is_empty(), "{}", d.id);
        for n in 1..=9 {
            assert!(d.doc.contains(&format!("\n## {n}. ")), "{} lacks section {n}", d.id);
        }
        let g = d.grade();
        assert!(g == "none" || shotpilot::finish::GRADES.iter().any(|(n, _)| *n == g), "{}: grade {g}", d.id);
        assert!(["mincho", "gothic"].contains(&d.typeface().as_str()), "{}", d.id);
        assert!(d.genres.iter().all(|g| shotpilot::refs::genres().iter().any(|r| r.id == g)), "{}: {:?}", d.id, d.genres);
        // the planner reads it whole: no self-review words (D-22)
        assert!(shotpilot::craft::self_review_words(d.doc).is_empty(), "{}", d.id);
    }
    assert_eq!(skills::all().len(), 18);
    let mains = skills::all().iter().filter(|s| s.is_main()).count();
    assert_eq!(mains, 14);
    for s in skills::all() {
        assert!(!s.description.is_empty() && !s.triggers.is_empty(), "{}", s.id);
        assert!(["main", "utility"].contains(&s.mode.as_str()), "{}", s.id);
        assert!(s.body.starts_with("\n# ") || s.body.starts_with("# "), "{}", s.id);
        assert!(s.body.contains("## Hard rules"), "{}", s.id);
        assert!(shotpilot::craft::self_review_words(s.body).is_empty(), "{}", s.id);
    }
    assert!(shotpilot::craft::self_review_words(&ai::discover_system()).is_empty());
}

#[test]
fn a_question_comes_back_as_a_form_and_writes_nothing() {
    let f = ws();
    let claude = Script::new(vec![question_reply(7)]);
    let d = ai::discover(&f.ws.defn, "ティザーを作りたい", "", true, &claude).unwrap();
    assert!(d.is_question());
    assert_eq!(d.form["questions"].as_array().unwrap().len(), 5); // never more than five
    assert!(d.brief.is_empty() && d.plan.is_empty());
    // the turn saw the catalogues and the project
    let user = claude.user(0);
    assert!(user.contains("## Design systems") && user.contains("`quiet-dread`") && user.contains("`rules-teaser`"), "{user}");
    assert!(user.contains("## Anchors in the level"), "{user}");
}

#[test]
fn without_questions_it_always_plans() {
    let f = ws();
    let mut reply = question_reply(2);
    reply["plan"] = json!("目的: 中庭を見せる。");
    reply["direction"] = json!("静かに見せる。");
    let claude = Script::new(vec![reply]);
    let d = ai::discover(&f.ws.defn, "", "", false, &claude).unwrap();
    assert!(!d.is_question());
    assert!(claude.user(0).contains("Questions are not possible"));
}

#[test]
fn the_plan_keeps_only_known_ids_and_lands_in_the_brief() {
    let mut f = ws();
    let before = f.ws.status().unwrap();
    let mut reply = common::discovery_reply();
    reply["design"] = json!("quiet-dread");
    reply["skills"] = json!(["rules-teaser", "made-up", "cold-open", "rules-teaser"]);
    reply["style"] = json!("not-a-style");
    let claude = Script::new(vec![reply]);
    let d = ai::discover(&f.ws.defn, "規則のホラー", "[form answers — discovery]\n- どんなゲーム？ [q0]: horror", true, &claude).unwrap();
    assert_eq!(d.skills, ids(&["rules-teaser", "cold-open"]));
    assert!(claude.user(0).contains("## Answers to your questions"));
    f.ws.set_brief(&d.brief).unwrap();
    let b = &f.ws.defn.brief;
    assert_eq!(b.design, "quiet-dread");
    assert_eq!(b.skills, ids(&["rules-teaser", "cold-open"]));
    assert!(b.plan.contains("Scout targets:") && b.plan.contains("井戸") && b.plan.contains("Assumptions:"));
    assert!(!b.direction.is_empty());
    assert!(b.style.is_empty()); // an unknown style is dropped
    // editorial: the plan never makes a take stale
    let after = f.ws.status().unwrap();
    assert_eq!(before.iter().map(|r| r.quality("previz")).collect::<Vec<_>>(), after.iter().map(|r| r.quality("previz")).collect::<Vec<_>>());
    // an unknown id written by hand is refused
    assert!(f.ws.set_brief(&[("design".into(), json!("no-such-look"))]).is_err());
}

#[test]
fn the_planner_reads_the_plan_the_look_and_the_skills() {
    let mut f = ws();
    f.ws.set_brief(&[
        ("design".into(), json!("vhs-found-footage")),
        ("skills".into(), json!(["count-run"])),
        ("plan".into(), json!("記録を数え上げる。")),
        ("direction".into(), json!("記録映像のふりをする。")),
    ])
    .unwrap();
    let slots = shotpilot::craft::skeleton(&f.ws.defn.brief, 30.0, None);
    let spec = shotpilot::craft::plan_spec(&f.ws.defn, &slots);
    assert!(spec.contains("## The plan") && spec.contains("Direction: 記録映像のふりをする。"), "{spec}");
    assert!(spec.contains("## Active design system — 記録映像（VHS） (vhs-found-footage)"), "{spec}");
    assert!(spec.contains("## Active design system tokens"));
    // a utility skill alone brings the main skill for the kind with it
    assert!(spec.contains("## Active skill — announce-teaser") && spec.contains("## Active skill — count-run"), "{spec}");
    // the project's own DESIGN.md wins for the prose
    std::fs::write(f.ws.root.join("DESIGN.md"), "# Our Game\n\n## 1. Visual Theme & Atmosphere\n\nInk and rust.\n").unwrap();
    let spec = shotpilot::craft::plan_spec(&f.ws.defn, &slots);
    assert!(spec.contains("the game's own DESIGN.md") && spec.contains("Ink and rust."), "{spec}");
    assert!(spec.contains("## Active design system tokens")); // numbers still from the preset
}

#[test]
fn the_look_sets_the_finish() {
    let d = designsys::get("neon-action").unwrap();
    let entries = designsys::finish_entries(d);
    assert!(entries.contains(&("grade".to_string(), json!("vivid"))));
    assert!(entries.contains(&("letterbox".to_string(), Value::Null)));
    assert!(entries.contains(&("typeface".to_string(), json!("gothic"))));
}

/// A new project with no anchors, its mock level offering landmarks.
fn untagged_level() -> common::Fixture {
    use shotpilot::templates::{self, EMPTY};
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("shots.yaml");
    std::fs::write(&p, templates::render(EMPTY, "t", "mock", "null", "/Game/Maps/Courtyard")).unwrap();
    let mut ws = shotpilot::Workspace::open(&p).unwrap();
    ws.set_brief(&[("plan".into(), json!("祭壇と扉の規則を見せる。")), ("direction".into(), json!("静かに。"))]).unwrap();
    let store = ws.root.join(".shotpilot/mock_engine");
    std::fs::create_dir_all(&store).unwrap();
    let marks = json!({
        "altar": {"at": [0, 0, 0], "size": [120, 80, 110], "center": [0, 0, 55]},
        "door": {"at": [900, 0, 0], "size": [20, 110, 220], "center": [900, 0, 110]},
        "crate": {"at": [400, 600, 0], "size": [100, 100, 80], "center": [400, 600, 40]},
        "wardrobe": {"at": [-800, 300, 0], "size": [120, 60, 200], "center": [-800, 300, 100]},
    });
    std::fs::write(store.join("landmarks.json"), marks.to_string()).unwrap();
    common::Fixture { dir, ws }
}

#[test]
fn the_scout_photographs_the_places_the_plan_needs() {
    let mut f = untagged_level();
    let claude = Script::new(vec![json!({"picks": [
        {"candidate": "altar", "why": "規則1の場所"}, {"candidate": "nowhere", "why": "x"}, {"candidate": "door", "why": "規則2"}]})]);
    let rep = f.ws.scout(&[], Some(&claude)).unwrap();
    let scan = rep.scan.unwrap();
    assert_eq!(scan.picked_by, "claude");
    assert_eq!(scan.added, ids(&["altar", "door"])); // a made-up candidate is dropped
    let names: Vec<String> = f.ws.defn.anchors.iter().map(|(n, _)| n.clone()).collect();
    assert_eq!(names, ids(&["altar", "door"]));
    assert_eq!(rep.index["anchors"]["door"].as_array().unwrap().len(), 4);
    // Claude read the plan and the candidates
    let user = claude.user(0);
    assert!(user.contains("祭壇と扉の規則を見せる。") && user.contains("`wardrobe` (SM_wardrobe"), "{user}");
    // one engine launch for the level, then previz photos
    assert_eq!(launches(&f.ws).iter().filter(|l| l["kind"] == "survey").count(), 1);
}

#[test]
fn without_claude_the_scout_spreads_out() {
    let mut f = untagged_level();
    let failing = Script::new(vec![]); // every call fails
    let rep = f.ws.scout(&[], Some(&failing)).unwrap();
    let scan = rep.scan.unwrap();
    assert_eq!(scan.picked_by, "spread");
    assert!(scan.note.is_some());
    assert_eq!(scan.added.len(), 4); // all four are at least 3 m apart
}

#[test]
fn spread_skips_landmarks_on_top_of_each_other() {
    let c = |name: &str, x: f64, s: f64| {
        shotpilot::scout::Candidate::from_json(&json!({"name": name, "at": [x, 0, 0], "size": [s, s, s], "center": [x, 0, s / 2.0], "score": s}))
            .unwrap()
    };
    let picked = shotpilot::scout::spread(&[c("big", 0.0, 300.0), c("next_to_big", 100.0, 200.0), c("far", 2000.0, 100.0)], 6);
    let names: Vec<&str> = picked.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["big", "far"]);
}
