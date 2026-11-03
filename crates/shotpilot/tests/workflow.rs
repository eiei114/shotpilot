//! The production workflow on top of the loop: brief, shot check, and approval gates.

mod common;

use common::{ids, launches, set, ws};
use shotpilot::fingerprint as fp;
use serde_json::json;

fn rules(f: &common::Fixture) -> Vec<String> {
    f.ws.check().unwrap().iter().map(|x| format!("{}:{}", x.rule, x.shot.clone().unwrap_or_default())).collect()
}

#[test]
fn demo_passes_the_blocking_checks() {
    let f = ws();
    assert!(f.ws.check().unwrap().iter().all(|x| x.severity > shotpilot::shotcheck::Severity::P1), "{:?}", rules(&f));
}

#[test]
fn opening_on_the_title_is_p0_and_blocks_approval() {
    let mut f = ws();
    let order = ids(&["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"]);
    f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    let found = f.ws.check().unwrap();
    assert_eq!(found[0].rule, "open.title");
    assert_eq!(found[0].severity.as_str(), "P0");
    let err = f.ws.approve("s06_title", true).unwrap_err();
    assert!(err.msg.contains("P0"), "{}", err.msg);
    // other shots are not blocked by a P0 on another shot
    f.ws.approve("s02_well", true).unwrap();
}

#[test]
fn same_subject_same_angle_is_a_jump_cut() {
    let mut f = ws();
    let cam = json!({"anchor": "well", "offset": [-190, 75, 60], "lens": {"focal_length": 85}});
    f.ws.set_fields("s03_window", &set(&[("camera", cam), ("environment", json!(null))])).unwrap();
    assert!(rules(&f).contains(&"cut.jump:s03_window".to_string()), "{:?}", rules(&f));
    // a different shot size cures it
    f.ws.set_fields("s03_window", &set(&[("camera.lens.focal_length", json!(24))])).unwrap();
    assert!(!rules(&f).contains(&"cut.jump:s03_window".to_string()), "{:?}", rules(&f));
}

#[test]
fn approval_follows_the_picture_not_the_notes() {
    let mut f = ws();
    f.ws.approve("s03_window", true).unwrap();
    let approval = |f: &common::Fixture| {
        f.ws.status().unwrap().into_iter().find(|r| r.id == "s03_window").unwrap().approval
    };
    assert_eq!(approval(&f), "approved");
    // editorial edits keep the approval
    f.ws.set_fields("s03_window", &set(&[("purpose", json!("別の説明")), ("beat", json!("escalate"))])).unwrap();
    assert_eq!(approval(&f), "approved");
    // a pixel edit drops it; undoing the edit brings it back
    f.ws.set_fields("s03_window", &set(&[("camera.lens.focal_length", json!(70))])).unwrap();
    assert_eq!(approval(&f), "outdated");
    f.ws.set_fields("s03_window", &set(&[("camera.lens.focal_length", json!(50))])).unwrap();
    assert_eq!(approval(&f), "approved");
    f.ws.approve("s03_window", false).unwrap();
    assert_eq!(approval(&f), "none");
}

#[test]
fn brief_is_editorial_and_keeps_comments() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let before = launches(&f.ws).len();
    f.ws.set_brief(&set(&[
        ("kind", json!("teaser")),
        ("genre", json!("一人称ホラー")),
        ("must_show", json!(["井戸", "2階の窓"])),
    ]))
    .unwrap();
    assert_eq!(f.ws.defn.brief.genre, "一人称ホラー");
    assert_eq!(f.ws.defn.brief.must_show, vec!["井戸", "2階の窓"]);
    assert!(f.text().contains("# Shotpilot shot definition"));
    let st = f.ws.status().unwrap();
    assert!(st.iter().all(|r| r.quality("previz") == "fresh"));
    let rep = f.ws.render(&[], "previz", false, None).unwrap();
    assert!(rep.iter().all(|o| o.action == "reused"));
    assert_eq!(launches(&f.ws).len(), before);

    let err = f.ws.set_brief(&set(&[("kind", json!("trailer"))])).unwrap_err();
    assert!(err.msg.contains("brief.kind"), "{}", err.msg);
    let err = f.ws.set_brief(&set(&[("colour", json!("red"))])).unwrap_err();
    assert!(err.msg.contains("unknown brief field"), "{}", err.msg);
}

