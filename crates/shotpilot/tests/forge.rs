//! Sounds made in code (D-103): an effect the library lacks is synthesized by shotpilot into the
//! project's audio/made/, placed by its moment, and never re-renders a shot. No outside program:
//! these tests render the real recipes.

mod common;

use std::cell::RefCell;
use std::path::Path;

use common::{launches, ws};
use serde_json::{json, Value};
use shotpilot::ai::{self, Claude, Pointer};
use shotpilot::library::Library;
use shotpilot::{forge, sound, Result};

/// A library with one drone loop; `Library::open` offers the sounds made in code next to it.
fn library(dir: &Path) -> Library {
    let sounds = dir.join("sounds");
    std::fs::create_dir_all(sounds.join("p")).unwrap();
    sound::write_wav16(&sounds.join("p/drone.wav"), &sound::Pcm { l: vec![0.1; 4800], r: vec![0.1; 4800] }).unwrap();
    std::fs::write(sounds.join("library.json"), json!({"sounds": [
        {"id": "x:drone", "path": "p/drone.wav", "kind": "loop", "duration_s": 64.0, "tags": ["drone"], "status": "ok"},
    ]}).to_string()).unwrap();
    let lib = Library::open(Some(dir)).unwrap();
    assert!(lib.made.is_some(), "the sounds made in code are always offered");
    lib
}

struct Designer {
    reply: Value,
    asked: RefCell<Vec<String>>,
}

impl Claude for Designer {
    fn ask(&self, request: &Value) -> Result<String> {
        // the user turn is one text, or blocks of text (and images)
        let text = match &request["messages"][0]["content"] {
            Value::String(s) => s.clone(),
            Value::Array(blocks) => blocks.iter().filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n"),
            _ => String::new(),
        };
        self.asked.borrow_mut().push(text);
        Ok(self.reply.to_string())
    }
}

