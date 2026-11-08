//! The three game (D-72) as previz draws it: every part in its material, the jointed figures
//! posed at the frame's time, the lamps, and the scene's sky, fill, haze and exposure. Read from
//! the same JSON the page reads, and placed the way the page places it.

use serde_json::{json, Value};

use crate::geometry::{self as g, Vec3};
use crate::model::{num, vec3};
use crate::three::{Object, Scene};

use super::figure;
use super::mesh::{hex, unit, Builder, Col, Geo, Order, Surf, Xf};
use super::scene::{Beam, Env, Flicker, Fog, Light, Shadow};

/// cm -> m, as the page's `S`.
const S: f64 = 0.01;

/// The page's `P`: UE cm to three's metres.
fn p3(v: Vec3) -> Vec3 {
    [v[1] * S, v[2] * S, -v[0] * S]
}

fn parts(o: &Object) -> impl Iterator<Item = &Value> {
    o.raw.get("parts").and_then(Value::as_array).into_iter().flatten()
}

/// Whether an object walks or moves over a shot (a path, a figure, turning sails): its triangles
/// are built for each frame.
pub(crate) fn moves(o: &Object) -> bool {
    o.raw.get("path").and_then(Value::as_array).is_some_and(|a| !a.is_empty())
        || parts(o).any(|p| p.get("figure").is_some() || p.get("shape").and_then(Value::as_str) == Some("sails"))
}

fn scaled(c: Col, k: f64) -> Col {
    let k = k as f32;
    [c[0] * k, c[1] * k, c[2] * k]
}

/// The page's `look(part)` made into a surface; None for what previz leaves out (glass, faint
/// see-through things).
fn surface(globals: &Value, part: &Value) -> Option<Surf> {
    let m = part.get("mat").and_then(Value::as_str).and_then(|n| globals["materials"].get(n)).cloned().unwrap_or(Value::Null);
    if m.get("glass").and_then(Value::as_bool) == Some(true) || m.get("opacity").and_then(num).is_some_and(|o| o < 0.5) {
        return None;
    }
    // the part's own colour or glow when it gives one (a null is none), else the material's
    let color = part.get("color").and_then(Value::as_str).or_else(|| m.get("color").and_then(Value::as_str)).and_then(hex).unwrap_or([0.216; 3]);
    let glow = part.get("glow").and_then(num).or_else(|| m.get("glow").and_then(num));
    let tex = m.get("tex").and_then(Value::as_str);
    let metal = m.get("metal").and_then(num).unwrap_or(if tex == Some("metal") { 0.85 } else { 0.0 });
    // a baked texture averages a little darker than its colour (grime, wear)
    let k = (1.0 - metal) * if tex.is_some() { 0.9 } else { 1.0 };
    let tiles = (tex == Some("tiles")).then(|| {
        let n = m.get("n").and_then(num).unwrap_or(8.0).max(1.0);
        let sc: Vec<f64> = m.get("scale").and_then(Value::as_array).map(|a| a.iter().filter_map(num).collect()).unwrap_or_default();
        let other = m.get("color2").and_then(Value::as_str).and_then(hex).unwrap_or([0.03; 3]);
        (scaled(other, k), (sc.first().copied().unwrap_or(1.0) * 100.0 / n) as f32, (sc.get(1).copied().unwrap_or(1.0) * 100.0 / n) as f32)
    });
    let emit = glow.map(|gl| {
        let c = m.get("glow_color").and_then(Value::as_str).and_then(hex).unwrap_or(color);
        scaled(c, gl)
    });
    // a baked texture's roughness map, on average
    let rough = m.get("rough").and_then(num).unwrap_or(match tex {
        Some("tiles") => 0.3,
        Some("metal") | Some("plastic") => 0.45,
        Some("paint") => 0.55,
        Some("wood") => 0.6,
        Some("ceiling") => 0.7,
        Some(_) => 0.85,
        None => 0.8,
    });
    let f0 = [0, 1, 2].map(|i| (0.04 + (color[i] as f64 - 0.04) * metal) as f32);
    Some(Surf {
        albedo: scaled(color, k),
        tiles,
        emit: emit.unwrap_or([0.0; 3]),
        flicker: m.get("flicker").and_then(Value::as_bool).unwrap_or(false),
        rough: rough as f32,
        f0,
    })
}

