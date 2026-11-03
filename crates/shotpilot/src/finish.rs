//! Finishing: the design burned into the cut (letterbox, grade, script cards and captions,
//! title, end card, dips and flashes between shots).
//!
//! Like sound (D-19), this is part of the definition, not editing after the fact (D-25):
//! it is written in shots.yaml (`finish:`, `script:`, and each shot's `caption` /
//! `transition` / `exposure`) and applied by ffmpeg when the cut is assembled. None of it is
//! in any fingerprint, so changing a line of text never re-renders a shot; it only re-runs
//! the cut.
//!
//! No crossfades: an overlap would shorten the cut and pull the sound effects off their cuts.
//! Dips (through black) and flashes (from white) keep every shot's length.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::model::{fmt_g, num, str_of};
use crate::script::Line;

pub const TRANSITIONS: &[&str] = &["cut", "dip", "flash"];
/// Seconds a dip takes on each side of the cut.
pub const DIP: f64 = 0.25;

/// Colour grades applied to every shot at the cut (ffmpeg filters; `{b}` is the brightness,
/// which a shot's `exposure` shifts). `horror`: cooler, desaturated, contrast up, vignette,
/// film grain. `red`: the same with a red cast, for a jolt.
pub const GRADES: &[(&str, &str)] = &[
    ("horror", "eq=contrast=1.10:brightness={b}:saturation=0.80,colorbalance=bs=0.03:bm=0.015:rs=-0.015,vignette=angle=PI/4.5,noise=alls=4:allf=t"),
    ("red", "eq=contrast=1.15:brightness={b}:saturation=0.70,colorbalance=rs=0.25:rm=0.18:gs=-0.06:bs=-0.08,vignette=angle=PI/4,noise=alls=5:allf=t"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub text: String,
    /// Seconds from the start of its shot (title: of the title shot).
    pub at: f64,
    pub duration: f64,
}

#[derive(Debug, Clone)]
pub struct FinishCfg {
    /// Picture aspect inside black bars (2.39 = scope); None = full frame.
    pub letterbox: Option<f64>,
    pub font: Option<String>,
    /// `mincho` (serif, the default when the system has it) or `gothic`.
    pub typeface: String,
    /// A colour grade over every shot: `none` or a preset name (GRADES).
    pub grade: String,
    pub title: Option<Text>,
    pub end_card: Option<(Vec<String>, f64)>,
}

impl Default for FinishCfg {
    fn default() -> Self {
        FinishCfg { letterbox: None, font: None, typeface: "mincho".into(), grade: "none".into(), title: None, end_card: None }
    }
}

impl FinishCfg {
    pub fn is_empty(&self) -> bool {
        self.letterbox.is_none() && self.title.is_none() && self.end_card.is_none() && self.grade == "none"
    }
}

fn text_of(v: &Value, default_at: f64, default_dur: f64) -> Option<Text> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(Text { text: s.clone(), at: default_at, duration: default_dur }),
        Value::Object(m) => {
            let text = m.get("text").map(str_of).unwrap_or_default();
            (!text.trim().is_empty()).then(|| Text {
                text,
                at: m.get("at").and_then(num).unwrap_or(default_at),
                duration: m.get("duration").and_then(num).unwrap_or(default_dur),
            })
        }
        _ => None,
    }
}

