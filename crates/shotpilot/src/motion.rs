//! How the burned-in text moves and how shots hand over (D-53).
//!
//! A small vocabulary the planner picks from and code turns into ffmpeg expressions, after
//! open-design's motion-design profile ("the model picks from enums, code sets the numbers"):
//! entrances (`cut fade flicker focus slam pop type wipe glitch`), holds (`still drift glow`),
//! exits (`cut fade glitch`) and the hand-overs between shots (`cut dip flash whiteout glitch`).
//!
//! The numbers come from `references/motion/profiles.json`: one profile per genre, measured
//! frame by frame on 33 teasers and from luma on 736 (research_notes/モーションデザイン/).
//! `finish.motion` names a profile and may change its energy, rule moves out (`avoid`) or
//! override a role; a script line may ask for its own entrance (`motion:`). With no
//! `finish.motion` the `base` profile applies: the motion the cut had before it was settable.
//!
//! Everything here is editorial: it changes the cut, never a take, and never the cut's length.

use std::sync::OnceLock;

use serde_json::{json, Map, Value};

use crate::model::{num, str_of};

const PROFILES: &str = include_str!("../../../references/motion/profiles.json");

pub const ENTRANCES: &[&str] = &["cut", "fade", "flicker", "focus", "slam", "pop", "type", "wipe", "glitch"];
pub const HOLDS: &[&str] = &["still", "drift", "glow"];
pub const EXITS: &[&str] = &["cut", "fade", "glitch"];
/// The roles a line plays: cards on black (card, rule, message), text over a shot (caption,
/// count), the title and the end card.
pub const ROLES: &[&str] = &["card", "caption", "count", "title", "end"];
pub const ENERGIES: &[&str] = &["calm", "measured", "driving"];
/// Moves a look or a note may rule out.
pub const AVOIDABLE: &[&str] =
    &["fade", "flicker", "focus", "slam", "pop", "type", "wipe", "glitch", "drift", "glow", "dip", "flash", "whiteout",
      "chroma", "static", "vhs", "shake", "redflash"];

/// Seconds an entrance takes when a line asks for one its profile has no time for
/// (measured: slams 0.10-0.17 s, glitches 0.3-0.75 s, focus pulls 0.9-1.8 s).
pub fn default_secs(kind: &str) -> f64 {
    match kind {
        "cut" => 0.0,
        "flicker" => 0.6,
        "focus" => 1.0,
        "slam" => 0.15,
        "type" => 0.6,
        "wipe" => 0.4,
        "glitch" => 0.3,
        _ => 0.5,
    }
}

/// How one role moves: in, hold, out (seconds already scaled by energy).
#[derive(Debug, Clone, PartialEq)]
pub struct RoleMotion {
    pub enter: String,
    pub enter_s: f64,
    pub hold: String,
    pub exit: String,
    pub exit_s: f64,
}

/// A profile resolved for one cut.
#[derive(Debug, Clone, PartialEq)]
pub struct Motion {
    pub profile: String,
    pub energy: String,
    pub card: RoleMotion,
    pub caption: RoleMotion,
    pub count: RoleMotion,
    pub title: RoleMotion,
    pub end: RoleMotion,
    /// Seconds a dip takes on each side of the cut.
    pub dip: f64,
    /// Seconds of a flash from white.
    pub flash: f64,
    /// The hand-over the profile uses for a jolt.
    pub accent: String,
    pub avoid: Vec<String>,
}

impl Default for Motion {
    fn default() -> Self {
        resolve(&MotionCfg::default())
    }
}

pub struct Profile {
    pub id: String,
    pub name: String,
    pub energy: String,
    roles: Map<String, Value>,
    pub dip: f64,
    pub flash: f64,
    pub accent: String,
    pub avoid: Vec<String>,
}

fn raw() -> &'static Value {
    static CELL: OnceLock<Value> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(PROFILES).unwrap_or(Value::Null))
}

pub fn profiles() -> &'static [Profile] {
    static CELL: OnceLock<Vec<Profile>> = OnceLock::new();
    CELL.get_or_init(|| {
        raw()["profiles"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(id, p)| Profile {
                id: id.clone(),
                name: p["name"].as_str().unwrap_or(id).to_string(),
                energy: p["energy"].as_str().unwrap_or("measured").to_string(),
                roles: ROLES.iter().filter_map(|r| p.get(*r).map(|v| (r.to_string(), v.clone()))).collect(),
                dip: num(&p["dip"]).unwrap_or(0.25),
                flash: num(&p["flash"]).unwrap_or(0.12),
                accent: p["accent"].as_str().unwrap_or("flash").to_string(),
                avoid: p["avoid"].as_array().into_iter().flatten().map(str_of).collect(),
            })
            .collect()
    })
}

