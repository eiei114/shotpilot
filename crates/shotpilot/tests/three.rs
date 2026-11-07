//! The three engine (D-72): the sample game films without UE, and the loop holds on it.
//!
//! The browser is replaced by a stand-in that speaks the same HTTP to the loopback server
//! (`three::use_stand_in_browser`): it fetches the job and posts a blank frame for each, so
//! these tests run the real server, the frame files, what each take recorded as on screen and
//! what goes stale, without starting Edge or Chrome.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use common::{actions, set};
use serde_json::{json, Value};
use shotpilot::{templates, three, threegame, Workspace};

fn open(dir: &Path, text: &str) -> Workspace {
    three::use_stand_in_browser(true);
    let p = dir.join("shots.yaml");
    std::fs::write(&p, text).unwrap();
    Workspace::open(&p).unwrap()
}

/// The sample: the Lanternless teaser and the ward it films.
fn ward() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    assert!(threegame::write(&dir.path().join("game")).unwrap());
    let ws = open(dir.path(), &templates::render(templates::THREE_WARD, "ward", "three", "null", "null"));
    (dir, ws)
}

fn launches(ws: &Workspace) -> Vec<Value> {
    std::fs::read_to_string(ws.root.join(".shotpilot").join("three_engine").join("launches.jsonl"))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(|l| serde_json::from_str(l).unwrap()).collect())
        .unwrap_or_default()
}

/// Per shot, the game objects its current `q` take recorded as on screen.
fn seen(ws: &Workspace, q: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for s in ws.resolved().unwrap() {
        let rfp = shotpilot::fingerprint::render_fingerprint(&ws.defn, &s, q).unwrap();
        let mut names = BTreeSet::new();
        for t in ws.state.takes_for(&s.id, q, &rfp) {
            for d in t.info.get("deps").and_then(Value::as_array).into_iter().flatten() {
                if let Some(n) = d["package"].as_str().and_then(|p| p.strip_prefix("game/objects/")) {
                    names.insert(n.to_string());
                }
            }
        }
        out.insert(s.id.clone(), names);
    }
    out
}

fn status_of(ws: &Workspace, q: &str) -> BTreeMap<String, (String, Vec<String>)> {
    ws.status()
        .unwrap()
        .into_iter()
        .map(|r| {
            let changed = r.changed.iter().find(|(k, _)| k == q).map(|(_, v)| v.clone()).unwrap_or_default();
            (r.id.clone(), (r.quality(q).to_string(), changed))
        })
        .collect()
}

#[test]
fn the_three_sample_films_without_ue_and_redraws_only_the_shot_that_changed() {
    let (_d, mut ws) = ward();
    assert_eq!(ws.defn.project.engine, "three");
    let out = ws.render(&[], "still", false, None).unwrap();
    assert_eq!(out.len(), 10);
    assert!(out.iter().all(|o| o.action == "rendered"), "{out:?}");
    // one launch drew every frame, and each take holds its three key frames
    assert_eq!(launches(&ws).iter().filter(|l| l["kind"] == "render").count(), 1);
    for o in &out {
        let frames = std::fs::read_dir(&o.detail).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with("frame.")).count();
        assert_eq!(frames, 3, "{}", o.shot_id);
    }

    // one lens changes: that shot alone is drawn again
    ws.set_fields("s03_stretcher_empty", &set(&[("camera.lens.focal_length", json!(50))])).unwrap();
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    for (id, a) in &acts {
        assert_eq!(a, if id == "s03_stretcher_empty" { "rendered" } else { "reused" }, "{id}");
    }
    let last = launches(&ws).into_iter().rfind(|l| l["kind"] == "render").unwrap();
    assert_eq!(last["shots"], json!(["s03_stretcher_empty:still"]));

    // editorial edits never launch the browser
    let before = launches(&ws).len();
    ws.set_fields("s03_stretcher_empty", &set(&[("purpose", json!("寝台だけを見せる"))])).unwrap();
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    assert!(acts.values().all(|a| a == "reused"), "{acts:?}");
    assert_eq!(launches(&ws).len(), before);
}

