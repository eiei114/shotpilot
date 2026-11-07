//! The planning turn (D-45): a question only for an essential gap, otherwise a plan with a
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
    // D-52: its typefaces and title size, as the finish reads them
    assert!(entries.contains(&("fonts".to_string(),
        json!({"display": "Dela Gothic One:400", "body": "M PLUS 1:700", "latin": "Big Shoulders:800"}))));
    assert!(entries.contains(&("title_size".to_string(), json!(0.15))));
}

#[test]
fn every_look_names_catalogue_faces_that_are_not_overused() {
    // D-52: the faces come from references/fonts/catalog.json, none of them a genre cliché or
    // a language model's default, and the Japanese ones carry the kanji cards need
    use shotpilot::fonts;
    for d in designsys::all() {
        let t = &d.tokens["type"];
        for role in ["display", "body", "latin"] {
            let raw = t[role].as_str().unwrap_or_else(|| panic!("{}: type.{role}", d.id));
            let spec = fonts::Spec::parse(raw).unwrap_or_else(|e| panic!("{}: {e}", d.id));
            assert_eq!(spec.label(), raw, "{}: {role} names a weight the family has", d.id);
            assert!(fonts::overused(&spec.family).is_none(), "{}: {} is overused", d.id, spec.family);
            let f = fonts::family(&spec.family).unwrap();
            if role != "latin" {
                assert!(["full", "level1"].contains(&f.kanji.as_str()), "{}: {} lacks kanji", d.id, spec.family);
            }
            // the prose names the same face
            assert!(d.doc.contains(&format!("`{}` {}", spec.family, spec.weight)), "{}: DESIGN.md §3 names {raw}", d.id);
        }
        let size = t["title_size"].as_f64().unwrap();
        assert!((0.04..=0.35).contains(&size), "{}", d.id);
        // D-53: its motion builds on a measured profile, and the prose says which
        let profile = d.tokens["motion"]["profile"].as_str().unwrap_or_else(|| panic!("{}: motion.profile", d.id));
        assert!(shotpilot::motion::profile(profile).is_some(), "{}: {profile}", d.id);
        assert!(d.doc.contains(&format!("the `{profile}` profile")), "{}: DESIGN.md §5 names its motion", d.id);
        // and the finish accepts what the look writes
        let entries: serde_json::Map<String, Value> = designsys::finish_entries(d).into_iter().collect();
        let mut errs = Vec::new();
        let cfg = shotpilot::finish::parse(Some(&Value::Object(entries)), &mut errs);
        assert!(errs.is_empty(), "{}: {errs:?}", d.id);
        assert!(cfg.fonts.display.is_some() && cfg.fonts.body.is_some() && cfg.fonts.latin.is_some(), "{}", d.id);
    }
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

#[test]
fn a_scout_camera_the_engine_pulled_in_front_of_a_wall_is_recorded_where_it_stood() {
    // ue/sp_entry.py answers a `clear_view` build item with where the camera ended up
    let built = json!({"status": "built", "clear_view": {"location": [1200.0, -300.5, 30.0], "focal_length": 18.5}});
    assert_eq!(shotpilot::scout::moved_camera(&built), Some(([1200.0, -300.5, 30.0], 18.5)));
    // nothing in the way (or an engine without geometry): the camera stood where it was planned
    assert_eq!(shotpilot::scout::moved_camera(&json!({"status": "built"})), None);
    assert_eq!(shotpilot::scout::moved_camera(&Value::Null), None);
}

// --------------------------------------------------------------------------- concept frames (D-46)

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0];

struct Painter(RefCell<Vec<String>>);

impl shotpilot::moodboard::ImageGen for Painter {
    fn generate(&self, job: &shotpilot::moodboard::ImageJob) -> Result<Vec<u8>> {
        self.0.borrow_mut().push(job.prompt.clone());
        Ok(PNG.to_vec())
    }
}

