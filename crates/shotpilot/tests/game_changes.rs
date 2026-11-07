//! A change to the game (not to shots.yaml) re-renders only the shots that showed it.
//!
//! The mock engine reports, per rendered shot, the content files of what was in view
//! (`content/<anchor>.*`), as UE reports the packages behind what was on screen. A take
//! stays fresh while its definition matches and none of those files changed; put a file
//! back and the old take is used again (takes are never overwritten).

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::{actions, launches, ws, Fixture};
use serde_json::{json, Value};

fn write(f: &Fixture, rel: &str, text: &str) {
    let p = f.ws.root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}

fn take_json(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("take.json")).unwrap()).unwrap()
}

/// The shots whose current `q` take recorded `file` among what it showed.
fn showing(f: &Fixture, q: &str, file: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for s in f.ws.resolved().unwrap() {
        let rfp = shotpilot::fingerprint::render_fingerprint(&f.ws.defn, &s, q).unwrap();
        for t in f.ws.state.takes_for(&s.id, q, &rfp) {
            let deps = t.info.get("deps").and_then(Value::as_array).cloned().unwrap_or_default();
            if deps.iter().any(|d| d["file"] == file) {
                out.insert(s.id.clone());
            }
        }
    }
    out
}

fn status_of(f: &Fixture, q: &str) -> Vec<(String, String, Vec<String>)> {
    f.ws.status()
        .unwrap()
        .into_iter()
        .map(|r| {
            let changed = r.changed.iter().find(|(k, _)| k == q).map(|(_, v)| v.clone()).unwrap_or_default();
            (r.id.clone(), r.quality(q).to_string(), changed)
        })
        .collect()
}

