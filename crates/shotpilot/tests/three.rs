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

/// The sample: the KettleHill teaser and the village it films.
fn village() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir().unwrap();
    assert!(threegame::write(&dir.path().join("game")).unwrap());
    let ws = open(dir.path(), &templates::render(templates::THREE_VILLAGE, "village", "three", "null", "null"));
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
    let (_d, mut ws) = village();
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
    ws.set_fields("s04_market", &set(&[("camera.lens.focal_length", json!(50))])).unwrap();
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    for (id, a) in &acts {
        assert_eq!(a, if id == "s04_market" { "rendered" } else { "reused" }, "{id}");
    }
    let last = launches(&ws).into_iter().rfind(|l| l["kind"] == "render").unwrap();
    assert_eq!(last["shots"], json!(["s04_market:still"]));

    // editorial edits never launch the browser
    let before = launches(&ws).len();
    ws.set_fields("s04_market", &set(&[("purpose", json!("屋台の野菜だけを見せる"))])).unwrap();
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    assert!(acts.values().all(|a| a == "reused"), "{acts:?}");
    assert_eq!(launches(&ws).len(), before);
}

#[test]
fn editing_one_thing_in_the_three_game_redraws_only_the_shots_that_showed_it() {
    let (d, mut ws) = village();
    ws.render(&[], "still", false, None).unwrap();
    let seen = seen(&ws, "still");
    let with: BTreeSet<String> = seen.iter().filter(|(_, n)| n.contains("market_stall")).map(|(k, _)| k.clone()).collect();
    // the stall is the subject of s04 and stands behind the camera that looks up at the windmill
    assert!(with.contains("s04_market"), "{seen:?}");
    assert!(!with.contains("s01_windmill"), "{seen:?}");
    assert!(with.len() < seen.len());

    // repaint the awning's first stripe
    let file = d.path().join("game").join("objects").join("market_stall.json");
    let original = std::fs::read_to_string(&file).unwrap();
    let mut v: Value = serde_json::from_str(&original).unwrap();
    v["parts"][0]["color"] = json!("#2f5f8f");
    std::fs::write(&file, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    for (id, (st, changed)) in status_of(&ws, "still") {
        if with.contains(&id) {
            assert_eq!((st.as_str(), changed), ("stale", vec!["game/objects/market_stall".to_string()]), "{id}");
        } else {
            assert_eq!((st.as_str(), changed.len()), ("fresh", 0), "{id}");
        }
    }
    let acts = actions(&ws.render(&[], "still", false, None).unwrap());
    for (id, a) in &acts {
        assert_eq!(a, if with.contains(id) { "rendered" } else { "reused" }, "{id}");
    }

    // put the awning back: the first takes are used again, nothing is drawn
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
    let (_d, mut ws) = village();
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
    assert!(moved.count() > 0, "the teahouse's walls stand between its middle and some scout cameras");
    ws.reload().unwrap();
    assert_eq!(ws.defn.shot_ids(), shots);
    assert!(launches(&ws).iter().any(|l| l["kind"] == "render"));
}

#[test]
fn the_three_game_offers_its_anchors_places_and_props() {
    let (_d, ws) = village();
    let engine = shotpilot::engines::make_engine(&ws.defn).unwrap();
    let found = engine.survey("shotpilot", "SP_", true, true).unwrap();
    let anchors: BTreeSet<&str> = found["anchors"].as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(anchors, ["crop_row", "market_stall", "old_oak", "teahouse", "windmill"].into_iter().collect());
    // the sample's anchors are the game's tagged places, as written
    assert_eq!(found["anchors"]["market_stall"]["center"], ws.defn.known_box("market_stall").map(|b| json!(b.center)).unwrap());
    let places: BTreeSet<&str> = found["candidates"].as_array().unwrap().iter().filter_map(|c| c["name"].as_str()).collect();
    assert!(places.contains("well") && places.contains("tea_master"), "{places:?}");
    assert!(!places.contains("paths") && !places.contains("market_stall"), "the ground, paths and anchors are not offered: {places:?}");
    assert_eq!(found["assets"], json!([{"asset": "props/traveler", "kind": "skeletal", "size": [45.0, 55.0, 175.0]}]));

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
    let (d, ws) = village();
    let text = std::fs::read_to_string(&ws.path).unwrap();
    let dir = tempfile::tempdir().unwrap();
    threegame::write(&dir.path().join("game")).unwrap();
    let mock = open(dir.path(), &text.replace("engine: three", "engine: mock"));
    let s3 = &ws.resolved().unwrap()[2];
    let sm = &mock.resolved().unwrap()[2];
    let fp = |w: &Workspace, s, q| shotpilot::fingerprint::render_fingerprint(&w.defn, s, q).unwrap();
    // engine takes of the two never stand for each other; the previz differs only in drawing the game
    assert_ne!(fp(&ws, s3, "still"), fp(&mock, sm, "still"));
    assert_ne!(fp(&ws, s3, "previz"), fp(&mock, sm, "previz"), "the three previz draws its game");
    std::fs::remove_dir_all(d.path().join("game")).unwrap();
    assert_eq!(fp(&ws, s3, "previz"), fp(&mock, sm, "previz"));
}

#[test]
fn previz_draws_the_game_and_redraws_only_the_shots_that_showed_a_changed_thing() {
    let (d, mut ws) = village();
    // the village is drawn around the anchors, without a browser
    let s1 = ws.resolved().unwrap()[0].clone();
    let world = shotpilot::previz::World::load(&ws.defn);
    assert!(world.has_level());
    let bare = shotpilot::previz::World::bare(ws.defn.anchor_boxes(), 0.0);
    let png = |w: &shotpilot::previz::World| shotpilot::previz::render_frame(&s1, 0, w, 160, 90, None).png();
    assert_ne!(png(&world), png(&bare));
    let out = ws.render(&[], "previz", false, None).unwrap();
    assert!(out.iter().all(|o| o.action == "rendered"), "{out:?}");
    assert!(launches(&ws).is_empty(), "previz started a browser");

    // each previz take keeps the objects it drew, as the engine takes do (D-37)
    let seen = seen(&ws, "previz");
    let with: BTreeSet<String> = seen.iter().filter(|(_, n)| n.contains("market_stall")).map(|(k, _)| k.clone()).collect();
    assert!(with.contains("s04_market"), "{seen:?}");
    assert!(!with.contains("s01_windmill"), "{seen:?}");

    let file = d.path().join("game").join("objects").join("market_stall.json");
    let original = std::fs::read_to_string(&file).unwrap();
    let mut v: Value = serde_json::from_str(&original).unwrap();
    v["parts"][0]["at"][2] = json!(v["parts"][0]["at"][2].as_f64().unwrap_or(0.0) + 30.0);
    std::fs::write(&file, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    for (id, (st, changed)) in status_of(&ws, "previz") {
        if with.contains(&id) {
            assert_eq!((st.as_str(), changed), ("stale", vec!["game/objects/market_stall".to_string()]), "{id}");
        } else {
            assert_eq!((st.as_str(), changed.len()), ("fresh", 0), "{id}");
        }
    }
    let acts = actions(&ws.render(&[], "previz", false, None).unwrap());
    for (id, a) in &acts {
        assert_eq!(a, if with.contains(id) { "rendered" } else { "reused" }, "{id}");
    }

    // put the awning back: the first previz takes are used again
    std::fs::write(&file, original).unwrap();
    assert!(status_of(&ws, "previz").values().all(|(st, _)| st == "fresh"));
    let acts = actions(&ws.render(&[], "previz", false, None).unwrap());
    assert!(acts.values().all(|a| a == "reused"), "{acts:?}");
}

#[test]
fn previz_is_lit_by_the_games_lamps_and_follows_the_ones_that_light_it() {
    let (d, mut ws) = village();
    ws.render(&[], "previz", false, None).unwrap();
    let packages = |ws: &Workspace| -> BTreeMap<String, BTreeSet<String>> {
        let mut out = BTreeMap::new();
        for s in ws.resolved().unwrap() {
            let rfp = shotpilot::fingerprint::render_fingerprint(&ws.defn, &s, "previz").unwrap();
            let names = ws
                .state
                .takes_for(&s.id, "previz", &rfp)
                .iter()
                .flat_map(|t| t.info.get("deps").and_then(Value::as_array).cloned().unwrap_or_default())
                .filter_map(|d| d["package"].as_str().map(String::from))
                .collect();
            out.insert(s.id.clone(), names);
        }
        out
    };
    // the scene file sets every frame's light and haze; the teahouse's lamplight has no body to be
    // seen, so a take holds it only because its light reached the frame
    let deps = packages(&ws);
    assert!(deps.values().all(|p| p.contains("game/scene.json")), "{deps:?}");
    let lit: BTreeSet<String> = deps.iter().filter(|(_, p)| p.contains("game/objects/lamplight")).map(|(k, _)| k.clone()).collect();
    assert!(!lit.is_empty() && lit.len() < deps.len(), "{deps:?}");

    // dim the lamps: their light changes the frames it reached, and only those go stale
    let shot = ws.select_one(lit.iter().next().unwrap()).unwrap();
    let frame = |ws: &Workspace| shotpilot::previz::render_frame(&shot, 0, &shotpilot::previz::World::load(&ws.defn), 160, 90, None).png();
    let before = frame(&ws);
    let file = d.path().join("game").join("objects").join("lamplight.json");
    let original = std::fs::read_to_string(&file).unwrap();
    let mut v: Value = serde_json::from_str(&original).unwrap();
    for p in v["parts"].as_array_mut().unwrap() {
        p["intensity"] = json!(p["intensity"].as_f64().unwrap() / 4.0);
    }
    std::fs::write(&file, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    assert_ne!(frame(&ws), before, "the lamplight lights the shot");
    for (id, (st, changed)) in status_of(&ws, "previz") {
        if lit.contains(&id) {
            assert_eq!((st.as_str(), changed), ("stale", vec!["game/objects/lamplight".to_string()]), "{id}");
        } else {
            assert_eq!((st.as_str(), changed.len()), ("fresh", 0), "{id}");
        }
    }
    std::fs::write(&file, original).unwrap();
    assert!(status_of(&ws, "previz").values().all(|(st, _)| st == "fresh"));
    assert_eq!(frame(&ws), before);
}

/// The app's 3D view (D-97) and the guides over a take (D-96) only read what previz draws: the
/// same triangles, what stands still sent once, the figures again for another frame, the shot's
/// camera at every frame. A previz take's guides lie beside its frames, one entry per frame.
#[test]
fn the_3d_view_and_the_guides_read_what_previz_draws() {
    let (_d, mut ws) = village();
    let s = ws.resolved().unwrap().into_iter().find(|s| s.id == "s05_oak_kids").unwrap();
    let world = shotpilot::previz::World::load(&ws.defn);
    let a = shotpilot::previz::view_scene(&s, 0, &world, 1920, 1080, None);
    let still = a["still"]["s"].as_array().unwrap().len();
    assert!(still > 100, "the village: {still} triangles");
    assert_eq!(a["still"]["p"].as_array().unwrap().len(), still * 9);
    assert_eq!(a["cameras"].as_array().unwrap().len() as i64, s.frames);
    assert_eq!(a["dynamic"], json!(true), "the children run");
    let key = a["still"]["key"].as_str().unwrap();
    let b = shotpilot::previz::view_scene(&s, s.frames - 1, &world, 1920, 1080, Some(key));
    assert_eq!(b["still"]["key"], json!(key), "what stands still is the same at every frame");
    assert!(b["still"].get("p").is_none(), "the page already holds what stands still");
    assert_ne!(a["moving"]["p"], b["moving"]["p"], "the figures moved between the frames");
    assert_eq!(a["cameras"], b["cameras"]);
    let cam = shotpilot::three::frame_camera(&s.camera_at(5), s.focal_length(), s.sensor_width(), 1920, 1080);
    assert_eq!(a["cameras"][5], cam, "the view looks through the camera the take is drawn from");

    // the take's guides: one entry per frame file, naming the places in frame
    let out = ws.render(std::slice::from_ref(&s.id), "previz", false, None).unwrap();
    let dir = ws.state.take_dir(&s.id, "previz", &out[0].fingerprint);
    let g: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(shotpilot::previz::GUIDES)).unwrap()).unwrap();
    let pngs = std::fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "png")).count();
    assert_eq!(g["frames"].as_array().unwrap().len(), pngs);
    assert!(g["frames"][0]["places"].as_array().unwrap().iter().any(|p| p["name"] == "old_oak"), "{}", g["frames"][0]);
    for f in g["frames"].as_array().unwrap() {
        for p in f["places"].as_array().unwrap() {
            let at = p["at"].as_array().unwrap();
            assert!(at.iter().all(|v| (0.0..=1.0).contains(&v.as_f64().unwrap())), "{p}");
        }
    }
}

