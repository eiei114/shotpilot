//! The sounds shotpilot makes in code (D-103): a catalogue of effects for teasers, each a recipe
//! of physical or designed synthesis with named presets. The sub-drop, reverse-cymbal and
//! struck-body (knock) models and their preset values are adapted from MIT-licensed code; the
//! notice is in THIRD_PARTY_NOTICES.md.

use std::f64::consts::{PI, TAU};

use super::dsp::{self, db, n_of, t_of, Mode, Rng, St, SR};

/// Where a take is placed from: its onset (hits, knocks, drops) or its loudest moment (builds,
/// swells, passes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Onset,
    Peak,
}

/// One sound of the catalogue.
pub struct Spec {
    pub id: &'static str,
    pub kinds: &'static [&'static str],
    pub note: &'static str,
    pub align: Align,
    /// The pitches a `pitch` knob may ask for (Hz).
    pub pitch: Option<(f64, f64)>,
    pub recipe: Recipe,
}

pub enum Recipe {
    Impact(Impact),
    SubDrop(SubDrop),
    Riser(Riser),
    Swell(ReverseSwell),
    Whoosh(Whoosh),
    Fall(Downlifter),
    Knock(Knock),
    Shutter(Shutter),
    Drip(Drip),
    Mallet(Mallet),
    Glitch(Glitch),
}

impl Spec {
    /// Its own length in seconds (about).
    pub fn length(&self) -> f64 {
        match &self.recipe {
            Recipe::Impact(p) => p.pre + p.length,
            Recipe::SubDrop(p) => p.length.min(p.bloom + p.hold + 70.0 / p.decay_db).max(0.3),
            Recipe::Riser(p) => p.length,
            Recipe::Swell(p) => p.length + p.hold + p.tail_tau * 6.9 + 0.02,
            Recipe::Whoosh(p) => p.length,
            Recipe::Fall(p) => p.length,
            Recipe::Knock(p) => p.span(),
            Recipe::Shutter(p) => p.length,
            Recipe::Drip(p) => p.length,
            Recipe::Mallet(p) => p.length,
            Recipe::Glitch(p) => p.length,
        }
    }

    /// The pitch it sounds at, for a `pitch` knob to move from.
    pub fn native_hz(&self) -> Option<f64> {
        self.pitch?;
        match &self.recipe {
            Recipe::Impact(p) => Some(p.body_f1),
            Recipe::SubDrop(p) => Some(p.f1),
            Recipe::Riser(p) => Some(p.tf0),
            Recipe::Fall(p) => Some(p.sub_f1),
            Recipe::Knock(p) => Some(p.f1),
            Recipe::Drip(p) => Some(p.f0),
            Recipe::Mallet(p) => Some(p.f0),
            _ => None,
        }
    }

    /// Whether `dur` sets its length (a build, a pass) rather than trimming its tail.
    pub fn shaped_by_length(&self) -> bool {
        matches!(self.recipe, Recipe::Riser(_) | Recipe::Swell(_) | Recipe::Whoosh(_) | Recipe::Fall(_) | Recipe::SubDrop(_))
    }
}

/// Render one take: stereo at 48 kHz, peak -1 dBFS.
pub fn render(spec: &Spec, seed: u64, dur: Option<f64>, hz: Option<f64>) -> St {
    let mut r = Rng::new(seed);
    let ratio = hz.zip(spec.native_hz()).map(|(want, own)| want / own);
    let shaped = if spec.shaped_by_length() { dur } else { None };
    let mut y = match &spec.recipe {
        Recipe::Impact(p) => p.render(&mut r, ratio),
        Recipe::SubDrop(p) => p.render(&mut r, shaped, ratio),
        Recipe::Riser(p) => p.render(&mut r, shaped, ratio),
        Recipe::Swell(p) => p.render(&mut r, shaped),
        Recipe::Whoosh(p) => p.render(&mut r, shaped),
        Recipe::Fall(p) => p.render(&mut r, shaped, ratio),
        Recipe::Knock(p) => p.render(&mut r, ratio),
        Recipe::Shutter(p) => p.render(&mut r),
        Recipe::Drip(p) => p.render(&mut r, ratio),
        Recipe::Mallet(p) => p.render(&mut r, ratio),
        Recipe::Glitch(p) => p.render(&mut r),
    };
    // a long tail asked to be shorter is faded out to the length (a quarter of it, at most 0.3 s)
    if let Some(d) = dur.filter(|_| !spec.shaped_by_length()) {
        let n = n_of(d);
        if n < y.len() {
            y.truncate(n);
            y.fade(0.0, (0.25 * d).min(0.3));
        } else {
            let pad = n - y.len();
            y.l.extend(std::iter::repeat(0.0).take(pad));
            y.r.extend(std::iter::repeat(0.0).take(pad));
        }
    }
    y.normalize();
    y
}

// ------------------------------------------------------------------------------- shared parts

/// A tone gliding from `f0` toward `f1` with time constant `tau` (s).
fn glide(n: usize, f0: f64, f1: f64, tau: f64, start: f64) -> Vec<f64> {
    dsp::phase(n, |i| f1 + (f0 - f1) * (-t_of(i) / tau.max(1e-4)).exp(), start).into_iter().map(f64::sin).collect()
}

/// Damped modes (a body ringing): pitch starts `bend` above and settles in `bend_tau`.
#[allow(clippy::too_many_arguments)]
fn modes(n: usize, r: &mut Rng, freqs: &[f64], amps: &[f64], taus: &[f64], jitter: f64, bend: f64, bend_tau: f64) -> Vec<f64> {
    let mut y = vec![0.0; n];
    for ((&f, &a), &tau) in freqs.iter().zip(amps).zip(taus) {
        if f >= 0.42 * SR || a == 0.0 {
            continue;
        }
        let f = f * (1.0 + jitter * r.normal());
        let mut ph = r.uniform(0.0, TAU);
        for (i, v) in y.iter_mut().enumerate() {
            let t = t_of(i);
            *v += a * (-t / tau.max(1e-5)).exp() * ph.sin();
            ph += TAU * f * (1.0 + bend * (-t / bend_tau.max(1e-5)).exp()) / SR;
        }
    }
    y
}

/// Drive a resonance with a contact: a raised-cosine force pulse of `contact` s, with grit.
fn strike(y: &[f64], r: &mut Rng, contact: f64, grit: f64) -> Vec<f64> {
    let m = n_of(contact).max(2);
    let pulse: Vec<f64> = (0..m)
        .map(|i| {
            let p = 0.5 - 0.5 * (TAU * i as f64 / (m - 1) as f64).cos();
            p + grit * r.normal() * p
        })
        .collect();
    let mut out = dsp::convolve(y, &pulse);
    out.truncate(y.len());
    out
}

/// A burst of band-limited noise dying with time constant `tau`, unit peak.
fn burst(n: usize, r: &mut Rng, lo: f64, hi: f64, tau: f64, rise: f64) -> Vec<f64> {
    let w = dsp::bandpass(&dsp::white(n, r), lo, hi, 2);
    dsp::unit_peak(w.iter().enumerate().map(|(i, v)| v * (-t_of(i) / tau).exp() * (1.0 - (-t_of(i) / rise.max(1e-5)).exp())).collect())
}

/// A low sine thump, unit peak.
fn thump(n: usize, f: f64, tau: f64, rise: f64) -> Vec<f64> {
    dsp::unit_peak((0..n).map(|i| {
        let t = t_of(i);
        (TAU * f * t).sin() * (-t / tau).exp() * (1.0 - (-t / rise).exp())
    }).collect())
}

/// High-pass, low-pass, a small room and a little width; cut at -60 dB.
#[allow(clippy::too_many_arguments)]
fn finish(y: &[f64], r: &mut Rng, hp: f64, lp: f64, wet: f64, rt60: f64, width: f64) -> St {
    let y = dsp::lowpass(&dsp::highpass(y, hp, 2), lp, 2);
    let st = dsp::widen(&y, r, width);
    let mut st = dsp::reverb(&st, r, rt60, 0.5, wet, 0.6, 0.0);
    st.trim_tail(-60.0, 0.006);
    st.fade(0.0005, 0.006);
    st
}

// ------------------------------------------------------------------------------- impact

/// The cinematic hit: a pitch-dropping membrane, a sub, the crack and sheen of the strike, a
/// blast of air, a granular rumble that holds then falls, an optional struck plate and pre-swell,
/// all in a hall.
#[derive(Clone, Copy)]
pub struct Impact {
    pub length: f64,
    pub body: f64,
    pub body_f0: f64,
    pub body_f1: f64,
    pub drop_tau: f64,
    pub body_tau: f64,
    pub body_attack: f64,
    pub modes: f64,
    pub drive: f64,
    pub sub: f64,
    pub sub_f: f64,
    pub sub_semis: f64,
    pub sub_tau: f64,
    pub crack: f64,
    pub crack_f: f64,
    pub crack_ms: f64,
    pub sheen: f64,
    pub sheen_f: f64,
    pub sheen_ms: f64,
    pub blast: f64,
    pub blast_f: f64,
    pub blast_tau: f64,
    pub roll: f64,
    pub roll_cut: f64,
    pub roll_hold: f64,
    pub roll_fall: f64,
    pub roll_depth: f64,
    pub shell: f64,
    pub shell_lo: f64,
    pub shell_tau: f64,
    pub pre: f64,
    pub pre_level: f64,
    pub hall_rt: f64,
    pub hall_wet: f64,
    pub hall_damp: f64,
}