pub fn profile(id: &str) -> Option<&'static Profile> {
    profiles().iter().find(|p| p.id == id)
}

fn energy_factor(e: &str) -> f64 {
    num(&raw()["energies"][e]).unwrap_or(1.0)
}

/// `finish.motion` as written in shots.yaml.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotionCfg {
    pub profile: Option<String>,
    pub energy: Option<String>,
    pub avoid: Vec<String>,
    /// Role overrides (`card: {in: fade, in_s: 0.8}`), as written.
    pub roles: Vec<(String, Map<String, Value>)>,
    pub dip: Option<f64>,
    pub accent: Option<String>,
}

impl MotionCfg {
    pub fn is_set(&self) -> bool {
        *self != MotionCfg::default()
    }

    /// Back to the shape it is written in (for edits that merge into it).
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        if let Some(p) = &self.profile {
            m.insert("profile".into(), json!(p));
        }
        if let Some(e) = &self.energy {
            m.insert("energy".into(), json!(e));
        }
        if !self.avoid.is_empty() {
            m.insert("avoid".into(), json!(self.avoid));
        }
        for (r, o) in &self.roles {
            m.insert(r.clone(), Value::Object(o.clone()));
        }
        if let Some(d) = self.dip {
            m.insert("dip".into(), json!(d));
        }
        if let Some(a) = &self.accent {
            m.insert("accent".into(), json!(a));
        }
        if m.is_empty() { Value::Null } else { Value::Object(m) }
    }
}

const ACCENTS: &[&str] = &["dip", "flash", "whiteout", "glitch"];

/// `finish.motion`: a profile name, or {profile, energy, avoid, dip, accent, <role>: {in,
/// in_s, hold, out, out_s}}. Problems go to `errs`.
pub fn parse(v: Option<&Value>, errs: &mut Vec<String>) -> MotionCfg {
    let mut cfg = MotionCfg::default();
    let names = || profiles().iter().map(|p| p.id.as_str()).collect::<Vec<_>>().join(", ");
    let m = match v {
        None | Some(Value::Null) => return cfg,
        Some(Value::String(s)) if s.trim().is_empty() => return cfg,
        Some(Value::String(s)) => {
            let mut m = Map::new();
            m.insert("profile".into(), json!(s.trim()));
            m
        }
        Some(Value::Object(m)) => m.clone(),
        Some(_) => {
            errs.push("finish.motion: a profile name or {profile, energy, avoid, <role>: {in, out, ...}}".into());
            return cfg;
        }
    };
    for (k, v) in &m {
        match k.as_str() {
            "profile" => {
                let p = str_of(v);
                if profile(&p).is_none() {
                    errs.push(format!("finish.motion.profile: one of {} (got '{p}')", names()));
                } else {
                    cfg.profile = Some(p);
                }
            }
            "energy" => {
                let e = str_of(v);
                if !e.is_empty() && !ENERGIES.contains(&e.as_str()) {
                    errs.push(format!("finish.motion.energy: one of {} (got '{e}')", ENERGIES.join(", ")));
                } else if !e.is_empty() {
                    cfg.energy = Some(e);
                }
            }
            "avoid" => {
                for a in v.as_array().into_iter().flatten().map(str_of) {
                    if AVOIDABLE.contains(&a.as_str()) {
                        if !cfg.avoid.contains(&a) {
                            cfg.avoid.push(a);
                        }
                    } else {
                        errs.push(format!("finish.motion.avoid: '{a}' is not a move ({})", AVOIDABLE.join(", ")));
                    }
                }
            }
            "dip" => match num(v) {
                Some(d) if (0.05..=1.5).contains(&d) => cfg.dip = Some(d),
                _ => errs.push("finish.motion.dip: seconds on each side of the cut, 0.05..1.5".into()),
            },
            "accent" => {
                let a = str_of(v);
                if ACCENTS.contains(&a.as_str()) {
                    cfg.accent = Some(a);
                } else {
                    errs.push(format!("finish.motion.accent: one of {} (got '{a}')", ACCENTS.join(", ")));
                }
            }
            role if ROLES.contains(&role) => {
                let Some(o) = v.as_object() else {
                    errs.push(format!("finish.motion.{role}: {{in, in_s, hold, out, out_s}}"));
                    continue;
                };
                for (f, x) in o {
                    let ok = match f.as_str() {
                        "in" => ENTRANCES.contains(&str_of(x).as_str()),
                        "hold" => HOLDS.contains(&str_of(x).as_str()),
                        "out" => EXITS.contains(&str_of(x).as_str()),
                        "in_s" | "out_s" => num(x).is_some_and(|s| (0.0..=3.0).contains(&s)),
                        _ => false,
                    };
                    if !ok {
                        errs.push(format!(
                            "finish.motion.{role}.{f}: in ({}), hold ({}), out ({}), in_s / out_s (0..3 s)",
                            ENTRANCES.join(" "), HOLDS.join(" "), EXITS.join(" ")
                        ));
                    }
                }
                cfg.roles.push((role.to_string(), o.clone()));
            }
            other => errs.push(format!("finish.motion.{other}: unknown (profile, energy, avoid, dip, accent, {})", ROLES.join(", "))),
        }
    }
    cfg
}

