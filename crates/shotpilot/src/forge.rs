//! Sounds made in code (D-103). When the sound library (D-26) has nothing that fits an effect,
//! the sound designer may name one of the effects shotpilot synthesizes itself: hits, sub drops,
//! risers, reverse swells, whooshes, knocks, shutters, drips, mallets, a glitch. They are built
//! from physical and designed synthesis in Rust (forge/dsp.rs, forge/recipes.rs), with no
//! recordings, no models, no network and no outside program. Parts of the designs are adapted
//! from MIT-licensed code; the notice is in THIRD_PARTY_NOTICES.md.
//!
//! Claude names a sound as `make:<id>`, with the knobs its line offers after `?`
//! (`make:riser/tension?dur=2.4`, `make:impact/deep?pitch=C1`). Code checks the id and drops
//! knobs the sound cannot take. The take is written into the project's `audio/made/` and
//! shots.yaml names that file. A made sound is placed by its moment: `at` says where its hit, or
//! the top of a riser, swell or whoosh, lands, and the file starts that much earlier.
//!
//! Each project renders its own take: the seed comes from the project, so two games that pick
//! the same sound do not share a take (D-64). Rendering is deterministic, so one definition
//! always gives one file, and it is rendered once.
//!
//! Next to the effects, the forge makes music and the air of places (`music/<id>`, `air/<id>`,
//! forge/beds.rs, D-106). The designer is offered those only in a teaser that takes no sound from
//! the library (`audio.library: false`, the Three.js sample's default): there every sound comes
//! from here.

mod beds;
mod dsp;
mod recipes;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Map, Value};

use crate::bail;
use crate::error::Result;
use recipes::{Align, Spec};

/// Where made sounds land inside a project.
pub const MADE_DIR: &str = "audio/made";
/// How a sound made in code is named in place of a file.
pub const PREFIX: &str = "make:";
/// The version of the recipes: a take made by another version is still used (files are never
/// rewritten), the index says which version made it.
pub const VERSION: &str = "shotpilot-forge-1";
/// What each made file was made from (id, knobs, seed, its moment), next to the files.
const INDEX: &str = "index.json";
/// Lengths a `dur` knob may ask for, in seconds.
const DUR_RANGE: (f64, f64) = (0.05, 30.0);
/// Renders that run at once.
const PARALLEL: usize = 4;

/// One sound shotpilot can make.
#[derive(Debug, Clone)]
pub struct Made {
    /// `<category>/<member>`, e.g. `impact/deep`.
    pub id: String,
    /// The cue kinds it serves (hit, riser, whoosh.long, ...).
    pub kinds: Vec<String>,
    /// Its own length in seconds (about).
    pub dur_s: f64,
    /// It takes a `dur` knob (every sound does: builds and passes are shaped to it, hits trimmed).
    pub dur_knob: bool,
    /// The pitches a `pitch` knob may ask for, in Hz, and its own pitch as a note.
    pub pitch: Option<(f64, f64, String)>,
    /// What it is for, in a line.
    pub note: String,
}

impl Made {
    fn haystack(&self) -> String {
        format!("{} {} {}", self.id.replace(['/', '_'], " "), self.kinds.join(" "), self.note).to_lowercase()
    }

    /// The start of its file names in `audio/made/` (`impact/deep` -> `impact-deep`).
    fn stem(&self) -> String {
        safe_name(&self.id.replace('/', "-"))
    }
}

/// The knobs a made sound was asked for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Knobs {
    pub dur: Option<f64>,
    pub pitch: Option<String>,
}

/// A made sound placed in a project.
#[derive(Debug, Clone)]
pub struct Placed {
    /// The project path for shots.yaml (`audio/made/<file>.wav`).
    pub file: String,
    /// The id it was made from.
    pub id: String,
    /// Seconds from the file's start to its moment.
    pub moment_s: f64,
    /// Knobs that were dropped, for the human.
    pub notes: Vec<String>,
}

/// The sounds shotpilot can make.
pub struct Bank {
    pub version: String,
    /// The effects (D-103).
    pub sounds: Vec<Made>,
    specs: Vec<Spec>,
    /// Music and the air of places (D-106), for a teaser that takes no library sound.
    pub beds: Vec<Made>,
    bed_specs: Vec<beds::Bed>,
}

