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
//! Dips (through black), flashes and whiteouts (from white) and glitches keep every shot's
//! length. How the text moves and how long a dip takes come from `finish.motion` (motion.rs).

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::model::{fmt_g, num, str_of};
use crate::script::Line;

pub const TRANSITIONS: &[&str] = &["cut", "dip", "flash", "whiteout", "glitch"];
/// Seconds a dip takes on each side of the cut (the base profile's).
pub const DIP: f64 = 0.25;

/// Colour grades applied to every shot at the cut (ffmpeg filters; `{b}` is the brightness,
/// which a shot's `exposure` shifts). `horror`: cooler, desaturated, contrast up, vignette,
/// film grain. `red`: the same with a red cast, for a jolt. `warm`: brighter colour and a
/// sunny cast (cozy). `vivid`: contrast and saturation up, a little sharper (action).
/// `cool`: a blue cast, slightly desaturated (sci-fi, mystery, night).
pub const GRADES: &[(&str, &str)] = &[
    ("horror", "eq=contrast=1.10:brightness={b}:saturation=0.80,colorbalance=bs=0.03:bm=0.015:rs=-0.015,vignette=angle=PI/4.5,noise=alls=4:allf=t"),
    ("red", "eq=contrast=1.15:brightness={b}:saturation=0.70,colorbalance=rs=0.25:rm=0.18:gs=-0.06:bs=-0.08,vignette=angle=PI/4,noise=alls=5:allf=t"),
    ("warm", "eq=contrast=1.04:brightness={b}:saturation=1.12,colorbalance=rs=0.05:rm=0.04:bs=-0.04:bm=-0.02"),
    ("vivid", "eq=contrast=1.12:brightness={b}:saturation=1.25,unsharp=5:5:0.4"),
    ("cool", "eq=contrast=1.06:brightness={b}:saturation=0.92,colorbalance=bs=0.06:bm=0.03:rs=-0.03"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub text: String,
    /// Seconds from the start of its shot (title: of the title shot).
    pub at: f64,
    pub duration: f64,
}

/// The faces a look sets (`finish.fonts`, D-52): `display` for the title, cards and rules,
/// `body` for captions, counts, messages and the end card, `latin` for the English lines,
/// and `title` (D-59) for the title alone when it should not be the display face (a worn or
/// broken face; the display face stands in where it has no glyph). Each is a family of
/// `references/fonts/catalog.json` at one weight; unset roles use the system mincho / gothic.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FaceCfg {
    pub display: Option<crate::fonts::Spec>,
    pub body: Option<crate::fonts::Spec>,
    pub latin: Option<crate::fonts::Spec>,
    pub title: Option<crate::fonts::Spec>,
}

pub const FACE_ROLES: &[&str] = &["display", "body", "latin", "title"];

impl FaceCfg {
    /// The faces that are set, in role order.
    pub fn specs(&self) -> Vec<crate::fonts::Spec> {
        [&self.display, &self.body, &self.latin, &self.title].into_iter().flatten().cloned().collect()
    }

    /// As `finish.fonts` is written: the roles that are set.
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        for (role, spec) in FACE_ROLES.iter().zip([&self.display, &self.body, &self.latin, &self.title]) {
            if let Some(s) = spec {
                m.insert(role.to_string(), Value::String(s.label()));
            }
        }
        Value::Object(m)
    }
}

/// The title's glyph height (kanji, or capitals for a Latin title) as a share of the frame
/// height, when `finish.title_size` is not set. Measured teasers range 0.07-0.35 by genre
/// (references/craft/TEASER-TYPE.md); 0.122 is the size the title had before it was settable.
pub const TITLE_SIZE: f64 = 0.122;

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
    /// Faces from the font catalogue (fetched once, then cached).
    pub fonts: FaceCfg,
    /// The title's glyph height as a share of the frame height (None: TITLE_SIZE).
    pub title_size: Option<f64>,
    /// The game's logo (an image next to shots.yaml), drawn instead of the title's text.
    pub logo: Option<String>,
    /// How the text moves and the shots hand over (D-53).
    pub motion: crate::motion::MotionCfg,
}