fn role_from(v: &Value, scale: f64) -> Option<RoleMotion> {
    let o = v.as_object()?;
    Some(RoleMotion {
        enter: o.get("in").map(str_of).unwrap_or_else(|| "fade".into()),
        enter_s: o.get("in_s").and_then(num).unwrap_or(0.0) * scale,
        hold: o.get("hold").map(str_of).unwrap_or_else(|| "still".into()),
        exit: o.get("out").map(str_of).unwrap_or_else(|| "fade".into()),
        exit_s: o.get("out_s").and_then(num).unwrap_or(0.0) * scale,
    })
}

/// The motion of a cut: the profile, its energy, the overrides, then what is avoided.
pub fn resolve(cfg: &MotionCfg) -> Motion {
    let base = profile("base");
    let p = cfg.profile.as_deref().and_then(profile).or(base);
    let p_energy = p.map_or("measured", |p| p.energy.as_str());
    let energy = cfg.energy.clone().unwrap_or_else(|| p_energy.to_string());
    let scale = energy_factor(&energy) / energy_factor(p_energy);
    let fallback = RoleMotion { enter: "fade".into(), enter_s: 0.4, hold: "still".into(), exit: "fade".into(), exit_s: 0.4 };
    let role = |r: &str| -> RoleMotion {
        let from_profile = p.and_then(|p| p.roles.get(r)).or_else(|| base.and_then(|b| b.roles.get(r)));
        let mut rm = from_profile.and_then(|v| role_from(v, scale)).unwrap_or_else(|| fallback.clone());
        if let Some((_, o)) = cfg.roles.iter().find(|(k, _)| k == r) {
            if let Some(x) = o.get("in").map(str_of) {
                if x != rm.enter {
                    rm.enter_s = o.get("in_s").and_then(num).unwrap_or(default_secs(&x) * energy_factor(&energy));
                }
                rm.enter = x;
            }
            if let Some(s) = o.get("in_s").and_then(num) {
                rm.enter_s = s;
            }
            if let Some(x) = o.get("hold").map(str_of) {
                rm.hold = x;
            }
            if let Some(x) = o.get("out").map(str_of) {
                if x != rm.exit {
                    rm.exit_s = o.get("out_s").and_then(num).unwrap_or(if x == "cut" { 0.0 } else { 0.5 });
                }
                rm.exit = x;
            }
            if let Some(s) = o.get("out_s").and_then(num) {
                rm.exit_s = s;
            }
        }
        rm
    };
    let mut avoid: Vec<String> = p.map(|p| p.avoid.clone()).unwrap_or_default();
    for a in &cfg.avoid {
        if !avoid.contains(a) {
            avoid.push(a.clone());
        }
    }
    let mut m = Motion {
        profile: p.map_or("base".into(), |p| p.id.clone()),
        energy: energy.clone(),
        card: role("card"),
        caption: role("caption"),
        count: role("count"),
        title: role("title"),
        end: role("end"),
        dip: cfg.dip.unwrap_or(p.map_or(0.25, |p| p.dip) * scale),
        flash: p.map_or(0.12, |p| p.flash),
        accent: cfg.accent.clone().unwrap_or_else(|| p.map_or("flash".into(), |p| p.accent.clone())),
        avoid,
    };
    for rm in [&mut m.card, &mut m.caption, &mut m.count, &mut m.title, &mut m.end] {
        let e = substitute_entrance(&rm.enter, &m.avoid);
        if e != rm.enter {
            rm.enter_s = default_secs(&e) * energy_factor(&energy);
            rm.enter = e;
        }
        if m.avoid.contains(&rm.hold) {
            rm.hold = "still".into();
        }
        if m.avoid.contains(&rm.exit) {
            let fade_ok = !m.avoid.iter().any(|a| a == "fade");
            rm.exit = if fade_ok { "fade".into() } else { "cut".into() };
            rm.exit_s = if fade_ok { 0.4 } else { 0.0 };
        }
    }
    m.accent = m.transition(&m.accent.clone());
    m
}