/// Mode ratios of a struck membrane.
const MEMBRANE: [f64; 6] = [1.0, 1.594, 2.136, 2.296, 2.653, 2.918];

const IMPACT: Impact = Impact {
    length: 4.8, body: 1.0, body_f0: 150.0, body_f1: 52.0, drop_tau: 0.06, body_tau: 0.32, body_attack: 0.003,
    modes: 0.3, drive: 0.35, sub: 0.55, sub_f: 40.0, sub_semis: 7.0, sub_tau: 0.7, crack: 0.45, crack_f: 2500.0,
    crack_ms: 9.0, sheen: 0.12, sheen_f: 6000.0, sheen_ms: 7.0, blast: 0.5, blast_f: 1200.0, blast_tau: 0.25,
    roll: 0.4, roll_cut: 200.0, roll_hold: 1.6, roll_fall: 15.0, roll_depth: 0.3, shell: 0.0, shell_lo: 260.0,
    shell_tau: 1.2, pre: 0.0, pre_level: 0.35, hall_rt: 3.0, hall_wet: 0.3, hall_damp: 0.5,
};

impl Impact {
    fn render(&self, r: &mut Rng, ratio: Option<f64>) -> St {
        let mut p = *self;
        if let Some(k) = ratio {
            p.body_f0 *= k;
            p.body_f1 *= k;
            p.sub_f *= k;
        }
        let n = n_of(p.length);
        let a = p.body_attack;
        // the membrane and its overtones, swelling over body_attack, then saturated
        let mut body = vec![0.0; n];
        for (j, q) in MEMBRANE.iter().enumerate() {
            let amp = if j == 0 { 1.0 } else { p.modes * 0.75f64.powi(j as i32 - 1) * r.uniform(0.6, 1.0) };
            let tau = if j == 0 { p.body_tau } else { p.body_tau / (1.5 + 0.8 * (j - 1) as f64) };
            let tone = glide(n, p.body_f0 * q, p.body_f1 * q, p.drop_tau, r.uniform(0.0, TAU));
            for (i, v) in body.iter_mut().enumerate() {
                let t = t_of(i);
                *v += amp * tone[i] * (-(t - a).max(0.0) / tau).exp() * dsp::rise(t, a);
            }
        }
        let body = dsp::unit_peak(dsp::lowpass(&dsp::saturate(&body, p.drive), (900.0 + 2500.0 * (1.0 - p.drive)).min(2700.0), 2));
        // the dry sub falling onto sub_f
        let sa = a.max(0.008);
        let sub_tone = glide(n, p.sub_f * 2f64.powf(p.sub_semis / 12.0), p.sub_f, 0.09, 0.0);
        let sub: Vec<f64> = (0..n).map(|i| {
            let t = t_of(i);
            sub_tone[i] * dsp::rise(t, sa) * (-(t - sa).max(0.0) / p.sub_tau).exp()
        }).collect();
        // the strike: a crack, a short bright sheen, a blast of air
        let crack = burst(n_of(0.12).min(n), &mut r.fork(), p.crack_f * 0.6, p.crack_f * 3.0, p.crack_ms / 1000.0, 0.0002);
        let sheen = burst(n_of(0.08).min(n), &mut r.fork(), p.sheen_f, 16000.0, p.sheen_ms / 1000.0, 0.0002);
        let blast_n = dsp::lowpass(&dsp::white(n, &mut r.fork()), p.blast_f, 2);
        let blast = dsp::unit_peak(blast_n.iter().enumerate().map(|(i, v)| {
            let t = t_of(i);
            v * (-t / p.blast_tau).exp() * (1.0 - (-t / 0.001).exp())
        }).collect());
        // the rumble: dark noise, granular, holding then falling straight in dB
        let grains = dsp::lfnoise(n, 7.0, &mut r.fork());
        let rumble_n = dsp::lowpass(&dsp::brown(n, &mut r.fork()), p.roll_cut, 2);
        let rumble = dsp::unit_peak(rumble_n.iter().enumerate().map(|(i, v)| {
            let t = t_of(i);
            let level = if t < p.roll_hold { -6.0 * t / p.roll_hold } else { -6.0 - p.roll_fall * (t - p.roll_hold) };
            v * dsp::rise(t, 0.05) * db(level) * (1.0 + p.roll_depth * grains[i]).max(0.1)
        }).collect());
        // a struck plate (metal hits)
        let shell = if p.shell > 0.01 {
            let mut rs = r.fork();
            let f: Vec<f64> = (0..24).map(|_| p.shell_lo * 14f64.powf(rs.unit())).collect();
            let am: Vec<f64> = f.iter().map(|x| db(-3.0 * (x / p.shell_lo).log2()) * rs.uniform(0.3, 1.0)).collect();
            let ta: Vec<f64> = f.iter().map(|x| p.shell_tau * (x / p.shell_lo).powf(-0.5)).collect();
            dsp::unit_peak(strike(&modes(n, &mut rs, &f, &am, &ta, 0.0, 0.0, 0.004), &mut rs, 0.0008, 0.3))
        } else {
            vec![0.0; n]
        };
        let mut hit = vec![0.0; n];
        for (layer, w) in [(&body, p.body), (&sub, p.sub), (&blast, p.blast), (&rumble, p.roll), (&shell, p.shell)] {
            dsp::place(&mut hit, layer, 0, w);
        }
        dsp::place(&mut hit, &crack, 0, p.crack);
        dsp::place(&mut hit, &sheen, 0, p.sheen);
        // a swell rising into the hit
        let pre_n = if p.pre > 0.01 { n_of(p.pre) } else { 0 };
        let mut y = vec![0.0; pre_n + n];
        if pre_n > 0 {
            let w = dsp::white(pre_n, &mut r.fork());
            let up = dsp::svf(&w, |i| 200.0 * 20f64.powf((i as f64 / pre_n as f64).powf(1.5)), 0.7, Mode::Low);
            let swell: Vec<f64> = dsp::unit_rms(up).iter().enumerate()
                .map(|(i, v)| v * db(-30.0 * (1.0 - i as f64 / pre_n as f64).powf(1.5)))
                .collect();
            dsp::place(&mut y, &dsp::unit_peak(swell), 0, p.pre_level);
        }
        dsp::place(&mut y, &hit, pre_n, 1.0);
        let st = dsp::widen(&y, r, 0.25);
        let mut st = dsp::reverb(&st, r, p.hall_rt, p.hall_damp, p.hall_wet, 1.0, 120.0);
        st.truncate(pre_n + n);
        st.trim_tail(-60.0, 0.05);
        st.fade(0.0005, 0.2);
        st
    }
}

// ------------------------------------------------------------------------------- sub drop

/// A sine gliding down into the sub, blooming to its peak and decaying straight in dB, with
/// harmonics for small speakers, a click and a thump on top.
#[derive(Clone, Copy)]
pub struct SubDrop {
    pub f0: f64,
    pub f1: f64,
    pub glide: f64,
    pub glide_curve: f64,
    pub bloom: f64,
    pub hold: f64,
    pub decay_db: f64,
    pub length: f64,
    pub harm: f64,
    pub harm_lp: f64,
    pub click: f64,
    pub click_f: f64,
    pub click_ms: f64,
    pub thump: f64,
    pub grit: f64,
    pub width: f64,
    pub room: f64,
}

impl SubDrop {
    fn render(&self, r: &mut Rng, dur: Option<f64>, ratio: Option<f64>) -> St {
        let mut p = *self;
        if let Some(d) = dur {
            p.length = d.max(0.3);
            // a longer drop decays slower so it lasts as long as asked
            p.decay_db = p.decay_db.min(70.0 / (p.length - p.bloom - p.hold).max(0.1));
        }
        if let Some(k) = ratio {
            p.f0 *= k;
            p.f1 *= k;
        }
        let life = p.bloom + p.hold + 70.0 / p.decay_db;
        let seconds = p.length.min(life).max(0.3);
        let n = n_of(seconds);
        let path = |i: usize| {
            let s = (t_of(i) / p.glide.max(1e-3)).clamp(0.0, 1.0).powf(p.glide_curve);
            (p.f0.ln() + (p.f1.ln() - p.f0.ln()) * s).exp()
        };
        let ph = dsp::phase(n, path, r.uniform(0.0, 0.2));
        let env: Vec<f64> = (0..n).map(|i| {
            let t = t_of(i);
            if t < p.bloom {
                0.5 - 0.5 * (PI * t / p.bloom).cos()
            } else if t <= p.bloom + p.hold {
                1.0
            } else {
                db(-p.decay_db * (t - p.bloom - p.hold))
            }
        }).collect();
        let sub: Vec<f64> = (0..n).map(|i| ph[i].sin() * env[i]).collect();
        // partials 2-6 of the gliding sub under a roll-off at harm_lp, nothing above 300 Hz
        let mut harm = vec![0.0; n];
        if p.harm > 0.01 {
            for (i, h) in harm.iter_mut().enumerate() {
                let f = path(i);
                for k in 2..7 {
                    let fk = k as f64 * f;
                    let taper = ((300.0 - fk) / 80.0).clamp(0.0, 1.0);
                    *h += (0.6 / k as f64) / (1.0 + (fk / p.harm_lp).powi(4)) * (k as f64 * ph[i] + 0.4 * k as f64).sin() * taper;
                }
                *h *= env[i];
            }
            harm = dsp::unit_peak(harm).iter().map(|v| v * p.harm).collect();
        }
        let mut grit = vec![0.0; n];
        if p.grit > 0.01 {
            let g = dsp::unit_peak(dsp::bandpass(&dsp::brown(n, r), 35.0, 160.0, 2));
            grit = g.iter().zip(&env).map(|(v, e)| 0.3 * p.grit * v * e).collect();
        }
        let mut top = vec![0.0; n];
        if p.click > 0.0 {
            dsp::place(&mut top, &self::click(&p, n, r), 0, p.click);
        }
        if p.thump > 0.0 {
            let k: Vec<f64> = dsp::phase(n, |i| 90.0 + 60.0 * (-t_of(i) / 0.03).exp(), r.uniform(0.0, 0.3))
                .into_iter().enumerate()
                .map(|(i, ph)| ph.sin() * (-t_of(i) / 0.06).exp() * (1.0 - (-t_of(i) / 0.001).exp()))
                .collect();
            dsp::place(&mut top, &k, 0, 0.7 * p.thump);
        }
        if p.room > 0.01 && dsp::peak(&top) > 0.0 {
            top = dsp::reverb(&St::mono(top), r, 0.35, 0.7, p.room, 0.0, 0.0).l;
            top.truncate(n);
        }
        let mono: Vec<f64> = (0..n).map(|i| sub[i] + harm[i] + grit[i] + top[i]).collect();
        // a little width above 150 Hz; the sub stays centred
        let hi = dsp::highpass(&(0..n).map(|i| harm[i] + top[i]).collect::<Vec<_>>(), 150.0, 2);
        let d = n_of(0.0045);
        let side: Vec<f64> = (0..n).map(|i| if i >= d { p.width * hi[i - d] } else { 0.0 }).collect();
        let st = St { l: (0..n).map(|i| mono[i] + side[i]).collect(), r: (0..n).map(|i| mono[i] - side[i]).collect() };
        let mut st = st.map(|c| dsp::highpass(c, 22.0, 2));
        st.fade(0.0015, (0.15 * seconds).max(0.05));
        st
    }
}