/// A sign, board or screen: its picture's background, glowing when it is lit.
fn sign(globals: &Value, part: &Value) -> Surf {
    let base = surface(globals, part).unwrap_or(Surf::plain([0.2; 3]));
    let bg = part.get("bg").and_then(Value::as_str).and_then(hex).unwrap_or(base.albedo);
    let glow = part.get("glow").and_then(num).unwrap_or(0.0);
    let emit = if part.get("draw").and_then(Value::as_str) == Some("static") {
        scaled([0.3; 3], glow)
    } else {
        let fg = part.get("fg").and_then(Value::as_str).and_then(hex).unwrap_or(bg);
        scaled([0, 1, 2].map(|i| bg[i] * 0.7 + fg[i] * 0.3), glow * 0.6)
    };
    Surf { albedo: bg, emit, flicker: part.get("flicker").and_then(Value::as_bool).unwrap_or(false), ..base }
}

fn deg3(v: [f64; 3]) -> Vec3 {
    v.map(f64::to_radians)
}

/// Adds one object's parts placed at `at` facing `yaw` (UE), at time `t`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn object(b: &mut Builder, globals: &Value, o: &Object, at: Vec3, yaw: f64, t: f64, src: Option<usize>, name: &str) {
    let group = Xf::three(p3(at), deg3([0.0, -yaw, 0.0]), Order::Xyz, [1.0; 3]);
    let base = Xf::THREE_TO_UE.then(&group);
    for p in parts(o) {
        let pa = p.get("at").and_then(vec3).unwrap_or([0.0; 3]);
        let tilt: Vec<f64> = p.get("tilt").and_then(Value::as_array).map(|a| a.iter().filter_map(num).collect()).unwrap_or_default();
        let py = p.get("yaw").and_then(num).unwrap_or(0.0);
        let placed = Xf::three(
            p3(pa),
            deg3([tilt.first().copied().unwrap_or(0.0), -py, tilt.get(1).copied().unwrap_or(0.0)]),
            Order::Yxz,
            [1.0; 3],
        );
        let at_part = base.then(&placed);
        if let Some(size) = p.get("box").and_then(vec3) {
            let surf = match p.get("shape").and_then(Value::as_str) {
                Some("decal") => continue,
                Some("sign") => sign(globals, p),
                _ => match surface(globals, p) {
                    Some(s) => s,
                    None => continue,
                },
            };
            if p.get("shape").and_then(Value::as_str) == Some("sails") {
                sails(b, globals, p, size, &at_part, t, src);
                continue;
            }
            let (s, piece) = (b.surf(surf), b.piece(src));
            b.add(&Geo::cube([size[1] * S, size[2] * S, size[0] * S]), &at_part, s, piece);
        } else if let Some(c) = p.get("cylinder").and_then(Value::as_array) {
            let rh: Vec<f64> = c.iter().filter_map(num).collect();
            let [r, h] = rh[..] else { continue };
            let Some(surf) = surface(globals, p) else { continue };
            let top = p.get("top").and_then(num).unwrap_or(r);
            let sides = p.get("sides").and_then(num).map(|s| s as usize);
            let segs = sides.map(|s| s.clamp(3, 16)).unwrap_or(16);
            let open = p.get("open").and_then(Value::as_bool).unwrap_or(false);
            let turn = match p.get("axis").and_then(Value::as_str) {
                Some("x") => [std::f64::consts::FRAC_PI_2, 0.0, 0.0],
                Some("y") => [0.0, 0.0, std::f64::consts::FRAC_PI_2],
                _ => [0.0; 3],
            };
            let xf = at_part.then(&Xf::three([0.0; 3], turn, Order::Xyz, [1.0; 3]));
            let (s, piece) = (b.surf(surf), b.piece(src));
            let geo = Geo::cylinder(top * S, r * S, h * S, segs, !open);
            // a few sides (a hipped roof) are flat faces, as the page draws them
            let geo = if sides.is_some_and(|s| s <= 8) { geo.flat() } else { geo };
            b.add(&geo, &xf, s, piece);
        } else if let Some(r) = p.get("sphere").and_then(num) {
            let Some(surf) = surface(globals, p) else { continue };
            let sc = p.get("scale").and_then(vec3).map(|s| [s[1], s[2], s[0]]).unwrap_or([1.0; 3]);
            let (s, piece) = (b.surf(surf), b.piece(src));
            // big balls (a hill, a tree's crown) keep a round outline
            let (w, h) = if r >= 1000.0 { (48, 24) } else if r >= 100.0 { (24, 12) } else { (14, 8) };
            b.add(&Geo::sphere(r * S, w, h), &at_part.then(&Xf::scale(sc)), s, piece);
        } else if let Some(mv) = p.get("figure") {
            let thin = p.get("build").and_then(Value::as_str) == Some("thin");
            let skin_mat = p.get("mat").and_then(Value::as_str).unwrap_or(if thin { "shade" } else { "figure" });
            let skin = surface(globals, &json!({"mat": skin_mat, "color": p.get("color")})).unwrap_or(Surf::plain([0.6; 3]));
            let dress = p.get("dress").and_then(Value::as_str);
            let cloth = dress.and_then(|d| surface(globals, &json!({"mat": d})));
            let facing = p.get("facing").and_then(num).unwrap_or(0.0);
            let root = base.then(&Xf::three(p3(pa), deg3([0.0, -facing, 0.0]), Order::Xyz, [1.0; 3]));
            let seed = p.get("seed").and_then(Value::as_str).unwrap_or(name);
            // what it carries, in the page's material for it
            let holds = p.get("holds").and_then(Value::as_str).map(|what| {
                let mat = p.get("holds_mat").and_then(Value::as_str).unwrap_or(match what {
                    "can" => "tin",
                    "teapot" => "teapot",
                    "basket" => "basket",
                    "broom" => "plank",
                    _ => "accent",
                });
                (what, b.surf(surface(globals, &json!({"mat": mat})).unwrap_or(Surf::plain([0.3; 3]))))
            });
            let spec = figure::Spec {
                mv: mv.as_str().unwrap_or("idle"),
                height: p.get("height").and_then(num).unwrap_or(175.0),
                thin,
                seed,
                skin: b.surf(skin),
                cloth: cloth.map(|c| {
                    let len = p.get("dress_len").and_then(num).unwrap_or(if dress == Some("uniform") { 0.86 } else { 0.98 });
                    (b.surf(c), len)
                }),
                holds,
            };
            let piece = b.piece(src);
            figure::add(b, &root, &spec, t, piece);
        }
    }
}

