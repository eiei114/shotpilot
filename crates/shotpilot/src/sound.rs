//! BGM and sound effects: what shots.yaml says to play, and the mix of it.
//!
//! ```yaml
//! audio:
//!   gain: 0                    # master, dB
//!   bgm: {file: audio/bgm.wav, start: 0, offset: 0, gain: -6, fade_in: 0, fade_out: 1.5}
//! shots:
//!   - id: s03_window
//!     sfx:
//!       - {file: audio/hit.wav, at: 0, gain: -3, label: hit}   # at: seconds from the shot's start
//! ```
//!
//! Sound never changes pixels, so none of it is in a render or build fingerprint:
//! editing a sound effect re-mixes the teaser and never re-renders a shot. A sound
//! effect belongs to its shot, so it moves with the shot when the cut is reordered.
//! The mix is cached by a digest of the placements and the source files' size and mtime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Map, Value};

use crate::bail;
use crate::cut::ffmpeg;
use crate::error::{Error, Result};
use crate::fingerprint as fp;
use crate::model::{fmt_g, num, str_of, Definition, ResolvedShot};
use crate::soundcheck::{self, Cut, CHECK_VERSION};

/// Sample rate of every mix and of everything decoded for it.
pub const SR: u32 = 48_000;
/// Bump when the mix of the same placements changes.
pub const MIX_VERSION: i64 = 1;
pub const AUDIO_EXTS: &[&str] = &["wav", "mp3", "ogg", "flac", "m4a", "aac", "opus", "aif", "aiff"];
/// A sound effect may start this many seconds before its shot (a whoosh into the cut).
pub const LEAD_MAX: f64 = 3.0;
/// Peaks per second in the lanes the app draws.
pub const PEAKS_PER_S: f64 = 50.0;

const AUDIO_KEYS: &[&str] = &["gain", "bgm", "music", "loudness"];
const MUSIC_KEYS: &[&str] = &["file", "from", "until", "at", "offset", "gain", "fade_in", "fade_out", "label"];
/// Integrated loudness the cut is brought to when it carries sound (LUFS).
pub const DEFAULT_LOUDNESS: f64 = -16.0;
const BGM_KEYS: &[&str] = &["file", "start", "offset", "gain", "fade_in", "fade_out"];
const SFX_KEYS: &[&str] = &["file", "at", "gain", "label"];

// --------------------------------------------------------------------------- definition

/// A file path as written in shots.yaml: relative to the project, inside it, a known audio type.
pub fn valid_file(s: &str) -> std::result::Result<(), String> {
    let p = s.replace('\\', "/");
    if p.trim().is_empty() {
        return Err("a file path is required (e.g. audio/hit.wav)".into());
    }
    if p.starts_with('/') || p.contains(':') {
        return Err(format!("'{s}' must be relative to the folder of shots.yaml (e.g. audio/hit.wav)"));
    }
    if p.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return Err(format!("'{s}' must stay inside the project folder"));
    }
    let ext = Path::new(&p).extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if !AUDIO_EXTS.contains(&ext.as_str()) {
        return Err(format!("'{s}': audio files are {}", AUDIO_EXTS.join(" / ")));
    }
    Ok(())
}

fn check_keys(m: &Map<String, Value>, allowed: &[&str], at: &str, errs: &mut Vec<String>) {
    for k in m.keys() {
        if !allowed.contains(&k.as_str()) {
            errs.push(format!("{at}.{k}: unknown key (known: {})", allowed.join(", ")));
        }
    }
}

fn check_num(m: &Map<String, Value>, key: &str, lo: f64, hi: f64, at: &str, errs: &mut Vec<String>) {
    match m.get(key) {
        None | Some(Value::Null) => {}
        Some(v) => match num(v) {
            Some(x) if (lo..=hi).contains(&x) => {}
            _ => errs.push(format!("{at}.{key}: a number in {}..{}", fmt_g(lo), fmt_g(hi))),
        },
    }
}

fn check_file(m: &Map<String, Value>, at: &str, errs: &mut Vec<String>) {
    match m.get("file") {
        Some(Value::String(f)) => {
            if let Err(e) = valid_file(f) {
                errs.push(format!("{at}.file: {e}"));
            }
        }
        _ => errs.push(format!("{at}.file: required (e.g. audio/hit.wav)")),
    }
}

