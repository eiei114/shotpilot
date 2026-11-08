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
