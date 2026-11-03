//! Sound (BGM + sound effects): edited and mixed without re-rendering a single shot.

mod common;

use std::cell::RefCell;

use common::{actions, ids, launches, ws, Fixture};
use serde_json::{json, Map, Value};
use shotpilot::ai::{self, Claude, Pointer};
use shotpilot::{sound, Result};

/// The demo project with its placeholder sounds on disk, rendered once at previz and still.
fn rendered() -> Fixture {
    let mut f = ws();
    sound::write_demo_audio(&f.ws.root).unwrap();
    f.ws.render(&[], "previz", false, None).unwrap();
    f.ws.render(&[], "still", false, None).unwrap();
    f
}

fn sfx_of(f: &Fixture, id: &str) -> Value {
    f.ws.defn.find(id).unwrap().get("sfx").cloned().unwrap_or(Value::Null)
}

#[test]
fn editing_sound_never_rerenders() {
    let mut f = rendered();
    let before = launches(&f.ws).len();
    let rep = f
        .ws
        .edit_sound(
            &[("s04_lantern".into(), Some(json!([{"file": "audio/hit.wav", "at": 0, "gain": -2}])))],
            None,
            "hit on the lantern",
        )
        .unwrap();
    assert_eq!(rep.edited, ids(&["s04_lantern"]));
    assert!(rep.stale.is_empty(), "sound made shots stale: {:?}", rep.stale);
    let rep = f.ws.set_audio(&[("bgm.gain".into(), json!(-10)), ("gain".into(), json!(2))]).unwrap();
    assert!(rep.audio && rep.stale.is_empty());

    for q in ["previz", "still"] {
        let out = f.ws.render(&[], q, false, None).unwrap();
        assert!(actions(&out).values().all(|a| a == "reused"), "{q}: {:?}", actions(&out));
    }
    assert_eq!(launches(&f.ws).len(), before, "the engine was launched for a sound edit");
    let last = f.ws.state.history().into_iter().rev().find(|e| e["action"] == "sound").unwrap();
    assert_eq!(last["stale"], json!([]));
}

#[test]
fn sound_edits_keep_the_rest_of_the_file() {
    let mut f = rendered();
    let text = f.text();
    f.ws.set_audio(&[("bgm.gain".into(), json!(-9))]).unwrap();
    let after = f.text();
    assert!(after.contains("bgm: {file: audio/drone.wav, gain: -9, fade_in: 1.5, fade_out: 2}"), "{after}");
    // every other line, comments included, is untouched
    let changed: Vec<(&str, &str)> = text.lines().zip(after.lines()).filter(|(a, b)| a != b).collect();
    assert_eq!(changed.len(), 1, "{changed:?}");
}

#[test]
fn mix_is_cached_until_the_sound_changes() {
    let mut f = rendered();
    let a = f.ws.mix().unwrap().unwrap();
    assert!(!a.reused);
    assert!(a.wav.is_file());
    assert_eq!(a.lanes.len(), 4); // BGM + three effects
    assert!(a.check["metrics"].as_array().is_some_and(|m| !m.is_empty()));
    let b = f.ws.mix().unwrap().unwrap();
    assert!(b.reused && b.key == a.key);

    f.ws.edit_sound(&[("s03_window".into(), Some(json!([{"file": "audio/hit.wav", "at": 1.0}])))], None, "").unwrap();
    let c = f.ws.mix().unwrap().unwrap();
    assert!(!c.reused && c.key != a.key);
}

#[test]
fn effects_move_with_their_shot() {
    let mut f = rendered();
    let at = |f: &Fixture| {
        let shots = f.ws.resolved().unwrap();
        sound::plan(&f.ws.defn, &shots).into_iter().find(|p| p.label == "silhouette").unwrap().start
    };
    assert!((at(&f) - (5.0 + 3.5 + 2.2)).abs() < 1e-6);
    let mut order = f.ws.defn.shot_ids();
    order.retain(|s| s != "s03_window");
    order.insert(0, "s03_window".into());
    let rep = f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    assert!(rep.stale.is_empty());
    assert!((at(&f) - 2.2).abs() < 1e-6);
}

#[test]
fn bad_sound_is_rejected_before_writing() {
    let mut f = rendered();
    let text = f.text();
    for (sfx, why) in [
        (json!([{"file": "../outside.wav", "at": 0}]), "inside the project"),
        (json!([{"file": "audio/hit.wav", "at": 99}]), "seconds from the shot's start"),
        (json!([{"file": "audio/notes.txt", "at": 0}]), "audio files are"),
        (json!([{"at": 0}]), "required"),
    ] {
        let e = f.ws.edit_sound(&[("s02_well".into(), Some(sfx))], None, "").unwrap_err();
        assert!(e.msg.contains(why), "{why}: {}", e.msg);
    }
    assert_eq!(f.text(), text);
}

