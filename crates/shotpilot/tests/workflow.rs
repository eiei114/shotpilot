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

struct BoardImages(std::cell::RefCell<Vec<shotpilot::moodboard::ImageJob>>);
impl shotpilot::moodboard::ImageGen for BoardImages {
    fn generate(&self, job: &shotpilot::moodboard::ImageJob) -> shotpilot::Result<Vec<u8>> {
        self.0.borrow_mut().push(job.clone());
        Ok(PNG.to_vec())
    }
}

#[test]
fn storyboard_frames_are_a_preview_outside_the_takes() {
    use shotpilot::storyboard;
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.take_scout(&[]).unwrap();
    f.ws.add_moodboard_image("mood.png", PNG).unwrap();
    let before_launches = launches(&f.ws).len();
    let before_yaml = f.text();
    let gen = BoardImages(Default::default());

    let out = f.ws.generate_storyboard(&gen, &[], "冷たい青", false).unwrap();
    assert_eq!(out.len(), 6);
    assert!(out.iter().all(|o| o.action == "made" && o.file.starts_with(f.ws.root.join("storyboard"))));
    {
        let jobs = gen.0.borrow();
        assert_eq!(jobs.len(), 6);
        for j in jobs.iter() {
            assert!(j.layout && j.base.as_ref().unwrap().to_string_lossy().ends_with(".layout.png"));
            assert!(j.base.as_ref().unwrap().is_file());
            assert!(j.prompt.contains("photorealistic") && j.prompt.contains("No text") && j.prompt.contains("Art direction: 冷たい青"));
            assert!(j.refs.last().unwrap().to_string_lossy().contains("moodboard")); // the mood board sets the palette
        }
        // the scout photos of what is in frame come along as references
        assert!(jobs.iter().any(|j| j.refs.iter().any(|r| r.starts_with(f.ws.root.join("scout")))));
        let (ins, _) = shotpilot::moodboard::CodexImages::instruction(&jobs[..1]);
        assert!(ins.contains("Attached image #1 is a wireframe layout") && ins.contains("frames from the actual game"));
        assert!(jobs[0].attachment_note().starts_with("The first attached image is a wireframe layout"));
    }
    // a preview only: no engine launch, no take, shots.yaml untouched, nothing goes stale
    assert_eq!(launches(&f.ws).len(), before_launches);
    assert_eq!(f.text(), before_yaml);
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
    assert!(!f.ws.root.join("render").join("s01_gate").join("storyboard").exists());

    // frames that match their shot are kept
    let again = f.ws.generate_storyboard(&gen, &[], "", false).unwrap();
    assert!(again.iter().all(|o| o.action == "kept"));
    assert_eq!(gen.0.borrow().len(), 6);

    // editorial edits keep the frame; a picture edit makes only that shot's frame stale
    f.ws.set_fields("s02_well", &set(&[("purpose", json!("別の説明")), ("notes", json!("memo"))])).unwrap();
    f.ws.set_fields("s03_window", &set(&[("camera.lens.focal_length", json!(85))])).unwrap();
    let fresh = |f: &common::Fixture, id: &str| {
        let s = f.ws.resolved().unwrap().into_iter().find(|s| s.id == id).unwrap();
        storyboard::current(&f.ws.defn, &s).unwrap().fresh
    };
    assert!(fresh(&f, "s02_well"));
    assert!(!fresh(&f, "s03_window")); // the old frame is still shown, marked stale
    let out = f.ws.generate_storyboard(&gen, &[], "", false).unwrap();
    let made: Vec<&str> = out.iter().filter(|o| o.action == "made").map(|o| o.shot_id.as_str()).collect();
    assert_eq!(made, ["s03_window"]);
    assert!(fresh(&f, "s03_window"));
    // forcing repaints in place
    let out = f.ws.generate_storyboard(&gen, &ids(&["s01_gate"]), "", true).unwrap();
    assert_eq!(out[0].action, "made");
    let files = std::fs::read_dir(f.ws.root.join("storyboard").join("s01_gate")).unwrap().count();
    assert_eq!(files, 2); // one frame and its layout
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
    let shot = |p: &str, b: Option<&str>| ImageJob { prompt: p.into(), base: b.map(std::path::PathBuf::from), ..Default::default() };
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

/// A new project with no anchors, as the home makes it.
fn empty_project() -> common::Fixture {
    use shotpilot::templates::{self, EMPTY};
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("shots.yaml");
    std::fs::write(&p, templates::render(EMPTY, "t", "mock", "null", "/Game/Maps/Courtyard")).unwrap();
    let ws = shotpilot::Workspace::open(&p).unwrap();
    common::Fixture { dir, ws }
}

#[test]
fn scouting_an_empty_project_brings_the_level_in_one_launch() {
    let mut f = empty_project();
    assert!(f.ws.defn.anchors.is_empty());
    // nothing tagged in the level: the engine offers landmarks it picked
    let store = f.ws.root.join(".shotpilot/mock_engine");
    std::fs::create_dir_all(&store).unwrap();
    let marks = json!({
        "bed": {"at": [0, 0, 0], "size": [200, 140, 60], "center": [0, 0, 30]},
        "wardrobe": {"at": [600, 300, 0], "size": [120, 60, 200], "center": [600, 300, 100]},
    });
    std::fs::write(store.join("landmarks.json"), marks.to_string()).unwrap();

    let rep = f.ws.scout(&[], None).unwrap();
    let scan = rep.scan.as_ref().unwrap();
    assert!(scan.auto);
    assert_eq!(scan.picked_by, "spread"); // no Claude: the most spread-out landmarks
    assert_eq!(scan.added, ids(&["bed", "wardrobe"]));
    assert_eq!(f.ws.defn.known_box("wardrobe").unwrap().size, [120.0, 60.0, 200.0]);
    assert_eq!(rep.assets, Some(2)); // the inventory came along
    assert_eq!(shotpilot::scout::load_assets(&f.ws.root).len(), 2);
    assert_eq!(rep.index["anchors"]["bed"].as_array().unwrap().len(), 4);
    // one launch gathered both, then the photos (previz, no engine)
    let kinds: Vec<String> = launches(&f.ws).iter().map(|l| l["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ids(&["survey"]));
    assert_eq!(launches(&f.ws)[0]["assets"], true);

    // with anchors in shots.yaml, scouting again only takes photos
    let rep = f.ws.scout(&ids(&["bed"]), None).unwrap();
    assert!(rep.scan.is_none() && rep.assets.is_none());
    assert_eq!(launches(&f.ws).len(), 1);
}

#[test]
fn scouting_prefers_tagged_actors_and_keeps_the_inventory() {
    let mut f = empty_project();
    let store = f.ws.root.join(".shotpilot/mock_engine");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("level.json"), json!({"altar": {"at": [0, 0, 0], "size": [100, 100, 100], "center": [0, 0, 50]}}).to_string()).unwrap();
    std::fs::write(store.join("landmarks.json"), json!({"bed": [500, 0, 0]}).to_string()).unwrap();
    f.ws.scan_assets().unwrap();
    let rep = f.ws.scout(&[], None).unwrap();
    let scan = rep.scan.unwrap();
    assert!(!scan.auto);
    assert_eq!(scan.added, ids(&["altar"]));
    assert!(rep.assets.is_none()); // already inventoried: not asked again
    assert_eq!(launches(&f.ws).last().unwrap()["assets"], false);
}

#[test]
fn scouting_a_level_with_nothing_to_find_says_what_to_do() {
    let mut f = empty_project();
    let before = f.text();
    let err = f.ws.scout(&[], None).unwrap_err();
    assert!(err.msg.contains("anchors:"), "{}", err.msg);
    assert_eq!(f.text(), before);
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

#[test]
fn the_finish_is_definition_and_never_rerenders() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.set_finish(&set(&[
        ("letterbox", json!(2.39)),
        ("title", json!({"text": "夜の中庭", "at": 1, "duration": 4})),
        ("end_card", json!({"lines": ["Steam でウィッシュリスト登録受付中"], "duration": 3})),
    ]))
    .unwrap();
    let rep = f.ws.set_fields("s02_well", &set(&[("caption", json!("住人が、一人ずつ消えていく")), ("transition", json!("dip"))])).unwrap();
    assert!(rep.stale.is_empty());
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
    assert_eq!(f.ws.defn.finish.letterbox, Some(2.39));
    assert!(f.text().contains("# Shotpilot shot definition"));

    // bad values are refused before anything is written
    assert!(f.ws.set_finish(&set(&[("letterbox", json!(9))])).unwrap_err().msg.contains("letterbox"));
    assert!(f.ws.set_finish(&set(&[("logo", json!("x"))])).unwrap_err().msg.contains("unknown finish field"));
    assert!(f.ws.set_fields("s03_window", &set(&[("transition", json!("wipe"))])).unwrap_err().msg.contains("transition"));
    assert!(f.ws.set_fields("s03_window", &set(&[("caption", json!({"text": "x", "at": 9}))])).unwrap_err().msg.contains("caption.at"));

    // the burn-in graph: each shot cut out with its dip, bars, caption, the title on the title
    // shot (local time), the end card after the last piece
    use shotpilot::finish::{self, CutPiece, Fonts, ShotPiece};
    let shots = f.ws.resolved().unwrap();
    let mut t = 0.0;
    let pieces: Vec<CutPiece> = shots.iter().map(|s| {
        let raw = &f.ws.defn.shots[s.index];
        let p = CutPiece::Shot(ShotPiece { concat_start: t, duration: s.duration, beat: s.beat.clone(), caption: finish::caption(raw),
                                           transition: finish::transition(raw), exposure: 0.0, overlays: Vec::new() });
        t += s.duration;
        p
    }).collect();
    let dir = tempfile::tempdir().unwrap();
    let fonts = Fonts { main: "C:/Windows/Fonts/x.ttf".into(), sans: "C:/Windows/Fonts/y.ttc".into() };
    let (g, total) = finish::filter_graph(&f.ws.defn.finish, &pieces, 1280, 720, 30, Some(&fonts), dir.path()).unwrap();
    assert!((total - (27.5 + 3.0)).abs() < 1e-6);
    assert!(g.contains("split=6") && g.contains("fade=t=out") && g.contains("fade=t=in"), "{g}");
    assert!(g.contains("drawbox=x=0:y=0:w=iw:h=92"), "{g}"); // (720 - 1280/2.39) / 2
    assert!(g.contains(r"fontfile='C\:/Windows/Fonts/x.ttf'"));
    assert!(g.contains("color=c=black:s=1280x720") && g.contains("concat=n=7"));
    let texts: Vec<String> = (1..=3).map(|i| std::fs::read_to_string(dir.path().join(format!("text{i}.txt"))).unwrap()).collect();
    assert!(texts.contains(&"住人が、一人ずつ消えていく".to_string()) && texts.contains(&"夜の中庭".to_string()));
    // the title sits on the title shot, in its own time (at 1 s for 4 s)
    assert!(g.contains("if(lt(t,1.000),0"), "{g}");
}

#[test]
fn the_script_puts_cards_between_shots_and_keeps_sound_on_its_shot() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let (starts_before, total_before) = shotpilot::sound::timing(&f.ws.defn, &f.ws.resolved().unwrap());
    let mut text = f.text();
    text.push_str(r#"
script:
  - {id: intro, kind: card, ja: 今日から、ここに住む。, en: "TODAY, I MOVE IN.", dur: 3, after: start}
  - {id: rooms, kind: caption, ja: 部屋は、六つ。, on: s02_well, at: 0.4, dur: 2.6}
  - {id: seven, kind: card, ja: 自分を入れて、七人。, dur: 2.4, after: s02_well}
  - {id: cups, kind: count, ja: コップ、七つ。, on: s03_window}
  - {id: gap, kind: black, dur: 0.8, after: s05_door}
  - {id: title, kind: title, ja: 七号室, en: ROOM SEVEN, dur: 4, after: gap}
"#);
    f.write(&text);
    f.ws.reload().unwrap();
    // editorial: nothing re-renders
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));

    let shots = f.ws.resolved().unwrap();
    let items = shotpilot::script::layout(&f.ws.defn.script, &shots, 30);
    let order: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(order, ["intro", "s01_gate", "s02_well", "seven", "s03_window", "s04_lantern", "s05_door", "gap", "title", "s06_title"]);
    // sound follows its shot: s01 now starts after the 3 s intro card, s03 after the card too
    let (starts, total) = shotpilot::sound::timing(&f.ws.defn, &shots);
    assert!((starts[0] - (starts_before[0] + 3.0)).abs() < 1e-6);
    assert!((starts[2] - (starts_before[2] + 3.0 + 2.4)).abs() < 1e-6);
    assert!((total - (total_before + 3.0 + 2.4 + 0.8 + 4.0)).abs() < 1e-6);
    assert_eq!(shotpilot::script::overlays(&f.ws.defn.script, "s03_window")[0].kind, "count");

    // bad references are refused
    f.write(&f.text().replace("after: s02_well}", "after: nowhere}"));
    let err = f.ws.reload().unwrap_err();
    assert!(err.msg.contains("'nowhere' is not a shot"), "{}", err.msg);
}