/// A windmill's sails at time `t`, as the page's `sails()`: `blades` arms of a spar and a cloth
/// panel round the hub on the box's thin axis, turned `spin` degrees a second.
fn sails(b: &mut Builder, globals: &Value, p: &Value, size: Vec3, at: &Xf, t: f64, src: Option<usize>) {
    let r = (size[1] * S).min(size[2] * S) / 2.0;
    let n = p.get("blades").and_then(num).unwrap_or(4.0).max(1.0) as usize;
    let rate = p.get("spin").and_then(num).unwrap_or(20.0).to_radians();
    let wood_mat = p.get("frame_mat").and_then(Value::as_str).unwrap_or("timber");
    let wood = surface(globals, &json!({"mat": wood_mat})).unwrap_or(Surf::plain([0.3, 0.2, 0.12]));
    let cloth = surface(globals, p).unwrap_or(Surf::plain([0.8; 3]));
    let (wood, cloth, piece) = (b.surf(wood), b.surf(cloth), b.piece(src));
    let hub = at.then(&Xf::three([0.0; 3], [std::f64::consts::FRAC_PI_2, 0.0, 0.0], Order::Xyz, [1.0; 3]));
    b.add(&Geo::cylinder(r * 0.07, r * 0.07, (size[0] * S).max(r * 0.12), 10, true), &hub, wood, piece);
    for i in 0..n {
        let a = i as f64 * std::f64::consts::TAU / n as f64 - rate * t;
        let arm = at.then(&Xf::three([0.0; 3], [0.0, 0.0, a], Order::Xyz, [1.0; 3]));
        b.add(&Geo::cube([r * 0.035, r, r * 0.035]), &arm.then(&Xf::translate([0.0, r / 2.0, 0.0])), wood, piece);
        b.add(&Geo::cube([r * 0.26, r * 0.72, r * 0.012]), &arm.then(&Xf::translate([r * 0.15, r * 0.6, 0.0])), cloth, piece);
    }
}