/// `finish:` from shots.yaml; problems go to `errs`.
pub fn parse(v: Option<&Value>, errs: &mut Vec<String>) -> FinishCfg {
    let m = match v {
        None | Some(Value::Null) => return FinishCfg::default(),
        Some(Value::Object(m)) => m.clone(),
        Some(_) => {
            errs.push("finish: must be a mapping".into());
            return FinishCfg::default();
        }
    };
    let letterbox = match m.get("letterbox") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(false)) => None,
        Some(x) => match num(x) {
            Some(r) if (1.0..=3.0).contains(&r) => Some(r),
            _ => {
                errs.push("finish.letterbox: a picture aspect between 1.0 and 3.0 (e.g. 2.39), or null".into());
                None
            }
        },
    };
    let end_card = match m.get("end_card") {
        None | Some(Value::Null) => None,
        Some(Value::Object(e)) => {
            let lines: Vec<String> = match e.get("lines") {
                Some(Value::Array(a)) => a.iter().map(str_of).filter(|s| !s.trim().is_empty()).collect(),
                Some(Value::String(s)) => vec![s.clone()],
                _ => Vec::new(),
            };
            let d = e.get("duration").and_then(num).unwrap_or(3.0);
            if !(0.5..=10.0).contains(&d) {
                errs.push("finish.end_card.duration: 0.5..10 seconds".into());
            }
            (!lines.is_empty()).then_some((lines, d))
        }
        Some(_) => {
            errs.push("finish.end_card: {lines: [...], duration: seconds}".into());
            None
        }
    };
    let grade = m.get("grade").map(str_of).filter(|s| !s.is_empty()).unwrap_or_else(|| "none".into());
    if grade != "none" && !GRADES.iter().any(|(n, _)| *n == grade) {
        let names: Vec<&str> = GRADES.iter().map(|(n, _)| *n).collect();
        errs.push(format!("finish.grade: none or one of {} (got '{grade}')", names.join(", ")));
    }
    let typeface = m.get("typeface").map(str_of).filter(|s| !s.is_empty()).unwrap_or_else(|| "mincho".into());
    if !["mincho", "gothic"].contains(&typeface.as_str()) {
        errs.push(format!("finish.typeface: mincho or gothic (got '{typeface}')"));
    }
    FinishCfg {
        letterbox,
        font: m.get("font").map(str_of).filter(|s| !s.is_empty()),
        typeface,
        grade,
        title: m.get("title").and_then(|t| text_of(t, 0.5, 3.0)),
        end_card,
    }
}

/// A shot's caption (`caption: text` or `{text, at, duration}`).
pub fn caption(shot: &Value) -> Option<Text> {
    let d = shot.get("duration").and_then(num).unwrap_or(3.0);
    shot.get("caption").and_then(|c| text_of(c, 0.3, (d - 0.6).max(0.8)))
}

/// How a shot hands over to the next one.
pub fn transition(shot: &Value) -> String {
    shot.get("transition").map(str_of).filter(|t| !t.is_empty()).unwrap_or_else(|| "cut".into())
}

/// A shot's brightness shift at the cut (`exposure`, -0.5..0.5).
pub fn exposure(shot: &Value) -> f64 {
    shot.get("exposure").and_then(num).unwrap_or(0.0)
}

/// Per-shot checks (called from model validation).
pub fn validate_shot(at: &str, shot: &Value, errs: &mut Vec<String>) {
    let d = shot.get("duration").and_then(num).unwrap_or(0.0);
    if let Some(c) = caption(shot) {
        if c.at < 0.0 || c.at >= d.max(0.0) {
            errs.push(format!("{at}.caption.at: {}s is outside the shot (0..{}s)", fmt_g(c.at), fmt_g(d)));
        }
    }
    if let Some(e) = shot.get("exposure").filter(|v| !v.is_null()) {
        if !num(e).is_some_and(|e| (-0.5..=0.5).contains(&e)) {
            errs.push(format!("{at}.exposure: a brightness shift between -0.5 and 0.5"));
        }
    }
    if let Some(t) = shot.get("transition").filter(|v| !v.is_null()) {
        let t = str_of(t);
        if !TRANSITIONS.contains(&t.as_str()) {
            errs.push(format!("{at}.transition: one of {} (got '{t}')", TRANSITIONS.join(", ")));
        }
    }
}

/// Fonts for the burned-in text: a Japanese face (mincho or gothic) and a sans for the
/// quiet English lines.
#[derive(Debug, Clone)]
pub struct Fonts {
    pub main: PathBuf,
    pub sans: PathBuf,
}

fn first(cands: &[PathBuf]) -> Option<PathBuf> {
    cands.iter().find(|p| p.is_file()).cloned()
}