#[test]
fn the_first_pass_goes_all_the_way_in_one_go() {
    use shotpilot::ai::Claude;
    struct Director(std::cell::RefCell<Vec<String>>);
    impl Claude for Director {
        fn ask(&self, request: &serde_json::Value) -> shotpilot::Result<String> {
            let props = &request["output_config"]["format"]["schema"]["properties"];
            if common::is_discover(request) {
                self.0.borrow_mut().push("discover".into());
                return Ok(common::discovery_reply().to_string());
            }
            if props.get("summary").is_some() {
                self.0.borrow_mut().push("stage".into());
                return Ok(json!({"summary": "中庭", "anchors": [{"name": "well", "description": "井戸"}],
                                 "mood": {"place": "gate", "detail": "lantern", "threat": "window", "title": "well"}}).to_string());
            }
            self.0.borrow_mut().push("plan".into());
            let user = request["messages"][0]["content"].to_string();
            assert!(user.contains("## Script") && user.contains("## Stage") && user.contains("Location scout"), "{user}");
            let shot = |id: &str, beat: &str, d: f64, anchor: &str, cap: &str, tr: &str| json!({
                "id": id, "beat": beat, "purpose": "p", "duration": d, "anchor": anchor, "offset": [-400, -300, 100],
                "look_at": "", "focal_length": 35, "move": "static", "move_amount": 0, "time_of_day": "default",
                "notes": "", "appear": [], "caption": cap, "transition": tr});
            let line = |id: &str, kind: &str, ja: &str, after: &str, on: &str| json!({"id": id, "kind": kind, "ja": ja, "en": "",
                "head": "", "from": "", "dur": 2.0, "after": after, "on": on, "at": 0.5});
            Ok(json!({"rationale": "静かに始める", "grade": "horror", "letterbox": 2.39,
                      "script": [line("intro", "card", "今日から、ここに住む。", "start", ""), line("gone", "caption", "住人が消える", "", "e"),
                                 line("gap", "black", "", "x2", ""), line("ttl", "title", "夜の中庭", "gap", ""),
                                 line("lost", "caption", "x", "", "nowhere")],
                      "shots": [shot("h", "hook", 4.0, "lantern", "", "cut"), shot("e", "establish", 8.0, "gate", "住人が消える", "dip"),
                                shot("x", "escalate", 6.0, "window", "", "flash"), shot("x2", "escalate", 5.0, "door", "", "cut"),
                                shot("t", "title", 6.0, "well", "", "cut")]}).to_string())
        }
    }
    let mut f = ws();
    let claude = Director(Default::default());
    let gen = FakeImages(Default::default());
    let mut events = Vec::new();
    let mut on = |e: &str, d: &serde_json::Value| events.push((e.to_string(), d.clone()));
    let out = shotpilot::produce::produce(&mut f.ws, &claude, Some(&gen), &shotpilot::produce::ProduceOpts::default(), &mut on).unwrap();

    assert_eq!(*claude.0.borrow(), ids(&["discover", "stage", "plan"])); // one call each, no self-review rounds
    let done: Vec<String> = events.iter().filter(|(e, d)| e == "step" && d["status"] != "start")
        .map(|(_, d)| format!("{}:{}", d["key"].as_str().unwrap(), d["status"].as_str().unwrap())).collect();
    assert_eq!(done, ["discover:done", "scout:done", "stage:done", "mood:done", "plan:done", "sound:skipped", "render:done", "cut:done"]);
    assert_eq!(f.ws.defn.shot_ids(), ids(&["h", "e", "x", "x2", "t"]));
    assert_eq!(f.ws.defn.brief.images.len(), 4);
    // the script came with the shots: cards between them, a line over one, a broken line left out
    let script: Vec<(&str, &str)> = f.ws.defn.script.iter().map(|l| (l.id.as_str(), l.kind.as_str())).collect();
    assert_eq!(script, [("intro", "card"), ("gone", "caption"), ("gap", "black"), ("ttl", "title")]);
    assert_eq!(f.ws.defn.script[1].on.as_deref(), Some("e"));
    // the planning turn chose the look; its tokens set the finish
    assert_eq!(f.ws.defn.brief.design, "quiet-dread");
    assert_eq!(f.ws.defn.brief.skills, ids(&["rules-teaser", "title-drop"]));
    assert_eq!(f.ws.defn.finish.grade, "horror");
    assert_eq!(f.ws.defn.finish.letterbox, Some(2.39));
    assert_eq!(f.ws.defn.finish.typeface, "mincho");
    assert!(f.ws.defn.finish.title.is_none() && f.ws.defn.finish.end_card.is_none()); // the title is a script slot now
    assert_eq!(f.ws.defn.find("x").unwrap()["transition"], json!("flash"));
    assert!(f.ws.status().unwrap().iter().all(|r| r.quality("previz") == "fresh"));
    assert_eq!(out["shots"].as_array().unwrap().len(), 5);
    assert_eq!(f.ws.state.history().last().unwrap()["action"], json!("produce"));

    // a second pass reuses the scout, the stage and the mood board
    let mut events2 = Vec::new();
    let mut on2 = |e: &str, d: &serde_json::Value| events2.push((e.to_string(), d.clone()));
    shotpilot::produce::produce(&mut f.ws, &claude, Some(&gen), &shotpilot::produce::ProduceOpts::default(), &mut on2).unwrap();
    let skipped: Vec<String> = events2.iter().filter(|(e, d)| e == "step" && d["status"] == "skipped")
        .map(|(_, d)| d["key"].as_str().unwrap().to_string()).collect();
    assert_eq!(skipped, ["discover", "scout", "stage", "mood", "sound"]);
}

