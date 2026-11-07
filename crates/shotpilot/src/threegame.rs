//! The three engine's sample game (D-72): Lanternless, the closed-down ward at night. The floor
//! plan and the five anchors stand where the UE sample has them, so the same shots.yaml films
//! both; what fills the rooms is the three game's own (handrails, doors, beds and curtains, a
//! vending machine and a television of static, patients in gowns, the yard and the moon).
//!
//! One file per thing (`objects/<name>.json`): edit the stretcher and only the shots that showed
//! it go stale. Written once by `shotpilot init --engine three`; after that it is the game, and
//! people edit it by hand.
//!
//! Every part's `box` (or cylinder, sphere, figure) is also the shape shotpilot uses to decide
//! what a shot saw. Keys only the page reads refine how it looks: `shape` (round, curtain, torus,
//! sign, decal), `tilt`, `dress`, `build`, `halo`, `beam`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::Result;
use crate::three::{OBJECTS_DIR, PROPS_DIR, SCENE_FILE};

type V3 = [f64; 3];

const H: f64 = 300.0; // ceiling
const THICK: f64 = 20.0;
const DADO: f64 = 105.0; // where the painted lower wall meets the plaster

/// `v` with the keys of `extra` added (or replaced).
fn with(mut v: Value, extra: Value) -> Value {
    if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra) {
        o.extend(e);
    }
    v
}

fn bx(size: V3, at: V3, mat: &str) -> Value {
    json!({"box": size, "at": at, "mat": mat})
}

fn bxy(size: V3, at: V3, mat: &str, yaw: f64) -> Value {
    json!({"box": size, "at": at, "mat": mat, "yaw": yaw})
}

/// A box with rounded edges.
fn rb(size: V3, at: V3, mat: &str) -> Value {
    json!({"box": size, "at": at, "mat": mat, "shape": "round"})
}

/// Something too thin or small to hide what is behind it.
fn thin(v: Value) -> Value {
    with(v, json!({"solid": false}))
}

/// An upright cylinder standing on `base`.
fn post(base: V3, diameter: f64, height: f64, mat: &str) -> Value {
    json!({"cylinder": [diameter / 2.0, height], "at": [base[0], base[1], base[2] + height / 2.0], "mat": mat})
}

/// A cylinder lying along X (or Y) centred on `at`.
fn rod(at: V3, diameter: f64, length: f64, axis: &str, mat: &str) -> Value {
    json!({"cylinder": [diameter / 2.0, length], "axis": axis, "at": at, "mat": mat})
}

/// A sign or board: text (or a drawing) on its two broad faces.
fn sign(size: V3, at: V3, text: &str, fg: &str, bg: &str) -> Value {
    json!({"box": size, "at": at, "mat": "frame", "shape": "sign", "text": text, "fg": fg, "bg": bg})
}

fn drawn(size: V3, at: V3, draw: &str) -> Value {
    json!({"box": size, "at": at, "mat": "frame", "shape": "sign", "draw": draw})
}

/// A stain on the floor.
fn stain(size: [f64; 2], at: [f64; 2], yaw: f64, kind: &str) -> Value {
    json!({"box": [size[0], size[1], 0.4], "at": [at[0], at[1], 0.2], "yaw": yaw, "shape": "decal", "stain": kind, "solid": false})
}

fn figure(mv: &str, feet: V3, facing: f64) -> Value {
    json!({"figure": mv, "at": feet, "facing": facing, "height": 175})
}

fn patient(mv: &str, feet: V3, facing: f64) -> Value {
    with(figure(mv, feet, facing), json!({"dress": "gown"}))
}

fn point(at: V3, color: &str, intensity: f64, range: f64) -> Value {
    json!({"light": "point", "at": at, "color": color, "intensity": intensity, "range": range})
}

/// A light pointing straight down from `at`.
fn downlight(at: V3, color: &str, intensity: f64, cone: f64) -> Value {
    json!({"light": "spot", "at": at, "toward": [at[0], at[1], 0.0], "color": color, "intensity": intensity,
           "range": 900, "cone": cone, "penumbra": 0.85})
}

fn along(a: (f64, f64), b: (f64, f64), u: f64) -> (f64, f64) {
    let l = (b.0 - a.0).hypot(b.1 - a.1);
    (a.0 + (b.0 - a.0) * u / l, a.1 + (b.1 - a.1) * u / l)
}

fn yaw_of(a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.1 - a.1).atan2(b.0 - a.0).to_degrees()
}

/// A straight wall from a to b with openings (u0, u1, bottom, top) measured from a; `ends` adds
/// length at each end, as `Level.wall` does in the UE builder. Painted below the dado, plaster
/// above, and a rail where they meet.
fn wall(a: (f64, f64), b: (f64, f64), holes: &[(f64, f64, f64, f64)], ends: (f64, f64)) -> Vec<Value> {
    let length = (b.0 - a.0).hypot(b.1 - a.1);
    let yaw = yaw_of(a, b);
    let mut pieces = Vec::new();
    let mut u = -ends.0;
    let mut sorted = holes.to_vec();
    sorted.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    for (u0, u1, bottom, top) in sorted {
        if u0 > u {
            pieces.push((u, u0, 0.0, H));
        }
        if bottom > 0.0 {
            pieces.push((u0, u1, 0.0, bottom));
        }
        if top < H {
            pieces.push((u0, u1, top, H));
        }
        u = u1;
    }
    if u < length + ends.1 {
        pieces.push((u, length + ends.1, 0.0, H));
    }
    let mut out = Vec::new();
    for (u0, u1, z0, z1) in pieces {
        let (x, y) = along(a, b, (u0 + u1) / 2.0);
        for (lo, hi, mat) in [(z0, z1.min(DADO), "dado"), (z0.max(DADO), z1, "plaster")] {
            if hi - lo > 0.5 {
                out.push(bxy([u1 - u0, THICK, hi - lo], [x, y, (lo + hi) / 2.0], mat, yaw));
            }
        }
        if z0 < DADO && z1 > DADO + 4.0 {
            out.push(thin(bxy([u1 - u0, THICK + 3.0, 5.0], [x, y, DADO + 2.0], "rail", yaw)));
        }
    }
    out
}

/// The window in the opening (u0..u1, bottom..top) of wall a -> b: a frame, glass, bars, a sill.
fn window(a: (f64, f64), b: (f64, f64), u0: f64, u1: f64, bottom: f64, top: f64) -> Vec<Value> {
    let yaw = yaw_of(a, b);
    let mut out = Vec::new();
    let (cx, cy) = along(a, b, (u0 + u1) / 2.0);
    let mid = (bottom + top) / 2.0;
    out.push(thin(bxy([u1 - u0, 1.0, top - bottom], [cx, cy, mid], "glass", yaw)));
    for (u, w) in [(u0 + 3.0, 6.0), (u1 - 3.0, 6.0), ((u0 + u1) / 2.0, 4.0)] {
        let (x, y) = along(a, b, u);
        out.push(thin(bxy([w, THICK + 4.0, top - bottom], [x, y, mid], "frame", yaw)));
    }
    out.push(thin(bxy([u1 - u0, THICK + 4.0, 6.0], [cx, cy, top - 3.0], "frame", yaw)));
    out.push(thin(bxy([u1 - u0, THICK + 4.0, 3.0], [cx, cy, (bottom + top) * 0.5 + 8.0], "frame", yaw)));
    out.push(thin(bxy([u1 - u0 + 10.0, THICK + 14.0, 4.0], [cx, cy, bottom + 2.0], "frame", yaw)));
    let n = ((u1 - u0) / 16.0).floor() as i64;
    for i in 1..=n {
        let (x, y) = along(a, b, u0 + (u1 - u0) * i as f64 / (n + 1) as f64);
        out.push(post([x, y, bottom], 2.6, top - bottom, "iron"));
    }
    out
}

