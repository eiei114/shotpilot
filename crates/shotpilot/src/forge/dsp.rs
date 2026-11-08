//! Building blocks for sounds made in code (D-103): bodies ring in damped modes, air is filtered
//! noise, every sound gets a room. 48 kHz, f64, no recorded audio anywhere. Deterministic for a
//! given seed: the same definition always gives the same samples.

use std::f64::consts::{PI, TAU};

pub const SR: f64 = 48_000.0;

/// A small seeded generator (SplitMix64), with normal draws by Box-Muller.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x5DEE_CE66_D1CE_4E5B)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }

    pub fn normal(&mut self) -> f64 {
        let u = self.unit().max(1e-300);
        (-2.0 * u.ln()).sqrt() * (TAU * self.unit()).cos()
    }

    /// A generator of its own for one layer, so layers do not shift each other's draws.
    pub fn fork(&mut self) -> Rng {
        Rng::new(self.next())
    }
}

pub fn n_of(seconds: f64) -> usize {
    ((seconds * SR).round() as usize).max(1)
}

pub fn t_of(i: usize) -> f64 {
    i as f64 / SR
}

pub fn peak(x: &[f64]) -> f64 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

pub fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

/// `x` scaled to a peak of 1 (silence stays silent).
pub fn unit_peak(mut x: Vec<f64>) -> Vec<f64> {
    let p = peak(&x);
    if p > 0.0 {
        x.iter_mut().for_each(|v| *v /= p);
    }
    x
}

/// `x` scaled to an RMS of 1.
pub fn unit_rms(mut x: Vec<f64>) -> Vec<f64> {
    let r = rms(&x);
    if r > 0.0 {
        x.iter_mut().for_each(|v| *v /= r);
    }
    x
}

pub fn white(n: usize, r: &mut Rng) -> Vec<f64> {
    (0..n).map(|_| r.normal()).collect()
}

/// Pink (1/f) noise, unit RMS (Paul Kellet's filter).
pub fn pink(n: usize, r: &mut Rng) -> Vec<f64> {
    let mut b = [0.0f64; 7];
    let x = (0..n)
        .map(|_| {
            let w = r.normal();
            b[0] = 0.99886 * b[0] + w * 0.0555179;
            b[1] = 0.99332 * b[1] + w * 0.0750759;
            b[2] = 0.96900 * b[2] + w * 0.1538520;
            b[3] = 0.86650 * b[3] + w * 0.3104856;
            b[4] = 0.55000 * b[4] + w * 0.5329522;
            b[5] = -0.7616 * b[5] - w * 0.0168980;
            let y = b.iter().sum::<f64>() + w * 0.5362;
            b[6] = w * 0.115926;
            y
        })
        .collect();
    unit_rms(x)
}

/// Brown (1/f^2) noise, unit RMS: leaky integrated white noise without its drift.
pub fn brown(n: usize, r: &mut Rng) -> Vec<f64> {
    let mut acc = 0.0;
    let x: Vec<f64> = (0..n)
        .map(|_| {
            acc = 0.998 * acc + r.normal();
            acc
        })
        .collect();
    unit_rms(highpass(&x, 20.0, 1))
}

/// A slow random control signal, unit std, mean 0 (low-passed noise at about `rate` Hz).
pub fn lfnoise(n: usize, rate: f64, r: &mut Rng) -> Vec<f64> {
    let w = white(n + n_of(0.5), r);
    let y = lowpass(&w, rate.max(0.2), 2);
    let y = &y[n_of(0.5)..];
    let mean = y.iter().sum::<f64>() / n.max(1) as f64;
    let c: Vec<f64> = y.iter().map(|v| v - mean).collect();
    unit_rms(c)
}

// ------------------------------------------------------------------------------- envelopes

/// 0 -> 1 over `a` seconds (raised cosine), 1 after.
pub fn rise(t: f64, a: f64) -> f64 {
    if a <= 0.0 || t >= a { 1.0 } else { 0.5 - 0.5 * (PI * t / a).cos() }
}