#[test]
fn editing_one_thing_in_the_three_game_redraws_only_the_shots_that_showed_it() {
    let (d, mut ws) = ward();
    ws.render(&[], "still", false, None).unwrap();
    let seen = seen(&ws, "still");
    let with: BTreeSet<String> = seen.iter().filter(|(_, n)| n.contains("stretcher")).map(|(k, _)| k.clone()).collect();
    // the stretcher is the subject of s03 and lies far from the chair the hook looks at
    assert!(with.contains("s03_stretcher_empty"), "{seen:?}");
    assert!(!with.contains("s01_chair_hook"), "{seen:?}");
    assert!(with.len() < seen.len());

    let file = d.path().join("game").join("objects").join("stretcher.json");
    let original = std::fs::read_to_string(&file).unwrap();
    let mut v: Value = serde_json::from_str(&original).unwrap();
    v["parts"][0]["color"] = json!("#2f5f8f");
    std::fs::write(&file, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    for (id, (st, changed)) in status_of(&ws, "still") {
        if with.contains(&id) {
            assert_eq!((st.as_str(), changed), ("stale", vec!["game/objects/stretcher".to_string()]), "{id}");
        } else {
            assert_eq!((st.as_str(), changed.len()), ("fresh", 0), "{id}");
        }
    }
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    for (id, a) in &acts {
        assert_eq!(a, if with.contains(id) { "rendered" } else { "reused" }, "{id}");
    }

    // put the stretcher back: the first takes are used again, nothing is drawn
    std::fs::write(&file, original).unwrap();
    assert!(status_of(&ws, "still").values().all(|(st, _)| st == "fresh"));
    let before = launches(&ws).len();
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    assert!(acts.values().all(|a| a == "reused"), "{acts:?}");
    assert_eq!(launches(&ws).len(), before);
}

/// A corridor of one wall: something behind it, something beside it, something behind the camera.
fn walled(dir: &Path) -> Workspace {
    let game = dir.join("game");
    std::fs::create_dir_all(game.join("objects")).unwrap();
    std::fs::write(game.join("scene.json"), "{}").unwrap();
    let put = |name: &str, v: Value| std::fs::write(game.join("objects").join(format!("{name}.json")), v.to_string()).unwrap();
    put("wall", json!({"structure": true, "parts": [{"box": [20, 400, 300], "at": [500, 0, 150]}]}));
    put("hidden", json!({"parts": [{"box": [50, 50, 100], "at": [800, 0, 50]}]}));
    put("beside", json!({"parts": [{"box": [50, 50, 100], "at": [300, 120, 50]}]}));
    put("behind", json!({"parts": [{"box": [50, 50, 100], "at": [-500, 0, 50]}]}));
    put("dancer", json!({"parts": [{"figure": "idle", "at": [200, -60, 0]}]}));
    open(dir, "version: 1\nproject:\n  engine: three\nteaser:\n  fps: 30\n  resolution: [1920, 1080]\nanchors: {}\n\
               shots:\n  - id: s01\n    duration: 1\n    camera:\n      location: [0, 0, 100]\n      rotation: [0, 0, 0]\n      lens: {focal_length: 35}\n")
}

#[test]
fn a_wall_hides_what_is_behind_it_from_the_three_takes() {
    let dir = tempfile::tempdir().unwrap();
    let mut ws = walled(dir.path());
    ws.render(&[], "still", false, None).unwrap();
    let names = &seen(&ws, "still")["s01"];
    assert!(names.contains("wall") && names.contains("beside") && names.contains("dancer"), "{names:?}");
    assert!(!names.contains("hidden"), "behind the wall: {names:?}");
    assert!(!names.contains("behind"), "behind the camera: {names:?}");
    // so repainting what the wall hides re-renders nothing
    std::fs::write(dir.path().join("game/objects/hidden.json"), json!({"parts": [{"box": [50, 50, 100], "at": [800, 0, 50], "color": "#ff0000"}]}).to_string()).unwrap();
    assert_eq!(status_of(&ws, "still")["s01"].0, "fresh");
}

#[test]
fn a_scout_camera_behind_a_three_wall_stands_in_front_of_it() {
    let dir = tempfile::tempdir().unwrap();
    walled(dir.path());
    let scene = three::Scene::load(&dir.path().join("game")).unwrap();
    // looking at the hidden box from the far side of the wall: pulled in front of the wall, wider
    let (at, focal) = scene.clear_view([800.0, 0.0, 50.0], [0.0, 0.0, 100.0], 35.0).unwrap();
    assert!(at[0] > 510.0 && at[0] < 800.0, "{at:?}");
    assert!((12.0..35.0).contains(&focal), "{focal}");
    // with a clear line it stays where it was
    assert!(scene.clear_view([300.0, 120.0, 50.0], [0.0, 0.0, 100.0], 35.0).is_none());

    // in the sample, the scout photographs every anchor from four sides with the engine; a camera
    // planned through a wall is recorded where it really stood, and no scout camera becomes a shot
    let (_d, mut ws) = ward();
    let planned = shotpilot::scout::cameras(&ws.defn, &[]);
    let shots = ws.defn.shot_ids();
    ws.scout(&[], None).unwrap();
    let idx = shotpilot::scout::load_index(&ws.root);
    let anchors = idx["anchors"].as_object().unwrap();
    assert_eq!(anchors.len(), 5);
    assert!(anchors.values().all(|v| v.as_array().unwrap().len() == 4));
    let moved = planned.iter().filter(|c| {
        let v = anchors[&c.anchor].as_array().unwrap().iter().find(|v| v["side"] == c.side.as_str()).unwrap();
        v["location"] != json!(c.location)
    });
    assert!(moved.count() > 0, "the ward's walls stand between some anchors and their scout cameras");
    ws.reload().unwrap();
    assert_eq!(ws.defn.shot_ids(), shots);
    assert!(launches(&ws).iter().any(|l| l["kind"] == "render"));
}

#[test]
fn the_three_game_offers_its_anchors_places_and_props() {
    let (_d, ws) = ward();
    let engine = shotpilot::engines::make_engine(&ws.defn).unwrap();
    let found = engine.survey("shotpilot", "SP_", true, true).unwrap();
    let anchors: BTreeSet<&str> = found["anchors"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(anchors, ["empty_chair", "fallen_patient", "figure", "reception_desk", "stretcher"].into_iter().collect());
    // the anchors are where the UE sample has them, so the same shots film both
    assert_eq!(found["anchors"]["stretcher"]["center"], ws.defn.known_box("stretcher").map(|b| json!(b.center)).unwrap());
    let places: BTreeSet<&str> = found["candidates"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
    assert!(places.contains("cabinet") && places.contains("night_nurse"), "{places:?}");
    assert!(!places.contains("lobby") && !places.contains("stretcher"), "walls and anchors are not offered: {places:?}");
    assert_eq!(found["assets"], json!([{"asset": "props/stranger", "kind": "skeletal", "size": [45.0, 55.0, 175.0]}]));

    // a new project on the game with no anchors yet takes the tagged ones in one scout
    let dir = tempfile::tempdir().unwrap();
    threegame::write(&dir.path().join("game")).unwrap();
    let mut empty = open(dir.path(), &templates::render(templates::EMPTY, "new", "three", "null", "null"));
    assert!(empty.defn.anchors.is_empty());
    empty.scout(&[], None).unwrap();
    empty.reload().unwrap();
    let got: BTreeSet<String> = empty.defn.anchors.iter().map(|(n, _)| n.clone()).collect();
    assert_eq!(got, anchors.iter().map(|s| s.to_string()).collect());
}

#[test]
fn three_takes_carry_their_own_renderer_version() {
    let (_d, ws) = ward();
    let text = std::fs::read_to_string(&ws.path).unwrap();
    let dir = tempfile::tempdir().unwrap();
    threegame::write(&dir.path().join("game")).unwrap();
    let mock = open(dir.path(), &text.replace("engine: three", "engine: mock"));
    let s3 = &ws.resolved().unwrap()[2];
    let sm = &mock.resolved().unwrap()[2];
    let fp = |w: &Workspace, s, q| shotpilot::fingerprint::render_fingerprint(&w.defn, s, q).unwrap();
    // engine takes of the two never stand for each other; the previz of the same shot is the same
    assert_ne!(fp(&ws, s3, "still"), fp(&mock, sm, "still"));
    assert_eq!(fp(&ws, s3, "previz"), fp(&mock, sm, "previz"));
}

#[test]
fn a_three_project_without_its_game_says_so_and_previz_still_works() {
    let (d, mut ws) = ward();
    std::fs::remove_dir_all(d.path().join("game")).unwrap();
    let err = ws.render(&[], "still", false, None).unwrap_err();
    assert!(err.msg.contains("scene.json") && err.msg.contains("init --engine three"), "{}", err.msg);
    let out = ws.render(&[], "previz", false, None).unwrap();
    assert!(out.iter().all(|o| o.action == "rendered"));
}

#[test]
fn the_home_makes_a_three_project_with_the_ward_to_film() {
    three::use_stand_in_browser(true);
    let dir = tempfile::tempdir().unwrap();
    let new = |name: &str, sample: bool| shotpilot::projects::NewProject {
        location: dir.path().to_path_buf(),
        name: name.into(),
        engine: "three".into(),
        pitch: if sample { String::new() } else { "夜の病棟で、廊下の奥から何かが近づくホラー。30秒。".into() },
        sample,
        ..Default::default()
    };
    // the sample: the Lanternless teaser and its game, ready to film
    let shots = shotpilot::projects::create(&new("サンプル", true)).unwrap();
    let mut ws = Workspace::open(&shots).unwrap();
    assert_eq!((ws.defn.project.engine.as_str(), ws.defn.shots.len()), ("three", 10));
    assert!(ws.root.join("game").join("scene.json").is_file());
    let out = ws.render(&[], "still", false, None).unwrap();
    assert!(out.iter().all(|o| o.action == "rendered"), "{out:?}");
    // from a prompt: an empty plan on the same game, which the first scout reads anchors from
    let shots = shotpilot::projects::create(&new("病棟", false)).unwrap();
    let ws = Workspace::open(&shots).unwrap();
    assert_eq!((ws.defn.project.engine.as_str(), ws.defn.shots.len()), ("three", 0));
    assert!(ws.root.join("game").join("objects").join("stretcher.json").is_file());
}
