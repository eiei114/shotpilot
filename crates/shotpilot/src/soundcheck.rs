//! Sound check: the teaser's mix measured against professional launch films: BS.1770
//! loudness, true peak, loudness range, the loudness arc and the ending, onset and
//! salient-hit density, sound on cuts, spectral balance and stereo width, scored against a
//! profile of 7 launch films with fixed weights, tolerances, practice windows and verdict
//! bands.
//!
//! Not measured: the checks that need ML models (realness, music and speech presence) and
//! beat tracking. They are listed as not measured and the verdict says "partial". Onsets
//! follow librosa's spectral-flux recipe on a 24 kHz copy. Picture events are the shot
//! boundaries themselves, which makes sound-on-cut exact.
//!
//! Adapted from MIT-licensed code (Copyright (c) 2026 Leon Lin); the full notice is in
//! THIRD_PARTY_NOTICES.md.

use serde_json::{json, Map, Value};

use crate::sound::{Pcm, SR as SR_U32};

/// Bump when a measurement or the scoring changes for the same mix.
pub const CHECK_VERSION: i64 = 2;

const SR: f64 = SR_U32 as f64;
/// 2 ms level frames used to find attacks and the edges of sound.
const E2: f64 = 0.002;
/// Onset analysis: 24 kHz, 2048-point frames, 256 hop (~10.7 ms), 128 mel bands.
const OSR: f64 = 24_000.0;
const OHOP: usize = 256;
const ONFFT: usize = 2048;
const MIN_S: f64 = 3.0;
const SHORT_S: f64 = 10.0;
const MONO_DB: f64 = -40.0;
const SHAPE: &[&str] = &["lra", "crest_lu", "intro_lu", "end_lu", "final_lu"];
const SHIFTS: &[f64] = &[0.23, 0.31, 0.43, 0.57, 0.71, 0.87, 1.13, 1.37, -0.29, -0.41, -0.53, -0.67, -0.79, -0.97, -1.19, -1.43];

// --------------------------------------------------------------------------- the reference profile

/// The reference profile: 7 professional launch films.
/// Per metric: n, min, max, median, p10, p90.
const PROFILE: &[(&str, f64, f64, f64, f64, f64, f64)] = &[
    ("lufs_i", 7.0, -22.2, -7.3, -14.8, -19.32, -9.16),
    ("true_peak", 7.0, -0.8, 2.0, 0.6, -0.26, 1.52),
    ("lra", 7.0, 1.7, 12.2, 5.5, 2.66, 9.08),
    ("crest_lu", 7.0, 1.1, 3.5, 2.3, 1.16, 3.32),
    ("intro_lu", 7.0, -17.5, 2.2, -1.1, -8.08, 0.58),
    ("end_lu", 7.0, -14.3, -0.3, -3.0, -11.96, -0.66),
    ("peak_pos", 7.0, 0.03, 0.88, 0.75, 0.096, 0.826),
    ("first_sound_s", 7.0, 0.04, 0.48, 0.13, 0.052, 0.402),
    ("max_gap_s", 7.0, 0.0, 1.66, 0.02, 0.0, 0.688),
    ("end_silence_s", 7.0, 0.02, 1.9, 0.04, 0.02, 1.162),
    ("fade_s", 7.0, 0.08, 2.54, 0.27, 0.092, 2.288),
    ("final_lu", 7.0, -2.2, 5.6, 0.2, -1.42, 5.18),
    ("tail_s", 6.0, 0.13, 3.58, 2.99, 1.065, 3.58),
    ("music_frac", 7.0, 0.67, 1.0, 1.0, 0.802, 1.0),
    ("speech_frac", 7.0, 0.0, 1.0, 0.83, 0.0, 1.0),
    ("tempo", 7.0, 108.0, 136.0, 123.0, 115.2, 136.0),
    ("onset_rate", 7.0, 2.46, 8.22, 5.66, 3.27, 6.876),
    ("hit_rate", 7.0, 0.1, 1.66, 1.25, 0.748, 1.6),
    ("stopdowns_min", 7.0, 0.0, 18.3, 1.2, 0.0, 9.06),
    ("cut_rate", 7.0, 0.3, 1.6, 0.63, 0.306, 1.42),
    ("cuts_with_sound", 7.0, 0.5, 1.0, 0.81, 0.56, 0.934),
    ("cut_hit_frac", 7.0, 0.03, 0.53, 0.29, 0.078, 0.488),
    ("cut_frame_sync", 7.0, 0.28, 0.67, 0.44, 0.34, 0.592),
    ("cuts_on_beat", 7.0, 0.18, 0.52, 0.22, 0.198, 0.448),
    ("band_sub", 7.0, -16.2, -4.1, -6.6, -12.36, -4.58),
    ("band_low", 7.0, -6.7, -2.1, -4.2, -5.92, -2.34),
    ("band_lowmid", 7.0, -16.8, -1.7, -5.3, -16.38, -3.2),
    ("band_mid", 7.0, -19.1, -10.5, -12.4, -15.92, -10.68),
    ("band_presence", 7.0, -30.3, -13.1, -17.8, -30.24, -13.52),
    ("band_air", 7.0, -34.2, -11.2, -23.6, -31.86, -14.74),
    ("brightness_hz", 7.0, 1169.0, 3633.0, 1966.0, 1183.4, 3030.0),
    ("side_to_mid_db", 7.0, -14.5, -5.0, -8.5, -11.8, -5.78),
    ("correlation", 7.0, 0.52, 0.93, 0.75, 0.58, 0.864),
];

/// The profile's loudness arc (LU vs integrated at 0 %, 10 % .. 100 %): median, p10, p90.
const ARC_MEDIAN: [f64; 11] = [-1.3, -0.8, 0.3, -0.3, -0.2, 0.2, 0.0, -0.4, 0.9, -0.3, -7.4];
const ARC_P10: [f64; 11] = [-12.4, -5.1, -4.8, -2.9, -3.0, -0.8, -2.7, -2.4, -3.3, -5.8, -12.2];
const ARC_P90: [f64; 11] = [1.3, -0.1, 1.0, 0.7, 0.4, 1.3, 0.8, 1.4, 2.0, 0.7, -2.5];

struct Stat {
    n: f64,
    min: f64,
    max: f64,
    median: f64,
    p10: f64,
    p90: f64,
}

fn stat(key: &str) -> Option<Stat> {
    PROFILE.iter().find(|r| r.0 == key).map(|r| Stat { n: r.1, min: r.2, max: r.3, median: r.4, p10: r.5, p90: r.6 })
}

// --------------------------------------------------------------------------- metric table

#[derive(Clone, Copy, PartialEq)]
enum Sides {
    Both,
    Upper,
    Lower,
}

struct Spec {
    key: &'static str,
    group: &'static str,
    label: &'static str,
    unit: &'static str,
    decimals: usize,
    /// 3 critical, 2 major, 1 minor, 0 info (shown, not scored).
    weight: u8,
    tol: f64,
    sides: Sides,
    /// Web-delivery practice window that replaces the reference range.
    practice: Option<(Option<f64>, Option<f64>)>,
    why: &'static str,
}

#[allow(clippy::too_many_arguments)] // one row of the metric table, read best positionally
const fn sp(key: &'static str, group: &'static str, label: &'static str, unit: &'static str, decimals: usize, weight: u8,
            tol: f64, sides: Sides, why: &'static str) -> Spec {
    Spec { key, group, label, unit, decimals, weight, tol, sides, practice: None, why }
}