/// Raised-cosine fades at both ends.
pub fn fade(x: &mut [f64], fin: f64, fout: f64) {
    let n = x.len();
    let a = n_of(fin).min(n);
    let b = n_of(fout).min(n);
    for (i, v) in x.iter_mut().take(a).enumerate() {
        *v *= 0.5 - 0.5 * (PI * i as f64 / a as f64).cos();
    }
    for (k, v) in x[n - b..].iter_mut().enumerate() {
        *v *= 0.5 + 0.5 * (PI * k as f64 / b as f64).cos();
    }
}

/// Add `src * gain` into `dst` from sample `at`.
pub fn place(dst: &mut [f64], src: &[f64], at: usize, gain: f64) {
    for (d, s) in dst.iter_mut().skip(at).zip(src) {
        *d += gain * s;
    }
}

pub fn db(v: f64) -> f64 {
    10f64.powf(v / 20.0)
}

// ------------------------------------------------------------------------------- filters

/// One second-order section (transposed direct form II).
#[derive(Clone, Copy)]
pub struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}

fn clamp_f(f: f64) -> f64 {
    f.clamp(10.0, SR * 0.475)
}

impl Biquad {
    fn new(b0: f64, b1: f64, b2: f64, a0: f64, a1: f64, a2: f64) -> Biquad {
        Biquad { b: [b0 / a0, b1 / a0, b2 / a0], a: [a1 / a0, a2 / a0], z: [0.0; 2] }
    }