impl Default for FinishCfg {
    fn default() -> Self {
        FinishCfg { letterbox: None, font: None, typeface: "mincho".into(), grade: "none".into(), title: None, end_card: None,
                    fonts: FaceCfg::default(), title_size: None, logo: None, motion: Default::default() }
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
    let mut fonts = FaceCfg::default();
    match m.get("fonts") {
        None | Some(Value::Null) => {}
        Some(Value::Object(f)) => {
            for (role, v) in f {
                let slot = match role.as_str() {
                    "display" => &mut fonts.display,
                    "body" => &mut fonts.body,
                    "latin" => &mut fonts.latin,
                    "title" => &mut fonts.title,
                    _ => {
                        errs.push(format!("finish.fonts.{role}: one of {}", FACE_ROLES.join(", ")));
                        continue;
                    }
                };
                match v {
                    Value::Null => {}
                    Value::String(s) if s.trim().is_empty() => {}
                    Value::String(s) => match crate::fonts::Spec::parse(s) {
                        Ok(spec) => *slot = Some(spec),
                        Err(e) => errs.push(format!("finish.fonts.{role}: {e}")),
                    },
                    _ => errs.push(format!("finish.fonts.{role}: a family, optionally with a weight (\"Shippori Mincho:800\")")),
                }
            }
        }
        Some(_) => errs.push("finish.fonts: {display, body, latin, title} (\"Family:weight\" each)".into()),
    }
    let title_size = match m.get("title_size") {
        None | Some(Value::Null) => None,
        Some(x) => match num(x) {
            Some(r) if (0.04..=0.35).contains(&r) => Some(r),
            _ => {
                errs.push("finish.title_size: the title's glyph height as a share of the frame height, 0.04..0.35".into());
                None
            }
        },
    };
    let logo = m.get("logo").map(str_of).filter(|s| !s.trim().is_empty());
    if let Some(l) = &logo {
        let p = Path::new(l);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if p.is_absolute() || l.contains("..") || !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
            errs.push(format!("finish.logo: an image (png, jpg, webp) inside the project, as a relative path (got '{l}')"));
        }
    }
    FinishCfg {
        letterbox,
        font: m.get("font").map(str_of).filter(|s| !s.is_empty()),
        typeface,
        grade,
        title: m.get("title").and_then(|t| text_of(t, 0.5, 3.0)),
        end_card,
        fonts,
        title_size,
        logo,
        motion: crate::motion::parse(m.get("motion"), errs),
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

/// A face from the catalogue, resolved to its cached file.
#[derive(Debug, Clone, Default)]
pub struct Face {
    /// `Family:weight`, for notes.
    pub label: String,
    pub path: PathBuf,
}

/// Fonts for the burned-in text: the system Japanese face (mincho or gothic) and a gothic
/// for the quiet lines, plus the look's faces where they could be had. A line falls back to
/// the system face when its catalogue face has no glyph for one of its characters.
#[derive(Debug, Clone, Default)]
pub struct Fonts {
    pub main: PathBuf,
    pub sans: PathBuf,
    pub display: Option<Face>,
    pub body: Option<Face>,
    pub latin: Option<Face>,
    pub title: Option<Face>,
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
    Some(Fonts { main, sans, ..Default::default() })
}

/// The system fonts plus the look's faces (`finish.fonts`), fetched into the cache when the
/// binary allows it. A face that cannot be had is reported in `notes` and left to the system
/// font; with no system Japanese font, the look's faces stand in for it.
pub fn faces(cfg: &FinishCfg, notes: &mut Vec<String>) -> Option<Fonts> {
    let mut get = |spec: &Option<crate::fonts::Spec>| -> Option<Face> {
        let spec = spec.as_ref()?;
        match crate::fonts::resolve(spec) {
            Ok(path) => Some(Face { label: spec.label(), path }),
            Err(e) => {
                notes.push(format!("font {}: {}; the system font was used", spec.label(), e.msg));
                None
            }
        }
    };
    let (display, body, latin, title) =
        (get(&cfg.fonts.display), get(&cfg.fonts.body), get(&cfg.fonts.latin), get(&cfg.fonts.title));
    let mut f = match fonts(cfg) {
        Some(f) => f,
        None => {
            let any = body.as_ref().or(display.as_ref())?;
            Fonts { main: any.path.clone(), sans: any.path.clone(), ..Default::default() }
        }
    };
    f.display = display;
    f.body = body;
    f.latin = latin;
    f.title = title;
    Some(f)
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
    /// Camera effects on the picture (fx.rs, D-57).
    pub fx: Vec<crate::fx::Fx>,
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
    let fx: usize = pieces.iter().map(|p| if let CutPiece::Shot(s) = p { s.fx.len() } else { 0 }).sum();
    m.insert("fx".into(), serde_json::json!(fx));
    let label = |s: &Option<crate::fonts::Spec>| s.as_ref().map(crate::fonts::Spec::label);
    m.insert("fonts".into(), serde_json::json!({
        "display": label(&cfg.fonts.display), "body": label(&cfg.fonts.body), "latin": label(&cfg.fonts.latin),
        "title": label(&cfg.fonts.title),
    }));
    m.insert("logo".into(), serde_json::json!(cfg.logo));
    m
}

/// Whether there is anything to burn in.
pub fn needed(cfg: &FinishCfg, pieces: &[CutPiece]) -> bool {
    !cfg.is_empty()
        || pieces.iter().any(|p| match p {
            CutPiece::Slot(..) => true,
            CutPiece::Shot(s) => {
                s.caption.is_some() || s.transition != "cut" || !s.overlays.is_empty() || s.exposure != 0.0 || !s.fx.is_empty()
            }
        })
}

/// Colours of the text: warm off-white ink, dim grey, soft grey for sub-lines.
const INK: &str = "0xE0DBCF";
const DIM: &str = "0x80877F";
const SUB: &str = "0xB3B8B0";

use crate::motion::{fade, Motion, RoleMotion};

/// Which face a line is set in (D-52): the look's display face for the title, cards and
/// rules; its body face for captions and counts (system fallback: the main face) and for
/// the quiet lines of a message (fallback: the gothic); its Latin face for English lines.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Role {
    /// The title: its own face when set, else the display face.
    Title,
    Display,
    Body,
    Quiet,
    Latin,
}

/// Sizes on the 1080-line layout, from references/craft/TEASER-TYPE.md: captions at about 5%
/// of the frame height, English sub-lines at 2.5% or more (smaller reads as legal text).
const CARD: f64 = 64.0;
const RULE: f64 = 62.0;
const CAPTION: f64 = 62.0;
const COUNT: f64 = 46.0;
const SUB_LINE: f64 = 38.0;
const TITLE_SUB: f64 = 40.0;
const HEAD: f64 = 32.0;
const MESSAGE: f64 = 44.0;
const END_CARD: f64 = 58.0;
/// A glyph's height per em: kanji fill most of the em, Latin capitals about 70% of it.
const CJK_FACE: f64 = 0.88;
const CAP_FACE: f64 = 0.70;

fn is_wide(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x20000..=0x3FFFD)
}