/// A noise burst ((1 - t / len)^3) high-passed at click_f / 2, and a short ping at click_f.
fn click(p: &SubDrop, n: usize, r: &mut Rng) -> Vec<f64> {
    let l = n_of(p.click_ms / 1000.0).max(16).min(n);
    let w: Vec<f64> = (0..l).map(|i| {
        let u = i as f64 / l as f64;
        r.normal() * (1.0 - u).powi(3) * (1.0 - (-(i as f64) / (SR * 0.0002)).exp())
    }).collect();
    let w = dsp::unit_peak(dsp::highpass(&w, p.click_f * 0.5, 2));
    let mut y = vec![0.0; n];
    dsp::place(&mut y, &w, 0, 1.0);
    let k = n_of(0.02).min(n);
    for (i, v) in y.iter_mut().take(k).enumerate() {
        let t = t_of(i);
        *v += 0.6 * (TAU * p.click_f * t).sin() * (-t / 0.003).exp() * (1.0 - (-t / 0.0003).exp());
    }
    dsp::unit_peak(y)
}

// ------------------------------------------------------------------------------- riser

/// A build into a reveal that stops on its loudest sample: a noise band climbing, a detuned
/// chord gliding up, an endless Shepard tone, a sub swell, crackle and a speeding stutter, as
/// each preset mixes them.
#[derive(Clone, Copy)]
pub struct Riser {
    pub length: f64,
    pub rise_db: f64,
    pub curve: f64,
    pub noise: f64,
    pub nf0: f64,
    pub nf1: f64,
    pub nbw: f64,
    pub tone: f64,
    pub tf0: f64,
    pub toct: f64,
    pub fifth_db: f64,
    pub detune: f64,
    pub tbright: f64,
    pub shep: f64,
    pub sub: f64,
    pub sub_f0: f64,
    pub sub_f1: f64,
    pub crackle: f64,
    pub gate: f64,
    pub gate_p0: f64,
    pub gate_p1: f64,
    pub hp: f64,
    pub width: f64,
}

const RISER: Riser = Riser {
    length: 2.8, rise_db: 32.0, curve: 2.2, noise: 0.8, nf0: 300.0, nf1: 9000.0, nbw: 1.6, tone: 0.0, tf0: 110.0,
    toct: 2.0, fifth_db: -6.0, detune: 0.012, tbright: 6.0, shep: 0.0, sub: 0.0, sub_f0: 38.0, sub_f1: 70.0,
    crackle: 0.0, gate: 0.0, gate_p0: 0.11, gate_p1: 0.025, hp: 80.0, width: 0.8,
};

impl Riser {
    fn render(&self, r: &mut Rng, dur: Option<f64>, ratio: Option<f64>) -> St {
        let mut p = *self;
        if let Some(d) = dur {
            p.length = d.max(0.2);
        }
        if let Some(k) = ratio {
            p.tf0 *= k;
        }
        let n = n_of(p.length);
        let u = |i: usize| i as f64 / n as f64;
        let w = |i: usize| u(i).powf(p.curve);
        let mut chans = [vec![0.0; n], vec![0.0; n]];
        // the noise band climbing, one draw per channel blended by width
        if p.noise > 0.0 {
            let fc = |i: usize| (p.nf0 * (p.nf1 / p.nf0).powf(w(i))).min(18000.0);
            let band = |x: &[f64]| {
                let lo = dsp::svf(x, fc, 0.7, Mode::Low);
                dsp::unit_rms(dsp::svf(&lo, |i| (fc(i) / 2f64.powf(p.nbw)).max(20.0), 0.7, Mode::High))
            };
            let a = band(&dsp::white(n, r));
            let b = band(&dsp::white(n, r));
            let k = (1.0 - p.width * p.width).max(0.0).sqrt();
            for i in 0..n {
                chans[0][i] += p.noise * a[i];
                chans[1][i] += p.noise * (k * a[i] + p.width * b[i]);
            }
        }
        // the chord: root and fifth, three detuned saws each, gliding up
        if p.tone > 0.0 {
            let f = |i: usize| p.tf0 * 2f64.powf(p.toct * u(i).powf(p.curve.max(1.0) * 0.8));
            for (c, ch) in chans.iter_mut().enumerate() {
                let mut x = vec![0.0; n];
                for (ratio, level) in [(1.0, 1.0), (1.5, db(p.fifth_db))] {
                    for d in [-1.0, 0.0, 1.0] {
                        let det = 1.0 + d * p.detune * (1.0 + 0.3 * c as f64);
                        let s = dsp::saw(n, |i| f(i) * ratio * det, r.unit());
                        dsp::place(&mut x, &s, 0, level / 3.0);
                    }
                }
                let x = dsp::svf(&x, |i| (f(i) * p.tbright).min(18000.0), 0.7, Mode::Low);
                let x = dsp::unit_rms(dsp::highpass(&x, p.hp * 0.8, 2));
                dsp::place(ch, &x, 0, p.tone);
            }
        }
        if p.shep > 0.0 {
            let s = dsp::shepard(n, r, true, 0.5, 40.0, 8000.0, -1.5);
            let st = dsp::widen(&s, r, 0.5);
            dsp::place(&mut chans[0], &st.l, 0, p.shep);
            dsp::place(&mut chans[1], &st.r, 0, p.shep);
        }
        if p.sub > 0.0 {
            let s: Vec<f64> = dsp::phase(n, |i| p.sub_f0 * (p.sub_f1 / p.sub_f0).powf(w(i)), 0.0).into_iter().map(f64::sin).collect();
            let s = dsp::unit_rms(s);
            for ch in chans.iter_mut() {
                dsp::place(ch, &s, 0, p.sub);
            }
        }
        if p.crackle > 0.0 {
            // crackle: tiny windowed grains of bright noise, denser as it builds (no hard edges)
            let mut c = vec![0.0; n];
            let g = n_of(0.0015);
            let mut i = 0;
            while i + g < n {
                if r.unit() < (0.002 + 0.02 * w(i)) * g as f64 {
                    let level = r.uniform(0.3, 1.0);
                    for k in 0..g {
                        c[i + k] += level * r.normal() * (PI * k as f64 / g as f64).sin().powi(2);
                    }
                }
                i += g;
            }
            let c = dsp::unit_rms(dsp::bandpass(&c, 1500.0, 7000.0, 2));
            let st = dsp::widen(&c, r, 0.8);
            dsp::place(&mut chans[0], &st.l, 0, p.crackle);
            dsp::place(&mut chans[1], &st.r, 0, p.crackle);
        }
        // the level climbing rise_db, and the stutter speeding up (open on the last 40 ms)
        let mut gph = 0.0;
        let [l, rr] = &mut chans;
        for (i, (a, b)) in l.iter_mut().zip(rr.iter_mut()).enumerate() {
            let lv = db(-p.rise_db * (1.0 - w(i)));
            let per = p.gate_p0 * (p.gate_p1 / p.gate_p0).powf(u(i).powf(1.4));
            gph += 1.0 / (per * SR);
            let g = 0.5 + 0.5 * (TAU * gph).cos();
            let on = ((u(i) - 0.25) / 0.2).clamp(0.0, 1.0);
            let open = ((n - i) as f64 / n_of(0.04) as f64).clamp(0.0, 1.0);
            let depth = p.gate * on * on * (3.0 - 2.0 * on) * open;
            let gain = lv * (1.0 - depth * (1.0 - g.powf(0.5)));
            *a *= gain;
            *b *= gain;
        }
        let [l, rr] = chans;
        let mut st = St { l, r: rr };
        st.fade(0.01, 0.003);
        st
    }
}