/// The lamps of one object placed at `at` facing `yaw`.
fn object_lights(o: &Object, at: Vec3, yaw: f64, src: Option<usize>, out: &mut Vec<Light>) {
    let base = Xf::THREE_TO_UE.then(&Xf::three(p3(at), deg3([0.0, -yaw, 0.0]), Order::Xyz, [1.0; 3]));
    for p in parts(o) {
        let Some(kind) = p.get("light").and_then(Value::as_str) else { continue };
        let pos = base.point(p3(p.get("at").and_then(vec3).unwrap_or([0.0; 3])));
        let color = p.get("color").and_then(Value::as_str).and_then(hex).unwrap_or([1.0; 3]);
        let beam = if kind == "spot" {
            let to = base.point(p3(p.get("toward").and_then(vec3).unwrap_or([0.0; 3])));
            let cone = p.get("cone").and_then(num).unwrap_or(30.0).to_radians();
            let pen = p.get("penumbra").and_then(num).unwrap_or(0.5);
            Beam::Spot { dir: unit(g::sub(to, pos)), cos_outer: cone.cos(), cos_inner: (cone * (1.0 - pen)).cos() }
        } else {
            Beam::Point
        };
        let shadow = (p.get("shadow").and_then(Value::as_bool) == Some(true))
            .then_some(Shadow { size: if kind == "spot" { 384 } else { 160 }, area: None });
        out.push(Light {
            at: pos,
            beam,
            color: scaled(color, p.get("intensity").and_then(num).unwrap_or(10.0)),
            range: p.get("range").and_then(num).unwrap_or(1000.0),
            shadow,
            flicker: p.get("flicker").and_then(Value::as_bool).unwrap_or(false),
            halo: p.get("halo").and_then(num).map(|h| (h, p.get("halo_opacity").and_then(num).unwrap_or(0.25) as f32, color)),
            haze_beam: if kind == "spot" { p.get("beam").and_then(num).unwrap_or(0.0) as f32 } else { 0.0 },
            src,
            fitting: 0.0,
        });
    }
}

/// Every lamp in the game at `t`, and the moon. Each comes from its object; the moon from
/// the scene file, the source after the objects.
pub(crate) fn lights(scene: &Scene, t: f64) -> Vec<Light> {
    let mut out = Vec::new();
    for (i, o) in scene.objects.iter().enumerate() {
        let (at, yaw) = o.pose_at(t);
        object_lights(o, at, yaw, Some(i), &mut out);
    }
    let gl = &scene.globals;
    if let Some(m) = far_light(gl) {
        let centre = m.get("center").and_then(vec3).unwrap_or([0.0; 3]);
        let reach = m.get("reach").and_then(num).unwrap_or(3000.0);
        out.push(Light {
            at: centre,
            beam: Beam::Sun { toward: unit(m.get("from").and_then(vec3).unwrap_or([-1.0, 0.0, 1.0])) },
            color: scaled(m.get("color").and_then(Value::as_str).and_then(hex).unwrap_or([1.0; 3]), m.get("intensity").and_then(num).unwrap_or(0.5)),
            range: 0.0,
            shadow: (m.get("shadow").and_then(Value::as_bool) == Some(true)).then_some(Shadow { size: 1024, area: Some((centre, reach)) }),
            flicker: false,
            halo: None,
            haze_beam: 0.0,
            src: Some(scene.objects.len()),
            fitting: 0.0,
        });
    }
    out
}