/// Door frame inside an opening of wall a -> b: two jambs and a head.
fn doorway(a: (f64, f64), b: (f64, f64), u0: f64, u1: f64, top: f64) -> Vec<Value> {
    let yaw = yaw_of(a, b);
    let mut out: Vec<Value> = [u0 + 3.0, u1 - 3.0]
        .iter()
        .map(|u| {
            let (x, y) = along(a, b, *u);
            thin(bxy([6.0, THICK + 6.0, top], [x, y, top / 2.0], "frame", yaw))
        })
        .collect();
    let (x, y) = along(a, b, (u0 + u1) / 2.0);
    out.push(thin(bxy([u1 - u0, THICK + 6.0, 8.0], [x, y, top + 4.0], "frame", yaw)));
    out
}

/// A door leaf hung at `hinge`, opened to `angle` degrees (world yaw of the leaf from the hinge).
fn door(hinge: (f64, f64), angle: f64, width: f64, height: f64) -> Vec<Value> {
    let (s, c) = angle.to_radians().sin_cos();
    let at = |u: f64| (hinge.0 + c * u, hinge.1 + s * u);
    let (x, y) = at(width / 2.0);
    let (hx, hy) = at(width - 12.0);
    vec![
        json!({"box": [width, 4.5, height], "at": [x, y, height / 2.0 + 1.0], "yaw": angle, "mat": "door", "shape": "round", "r": 1.0}),
        thin(json!({"box": [26.0, 1.0, 34.0], "at": [at(width * 0.5).0, at(width * 0.5).1, 165.0], "yaw": angle, "mat": "glass"})),
        thin(json!({"box": [4.0, 9.0, 14.0], "at": [hx, hy, 100.0], "yaw": angle, "mat": "steel", "shape": "round", "r": 1.0})),
    ]
}

/// A plastic chair: seat, back and four legs, standing at `at` facing `facing`.
fn chair_parts(mat: &str) -> Vec<Value> {
    let mut p = vec![rb([44.0, 44.0, 5.0], [0.0, 0.0, 44.0], mat), with(rb([6.0, 44.0, 40.0], [-21.0, 0.0, 68.0], mat), json!({"tilt": [0.0, 0.0]}))];
    for (x, y) in [(-18.0, -18.0), (-18.0, 18.0), (18.0, -18.0), (18.0, 18.0)] {
        p.push(thin(post([x, y, 0.0], 2.4, 42.0, "steel")));
    }
    p
}

/// A hospital bed in its own frame (head towards -x), length 200 along x.
fn bed_parts(with_mattress: bool) -> Vec<Value> {
    let mut p = Vec::new();
    if with_mattress {
        p.push(rb([194.0, 86.0, 14.0], [2.0, 0.0, 51.0], "vinyl"));
        p.push(with(rb([196.0, 90.0, 3.0], [4.0, 0.0, 59.0], "sheet"), json!({"r": 1.5})));
        p.push(rb([34.0, 58.0, 12.0], [-78.0, 0.0, 66.0], "sheet"));
        p.push(json!({"box": [150.0, 4.0, 34.0], "at": [12.0, 45.0, 44.0], "mat": "sheet", "shape": "curtain"}));
    } else {
        for i in 0..9 {
            p.push(thin(rod([-90.0 + 22.5 * i as f64, 0.0, 44.0], 1.6, 84.0, "y", "iron")));
        }
    }
    p.push(rb([200.0, 90.0, 6.0], [0.0, 0.0, 41.0], "steel"));
    p.push(rb([6.0, 94.0, 96.0], [-103.0, 0.0, 48.0], "bedhead"));
    p.push(rb([6.0, 94.0, 66.0], [103.0, 0.0, 33.0], "bedhead"));
    for (x, y) in [(-92.0, -40.0), (-92.0, 40.0), (92.0, -40.0), (92.0, 40.0)] {
        p.push(thin(post([x, y, 10.0], 4.0, 32.0, "steel")));
        p.push(thin(json!({"box": [10.0, 3.0, 10.0], "at": [x, y, 5.5], "mat": "rubber", "shape": "torus"})));
    }
    for y in [-45.0, 45.0] {
        p.push(thin(rod([0.0, y, 72.0], 2.4, 110.0, "x", "steel")));
        for x in [-50.0, 50.0] {
            p.push(thin(post([x, y, 44.0], 2.0, 28.0, "steel")));
        }
    }
    p
}

/// A wheelchair in its own frame, facing +x.
fn wheelchair_parts() -> Vec<Value> {
    let mut p = vec![
        rb([44.0, 44.0, 5.0], [0.0, 0.0, 50.0], "vinyl"),
        with(rb([6.0, 44.0, 44.0], [-24.0, 0.0, 76.0], "vinyl"), json!({"tilt": [0.0, 0.0]})),
    ];
    for y in [-27.0, 27.0] {
        p.push(thin(json!({"box": [62.0, 3.5, 62.0], "at": [-6.0, y, 31.0], "mat": "rubber", "shape": "torus"})));
        p.push(thin(json!({"box": [50.0, 1.5, 50.0], "at": [-6.0, y * 1.07, 31.0], "mat": "steel", "shape": "torus"})));
        p.push(thin(rod([4.0, y * 0.85, 68.0], 2.4, 46.0, "x", "steel")));
        p.push(thin(json!({"box": [10.0, 2.5, 10.0], "at": [26.0, y * 0.75, 5.0], "mat": "rubber", "shape": "torus"})));
        p.push(thin(post([26.0, y * 0.75, 8.0], 2.0, 40.0, "steel")));
        p.push(thin(post([-26.0, y * 0.85, 48.0], 2.0, 50.0, "steel")));
    }
    p.push(thin(rb([16.0, 40.0, 2.0], [32.0, 0.0, 14.0], "steel")));
    p
}

fn structure(label: &str, parts: Vec<Value>) -> Value {
    json!({"label": label, "structure": true, "parts": parts})
}

fn thing(label: &str, parts: Vec<Value>) -> Value {
    json!({"label": label, "parts": parts})
}

/// A thing built in its own frame, standing at `at` and turned by `yaw`.
fn thing_at(label: &str, at: V3, yaw: f64, parts: Vec<Value>) -> Value {
    json!({"label": label, "at": at, "yaw": yaw, "parts": parts})
}

fn anchored(label: &str, anchor: Value, parts: Vec<Value>) -> Value {
    json!({"label": label, "anchor": anchor, "parts": parts})
}