// ------------------------------------------------------------------------------- reverse swell

/// A reverse cymbal or swell built in the order a reversed ring plays: stationary metal (partials
/// and band noise) under a level that climbs to the peak, a plateau, then a stop or a ring-out.
#[derive(Clone, Copy)]
pub struct ReverseSwell {
    pub length: f64,
    pub swell_db: f64,
    pub curve: f64,
    pub hold: f64,
    pub hold_db: f64,
    pub tail_tau: f64,
    pub cym_f: f64,
    pub bw: f64,
    pub metal: f64,
    pub partials: usize,
    pub shimmer: f64,
    pub open: f64,
    pub hp: f64,
    pub low_db: f64,
    pub flut_depth: f64,
    pub flut_f: f64,
    pub width: f64,
    pub room: f64,
    pub rt60: f64,
}

impl ReverseSwell {
    fn texture(&self, n: usize, r: &mut Rng) -> (Vec<f64>, f64) {
        let lo = (self.cym_f / 2f64.powf(self.bw / 2.0)).max(60.0);
        let hi = (self.cym_f * 2f64.powf(self.bw / 2.0)).min(18000.0);
        let sig = (self.bw / 3.0).max(0.3);
        let sh = dsp::lfnoise(n, 3.0, r);
        let mut met = vec![0.0; n];
        for _ in 0..self.partials {
            let f = (lo.ln() + (hi.ln() - lo.ln()) * r.unit()).exp();
            let a = (-0.5 * ((f / self.cym_f).log2() / sig).powi(2)).exp() * r.uniform(0.4, 1.0);
            let off = (r.unit() * n as f64) as usize;
            let mut ph = r.uniform(0.0, TAU);
            let dph = TAU * f / SR;
            for (i, v) in met.iter_mut().enumerate() {
                *v += a * (1.0 + 0.35 * self.shimmer * sh[(i + off) % n]) * ph.sin();
                ph += dph;
            }
        }
        let met = dsp::unit_rms(met);
        let nz = dsp::unit_rms(dsp::peq(&dsp::bandpass(&dsp::white(n, r), lo, hi, 2), self.cym_f, 6.0, 1.0));
        let x: Vec<f64> = met.iter().zip(&nz).map(|(m, z)| self.metal * m + (1.0 - self.metal) * z).collect();
        (dsp::unit_rms(x), hi)
    }

    fn render(&self, r: &mut Rng, dur: Option<f64>) -> St {
        let mut p = *self;
        if let Some(d) = dur {
            p.length = (d - p.hold - 3.0 * p.tail_tau).max(0.15);
        }
        let tail = p.length + p.hold + p.tail_tau * 60.0 / 8.686 + 0.02;
        let n = n_of((tail + 0.3 * p.rt60 * (4.0 * p.room).min(1.0)).min(4.0));
        let level = |t: f64| {
            if t <= p.length {
                -p.swell_db * (1.0 - t / p.length).powf(p.curve)
            } else if t <= p.length + p.hold {
                p.hold_db * (t - p.length) / p.hold.max(1e-6)
            } else {
                p.hold_db - 8.686 * (t - p.length - p.hold) / p.tail_tau
            }
        };
        let a: Vec<f64> = (0..n).map(|i| db(level(t_of(i)).max(-120.0))).collect();
        let (xl, hi) = p.texture(n, r);
        let (xr, _) = p.texture(n, r);
        let low_l = dsp::unit_rms(dsp::bandpass(&dsp::white(n, r), 100.0, 450.0, 2));
        let low_r = dsp::unit_rms(dsp::bandpass(&dsp::white(n, r), 100.0, 450.0, 2));
        let flut = |r: &mut Rng| -> Vec<f64> {
            dsp::lfnoise(n, p.flut_f, r).iter().map(|v| (p.flut_depth * v - 0.5 * p.flut_depth * p.flut_depth).exp()).collect()
        };
        let (f0, f1) = (flut(r), flut(r));
        let k = (1.0 - p.width * p.width).max(0.0).sqrt();
        let mix = |a1: &[f64], b1: &[f64]| -> Vec<f64> { a1.iter().zip(b1).map(|(x, y)| k * x + p.width * y).collect() };
        let cut = |i: usize| (hi * 1.3 * 2f64.powf(-p.open * (1.0 - a[i].sqrt()))).clamp(300.0, 20000.0);
        let chan = |x: &[f64], lo: &[f64], fl: &[f64]| -> Vec<f64> {
            let y = dsp::svf(x, cut, 0.7, Mode::Low);
            (0..n).map(|i| {
                let body = if p.low_db > -49.5 { db(p.low_db) * lo[i] } else { 0.0 };
                (y[i] + body) * a[i] * fl[i]
            }).collect()
        };
        let st = St { l: chan(&xl, &low_l, &f0), r: chan(&mix(&xl, &xr), &mix(&low_l, &low_r), &mix(&f0, &f1)) };
        let st = st.map(|c| dsp::highpass(c, p.hp, 2));
        let mut st = if p.room > 0.01 { dsp::reverb(&st, r, p.rt60, 0.4, p.room, 1.0, 0.0) } else { st };
        st.truncate(n);
        let mut st = st.map(|c| dsp::lowpass(c, 19000.0, 4));
        st.trim_tail(-60.0, 0.02);
        st.fade(0.004, 0.02);
        st
    }
}

// ------------------------------------------------------------------------------- whoosh

/// Moving air: noise through a band that brightens as the thing comes close and darkens as it
/// leaves, panned across, loudest at the apex; optional falling ring and landing thump.
#[derive(Clone, Copy)]
pub struct Whoosh {
    pub length: f64,
    pub apex: f64,
    pub f_lo: f64,
    pub f_hi: f64,
    pub q: f64,
    pub rise_pow: f64,
    pub fall_pow: f64,
    pub pan: f64,
    pub ring: f64,
    pub ring_f: f64,
    pub thump: f64,
    pub room: f64,
}

impl Whoosh {
    fn render(&self, r: &mut Rng, dur: Option<f64>) -> St {
        let mut p = *self;
        if let Some(d) = dur {
            p.apex *= d / p.length;
            p.length = d.max(0.2);
        }
        let n = n_of(p.length);
        let env: Vec<f64> = (0..n).map(|i| {
            let t = t_of(i);
            if t < p.apex {
                (t / p.apex).powf(p.rise_pow)
            } else {
                (1.0 - (t - p.apex) / (p.length - p.apex).max(1e-3)).max(0.0).powf(p.fall_pow)
            }
        }).collect();
        let fc = |i: usize| p.f_lo * (p.f_hi / p.f_lo).powf(env[i].sqrt());
        let pk = dsp::pink(n, r);
        let wh = dsp::white(n, r);
        let air: Vec<f64> = pk.iter().zip(&wh).map(|(a, b)| 0.75 * a + 0.25 * b).collect();
        let band = dsp::unit_rms(dsp::svf(&air, fc, p.q, Mode::Band));
        let body = dsp::unit_rms(dsp::svf(&air, |i| fc(i) * 0.5, 0.6, Mode::Low));
        let mut x: Vec<f64> = (0..n).map(|i| (band[i] + 0.4 * body[i]) * env[i]).collect();
        if p.ring > 0.0 {
            let ph = dsp::phase(n, |i| {
                let t = t_of(i);
                p.ring_f * if t < p.apex { 1.0 } else { 0.6f64.powf((t - p.apex) / (p.length - p.apex).max(1e-3)) }
            }, 0.0);
            let ring = dsp::unit_rms(ph.iter().map(|v| v.sin()).collect());
            dsp::place(&mut x, &ring.iter().zip(&env).map(|(v, e)| v * e * e).collect::<Vec<_>>(), 0, p.ring);
        }
        if p.thump > 0.0 {
            let at = n_of(p.apex + 0.08).min(n.saturating_sub(1));
            let th: Vec<f64> = dsp::phase(n - at, |i| 50.0 + 40.0 * (-t_of(i) / 0.05).exp(), 0.0)
                .into_iter().enumerate().map(|(i, ph)| ph.sin() * (-t_of(i) / 0.12).exp() * dsp::rise(t_of(i), 0.004)).collect();
            let level = p.thump * dsp::peak(&x).max(1e-9);
            dsp::place(&mut x, &th, at, level);
        }
        let span = p.length;
        let st = dsp::pan(&x, |i| p.pan * ((t_of(i) - p.apex) / (0.5 * span)).clamp(-1.0, 1.0));
        let mut st = dsp::reverb(&st, r, 0.6, 0.5, p.room, 0.8, 100.0);
        st.truncate(n + n_of(0.3));
        st.trim_tail(-60.0, 0.03);
        st.fade(0.003, 0.03);
        st
    }
}

// ------------------------------------------------------------------------------- downlifter

/// Energy falling away after a hit: a noise band falling, a sine sinking into the sub.
#[derive(Clone, Copy)]
pub struct Downlifter {
    pub length: f64,
    pub attack: f64,
    pub f_hi: f64,
    pub f_lo: f64,
    pub noise: f64,
    pub sub: f64,
    pub sub_f0: f64,
    pub sub_f1: f64,
    pub fall_pow: f64,
    pub room: f64,
}

