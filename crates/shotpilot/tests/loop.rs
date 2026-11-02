//! The D-13 partial-fix loop: per-shot re-render, definition/output separation,
//! state, cheap previews, and reverse sync.

mod common;

use std::collections::HashSet;

use common::{actions, ids, launches, set, ws};
use serde_json::{json, Value};
use shotpilot::fingerprint as fp;
use shotpilot::sync::PullKind;
use shotpilot::yamlio;

fn values(m: &std::collections::HashMap<String, String>) -> HashSet<String> {
    m.values().cloned().collect()
}

fn one(s: &str) -> HashSet<String> {
    HashSet::from([s.to_string()])
}

#[test]
fn definition_validates_and_resolves() {
    let f = ws();
    let shots = f.ws.resolved().unwrap();
    assert_eq!(shots[0].id, "s01_gate");
    assert_eq!(shots[1].id, "s02_well");
    let s1 = &shots[0];
    assert_eq!(s1.frames, 150);
    assert_ne!(s1.start.location, s1.end.location); // dolly_in moved the camera
    assert_eq!(s1.lens["focal_length"], json!(24.0));
    assert!(s1.lens["focus_distance"].as_f64().unwrap() > 0.0); // auto-focus on the look target
}

#[test]
fn bad_definition_reports_all_problems() {
    let mut f = ws();
    f.write(&f.text().replacen("duration: 5\n", "duration: -1\n", 1).replacen("anchor: well", "anchor: nowhere", 1));
    let err = f.ws.reload().unwrap_err();
    assert!(err.msg.contains("duration"), "{}", err.msg);
}

#[test]
fn only_changed_shot_rerenders() {
    let mut f = ws();
    let first = actions(&f.ws.render(&[], "previz", false, None).unwrap());
    assert_eq!(values(&first), one("rendered"));

    let rep = f.ws.set_fields("s03_window", &set(&[("camera.lens.focal_length", json!(85))])).unwrap();
    assert_eq!(rep.stale, ids(&["s03_window"]));

    let mut second = actions(&f.ws.render(&[], "previz", false, None).unwrap());
    assert_eq!(second.remove("s03_window").unwrap(), "rendered");
    assert_eq!(values(&second), one("reused"));
}

#[test]
fn revert_reuses_old_take() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.set_fields("s02_well", &set(&[("duration", json!(2))])).unwrap();
    assert_eq!(actions(&f.ws.render(&ids(&["s02_well"]), "previz", false, None).unwrap())["s02_well"], "rendered");
    f.ws.set_fields("s02_well", &set(&[("duration", json!(3.5))])).unwrap();
    assert_eq!(actions(&f.ws.render(&ids(&["s02_well"]), "previz", false, None).unwrap())["s02_well"], "reused");
}

#[test]
fn order_purpose_notes_never_invalidate() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let order = ids(&["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"]);
    let rep = f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    assert!(rep.stale.is_empty());
    let rep = f.ws.set_fields("s04_lantern", &set(&[("purpose", json!("別の説明")), ("notes", json!("memo"))])).unwrap();
    assert!(rep.stale.is_empty());
    assert_eq!(values(&actions(&f.ws.render(&[], "previz", false, None).unwrap())), one("reused"));
    let info = f.ws.cut("previz", false).unwrap();
    let m: Value =
        serde_json::from_str(&std::fs::read_to_string(f.ws.root.join("cut/previz/manifest.json")).unwrap()).unwrap();
    assert_eq!(m["shots"][0]["id"], "s06_title");
    assert_eq!(m["shots"][1]["record_in"], m["shots"][0]["frames"]);
    assert_eq!(info.shots, 6);
}