#[test]
fn gameplay_trailers_want_a_shorter_hook() {
    let mut f = ws();
    assert!(!rules(&f).iter().any(|r| r.starts_with("hook.long")));
    f.ws.set_brief(&set(&[("kind", json!("gameplay"))])).unwrap();
    assert!(rules(&f).iter().any(|r| r.starts_with("hook.long")), "{:?}", rules(&f));
}

#[test]
fn anchors_take_a_size_and_previz_alone_follows_it() {
    let mut f = ws();
    let s = f.ws.select_one("s02_well").unwrap();
    let well = f.ws.defn.known_box("well").unwrap();
    assert_eq!(well.size, [140.0, 140.0, 90.0]);
    assert_eq!(well.center, [0.0, 0.0, 45.0]);
    let (pv, still) = (fp::render_fingerprint(&f.ws.defn, &s, "previz").unwrap(), fp::render_fingerprint(&f.ws.defn, &s, "still").unwrap());
    f.write(&f.text().replace("size: [140, 140, 90]", "size: [140, 140, 200]"));
    f.ws.reload().unwrap();
    let s = f.ws.select_one("s02_well").unwrap();
    assert_ne!(fp::render_fingerprint(&f.ws.defn, &s, "previz").unwrap(), pv); // the drawn box changed
    assert_eq!(fp::render_fingerprint(&f.ws.defn, &s, "still").unwrap(), still); // engine takes stay valid

    f.write(&f.text().replace("size: [140, 140, 200]", "size: [140, 0, 200]"));
    let err = f.ws.reload().unwrap_err();
    assert!(err.msg.contains("anchors.well") && err.msg.contains("size"), "{}", err.msg);
}

#[test]
fn framing_is_measured_from_the_subject_box() {
    let mut f = ws();
    // camera inside the well's box
    f.ws.set_fields("s03_window", &set(&[("camera", json!({"anchor": "well", "offset": [10, 10, 0]}))])).unwrap();
    assert!(rules(&f).contains(&"camera.inside:s03_window".to_string()), "{:?}", rules(&f));
    // the lantern (45 cm) from 40 m at 18mm: lost in the frame
    f.ws.set_fields("s03_window", &set(&[("camera", json!({"anchor": "lantern", "offset": [-4000, 0, 0], "lens": {"focal_length": 18}}))])).unwrap();
    assert!(rules(&f).contains(&"frame.lost:s03_window".to_string()), "{:?}", rules(&f));
    // the gate (260 cm tall) from 1 m at 85mm: a sliver
    f.ws.set_fields("s03_window", &set(&[("camera", json!({"anchor": "gate", "offset": [0, -100, 0], "lens": {"focal_length": 85}}))])).unwrap();
    assert!(rules(&f).contains(&"frame.cropped:s03_window".to_string()), "{:?}", rules(&f));
    let s = f.ws.select_one("s03_window").unwrap();
    let fr = shotpilot::shotcheck::framing(&f.ws.defn, &s).unwrap();
    assert_eq!(fr.subject, "gate");
    assert!(fr.known && fr.fill.0 > 4.0);
}

#[test]
fn scan_brings_sizes_from_the_level() {
    let mut f = ws();
    let level = json!({
        "well": {"at": [0, 0, 80], "size": [150, 150, 95], "center": [0, 0, 47]},
        "tower": {"at": [1200, 0, 900], "size": [400, 400, 1800]},
        "marker": [5, 5, 0],
    });
    let store = f.ws.root.join(".shotpilot/mock_engine");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("level.json"), level.to_string()).unwrap();
    let rep = f.ws.scan("shotpilot", "SP_").unwrap();
    assert_eq!(rep.added, ids(&["marker", "tower"]));
    assert_eq!(rep.moved, ids(&["well"])); // same point, new size
    assert_eq!(f.ws.defn.known_box("well").unwrap().size, [150.0, 150.0, 95.0]);
    assert_eq!(f.ws.defn.known_box("tower").unwrap().center, [1200.0, 0.0, 900.0]);
    assert!(f.ws.defn.known_box("marker").is_none());
    assert!(f.text().contains("# Named points in the level"));
}

