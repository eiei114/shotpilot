//! How shots hand over at a cut (D-70): the recipes in `references/motion/transitions.json`,
//! turned into ffmpeg filters on the two shots either side of the cut.
//!
//! Two pictures never share a frame (no crossfade, D-25). The shot going out is covered
//! (black, white or broken frames) over its last moments, the next one starts covered and
//! uncovers, and the swap is always the cut itself, made while the picture is covered (at
//! peak white, or under black or broken frames). Frames are changed, never
//! added, so no shot changes length. The planner names a hand-over (`transition:`); the recipe,
//! the look's motion profile and its energy decide how long each side runs and how it moves.
//!
//! A cut with no `finish.motion` keeps the hand-overs it had before the recipes: straight fades
//! and the old glitch (`plain`, the D-53 invariant).

use std::sync::OnceLock;

use serde_json::Value;

use crate::model::{num, str_of};
use crate::motion::Motion;

const RECIPES: &str = include_str!("../../../references/motion/transitions.json");

/// Ease names a recipe may use: GSAP's (power1 = squared ... power4
/// = fifth power). `.in` accelerates (into a cover), `.out` decelerates (out of one).
pub const EASES: &[&str] = &[
    "linear", "sine.in", "sine.out", "sine.inOut", "power1.in", "power1.out", "power1.inOut", "power2.in", "power2.out",
    "power2.inOut", "power3.in", "power3.out", "power3.inOut", "power4.in", "power4.out", "power4.inOut", "expo.in",
    "expo.out", "expo.inOut",
];

/// The side of the cut a step is drawn on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The last moments of the shot going out.
    Out,
    /// The first moments of the shot coming in.
    In,
}

/// One thing a hand-over does on one side of the cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub side: Side,
    /// `black` or `white` for a cover; empty for a break (broken frames).
    pub cover: String,
    /// Seconds: a number, or the name of a length the motion profile holds (`dip`, `flash`,
    /// `rise`, `peak`, `decay`, `glitch`).
    pub len: String,
    /// Seconds the cover is held before it lifts (incoming side only).
    pub hold: Option<String>,
    /// The share of the shot this side may take at most.
    pub max: f64,
    pub ease: String,
    /// How far the exposure is pushed as the picture goes to white (a factor; 1 for none).
    pub burn: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub name: String,
    /// What the hand-over says about the two shots (every prompt names them this way).
    pub means: String,
    pub steps: Vec<Step>,
}

fn raw() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(RECIPES).unwrap_or(Value::Null))
}

/// The recipes, in the order the file lists them.
pub fn recipes() -> &'static [Recipe] {
    static CELL: OnceLock<Vec<Recipe>> = OnceLock::new();
    CELL.get_or_init(|| {
        raw()["transitions"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(name, r)| Recipe {
                name: name.clone(),
                means: r["means"].as_str().unwrap_or("").to_string(),
                steps: r["steps"].as_array().into_iter().flatten().map(step).collect(),
            })
            .collect()
    })
}

fn step(v: &Value) -> Step {
    let amount = |k: &str| match &v[k] {
        Value::Null => None,
        Value::Number(n) => Some(n.to_string()),
        x => Some(str_of(x)),
    };
    Step {
        side: if v["side"].as_str() == Some("in") { Side::In } else { Side::Out },
        cover: v["cover"].as_str().unwrap_or("").to_string(),
        len: amount("len").unwrap_or_else(|| "0".into()),
        hold: amount("hold"),
        max: num(&v["max"]).unwrap_or(0.5),
        ease: v["ease"].as_str().unwrap_or("linear").to_string(),
        burn: amount("burn"),
    }
}

pub fn recipe(name: &str) -> Option<&'static Recipe> {
    recipes().iter().find(|r| r.name == name)
}

/// A length the recipes name, from `transitions.json` (`lengths`): the defaults a motion
/// profile's `whiteout` and `glitch` fall back to.
pub fn default_length(name: &str) -> f64 {
    num(&raw()["lengths"][name]).unwrap_or(match name {
        "burn" => 1.0,
        "glitch" => 0.1,
        _ => 0.0,
    })
}