/// Problems with `audio:` and every shot's `sfx:` (appended to `errs`).
pub fn validate(top: &Map<String, Value>, shots: &[Value], errs: &mut Vec<String>) {
    match top.get("audio") {
        None | Some(Value::Null) => {}
        Some(Value::Object(a)) => {
            check_keys(a, AUDIO_KEYS, "audio", errs);
            check_num(a, "gain", -30.0, 12.0, "audio", errs);
            check_num(a, "loudness", -30.0, -8.0, "audio", errs);
            match a.get("music") {
                None | Some(Value::Null) => {}
                Some(Value::Array(items)) => {
                    for (k, it) in items.iter().enumerate() {
                        let at = format!("audio.music[{k}]");
                        let Some(m) = it.as_object() else {
                            errs.push(format!("{at}: a mapping like {{file: audio/cue.wav, from: s02, until: title}}"));
                            continue;
                        };
                        check_keys(m, MUSIC_KEYS, &at, errs);
                        check_file(m, &at, errs);
                        if m.get("from").map_or(true, |f| str_of(f).is_empty()) {
                            errs.push(format!("{at}.from: the shot or script line the cue starts on (or start)"));
                        }
                        check_num(m, "at", -30.0, 600.0, &at, errs);
                        check_num(m, "offset", 0.0, 36000.0, &at, errs);
                        check_num(m, "gain", -60.0, 12.0, &at, errs);
                        check_num(m, "fade_in", 0.0, 30.0, &at, errs);
                        check_num(m, "fade_out", 0.0, 30.0, &at, errs);
                    }
                }
                Some(_) => errs.push("audio.music: a list of cues {file, from, until, at, gain, fade_in, fade_out}".into()),
            }
            match a.get("bgm") {
                None | Some(Value::Null) => {}
                Some(Value::Object(b)) => {
                    check_keys(b, BGM_KEYS, "audio.bgm", errs);
                    check_file(b, "audio.bgm", errs);
                    check_num(b, "start", 0.0, 3600.0, "audio.bgm", errs);
                    check_num(b, "offset", 0.0, 36000.0, "audio.bgm", errs);
                    check_num(b, "gain", -60.0, 12.0, "audio.bgm", errs);
                    check_num(b, "fade_in", 0.0, 30.0, "audio.bgm", errs);
                    check_num(b, "fade_out", 0.0, 30.0, "audio.bgm", errs);
                }
                Some(_) => errs.push("audio.bgm: a mapping like {file: audio/bgm.wav, gain: -6}".into()),
            }
        }
        Some(_) => errs.push("audio: a mapping (bgm, music, gain, loudness)".into()),
    }
    for (i, s) in shots.iter().enumerate() {
        let at = format!("shots[{i}]({}).sfx", str_of(&s["id"]));
        let dur = s.get("duration").and_then(num).unwrap_or(0.0);
        match s.get("sfx") {
            None | Some(Value::Null) => {}
            Some(Value::Array(items)) => {
                for (k, it) in items.iter().enumerate() {
                    let at = format!("{at}[{k}]");
                    let Some(m) = it.as_object() else {
                        errs.push(format!("{at}: a mapping like {{file: audio/hit.wav, at: 0}}"));
                        continue;
                    };
                    check_keys(m, SFX_KEYS, &at, errs);
                    check_file(m, &at, errs);
                    match m.get("at").and_then(num) {
                        Some(t) if t >= -LEAD_MAX && t < dur.max(0.0) => {}
                        _ => errs.push(format!(
                            "{at}.at: seconds from the shot's start, -{}..{} (negative = leads into the cut)",
                            fmt_g(LEAD_MAX), fmt_g(dur)
                        )),
                    }
                    check_num(m, "gain", -60.0, 12.0, &at, errs);
                    if m.get("label").is_some_and(|l| !l.is_string() && !l.is_null()) {
                        errs.push(format!("{at}.label: text"));
                    }
                }
            }
            Some(_) => errs.push(format!("{at}: a list of {{file, at, gain, label}}")),
        }
    }
}

/// Sound effects on a script line (`sfx` on a card, a caption...): `at` from the line's start.
pub fn validate_line_sfx(at: &str, v: Option<&Value>, errs: &mut Vec<String>) {
    match v {
        None | Some(Value::Null) => {}
        Some(Value::Array(items)) => {
            for (k, it) in items.iter().enumerate() {
                let at = format!("{at}.sfx[{k}]");
                let Some(m) = it.as_object() else {
                    errs.push(format!("{at}: a mapping like {{file: audio/hit.wav, at: 0}}"));
                    continue;
                };
                check_keys(m, SFX_KEYS, &at, errs);
                check_file(m, &at, errs);
                check_num(m, "at", -LEAD_MAX, 600.0, &at, errs);
                check_num(m, "gain", -60.0, 12.0, &at, errs);
            }
        }
        Some(_) => errs.push(format!("{at}.sfx: a list of {{file, at, gain, label}}")),
    }
}

/// Where each shot, script slot and overlay sits in the cut: id -> (start, end) seconds.
pub fn spans(defn: &Definition, shots: &[ResolvedShot]) -> (HashMap<String, (f64, f64)>, f64) {
    let fps = defn.teaser.fps.max(1) as f64;
    let items = crate::script::layout(&defn.script, shots, defn.teaser.fps.max(1));
    let mut m = HashMap::new();
    for it in &items {
        m.insert(it.id.clone(), (it.start as f64 / fps, (it.start + it.frames) as f64 / fps));
    }
    for l in defn.script.iter().filter(|l| !l.is_slot()) {
        if let Some(&(s, e)) = l.on.as_ref().and_then(|on| m.get(on)) {
            let a = (s + l.at).min(e);
            m.insert(l.id.clone(), (a, (a + l.dur).min(e)));
        }
    }
    let total = items.last().map(|it| (it.start + it.frames) as f64 / fps).unwrap_or(0.0);
    (m, total)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Bgm,
    Sfx,
}