#[test]
fn a_hit_the_library_lacks_is_made_in_code_and_lands_on_its_moment() {
    let mut f = ws();
    let mut text = f.text();
    text.push_str("\nscript:\n  - {id: seven, kind: card, ja: 自分を入れて、七人。, dur: 2, after: s02_well}\n");
    f.write(&text);
    f.ws.reload().unwrap();
    sound::write_demo_audio(&f.ws.root).unwrap();
    f.ws.render(&[], "previz", false, None).unwrap();
    let before = launches(&f.ws).len();
    let libdir = tempfile::tempdir().unwrap();
    let lib = library(libdir.path());
    let d = Designer {
        reply: json!({"rationale": "r",
            "music": [{"lib": "x:drone", "from": "start", "until": "seven", "at": 0, "gain": -11, "fade_in": 1, "fade_out": 0.5, "label": "drone"},
                      {"lib": "make:riser/tension", "from": "seven", "until": "end", "at": 0, "gain": -11, "fade_in": 0, "fade_out": 0, "label": "x"}],
            "hits": [{"lib": "make:riser/tension", "on": "s03_window", "at": 0, "gain": -4, "label": "build"},
                     {"lib": "make:impact/deep?pitch=C2", "on": "seven", "at": 0, "gain": -3, "label": "boom"},
                     {"lib": "make:impact/deep?pitch=C2", "on": "s05_door", "at": 1.0, "gain": -6, "label": "door"},
                     {"lib": "make:glass/nope", "on": "s04_lantern", "at": 0, "gain": 0, "label": "y"}],
            "ambience": [], "missing": ["low strings"]}),
        asked: RefCell::new(vec![]),
    };
    let design = shotpilot::sounddesign::design(&f.ws, &lib, &d).unwrap();
    // the designer saw the catalogue, how to name a made sound and where its `at` lands
    let prompt = d.asked.borrow()[0].clone();
    assert!(prompt.contains("- make:riser/tension 2.8s [riser] knobs: dur, pitch (own A2):"), "{prompt}");
    assert!(prompt.contains("Sounds made in code") && prompt.contains("where its moment lands"), "{prompt}");
    assert!(shotpilot::craft::self_review_words(&prompt).is_empty());
    // the unknown sound and the made "music" are left out and said so
    assert!(design.notes.iter().any(|n| n.contains("glass/nope")), "{:?}", design.notes);
    assert!(design.notes.iter().any(|n| n.contains("effects, not music")), "{:?}", design.notes);
    assert!(design.notes.iter().any(|n| n == "コードで作った音: impact/deep / riser/tension"), "{:?}", design.notes);
    assert_eq!(design.music.len(), 1);
    shotpilot::sounddesign::apply(&mut f.ws, &design).unwrap();
    // each made sound is one file, though the boom lands twice
    let made: Vec<String> = std::fs::read_dir(f.ws.root.join(forge::MADE_DIR)).unwrap().flatten()
        .map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".wav")).collect();
    assert_eq!(made.len(), 2, "{made:?}");

    // the riser's top (at the end of its 2.8 s) lands on the cut into s03: the file starts that
    // much earlier, as far as a sound may lead
    let build = &f.ws.defn.find("s03_window").unwrap()["sfx"][0];
    let file = build["file"].as_str().unwrap();
    assert!(file.starts_with("audio/made/riser-tension-s") && f.ws.root.join(file).is_file(), "{file}");
    let at = build["at"].as_f64().unwrap();
    assert!((-2.85..=-2.6).contains(&at), "{at}");
    // a hit lands its onset: the boom starts on the second it was asked for
    let door = &f.ws.defn.find("s05_door").unwrap()["sfx"][0];
    assert!(door["file"].as_str().unwrap().starts_with("audio/made/impact-deep-pC2-s"), "{door}");
    let at = door["at"].as_f64().unwrap();
    assert!((0.95..=1.0).contains(&at), "{at}");
    assert_eq!(f.ws.defn.script[0].sfx[0]["file"], door["file"]);
    // it mixes, and nothing re-rendered for sound
    let shots = f.ws.resolved().unwrap();
    let plan = sound::plan(&f.ws.defn, &shots);
    let riser = plan.iter().find(|p| p.label == "build").unwrap();
    assert!((riser.start - (5.0 + 3.5 + 2.0 + build["at"].as_f64().unwrap())).abs() < 1e-6, "{}", riser.start);
    assert!(f.ws.mix().unwrap().is_some());
    assert_eq!(launches(&f.ws).len(), before, "sound never renders a shot");
    let fresh = |q: &str| f.ws.status().unwrap().iter().all(|s| s.qualities.iter().any(|(k, v)| k == q && v == "fresh"));
    assert!(fresh("previz"), "every previz take is still the one to use");

    // with no library at all, the designer still makes effects and keeps the project's music
    let music_before = f.ws.defn.audio.get("music").cloned();
    let bare = Library { made: Some(forge::bank()), ..Library::default() };
    let d = Designer { reply: json!({"rationale": "r", "music": [], "hits": [], "ambience": [], "missing": []}), asked: RefCell::new(vec![]) };
    let design = shotpilot::sounddesign::design(&f.ws, &bare, &d).unwrap();
    assert!(d.asked.borrow()[0].contains("music and the air already in the project stay"));
    shotpilot::sounddesign::apply(&mut f.ws, &design).unwrap();
    assert_eq!(f.ws.defn.audio.get("music").cloned(), music_before, "no library: the music already there stays");
    assert!(shotpilot::sounddesign::design(&f.ws, &Library::default(), &d).is_err(), "nothing to pick from");
}

#[test]
fn a_made_sound_is_made_once_per_project_and_each_game_gets_its_own_take() {
    let games = tempfile::tempdir().unwrap();
    let (a, b) = (games.path().join("KettleHill"), games.path().join("Gridhold"));
    for p in [&a, &b] {
        std::fs::create_dir_all(p).unwrap();
    }
    let bank = forge::bank();
    let first = bank.make("make:impact/deep", &a).unwrap();
    let written = std::fs::metadata(a.join(&first.file)).unwrap().modified().unwrap();
    let again = bank.make("make:impact/deep", &a).unwrap();
    assert_eq!(first.file, again.file);
    assert_eq!(std::fs::metadata(a.join(&first.file)).unwrap().modified().unwrap(), written, "one definition, one render");
    assert_eq!(first.moment_s, again.moment_s);
    // another game picks the same sound and gets a take of its own
    let other = bank.make("make:impact/deep", &b).unwrap();
    assert_ne!(first.file, other.file);
    assert_ne!(std::fs::read(a.join(&first.file)).unwrap(), std::fs::read(b.join(&other.file)).unwrap());
    // knobs it cannot take are dropped and said so: the air riser has no pitch, nothing goes past 30 s
    let p = bank.make("make:riser/air?pitch=C2&dur=99", &a).unwrap();
    assert_eq!(p.notes.len(), 2, "{:?}", p.notes);
    assert!(p.file.starts_with("audio/made/riser-air-s"), "{}", p.file);
    // a length it can take shapes it, and the file says so
    let short = bank.make("make:riser/air?dur=1.5", &a).unwrap();
    assert!(short.file.starts_with("audio/made/riser-air-d1.5-s") && (1.3..=1.5).contains(&short.moment_s), "{short:?}");
    assert!(bank.make("make:glass/nope", &a).unwrap_err().msg.contains("glass/nope"));
}