    pub fn lowpass(f: f64, q: f64) -> Biquad {
        let w = TAU * clamp_f(f) / SR;
        let (c, al) = (w.cos(), w.sin() / (2.0 * q));
        Biquad::new((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0, 1.0 + al, -2.0 * c, 1.0 - al)
    }

    pub fn highpass(f: f64, q: f64) -> Biquad {
        let w = TAU * clamp_f(f) / SR;
        let (c, al) = (w.cos(), w.sin() / (2.0 * q));
        Biquad::new((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0, 1.0 + al, -2.0 * c, 1.0 - al)
    }

    pub fn peaking(f: f64, gain_db: f64, q: f64) -> Biquad {
        let a = 10f64.powf(gain_db / 40.0);
        let w = TAU * clamp_f(f) / SR;
        let (c, al) = (w.cos(), w.sin() / (2.0 * q));
        Biquad::new(1.0 + al * a, -2.0 * c, 1.0 - al * a, 1.0 + al / a, -2.0 * c, 1.0 - al / a)
    }

    /// First order, by the bilinear transform.
    fn first(f: f64, high: bool) -> Biquad {
        let k = (PI * clamp_f(f) / SR).tan();
        if high {
            Biquad::new(1.0, -1.0, 0.0, 1.0 + k, k - 1.0, 0.0)
        } else {
            Biquad::new(k, k, 0.0, 1.0 + k, k - 1.0, 0.0)
        }
    }

    #[inline]
    pub fn run(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}

pub fn filter(x: &[f64], mut sections: Vec<Biquad>) -> Vec<f64> {
    x.iter()
        .map(|&v| sections.iter_mut().fold(v, |acc, s| s.run(acc)))
        .collect()
}

/// Butterworth sections of order 1, 2 or 4.
fn butter(f: f64, order: usize, high: bool) -> Vec<Biquad> {
    let make = |q: f64| if high { Biquad::highpass(f, q) } else { Biquad::lowpass(f, q) };
    match order {
        0 | 1 => vec![Biquad::first(f, high)],
        2 | 3 => vec![make(std::f64::consts::FRAC_1_SQRT_2)],
        _ => vec![make(0.541_196_1), make(1.306_563)],
    }
}

pub fn lowpass(x: &[f64], f: f64, order: usize) -> Vec<f64> {
    filter(x, butter(f, order, false))
}

pub fn highpass(x: &[f64], f: f64, order: usize) -> Vec<f64> {
    filter(x, butter(f, order, true))
}

pub fn bandpass(x: &[f64], lo: f64, hi: f64, order: usize) -> Vec<f64> {
    let mut s = butter(lo, order, true);
    s.extend(butter(hi.max(lo * 1.05), order, false));
    filter(x, s)
}

pub fn peq(x: &[f64], f: f64, gain_db: f64, q: f64) -> Vec<f64> {
    filter(x, vec![Biquad::peaking(f, gain_db, q)])
}

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Low,
    /// Band-pass normalised to a peak gain of 1.
    Band,
    High,
}

/// A state-variable filter whose cutoff moves (Cytomic TPT): `fc(i)` per sample.
pub fn svf(x: &[f64], fc: impl Fn(usize) -> f64, q: f64, mode: Mode) -> Vec<f64> {
    let k = 1.0 / q.max(0.05);
    let (mut ic1, mut ic2) = (0.0, 0.0);
    x.iter()
        .enumerate()
        .map(|(i, &v)| {
            let g = (PI * fc(i).clamp(20.0, SR * 0.45) / SR).tan();
            let a1 = 1.0 / (1.0 + g * (g + k));
            let a2 = g * a1;
            let a3 = g * a2;
            let v3 = v - ic2;
            let v1 = a1 * ic1 + a2 * v3;
            let v2 = ic2 + a2 * ic1 + a3 * v3;
            ic1 = 2.0 * v1 - ic1;
            ic2 = 2.0 * v2 - ic2;
            match mode {
                Mode::Low => v2,
                Mode::Band => k * v1,
                Mode::High => v - k * v1 - v2,
            }
        })
        .collect()
}

// ------------------------------------------------------------------------------- oscillators

/// The running phase (radians) of a tone whose pitch is `f(i)` Hz.
pub fn phase(n: usize, f: impl Fn(usize) -> f64, start: f64) -> Vec<f64> {
    let mut ph = start;
    (0..n)
        .map(|i| {
            let p = ph;
            ph += TAU * f(i) / SR;
            p
        })
        .collect()
}

fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// A band-limited saw (PolyBLEP), so sweeps do not alias.
pub fn saw(n: usize, f: impl Fn(usize) -> f64, start: f64) -> Vec<f64> {
    let mut ph = start.rem_euclid(1.0);
    (0..n)
        .map(|i| {
            let dt = (f(i) / SR).clamp(1e-9, 0.49);
            let y = 2.0 * ph - 1.0 - blep(ph, dt);
            ph += dt;
            if ph >= 1.0 {
                ph -= 1.0;
            }
            y
        })
        .collect()
}

/// Shepard / Risset glissando of sines an octave apart under a bell over log-frequency: a
/// pitch that seems to rise (or fall) without end. Unit RMS.
pub fn shepard(n: usize, r: &mut Rng, up: bool, rate: f64, f_lo: f64, f_hi: f64, tilt: f64) -> Vec<f64> {
    let octs = (f_hi / f_lo).log2();
    let voices = (octs.round() as usize).max(2);
    let sgn = if up { 1.0 } else { -1.0 };
    let mut out = vec![0.0; n];
    for k in 0..voices {
        let mut ph = r.uniform(0.0, TAU);
        for (i, o) in out.iter_mut().enumerate() {
            let u = (k as f64 / voices as f64 + sgn * rate * t_of(i) / octs).rem_euclid(1.0);
            let f = f_lo * 2f64.powf(u * octs);
            let a = (0.5 - 0.5 * (TAU * u).cos()) * db(tilt * u * octs);
            *o += a * ph.sin();
            ph += TAU * f / SR;
        }
    }
    unit_rms(out)
}

// ------------------------------------------------------------------------------- FFT

fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = TAU / len as f64 * if inverse { 1.0 } else { -1.0 };
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (xr, xi) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - xr;
                im[b] = im[a] - xi;
                re[a] += xr;
                im[a] += xi;
                let t = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = t;
            }
        }
        len <<= 1;
    }
    if inverse {
        re.iter_mut().for_each(|v| *v /= n as f64);
        im.iter_mut().for_each(|v| *v /= n as f64);
    }
}

/// Linear convolution, by FFT.
pub fn convolve(a: &[f64], b: &[f64]) -> Vec<f64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let len = a.len() + b.len() - 1;
    if a.len().min(b.len()) <= 64 {
        let mut out = vec![0.0; len];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                out[i + j] += x * y;
            }
        }
        return out;
    }
    let n = len.next_power_of_two();
    let (mut ar, mut ai) = (a.to_vec(), vec![0.0; n]);
    ar.resize(n, 0.0);
    let (mut br, mut bi) = (b.to_vec(), vec![0.0; n]);
    br.resize(n, 0.0);
    fft(&mut ar, &mut ai, false);
    fft(&mut br, &mut bi, false);
    for k in 0..n {
        let (r, i) = (ar[k] * br[k] - ai[k] * bi[k], ar[k] * bi[k] + ai[k] * br[k]);
        ar[k] = r;
        ai[k] = i;
    }
    fft(&mut ar, &mut ai, true);
    ar.truncate(len);
    ar
}