/// What a `make:` reference is for, by its id: music, the air of a place, or an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    Music,
    Air,
    Effect,
}

/// What `reference` (`make:<id>[?..]`, or a bare id) is for.
pub fn use_of(reference: &str) -> Use {
    let id = reference.trim();
    let id = id.strip_prefix(PREFIX).unwrap_or(id).trim_start();
    if id.starts_with(beds::MUSIC) {
        Use::Music
    } else if id.starts_with(beds::AIR) {
        Use::Air
    } else {
        Use::Effect
    }
}

impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bank({}, {} sounds, {} beds)", self.version, self.sounds.len(), self.beds.len())
    }
}

/// The catalogue, built once per process.
pub fn bank() -> Arc<Bank> {
    static BANK: OnceLock<Arc<Bank>> = OnceLock::new();
    BANK.get_or_init(|| Arc::new(Bank::new())).clone()
}

/// The note nearest a pitch (`A#1`), for the list.
fn note_name(hz: f64) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32;
    format!("{}{}", NAMES[midi.rem_euclid(12) as usize], midi.div_euclid(12) - 1)
}

impl Bank {
    fn new() -> Bank {
        let specs = recipes::catalogue();
        let sounds = specs
            .iter()
            .map(|s| Made {
                id: s.id.to_string(),
                kinds: s.kinds.iter().map(|k| k.to_string()).collect(),
                dur_s: s.length(),
                dur_knob: true,
                pitch: s.pitch.zip(s.native_hz()).map(|((lo, hi), own)| (lo, hi, note_name(own))),
                note: s.note.to_string(),
            })
            .collect();
        let bed_specs = beds::catalogue();
        let beds = bed_specs
            .iter()
            .map(|b| Made {
                id: b.id.to_string(),
                kinds: b.kinds.iter().map(|k| k.to_string()).collect(),
                dur_s: b.length,
                dur_knob: true,
                pitch: None,
                note: b.note.to_string(),
            })
            .collect();
        Bank { version: VERSION.to_string(), sounds, specs, beds, bed_specs }
    }

    /// An effect by its id.
    pub fn get(&self, id: &str) -> Option<&Made> {
        self.sounds.iter().find(|s| s.id == id)
    }

    /// Music or the air of a place by its id.
    pub fn bed(&self, id: &str) -> Option<&Made> {
        self.beds.iter().find(|s| s.id == id)
    }

    fn spec(&self, id: &str) -> Option<&Spec> {
        self.specs.iter().find(|s| s.id == id)
    }

    fn bed_spec(&self, id: &str) -> Option<&beds::Bed> {
        self.bed_specs.iter().find(|b| b.id == id)
    }

    /// The effect or bed `id` names, with how it is made.
    fn find(&self, id: &str) -> Option<(&Made, Src<'_>)> {
        match (self.get(id), self.spec(id)) {
            (Some(s), Some(spec)) => Some((s, Src::Effect(spec))),
            _ => self.bed(id).zip(self.bed_spec(id)).map(|(s, b)| (s, Src::Bed(b))),
        }
    }