/// The hand-overs and what each says, for every prompt that names them (one wording).
pub fn vocabulary() -> String {
    crate::finish::TRANSITIONS
        .iter()
        .map(|t| match recipe(t).map(|r| r.means.as_str()).filter(|m| !m.is_empty()) {
            Some(m) => format!("{t} ({m})"),
            None => t.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The eased share (0..1) of a progress expression `p` (0..1), by GSAP's names.
pub fn ease(name: &str, p: &str) -> String {
    let (family, dir) = name.split_once('.').unwrap_or((name, ""));
    let n = match family {
        "power1" => 2,
        "power2" => 3,
        "power3" => 4,
        "power4" => 5,
        _ => 0,
    };
    match (family, dir) {
        ("sine", "in") => format!("(1-cos({p}*PI/2))"),
        ("sine", "out") => format!("sin({p}*PI/2)"),
        ("sine", "inOut") => format!("(0.5-0.5*cos({p}*PI))"),
        ("expo", "in") => format!("if(lte({p},0),0,pow(2,10*{p}-10))"),
        ("expo", "out") => format!("if(gte({p},1),1,1-pow(2,-10*{p}))"),
        ("expo", "inOut") => {
            format!("if(lte({p},0),0,if(gte({p},1),1,if(lt({p},0.5),pow(2,20*{p}-10)/2,(2-pow(2,10-20*{p}))/2)))")
        }
        (_, "in") if n > 0 => format!("pow({p},{n})"),
        (_, "out") if n > 0 => format!("(1-pow(1-{p},{n}))"),
        (_, "inOut") if n > 0 => format!("if(lt({p},0.5),pow(2*{p},{n})/2,1-pow(2-2*{p},{n})/2)"),
        _ => p.to_string(),
    }
}

/// The ffmpeg filters one side of a hand-over puts on a shot `d` seconds long, on its own clock
/// (`t` is 0 at its first frame; `h` lines high, `fps` frames a second). The hand-over is first
/// put through the moves the cut avoids.
pub fn filters(m: &Motion, kind: &str, side: Side, d: f64, h: i64, fps: i64) -> Vec<String> {
    let kind = m.transition(kind);
    if m.plain {
        return plain(m, &kind, side, d, h);
    }
    let Some(r) = recipe(&kind) else {
        return Vec::new();
    };
    let frame = 1.0 / fps.max(1) as f64;
    let s = h as f64 / 1080.0;
    let mut out = Vec::new();
    for st in r.steps.iter().filter(|st| st.side == side) {
        let mut len = m.handover_value(&st.len);
        let mut hold = st.hold.as_deref().map_or(0.0, |x| m.handover_value(x));
        let room = st.max * d;
        if hold + len > room && hold + len > 0.0 {
            let k = room / (hold + len);
            hold *= k;
            len *= k;
        }
        if len <= 0.0 {
            continue;
        }
        if st.cover.is_empty() {
            let (a, b) = match side {
                Side::Out => ((d - len).max(0.0), d),
                Side::In => (0.0, len.min(d)),
            };
            out.push(broken(a, b, s));
            continue;
        }
        let burn = st.burn.as_deref().map_or(1.0, |x| m.handover_value(x)).max(1.0);
        let (a, b, k) = match side {
            // fully covered on the shot's last frame: the swap happens under the cover
            Side::Out => {
                let end = (d - frame).max(0.0);
                let a = (end - len).max(0.0);
                let p = format!("clip((t-{a:.3})/{:.3},0,1)", (end - a).max(frame));
                (a, d, ease(&st.ease, &p))
            }
            Side::In => {
                let p = format!("clip((t-{hold:.3})/{len:.3},0,1)");
                (0.0, (hold + len).min(d), format!("(1-{})", ease(&st.ease, &p)))
            }
        };
        out.push(cover(&st.cover, &k, burn, a, b));
    }
    out
}

/// A cover over [a, b] of a shot: `k` (an expression of `t`, 0..1) of the picture is turned to
/// black or white, a straight blend done with `eq` (contrast g(1-k), brightness k*w + g(1-k)/2
/// - 1/2, saturation 1-k). `g` pushes the exposure up with the cover, to `burn` at full white.
fn cover(colour: &str, k: &str, burn: f64, a: f64, b: f64) -> String {
    // video levels: black is 16 and white 235 of 255, as ffmpeg's fade draws them
    let w = if colour == "white" { 235.0 / 255.0 } else { 16.0 / 255.0 };
    let gain = if burn > 1.0 { format!("(1+{:.3}*{k})", burn - 1.0) } else { "1".to_string() };
    let c = format!("{gain}*(1-{k})");
    format!(
        "eq=contrast='{c}':brightness='{w:.4}*{k}+0.5*{c}-0.5':saturation='1-{k}':eval=frame:enable='between(t,{a:.3},{b:.3})'"
    )
}

/// Broken frames over [a, b] of a shot: the colour layers thrown one way on even frames and the
/// other way on odd ones, with grain (a glitch jitters every 0.03 s or so).
fn broken(a: f64, b: f64, s: f64) -> String {
    let px = |v: f64| (v * s).round().max(2.0) as i64;
    let (p, q, v) = (px(12.0), px(20.0), px(3.0));
    let on = format!("between(t,{a:.3},{b:.3})");
    format!(
        "rgbashift=rh=-{p}:bh={p}:enable='{on}*not(mod(n,2))',rgbashift=rh={q}:bh=-{q}:rv={v}:enable='{on}*mod(n,2)',\
         noise=alls=40:allf=t:enable='{on}'"
    )
}

/// The hand-overs as the cut drew them before the recipes (no `finish.motion`): straight fades,
/// the old glitch. Kept expression for expression.
fn plain(m: &Motion, kind: &str, side: Side, d: f64, h: i64) -> Vec<String> {
    let s = h as f64 / 1080.0;
    let f = match (side, kind) {
        (Side::In, "dip") => format!("fade=t=in:st=0:d={:.3}:color=black", m.dip.min(d / 2.0)),
        (Side::In, "flash") => format!("fade=t=in:st=0:d={:.3}:color=white", m.flash.min(d / 2.0)),
        // the picture comes back out of white
        (Side::In, "whiteout") => format!("fade=t=in:st=0:d={:.3}:color=white", m.whiteout.decay.min(d / 2.0)),
        (Side::In, "glitch") => glitch(0.0, m.glitch, s),
        (Side::Out, "dip") => {
            let x = m.dip.min(d / 2.0);
            format!("fade=t=out:st={:.3}:d={x:.3}:color=black", d - x)
        }
        // up to white over the last moments
        (Side::Out, "whiteout") => {
            let x = m.whiteout.rise.min(d / 3.0);
            format!("fade=t=out:st={:.3}:d={x:.3}:color=white", d - x)
        }
        (Side::Out, "glitch") => glitch((d - m.glitch).max(0.0), d, s),
        _ => return Vec::new(),
    };
    vec![f]
}

/// The glitch before the recipes: the colour channels pulled apart and grain on top over
/// [t0, t1] of a shot.
fn glitch(t0: f64, t1: f64, s: f64) -> String {
    let px = (10.0 * s).round().max(2.0) as i64;
    format!(
        "rgbashift=rh=-{px}:bh={px}:enable='between(t,{t0:.3},{t1:.3})',noise=alls=40:allf=t:enable='between(t,{t0:.3},{t1:.3})'"
    )
}

/// Keeps a planned cut to one main hand-over (`cut`) and at most two others (one primary
/// transition and one or two accents): `dip` and the look's accent stay first, then
/// the most used; the rest become cuts. Returns a note for the human when anything changed.
pub fn shape(shots: &mut [Value], accent: &str) -> Option<String> {
    // (kind, uses, first shot)
    let mut kinds: Vec<(String, usize, usize)> = Vec::new();
    for (i, s) in shots.iter().enumerate() {
        let Some(t) = s["transition"].as_str().filter(|t| *t != "cut") else {
            continue;
        };
        match kinds.iter_mut().find(|k| k.0 == t) {
            Some(k) => k.1 += 1,
            None => kinds.push((t.to_string(), 1, i)),
        }
    }
    if kinds.len() <= 2 {
        return None;
    }
    kinds.sort_by_key(|k| (k.0 != accent, k.0 != "dip", std::cmp::Reverse(k.1), k.2));
    let keep: Vec<String> = kinds.iter().take(2).map(|k| k.0.clone()).collect();
    let mut changed = Vec::new();
    for s in shots.iter_mut() {
        let Some(t) = s["transition"].as_str().filter(|t| *t != "cut" && !keep.iter().any(|k| k == t)).map(str::to_string)
        else {
            continue;
        };
        if let Some(o) = s.as_object_mut() {
            o.remove("transition");
        }
        changed.push(format!("{} {t}", str_of(&s["id"])));
    }
    Some(format!("つなぎを cut のほか 2 種（{}）に絞った: {}", keep.join(", "), changed.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{parse, resolve};
    use serde_json::json;

    fn motion(v: Value) -> Motion {
        let mut errs = Vec::new();
        let m = resolve(&parse(Some(&v), &mut errs));
        assert!(errs.is_empty(), "{errs:?}");
        m
    }

    #[test]
    fn every_recipe_is_a_hand_over_with_known_parts() {
        let names: Vec<&str> = recipes().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, crate::finish::TRANSITIONS, "one recipe per hand-over, in the same order");
        let lengths = ["dip", "flash", "rise", "peak", "decay", "burn", "glitch"];
        for r in recipes() {
            assert!(!r.means.is_empty(), "{}", r.name);
            for st in &r.steps {
                assert!(EASES.contains(&st.ease.as_str()), "{}: {}", r.name, st.ease);
                assert!(["", "black", "white"].contains(&st.cover.as_str()), "{}: {}", r.name, st.cover);
                assert!((0.0..=0.5).contains(&st.max), "{}: one side takes at most half the shot", r.name);
                for x in [Some(&st.len), st.hold.as_ref(), st.burn.as_ref()].into_iter().flatten() {
                    assert!(lengths.contains(&x.as_str()) || x.parse::<f64>().is_ok(), "{}: {x}", r.name);
                }
            }
        }
        assert!(vocabulary().starts_with("cut (") && vocabulary().contains("whiteout (up to white"), "{}", vocabulary());
    }

    #[test]
    fn the_base_profile_draws_the_hand_overs_it_drew_before() {
        // D-53: with no finish.motion the cut is drawn expression for expression as before
        let m = Motion::default();
        assert!(m.plain);
        let f = |k: &str, side: Side, d: f64| filters(&m, k, side, d, 1080, 30).join(",");
        assert_eq!(f("dip", Side::In, 3.0), "fade=t=in:st=0:d=0.250:color=black");
        assert_eq!(f("dip", Side::Out, 3.0), "fade=t=out:st=2.750:d=0.250:color=black");
        assert_eq!(f("flash", Side::In, 3.0), "fade=t=in:st=0:d=0.120:color=white");
        assert_eq!(f("flash", Side::Out, 3.0), "");
        assert_eq!(f("whiteout", Side::In, 3.0), "fade=t=in:st=0:d=0.800:color=white");
        assert_eq!(f("whiteout", Side::Out, 0.6), "fade=t=out:st=0.400:d=0.200:color=white");
        assert_eq!(
            f("glitch", Side::Out, 2.0),
            "rgbashift=rh=-10:bh=10:enable='between(t,1.900,2.000)',noise=alls=40:allf=t:enable='between(t,1.900,2.000)'"
        );
        assert_eq!(f("cut", Side::Out, 2.0), "");
    }

    #[test]
    fn eases_follow_gsaps_shapes() {
        // evaluate the expressions the way ffmpeg would, for a few progress values
        let eval = |e: &str, p: f64| calc(&e.replace('X', &format!("({p})")));
        for (name, p, want) in [
            ("linear", 0.5, 0.5),
            ("power2.in", 0.5, 0.125),
            ("power2.out", 0.5, 0.875),
            ("power1.inOut", 0.25, 0.125),
            ("sine.out", 1.0, 1.0),
            ("expo.in", 0.0, 0.0),
        ] {
            let got = eval(&ease(name, "X"), p);
            assert!((got - want).abs() < 1e-9, "{name}({p}) = {got}, want {want}");
        }
        for name in EASES {
            // every ease starts at 0 and lands on 1
            let e = ease(name, "X");
            assert!(eval(&e, 0.0).abs() < 1e-9 && (eval(&e, 1.0) - 1.0).abs() < 1e-9, "{name}: {e}");
        }
    }

    /// A small evaluator for the expressions `ease` writes (numbers, + - * /, pow, cos, sin, PI,
    /// if, lt, lte, gte).
    fn calc(e: &str) -> f64 {
        fn expr(s: &[u8], i: &mut usize) -> f64 {
            let mut v = term(s, i);
            while *i < s.len() && (s[*i] == b'+' || s[*i] == b'-') {
                let op = s[*i];
                *i += 1;
                let r = term(s, i);
                v = if op == b'+' { v + r } else { v - r };
            }
            v
        }
        fn term(s: &[u8], i: &mut usize) -> f64 {
            let mut v = atom(s, i);
            while *i < s.len() && (s[*i] == b'*' || s[*i] == b'/') {
                let op = s[*i];
                *i += 1;
                let r = atom(s, i);
                v = if op == b'*' { v * r } else { v / r };
            }
            v
        }
        fn args(s: &[u8], i: &mut usize) -> Vec<f64> {
            *i += 1; // (
            let mut out = vec![expr(s, i)];
            while s[*i] == b',' {
                *i += 1;
                out.push(expr(s, i));
            }
            *i += 1; // )
            out
        }
        fn atom(s: &[u8], i: &mut usize) -> f64 {
            if s[*i] == b'(' {
                *i += 1;
                let v = expr(s, i);
                *i += 1;
                return v;
            }
            if s[*i] == b'-' {
                *i += 1;
                return -atom(s, i);
            }
            let start = *i;
            while *i < s.len() && (s[*i].is_ascii_alphanumeric() || s[*i] == b'.') {
                *i += 1;
            }
            let word = std::str::from_utf8(&s[start..*i]).unwrap();
            match word {
                "PI" => std::f64::consts::PI,
                "pow" => {
                    let a = args(s, i);
                    a[0].powf(a[1])
                }
                "cos" => args(s, i)[0].cos(),
                "sin" => args(s, i)[0].sin(),
                "lt" | "lte" | "gte" => {
                    let a = args(s, i);
                    let t = match word {
                        "lt" => a[0] < a[1],
                        "lte" => a[0] <= a[1],
                        _ => a[0] >= a[1],
                    };
                    f64::from(u8::from(t))
                }
                "if" => {
                    let a = args(s, i);
                    if a[0] != 0.0 { a[1] } else { a[2] }
                }
                n => n.parse().unwrap(),
            }
        }
        let s = e.replace(' ', "");
        let mut i = 0;
        expr(s.as_bytes(), &mut i)
    }

    #[test]
    fn a_dip_is_covered_on_both_sides_of_the_cut() {
        let m = motion(json!("strategy")); // dip 0.55 s
        let out = filters(&m, "dip", Side::Out, 3.0, 1080, 30).join(",");
        // eased into black, over the last 0.55 s, black on the last frame (2.967 s)
        assert!(out.starts_with("eq=contrast='1*(1-pow(clip((t-2.417)/0.550,0,1),3))'"), "{out}");
        assert!(out.contains(":eval=frame:enable='between(t,2.417,3.000)'"), "{out}");
        let inn = filters(&m, "dip", Side::In, 3.0, 1080, 30).join(",");
        assert!(inn.contains("(1-(1-pow(1-clip((t-0.000)/0.550,0,1),3)))"), "{inn}");
        assert!(inn.contains("enable='between(t,0.000,0.550)'"), "{inn}");
        // a short shot gives each side at most half of itself
        let short = filters(&m, "dip", Side::In, 0.8, 1080, 30).join(",");
        assert!(short.contains("/0.400,0,1)"), "{short}");
    }

    #[test]
    fn a_flash_holds_white_for_its_length_then_lets_go() {
        // measured flash lengths are how long the white is seen (D-71 measures them that way)
        let m = motion(json!("horror")); // flash 0.12 s
        let inn = filters(&m, "flash", Side::In, 3.0, 1080, 30).join(",");
        assert!(inn.contains("clip((t-0.120)/0.050,0,1)") && inn.contains("between(t,0.000,0.170)"), "{inn}");
        assert!(filters(&m, "flash", Side::Out, 3.0, 1080, 30).is_empty(), "the shot going out just cuts");
    }

    #[test]
    fn a_whiteout_burns_holds_and_comes_back() {
        let m = motion(json!("rpg"));
        let out = filters(&m, "whiteout", Side::Out, 3.0, 1080, 30).join(",");
        assert!(out.contains("(1+1.000*pow(clip((t-2.617)/0.350,0,1),3))*(1-"), "{out}"); // exposure to 2x
        assert!(out.contains("0.9216*"), "{out}"); // white at video level 235
        let inn = filters(&m, "whiteout", Side::In, 3.0, 1080, 30).join(",");
        // white held 0.15 s, then 0.85 s back
        assert!(inn.contains("clip((t-0.150)/0.850,0,1)") && inn.contains("between(t,0.000,1.000)"), "{inn}");
        // the hold and the return shrink together to fit half of a short shot
        let short = filters(&m, "whiteout", Side::In, 1.0, 1080, 30).join(",");
        assert!(short.contains("clip((t-0.075)/0.425,0,1)"), "{short}");
    }

    #[test]
    fn energy_scales_every_hand_over_and_glitches_jitter() {
        let calm = motion(json!({"profile": "rpg", "energy": "calm"}));
        let rpg = motion(json!("rpg"));
        assert!((calm.whiteout.decay - rpg.whiteout.decay * 1.4).abs() < 1e-9);
        assert!((calm.flash - rpg.flash * 1.4).abs() < 1e-9);
        assert_eq!(calm.whiteout.burn, rpg.whiteout.burn, "the exposure push is not a length");
        let g = filters(&motion(json!("horror-ja")), "glitch", Side::In, 2.0, 1080, 30).join(",");
        assert!(g.contains("enable='between(t,0.000,0.100)*not(mod(n,2))'") && g.contains("*mod(n,2)'"), "{g}");
        assert!(g.contains("rh=-12:bh=12") && g.contains("rh=20:bh=-20:rv=3"), "{g}");
        // an avoided hand-over is drawn as what stands in for it
        let no_white = motion(json!({"profile": "rpg", "avoid": ["whiteout"]}));
        assert!(filters(&no_white, "whiteout", Side::Out, 3.0, 1080, 30)[0].contains("0.0627*"), "dip instead");
        let no_flash = motion(json!({"profile": "action", "avoid": ["flash"]}));
        assert!(filters(&no_flash, "flash", Side::In, 3.0, 1080, 30).is_empty());
    }

    #[test]
    fn a_plan_keeps_cut_and_two_other_hand_overs() {
        let mut shots = vec![
            json!({"id": "a", "transition": "glitch"}),
            json!({"id": "b"}),
            json!({"id": "c", "transition": "flash"}),
            json!({"id": "d", "transition": "whiteout"}),
            json!({"id": "e", "transition": "flash"}),
            json!({"id": "f", "transition": "dip"}),
            json!({"id": "g"}),
        ];
        // the look's accent (whiteout) and dip stay; flash and glitch become cuts
        let note = shape(&mut shots, "whiteout").unwrap();
        let left: Vec<&str> = shots.iter().map(|s| s["transition"].as_str().unwrap_or("cut")).collect();
        assert_eq!(left, ["cut", "cut", "cut", "whiteout", "cut", "dip", "cut"]);
        assert!(note.contains("whiteout, dip") && note.contains("a glitch") && note.contains("c flash"), "{note}");
        // two kinds or fewer are left alone
        let mut two = vec![json!({"id": "a", "transition": "flash"}), json!({"id": "b", "transition": "dip"})];
        assert!(shape(&mut two, "glitch").is_none());
        // with the accent unused, dip and the most used stay
        let mut shots = vec![
            json!({"id": "a", "transition": "glitch"}),
            json!({"id": "b", "transition": "flash"}),
            json!({"id": "c", "transition": "flash"}),
            json!({"id": "d", "transition": "dip"}),
        ];
        shape(&mut shots, "whiteout").unwrap();
        assert!(shots[0].get("transition").is_none() && shots[1]["transition"] == "flash");
    }
}