#[test]
fn a_missing_file_does_not_block_the_cut() {
    let mut f = rendered();
    std::fs::remove_file(f.ws.root.join("audio/hit.wav")).unwrap();
    let info = f.ws.cut("previz", false).unwrap();
    assert!(info.audio.is_none());
    assert!(info.notes.iter().any(|n| n.contains("audio/hit.wav")), "{:?}", info.notes);
}

#[test]
fn removing_all_effects_of_a_shot_drops_the_key() {
    let mut f = rendered();
    f.ws.edit_sound(&[("s02_well".into(), None)], None, "").unwrap();
    assert_eq!(sfx_of(&f, "s02_well"), Value::Null);
    assert!(!f.text().contains("whoosh.wav, at: -0.9"));
}

// ------------------------------------------------------------------ Claude

struct FakeClaude {
    payload: Value,
    calls: RefCell<Vec<Value>>,
}

impl Claude for FakeClaude {
    fn ask(&self, request: &Value) -> Result<String> {
        self.calls.borrow_mut().push(request.clone());
        Ok(self.payload.to_string())
    }
}

fn fake(payload: Value) -> FakeClaude {
    FakeClaude { payload, calls: RefCell::new(Vec::new()) }
}

const S04_WITH_HIT_AND_NEW_LENS: &str = "id: s04_lantern
beat: escalate
purpose: 提灯が揺れる。風はない
duration: 4
camera:
  anchor: lantern
  offset: [-250, -150, -20]
  move: {type: orbit_left, degrees: 25}
  lens: {focal_length: 85}
sfx:
  - {file: audio/hit.wav, at: 0, gain: -2, label: lantern}
";

fn sound_pointer() -> Pointer {
    Pointer { sound: true, ..Default::default() }
}

#[test]
fn a_sound_note_takes_only_the_sfx() {
    let mut f = rendered();
    let c = fake(json!({"summary": "hit", "edits": [{"id": "s04_lantern", "yaml": S04_WITH_HIT_AND_NEW_LENS, "reason": "r"}],
                        "order": [], "audio": ""}));
    let rev = ai::revise_at(&f.ws.defn, "提灯のカットの頭にヒット", Some(&ids(&["s04_lantern"])), &sound_pointer(), &c).unwrap();
    let (_, node) = &rev.edits[0];
    // the lens change Claude slipped in is dropped; the sound is kept
    assert!(node["camera"].get("lens").is_none(), "{node}");
    assert_eq!(node["sfx"][0]["label"], "lantern");
    let rep = f.ws.edit_definition_with(&rev.edits, None, rev.audio.as_ref(), "revise", "n").unwrap();
    assert!(rep.stale.is_empty());
    let req = &c.calls.borrow()[0];
    let text = req["messages"][0]["content"].to_string();
    assert!(text.contains("audio/hit.wav") && text.contains("only `sfx`"), "{text}");
}

#[test]
fn bgm_is_out_of_scope_for_a_shot_note() {
    let f = rendered();
    let c = fake(json!({"summary": "x", "edits": [], "order": [],
                        "audio": "gain: 0\nbgm: {file: audio/drone.wav, gain: -20}"}));
    let e = ai::revise(&f.ws.defn, "もっと寄って", Some(&ids(&["s03_window"])), &c).unwrap_err();
    assert!(e.msg.contains("background music"), "{}", e.msg);
    // ...but in a sound note it may change
    let rev = ai::revise_at(&f.ws.defn, "BGM を下げて", None, &sound_pointer(), &c).unwrap();
    assert_eq!(rev.audio.unwrap()["bgm"]["gain"], json!(-20));
}

#[test]
fn claude_cannot_bring_in_missing_files_or_reorder_in_a_sound_note() {
    let f = rendered();
    let c = fake(json!({"summary": "x", "edits": [{"id": "s01_gate", "yaml":
        "id: s01_gate\nduration: 5\ncamera: {anchor: gate, offset: [-300, -250, 70]}\nsfx: [{file: audio/boom.wav, at: 0}]",
        "reason": "r"}], "order": [], "audio": ""}));
    let e = ai::revise_at(&f.ws.defn, "門にブーム", None, &sound_pointer(), &c).unwrap_err();
    assert!(e.msg.contains("audio/boom.wav"), "{}", e.msg);

    let mut order = f.ws.defn.shot_ids();
    order.reverse();
    let c = fake(json!({"summary": "x", "edits": [], "order": order, "audio": ""}));
    let e = ai::revise_at(&f.ws.defn, "BGM に合わせて", None, &sound_pointer(), &c).unwrap_err();
    assert!(e.msg.contains("reorder"), "{}", e.msg);
}