#[test]
fn comments_survive_machine_edits() {
    let mut f = ws();
    let before = f.text();
    f.ws.set_fields("s01_gate", &set(&[("duration", json!(6))])).unwrap();
    let text = f.text();
    assert!(text.contains("# Order here is the order of the cut"));
    assert!(text.contains("isolate_userdir: true            # run UE with its own -UserDir"));
    // every line outside the edited shot is byte-for-byte unchanged
    let start = before.find("  - id: s02_well").unwrap();
    assert_eq!(&before[start..], &text[text.find("  - id: s02_well").unwrap()..]);
    // and reordering moves whole shots without touching other lines
    let order = ids(&["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"]);
    f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    let text = f.text();
    assert!(text.find("  - id: s06_title").unwrap() < text.find("  - id: s01_gate").unwrap());
    assert!(text.contains("    notes: タイトルは編集で載せる。上1/3を空けておく"));
}

#[test]
fn cut_needs_takes() {
    let mut f = ws();
    let err = f.ws.cut("preview", false).unwrap_err();
    assert!(err.msg.contains("no fresh"));
}

#[test]
fn best_cut_mixes_qualities() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.render(&ids(&["s02_well"]), "preview", false, None).unwrap();
    let picks: std::collections::HashMap<String, String> =
        f.ws.pick_takes("best").unwrap().into_iter().map(|(s, _, q)| (s.id, q.unwrap())).collect();
    assert_eq!(picks["s02_well"], "preview");
    assert_eq!(picks["s01_gate"], "previz");
}

#[test]
fn engine_builds_only_dirty_shots() {
    let mut f = ws();
    let out = actions(&f.ws.render(&[], "preview", false, None).unwrap());
    assert_eq!(values(&out), one("rendered"));
    assert_eq!(launches(&f.ws).last().unwrap()["built"].as_array().unwrap().len(), 6);

    f.ws.set_fields("s05_door", &set(&[("camera.move.distance", json!(300))])).unwrap();
    let mut out = actions(&f.ws.render(&[], "preview", false, None).unwrap());
    assert_eq!(out.remove("s05_door").unwrap(), "rendered");
    assert_eq!(values(&out), one("reused"));
    let last = launches(&f.ws).last().unwrap().clone();
    assert_eq!(last["built"], json!(["s05_door"]));
    assert_eq!(last["shots"], json!(["s05_door:preview"]));
    assert!(f.ws.status().unwrap().iter().all(|r| r.build == "synced"));
}

#[test]
fn preview_and_still_frame_counts() {
    let f = ws();
    let s = &f.ws.resolved().unwrap()[0];
    assert_eq!(fp::frame_list(s, &fp::quality(&f.ws.defn, "preview").unwrap()).len() as i64, s.frames);
    assert_eq!(fp::frame_list(s, &fp::quality(&f.ws.defn, "still").unwrap()), vec![0, (s.frames - 1) / 2, s.frames - 1]);
    assert!((fp::frame_list(s, &fp::quality(&f.ws.defn, "previz").unwrap()).len() as i64) < s.frames);
}

fn human_edit(ws: &shotpilot::Workspace, shot_id: &str, focal: f64, dz: f64) {
    ws.mock_engine()
        .unwrap()
        .edit_asset(shot_id, |d| {
            d["lens"]["focal_length"] = json!(focal);
            let z = d["start"]["location"][2].as_f64().unwrap();
            d["start"]["location"][2] = json!(z + dz);
        })
        .unwrap();
}

#[test]
fn pull_brings_editor_edits_back() {
    let mut f = ws();
    f.ws.build(&[], false).unwrap();
    human_edit(&f.ws, "s02_well", 60.0, 40.0);
    let before = fp::render_fingerprint(&f.ws.defn, &f.ws.select_one("s02_well").unwrap(), "preview").unwrap();

    let decisions = f.ws.pull(None, false).unwrap();
    let kind = |id: &str| decisions.iter().find(|d| d.shot_id == id).unwrap().kind.clone();
    assert_eq!(kind("s02_well"), PullKind::EngineEdit);
    assert_eq!(kind("s01_gate"), PullKind::Clean);

    let s2 = f.ws.select_one("s02_well").unwrap();
    assert_eq!(s2.lens["focal_length"], json!(60.0));
    let doc = yamlio::load(&f.path()).unwrap();
    let node = doc["shots"].as_array().unwrap().iter().find(|n| n["id"] == "s02_well").unwrap().clone();
    assert!(node["camera"].get("location").is_some() && node["camera"].get("anchor").is_none());
    assert_ne!(fp::render_fingerprint(&f.ws.defn, &s2, "preview").unwrap(), before); // needs re-render
    assert!(f.ws.status().unwrap().iter().all(|r| r.build == "synced")); // yaml == engine now
    assert!(f.text().contains("camera:  # pulled from engine")); // old intent kept as a comment
    let again = f.ws.pull(None, false).unwrap();
    assert_eq!(again.iter().filter(|d| d.kind == PullKind::EngineEdit).count(), 0); // idempotent
}

