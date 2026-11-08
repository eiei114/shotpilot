//! The three game's jointed mannequin, posed at a time, for previz.
//!
//! Kept in step with `figure()` and `MOVES` in three/render.html: the same joints, lengths and
//! angles, built in three's frame (metres, Y up, facing -Z). Previz draws fewer segments.

use std::collections::HashMap;
use std::f64::consts::PI;

use super::mesh::{Builder, Geo, Order, Xf};

/// (radius, height) of the turned torso, as the page has it.
const TORSO: [(f64, f64); 11] = [
    (0.0, -0.06),
    (0.12, -0.05),
    (0.14, 0.0),
    (0.13, 0.08),
    (0.118, 0.17),
    (0.134, 0.26),
    (0.158, 0.35),
    (0.168, 0.41),
    (0.148, 0.46),
    (0.085, 0.5),
    (0.0, 0.52),
];

/// One joint's angles (radians) and, when set, its height in its parent.
#[derive(Debug, Clone, Copy, Default)]
struct Ang {
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    h: Option<f64>,
}

fn a(x: f64) -> Ang {
    Ang { x: Some(x), ..Ang::default() }
}

fn xz(x: f64, z: f64) -> Ang {
    Ang { x: Some(x), z: Some(z), ..Ang::default() }
}

fn zz(z: f64) -> Ang {
    Ang { z: Some(z), ..Ang::default() }
}

fn yy(y: f64) -> Ang {
    Ang { y: Some(y), ..Ang::default() }
}

fn mod_(a: f64, b: f64) -> f64 {
    ((a % b) + b) % b
}

fn spike(t: f64, period: f64, at: f64) -> f64 {
    (1.0 - (mod_(t - at, period) - 0.08).abs() / 0.07).max(0.0)
}