/// Weights and tolerances per metric; labels and reasons in Japanese.
const SPECS: &[Spec] = &[
    Spec { practice: Some((Some(-16.0), Some(-9.0))),
           ..sp("lufs_i", "loudness", "統合ラウドネス", "LUFS", 1, 3, 1.0, Sides::Both,
                "ほかのローンチ映像と並んだときの音の大きさ。X・LinkedIn・サイトの自動再生は音量をそろえない。YouTube は -14 LUFS より大きいマスターを下げる") },
    Spec { practice: Some((None, Some(-1.0))),
           ..sp("true_peak", "loudness", "トゥルーピーク", "dBTP", 1, 2, 0.5, Sides::Upper,
                "-1 dBTP を超えるマスターは AAC / MP3 に変換すると割れる。いちばん大きいヒットで音がバリバリする") },
    sp("music_frac", "music", "音楽が鳴っている割合", "share", 2, 3, 0.05, Sides::Lower,
       "基準の映像は音楽が主役。1本の曲が全体のエネルギーを運び、編集のつなぎ目を隠す"),
    sp("cuts_with_sound", "sync", "音が乗ったカット（±100 ms）", "share", 2, 2, 0.05, Sides::Lower,
       "音のないカットや大きな画の変化は死んで見える。動きには音が付くものだと見る人は思っている"),
    sp("lra", "loudness", "ラウドネスレンジ", "LU", 1, 2, 1.0, Sides::Both,
       "音量の振れ幅。ローンチ映像は本編をずっと強めに保つ"),
    sp("crest_lu", "loudness", "最も大きい3秒と平均の差", "LU", 1, 2, 0.5, Sides::Both,
       "いちばん大きい3秒が平均からどれだけ飛び出すか"),
    sp("first_sound_s", "arc", "最初に音が鳴るまで", "s", 2, 2, 0.1, Sides::Upper,
       "冒頭の無音で自動再生の視聴者が離れる。基準の映像はすぐに鳴る"),
    sp("end_silence_s", "ending", "終わりの無音", "s", 2, 2, 0.25, Sides::Upper,
       "音が止まったあとの空白は、終わりが未完成に感じられる"),
    sp("max_gap_s", "arc", "途中の最長の無音", "s", 2, 2, 0.25, Sides::Upper,
       "途中の無音の穴は、意図したストップダウンでない限りミスに見える"),
    sp("final_lu", "ending", "最後の瞬間の音量", "LU", 1, 2, 1.5, Sides::Both,
       "ロゴやエンドカードの瞬間は、本編と同じかそれ以上で着地させる。フェードで消さない"),
    sp("onset_rate", "density", "音の立ち上がりの密度", "/s", 2, 2, 0.3, Sides::Both,
       "リズムの密度。少なすぎると空っぽ、多すぎるとうるさい"),
    sp("hit_rate", "density", "目立つヒットの密度", "/s", 2, 2, 0.2, Sides::Both,
       "パンチ。目立つヒットが大事な瞬間に印を付ける。無いと何も決まらない"),
    sp("intro_lu", "arc", "冒頭10%の音量", "LU", 1, 1, 1.5, Sides::Both,
       "始まり方。基準の映像は本編に近い音量で始まる（1本はビルドアップ）"),
    sp("end_lu", "arc", "最後10%の音量", "LU", 1, 1, 1.5, Sides::Both, "最後の10%が本編に対してどこにいるか"),
    sp("fade_s", "ending", "最後のフェード／余韻", "s", 2, 1, 0.5, Sides::Both,
       "ローンチ映像はスティングかハードアウトで終わる。長いフェードアウトは締まらない"),
    sp("band_sub", "spectrum", "サブ 20–60 Hz の割合", "dB", 1, 1, 1.5, Sides::Both, "ヒットと音楽の下のサブの重さ"),
    sp("band_low", "spectrum", "ロー 60–250 Hz の割合", "dB", 1, 1, 1.5, Sides::Both, "低域の胴鳴り"),
    sp("band_lowmid", "spectrum", "ローミッド 250 Hz–1 kHz の割合", "dB", 1, 1, 1.5, Sides::Both,
       "多すぎるとこもる、少なすぎるとスカスカ"),
    sp("band_mid", "spectrum", "ミッド 1–4 kHz の割合", "dB", 1, 1, 1.5, Sides::Both,
       "クリックや声がスマホのスピーカーで抜けてくる帯域"),
    sp("band_presence", "spectrum", "プレゼンス 4–8 kHz の割合", "dB", 1, 1, 1.5, Sides::Both,
       "トランジェント、クリック、タイピングの噛みつき"),
    sp("band_air", "spectrum", "エア 8–16 kHz の割合", "dB", 1, 1, 1.5, Sides::Both,
       "開放感。無いとこもった低ビットレートの音に聞こえる"),
    sp("side_to_mid_db", "stereo", "ステレオ幅（サイド／ミッド）", "dB", 1, 1, 1.0, Sides::Both,
       "モノラルに近いと小さく聞こえ、広すぎるとスマホで崩れる"),
    sp("speech_frac", "music", "声の割合", "share", 2, 0, 0.0, Sides::Both, "声の割合（基準7本のうち4本にナレーションあり）"),
    sp("tempo", "music", "テンポ", "BPM", 0, 0, 0.0, Sides::Both, "音楽のテンポ"),
    sp("peak_pos", "arc", "最も大きい3秒の位置", "0-1", 2, 0, 0.0, Sides::Both, "いちばん大きい瞬間がどこにあるか（0 = 冒頭、1 = 最後）"),
    sp("tail_s", "ending", "最後のヒットの余韻", "s", 2, 0, 0.0, Sides::Both, "最後のヒットがどれだけ鳴り残るか"),
    sp("stopdowns_min", "density", "ストップダウン", "/min", 1, 0, 0.0, Sides::Both,
       "ヒットの直前に音を一瞬抜く手法（基準7本のうち4本が使う）"),
    sp("cut_rate", "sync", "カットの頻度", "/s", 2, 0, 0.0, Sides::Both, "編集のテンポ（1秒あたりのカット数）"),
    sp("cut_hit_frac", "sync", "目立つヒットが乗ったカット", "share", 2, 0, 0.0, Sides::Both,
       "目立つヒットが乗ったカット（基準の映像ではほぼ偶然並み）"),
    sp("cut_frame_sync", "sync", "1フレーム以内に音が乗ったカット", "share", 2, 0, 0.0, Sides::Both,
       "1フレーム以内に音の立ち上がりがあるカット（基準ではほぼ偶然並み）"),
    sp("cuts_on_beat", "sync", "拍に乗ったカット", "share", 2, 0, 0.0, Sides::Both, "拍の ±70 ms に乗ったカット"),
    sp("brightness_hz", "spectrum", "明るさ（スペクトル重心の中央値）", "Hz", 0, 0, 0.0, Sides::Both, "ミックスの明るさ"),
    sp("correlation", "stereo", "左右の相関", "r", 2, 0, 0.0, Sides::Both, "左右の相関（1 = モノラル）"),
];

/// Metrics that need a model or beat tracking: never measured here.
const NEEDS_MODEL: &[(&str, &str)] = &[
    ("p_real", "本物の録音らしさ（realness）。学習済みのモデルが必要"),
    ("music_frac", "音楽が鳴っている割合。AudioSet の分類モデルが必要"),
    ("speech_frac", "声の割合。AudioSet の分類モデルが必要"),
    ("tempo", "テンポ。ビート追跡が必要"),
    ("cuts_on_beat", "拍に乗ったカット。ビート追跡が必要"),
];

fn spec(key: &str) -> &'static Spec {
    SPECS.iter().find(|s| s.key == key).expect("known metric")
}

pub fn group_ja(g: &str) -> &'static str {
    match g {
        "loudness" => "音量",
        "arc" => "展開",
        "ending" => "終わり方",
        "music" => "音楽",
        "density" => "密度",
        "sync" => "映像との同期",
        "spectrum" => "帯域",
        _ => "ステレオ",
    }
}

pub fn verdict_ja(v: &str) -> &'static str {
    match v {
        "launch-grade" => "公開水準",
        "close" => "あと少し",
        "needs work" => "要修正",
        "not launch-grade" => "公開水準に届かない",
        _ => "採点なし",
    }
}

pub fn weight_ja(w: &str) -> &'static str {
    match w {
        "critical" => "重大",
        "major" => "要対応",
        "minor" => "軽微",
        _ => "参考",
    }
}

fn weight_name(w: u8) -> &'static str {
    match w {
        3 => "critical",
        2 => "major",
        1 => "minor",
        _ => "info",
    }
}

// --------------------------------------------------------------------------- small numerics

fn db(v: f64) -> f64 {
    20.0 * v.max(1e-12).log10()
}

fn energy_mean(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    Some(10.0 * (xs.iter().map(|l| 10f64.powf(l / 10.0)).sum::<f64>() / xs.len() as f64).log10())
}

/// numpy's default (linear) percentile.
fn percentile(xs: &[f64], q: f64) -> f64 {
    if xs.is_empty() {
        return f64::NAN;
    }
    let mut v = xs.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    let pos = q / 100.0 * (v.len() - 1) as f64;
    let (i, f) = (pos.floor() as usize, pos - pos.floor());
    if i + 1 < v.len() {
        v[i] + (v[i + 1] - v[i]) * f
    } else {
        v[i]
    }
}

fn round(v: f64, nd: usize) -> f64 {
    crate::geometry::round_to(v, nd)
}

fn argmax(xs: &[f64]) -> usize {
    let mut best = 0;
    for (i, x) in xs.iter().enumerate() {
        if *x > xs[best] {
            best = i;
        }
    }
    best
}

/// `m:ss.ss`.
pub fn fmt_t(t: f64) -> String {
    let t = t.max(0.0);
    let m = (t / 60.0).floor();
    format!("{}:{:05.2}", m as i64, t - m * 60.0)
}

fn times(ts: &[f64], k: usize) -> String {
    let s: Vec<String> = ts.iter().take(k).map(|t| fmt_t(*t)).collect();
    let more = if ts.len() > k { format!("（ほか {} 箇所）", ts.len() - k) } else { String::new() };
    format!("{}{more}", s.join("、"))
}

fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0, 1.0, 1.0);
    while term > 1e-12 * sum {
        term *= (x / (2.0 * k)).powi(2);
        sum += term;
        k += 1.0;
    }
    sum
}

/// scipy's `firwin(taps, cutoff, window=('kaiser', beta))`: cutoff relative to Nyquist, unit DC gain.
fn firwin(taps: usize, cutoff: f64, beta: f64) -> Vec<f64> {
    let m = (taps - 1) as f64 / 2.0;
    let mut h: Vec<f64> = (0..taps)
        .map(|n| {
            let x = n as f64 - m;
            let sinc = if x == 0.0 { 1.0 } else { (std::f64::consts::PI * cutoff * x).sin() / (std::f64::consts::PI * cutoff * x) };
            let r = 2.0 * n as f64 / (taps - 1) as f64 - 1.0;
            cutoff * sinc * bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / bessel_i0(beta)
        })
        .collect();
    let s: f64 = h.iter().sum();
    for v in &mut h {
        *v /= s;
    }
    h
}

/// Periodic Hann window (scipy / librosa `get_window('hann', n)`).
fn hann(n: usize) -> Vec<f64> {
    (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()).collect()
}

/// In-place iterative radix-2 FFT (length a power of two).
fn fft(re: &mut [f64], im: &mut [f64]) {
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
        let ang = -2.0 * std::f64::consts::PI / len as f64;
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
}