#[test]
fn a_three_project_without_its_game_says_so_and_previz_still_works() {
    let (d, mut ws) = village();
    std::fs::remove_dir_all(d.path().join("game")).unwrap();
    let err = ws.render(&[], "still", false, None).unwrap_err();
    assert!(err.msg.contains("scene.json") && err.msg.contains("init --engine three"), "{}", err.msg);
    let out = ws.render(&[], "previz", false, None).unwrap();
    assert!(out.iter().all(|o| o.action == "rendered"));
}

#[test]
fn the_home_makes_a_three_project_with_the_village_to_film() {
    three::use_stand_in_browser(true);
    let dir = tempfile::tempdir().unwrap();
    let new = |name: &str, sample: bool| shotpilot::projects::NewProject {
        location: dir.path().to_path_buf(),
        name: name.into(),
        engine: "three".into(),
        pitch: if sample { String::new() } else { "丘の上の村で、午後のお茶を楽しむコージーゲーム。30秒。".into() },
        sample,
        ..Default::default()
    };
    // the sample: the KettleHill teaser and its game, ready to film
    let shots = shotpilot::projects::create(&new("サンプル", true)).unwrap();
    let mut ws = Workspace::open(&shots).unwrap();
    assert_eq!((ws.defn.project.engine.as_str(), ws.defn.shots.len()), ("three", 10));
    assert!(ws.root.join("game").join("scene.json").is_file());
    let out = ws.render(&[], "still", false, None).unwrap();
    assert!(out.iter().all(|o| o.action == "rendered"), "{out:?}");
    // from a prompt: an empty plan on the same game, which the first scout reads anchors from
    let shots = shotpilot::projects::create(&new("村", false)).unwrap();
    let ws = Workspace::open(&shots).unwrap();
    assert_eq!((ws.defn.project.engine.as_str(), ws.defn.shots.len()), ("three", 0));
    assert!(ws.root.join("game").join("objects").join("market_stall.json").is_file());
}