fn with_frames(reply: &mut Value) {
    reply["frames"] = json!([
        {"beat": "hook", "look": "暗い祭壇に一本の蝋燭"},
        {"beat": "establish", "look": "長い廊下の奥に扉"},
        {"beat": "escalate", "look": "背の高い衣装箪笥が少し開いている"},
    ]);
}

#[test]
fn the_plan_describes_concept_frames_and_pictures_come_on_request() {
    let mut f = ws();
    let before = f.text();
    let status_before: Vec<String> = f.ws.status().unwrap().iter().map(|r| r.quality("previz").to_string()).collect();
    let mut reply = common::discovery_reply();
    with_frames(&mut reply);
    reply["frames"].as_array_mut().unwrap().push(json!({"beat": "credits", "look": "x"})); // not a beat: dropped
    let claude = Script::new(vec![reply]);
    let d = ai::discover(&f.ws.defn, "規則のホラー", "", true, &claude).unwrap();
    assert_eq!(d.frames.len(), 3);
    f.ws.apply_discovery(&d, "規則のホラー", &Value::Null).unwrap();
    let frames = shotpilot::concept::load(&f.ws.root);
    assert_eq!(frames.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), ["c1", "c2", "c3"]);
    assert!(frames.iter().all(|x| x.file.is_none())); // no picture until asked

    // pictures are made when the human asks, in the look of the plan
    let painter = Painter(Default::default());
    let made = f.ws.generate_concept(&painter).unwrap();
    assert_eq!(made.len(), 3);
    assert!(made.iter().all(|m| m.starts_with("concept/") && f.ws.root.join(m).is_file()));
    assert!(painter.0.borrow()[0].contains("暗い祭壇に一本の蝋燭") && painter.0.borrow()[0].contains("Look: Design System Inspired by Quiet Dread"));
    assert!(f.ws.generate_concept(&painter).unwrap().is_empty()); // nothing left to make
    // references only: no take, no fingerprint, shots.yaml only got the brief
    let status_after: Vec<String> = f.ws.status().unwrap().iter().map(|r| r.quality("previz").to_string()).collect();
    assert_eq!(status_before, status_after);
    assert!(!f.ws.root.join("render").exists() || f.ws.root.join("render").read_dir().unwrap().count() == 0);
    assert!(f.text().len() > before.len() && f.text().contains("design: quiet-dread"));

    // a new plan keeps the pictures of the frames it did not change
    let kept = shotpilot::concept::replace(&f.ws.root, &[("hook".into(), "暗い祭壇に一本の蝋燭".into()), ("title".into(), "夜明け".into())]).unwrap();
    assert!(kept[0].file.is_some() && kept[1].file.is_none());
}