/// The fonts to use: `finish.font` for the main face, else a system font of the typeface.
pub fn fonts(cfg: &FinishCfg) -> Option<Fonts> {
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/Windows"));
    let f = windir.join("Fonts");
    let gothic = [
        f.join("YuGothM.ttc"),
        f.join("YuGothB.ttc"),
        f.join("BIZ-UDGothicR.ttc"),
        f.join("meiryo.ttc"),
        f.join("NotoSansJP-VF.ttf"),
        PathBuf::from("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
    ];
    let mincho = [
        f.join("yumin.ttf"),
        f.join("BIZ-UDMinchoM.ttc"),
        f.join("msmincho.ttc"),
        PathBuf::from("/System/Library/Fonts/ヒラギノ明朝 ProN.ttc"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc"),
    ];
    let sans = first(&gothic)?;
    let main = match &cfg.font {
        Some(p) => {
            let p = PathBuf::from(p);
            if !p.is_file() {
                return None;
            }
            p
        }
        None if cfg.typeface == "gothic" => sans.clone(),
        None => first(&mincho).unwrap_or_else(|| sans.clone()),
    };
    Some(Fonts { main, sans })
}

/// The Japanese face only (for callers that need one font).
pub fn font(cfg: &FinishCfg) -> Option<PathBuf> {
    fonts(cfg).map(|f| f.main)
}

/// A path inside an ffmpeg filter option: forward slashes, `:` and `'` escaped.
pub fn filter_path(p: &Path) -> String {
    p.display().to_string().replace('\\', "/").replace(':', "\\:").replace('\'', "\\'")
}

/// One shot in the cut, as finishing sees it.
#[derive(Debug, Clone)]
pub struct ShotPiece {
    /// Where the shot starts inside the plain concatenation of shots (seconds).
    pub concat_start: f64,
    pub duration: f64,
    pub beat: Option<String>,
    /// The per-shot caption (`caption:` on the shot).
    pub caption: Option<Text>,
    pub transition: String,
    pub exposure: f64,
    /// Script lines shown over this shot (caption, count).
    pub overlays: Vec<Line>,
}

/// A piece of the cut: a shot, or a script slot (a card on black) and its length.
#[derive(Debug, Clone)]
pub enum CutPiece {
    Shot(ShotPiece),
    Slot(Line, f64),
}

impl CutPiece {
    pub fn duration(&self) -> f64 {
        match self {
            CutPiece::Shot(s) => s.duration,
            CutPiece::Slot(_, d) => *d,
        }
    }
}

/// What finishing will burn in, for the cut's spec and the app.
pub fn describe(cfg: &FinishCfg, pieces: &[CutPiece]) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("letterbox".into(), serde_json::json!(cfg.letterbox));
    m.insert("grade".into(), serde_json::json!(cfg.grade));
    m.insert("slots".into(), serde_json::json!(pieces.iter().filter(|p| matches!(p, CutPiece::Slot(..))).count()));
    let overlays: usize = pieces
        .iter()
        .map(|p| match p {
            CutPiece::Shot(s) => s.overlays.len() + s.caption.is_some() as usize,
            _ => 0,
        })
        .sum();
    m.insert("overlays".into(), serde_json::json!(overlays));
    m
}

/// Whether there is anything to burn in.
pub fn needed(cfg: &FinishCfg, pieces: &[CutPiece]) -> bool {
    !cfg.is_empty()
        || pieces.iter().any(|p| match p {
            CutPiece::Slot(..) => true,
            CutPiece::Shot(s) => s.caption.is_some() || s.transition != "cut" || !s.overlays.is_empty() || s.exposure != 0.0,
        })
}

/// Colours of the text: warm off-white ink, dim grey, soft grey for sub-lines.
const INK: &str = "0xE0DBCF";
const DIM: &str = "0x80877F";
const SUB: &str = "0xB3B8B0";

/// Opacity that rises over `fin` from `a` and falls over `fout` until `b` (local time `t`).
fn fade(a: f64, b: f64, fin: f64, fout: f64) -> String {
    let half = ((b - a) / 2.0).max(0.01);
    let (fin, fout) = (fin.clamp(0.01, half), fout.clamp(0.01, half));
    format!(
        "if(lt(t,{a:.3}),0,if(lt(t,{:.3}),(t-{a:.3})/{fin:.3},if(lt(t,{:.3}),1,if(lt(t,{b:.3}),({b:.3}-t)/{fout:.3},0))))",
        a + fin,
        b - fout
    )
}

/// The title's opacity: a short irregular flicker in, a steady hold, then a fade.
fn flicker(d: f64) -> String {
    let keys = [(0.0, 0.0), (0.10, 0.85), (0.16, 0.10), (0.24, 1.0), (0.30, 0.35), (0.40, 1.0), ((d - 0.6).max(0.5), 1.0), (d, 0.0)];
    let mut e = "0".to_string();
    for w in keys.windows(2).rev() {
        let ((t0, v0), (t1, v1)) = (w[0], w[1]);
        let seg = format!("{v0:.3}+({v1:.3}-{v0:.3})*(t-{t0:.3})/{:.3}", (t1 - t0).max(0.001));
        e = format!("if(lt(t,{t1:.3}),{seg},{e})");
    }
    e
}

/// Writes drawtext filters; text goes through files so nothing in it needs escaping.
struct Ink<'a> {
    fonts: &'a Fonts,
    dir: &'a Path,
    n: usize,
    /// Scale from the 1080-line layout.
    s: f64,
    w: i64,
    /// The picture inside the letterbox bars (top edge, height), for text over shots.
    pic_top: f64,
    pic_h: f64,
}