#[test]
fn a_prop_popping_in_rerenders_only_its_shot() {
    let mut f = ws();
    f.ws.render(&[], "still", false, None).unwrap();
    let before = launches(&f.ws).len();
    let ap = json!([{"prop": "stranger", "at": 1.5, "anchor": "door", "offset": [150, 0, -120]}]);
    let rep = f.ws.set_fields("s05_door", &set(&[("appear", ap)])).unwrap();
    assert_eq!(rep.stale, ids(&["s05_door"]));
    let out = f.ws.render(&[], "still", false, None).unwrap();
    let acts = common::actions(&out);
    assert_eq!(acts["s05_door"], "rendered");
    assert!(acts.iter().filter(|(k, _)| *k != "s05_door").all(|(_, v)| v == "reused"), "{acts:?}");
    assert_eq!(launches(&f.ws).len(), before + 1);

    let s = f.ws.select_one("s05_door").unwrap();
    let a = &s.appear[0];
    assert_eq!(a.frame, 45); // 1.5 s at 30 fps
    assert_eq!(a.location, [-550.0, 500.0, 0.0]);
    assert_eq!(a.kind, "skeletal");
    // faces the camera: the camera is to the -x/-y side of the door
    let to_cam = (s.start.location[1] - a.location[1]).atan2(s.start.location[0] - a.location[0]).to_degrees();
    assert!((a.yaw - to_cam).abs() < 1.0, "{} vs {to_cam}", a.yaw);
    assert!(s.engine_payload()["appear"].is_array());
    // shots without props keep their payload (and fingerprints) as they were
    assert!(f.ws.select_one("s04_lantern").unwrap().engine_payload().get("appear").is_none());
}

#[test]
fn appear_is_validated() {
    let mut f = ws();
    let bad_time = json!([{"prop": "stranger", "at": 9, "anchor": "door"}]);
    let err = f.ws.set_fields("s05_door", &set(&[("appear", bad_time)])).unwrap_err();
    assert!(err.msg.contains("outside the shot"), "{}", err.msg);
    let unknown = json!([{"prop": "ghost", "at": 1, "anchor": "door"}]);
    let err = f.ws.set_fields("s05_door", &set(&[("appear", unknown)])).unwrap_err();
    assert!(err.msg.contains("unknown prop 'ghost'"), "{}", err.msg);
    f.write(&f.text().replace("kind: skeletal", "kind: rigged"));
    let err = f.ws.reload().unwrap_err();
    assert!(err.msg.contains("props.stranger"), "{}", err.msg);
}

#[test]
fn previz_shows_the_prop_only_from_its_frame() {
    let mut f = ws();
    let ap = json!([{"prop": "stranger", "at": 2, "anchor": "door", "offset": [150, 0, -120]}]);
    f.ws.set_fields("s05_door", &set(&[("appear", ap)])).unwrap();
    let s = f.ws.select_one("s05_door").unwrap();
    let boxes = f.ws.defn.anchor_boxes();
    let png = |shot: &shotpilot::model::ResolvedShot, frame| shotpilot::previz::render_frame(shot, frame, &boxes, 320, 180, 0.0, None).png();
    let mut without = s.clone();
    without.appear.clear();
    assert_eq!(png(&s, 30), png(&without, 30)); // 1 s: not there yet
    assert_ne!(png(&s, 90), png(&without, 90)); // 3 s: there
}

/// A 1x1 PNG.
const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0];

struct FakeImages(std::cell::RefCell<Vec<String>>);
impl shotpilot::moodboard::ImageGen for FakeImages {
    fn generate(&self, job: &shotpilot::moodboard::ImageJob) -> shotpilot::Result<Vec<u8>> {
        let base = job.base.as_ref().map(|b| format!(" [base {}]", b.file_name().unwrap().to_string_lossy())).unwrap_or_default();
        self.0.borrow_mut().push(format!("{}{base}", job.prompt));
        Ok(PNG.to_vec())
    }
}

