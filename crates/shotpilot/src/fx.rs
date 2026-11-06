//! Camera effects (D-57): what a shot's picture goes through at the cut, chosen from a closed
//! list. A shot names them (`fx: [shake, {type: glitch, at: 1.2}]`); the code decides how long
//! a burst lasts and how strong it is, as with motion (D-53).
//!
//! Like the rest of the finish (D-25) they are part of the definition and are burned in by
//! ffmpeg when the cut is assembled: no fingerprint holds them, so adding or removing one never
//! re-renders a shot. None changes a shot's length. Effects that flash (`flicker`, `static`,
//! `redflash`) count toward the cut's flash limit (`flash.rate`, three a second), and the
//! moves a cut avoids (`finish.motion.avoid`) take the matching effects out.

use serde_json::Value;

use crate::model::{fmt_g, num, str_of};

/// The effects, with what each is for (the planner reads these).
pub const FX: &[(&str, &str)] = &[
    ("chroma", "the colour channels a little apart for the whole shot: a lens that is not right"),
    ("glitch", "one burst of torn, colour-split picture: a signal breaking, something that should not be there"),
    ("flicker", "the picture dips dark twice, like a failing light"),
    ("static", "a burst of TV static that the shot comes out of: a feed cutting in"),
    ("vhs", "the whole shot as worn tape: soft, scanlines, bled colour, grain"),
    ("shake", "a handheld wobble over the whole shot: someone is holding the camera"),
    ("redflash", "the picture floods red for a moment: a jolt"),
];

/// Effects that last the whole shot; the others are bursts at `at`.
const WHOLE: &[&str] = &["chroma", "vhs", "shake"];
/// At most this many effects on one shot.
pub const MAX_PER_SHOT: usize = 3;

/// Seconds a burst lasts.
pub fn burst(kind: &str) -> f64 {
    match kind {
        "glitch" => 0.18,
        "flicker" => 0.22,
        "static" => 0.35,
        "redflash" => 0.12,
        _ => 0.0,
    }
}

/// Where a burst falls when the shot does not say (as a share of the shot).
fn default_at(kind: &str, d: f64) -> f64 {
    let share = match kind {
        "glitch" => 0.55,
        "flicker" => 0.3,
        "redflash" => 0.5,
        _ => 0.0, // static opens the shot
    };
    share * d
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fx {
    pub kind: String,
    /// Seconds into the shot (0 for the whole-shot effects).
    pub at: f64,
}

impl Fx {
    pub fn whole(&self) -> bool {
        WHOLE.contains(&self.kind.as_str())
    }

    /// The burst's span in the shot (the whole shot for the whole-shot effects).
    pub fn span(&self, d: f64) -> (f64, f64) {
        if self.whole() {
            (0.0, d)
        } else {
            (self.at, (self.at + burst(&self.kind)).min(d))
        }
    }
}

fn items(shot: &Value) -> Vec<Value> {
    match shot.get("fx") {
        Some(Value::Array(a)) => a.clone(),
        Some(Value::String(s)) if !s.is_empty() => vec![Value::String(s.clone())],
        _ => Vec::new(),
    }
}

/// A shot's effects, with each burst placed (its own `at`, else the default for its kind,
/// kept inside the shot). Unknown names are left out (validation reports them).
pub fn of(shot: &Value) -> Vec<Fx> {
    let d = shot.get("duration").and_then(num).unwrap_or(3.0);
    let mut out: Vec<Fx> = Vec::new();
    for it in items(shot) {
        let (kind, at) = match &it {
            Value::Object(m) => (m.get("type").map(str_of).unwrap_or_default(), m.get("at").and_then(num)),
            other => (str_of(other), None),
        };
        if !FX.iter().any(|(k, _)| *k == kind) || out.iter().any(|f| f.kind == kind) {
            continue;
        }
        let at = if WHOLE.contains(&kind.as_str()) {
            0.0
        } else {
            at.unwrap_or_else(|| default_at(&kind, d)).clamp(0.0, (d - burst(&kind)).max(0.0))
        };
        out.push(Fx { kind, at });
    }
    out.truncate(MAX_PER_SHOT);
    out
}

/// Per-shot checks (called from model validation).
pub fn validate(at: &str, shot: &Value, errs: &mut Vec<String>) {
    let Some(v) = shot.get("fx").filter(|v| !v.is_null()) else { return };
    if !matches!(v, Value::Array(_) | Value::String(_)) {
        errs.push(format!("{at}.fx: a list of effects ({})", names().join(", ")));
        return;
    }
    let d = shot.get("duration").and_then(num).unwrap_or(0.0);
    let list = items(shot);
    if list.len() > MAX_PER_SHOT {
        errs.push(format!("{at}.fx: at most {MAX_PER_SHOT} effects on one shot"));
    }
    let mut seen: Vec<String> = Vec::new();
    for (i, it) in list.iter().enumerate() {
        let (kind, when) = match it {
            Value::Object(m) => (m.get("type").map(str_of).unwrap_or_default(), m.get("at")),
            other => (str_of(other), None),
        };
        if !FX.iter().any(|(k, _)| *k == kind) {
            errs.push(format!("{at}.fx[{i}]: one of {} (got '{kind}')", names().join(", ")));
            continue;
        }
        if seen.contains(&kind) {
            errs.push(format!("{at}.fx[{i}]: '{kind}' twice on one shot"));
        }
        seen.push(kind.clone());
        if let Some(w) = when {
            match num(w) {
                Some(t) if (0.0..d.max(0.0)).contains(&t) => {}
                _ => errs.push(format!("{at}.fx[{i}].at: seconds into the shot (0..{}s)", fmt_g(d))),
            }
        }
    }
}

pub fn names() -> Vec<&'static str> {
    FX.iter().map(|(k, _)| *k).collect()
}