impl Ink<'_> {
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, body: &str, sans: bool, size: f64, color: &str, x: &str, y: &str, alpha: &str, border: f64)
        -> crate::Result<String> {
        self.n += 1;
        let p = self.dir.join(format!("text{}.txt", self.n));
        std::fs::write(&p, body)?;
        let font = if sans { &self.fonts.sans } else { &self.fonts.main };
        let b = if border > 0.0 { format!(":borderw={:.0}:bordercolor=black@0.55", border * self.s) } else { String::new() };
        Ok(format!(
            "drawtext=fontfile='{}':textfile='{}':fontsize={:.0}:fontcolor={color}{b}:x={x}:y={y}:alpha='{alpha}'",
            filter_path(font),
            filter_path(&p),
            size * self.s
        ))
    }

    fn cy(&self, y: f64) -> String {
        format!("{:.0}-text_h/2", y * self.s)
    }

    /// A 1080-layout height over a shot, kept inside the letterbox picture.
    fn pic_y(&self, y: f64) -> f64 {
        self.pic_top + y / 1080.0 * self.pic_h
    }
}

const CX: &str = "(w-text_w)/2";

/// Filters that draw a script line over its shot, local time.
fn overlay(ink: &mut Ink, l: &Line, shot_dur: f64) -> crate::Result<Vec<String>> {
    let s = ink.s;
    let (a, b) = (l.at.max(0.0), (l.at + l.dur).min(shot_dur));
    if b <= a {
        return Ok(Vec::new());
    }
    let band = |ink: &Ink, y: f64, hh: f64| {
        let hh = hh * ink.pic_h / 1080.0;
        format!("drawbox=x=0:y={:.0}:w=iw:h={:.0}:color=black@0.42:t=fill:enable='between(t,{a:.3},{b:.3})'",
                ink.pic_y(y) - hh / 2.0, hh)
    };
    let py = |ink: &Ink, y: f64| format!("{:.0}-text_h/2", ink.pic_y(y));
    let mut out = Vec::new();
    if l.kind == "count" {
        // like a note under a photograph: lower left
        out.push(band(ink, 950.0, 190.0));
        let y = py(ink, 950.0);
        out.push(ink.text(&l.ja, false, 46.0, INK, &format!("{:.0}", 150.0 * s), &y, &fade(a, b, 0.18, 0.25), 3.0)?);
    } else {
        out.push(band(ink, 925.0, 230.0));
        let y = py(ink, 900.0);
        out.push(ink.text(&l.ja, false, 54.0, INK, CX, &y, &fade(a, b, 0.35, 0.4), 3.0)?);
        if !l.en.trim().is_empty() {
            let y = py(ink, 965.0);
            out.push(ink.text(&l.en, true, 22.0, SUB, CX, &y, &fade(a + 0.15, b, 0.35, 0.4), 2.0)?);
        }
    }
    Ok(out)
}

