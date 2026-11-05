//! A reference teaser (D-59): a video the director likes, measured the way the corpus was
//! (D-29, D-53) so the cut can borrow how it is paced and how it hands over. After open-design's
//! "replicate a reference video" (`od media reference` / `od media compare`): measure the timeline,
//! never estimate it, and let what was measured outrank taste.
//!
//! What is borrowed is timing only: the lengths of the dips, flashes and whiteouts and of the
//! text on black go into `finish.motion`; how often and where they fall reaches the planner. The
//! reference's pictures, words and sound never reach the cut (D-23, D-39). It lives in
//! `reference/<id>/`: the measurements (`facts.json`), a small silent copy for comparing
//! (`proxy.mp4`) and a contact sheet. `compare` measures the latest cut the same way and pairs
//! frames at the same share of each runtime, for the director to look at (D-22: no model reads
//! the comparison back).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Map, Value};

use crate::geometry::round_to;
use crate::model::{fmt_g, num, str_of, Definition};
use crate::{bail, Result};

/// Samples a second, on a 320 px wide picture (the corpus's measure).
pub const FPS: f64 = 12.0;
/// open-design's limits for a reference video.
pub const MAX_SECS: f64 = 180.0;
pub const MAX_BYTES: u64 = 200 * 1024 * 1024;

/// One sample of the picture: its mean and max luma.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sample {
    pub avg: f64,
    pub max: f64,
}

/// A run of black: how long nothing is on screen, and how the picture gets there and back
/// (seconds of steady dimming; about 0 for a cut).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Black {
    pub at: f64,
    pub gap: f64,
    pub fade_out: f64,
    pub fade_in: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flash {
    pub at: f64,
    pub len: f64,
}

/// White held longer than a flash: the rise into it, how long it stays, the way back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct White {
    pub at: f64,
    pub rise: f64,
    pub hold: f64,
    pub decay: f64,
}

/// Text on a dark frame: how it comes in, stays and leaves (0 = by a cut).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Text {
    pub at: f64,
    pub enter: f64,
    pub hold: f64,
    pub exit: f64,
    /// It came in on a frame already dark (else it came in with the cut to black).
    pub in_dark: bool,
    pub out_dark: bool,
    pub contrast: f64,
    pub steady: f64,
}

impl Text {
    /// Held, steady text that reads (the corpus's filter): not a lamp in a dark game.
    pub fn reads(&self) -> bool {
        self.hold >= 1.0 && self.contrast >= 12.0 && self.steady <= 0.12
    }
    /// Seconds it takes to come in (0 when it arrives with the cut).
    pub fn in_secs(&self) -> f64 {
        if self.in_dark { self.enter } else { 0.0 }
    }
    pub fn out_secs(&self) -> f64 {
        if self.out_dark { self.exit } else { 0.0 }
    }
}

/// What a video does over time, read from its luma and its scene changes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Measure {
    pub secs: f64,
    /// Edit points: hard cuts, the two cuts of a flash counted once (D-29's measure).
    pub cuts: Vec<f64>,
    pub blacks: Vec<Black>,
    pub flashes: Vec<Flash>,
    pub whites: Vec<White>,
    pub texts: Vec<Text>,
}

// ------------------------------------------------------------------ reading ffmpeg

/// The samples in ffmpeg's `metadata=print` output of `signalstats`.
pub fn parse_metadata(text: &str) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut cur: Option<Sample> = None;
    for line in text.lines() {
        if line.starts_with("frame:") {
            out.extend(cur.take());
            cur = Some(Sample::default());
            continue;
        }
        let Some((k, v)) = line.trim().split_once('=') else { continue };
        let (Some(s), Ok(v)) = (cur.as_mut(), v.trim().parse::<f64>()) else { continue };
        match k {
            "lavfi.signalstats.YAVG" => s.avg = v,
            "lavfi.signalstats.YMAX" => s.max = v,
            _ => {}
        }
    }
    out.extend(cur);
    out
}

/// (seconds, scene score) for every frame in ffmpeg's `metadata=print` output of `select`.
pub fn parse_scenes(text: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut t: Option<f64> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("frame:") {
            t = rest.split("pts_time:").nth(1).and_then(|x| x.split_whitespace().next()).and_then(|x| x.parse().ok());
        } else if let (Some(at), Some(v)) = (t, line.trim().strip_prefix("lavfi.scene_score=")) {
            if let Ok(v) = v.trim().parse::<f64>() {
                out.push((at, v));
            }
        }
    }
    out
}

/// Hard cuts from per-frame scene scores (D-29's measure, research_data/tools/build_refs.py):
/// a local peak of at least 0.1 that stands three times above the median of the 12 frames on
/// either side, plus 0.03. A fixed threshold misses cuts in dark footage; a fade lifts its whole
/// neighbourhood and is not counted.
pub fn detect_cuts(scores: &[(f64, f64)]) -> Vec<f64> {
    let s: Vec<f64> = scores.iter().map(|x| x.1).collect();
    let mut out = Vec::new();
    for (i, (t, v)) in scores.iter().enumerate() {
        let around = &s[i.saturating_sub(2)..(i + 3).min(s.len())];
        if *v < 0.1 || around.iter().any(|x| x > v) {
            continue;
        }
        let nb: Vec<f64> = s[i.saturating_sub(12)..i].iter().chain(&s[(i + 1).min(s.len())..(i + 13).min(s.len())]).copied().collect();
        if *v >= 3.0 * median(&nb) + 0.03 {
            out.push(round_to(*t, 2));
        }
    }
    out
}

/// Edit points: a cut within 0.25 s of the last one kept belongs to the same point (a flash).
pub fn edit_points(cuts: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for c in cuts {
        if out.last().map_or(true, |l| c - l >= 0.25) {
            out.push(*c);
        }
    }
    out
}