/// The page's `MOVES[mv](t, k)`.
fn pose(mv: &str, t: f64, k: f64) -> HashMap<&'static str, Ang> {
    let sin = f64::sin;
    let cos = f64::cos;
    let mut q: HashMap<&'static str, Ang> = HashMap::new();
    let mut set = |n: &'static str, v: Ang| {
        q.insert(n, v);
    };
    match mv {
        "stand" => {
            set("chest", a(0.015 * sin(t * 1.3)));
            set("head", xz(-0.18, 0.22));
            set("uaL", zz(-0.05));
            set("uaR", zz(0.05));
            set("faL", a(0.08));
            set("faR", a(0.1));
        }
        "sway" => {
            let s = sin(t * 1.1);
            set("hips", zz(0.06 * s));
            set("spine", xz(-0.12, -0.04 * s));
            set("head", xz(-0.55, 0.15 * s));
            set("uaL", zz(-0.04 + 0.05 * s));
            set("uaR", zz(0.04 + 0.05 * s));
        }
        "cower" => {
            let tr = 0.02 * sin(t * 23.0);
            set("hips", Ang { h: Some(0.36 * k), ..Ang::default() });
            set("thL", xz(1.6, -0.15));
            set("thR", xz(1.6, 0.15));
            set("shL", a(-2.3));
            set("shR", a(-2.3));
            set("spine", a(-0.7));
            set("chest", zz(tr));
            set("head", a(-0.45));
            set("uaL", xz(1.1, 0.25));
            set("faL", a(1.7));
            set("uaR", xz(1.1, -0.25));
            set("faR", a(1.7));
        }
        "peek" => {
            set("spine", zz(0.3 + 0.05 * sin(t * 0.5)));
            set("head", Ang { z: Some(0.2), y: Some(0.18 * sin(t * 0.6)), ..Ang::default() });
            set("uaR", xz(0.9, 0.3));
            set("faR", a(0.6));
            set("uaL", zz(-0.05));
        }
        "twitch" => {
            let (p, b, c) = (spike(t, 1.9, 0.3), spike(t, 2.7, 1.1), spike(t, 3.1, 2.0));
            set("head", xz(-0.25 - 0.3 * b, 0.55 * p - 0.35 * c));
            set("spine", zz(0.1 * p));
            set("uaR", xz(0.2 * c, 0.1 + 0.45 * b));
            set("uaL", zz(-0.08 - 0.3 * c));
        }
        "head_turn" => {
            let u = 0.5 - 0.5 * cos(t * 0.5);
            set("head", yy(1.1 * u));
            set("neck", yy(0.2 * u));
            set("chest", yy(0.15 * u));
        }
        "reach" => {
            set("body", Ang { x: Some(PI / 2.0), h: Some(0.1 * k), ..Ang::default() });
            set("uaR", a(2.6 + 0.25 * sin(t * 0.7)));
            set("faR", a(0.3 + 0.2 * sin(t * 1.1)));
            set("uaL", zz(-0.1));
            set("head", yy(0.3 * sin(t * 0.3)));
        }
        "lie_back" => {
            set("body", Ang { x: Some(PI / 2.0), h: Some(0.1 * k), ..Ang::default() });
            set("uaL", zz(-0.2));
            set("uaR", zz(0.2));
            set("head", yy(0.5));
        }
        "face_down" => {
            set("body", Ang { x: Some(-PI / 2.0), h: Some(0.1 * k), ..Ang::default() });
            set("uaL", xz(2.9, -0.3));
            set("uaR", xz(2.7, 0.25));
            set("faR", a(0.3));
            set("head", yy(0.7));
            set("thL", zz(-0.1));
            set("thR", zz(0.12));
        }
        "crawl" => {
            let c = sin(t * 1.4);
            set("body", Ang { x: Some(-PI / 2.0 + 0.25), h: Some(0.16 * k), ..Ang::default() });
            set("uaL", a(2.5 + 0.4 * c));
            set("uaR", a(2.5 - 0.4 * c));
            set("faL", a(0.4 - 0.3 * c));
            set("faR", a(0.4 + 0.3 * c));
            set("thL", a(0.3 * c));
            set("shL", a(-0.6));
            set("thR", a(-0.3 * c));
            set("shR", a(-0.6));
            set("head", a(0.4));
        }
        "nurse" => {
            set("head", a(-0.35));
            set("uaL", xz(0.5, -0.1));
            set("faL", a(1.5));
            set("uaR", xz(0.55, 0.05 * sin(t * 3.5)));
            set("faR", a(1.45 + 0.08 * sin(t * 7.0)));
        }
        "shamble" | "stalk" => {
            let stalk = mv == "stalk";
            let ph = t * 2.0 * PI / if stalk { 2.4 } else { 1.8 };
            let s = sin(ph);
            let (swing, knee, base, bob) = if stalk { (0.28, 0.45, -0.1, 0.015) } else { (0.35, 0.5, -0.15, 0.02) };
            set("thL", a(swing * s));
            set("thR", a(-swing * s));
            set("shL", a(base - knee * (-cos(ph)).max(0.0)));
            set("shR", a(base - knee * cos(ph).max(0.0)));
            set("hips", Ang { h: Some(0.95 * k - bob * k * cos(ph).abs()), y: Some(if stalk { 0.05 } else { 0.08 } * s), ..Ang::default() });
            if stalk {
                set("spine", a(-0.08));
                set("chest", zz(0.04 * s));
                set("head", xz(0.12, 0.42 + 0.05 * sin(t * 0.7)));
                set("uaL", xz(0.08 - 0.06 * s, -0.04));
                set("uaR", xz(0.08 + 0.06 * s, 0.04));
                set("faL", a(0.15));
                set("faR", a(0.2));
            } else {
                set("spine", a(-0.25));
                set("head", xz(-0.3, 0.2));
                set("uaL", a(0.25 - 0.15 * s));
                set("uaR", a(0.25 + 0.15 * s));
            }
        }
        // the village's (D-101)
        "walk" | "run" => {
            let run = mv == "run";
            let ph = t * 2.0 * PI / if run { 0.62 } else { 1.1 };
            let s = sin(ph);
            let (swing, knee, base, arm) = if run { (0.75, 1.1, -0.3, 0.7) } else { (0.42, 0.55, -0.1, 0.35) };
            set("thL", a(swing * s));
            set("thR", a(-swing * s));
            set("shL", a(base - knee * (-cos(ph)).max(0.0)));
            set("shR", a(base - knee * cos(ph).max(0.0)));
            if run {
                set("hips", Ang { h: Some(0.93 * k + 0.03 * k * sin(ph).abs()), y: Some(0.08 * s), ..Ang::default() });
                set("spine", a(-0.15));
                set("chest", yy(-0.12 * s));
                set("head", a(0.12));
                set("uaL", xz(-0.7 * s, -0.12));
                set("uaR", xz(0.7 * s, 0.12));
                set("faL", a(1.2));
                set("faR", a(1.2));
            } else {
                set("hips", Ang { h: Some(0.95 * k - 0.02 * k * cos(ph).abs()), y: Some(0.06 * s), ..Ang::default() });
                set("chest", yy(-0.08 * s));
                set("head", a(-0.04));
                set("uaL", xz(-arm * s, -0.06));
                set("uaR", xz(arm * s, 0.06));
                set("faL", a(0.25));
                set("faR", a(0.25));
            }
        }
        "water" => {
            set("spine", a(-0.15));
            set("head", a(0.25));
            set("uaR", xz(0.95 + 0.06 * sin(t * 1.3), 0.12));
            set("faR", a(0.3));
            set("uaL", zz(-0.14));
            set("faL", a(0.2));
        }
        "pour" => {
            let u = 0.5 - 0.5 * cos(t * 1.1);
            set("spine", a(-0.12));
            set("head", a(0.3));
            set("uaR", xz(0.75 + 0.25 * u, 0.18));
            set("faR", xz(0.9 - 0.3 * u, 0.25 * u));
            set("uaL", xz(0.5, -0.1));
            set("faL", a(1.3));
        }
        "sweep" => {
            let s = sin(t * 2.4);
            set("spine", Ang { x: Some(-0.3), y: Some(0.25 * s), ..Ang::default() });
            set("chest", yy(0.15 * s));
            set("head", a(0.35));
            set("uaL", xz(0.6 + 0.2 * s, 0.2));
            set("faL", a(0.6));
            set("uaR", xz(0.5 + 0.2 * s, -0.2));
            set("faR", a(0.9));
        }
        "wave" => {
            set("chest", a(0.02 * sin(t * 1.7)));
            set("head", zz(0.1 * sin(t * 0.9)));
            set("uaR", xz(0.3, 2.5));
            set("faR", zz(0.45 * sin(t * 6.0)));
            set("uaL", zz(-0.08));
        }
        "sit" => {
            set("hips", Ang { h: Some(0.6 * k), ..Ang::default() });
            set("thL", xz(1.5, -0.05));
            set("thR", xz(1.5, 0.05));
            set("shL", a(-1.45));
            set("shR", a(-1.45));
            set("spine", a(0.05));
            set("chest", a(0.02 * sin(t * 1.5)));
            set("head", Ang { x: Some(0.05), y: Some(0.25 * sin(t * 0.35)), ..Ang::default() });
            set("uaL", xz(0.1, -0.06));
            set("faL", a(1.0));
            set("uaR", xz(0.1, 0.06));
            set("faR", a(1.0));
        }
        _ => {
            // idle
            set("chest", a(0.02 * sin(t * 1.7)));
            set("head", yy(0.1 * sin(t * 0.4)));
            set("uaL", zz(-0.08));
            set("uaR", zz(0.08));
        }
    }
    q
}