/// A line's width in ems, roughly: full-width characters 1, Latin 0.6, spaces 0.3.
fn ems(line: &str) -> f64 {
    line.chars().map(|c| if is_wide(c) { 1.0 } else if c == ' ' { 0.3 } else { 0.6 }).sum()
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
    /// The title's glyph height as a share of the frame (None: the fixed 150 px).
    title_size: Option<f64>,
    /// The game's logo, drawn instead of the title's text.
    logo: Option<PathBuf>,
    /// Lines that left their face for the system font (a missing glyph), for the cut's notes.
    notes: Vec<String>,
    /// How the text moves and the shots hand over (D-53).
    motion: Motion,
}

/// Where a moving line sits across: centred, or from a left edge (pixels).
#[derive(Debug, Clone, Copy)]
enum Across {
    Centre,
    Left(f64),
}

/// A wipe's black mask on a card: it slides off the line from left to right.
#[derive(Debug, Clone, Copy)]
struct Wipe {
    x: f64,
    w: f64,
    y: f64,
    h: f64,
    a: f64,
    fin: f64,
}

impl Ink<'_> {
    /// The font file for a line: the look's face for its role, unless that face has no glyph
    /// for one of the line's characters (then the system face, and a note).
    fn font_for(&mut self, role: Role, body: &str) -> PathBuf {
        let fonts = self.fonts;
        if let (Role::Title, Some(t)) = (role, &fonts.title) {
            if crate::fonts::covers(&t.path, body) != Some(false) {
                return t.path.clone();
            }
            let note = format!("{} has no glyph for 「{}」; the title used the display face",
                               t.label, crate::fonts::missing(&t.path, body));
            if !self.notes.contains(&note) {
                self.notes.push(note);
            }
        }
        let (face, system) = match role {
            Role::Title | Role::Display => (&fonts.display, &fonts.main),
            Role::Body => (&fonts.body, &fonts.main),
            Role::Quiet => (&fonts.body, &fonts.sans),
            Role::Latin => (&fonts.latin, &fonts.sans),
        };
        let Some(face) = face else { return system.clone() };
        if crate::fonts::covers(&face.path, body) == Some(false) {
            let note = format!("{} has no glyph for 「{}」; that line used the system font",
                               face.label, crate::fonts::missing(&face.path, body));
            if !self.notes.contains(&note) {
                self.notes.push(note);
            }
            return system.clone();
        }
        face.path.clone()
    }

    /// A still line (sub-lines, headings): fixed size, faded by `alpha`.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, body: &str, role: Role, size: f64, color: &str, x: &str, y: &str, alpha: &str, border: f64)
        -> crate::Result<String> {
        let font = self.font_for(role, body);
        self.draw(body, &font, &format!("{:.0}", size * self.s), color, x, y, alpha, border)
    }

    /// One drawtext; the size may be an expression of `t`.
    #[allow(clippy::too_many_arguments)]
    fn draw(&mut self, body: &str, font: &Path, size: &str, color: &str, x: &str, y: &str, alpha: &str, border: f64)
        -> crate::Result<String> {
        self.n += 1;
        let p = self.dir.join(format!("text{}.txt", self.n));
        std::fs::write(&p, body)?;
        let b = if border > 0.0 { format!(":borderw={:.0}:bordercolor=black@0.55", border * self.s) } else { String::new() };
        // an expression with commas has to be quoted inside the filter graph
        let q = |e: &str| if e.contains(',') { format!("'{e}'") } else { e.to_string() };
        let (size, x, y) = (q(size), q(x), q(y));
        Ok(format!(
            "drawtext=fontfile='{}':textfile='{}':fontsize={size}:fontcolor={color}{b}:x={x}:y={y}:alpha='{alpha}'",
            filter_path(font),
            filter_path(&p),
        ))
    }

    /// A line that moves by `rm` while it shows in [a, b] (local time), centred on `yc`
    /// (pixels). Entrances that need a black card behind them (a wipe) fall back to a fade
    /// over a picture. Returns its filters and, for a wipe, the mask the card slides off it.
    #[allow(clippy::too_many_arguments)]
    fn line(&mut self, body: &str, role: Role, rm: &RoleMotion, a: f64, b: f64, px: f64, color: &str, across: Across,
            yc: f64, border: f64, on_black: bool) -> crate::Result<(Vec<String>, Option<Wipe>)> {
        let mut rm = rm.clone();
        if rm.enter == "wipe" && !on_black {
            rm.enter = "fade".into();
        }
        let tm = crate::motion::timing(&rm, a, b);
        let size_px = px * self.s;
        let font = self.font_for(role, body);
        let x = match across {
            Across::Centre => "(w-text_w)/2".to_string(),
            Across::Left(v) => format!("{v:.0}"),
        };
        let mut out = Vec::new();
        for (colour, size, alpha, dx) in crate::motion::layers(&rm, &tm, size_px, self.s) {
            let c = if colour.is_empty() { color.to_string() } else { colour };
            out.push(self.draw(body, &font, &size, &c, &format!("{x}+{dx}"), &format!("{yc:.0}-text_h/2"), &alpha, 0.0)?);
        }
        if rm.enter == "type" {
            // the first k characters at a time, from a fixed left edge and a fixed baseline
            let chars: Vec<char> = body.chars().collect();
            let steps = crate::motion::type_steps(chars.len(), &tm);
            let left = match across {
                Across::Centre => {
                    let widest = body.lines().map(ems).fold(0.0, f64::max);
                    format!("{:.0}", ((self.w as f64 - widest * size_px) / 2.0).max(0.0))
                }
                Across::Left(v) => format!("{v:.0}"),
            };
            let exit = crate::motion::exit_alpha(&rm, &tm);
            let y = format!("{yc:.0}-max_glyph_h/2");
            for (i, (k, t0)) in steps.iter().enumerate() {
                let prefix: String = chars[..*k].iter().collect();
                let alpha = match steps.get(i + 1) {
                    Some((_, t1)) => format!("gte(t,{t0:.3})*lt(t,{t1:.3})"),
                    None => format!("gte(t,{t0:.3})*{exit}"),
                };
                out.push(self.draw(&prefix, &font, &format!("{size_px:.0}"), color, &left, &y, &alpha, border)?);
            }
            return Ok((out, None));
        }
        let alpha = crate::motion::alpha(&rm, &tm);
        let size = crate::motion::size(&rm, &tm, size_px);
        out.push(self.draw(body, &font, &size, color, &x, &format!("{yc:.0}-text_h/2"), &alpha, border)?);
        let wipe = (rm.enter == "wipe").then(|| {
            let widest = body.lines().map(ems).fold(0.0, f64::max) * size_px;
            let pad = 0.02 * self.w as f64;
            let rows = body.lines().count().max(1) as f64;
            Wipe { x: (self.w as f64 - widest) / 2.0 - pad, w: widest + 2.0 * pad, y: yc - size_px * 0.8 * rows,
                   h: size_px * 1.6 * rows, a: tm.a, fin: tm.fin }
        });
        Ok((out, wipe))
    }

    fn cy(&self, y: f64) -> String {
        format!("{:.0}-text_h/2", y * self.s)
    }

    /// A 1080-layout height over a shot, kept inside the letterbox picture.
    fn pic_y(&self, y: f64) -> f64 {
        self.pic_top + y / 1080.0 * self.pic_h
    }

    /// `size` (1080 layout), made smaller when the widest line would not fit in `share` of
    /// the frame width.
    fn fit(&self, text: &str, size: f64, share: f64) -> f64 {
        let widest = text.lines().map(ems).fold(0.0, f64::max);
        if widest <= 0.0 {
            return size;
        }
        size.min(self.w as f64 / self.s * share / widest)
    }

    /// The title's font size (1080 layout): `title_size` of the frame height for the glyphs,
    /// kept inside 86% of the width.
    fn title_px(&self, text: &str) -> f64 {
        let px = match self.title_size {
            Some(frac) => frac * 1080.0 / if text.chars().any(is_wide) { CJK_FACE } else { CAP_FACE },
            None => 150.0,
        };
        self.fit(text, px, 0.86)
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
    let rm = ink.motion.for_line(&l.kind, l.motion.as_deref());
    let mut out = Vec::new();
    if l.kind == "count" {
        // like a note under a photograph: lower left
        out.push(band(ink, 950.0, 190.0));
        let yc = ink.pic_y(950.0);
        out.extend(ink.line(&l.ja, Role::Body, &rm, a, b, COUNT, INK, Across::Left(150.0 * s), yc, 3.0, false)?.0);
    } else {
        out.push(band(ink, 925.0, 230.0));
        let yc = ink.pic_y(895.0);
        let size = ink.fit(&l.ja, CAPTION, 0.9);
        out.extend(ink.line(&l.ja, Role::Body, &rm, a, b, size, INK, Across::Centre, yc, 3.0, false)?.0);
        if !l.en.trim().is_empty() {
            let y = py(ink, 968.0);
            let size = ink.fit(&l.en, SUB_LINE, 0.9);
            out.push(ink.text(&l.en, Role::Latin, size, SUB, CX, &y, &fade(a + 0.15, b, 0.35, 0.4), 2.0)?);
        }
    }
    Ok(out)
}

