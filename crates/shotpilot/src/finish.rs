//! Finishing: the design burned into the cut (letterbox, title card, end card, captions,
//! dips between shots).
//!
//! Like sound (D-19), this is part of the definition, not editing after the fact (D-25):
//! it is written in shots.yaml (`finish:` and each shot's `caption` / `transition`) and
//! applied by ffmpeg when the cut is assembled. None of it is in any fingerprint, so changing
//! a line of text never re-renders a shot; it only re-runs the cut.
//!
//! No crossfades: an overlap would shorten the cut and pull the sound effects off their cuts.
//! Dips (to black or white) keep every shot's length.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::model::{fmt_g, num, str_of};

pub const TRANSITIONS: &[&str] = &["cut", "dip", "flash"];
/// Seconds a dip takes on each side of the cut.
pub const DIP: f64 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    pub text: String,
    /// Seconds from the start of its shot (title: of the title shot).
    pub at: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Default)]
pub struct FinishCfg {
    /// Picture aspect inside black bars (2.39 = scope); None = full frame.
    pub letterbox: Option<f64>,
    pub font: Option<String>,
    pub title: Option<Text>,
    pub end_card: Option<(Vec<String>, f64)>,
}

impl FinishCfg {
    pub fn is_empty(&self) -> bool {
        self.letterbox.is_none() && self.title.is_none() && self.end_card.is_none()
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
    FinishCfg {
        letterbox,
        font: m.get("font").map(str_of).filter(|s| !s.is_empty()),
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

/// Per-shot checks (called from model validation).
pub fn validate_shot(at: &str, shot: &Value, errs: &mut Vec<String>) {
    let d = shot.get("duration").and_then(num).unwrap_or(0.0);
    if let Some(c) = caption(shot) {
        if c.at < 0.0 || c.at >= d.max(0.0) {
            errs.push(format!("{at}.caption.at: {}s is outside the shot (0..{}s)", fmt_g(c.at), fmt_g(d)));
        }
    }
    if let Some(t) = shot.get("transition").filter(|v| !v.is_null()) {
        let t = str_of(t);
        if !TRANSITIONS.contains(&t.as_str()) {
            errs.push(format!("{at}.transition: one of {} (got '{t}')", TRANSITIONS.join(", ")));
        }
    }
}

/// A font file that has Japanese glyphs: `finish.font`, else a known system font.
pub fn font(cfg: &FinishCfg) -> Option<PathBuf> {
    if let Some(f) = &cfg.font {
        let p = PathBuf::from(f);
        return p.is_file().then_some(p);
    }
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:/Windows"));
    let fonts = windir.join("Fonts");
    let candidates = [
        fonts.join("YuGothB.ttc"),
        fonts.join("BIZ-UDGothicB.ttc"),
        fonts.join("meiryob.ttc"),
        fonts.join("NotoSansJP-VF.ttf"),
        PathBuf::from("/System/Library/Fonts/ヒラギノ角ゴシック W6.ttc"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// A path inside an ffmpeg filter option: forward slashes, `:` and `'` escaped.
pub fn filter_path(p: &Path) -> String {
    p.display().to_string().replace('\\', "/").replace(':', "\\:").replace('\'', "\\'")
}

/// One shot as the finishing pass sees it: where it sits in the cut and what it carries.
#[derive(Debug, Clone)]
pub struct Placed {
    pub start: f64,
    pub duration: f64,
    pub beat: Option<String>,
    pub caption: Option<Text>,
    pub transition: String,
}

/// What finishing will burn in, for the cut's spec and the app.
pub fn describe(cfg: &FinishCfg, placed: &[Placed]) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("letterbox".into(), serde_json::json!(cfg.letterbox));
    m.insert("title".into(), serde_json::json!(cfg.title.as_ref().map(|t| &t.text)));
    m.insert("end_card".into(), serde_json::json!(cfg.end_card.as_ref().map(|(l, d)| serde_json::json!({"lines": l, "duration": d}))));
    m.insert("captions".into(), serde_json::json!(placed.iter().filter(|p| p.caption.is_some()).count()));
    m.insert("dips".into(), serde_json::json!(placed.iter().filter(|p| p.transition != "cut").count()));
    m
}

/// Whether there is anything to burn in.
pub fn needed(cfg: &FinishCfg, placed: &[Placed]) -> bool {
    !cfg.is_empty() || placed.iter().any(|p| p.caption.is_some() || p.transition != "cut")
}

/// The ffmpeg filter graph for the finishing pass. Input 0 is the plain cut; text comes from
/// files in `text_dir` (so nothing in the text needs escaping). Returns (filter, total seconds).
pub fn filter_graph(cfg: &FinishCfg, placed: &[Placed], w: i64, h: i64, fps: i64, font: &Path, text_dir: &Path)
    -> crate::Result<(String, f64)> {
    let total: f64 = placed.last().map(|p| p.start + p.duration).unwrap_or(0.0);
    let fontf = filter_path(font);
    let mut n = 0;
    let mut textfile = |s: &str| -> crate::Result<String> {
        n += 1;
        let p = text_dir.join(format!("text{n}.txt"));
        std::fs::write(&p, s)?;
        Ok(filter_path(&p))
    };
    let bar = cfg.letterbox.map(|r| (((h as f64 - w as f64 / r) / 2.0).max(0.0)).round() as i64).unwrap_or(0);
    let mut graph: Vec<String> = Vec::new();
    // dips: cut the picture into shots, fade the ends that meet a dip, and join them again
    // (a fade over the whole cut would black out everything after it)
    let dipped = placed.iter().enumerate().any(|(i, p)| p.transition != "cut" && i + 1 < placed.len());
    let source = if dipped {
        let k = placed.len();
        graph.push(format!("[0:v]split={k}{}", (0..k).map(|i| format!("[s{i}]")).collect::<String>()));
        for (i, p) in placed.iter().enumerate() {
            let mut f = vec![format!("trim=start={:.4}:end={:.4}", p.start, p.start + p.duration), "setpts=PTS-STARTPTS".to_string()];
            if i > 0 && placed[i - 1].transition != "cut" {
                let color = if placed[i - 1].transition == "flash" { "white" } else { "black" };
                let d = DIP.min(p.duration / 2.0).min(placed[i - 1].duration / 2.0);
                f.push(format!("fade=t=in:st=0:d={d:.3}:color={color}"));
            }
            if p.transition != "cut" && i + 1 < placed.len() {
                let color = if p.transition == "flash" { "white" } else { "black" };
                let d = DIP.min(p.duration / 2.0).min(placed[i + 1].duration / 2.0);
                f.push(format!("fade=t=out:st={:.3}:d={d:.3}:color={color}", p.duration - d));
            }
            graph.push(format!("[s{i}]{}[t{i}]", f.join(",")));
        }
        graph.push(format!("{}concat=n={k}:v=1:a=0[cat]", (0..k).map(|i| format!("[t{i}]")).collect::<String>()));
        "[cat]"
    } else {
        "[0:v]"
    };
    let mut v: Vec<String> = Vec::new();
    if bar > 0 {
        v.push(format!("drawbox=x=0:y=0:w=iw:h={bar}:color=black:t=fill"));
        v.push(format!("drawbox=x=0:y=ih-{bar}:w=iw:h={bar}:color=black:t=fill"));
    }
    let fade_alpha = |a: f64, b: f64| {
        let f = 0.35_f64.min((b - a) / 3.0);
        format!("if(lt(t,{a:.3}),0,if(lt(t,{fa:.3}),(t-{a:.3})/{f:.3},if(lt(t,{fb:.3}),1,if(lt(t,{b:.3}),({b:.3}-t)/{f:.3},0))))",
                fa = a + f, fb = b - f)
    };
    for p in placed {
        if let Some(c) = &p.caption {
            let (a, b) = (p.start + c.at, (p.start + c.at + c.duration).min(p.start + p.duration));
            let size = (h as f64 * 0.045).round() as i64;
            let y = format!("h-{}-text_h", bar + (h as f64 * 0.09).round() as i64);
            v.push(format!(
                "drawtext=fontfile='{fontf}':textfile='{}':fontsize={size}:fontcolor=white:borderw=2:bordercolor=black@0.6:\
                 x=(w-text_w)/2:y={y}:alpha='{}':enable='between(t,{a:.3},{b:.3})'",
                textfile(&c.text)?,
                fade_alpha(a, b)
            ));
        }
    }
    if let Some(t) = &cfg.title {
        // over the title beat (else the last shot), so the card sits on the calm closing image
        let host = placed.iter().rev().find(|p| p.beat.as_deref() == Some("title")).or(placed.last());
        if let Some(hp) = host {
            let a = hp.start + t.at.min(hp.duration - 0.5).max(0.0);
            let b = (a + t.duration).min(hp.start + hp.duration);
            let size = (h as f64 * 0.11).round() as i64;
            v.push(format!(
                "drawtext=fontfile='{fontf}':textfile='{}':fontsize={size}:fontcolor=white:borderw=1:bordercolor=black@0.4:\
                 x=(w-text_w)/2:y=(h-text_h)/2-h*0.08:alpha='{}':enable='between(t,{a:.3},{b:.3})'",
                textfile(&t.text)?,
                fade_alpha(a, b)
            ));
        }
    }
    graph.push(if v.is_empty() { format!("{source}null[main]") } else { format!("{source}{}[main]", v.join(",")) });
    let mut out_total = total;
    if let Some((lines, d)) = &cfg.end_card {
        let size = (h as f64 * 0.05).round() as i64;
        let mut card = vec![format!("color=c=black:s={w}x{h}:r={fps}:d={d:.3}")];
        let n_lines = lines.len() as f64;
        for (i, l) in lines.iter().enumerate() {
            let y = format!("(h-text_h)/2+({:.3})*{}", i as f64 - (n_lines - 1.0) / 2.0, (size as f64 * 1.6).round());
            card.push(format!(
                "drawtext=fontfile='{fontf}':textfile='{}':fontsize={size}:fontcolor=white:x=(w-text_w)/2:y={y}:alpha='{}'",
                textfile(l)?,
                fade_alpha(0.0, *d)
            ));
        }
        card.push("format=yuv420p".into());
        graph.push(format!("{}[card]", card.join(",")));
        graph.push("[main]format=yuv420p,setsar=1[m2]".into());
        graph.push("[card]setsar=1[c2]".into());
        graph.push("[m2][c2]concat=n=2:v=1:a=0[v]".into());
        out_total += d;
    } else {
        graph.push("[main]format=yuv420p[v]".into());
    }
    Ok((graph.join(";"), out_total))
}