/// One sound placed on the teaser's timeline.
#[derive(Debug, Clone)]
pub struct Placement {
    pub role: Role,
    /// Owning shot (sound effects only) and the effect's index in that shot's list.
    pub shot: Option<String>,
    pub index: usize,
    pub label: String,
    /// As written in shots.yaml (forward slashes), relative to the project root.
    pub file: String,
    /// Teaser seconds where it starts (an effect leading into the first shot can be < 0).
    pub start: f64,
    /// Seconds into the file.
    pub offset: f64,
    /// Teaser seconds where it must stop (the BGM stops with the teaser), None = natural end.
    pub until: Option<f64>,
    pub gain_db: f64,
    pub fade_in: f64,
    pub fade_out: f64,
}

impl Placement {
    pub fn to_json(&self) -> Value {
        json!({
            "role": if self.role == Role::Bgm { "bgm" } else { "sfx" },
            "shot": self.shot, "index": self.index, "label": self.label, "file": self.file,
            "start": r3(self.start), "offset": r3(self.offset), "gain": self.gain_db,
            "fade_in": self.fade_in, "fade_out": self.fade_out,
        })
    }
}

fn r3(v: f64) -> f64 {
    crate::geometry::round_to(v, 3)
}

/// The cut's timing: each shot's start in teaser seconds (frames at the teaser fps, as
/// the video is assembled), and the total length.
/// Where each shot starts in the cut (seconds) and the cut's length, cards between shots
/// included (script.rs), so sound stays on its shot when cards are added or moved.
pub fn timing(defn: &Definition, shots: &[ResolvedShot]) -> (Vec<f64>, f64) {
    let fps = defn.teaser.fps.max(1);
    let items = crate::script::layout(&defn.script, shots, fps);
    let mut starts = vec![0.0; shots.len()];
    for it in &items {
        if let crate::script::Piece::Shot(k) = it.piece {
            starts[k] = it.start as f64 / fps as f64;
        }
    }
    let total = items.last().map(|it| it.start + it.frames).unwrap_or(0);
    (starts, total as f64 / fps as f64)
}

/// A music cue whose `from` or `until` names a piece that is not in the cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Orphan {
    /// Index in `audio.music`.
    pub cue: usize,
    pub label: String,
    /// `from` or `until`.
    pub key: &'static str,
    pub target: String,
}

impl Orphan {
    pub fn to_json(&self) -> Value {
        json!({"cue": self.cue, "label": self.label, "key": self.key, "target": self.target})
    }

    pub fn text(&self) -> String {
        let what = if self.key == "from" { "is never played" } else { "plays to the end" };
        format!("audio.music[{}] ({}): {} '{}' is not a shot or script line, so the cue {what}",
                self.cue, self.label, self.key, self.target)
    }
}

/// Music cues pointing at pieces that are gone (a shot renamed by a re-plan, a removed
/// card). Parsing allows them, and [`plan`] skips or stretches them, so say so.
pub fn orphans(defn: &Definition, shots: &[ResolvedShot]) -> Vec<Orphan> {
    let (spans, _) = spans(defn, shots);
    let mut out = Vec::new();
    let Some(Value::Array(cues)) = defn.audio.get("music") else { return out };
    for (k, c) in cues.iter().enumerate() {
        for key in ["from", "until"] {
            let t = c.get(key).map(str_of).unwrap_or_default();
            if t.is_empty() || t == "start" || t == "end" || spans.contains_key(&t) {
                continue;
            }
            let label = c.get("label").map(str_of).filter(|l| !l.is_empty())
                .unwrap_or_else(|| c.get("file").map(str_of).unwrap_or_default());
            out.push(Orphan { cue: k, label, key, target: t });
        }
    }
    out
}

pub fn master_gain(defn: &Definition) -> f64 {
    defn.audio.get("gain").and_then(num).unwrap_or(0.0)
}