/// The filter chain of a slot (a card on black), from a black colour source.
fn slot(ink: &mut Ink, l: &Line, d: f64, w: i64, h: i64, fps: i64) -> crate::Result<String> {
    let s = ink.s;
    let mut f = vec![format!("color=c=black:s={w}x{h}:r={fps}:d={d:.3}")];
    let mut wipe = None;
    let rm = ink.motion.for_line(&l.kind, l.motion.as_deref());
    match l.kind.as_str() {
        "black" => {}
        "title" if ink.logo.is_some() => return Ok(logo_slot(ink, d, w, h, fps)),
        "rule" => {
            if !l.head.is_empty() {
                let y = ink.cy(452.0);
                f.push(ink.text(&l.head, Role::Quiet, HEAD, DIM, CX, &y, &fade(0.1, d - 0.05, 0.3, 0.35), 0.0)?);
            }
            let size = ink.fit(&l.ja, RULE, 0.9);
            let (lines, wp) = ink.line(&l.ja, Role::Display, &rm, 0.2, d - 0.05, size, INK, Across::Centre, 545.0 * s, 0.0, true)?;
            f.extend(lines);
            wipe = wp;
        }
        "title" => {
            let size = ink.title_px(&l.ja);
            let (lines, wp) = ink.line(&l.ja, Role::Title, &rm, 0.0, d, size, INK, Across::Centre, 530.0 * s, 0.0, true)?;
            f.extend(lines);
            wipe = wp;
            if !l.en.is_empty() {
                // under the title's glyphs, wherever their size puts them
                let y = ink.cy(530.0 + size * 0.5 + 55.0);
                f.push(ink.text(&l.en, Role::Latin, TITLE_SUB, DIM, CX, &y, &fade(0.6, d, 0.6, 0.6), 0.0)?);
            }
        }
        "message" => {
            // like the house chat: room name, a dark bubble, the sender, the message
            if !l.head.is_empty() {
                let y = ink.cy(405.0);
                f.push(ink.text(&l.head, Role::Quiet, HEAD, DIM, CX, &y, &fade(0.1, d, 0.25, 0.5), 0.0)?);
            }
            let bubble = (ems(&l.ja) * MESSAGE + 80.0).clamp(420.0, 1500.0);
            f.push(format!(
                "drawbox=x={:.0}:y={:.0}:w={:.0}:h={:.0}:color=0x292B2E:t=fill:enable='gte(t,0.25)'",
                ink.w as f64 / 2.0 - bubble / 2.0 * s,
                (548.0 - 46.0) * s,
                bubble * s,
                92.0 * s
            ));
            if !l.from.is_empty() {
                let (x, y) = (format!("{:.0}", ink.w as f64 / 2.0 - (bubble / 2.0 - 20.0) * s), ink.cy(474.0));
                f.push(ink.text(&l.from, Role::Quiet, 28.0, DIM, &x, &y, &fade(0.25, d, 0.25, 0.5), 0.0)?);
            }
            let y = ink.cy(562.0);
            f.push(ink.text(&l.ja, Role::Quiet, MESSAGE, INK, CX, &y, &fade(0.45, d, 0.3, 0.5), 0.0)?);
        }
        "end" => {
            // the end card: facts, set in the body face
            let size = ink.fit(&l.ja, END_CARD, 0.86);
            let (lines, wp) = ink.line(&l.ja, Role::Body, &rm, 0.15, d - 0.05, size, INK, Across::Centre, 540.0 * s, 0.0, true)?;
            f.extend(lines);
            wipe = wp;
        }
        _ => {
            // card: a centred line on black, a quiet English line beneath
            let size = ink.fit(&l.ja, CARD, 0.9);
            let (lines, wp) = ink.line(&l.ja, Role::Display, &rm, 0.15, d - 0.05, size, INK, Across::Centre, 520.0 * s, 0.0, true)?;
            f.extend(lines);
            wipe = wp;
            if !l.en.is_empty() {
                let y = ink.cy(610.0);
                let size = ink.fit(&l.en, SUB_LINE, 0.9);
                f.push(ink.text(&l.en, Role::Latin, size, DIM, CX, &y, &fade(0.35, d - 0.05, 0.45, 0.5), 0.0)?);
            }
        }
    }
    let mut chain = f.join(",");
    if let Some(m) = wipe {
        // a black plate over the line that slides off it to the right
        ink.n += 1;
        let n = ink.n;
        let (pw, ph) = (m.w.max(2.0).round() as i64, m.h.max(2.0).round() as i64);
        chain = format!(
            "{chain}[tx{n}];color=c=black:s={pw}x{ph}:r={fps}:d={d:.3}[mk{n}];[tx{n}][mk{n}]overlay=x='{:.0}+{:.0}*clip((t-{:.3})/{:.3},0,1)':y={:.0}:enable='lte(t,{:.3})'",
            m.x, m.w, m.a, m.fin.max(0.01), m.y, m.a + m.fin
        );
    }
    Ok(format!("{chain},format=yuv420p,setsar=1"))
}