#[test]
fn a_sound_note_may_make_a_sound_in_code() {
    let mut f = ws();
    f.ws.render(&[], "previz", false, None).unwrap();
    let before = launches(&f.ws).len();
    let libdir = tempfile::tempdir().unwrap();
    let pointer = Pointer { sound: true, library: Some(library(libdir.path())), ..Default::default() };
    let reply = json!({"summary": "x", "edits": [{"id": "s01_gate", "yaml":
        "id: s01_gate\nduration: 5\ncamera: {anchor: gate, offset: [-300, -250, 70]}\nsfx: [{file: 'make:riser/tension?dur=2', at: 0.5, gain: -4}]",
        "reason": "r"}], "order": [], "audio": ""});
    let c = Designer { reply: reply.clone(), asked: RefCell::new(vec![]) };
    let rev = ai::revise_at(&f.ws.defn, "門の前でじわっと上げて", Some(&["s01_gate".to_string()]), &pointer, &c).unwrap();
    let req = c.asked.borrow()[0].clone();
    assert!(req.contains("`file: make:<id>`") && req.contains("make:impact/deep"), "{req}");
    let sfx = &rev.edits[0].1["sfx"][0];
    assert!(sfx["file"].as_str().unwrap().starts_with("audio/made/riser-tension-d2-s"), "{sfx}");
    let at = sfx["at"].as_f64().unwrap();
    assert!((-1.5..=-1.3).contains(&at), "its top (at the end of its 2 s) lands 0.5 s into the shot: {at}");
    let rep = f.ws.edit_definition_with(&rev.edits, None, rev.audio.as_ref(), "revise", "n").unwrap();
    assert!(rep.stale.is_empty());
    assert_eq!(launches(&f.ws).len(), before, "a made sound never renders a shot");

    // a library built without the sounds made in code applies nothing
    let bare = Pointer { sound: true, library: Some(Library::default()), ..Default::default() };
    let c = Designer { reply, asked: RefCell::new(vec![]) };
    let e = ai::revise_at(&f.ws.defn, "門の前でじわっと上げて", Some(&["s01_gate".to_string()]), &bare, &c).unwrap_err();
    assert!(e.msg.contains("none are offered"), "{}", e.msg);
    assert!(!c.asked.borrow()[0].contains("make:<id>"), "none offered, none listed");
}

/// The project's sound so far: every file its audio and its effects name.
fn sound_files(ws: &shotpilot::Workspace) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for c in ws.defn.audio.get("music").and_then(Value::as_array).into_iter().flatten() {
        files.push(c["file"].as_str().unwrap_or("").to_string());
    }
    for v in ws.defn.audio.get("ambience").and_then(Value::as_object).into_iter().flat_map(|m| m.values()) {
        files.push(v.as_str().or_else(|| v["file"].as_str()).unwrap_or("").to_string());
    }
    for s in &ws.defn.shots {
        for e in s.get("sfx").and_then(Value::as_array).into_iter().flatten() {
            files.push(e["file"].as_str().unwrap_or("").to_string());
        }
    }
    files
}