/// The one far light: `sun` by day, `moon` by night (the same keys), as the page's `SUN`.
fn far_light(gl: &Value) -> Option<&Value> {
    gl.get("sun").filter(|m| m.is_object()).or_else(|| gl.get("moon").filter(|m| m.is_object()))
}

/// The scene's sky, fill, haze and finish, with the page's defaults.
pub(crate) fn env(gl: &Value) -> Env {
    let c = |v: Option<&Value>, d: &str| v.and_then(Value::as_str).and_then(hex).or_else(|| hex(d)).unwrap_or([0.0; 3]);
    let (ambient_sky, ambient_ground) = match gl.get("ambient").filter(|a| a.is_object()) {
        Some(a) => {
            let k = a.get("intensity").and_then(num).unwrap_or(0.3);
            (scaled(c(a.get("sky"), "#404040"), k), scaled(c(a.get("ground"), "#101010"), k))
        }
        None => ([0.0; 3], [0.0; 3]),
    };
    let background = c(gl.get("background"), "#000000");
    let (sky_top, sky_horizon, moon) = match gl.get("sky").filter(|s| s.is_object()) {
        Some(sk) => {
            let from = far_light(gl).and_then(|m| m.get("from")).and_then(vec3).unwrap_or([-1.0, 0.0, 1.0]);
            let size = sk.get("sun_size").or_else(|| sk.get("moon_size")).and_then(num).unwrap_or(1.1).to_radians();
            let disc = c(sk.get("sun").or_else(|| sk.get("moon")), "#dfe8ff");
            (c(sk.get("top"), "#03060c"), c(sk.get("horizon"), "#0c1218"), Some((unit(from), disc, size)))
        }
        None => (background, background, None),
    };
    let f = gl.get("fog").cloned().unwrap_or(Value::Null);
    let fog = Fog::Three {
        color: c(f.get("color"), "#000000"),
        density: f.get("density").and_then(num).unwrap_or(0.03),
        ground: f.get("ground").and_then(num).unwrap_or(0.0),
        height: f.get("height").and_then(num).unwrap_or(0.8),
        floor: f.get("floor").and_then(num).unwrap_or(0.0),
    };
    let post = gl.get("post").cloned().unwrap_or(Value::Null);
    let fl = gl.get("flicker").cloned().unwrap_or(Value::Null);
    let dips = fl
        .get("dips")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|d| Some((num(d.get(0)?)?, num(d.get(1)?)?))).collect())
        .unwrap_or_else(|| vec![(0.0, 0.08), (0.17, 0.25)]);
    Env {
        ambient_sky,
        ambient_ground,
        sky_top,
        sky_horizon,
        moon,
        fog: Some(fog),
        exposure: gl.get("exposure").and_then(num).unwrap_or(1.0) as f32,
        auto_exposure: None,
        bloom: (post.get("bloom").and_then(num).unwrap_or(0.6) as f32, post.get("threshold").and_then(num).unwrap_or(1.2) as f32),
        vignette: post.get("vignette").and_then(num).unwrap_or(0.55) as f32,
        flicker: Some(Flicker {
            offset: fl.get("offset").and_then(num).unwrap_or(1.4),
            period: fl.get("period").and_then(num).unwrap_or(2.6).max(1e-3),
            dips,
            low: fl.get("low").and_then(num).unwrap_or(0.12) as f32,
        }),
        ground: None,
        sky_cover: None,
        bounce: None,
        squared_cone: false,
    }
}

/// The objects that stand still (`moving` false) or move (`moving` true), at `t`.
pub(crate) fn build(scene: &Scene, t: f64, moving: bool) -> Builder {
    let mut b = Builder::default();
    for (i, o) in scene.objects.iter().enumerate() {
        if moves(o) != moving {
            continue;
        }
        let (at, yaw) = o.pose_at(t);
        object(&mut b, &scene.globals, o, at, yaw, t, Some(i), &o.name);
    }
    b
}
