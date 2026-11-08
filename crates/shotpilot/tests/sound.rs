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
    // the notes stay with the cut in the history: the app's list of findings reads them from there (D-95)
    let logged = f.ws.state.history().into_iter().rev().find(|e| e["action"] == "cut").unwrap();
    assert_eq!(logged["notes"], json!(info.notes));
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

/// The fixture with a script: a card after s02 with a hit, a caption over s03.
fn with_script() -> Fixture {
    let mut f = rendered();
    let mut text = f.text();
    text.push_str("\nscript:\n  - {id: seven, kind: card, ja: 自分を入れて、七人。, dur: 2, after: s02_well, sfx: [{file: audio/hit.wav, at: 0, gain: -6}]}\n  - {id: cups, kind: caption, ja: コップ、七つ。, on: s03_window, at: 0.5, dur: 2}\n");
    f.write(&text);
    f.ws.reload().unwrap();
    f
}

#[test]
fn a_sound_note_reaches_script_lines_and_music_cues() {
    let mut f = with_script();
    let lib_dir = tempfile::tempdir().unwrap();
    let pointer = Pointer { sound: true, library: Some(library_in(lib_dir.path())), ..Default::default() };
    let c = fake(json!({"summary": "x", "edits": [], "order": [],
        "audio": "loudness: -14\nmusic:\n  - {file: 'lib:splice:drone1', from: seven, until: s05_door, gain: -10}",
        "lines": [{"id": "seven", "sfx": "- {file: audio/hit.wav, at: 0, gain: -2}"},
                  {"id": "cups", "sfx": "- {file: 'lib:splice:hit1', at: 0}"}]}));
    let rev = ai::revise_at(&f.ws.defn, "カードの音を強く、コップにも音", None, &pointer, &c).unwrap();
    // Claude was shown the script with where each line sits
    let req = c.calls.borrow()[0].to_string();
    assert!(req.contains("Script lines") && req.contains("seven (card, after s02_well"), "{req}");
    assert_eq!(rev.audio.as_ref().unwrap()["music"][0]["file"], "audio/lib/lib_drone.wav");
    let before = launches(&f.ws).len();
    let rep = f.ws.edit_definition_full(&rev.edits, &rev.lines, None, rev.audio.as_ref(), "revise", "n").unwrap();
    assert!(rep.stale.is_empty());
    assert_eq!(launches(&f.ws).len(), before, "sound never renders");
    let line = |id: &str| f.ws.defn.script.iter().find(|l| l.id == id).unwrap().sfx.clone();
    assert_eq!(line("seven")[0]["gain"], json!(-2));
    assert_eq!(line("cups")[0]["file"], "audio/lib/lib_hit.wav");
    assert!(f.text().contains("ja: 自分を入れて、七人。"), "the rest of the line is kept");
}

#[test]
fn a_note_on_one_lines_sound_changes_only_that_line() {
    let f = with_script();
    let pointer = Pointer { sound: true, line: Some("seven".into()), ..Default::default() };
    // the other line is out of scope
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "audio": "",
        "lines": [{"id": "cups", "sfx": "- {file: audio/hit.wav, at: 0}"}]}));
    let e = ai::revise_at(&f.ws.defn, "小さく", None, &pointer, &c).unwrap_err();
    assert!(e.msg.contains("cups"), "{}", e.msg);
    // so is the music
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "audio": "bgm: {file: audio/drone.wav, gain: -20}",
        "lines": [{"id": "seven", "sfx": ""}]}));
    assert!(ai::revise_at(&f.ws.defn, "小さく", None, &pointer, &c).unwrap_err().msg.contains("background music"));
    // removing its effect is fine
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "audio": "", "lines": [{"id": "seven", "sfx": ""}]}));
    let rev = ai::revise_at(&f.ws.defn, "音を消して", None, &pointer, &c).unwrap();
    assert_eq!(rev.lines, vec![("seven".to_string(), Value::Null)]);
    // a camera note cannot touch a line's sound
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "audio": "", "lines": [{"id": "seven", "sfx": ""}]}));
    assert!(ai::revise(&f.ws.defn, "寄って", None, &c).is_err());
}