#[test]
fn the_mood_board_is_editorial_and_reaches_the_planner() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let gen = FakeImages(Default::default());
    let made = f.ws.generate_moodboard(&gen, "冷たい青、霧", 4).unwrap();
    assert_eq!(made.len(), 4);
    assert!(made.iter().all(|m| m.starts_with("moodboard/gen-") && f.ws.root.join(m).is_file()));
    let prompts = gen.0.borrow();
    assert!(prompts[0].contains("Genre: 一人称ホラー") && prompts[0].contains("Art direction: 冷たい青、霧"));
    assert!(prompts.iter().all(|p| p.contains("No text, no logos")));
    assert_eq!(f.ws.defn.brief.images, made);
    // nothing re-renders
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));

    let own = f.ws.add_moodboard_image("my shot.png", PNG).unwrap();
    assert!(own.starts_with("moodboard/my_shot-"));
    assert!(f.ws.add_moodboard_image("x.txt", b"hello").is_err());
    f.ws.remove_moodboard_image(&made[0]).unwrap();
    assert!(!f.ws.root.join(&made[0]).exists());
    assert_eq!(f.ws.defn.brief.images.len(), 4);
    assert!(f.ws.remove_moodboard_image("moodboard/nope.png").is_err());

    let blocks = shotpilot::moodboard::image_blocks(&f.ws.defn, 4);
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[0]["source"]["media_type"], "image/png");
}

#[test]
fn base64_round_trips() {
    for data in [&b""[..], b"a", b"ab", b"abc", b"abcd", PNG] {
        let enc = shotpilot::ai::base64(data);
        assert_eq!(shotpilot::moodboard::base64_decode(&enc).unwrap(), data);
    }
}

#[test]
fn codex_output_is_read_back_from_its_image_folder() {
    let out = "{\"type\":\"thread.started\",\"thread_id\":\"01ab\"}\n{\"type\":\"turn.started\"}\n";
    assert_eq!(shotpilot::moodboard::codex_thread_id(out).as_deref(), Some("01ab"));
    assert!(shotpilot::moodboard::codex_thread_id("garbage").is_none());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), PNG).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let mut second = PNG.to_vec();
    second.push(7);
    std::fs::write(dir.path().join("b.png"), &second).unwrap();
    std::fs::write(dir.path().join("notes.txt"), "x").unwrap();
    let imgs = shotpilot::moodboard::codex_images(dir.path());
    assert_eq!(imgs.len(), 2);
    assert_eq!(imgs[1], second); // oldest first: the order they were asked for
    use shotpilot::moodboard::{CodexImages, ImageJob};
    let shot = |p: &str, b: Option<&str>| ImageJob { prompt: p.into(), base: b.map(std::path::PathBuf::from) };
    let (ins, attach) = CodexImages::instruction(&[shot("one", Some("a.png")), shot("two", None), shot("three", Some("a.png"))]);
    assert!(ins.contains("exactly 3 image(s)") && ins.contains("--- Image 2 ---\ntwo") && ins.contains("Do not write"));
    assert_eq!(attach.len(), 1); // the same scout photo is attached once
    assert!(ins.contains("Start from attached image #1"));
}