/// Everything the definition says to play, BGM first, then effects in cut order.
pub fn plan(defn: &Definition, shots: &[ResolvedShot]) -> Vec<Placement> {
    let (starts, total) = timing(defn, shots);
    let mut out = Vec::new();
    if let Some(Value::Object(b)) = defn.audio.get("bgm") {
        let g = |k: &str, d: f64| b.get(k).and_then(num).unwrap_or(d);
        out.push(Placement {
            role: Role::Bgm,
            shot: None,
            index: 0,
            label: "BGM".into(),
            file: b.get("file").map(str_of).unwrap_or_default().replace('\\', "/"),
            start: g("start", 0.0),
            offset: g("offset", 0.0),
            until: Some(total),
            gain_db: g("gain", 0.0),
            fade_in: g("fade_in", 0.0),
            fade_out: g("fade_out", 0.0),
        });
    }
    // music cues: each from a piece of the cut until another (or the end)
    let (spans, _) = spans(defn, shots);
    if let Some(Value::Array(cues)) = defn.audio.get("music") {
        for (k, c) in cues.iter().enumerate() {
            let g = |key: &str, d: f64| c.get(key).and_then(num).unwrap_or(d);
            let from = c.get("from").map(str_of).unwrap_or_default();
            let Some(&(s0, _)) = (if from == "start" { Some(&(0.0, 0.0)) } else { spans.get(&from) }) else { continue };
            let until = c.get("until").map(str_of).unwrap_or_default();
            let end = if until.is_empty() || until == "end" { total } else { spans.get(&until).map(|v| v.1).unwrap_or(total) };
            let file = c.get("file").map(str_of).unwrap_or_default().replace('\\', "/");
            let label = c.get("label").map(str_of).filter(|l| !l.is_empty()).unwrap_or_else(|| {
                Path::new(&file).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            });
            let start = (s0 + g("at", 0.0)).max(0.0);
            out.push(Placement {
                role: Role::Bgm,
                shot: None,
                index: k + 1,
                label,
                file,
                start,
                offset: g("offset", 0.0),
                until: Some(end.max(start)),
                gain_db: g("gain", 0.0),
                fade_in: g("fade_in", 0.0),
                fade_out: g("fade_out", 0.0),
            });
        }
    }
    // effects on script lines (a hit on a card, a stinger before the black)
    for l in &defn.script {
        let Some(&(s0, _)) = spans.get(&l.id) else { continue };
        for (k, it) in l.sfx.iter().enumerate() {
            let file = it.get("file").map(str_of).unwrap_or_default().replace('\\', "/");
            let label = it.get("label").map(str_of).filter(|x| !x.is_empty()).unwrap_or_else(|| {
                Path::new(&file).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            });
            out.push(Placement {
                role: Role::Sfx,
                shot: Some(l.id.clone()),
                index: k,
                label,
                file,
                start: s0 + it.get("at").and_then(num).unwrap_or(0.0),
                offset: 0.0,
                until: None,
                gain_db: it.get("gain").and_then(num).unwrap_or(0.0),
                fade_in: 0.0,
                fade_out: 0.0,
            });
        }
    }
    for (s, start) in shots.iter().zip(&starts) {
        let Some(Value::Array(items)) = defn.shots[s.index].get("sfx") else { continue };
        for (k, it) in items.iter().enumerate() {
            let file = it.get("file").map(str_of).unwrap_or_default().replace('\\', "/");
            let label = it.get("label").map(str_of).filter(|l| !l.is_empty()).unwrap_or_else(|| {
                Path::new(&file).file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
            });
            out.push(Placement {
                role: Role::Sfx,
                shot: Some(s.id.clone()),
                index: k,
                label,
                file,
                start: start + it.get("at").and_then(num).unwrap_or(0.0),
                offset: 0.0,
                until: None,
                gain_db: it.get("gain").and_then(num).unwrap_or(0.0),
                fade_in: 0.0,
                fade_out: 0.0,
            });
        }
    }
    out
}

/// Audio files under the project's audio/ folder (relative paths, forward slashes).
pub fn list_files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>, depth: usize) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 4 {
                walk(&p, root, out, depth + 1);
            } else if p.extension().is_some_and(|x| AUDIO_EXTS.contains(&x.to_string_lossy().to_ascii_lowercase().as_str())) {
                if let Ok(rel) = p.strip_prefix(root) {
                    let parts: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().to_string()).collect();
                    out.push(parts.join("/"));
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("audio"), root, &mut out, 0);
    out.sort();
    out
}

// --------------------------------------------------------------------------- PCM

/// Stereo samples at [`SR`].
#[derive(Debug, Clone, Default)]
pub struct Pcm {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Pcm {
    pub fn silent(n: usize) -> Pcm {
        Pcm { l: vec![0.0; n], r: vec![0.0; n] }
    }

    pub fn len(&self) -> usize {
        self.l.len()
    }

    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }

    pub fn seconds(&self) -> f64 {
        self.len() as f64 / SR as f64
    }
}

fn u16le(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}