#[test]
fn a_music_cue_must_start_on_a_piece_of_the_cut() {
    let f = with_script();
    let c = fake(json!({"summary": "x", "edits": [], "order": [], "lines": [],
        "audio": "music:\n  - {file: audio/drone.wav, from: s09_nowhere}"}));
    let e = ai::revise_at(&f.ws.defn, "曲", None, &sound_pointer(), &c).unwrap_err();
    assert!(e.msg.contains("s09_nowhere"), "{}", e.msg);
}

#[test]
fn tuning_one_sound_filters_it_and_never_rerenders() {
    let mut f = with_script();
    let before = launches(&f.ws).len();
    let a = f.ws.mix().unwrap().unwrap();
    // how much of the sound is heard: the area under its lane's peaks (the max is the
    // onset's noise crack, which a low cut leaves)
    let peak = |m: &sound::Mixed, shot: &str| -> f64 {
        let l = m.lanes.iter().find(|l| l["shot"] == shot).unwrap();
        l["peaks"].as_array().unwrap().iter().filter_map(Value::as_f64).sum()
    };
    // the hit on s03 loses its sub (the demo hit is mostly a 40-130 Hz thump), the card's hit
    // gets louder and duller
    let rep = f
        .ws
        .edit_sound_full(
            &[("s03_window".into(), Some(json!([{"file": "audio/hit.wav", "at": 2.2, "gain": -3, "low_cut": 400}])))],
            &[("seven".into(), json!([{"file": "audio/hit.wav", "at": 0, "gain": 0, "high_cut": 2000}]))],
            None,
            "duller hit, louder card",
        )
        .unwrap();
    assert!(rep.stale.is_empty(), "sound made shots stale: {:?}", rep.stale);
    let line = f.ws.defn.script.iter().find(|l| l.id == "seven").unwrap();
    assert_eq!(line.sfx[0]["high_cut"], json!(2000));
    assert!(f.text().contains("low_cut: 400"), "{}", f.text());

    let b = f.ws.mix().unwrap().unwrap();
    assert!(!b.reused && b.key != a.key);
    assert!(peak(&b, "s03_window") < peak(&a, "s03_window") * 0.5, "the low cut did not take the thump off");
    assert!(peak(&b, "seven") > peak(&a, "seven"), "the card's hit did not get louder");
    let out = f.ws.render(&[], "previz", false, None).unwrap();
    assert!(actions(&out).values().all(|a| a == "reused"), "{:?}", actions(&out));
    assert_eq!(launches(&f.ws).len(), before, "the engine was launched for a sound edit");

    // a low cut above the high cut leaves nothing: refused, nothing written
    let text = f.text();
    let e = f
        .ws
        .edit_sound(&[("s03_window".into(), Some(json!([{"file": "audio/hit.wav", "at": 2.2, "high_cut": 300, "low_cut": 900}])))], None, "")
        .unwrap_err();
    assert!(e.to_string().contains("low_cut"), "{e}");
    assert_eq!(f.text(), text);
}

/// The air of `places` written into `audio.ambience` (the rest of `audio` kept).
fn with_air(f: &mut Fixture, places: Value) {
    let mut audio = f.ws.defn.audio.clone();
    audio.insert("ambience".into(), places);
    f.ws.edit_sound(&[], Some(&audio), "air").unwrap();
}

fn airs(f: &Fixture) -> Vec<sound::Placement> {
    let shots = f.ws.resolved().unwrap();
    sound::plan(&f.ws.defn, &shots).into_iter().filter(|p| p.role == sound::Role::Ambience).collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.02
}