/// The scale from the 1080-line layout for a frame `h` lines high.
fn s_of(h: i64) -> f64 {
    h as f64 / 1080.0
}

/// A glitch over [t0, t1] of a shot: the colour channels pulled apart and grain on top. The
/// frames are changed, not added, so the shot keeps its length.
fn glitch(t0: f64, t1: f64, s: f64) -> String {
    let px = (10.0 * s).round().max(2.0) as i64;
    format!(
        "rgbashift=rh=-{px}:bh={px}:enable='between(t,{t0:.3},{t1:.3})',noise=alls=40:allf=t:enable='between(t,{t0:.3},{t1:.3})'"
    )
}

/// A title slot that shows the game's logo: the image fitted into the middle of the frame
/// (its height follows the title size, 22-45% of the frame), faded in and out through black.
fn logo_slot(ink: &mut Ink, d: f64, w: i64, h: i64, fps: i64) -> String {
    ink.n += 1;
    let n = ink.n;
    let logo = ink.logo.clone().unwrap_or_default();
    let box_h = (ink.title_size.unwrap_or(TITLE_SIZE) * 2.6).clamp(0.22, 0.45) * h as f64;
    let box_w = 0.7 * w as f64;
    let fin = (d * 0.25).min(0.6);
    format!(
        "movie='{}',scale=w={box_w:.0}:h={box_h:.0}:force_original_aspect_ratio=decrease,format=rgba[lg{n}];\
         color=c=black:s={w}x{h}:r={fps}:d={d:.3}[bg{n}];\
         [bg{n}][lg{n}]overlay=x=(W-w)/2:y=(H-h)/2,fade=t=in:st=0:d={fin:.3}:color=black,\
         fade=t=out:st={:.3}:d={fin:.3}:color=black,format=yuv420p,setsar=1",
        filter_path(&logo),
        (d - fin).max(0.0)
    )
}

