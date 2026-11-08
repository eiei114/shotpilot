//! The three engine's sample game (D-72, D-101): KettleHill, a village on a green hill on a
//! clear afternoon. Five places to film, each with its own colour and outline so a camera that
//! points at one is plain to see: the white windmill with its turning sails, the market stall
//! under a red and cream awning, the teahouse with its dark roof and red lanterns, the old oak
//! that children run around, and the vegetable rows with the sunflowers. Houses, a well, the
//! paths between them and the hills far off fill the rest; villagers pour tea, wave, water,
//! sweep and walk.
//!
//! One file per thing (`objects/<name>.json`): repaint the stall's awning and only the shots that
//! showed it go stale. Written once by `shotpilot init --engine three`; after that it is the game,
//! and people edit it by hand.
//!
//! Every part's `box` (or cylinder, sphere, figure) is also the shape shotpilot uses to decide
//! what a shot saw. Keys only the page reads refine how it looks: `shape` (round, curtain, torus,
//! sign, decal, sails), `tilt`, `sides`, `dress`, `holds`, `build`, `halo`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::error::Result;
use crate::three::{OBJECTS_DIR, PROPS_DIR, SCENE_FILE};

type V3 = [f64; 3];

const FH: f64 = 45.0; // the teahouse floor
const EAVE: f64 = 280.0; // where the teahouse roof starts
const SQRT2: f64 = std::f64::consts::SQRT_2;

/// The windmill's sails face the square: the way from the mill to the well.
const MILL: (f64, f64) = (2400.0, -1800.0);
const MILL_YAW: f64 = 143.130_102_354_155_98; // atan2(1800, -2400)

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

/// A cone standing on `base`.
fn cone(base: V3, radius: f64, height: f64, mat: &str) -> Value {
    json!({"cylinder": [radius, height], "top": 0.0, "at": [base[0], base[1], base[2] + height / 2.0], "mat": mat})
}

/// A cylinder lying along X (or Y) centred on `at`.
fn rod(at: V3, diameter: f64, length: f64, axis: &str, mat: &str) -> Value {
    json!({"cylinder": [diameter / 2.0, length], "axis": axis, "at": at, "mat": mat})
}

fn ball(r: f64, at: V3, mat: &str) -> Value {
    json!({"sphere": r, "at": at, "mat": mat})
}

/// A ball in a colour of its own.
fn ball_c(r: f64, at: V3, mat: &str, color: &str) -> Value {
    json!({"sphere": r, "at": at, "mat": mat, "color": color})
}

/// A sign or board: text on its two broad faces.
fn sign(size: V3, at: V3, text: &str, fg: &str, bg: &str) -> Value {
    json!({"box": size, "at": at, "mat": "timber", "shape": "sign", "text": text, "fg": fg, "bg": bg})
}

/// A square hipped roof: its eaves `half` from the middle on every side, `height` tall, standing on
/// `eave`. A four-sided cylinder turned by 45 degrees. Never hides what is behind it: its box is
/// far larger than the roof.
fn roof(at: (f64, f64), eave: f64, half: f64, top: f64, height: f64, mat: &str) -> Value {
    json!({"cylinder": [half * SQRT2, height], "top": top * SQRT2, "sides": 4, "yaw": 45.0,
           "at": [at.0, at.1, eave + height / 2.0], "mat": mat, "solid": false})
}

/// A villager: a figure in clothes of `dress`, its skirt or tunic `len` long.
fn villager(mv: &str, feet: V3, facing: f64, dress: &str, len: f64, height: f64) -> Value {
    json!({"figure": mv, "at": feet, "facing": facing, "height": height, "dress": dress, "dress_len": len})
}

fn point(at: V3, color: &str, intensity: f64, range: f64) -> Value {
    json!({"light": "point", "at": at, "color": color, "intensity": intensity, "range": range})
}

fn yaw_of(a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.1 - a.1).atan2(b.0 - a.0).to_degrees()
}

/// A path of trodden earth from a to b, `width` wide.
fn path(a: (f64, f64), b: (f64, f64), width: f64) -> Value {
    let len = (b.0 - a.0).hypot(b.1 - a.1);
    thin(bxy([len, width, 1.2], [(a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0, 0.6], "path", yaw_of(a, b)))
}

/// `at` moved `d` along the way `yaw` (degrees) points.
fn ahead(at: (f64, f64), yaw: f64, d: f64) -> (f64, f64) {
    let (s, c) = yaw.to_radians().sin_cos();
    (at.0 + c * d, at.1 + s * d)
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
        "name": "KettleHill",
        "background": "#cfe1ef",
        "exposure": 1.0,
        "fog": {"color": "#c9dbea", "density": 0.0045, "ground": 0.0, "height": 6.0},
        "sky": {"top": "#4a8bd6", "horizon": "#d4e5f2", "sun": "#fff4d8", "sun_size": 1.6, "glow": 0.8, "stars": 0},
        "post": {"bloom": 0.2, "threshold": 2.4, "ao": 0.7, "ao_radius": 0.6, "vignette": 0.22, "grain": 0.01, "aberration": 0.0006, "dof": 1.0},
        "ambient": {"sky": "#b9d3ef", "ground": "#7a8a4c", "intensity": 1.0},
        "sun": {"from": [-0.56, -0.5, 0.66], "color": "#fff0d6", "intensity": 3.0, "shadow": true, "center": [0, 0, 0], "reach": 3600},
        "materials": materials(),
    })
}