#[test]
fn music_cues_and_card_hits_follow_the_script() {
    let mut f = ws();
    let mut text = f.text();
    text.push_str(r#"
script:
  - {id: intro, kind: card, ja: 今日から、ここに住む。, dur: 3, after: start, sfx: [{file: audio/boom.wav, at: 0, gain: -6}]}
  - {id: seven, kind: card, ja: 自分を入れて、七人。, dur: 2, after: s02_well, sfx: [{file: audio/boom.wav, at: 0.1}]}
  - {id: rooms, kind: caption, ja: 部屋は、六つ。, on: s02_well, at: 0.5, dur: 2, sfx: [{file: audio/tick.wav, at: 0}]}
"#);
    f.write(&text.replace("  bgm: {file: audio/drone.wav, gain: -6, fade_in: 1.5, fade_out: 2}",
        "  bgm: {file: audio/drone.wav, gain: -6, fade_in: 1.5, fade_out: 2}\n  loudness: -16\n  music:\n    - {file: audio/tense.wav, from: seven, until: s05_door, gain: -9, label: tension}"));
    f.ws.reload().unwrap();
    let shots = f.ws.resolved().unwrap();
    let plan = shotpilot::sound::plan(&f.ws.defn, &shots);
    let at = |label: &str| plan.iter().find(|p| p.label == label).map(|p| (p.start, p.until)).unwrap();
    // intro 0-3, s01 3-8, s02 8-11.5, seven 11.5-13.5, s03 13.5-17.5, s04 17.5-21.5, s05 21.5-26.5
    let (s, u) = at("tension");
    assert!((s - 11.5).abs() < 1e-6 && (u.unwrap() - 26.5).abs() < 1e-6, "{s} {u:?}");
    let booms: Vec<f64> = plan.iter().filter(|p| p.label == "boom").map(|p| p.start).collect();
    assert!((booms[0] - 0.0).abs() < 1e-6 && (booms[1] - 11.6).abs() < 1e-6, "{booms:?}");
    assert!((at("tick").0 - 8.5).abs() < 1e-6); // the caption shows 0.5 s into s02
    // reordering shots moves the hits with their lines
    let order = ids(&["s01_gate", "s03_window", "s02_well", "s04_lantern", "s05_door", "s06_title"]);
    f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    let plan = shotpilot::sound::plan(&f.ws.defn, &f.ws.resolved().unwrap());
    let tick = plan.iter().find(|p| p.label == "tick").unwrap().start;
    assert!((tick - (3.0 + 5.0 + 4.0 + 0.5)).abs() < 1e-6, "{tick}");
    // a cue that starts on nothing is refused
    f.write(&f.text().replace("from: seven", "from: \"\""));
    assert!(f.ws.reload().unwrap_err().msg.contains("audio.music[0].from"));
}

#[test]
fn sound_design_picks_from_the_library_and_lands_on_the_pieces() {
    use shotpilot::ai::Claude;
    // a tiny library: a drone loop and an impact
    let libdir = tempfile::tempdir().unwrap();
    let sounds = libdir.path().join("sounds");
    std::fs::create_dir_all(sounds.join("splice/Horror")).unwrap();
    std::fs::write(sounds.join("splice/Horror/drone.wav"), b"RIFF-drone").unwrap();
    std::fs::write(sounds.join("splice/Horror/boom.wav"), b"RIFF-boom").unwrap();
    std::fs::write(sounds.join("library.json"), json!({"sounds": [
        {"id": "splice:drone", "path": "splice/Horror/drone.wav", "kind": "loop", "duration_s": 64.0, "tags": ["drone"], "status": "ok"},
        {"id": "splice:boom", "path": "splice/Horror/boom.wav", "kind": "oneshot", "duration_s": 2.0, "tags": ["impact"], "status": "ok"},
    ]}).to_string()).unwrap();
    let lib = shotpilot::library::Library::open(Some(libdir.path())).unwrap();

    let mut f = ws();
    let mut text = f.text();
    text.push_str("\nscript:\n  - {id: seven, kind: card, ja: 自分を入れて、七人。, dur: 2, after: s02_well}\n");
    f.write(&text);
    f.ws.reload().unwrap();
    struct Designer;
    impl Claude for Designer {
        fn ask(&self, request: &serde_json::Value) -> shotpilot::Result<String> {
            let user = request["messages"][0]["content"].as_str().unwrap();
            assert!(user.contains("card `seven`") && user.contains("splice:drone") && user.contains("Each text card lands"), "{user}");
            Ok(json!({"rationale": "低いドローンで始め、カードで落とす",
                      "music": [{"lib": "splice:drone", "from": "start", "until": "seven", "at": 0, "gain": -11, "fade_in": 1.5, "fade_out": 0.5, "label": "drone"}],
                      "hits": [{"lib": "splice:boom", "on": "seven", "at": 0, "gain": -4, "label": "boom"},
                               {"lib": "splice:boom", "on": "s05_door", "at": 1.5, "gain": -6, "label": "door"},
                               {"lib": "splice:nope", "on": "s05_door", "at": 0, "gain": 0, "label": "x"},
                               {"lib": "splice:boom", "on": "ghost", "at": 0, "gain": 0, "label": "y"}]}).to_string())
        }
    }
    let d = shotpilot::sounddesign::design(&f.ws, &lib, &Designer).unwrap();
    assert_eq!(d.notes.len(), 2, "{:?}", d.notes); // an unknown sound and an unknown piece were left out
    shotpilot::sounddesign::apply(&mut f.ws, &d).unwrap();
    assert!(f.ws.root.join("audio/lib").read_dir().unwrap().count() == 2); // both files copied in
    let music = f.ws.defn.audio["music"].as_array().unwrap();
    assert_eq!(music[0]["from"], json!("start"));
    assert_eq!(music[0]["until"], json!("seven"));
    assert_eq!(f.ws.defn.script[0].sfx.len(), 1);
    assert_eq!(f.ws.defn.find("s05_door").unwrap()["sfx"][0]["label"], json!("door"));
    assert!(f.ws.defn.find("s01_gate").unwrap().get("sfx").is_none()); // a fresh design clears the rest
    assert_eq!(f.ws.defn.audio["loudness"], json!(-16.0));
    // nothing re-renders for sound
    assert!(f.ws.state.history().iter().all(|h| h["action"] != "render" || h["stale"].is_null()));
    let plan = shotpilot::sound::plan(&f.ws.defn, &f.ws.resolved().unwrap());
    let boom = plan.iter().find(|p| p.label == "boom").unwrap();
    assert!((boom.start - 8.5).abs() < 1e-6, "{}", boom.start); // the card starts after s01 (5 s) and s02 (3.5 s)
}

#[test]
fn a_fresh_plan_keeps_the_sound_on_the_pieces_it_replaces() {
    let mut f = ws();
    let mut text = f.text();
    text.push_str(r#"
script:
  - {id: seven, kind: card, ja: 自分を入れて、七人。, dur: 2, after: s02_well, sfx: [{file: audio/boom.wav, at: 0}]}
  - {id: c4, kind: caption, ja: 灯りが消えた。, on: s04_lantern, at: 0.5, dur: 2, sfx: [{file: audio/tick.wav, at: 0}]}
"#);
    f.write(&text.replace("  bgm: {file: audio/drone.wav, gain: -6, fade_in: 1.5, fade_out: 2}",
        "  bgm: {file: audio/drone.wav, gain: -6, fade_in: 1.5, fade_out: 2}\n  music:\n    - {file: audio/tense.wav, from: s04_lantern, until: seven, label: tension}"));
    f.ws.reload().unwrap();
    f.ws.edit_sound(&[("s03_window".into(), Some(json!([{"file": "audio/hit.wav", "at": 0.5}])))], None, "").unwrap();

    // Claude re-plans: s04 comes back under a new id, the caption on it too, nothing carries sound
    let shots: Vec<serde_json::Value> = f.ws.defn.shots.iter().map(|s| {
        let mut s = s.clone();
        s.as_object_mut().unwrap().remove("sfx");
        if s["id"] == "s04_lantern" {
            s["id"] = json!("s04_yard");
        }
        s
    }).collect();
    let script = vec![
        json!({"id": "intro", "kind": "card", "ja": "今日から。", "dur": 2, "after": "start"}),
        json!({"id": "seven", "kind": "card", "ja": "七人。", "dur": 2, "after": "s02_well"}),
        json!({"id": "n4", "kind": "caption", "ja": "灯りが、消えた。", "on": "s04_yard", "at": 0.4, "dur": 2}),
    ];
    f.ws.replace_plan(&shots, Some(&script)).unwrap();

    let line = |id: &str| f.ws.defn.script.iter().find(|l| l.id == id).unwrap().sfx.clone();
    assert_eq!(line("seven")[0]["file"], "audio/boom.wav");
    assert_eq!(line("n4")[0]["file"], "audio/tick.wav");
    assert!(line("intro").is_empty());
    assert_eq!(f.ws.defn.find("s03_window").unwrap()["sfx"][0]["file"], "audio/hit.wav");
    let cue = &f.ws.defn.audio["music"][0];
    assert_eq!((cue["from"].as_str(), cue["until"].as_str()), (Some("s04_yard"), Some("seven")));
}

#[test]
fn a_music_cue_on_a_missing_piece_is_reported() {
    let mut f = ws();
    f.write(&f.text().replace("  bgm: {file: audio/drone.wav, gain: -6, fade_in: 1.5, fade_out: 2}",
        "  music:\n    - {file: audio/tense.wav, from: s04_dining, label: tension}\n    - {file: audio/hum.wav, from: s02_well, until: gone}"));
    f.ws.reload().unwrap();
    let o = shotpilot::sound::orphans(&f.ws.defn, &f.ws.resolved().unwrap());
    let got: Vec<(usize, &str, &str)> = o.iter().map(|o| (o.cue, o.key, o.target.as_str())).collect();
    assert_eq!(got, vec![(0, "from", "s04_dining"), (1, "until", "gone")]);
    assert!(o[0].text().contains("never played"));
}

/// The engine's frames, once after the first shoot (D-50): Claude says whether each subject is
/// in the picture; a shot whose subject is not gets its camera fixed once and only it is filmed
/// again. A second miss stays on the cut sheet; there is no second round.
struct Crew {
    calls: std::cell::RefCell<Vec<String>>,
    /// subject_visible for s03_window on each look
    s03: std::cell::RefCell<Vec<&'static str>>,
}

impl shotpilot::ai::Claude for Crew {
    fn ask(&self, request: &serde_json::Value) -> shotpilot::Result<String> {
        let props = &request["output_config"]["format"]["schema"]["properties"];
        if props["shots"]["items"]["properties"].get("subject_visible").is_some() {
            self.calls.borrow_mut().push("see".into());
            let labels: Vec<String> = request["messages"][0]["content"].as_array().unwrap().iter()
                .filter_map(|b| b["text"].as_str()).filter(|t| t.starts_with("Shot `"))
                .map(|t| t["Shot `".len()..].split('`').next().unwrap().to_string()).collect();
            let s03 = self.s03.borrow_mut().remove(0);
            let shots: Vec<serde_json::Value> = labels.iter().map(|id| {
                let v = if id == "s03_window" { s03 } else { "yes" };
                json!({"shot": id, "subject_visible": v, "blocked_by": if v == "no" { "壁" } else { "" }, "note": "n"})
            }).collect();
            return Ok(json!({"shots": shots}).to_string());
        }
        if props.get("edits").is_some() {
            self.calls.borrow_mut().push("revise".into());
            let user = request["messages"][0]["content"].to_string();
            assert!(user.contains("`window` が写っていない（壁が遮っている）"), "{user}");
            assert_eq!(request["messages"][0]["content"][0]["type"], "image"); // the frame it saw
            let yaml = "id: s03_window\npurpose: 2階の窓\nduration: 4\ncamera:\n  anchor: window\n  offset: [-500, -200, -100]\n  lens: {focal_length: 50}\n";
            return Ok(json!({"summary": "引いた", "edits": [{"id": "s03_window", "yaml": yaml, "reason": "r"}], "order": [], "audio": ""}).to_string());
        }
        panic!("unexpected request");
    }
}

fn rendered(ws: &shotpilot::workspace::Workspace, quality: &str) -> Vec<String> {
    let tag = format!(":{quality}");
    launches(ws).iter().filter_map(|l| l["shots"].as_array().cloned()).flatten()
        .filter_map(|s| s.as_str().and_then(|s| s.strip_suffix(&tag)).map(str::to_string)).collect()
}

#[test]
fn after_the_first_shoot_only_a_shot_whose_subject_is_hidden_is_fixed_once() {
    let mut f = ws();
    let crew = Crew { calls: Default::default(), s03: std::cell::RefCell::new(vec!["no", "no"]) };
    let opts = shotpilot::produce::ProduceOpts { quality: "still".into(), mood: false, ..Default::default() };
    let mut events = Vec::new();
    let mut on = |e: &str, d: &serde_json::Value| events.push((e.to_string(), d.clone()));
    let out = shotpilot::produce::resume(&mut f.ws, &crew, &opts, &mut on).unwrap();

    // one look, one repair, one look at the repaired shot, and no more
    assert_eq!(*crew.calls.borrow(), ids(&["see", "revise", "see"]));
    let mut once = rendered(&f.ws, "still");
    once.sort();
    let mut want = f.ws.defn.shot_ids();
    want.push("s03_window".into());
    want.sort();
    assert_eq!(once, want, "only s03_window is filmed twice");
    let done: Vec<String> = events.iter().filter(|(e, d)| e == "step" && d["status"] != "start")
        .map(|(_, d)| format!("{}:{}", d["key"].as_str().unwrap(), d["status"].as_str().unwrap())).collect();
    assert!(done.contains(&"see:done".to_string()) && done.contains(&"repair:done".to_string()), "{done:?}");
    assert!(out["notes"].to_string().contains("s03_window"), "{}", out["notes"]);
    // still hidden after the repair: it stays on the cut sheet, tied to the take it was seen in
    let blocked: Vec<_> = f.ws.check().unwrap().into_iter().filter(|x| x.rule == "frame.blocked").collect();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].shot.as_deref(), Some("s03_window"));
    assert!(blocked[0].message.contains("(壁)"), "{}", blocked[0].message);
    // filming it again drops what was seen in the old take
    f.ws.render(&ids(&["s03_window"]), "still", true, None).unwrap();
    assert!(f.ws.check().unwrap().iter().all(|x| x.rule != "frame.blocked"));
}

#[test]
fn previz_and_clear_frames_need_no_repair() {
    // previz draws boxes: nothing to look at
    let mut f = ws();
    let crew = Crew { calls: Default::default(), s03: std::cell::RefCell::new(vec!["yes"]) };
    let mut on = |_: &str, _: &serde_json::Value| {};
    shotpilot::produce::resume(&mut f.ws, &crew, &shotpilot::produce::ProduceOpts { mood: false, ..Default::default() }, &mut on).unwrap();
    assert!(crew.calls.borrow().is_empty());
    // every subject in the picture: one look, nothing filmed again
    shotpilot::produce::resume(&mut f.ws, &crew, &shotpilot::produce::ProduceOpts { quality: "still".into(), mood: false, ..Default::default() }, &mut on).unwrap();
    assert_eq!(*crew.calls.borrow(), ids(&["see"]));
    assert_eq!(rendered(&f.ws, "still").len(), f.ws.defn.shot_ids().len());
}