fn scene() -> Value {
    json!({
        "name": "Lanternless",
        "background": "#020306",
        "exposure": 1.15,
        "fog": {"color": "#0a1016", "density": 0.022, "ground": 0.09, "height": 0.7},
        "sky": {"top": "#010205", "horizon": "#0b1219", "moon": "#d9e4ff", "moon_size": 1.2},
        "post": {"bloom": 0.55, "threshold": 1.4, "ao": 0.9, "ao_radius": 0.55, "vignette": 0.6, "grain": 0.035, "aberration": 0.0022, "dof": 1.0},
        "ambient": {"sky": "#1a2633", "ground": "#050606", "intensity": 0.18},
        "moon": {"from": [-0.742, -0.52, 0.423], "color": "#8fa8ff", "intensity": 0.55, "shadow": true,
                 "center": [900, 0, 0], "reach": 2600},
        "flicker": {"offset": 1.4, "period": 2.6, "dips": [[0, 0.08], [0.17, 0.25]], "low": 0.12},
        "materials": materials(),
    })
}

/// The materials, by name: `tex` names the surface the page bakes from noise.
fn materials() -> Value {
    let mut m = json!({
                "floor": {"tex": "tiles", "color": "#aaa592", "color2": "#283330", "color3": "#3b3a34", "n": 8, "scale": [2.4, 2.4]},
                "yard": {"tex": "concrete", "color": "#33362f", "color2": "#1b2617", "scale": [6, 6]},
                "plaster": {"tex": "plaster", "color": "#8d9184", "color2": "#26301f", "scale": [4, 3]},
                "dado": {"tex": "paint", "color": "#2c4740", "color2": "#8a8775", "scale": [2, 3]},
                "ceiling": {"tex": "ceiling", "color": "#a3a399", "color2": "#77786f", "n": 4, "scale": [2.4, 2.4]},
                "rail": {"tex": "paint", "color": "#4d5a52", "color2": "#8a8775", "scale": [2, 3], "wear": 0.6},
                "frame": {"tex": "paint", "color": "#59625b", "color2": "#8f8b78", "scale": [1, 1.5], "wear": 0.8},
                "door": {"tex": "paint", "color": "#556a64", "color2": "#9a9684", "scale": [1.2, 2.2]},
                "bedhead": {"tex": "paint", "color": "#7d8a86", "color2": "#a8a594", "scale": [1, 1], "wear": 0.7},
                "laminate": {"tex": "paint", "color": "#6b5a45", "color2": "#a89878", "scale": [1.5, 1.5], "wear": 0.6},
                "cart": {"tex": "paint", "color": "#8fa39a", "color2": "#b5b2a2", "scale": [1, 1], "wear": 0.8},
                "concrete": {"tex": "concrete", "color": "#4a4c46", "color2": "#26301f", "scale": [3, 3]},
                "metal": {"tex": "metal", "color": "#5e6468", "color2": "#5a3a22", "metal": 0.8, "scale": [1, 1]},
                "steel": {"tex": "metal", "color": "#a6acb0", "color2": "#6b4a2c", "metal": 0.9, "wear": 0.35, "scale": [0.8, 0.8]},
                "iron": {"tex": "metal", "color": "#3a3d3f", "color2": "#5a3a22", "metal": 0.7, "wear": 1.2, "scale": [0.6, 0.6]},
                "paint_metal": {"tex": "metal", "color": "#6f7d76", "color2": "#5d4630", "metal": 0.15, "wear": 0.45, "scale": [1, 1]},
                "wood": {"tex": "wood", "color": "#3a281b", "color2": "#5c4330", "scale": [1.2, 1.2]},
                "bark": {"tex": "wood", "color": "#1c1814", "color2": "#2a241e", "scale": [1.5, 0.6], "wear": 1.4},
                "vinyl": {"tex": "plastic", "color": "#2f4f4c", "scale": [1, 1]},
                "seat": {"tex": "plastic", "color": "#2d5560", "scale": [0.8, 0.8], "wear": 0.8},
                "plastic_old": {"tex": "plastic", "color": "#b7ad95", "scale": [0.6, 0.6]}
    });
    if let (Some(o), Value::Object(more)) = (m.as_object_mut(), json!({
                "sheet": {"tex": "fabric", "color": "#b9bab0", "n": 110, "scale": [1, 1], "double": true},
                "gown": {"tex": "fabric", "color": "#7c97a0", "n": 80, "scale": [0.8, 0.8], "double": true},
                "uniform": {"tex": "fabric", "color": "#3a4850", "n": 80, "scale": [0.8, 0.8], "double": true},
                "curtain": {"tex": "fabric", "color": "#62806f", "n": 60, "scale": [1.5, 2.5], "double": true},
                "figure": {"tex": "plastic", "color": "#d2cbbd", "scale": [0.5, 0.5], "wear": 0.6},
                "shade": {"color": "#08090a", "rough": 0.85},
                "glass": {"glass": true, "color": "#a8bccb", "opacity": 0.1},
                "bag": {"glass": true, "color": "#dfe9e4", "opacity": 0.45, "rough": 0.2},
                "paper": {"color": "#cfcabb", "rough": 0.9},
                "binder": {"color": "#33465e", "rough": 0.6},
                "void": {"color": "#000000", "rough": 1.0},
                "rubber": {"color": "#141414", "rough": 0.75},
                "dry": {"color": "#4a3f2c", "rough": 0.9},
                "soil": {"color": "#1f1812", "rough": 1.0},
                "accent": {"color": "#6a2b25", "rough": 0.6},
                "screen_dead": {"color": "#0a0d0e", "rough": 0.08, "metal": 0.2},
                "lamp_on": {"color": "#e6f2ec", "glow": 9.0},
                "lamp_off": {"color": "#26292a", "rough": 0.25},
                "lamp_flicker": {"color": "#d6f2e6", "glow": 9.0, "flicker": true},
                "lamp_warm": {"color": "#ffd3a2", "glow": 7.0},
                "exit": {"color": "#ff1a10", "glow": 9.0}
    })) {
        o.extend(more);
    }
    m
}