/// The materials, by name: `tex` names the surface the page bakes from noise.
fn materials() -> Value {
    let mut m = json!({
        "grass": {"tex": "grass", "color": "#86b44c", "color2": "#5f8f36", "color3": "#b4c763", "scale": [3, 3]},
        "meadow": {"tex": "grass", "color": "#8fbb58", "color2": "#6c9b42", "color3": "#b9c975", "scale": [9, 9], "wear": 0.2},
        "path": {"tex": "concrete", "color": "#c2a77e", "color2": "#9a7c55", "scale": [2.5, 2.5], "wear": 0.5},
        "stone": {"tex": "concrete", "color": "#cfc8b8", "color2": "#93a06e", "scale": [1.6, 1.6], "wear": 0.35},
        "soil": {"tex": "concrete", "color": "#7a5636", "color2": "#5c3f27", "scale": [1.5, 1.5], "wear": 0.6},
        "plaster_white": {"tex": "plaster", "color": "#f1ece0", "color2": "#a3ad8e", "scale": [4, 3], "wear": 0.2},
        "plaster_cream": {"tex": "plaster", "color": "#ecdcbb", "color2": "#a3ad8e", "scale": [4, 3], "wear": 0.2},
        "plaster_pink": {"tex": "plaster", "color": "#efd2c4", "color2": "#a3ad8e", "scale": [4, 3], "wear": 0.2},
        "timber": {"tex": "wood", "color": "#6e4a2c", "color2": "#8c6340", "scale": [1.2, 1.2], "wear": 0.4},
        "plank": {"tex": "wood", "color": "#a87d50", "color2": "#c79a68", "scale": [1.2, 1.2], "wear": 0.5},
        "crate": {"tex": "wood", "color": "#b98f5c", "color2": "#d2a874", "scale": [0.8, 0.8], "wear": 0.6},
        "bark": {"tex": "wood", "color": "#5b4330", "color2": "#715640", "scale": [1.5, 0.6], "wear": 0.8},
        "roof_orange": {"tex": "shingle", "color": "#d86b3c", "color2": "#b24f2c", "n": 10, "scale": [1, 1.4]},
        "roof_red": {"tex": "shingle", "color": "#c4493b", "color2": "#9c3a30", "n": 10, "scale": [1, 1.4]},
        "roof_blue": {"tex": "shingle", "color": "#4f73a3", "color2": "#3b5a84", "n": 10, "scale": [1, 1.4]},
        "roof_green": {"tex": "shingle", "color": "#5f8f5e", "color2": "#466f48", "n": 10, "scale": [1, 1.4]},
        "kawara": {"tex": "shingle", "color": "#55606c", "color2": "#3d4651", "n": 12, "scale": [1, 1.4]},
        "awning_red": {"tex": "fabric", "color": "#d9433b", "n": 60, "scale": [1, 1], "wear": 0.2, "double": true},
        "awning_cream": {"tex": "fabric", "color": "#f3ebd8", "n": 60, "scale": [1, 1], "wear": 0.2, "double": true},
        "sail": {"tex": "fabric", "color": "#f2e9d4", "n": 50, "scale": [1, 1], "wear": 0.3, "double": true},
        "noren": {"tex": "fabric", "color": "#2f5486", "n": 60, "scale": [1, 1], "wear": 0.1, "double": true},
        "tatami": {"tex": "fabric", "color": "#c9c08a", "n": 140, "scale": [1, 1], "wear": 0.2},
        "figure": {"tex": "plastic", "color": "#efdcc2", "scale": [0.5, 0.5], "wear": 0.2},
        "cloth_indigo": {"tex": "fabric", "color": "#34508a", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_rose": {"tex": "fabric", "color": "#df8e93", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_yellow": {"tex": "fabric", "color": "#ecc04e", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_blue": {"tex": "fabric", "color": "#5d8dd0", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_green": {"tex": "fabric", "color": "#69a85b", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_red": {"tex": "fabric", "color": "#e0574a", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_teal": {"tex": "fabric", "color": "#3fa7a0", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_brown": {"tex": "fabric", "color": "#9a6b45", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "cloth_plum": {"tex": "fabric", "color": "#8d64a8", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true},
        "apron_green": {"tex": "fabric", "color": "#4f9a6a", "n": 80, "scale": [0.8, 0.8], "wear": 0.1, "double": true}
    });
    if let (Some(o), Value::Object(more)) = (m.as_object_mut(), json!({
        "leaves": {"color": "#5d9a3a", "rough": 0.85},
        "pine": {"color": "#3d6d3c", "rough": 0.9},
        "bush": {"color": "#4f8f3a", "rough": 0.9},
        "stem": {"color": "#5e8f34", "rough": 0.8},
        "pumpkin": {"color": "#ec8a2c", "rough": 0.55},
        "apple": {"color": "#cf3a2e", "rough": 0.35},
        "cabbage": {"color": "#9ccb5c", "rough": 0.6},
        "petal": {"color": "#f6c824", "rough": 0.7},
        "seed": {"color": "#5a3b20", "rough": 0.9},
        "sack": {"color": "#d8c28f", "rough": 0.95},
        "straw": {"color": "#e3c66e", "rough": 0.9},
        "window": {"color": "#33475a", "rough": 0.15, "metal": 0.1},
        "door_wood": {"color": "#7a4b2b", "rough": 0.6},
        "water": {"color": "#2c5466", "rough": 0.05, "metal": 0.2},
        "tin": {"color": "#9aa6ad", "rough": 0.35, "metal": 0.8},
        "teapot": {"color": "#3c5c4c", "rough": 0.3},
        "cup": {"color": "#f1eadc", "rough": 0.3},
        "basket": {"color": "#b8894f", "rough": 0.85},
        "cushion_red": {"color": "#b8433c", "rough": 0.9},
        "lantern_red": {"color": "#e8523e", "glow": 1.4},
        "lantern_paper": {"color": "#f6dfae", "glow": 1.6},
        "flower_pink": {"color": "#f08ab0", "rough": 0.7},
        "flower_white": {"color": "#f6f2ea", "rough": 0.7},
        "flower_violet": {"color": "#9a78d0", "rough": 0.7}
    })) {
        o.extend(more);
    }
    m
}

/// A cottage: plastered walls on a stone footing, a door on the side `door_yaw` faces, windows,
/// a hipped roof and a chimney, built around (x, y).
fn cottage(at: (f64, f64), size: (f64, f64), wall: &str, roof_mat: &str, door_yaw: f64) -> Vec<Value> {
    let (x, y) = at;
    let (w, d) = size;
    let h = 300.0;
    let mut p = vec![
        bx([w, d, h], [x, y, h / 2.0], wall),
        bx([w + 10.0, d + 10.0, 34.0], [x, y, 17.0], "stone"),
    ];
    // timber corners and a beam under the eaves
    for (cx, cy) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        p.push(thin(bx([14.0, 14.0, h], [x + cx * (w / 2.0 + 2.0), y + cy * (d / 2.0 + 2.0), h / 2.0], "timber")));
    }
    p.push(thin(bx([w + 8.0, d + 8.0, 14.0], [x, y, h - 7.0], "timber")));
    // the door and two windows on the door side, one window on each other side
    let half = |yaw: f64| if (yaw.to_radians().cos()).abs() > 0.5 { w / 2.0 } else { d / 2.0 };
    for side in [0.0, 90.0, 180.0, 270.0] {
        let yaw = door_yaw + side;
        let n = half(yaw) + 3.0;
        let (fx, fy) = ahead((x, y), yaw, n);
        let (sx, sy) = (-(yaw.to_radians().sin()), yaw.to_radians().cos()); // along the wall
        let span = if (yaw.to_radians().cos()).abs() > 0.5 { d } else { w };
        if side == 0.0 {
            p.push(bxy([8.0, 96.0, 190.0], [fx, fy, 34.0 + 95.0], "door_wood", yaw));
            p.push(thin(bxy([10.0, 112.0, 10.0], [fx, fy, 34.0 + 195.0], "timber", yaw)));
            for u in [-0.3, 0.3] {
                let (wx, wy) = (fx + sx * span * u, fy + sy * span * u);
                p.push(bxy([6.0, 70.0, 80.0], [wx, wy, 185.0], "window", yaw));
                p.push(thin(bxy([10.0, 82.0, 8.0], [wx, wy, 141.0], "timber", yaw)));
                // a box of flowers under the window
                p.push(thin(bxy([22.0, 78.0, 16.0], [wx + (yaw.to_radians().cos()) * 10.0, wy + (yaw.to_radians().sin()) * 10.0, 130.0], "crate", yaw)));
                for k in 0..5 {
                    let v = (k as f64 - 2.0) * 15.0;
                    let mat = ["flower_pink", "flower_white", "flower_violet"][k % 3];
                    p.push(thin(ball(7.0, [wx + (yaw.to_radians().cos()) * 12.0 + sx * v, wy + (yaw.to_radians().sin()) * 12.0 + sy * v, 143.0], mat)));
                }
            }
        } else if side != 180.0 {
            p.push(bxy([6.0, 70.0, 80.0], [fx, fy, 185.0], "window", yaw));
            p.push(thin(bxy([10.0, 82.0, 8.0], [fx, fy, 141.0], "timber", yaw)));
        }
    }
    p.push(roof(at, h, w.max(d) / 2.0 + 55.0, 18.0, 230.0, roof_mat));
    p.push(rb([42.0, 42.0, 170.0], [x + w * 0.22, y - d * 0.18, h + 130.0], "stone"));
    p
}