// ------------------------------------------------------------------------------- stereo and space

/// Two channels.
#[derive(Clone)]
pub struct St {
    pub l: Vec<f64>,
    pub r: Vec<f64>,
}

impl St {
    pub fn mono(x: Vec<f64>) -> St {
        St { r: x.clone(), l: x }
    }

    pub fn len(&self) -> usize {
        self.l.len()
    }

    pub fn is_empty(&self) -> bool {
        self.l.is_empty()
    }

    pub fn peak(&self) -> f64 {
        peak(&self.l).max(peak(&self.r))
    }

    pub fn map(&self, f: impl Fn(&[f64]) -> Vec<f64>) -> St {
        St { l: f(&self.l), r: f(&self.r) }
    }

    pub fn truncate(&mut self, n: usize) {
        self.l.truncate(n);
        self.r.truncate(n);
    }

    pub fn fade(&mut self, fin: f64, fout: f64) {
        fade(&mut self.l, fin, fout);
        fade(&mut self.r, fin, fout);
    }

    /// Scaled to a sample peak of -1 dBFS.
    pub fn normalize(&mut self) {
        let p = self.peak();
        if p > 0.0 {
            let g = 0.89 / p;
            self.l.iter_mut().chain(self.r.iter_mut()).for_each(|v| *v *= g);
        }
    }

    /// Cut where the 5 ms level has fallen `end_db` under its loudest, then fade the last `fout` s.
    pub fn trim_tail(&mut self, end_db: f64, fout: f64) {
        let h = n_of(0.005);
        let frames = self.len() / h;
        if frames < 2 {
            return;
        }
        let lv: Vec<f64> = (0..frames)
            .map(|k| {
                let s: f64 = (k * h..(k + 1) * h).map(|i| (self.l[i] + self.r[i]).powi(2) / 4.0).sum();
                10.0 * (s / h as f64 + 1e-24).log10()
            })
            .collect();
        let top = (0..frames).max_by(|a, b| lv[*a].total_cmp(&lv[*b])).unwrap_or(0);
        let end = (top..frames).find(|k| lv[*k] < lv[top] + end_db).map_or(self.len(), |k| (k * h + n_of(fout)).min(self.len()));
        self.truncate(end);
        self.fade(0.0, fout);
    }
}

/// Mono to stereo with a decorrelated side above 300 Hz (two short delays), as a mic pair hears
/// a small source: `width` 0..1.
pub fn widen(x: &[f64], r: &mut Rng, width: f64) -> St {
    let (d1, d2) = (n_of(r.uniform(0.2, 0.9) * 1e-3), n_of(r.uniform(0.2, 0.9) * 1e-3));
    let mut side = vec![0.0; x.len()];
    for i in 0..x.len() {
        side[i] = if i >= d1 { x[i - d1] } else { 0.0 } - if i >= d2 { x[i - d2] } else { 0.0 };
    }
    let side = highpass(&side, 300.0, 2);
    St {
        l: x.iter().zip(&side).map(|(a, s)| a + width * s).collect(),
        r: x.iter().zip(&side).map(|(a, s)| a - width * s).collect(),
    }
}

/// Constant-power pan that moves: `pan(i)` from -1 (left) to 1 (right).
pub fn pan(x: &[f64], pan: impl Fn(usize) -> f64) -> St {
    let mut l = Vec::with_capacity(x.len());
    let mut r = Vec::with_capacity(x.len());
    for (i, v) in x.iter().enumerate() {
        let th = (pan(i).clamp(-1.0, 1.0) + 1.0) * PI / 4.0;
        l.push(v * th.cos() * std::f64::consts::SQRT_2);
        r.push(v * th.sin() * std::f64::consts::SQRT_2);
    }
    St { l, r }
}