#[test]
fn build_refuses_to_clobber_editor_edits() {
    let mut f = ws();
    f.ws.build(&[], false).unwrap();
    human_edit(&f.ws, "s03_window", 60.0, 40.0);
    f.ws.set_fields("s03_window", &set(&[("duration", json!(5))])).unwrap();
    let rep = f.ws.build(&[], false).unwrap();
    assert_eq!(rep.diverged, ids(&["s03_window"]));

    let decisions = f.ws.pull(None, false).unwrap();
    assert_eq!(decisions.iter().find(|d| d.shot_id == "s03_window").unwrap().kind, PullKind::Conflict);

    f.ws.pull(Some("yaml"), false).unwrap();
    let rep = f.ws.build(&[], false).unwrap();
    assert_eq!(rep.built, ids(&["s03_window"]));
    assert_eq!(f.ws.select_one("s03_window").unwrap().lens["focal_length"], json!(50.0)); // yaml won
}

#[test]
fn conflict_prefer_engine() {
    let mut f = ws();
    f.ws.build(&[], false).unwrap();
    human_edit(&f.ws, "s04_lantern", 28.0, 40.0);
    f.ws.set_fields("s04_lantern", &set(&[("duration", json!(3))])).unwrap();
    f.ws.pull(Some("engine"), false).unwrap();
    let s = f.ws.select_one("s04_lantern").unwrap();
    assert_eq!(s.lens["focal_length"], json!(28.0));
    assert_eq!(s.duration, 4.0); // engine's frame count wins too
    assert!(f.ws.build(&[], false).unwrap().diverged.is_empty());
}

#[test]
fn render_blocked_when_diverged() {
    let mut f = ws();
    f.ws.render(&ids(&["s01_gate"]), "preview", false, None).unwrap();
    human_edit(&f.ws, "s01_gate", 60.0, 40.0);
    f.ws.set_fields("s01_gate", &set(&[("camera.lens.focal_length", json!(30))])).unwrap();
    let out = f.ws.render(&ids(&["s01_gate"]), "preview", false, None).unwrap();
    assert_eq!(out[0].action, "failed");
    assert!(out[0].detail.contains("pull"));
}

#[test]
fn gc_keeps_current() {
    let mut f = ws();
    f.ws.render(&ids(&["s02_well"]), "previz", false, None).unwrap();
    for d in [2.0, 2.5, 3.0] {
        f.ws.set_fields("s02_well", &set(&[("duration", json!(d))])).unwrap();
        f.ws.render(&ids(&["s02_well"]), "previz", false, None).unwrap();
    }
    let removed = f.ws.gc(1, false).unwrap();
    assert_eq!(removed.len(), 2);
    assert_eq!(f.ws.status().unwrap()[1].quality("previz"), "fresh");
}

#[test]
fn history_records_actions() {
    let mut f = ws();
    f.ws.render(&ids(&["s01_gate"]), "previz", false, None).unwrap();
    f.ws.set_fields("s01_gate", &set(&[("duration", json!(4))])).unwrap();
    let acts: Vec<String> = f.ws.state.history().iter().map(|h| h["action"].as_str().unwrap().to_string()).collect();
    assert_eq!(&acts[acts.len() - 2..], &["render".to_string(), "set".to_string()]);
}