fn plain_line(kind: &str, ja: String, dur: f64, at: f64) -> Line {
    Line { id: String::new(), kind: kind.into(), ja, en: String::new(), head: String::new(), from: String::new(), dur,
           after: None, on: None, at, sfx: Vec::new(), motion: None }
}

/// The ffmpeg filter graph for the finishing pass. Input 0 is the plain concatenation of the
/// shots; slots are made from black inside the graph. Without fonts, text is left out (a
/// logo still shows). `cfg.logo` is read as a file path here (the caller resolves it).
/// Lines that left their face for the system font are added to `notes`.
/// Returns (graph ending in [v], seconds).
#[allow(clippy::too_many_arguments)]
pub fn filter_graph(cfg: &FinishCfg, pieces: &[CutPiece], w: i64, h: i64, fps: i64, fonts: Option<&Fonts>, text_dir: &Path,
                    notes: &mut Vec<String>) -> crate::Result<(String, f64)> {
    let fallback = Fonts::default();
    let bar = cfg.letterbox.map(|r| ((h as f64 - w as f64 / r) / 2.0).max(0.0).round() as i64).unwrap_or(0);
    let mut ink = Ink { fonts: fonts.unwrap_or(&fallback), dir: text_dir, n: 0, s: h as f64 / 1080.0, w,
                        pic_top: bar as f64, pic_h: (h - 2 * bar) as f64, title_size: cfg.title_size,
                        logo: cfg.logo.as_ref().map(PathBuf::from), notes: Vec::new(),
                        motion: crate::motion::resolve(&cfg.motion) };
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
    let mut clock = 0.0;
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
                // the hand-over from the shot before (D-53: lengths from the motion profile)
                let mo = &ink.motion;
                if let Some(CutPiece::Shot(prev)) = i.checked_sub(1).map(|j| &pieces[j]) {
                    match mo.transition(&prev.transition).as_str() {
                        "dip" => f.push(format!("fade=t=in:st=0:d={:.3}:color=black", mo.dip.min(x.duration / 2.0))),
                        "flash" => f.push(format!("fade=t=in:st=0:d={:.3}:color=white", mo.flash.min(x.duration / 2.0))),
                        // the picture comes back out of white (measured decay 0.7-1.0 s)
                        "whiteout" => f.push(format!("fade=t=in:st=0:d={:.3}:color=white", 0.8_f64.min(x.duration / 2.0))),
                        "glitch" => f.push(glitch(0.0, 0.1, s_of(h))),
                        _ => {}
                    }
                }
                if matches!(pieces.get(i + 1), Some(CutPiece::Shot(_))) {
                    match mo.transition(&x.transition).as_str() {
                        "dip" => {
                            let d = mo.dip.min(x.duration / 2.0);
                            f.push(format!("fade=t=out:st={:.3}:d={d:.3}:color=black", x.duration - d));
                        }
                        // up to white over the last moments (measured rise 0.2-0.5 s)
                        "whiteout" => {
                            let d = 0.3_f64.min(x.duration / 3.0);
                            f.push(format!("fade=t=out:st={:.3}:d={d:.3}:color=white", x.duration - d));
                        }
                        "glitch" => f.push(glitch((x.duration - 0.1).max(0.0), x.duration, s_of(h))),
                        _ => {}
                    }
                }
                // camera effects on the picture, under the text (D-57); the moves the cut avoids take theirs out
                let mut fx: Vec<crate::fx::Fx> = Vec::new();
                for e in &x.fx {
                    match crate::fx::avoided_by(&e.kind, &ink.motion.avoid) {
                        Some(by) => ink.notes.push(format!(
                            "the `{}` effect at {}s was left out: the cut's motion avoids `{by}`",
                            e.kind,
                            fmt_g(crate::geometry::round_to(clock + e.at, 2))
                        )),
                        None => fx.push(e.clone()),
                    }
                }
                f.extend(crate::fx::filters(&fx, x.duration, w, h));
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
                            let size = if cfg.title_size.is_some() { ink.title_px(&t.text) } else { ink.fit(&t.text, 120.0, 0.86) };
                            f.push(ink.text(&t.text, Role::Title, size, INK, CX, &y, &fade(a, b, 0.4, 0.5), 1.0)?);
                        }
                    }
                }
                f.push("format=yuv420p,setsar=1".into());
                graph.push(format!("[s{k}]{}[p{i}]", f.join(",")));
                k += 1;
            }
            CutPiece::Slot(l, d) => {
                let chain = if has_text || (l.kind == "title" && ink.logo.is_some()) {
                    slot(&mut ink, l, *d, w, h, fps)?
                } else {
                    format!("color=c=black:s={w}x{h}:r={fps}:d={d:.3},format=yuv420p,setsar=1")
                };
                graph.push(format!("{chain}[p{i}]"));
            }
        }
        labels.push(format!("[p{i}]"));
        clock += p.duration();
    }
    let mut total: f64 = pieces.iter().map(CutPiece::duration).sum();
    if let Some((lines, d)) = cfg.end_card.as_ref().filter(|_| has_text) {
        graph.push(format!("{}[pend]", slot(&mut ink, &plain_line("end", lines.join("\n"), *d, 0.0), *d, w, h, fps)?));
        labels.push("[pend]".into());
        total += d;
    }
    notes.append(&mut ink.notes);
    let bars = if bar > 0 {
        format!(",drawbox=x=0:y=0:w=iw:h={bar}:color=black:t=fill,drawbox=x=0:y=ih-{bar}:w=iw:h={bar}:color=black:t=fill")
    } else {
        String::new()
    };
    graph.push(format!("{}concat=n={}:v=1:a=0{bars},format=yuv420p[v]", labels.join(""), labels.len()));
    Ok((graph.join(";"), total))
}