fn u32le(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// PCM (8/16/24/32-bit) or IEEE float (32/64-bit) WAV, any channel count: (rate, channels).
pub fn read_wav(bytes: &[u8], origin: &str) -> Result<(u32, Vec<Vec<f32>>)> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("{origin}: not a WAV file");
    }
    let mut i = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let size = u32le(bytes, i + 4) as usize;
        let body = i + 8;
        let end = body.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end - body >= 16 {
            let mut tag = u16le(bytes, body);
            if tag == 0xFFFE && end - body >= 26 {
                tag = u16le(bytes, body + 24); // WAVE_FORMAT_EXTENSIBLE: the sub-format GUID starts with the tag
            }
            fmt = Some((tag, u16le(bytes, body + 2), u32le(bytes, body + 4), u16le(bytes, body + 14)));
        } else if id == b"data" {
            let Some((tag, ch, rate, bits)) = fmt else { bail!("{origin}: WAV data before its format") };
            if ch == 0 || rate == 0 {
                bail!("{origin}: WAV with no channels");
            }
            let data = &bytes[body..end];
            let width = (bits as usize).div_ceil(8);
            let frame = width * ch as usize;
            if frame == 0 {
                bail!("{origin}: unsupported WAV sample size");
            }
            let n = data.len() / frame;
            let mut chans = vec![Vec::with_capacity(n); ch as usize];
            for f in 0..n {
                for (c, out) in chans.iter_mut().enumerate() {
                    let o = f * frame + c * width;
                    let s = &data[o..o + width];
                    let v = match (tag, bits) {
                        (1, 8) => (s[0] as f32 - 128.0) / 128.0,
                        (1, 16) => i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0,
                        (1, 24) => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8_388_608.0,
                        (1, 32) => i32::from_le_bytes([s[0], s[1], s[2], s[3]]) as f32 / 2_147_483_648.0,
                        (3, 32) => f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                        (3, 64) => f64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]) as f32,
                        _ => bail!("{origin}: unsupported WAV encoding (format {tag}, {bits}-bit); convert it to 16-bit PCM"),
                    };
                    out.push(v);
                }
            }
            return Ok((rate, chans));
        }
        i = body + size + (size & 1); // chunks are word-aligned
    }
    bail!("{origin}: WAV without audio data")
}

/// Linear-interpolation resample. Good enough for a preview mix of sources that are not
/// already at 48 kHz; install ffmpeg for a band-limited conversion.
pub fn resample_linear(x: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || x.is_empty() {
        return x.to_vec();
    }
    let n = ((x.len() as f64) * to as f64 / from as f64).round() as usize;
    let step = from as f64 / to as f64;
    (0..n)
        .map(|i| {
            let p = i as f64 * step;
            let k = p.floor() as usize;
            let f = (p - k as f64) as f32;
            let a = x[k.min(x.len() - 1)];
            let b = x[(k + 1).min(x.len() - 1)];
            a + (b - a) * f
        })
        .collect()
}

fn to_stereo(rate: u32, mut chans: Vec<Vec<f32>>) -> Pcm {
    if chans.len() == 1 {
        chans.push(chans[0].clone());
    }
    let r = chans.swap_remove(1);
    let l = chans.swap_remove(0);
    Pcm { l: resample_linear(&l, rate, SR), r: resample_linear(&r, rate, SR) }
}

/// Decode one source to stereo at [`SR`]: WAV natively, everything else through ffmpeg.
pub fn decode(path: &Path) -> Result<Pcm> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let shown = path.display().to_string();
    if ext == "wav" {
        let bytes = std::fs::read(path).map_err(|e| Error::user(format!("cannot read {shown}: {e}")))?;
        let parsed = read_wav(&bytes, &shown);
        match parsed {
            Ok((rate, chans)) if rate == SR || ffmpeg().is_none() => return Ok(to_stereo(rate, chans)),
            Ok(_) => {} // other rates: let ffmpeg resample properly
            Err(e) if ffmpeg().is_none() => return Err(e),
            Err(_) => {} // an encoding we do not read: ffmpeg may
        }
    }
    let Some(exe) = ffmpeg() else {
        bail!("{shown}: reading .{ext} needs ffmpeg on PATH (or convert it to .wav)");
    };
    let out = crate::cut::no_window(&mut Command::new(exe))
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-vn", "-f", "f32le", "-acodec", "pcm_f32le", "-ac", "2", "-ar", &SR.to_string(), "-"])
        .stdin(Stdio::null())
        .output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("ffmpeg could not decode {shown}: {}", err.trim().chars().take(400).collect::<String>());
    }
    let n = out.stdout.len() / 8;
    let mut pcm = Pcm { l: Vec::with_capacity(n), r: Vec::with_capacity(n) };
    for f in out.stdout.chunks_exact(8) {
        pcm.l.push(f32::from_le_bytes([f[0], f[1], f[2], f[3]]));
        pcm.r.push(f32::from_le_bytes([f[4], f[5], f[6], f[7]]));
    }
    Ok(pcm)
}