/// One channel of a room's impulse response: a few early reflections and a decaying noise tail
/// whose highs die sooner (`damp` 0..1), at unit energy.
fn room_ir(r: &mut Rng, rt60: f64, damp: f64, predelay: f64) -> Vec<f64> {
    let rt_hi = rt60 * (1.0 - 0.85 * damp).max(0.05);
    let n = n_of((1.2 * rt60).clamp(0.05, 5.0));
    let pd = n_of(predelay.max(0.0)).min(n / 2);
    let w = white(n, r);
    let lo = lowpass(&w, 2500.0, 2);
    let hi = highpass(&w, 2500.0, 2);
    let mut ir: Vec<f64> = (0..n)
        .map(|i| {
            let t = t_of(i);
            lo[i] * (-6.91 * t / rt60.max(0.02)).exp() + hi[i] * (-6.91 * t / rt_hi.max(0.02)).exp()
        })
        .collect();
    // the onset swells in over a few ms (a diffuse field, not a click)
    for (i, v) in ir.iter_mut().take(n_of(0.004)).enumerate() {
        *v *= i as f64 / n_of(0.004) as f64;
    }
    for _ in 0..6 {
        let d = n_of(r.uniform(0.003, 0.025));
        if d < n {
            ir[d] += r.uniform(0.6, 1.4) * if r.unit() < 0.5 { -1.0 } else { 1.0 } * rms(&ir) * 6.0;
        }
    }
    let e = ir.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
    let mut out = vec![0.0; pd];
    out.extend(ir.iter().map(|v| v / e));
    out
}

/// `x` in a room: the dry sound plus `wet` of its reverb (unit-energy impulse response, one per
/// channel, `width` 0..1 apart). Lows under `lowcut` Hz stay dry. Longer than `x` by the tail.
pub fn reverb(x: &St, r: &mut Rng, rt60: f64, damp: f64, wet: f64, width: f64, lowcut: f64) -> St {
    if wet <= 0.001 {
        return x.clone();
    }
    let ir_l = room_ir(r, rt60, damp, 0.008);
    let ir_r0 = room_ir(r, rt60, damp, 0.008);
    let k = (1.0 - width * width).max(0.0).sqrt();
    let ir_r: Vec<f64> = ir_l.iter().zip(ir_r0.iter().chain(std::iter::repeat(&0.0))).map(|(a, b)| k * a + width * b).collect();
    let send = |c: &[f64]| if lowcut > 20.0 { highpass(c, lowcut, 2) } else { c.to_vec() };
    let wl = convolve(&send(&x.l), &ir_l);
    let wr = convolve(&send(&x.r), &ir_r);
    let n = wl.len().max(wr.len());
    let mix = |dry: &[f64], w: &[f64]| -> Vec<f64> {
        (0..n).map(|i| dry.get(i).copied().unwrap_or(0.0) + wet * w.get(i).copied().unwrap_or(0.0)).collect()
    };
    St { l: mix(&x.l, &wl), r: mix(&x.r, &wr) }
}

/// A gentle saturation: `drive` 0..1, peak kept.
pub fn saturate(x: &[f64], drive: f64) -> Vec<f64> {
    if drive <= 0.01 {
        return x.to_vec();
    }
    let g = 1.0 + 6.0 * drive;
    let p = peak(x).max(1e-12);
    x.iter().map(|v| p * (g * v / p).tanh() / g.tanh()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fft_convolves_like_the_sum() {
        let mut r = Rng::new(3);
        let a = white(300, &mut r);
        let b = white(200, &mut r);
        let fast = convolve(&a, &b);
        let mut slow = vec![0.0; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                slow[i + j] += x * y;
            }
        }
        assert!(fast.iter().zip(&slow).all(|(x, y)| (x - y).abs() < 1e-9));
    }

    #[test]
    fn a_lowpass_keeps_the_lows_and_takes_the_highs() {
        let tone = |f: f64| -> Vec<f64> { (0..4800).map(|i| (TAU * f * t_of(i)).sin()).collect() };
        let lo = lowpass(&tone(100.0), 1000.0, 4);
        let hi = lowpass(&tone(8000.0), 1000.0, 4);
        assert!(rms(&lo[2400..]) > 0.68 && rms(&hi[2400..]) < 0.01);
    }

    #[test]
    fn the_same_seed_draws_the_same_noise() {
        assert_eq!(white(10, &mut Rng::new(7)), white(10, &mut Rng::new(7)));
        assert_ne!(white(10, &mut Rng::new(7)), white(10, &mut Rng::new(8)));
    }
}