/// Every object of the ward, by file name.
fn objects() -> Vec<(&'static str, Value)> {
    let mut o: Vec<(&str, Value)> = Vec::new();

    o.push(("floor", structure("Ground and roof", vec![
        bx([8000.0, 6000.0, 20.0], [0.0, 0.0, -12.0], "yard"),
        bx([3400.0, 1300.0, 2.0], [900.0, 0.0, -1.0], "floor"),
        bx([3440.0, 1340.0, 20.0], [900.0, 0.0, H + 10.0], "ceiling"),
    ])));

    // lobby x -800..0, y -650..650
    let mut lobby = Vec::new();
    let (fa, fb) = ((-800.0, 650.0), (-800.0, -650.0));
    lobby.extend(wall(fa, fb, &[(150.0, 450.0, 100.0, 230.0), (850.0, 1150.0, 100.0, 230.0)], (10.0, 10.0)));
    lobby.extend(window(fa, fb, 150.0, 450.0, 100.0, 230.0));
    lobby.extend(window(fa, fb, 850.0, 1150.0, 100.0, 230.0));
    let (sa, sb) = ((-800.0, -650.0), (0.0, -650.0));
    lobby.extend(wall(sa, sb, &[(300.0, 600.0, 100.0, 230.0)], (-10.0, -10.0)));
    lobby.extend(window(sa, sb, 300.0, 600.0, 100.0, 230.0));
    lobby.extend(wall((-800.0, 650.0), (0.0, 650.0), &[], (-10.0, -10.0)));
    lobby.extend(wall((0.0, -650.0), (0.0, 650.0), &[(510.0, 790.0, 0.0, 260.0)], (10.0, 10.0)));
    lobby.extend(doorway((0.0, -650.0), (0.0, 650.0), 510.0, 790.0, 260.0));
    o.push(("lobby", structure("Lobby walls", lobby)));

    // corridor x 0..2600, y -150..150, ending in a blank wall
    let mut corridor = Vec::new();
    let (s0, s1) = ((0.0, -150.0), (2600.0, -150.0));
    let (n0, n1) = ((0.0, 150.0), (2600.0, 150.0));
    corridor.extend(wall(s0, s1, &[(1100.0, 1220.0, 0.0, 220.0)], (-10.0, 10.0)));
    corridor.extend(wall(n0, n1, &[(600.0, 720.0, 0.0, 220.0), (1550.0, 1670.0, 0.0, 220.0)], (-10.0, 10.0)));
    corridor.extend(wall((2600.0, -150.0), (2600.0, 150.0), &[], (-10.0, -10.0)));
    corridor.extend(doorway(s0, s1, 1100.0, 1220.0, 220.0));
    corridor.extend(doorway(n0, n1, 600.0, 720.0, 220.0));
    corridor.extend(doorway(n0, n1, 1550.0, 1670.0, 220.0));
    o.push(("corridor", structure("Corridor walls", corridor)));

    // handrails along both corridor walls, broken by the doors
    let mut rails = Vec::new();
    for (y, runs) in [(134.0, vec![(10.0, 590.0), (730.0, 1540.0), (1680.0, 2590.0)]), (-134.0, vec![(10.0, 1090.0), (1230.0, 2590.0)])] {
        for (x0, x1) in runs {
            rails.push(thin(rod([(x0 + x1) / 2.0, y, 88.0], 5.0, x1 - x0, "x", "rail")));
            let n = ((x1 - x0) / 150.0).ceil() as i64;
            for i in 0..=n {
                let x = x0 + 8.0 + (x1 - x0 - 16.0) * i as f64 / n as f64;
                rails.push(thin(bx([3.0, 8.0, 3.0], [x, y + 4.0 * y.signum(), 86.0], "steel")));
            }
        }
    }
    o.push(("handrails", structure("Corridor handrails", rails)));

    // the doors stand open into their rooms
    let mut doors = door((602.0, 162.0), 100.0, 114.0, 216.0);
    doors.extend(door((1552.0, 162.0), 70.0, 114.0, 216.0));
    doors.extend(door((1218.0, -162.0), 262.0, 114.0, 216.0));
    o.push(("doors", structure("Room doors", doors)));

    o.push(("room_signs", structure("Room signs", vec![
        sign([3.0, 30.0, 16.0], [590.0, 122.0, 245.0], "101", "#1d2420", "#d9d6c8"),
        sign([3.0, 30.0, 16.0], [1540.0, 122.0, 245.0], "102", "#1d2420", "#d9d6c8"),
        sign([3.0, 30.0, 16.0], [1090.0, -122.0, 245.0], "103", "#1d2420", "#d9d6c8"),
        sign([3.0, 120.0, 34.0], [950.0, 0.0, 262.0], "病棟  WARD", "#e3e0d2", "#24443c"),
        thin(post([950.0, -45.0, 279.0], 0.6, 21.0, "steel")),
        thin(post([950.0, 45.0, 279.0], 0.6, 21.0, "steel")),
    ])));

    o.push(("pipes", structure("Ceiling pipes", vec![
        thin(rod([-400.0, -120.0, H - 7.0], 10.0, 780.0, "x", "metal")),
        thin(rod([1300.0, 108.0, H - 8.0], 11.0, 2540.0, "x", "metal")),
        thin(rod([1300.0, 92.0, H - 6.0], 5.0, 2540.0, "x", "iron")),
    ])));

    // lobby: the reception desk (nobody behind it), its lamp still on
    let mut desk = vec![
        rb([100.0, 300.0, 75.0], [-290.0, 380.0, 37.5], "laminate"),
        rb([104.0, 304.0, 4.0], [-290.0, 380.0, 77.0], "wood"),
        rb([20.0, 300.0, 110.0], [-350.0, 380.0, 55.0], "dado"),
        rb([38.0, 312.0, 5.0], [-352.0, 380.0, 112.0], "wood"),
        rb([38.0, 40.0, 34.0], [-305.0, 330.0, 96.0], "plastic_old"),
        thin(bx([1.0, 30.0, 24.0], [-285.5, 330.0, 98.0], "screen_dead")),
        thin(rb([15.0, 44.0, 3.0], [-262.0, 330.0, 80.5], "plastic_old")),
        thin(rb([18.0, 22.0, 8.0], [-268.0, 255.0, 83.0], "rubber")),
        thin(bxy([22.0, 30.0, 4.0], [-290.0, 455.0, 81.0], "paper", 8.0)),
        thin(bxy([22.0, 30.0, 0.4], [-272.0, 418.0, 79.2], "paper", -21.0)),
        thin(post([-318.0, 500.0, 79.0], 14.0, 2.0, "iron")),
        thin(json!({"cylinder": [1.0, 34.0], "at": [-316.0, 497.0, 96.0], "mat": "iron", "tilt": [0.0, -18.0]})),
        thin(json!({"cylinder": [10.0, 12.0], "top": 4.0, "open": true, "at": [-310.0, 492.0, 114.0], "mat": "iron", "tilt": [0.0, 25.0]})),
        thin(json!({"sphere": 3.0, "at": [-309.0, 491.0, 111.0], "mat": "lamp_warm"})),
        with(point([-306.0, 488.0, 104.0], "#ffbb7a", 5.0, 520.0), json!({"shadow": true, "halo": 60, "halo_opacity": 0.18})),
    ];
    for (i, c) in ["#33465e", "#5e3333", "#3d5e33", "#33465e", "#6b5a2a", "#2e2e2e"].iter().enumerate() {
        desk.push(thin(json!({"box": [24.0, 5.0, 30.0], "at": [-270.0, 470.0 + 6.0 * i as f64, 94.0], "mat": "binder", "color": c})));
    }
    o.push(("reception_desk", anchored("SP_reception_desk",
        json!({"at": [-360.0, 230.0, 0.0], "size": [120.0, 300.0, 110.0], "center": [-300.0, 380.0, 55.0]}), desk)));
    o.push(("reception_sign", thing("Reception sign", vec![
        sign([3.0, 130.0, 42.0], [-300.0, 380.0, 252.0], "受付\nRECEPTION", "#e8e4d4", "#20342f"),
        thin(post([-300.0, 330.0, 273.0], 0.6, 27.0, "steel")),
        thin(post([-300.0, 430.0, 273.0], 0.6, 27.0, "steel")),
    ])));
    o.push(("clerk_chair", thing_at("Clerk's chair", [-195.0, 410.0, 0.0], 215.0, {
        let mut p = vec![
            rb([48.0, 48.0, 8.0], [0.0, 0.0, 48.0], "seat"),
            rb([8.0, 44.0, 50.0], [-24.0, 0.0, 80.0], "seat"),
            thin(post([0.0, 0.0, 10.0], 5.0, 36.0, "steel")),
        ];
        for a in 0..5 {
            let r = (a as f64 * 72.0).to_radians();
            p.push(thin(bxy([30.0, 4.0, 3.0], [15.0 * r.cos(), 15.0 * r.sin(), 8.0], "steel", a as f64 * 72.0)));
        }
        p
    })));
    let mut shelf = vec![
        rb([100.0, 34.0, 190.0], [-140.0, 622.0, 95.0], "paint_metal"),
    ];
    for z in [55.0, 105.0, 155.0] {
        for i in 0..11 {
            let c = ["#33465e", "#5e3333", "#3d5e33", "#6b5a2a", "#2e2e2e", "#4a4a6e"][(i * 7 + z as usize / 5) % 6];
            let lean = if i == 9 { 12.0 } else { 0.0 };
            shelf.push(thin(json!({"box": [6.5, 26.0, 32.0], "at": [-184.0 + 8.0 * i as f64, 600.0, z + 17.0], "mat": "binder", "color": c, "tilt": [0.0, lean]})));
        }
    }
    shelf.push(drawn([40.0, 3.0, 40.0], [-140.0, 637.0, 245.0], "clock"));
    o.push(("records_shelf", thing("Records shelf and clock", shelf)));

    // waiting area: two rows of linked seats, one knocked over
    for (name, y) in [("waiting_seats", -560.0), ("waiting_seats_front", -360.0)] {
        let mut p = vec![thin(bx([370.0, 6.0, 5.0], [-505.0, y - 6.0, 38.0], "steel"))];
        for i in 0..7 {
            let x = -660.0 + 52.0 * i as f64;
            if name == "waiting_seats_front" && i == 2 {
                continue;
            }
            p.push(rb([46.0, 44.0, 5.0], [x, y, 44.0], "seat"));
            p.push(rb([46.0, 5.0, 40.0], [x, y - 22.0, 70.0], "seat"));
        }
        for x in [-680.0, -505.0, -330.0] {
            p.push(thin(bx([4.0, 40.0, 38.0], [x, y - 6.0, 19.0], "steel")));
        }
        o.push((if name == "waiting_seats" { "waiting_benches" } else { "waiting_benches_front" },
                thing(if name == "waiting_seats" { "Waiting seats by the wall" } else { "Waiting seats" }, p)));
    }
    o.push(("fallen_seat", thing_at("A seat on its side", [-540.0, -270.0, 22.0], 35.0, {
        let mut p = chair_parts("seat");
        for v in p.iter_mut() {
            v["tilt"] = json!([0.0, 88.0]);
            if let Some(at) = v["at"].as_array().cloned() {
                // lying on its side: up becomes sideways
                let (x, y, z) = (at[0].as_f64().unwrap_or(0.0), at[1].as_f64().unwrap_or(0.0), at[2].as_f64().unwrap_or(0.0));
                v["at"] = json!([x, -(z - 22.0), y + 0.0]);
            }
        }
        p
    })));
    o.push(("vending_machine", thing("Vending machine", vec![
        with(rb([92.0, 74.0, 183.0], [-95.0, -604.0, 91.5], "paint_metal"), json!({"color": "#6d1b17"})),
        json!({"box": [80.0, 3.0, 168.0], "at": [-95.0, -566.0, 96.0], "mat": "frame", "shape": "sign", "draw": "vending", "glow": 1.4}),
        with(point([-95.0, -500.0, 120.0], "#cfe0ff", 3.2, 520.0), json!({"halo": 150, "halo_opacity": 0.12})),
    ])));
    o.push(("television", thing("Television of static", vec![
        rb([40.0, 72.0, 52.0], [-772.0, 0.0, 232.0], "plastic_old"),
        json!({"box": [3.0, 62.0, 42.0], "at": [-751.0, 0.0, 232.0], "mat": "frame", "shape": "sign", "draw": "static", "glow": 1.4}),
        thin(bx([22.0, 12.0, 8.0], [-788.0, 0.0, 204.0], "iron")),
        with(point([-700.0, 0.0, 228.0], "#a8bdd9", 1.4, 500.0), json!({"halo": 90, "halo_opacity": 0.08})),
    ])));
    o.push(("notice_board", thing("Notice board", vec![drawn([3.0, 130.0, 85.0], [-12.0, 380.0, 150.0], "notices")])));
    for (name, at) in [("dead_plant", [-735.0, 585.0]), ("dead_plant_door", [-60.0, -210.0])] {
        let mut p = vec![
            post([at[0], at[1], 0.0], 44.0, 42.0, "concrete"),
            thin(post([at[0], at[1], 38.0], 38.0, 2.0, "soil")),
        ];
        for i in 0..6 {
            let a = i as f64 * 60.0 + 15.0;
            p.push(thin(json!({"cylinder": [0.8, 70.0 + 12.0 * (i % 3) as f64], "at": [at[0], at[1], 75.0], "mat": "dry", "yaw": a, "tilt": [12.0 + 6.0 * (i % 2) as f64, 0.0]})));
            p.push(thin(json!({"box": [26.0, 9.0, 0.6], "at": [at[0] + 18.0 * a.to_radians().cos(), at[1] + 18.0 * a.to_radians().sin(), 92.0 + 8.0 * (i % 3) as f64],
                               "mat": "dry", "yaw": a, "tilt": [0.0, -55.0]})));
        }
        o.push((name, thing("Dead plant", p)));
    }
    o.push(("wheelchair", thing_at("Wheelchair", [-150.0, 110.0, 0.0], 205.0, wheelchair_parts())));
    let mut litter = vec![
        stain([180.0, 140.0], [-430.0, 120.0], 0.0, "pool"),
        stain([120.0, 90.0], [-140.0, -380.0], 30.0, "pool"),
    ];
    for (i, (x, y)) in [(-470.0, 40.0), (-430.0, 85.0), (-395.0, -20.0), (-520.0, 140.0), (-250.0, -150.0), (-610.0, -120.0), (-180.0, 250.0)].iter().enumerate() {
        litter.push(thin(bxy([21.0, 29.7, 0.3], [*x, *y, 0.4], "paper", (i as f64 * 47.0) % 180.0)));
    }
    litter.push(thin(json!({"box": [60.0, 60.0, 1.5], "at": [-505.0, -300.0, 276.0], "mat": "ceiling", "tilt": [28.0, 12.0]})));
    litter.push(thin(bx([60.0, 60.0, 0.6], [-505.0, -330.0, 299.6], "void")));
    o.push(("lobby_litter", thing("Papers, stains and a fallen ceiling tile", litter)));

    // the stretcher left askew in the corridor, and an IV stand beside it
    let mut stretcher = vec![
        with(rb([196.0, 66.0, 10.0], [0.0, 0.0, 81.0], "vinyl"), json!({"r": 4.0, "color": "#232b33"})),
        with(rb([150.0, 70.0, 2.0], [22.0, 2.0, 86.5], "sheet"), json!({"yaw": 4.0, "r": 0.8})),
        rb([30.0, 52.0, 9.0], [-80.0, 0.0, 91.0], "sheet"),
        json!({"box": [120.0, 4.0, 50.0], "at": [26.0, 37.0, 63.0], "mat": "sheet", "shape": "curtain"}),
    ];
    for y in [-33.0, 33.0] {
        stretcher.push(thin(rod([0.0, y, 74.0], 3.6, 200.0, "x", "steel")));
        stretcher.push(thin(rod([0.0, y * 0.8, 16.0], 3.0, 180.0, "x", "steel")));
    }
    for x in [-90.0, 0.0, 90.0] {
        stretcher.push(thin(rod([x, 0.0, 74.0], 3.0, 66.0, "y", "steel")));
    }
    for (lx, ly) in [(-90.0, -26.0), (-90.0, 26.0), (90.0, -26.0), (90.0, 26.0)] {
        stretcher.push(thin(post([lx, ly, 10.0], 4.0, 64.0, "steel")));
        stretcher.push(thin(json!({"box": [12.0, 3.0, 12.0], "at": [lx, ly, 6.0], "mat": "rubber", "shape": "torus"})));
    }
    o.push(("stretcher", json!({
        "label": "SP_stretcher", "at": [1300.0, 50.0, 0.0], "yaw": 12.0,
        "anchor": {"at": [1209.5, -5.0, 76.0], "size": [210.2, 110.1, 8.0], "center": [1300.0, 50.0, 80.0]},
        "parts": stretcher,
    })));
    let mut iv = Vec::new();
    for a in 0..5 {
        let r = (a as f64 * 72.0).to_radians();
        iv.push(thin(bxy([30.0, 3.0, 2.5], [14.0 * r.cos(), 14.0 * r.sin(), 6.0], "steel", a as f64 * 72.0)));
        iv.push(thin(json!({"sphere": 2.5, "at": [27.0 * r.cos(), 27.0 * r.sin(), 2.5], "mat": "rubber"})));
    }
    iv.push(thin(post([0.0, 0.0, 6.0], 2.6, 172.0, "steel")));
    iv.push(thin(bx([3.0, 34.0, 2.0], [0.0, 0.0, 176.0], "steel")));
    iv.push(thin(rb([5.0, 13.0, 22.0], [0.0, 14.0, 160.0], "bag")));
    iv.push(thin(json!({"cylinder": [0.35, 90.0], "at": [6.0, 18.0, 110.0], "mat": "bag", "tilt": [14.0, 0.0]})));
    o.push(("iv_stand", thing_at("IV stand", [1190.0, 105.0, 0.0], 0.0, iv)));
    o.push(("medical_cart", thing_at("Medical cart", [2150.0, -110.0, 0.0], 0.0, vec![
        rb([72.0, 46.0, 84.0], [0.0, 0.0, 50.0], "cart"),
        rb([76.0, 50.0, 3.0], [0.0, 0.0, 93.5], "steel"),
        thin(rb([60.0, 3.0, 3.0], [0.0, 25.0, 74.0], "steel")),
        thin(rb([60.0, 3.0, 3.0], [0.0, 25.0, 52.0], "steel")),
        thin(rb([60.0, 3.0, 3.0], [0.0, 25.0, 30.0], "steel")),
        thin(post([-18.0, 6.0, 95.0], 7.0, 16.0, "bag")),
        thin(post([-6.0, -8.0, 95.0], 5.0, 12.0, "glass")),
        thin(rb([22.0, 14.0, 4.0], [16.0, 4.0, 97.0], "steel")),
        thin(json!({"box": [10.0, 3.0, 10.0], "at": [-30.0, 18.0, 5.0], "mat": "rubber", "shape": "torus"})),
        thin(json!({"box": [10.0, 3.0, 10.0], "at": [30.0, 18.0, 5.0], "mat": "rubber", "shape": "torus"})),
        thin(json!({"box": [10.0, 3.0, 10.0], "at": [-30.0, -18.0, 5.0], "mat": "rubber", "shape": "torus"})),
        thin(json!({"box": [10.0, 3.0, 10.0], "at": [30.0, -18.0, 5.0], "mat": "rubber", "shape": "torus"})),
    ])));
    let mut cmess = vec![
        stain([90.0, 70.0], [1360.0, -60.0], 15.0, "pool"),
        stain([160.0, 60.0], [1680.0, 110.0], 205.0, "smear"),
        stain([110.0, 80.0], [2300.0, 40.0], 60.0, "pool"),
    ];
    for (i, (x, y)) in [(1240.0, -80.0), (1420.0, 20.0), (1470.0, -95.0), (900.0, 60.0), (2000.0, -30.0)].iter().enumerate() {
        cmess.push(thin(bxy([21.0, 29.7, 0.3], [*x, *y, 0.4], "paper", (i as f64 * 61.0 + 20.0) % 180.0)));
    }
    o.push(("corridor_litter", thing("Papers and stains in the corridor", cmess)));

    // ceiling lamps; the first still lights, the one at 1500 is failing, most are dead; the one
    // above the end of the corridor is the last that works
    let mut lamps = Vec::new();
    for x in [300.0, 600.0, 900.0, 1200.0, 1500.0, 1800.0, 2100.0, 2400.0] {
        let mat = match x as i64 {
            300 => "lamp_on",
            1500 => "lamp_flicker",
            _ => "lamp_off",
        };
        let mut f = vec![
            rb([64.0, 34.0, 5.0], [x, 0.0, H - 2.5], "frame"),
            bx([58.0, 28.0, 1.0], [x, 0.0, H - 5.4], mat),
        ];
        if x as i64 == 1200 {
            // hanging by one end
            f = vec![
                json!({"box": [64.0, 34.0, 5.0], "at": [x + 4.0, 0.0, H - 14.0], "mat": "frame", "shape": "round", "tilt": [0.0, 0.0], "yaw": 0.0}),
                thin(json!({"box": [58.0, 28.0, 1.0], "at": [x + 4.0, 0.0, H - 16.9], "mat": "lamp_off"})),
                thin(post([x - 26.0, 0.0, H - 14.0], 0.6, 14.0, "iron")),
            ];
        }
        lamps.extend(f.into_iter().map(thin));
    }
    lamps.push(thin(rb([90.0, 40.0, 5.0], [2540.0, 0.0, H - 2.5], "frame")));
    lamps.push(thin(bx([84.0, 34.0, 1.0], [2540.0, 0.0, H - 5.4], "lamp_on")));
    lamps.push(with(downlight([300.0, 0.0, H - 8.0], "#dff0ff", 70.0, 62.0), json!({"shadow": true, "halo": 90, "halo_opacity": 0.14})));
    lamps.push(with(downlight([1500.0, 0.0, H - 8.0], "#d9ffe6", 70.0, 62.0), json!({"shadow": true, "flicker": true, "halo": 90, "halo_opacity": 0.14})));
    lamps.push(with(downlight([2540.0, 0.0, H - 8.0], "#c8d8ff", 150.0, 55.0),
                    json!({"shadow": true, "shadow_size": 2048, "halo": 130, "halo_opacity": 0.22, "beam": 0.05})));
    lamps.push(point([2480.0, 0.0, 60.0], "#7f96c0", 2.0, 400.0));
    // the rooms and the lobby
    for (x, y, mat) in [(700.0, 400.0, "lamp_on"), (1300.0, -300.0, "lamp_on"), (1700.0, 400.0, "lamp_off"), (-200.0, -300.0, "lamp_off"), (-600.0, 300.0, "lamp_off")] {
        lamps.push(thin(rb([64.0, 34.0, 5.0], [x, y, H - 2.5], "frame")));
        lamps.push(thin(bx([58.0, 28.0, 1.0], [x, y, H - 5.4], mat)));
    }
    lamps.push(with(downlight([700.0, 400.0, H - 8.0], "#cdf2d9", 22.0, 65.0), json!({"halo": 70, "halo_opacity": 0.1})));
    lamps.push(with(downlight([1300.0, -300.0, H - 8.0], "#bfd9ff", 26.0, 65.0), json!({"shadow": true, "halo": 70, "halo_opacity": 0.1})));
    // the lobby pendant, warm and low
    lamps.push(thin(post([-400.0, 0.0, 225.0], 0.8, 75.0, "iron")));
    lamps.push(thin(json!({"cylinder": [28.0, 22.0], "top": 6.0, "open": true, "at": [-400.0, 0.0, 216.0], "mat": "iron"})));
    lamps.push(thin(json!({"sphere": 5.0, "at": [-400.0, 0.0, 210.0], "mat": "lamp_warm"})));
    lamps.push(with(point([-400.0, 0.0, 200.0], "#ffc28a", 26.0, 1100.0), json!({"shadow": true, "halo": 130, "halo_opacity": 0.16})));
    o.push(("ceiling_lamps", thing("Ceiling lamps", lamps)));
    o.push(("exit_sign", thing("Exit sign", vec![
        json!({"box": [40.0, 4.0, 15.0], "at": [1160.0, -137.0, 245.0], "mat": "frame", "shape": "sign", "text": "非常口  EXIT",
               "fg": "#ffe9e0", "bg": "#d0180d", "glow": 5.0, "border": false}),
        with(point([1160.0, -110.0, 236.0], "#ff1408", 0.7, 380.0), json!({"halo": 60, "halo_opacity": 0.25})),
    ])));

    // R1: a bed with someone in it reaching up, one swaying at the west wall, one cowering in
    // the far corner, one leaning out of the doorway, and the night nurse with her clipboard
    o.push(("bed_r1", thing_at("Bed in R1", [700.0, 520.0, 0.0], 0.0, bed_parts(true))));
    o.push(("bed_patient", thing("Patient in the bed", vec![patient("reach", [790.0, 520.0, 51.0], 0.0)])));
    o.push(("swaying_patient", thing("Swaying patient", vec![patient("sway", [440.0, 560.0, 0.0], 180.0)])));
    o.push(("cowering_patient", thing("Cowering patient", vec![patient("cower", [880.0, 600.0, 0.0], 90.0)])));
    o.push(("peeking_patient", thing("Peeking patient", vec![patient("peek", [748.0, 192.0, 0.0], 270.0)])));
    o.push(("night_nurse", thing("Night nurse", vec![with(figure("nurse", [700.0, 405.0, 0.0], 0.0), json!({"holds": "clipboard", "dress": "uniform"}))])));
    o.push(("curtain_r1", thing("Bed curtain in R1", vec![
        json!({"box": [160.0, 12.0, 205.0], "at": [905.0, 335.0, 190.0], "mat": "curtain", "shape": "curtain"}),
        thin(rod([900.0, 335.0, 294.0], 2.5, 190.0, "x", "steel")),
    ])));
    o.push(("bedside_r1", thing("Bedside cabinet in R1", vec![
        rb([40.0, 42.0, 74.0], [632.0, 612.0, 37.0], "paint_metal"),
        thin(post([622.0, 606.0, 74.0], 7.0, 10.0, "plastic_old")),
        thin(bxy([18.0, 24.0, 0.4], [640.0, 620.0, 74.3], "paper", 14.0)),
    ])));

    // R2: the fallen patient by a lamp on the floor, one twitching in the doorway, one at the wall
    o.push(("fallen_lamp", thing("Lamp on the floor", vec![
        thin(json!({"cylinder": [1.4, 150.0], "axis": "x", "at": [1580.0, 450.0, 3.0], "mat": "iron", "yaw": 30.0})),
        json!({"cylinder": [18.0, 26.0], "top": 9.0, "open": true, "axis": "x", "at": [1652.0, 492.0, 14.0], "mat": "iron", "yaw": 30.0}),
        thin(json!({"sphere": 5.0, "at": [1648.0, 489.0, 14.0], "mat": "lamp_warm"})),
        with(point([1665.0, 482.0, 22.0], "#ffb878", 2.2, 650.0), json!({"shadow": true, "halo": 60, "halo_opacity": 0.2})),
    ])));
    let (c, s) = (20f64.to_radians().cos(), 20f64.to_radians().sin());
    o.push(("fallen_patient", anchored("SP_fallen_patient",
        json!({"at": [1507.4, 300.9, 31.7], "size": [230.5, 177.5, 48.2], "center": [1600.0, 340.0, 24.1]}),
        vec![patient("face_down", [1600.0 - 87.5 * c, 340.0 - 87.5 * s, 0.0], 20.0)])));
    o.push(("twitching_patient", thing("Twitching patient", vec![patient("twitch", [1610.0, 215.0, 0.0], 270.0)])));
    o.push(("standing_patient", thing("Patient facing the wall", vec![patient("head_turn", [1950.0, 612.0, 0.0], 90.0)])));
    o.push(("bed_r2", thing_at("Bed in R2", [1510.0, 560.0, 0.0], 0.0, {
        let mut p = bed_parts(false);
        p.push(with(rb([190.0, 84.0, 14.0], [8.0, -14.0, 52.0], "vinyl"), json!({"yaw": 9.0})));
        p.push(json!({"box": [150.0, 6.0, 52.0], "at": [6.0, -52.0, 28.0], "mat": "sheet", "shape": "curtain"}));
        p
    })));
    o.push(("bedside_r2", thing_at("Overturned cabinet in R2", [1730.0, 590.0, 20.0], 25.0, vec![
        with(rb([40.0, 74.0, 42.0], [0.0, 0.0, 1.0], "paint_metal"), json!({"tilt": [0.0, 0.0]})),
        thin(bxy([18.0, 24.0, 0.4], [40.0, -30.0, -19.5], "paper", 40.0)),
    ])));

    // L1: one chair facing the window, a cabinet, a stripped bed
    let mut chair = vec![rb([45.0, 45.0, 4.0], [1200.0, -460.0, 44.0], "wood")];
    for (x, y) in [(-19.0, -19.0), (19.0, -19.0), (-19.0, 19.0), (19.0, 19.0)] {
        chair.push(thin(rb([3.5, 3.5, 42.0], [1200.0 + x, -460.0 + y, 21.0], "wood")));
    }
    for x in [-19.0, 19.0] {
        chair.push(thin(rb([3.5, 3.5, 52.0], [1200.0 + x, -440.0, 70.0], "wood")));
    }
    for z in [62.0, 78.0, 92.0] {
        chair.push(thin(rb([40.0, 2.5, 7.0], [1200.0, -440.0, z], "wood")));
    }
    o.push(("empty_chair", anchored("SP_empty_chair",
        json!({"at": [1177.5, -482.5, 0.0], "size": [45.0, 45.0, 46.0], "center": [1200.0, -460.0, 23.0]}), chair)));
    o.push(("cabinet", thing("Cabinet", vec![
        rb([60.0, 120.0, 180.0], [960.0, -560.0, 90.0], "paint_metal"),
        thin(bx([1.0, 1.0, 170.0], [990.5, -560.0, 90.0], "iron")),
        thin(rb([2.5, 3.0, 16.0], [991.5, -572.0, 100.0], "steel")),
        thin(rb([2.5, 3.0, 16.0], [991.5, -548.0, 100.0], "steel")),
        thin(rb([30.0, 40.0, 22.0], [955.0, -580.0, 191.0], "plastic_old")),
    ])));
    o.push(("bed_l1", thing_at("Stripped bed in L1", [1400.0, -560.0, 0.0], 0.0, bed_parts(false))));
    o.push(("curtain_l1", thing("Window curtain in L1", vec![
        json!({"box": [64.0, 10.0, 150.0], "at": [1132.0, -626.0, 160.0], "mat": "curtain", "shape": "curtain"}),
        thin(rod([1200.0, -628.0, 238.0], 2.0, 230.0, "x", "steel")),
    ])));

    // the ones that move, over one 12 s loop (every shot starts at 0 s)
    o.push(("figure", json!({
        "label": "SP_figure", "at": [2450, 0, 0],
        "anchor": {"at": [2450.0, 0.0, 0.0], "size": [61.1, 61.6, 178.0], "center": [2446.9, -0.5, 88.5]},
        "path": [[0, 2450, 0, 180], [1.5, 2450, 0, 180], [12, 1550, 0, 180]],
        "parts": [with(figure("stalk", [0.0, 0.0, 0.0], 0.0), json!({"build": "thin", "height": 186}))],
    })));
    o.push(("pacing_patient", json!({
        "label": "Pacing patient", "at": [500, 300, 0],
        "path": [[0, 500, 300, 0], [5, 920, 300, 0], [6, 920, 300, 180], [11, 500, 300, 180], [12, 500, 300, 360]],
        "parts": [patient("shamble", [0.0, 0.0, 0.0], 0.0)],
    })));
    o.push(("crawling_patient", json!({
        "label": "Crawling patient", "at": [1850, 480, 0],
        "path": [[0, 1850, 480, 233], [12, 1770, 375, 233]],
        "parts": [patient("crawl", [0.0, 0.0, 0.0], 0.0)],
    })));

    // outside: dead trees, the yard light, the wall around the grounds
    let mut trees = Vec::new();
    for (i, (x, y, h)) in [(-1300.0, 300.0, 650.0), (-1550.0, -250.0, 800.0), (-1200.0, -900.0, 550.0), (700.0, 1200.0, 700.0),
                           (1700.0, 1350.0, 600.0), (1200.0, -1250.0, 750.0), (-1450.0, 900.0, 620.0), (2300.0, -1150.0, 680.0)].iter().enumerate() {
        let r = h * 0.035;
        trees.push(json!({"cylinder": [r, *h], "top": r * 0.35, "at": [*x, *y, h / 2.0], "mat": "bark"}));
        for b in 0..5 {
            let a = (i * 97 + b * 71) as f64 % 360.0;
            let z = h * (0.45 + 0.1 * b as f64);
            let len = h * (0.35 - 0.04 * b as f64);
            let (sa, ca) = a.to_radians().sin_cos();
            trees.push(thin(json!({"cylinder": [r * 0.3, len], "top": r * 0.06, "at": [x + ca * len * 0.35, y + sa * len * 0.35, z + len * 0.35],
                                   "mat": "bark", "yaw": a, "tilt": [0.0, -(40.0 + 8.0 * b as f64)]})));
        }
    }
    o.push(("dead_trees", structure("Dead trees outside", trees)));
    o.push(("yard", structure("The grounds", vec![
        bx([40.0, 4200.0, 240.0], [-2100.0, 0.0, 120.0], "concrete"),
        bx([5400.0, 40.0, 240.0], [600.0, 2100.0, 120.0], "concrete"),
        bx([5400.0, 40.0, 240.0], [600.0, -2100.0, 120.0], "concrete"),
        bx([40.0, 4200.0, 240.0], [3300.0, 0.0, 120.0], "concrete"),
        post([-1150.0, 0.0, 0.0], 12.0, 460.0, "iron"),
        thin(rod([-1120.0, 0.0, 455.0], 6.0, 60.0, "x", "iron")),
        thin(rb([40.0, 22.0, 10.0], [-1090.0, 0.0, 450.0], "iron")),
        thin(bx([34.0, 16.0, 1.0], [-1090.0, 0.0, 444.5], "lamp_on")),
        with(json!({"light": "spot", "at": [-1090.0, 0.0, 440.0], "toward": [-900.0, 0.0, 0.0], "color": "#9db4ff", "intensity": 140.0,
                    "range": 1600, "cone": 50, "penumbra": 0.7}), json!({"halo": 200, "halo_opacity": 0.25, "beam": 0.03})),
    ])));

    // moonlight through each barred window, seen in the haze
    let mut moon = Vec::new();
    for (i, (at, toward)) in [([-1050.0, 350.0, 330.0], [-600.0, 350.0, 0.0]), ([-1050.0, -350.0, 330.0], [-600.0, -350.0, 0.0]),
                              ([-350.0, -900.0, 330.0], [-350.0, -450.0, 0.0]), ([700.0, 900.0, 330.0], [700.0, 450.0, 0.0]),
                              ([1700.0, 900.0, 330.0], [1700.0, 450.0, 0.0]), ([1200.0, -900.0, 330.0], [1200.0, -450.0, 0.0])].iter().enumerate() {
        let mut l = json!({"light": "spot", "at": at, "toward": toward, "color": "#8caeff", "intensity": 320, "range": 1200,
                           "cone": 18, "penumbra": 0.6, "beam": 0.045});
        if matches!(i, 1 | 4 | 5) {
            l["shadow"] = json!(true);
        }
        moon.push(l);
    }
    o.push(("moonlight", structure("Moonlight", moon)));

    // dust in the air of each room
    o.push(("dust", structure("Dust in the air", vec![
        json!({"dust": [2600, 280, 260], "at": [1300, 0, 140], "count": 900}),
        json!({"dust": [780, 1280, 260], "at": [-400, 0, 140], "count": 700}),
        json!({"dust": [580, 480, 260], "at": [700, 400, 140], "count": 260}),
        json!({"dust": [580, 480, 260], "at": [1700, 400, 140], "count": 260}),
        json!({"dust": [580, 480, 260], "at": [1200, -400, 140], "count": 260}),
    ])));
    o
}

/// What a shot can make appear (`props:` with `asset: props/<name>`).
fn props() -> Vec<(&'static str, Value)> {
    vec![("stranger", json!({"label": "A stranger", "parts": [patient("idle", [0.0, 0.0, 0.0], 0.0)]}))]
}

/// Write the sample game into `dir` (scene.json, objects/, props/). A folder that already has a
/// scene.json is someone's game: left as it is (returns false).
pub fn write(dir: &Path) -> Result<bool> {
    if dir.join(SCENE_FILE).exists() {
        return Ok(false);
    }
    let put = |p: PathBuf, v: &Value| -> Result<()> {
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(v)? + "\n")?;
        Ok(())
    };
    for (name, v) in objects() {
        put(dir.join(OBJECTS_DIR).join(format!("{name}.json")), &v)?;
    }
    for (name, v) in props() {
        put(dir.join(PROPS_DIR).join(format!("{name}.json")), &v)?;
    }
    put(dir.join(SCENE_FILE), &scene())?;
    Ok(true)
}