#[test]
fn a_game_change_rerenders_only_the_shots_that_show_it() {
    let mut f = ws();
    write(&f, "content/well.txt", "well texture v1");
    write(&f, "content/lantern.txt", "lantern v1");
    write(&f, "content/unused.txt", "nothing shows this");
    f.ws.render(&[], "still", false, None).unwrap();
    let with_well = showing(&f, "still", "content/well.txt");
    let all: BTreeSet<String> = f.ws.defn.shot_ids().into_iter().collect();
    assert!(!with_well.is_empty() && with_well != all, "the demo needs some shots with the well and some without: {with_well:?}");
    assert!(with_well.contains("s02_well"));
    assert!(showing(&f, "still", "content/unused.txt").is_empty());
    let launched = launches(&f.ws).len();

    // the well's texture changes in the game: only the shots that showed it go stale
    write(&f, "content/well.txt", "well texture v2, repainted");
    for (id, st, changed) in status_of(&f, "still") {
        if with_well.contains(&id) {
            assert_eq!((st.as_str(), changed), ("stale", vec!["/Game/Mock/well".to_string()]), "{id}");
        } else {
            assert_eq!((st.as_str(), changed.len()), ("fresh", 0), "{id}");
        }
    }
    let out = f.ws.render(&[], "still", false, None).unwrap();
    let acts = actions(&out);
    for (id, a) in &acts {
        assert_eq!(a, if with_well.contains(id) { "rendered" } else { "reused" }, "{id}");
    }
    assert_eq!(launches(&f.ws).len(), launched + 1);
    let shots_launched: BTreeSet<String> = launches(&f.ws).last().unwrap()["shots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap().split(':').next().unwrap().to_string())
        .collect();
    assert_eq!(shots_launched, with_well);
    // the new take sits next to the old one; nothing was overwritten
    let s02 = f.ws.select_one("s02_well").unwrap();
    let rfp = shotpilot::fingerprint::render_fingerprint(&f.ws.defn, &s02, "still").unwrap();
    let takes = f.ws.state.takes_for("s02_well", "still", &rfp);
    assert_eq!(takes.len(), 2);
    assert!(takes[0].dir.file_name().unwrap().to_string_lossy().ends_with("~2"));
    assert!(status_of(&f, "still").iter().all(|(_, st, _)| st == "fresh"));

    // put the old texture back: the old takes match again, nothing renders
    write(&f, "content/well.txt", "well texture v1");
    assert!(status_of(&f, "still").iter().all(|(_, st, _)| st == "fresh"));
    let picked = f.ws.current_take("s02_well", "still", &rfp);
    match picked {
        shotpilot::workspace::TakeMatch::Fresh(t) => assert_eq!(take_json(&t.dir)["deps"][0]["sha256"].as_str().map(str::len), Some(64)),
        other => panic!("expected the first take again, got {other:?}"),
    }
    let out = f.ws.render(&[], "still", false, None).unwrap();
    assert!(actions(&out).values().all(|a| a == "reused"));
    assert_eq!(launches(&f.ws).len(), launched + 1);
}

#[test]
fn a_game_without_tracked_content_behaves_as_before() {
    let mut f = ws();
    f.ws.render(&["s01_gate".into()], "still", false, None).unwrap();
    let s = f.ws.select_one("s01_gate").unwrap();
    let rfp = shotpilot::fingerprint::render_fingerprint(&f.ws.defn, &s, "still").unwrap();
    let t = f.ws.state.find_take("s01_gate", "still", &rfp).unwrap();
    assert!(t.info.get("deps").is_none());
    assert!(status_of(&f, "still").iter().all(|(_, _, changed)| changed.is_empty()));
}

#[test]
fn previz_never_goes_stale_from_the_game_and_the_cut_falls_back_to_it() {
    let mut f = ws();
    write(&f, "content/well.txt", "v1");
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.render(&[], "still", false, None).unwrap();
    let with_well = showing(&f, "still", "content/well.txt");
    write(&f, "content/well.txt", "v2 changed");
    // previz does not look at the game, so it stays fresh
    assert!(status_of(&f, "previz").iter().all(|(_, st, _)| st == "fresh"));
    // the cut uses the best fresh take: still where the game is unchanged, previz where it changed
    for (s, _, q) in f.ws.pick_takes("best").unwrap() {
        let want = if with_well.contains(&s.id) { "previz" } else { "still" };
        assert_eq!(q.as_deref(), Some(want), "{}", s.id);
    }
}

#[test]
fn a_level_read_from_the_engine_is_drawn_in_previz_and_only_previz_follows_it() {
    // D-86's plan of the level is what previz draws around the anchors. Engine takes film the
    // level itself, so reading the plan (again) never touches them.
    let mut f = ws();
    write(&f, "content/well.txt", "v1");
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.render(&[], "still", false, None).unwrap();
    let s = f.ws.select_one("s02_well").unwrap();
    let fp = |f: &Fixture, q: &str| shotpilot::fingerprint::render_fingerprint(&f.ws.defn, &s, q).unwrap();
    let (pv, still) = (fp(&f, "previz"), fp(&f, "still"));
    let world = shotpilot::previz::World::load(&f.ws.defn);
    assert!(!world.has_level());
    let before = shotpilot::previz::render_frame(&s, 0, &world, 160, 90, None).png();

    let level = json!({"shapes": [
            {"label": "SM_Wall", "kind": "structure", "center": [-300, 0, 150], "half": [20, 900, 150], "yaw": 0},
            {"label": "BP_Door", "kind": "door", "center": [-280, 0, 110], "half": [10, 60, 110], "yaw": 0},
            {"label": "PlayerStart", "kind": "prop", "center": [0, 0, 92], "half": [140, 80, 92], "yaw": 0}],
        "points": [{"kind": "light", "label": "Lamp", "at": [0, 0, 300]}, {"kind": "start", "label": "PlayerStart", "at": [0, 0, 92]}],
        "floors": [0.0]});
    shotpilot::scout::save_level(&f.ws.root, &level).unwrap();
    let world = shotpilot::previz::World::load(&f.ws.defn);
    assert!(world.has_level());
    assert_ne!(shotpilot::previz::render_frame(&s, 0, &world, 160, 90, None).png(), before);
    assert_ne!(fp(&f, "previz"), pv, "previz draws the level from now on");
    assert_eq!(fp(&f, "still"), still, "engine takes stay");
    assert!(status_of(&f, "still").iter().all(|(_, st, _)| st == "fresh"));
    let acts = actions(&f.ws.render(&[], "previz", false, None).unwrap());
    assert!(acts.values().all(|a| a == "rendered"), "{acts:?}");
    let all: BTreeSet<String> = f.ws.defn.shot_ids().into_iter().collect();
    assert_eq!(showing(&f, "previz", "scout/level.json"), all);

    // read again with a wall moved: previz goes stale for the level, engine takes do not
    let mut moved = level.clone();
    moved["shapes"][0]["center"] = json!([-400, 0, 150]);
    shotpilot::scout::save_level(&f.ws.root, &moved).unwrap();
    for (id, st, changed) in status_of(&f, "previz") {
        assert_eq!((st.as_str(), changed), ("stale", vec!["scout/level.json".to_string()]), "{id}");
    }
    assert!(status_of(&f, "still").iter().all(|(_, st, _)| st == "fresh"));
    let acts = actions(&f.ws.render(&[], "previz", false, None).unwrap());
    assert!(acts.values().all(|a| a == "rendered"), "{acts:?}");

    // read back as it was: the previz drawn with it is used again
    shotpilot::scout::save_level(&f.ws.root, &level).unwrap();
    assert!(status_of(&f, "previz").iter().all(|(_, st, _)| st == "fresh"));
    let acts = actions(&f.ws.render(&[], "previz", false, None).unwrap());
    assert!(acts.values().all(|a| a == "reused"), "{acts:?}");
}

#[test]
fn a_forced_take_replaces_and_a_deleted_file_counts_as_a_change() {
    let mut f = ws();
    write(&f, "content/well.txt", "v1");
    f.ws.render(&["s02_well".into()], "still", false, None).unwrap();
    std::fs::remove_file(f.ws.root.join("content/well.txt")).unwrap();
    let st = status_of(&f, "still");
    let s02 = st.iter().find(|(id, _, _)| id == "s02_well").unwrap();
    assert_eq!((s02.1.as_str(), s02.2.clone()), ("stale", vec!["/Game/Mock/well".to_string()]));
    // a forced re-take overwrites this definition's first take instead of adding one
    write(&f, "content/well.txt", "v3");
    f.ws.render(&["s02_well".into()], "still", true, None).unwrap();
    let s = f.ws.select_one("s02_well").unwrap();
    let rfp = shotpilot::fingerprint::render_fingerprint(&f.ws.defn, &s, "still").unwrap();
    assert_eq!(f.ws.state.takes_for("s02_well", "still", &rfp).len(), 1);
}