    /// The effects where every word of `query` appears in the id, the kinds or the line about it.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&Made> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.sounds.iter().filter(|s| words.iter().all(|w| s.haystack().contains(w.as_str()))).take(limit).collect()
    }

    /// The music and air, searched the same way.
    pub fn search_beds(&self, query: &str, limit: usize) -> Vec<&Made> {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.beds.iter().filter(|s| words.iter().all(|w| s.haystack().contains(w.as_str()))).take(limit).collect()
    }

    /// Whether this sound has been made into the project (any knobs, any seed).
    pub fn made_here(&self, s: &Made, project_root: &Path) -> bool {
        let head = format!("{}-", s.stem());
        std::fs::read_dir(project_root.join(MADE_DIR))
            .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().starts_with(&head)))
            .unwrap_or(false)
    }

    /// The list Claude picks from: one line per sound, with its length, kinds and knobs.
    pub fn prompt_list(&self) -> String {
        self.sounds
            .iter()
            .map(|s| {
                let mut knobs = vec!["dur".to_string()];
                if let Some((_, _, own)) = &s.pitch {
                    knobs.push(format!("pitch (own {own})"));
                }
                format!("- {PREFIX}{} {:.1}s [{}] knobs: {}: {}", s.id, s.dur_s, s.kinds.join(", "), knobs.join(", "), s.note)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The music and air Claude may pick from in a teaser that makes every sound (D-106).
    pub fn prompt_list_beds(&self) -> String {
        self.beds
            .iter()
            .map(|s| format!("- {PREFIX}{} {:.0}s [{}] knobs: dur: {}", s.id, s.dur_s, s.kinds.join(", "), s.note))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Render one take of an effect or a bed, and where its moment is (a bed's is its start).
    fn take_src(&self, src: &Src, knobs: &Knobs, seed: u64) -> Result<(crate::sound::Pcm, f64)> {
        match src {
            Src::Effect(spec) => self.take(spec, knobs, seed),
            Src::Bed(b) => {
                let y = beds::render(b, seed, knobs.dur);
                if y.is_empty() || y.l.iter().chain(&y.r).any(|v| !v.is_finite()) || y.peak() < 1e-4 {
                    bail!("{} came out silent or broken; nothing was written", b.id);
                }
                let pcm = crate::sound::Pcm { l: y.l.iter().map(|v| *v as f32).collect(), r: y.r.iter().map(|v| *v as f32).collect() };
                Ok((pcm, 0.0))
            }
        }
    }

    /// Render one take (stereo, 48 kHz) and where its moment is.
    fn take(&self, spec: &Spec, knobs: &Knobs, seed: u64) -> Result<(crate::sound::Pcm, f64)> {
        let hz = knobs.pitch.as_deref().and_then(note_hz);
        let y = recipes::render(spec, seed, knobs.dur, hz);
        if y.is_empty() || y.l.iter().chain(&y.r).any(|v| !v.is_finite()) || y.peak() < 1e-4 {
            bail!("{} came out silent or broken; nothing was written", spec.id);
        }
        let moment = moment(&y, spec.align);
        let pcm = crate::sound::Pcm { l: y.l.iter().map(|v| *v as f32).collect(), r: y.r.iter().map(|v| *v as f32).collect() };
        Ok((pcm, moment))
    }

    /// Make `reference` (`make:<id>[?dur=..&pitch=..]`) into the project, or reuse the file
    /// this definition was made into before.
    pub fn make(&self, reference: &str, root: &Path) -> Result<Placed> {
        let (id, mut knobs, mut notes) = parse_ref(reference)?;
        let Some((s, src)) = self.find(&id) else {
            bail!("no sound '{id}' among the sounds shotpilot can make (shotpilot sound --made lists them)")
        };
        let range = match src {
            Src::Effect(_) => DUR_RANGE,
            Src::Bed(_) => beds::DUR_RANGE,
        };
        if let Some(d) = knobs.dur {
            if !(range.0..=range.1).contains(&d) {
                notes.push(format!("{id}: dur={d} left out (it keeps its own {:.1}s)", s.dur_s));
                knobs.dur = None;
            }
        }
        if let Some(p) = knobs.pitch.clone() {
            let fits = s.pitch.as_ref().zip(note_hz(&p)).is_some_and(|((lo, hi, _), hz)| hz >= *lo * 0.999 && hz <= *hi * 1.001);
            if !fits {
                notes.push(format!("{id}: pitch={p} left out (it keeps its own pitch)"));
                knobs.pitch = None;
            }
        }
        let seed = project_seed(root, &id);
        let name = file_name(s, &knobs, seed);
        let dir = root.join(MADE_DIR);
        let dest = dir.join(&name);
        let placed = |moment_s: f64, notes: Vec<String>| Placed { file: format!("{MADE_DIR}/{name}"), id: id.clone(), moment_s, notes };
        if dest.is_file() {
            if let Some(e) = read_index(&dir).get(&name) {
                return Ok(placed(e["moment_s"].as_f64().unwrap_or(0.0), notes));
            }
        }
        let (pcm, moment_s) = self.take_src(&src, &knobs, seed)?;
        // the same number the index keeps, so a reused file is placed exactly as the new one
        let moment_s = crate::geometry::round_to(moment_s, 4);
        std::fs::create_dir_all(&dir)?;
        let tmp = dir.join(format!(".{name}.partial"));
        crate::sound::write_wav16(&tmp, &pcm)?;
        std::fs::rename(&tmp, &dest)?;
        write_index(&dir, &name, json!({
            "id": id, "dur": knobs.dur, "pitch": knobs.pitch, "seed": seed,
            "moment_s": moment_s, "dur_s": crate::geometry::round_to(pcm.len() as f64 / dsp::SR, 4),
            "version": self.version,
        }))?;
        Ok(placed(moment_s, notes))
    }

    /// Make several references at once (each once), a few renders at a time.
    pub fn make_all(&self, refs: &[String], root: &Path) -> HashMap<String, std::result::Result<Placed, String>> {
        let mut uniq: Vec<String> = refs.to_vec();
        uniq.sort();
        uniq.dedup();
        let mut out = HashMap::new();
        for chunk in uniq.chunks(PARALLEL) {
            std::thread::scope(|sc| {
                let jobs: Vec<_> = chunk.iter().map(|r| (r, sc.spawn(move || self.make(r, root).map_err(|e| e.msg)))).collect();
                for (r, j) in jobs {
                    out.insert(r.clone(), j.join().unwrap_or_else(|_| Err("the render stopped".into())));
                }
            });
        }
        out
    }

    /// One take at its own length and a fixed seed, for listening before it is used (kept in
    /// the temp folder, never in a project).
    pub fn preview(&self, id: &str) -> Result<PathBuf> {
        let Some((s, src)) = self.find(id) else {
            bail!("no sound '{id}' among the sounds shotpilot can make")
        };
        let dir = std::env::temp_dir().join("shotpilot-made").join(VERSION);
        let dest = dir.join(format!("{}.wav", s.stem()));
        if dest.is_file() {
            return Ok(dest);
        }
        let (pcm, _) = self.take_src(&src, &Knobs::default(), 1)?;
        std::fs::create_dir_all(&dir)?;
        let tmp = dir.join(format!(".{}.{}.partial", s.stem(), std::process::id()));
        crate::sound::write_wav16(&tmp, &pcm)?;
        std::fs::rename(&tmp, &dest)?;
        Ok(dest)
    }
}

/// How a sound of the bank is made.
enum Src<'a> {
    Effect(&'a Spec),
    Bed(&'a beds::Bed),
}

/// Seconds from the start to where a take is placed from: its onset (24 dB under its peak) or
/// its loudest 5 ms.
fn moment(y: &dsp::St, align: Align) -> f64 {
    let m: Vec<f64> = y.l.iter().zip(&y.r).map(|(a, b)| a.abs().max(b.abs())).collect();
    let top = m.iter().cloned().fold(0.0, f64::max);
    match align {
        Align::Onset => m.iter().position(|v| *v >= top * dsp::db(-24.0)).unwrap_or(0) as f64 / dsp::SR,
        Align::Peak => {
            let h = dsp::n_of(0.005);
            let frames = (m.len() / h).max(1);
            let loud = (0..frames)
                .map(|k| (k, m[k * h..((k + 1) * h).min(m.len())].iter().map(|v| v * v).sum::<f64>()))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map_or(0, |(k, _)| k);
            ((loud * h) as f64 + h as f64 / 2.0).min(m.len() as f64) / dsp::SR
        }
    }
}

/// `make:<id>[?dur=<s>&pitch=<note>]` -> the id, the knobs, and notes about what was not
/// understood.
pub fn parse_ref(reference: &str) -> Result<(String, Knobs, Vec<String>)> {
    let Some(rest) = reference.trim().strip_prefix(PREFIX) else { bail!("'{reference}' is not a sound made in code ({PREFIX}<id>)") };
    let (id, query) = rest.split_once('?').unwrap_or((rest, ""));
    let id = id.trim().to_string();
    if id.is_empty() {
        bail!("'{reference}': name the sound ({PREFIX}<id>)");
    }
    let mut knobs = Knobs::default();
    let mut notes = Vec::new();
    for kv in query.split('&').map(str::trim).filter(|s| !s.is_empty()) {
        match kv.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("dur", v)) => match v.trim_end_matches('s').parse::<f64>() {
                Ok(d) if d.is_finite() => knobs.dur = Some(d),
                _ => notes.push(format!("{id}: dur={v} is not a length in seconds")),
            },
            Some(("pitch", v)) if !v.is_empty() => knobs.pitch = Some(v.to_string()),
            _ => notes.push(format!("{id}: knob `{kv}` left out (dur and pitch only)")),
        }
    }
    Ok((id, knobs, notes))
}

/// A note name (`C2`, `F#3`, `Eb6`, `Cs5`) or a number of Hz, in Hz.
fn note_hz(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Ok(hz) = s.trim_end_matches("hz").trim_end_matches("Hz").trim().parse::<f64>() {
        return (hz.is_finite() && hz > 0.0).then_some(hz);
    }
    let mut chars = s.chars();
    let base = match chars.next()?.to_ascii_uppercase() {
        'C' => 0, 'D' => 2, 'E' => 4, 'F' => 5, 'G' => 7, 'A' => 9, 'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (shift, octave) = match rest.chars().next() {
        Some('#') | Some('s') => (1, &rest[1..]),
        Some('b') => (-1, &rest[1..]),
        _ => (0, rest.as_str()),
    };
    let octave: i32 = octave.parse().ok()?;
    let midi = 12 * (octave + 1) + base + shift;
    Some(440.0 * 2f64.powf((midi as f64 - 69.0) / 12.0))
}

/// A seed of this project's own, so another game's teaser does not get the same take.
fn project_seed(root: &Path, id: &str) -> u64 {
    let name = root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .or_else(|| root.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("{name}/{id}").bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h % 9973 + 1
}

fn safe_name(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' }).collect()
}

/// `impact-deep-d1.6-pC2-s417.wav`: the sound, its knobs and the project's seed.
fn file_name(s: &Made, knobs: &Knobs, seed: u64) -> String {
    let mut name = s.stem();
    if let Some(d) = knobs.dur {
        name.push_str(&format!("-d{}", crate::model::fmt_g(crate::geometry::round_to(d, 3))));
    }
    if let Some(p) = &knobs.pitch {
        name.push_str(&format!("-p{}", safe_name(&p.replace('#', "s"))));
    }
    format!("{name}-s{seed}.wav")
}

/// The index of `audio/made/` is read and written by one thread at a time.
fn index_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

fn read_index(dir: &Path) -> Map<String, Value> {
    let _l = index_lock();
    std::fs::read_to_string(dir.join(INDEX))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_index(dir: &Path, name: &str, entry: Value) -> Result<()> {
    let _l = index_lock();
    let path = dir.join(INDEX);
    let mut all: Map<String, Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    all.insert(name.to_string(), entry);
    std::fs::write(&path, serde_json::to_string_pretty(&Value::Object(all)).unwrap_or_default())?;
    Ok(())
}

/// How the designer is told about the sounds made in code (the one-go design and a sound note).
/// `all_made`: the teaser takes no library sound, so its music and air are made here too (D-106).
pub fn guide(all_made: bool) -> String {
    let lead = if all_made {
        "Sounds made in code (synthesized by shotpilot, no recordings). This teaser takes no sound from the \
         library: its music, the air of its places and its effects are all made here, each rendered for this \
         teaser alone, so no other teaser has the same take. Music: name a `make:music/<id>` in a music cue, \
         with `dur=<seconds>` as long as the cue must run (it plays once and ends on a last chord). The air of \
         a place: name a `make:air/<id>` for that place (it loops under the place's shots). Effects: name one \
         from the effect list on a shot or a line."
    } else {
        "Sounds made in code (synthesized by shotpilot, no recordings). Each is rendered for this teaser \
         alone, so no other teaser has the same take. Take one for an effect when no library sound fits the \
         moment or the look; music and the air of a place still come from the library."
    };
    format!(
        "{lead} Name it as `{PREFIX}<id>`; add knobs after `?` joined by `&`: `dur=<seconds>` sets its length \
         (a riser that must fit before the cut, a hit whose tail is too long), `pitch=<note>` tunes an effect \
         where its line lists a pitch (C2, F#3) to sit with the music. For a made effect, `at` is where its \
         moment lands: the hit, or the top of a riser, swell or whoosh. The code starts the file that much \
         earlier, at most {}s before its piece, so give a long riser a shorter `dur`.",
        crate::model::fmt_g(crate::sound::LEAD_MAX)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_and_notes_parse() {
        let (id, k, notes) = parse_ref("make:riser/tension?dur=2.4&pitch=C#2").unwrap();
        assert_eq!(id, "riser/tension");
        assert_eq!(k, Knobs { dur: Some(2.4), pitch: Some("C#2".into()) });
        assert!(notes.is_empty());
        let (_, k, notes) = parse_ref("make:impact/deep?apex=1&dur=abc").unwrap();
        assert_eq!(k, Knobs::default());
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(parse_ref("lib:x").is_err() && parse_ref("make:").is_err());
    }

    #[test]
    fn notes_are_pitches() {
        let near = |a: f64, b: f64| (a - b).abs() < 0.01;
        assert!(near(note_hz("A4").unwrap(), 440.0));
        assert!(near(note_hz("C2").unwrap(), 65.406));
        assert!(near(note_hz("F#3").unwrap(), note_hz("Fs3").unwrap()));
        assert!(near(note_hz("Eb6").unwrap(), 1244.51));
        assert!(near(note_hz("110").unwrap(), 110.0));
        assert!(note_hz("H2").is_none() && note_hz("C").is_none());
        assert_eq!(note_name(440.0), "A4");
        assert_eq!(note_name(41.2), "E1");
    }

    #[test]
    fn every_sound_in_the_catalogue_renders_clean_at_its_length_and_its_knobs() {
        // each recipe makes a finite, audible, -1 dBFS take near its listed length; `dur` sets the
        // length, `pitch` moves the sound, and the same seed gives the same samples
        let bank = bank();
        assert!(bank.sounds.len() >= 30);
        for (s, spec) in bank.sounds.iter().zip(&bank.specs) {
            let (pcm, moment) = bank.take(spec, &Knobs::default(), 7).unwrap();
            let len = pcm.len() as f64 / dsp::SR;
            assert!(len > 0.05 && len <= s.dur_s * 1.6 + 0.4, "{}: {len:.2}s, listed {:.2}s", s.id, s.dur_s);
            let peak = pcm.l.iter().chain(&pcm.r).fold(0.0f32, |m, v| m.max(v.abs()));
            assert!((0.85..=0.9).contains(&peak), "{}: peak {peak}", s.id);
            assert!(moment < len, "{}: moment {moment:.2}s past its end", s.id);
            if spec.align == Align::Peak && s.kinds.contains(&"riser".to_string()) {
                assert!(moment > 0.75 * len, "{}: a riser peaks at its end, not at {moment:.2}s of {len:.2}s", s.id);
            }
            let (short, _) = bank.take(spec, &Knobs { dur: Some(0.5), pitch: None }, 7).unwrap();
            assert!((short.len() as f64 / dsp::SR - 0.5).abs() < 0.12 || spec.shaped_by_length(), "{}: dur 0.5", s.id);
        }
        let spec = bank.spec("impact/classic").unwrap();
        let a = bank.take(spec, &Knobs::default(), 7).unwrap().0;
        assert_eq!(a.l, bank.take(spec, &Knobs::default(), 7).unwrap().0.l);
        assert_ne!(a.l, bank.take(spec, &Knobs::default(), 8).unwrap().0.l);
        assert_ne!(a.l, bank.take(spec, &Knobs { dur: None, pitch: Some("C1".into()) }, 7).unwrap().0.l);
        let riser = bank.spec("riser/tension").unwrap();
        let (two, m) = bank.take(riser, &Knobs { dur: Some(2.0), pitch: None }, 7).unwrap();
        assert!((two.len() as f64 / dsp::SR - 2.0).abs() < 0.01 && m > 1.8, "a 2 s riser tops out at its end");
    }
}