/// The filter chain of a slot (a card on black), from a black colour source.
fn slot(ink: &mut Ink, l: &Line, d: f64, w: i64, h: i64, fps: i64) -> crate::Result<String> {
    let s = ink.s;
    let mut f = vec![format!("color=c=black:s={w}x{h}:r={fps}:d={d:.3}")];
    match l.kind.as_str() {
        "black" => {}
        "rule" => {
            if !l.head.is_empty() {
                let y = ink.cy(455.0);
                f.push(ink.text(&l.head, true, 26.0, DIM, CX, &y, &fade(0.1, d - 0.05, 0.3, 0.35), 0.0)?);
            }
            let y = ink.cy(545.0);
            f.push(ink.text(&l.ja, false, 62.0, INK, CX, &y, &fade(0.2, d - 0.05, 0.3, 0.35), 0.0)?);
        }
        "title" => {
            let y = ink.cy(530.0);
            f.push(ink.text(&l.ja, false, 150.0, INK, CX, &y, &flicker(d), 0.0)?);
            if !l.en.is_empty() {
                let y = ink.cy(640.0);
                f.push(ink.text(&l.en, true, 30.0, DIM, CX, &y, &fade(0.6, d, 0.6, 0.6), 0.0)?);
            }
        }
        "message" => {
            // like the house chat: room name, a dark bubble, the sender, the message
            if !l.head.is_empty() {
                let y = ink.cy(410.0);
                f.push(ink.text(&l.head, true, 22.0, DIM, CX, &y, &fade(0.1, d, 0.25, 0.5), 0.0)?);
            }
            f.push(format!(
                "drawbox=x={:.0}:y={:.0}:w={:.0}:h={:.0}:color=0x292B2E:t=fill:enable='gte(t,0.25)'",
                ink.w as f64 / 2.0 - 210.0 * s,
                (548.0 - 46.0) * s,
                420.0 * s,
                92.0 * s
            ));
            if !l.from.is_empty() {
                let (x, y) = (format!("{:.0}", ink.w as f64 / 2.0 - 190.0 * s), ink.cy(478.0));
                f.push(ink.text(&l.from, true, 24.0, DIM, &x, &y, &fade(0.25, d, 0.25, 0.5), 0.0)?);
            }
            let y = ink.cy(562.0);
            f.push(ink.text(&l.ja, true, 44.0, INK, CX, &y, &fade(0.45, d, 0.3, 0.5), 0.0)?);
        }
        _ => {
            // card: a centred line on black, a quiet English line beneath
            let y = ink.cy(520.0);
            f.push(ink.text(&l.ja, false, 64.0, INK, CX, &y, &fade(0.15, d - 0.05, 0.45, 0.5), 0.0)?);
            if !l.en.is_empty() {
                let y = ink.cy(600.0);
                f.push(ink.text(&l.en, true, 22.0, DIM, CX, &y, &fade(0.35, d - 0.05, 0.45, 0.5), 0.0)?);
            }
        }
    }
    f.push("format=yuv420p,setsar=1".into());
    Ok(f.join(","))
}

fn plain_line(kind: &str, ja: String, dur: f64, at: f64) -> Line {
    Line { id: String::new(), kind: kind.into(), ja, en: String::new(), head: String::new(), from: String::new(), dur,
           after: None, on: None, at }
}

