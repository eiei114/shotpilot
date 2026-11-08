//! Music and the air of places made in code (D-106): the forge's beds, next to its effects
//! (recipes.rs). A teaser that takes no sound from the library (`audio.library: false`, the
//! default of the Three.js sample) takes every sound from the forge, and these are its music and
//! the air of its places. Plain synthesis seeded by the project, like the effects, so another
//! game's teaser does not get the same take: music is plucked and held tones over a four-chord
//! loop at 120 bpm (cuts on the half second land on a beat) and ends on a last chord; the air is
//! filtered noise with a few living things in it, its tail folded over its head so the loop has
//! no seam.

use std::f64::consts::{PI, TAU};

use super::dsp::{self, Mode, Rng, St, SR};

/// Ids of music start with this.
pub const MUSIC: &str = "music/";
/// Ids of the air of a place start with this.
pub const AIR: &str = "air/";
/// Lengths a bed's `dur` knob may ask for (s).
pub const DUR_RANGE: (f64, f64) = (4.0, 180.0);
/// 120 bpm: a beat is half a second, a bar two seconds.
const BEAT: f64 = 0.5;
const BAR: f64 = 4.0 * BEAT;
/// The stretch of a loop's tail that is folded over its head (s).
const FOLD: f64 = 0.5;

/// One bed of the catalogue.
pub struct Bed {
    pub id: &'static str,
    pub kinds: &'static [&'static str],
    pub note: &'static str,
    /// Its own length (s).
    pub length: f64,
    make: fn(&mut Rng, f64) -> St,
}

impl Bed {
    /// The air loops under its place's shots; music runs once and ends on a last chord.
    pub fn loops(&self) -> bool {
        self.id.starts_with(AIR)
    }
}

pub fn catalogue() -> Vec<Bed> {
    const MUSIC_KINDS: &[&str] = &["bgm", "music"];
    const AIR_KINDS: &[&str] = &["ambience", "loop"];
    vec![
        Bed { id: "music/musicbox", kinds: MUSIC_KINDS, length: 48.0, make: musicbox,
              note: "a music box over a four-chord loop with a soft bass and pad under it: light, warm, small (a village, a garden, a cafe)" },
        Bed { id: "music/pad", kinds: MUSIC_KINDS, length: 48.0, make: pad,
              note: "slow warm chords on a soft pad, no beat: calm, wide, hopeful (open land, a quiet reveal)" },
        Bed { id: "music/pulse", kinds: MUSIC_KINDS, length: 48.0, make: pulse,
              note: "a steady eighth-note pulse on a low pluck over minor chords, ticks joining and the level climbing: drive, a march, escalation" },
        Bed { id: "music/drone", kinds: MUSIC_KINDS, length: 48.0, make: drone,
              note: "a low drone with slow beating, dark air thickening toward the end and a faint heartbeat: unease, night, a held breath" },
        Bed { id: "air/breeze", kinds: AIR_KINDS, length: 16.0, make: breeze,
              note: "wind over open ground, leaves and a few birds: a sunny day outside" },
        Bed { id: "air/wind", kinds: AIR_KINDS, length: 16.0, make: wind,
              note: "colder wind in slow gusts with a thin whistle and nothing alive: night outside, a ruin, a high place" },
        Bed { id: "air/room", kinds: AIR_KINDS, length: 16.0, make: room,
              note: "a quiet room: soft air from a vent and a low mains hum (inside a building)" },
        Bed { id: "air/crickets", kinds: AIR_KINDS, length: 16.0, make: crickets,
              note: "an evening field: crickets near and far over a soft breeze" },
    ]
}

/// Render one take: stereo at 48 kHz, `dur` long (its own length when not asked), peak -1 dBFS.
/// The air comes out as a loop; music fades out at its end.
pub fn render(bed: &Bed, seed: u64, dur: Option<f64>) -> St {
    let len = dur.unwrap_or(bed.length).clamp(DUR_RANGE.0, DUR_RANGE.1);
    let n = at(len);
    let mut r = Rng::new(seed);
    let mut y = (bed.make)(&mut r, len);
    if bed.loops() {
        y = fold(y, n);
    } else {
        fit(&mut y, n);
        y.fade(0.005, (0.1 * len).min(1.5));
    }
    y.normalize();
    y
}