/// The page's `rng(seed)()`: the first draw of mulberry32 seeded from the string.
pub(crate) fn first_draw(seed: &str) -> f64 {
    let mut s: u32 = 0;
    for ch in seed.encode_utf16() {
        s = s.wrapping_mul(31).wrapping_add(ch as u32);
    }
    s = s.wrapping_add(0x6d2b_79f5);
    let mut x = s;
    x = (x ^ (x >> 15)).wrapping_mul(x | 1);
    x ^= x.wrapping_add((x ^ (x >> 7)).wrapping_mul(x | 61));
    (x ^ (x >> 14)) as f64 / 4_294_967_296.0
}

/// What the figure part asks for.
pub(crate) struct Spec<'a> {
    pub mv: &'a str,
    pub height: f64,
    pub thin: bool,
    /// The seed of its phase (the part's `seed`, else the object's name).
    pub seed: &'a str,
    pub skin: u32,
    /// Its clothes: their surface and how long the skirt or tunic is (in the figure's height).
    pub cloth: Option<(u32, f64)>,
    /// What it carries (the page's `holds`) and that thing's surface.
    pub holds: Option<(&'a str, u32)>,
}

/// Adds the mannequin at time `t`. `root` places its feet: three's frame (the page's figure
/// group), already carried into UE's.
pub(crate) fn add(b: &mut Builder, root: &Xf, spec: &Spec, t: f64, piece: u32) {
    let k = spec.height / 175.0;
    let thin = spec.thin;
    let tk = if thin { 0.68 } else { 1.0 };
    let al = if thin { 1.16 } else { 1.0 };
    let q = pose(spec.mv, t + first_draw(spec.seed) * 10.0, k);
    // a joint: its rest position (height overridden by the move) and the move's angles
    let joint = |parent: &Xf, name: &str, rest: [f64; 3]| -> Xf {
        let a = q.get(name).copied().unwrap_or_default();
        let pos = [rest[0], a.h.unwrap_or(rest[1]), rest[2]];
        parent.then(&Xf::three(pos, [a.x.unwrap_or(0.0), a.y.unwrap_or(0.0), a.z.unwrap_or(0.0)], Order::Xyz, [1.0; 3]))
    };
    let put = |b: &mut Builder, geo: &Geo, at: &Xf, surf: u32| b.add(geo, at, surf, piece);
    let skin = spec.skin;
    // a limb: a tapered cylinder hanging from the joint, and a ball at the joint
    let seg = |b: &mut Builder, put: &dyn Fn(&mut Builder, &Geo, &Xf, u32), j: &Xf, len: f64, r0: f64, r1: f64| {
        put(b, &Geo::cylinder(r0 * tk * k, r1 * tk * k, len * k, 10, false), &j.then(&Xf::translate([0.0, -len * k / 2.0, 0.0])), skin);
        put(b, &Geo::sphere(r0 * tk * k * 1.06, 8, 5), j, skin);
    };
    let ball = |r: f64, s: [f64; 3]| (Geo::sphere(r, 12, 7), Xf::scale(s));

    let body = joint(root, "body", [0.0; 3]);
    let hips = joint(&body, "hips", [0.0, 0.95 * k, 0.0]);
    let (g, s) = ball(0.15 * k, [1.05 * if thin { 0.8 } else { 1.0 }, 0.78, 0.74]);
    put(b, &g, &hips.then(&s), skin);
    let spine = joint(&hips, "spine", [0.0, 0.04 * k, 0.0]);
    let torso: Vec<(f64, f64)> = TORSO.iter().map(|(r, y)| (r * k, y * k)).collect();
    put(b, &Geo::lathe(&torso, 14), &spine.then(&Xf::scale([if thin { 0.8 } else { 1.0 }, 1.0, if thin { 0.56 } else { 0.66 }])), skin);
    if let Some((cloth, len)) = spec.cloth {
        let at = spine.then(&Xf::translate([0.0, 0.47 * k - len * k / 2.0, 0.0])).then(&Xf::scale([1.0, 1.0, 0.76]));
        put(b, &Geo::cylinder(0.168 * k, 0.26 * k, len * k, 16, false), &at, cloth);
    }
    let chest = joint(&spine, "chest", [0.0, 0.44 * k, 0.0]);
    let neck = joint(&chest, "neck", [0.0, 0.02 * k, 0.0]);
    put(b, &Geo::cylinder(0.036 * k * tk, 0.046 * k * tk, 0.12 * k, 8, false), &neck.then(&Xf::translate([0.0, 0.05 * k, 0.0])), skin);
    let head = joint(&neck, "head", [0.0, 0.1 * k, 0.0]);
    let (g, s) = ball(0.104 * k, [0.86, 1.15, 0.98]);
    put(b, &g, &head.then(&Xf::translate([0.0, 0.1 * k, 0.0])).then(&s), skin);
    let (g, s) = ball(0.068 * k, [0.92, 0.85, 1.0]);
    put(b, &g, &head.then(&Xf::translate([0.0, 0.04 * k, -0.035 * k])).then(&s), skin);

    // the figure's left is -X when it faces -Z
    for (side, ua_n, fa_n, th_n, sh_n) in [(-1.0, "uaL", "faL", "thL", "shL"), (1.0, "uaR", "faR", "thR", "shR")] {
        let ua = joint(&chest, ua_n, [side * if thin { 0.16 } else { 0.185 } * k, -0.03 * k, 0.0]);
        seg(b, &put, &ua, 0.29 * al, 0.05, 0.04);
        if let Some((cloth, _)) = spec.cloth {
            put(b, &Geo::cylinder(0.062 * k, 0.07 * k, 0.15 * k, 10, false), &ua.then(&Xf::translate([0.0, -0.07 * k, 0.0])), cloth);
        }
        let fa = joint(&ua, fa_n, [0.0, -0.29 * al * k, 0.0]);
        seg(b, &put, &fa, 0.27 * al, 0.04, 0.03);
        if let Some((what, surf)) = spec.holds {
            if (what == "clipboard" || what == "basket") == (side < 0.0) {
                carried(b, &put, &fa, what, surf, k, al);
            }
        }
        put(b, &Geo::cube([0.07 * k * tk, 0.16 * k * al, 0.032 * k]), &fa.then(&Xf::translate([0.0, -(0.27 * al + 0.08 * al) * k, 0.0])), skin);
        let th = joint(&hips, th_n, [side * 0.095 * k * if thin { 0.8 } else { 1.0 }, -0.04 * k, 0.0]);
        seg(b, &put, &th, 0.46, 0.076, 0.055);
        let sh = joint(&th, sh_n, [0.0, -0.46 * k, 0.0]);
        seg(b, &put, &sh, 0.44, 0.054, 0.038);
        put(b, &Geo::cube([0.085 * k * tk, 0.06 * k, 0.24 * k]), &sh.then(&Xf::translate([0.0, -0.44 * k + 0.02 * k, -0.06 * k])), skin);
    }
}