/// The ffmpeg filter graph for the finishing pass. Input 0 is the plain concatenation of the
/// shots; slots are made from black inside the graph. Without fonts, text is left out.
/// Returns (graph ending in [v], seconds).
pub fn filter_graph(cfg: &FinishCfg, pieces: &[CutPiece], w: i64, h: i64, fps: i64, fonts: Option<&Fonts>, text_dir: &Path)
    -> crate::Result<(String, f64)> {
    let fallback = Fonts { main: PathBuf::new(), sans: PathBuf::new() };
    let bar = cfg.letterbox.map(|r| ((h as f64 - w as f64 / r) / 2.0).max(0.0).round() as i64).unwrap_or(0);
    let mut ink = Ink { fonts: fonts.unwrap_or(&fallback), dir: text_dir, n: 0, s: h as f64 / 1080.0, w,
                        pic_top: bar as f64, pic_h: (h - 2 * bar) as f64 };
    let has_text = fonts.is_some();
    let n_shots = pieces.iter().filter(|p| matches!(p, CutPiece::Shot(_))).count();
    let mut graph: Vec<String> = Vec::new();
    if n_shots > 0 {
        graph.push(format!("[0:v]split={n_shots}{}", (0..n_shots).map(|i| format!("[s{i}]")).collect::<String>()));
    }
    let grade = GRADES.iter().find(|(n, _)| *n == cfg.grade).map(|(_, g)| *g);
    let title_host = pieces
        .iter()
        .rposition(|p| matches!(p, CutPiece::Shot(x) if x.beat.as_deref() == Some("title")))
        .or_else(|| pieces.iter().rposition(|p| matches!(p, CutPiece::Shot(_))));
    let mut labels = Vec::new();
    let mut k = 0;
    for (i, p) in pieces.iter().enumerate() {
        match p {
            CutPiece::Shot(x) => {
                let mut f = vec![
                    format!("trim=start={:.4}:end={:.4}", x.concat_start, x.concat_start + x.duration),
                    "setpts=PTS-STARTPTS".to_string(),
                ];
                match grade {
                    Some(g) => f.push(g.replace("{b}", &format!("{:.3}", -0.015 + x.exposure))),
                    None if x.exposure != 0.0 => f.push(format!("eq=brightness={:.3}", x.exposure)),
                    None => {}
                }
                // the hand-over from the shot before
                if let Some(CutPiece::Shot(prev)) = i.checked_sub(1).map(|j| &pieces[j]) {
                    match prev.transition.as_str() {
                        "dip" => f.push(format!("fade=t=in:st=0:d={:.3}:color=black", DIP.min(x.duration / 2.0))),
                        "flash" => f.push("fade=t=in:st=0:d=0.12:color=white".into()),
                        _ => {}
                    }
                }
                if x.transition == "dip" && matches!(pieces.get(i + 1), Some(CutPiece::Shot(_))) {
                    let d = DIP.min(x.duration / 2.0);
                    f.push(format!("fade=t=out:st={:.3}:d={d:.3}:color=black", x.duration - d));
                }
                if has_text {
                    for l in &x.overlays {
                        f.extend(overlay(&mut ink, l, x.duration)?);
                    }
                    if let Some(c) = &x.caption {
                        f.extend(overlay(&mut ink, &plain_line("caption", c.text.clone(), c.duration, c.at), x.duration)?);
                    }
                    if Some(i) == title_host {
                        if let Some(t) = &cfg.title {
                            let (a, b) = (t.at.min(x.duration - 0.5).max(0.0), (t.at + t.duration).min(x.duration));
                            let y = ink.cy(500.0);
                            f.push(ink.text(&t.text, false, 120.0, INK, CX, &y, &fade(a, b, 0.4, 0.5), 1.0)?);
                        }
                    }
                }
                f.push("format=yuv420p,setsar=1".into());
                graph.push(format!("[s{k}]{}[p{i}]", f.join(",")));
                k += 1;
            }
            CutPiece::Slot(l, d) => {
                let chain = if has_text {
                    slot(&mut ink, l, *d, w, h, fps)?
                } else {
                    format!("color=c=black:s={w}x{h}:r={fps}:d={d:.3},format=yuv420p,setsar=1")
                };
                graph.push(format!("{chain}[p{i}]"));
            }
        }
        labels.push(format!("[p{i}]"));
    }
    let mut total: f64 = pieces.iter().map(CutPiece::duration).sum();
    if let Some((lines, d)) = cfg.end_card.as_ref().filter(|_| has_text) {
        graph.push(format!("{}[pend]", slot(&mut ink, &plain_line("card", lines.join("\n"), *d, 0.0), *d, w, h, fps)?));
        labels.push("[pend]".into());
        total += d;
    }
    let bars = if bar > 0 {
        format!(",drawbox=x=0:y=0:w=iw:h={bar}:color=black:t=fill,drawbox=x=0:y=ih-{bar}:w=iw:h={bar}:color=black:t=fill")
    } else {
        String::new()
    };
    graph.push(format!("{}concat=n={}:v=1:a=0{bars},format=yuv420p[v]", labels.join(""), labels.len()));
    Ok((graph.join(";"), total))
}