/// An entrance with the avoided moves taken out: fade stands in, else a cut.
fn substitute_entrance(e: &str, avoid: &[String]) -> String {
    if !avoid.iter().any(|a| a == e) {
        return e.to_string();
    }
    if avoid.iter().any(|a| a == "fade") { "cut".into() } else { "fade".into() }
}

impl Motion {
    /// The role a script line (or `end` for the end card) plays.
    pub fn role(&self, kind: &str) -> &RoleMotion {
        match kind {
            "title" => &self.title,
            "caption" => &self.caption,
            "count" => &self.count,
            "end" => &self.end,
            _ => &self.card,
        }
    }

    /// How a line moves: its role, with the entrance the line asked for (`motion:`) unless
    /// the cut avoids it.
    pub fn for_line(&self, kind: &str, asked: Option<&str>) -> RoleMotion {
        let mut rm = self.role(kind).clone();
        if let Some(e) = asked.filter(|e| ENTRANCES.contains(e)) {
            let e = substitute_entrance(e, &self.avoid);
            if e != rm.enter {
                rm.enter_s = default_secs(&e) * energy_factor(&self.energy);
                rm.enter = e;
            }
        }
        rm
    }

    /// A hand-over with the avoided ones taken out (a flash or a glitch becomes a cut, a
    /// whiteout a dip).
    pub fn transition(&self, t: &str) -> String {
        if !self.avoid.iter().any(|a| a == t) {
            return t.to_string();
        }
        match t {
            "whiteout" if !self.avoid.iter().any(|a| a == "dip") => "dip".into(),
            _ => "cut".into(),
        }
    }
}

/// When a line's motion runs inside the time it shows [a, b]: the entrance takes at most 35%
/// of it and the exit 25%, so the hold that is read stays (references/craft/TEASER-BASELINE.md).
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub a: f64,
    pub b: f64,
    pub fin: f64,
    pub fout: f64,
}

pub fn timing(rm: &RoleMotion, a: f64, b: f64) -> Timing {
    let span = (b - a).max(0.02);
    let fin = if rm.enter == "cut" { 0.0 } else { rm.enter_s.min(0.35 * span).max(0.04) };
    let fout = if rm.exit == "cut" { 0.0 } else { rm.exit_s.min(0.25 * span).max(0.04) };
    Timing { a, b, fin, fout }
}

/// Opacity that rises over `fin` from `a` and falls over `fout` until `b` (local time `t`).
pub fn fade(a: f64, b: f64, fin: f64, fout: f64) -> String {
    let half = ((b - a) / 2.0).max(0.01);
    let (fin, fout) = (fin.clamp(0.01, half), fout.clamp(0.01, half));
    format!(
        "if(lt(t,{a:.3}),0,if(lt(t,{:.3}),(t-{a:.3})/{fin:.3},if(lt(t,{:.3}),1,if(lt(t,{b:.3}),({b:.3}-t)/{fout:.3},0))))",
        a + fin,
        b - fout
    )
}

/// Piecewise-linear opacity through (time, value) keys; 0 before the first, the last value after.
fn keyed(keys: &[(f64, f64)]) -> String {
    let last = keys.last().map_or(0.0, |k| k.1);
    let mut e = format!("{last:.3}");
    for w in keys.windows(2).rev() {
        let ((t0, v0), (t1, v1)) = (w[0], w[1]);
        let seg = format!("{v0:.3}+({v1:.3}-{v0:.3})*(t-{t0:.3})/{:.3}", (t1 - t0).max(0.001));
        e = format!("if(lt(t,{t1:.3}),{seg},{e})");
    }
    format!("if(lt(t,{:.3}),0,{e})", keys.first().map_or(0.0, |k| k.0))
}