impl Downlifter {
    fn render(&self, r: &mut Rng, dur: Option<f64>, ratio: Option<f64>) -> St {
        let mut p = *self;
        if let Some(d) = dur {
            p.length = d.max(0.3);
        }
        if let Some(k) = ratio {
            p.sub_f0 *= k;
            p.sub_f1 *= k;
        }
        let n = n_of(p.length);
        let u = |i: usize| (t_of(i) - p.attack).max(0.0) / (p.length - p.attack).max(1e-3);
        let env: Vec<f64> = (0..n).map(|i| dsp::rise(t_of(i), p.attack) * (1.0 - u(i)).max(0.0).powf(p.fall_pow)).collect();
        let fc = |i: usize| p.f_hi * (p.f_lo / p.f_hi).powf(u(i).powf(0.6));
        let mut chans = Vec::new();
        for _ in 0..2 {
            let x = dsp::unit_rms(dsp::svf(&dsp::white(n, r), fc, 0.9, Mode::Low));
            chans.push(x);
        }
        let s = dsp::unit_rms(dsp::phase(n, |i| p.sub_f0 * (p.sub_f1 / p.sub_f0).powf(u(i).powf(0.7)), 0.0).iter().map(|v| v.sin()).collect());
        let mk = |x: &[f64]| -> Vec<f64> { (0..n).map(|i| (p.noise * x[i] + p.sub * s[i]) * env[i]).collect() };
        let st = St { l: mk(&chans[0]), r: mk(&chans[1]) };
        let mut st = dsp::reverb(&st, r, 1.2, 0.5, p.room, 1.0, 120.0);
        st.truncate(n);
        st.fade(0.002, 0.08);
        st
    }
}

// ------------------------------------------------------------------------------- knock

type Mat = &'static [(f64, f64, f64)];

/// Mode sets (ratio, amplitude, decay multiplier) on the lowest mode.
const BLOCK: Mat = &[(1.0, 1.0, 1.0), (1.48, 0.55, 0.6), (2.15, 0.45, 0.45), (3.4, 0.25, 0.3), (4.6, 0.12, 0.2)];
const TOK: Mat = &[(1.0, 1.0, 1.0), (2.7, 0.15, 0.3), (5.1, 0.05, 0.15)];

/// A struck body: its modes and a cloud of weaker ones, a contact with grit and noise, a softer
/// contact before (two knuckles) or after (a bounce), a desk under it.
#[derive(Clone, Copy)]
pub struct Knock {
    pub mat: Mat,
    pub f1: f64,
    pub decay: f64,
    pub slope: f64,
    pub tilt: f64,
    pub cloud: f64,
    pub cloud_hi: f64,
    pub cloud_dec: f64,
    pub jitter: f64,
    pub bend: f64,
    pub bend_ms: f64,
    pub contact: f64,
    pub grit: f64,
    pub noise: f64,
    pub noise_hz: f64,
    pub noise_ms: f64,
    pub flam: f64,
    pub flam_ms: f64,
    pub bounce: f64,
    pub bounce_ms: f64,
    pub body: f64,
    pub body_f: f64,
    pub body_ms: f64,
    pub hp: f64,
    pub lp: f64,
    pub room: f64,
    pub rt: f64,
    pub width: f64,
}

impl Knock {
    fn ring(&self) -> f64 {
        let mut ring = self.mat.iter().map(|(q, _, dm)| self.decay * dm * q.powf(-self.slope)).fold(0.0, f64::max);
        if self.cloud > -59.0 {
            ring = ring.max(self.decay * self.cloud_dec * 1.1f64.powf(-self.slope).max(self.cloud_hi.powf(-self.slope)));
        }
        ring
    }

    fn span(&self) -> f64 {
        let lead = if self.flam > -59.0 { self.flam_ms / 1000.0 } else { 0.0 };
        let bounce = if self.bounce > -59.0 { self.bounce_ms / 1000.0 } else { 0.0 };
        let body = if self.body > -59.0 { self.body_ms / 1000.0 } else { 0.0 };
        (lead + bounce + 7.0 * self.ring().max(body) + 0.02).min(1.0)
    }

    fn hit(&self, r: &mut Rng, n: usize, contact: f64) -> Vec<f64> {
        let (mut fr, mut am, mut ta) = (Vec::new(), Vec::new(), Vec::new());
        for (q, a, dm) in self.mat {
            fr.push(self.f1 * q);
            am.push(a * q.powf(self.tilt));
            ta.push((self.decay * dm * q.powf(-self.slope)).max(0.0008));
        }
        let mut y = dsp::unit_peak(modes(n, r, &fr, &am, &ta, self.jitter, self.bend, self.bend_ms / 1000.0));
        if self.cloud > -59.0 {
            let mut rc = r.fork();
            let hi = (self.f1 * self.cloud_hi).min(0.4 * SR).max(1.2 * self.f1);
            let fc: Vec<f64> = (0..28).map(|_| ((1.1 * self.f1).ln() + (hi.ln() - (1.1 * self.f1).ln()) * rc.unit()).exp()).collect();
            let tc: Vec<f64> = fc.iter().map(|f| (self.decay * self.cloud_dec * (f / self.f1).powf(-self.slope)).max(0.0008)).collect();
            let ac: Vec<f64> = fc.iter().map(|f| rc.uniform(0.3, 1.0) * (f / self.f1).powf(self.tilt - 0.5)).collect();
            let c = dsp::unit_peak(modes(n, &mut rc, &fc, &ac, &tc, 0.0, self.bend, self.bend_ms / 1000.0));
            dsp::place(&mut y, &c, 0, db(self.cloud));
        }
        let mut y = dsp::unit_peak(strike(&y, r, contact, self.grit));
        if self.noise > -59.0 {
            let b = burst(n, r, self.noise_hz / 2.0, self.noise_hz * 2.0, self.noise_ms / 1000.0, 0.0002);
            dsp::place(&mut y, &b, 0, db(self.noise));
        }
        y
    }

    fn render(&self, r: &mut Rng, ratio: Option<f64>) -> St {
        let mut p = *self;
        let j = [r.normal(), r.normal(), r.normal()];
        p.f1 *= ratio.unwrap_or(1.0) * (0.02 * 0.3 * j[0]).exp();
        p.decay *= (0.1 * 0.3 * j[1]).exp();
        p.contact *= (0.15 * 0.3 * j[2]).exp();
        let n = n_of(p.span());
        let flam = p.flam > -59.0;
        let k = if flam { n_of(p.flam_ms / 1000.0).min(n / 2) } else { 0 };
        let mut y = vec![0.0; n];
        dsp::place(&mut y, &p.hit(r, n - k, p.contact), k, 1.0);
        if flam {
            dsp::place(&mut y, &p.hit(r, n, p.contact * 1.5), 0, db(p.flam));
        }
        if p.bounce > -59.0 {
            let kb = k + n_of(p.bounce_ms / 1000.0);
            if kb + 8 < n {
                dsp::place(&mut y, &p.hit(r, n - kb, p.contact * 1.3), kb, db(p.bounce));
            }
        }
        if p.body > -59.0 {
            let tb = thump(n - k, p.body_f * (1.0 + 0.03 * r.normal()), p.body_ms / 1000.0, 0.001);
            dsp::place(&mut y, &tb, k, db(p.body));
        }
        finish(&y, r, p.hp, p.lp, p.room, p.rt, p.width)
    }
}

const KNOCK: Knock = Knock {
    mat: BLOCK, f1: 620.0, decay: 0.03, slope: 0.6, tilt: 0.0, cloud: -60.0, cloud_hi: 4.0, cloud_dec: 0.8,
    jitter: 0.006, bend: 0.05, bend_ms: 4.0, contact: 0.0012, grit: 0.4, noise: -18.0, noise_hz: 1800.0,
    noise_ms: 1.5, flam: -60.0, flam_ms: 14.0, bounce: -60.0, bounce_ms: 20.0, body: -20.0, body_f: 140.0,
    body_ms: 15.0, hp: 70.0, lp: 8000.0, room: 0.08, rt: 0.12, width: 0.08,
};

// ------------------------------------------------------------------------------- shutter

/// A camera shutter: clicks (small modal pings struck with noise), the curtain's travel between
/// them, the body's thump on the landing.
#[derive(Clone, Copy)]
pub struct Shutter {
    pub length: f64,
    /// (at s, level, ping Hz, decay ms)
    pub clicks: &'static [(f64, f64, f64, f64)],
    pub travel: f64,
    pub body: f64,
    pub body_f: f64,
    pub room: f64,
}

impl Shutter {
    fn render(&self, r: &mut Rng) -> St {
        let n = n_of(self.length);
        let mut y = vec![0.0; n];
        for &(at, level, f, ms) in self.clicks {
            let k = n_of(at).min(n - 1);
            let m = n - k;
            let fr = [f, f * 1.47, f * 2.31, f * 3.2];
            let am = [1.0, 0.6, 0.35, 0.2];
            let ta = [ms / 1000.0, ms * 0.6 / 1000.0, ms * 0.4 / 1000.0, ms * 0.3 / 1000.0];
            let ping = dsp::unit_peak(strike(&modes(m, r, &fr, &am, &ta, 0.01, 0.0, 0.004), r, 0.0003, 0.6));
            let nz = burst(m, r, 2500.0, 12000.0, 0.0006, 0.00005);
            dsp::place(&mut y, &ping, k, level);
            dsp::place(&mut y, &nz, k, 0.5 * level);
        }
        if self.travel > 0.0 && self.clicks.len() > 1 {
            let (a, b) = (n_of(self.clicks[0].0 + 0.003), n_of(self.clicks[1].0));
            if b > a {
                let w = dsp::bandpass(&dsp::white(b - a, r), 800.0, 5000.0, 2);
                let m = w.len();
                let whir: Vec<f64> = w.iter().enumerate().map(|(i, v)| v * (PI * i as f64 / m as f64).sin()).collect();
                dsp::place(&mut y, &dsp::unit_peak(whir), a, self.travel);
            }
        }
        if self.body > 0.0 {
            if let Some(last) = self.clicks.last() {
                let k = n_of(last.0).min(n - 1);
                dsp::place(&mut y, &thump(n - k, self.body_f, 0.025, 0.001), k, self.body);
            }
        }
        finish(&y, r, 120.0, 14000.0, self.room, 0.1, 0.2)
    }
}