/// 16-bit PCM stereo WAV (what browsers and the AAC encoder take without surprises).
pub fn write_wav16(path: &Path, pcm: &Pcm) -> Result<()> {
    let n = pcm.len();
    let data_len = (n * 4) as u32;
    let mut b = Vec::with_capacity(44 + n * 4);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&SR.to_le_bytes());
    b.extend_from_slice(&(SR * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    let q = |v: f32| ((v.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes();
    for i in 0..n {
        b.extend_from_slice(&q(pcm.l[i]));
        b.extend_from_slice(&q(pcm.r[i]));
    }
    write_atomic(path, &b)
}

/// Write through a uniquely named temp file, so two mixes racing never tear a file.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    let tid = format!("{:?}", std::thread::current().id()).chars().filter(char::is_ascii_digit).collect::<String>();
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}-{tid}-{nanos}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

// --------------------------------------------------------------------------- mix

/// Where one placement actually sounds in the mix, for the app's lanes.
#[derive(Debug, Clone)]
pub struct Lane {
    pub placement: Placement,
    /// Teaser seconds it sounds from / to (clipped to the teaser).
    pub from: f64,
    pub to: f64,
    /// Max |sample| after gain, [`PEAKS_PER_S`] per second from `from`.
    pub peaks: Vec<f32>,
}

impl Lane {
    pub fn to_json(&self) -> Value {
        let mut v = self.placement.to_json();
        v["from"] = json!(r3(self.from));
        v["to"] = json!(r3(self.to));
        v["peaks"] = json!(self.peaks.iter().map(|p| (p * 1000.0).round() / 1000.0).collect::<Vec<_>>());
        v
    }
}

fn db_to_gain(db: f64) -> f32 {
    10f64.powf(db / 20.0) as f32
}

/// Mix the placements into a teaser-long stereo buffer. `sources` maps each placement's
/// `file` to its decoded samples.
pub fn mix(placements: &[Placement], sources: &HashMap<String, Pcm>, total: f64, master_db: f64) -> (Pcm, Vec<Lane>) {
    let n = (total * SR as f64).round().max(0.0) as usize;
    let mut out = Pcm::silent(n);
    let master = db_to_gain(master_db);
    let mut lanes = Vec::new();
    for p in placements {
        let Some(src) = sources.get(&p.file) else { continue };
        let sr = SR as f64;
        let start = (p.start * sr).round() as i64;
        let off = (p.offset * sr).round().max(0.0) as i64;
        let mut len = src.len() as i64 - off;
        if let Some(u) = p.until {
            len = len.min((u * sr).round() as i64 - start);
        }
        let first = (-start).max(0); // samples before the teaser starts are not heard
        let last = len.min(n as i64 - start);
        if last <= first {
            lanes.push(Lane { placement: p.clone(), from: p.start.max(0.0), to: p.start.max(0.0), peaks: Vec::new() });
            continue;
        }
        let g = db_to_gain(p.gain_db) * master;
        let fi = (p.fade_in * sr) as i64;
        let fo = (p.fade_out * sr) as i64;
        let hop = (sr / PEAKS_PER_S) as i64;
        let mut peaks = Vec::new();
        let mut peak = 0f32;
        for k in first..last {
            let mut a = g;
            if fi > 0 && k < fi {
                a *= k as f32 / fi as f32;
            }
            if fo > 0 && k >= len - fo {
                a *= ((len - k) as f32 / fo as f32).clamp(0.0, 1.0);
            }
            let si = (off + k) as usize;
            let oi = (start + k) as usize;
            let (l, r) = (src.l[si] * a, src.r[si] * a);
            out.l[oi] += l;
            out.r[oi] += r;
            peak = peak.max(l.abs()).max(r.abs());
            if (k - first + 1) % hop == 0 {
                peaks.push(peak);
                peak = 0.0;
            }
        }
        if (last - first) % hop != 0 {
            peaks.push(peak);
        }
        lanes.push(Lane {
            placement: p.clone(),
            from: (start + first) as f64 / sr,
            to: (start + last) as f64 / sr,
            peaks,
        });
    }
    (out, lanes)
}

/// Max |sample| per window, for drawing a waveform.
pub fn peaks(pcm: &Pcm, per_s: f64) -> Vec<f32> {
    let hop = ((SR as f64 / per_s) as usize).max(1);
    (0..pcm.len().div_ceil(hop))
        .map(|w| {
            let (a, b) = (w * hop, ((w + 1) * hop).min(pcm.len()));
            (a..b).fold(0f32, |m, i| m.max(pcm.l[i].abs()).max(pcm.r[i].abs()))
        })
        .collect()
}

fn file_stamp(p: &Path) -> Value {
    match std::fs::metadata(p) {
        Ok(m) => {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_nanos()).unwrap_or(0);
            json!([m.len(), t.to_string()])
        }
        Err(_) => Value::Null,
    }
}

/// Cache key of a mix: placements, timing, master gain and each source file's size + mtime.
pub fn mix_key(defn: &Definition, root: &Path, placements: &[Placement], shots: &[ResolvedShot]) -> String {
    let (starts, total) = timing(defn, shots);
    let mut files = Map::new();
    for p in placements {
        files.insert(p.file.clone(), file_stamp(&root.join(&p.file)));
    }
    fp::digest(&json!({
        "v": MIX_VERSION, "total": r3(total), "cuts": starts.iter().map(|s| r3(*s)).collect::<Vec<_>>(),
        "master": master_gain(defn), "placements": placements.iter().map(Placement::to_json).collect::<Vec<_>>(),
        "files": files,
    }))
}

/// Decode every distinct source of the placements.
pub fn load_sources(root: &Path, placements: &[Placement]) -> Result<HashMap<String, Pcm>> {
    let mut missing: Vec<String> = Vec::new();
    for p in placements {
        if !root.join(&p.file).is_file() && !missing.contains(&p.file) {
            missing.push(p.file.clone());
        }
    }
    if !missing.is_empty() {
        bail!("audio file(s) not found: {} (put them under {}/)", missing.join(", "), root.join("audio").display());
    }
    let mut out = HashMap::new();
    for p in placements {
        if !out.contains_key(&p.file) {
            out.insert(p.file.clone(), decode(&root.join(&p.file))?);
        }
    }
    Ok(out)
}

/// Where the mix and its check live (served to the app under /files/cut/sound/).
pub fn sound_dir(root: &Path) -> PathBuf {
    root.join("cut").join("sound")
}

/// The picture cuts of the teaser (shot boundaries), for the sound check.
pub fn cuts(shots: &[ResolvedShot], starts: &[f64]) -> Vec<Cut> {
    (1..shots.len())
        .map(|i| Cut { t: starts[i], from: shots[i - 1].id.clone(), to: shots[i].id.clone(), to_beat: shots[i].beat.clone() })
        .collect()
}

/// A mix on disk: cut/sound/mix.wav plus what the app draws and the sound check.
#[derive(Debug, Clone)]
pub struct Mixed {
    pub key: String,
    pub wav: PathBuf,
    pub total: f64,
    pub lanes: Vec<Value>,
    pub check: Value,
    /// The cached mix was still valid: nothing was decoded or measured.
    pub reused: bool,
}

impl Mixed {
    pub fn to_json(&self) -> Value {
        json!({"key": self.key, "total": r3(self.total), "lanes": self.lanes, "check": self.check, "reused": self.reused})
    }
}

/// Mix what the definition says to play, reusing the last mix when nothing it depends
/// on changed. None when there is nothing to play.
pub fn mix_cached(defn: &Definition, root: &Path, shots: &[ResolvedShot]) -> Result<Option<Mixed>> {
    let placements = plan(defn, shots);
    if placements.is_empty() {
        return Ok(None);
    }
    let key = mix_key(defn, root, &placements, shots);
    let dir = sound_dir(root);
    let wav = dir.join("mix.wav");
    let meta = dir.join("mix.json");
    if let Some(m) = std::fs::read(&meta).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
        if m["key"] == key.as_str() && m["check"]["version"] == CHECK_VERSION && wav.is_file() {
            return Ok(Some(Mixed {
                key,
                wav,
                total: m["total"].as_f64().unwrap_or(0.0),
                lanes: m["lanes"].as_array().cloned().unwrap_or_default(),
                check: m["check"].clone(),
                reused: true,
            }));
        }
    }
    let sources = load_sources(root, &placements)?;
    let (starts, total) = timing(defn, shots);
    let (pcm, lanes) = mix(&placements, &sources, total, master_gain(defn));
    let check = soundcheck::check(&pcm, &cuts(shots, &starts), defn.teaser.fps.max(1) as f64);
    write_wav16(&wav, &pcm)?;
    let mixed = Mixed { key, wav, total, lanes: lanes.iter().map(Lane::to_json).collect(), check, reused: false };
    write_atomic(&meta, serde_json::to_string(&mixed.to_json())?.as_bytes())?;
    Ok(Some(mixed))
}