/// The flicker in: two dips to dark over `fin` (at most three bright-dark pairs a second,
/// WCAG 2.3.1), the shape the title had before motion was settable.
fn flicker_in(a: f64, fin: f64) -> String {
    keyed(&[(a, 0.0), (a + 0.25 * fin, 0.85), (a + 0.4 * fin, 0.10), (a + 0.6 * fin, 1.0), (a + 0.75 * fin, 0.35), (a + fin, 1.0)])
}

/// How many bright-dark pairs an entrance flashes (for the photosensitivity check).
pub fn flashes(entrance: &str) -> usize {
    if entrance == "flicker" { 2 } else { 0 }
}

/// The opacity of a line moving by `rm` over `tm` (local time `t`).
pub fn alpha(rm: &RoleMotion, tm: &Timing) -> String {
    let Timing { a, b, fin, fout } = *tm;
    // the motion the cut had before: plain fades in and out keep their exact expression
    if rm.enter == "fade" && rm.exit == "fade" && rm.hold == "still" {
        return fade(a, b, fin, fout);
    }
    let p = |len: f64| format!("clip((t-{a:.3})/{:.3},0,1)", len.max(0.01));
    let enter = match rm.enter.as_str() {
        "fade" => p(fin),
        "focus" => format!("(1-pow(1-{},2))", p(fin)),
        "slam" => p(fin * 0.3),
        "pop" => p(fin * 0.4),
        "flicker" => flicker_in(a, fin),
        _ => format!("gte(t,{a:.3})"),
    };
    let exit = exit_alpha(rm, tm);
    let glow = if rm.hold == "glow" {
        format!("*if(between(t,{:.3},{:.3}),0.94+0.06*cos(2*PI*(t-{:.3})/2),1)", a + fin, b - fout, a + fin)
    } else {
        String::new()
    };
    format!("{enter}*{exit}{glow}")
}

/// The exit part of a line's opacity: a fade over `fout`, else gone at `b`.
pub fn exit_alpha(rm: &RoleMotion, tm: &Timing) -> String {
    match rm.exit.as_str() {
        "fade" => format!("clip(({:.3}-t)/{:.3},0,1)", tm.b, tm.fout.max(0.01)),
        _ => format!("lt(t,{:.3})", tm.b),
    }
}

/// The font size (pixels, already scaled) of a line moving by `rm`: a number, or an
/// expression of `t` for the entrances and holds that change it.
pub fn size(rm: &RoleMotion, tm: &Timing, px: f64) -> String {
    let Timing { a, b, fin, fout } = *tm;
    let p = format!("clip((t-{a:.3})/{:.3},0,1)", fin.max(0.01));
    let enter = match rm.enter.as_str() {
        // from 2.5 times its size, landing with a sharp ease-out
        "slam" => format!("(1+1.5*pow(1-{p},3))"),
        // up past its size and settling back
        "pop" => format!(
            "if(lt(t,{:.3}),0.85+0.23*clip((t-{a:.3})/{:.3},0,1),1.08-0.08*clip((t-{:.3})/{:.3},0,1))",
            a + 0.6 * fin, (0.6 * fin).max(0.01), a + 0.6 * fin, (0.4 * fin).max(0.01)
        ),
        "focus" => format!("(1+0.06*pow(1-{p},2))"),
        _ => String::new(),
    };
    let drift = if rm.hold == "drift" {
        format!("(1+0.03*clip((t-{:.3})/{:.3},0,1))", a + fin, (b - fout - a - fin).max(0.01))
    } else {
        String::new()
    };
    match (enter.is_empty(), drift.is_empty()) {
        (true, true) => format!("{px:.0}"),
        (false, true) => format!("{px:.1}*{enter}"),
        (true, false) => format!("{px:.1}*{drift}"),
        (false, false) => format!("{px:.1}*{enter}*{drift}"),
    }
}