#[test]
fn the_scout_finds_a_place_for_each_frame_and_tries_others_when_none_fits() {
    let mut f = untagged_level();
    let mut reply = common::discovery_reply();
    with_frames(&mut reply);
    let d = ai::discover(&f.ws.defn, "", "", true, &Script::new(vec![reply])).unwrap();
    f.ws.apply_discovery(&d, "", &Value::Null).unwrap();
    f.ws.generate_concept(&Painter(Default::default())).unwrap();

    let claude = Script::new(vec![
        // the first places, picked with the pictures in mind
        json!({"picks": [{"candidate": "altar", "why": "蝋燭"}, {"candidate": "door", "why": "廊下"}]}),
        // the photos against the frames: c3 finds nothing
        json!({"matches": [
            {"frame": "c1", "photo": "altar/front", "fit": "good", "why": "暗い祭壇"},
            {"frame": "c2", "photo": "door/left", "fit": "rough", "why": "扉はあるが廊下が短い"},
            {"frame": "c3", "photo": "", "fit": "none", "why": "箪笥がない"},
            {"frame": "c9", "photo": "altar/front", "fit": "good", "why": "unknown frame"}]}),
        // other places for c3
        json!({"picks": [{"candidate": "wardrobe", "why": "箪笥"}]}),
        json!({"matches": [{"frame": "c3", "photo": "wardrobe/back", "fit": "good", "why": "背の高い箪笥"}]}),
    ]);
    let rep = f.ws.scout(&[], Some(&claude)).unwrap();
    let m = rep.matched.unwrap();
    assert_eq!(m.rounds, 2);
    assert_eq!(m.added, ids(&["wardrobe"]));
    assert_eq!(m.found, 3);
    assert!(m.open.is_empty());
    let frames = shotpilot::concept::load(&f.ws.root);
    let at = |id: &str| frames.iter().find(|x| x.id == id).unwrap().found.clone().unwrap();
    assert_eq!((at("c1").anchor.as_str(), at("c1").side.as_str(), at("c1").fit.as_str()), ("altar", "front", "good"));
    assert_eq!(at("c2").fit, "rough");
    assert_eq!((at("c3").anchor.as_str(), at("c3").side.as_str()), ("wardrobe", "back"));
    assert!(f.ws.root.join(&at("c3").file).is_file());
    assert_eq!(frames.iter().find(|x| x.id == "c3").unwrap().tried, ids(&["altar", "door"]));
    // the first pick saw the pictures' looks; the second only the one still open
    assert!(claude.user(0).contains("Pictures still without a place") && claude.user(0).contains("暗い祭壇"));
    assert!(claude.user(2).contains("衣装箪笥") && !claude.user(2).contains("暗い祭壇"));
    // the second comparison only looked at the new place's photos
    let second = claude.user(3);
    assert!(second.contains("Scout photo `wardrobe/front`") && !second.contains("altar/front"), "{second}");
    assert!(second.contains("Concept frame `c3`") && !second.contains("Concept frame `c1`"));
    // one survey for the level, no second one: the candidates were already known
    assert_eq!(launches(&f.ws).iter().filter(|l| l["kind"] == "survey").count(), 1);

    // the planner gets each frame with its place
    let (blocks, text) = shotpilot::concept::planner_blocks(&f.ws.defn);
    assert!(text.contains("film it at anchor `wardrobe`, framed like its back scout photo"), "{text}");
    assert_eq!(blocks.iter().filter(|b| b["type"] == "image").count(), 6); // three frames, three places
}

#[test]
fn frames_with_no_place_stay_mood_only() {
    let mut f = untagged_level();
    let mut reply = common::discovery_reply();
    with_frames(&mut reply);
    let d = ai::discover(&f.ws.defn, "", "", true, &Script::new(vec![reply])).unwrap();
    f.ws.apply_discovery(&d, "", &Value::Null).unwrap();
    f.ws.generate_concept(&Painter(Default::default())).unwrap();
    // scouted before: all four places are anchors already, nothing else to try
    f.ws.scout(&[], None).unwrap();
    let none = json!({"matches": [
        {"frame": "c1", "photo": "", "fit": "none", "why": ""}, {"frame": "c2", "photo": "", "fit": "none", "why": ""},
        {"frame": "c3", "photo": "crate/front", "fit": "good", "why": "x"}]});
    let claude = Script::new(vec![none]);
    let m = f.ws.match_concept(&claude, None).unwrap();
    assert_eq!(m.rounds, 1);
    assert_eq!(m.open, ids(&["c1", "c2"]));
    assert_eq!(launches(&f.ws).iter().filter(|l| l["kind"] == "survey").count(), 2); // asked the level for others once
    let (_, text) = shotpilot::concept::planner_blocks(&f.ws.defn);
    assert!(text.contains("`c1` (hook): no place in the level looks like it; take its mood"), "{text}");

    // the place c3 was found at is scouted away: c3 is open again, to be matched among the new places
    assert!(shotpilot::concept::load_live(&f.ws.defn).iter().any(|x| x.id == "c3" && x.found.is_some()));
    f.ws.defn.anchors.retain(|(n, _)| n != "crate");
    let live = shotpilot::concept::load_live(&f.ws.defn);
    assert!(live.iter().any(|x| x.id == "c3" && x.found.is_none() && x.open()), "{:?}", live.iter().map(|x| (&x.id, &x.found)).collect::<Vec<_>>());
}

// ---------------------------------------------------------------------------
// Invariant guards added for D-51 (English dictionary) and D-49 (reading time).