/// Whether the moves a cut avoids take this effect out: its own name; a red flash or a burst
/// of static when flashes are avoided; static when glitches are. A flicker only dims, so only
/// its own name takes it out.
pub fn avoided(kind: &str, avoid: &[String]) -> bool {
    let has = |n: &str| avoid.iter().any(|a| a == n);
    has(kind) || (matches!(kind, "static" | "redflash") && has("flash")) || (kind == "static" && has("glitch"))
}

/// Which avoided move takes an effect out, for the note that says so.
pub fn avoided_by<'a>(kind: &str, avoid: &'a [String]) -> Option<&'a str> {
    avoid.iter().map(String::as_str).find(|a| avoided(kind, std::slice::from_ref(&a.to_string())))
}

/// The flashes an effect puts into the cut, as seconds into the shot (for `flash.rate`): two
/// dips for a flicker, one for a static burst or a red flash.
pub fn flash_times(f: &Fx) -> Vec<f64> {
    match f.kind.as_str() {
        "flicker" => vec![f.at, f.at + 0.15],
        "static" | "redflash" => vec![f.at],
        _ => Vec::new(),
    }
}

fn between(a: f64, b: f64) -> String {
    format!("between(t,{a:.3},{b:.3})")
}

/// The ffmpeg filters for a shot's effects, for a frame `h` lines high (the shot's own clock:
/// `t` is 0 at its first frame). They change frames, never add or drop them.
pub fn filters(fx: &[Fx], d: f64, w: i64, h: i64) -> Vec<String> {
    let s = h as f64 / 1080.0;
    let px = |v: f64| (v * s).round().max(1.0) as i64;
    let mut out = Vec::new();
    for f in fx {
        let (a, b) = f.span(d);
        let on = between(a, b);
        match f.kind.as_str() {
            "chroma" => out.push(format!("rgbashift=rh=-{}:bh={}", px(3.0), px(3.0))),
            "vhs" => out.push(format!(
                "gblur=sigma={:.2},rgbashift=rh=-{p}:bh={p},eq=saturation=0.72:contrast=1.05,noise=alls=12:allf=t,\
                 drawgrid=w=iw:h={g}:t=1:c=black@0.25",
                0.7 * s.max(0.5),
                p = px(4.0),
                g = px(4.0).max(2)
            )),
            "shake" => out.push(format!(
                "crop=w=iw*0.94:h=ih*0.94:x='(iw-ow)/2*(1+0.6*sin(t*5.3)*sin(t*1.7+0.4))':\
                 y='(ih-oh)/2*(1+0.6*sin(t*4.1+1.0)*sin(t*2.3))',scale={w}:{h}"
            )),
            "glitch" => {
                let (dx, dc) = (px(80.0), px(40.0));
                // bands of rows slid sideways (different bands each frame), colour split, grain
                let tear = |amp: i64, rows: &str| {
                    format!("p(X+if(lt(mod({rows}+N*37\\,H/6)\\,H/24)\\,{amp}*sin(N*1.7+Y*0.05)\\,0)\\,Y)")
                };
                out.push(format!(
                    "rgbashift=rh=-{p}:bh={p}:rv={q}:enable='{on}',geq=lum='{l}':cb='{c}':cr='{c}':enable='{on}',\
                     noise=alls=35:allf=t:enable='{on}'",
                    p = px(14.0),
                    q = px(4.0),
                    l = tear(dx, "Y"),
                    c = tear(dc, "Y*2")
                ));
            }
            "flicker" => {
                let dips = format!("{}+{}", between(a, a + 0.07), between(a + 0.15, a + 0.22));
                out.push(format!("eq=brightness=-0.32:contrast=0.85:enable='{dips}'"));
            }
            "static" => {
                // full static, then the picture coming back through the noise
                let full = between(a, a + 0.22);
                out.push(format!(
                    "geq=lum='255*random(1)*(0.75+0.25*sin(Y*0.6+N*3))':cb=128:cr=128:enable='{full}',\
                     gblur=sigma={:.1}:sigmaV=0.1:enable='{full}',noise=alls=60:allf=t:enable='{}'",
                    1.5 * s.max(0.5),
                    between(a + 0.22, b)
                ));
            }
            "redflash" => out.push(format!(
                "colorchannelmixer=rr=1.4:rg=0.6:rb=0.4:gr=0:gg=0.12:gb=0:br=0:bg=0:bb=0.12:enable='{on}',\
                 eq=brightness=0.05:enable='{on}'"
            )),
            _ => {}
        }
    }
    out
}