/// |FFT|^2 of a windowed real frame (zero-padded to `nfft`), bins 0..=nfft/2.
fn power(frame: &[f64], nfft: usize) -> Vec<f64> {
    let mut re = vec![0.0; nfft];
    re[..frame.len()].copy_from_slice(frame);
    let mut im = vec![0.0; nfft];
    fft(&mut re, &mut im);
    (0..=nfft / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect()
}

// --------------------------------------------------------------------------- measurements

/// BS.1770 K-weighting at 48 kHz (two biquads).
fn kweight(x: &[f32]) -> Vec<f64> {
    let stages = [
        ([1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]),
        ([1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]),
    ];
    let mut y: Vec<f64> = x.iter().map(|v| *v as f64).collect();
    for (b, a) in stages {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        for v in &mut y {
            let x0 = *v;
            let y0 = b[0] * x0 + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2;
            x2 = x1;
            x1 = x0;
            y2 = y1;
            y1 = y0;
            *v = y0;
        }
    }
    y
}

/// Prefix sums of K-weighted energy (both channels), for any block loudness.
struct Loud {
    p: Vec<f64>,
}

impl Loud {
    fn new(pcm: &Pcm) -> Loud {
        let (l, r) = (kweight(&pcm.l), kweight(&pcm.r));
        let mut p = Vec::with_capacity(l.len() + 1);
        p.push(0.0);
        let mut acc = 0.0;
        for i in 0..l.len() {
            acc += l[i] * l[i] + r[i] * r[i];
            p.push(acc);
        }
        Loud { p }
    }

    fn n(&self) -> usize {
        self.p.len() - 1
    }

    /// (block centres, LUFS) of sliding blocks: win 0.4 = momentary, 3.0 = short-term.
    fn curve(&self, win: f64, hop: f64) -> (Vec<f64>, Vec<f64>) {
        let n = self.n();
        let (w, h) = ((win * SR) as usize, ((hop * SR) as usize).max(1));
        if n < w || w == 0 {
            let ms = self.p[n] / n.max(1) as f64;
            return (vec![n as f64 / SR / 2.0], vec![-0.691 + 10.0 * (ms + 1e-20).log10()]);
        }
        let mut t = Vec::new();
        let mut l = Vec::new();
        let mut st = 0;
        while st + w <= n {
            let ms = (self.p[st + w] - self.p[st]) / w as f64;
            t.push((st as f64 + w as f64 / 2.0) / SR);
            l.push(-0.691 + 10.0 * (ms + 1e-20).log10());
            st += h;
        }
        (t, l)
    }

    fn integrated(&self) -> f64 {
        let (_, l) = self.curve(0.4, 0.1);
        let l: Vec<f64> = l.into_iter().filter(|v| *v > -70.0).collect();
        if l.is_empty() {
            return -70.0;
        }
        let rel = energy_mean(&l).unwrap_or(-70.0) - 10.0;
        let l2: Vec<f64> = l.iter().copied().filter(|v| *v > rel).collect();
        energy_mean(&l2).unwrap_or(rel + 10.0)
    }

    fn lra(&self) -> Option<f64> {
        let (_, s) = self.curve(3.0, 0.1);
        let s: Vec<f64> = s.into_iter().filter(|v| *v > -70.0).collect();
        if s.len() < 5 {
            return None;
        }
        let rel = energy_mean(&s)? - 20.0;
        let s: Vec<f64> = s.into_iter().filter(|v| *v > rel).collect();
        (s.len() > 4).then(|| percentile(&s, 95.0) - percentile(&s, 10.0))
    }
}

/// Bring a mix to the integrated loudness the cut ships at (`audio.loudness`), with a peak
/// limiter at `ceiling_db`. The cut lays this mix under the picture as it is, so what is
/// heard in the app, what is checked and what ships are the same. Silence is left as it is.
pub fn normalize(pcm: &mut Pcm, target_lufs: f64, ceiling_db: f64) {
    if pcm.is_empty() {
        return;
    }
    let now = Loud::new(pcm).integrated();
    if now <= -69.5 {
        return;
    }
    let g = 10f64.powf((target_lufs - now) / 20.0) as f32;
    let ceil = 10f64.powf(ceiling_db / 20.0) as f32;
    let release = (-1.0 / (0.05 * SR)).exp() as f32; // 50 ms back to unity
    let mut gr = 1f32;
    for i in 0..pcm.len() {
        let (l, r) = (pcm.l[i] * g, pcm.r[i] * g);
        let need = (ceil / l.abs().max(r.abs()).max(1e-9)).min(1.0);
        gr = need.min(1.0 - (1.0 - gr) * release);
        pcm.l[i] = l * gr;
        pcm.r[i] = r * gr;
    }
}

/// 4x oversampled peak (scipy `resample_poly(x, 4, 1)`: 81-tap Kaiser(5) FIR), in dBTP.
fn true_peak(pcm: &Pcm) -> f64 {
    let mut h = firwin(81, 0.25, 5.0);
    for v in &mut h {
        *v *= 4.0;
    }
    let mut peak = 0f64;
    for ch in [&pcm.l, &pcm.r] {
        let n = ch.len() as i64;
        for i in 0..n {
            for p in 0..4usize {
                let mut acc = 0.0;
                let mut j = 0i64;
                while p + 4 * j as usize <= 80 {
                    let idx = i + 10 - j;
                    if (0..n).contains(&idx) {
                        acc += h[p + 4 * j as usize] * ch[idx as usize] as f64;
                    }
                    j += 1;
                }
                peak = peak.max(acc.abs());
            }
        }
    }
    db(peak)
}

fn mono(pcm: &Pcm) -> Vec<f64> {
    pcm.l.iter().zip(&pcm.r).map(|(a, b)| (*a as f64 + *b as f64) / 2.0).collect()
}

fn frame_rms(m: &[f64], win: usize) -> Vec<f64> {
    let n = m.len() / win;
    if n == 0 {
        return vec![(m.iter().map(|v| v * v).sum::<f64>() / m.len().max(1) as f64 + 1e-20).sqrt()];
    }
    (0..n).map(|f| (m[f * win..(f + 1) * win].iter().map(|v| v * v).sum::<f64>() / win as f64 + 1e-20).sqrt()).collect()
}

/// Energy share per band in dB over the whole file (Welch PSD of the mid signal).
fn bands(m: &[f64]) -> Vec<(&'static str, f64)> {
    let len = m.len();
    let pow2 = ((len.clamp(1024, 65536) as f64).log2().ceil() as u32).clamp(10, 16);
    let nper = (1usize << pow2).min(len);
    let nfft = nper.next_power_of_two();
    let w = hann(nper);
    let step = (nper / 2).max(1);
    let mut psd = vec![0.0; nfft / 2 + 1];
    let mut count = 0;
    let mut st = 0;
    while st + nper <= len {
        let seg = &m[st..st + nper];
        let mean = seg.iter().sum::<f64>() / nper as f64;
        let frame: Vec<f64> = seg.iter().zip(&w).map(|(x, w)| (x - mean) * w).collect();
        for (k, p) in power(&frame, nfft).iter().enumerate() {
            psd[k] += p;
        }
        count += 1;
        st += step;
    }
    for (k, p) in psd.iter_mut().enumerate() {
        *p /= count.max(1) as f64;
        if k != 0 && k != nfft / 2 {
            *p *= 2.0; // one-sided
        }
        *p += 1e-30;
    }
    let tot: f64 = psd.iter().sum();
    let f = |k: usize| k as f64 * SR / nfft as f64;
    [("sub", 20.0, 60.0), ("low", 60.0, 250.0), ("lowmid", 250.0, 1000.0), ("mid", 1000.0, 4000.0),
     ("presence", 4000.0, 8000.0), ("air", 8000.0, 16000.0)]
        .iter()
        .map(|(name, a, b)| {
            let s: f64 = (0..psd.len()).filter(|k| f(*k) >= *a && f(*k) < *b).map(|k| psd[k]).sum();
            (*name, round(10.0 * (s / tot + 1e-12).log10(), 1))
        })
        .collect()
}

/// Median magnitude-weighted spectral centroid of frames within 30 dB of the loudest.
fn brightness(m: &[f64]) -> Option<f64> {
    if m.len() < 1024 {
        return None;
    }
    let w = hann(1024);
    let mut cents = Vec::new();
    let mut energies = Vec::new();
    let mut st = 0;
    while st + 1024 <= m.len() {
        let frame: Vec<f64> = m[st..st + 1024].iter().zip(&w).map(|(x, w)| x * w).collect();
        let a: Vec<f64> = power(&frame, 1024).iter().map(|p| p.sqrt()).collect();
        let e: f64 = a.iter().sum();
        let c = a.iter().enumerate().map(|(k, v)| k as f64 * SR / 1024.0 * v).sum::<f64>() / (e + 1e-20);
        energies.push(e);
        cents.push(c);
        st += 256;
    }
    let emax = energies.iter().cloned().fold(0.0, f64::max);
    let act: Vec<f64> = cents.iter().zip(&energies).filter(|(_, e)| **e >= emax * 10f64.powf(-1.5)).map(|(c, _)| *c).collect();
    (!act.is_empty() && emax > 0.0).then(|| percentile(&act, 50.0).floor())
}

/// librosa-style onset strength on a 24 kHz copy: mel power in dB, half-wave rectified
/// frame difference, mean over bands. Frame i sits at i * 256 / 24000 s.
fn onset_envelope(m48: &[f64]) -> Vec<f64> {
    // 48 kHz -> 24 kHz: Kaiser low-pass at 10.8 kHz, keep every other sample
    let h = firwin(63, 0.45, 8.0);
    let n = m48.len() / 2;
    let y: Vec<f64> = (0..n)
        .map(|k| {
            let c = 2 * k as i64;
            h.iter().enumerate().fold(0.0, |acc, (j, hj)| {
                let idx = c + j as i64 - 31;
                if (0..m48.len() as i64).contains(&idx) { acc + hj * m48[idx as usize] } else { acc }
            })
        })
        .collect();
    // mel filterbank (Slaney scale and norm, 128 bands, 0..12 kHz)
    let hz_to_mel = |f: f64| if f < 1000.0 { f * 3.0 / 200.0 } else { 15.0 + (f / 1000.0).ln() / (6.4f64.ln() / 27.0) };
    let mel_to_hz = |m: f64| if m < 15.0 { m * 200.0 / 3.0 } else { 1000.0 * ((m - 15.0) * (6.4f64.ln() / 27.0)).exp() };
    let n_mels = 128;
    let top = hz_to_mel(OSR / 2.0);
    let mel_f: Vec<f64> = (0..n_mels + 2).map(|i| mel_to_hz(top * i as f64 / (n_mels + 1) as f64)).collect();
    let bins = ONFFT / 2 + 1;
    let freqs: Vec<f64> = (0..bins).map(|k| k as f64 * OSR / ONFFT as f64).collect();
    // each triangle as (first non-zero bin, its weights): the rest of the row is zero, and
    // skipping it gives the same sums at a fraction of the work
    let fb: Vec<(usize, Vec<f64>)> = (0..n_mels)
        .map(|i| {
            let enorm = 2.0 / (mel_f[i + 2] - mel_f[i]);
            let row: Vec<f64> = freqs
                .iter()
                .map(|f| {
                    let lower = (f - mel_f[i]) / (mel_f[i + 1] - mel_f[i]);
                    let upper = (mel_f[i + 2] - f) / (mel_f[i + 2] - mel_f[i + 1]);
                    lower.min(upper).max(0.0) * enorm
                })
                .collect();
            let lo = row.iter().position(|v| *v != 0.0).unwrap_or(0);
            let hi = row.iter().rposition(|v| *v != 0.0).map_or(lo, |p| p + 1);
            (lo, row[lo..hi].to_vec())
        })
        .collect();
    // centred STFT (zero padding), power, mel, dB with an 80 dB floor
    let pad = ONFFT / 2;
    let frames = 1 + y.len() / OHOP;
    let w = hann(ONFFT);
    let mut s = vec![vec![0.0; frames]; n_mels];
    let mut smax = f64::MIN;
    #[allow(clippy::needless_range_loop)] // t indexes the columns of every mel row
    for t in 0..frames {
        let frame: Vec<f64> = (0..ONFFT)
            .map(|i| {
                let idx = (t * OHOP + i) as i64 - pad as i64;
                if (0..y.len() as i64).contains(&idx) { y[idx as usize] * w[i] } else { 0.0 }
            })
            .collect();
        let p = power(&frame, ONFFT);
        for (i, (lo, row)) in fb.iter().enumerate() {
            let e: f64 = row.iter().zip(&p[*lo..]).map(|(a, b)| a * b).sum();
            let d = 10.0 * e.max(1e-10).log10();
            s[i][t] = d;
            smax = smax.max(d);
        }
    }
    for row in &mut s {
        for v in row.iter_mut() {
            *v = v.max(smax - 80.0);
        }
    }
    let shift = 1 + ONFFT / (2 * OHOP); // librosa pads lag + n_fft / (2 hop) frames in front
    let mut env = vec![0.0; frames];
    for t in 1..frames {
        let d = s.iter().map(|row| (row[t] - row[t - 1]).max(0.0)).sum::<f64>() / n_mels as f64;
        let at = t - 1 + shift;
        if at < frames {
            env[at] = d;
        }
    }
    env
}

/// librosa `onset_detect` peak picking on the normalized envelope, in seconds.
fn onset_times(env: &[f64]) -> Vec<f64> {
    if env.is_empty() {
        return Vec::new();
    }
    let lo = env.iter().cloned().fold(f64::MAX, f64::min);
    let x: Vec<f64> = env.iter().map(|v| v - lo).collect();
    let hi = x.iter().cloned().fold(0.0, f64::max);
    if hi <= 0.0 {
        return Vec::new();
    }
    let x: Vec<f64> = x.iter().map(|v| v / hi).collect();
    let per = OSR / OHOP as f64;
    let (pre_max, post_max) = ((0.03 * per) as usize, 1usize);
    let (pre_avg, post_avg) = ((0.10 * per) as usize, (0.10 * per) as usize + 1);
    let wait = (0.03 * per) as usize;
    let delta = 0.07;
    let n = x.len();
    let mut out = Vec::new();
    let mut last: Option<usize> = None;
    for i in 0..n {
        let (a, b) = (i.saturating_sub(pre_max), (i + post_max).min(n));
        let mx = x[a..b].iter().cloned().fold(f64::MIN, f64::max);
        if x[i] < mx {
            continue;
        }
        let (a, b) = (i.saturating_sub(pre_avg), (i + post_avg).min(n));
        let avg = x[a..b].iter().sum::<f64>() / (b - a) as f64;
        if x[i] < avg + delta {
            continue;
        }
        if last.is_some_and(|l| i - l <= wait) {
            continue;
        }
        last = Some(i);
        out.push(i as f64 / per);
    }
    out
}

/// Move onset / hit times to the attack: the steepest rise of the 2 ms level within
/// 60 ms before .. 20 ms after (an editor aligns on the attack).
fn attack_times(ts: &[f64], e2: &[f64]) -> Vec<f64> {
    ts.iter()
        .map(|&t| {
            let a = (((t - 0.06) / E2) as i64).max(1) as usize;
            let b = ((((t + 0.02) / E2) as i64 + 1) as usize).min(e2.len().saturating_sub(1));
            if b < a + 2 {
                return t;
            }
            let d: Vec<f64> = (a..=b).map(|i| e2[i] - e2[i - 1]).collect();
            round((a + argmax(&d)) as f64 * E2, 4)
        })
        .collect()
}

fn env_time(i: usize) -> f64 {
    i as f64 * OHOP as f64 / OSR
}

/// Salient hits: 100 ms loudness jumps of >= 6 dB within 80 ms that land within 8 LU of
/// the integrated level; one per run, timed at the strongest onset frame, moved to the attack.
fn salient_hits(tf: &[f64], lf: &[f64], lufs: f64, env: &[f64], e2: &[f64]) -> Vec<f64> {
    if lf.len() < 20 {
        return Vec::new();
    }
    let h = tf[1] - tf[0];
    let (a, b) = (((0.2 / h).round() as usize).max(1), ((0.08 / h).round() as usize).max(1));
    let n = lf.len();
    let idx: Vec<usize> = (0..n)
        .filter(|&i| {
            let pre = (i as i64 - a as i64..i as i64).map(|k| lf[k.max(0) as usize]).fold(f64::MAX, f64::min);
            let post = (i..i + b).map(|k| lf[k.min(n - 1)]).fold(f64::MIN, f64::max);
            post - pre >= 6.0 && post >= lufs - 8.0
        })
        .collect();
    let mut runs: Vec<Vec<usize>> = Vec::new();
    for i in idx {
        match runs.last_mut() {
            Some(r) if i - r[r.len() - 1] <= 2 => r.push(i),
            _ => runs.push(vec![i]),
        }
    }
    let mut out: Vec<f64> = Vec::new();
    for run in runs {
        let (t0, t1) = (tf[run[0]] - 0.05, tf[run[run.len() - 1]] + 0.1);
        let sel: Vec<usize> = (0..env.len()).filter(|&i| env_time(i) >= t0 && env_time(i) <= t1).collect();
        let t = if sel.is_empty() {
            tf[run[0]] + 0.03
        } else {
            env_time(sel[argmax(&sel.iter().map(|&i| env[i]).collect::<Vec<_>>())])
        };
        if out.last().map_or(true, |l| t - l > 0.1) {
            out.push(t);
        }
    }
    attack_times(&out, e2)
}

/// Short dips >= 10 dB below the median of the preceding 2 s, 0.15-1.5 s long, back to
/// within 3 dB of it within 0.3 s: (start, length, depth).
fn stopdowns(tf: &[f64], lf: &[f64]) -> Vec<(f64, f64, f64)> {
    if lf.len() < 3 {
        return Vec::new();
    }
    let h = tf[1] - tf[0];
    let w = (2.0 / h).round() as usize;
    let n = lf.len();
    if n <= w {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut i = w;
    while i < n {
        let r = percentile(&lf[i - w..i], 50.0);
        if lf[i] < r - 10.0 {
            let mut j = i;
            while j < n && lf[j] < r - 10.0 {
                j += 1;
            }
            let ln = (j - i) as f64 * h;
            let back = (j < n) && lf[j..(j + (0.3 / h).round() as usize + 1).min(n)].iter().cloned().fold(f64::MIN, f64::max) >= r - 3.0;
            if (0.15..=1.5).contains(&ln) && back {
                let depth = r - lf[i..j].iter().cloned().fold(f64::MAX, f64::min);
                out.push((round(tf[i] - 0.05, 3), round(ln, 3), round(depth, 1)));
            }
            i = j + (0.5 / h).round() as usize;
        } else {
            i += 1;
        }
    }
    out
}

fn near(events: &[f64], targets: &[f64], tol: f64) -> Vec<bool> {
    events.iter().map(|e| targets.iter().any(|t| (t - e).abs() <= tol)).collect()
}

fn near_frac(events: &[f64], targets: &[f64], tol: f64) -> Option<f64> {
    if events.is_empty() {
        return None;
    }
    let hits = near(events, targets, tol).iter().filter(|b| **b).count();
    Some(hits as f64 / events.len() as f64)
}

/// The same share for the events shifted off the picture: what chance scores.
fn chance_frac(events: &[f64], targets: &[f64], tol: f64, dur: f64) -> Option<f64> {
    if events.is_empty() || dur <= 0.0 {
        return None;
    }
    let v: Vec<f64> = SHIFTS
        .iter()
        .filter_map(|s| near_frac(&events.iter().map(|e| (e + s).rem_euclid(dur)).collect::<Vec<_>>(), targets, tol))
        .collect();
    Some(v.iter().sum::<f64>() / v.len() as f64)
}

/// Refine the start (first) or end of sound found with a 100 ms block centred at t.
fn edge(e2: &[f64], t: f64, first: bool) -> f64 {
    let a = (((t - 0.05) / E2) as i64).max(0) as usize;
    let b = ((((t + 0.05) / E2) as i64 + 1) as usize).min(e2.len());
    if b <= a {
        return t.max(0.0);
    }
    let w = (0.15 / E2) as usize;
    let rf = if first { &e2[a..(a + w).min(e2.len())] } else { &e2[b.saturating_sub(w)..b] };
    let r = rf.iter().cloned().fold(f64::MIN, f64::max);
    let ok: Vec<usize> = (a..b).filter(|&i| e2[i] >= r - 20.0).collect();
    match (ok.first(), ok.last()) {
        (Some(f), Some(l)) => round(if first { *f } else { l + 1 } as f64 * E2, 3),
        _ => t.max(0.0),
    }
}

fn longest_run(mask: impl Iterator<Item = bool>) -> usize {
    let (mut best, mut cur) = (0, 0);
    for b in mask {
        cur = if b { cur + 1 } else { 0 };
        best = best.max(cur);
    }
    best
}

// --------------------------------------------------------------------------- the check

/// A picture cut: the boundary between two shots, in teaser seconds.
#[derive(Debug, Clone)]
pub struct Cut {
    pub t: f64,
    pub from: String,
    pub to: String,
    pub to_beat: Option<String>,
}

struct Measured {
    dur: f64,
    v: Map<String, Value>,
    chance: Map<String, Value>,
    arc: Option<Vec<Option<f64>>>,
    onsets: Vec<f64>,
    hits: Vec<f64>,
    stopdowns: Vec<(f64, f64, f64)>,
    tf: Vec<f64>,
    lf: Vec<f64>,
    tm: Vec<f64>,
    lm: Vec<f64>,
    ts: Vec<f64>,
    ls: Vec<f64>,
    lufs: f64,
    mono: bool,
}

fn put(v: &mut Map<String, Value>, key: &str, x: Option<f64>) {
    let nd = spec(key).decimals;
    v.insert(key.into(), match x.filter(|x| x.is_finite()) {
        Some(x) if nd == 0 => json!(x.floor() as i64),
        Some(x) => json!(round(x, nd)),
        None => Value::Null,
    });
}

fn get(v: &Map<String, Value>, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64)
}

fn measure(pcm: &Pcm, cuts: &[Cut], fps: f64) -> Measured {
    let n = pcm.len();
    let dur = n as f64 / SR;
    let loud = Loud::new(pcm);
    let m = mono(pcm);
    let mut v = Map::new();
    for s in SPECS {
        v.insert(s.key.into(), Value::Null);
    }

    let lufs = if n >= (0.4 * SR) as usize { round(loud.integrated(), 1) } else { -70.0 };
    let st_max = (n >= (3.0 * SR) as usize).then(|| loud.curve(3.0, 0.5).1.into_iter().fold(f64::MIN, f64::max));
    let lra = if n >= (6.0 * SR) as usize { loud.lra() } else { None };
    let (side, mid): (Vec<f64>, Vec<f64>) =
        pcm.l.iter().zip(&pcm.r).map(|(a, b)| ((*a as f64 - *b as f64) / 2.0, (*a as f64 + *b as f64) / 2.0)).unzip();
    let rms = |x: &[f64]| (x.iter().map(|v| v * v).sum::<f64>() / x.len().max(1) as f64).sqrt();
    let sm = rms(&side);
    let side_to_mid = (sm > 1e-7).then(|| round(db(sm) - db(rms(&mid) + 1e-12), 1));
    let std = |x: &[f32]| {
        let mu = x.iter().map(|v| *v as f64).sum::<f64>() / x.len().max(1) as f64;
        (x.iter().map(|v| (*v as f64 - mu).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
    };
    let corr = (std(&pcm.l) > 1e-9 && std(&pcm.r) > 1e-9).then(|| {
        let (ml, mr) = (pcm.l.iter().map(|v| *v as f64).sum::<f64>() / n as f64, pcm.r.iter().map(|v| *v as f64).sum::<f64>() / n as f64);
        let cov = pcm.l.iter().zip(&pcm.r).map(|(a, b)| (*a as f64 - ml) * (*b as f64 - mr)).sum::<f64>() / n as f64;
        cov / (std(&pcm.l) * std(&pcm.r))
    });
    let mono_mix = side_to_mid.is_none() && lufs > -69.5;

    put(&mut v, "lufs_i", (n >= (0.4 * SR) as usize).then_some(lufs));
    put(&mut v, "true_peak", (n > 0).then(|| round(true_peak(pcm), 1)));
    put(&mut v, "lra", lra.map(|x| round(x, 1)));
    put(&mut v, "correlation", corr);
    put(&mut v, "side_to_mid_db", if mono_mix { Some(MONO_DB) } else { side_to_mid });
    put(&mut v, "brightness_hz", brightness(&m));
    for (name, x) in bands(&m) {
        put(&mut v, &format!("band_{name}"), Some(x));
    }
    if let Some(s) = st_max {
        put(&mut v, "crest_lu", Some(round(s, 1) - lufs));
    }

    let (tf, lf) = loud.curve(0.1, 0.02);
    let (tm, lm) = loud.curve(0.4, 0.1);
    let (ts, ls) = loud.curve(3.0, 0.1);
    let e2: Vec<f64> = frame_rms(&m, (E2 * SR) as usize).iter().map(|r| 20.0 * (r + 1e-9).log10()).collect();
    let env = onset_envelope(&m);
    let onsets = attack_times(&onset_times(&env), &e2);
    let hits = salient_hits(&tf, &lf, lufs, &env, &e2);
    let sd = stopdowns(&tf, &lf);
    let i_ = lufs;

    if lm.len() > 2 {
        let win = (0.1 * dur).max(0.5);
        let sel = |f: &dyn Fn(f64) -> bool| lm.iter().zip(&tm).filter(|(_, t)| f(**t)).map(|(l, _)| *l).collect::<Vec<_>>();
        put(&mut v, "intro_lu", energy_mean(&sel(&|t| t <= win)).map(|e| e - i_));
        put(&mut v, "end_lu", energy_mean(&sel(&|t| t >= dur - win)).map(|e| e - i_));
    }
    if ls.len() > 1 && dur >= 6.0 {
        put(&mut v, "peak_pos", Some(ts[argmax(&ls)] / dur));
    }
    let arc = (lm.len() > 2).then(|| {
        let w = (0.05 * dur).max(1.5);
        (0..11)
            .map(|k| {
                let c = k as f64 / 10.0 * dur;
                let sel: Vec<f64> = lm.iter().zip(&tm).filter(|(_, t)| (**t - c).abs() <= w).map(|(l, _)| *l).collect();
                energy_mean(&sel).map(|e| round(e - i_, 1))
            })
            .collect::<Vec<_>>()
    });
    let loud_i: Vec<usize> = (0..lf.len()).filter(|&i| lf[i] > i_ - 20.0).collect();
    let audible: Vec<usize> = (0..lf.len()).filter(|&i| lf[i] > i_ - 30.0).collect();
    let first_sound = loud_i.first().map(|&i| edge(&e2, tf[i], true));
    put(&mut v, "first_sound_s", first_sound);
    if let Some(&last) = audible.last() {
        let t_end = dur.min(edge(&e2, tf[last], false));
        put(&mut v, "end_silence_s", Some((dur - t_end).max(0.0)));
        if let Some(k) = (0..lm.len()).rev().find(|&k| lm[k] >= i_ - 6.0 && tm[k] <= t_end) {
            put(&mut v, "fade_s", Some((t_end - tm[k] - 0.2).max(0.0)));
        }
        let fs = first_sound.unwrap_or(0.0);
        let inside: Vec<usize> = (0..lf.len()).filter(|&i| tf[i] >= fs && tf[i] <= t_end).collect();
        let gap = if inside.len() > 1 {
            longest_run(inside.iter().map(|&i| lf[i] < i_ - 30.0)) as f64 * (tf[1] - tf[0])
        } else {
            0.0
        };
        put(&mut v, "max_gap_s", Some(gap));
        let last3: Vec<f64> = (0..lm.len()).filter(|&k| tm[k] >= t_end - 3.0 && tm[k] <= t_end).map(|k| lm[k]).collect();
        if !last3.is_empty() {
            put(&mut v, "final_lu", Some(last3.iter().cloned().fold(f64::MIN, f64::max) - i_));
        }
        // the final hit's ring-out: the strongest onset of the last 4 s, if it is a real hit
        let strength: Vec<f64> = onsets.iter().map(|o| env[((o / env_time(1)).round() as usize).min(env.len().saturating_sub(1))]).collect();
        let sel: Vec<usize> = (0..onsets.len()).filter(|&j| onsets[j] >= t_end - 4.0 && onsets[j] < t_end).collect();
        if !sel.is_empty() {
            let j = sel[argmax(&sel.iter().map(|&j| strength[j]).collect::<Vec<_>>())];
            if strength[j] >= percentile(&strength, 75.0) {
                put(&mut v, "tail_s", Some(t_end - onsets[j]));
            }
        }
    }
    if dur > 0.0 {
        put(&mut v, "onset_rate", Some(onsets.len() as f64 / dur));
        put(&mut v, "hit_rate", Some(hits.len() as f64 / dur));
    }
    if dur >= 6.0 {
        put(&mut v, "stopdowns_min", Some(sd.len() as f64 / dur * 60.0));
    }
    if dur < SHORT_S {
        for k in SHAPE {
            v.insert(k.to_string(), Value::Null);
        }
    }
    let ct: Vec<f64> = cuts.iter().map(|c| c.t).collect();
    let mut chance = Map::new();
    if dur > 0.0 {
        put(&mut v, "cut_rate", Some(ct.len() as f64 / dur));
    }
    if ct.len() >= 3 {
        let tol_f = 1.0 / fps.max(1.0) + 0.006;
        for (key, targets, tol) in [("cuts_with_sound", &onsets, 0.1), ("cut_hit_frac", &hits, 0.1), ("cut_frame_sync", &onsets, tol_f)] {
            put(&mut v, key, near_frac(&ct, targets, tol));
            chance.insert(key.into(), json!(chance_frac(&ct, targets, tol, dur).map(|c| round(c, 2))));
        }
    }
    Measured { dur, v, chance, arc, onsets, hits, stopdowns: sd, tf, lf, tm, lm, ts, ls, lufs, mono: mono_mix }
}

fn fmt_value(key: &str, v: Option<f64>) -> String {
    let Some(v) = v else { return "n.a.".into() };
    let s = spec(key);
    match s.unit {
        "share" => format!("{:.0}%", 100.0 * v),
        "0-1" | "r" => format!("{v:.prec$}", prec = s.decimals),
        u => format!("{v:.prec$} {u}", prec = s.decimals),
    }
}

/// (lo, hi, basis) for a metric: practice window, or the reference hull widened by the tolerance.
fn window(s: &Spec) -> (Option<f64>, Option<f64>, &'static str) {
    if let Some((lo, hi)) = s.practice {
        return (lo, hi, "web");
    }
    let Some(st) = stat(s.key) else { return (None, None, "no reference data") };
    let mut lo = Some(st.min - s.tol);
    let hi = st.max + s.tol;
    if lo.is_some_and(|l| l <= 0.0) && ["s", "/s", "/min", "share"].contains(&s.unit) {
        lo = (s.unit == "/s" && st.min > 0.0).then(|| round(st.min / 2.0, 3));
    }
    let lo = if matches!(s.sides, Sides::Both | Sides::Lower) { lo } else { None };
    let hi = if matches!(s.sides, Sides::Both | Sides::Upper) { Some(hi) } else { None };
    (lo, hi, "references")
}

fn range_txt(key: &str, lo: Option<f64>, hi: Option<f64>) -> String {
    match (lo, hi) {
        (Some(l), Some(h)) => format!("{} .. {}", fmt_value(key, Some(l)), fmt_value(key, Some(h))),
        (Some(l), None) => format!(">= {}", fmt_value(key, Some(l))),
        (None, Some(h)) => format!("<= {}", fmt_value(key, Some(h))),
        _ => String::new(),
    }
}

struct Action {
    w: u8,
    dist: f64,
    metric: &'static str,
    text: String,
    at: Option<f64>,
}

/// Concrete fixes for one out-of-range metric (in Japanese).
fn actions(key: &'static str, st: &str, v: f64, me: &Measured, cuts: &[Cut]) -> Vec<(String, Option<f64>)> {
    let rng = stat(key).map(|s| format!("基準 {} .. {}", fmt_value(key, Some(s.min)), fmt_value(key, Some(s.max)))).unwrap_or_default();
    let low = st == "low";
    let dur = me.dur;
    let one = |s: String| vec![(s, None)];
    match key {
        "lufs_i" if low => one(format!(
            "統合ラウドネスを {:+.1} dB 上げて -14 LUFS 前後（Web）にする。Cut はミックスを `audio.loudness` の値にそろえるので、`audio.loudness` を -14 にする（`audio.gain` では変わらない）。リミッターが強くかかるなら先に音楽を圧縮する（{rng}）",
            -14.0 - v)),
        "lufs_i" => one(format!(
            "統合ラウドネスを {:.1} dB 下げて -12 LUFS 前後にする（`audio.loudness` を -12 に）。-9 LUFS を超えると配信側で下げられ、リミッターがヒットを全部つぶす（{rng}）",
            v + 12.0)),
        "true_peak" => one(format!(
            "トゥルーピークが {v:+.1} dBTP。`audio.loudness` を {:.1} dB 下げるか、ピークの強い SE の `gain` を下げる",
            v + 1.0)),
        "cuts_with_sound" => {
            let ct: Vec<f64> = cuts.iter().map(|c| c.t).collect();
            let miss: Vec<&Cut> = cuts.iter().zip(near(&ct, &me.onsets, 0.1)).filter(|(_, ok)| !ok).map(|(c, _)| c).collect();
            let names: Vec<String> = miss.iter().take(6).map(|c| format!("{}（{} → {}）", fmt_t(c.t), c.from, c.to)).collect();
            vec![(format!(
                "{} 本のカットのうち {} 本で、±100 ms 以内に音の立ち上がりがない（{rng}）: {}。カットに向かうウーッシュ（ピークをカットに合わせる）か、ヒット・クリックを置く",
                cuts.len(), miss.len(), names.join("、")), miss.first().map(|c| c.t))]
        }
        "lra" | "crest_lu" => {
            let body: Vec<usize> = (0..me.ts.len()).filter(|&i| me.ts[i] > 0.1 * dur && me.ts[i] < 0.9 * dur).collect();
            if st == "high" && !body.is_empty() {
                let iq = body[argmax(&body.iter().map(|&i| -me.ls[i]).collect::<Vec<_>>())];
                let il = argmax(&me.ls);
                let quiet = format!("{}–{}（短期 {:.1} LUFS）", fmt_t(me.ts[iq] - 1.5), fmt_t(me.ts[iq] + 1.5), me.ls[iq]);
                if key == "lra" {
                    return vec![(format!(
                        "音量が {v:.1} LU 振れている（LRA、{rng}）。いちばん静かな本編は {quiet}、いちばん大きい3秒は {} 付近（{:.1} LUFS）。静かなところを上げるか、音楽を 2–4 dB 圧縮して本編を保つ",
                        fmt_t(me.ts[il]), me.ls[il]), Some(me.ts[iq]))];
                }
                return vec![(format!(
                    "いちばん大きい3秒（{} 付近、{:.1} LUFS）が平均より {v:.1} LU 大きく、ほかが小さく聞こえる（{rng}）。本編（静かなところ: {quiet}）を上げるか、そのピークを抑える",
                    fmt_t(me.ts[il]), me.ls[il]), Some(me.ts[il]))];
            }
            if key == "lra" {
                one(format!("ラウドネスレンジが {v:.1} LU しかなく、音量が動かない（{rng}）。静かな区間や、見せ場に向けた盛り上がりを作る"))
            } else {
                one(format!("いちばん大きい3秒が平均より {v:.1} LU しか大きくない（{rng}）。見せ場やロゴのヒットを本編より 2–3 LU 上げる"))
            }
        }
        "first_sound_s" => vec![(format!(
            "{} まで本編の音量に届かない（{rng}）。0:00 から音楽か環境音、ライザーを鳴らす（BGM なら `audio.bgm.start: 0`、`fade_in` を短く）",
            fmt_t(v)), Some(0.0))],
        "end_silence_s" => vec![(format!(
            "{} で音が終わり、そのあと {v:.1} 秒の無音がある。最後のスティングをエンドカードの上で鳴らし残すか、BGM を最後まで伸ばす（{rng}）",
            fmt_t(dur - v)), Some(dur - v))],
        "max_gap_s" => {
            let h = me.tf[1] - me.tf[0];
            let mut gaps = Vec::new();
            let mut run: Vec<f64> = Vec::new();
            let fs = get(&me.v, "first_sound_s").unwrap_or(0.0);
            let es = get(&me.v, "end_silence_s").unwrap_or(0.0);
            for (t, l) in me.tf.iter().zip(&me.lf) {
                if *l < me.lufs - 30.0 {
                    run.push(*t);
                } else if !run.is_empty() {
                    if run.len() as f64 * h >= 0.5 && run[0] > fs && *t < dur - es {
                        gaps.push((run[0] - 0.05, run[run.len() - 1] + 0.05));
                    }
                    run.clear();
                }
            }
            let where_: Vec<String> = gaps.iter().take(6).map(|(a, b)| format!("{}–{}", fmt_t(*a), fmt_t(*b))).collect();
            vec![(format!(
                "途中に無音の穴がある（最長 {v:.1} 秒、{rng}）: {}。BGM か環境音を下に敷くか、ヒットで終わる意図したストップダウンにする",
                where_.join("、")), gaps.first().map(|g| g.0))]
        }
        "final_lu" if low => {
            let last = cuts.last().map(|c| c.t);
            vec![(format!(
                "最後の瞬間が本編より {:.1} LU 小さい。フェードで消さず、ロゴのスティングかヒットで終える{}（{rng}）",
                -v, last.map(|t| format!("（最後のカット {} から）", fmt_t(t))).unwrap_or_default()), last)]
        }
        "final_lu" => one(format!("最後のヒットが本編より {v:.1} LU 大きく飛び出す。2–3 dB 下げる（{rng}）")),
        "onset_rate" if low => one(format!(
            "音がまばら（{v:.1} 回/秒、{rng}）。リズムのある曲を使うか、画面の動きにクリックや足音などの SE を足す")),
        "onset_rate" => one(format!("音が多すぎる（{v:.1} 回/秒、{rng}）。SE を減らし、大事な瞬間だけにアクセントを残す")),
        "hit_rate" if low => {
            let cand: Vec<&Cut> = cuts.iter().filter(|c| !me.hits.iter().any(|h| (h - c.t).abs() <= 0.1)).collect();
            let at = if cand.is_empty() {
                String::new()
            } else {
                format!("。ヒットのないカット: {}", times(&cand.iter().map(|c| c.t).collect::<Vec<_>>(), 6))
            };
            vec![(format!(
                "目立つヒットが{}（{v:.2} 回/秒、{rng}）。見せ場にインパクト、サブドロップ、鋭いクリックを置く{at}",
                if v == 0.0 { "ない" } else { "少ない" }), cand.first().map(|c| c.t))]
        }
        "hit_rate" => one(format!("目立つヒットが多すぎる（{v:.2} 回/秒、{rng}）。見せ場の分だけ残し、ほかは 3–6 dB 下げる")),
        "intro_lu" | "end_lu" => {
            let (span, part, at) = if key == "intro_lu" {
                (format!("0:00–{}", fmt_t(0.1 * dur)), "冒頭", 0.0)
            } else {
                (format!("{}–{}", fmt_t(0.9 * dur), fmt_t(dur)), "最後", 0.9 * dur)
            };
            let text = if low {
                format!("{part}（{span}）が本編より {:.1} LU 小さい（{rng}）。{}", -v,
                        if key == "intro_lu" { "最初から本編に近い音量で始めるか、ビルドアップを短くする" } else { "エンドカードまでエネルギーを保ち、スティングで終える" })
            } else {
                format!("{part}（{span}）が本編より {v:.1} LU 大きい（{rng}）。そこを下げるか本編を上げる")
            };
            vec![(text, Some(at))]
        }
        "fade_s" if st == "high" => vec![(format!(
            "{v:.1} 秒のフェードアウト（{rng}）。短い余韻のスティングで終えるか、最後のカットでハードに切る（`audio.bgm.fade_out` を短く）"),
            Some(dur - v))],
        "fade_s" => one(format!("終わりが急に切れている。最後のヒットを少し鳴り残す（{rng}）")),
        k if k.starts_with("band_") => {
            let (lo_fix, hi_fix) = match k {
                "band_sub" => ("下が薄い。見せ場にサブ（サブドロップやキック）を足すか、低域のある曲を選ぶ",
                               "20–60 Hz が多すぎて余裕を食い、小さいスピーカーで歪む。SE と環境音を 30–40 Hz でハイパスし、音楽のサブを 2–4 dB 下げる"),
                "band_low" => ("低域が弱く、音楽に胴がない。厚い曲を選ぶか低い層（ベース、低いヒット）を足す",
                               "ブーミー。音楽と重い SE の 100–200 Hz を 2–3 dB 削る"),
                "band_lowmid" => ("スカスカ。250 Hz–1 kHz が抜けている。音楽で持ち上げるか厚みのある素材を使う",
                                  "こもって箱鳴りしている。音楽と響きの多い SE の 300–500 Hz を 2–3 dB 削る"),
                "band_mid" => ("ミッドが引っ込み、クリックや声がスマホで抜けない。SE の 1–3 kHz を持ち上げる",
                               "ミッドがきつい。いちばん大きい層の 1–3 kHz を 2 dB 削る"),
                "band_presence" => ("こもって鈍い。トランジェントに噛みつきがない。SE を明るくするか明るい素材を使う",
                                    "耳に痛い。いちばん明るい層の 5–7 kHz を削るかディエッサーをかける"),
                _ => ("エアがなく、こもった低ビットレートの音に聞こえる。帯域の広い素材（44.1 / 48 kHz、ローパスなし）を使う",
                      "シャリシャリする。いちばんノイズの多い層をローパスかノイズ除去する"),
            };
            one(format!("{}（割合 {}、{rng}）", if low { lo_fix } else { hi_fix }, fmt_value(key, Some(v))))
        }
        "side_to_mid_db" if low => one(if me.mono {
            format!("ミックスがモノラル（左右が同じ）。ステレオの音楽と環境音を使い、SE を動きに合わせて振る（{rng}）")
        } else {
            format!("ほぼモノラル（サイドがミッドより {v:.1} dB）。ステレオの音楽と環境音を使い、SE を動きに合わせて振る（{rng}）")
        }),
        "side_to_mid_db" => one(format!("広すぎる（サイドがミッドより {v:.1} dB）。モノラルで崩れないか確かめ、広い環境音やリバーブを狭める（{rng}）")),
        _ => Vec::new(),
    }
}

/// Mix + cut list -> the scorecard as JSON (see the module docs for what is measured).
pub fn check(pcm: &Pcm, cuts: &[Cut], fps: f64) -> Value {
    let dur = pcm.len() as f64 / SR;
    let base = |verdict: &str, msg: String| {
        json!({
            "version": CHECK_VERSION, "duration": round(dur, 3), "verdict": verdict, "verdict_ja": verdict_ja(verdict),
            "score": Value::Null, "in_range": 0, "scored": 0, "critical_issues": 0, "major_issues": 0,
            "partial": [], "actions": [], "tips": [], "metrics": [], "not_measured": [], "summary": msg,
        })
    };
    if dur < MIN_S {
        return base("n.a.", format!("ミックスが {dur:.2} 秒しかない（{MIN_S:.0} 秒未満は採点しない）"));
    }
    let me = measure(pcm, cuts, fps);
    if me.lufs <= -69.5 {
        return base("n.a.", "ミックスが無音（-70 LUFS 未満）。BGM か SE を置いてください".into());
    }

    let mut rows = Vec::new();
    let mut acts: Vec<Action> = Vec::new();
    let (mut wsum, mut wok, mut n_crit, mut n_major, mut in_range, mut scored) = (0u32, 0u32, 0, 0, 0, 0);
    let skip: Vec<&str> = NEEDS_MODEL.iter().map(|(k, _)| *k).collect();
    for s in SPECS {
        if skip.contains(&s.key) {
            continue;
        }
        let (lo, hi, basis) = window(s);
        let v = get(&me.v, s.key);
        let mut w = s.weight;
        let status = if basis == "no reference data" {
            "n.a."
        } else if w == 0 {
            if v.is_none() { "n.a." } else { "info" }
        } else {
            match v {
                None => "n.a.",
                Some(x) if lo.is_some_and(|l| x < l) => "low",
                Some(x) if hi.is_some_and(|h| x > h) => "high",
                Some(_) => "ok",
            }
        };
        if w == 3 && (status == "low" || status == "high") {
            let edge = if status == "low" { lo } else { hi }.unwrap_or(0.0);
            if (v.unwrap_or(0.0) - edge).abs() <= 2.0 * s.tol {
                w = 2; // a near miss of a critical rule counts as major
            }
        }
        let st = stat(s.key);
        let mut row = json!({
            "metric": s.key, "label": s.label, "group": s.group, "group_ja": group_ja(s.group), "value": v,
            "shown": fmt_value(s.key, v), "unit": s.unit, "range": [lo, hi], "range_text": range_txt(s.key, lo, hi),
            "basis": basis, "median": st.as_ref().map(|x| x.median), "median_text": st.as_ref().map(|x| fmt_value(s.key, Some(x.median))),
            "typical": st.as_ref().map(|x| json!([x.p10, x.p90])), "status": status, "weight": weight_name(w),
            "weight_ja": weight_ja(weight_name(w)), "why": s.why, "refs": st.as_ref().map(|x| x.n),
        });
        if let Some(c) = me.chance.get(s.key) {
            row["chance"] = c.clone();
        }
        if w > 0 && status != "n.a." {
            scored += 1;
            wsum += w as u32;
            if status == "ok" {
                in_range += 1;
                wok += w as u32;
            } else {
                if w == 3 {
                    n_crit += 1;
                }
                if w == 2 {
                    n_major += 1;
                }
                let edge = if status == "low" { lo } else { hi }.unwrap_or(0.0);
                let dist = (v.unwrap_or(0.0) - edge).abs() / s.tol.max(1e-3);
                let list = actions(s.key, status, v.unwrap_or(0.0), &me, cuts);
                row["actions"] = json!(list.iter().map(|(t, _)| t).collect::<Vec<_>>());
                for (text, at) in list {
                    acts.push(Action { w, dist, metric: s.key, text, at });
                }
            }
        }
        rows.push(row);
    }
    acts.sort_by(|a, b| b.w.cmp(&a.w).then(b.dist.total_cmp(&a.dist)));
    let score = (wsum > 0).then(|| wok as f64 / wsum as f64);
    let verdict = match score {
        None => "n.a.",
        Some(s) if s >= 0.9 && n_crit == 0 => "launch-grade",
        Some(s) if s >= 0.75 && n_crit == 0 => "close",
        Some(s) if s >= 0.5 => "needs work",
        Some(_) => "not launch-grade",
    };

    // tips: not scored
    let mut tips = Vec::new();
    let ct: Vec<f64> = cuts.iter().map(|c| c.t).collect();
    let flagged: Vec<&str> = acts.iter().map(|a| a.metric).collect();
    let miss: Vec<f64> = ct.iter().zip(near(&ct, &me.onsets, 0.1)).filter(|(_, ok)| !ok).map(|(t, _)| *t).collect();
    if !miss.is_empty() && !flagged.contains(&"cuts_with_sound") {
        tips.push(format!(
            "{} 本のカットのうち {} 本に音の立ち上がりがない: {}。大事なカットにはウーッシュ・ヒット・クリックを、1本は意図したストップダウンにしてもよい",
            ct.len(), miss.len(), times(&miss, 6)));
    }
    if get(&me.v, "stopdowns_min") == Some(0.0) && !cuts.is_empty() {
        let big = cuts.iter().find(|c| c.to_beat.as_deref() == Some("title")).or(cuts.last()).expect("non-empty");
        tips.push(format!(
            "ストップダウンがない。基準7本のうち4本は、大きなヒットの直前に 0.2–1 秒だけ音を抜く。{} → {} のカット（{}）の手前で試す",
            big.from, big.to, fmt_t(big.t)));
    }

    let first = acts.first().map(|a| a.text.clone());
    let pct = score.map(|s| (s * 100.0).round()).unwrap_or(0.0);
    let summary = format!(
        "{}（{} / {} 指標が範囲内、重み付き {pct:.0}%、重大 {n_crit}・要対応 {n_major}）。{}",
        verdict_ja(verdict), in_range, scored,
        first.map(|f| format!("まず直す: {f}")).unwrap_or_else(|| "基準に対して直すところはありません".into())
    );
    let not_measured: Vec<Value> = NEEDS_MODEL
        .iter()
        .map(|(k, why)| json!({"metric": k, "why": why}))
        .collect();
    let (t0, step) = (me.tm.first().copied().unwrap_or(0.0), 0.1);
    json!({
        "version": CHECK_VERSION,
        "source": "モデルを使わない検証",
        "profile": "ローンチ映像 7 本のプロファイル",
        "duration": round(me.dur, 3),
        "verdict": verdict, "verdict_ja": verdict_ja(verdict),
        "score": score.map(|s| round(s, 3)), "in_range": in_range, "scored": scored,
        "critical_issues": n_crit, "major_issues": n_major,
        "partial": ["realness", "music presence"],
        "summary": summary,
        "actions": acts.iter().map(|a| json!({
            "weight": weight_name(a.w), "weight_ja": weight_ja(weight_name(a.w)), "metric": a.metric,
            "label": spec(a.metric).label, "action": a.text, "at": a.at.map(|t| round(t.max(0.0), 2)),
        })).collect::<Vec<_>>(),
        "tips": tips,
        "metrics": rows,
        "not_measured": not_measured,
        "events": {
            "cuts": cuts.iter().zip(near(&ct, &me.onsets, 0.1)).map(|(c, ok)| json!({
                "t": round(c.t, 3), "from": c.from, "to": c.to, "onset": ok,
                "hit": me.hits.iter().any(|h| (h - c.t).abs() <= 0.1),
            })).collect::<Vec<_>>(),
            "hits": me.hits.iter().map(|h| round(*h, 3)).collect::<Vec<_>>(),
            "onsets": me.onsets.len(),
            "stopdowns": me.stopdowns.iter().map(|(t, l, d)| json!({"t": t, "length": l, "depth_db": d})).collect::<Vec<_>>(),
        },
        "arc": {"film": me.arc, "refs_median": ARC_MEDIAN, "refs_p10": ARC_P10, "refs_p90": ARC_P90},
        "integrated": me.lufs,
        "curve": {
            "t0": round(t0, 3), "step": step,
            "lufs": me.lm.iter().map(|l| round(l.max(-60.0), 1)).collect::<Vec<_>>(),
        },
    })
}

/// The scorecard as plain text, for the CLI.
pub fn text(r: &Value) -> String {
    let mut lines = vec![format!(
        "sound check: {} ({} s) — {}",
        r["verdict_ja"].as_str().unwrap_or(""), r["duration"], r["summary"].as_str().unwrap_or("")
    )];
    if let Some(a) = r["actions"].as_array().filter(|a| !a.is_empty()) {
        lines.push("まず直す（重要な順）:".into());
        for (i, x) in a.iter().enumerate() {
            lines.push(format!("  {}. [{}] {}", i + 1, x["weight_ja"].as_str().unwrap_or(""), x["action"].as_str().unwrap_or("")));
        }
    }
    for t in r["tips"].as_array().into_iter().flatten() {
        lines.push(format!("  ヒント: {}", t.as_str().unwrap_or("")));
    }
    let rows = r["metrics"].as_array().cloned().unwrap_or_default();
    if !rows.is_empty() {
        lines.push("採点表:".into());
    }
    for row in rows.iter().filter(|x| x["weight"] != "info") {
        let st = row["status"].as_str().unwrap_or("");
        let mark = match st {
            "ok" => "ok".to_string(),
            "n.a." => "n.a.".to_string(),
            s => format!("{s} ({})", row["weight_ja"].as_str().unwrap_or("")),
        };
        lines.push(format!(
            "  {:<28} {:>12}   {:<24} {}",
            row["label"].as_str().unwrap_or(""), row["shown"].as_str().unwrap_or(""), row["range_text"].as_str().unwrap_or(""), mark
        ));
    }
    let nm: Vec<&str> = r["not_measured"].as_array().into_iter().flatten().filter_map(|x| x["metric"].as_str()).collect();
    if !nm.is_empty() {
        lines.push(format!("未計測（モデルが必要）: {}", nm.join(", ")));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f64, secs: f64, amp: f64) -> Pcm {
        let n = (secs * SR) as usize;
        let l: Vec<f32> = (0..n).map(|i| (amp * (2.0 * std::f64::consts::PI * hz * i as f64 / SR).sin()) as f32).collect();
        Pcm { r: l.clone(), l }
    }

    #[test]
    fn loudness_of_a_1khz_tone() {
        // a 997 Hz sine at -20 dBFS peak in both channels reads about -20 LUFS (BS.1770)
        let p = sine(997.0, 5.0, 0.1);
        let l = Loud::new(&p).integrated();
        assert!((l - -20.0).abs() < 0.3, "{l}");
        let tp = true_peak(&p);
        assert!((tp - -20.0).abs() < 0.2, "{tp}");
    }

    #[test]
    fn a_quiet_mix_is_brought_to_the_cut_level() {
        // -40 dBFS: the check used to report the mix before the cut normalized it
        let mut p = sine(997.0, 5.0, 0.01);
        normalize(&mut p, -16.0, -1.5);
        let l = Loud::new(&p).integrated();
        assert!((l - -16.0).abs() < 0.3, "{l}");
        // a loud one is held under the ceiling
        let mut p = sine(997.0, 5.0, 0.9);
        normalize(&mut p, -8.0, -1.5);
        let peak = p.l.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak <= 10f32.powf(-1.5 / 20.0) + 1e-6, "{peak}");
        // silence stays silent
        let mut z = Pcm::silent(1000);
        normalize(&mut z, -16.0, -1.5);
        assert!(z.l.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn a_tone_sits_in_its_band() {
        let b = bands(&mono(&sine(2000.0, 2.0, 0.5)));
        let mid = b.iter().find(|x| x.0 == "mid").unwrap().1;
        assert!(mid > -0.5, "{b:?}");
    }

    /// Decaying noise bursts (50 ms) added at `at` seconds.
    fn add_clicks(p: &mut Pcm, at: &[f64]) {
        let mut seed = 7u32;
        for &t in at {
            let s = (t * SR) as usize;
            for k in 0..2400 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                let noise = ((seed >> 9) as f32 / (1 << 23) as f32) - 1.0;
                let v = noise * 0.8 * (-(k as f32) / 300.0).exp();
                p.l[s + k] += v;
                p.r[s + k] += v;
            }
        }
    }

    #[test]
    fn onsets_land_on_clicks() {
        let mut p = Pcm::silent((4.0 * SR) as usize);
        add_clicks(&mut p, &[0.5, 1.5, 2.5, 3.2]);
        let m = mono(&p);
        let e2: Vec<f64> = frame_rms(&m, (E2 * SR) as usize).iter().map(|r| 20.0 * (r + 1e-9).log10()).collect();
        let on = attack_times(&onset_times(&onset_envelope(&m)), &e2);
        for t in [0.5, 1.5, 2.5, 3.2] {
            assert!(on.iter().any(|o| (o - t).abs() < 0.03), "{t} not in {on:?}");
        }
    }

    #[test]
    fn silence_and_short_mixes_are_not_scored() {
        assert_eq!(check(&Pcm::silent((5.0 * SR) as usize), &[], 30.0)["verdict"], "n.a.");
        assert_eq!(check(&sine(440.0, 1.0, 0.5), &[], 30.0)["verdict"], "n.a.");
    }

    #[test]
    fn cuts_without_sound_are_named() {
        // a steady bed with hits between the cuts, none on them
        let mut p = sine(220.0, 12.0, 0.1);
        add_clicks(&mut p, &[1.5, 4.5, 7.5, 10.5]);
        let cuts: Vec<Cut> = [3.0, 6.0, 9.0]
            .iter()
            .enumerate()
            .map(|(i, t)| Cut { t: *t, from: format!("s{i}"), to: format!("s{}", i + 1), to_beat: None })
            .collect();
        let r = check(&p, &cuts, 30.0);
        let row = r["metrics"].as_array().unwrap().iter().find(|m| m["metric"] == "cuts_with_sound").unwrap();
        assert_eq!(row["value"], json!(0.0));
        assert_eq!(row["status"], "low");
        let a = r["actions"].as_array().unwrap().iter().find(|a| a["metric"] == "cuts_with_sound").unwrap();
        assert!(a["action"].as_str().unwrap().contains("s0 → s1"), "{a}");
        assert_eq!(a["at"], json!(3.0));
    }
}