#[test]
fn the_air_of_a_place_follows_the_shots_that_film_it() {
    // s01 gate, s02 well, [card seven], s03 window, s04 lantern, s05 door, s06 well
    let mut f = with_script();
    std::fs::copy(f.ws.root.join("audio/hit.wav"), f.ws.root.join("audio/room.wav")).unwrap();
    with_air(&mut f, json!({
        "gate": "audio/room.wav",
        "well": {"file": "audio/drone.wav", "gain": -8, "label": "風"},
        "window": {"file": "audio/room.wav"},
    }));
    let shots = f.ws.resolved().unwrap();
    let (spans, _) = sound::spans(&f.ws.defn, &shots);
    let at = |id: &str| spans[id];
    let air = airs(&f);
    let by = |first: &str| {
        air.iter().find(|p| p.shot.as_deref() == Some(first)).unwrap_or_else(|| panic!("no air from {first}: {air:?}")).clone()
    };
    assert_eq!(air.len(), 4, "{air:?}");
    // one stretch per run of shots of a place, looping, at the place's level (or -10 dB)
    let gate = by("s01_gate");
    assert!(near(gate.start, at("s01_gate").0) && near(gate.until.unwrap(), at("s01_gate").1), "{gate:?}");
    assert!(gate.looped && gate.gain_db == sound::AMBIENCE_GAIN && gate.label == "gate", "{gate:?}");
    let well = by("s02_well");
    assert!(near(well.until.unwrap(), at("s02_well").1) && well.gain_db == -8.0 && well.label == "風", "{well:?}");
    // across the card into another place, the next air comes in a little early (a J-cut)
    let window = by("s03_window");
    assert!(near(window.start, at("s03_window").0 - sound::AMBIENCE_LEAD), "{window:?} vs {:?}", at("s03_window"));
    // the well again at the end: a stretch of its own, after places that have no air
    assert!(near(by("s06_title").start, at("s06_title").0));

    // a shot of the well that borrows the gate's air, and a shot left without any
    f.ws.set_fields("s02_well", &[("ambience".into(), json!("gate"))]).unwrap();
    f.ws.set_fields("s06_title", &[("ambience".into(), json!("none"))]).unwrap();
    let air = airs(&f);
    assert_eq!(air.len(), 2, "{air:?}");
    let gate = air.iter().find(|p| p.place.as_deref() == Some("gate")).unwrap();
    assert!(near(gate.until.unwrap(), at("s02_well").1), "the gate's air carries on under s02: {gate:?}");
    assert!(air.iter().all(|p| p.shot.as_deref() != Some("s06_title")));
}

#[test]
fn a_shot_of_no_place_keeps_the_air_before_it_and_reordering_moves_the_air() {
    let mut f = rendered();
    with_air(&mut f, json!({"gate": "audio/drone.wav", "well": "audio/drone.wav"}));
    // s02 no longer stands by an anchor: it hears the gate before it
    f.ws.set_fields("s02_well", &[("camera".into(), json!({"location": [0, 0, 160], "rotation": [0, 0, 0]}))]).unwrap();
    let shots = f.ws.resolved().unwrap();
    assert_eq!(sound::heard(&f.ws.defn, &shots)[1].as_deref(), Some("gate"));
    let gate = airs(&f).into_iter().find(|p| p.place.as_deref() == Some("gate")).unwrap();
    let (spans, _) = sound::spans(&f.ws.defn, &shots);
    assert!(near(gate.until.unwrap(), spans["s02_well"].1), "{gate:?}");

    // the air is the place's: moved with the shots, never by position
    let mut order = f.ws.defn.shot_ids();
    order.reverse();
    f.ws.edit_definition(&[], Some(&order), "order", "").unwrap();
    let shots = f.ws.resolved().unwrap();
    let (spans, _) = sound::spans(&f.ws.defn, &shots);
    let gate = airs(&f).into_iter().find(|p| p.shot.as_deref() == Some("s01_gate")).unwrap();
    assert!(near(gate.start, spans["s01_gate"].0), "{gate:?}");
}