/// What the planner is told about the effects.
pub fn guide() -> String {
    let mut out = String::from(
        "- `fx`: camera effects on a shot, burned in at the cut (no re-render). Most shots have none. Read them off \
         the pitch, the plan and what the scout photos show: a failing lamp in the room is a `flicker`; a story told \
         through recordings or cameras is `vhs` or `static`; a person holding the camera, a chase or a panic is \
         `shake`; one wrong moment is a `glitch` or a `redflash`. One burst effect per beat at most, and none on the \
         title shot. The moves the cut avoids (`finish.motion.avoid`) are left out at the cut: an effect named there, \
         `redflash` and `static` when `flash` is, `static` when `glitch` is. Effects:\n",
    );
    for (k, what) in FX {
        out.push_str(&format!("  - `{k}`: {what}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn effects_are_placed_inside_the_shot_and_unknown_ones_dropped() {
        let shot = json!({"duration": 2.0, "fx": ["shake", {"type": "glitch", "at": 1.95}, "bogus", "shake", "flicker"]});
        let fx = of(&shot);
        assert_eq!(fx.iter().map(|f| f.kind.as_str()).collect::<Vec<_>>(), ["shake", "glitch", "flicker"]);
        assert_eq!(fx[0].at, 0.0);
        assert!((fx[1].at - (2.0 - 0.18)).abs() < 1e-9, "a late burst is pulled back inside the shot");
        assert!((fx[2].at - 0.6).abs() < 1e-9, "flicker defaults to 30% in");
        assert_eq!(fx[1].span(2.0), (fx[1].at, 2.0));
    }

    #[test]
    fn validation_names_what_is_wrong() {
        let mut errs = Vec::new();
        validate("shots[0]", &json!({"duration": 2.0, "fx": ["chroma", "nope", {"type": "glitch", "at": 5}]}), &mut errs);
        assert!(errs.iter().any(|e| e.contains("fx[1]") && e.contains("nope")), "{errs:?}");
        assert!(errs.iter().any(|e| e.contains("fx[2].at")), "{errs:?}");
        let mut ok = Vec::new();
        validate("shots[0]", &json!({"duration": 2.0, "fx": "vhs"}), &mut ok);
        assert!(ok.is_empty(), "{ok:?}");
    }

    #[test]
    fn every_effect_has_a_filter_and_bursts_are_timed() {
        for k in names() {
            let f = of(&json!({"duration": 3.0, "fx": [k]}));
            let chain = filters(&f, 3.0, 1920, 1080);
            assert_eq!(chain.len(), 1, "{k}");
            if !WHOLE.contains(&k) {
                assert!(chain[0].contains("enable='between(t,"), "{k}: {}", chain[0]);
            }
        }
    }

    #[test]
    fn avoiding_flashes_takes_out_the_flashing_effects() {
        let avoid = vec!["flash".to_string()];
        assert!(avoided("static", &avoid) && avoided("redflash", &avoid));
        assert!(!avoided("flicker", &avoid), "a flicker dims; only its own name takes it out");
        assert!(!avoided("glitch", &avoid) && !avoided("shake", &avoid));
        assert_eq!(avoided_by("static", &["slam".to_string(), "glitch".to_string()]), Some("glitch"));
        assert!(avoided("shake", &["shake".to_string()]));
        assert_eq!(flash_times(&Fx { kind: "flicker".into(), at: 1.0 }).len(), 2);
    }
}