// ------------------------------------------------------------------------------- drip

/// A water drop: a bubble whose pitch rises a few semitones within tens of ms as it closes, a soft
/// onset, an optional daughter bubble.
#[derive(Clone, Copy)]
pub struct Drip {
    pub length: f64,
    pub f0: f64,
    pub rise_st: f64,
    pub rise_ms: f64,
    pub tau: f64,
    pub onset_ms: f64,
    pub daughter: f64,
    pub daughter_at: f64,
    pub daughter_k: f64,
    pub room: f64,
}

impl Drip {
    fn bubble(&self, n: usize, f0: f64, r: &mut Rng) -> Vec<f64> {
        let rise = self.rise_ms / 1000.0 / 3.0;
        let ph = dsp::phase(n, |i| f0 * 2f64.powf(self.rise_st / 12.0 * (1.0 - (-t_of(i) / rise).exp())), r.uniform(0.0, TAU));
        dsp::unit_peak((0..n).map(|i| {
            let t = t_of(i);
            ph[i].sin() * (-t / self.tau).exp() * dsp::rise(t, self.onset_ms / 1000.0)
        }).collect())
    }

    fn render(&self, r: &mut Rng, ratio: Option<f64>) -> St {
        let f0 = self.f0 * ratio.unwrap_or(1.0) * (1.0 + 0.02 * r.normal());
        let n = n_of(self.length);
        let mut y = self.bubble(n, f0, r);
        if self.daughter > -59.0 {
            let k = n_of(self.daughter_at).min(n - 1);
            let d = self.bubble(n - k, f0 * self.daughter_k, r);
            dsp::place(&mut y, &d, k, db(self.daughter));
        }
        let tick = burst(n_of(0.004).min(n), r, 3000.0, 12000.0, 0.0004, 0.00005);
        dsp::place(&mut y, &tick, 0, 0.08);
        finish(&y, r, 90.0, 12000.0, self.room, 0.25, 0.15)
    }
}

// ------------------------------------------------------------------------------- mallet

/// A struck bar or bell: inharmonic partials, each a slowly beating doublet that dies faster the
/// higher it is, a mallet transient, optionally gated or doubled an octave under.
#[derive(Clone, Copy)]
pub struct Mallet {
    pub length: f64,
    pub f0: f64,
    pub partials: &'static [(f64, f64)],
    pub decay: f64,
    pub slope: f64,
    pub beat: f64,
    pub strike: f64,
    pub gate: f64,
    pub under: f64,
    pub width: f64,
    pub room: f64,
}

const BAR: &[(f64, f64)] = &[(1.0, 1.0), (2.756, 0.3), (5.404, 0.12), (8.933, 0.06)];
const TUBULAR: &[(f64, f64)] = &[(1.0, 0.35), (2.756, 1.0), (5.404, 0.7), (8.933, 0.4), (13.34, 0.2)];
const WOODBAR: &[(f64, f64)] = &[(1.0, 1.0), (3.93, 0.25), (9.2, 0.08)];

impl Mallet {
    fn tone(&self, n: usize, f0: f64, r: &mut Rng) -> Vec<f64> {
        let mut y = vec![0.0; n];
        for &(q, a) in self.partials {
            let f = f0 * q;
            if f > SR * 0.45 {
                continue;
            }
            let tau = (self.decay * q.powf(-self.slope)).max(1e-4);
            let b = self.beat * r.uniform(0.4, 1.6);
            let (p1, p2) = (r.uniform(0.0, TAU), r.uniform(0.0, TAU));
            for (i, v) in y.iter_mut().enumerate() {
                let t = t_of(i);
                *v += a * (-t / tau).exp() * ((TAU * f * t + p1).sin() + 0.7 * (TAU * (f + b) * t + p2).sin()) / 1.7;
            }
        }
        let a = n_of(0.0015).min(n);
        for (i, v) in y.iter_mut().take(a).enumerate() {
            *v *= i as f64 / a as f64;
        }
        if self.strike > 0.0 {
            let s = burst(n_of(0.006).min(n), r, f0 * 2.0, (f0 * 12.0).min(18000.0), 0.0008, 0.00005);
            let p = dsp::peak(&y);
            dsp::place(&mut y, &s, 0, self.strike * p);
        }
        y
    }

    fn render(&self, r: &mut Rng, ratio: Option<f64>) -> St {
        let f0 = self.f0 * ratio.unwrap_or(1.0);
        let n = n_of(self.length);
        let mut y = self.tone(n, f0, r);
        if self.under > 0.0 {
            let u = self.tone(n, f0 / 2.0, r);
            dsp::place(&mut y, &u, 0, self.under);
        }
        // the bar rings on past the buffer: let it die rather than stop
        dsp::fade(&mut y, 0.0, 0.3 * self.length);
        if self.gate > 0.0 {
            let (g, fall) = (n_of(self.gate), n_of(0.05));
            for (i, v) in y.iter_mut().enumerate().skip(g) {
                *v *= if i < g + fall { 0.5 + 0.5 * (PI * (i - g) as f64 / fall as f64).cos() } else { 0.0 };
            }
        }
        let st = dsp::widen(&y, r, self.width);
        let mut st = dsp::reverb(&st, r, 0.9, 0.4, self.room, 1.0, 150.0);
        st.truncate(n + n_of(0.4));
        st.trim_tail(-60.0, 0.02);
        st.fade(0.0, 0.02);
        st
    }
}

// ------------------------------------------------------------------------------- glitch

/// A digital stutter that speeds up into the cut: short slices of a noisy tone repeated with
/// gaps of silence, each at another pitch, bit-crushed and sample-held.
#[derive(Clone, Copy)]
pub struct Glitch {
    pub length: f64,
    pub tone_f: f64,
    pub p0: f64,
    pub p1: f64,
    pub bits: f64,
    pub hold: usize,
}

impl Glitch {
    fn render(&self, r: &mut Rng) -> St {
        let n = n_of(self.length);
        let src_n = n_of(0.08);
        let tone: Vec<f64> = (0..src_n).map(|i| ((TAU * self.tone_f * t_of(i)).sin() > 0.0) as i32 as f64 * 2.0 - 1.0).collect();
        let nz = dsp::bandpass(&dsp::white(src_n, r), 800.0, 9000.0, 2);
        let src: Vec<f64> = dsp::unit_peak(tone.iter().zip(dsp::unit_peak(nz).iter()).map(|(a, b)| 0.5 * a + 0.8 * b).collect());
        let mut l = vec![0.0; n];
        let mut rr = vec![0.0; n];
        let mut t = 0.0;
        while t < self.length {
            let u = t / self.length;
            let per = self.p0 * (self.p1 / self.p0).powf(u);
            let slice = (per * r.uniform(0.45, 0.8)).max(0.006);
            let speed = 2f64.powf(r.uniform(-0.6, 0.7));
            let k = n_of(t);
            let m = n_of(slice).min(n.saturating_sub(k));
            let pan = r.uniform(-0.7, 0.7);
            let level = 0.5 + 0.5 * u;
            for i in 0..m {
                let s = src[((i as f64 * speed) as usize) % src_n];
                l[k + i] += level * s * (1.0 - pan).min(1.0);
                rr[k + i] += level * s * (1.0 + pan).min(1.0);
            }
            t += per;
        }
        let q = 2f64.powf(self.bits - 1.0);
        let crush = |x: &[f64]| -> Vec<f64> {
            let held: Vec<f64> = (0..x.len()).map(|i| x[i - i % self.hold.max(1)]).collect();
            held.iter().map(|v| (v * q).round() / q).collect()
        };
        // the steps of the crush smoothed a little at the top (no flat tops at the peak)
        let mut st = St { l: crush(&l), r: crush(&rr) }.map(|c| dsp::lowpass(c, 12000.0, 2));
        st.fade(0.001, 0.004);
        st
    }
}

// ------------------------------------------------------------------------------- the catalogue

const HIT: &[&str] = &["hit", "land"];