/// D-51: every Japanese UI string in the pages and sources has an English entry in
/// web/i18n-en.json. Runs the extractor on a scratch copy so the repo's i18n-free.json is
/// never rewritten by a test run.
#[test]
fn every_ui_string_has_an_english_entry() {
    if std::process::Command::new("node").arg("--version").output().is_err() {
        eprintln!("skipped: node is not installed, so tools/i18n_extract.js cannot run");
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let copy = tempfile::tempdir().unwrap();
    let crate_dir = root.join("crates/shotpilot");
    let (web, src) = (copy.path().join("crates/shotpilot/web"), copy.path().join("crates/shotpilot/src"));
    std::fs::create_dir_all(&web).unwrap();
    std::fs::create_dir_all(&src).unwrap();
    for (from, to) in [(crate_dir.join("web"), &web), (crate_dir.join("src"), &src)] {
        for e in std::fs::read_dir(from).unwrap().flatten() {
            if e.path().is_file() {
                std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    let out = std::process::Command::new("node")
        .arg(root.join("tools/i18n_extract.js"))
        .arg(copy.path())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(out.status.success(), "extractor failed: {err}");
    let last = err.trim().lines().last().unwrap_or("");
    assert!(last.contains(", 0 not in i18n-en.json"), "strings missing from web/i18n-en.json: {last}\n{}", String::from_utf8_lossy(&out.stdout));
}

/// D-49: the reading-time arithmetic lives in `script::reading_secs` only; no other source
/// file carries its constants (4 characters or 3 words per second, plus 0.5 s).
#[test]
fn reading_time_is_computed_in_one_place() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let own = std::fs::read_to_string(dir.join("script.rs")).unwrap();
    let squash = |s: &str| s.split_whitespace().collect::<String>();
    // the scan must be looking for something that really is in script.rs
    assert!(squash(&own).contains("/4.0+0.5"), "reading_secs no longer uses the constants this guard scans for");
    let mut offenders = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "rs") || p.file_name().unwrap() == "script.rs" {
            continue;
        }
        let s = squash(&std::fs::read_to_string(&p).unwrap());
        if s.contains("/4.0+0.5") || s.contains("/3.0+0.5") || s.contains("/4.+0.5") || s.contains("/3.+0.5") {
            offenders.push(p.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(offenders.is_empty(), "reading time is reimplemented outside script::reading_secs in {offenders:?}");
}

/// D-49: a card whose words are rewritten through a revise (finishing note) is kept on screen
/// long enough to read, by `reading_secs`, even when Claude returns a short `dur`; a hand-typed
/// line is not touched by this path.
#[test]
fn revised_card_words_are_held_for_their_reading_time() {
    let mut f = ws();
    let mut text = f.text();
    text.push_str("\nscript:\n  - {id: seven, kind: card, ja: 七号室, dur: 2, after: s02_well}\n");
    f.write(&text);
    f.ws.reload().unwrap();
    let words = "今日から、ここに住む。";
    let need = shotpilot::script::reading_secs("card", words, "");
    assert!(need > 3.0, "the sample line must need more than the 1s Claude asks for");
    let reply = json!({"summary": "s", "edits": [], "order": [],
                       "texts": [{"id": "seven", "yaml": format!("ja: {words}\ndur: 1")}]});
    let fake = Script::new(vec![reply]);
    let pointer = ai::Pointer { fields: ai::aspect_fields("finish"), texts: Some(ids(&["seven"])), ..Default::default() };
    let rev = ai::revise_at(&f.ws.defn, "言葉を変えて", Some(&ids(&["s02_well"])), &pointer, &fake).unwrap();
    let (_, changes) = rev.texts.iter().find(|(id, _)| id == "seven").expect("the card edit comes back");
    let dur = changes.iter().find(|(k, _)| k == "dur").expect("dur is lengthened").1.as_f64().unwrap();
    assert!(dur + 0.05 >= need && dur <= need + 0.1, "dur {dur} should be the reading time {need}");
}