#[test]
fn scouting_photographs_anchors_without_touching_shots() {
    let mut f = ws();
    let before = f.text();
    f.ws.render(&[], "previz", false, None).unwrap();
    let idx = f.ws.take_scout(&[]).unwrap();
    let anchors = idx["anchors"].as_object().unwrap();
    assert_eq!(anchors.len(), 5);
    let well = anchors["well"].as_array().unwrap();
    assert_eq!(well.len(), 4);
    assert!(well.iter().all(|v| f.ws.root.join(v["file"].as_str().unwrap()).is_file()));
    // scout cameras frame the anchor from outside its box
    let b = f.ws.defn.known_box("well").unwrap();
    for v in well {
        let loc: Vec<f64> = v["location"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        assert!(!b.contains([loc[0], loc[1], loc[2]], 0.0));
    }
    assert_eq!(f.text(), before); // shots.yaml untouched
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
    // re-scouting one anchor keeps the others
    let idx = f.ws.take_scout(&ids(&["door"])).unwrap();
    assert_eq!(idx["anchors"].as_object().unwrap().len(), 5);
    // the planner gets one labelled photo per anchor
    let blocks = shotpilot::scout::planner_blocks(&f.ws.defn, 8);
    assert_eq!(blocks.len(), 10);
    assert!(blocks[0]["text"].as_str().unwrap().starts_with("Location scout: `"));
    assert_eq!(blocks[1]["type"], "image");
}

#[test]
fn assets_become_props_and_props_in_use_stay() {
    let mut f = ws();
    let list = f.ws.scan_assets().unwrap();
    assert!(list.iter().any(|a| a.asset == "/Game/Mock/Props/SM_Crate" && a.size == Some([100.0, 100.0, 80.0])));
    let crate_ = list.iter().find(|a| a.asset.ends_with("SM_Crate")).unwrap();
    assert_eq!(crate_.suggested_name(), "crate");
    f.ws.add_prop("crate", "/Game/Mock/Props/SM_Crate", None, None).unwrap();
    let p = f.ws.defn.prop("crate").unwrap();
    assert_eq!((p.kind.as_str(), p.size), ("static", [100.0, 100.0, 80.0])); // from the inventory
    assert!(f.text().contains("# Things a shot can make appear"));
    assert!(f.ws.add_prop("Bad Name", "/Game/X/Y", None, None).is_err());

    let ap = json!([{"prop": "crate", "at": 1, "anchor": "door"}]);
    f.ws.set_fields("s05_door", &set(&[("appear", ap)])).unwrap();
    let err = f.ws.remove_prop("crate").unwrap_err();
    assert!(err.msg.contains("s05_door"), "{}", err.msg);
    f.ws.set_fields("s05_door", &set(&[("appear", json!(null))])).unwrap();
    f.ws.remove_prop("crate").unwrap();
    assert!(f.ws.defn.prop("crate").is_none());
}

#[test]
fn the_stage_is_read_from_scout_photos_and_grounds_the_mood_board() {
    use shotpilot::ai::{self, Claude};
    struct Reader(std::cell::RefCell<Vec<serde_json::Value>>);
    impl Claude for Reader {
        fn ask(&self, request: &serde_json::Value) -> shotpilot::Result<String> {
            self.0.borrow_mut().push(request.clone());
            Ok(json!({"summary": "夕暮れの中庭。中央に井戸。",
                      "anchors": [{"name": "well", "description": "石の井戸"}, {"name": "ghost", "description": "x"}],
                      "mood": {"place": "gate", "detail": "lantern", "threat": "window", "title": "nowhere"}}).to_string())
        }
    }
    let mut f = ws();
    let reader = Reader(Default::default());
    assert!(ai::understand_stage(&f.ws.defn, &reader).unwrap_err().msg.contains("scout photos"));
    f.ws.take_scout(&[]).unwrap();
    let st = ai::understand_stage(&f.ws.defn, &reader).unwrap();
    assert_eq!(st.anchors, vec![("well".to_string(), "石の井戸".to_string())]); // unknown anchors dropped
    assert_eq!(st.mood.len(), 3); // `nowhere` is not an anchor
    let req = &reader.0.borrow()[0];
    assert_eq!(req["messages"][0]["content"][1]["type"], "image");
    assert!(shotpilot::craft::self_review_words(req["system"].as_str().unwrap()).is_empty());

    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.set_stage(&st).unwrap();
    assert_eq!(f.ws.defn.stage.summary, "夕暮れの中庭。中央に井戸。");
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));

    let gen = FakeImages(Default::default());
    f.ws.generate_moodboard(&gen, "", 4).unwrap();
    let p = gen.0.borrow();
    assert!(p[0].contains("The set: 夕暮れの中庭") && p[0].ends_with("[base gate-front.png]"), "{}", p[0]);
    assert!(p[1].ends_with("[base lantern-front.png]"));
    assert!(!p[3].contains("[base")); // title had no valid anchor: plain generation
}