// ------------------------------------------------------------------------------- shared parts

/// The sample a time falls on.
fn at(t: f64) -> usize {
    (t.max(0.0) * SR).round() as usize
}

fn hz(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

fn pick<T: Copy>(r: &mut Rng, xs: &[T]) -> T {
    xs[((r.unit() * xs.len() as f64) as usize).min(xs.len() - 1)]
}

fn silent(n: usize) -> St {
    St { l: vec![0.0; n], r: vec![0.0; n] }
}

/// Noise brought to `rms`.
fn scaled(x: Vec<f64>, rms: f64) -> Vec<f64> {
    dsp::unit_rms(x).into_iter().map(|v| v * rms).collect()
}

/// Exactly `n` samples: cut, or padded with silence.
fn fit(y: &mut St, n: usize) {
    y.truncate(n);
    y.l.resize(n, 0.0);
    y.r.resize(n, 0.0);
}

/// The tail past `n` folded over the head (equal power), so the loop runs on without a seam.
fn fold(mut y: St, n: usize) -> St {
    let extra = y.len().saturating_sub(n).min(n);
    for i in 0..extra {
        let u = i as f64 / extra as f64;
        let (win, wout) = ((0.5 * PI * u).sin(), (0.5 * PI * u).cos());
        y.l[i] = y.l[i] * win + y.l[n + i] * wout;
        y.r[i] = y.r[i] * win + y.r[n + i] * wout;
    }
    fit(&mut y, n);
    y
}

/// A sine of `f` added into `y` from sample `start` for `len` samples, its level `amp(i)` and
/// its place `pan` (-1 left .. 1 right). A running rotation, so a long note stays cheap.
fn ring(y: &mut St, start: usize, len: usize, f: f64, pan: f64, mut amp: impl FnMut(usize) -> f64) {
    if f >= 0.45 * SR || f <= 0.0 {
        return;
    }
    let w = TAU * f / SR;
    let (cw, sw) = (w.cos(), w.sin());
    let (mut c, mut s) = (1.0f64, 0.0f64);
    let (gl, gr) = (1.0 - pan, 1.0 + pan);
    let end = (start + len).min(y.len());
    for i in start.min(end)..end {
        let v = amp(i - start) * s;
        y.l[i] += v * gl;
        y.r[i] += v * gr;
        let nc = c * cw - s * sw;
        s = s * cw + c * sw;
        c = nc;
    }
}

/// A plucked note: three partials, a 4 ms attack and a ringing decay (`decay` s to 1/e).
fn pluck(y: &mut St, t: f64, f: f64, amp: f64, decay: f64, pan: f64) {
    let start = at(t);
    let len = at(decay * 4.5);
    let attack = at(0.004).max(1);
    for (mult, rel, dk) in [(1.0, 1.0, 1.0), (2.0, 0.3, 0.5), (3.01, 0.12, 0.3)] {
        let k = (-1.0 / (decay * dk * SR)).exp();
        let mut a = amp * rel;
        ring(y, start, len, f * mult, pan, |i| {
            let v = a * if i < attack { i as f64 / attack as f64 } else { 1.0 };
            a *= k;
            v
        });
    }
}

/// A held note from `t` for `hold` s: it rises over `rise`, falls over `fall` after the hold,
/// with partials at the levels `harm` and two voices `cents` apart.
#[allow(clippy::too_many_arguments)]
fn held(y: &mut St, t: f64, hold: f64, f: f64, amp: f64, rise: f64, fall: f64, harm: &[f64], cents: f64, pan: f64) {
    let start = at(t);
    let nh = at(hold);
    let nf = at(fall).max(1);
    let env: Vec<f64> = (0..nh + nf)
        .map(|i| {
            let up = dsp::rise(i as f64 / SR, rise);
            let down = if i < nh { 1.0 } else { 0.5 + 0.5 * (PI * ((i - nh) as f64 / nf as f64).min(1.0)).cos() };
            up * down
        })
        .collect();
    let spread = 2f64.powf(cents / 1200.0);
    for (k, &h) in harm.iter().enumerate() {
        let m = (k + 1) as f64;
        for (fv, p) in [(f * m * spread, pan - 0.15), (f * m / spread, pan + 0.15)] {
            ring(y, start, env.len(), fv, p.clamp(-1.0, 1.0), |i| amp * h * 0.5 * env[i]);
        }
    }
}

/// A ping-pong echo: each channel hears the other `delay` s earlier, `feedback` down.
fn echo(y: &mut St, delay: f64, feedback: f64) {
    let d = at(delay).max(1);
    for i in d..y.len() {
        let (l, r) = (y.r[i - d], y.l[i - d]);
        y.l[i] += feedback * l;
        y.r[i] += feedback * r;
    }
}

/// A short tick of bright noise (a closed hat).
fn tick(y: &mut St, r: &mut Rng, t: f64, amp: f64, pan: f64) {
    let start = at(t);
    let mut prev = 0.0;
    for i in 0..at(0.03) {
        let Some(idx) = Some(start + i).filter(|k| *k < y.len()) else { break };
        let w = r.uniform(-1.0, 1.0);
        let v = amp * (w - prev) * (-(i as f64) / (0.006 * SR)).exp();
        prev = w;
        y.l[idx] += v * (1.0 - pan);
        y.r[idx] += v * (1.0 + pan);
    }
}

/// A short tone gliding from `f0` to `f1` (a bird's chirp, a cricket's syllable).
fn chirp(y: &mut St, t: f64, dur: f64, f0: f64, f1: f64, amp: f64, pan: f64) {
    let start = at(t);
    let len = at(dur).max(2);
    let mut ph = 0.0;
    for i in 0..len {
        let Some(idx) = Some(start + i).filter(|k| *k < y.len()) else { break };
        let u = i as f64 / len as f64;
        ph += TAU * (f0 + (f1 - f0) * u) / SR;
        let v = ph.sin() * (PI * u).sin().powi(2) * amp;
        y.l[idx] += v * (1.0 - pan);
        y.r[idx] += v * (1.0 + pan);
    }
}

// ------------------------------------------------------------------------------- harmony

// chords in C, a bar each (MIDI notes, low to high)
const C: [f64; 4] = [60.0, 64.0, 67.0, 72.0];
const DM: [f64; 4] = [62.0, 65.0, 69.0, 74.0];
const EM: [f64; 4] = [52.0, 55.0, 59.0, 64.0];
const F: [f64; 4] = [53.0, 57.0, 60.0, 65.0];
const G: [f64; 4] = [55.0, 59.0, 62.0, 67.0];
const AM: [f64; 4] = [57.0, 60.0, 64.0, 69.0];
const MAJOR: [[[f64; 4]; 4]; 4] = [[C, AM, F, G], [C, G, AM, F], [F, G, EM, AM], [C, F, AM, G]];
const MINOR: [[[f64; 4]; 4]; 3] = [[AM, F, C, G], [AM, DM, EM, AM], [AM, F, G, EM]];
/// Keys a piece may move to (semitones from C).
const KEYS: [f64; 4] = [0.0, 2.0, -2.0, -3.0];

/// A chord's root in the bass, between A1 and G#2.
fn bass(root: f64) -> f64 {
    let mut m = root;
    while m >= 45.0 {
        m -= 12.0;
    }
    while m < 33.0 {
        m += 12.0;
    }
    m
}

fn moved(chord: [f64; 4], key: f64) -> [f64; 4] {
    chord.map(|m| m + key)
}

// ------------------------------------------------------------------------------- music

fn musicbox(r: &mut Rng, len: f64) -> St {
    let key = pick(r, &KEYS);
    let prog = pick(r, &MAJOR);
    let pattern = pick(r, &[[0usize, 1, 2, 3, 2, 1, 2, 3], [0, 2, 1, 3, 0, 2, 1, 3], [0, 1, 2, 1, 3, 1, 2, 1]]);
    let mut tune = r.fork();
    let mut y = silent(at(len) + 1);
    let eighth = BEAT / 2.0;
    let bars = ((len - 2.5) / BAR).floor().max(1.0) as usize;
    for b in 0..bars {
        let ch = moved(prog[b % 4], key);
        let t0 = b as f64 * BAR;
        pluck(&mut y, t0, hz(bass(ch[0])), 0.26, 1.4, 0.0);
        pluck(&mut y, t0 + BAR / 2.0, hz(bass(ch[0]) + 12.0), 0.16, 0.9, 0.0);
        for (k, &i) in pattern.iter().enumerate() {
            let pan = if k % 2 == 0 { -0.25 } else { 0.25 };
            pluck(&mut y, t0 + k as f64 * eighth, hz(ch[i] + 12.0), if k % 2 == 0 { 0.13 } else { 0.1 }, 0.45, pan);
        }
        // a short tune on top every other bar
        if b % 2 == 1 {
            for _ in 0..2 {
                let slot = (tune.unit() * 8.0) as usize % 8;
                let note = ch[(tune.unit() * 4.0) as usize % 4] + 24.0;
                let pan = tune.uniform(-0.15, 0.15);
                pluck(&mut y, t0 + slot as f64 * eighth, hz(note), 0.065, 0.9, pan);
            }
        }
        // a quiet pad of the chord, swelling in each bar
        for m in &ch[..3] {
            held(&mut y, t0, BAR, hz(*m), 0.018, 0.8, 0.9, &[1.0], 0.0, 0.0);
        }
    }
    // it ends on the chord it began with
    let end = bars as f64 * BAR;
    let home = moved(prog[0], key);
    for (k, m) in home.iter().enumerate() {
        pluck(&mut y, end + k as f64 * 0.09, hz(m + 12.0), 0.1, 1.2, 0.0);
    }
    pluck(&mut y, end, hz(bass(home[0])), 0.2, 2.0, 0.0);
    echo(&mut y, 0.375, 0.22);
    y
}

fn pad(r: &mut Rng, len: f64) -> St {
    let key = pick(r, &KEYS);
    let prog = pick(r, &MAJOR);
    let mut shimmer = r.fork();
    let mut y = silent(at(len) + 1);
    let span = 2.0 * BAR; // a chord every two bars
    let chords = (len / span).ceil() as usize;
    for k in 0..chords {
        let ch = moved(prog[k % 4], key);
        let t0 = k as f64 * span;
        for (j, m) in ch.iter().enumerate() {
            let pan = [-0.35, 0.35, -0.15, 0.15][j];
            held(&mut y, t0, span, hz(m - 12.0), 0.05, 1.6, 2.2, &[1.0, 0.45, 0.2, 0.1], 6.0, pan);
        }
        held(&mut y, t0, span, hz(bass(ch[0])), 0.07, 1.0, 1.6, &[1.0, 0.25], 0.0, 0.0);
        // now and then a high note rings over it
        if k % 2 == 1 {
            let m = ch[1 + (shimmer.unit() * 3.0) as usize % 3] + 12.0;
            let (t, pan) = (t0 + shimmer.uniform(0.5, 2.5), shimmer.uniform(-0.3, 0.3));
            pluck(&mut y, t, hz(m), 0.035, 1.6, pan);
        }
    }
    y
}

fn pulse(r: &mut Rng, len: f64) -> St {
    let key = pick(r, &KEYS);
    let prog = pick(r, &MINOR);
    let mut ticks = r.fork();
    let mut y = silent(at(len) + 1);
    let eighth = BEAT / 2.0;
    let bars = (len / BAR).ceil() as usize;
    for b in 0..bars {
        let ch = moved(prog[b % 4], key);
        let t0 = b as f64 * BAR;
        let lift = 0.55 + 0.45 * (t0 / len).min(1.0); // it climbs toward the end
        let root = bass(ch[0]);
        pluck(&mut y, t0, hz(root), 0.18 * lift, 0.6, 0.0);
        for k in 0..8 {
            let t = t0 + k as f64 * eighth;
            let up = if k % 4 == 3 { 12.0 } else { 0.0 };
            let accent = if k % 4 == 0 { 1.0 } else { 0.7 };
            pluck(&mut y, t, hz(root + 12.0 + up), 0.2 * accent * lift, 0.16, 0.0);
            if b >= 2 && k % 2 == 1 {
                tick(&mut y, &mut ticks, t, 0.05 * lift, if k % 4 == 1 { -0.2 } else { 0.2 });
            }
        }
        for (j, m) in ch[..3].iter().enumerate() {
            held(&mut y, t0, BAR, hz(m - 12.0), 0.028 * lift, 0.25, 0.5, &[1.0, 0.5, 0.25, 0.12], 8.0, [-0.3, 0.3, 0.0][j]);
        }
    }
    y
}

fn drone(r: &mut Rng, len: f64) -> St {
    let base = pick(r, &[55.0, 49.0, 51.91, 46.25]);
    let period = r.uniform(6.0, 9.0);
    let ph0 = r.uniform(0.0, TAU);
    let n = at(len) + 1;
    let air_l = scaled(dsp::lowpass(&dsp::white(n, r), 150.0, 2), 0.06);
    let air_r = scaled(dsp::lowpass(&dsp::white(n, r), 150.0, 2), 0.06);
    let mut y = silent(n);
    for i in 0..n {
        let t = i as f64 / SR;
        let build = (t / len).min(1.0); // it slowly rises toward the title
        let swell = 0.75 + 0.25 * (TAU * t / period + ph0).sin();
        let tone = 0.16 * (TAU * base * t).sin() + 0.08 * (TAU * base * 1.498 * t).sin() + 0.05 * (TAU * base * 2.006 * t).sin();
        // a slow heartbeat that tightens as the teaser escalates
        let beat_period = 1.6 - 0.6 * build;
        let p = t % beat_period;
        let beat = if t > 4.0 { 0.3 * dsp::rise(t - 4.0, 2.0) * (TAU * 48.0 * p).sin() * (-p * 9.0).exp() } else { 0.0 };
        let body = swell * tone * (0.7 + 0.5 * build) + beat;
        let air = swell * (0.6 + 0.6 * build);
        y.l[i] = body + air * air_l[i] + 0.016 * (TAU * base * 1.011 * t).sin();
        y.r[i] = body + air * air_r[i] + 0.016 * (TAU * base * 0.989 * t).sin();
    }
    y
}

// ------------------------------------------------------------------------------- the air

fn breeze(r: &mut Rng, len: f64) -> St {
    let n = at(len + FOLD);
    let (g1, g2) = (r.uniform(0.0, TAU), r.uniform(0.0, TAU));
    let wind_l = scaled(dsp::lowpass(&dsp::white(n, r), 90.0, 1), 0.05);
    let wind_r = scaled(dsp::lowpass(&dsp::white(n, r), 90.0, 1), 0.05);
    let leaves = scaled(dsp::bandpass(&dsp::white(n, r), 1500.0, 5000.0, 1), 0.012);
    let rustle = dsp::lfnoise(n, 0.6, r);
    let mut y = silent(n);
    for i in 0..n {
        let t = i as f64 / SR;
        let gust = 0.55 + 0.3 * (TAU * t * 2.0 / len + g1).sin() + 0.15 * (TAU * t * 5.0 / len + g2).sin();
        let lv = leaves[i] * (0.5 + 0.5 * rustle[i].tanh());
        y.l[i] = wind_l[i] * 1.4 * gust + lv;
        y.r[i] = wind_r[i] * 1.4 * gust + lv * 0.9;
    }
    // birds: short rising or falling chirps in ones, twos and threes
    let mut t = r.uniform(0.4, 1.6);
    let mut k = 0usize;
    while t < len - 0.6 {
        let calls = 1 + (r.unit() * 3.0) as usize % 3;
        let up = r.unit() < 0.5;
        let pan = r.uniform(-0.5, 0.5);
        let (lo, hi) = (r.uniform(2900.0, 3400.0), r.uniform(4100.0, 4600.0));
        let (f0, f1) = if up { (lo, hi) } else { (hi, lo) };
        for c in 0..calls {
            let amp = 0.09 * r.uniform(0.6, 1.0);
            chirp(&mut y, t + c as f64 * 0.22, 0.07 + 0.03 * ((k + c) % 3) as f64, f0, f1, amp, pan);
        }
        k += calls;
        t += r.uniform(1.2, 3.4);
    }
    y
}

fn wind(r: &mut Rng, len: f64) -> St {
    let n = at(len + FOLD);
    let gust = dsp::lfnoise(n, 0.12, r);
    let drift = dsp::lfnoise(n, 0.08, r);
    let body_l = scaled(dsp::lowpass(&dsp::white(n, r), 140.0, 2), 0.05);
    let body_r = scaled(dsp::lowpass(&dsp::white(n, r), 140.0, 2), 0.05);
    let hiss = dsp::white(n, r);
    let pitch = r.uniform(520.0, 820.0);
    let whistle = scaled(dsp::svf(&hiss, |i| pitch * (1.0 + 0.25 * drift[i].tanh()), 12.0, Mode::Band), 0.012);
    let mut y = silent(n);
    for i in 0..n {
        let g = 0.6 + 0.4 * gust[i].tanh();
        y.l[i] = body_l[i] * 1.5 * g + whistle[i] * g * g;
        y.r[i] = body_r[i] * 1.5 * g + whistle[i] * g * g * 0.8;
    }
    y
}

fn room(r: &mut Rng, len: f64) -> St {
    let n = at(len + FOLD);
    let mains = pick(r, &[50.0, 60.0]);
    let vent_l = scaled(dsp::lowpass(&dsp::pink(n, r), 900.0, 2), 0.04);
    let vent_r = scaled(dsp::lowpass(&dsp::pink(n, r), 900.0, 2), 0.04);
    let drift = dsp::lfnoise(n, 0.2, r);
    let mut y = silent(n);
    for i in 0..n {
        let t = i as f64 / SR;
        let hum = 0.006 * (TAU * mains * t).sin() + 0.004 * (TAU * 2.0 * mains * t).sin() + 0.0025 * (TAU * 3.0 * mains * t).sin();
        let g = 1.0 + 0.12 * drift[i].tanh();
        y.l[i] = vent_l[i] * g + hum;
        y.r[i] = vent_r[i] * g + hum;
    }
    y
}

fn crickets(r: &mut Rng, len: f64) -> St {
    let n = at(len + FOLD);
    let mut y = St {
        l: scaled(dsp::lowpass(&dsp::white(n, r), 110.0, 1), 0.03),
        r: scaled(dsp::lowpass(&dsp::white(n, r), 110.0, 1), 0.03),
    };
    let voices = 3 + (r.unit() * 3.0) as usize % 3;
    for v in 0..voices {
        let mut c = r.fork();
        let f = c.uniform(4200.0, 5200.0);
        let pan = c.uniform(-0.6, 0.6);
        // the first two are near, the rest far off
        let amp = if v < 2 { c.uniform(0.04, 0.07) } else { c.uniform(0.015, 0.03) };
        let period = c.uniform(0.55, 0.95);
        let syllables = 3 + (c.unit() * 2.0) as usize % 2;
        let rate = c.uniform(28.0, 34.0);
        let mut t = c.uniform(0.0, period);
        while t < len + FOLD - 0.2 {
            for s in 0..syllables {
                chirp(&mut y, t + s as f64 / rate, 0.015, f, f * 1.01, amp, pan);
            }
            t += period * c.uniform(0.92, 1.08);
        }
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bed_renders_its_length_and_the_air_loops_without_a_seam() {
        for bed in catalogue() {
            let y = render(&bed, 7, Some(12.0));
            assert_eq!(y.len(), at(12.0), "{}", bed.id);
            assert!(y.l.iter().chain(&y.r).all(|v| v.is_finite()), "{}", bed.id);
            assert!((0.85..=0.9).contains(&y.peak()), "{}: peak {}", bed.id, y.peak());
            let steps: Vec<f64> = y.l.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
            let mean = steps.iter().sum::<f64>() / steps.len() as f64;
            if bed.loops() {
                // from its last sample back to its first is a step like any other
                let seam = (y.l[0] - y.l[y.len() - 1]).abs().max((y.r[0] - y.r[y.len() - 1]).abs());
                assert!(seam < 8.0 * mean + 1e-4, "{}: seam {seam}, mean step {mean}", bed.id);
            } else {
                // music ends quiet, on its last chord
                assert!(y.l[y.len() - 1].abs() < 0.01 && y.r[y.len() - 1].abs() < 0.01, "{}", bed.id);
                let head = dsp::rms(&y.l[..at(4.0)]);
                assert!(head > 0.02, "{}: it plays from the start ({head})", bed.id);
            }
            // the same seed gives the same take, another seed another
            assert_eq!(y.l, render(&bed, 7, Some(12.0)).l, "{}", bed.id);
            assert_ne!(y.l, render(&bed, 8, Some(12.0)).l, "{}", bed.id);
        }
    }
}