#[test]
fn editing_the_air_never_rerenders() {
    let mut f = rendered();
    let before = launches(&f.ws).len();
    let lanes = f.ws.mix().unwrap().unwrap().lanes.len();
    let mut audio = f.ws.defn.audio.clone();
    audio.insert("ambience".into(), json!({"gate": {"file": "audio/drone.wav", "gain": -12}, "well": "audio/whoosh.wav"}));
    let rep = f.ws.edit_sound(&[], Some(&audio), "air").unwrap();
    assert!(rep.stale.is_empty(), "{:?}", rep.stale);
    let rep = f.ws.set_fields("s03_window", &[("ambience".into(), json!("none"))]).unwrap();
    assert!(rep.stale.is_empty(), "a shot's own air made {:?} stale", rep.stale);
    // the air is mixed (a lane per stretch) and the file keeps it
    let mix = f.ws.mix().unwrap().unwrap();
    assert!(mix.lanes.len() > lanes, "{} lanes, {lanes} before", mix.lanes.len());
    assert!(f.text().contains("ambience:"));
    for q in ["previz", "still"] {
        let out = f.ws.render(&[], q, false, None).unwrap();
        assert!(actions(&out).values().all(|a| a == "reused"), "{q}: {:?}", actions(&out));
    }
    assert_eq!(launches(&f.ws).len(), before, "the engine was launched for the air");
    // undo takes the shot's own air back
    f.ws.undo().unwrap();
    assert!(f.ws.defn.find("s03_window").unwrap().get("ambience").is_none());
}

#[test]
fn bad_air_is_rejected_before_writing() {
    let mut f = rendered();
    let text = f.text();
    let mut audio = f.ws.defn.audio.clone();
    audio.insert("ambience".into(), json!(["audio/drone.wav"]));
    assert!(f.ws.edit_sound(&[], Some(&audio), "").is_err());
    audio.insert("ambience".into(), json!({"gate": {"file": "audio/drone.wav", "volume": 3}}));
    assert!(f.ws.edit_sound(&[], Some(&audio), "").is_err());
    assert!(f.ws.set_fields("s01_gate", &[("ambience".into(), json!(3))]).is_err());
    assert_eq!(f.text(), text, "a rejected edit wrote the file");
}

#[test]
fn a_sound_note_may_give_a_place_its_air_and_silence_a_shot() {
    let mut f = rendered();
    let lib_dir = tempfile::tempdir().unwrap();
    let pointer = Pointer { sound: true, library: Some(library_in(lib_dir.path())), ..Default::default() };
    let s04 = format!("{S04_WITH_HIT_AND_NEW_LENS}ambience: none\n");
    let c = fake(json!({"summary": "x", "edits": [{"id": "s04_lantern", "yaml": s04, "reason": "r"}], "order": [],
        "audio": "ambience:\n  gate: {file: 'lib:splice:drone1', gain: -9, label: 風}"}));
    let rev = ai::revise_at(&f.ws.defn, "門に風、提灯のカットは無音で", None, &pointer, &c).unwrap();
    let req = c.calls.borrow()[0].to_string();
    assert!(req.contains("audio.ambience"), "Claude is told where the air goes");
    // the air came from the library into the project; the lens Claude slipped in did not
    assert_eq!(rev.audio.as_ref().unwrap()["ambience"]["gate"]["file"], "audio/lib/lib_drone.wav");
    let node = &rev.edits[0].1;
    assert_eq!(node["ambience"], "none");
    assert!(node["camera"].get("lens").is_none(), "{node}");
    let before = launches(&f.ws).len();
    let rep = f.ws.edit_definition_with(&rev.edits, None, rev.audio.as_ref(), "revise", "n").unwrap();
    assert!(rep.stale.is_empty());
    assert_eq!(launches(&f.ws).len(), before);
    assert!(airs(&f).iter().any(|p| p.place.as_deref() == Some("gate")));

    // a shot may only borrow the air of a place that exists
    let s04 = format!("{S04_WITH_HIT_AND_NEW_LENS}ambience: basement\n");
    let c = fake(json!({"summary": "x", "edits": [{"id": "s04_lantern", "yaml": s04, "reason": "r"}], "order": [], "audio": ""}));
    let e = ai::revise_at(&f.ws.defn, "地下の音", None, &pointer, &c).unwrap_err();
    assert!(e.msg.contains("basement"), "{}", e.msg);
}