/// Extra layers an entrance or exit draws over the line: for `focus` two soft copies that
/// close in on it (a defocused halo), for `glitch` red and cyan copies that jitter.
/// Each is (colour, size expression, alpha expression, x offset expression in pixels).
pub fn layers(rm: &RoleMotion, tm: &Timing, px: f64, s: f64) -> Vec<(String, String, String, String)> {
    let Timing { a, b, fin, fout } = *tm;
    let mut out = Vec::new();
    if rm.enter == "focus" {
        let p = format!("clip((t-{a:.3})/{:.3},0,1)", fin.max(0.01));
        for (grow, op) in [(0.12, 0.30), (0.24, 0.18)] {
            out.push((
                String::new(),
                format!("{px:.1}*(1+{grow}*(1-{p}))"),
                format!("{op}*(1-{p})*between(t,{a:.3},{:.3})", a + fin),
                "0".into(),
            ));
        }
    }
    let jitter = |sign: f64| format!("{:.1}*if(lt(mod(n,4),2),1,-1)", 8.0 * s * sign);
    let mut windows = Vec::new();
    if rm.enter == "glitch" {
        windows.push((a, a + fin.max(0.1)));
    }
    if rm.exit == "glitch" {
        windows.push(((b - fout.max(0.1)).max(a), b));
    }
    for (w0, w1) in windows {
        for (colour, sign) in [("0xFF3355", 1.0), ("0x33DDFF", -1.0)] {
            out.push((colour.to_string(), format!("{px:.0}"), format!("0.6*between(t,{w0:.3},{w1:.3})"), jitter(sign)));
        }
    }
    out
}