// --------------------------------------------------------------------------- demo sounds

/// Deterministic white noise in -1..1.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_103_515_245).wrapping_add(12345);
        ((self.0 >> 9) as f32 / (1 << 22) as f32) - 1.0
    }
}

fn demo_drone(secs: f64) -> Pcm {
    let n = (secs * SR as f64) as usize;
    let mut p = Pcm::silent(n);
    let mut noise = Noise(11);
    let mut lp = 0f32;
    let tau = std::f64::consts::TAU;
    for i in 0..n {
        let t = i as f64 / SR as f64;
        let swell = 0.75 + 0.25 * (tau * t / 7.0).sin();
        let build = (t / secs).min(1.0); // slowly rises toward the title
        let base = 0.16 * (tau * 55.0 * t).sin() + 0.08 * (tau * 82.4 * t).sin() + 0.05 * (tau * 110.3 * t).sin();
        lp += 0.02 * (noise.next() - lp); // dark air
        let air = lp as f64 * (0.6 + 0.6 * build);
        // a slow heartbeat that tightens as the teaser escalates
        let period = 1.6 - 0.6 * build;
        let ph = t % period;
        let beat = if t > 4.0 { 0.35 * (tau * 48.0 * ph).sin() * (-ph * 9.0).exp() } else { 0.0 };
        let v = (swell * (base * (0.7 + 0.5 * build) + air) + beat) as f32;
        p.l[i] = v * 0.9 + 0.1 * (tau * 55.6 * t).sin() as f32 * 0.16;
        p.r[i] = v * 0.9 + 0.1 * (tau * 54.4 * t).sin() as f32 * 0.16;
    }
    p
}