/// A round-headed tree: a trunk and two or three balls of leaves.
fn round_tree(at: (f64, f64), scale: f64, tone: usize) -> Vec<Value> {
    let (x, y) = at;
    let greens = ["#5d9a3a", "#6aa646", "#4f8b35", "#77b04e"];
    let g = |i: usize| greens[(tone + i) % greens.len()];
    vec![
        post([x, y, 0.0], 34.0 * scale, 240.0 * scale, "bark"),
        ball_c(135.0 * scale, [x, y, 320.0 * scale], "leaves", g(0)),
        ball_c(100.0 * scale, [x + 45.0 * scale, y - 30.0 * scale, 405.0 * scale], "leaves", g(1)),
        ball_c(95.0 * scale, [x - 55.0 * scale, y + 40.0 * scale, 375.0 * scale], "leaves", g(2)),
    ]
}

fn pine(at: (f64, f64), scale: f64) -> Vec<Value> {
    let (x, y) = at;
    vec![
        post([x, y, 0.0], 26.0 * scale, 130.0 * scale, "bark"),
        cone([x, y, 110.0 * scale], 125.0 * scale, 230.0 * scale, "pine"),
        cone([x, y, 230.0 * scale], 98.0 * scale, 190.0 * scale, "pine"),
        cone([x, y, 340.0 * scale], 68.0 * scale, 150.0 * scale, "pine"),
    ]
}

/// Keys for a figure that runs round (cx, cy) at `r`, starting at `from` degrees, once per `secs`.
fn round_path(c: (f64, f64), r: f64, from: f64, secs: f64) -> Value {
    let keys: Vec<Value> = (0..=8)
        .map(|j| {
            let a = from + 45.0 * j as f64;
            let (s, co) = a.to_radians().sin_cos();
            json!([secs * j as f64 / 8.0, (c.0 + r * co).round(), (c.1 + r * s).round(), a + 90.0])
        })
        .collect();
    Value::Array(keys)
}