fn ffmpeg_exe() -> Result<PathBuf> {
    match crate::cut::ffmpeg() {
        Some(p) => Ok(p),
        None => bail!("ffmpeg not found on PATH: install it to measure a reference"),
    }
}

/// Seconds a video runs, from ffmpeg's banner (no decoding).
pub fn probe_secs(video: &Path) -> Result<f64> {
    let out = crate::cut::no_window(&mut Command::new(ffmpeg_exe()?)).args(["-hide_banner", "-i"]).arg(video).output()?;
    let err = String::from_utf8_lossy(&out.stderr);
    let Some(d) = err.split("Duration: ").nth(1).and_then(|s| s.split(',').next()) else {
        bail!("ffmpeg cannot read {} as a video", video.display());
    };
    let p: Vec<f64> = d.trim().split(':').filter_map(|x| x.parse().ok()).collect();
    match p.as_slice() {
        [h, m, s] => Ok(h * 3600.0 + m * 60.0 + s),
        _ => bail!("ffmpeg cannot read {} as a video", video.display()),
    }
}

/// ffmpeg's `metadata=print` output for the video under `vf` (at most `limit` seconds).
fn print_metadata(video: &Path, limit: f64, vf: &str) -> Result<String> {
    let out = crate::cut::no_window(&mut Command::new(ffmpeg_exe()?))
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(video)
        .args(["-an", "-t", &format!("{limit:.0}"), "-vf"])
        .arg(format!("{vf},metadata=print:file=-"))
        .args(["-f", "null", "-"])
        .output()?;
    if !out.status.success() {
        bail!("ffmpeg could not read {}: {}", video.display(), String::from_utf8_lossy(&out.stderr).trim().chars().take(400).collect::<String>());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// The video's samples, 12 a second (at most `limit` seconds).
pub fn luma(video: &Path, limit: f64) -> Result<Vec<Sample>> {
    Ok(parse_metadata(&print_metadata(video, limit, &format!("fps={FPS},scale=320:-2,signalstats"))?))
}

/// Everything the measure reads from a video: its luma (12 a second) and its cuts (every frame,
/// on a 160 px picture, as the corpus was measured).
pub fn measure_video(video: &Path, limit: f64) -> Result<Measure> {
    let samples = luma(video, limit)?;
    if samples.len() < 24 {
        bail!("{} is too short to measure (under 2 s)", video.display());
    }
    let mut m = measure(&samples);
    let scenes = parse_scenes(&print_metadata(video, limit, "scale=160:-2,select='gte(scene,0)'")?);
    m.cuts = edit_points(&detect_cuts(&scenes));
    Ok(m)
}

// ------------------------------------------------------------------ measuring

/// (start, end) index pairs of the true runs.
fn runs(flags: &[bool]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, f) in flags.iter().chain(std::iter::once(&false)).enumerate() {
        match (f, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    out
}

fn median(xs: &[f64]) -> f64 {
    let mut v: Vec<f64> = xs.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

/// Samples of steady change (each 6% + 0.5 past the next) that end at `i` going back
/// (`back`) or start after it going forward; `brighter` asks for the frames away from `i` to
/// be brighter (a fade into black seen backwards, a fade out of it forwards) or darker.
fn ramp(fr: &[Sample], i: usize, back: bool, brighter: bool) -> usize {
    let mut n = 0;
    loop {
        let (near, far) = if back {
            if i < n + 1 {
                break;
            }
            (i - n, i - n - 1)
        } else {
            if i + n + 1 >= fr.len() {
                break;
            }
            (i + n, i + n + 1)
        };
        let steps = if brighter {
            fr[far].avg > fr[near].avg * 1.06 + 0.5
        } else {
            fr[far].avg * 1.06 + 0.5 < fr[near].avg
        };
        if !steps || n >= 36 {
            break;
        }
        n += 1;
    }
    n
}

/// What the luma samples show (D-53's measure, research_data/tools/measure_motion.py): black
/// and how the picture reaches it, white flashes and whiteouts, text on black. Cuts come from
/// the scene scores (`measure_video`).
pub fn measure(fr: &[Sample]) -> Measure {
    let n = fr.len();
    let s = |i: usize| i as f64 / FPS;
    let r3 = |x: f64| round_to(x, 3);
    let mut blacks = Vec::new();
    for (a, e) in runs(&fr.iter().map(|f| f.max < 35.0).collect::<Vec<_>>()) {
        if a == 0 || e >= n {
            continue; // leading or trailing black
        }
        let (fo, fi) = (ramp(fr, a, true, true), ramp(fr, e - 1, false, true));
        if e - a < 2 && fo < 2 && fi < 2 {
            continue; // one dark frame no fade leads to
        }
        blacks.push(Black { at: r3(s(a)), gap: r3(s(e - a)), fade_out: r3(s(fo)), fade_in: r3(s(fi)) });
    }
    let (mut flashes, mut whites) = (Vec::new(), Vec::new());
    for (a, e) in runs(&fr.iter().map(|f| f.avg > 190.0).collect::<Vec<_>>()) {
        let len = s(e - a);
        if len <= 0.25 + 1e-9 {
            if a > 0 && e < n && fr[a - 1].avg < 150.0 && fr[e].avg < 150.0 {
                flashes.push(Flash { at: r3(s(a)), len: r3(len) });
            }
            continue;
        }
        let (up, down) = (ramp(fr, a, true, false), ramp(fr, e - 1, false, false));
        whites.push(White { at: r3(s(a)), rise: r3(s(up)), hold: r3(len), decay: r3(s(down)) });
    }
    Measure { secs: r3(s(n)), cuts: Vec::new(), blacks, flashes, whites, texts: text_on_black(fr) }
}

/// Text that appears on a dark frame (the corpus's measure, D-53): its entrance (to 90% of its
/// plateau), hold and exit.
fn text_on_black(fr: &[Sample]) -> Vec<Text> {
    let mut out = Vec::new();
    for (s, e) in runs(&fr.iter().map(|f| f.avg < 28.0).collect::<Vec<_>>()) {
        if e - s < 6 {
            continue; // at least half a second of dark frames
        }
        let seg: Vec<f64> = fr[s..e].iter().map(|f| f.max).collect();
        let mut sorted = seg.clone();
        sorted.sort_by(f64::total_cmp);
        let base = sorted[seg.len() / 10];
        let peak = sorted[seg.len() - 1];
        if peak - base < 15.0 {
            continue;
        }
        let lit: Vec<bool> = seg.iter().map(|m| *m > base + (0.1 * (peak - base)).max(4.0)).collect();
        for (ls, le) in runs(&lit) {
            let top_of = seg[ls..le].iter().copied().fold(f64::MIN, f64::max);
            if le - ls < 4 || top_of - base < 15.0 {
                continue;
            }
            let top: Vec<f64> = seg[ls..le].iter().copied().filter(|m| *m >= base + 0.5 * (top_of - base)).collect();
            let plateau = median(&top);
            let level = base + 0.9 * (plateau - base);
            let mut k = ls;
            while k < le && seg[k] < level {
                k += 1;
            }
            let mut j = le - 1;
            while j > ls && seg[j] < level {
                j -= 1;
            }
            let held: Vec<f64> = if k <= j { seg[k..=j].to_vec() } else { Vec::new() };
            let steady = if held.len() > 1 {
                let mean = held.iter().sum::<f64>() / held.len() as f64;
                (held.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / held.len() as f64).sqrt() / (plateau - base)
            } else {
                1.0
            };
            out.push(Text {
                at: round_to((s + ls) as f64 / FPS, 3),
                enter: round_to(k.saturating_sub(ls) as f64 / FPS, 3),
                hold: round_to((j + 1).saturating_sub(k) as f64 / FPS, 3),
                exit: round_to((le - 1).saturating_sub(j) as f64 / FPS, 3),
                in_dark: ls > 0,
                out_dark: le < seg.len(),
                contrast: round_to(plateau - base, 1),
                steady: round_to(steady, 3),
            });
        }
    }
    out
}

impl Measure {
    /// Seconds between edit points, on median (the corpus's shot length, D-29).
    pub fn median_shot(&self) -> f64 {
        let mut at: Vec<f64> = vec![0.0];
        at.extend(self.cuts.iter().copied().filter(|c| *c > 0.0 && *c < self.secs));
        at.push(self.secs);
        let gaps: Vec<f64> = at.windows(2).map(|w| w[1] - w[0]).filter(|g| *g > 0.04).collect();
        round_to(median(&gaps), 2)
    }

    fn texts_that_read(&self) -> Vec<&Text> {
        self.texts.iter().filter(|t| t.reads()).collect()
    }

    /// The counts the app shows: hand-over whites and readable text only.
    pub fn summary(&self) -> Value {
        json!({
            "secs": round_to(self.secs, 1), "cuts": self.cuts.len(), "median_shot": self.median_shot(),
            "blacks": self.blacks.len(), "flashes": self.flashes.len(), "whiteouts": self.whiteouts().len(),
            "texts": self.texts_that_read().len(),
        })
    }

    /// White that hands over: held at most 1.2 s (measured whiteouts hold 0-0.5 s). Longer white
    /// is a white picture (a card, a bright scene), not a hand-over.
    pub fn whiteouts(&self) -> Vec<&White> {
        self.whites.iter().filter(|w| w.hold <= 1.2).collect()
    }

    /// One line per thing measured, for the director (CLI, compare).
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        let pct = |n: usize, of: usize| if of == 0 { "-".to_string() } else { format!("{n}/{of}") };
        let cut = |x: f64| x <= 0.09;
        let sides: Vec<f64> = self.blacks.iter().flat_map(|b| [b.fade_out, b.fade_in]).collect();
        let faded: Vec<f64> = sides.iter().copied().filter(|x| !cut(*x)).collect();
        let texts = self.texts_that_read();
        let ins: Vec<f64> = texts.iter().map(|t| t.in_secs()).collect();
        let outs: Vec<f64> = texts.iter().map(|t| t.out_secs()).collect();
        let fade_of = |xs: &[f64]| {
            let f: Vec<f64> = xs.iter().copied().filter(|x| !cut(*x)).collect();
            if f.is_empty() { "cut".to_string() } else { format!("cut {} · fade {}s", pct(xs.len() - f.len(), xs.len()), fmt_g(round_to(median(&f), 2))) }
        };
        vec![
            ("runtime", format!("{}s", fmt_g(round_to(self.secs, 1)))),
            ("cuts", format!("{} (a change every {}s)", self.cuts.len(), fmt_g(self.median_shot()))),
            ("black", if self.blacks.is_empty() { "0".into() } else {
                format!("{} · held {}s · into/out of black by a cut {}{}", self.blacks.len(),
                        fmt_g(round_to(median(&self.blacks.iter().map(|b| b.gap).collect::<Vec<_>>()), 2)),
                        pct(sides.len() - faded.len(), sides.len()),
                        if faded.is_empty() { String::new() } else { format!(" · fades {}s", fmt_g(round_to(median(&faded), 2))) })
            }),
            ("flashes", if self.flashes.is_empty() { "0".into() } else {
                format!("{} · {}s", self.flashes.len(), fmt_g(round_to(median(&self.flashes.iter().map(|f| f.len).collect::<Vec<_>>()), 2)))
            }),
            ("whiteouts", {
                let w = self.whiteouts();
                let long = self.whites.len() - w.len();
                let m = |f: fn(&White) -> f64| fmt_g(round_to(median(&w.iter().map(|x| f(x)).collect::<Vec<_>>()), 2));
                let main = if w.is_empty() { "0".to_string() } else {
                    format!("{} · up {}s, white {}s, back {}s", w.len(), m(|w| w.rise), m(|w| w.hold), m(|w| w.decay))
                };
                if long > 0 { format!("{main} (+{long} white held longer: a picture, not a hand-over)") } else { main }
            }),
            ("text on black", if texts.is_empty() { "0".into() } else {
                format!("{} · in {} · held {}s · out {}", texts.len(), fade_of(&ins),
                        fmt_g(round_to(median(&texts.iter().map(|t| t.hold).collect::<Vec<_>>()), 1)), fade_of(&outs))
            }),
        ]
    }

    pub fn to_json(&self) -> Value {
        json!({
            "secs": self.secs,
            "cuts": self.cuts,
            "blacks": self.blacks.iter().map(|b| json!({"at": b.at, "gap": b.gap, "fade_out": b.fade_out, "fade_in": b.fade_in})).collect::<Vec<_>>(),
            "flashes": self.flashes.iter().map(|f| json!({"at": f.at, "len": f.len})).collect::<Vec<_>>(),
            "whiteouts": self.whites.iter().map(|w| json!({"at": w.at, "rise": w.rise, "hold": w.hold, "decay": w.decay})).collect::<Vec<_>>(),
            "texts": self.texts.iter().map(|t| json!({"at": t.at, "in": t.enter, "hold": t.hold, "out": t.exit, "in_dark": t.in_dark,
                                                       "out_dark": t.out_dark, "contrast": t.contrast, "steady": t.steady})).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(v: &Value) -> Measure {
        let f = |o: &Value, k: &str| num(&o[k]).unwrap_or(0.0);
        let list = |k: &str| v[k].as_array().cloned().unwrap_or_default();
        Measure {
            secs: f(v, "secs"),
            cuts: list("cuts").iter().filter_map(num).collect(),
            blacks: list("blacks").iter().map(|o| Black { at: f(o, "at"), gap: f(o, "gap"), fade_out: f(o, "fade_out"), fade_in: f(o, "fade_in") }).collect(),
            flashes: list("flashes").iter().map(|o| Flash { at: f(o, "at"), len: f(o, "len") }).collect(),
            whites: list("whiteouts").iter().map(|o| White { at: f(o, "at"), rise: f(o, "rise"), hold: f(o, "hold"), decay: f(o, "decay") }).collect(),
            texts: list("texts")
                .iter()
                .map(|o| Text {
                    at: f(o, "at"), enter: f(o, "in"), hold: f(o, "hold"), exit: f(o, "out"),
                    in_dark: o["in_dark"].as_bool().unwrap_or(false), out_dark: o["out_dark"].as_bool().unwrap_or(false),
                    contrast: f(o, "contrast"), steady: f(o, "steady"),
                })
                .collect(),
        }
    }
}

// ------------------------------------------------------------------ what the cut borrows

/// The `finish.motion` entries a measure sets (D-59): the lengths of its hand-overs and of its
/// text on black, each with a note on where it came from. Seconds are as measured (energy does
/// not scale them); what the reference never does is left to the look.
pub fn motion_entries(m: &Measure) -> (Map<String, Value>, Vec<String>) {
    let mut out = Map::new();
    let mut notes = Vec::new();
    let frame = 1.0 / FPS;
    let r2 = |x: f64| round_to(x, 2);
    if !m.blacks.is_empty() {
        // each side of each black: a cut counts as one sample, a fade as its length
        let sides: Vec<f64> = m.blacks.iter().flat_map(|b| [b.fade_out, b.fade_in]).map(|x| if x <= 0.09 { frame } else { x }).collect();
        let dip = r2(median(&sides).clamp(0.05, 1.5));
        out.insert("dip".into(), json!(dip));
        notes.push(format!("dip {}s: {} times through black", fmt_g(dip), m.blacks.len()));
    }
    if !m.flashes.is_empty() {
        let flash = r2(median(&m.flashes.iter().map(|f| f.len).collect::<Vec<_>>()).clamp(0.04, 0.3));
        out.insert("flash".into(), json!(flash));
        notes.push(format!("flash {}s: {} white flashes", fmt_g(flash), m.flashes.len()));
    }
    let whites = m.whiteouts();
    if !whites.is_empty() {
        let med = |f: fn(&White) -> f64| median(&whites.iter().map(|w| f(w)).collect::<Vec<_>>());
        // the white run counts the frames on either side that are nearly white as well
        let (rise, peak, decay) = (r2(med(|w| w.rise).clamp(0.0, 1.5)), r2((med(|w| w.hold) - frame).clamp(0.0, 1.5)), r2(med(|w| w.decay).clamp(0.0, 2.0)));
        out.insert("whiteout".into(), json!({"rise": rise, "peak": peak, "decay": decay}));
        notes.push(format!("whiteout up {}s, white {}s, back {}s: {} whiteouts", fmt_g(rise), fmt_g(peak), fmt_g(decay), whites.len()));
    }
    let (nf, nw) = (m.flashes.len(), whites.len());
    let accent = if nw > 0 && nw >= nf {
        Some("whiteout")
    } else if nf > 0 {
        Some("flash")
    } else if !m.blacks.is_empty() {
        Some("dip")
    } else {
        None
    };
    if let Some(a) = accent {
        out.insert("accent".into(), json!(a));
        notes.push(format!("accent {a}: what the reference jolts with"));
    }
    let texts = m.texts_that_read();
    if texts.len() >= 2 {
        let side = |xs: Vec<f64>| -> (&'static str, f64) {
            let faded: Vec<f64> = xs.iter().copied().filter(|x| *x > 0.09).collect();
            if faded.len() * 2 <= xs.len() { ("cut", 0.0) } else { ("fade", r2(median(&faded).clamp(0.1, 2.0))) }
        };
        let (enter, enter_s) = side(texts.iter().map(|t| t.in_secs()).collect());
        let (exit, exit_s) = side(texts.iter().map(|t| t.out_secs()).collect());
        out.insert("card".into(), json!({"in": enter, "in_s": enter_s, "out": exit, "out_s": exit_s}));
        notes.push(format!("cards in by {enter}{}, out by {exit}{}: {} lines on black",
                           if enter == "fade" { format!(" {}s", fmt_g(enter_s)) } else { String::new() },
                           if exit == "fade" { format!(" {}s", fmt_g(exit_s)) } else { String::new() }, texts.len()));
    }
    (out, notes)
}

/// `finish.motion` with a measure's entries on top (measured outranks the look; the profile,
/// the energy and the moves the director ruled out stay).
pub fn merged_motion(current: &Value, m: &Measure) -> Value {
    let mut out = match current {
        Value::Object(o) => o.clone(),
        Value::String(s) if !s.trim().is_empty() => {
            let mut o = Map::new();
            o.insert("profile".into(), json!(s.trim()));
            o
        }
        _ => Map::new(),
    };
    for (k, v) in motion_entries(m).0 {
        match (out.get_mut(&k), v) {
            (Some(Value::Object(have)), Value::Object(add)) => have.extend(add),
            (_, v) => {
                out.insert(k, v);
            }
        }
    }
    Value::Object(out)
}

/// `finish.motion` with a measure's entries taken back out where they are still as it set them:
/// switching references leaves nothing of the old one behind, and what the director changed
/// since stays.
pub fn without(current: &Value, m: &Measure) -> Value {
    let Value::Object(mut out) = current.clone() else { return current.clone() };
    for (k, v) in motion_entries(m).0 {
        let gone = match (out.get_mut(&k), &v) {
            (Some(Value::Object(have)), Value::Object(set)) => {
                have.retain(|sk, sv| set.get(sk) != Some(sv));
                have.is_empty()
            }
            (Some(have), v) => have == v,
            (None, _) => false,
        };
        if gone {
            out.remove(&k);
        }
    }
    Value::Object(out)
}

/// The motion of a cut whose brief names a measured reference: `base` with its entries on top.
pub fn motion_for(defn: &Definition, base: Value) -> Value {
    match active(defn) {
        Some(r) => merged_motion(&base, &r.measure),
        None => base,
    }
}

// ------------------------------------------------------------------ the reference on disk

#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    pub id: String,
    /// The file it was measured from (its name).
    pub source: String,
    pub bytes: u64,
    pub measure: Measure,
}

pub fn dir(root: &Path, id: &str) -> PathBuf {
    root.join("reference").join(id)
}

pub fn load(root: &Path, id: &str) -> Option<Reference> {
    if !valid_id(id) {
        return None;
    }
    let v: Value = serde_json::from_str(&std::fs::read_to_string(dir(root, id).join("facts.json")).ok()?).ok()?;
    Some(Reference {
        id: id.to_string(),
        source: str_of(&v["source"]),
        bytes: v["bytes"].as_u64().unwrap_or(0),
        measure: Measure::from_json(&v),
    })
}

/// Every measured reference in the project.
pub fn list(root: &Path) -> Vec<Reference> {
    let mut ids: Vec<String> = std::fs::read_dir(root.join("reference"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect();
    ids.sort();
    ids.iter().filter_map(|id| load(root, id)).collect()
}

/// The reference the brief names (`brief.reference_teaser`), when it has been measured.
pub fn active(defn: &Definition) -> Option<Reference> {
    let id = defn.brief.reference_teaser.trim();
    if id.is_empty() { None } else { load(&defn.root(), id) }
}

/// A reference id: lower-case letters, digits and dashes.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 40 && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn new_id(root: &Path, video: &Path, bytes: u64) -> String {
    let name = video.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    // the same file measured again keeps its id
    if let Some(r) = list(root).into_iter().find(|r| r.source == name && r.bytes == bytes) {
        return r.id;
    }
    let stem = video.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut slug = String::new();
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let mut slug: String = slug.trim_matches('-').chars().take(32).collect();
    slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() {
        slug = "ref".into();
    }
    let mut id = slug.clone();
    let mut k = 2;
    while dir(root, &id).exists() {
        id = format!("{slug}-{k}");
        k += 1;
    }
    id
}

/// Measure a video into `reference/<id>/` (its facts, a small silent copy, a contact sheet).
pub fn add(root: &Path, video: &Path) -> Result<Reference> {
    let Ok(meta) = std::fs::metadata(video) else { bail!("no such video: {}", video.display()) };
    if !meta.is_file() {
        bail!("no such video: {}", video.display());
    }
    if meta.len() > MAX_BYTES {
        bail!("{} is {} MB; a reference is at most {} MB: trim the part you want", video.display(), meta.len() / (1024 * 1024), MAX_BYTES / (1024 * 1024));
    }
    let secs = probe_secs(video)?;
    if secs > MAX_SECS {
        bail!("{} runs {}s; a reference is at most {}s: trim the part you want", video.display(), fmt_g(round_to(secs, 1)), fmt_g(MAX_SECS));
    }
    let measure = measure_video(video, MAX_SECS + 1.0)?;
    let id = new_id(root, video, meta.len());
    let d = dir(root, &id);
    std::fs::create_dir_all(&d)?;
    let proxy = d.join("proxy.mp4");
    let s = |p: &Path| p.display().to_string();
    crate::cut::run_ffmpeg(&[
        "-i".into(), s(video), "-an".into(), "-vf".into(), "scale=640:-2,fps=30,format=yuv420p".into(),
        "-c:v".into(), "libx264".into(), "-preset".into(), "veryfast".into(), "-crf".into(), "28".into(), s(&proxy),
    ])?;
    crate::cut::run_ffmpeg(&[
        "-i".into(), s(&proxy), "-vf".into(), format!("fps={:.4},scale=320:-2,tile=6x4", 24.0 / measure.secs.max(1.0)),
        "-frames:v".into(), "1".into(), "-update".into(), "1".into(), s(&d.join("contact.jpg")),
    ])?;
    let name = video.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut facts = measure.to_json();
    if let Value::Object(o) = &mut facts {
        o.insert("id".into(), json!(id));
        o.insert("source".into(), json!(name));
        o.insert("bytes".into(), json!(meta.len()));
        o.insert("measured".into(), json!(chrono::Local::now().to_rfc3339()));
        o.insert("fps".into(), json!(FPS));
    }
    std::fs::write(d.join("facts.json"), serde_json::to_string_pretty(&facts).unwrap_or_default())?;
    Ok(Reference { id, source: name, bytes: meta.len(), measure })
}

// ------------------------------------------------------------------ what the planner reads

/// Shares of the runtime, as percentages (the first dozen).
fn shares(times: impl Iterator<Item = f64>, secs: f64) -> String {
    let v: Vec<String> = times.take(12).map(|t| format!("{:.0}%", 100.0 * t / secs.max(0.01))).collect();
    v.join(", ")
}

/// What the planner is told about the reference the brief names (D-59): its rhythm and where
/// its hand-overs fall, in shares of its runtime. Nothing of its content.
pub fn planner_text(defn: &Definition) -> String {
    let Some(r) = active(defn) else { return String::new() };
    let m = &r.measure;
    let mut out = format!(
        "\n## Reference teaser `{}` (measured: borrow its rhythm and its hand-overs, never its pictures, words or story)\n\
         - {}s long; the picture changes every {}s on median ({} cuts).\n",
        r.id, fmt_g(round_to(m.secs, 1)), fmt_g(m.median_shot()), m.cuts.len()
    );
    if !m.blacks.is_empty() {
        let into_cut = m.blacks.iter().filter(|b| b.fade_out <= 0.09).count();
        out.push_str(&format!(
            "- {} times through black (into black by a cut {} of them), black held about {}s, at {} of the runtime.\n",
            m.blacks.len(), into_cut,
            fmt_g(round_to(median(&m.blacks.iter().map(|b| b.gap).collect::<Vec<_>>()), 1)),
            shares(m.blacks.iter().map(|b| b.at), m.secs)
        ));
    }
    if !m.flashes.is_empty() {
        out.push_str(&format!("- {} white flashes, at {}.\n", m.flashes.len(), shares(m.flashes.iter().map(|f| f.at), m.secs)));
    }
    let whites = m.whiteouts();
    if !whites.is_empty() {
        out.push_str(&format!("- {} whiteouts, at {}.\n", whites.len(), shares(whites.iter().map(|w| w.at), m.secs)));
    }
    let texts = m.texts_that_read();
    if !texts.is_empty() {
        out.push_str(&format!("- {} lines of text on black, at {}.\n", texts.len(), shares(texts.iter().map(|t| t.at), m.secs)));
    }
    out.push_str(
        "- The look's motion already takes its lengths. Place cuts, `dip`, `flash`, `whiteout` and `black` slots at about \
         the same shares of this cut's own length, as many as fit it; where the reference does none of them, cut.\n",
    );
    out
}

// ------------------------------------------------------------------ comparing a cut with it

#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub label: String,
    pub at_ref: f64,
    pub at_cut: f64,
    /// Under `reference/<id>/compare/`.
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct Comparison {
    pub reference: Reference,
    pub cut: Measure,
    pub pairs: Vec<Pair>,
    pub dir: PathBuf,
}

/// Where to look: the opening, the reference's hand-overs and an even spread, as shares of the
/// runtime (at most 12).
pub fn pair_points(m: &Measure) -> Vec<(String, f64)> {
    let secs = m.secs.max(0.01);
    let mut pts: Vec<(String, f64)> = vec![("opening".into(), (0.5 / secs).min(0.05))];
    pts.extend(m.blacks.iter().map(|b| ("black".to_string(), b.at / secs)));
    pts.extend(m.flashes.iter().map(|f| ("flash".to_string(), f.at / secs)));
    pts.extend(m.whiteouts().iter().map(|w| ("whiteout".to_string(), (w.at + w.hold / 2.0) / secs)));
    pts.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut kept: Vec<(String, f64)> = Vec::new();
    for p in pts {
        if kept.len() < 9 && kept.last().map_or(true, |k| p.1 - k.1 >= 0.03) {
            kept.push(p);
        }
    }
    for x in [0.25, 0.5, 0.75, 0.95] {
        if kept.len() < 12 && kept.iter().all(|k| (k.1 - x).abs() >= 0.05) {
            kept.push(("even".into(), x));
        }
    }
    kept.sort_by(|a, b| a.1.total_cmp(&b.1));
    kept
}

/// Measure `cut_video` the same way as the reference and write frame pairs (the reference on
/// the left, the cut on the right) at the same shares of their runtimes, for the director.
pub fn compare(root: &Path, r: &Reference, cut_video: &Path) -> Result<Comparison> {
    let secs = probe_secs(cut_video)?;
    let cut = measure_video(cut_video, secs + 1.0)?;
    let proxy = dir(root, &r.id).join("proxy.mp4");
    if !proxy.is_file() {
        bail!("reference `{}` has no proxy.mp4: measure it again with `shotpilot reference <video>`", r.id);
    }
    let out_dir = dir(root, &r.id).join("compare");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir)?;
    let font = crate::finish::fonts(&crate::finish::FinishCfg::default()).map(|f| crate::finish::filter_path(&f.sans));
    // the label sits on a band under the picture, clear of the video's own subtitles
    let label = |text: &str| match &font {
        Some(f) => format!(
            ",pad=480:300:0:0:black,drawtext=fontfile='{f}':expansion=none:text='{text}':x=8:y=278:fontsize=17:fontcolor=white"
        ),
        None => ",pad=480:300:0:0:black".to_string(),
    };
    let fit = "scale=480:270:force_original_aspect_ratio=decrease,pad=480:270:(ow-iw)/2:(oh-ih)/2,setsar=1";
    let mut pairs = Vec::new();
    for (i, (what, share)) in pair_points(&r.measure).into_iter().enumerate() {
        let (at_ref, at_cut) = (round_to(share * r.measure.secs, 2), round_to((share * cut.secs).min((cut.secs - 0.1).max(0.0)), 2));
        let file = format!("pair-{:02}.jpg", i + 1);
        let tag = |who: &str, t: f64| format!("{who} {}s ({:.0}%)", fmt_g(t), 100.0 * share).replace(':', "\\:");
        crate::cut::run_ffmpeg(&[
            "-ss".into(), format!("{at_ref:.2}"), "-i".into(), proxy.display().to_string(),
            "-ss".into(), format!("{at_cut:.2}"), "-i".into(), cut_video.display().to_string(),
            "-filter_complex".into(),
            format!("[0:v]{fit}{}[a];[1:v]{fit}{}[b];[a][b]hstack=inputs=2", label(&tag(&format!("{} {what} - reference", i + 1), at_ref)), label(&tag("cut", at_cut))),
            "-frames:v".into(), "1".into(), "-update".into(), "1".into(), out_dir.join(&file).display().to_string(),
        ])?;
        pairs.push(Pair { label: what, at_ref, at_cut, file });
    }
    if !pairs.is_empty() {
        let rows = pairs.len().div_ceil(2);
        crate::cut::run_ffmpeg(&[
            "-i".into(), out_dir.join("pair-%02d.jpg").display().to_string(),
            "-vf".into(), format!("tile=2x{rows}:padding=6:margin=6"), "-frames:v".into(), "1".into(), "-update".into(), "1".into(),
            out_dir.join("sheet.jpg").display().to_string(),
        ])?;
    }
    let v = json!({
        "reference": r.id, "cut": cut_video.display().to_string(), "compared": chrono::Local::now().to_rfc3339(),
        "measure": {"reference": r.measure.to_json(), "cut": cut.to_json()},
        "pairs": pairs.iter().map(|p| json!({"label": p.label, "at_ref": p.at_ref, "at_cut": p.at_cut, "file": p.file})).collect::<Vec<_>>(),
    });
    std::fs::write(out_dir.join("compare.json"), serde_json::to_string_pretty(&v).unwrap_or_default())?;
    Ok(Comparison { reference: r.clone(), cut, pairs, dir: out_dir })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Samples from (seconds, mean luma, max luma) segments.
    fn clip(parts: &[(f64, f64, f64)]) -> Vec<Sample> {
        let mut out: Vec<Sample> = Vec::new();
        for &(secs, avg, max) in parts {
            for _ in 0..(secs * FPS).round() as usize {
                out.push(Sample { avg, max });
            }
        }
        out
    }

    /// A fade between two levels over `secs`, sample by sample.
    fn fade(from: f64, to: f64, secs: f64) -> Vec<(f64, f64, f64)> {
        let n = (secs * FPS).round() as usize;
        (1..=n).map(|k| {
            let v = from + (to - from) * k as f64 / (n + 1) as f64;
            (1.0 / FPS, v, (v * 2.5 + 20.0).min(235.0))
        }).collect()
    }

    #[test]
    fn metadata_is_read_frame_by_frame() {
        let text = "frame:0    pts:0       pts_time:0\nlavfi.signalstats.YMIN=17\nlavfi.signalstats.YAVG=125.122\n\
                    lavfi.signalstats.YMAX=214\nlavfi.scd.mafd=0.000\nlavfi.scd.score=0.000\n\
                    frame:1    pts:1       pts_time:0.0833333\nlavfi.signalstats.YAVG=98.2\nlavfi.signalstats.YMAX=165\n";
        let s = parse_metadata(text);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].avg, s[0].max, s[1].avg), (125.122, 214.0, 98.2));
        let sc = parse_scenes(
            "frame:0    pts:0       pts_time:0\nlavfi.scene_score=0.000000\nframe:1    pts:512     pts_time:0.0417\nlavfi.scene_score=0.183400\n",
        );
        assert_eq!(sc, vec![(0.0, 0.0), (0.0417, 0.1834)]);
    }

    #[test]
    fn cuts_are_peaks_that_stand_out_even_in_the_dark() {
        // dark footage: a cut scores 0.18 against 0.01-0.02 around it (a fixed 0.3 would miss it)
        let mut s: Vec<f64> = (0..48).map(|i| 0.01 + 0.01 * (i % 2) as f64).collect();
        s[24] = 0.18;
        // a slow fade lifts every frame a little: no cut
        for x in s.iter_mut().skip(30).take(12) {
            *x = 0.09;
        }
        // a white flash: into it and out of it, 0.17 s apart, are one edit point
        s.extend((0..24).map(|_| 0.02));
        s[60] = 0.6;
        s[64] = 0.55;
        let scores: Vec<(f64, f64)> = s.iter().enumerate().map(|(i, v)| (i as f64 / 24.0, *v)).collect();
        let cuts = detect_cuts(&scores);
        assert_eq!(cuts, vec![1.0, 2.5, 2.67], "{cuts:?}");
        assert_eq!(edit_points(&cuts), vec![1.0, 2.5]);
    }

    #[test]
    fn cuts_dips_flashes_and_whiteouts_are_told_apart() {
        let mut parts = vec![(2.0, 90.0, 200.0), (2.0, 40.0, 180.0)]; // a cut at 2 s
        parts.extend(fade(40.0, 10.0, 0.5)); // fades into black over 0.5 s
        parts.push((0.5, 10.0, 20.0)); // black held 0.5 s
        parts.push((2.0, 100.0, 220.0)); // back by a cut
        parts.push((0.17, 230.0, 235.0)); // a two-sample white flash
        parts.push((2.0, 80.0, 200.0));
        parts.extend(fade(80.0, 228.0, 0.34)); // up to white
        parts.push((0.5, 232.0, 235.0)); // white held
        parts.extend(fade(232.0, 70.0, 0.75)); // and back
        parts.push((2.0, 70.0, 190.0));
        let m = measure(&clip(&parts));
        assert_eq!(m.blacks.len(), 1, "{m:?}");
        let b = m.blacks[0];
        assert!((b.gap - 0.5).abs() < 0.1 && b.fade_out >= 0.4 && b.fade_in <= 0.09, "{b:?}");
        assert_eq!(m.flashes.len(), 1, "{m:?}");
        assert!((m.flashes[0].len - 0.167).abs() < 0.01);
        assert_eq!(m.whites.len(), 1, "{m:?}");
        let w = m.whites[0];
        assert!(w.rise >= 0.25 && w.decay >= 0.4 && w.hold >= 0.5, "{w:?}");
    }

    #[test]
    fn text_on_black_comes_in_holds_and_leaves() {
        let mut parts = vec![(2.0, 90.0, 200.0), (0.5, 8.0, 18.0)]; // cut to black
        // a line fades in over 0.5 s, holds 2 s, cuts out
        for k in 1..=6 {
            parts.push((1.0 / FPS, 9.0, 18.0 + 22.0 * k as f64));
        }
        parts.push((2.0, 12.0, 150.0));
        parts.push((0.5, 8.0, 18.0));
        parts.push((2.0, 90.0, 200.0));
        let m = measure(&clip(&parts));
        let t: Vec<&Text> = m.texts.iter().filter(|t| t.reads()).collect();
        assert_eq!(t.len(), 1, "{:?}", m.texts);
        assert!(t[0].in_dark && t[0].in_secs() >= 0.4 && t[0].out_secs() <= 0.09 && t[0].hold >= 1.9, "{:?}", t[0]);
    }

    #[test]
    fn the_cut_borrows_the_measured_lengths() {
        let m = Measure {
            secs: 40.0,
            cuts: vec![2.0, 5.0],
            blacks: vec![
                Black { at: 10.0, gap: 0.6, fade_out: 0.0, fade_in: 0.0 },
                Black { at: 20.0, gap: 0.5, fade_out: 0.0, fade_in: 0.33 },
                Black { at: 30.0, gap: 0.7, fade_out: 0.0, fade_in: 0.0 },
            ],
            flashes: vec![Flash { at: 12.0, len: 0.083 }, Flash { at: 14.0, len: 0.167 }, Flash { at: 15.0, len: 0.083 }],
            // the second is a white picture held 3.5 s, not a hand-over
            whites: vec![White { at: 33.0, rise: 0.33, hold: 0.42, decay: 0.92 }, White { at: 5.0, rise: 0.0, hold: 3.5, decay: 0.17 }],
            texts: vec![
                Text { at: 1.0, enter: 0.5, hold: 2.0, exit: 0.0, in_dark: true, out_dark: false, contrast: 80.0, steady: 0.02 },
                Text { at: 24.0, enter: 0.58, hold: 2.5, exit: 0.0, in_dark: true, out_dark: false, contrast: 90.0, steady: 0.03 },
                Text { at: 36.0, enter: 0.0, hold: 0.5, exit: 0.0, in_dark: true, out_dark: true, contrast: 5.0, steady: 0.5 },
            ],
        };
        let (e, notes) = motion_entries(&m);
        assert_eq!(e["dip"], json!(0.08)); // most sides are cuts: a one-sample dip
        assert_eq!(e["flash"], json!(0.08));
        assert_eq!(e["whiteout"], json!({"rise": 0.33, "peak": 0.34, "decay": 0.92}));
        assert_eq!(e["accent"], json!("flash")); // three flashes, one whiteout
        assert_eq!(e["card"], json!({"in": "fade", "in_s": 0.54, "out": "cut", "out_s": 0.0}));
        assert_eq!(notes.len(), 5, "{notes:?}");
        // merged over the look: its profile, energy and avoid stay, its card hold stays
        let look = json!({"profile": "horror", "energy": "calm", "avoid": ["glitch"], "card": {"hold": "glow", "in": "cut"}});
        let v = merged_motion(&look, &m);
        assert_eq!((v["profile"].clone(), v["energy"].clone(), v["avoid"].clone()), (json!("horror"), json!("calm"), json!(["glitch"])));
        assert_eq!(v["card"], json!({"hold": "glow", "in": "fade", "in_s": 0.54, "out": "cut", "out_s": 0.0}));
        // what it sets is a valid finish.motion, and resolves to the measured lengths as they are
        let mut errs = Vec::new();
        let cfg = crate::motion::parse(Some(&v), &mut errs);
        assert!(errs.is_empty(), "{errs:?}");
        let mo = crate::motion::resolve(&cfg);
        assert_eq!((mo.dip, mo.flash, mo.whiteout.peak, mo.card.enter_s), (0.08, 0.08, 0.34, 0.54));
        // a reference with no hand-overs leaves the look alone
        assert!(motion_entries(&Measure { secs: 30.0, ..Default::default() }).0.is_empty());
    }

    #[test]
    fn pairs_are_taken_where_the_reference_hands_over() {
        let m = Measure {
            secs: 40.0,
            blacks: vec![Black { at: 10.0, gap: 0.6, fade_out: 0.0, fade_in: 0.0 }],
            flashes: vec![Flash { at: 20.0, len: 0.1 }, Flash { at: 20.4, len: 0.1 }],
            ..Default::default()
        };
        let p = pair_points(&m);
        let labels: Vec<&str> = p.iter().map(|(l, _)| l.as_str()).collect();
        // the second flash is too close to the first; the even points skip where a hand-over is
        assert_eq!(labels, ["opening", "black", "flash", "even", "even"], "{p:?}");
        assert!(p.windows(2).all(|w| w[0].1 <= w[1].1));
    }

    #[test]
    fn ids_are_safe_names() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(new_id(root.path(), Path::new("C:/x/My Trailer (2025).mp4"), 1), "my-trailer-2025");
        assert_eq!(new_id(root.path(), Path::new("C:/x/★★.mp4"), 1), "ref");
        assert!(valid_id("my-trailer-2") && !valid_id("../x") && !valid_id("A"));
    }
}