#[test]
fn audio_null_entries_survive_a_round_trip() {
    let mut f = rendered();
    let mut audio = Map::new();
    audio.insert("gain".into(), json!(-1));
    f.ws.edit_sound(&[], Some(&audio), "no bgm").unwrap();
    assert!(f.ws.defn.audio.get("bgm").map_or(true, Value::is_null));
    let shots = f.ws.resolved().unwrap();
    assert!(sound::plan(&f.ws.defn, &shots).iter().all(|p| p.role == sound::Role::Sfx));
}

/// A two-sound library in `dir`: a one-shot hit and a 30 s drone loop.
fn library_in(dir: &std::path::Path) -> shotpilot::library::Library {
    let sounds = dir.join("sounds");
    std::fs::create_dir_all(sounds.join("splice/Kit")).unwrap();
    let pcm = sound::Pcm { l: vec![0.2; 4800], r: vec![0.2; 4800] };
    sound::write_wav16(&sounds.join("splice/Kit/lib_hit.wav"), &pcm).unwrap();
    sound::write_wav16(&sounds.join("splice/Kit/lib_drone.wav"), &pcm).unwrap();
    let index = json!({"sounds": [
        {"id": "splice:hit1", "path": "splice/Kit/lib_hit.wav", "kind": "oneshot", "duration_s": 0.1, "tags": ["impact"], "status": "ok"},
        {"id": "splice:drone1", "path": "splice/Kit/lib_drone.wav", "kind": "loop", "duration_s": 30.0, "tags": ["drones"], "status": "ok"},
    ]});
    std::fs::write(sounds.join("library.json"), index.to_string()).unwrap();
    shotpilot::library::Library::open(Some(dir)).unwrap()
}

#[test]
fn a_sound_note_can_take_sounds_from_the_library() {
    let mut f = rendered();
    let lib_dir = tempfile::tempdir().unwrap();
    let pointer = Pointer { sound: true, library: Some(library_in(lib_dir.path())), ..Default::default() };
    let c = fake(json!({"summary": "x", "edits": [{"id": "s01_gate", "yaml":
        "id: s01_gate\nduration: 5\ncamera: {anchor: gate, offset: [-300, -250, 70]}\nsfx: [{file: 'lib:splice:hit1', at: 0}]",
        "reason": "r"}], "order": [], "audio": "bgm: {file: 'lib:splice:drone1', gain: -8}"}));
    let rev = ai::revise_at(&f.ws.defn, "門にヒット、BGM はドローン", None, &pointer, &c).unwrap();
    // the library was offered, and what Claude picked was copied in and rewritten to project paths
    let req = c.calls.borrow()[0].to_string();
    assert!(req.contains("lib:<id>") && req.contains("splice:drone1"), "{req}");
    assert_eq!(rev.edits[0].1["sfx"][0]["file"], "audio/lib/lib_hit.wav");
    assert_eq!(rev.audio.as_ref().unwrap()["bgm"]["file"], "audio/lib/lib_drone.wav");
    assert!(f.ws.root.join("audio/lib/lib_hit.wav").is_file());
    let before = launches(&f.ws).len();
    let rep = f.ws.edit_definition_with(&rev.edits, None, rev.audio.as_ref(), "revise", "n").unwrap();
    assert!(rep.stale.is_empty());
    assert_eq!(launches(&f.ws).len(), before, "taking a sound never renders");

    // an id the library does not have is refused
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "audio": "bgm: {file: 'lib:splice:nope'}"}));
    let e = ai::revise_at(&f.ws.defn, "BGM", None, &pointer, &c).unwrap_err();
    assert!(e.msg.contains("splice:nope"), "{}", e.msg);
}

#[test]
fn a_camera_note_is_not_shown_the_library() {
    let f = rendered();
    let lib_dir = tempfile::tempdir().unwrap();
    let pointer = Pointer { sound: false, library: Some(library_in(lib_dir.path())), ..Default::default() };
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "audio": ""}));
    ai::revise_at(&f.ws.defn, "門をもっと寄って", None, &pointer, &c).unwrap();
    assert!(!c.calls.borrow()[0].to_string().contains("splice:drone1"));
}