/// What a figure carries, in its forearm's frame, as the page's `holds` builds it (the main
/// shapes only: previz leaves out handles and spouts).
fn carried(b: &mut Builder, put: &dyn Fn(&mut Builder, &Geo, &Xf, u32), fa: &Xf, what: &str, surf: u32, k: f64, al: f64) {
    let at = |x: f64, y: f64, z: f64, rx: f64| fa.then(&Xf::three([x * k, y * k, z * k], [rx, 0.0, 0.0], Order::Xyz, [1.0; 3]));
    if what == "clipboard" {
        put(b, &Geo::cube([0.22 * k, 0.3 * k, 0.012 * k]), &at(0.02, -0.3, -0.06, 0.9), surf);
        return;
    }
    let hand = fa.then(&Xf::translate([0.0, -(0.27 * al + 0.1 * al) * k, 0.0]));
    let at = |x: f64, y: f64, z: f64, rx: f64| hand.then(&Xf::three([x * k, y * k, z * k], [rx, 0.0, 0.0], Order::Xyz, [1.0; 3]));
    match what {
        "can" => put(b, &Geo::cylinder(0.075, 0.085, 0.17, 12, true), &at(0.0, -0.1, -0.03, 0.0), surf),
        "teapot" => put(b, &Geo::sphere(0.075, 12, 8), &at(0.0, -0.07, -0.05, 0.0).then(&Xf::scale([1.0, 0.82, 1.0])), surf),
        "basket" => put(b, &Geo::cylinder(0.15, 0.12, 0.13, 12, true), &at(0.0, -0.2, 0.0, 0.0), surf),
        "broom" => {
            put(b, &Geo::cylinder(0.012, 0.012, 1.3, 6, true), &at(0.0, -0.25, -0.12, 0.5), surf);
            put(b, &Geo::cylinder(0.03, 0.09, 0.3, 8, true), &at(0.0, -0.85, -0.42, 0.5), surf);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_matches_the_pages_rng() {
        // rng(seed)() as node computes it for the page
        for (seed, want) in [("a", 0.5655837582889944), ("fallen_patient", 0.03876341111026704), ("night_nurse", 0.8071963102556765)] {
            assert_eq!(first_draw(seed), want, "{seed}");
        }
    }

    #[test]
    fn a_standing_figure_is_as_tall_as_asked_and_one_lying_face_down_is_low() {
        let tall = |mv: &str| {
            let mut b = Builder::default();
            let spec = Spec { mv, height: 175.0, thin: false, seed: "x", skin: 0, cloth: None, holds: None };
            add(&mut b, &Xf::THREE_TO_UE, &spec, 0.0, 0);
            let z: Vec<f64> = b.tris.iter().flat_map(|t| t.p.iter().map(|p| p[2])).collect();
            (z.iter().cloned().fold(f64::INFINITY, f64::min), z.iter().cloned().fold(f64::NEG_INFINITY, f64::max))
        };
        let (lo, hi) = tall("idle");
        assert!(lo > -15.0 && lo < 5.0, "feet at {lo}");
        assert!(hi > 160.0 && hi < 185.0, "head at {hi}");
        let (_, hi) = tall("face_down");
        assert!(hi < 50.0, "lying figure reaches {hi}");
    }
}