/// Every sound shotpilot can make, by id.
pub fn catalogue() -> Vec<Spec> {
    let spec = |id, kinds, note, align, pitch, recipe| Spec { id, kinds, note, align, pitch, recipe };
    let pitch_lo = Some((28.0, 120.0));
    vec![
        // cinematic hits
        spec("impact/classic", HIT, "a cinematic hit: a dropping drum body, a crack and a rumble that holds, in a hall (title slam, reveal)",
             Align::Onset, pitch_lo, Recipe::Impact(IMPACT)),
        spec("impact/deep", HIT, "a darker, longer boom: more sub, a low rumble, a big hall",
             Align::Onset, pitch_lo, Recipe::Impact(Impact { length: 6.2, body_f0: 110.0, body_f1: 44.0, sub: 0.8, sub_f: 34.0,
                 sub_tau: 1.0, roll: 0.5, roll_cut: 140.0, roll_hold: 2.2, crack: 0.4, crack_f: 1600.0, blast_f: 700.0,
                 blast_tau: 0.45, hall_rt: 4.5, hall_damp: 0.6, ..IMPACT })),
        spec("impact/punch", HIT, "a tight drum hit: a short body and a fast fall (2.8 s)",
             Align::Onset, pitch_lo, Recipe::Impact(Impact { length: 2.8, body_f0: 210.0, body_f1: 58.0, drop_tau: 0.03,
                 body_tau: 0.18, crack: 0.5, crack_f: 3000.0, crack_ms: 7.0, blast_tau: 0.12, roll_hold: 0.9, roll_fall: 25.0,
                 hall_rt: 2.0, hall_wet: 0.35, sub: 0.35, sub_tau: 0.45, ..IMPACT })),
        spec("impact/metal", HIT, "a struck metal plate over the boom: a bright clang that rings",
             Align::Onset, pitch_lo, Recipe::Impact(Impact { length: 5.0, shell: 0.6, shell_lo: 260.0, modes: 0.5,
                 crack_f: 3500.0, crack: 0.55, hall_damp: 0.35, hall_rt: 3.6, blast_f: 2000.0, body_attack: 0.001, ..IMPACT })),
        spec("impact/swell", HIT, "a hit with a half-second swell rising into it; `at` is the hit",
             Align::Peak, pitch_lo, Recipe::Impact(Impact { length: 5.7, pre: 0.5, pre_level: 0.35, body_attack: 0.03,
                 roll_hold: 2.0, roll: 0.45, hall_rt: 3.8, ..IMPACT })),
        // sub weight
        spec("sub_drop/808", &["drop"], "an 808 boom: the pitch falls fast, a long decay (weight under a hit)",
             Align::Onset, Some((25.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 120.0, f1: 40.0, glide: 0.12, glide_curve: 0.6,
                 bloom: 0.004, hold: 0.05, decay_db: 30.0, length: 2.0, harm: 0.35, harm_lp: 300.0, click: 0.4, click_f: 3000.0,
                 click_ms: 3.0, thump: 0.5, grit: 0.0, width: 0.05, room: 0.1 })),
        spec("sub_drop/pro", &["drop"], "the designed drop: 64 -> 44 Hz, blooming 0.4 s after the cut under a bright click",
             Align::Onset, Some((25.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 64.0, f1: 44.0, glide: 0.75, glide_curve: 1.0,
                 bloom: 0.38, hold: 0.05, decay_db: 42.0, length: 1.4, harm: 0.3, harm_lp: 300.0, click: 0.6, click_f: 3500.0,
                 click_ms: 30.0, thump: 0.0, grit: 0.0, width: 0.05, room: 0.1 })),
        spec("sub_drop/long", &["drop"], "a long dread drop: 90 -> 30 Hz over 1.4 s with a low grit",
             Align::Onset, Some((25.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 90.0, f1: 30.0, glide: 1.4, glide_curve: 1.0,
                 bloom: 0.15, hold: 0.2, decay_db: 22.0, length: 3.0, harm: 0.25, harm_lp: 250.0, click: 0.15, click_f: 2500.0,
                 click_ms: 8.0, thump: 0.0, grit: 0.15, width: 0.08, room: 0.0 })),
        spec("sub_drop/punch", &["drop"], "a punchy short drop under an impact (0.8 s)",
             Align::Onset, Some((25.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 160.0, f1: 50.0, glide: 0.2, glide_curve: 0.5,
                 bloom: 0.004, hold: 0.0, decay_db: 70.0, length: 0.8, harm: 0.5, harm_lp: 400.0, click: 0.7, click_f: 4000.0,
                 click_ms: 2.0, thump: 0.8, grit: 0.0, width: 0.05, room: 0.15 })),
        spec("sub_drop/gritty", &["drop"], "a drop with a saturated, noisy texture under it",
             Align::Onset, Some((25.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 110.0, f1: 36.0, glide: 0.5, glide_curve: 0.8,
                 bloom: 0.05, hold: 0.1, decay_db: 35.0, length: 1.8, harm: 0.7, harm_lp: 450.0, click: 0.3, click_f: 2200.0,
                 click_ms: 6.0, thump: 0.2, grit: 0.6, width: 0.12, room: 0.05 })),
        spec("cut_hit/sub", HIT, "a short sub hit on a cut: 225 -> 46 Hz with a kick (0.3 s)",
             Align::Onset, Some((30.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 225.0, f1: 46.0, glide: 0.06, glide_curve: 0.6,
                 bloom: 0.003, hold: 0.0, decay_db: 150.0, length: 0.3, harm: 0.5, harm_lp: 320.0, click: 0.5, click_f: 3500.0,
                 click_ms: 3.0, thump: 0.9, grit: 0.0, width: 0.05, room: 0.05 })),
        spec("cut_hit/punch", HIT, "a short punchy cut hit, most of it under 150 Hz (0.28 s)",
             Align::Onset, Some((30.0, 90.0)), Recipe::SubDrop(SubDrop { f0: 180.0, f1: 55.0, glide: 0.04, glide_curve: 0.6,
                 bloom: 0.002, hold: 0.0, decay_db: 170.0, length: 0.28, harm: 0.6, harm_lp: 400.0, click: 0.8, click_f: 4500.0,
                 click_ms: 2.5, thump: 1.0, grit: 0.0, width: 0.05, room: 0.1 })),
        // builds
        spec("riser/tension", &["riser"], "a build into a reveal: a climbing noise band and a gliding chord; it stops on its loudest sample",
             Align::Peak, Some((55.0, 440.0)), Recipe::Riser(Riser { tone: 0.7, ..RISER })),
        spec("riser/air", &["riser"], "an airy noise build, bright at the top; it stops on its loudest sample",
             Align::Peak, None, Recipe::Riser(Riser { length: 2.4, rise_db: 30.0, curve: 2.0, noise: 1.0, nf0: 1500.0,
                 nf1: 16000.0, nbw: 2.5, hp: 300.0, width: 0.9, ..RISER })),
        spec("riser/shepard", &["riser"], "an endless Shepard rise that seems to climb forever; it stops on its loudest sample",
             Align::Peak, None, Recipe::Riser(Riser { length: 3.7, rise_db: 24.0, curve: 1.5, noise: 0.3, nf0: 400.0,
                 nf1: 6000.0, shep: 1.0, hp: 60.0, ..RISER })),
        spec("riser/dark", &["riser"], "a short dark riser: low noise, a sub swell and a speeding stutter, no synth sweep",
             Align::Peak, None, Recipe::Riser(Riser { length: 0.95, rise_db: 26.0, curve: 1.4, noise: 0.7, nf0: 150.0,
                 nf1: 1500.0, nbw: 2.0, sub: 0.8, crackle: 0.3, gate: 0.85, hp: 30.0, width: 0.4, ..RISER })),
        spec("reverse_swell/cymbal", &["swell"], "a bright wide reverse cymbal: a plateau after the peak, then a stop",
             Align::Peak, None, Recipe::Swell(ReverseSwell { length: 0.5037, swell_db: 19.55, curve: 1.831, hold: 0.22,
                 hold_db: -8.039, tail_tau: 0.03096, cym_f: 2600.0, bw: 3.469, metal: 0.6317, partials: 78, shimmer: 0.5093,
                 open: 0.0, hp: 118.7, low_db: -50.0, flut_depth: 0.6266, flut_f: 18.8, width: 0.98, room: 0.0, rt60: 1.386 })),
        spec("reverse_swell/dark", &["swell"], "a darker, near-mono reverse swell with a low body",
             Align::Peak, None, Recipe::Swell(ReverseSwell { length: 0.5391, swell_db: 30.0, curve: 0.5, hold: 0.03684,
                 hold_db: -0.4249, tail_tau: 0.03842, cym_f: 346.0, bw: 3.456, metal: 0.259, partials: 117, shimmer: 0.3166,
                 open: 2.554, hp: 80.0, low_db: 0.0, flut_depth: 0.004984, flut_f: 28.16, width: 0.35, room: 0.08302,
                 rt60: 0.5783 })),
        spec("reverse_swell/short", &["swell"], "a short bright reverse cymbal for a quick cut (0.33 s to the peak)",
             Align::Peak, None, Recipe::Swell(ReverseSwell { length: 0.33, swell_db: 24.0, curve: 0.8282, hold: 0.08,
                 hold_db: -6.0, tail_tau: 0.025, cym_f: 3200.0, bw: 3.2, metal: 0.6, partials: 70, shimmer: 0.04221,
                 open: 0.5, hp: 300.0, low_db: -50.0, flut_depth: 0.8351, flut_f: 24.0, width: 0.95, room: 0.05, rt60: 0.8 })),
        spec("reverse_swell/breath", &["swell"], "a mid-band reverse breath, half wide (1.2 kHz)",
             Align::Peak, None, Recipe::Swell(ReverseSwell { length: 0.7, swell_db: 30.0, curve: 1.3, hold: 0.094,
                 hold_db: 0.0, tail_tau: 0.06, cym_f: 1200.0, bw: 1.499, metal: 0.12, partials: 60, shimmer: 0.3,
                 open: 1.5, hp: 150.0, low_db: -24.0, flut_depth: 0.366, flut_f: 14.0, width: 0.5818, room: 0.2272,
                 rt60: 0.5846 })),
        // passes
        spec("whoosh_big/deep", &["whoosh.long", "swell"], "a big low pass across the frame, scene transitions",
             Align::Peak, None, Recipe::Whoosh(Whoosh { length: 2.0, apex: 0.75, f_lo: 60.0, f_hi: 380.0, q: 0.8,
                 rise_pow: 2.0, fall_pow: 1.6, pan: 0.4, ring: 0.0, ring_f: 0.0, thump: 0.0, room: 0.2 })),
        spec("whoosh_big/gust", &["whoosh.long", "swell"], "a gust of wind passing, mid-band",
             Align::Peak, None, Recipe::Whoosh(Whoosh { length: 1.9, apex: 0.6, f_lo: 250.0, f_hi: 1800.0, q: 0.7,
                 rise_pow: 1.8, fall_pow: 1.4, pan: 0.6, ring: 0.0, ring_f: 0.0, thump: 0.0, room: 0.15 })),
        spec("whoosh_big/thump", &["whoosh.long"], "a big low whoosh that lands with a thump",
             Align::Peak, None, Recipe::Whoosh(Whoosh { length: 1.05, apex: 0.14, f_lo: 80.0, f_hi: 700.0, q: 0.8,
                 rise_pow: 2.2, fall_pow: 2.0, pan: 0.3, ring: 0.0, ring_f: 0.0, thump: 0.9, room: 0.15 })),
        spec("whoosh_fast/crisp", &["whoosh.short", "suck"], "a fast crisp pass: whip pans, fly-ins",
             Align::Peak, None, Recipe::Whoosh(Whoosh { length: 1.0, apex: 0.13, f_lo: 350.0, f_hi: 2600.0, q: 1.1,
                 rise_pow: 2.5, fall_pow: 2.5, pan: 0.8, ring: 0.0, ring_f: 0.0, thump: 0.0, room: 0.12 })),
        spec("whoosh_fast/whip", &["whoosh.short"], "a designed bright whip: a wide ring that falls",
             Align::Peak, None, Recipe::Whoosh(Whoosh { length: 0.9, apex: 0.12, f_lo: 1200.0, f_hi: 7000.0, q: 1.6,
                 rise_pow: 2.5, fall_pow: 2.0, pan: 0.3, ring: 0.5, ring_f: 2800.0, thump: 0.0, room: 0.2 })),
        spec("downlifter/air_fall", &["drop"], "energy falling away after a hit: a noise band sinking",
             Align::Onset, None, Recipe::Fall(Downlifter { length: 2.0, attack: 0.15, f_hi: 9000.0, f_lo: 500.0, noise: 1.0,
                 sub: 0.0, sub_f0: 90.0, sub_f1: 30.0, fall_pow: 1.6, room: 0.2 })),
        spec("downlifter/sub_sink", &["drop"], "a tone sinking into the sub as a scene exits",
             Align::Onset, Some((20.0, 80.0)), Recipe::Fall(Downlifter { length: 1.8, attack: 0.15, f_hi: 900.0, f_lo: 120.0,
                 noise: 0.25, sub: 1.0, sub_f0: 90.0, sub_f1: 30.0, fall_pow: 1.4, room: 0.1 })),
        // small real sounds
        spec("knock/wood", &["tock", "land.light"], "a mid-band wooden knock: a 400 Hz block and its faint rattle",
             Align::Onset, Some((200.0, 2600.0)), Recipe::Knock(Knock { f1: 400.0, decay: 0.00898, slope: 1.4, tilt: 0.4048,
                 cloud: -9.5, cloud_hi: 6.712, cloud_dec: 0.8552, jitter: 0.01331, bend: 2.919, bend_ms: 3.488,
                 contact: 0.00076, grit: 0.6466, noise: 0.01577, noise_hz: 300.0, noise_ms: 0.3, flam: -42.5, flam_ms: 12.92,
                 bounce: -19.75, bounce_ms: 44.74, body: -11.36, body_f: 102.0, body_ms: 4.521, hp: 107.2, lp: 11210.0,
                 room: 0.1648, rt: 0.1519, width: 0.2037, ..KNOCK })),
        spec("knock/desk", &["tock", "land.light"], "a knuckle on a desk: a low wooden knock with the table under it",
             Align::Onset, Some((200.0, 2600.0)), Recipe::Knock(Knock { f1: 699.2, decay: 0.02025, slope: 0.552, tilt: 0.08284,
                 cloud: -0.8436, cloud_hi: 5.867, cloud_dec: 0.9966, jitter: 0.01775, bend: 2.157, bend_ms: 71.36,
                 contact: 0.00066, grit: 0.894, noise: 3.033, noise_hz: 1279.0, noise_ms: 14.94, flam: -20.16, flam_ms: 12.09,
                 bounce: -47.54, bounce_ms: 16.56, body: 4.088, body_f: 95.0, body_ms: 28.8, lp: 4889.0, room: 0.09192,
                 rt: 0.276, width: 0.5584, ..KNOCK })),
        spec("knock/double", &["tock"], "a knock-knock: two wooden taps 55 ms apart (a door, a visitor)",
             Align::Onset, Some((200.0, 2600.0)), Recipe::Knock(Knock { mat: TOK, f1: 537.0, decay: 0.005, slope: -0.00403,
                 tilt: 0.5245, cloud: -50.47, cloud_hi: 3.995, cloud_dec: 1.394, jitter: 0.01402, bend: 1.762, bend_ms: 9.208,
                 contact: 0.00244, grit: 0.6739, noise: -27.15, noise_hz: 2665.0, noise_ms: 3.251, flam: -2.088,
                 flam_ms: 55.14, bounce: -14.68, bounce_ms: 28.8, body: -15.67, body_f: 179.5, body_ms: 4.703, hp: 169.2,
                 lp: 7249.0, room: 0.00579, rt: 0.2958, width: 0.3936 })),
        spec("shutter/quick", &["snap", "click"], "a camera shutter, quick: release and curtain 21 ms apart (a count of evidence shots)",
             Align::Onset, None, Recipe::Shutter(Shutter { length: 0.16, clicks: &[(0.0, 0.8, 2600.0, 4.0), (0.021, 1.0, 2200.0, 6.0)],
                 travel: 0.15, body: 0.0, body_f: 180.0, room: 0.05 })),
        spec("shutter/film", &["snap"], "a heavier film camera: the curtain travels 95 ms, the body thumps",
             Align::Onset, None, Recipe::Shutter(Shutter { length: 0.28, clicks: &[(0.0, 0.6, 1800.0, 5.0), (0.095, 1.0, 1200.0, 9.0)],
                 travel: 0.35, body: 0.5, body_f: 180.0, room: 0.1 })),
        spec("drip/drop", &["pop"], "a water drop: a 306 Hz bubble rising 8 semitones within 40 ms, soft onset",
             Align::Onset, Some((150.0, 1500.0)), Recipe::Drip(Drip { length: 0.33, f0: 306.0, rise_st: 8.0, rise_ms: 40.0,
                 tau: 0.045, onset_ms: 4.0, daughter: -60.0, daughter_at: 0.0, daughter_k: 1.0, room: 0.15 })),
        spec("drip/bubble", &["pop"], "a lower bubble rising 10 semitones, with a daughter bubble",
             Align::Onset, Some((150.0, 1500.0)), Recipe::Drip(Drip { length: 0.29, f0: 220.0, rise_st: 10.0, rise_ms: 60.0,
                 tau: 0.05, onset_ms: 3.0, daughter: -8.0, daughter_at: 0.09, daughter_k: 1.7, room: 0.12 })),
        // tones
        spec("mallet/glock", &["bell", "text"], "a gated glockenspiel note doubled an octave under (G6)",
             Align::Onset, Some((200.0, 4000.0)), Recipe::Mallet(Mallet { length: 0.45, f0: 1568.0, partials: BAR, decay: 0.6,
                 slope: 0.8, beat: 1.0, strike: 0.4, gate: 0.35, under: 0.5, width: 0.3, room: 0.1 })),
        spec("mallet/celesta", &["bell", "text"], "a soft wide celesta note (G5), gated",
             Align::Onset, Some((200.0, 4000.0)), Recipe::Mallet(Mallet { length: 0.45, f0: 784.0, partials: BAR, decay: 0.5,
                 slope: 0.8, beat: 1.0, strike: 0.2, gate: 0.4, under: 0.0, width: 0.7, room: 0.12 })),
        spec("mallet/marimba", &["bell", "text"], "a short dry wooden marimba note (G4)",
             Align::Onset, Some((150.0, 2000.0)), Recipe::Mallet(Mallet { length: 0.55, f0: 392.0, partials: WOODBAR, decay: 0.18,
                 slope: 1.2, beat: 0.5, strike: 0.5, gate: 0.0, under: 0.0, width: 0.15, room: 0.08 })),
        spec("mallet/chime", &["bell"], "a tubular chime ringing out (C6), slow beating",
             Align::Onset, Some((200.0, 3000.0)), Recipe::Mallet(Mallet { length: 2.5, f0: 1046.5, partials: TUBULAR, decay: 1.4,
                 slope: 0.6, beat: 1.2, strike: 0.3, gate: 0.0, under: 0.0, width: 0.5, room: 0.25 })),
        spec("glitch/stutter", &["glitch"], "a digital stutter that speeds up into the cut (error states, glitch transitions)",
             Align::Peak, None, Recipe::Glitch(Glitch { length: 0.64, tone_f: 880.0, p0: 0.07, p1: 0.022, bits: 5.0, hold: 3 })),
    ]
}