/// Type-on: the times at which a line of `n` characters shows its first k characters. The
/// pace is about 20 characters a second (measured), sped up to fit the entrance's share.
pub fn type_steps(n: usize, tm: &Timing) -> Vec<(usize, f64)> {
    if n == 0 {
        return Vec::new();
    }
    let chunk = n.div_ceil(24);
    let steps = n.div_ceil(chunk);
    let span = (n as f64 * 0.05).min(0.35 * (tm.b - tm.a)).max(0.1);
    (1..=steps).map(|k| ((k * chunk).min(n), tm.a + span * (k - 1) as f64 / steps as f64)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(v: Value) -> MotionCfg {
        let mut errs = Vec::new();
        let c = parse(Some(&v), &mut errs);
        assert!(errs.is_empty(), "{errs:?}");
        c
    }

    #[test]
    fn the_base_profile_is_the_motion_the_cut_had() {
        let m = Motion::default();
        assert_eq!(m.profile, "base");
        assert_eq!((m.card.enter.as_str(), m.card.enter_s, m.card.exit_s), ("fade", 0.45, 0.5));
        assert_eq!(m.title.enter, "flicker");
        assert_eq!((m.dip, m.flash, m.accent.as_str()), (0.25, 0.12, "flash"));
        // the same fade expression as before
        let tm = timing(&m.card, 0.15, 2.55);
        assert_eq!(alpha(&m.card, &tm), fade(0.15, 2.55, 0.45, 0.5));
    }

    #[test]
    fn energy_scales_the_profile_and_overrides_win() {
        let calm = resolve(&cfg(json!({"profile": "rpg", "energy": "calm"})));
        assert!((calm.card.enter_s - 0.75 * 1.4).abs() < 1e-9 && (calm.dip - 0.42 * 1.4).abs() < 1e-9);
        let m = resolve(&cfg(json!({"profile": "horror", "card": {"in": "type"}, "title": {"in": "fade", "in_s": 1.2}})));
        assert_eq!(m.card.enter, "type");
        assert_eq!((m.title.enter.as_str(), m.title.enter_s), ("fade", 1.2));
        assert_eq!(m.card.exit, "cut"); // the profile's, untouched
    }

    #[test]
    fn avoided_moves_are_taken_out() {
        let m = resolve(&cfg(json!({"profile": "action-ja", "avoid": ["slam", "flash"]})));
        assert_eq!(m.card.enter, "fade");
        assert_eq!(m.transition("flash"), "cut");
        assert_eq!(m.accent, "cut");
        assert_eq!(m.for_line("card", Some("slam")).enter, "fade");
        // cozy rules out flashes on its own
        let cozy = resolve(&cfg(json!("cozy")));
        assert_eq!(cozy.transition("flash"), "cut");
        assert_eq!(cozy.transition("whiteout"), "whiteout");
        let no_dip = resolve(&cfg(json!({"profile": "rpg", "avoid": ["whiteout", "dip"]})));
        assert_eq!(no_dip.transition("whiteout"), "cut");
    }

    #[test]
    fn bad_values_are_named() {
        let mut errs = Vec::new();
        parse(Some(&json!({"profile": "western", "energy": "loud", "card": {"in": "spin"}, "avoid": ["spin"]})), &mut errs);
        assert_eq!(errs.len(), 4, "{errs:?}");
        assert!(errs.iter().any(|e| e.contains("finish.motion.profile")), "{errs:?}");
    }

    #[test]
    fn a_line_asks_for_its_own_entrance() {
        let m = resolve(&cfg(json!("horror")));
        let rm = m.for_line("card", Some("slam"));
        assert_eq!(rm.enter, "slam");
        assert!((rm.enter_s - 0.15 * 1.4).abs() < 1e-9); // calm: a little slower
        let tm = timing(&rm, 0.0, 3.0);
        assert!(size(&rm, &tm, 64.0).contains("pow(1-clip"));
        // entrances stay inside the line: at most 35% of it
        let slow = RoleMotion { enter: "fade".into(), enter_s: 3.0, hold: "still".into(), exit: "cut".into(), exit_s: 0.0 };
        assert!((timing(&slow, 0.0, 2.0).fin - 0.7).abs() < 1e-9);
        assert_eq!(type_steps(10, &timing(&rm, 1.0, 4.0)).len(), 10);
        assert_eq!(type_steps(60, &timing(&rm, 1.0, 9.0)).len(), 20); // in chunks
    }

    /// Guards D-53: whatever the vocabulary word, profile, energy or (huge) override, the
    /// entrance takes at most 35% of the line's time and the exit at most 25%.
    #[test]
    fn no_entrance_or_exit_outruns_its_share_of_the_line() {
        assert!(!profiles().is_empty());
        let spans = [0.8, 1.5, 4.0];
        let check = |m: &Motion, what: &str| {
            for (role, rm) in [("card", &m.card), ("caption", &m.caption), ("count", &m.count), ("title", &m.title), ("end", &m.end)] {
                for span in spans {
                    let tm = timing(rm, 2.0, 2.0 + span);
                    assert!(tm.fin <= 0.35 * span + 1e-9, "{what} {role} in {} span {span}: {}", rm.enter, tm.fin);
                    assert!(tm.fout <= 0.25 * span + 1e-9, "{what} {role} out {} span {span}: {}", rm.exit, tm.fout);
                }
            }
        };
        for p in profiles() {
            for &e in ENERGIES {
                let m = resolve(&cfg(json!({"profile": p.id, "energy": e})));
                check(&m, &format!("{}/{e}", p.id));
                for &role in ROLES {
                    for &ent in ENTRANCES {
                        for (&hold, &ex) in HOLDS.iter().zip(EXITS.iter().cycle()) {
                            let c = cfg(json!({"profile": p.id, role: {"in": ent, "in_s": 3.0, "hold": hold, "out": ex, "out_s": 3.0}}));
                            check(&resolve(&c), &format!("{}/{role}/{ent}/{hold}/{ex}", p.id));
                        }
                    }
                    for &ex in EXITS {
                        let c = cfg(json!({"profile": p.id, role: {"out": ex, "out_s": 3.0}}));
                        check(&resolve(&c), &format!("{}/{role}/out {ex}", p.id));
                    }
                }
            }
        }
    }

    /// Guards D-53: profiles only use words of the closed vocabulary.
    #[test]
    fn every_profile_speaks_the_closed_vocabulary() {
        for p in profiles() {
            let m = resolve(&cfg(json!({"profile": p.id})));
            for rm in [&m.card, &m.caption, &m.count, &m.title, &m.end] {
                assert!(ENTRANCES.contains(&rm.enter.as_str()), "{}: entrance {}", p.id, rm.enter);
                assert!(HOLDS.contains(&rm.hold.as_str()), "{}: hold {}", p.id, rm.hold);
                assert!(EXITS.contains(&rm.exit.as_str()), "{}: exit {}", p.id, rm.exit);
            }
        }
    }

    /// Guards D-25 / D-53: no transition may overlap two cuts (that would shorten the cut).
    #[test]
    fn the_transition_vocabulary_has_no_crossfade() {
        for t in crate::finish::TRANSITIONS {
            for bad in ["dissolve", "crossfade", "cross", "mix", "xfade", "blend"] {
                assert!(!t.contains(bad), "transition '{t}' looks like a crossfade");
            }
        }
    }
}
