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
    fn generate(&self, prompt: &str) -> shotpilot::Result<Vec<u8>> {
        self.0.borrow_mut().push(prompt.to_string());
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