/// Every object of the village, by file name.
fn objects() -> Vec<(&'static str, Value)> {
    let mut o: Vec<(&str, Value)> = Vec::new();

    o.push(("ground", structure("The ground", vec![bx([30000.0, 30000.0, 20.0], [0.0, 0.0, -10.0], "grass")])));

    // far hills all round, in the haze
    let mut hills = Vec::new();
    // each stands clear of the village: its near foot more than 45 m from the well
    for (x, y, r, sx, sz) in [(10500.0, -3000.0, 5200.0, 1.2, 0.2), (-2000.0, 10800.0, 5600.0, 1.0, 0.18), (-10600.0, -1500.0, 5000.0, 1.0, 0.22),
                              (2000.0, -10600.0, 5200.0, 1.1, 0.2), (8600.0, 7800.0, 4800.0, 1.0, 0.24), (-8200.0, 7600.0, 4600.0, 1.1, 0.2),
                              (-7400.0, -8200.0, 5000.0, 1.0, 0.19)] {
        hills.push(json!({"sphere": r, "scale": [sx, 1.0, sz], "at": [x, y, 0.0], "mat": "meadow", "solid": false}));
    }
    o.push(("hills", structure("Hills far off", hills)));

    // the square with the well, and the paths out to each place
    let square = (0.0, 0.0);
    o.push(("paths", structure("The square and the paths", vec![
        json!({"cylinder": [520.0, 1.6], "sides": 48, "at": [0.0, 0.0, 0.8], "mat": "stone", "solid": false}),
        path(square, (-500.0, 690.0), 150.0),
        path(square, (760.0, -290.0), 150.0),
        path(square, ahead(MILL, MILL_YAW, 240.0), 140.0),
        path(square, (-1160.0, -1005.0), 140.0),
        path(square, (-1740.0, 600.0), 140.0),
        path((-500.0, 690.0), (-500.0, 640.0), 190.0),
    ])));
    let mut well = vec![
        post([0.0, 0.0, 0.0], 150.0, 70.0, "stone"),
        thin(post([0.0, 0.0, 70.0], 162.0, 8.0, "stone")),
        thin(post([0.0, 0.0, 69.0], 128.0, 2.0, "water")),
        rod([0.0, 0.0, 212.0], 9.0, 170.0, "y", "timber"),
        thin(post([22.0, 0.0, 120.0], 1.0, 90.0, "timber")),
        thin(json!({"cylinder": [13.0, 20.0], "top": 15.0, "at": [22.0, 0.0, 112.0], "mat": "crate"})),
        roof(square, 228.0, 108.0, 8.0, 58.0, "roof_blue"),
    ];
    for y in [-76.0, 76.0] {
        well.push(thin(post([0.0, y, 70.0], 11.0, 160.0, "timber")));
    }
    o.push(("well", thing("The well in the square", well)));

    // ------------------------------------------------------------------ the windmill
    let (mx, my) = MILL;
    let door = ahead(MILL, MILL_YAW, 190.0);
    let hub = ahead(MILL, MILL_YAW, 232.0);
    let axle = ahead(MILL, MILL_YAW, 185.0);
    let w1 = ahead(MILL, MILL_YAW, 172.0);
    let w2 = ahead(MILL, MILL_YAW, 161.0);
    o.push(("windmill", anchored("SP_windmill",
        json!({"at": [mx, my, 0.0], "size": [700.0, 700.0, 900.0], "center": [2330.0, -1750.0, 450.0]}),
        vec![
            post([mx, my, 0.0], 430.0, 30.0, "stone"),
            json!({"cylinder": [190.0, 610.0], "top": 145.0, "at": [mx, my, 335.0], "mat": "plaster_white"}),
            thin(post([mx, my, 633.0], 306.0, 14.0, "timber")),
            json!({"cylinder": [208.0, 220.0], "top": 0.0, "at": [mx, my, 750.0], "mat": "roof_orange"}),
            bxy([12.0, 92.0, 160.0], [door.0, door.1, 110.0], "door_wood", MILL_YAW),
            thin(bxy([14.0, 104.0, 10.0], [door.0, door.1, 195.0], "timber", MILL_YAW)),
            bxy([10.0, 42.0, 56.0], [w1.0, w1.1, 340.0], "window", MILL_YAW),
            bxy([10.0, 40.0, 52.0], [w2.0, w2.1, 490.0], "window", MILL_YAW),
            thin(bxy([100.0, 18.0, 18.0], [axle.0, axle.1, 650.0], "timber", MILL_YAW)),
            json!({"box": [14.0, 960.0, 960.0], "at": [hub.0, hub.1, 650.0], "yaw": MILL_YAW, "shape": "sails", "blades": 4,
                   "spin": 24.0, "mat": "sail", "frame_mat": "timber", "solid": false}),
        ])));
    let sweep_at = ahead(MILL, MILL_YAW + 28.0, 330.0);
    o.push(("miller", thing("The miller sweeping by the door", vec![with(
        villager("sweep", [sweep_at.0, sweep_at.1, 0.0], MILL_YAW + 200.0, "cloth_green", 0.86, 172.0),
        json!({"holds": "broom"}),
    )])));
    o.push(("sacks", thing("Flour sacks by the mill", {
        let s = ahead(MILL, MILL_YAW - 35.0, 245.0);
        vec![
            json!({"sphere": 32.0, "scale": [1.0, 0.8, 1.1], "at": [s.0, s.1, 32.0], "mat": "sack"}),
            json!({"sphere": 30.0, "scale": [0.9, 0.8, 1.1], "at": [s.0 + 40.0, s.1 + 35.0, 30.0], "mat": "sack"}),
            json!({"sphere": 28.0, "scale": [1.0, 0.8, 1.0], "at": [s.0 + 18.0, s.1 + 12.0, 78.0], "mat": "sack"}),
        ]
    })));

    // ------------------------------------------------------------------ the market stall
    let mut stall = Vec::new();
    // the awning: red and cream stripes, high at the back, a fringe hanging at the front
    for i in 0..8 {
        let y = -510.0 + 52.5 * (i as f64 + 0.5);
        let mat = if i % 2 == 0 { "awning_red" } else { "awning_cream" };
        stall.push(thin(json!({"box": [236.0, 52.5, 3.0], "at": [915.0, y, 252.0], "mat": mat, "tilt": [16.0, 0.0]})));
        stall.push(thin(bx([3.0, 52.5, 22.0], [801.0, y, 208.0], mat)));
    }
    for (x, y, h) in [(812.0, -505.0, 220.0), (812.0, -95.0, 220.0), (1015.0, -505.0, 312.0), (1015.0, -95.0, 312.0)] {
        stall.push(thin(post([x, y, 0.0], 10.0, h, "timber")));
    }
    stall.push(rb([110.0, 400.0, 92.0], [872.0, -300.0, 46.0], "plank"));
    stall.push(rb([126.0, 412.0, 5.0], [870.0, -300.0, 94.5], "timber"));
    stall.push(sign([3.0, 200.0, 40.0], [1018.0, -300.0, 292.0], "やさい  VEGETABLES", "#fff6e4", "#3f7a4a"));
    for (y, kind) in [(-440.0, 0), (-345.0, 1), (-250.0, 2), (-155.0, 3)] {
        stall.push(rb([62.0, 82.0, 14.0], [852.0, y, 104.0], "crate"));
        match kind {
            0 => {
                for (dx, dy) in [(-12.0, -22.0), (10.0, 4.0), (-8.0, 26.0)] {
                    stall.push(json!({"sphere": 15.0, "scale": [1.0, 1.0, 0.78], "at": [852.0 + dx, y + dy, 122.0], "mat": "pumpkin"}));
                }
            }
            1 => {
                for k in 0..9 {
                    let (dx, dy) = ((k % 3) as f64 * 16.0 - 16.0, (k / 3) as f64 * 22.0 - 22.0);
                    stall.push(thin(ball(6.0, [852.0 + dx, y + dy, 117.0], "apple")));
                }
            }
            2 => {
                for (dx, dy) in [(-12.0, -20.0), (12.0, -18.0), (-10.0, 18.0), (13.0, 20.0)] {
                    stall.push(thin(ball(12.0, [852.0 + dx, y + dy, 120.0], "cabbage")));
                }
            }
            _ => {
                for k in 0..6 {
                    let dy = (k as f64 - 2.5) * 12.0;
                    stall.push(thin(json!({"cylinder": [3.2, 30.0], "top": 0.6, "axis": "x", "at": [852.0, y + dy, 115.0], "mat": "pumpkin"})));
                }
            }
        }
    }
    stall.push(rb([50.0, 40.0, 32.0], [785.0, -470.0, 16.0], "crate"));
    stall.push(json!({"sphere": 17.0, "scale": [1.0, 1.0, 0.78], "at": [785.0, -470.0, 44.0], "mat": "pumpkin"}));
    stall.push(rb([50.0, 40.0, 32.0], [785.0, -130.0, 16.0], "crate"));
    stall.push(rb([46.0, 36.0, 28.0], [787.0, -132.0, 46.0], "crate"));
    stall.push(json!({"sphere": 25.0, "scale": [1.0, 1.0, 0.75], "at": [770.0, -360.0, 18.0], "mat": "pumpkin"}));
    stall.push(json!({"sphere": 21.0, "scale": [1.0, 1.0, 0.75], "at": [760.0, -255.0, 15.0], "mat": "pumpkin"}));
    o.push(("market_stall", anchored("SP_market_stall",
        json!({"at": [900.0, -300.0, 0.0], "size": [300.0, 440.0, 280.0], "center": [910.0, -300.0, 140.0]}), stall)));
    o.push(("stall_keeper", thing("The stall keeper waving", vec![villager("wave", [985.0, -300.0, 0.0], 180.0, "apron_green", 0.86, 168.0)])));
    o.push(("barrels", thing("Barrels by the stall", vec![
        json!({"cylinder": [28.0, 72.0], "top": 25.0, "at": [790.0, -40.0, 36.0], "mat": "crate"}),
        json!({"cylinder": [28.0, 72.0], "top": 25.0, "at": [845.0, -8.0, 36.0], "mat": "crate"}),
        thin(json!({"sphere": 6.0, "at": [790.0, -40.0, 76.0], "mat": "apple"})),
    ])));
    o.push(("shopper", json!({
        "label": "A shopper with a basket", "at": [260, -60, 0],
        "path": [[0, 260, -60, 337.8], [5, 690, -236, 337.8], [6, 690, -236, 157.8], [11, 260, -60, 157.8], [12, 260, -60, 337.8]],
        "parts": [with(villager("walk", [0.0, 0.0, 0.0], 0.0, "cloth_yellow", 0.9, 164.0), json!({"holds": "basket"}))],
    })));

    // ------------------------------------------------------------------ the teahouse
    let mid = (FH + EAVE) / 2.0;
    let wall_h = EAVE - FH;
    let mut tea = vec![
        rb([700.0, 850.0, FH], [-500.0, 1125.0, FH / 2.0], "plank"),
        rb([190.0, 64.0, 22.0], [-500.0, 668.0, 11.0], "stone"),
        thin(bx([660.0, 660.0, 1.0], [-500.0, 1200.0, FH + 0.5], "tatami")),
        bx([700.0, 18.0, wall_h], [-500.0, 1541.0, mid], "plaster_cream"),
        bx([18.0, 690.0, wall_h], [-841.0, 1195.0, mid], "plaster_cream"),
        bx([18.0, 690.0, wall_h], [-159.0, 1195.0, mid], "plaster_cream"),
        bx([150.0, 14.0, wall_h], [-775.0, 857.0, mid], "plaster_cream"),
        bx([150.0, 14.0, wall_h], [-225.0, 857.0, mid], "plaster_cream"),
        thin(bx([700.0, 16.0, 18.0], [-500.0, 857.0, EAVE - 9.0], "timber")),
        thin(bx([700.0, 14.0, 16.0], [-500.0, 707.0, EAVE - 8.0], "timber")),
        json!({"box": [190.0, 6.0, 72.0], "at": [-500.0, 849.0, EAVE - 20.0 - 36.0], "mat": "noren", "shape": "curtain", "solid": false}),
        sign([84.0, 3.0, 38.0], [-500.0, 704.0, EAVE - 34.0], "茶  TEA", "#f6efe0", "#3b2a1e"),
        sign([56.0, 3.0, 110.0], [-500.0, 1530.0, 175.0], "一\n期\n一\n会", "#2a2420", "#efe6d0"),
        rb([116.0, 72.0, 6.0], [-500.0, 1260.0, FH + 30.0], "timber"),
        rb([48.0, 48.0, 8.0], [-595.0, 1260.0, FH + 4.0], "cushion_red"),
        rb([48.0, 48.0, 8.0], [-405.0, 1260.0, FH + 4.0], "cushion_red"),
        rb([56.0, 40.0, 4.0], [-548.0, 748.0, FH + 68.0], "timber"),
        thin(post([-560.0, 742.0, FH + 70.0], 8.0, 7.0, "cup")),
        thin(post([-536.0, 754.0, FH + 70.0], 8.0, 7.0, "cup")),
        roof((-500.0, 1125.0), EAVE, 495.0, 50.0, 250.0, "kawara"),
        rb([86.0, 86.0, 22.0], [-500.0, 1125.0, EAVE + 250.0 + 11.0], "kawara"),
    ];
    for (x, y) in [(-843.0, 707.0), (-157.0, 707.0), (-843.0, 857.0), (-157.0, 857.0), (-843.0, 1543.0), (-157.0, 1543.0)] {
        tea.push(thin(bx([14.0, 14.0, wall_h], [x, y, mid], "timber")));
    }
    for (dx, dy) in [(-50.0, -30.0), (50.0, -30.0), (-50.0, 30.0), (50.0, 30.0)] {
        tea.push(thin(bx([4.0, 4.0, 26.0], [-500.0 + dx, 1260.0 + dy, FH + 14.0], "timber")));
    }
    // the little stand the cups wait on, by the guest
    for (dx, dy) in [(-24.0, -16.0), (24.0, -16.0), (-24.0, 16.0), (24.0, 16.0)] {
        tea.push(thin(bx([4.0, 4.0, 66.0], [-548.0 + dx, 748.0 + dy, FH + 33.0], "timber")));
    }
    o.push(("teahouse", anchored("SP_teahouse",
        json!({"at": [-500.0, 1125.0, 0.0], "size": [760.0, 900.0, 540.0], "center": [-500.0, 1125.0, 270.0]}), tea)));
    // red lanterns at the front corners of the eaves, an andon inside; their light is a thing of
    // its own, as a lamp and its light are two actors in UE
    let mut lanterns = Vec::new();
    let mut light = Vec::new();
    for x in [-805.0, -195.0] {
        lanterns.push(thin(post([x, 712.0, 262.0], 1.0, 18.0, "timber")));
        lanterns.push(thin(post([x, 712.0, 256.0], 14.0, 5.0, "timber")));
        lanterns.push(json!({"sphere": 17.0, "scale": [1.0, 1.0, 1.35], "at": [x, 712.0, 234.0], "mat": "lantern_red"}));
        lanterns.push(thin(post([x, 712.0, 209.0], 14.0, 4.0, "timber")));
        light.push(with(point([x, 712.0, 234.0], "#ffb070", 2.6, 420.0), json!({"halo": 40, "halo_opacity": 0.1})));
    }
    lanterns.push(rb([26.0, 26.0, 48.0], [-760.0, 1460.0, FH + 24.0], "lantern_paper"));
    light.push(with(point([-760.0, 1460.0, FH + 34.0], "#ffd29a", 3.0, 520.0), json!({"shadow": true})));
    o.push(("lanterns", thing("Lanterns of the teahouse", lanterns)));
    o.push(("lamplight", thing("Lamplight of the teahouse", light)));
    o.push(("stone_lantern", thing("Stone lantern by the steps", vec![
        rb([40.0, 40.0, 12.0], [-690.0, 630.0, 6.0], "stone"),
        post([-690.0, 630.0, 12.0], 18.0, 70.0, "stone"),
        rb([44.0, 44.0, 34.0], [-690.0, 630.0, 99.0], "stone"),
        json!({"cylinder": [36.0, 22.0], "top": 6.0, "sides": 4, "yaw": 45.0, "at": [-690.0, 630.0, 127.0], "mat": "stone"}),
    ])));
    o.push(("tea_master", thing("The tea master pouring", vec![with(
        villager("pour", [-470.0, 792.0, FH], 210.0, "cloth_indigo", 0.86, 170.0),
        json!({"holds": "teapot"}),
    )])));
    o.push(("tea_guest", thing("A guest on the veranda", vec![villager("sit", [-640.0, 724.0, 0.0], 270.0, "cloth_rose", 0.75, 162.0)])));

    // ------------------------------------------------------------------ the old oak
    let (ox, oy) = (-1500.0, -1300.0);
    let mut oak = vec![
        json!({"cylinder": [88.0, 50.0], "top": 60.0, "at": [ox, oy, 25.0], "mat": "bark"}),
        json!({"cylinder": [60.0, 330.0], "top": 40.0, "at": [ox, oy, 215.0], "mat": "bark"}),
    ];
    for (i, a) in [20.0f64, 95.0, 170.0, 245.0, 315.0].iter().enumerate() {
        let len = 230.0 - 18.0 * i as f64;
        let (s, c) = a.to_radians().sin_cos();
        oak.push(thin(json!({"cylinder": [17.0, len], "top": 8.0, "at": [ox + c * len * 0.35, oy + s * len * 0.35, 330.0 + len * 0.3],
                             "mat": "bark", "yaw": a, "tilt": [0.0, -52.0]})));
    }
    let greens = ["#5d9a3a", "#6aa646", "#4f8b35", "#77b04e", "#609d3f"];
    for (i, (dx, dy, z, r)) in [(0.0, 0.0, 610.0, 250.0), (190.0, 120.0, 520.0, 190.0), (-180.0, 150.0, 540.0, 200.0), (150.0, -180.0, 500.0, 185.0),
                                (-170.0, -160.0, 520.0, 195.0), (40.0, 230.0, 470.0, 160.0), (-230.0, 10.0, 470.0, 165.0), (220.0, 20.0, 640.0, 150.0),
                                (-60.0, -40.0, 730.0, 170.0)].iter().enumerate() {
        oak.push(ball_c(*r, [ox + dx, oy + dy, *z], "leaves", greens[i % greens.len()]));
    }
    o.push(("old_oak", anchored("SP_old_oak",
        json!({"at": [ox, oy, 0.0], "size": [820.0, 820.0, 780.0], "center": [ox, oy, 390.0]}), oak)));
    for (name, label, from, h, dress) in [("kid_red", "A child running round the oak", 0.0, 118.0, "cloth_red"),
                                          ("kid_yellow", "A child running round the oak", 120.0, 112.0, "cloth_yellow"),
                                          ("kid_teal", "A child running round the oak", 240.0, 126.0, "cloth_teal")] {
        let keys = round_path((ox, oy), 330.0, from, 9.0);
        let start = keys[0].clone();
        o.push((name, json!({
            "label": label, "at": [start[1], start[2], 0], "loop": 9,
            "path": keys,
            "parts": [villager("run", [0.0, 0.0, 0.0], 0.0, dress, 0.55, h)],
        })));
    }
    let mut bench = vec![
        rb([150.0, 42.0, 6.0], [ox, -850.0, 42.0], "plank"),
        rb([150.0, 6.0, 34.0], [ox, -826.0, 76.0], "plank"),
    ];
    for x in [ox - 62.0, ox + 62.0] {
        bench.push(thin(bx([6.0, 38.0, 39.0], [x, -850.0, 19.5], "timber")));
        bench.push(thin(bx([6.0, 6.0, 90.0], [x, -826.0, 45.0], "timber")));
    }
    o.push(("oak_bench", thing("The bench facing the oak", bench)));
    o.push(("elder", thing("An old man on the bench", vec![villager("sit", [ox + 20.0, -838.0, 0.0], 270.0, "cloth_plum", 0.62, 166.0)])));

    // ------------------------------------------------------------------ the fields
    let (fx, fy) = (-2300.0, 580.0);
    let mut field = vec![thin(bx([1000.0, 480.0, 2.0], [fx, fy, 1.0], "soil"))];
    for y in [400.0, 520.0, 640.0, 760.0] {
        field.push(rb([960.0, 70.0, 18.0], [fx, y, 9.0], "soil"));
    }
    for (row, y) in [(0, 400.0), (1, 520.0)] {
        for k in 0..12 {
            let x = fx - 440.0 + 80.0 * k as f64 + if row == 1 { 40.0 } else { 0.0 };
            field.push(ball_c(15.0, [x, y, 28.0], "cabbage", if (k + row) % 3 == 0 { "#8fc152" } else { "#a6d266" }));
        }
    }
    for k in 0..30 {
        let x = fx - 450.0 + 31.0 * k as f64;
        field.push(thin(cone([x, 640.0, 18.0], 6.0, 20.0 + (k % 3) as f64 * 4.0, "stem")));
    }
    o.push(("crop_row", anchored("SP_crop_row",
        json!({"at": [fx, fy, 0.0], "size": [1000.0, 520.0, 80.0], "center": [fx, fy, 40.0]}), field)));
    let mut sunflowers = Vec::new();
    for k in 0..6 {
        let x = fx - 400.0 + 160.0 * k as f64;
        let h = 150.0 + (k % 3) as f64 * 14.0;
        sunflowers.push(thin(post([x, 760.0, 18.0], 5.0, h, "stem")));
        sunflowers.push(thin(json!({"box": [3.0, 26.0, 12.0], "at": [x, 760.0, 18.0 + h * 0.5], "mat": "stem", "tilt": [0.0, 25.0]})));
        sunflowers.push(thin(json!({"cylinder": [24.0, 3.0], "axis": "x", "at": [x + 3.0, 760.0, 18.0 + h], "mat": "petal"})));
        sunflowers.push(thin(json!({"cylinder": [11.0, 5.0], "axis": "x", "at": [x + 5.0, 760.0, 18.0 + h], "mat": "seed"})));
    }
    o.push(("sunflowers", thing("Sunflowers along the last row", sunflowers)));
    o.push(("farmer", thing("The farmer watering the rows", vec![with(
        villager("water", [-2380.0, 580.0, 2.0], 270.0, "cloth_blue", 0.6, 170.0),
        json!({"holds": "can"}),
    )])));
    o.push(("scarecrow", thing("A scarecrow", vec![
        thin(post([-1960.0, 860.0, 0.0], 8.0, 190.0, "timber")),
        thin(rod([-1960.0, 860.0, 150.0], 6.0, 130.0, "y", "timber")),
        thin(bx([22.0, 70.0, 62.0], [-1960.0, 860.0, 135.0], "cloth_red")),
        ball(18.0, [-1960.0, 860.0, 196.0], "sack"),
        thin(post([-1960.0, 860.0, 208.0], 64.0, 3.0, "straw")),
        thin(json!({"cylinder": [15.0, 18.0], "top": 11.0, "at": [-1960.0, 860.0, 219.0], "mat": "straw"})),
    ])));
    let mut fence = Vec::new();
    let (x0, x1, y0, y1) = (-2850.0, -1750.0, 300.0, 860.0);
    let mut posts: Vec<(f64, f64)> = Vec::new();
    for k in 0..=7 {
        let x = x0 + (x1 - x0) * k as f64 / 7.0;
        posts.push((x, y0));
        posts.push((x, y1 + 40.0));
    }
    for k in 1..4 {
        let y = y0 + (y1 + 40.0 - y0) * k as f64 / 4.0;
        posts.push((x0, y));
        if !(540.0..=700.0).contains(&y) {
            posts.push((x1, y));
        }
    }
    for (x, y) in posts {
        fence.push(thin(post([x, y, 0.0], 10.0, 105.0, "timber")));
    }
    for z in [52.0, 92.0] {
        fence.push(thin(rod([(x0 + x1) / 2.0, y0, z], 6.0, x1 - x0, "x", "timber")));
        fence.push(thin(rod([(x0 + x1) / 2.0, y1 + 40.0, z], 6.0, x1 - x0, "x", "timber")));
        fence.push(thin(rod([x0, (y0 + y1 + 40.0) / 2.0, z], 6.0, y1 + 40.0 - y0, "y", "timber")));
        fence.push(thin(rod([x1, (y0 + 540.0) / 2.0, z], 6.0, 540.0 - y0, "y", "timber")));
        fence.push(thin(rod([x1, (700.0 + y1 + 40.0) / 2.0, z], 6.0, y1 + 40.0 - 700.0, "y", "timber")));
    }
    o.push(("field_fence", structure("The fence round the fields", fence)));
    o.push(("wheelbarrow", thing_at("A wheelbarrow of pumpkins", [-1640.0, 760.0, 0.0], 200.0, vec![
        rb([90.0, 60.0, 30.0], [0.0, 0.0, 45.0], "crate"),
        thin(json!({"box": [44.0, 6.0, 44.0], "at": [58.0, 0.0, 22.0], "mat": "timber", "shape": "torus"})),
        thin(rod([-70.0, -24.0, 52.0], 4.0, 80.0, "x", "timber")),
        thin(rod([-70.0, 24.0, 52.0], 4.0, 80.0, "x", "timber")),
        thin(bx([5.0, 5.0, 30.0], [-25.0, -24.0, 15.0], "timber")),
        thin(bx([5.0, 5.0, 30.0], [-25.0, 24.0, 15.0], "timber")),
        json!({"sphere": 16.0, "scale": [1.0, 1.0, 0.78], "at": [-10.0, -10.0, 66.0], "mat": "pumpkin"}),
        json!({"sphere": 15.0, "scale": [1.0, 1.0, 0.78], "at": [18.0, 12.0, 66.0], "mat": "pumpkin"}),
    ])));

    // ------------------------------------------------------------------ houses, trees, flowers
    o.push(("house_red", thing("Cottage with the red roof", cottage((700.0, 1450.0), (520.0, 420.0), "plaster_white", "roof_red", 270.0))));
    o.push(("house_blue", thing("Cottage with the blue roof", cottage((1750.0, 450.0), (460.0, 460.0), "plaster_pink", "roof_blue", 180.0))));
    o.push(("farmhouse", thing("The farmhouse", cottage((-2650.0, -250.0), (560.0, 420.0), "plaster_cream", "roof_green", 0.0))));
    let mut trees = Vec::new();
    for (i, (x, y, s)) in [(1300.0, 2000.0, 1.1), (-1300.0, 2400.0, 1.2), (3200.0, -500.0, 1.0), (-3100.0, 1700.0, 1.15), (300.0, -2600.0, 1.05),
                           (3500.0, -2700.0, 1.2), (-3300.0, -1700.0, 1.1), (2500.0, 1600.0, 0.95), (-400.0, 2700.0, 1.0), (4200.0, 600.0, 1.3)]
        .iter()
        .enumerate()
    {
        trees.extend(round_tree((*x, *y), *s, i));
    }
    for (x, y, s) in [(3900.0, 1500.0, 1.2), (-3800.0, 300.0, 1.3), (1500.0, -3400.0, 1.25), (-2100.0, -3200.0, 1.15), (-4200.0, -900.0, 1.1), (2600.0, -3600.0, 1.2)] {
        trees.extend(pine((x, y), s));
    }
    o.push(("trees", structure("Trees round the village", trees)));
    let mut garden = Vec::new();
    for (x, y, r) in [(-380.0, 520.0, 46.0), (-640.0, 540.0, 40.0), (480.0, 380.0, 52.0), (560.0, 300.0, 40.0), (1300.0, 160.0, 50.0),
                      (2000.0, 120.0, 46.0), (-1000.0, 180.0, 44.0), (-1100.0, -500.0, 50.0), (1500.0, 1200.0, 48.0), (230.0, 1300.0, 46.0)] {
        garden.push(ball(r, [x, y, r * 0.7], "bush"));
    }
    let blooms = ["flower_pink", "flower_white", "flower_violet", "petal"];
    for (i, (cx, cy)) in [(-300.0, 560.0), (-760.0, 520.0), (420.0, 330.0), (330.0, 1240.0), (1520.0, 260.0)].iter().enumerate() {
        for k in 0..14 {
            let a = k as f64 * 137.5;
            let rr = 12.0 + 6.0 * (k as f64).sqrt() * 3.0;
            let (s, c) = a.to_radians().sin_cos();
            garden.push(thin(ball(7.5, [cx + c * rr, cy + s * rr, 8.0], blooms[(i + k) % blooms.len()])));
        }
    }
    o.push(("garden", structure("Bushes and flowers", garden)));
    o
}

/// What a shot can make appear (`props:` with `asset: props/<name>`).
fn props() -> Vec<(&'static str, Value)> {
    vec![("traveler", json!({"label": "A traveler", "parts": [villager("idle", [0.0, 0.0, 0.0], 0.0, "cloth_brown", 0.9, 175.0)]}))]
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