fn demo_hit() -> Pcm {
    let n = (1.6 * SR as f64) as usize;
    let mut p = Pcm::silent(n);
    let mut noise = Noise(23);
    let mut phase = 0f64;
    for i in 0..n {
        let t = i as f64 / SR as f64;
        let f = 38.0 + 90.0 * (-t * 18.0).exp(); // pitch drop of a sub hit
        phase += std::f64::consts::TAU * f / SR as f64;
        let sub = 0.9 * phase.sin() * (-t * 2.6).exp();
        let crack = noise.next() as f64 * 0.7 * (-t * 22.0).exp();
        let v = (sub + crack) as f32;
        p.l[i] = v;
        p.r[i] = v * 0.96;
    }
    p
}

fn demo_whoosh() -> Pcm {
    let secs = 1.2;
    let n = (secs * SR as f64) as usize;
    let mut p = Pcm::silent(n);
    let mut noise = Noise(37);
    let (mut lo_l, mut lo_r) = (0f32, 0f32);
    for i in 0..n {
        let t = i as f64 / SR as f64;
        let x = t / secs;
        let env = if x < 0.82 { (x / 0.82).powf(2.2) } else { (1.0 - (x - 0.82) / 0.18).max(0.0).powf(0.6) };
        let cutoff = (0.01 + 0.25 * x * x) as f32; // opens up as it arrives
        lo_l += cutoff * (noise.next() - lo_l);
        lo_r += cutoff * (noise.next() - lo_r);
        p.l[i] = lo_l * 1.6 * env as f32;
        p.r[i] = lo_r * 1.6 * env as f32;
    }
    p
}

/// Placeholder sounds for the demo project, written under audio/ only where missing.
/// They are synthesized, so the demo needs no downloaded assets.
pub fn write_demo_audio(root: &Path) -> Result<Vec<String>> {
    let mut written = Vec::new();
    type Synth = fn() -> Pcm;
    let items: [(&str, Synth); 3] =
        [("audio/drone.wav", || demo_drone(32.0)), ("audio/hit.wav", demo_hit), ("audio/whoosh.wav", demo_whoosh)];
    for (rel, make) in items {
        let p = root.join(rel);
        if !p.exists() {
            write_wav16(&p, &make())?;
            written.push(rel.to_string());
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f64, secs: f64, amp: f32) -> Pcm {
        let n = (secs * SR as f64) as usize;
        let l: Vec<f32> = (0..n).map(|i| amp * (2.0 * std::f64::consts::PI * hz * i as f64 / SR as f64).sin() as f32).collect();
        Pcm { r: l.clone(), l }
    }

    #[test]
    fn wav_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.wav");
        let s = sine(440.0, 0.1, 0.5);
        write_wav16(&p, &s).unwrap();
        let back = decode(&p).unwrap();
        assert_eq!(back.len(), s.len());
        assert!((back.l[100] - s.l[100]).abs() < 1e-3);
    }

    #[test]
    fn files_must_stay_inside() {
        assert!(valid_file("audio/hit.wav").is_ok());
        assert!(valid_file("../hit.wav").is_err());
        assert!(valid_file("C:/x/hit.wav").is_err());
        assert!(valid_file("/abs/hit.wav").is_err());
        assert!(valid_file("audio/notes.txt").is_err());
    }

    #[test]
    fn mix_places_effects_at_their_time() {
        let click = Pcm { l: vec![1.0; 10], r: vec![1.0; 10] };
        let mut src = HashMap::new();
        src.insert("c.wav".to_string(), click);
        let p = Placement {
            role: Role::Sfx, shot: Some("a".into()), index: 0, label: "c".into(), file: "c.wav".into(),
            start: 0.5, offset: 0.0, until: None, gain_db: -6.0206, fade_in: 0.0, fade_out: 0.0,
        };
        let (out, lanes) = mix(std::slice::from_ref(&p), &src, 1.0, 0.0);
        assert_eq!(out.len(), 48_000);
        assert_eq!(out.l[23_999], 0.0);
        assert!((out.l[24_000] - 0.5).abs() < 1e-3);
        assert_eq!(out.l[24_010], 0.0);
        assert!((lanes[0].from - 0.5).abs() < 1e-9);
        // an effect leading into the first shot is clipped at 0, not shifted
        let early = Placement { start: -5.0 / SR as f64, ..p };
        let (out, _) = mix(&[early], &src, 1.0, 0.0);
        assert!(out.l[4] > 0.4 && out.l[5] == 0.0);
    }
}