#[test]
fn beats_are_editorial_and_checked() {
    let mut f = ws();
    assert!(f.ws.structure_warnings().unwrap().is_empty());
    f.ws.render(&[], "previz", false, None).unwrap();
    let rep = f.ws.set_fields("s02_well", &set(&[("beat", json!("hook"))])).unwrap();
    assert!(rep.stale.is_empty());
    let order = ids(&["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"]);
    f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    let warnings = f.ws.structure_warnings().unwrap().join(" ");
    assert!(warnings.contains("opens on `title`") && warnings.contains("end on the title beat"), "{warnings}");
}

#[test]
fn map_must_be_package_path() {
    let mut f = ws();
    f.write(&f.text().replace("map: /Game/Maps/Courtyard", "map: C:/Program Files/Git/Game/X"));
    let err = f.ws.reload().unwrap_err();
    assert!(err.msg.contains("MSYS_NO_PATHCONV"));
}

#[test]
fn scan_adds_anchors_and_keeps_comments() {
    let mut f = ws();
    let level = json!({"well": [0, 0, 80], "gate": [10, -1500, 100], "tower": [1200, 0, 900]});
    let store = f.ws.root.join(".shotpilot/mock_engine");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("level.json"), level.to_string()).unwrap();
    let rep = f.ws.scan("shotpilot", "SP_").unwrap();
    assert_eq!(rep.added, ids(&["tower"]));
    assert_eq!(rep.moved, ids(&["gate"]));
    assert_eq!(f.ws.defn.anchor("tower"), Some([1200.0, 0.0, 900.0]));
    assert!(f.text().contains("# Named points in the level."));
}

#[test]
fn ue_args_invalidate_engine_takes_only() {
    let mut f = ws();
    let s = f.ws.select_one("s02_well").unwrap();
    let (still, previz) = (fp::render_fingerprint(&f.ws.defn, &s, "still").unwrap(),
                           fp::render_fingerprint(&f.ws.defn, &s, "previz").unwrap());
    f.write(&f.text().replace("  isolate_userdir: true", "  ue_args: [-R7StageOnly]
  isolate_userdir: true"));
    f.ws.reload().unwrap();
    assert_eq!(f.ws.defn.project.ue_args, vec!["-R7StageOnly".to_string()]);
    let s = f.ws.select_one("s02_well").unwrap();
    assert_ne!(fp::render_fingerprint(&f.ws.defn, &s, "still").unwrap(), still);
    assert_eq!(fp::render_fingerprint(&f.ws.defn, &s, "previz").unwrap(), previz);
}

#[test]
fn fingerprints_match_the_python_version() {
    // Engine takes rendered before the Rust port must stay valid (same canonical JSON + SHA-256).
    let f = ws();
    let s = f.ws.select_one("s02_well").unwrap();
    assert_eq!(fp::build_fingerprint(&f.ws.defn, &s), PY_BUILD_S02);
    assert_eq!(fp::render_fingerprint(&f.ws.defn, &s, "still").unwrap(), PY_STILL_S02);
    // every camera move kind in the demo (dolly, crane, orbit, static)
    for (s, want) in f.ws.resolved().unwrap().iter().zip(PY_FINAL_FP12) {
        assert_eq!(&fp::render_fingerprint(&f.ws.defn, s, "final").unwrap()[..12], want, "{}", s.id);
    }
}

// Computed with the Python implementation (commit 9a84aeb) on the same DEMO template.
const PY_BUILD_S02: &str = "2541818c6abdd7ebe482d6484d5205cd4c2c7f88518f5fc0e65d51f83241cb07";
const PY_STILL_S02: &str = "88b637b4e357e81d799e9d4dbe97b035ed6220325f031f576264aca235cc4075";
const PY_FINAL_FP12: [&str; 6] = ["2dfed89ebbad", "c2c9a5ab699e", "28f84730860f", "687ec2bf2a35", "3b90dabba4c7", "76bcf5e812e6"];