#[test]
fn a_sample_teaser_takes_no_library_sound_and_makes_its_music_and_air() {
    // a three project films the sample game, which anyone can fetch and film again: it takes no
    // library sound unless it says so; any other project may say so too
    let home = tempfile::tempdir().unwrap();
    let three = shotpilot::projects::create(&shotpilot::projects::NewProject {
        location: home.path().to_path_buf(),
        name: "村".into(),
        engine: "three".into(),
        pitch: "丘の上の村を歩くように見せたい".into(),
        kind: "teaser".into(),
        ..Default::default()
    })
    .unwrap();
    assert!(!sound::uses_library(&shotpilot::Workspace::open(&three).unwrap().defn));
    let mut f = ws();
    assert!(sound::uses_library(&f.ws.defn), "a UE or mock project picks from the library");
    let text = f.text().replacen("audio:\n  gain: 0\n", "audio:\n  library: false\n  gain: 0\n", 1);
    assert!(text.contains("library: false"));
    f.write(&text);
    f.ws.reload().unwrap();
    assert!(!sound::uses_library(&f.ws.defn));
    sound::write_demo_audio(&f.ws.root).unwrap(); // the template's own bgm file
    f.ws.render(&[], "previz", false, None).unwrap();
    let before = launches(&f.ws).len();

    // the one-go sound design: the library is not shown, the music and air made in code are
    let libdir = tempfile::tempdir().unwrap();
    let lib = library(libdir.path());
    let d = Designer {
        reply: json!({"rationale": "r",
            "music": [{"lib": "make:music/musicbox?dur=20", "from": "start", "until": "end", "at": 0, "gain": -12, "fade_in": 0.5, "fade_out": 1, "label": "box"},
                      {"lib": "x:drone", "from": "start", "until": "end", "at": 0, "gain": -12, "fade_in": 0, "fade_out": 0, "label": "from the library"},
                      {"lib": "make:impact/deep", "from": "start", "until": "end", "at": 0, "gain": -12, "fade_in": 0, "fade_out": 0, "label": "a hit as music"}],
            "hits": [{"lib": "make:impact/deep", "on": "s03_window", "at": 0, "gain": -4, "label": "boom"},
                     {"lib": "make:air/breeze", "on": "s02_well", "at": 0, "gain": -4, "label": "air on a cut"},
                     {"lib": "x:drone", "on": "s04_lantern", "at": 0, "gain": -4, "label": "from the library"}],
            "ambience": [{"place": "gate", "lib": "make:air/breeze", "gain": -10, "label": "breeze"},
                         {"place": "well", "lib": "x:drone", "gain": -10, "label": "from the library"},
                         {"place": "window", "lib": "make:music/pad", "gain": -10, "label": "music as air"}],
            "missing": []}),
        asked: RefCell::new(vec![]),
    };
    let design = shotpilot::sounddesign::design(&f.ws, &lib, &d).unwrap();
    let prompt = d.asked.borrow()[0].clone();
    assert!(!prompt.contains("x:drone"), "the library is not offered: {prompt}");
    assert!(prompt.contains("- make:music/musicbox 48s [bgm, music] knobs: dur:"), "{prompt}");
    assert!(prompt.contains("make:air/breeze") && prompt.contains("make:impact/deep"), "{prompt}");
    assert!(prompt.contains("This teaser takes no sound from the library"), "{prompt}");
    assert!(shotpilot::craft::self_review_words(&prompt).is_empty());
    assert_eq!(design.notes[0], "音源ライブラリは使わず、音はすべてコードで作りました（audio.library: false）");
    for left in ["x:drone", "a music cue takes a make:music/", "not on a cut", "a place takes a make:air/"] {
        assert!(design.notes.iter().any(|n| n.contains(left)), "{left}: {:?}", design.notes);
    }
    assert!(!design.keep_music, "the music is made, not kept");
    shotpilot::sounddesign::apply(&mut f.ws, &design).unwrap();
    assert_eq!(f.ws.defn.audio.get("library"), Some(&json!(false)), "the human's choice stays");
    assert!(!f.ws.root.join(shotpilot::library::IMPORT_DIR).exists(), "nothing was copied from the library");
    let files = sound_files(&f.ws);
    assert!(!files.is_empty() && files.iter().all(|p| p.starts_with("audio/made/")), "{files:?}");
    let music = f.ws.defn.audio["music"][0]["file"].as_str().unwrap().to_string();
    assert!(music.starts_with("audio/made/music-musicbox-d20-s"), "{music}");
    let len = sound::decode(&f.ws.root.join(&music)).unwrap().len() as f64 / sound::SR as f64;
    assert!((len - 20.0).abs() < 0.01, "the cue is as long as asked: {len}");
    let air = f.ws.defn.audio["ambience"]["gate"]["file"].as_str().unwrap();
    assert!(air.starts_with("audio/made/air-breeze-s"), "{air}");
    assert_eq!(f.ws.defn.audio["ambience"].as_object().unwrap().len(), 1);
    assert!(f.ws.defn.find("s03_window").unwrap()["sfx"][0]["file"].as_str().unwrap().starts_with("audio/made/impact-deep-s"));
    let well = f.ws.defn.find("s02_well").unwrap();
    assert!(well.get("sfx").and_then(Value::as_array).is_none_or(|a| a.is_empty()), "{well}");
    // it mixes, and sound never renders a shot
    assert!(f.ws.mix().unwrap().is_some());
    assert_eq!(launches(&f.ws).len(), before);
    assert!(f.ws.status().unwrap().iter().all(|s| s.qualities.iter().any(|(k, v)| k == "previz" && v == "fresh")));

    // a sound note: made music and air go in, a library sound does not, and Claude cannot turn
    // the library back on
    let pointer = Pointer { sound: true, library: Some(library(libdir.path())), ..Default::default() };
    let reply = |audio: &str| json!({"summary": "x", "edits": [], "order": [], "audio": audio});
    let c = Designer {
        reply: reply("library: true\ngain: 0\nmusic: [{file: 'make:music/pad?dur=12', from: start, gain: -12}]\nambience: {gate: 'make:air/wind'}"),
        asked: RefCell::new(vec![]),
    };
    let rev = ai::revise_at(&f.ws.defn, "曲をもっと穏やかに", None, &pointer, &c).unwrap();
    let req = c.asked.borrow()[0].clone();
    assert!(!req.contains("x:drone") && req.contains("make:music/pad") && req.contains("make:air/wind"), "{req}");
    let audio = rev.audio.clone().unwrap();
    assert_eq!(audio["library"], json!(false));
    assert!(audio["music"][0]["file"].as_str().unwrap().starts_with("audio/made/music-pad-d12-s"), "{audio:?}");
    assert!(audio["ambience"]["gate"].as_str().unwrap().starts_with("audio/made/air-wind-s"), "{audio:?}");
    for bad in ["music: [{file: 'lib:x:drone', from: start}]", "music: [{file: 'make:impact/deep', from: start}]",
                "ambience: {gate: 'make:music/pad'}"] {
        let c = Designer { reply: reply(&format!("library: false\ngain: 0\n{bad}")), asked: RefCell::new(vec![]) };
        let e = ai::revise_at(&f.ws.defn, "曲を替えて", None, &pointer, &c).unwrap_err();
        assert!(e.msg.contains("nothing applied"), "{bad}: {}", e.msg);
    }
    f.ws.edit_definition_with(&rev.edits, None, rev.audio.as_ref(), "revise", "n").unwrap();
    assert_eq!(launches(&f.ws).len(), before, "made music never renders a shot");

    // a teaser that picks from the library is offered no made music, and cannot slip one in
    let mut g = ws();
    let c = Designer { reply: reply("gain: 0\nmusic: [{file: 'make:music/pad', from: start}]"), asked: RefCell::new(vec![]) };
    let e = ai::revise_at(&g.ws.defn, "曲を替えて", None, &pointer, &c).unwrap_err();
    assert!(e.msg.contains("make:music/pad"), "{}", e.msg);
    assert!(!c.asked.borrow()[0].contains("make:music/"), "no made music listed");
    let d = Designer {
        reply: json!({"rationale": "r", "hits": [], "missing": [],
            "music": [{"lib": "make:music/pad", "from": "start", "until": "end", "at": 0, "gain": -12, "fade_in": 0, "fade_out": 0, "label": "m"}],
            "ambience": [{"place": "gate", "lib": "make:air/breeze", "gain": -10, "label": "a"}]}),
        asked: RefCell::new(vec![]),
    };
    let design = shotpilot::sounddesign::design(&g.ws, &lib, &d).unwrap();
    assert!(!d.asked.borrow()[0].contains("make:music/"));
    assert!(design.music.is_empty() && design.ambience.is_empty(), "{:?}", design.notes);
    shotpilot::sounddesign::apply(&mut g.ws, &design).unwrap();
    assert!(!g.ws.root.join(forge::MADE_DIR).exists());
}
