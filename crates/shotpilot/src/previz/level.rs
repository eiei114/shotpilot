//! A UE level as previz draws it: the plan the engine read into `scout/level.json` (D-86).
//!
//! A shape the engine sent with its mesh (`mesh` and `xf`, the triangles under `meshes`) is
//! drawn as it is, so a door frame keeps its opening and a ramp its slope; any other shape is
//! its box. When the read also brought the lamps (`light` on a point) and the level's haze,
//! sky light, sun and exposure (`env`), the frame is lit as the level is; a plan read before
//! that is drawn as a graybox.

use std::collections::{BTreeSet, HashMap};

use serde_json::Value;

use crate::geometry::{self as g, Rot, Vec3};
use crate::model::{num, vec3};

use super::mesh::{cross, dot, hex, smooth_normals, unit, Builder, Col, Geo, Surf, Tri, Xf};
use super::scene::{AutoExposure, Beam, Env, Fog, Light, Shadow};

/// What a piece is, which sets its colour when the level does not say (doors orange and lamps
/// yellow as on the blocking map). Colours as shown on screen.
fn kind_color(kind: &str) -> [f32; 3] {
    match kind {
        "structure" => [0.74, 0.74, 0.72],
        "door" => [0.80, 0.57, 0.36],
        "light" => [0.88, 0.82, 0.56],
        "character" => [0.48, 0.62, 0.82],
        _ => [0.60, 0.62, 0.64],
    }
}

fn linear(c: [f32; 3]) -> Col {
    c.map(super::mesh::srgb_to_linear)
}

/// The level, ready to draw.
pub(crate) struct Level {
    pub built: Builder,
    /// Where the lamps are (the graybox draws a glow there).
    pub lamps: Vec<Vec3>,
    pub lights: Vec<Light>,
    /// Present when the read brought the level's light: the frame is lit as the level is.
    pub env: Option<Env>,
}

/// A mesh as the engine sent it: local vertices (cm) and triangles.
struct Mesh {
    v: Vec<Vec3>,
    i: Vec<[u32; 3]>,
}

fn meshes(v: &Value) -> HashMap<String, Mesh> {
    let mut out = HashMap::new();
    for (key, m) in v["meshes"].as_object().into_iter().flatten() {
        let flat: Vec<f64> = m["v"].as_array().map(|a| a.iter().filter_map(num).collect()).unwrap_or_default();
        let idx: Vec<u32> = m["i"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|x| x as u32)).collect()).unwrap_or_default();
        let verts: Vec<Vec3> = flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
        let tris: Vec<[u32; 3]> = idx
            .chunks_exact(3)
            .filter(|t| t.iter().all(|&k| (k as usize) < verts.len()))
            .map(|t| [t[0], t[1], t[2]])
            .collect();
        if !tris.is_empty() {
            out.insert(key.clone(), Mesh { v: verts, i: tris });
        }
    }
    out
}

/// Adds a mesh placed by `xf`. Faces point out of the mesh (by its enclosed volume); a mesh
/// with no inside (a plane, a card) is drawn from both sides.
fn add_mesh(b: &mut Builder, m: &Mesh, xf: &Xf, surf: u32, piece: u32) {
    let world: Vec<Vec3> = m.v.iter().map(|p| xf.point(*p)).collect();
    let mut normals = smooth_normals(&world, &m.i, 40.0);
    let vol: f64 = m.i.iter().map(|t| dot(world[t[0] as usize], cross(world[t[1] as usize], world[t[2] as usize]))).sum::<f64>() / 6.0;
    let (lo, hi) = world.iter().fold(([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]), |(lo, hi), p| {
        (std::array::from_fn(|k| lo[k].min(p[k])), std::array::from_fn(|k| hi[k].max(p[k])))
    });
    let ext: Vec3 = std::array::from_fn(|k| (hi[k] - lo[k]).max(1.0));
    let closed = vol.abs() > 0.02 * ext[0] * ext[1] * ext[2];
    if closed && vol < 0.0 {
        for n in &mut normals {
            *n = n.map(|v| g::scale(v, -1.0));
        }
    }
    for (t, n) in m.i.iter().zip(&normals) {
        let p = t.map(|k| world[k as usize]);
        b.tris.push(Tri { p, n: *n, surf, piece });
        if !closed {
            b.tris.push(Tri { p, n: n.map(|v| g::scale(v, -1.0)), surf, piece });
        }
    }
}

fn add_box(b: &mut Builder, center: Vec3, half: Vec3, yaw: f64, surf: u32, piece: u32) {
    let xf = Xf::ue(center, Rot::new(0.0, yaw, 0.0), [1.0; 3]);
    b.add(&Geo::cube(g::scale(half, 2.0)), &xf, surf, piece);
}

/// A standing character as a body and a head; one lying down stays a single block.
fn add_character(b: &mut Builder, center: Vec3, half: Vec3, yaw: f64, surf: u32, piece: u32) {
    if half[2] < 1.2 * half[0].max(half[1]) {
        add_box(b, center, half, yaw, surf, piece);
        return;
    }
    let h = 2.0 * half[2];
    let foot = center[2] - half[2];
    let body = [half[0] * 0.8, half[1] * 0.8, h * 0.41];
    add_box(b, [center[0], center[1], foot + body[2]], body, yaw, surf, piece);
    let r = half[0].min(half[1]) * 0.62;
    add_box(b, [center[0], center[1], foot + h - h * 0.075], [r, r, h * 0.075], yaw, surf, piece);
}

fn color_of(v: &Value) -> Option<Col> {
    match v {
        Value::String(s) => hex(s),
        Value::Array(a) if a.len() >= 3 => {
            let c: Vec<f32> = a.iter().filter_map(num).map(|x| x as f32).collect();
            (c.len() >= 3).then(|| [c[0], c[1], c[2]])
        }
        _ => None,
    }
}

fn lamp(p: &Value) -> Option<Light> {
    let at = p.get("at").and_then(vec3)?;
    let l = p.get("light").filter(|l| l.is_object())?;
    let color = l.get("color").and_then(color_of).unwrap_or([1.0; 3]);
    let kind = l.get("type").and_then(Value::as_str).unwrap_or("point");
    let dir = unit(l.get("dir").and_then(vec3).unwrap_or([0.0, 0.0, -1.0]));
    let intensity = l.get("cd").or_else(|| l.get("lux")).and_then(num).unwrap_or(0.0);
    let beam = match kind {
        "directional" => Beam::Sun { toward: g::scale(dir, -1.0) },
        "spot" => {
            let outer = l.get("outer").and_then(num).unwrap_or(44.0).clamp(1.0, 89.0);
            let inner = l.get("inner").and_then(num).unwrap_or(0.0).clamp(0.0, outer - 0.5);
            Beam::Spot { dir, cos_outer: outer.to_radians().cos(), cos_inner: inner.to_radians().cos() }
        }
        // a rect light throws its light over the half space in front of it
        "rect" => Beam::Spot { dir, cos_outer: 0.0, cos_inner: 0.5 },
        _ => Beam::Point,
    };
    let sun = matches!(beam, Beam::Sun { .. });
    let shadows = l.get("shadows").and_then(Value::as_bool).unwrap_or(true);
    let k = intensity as f32;
    Some(Light {
        at,
        beam,
        color: [color[0] * k, color[1] * k, color[2] * k],
        range: if sun { 0.0 } else { l.get("radius").and_then(num).unwrap_or(1000.0) },
        shadow: shadows.then_some(Shadow { size: if sun { 1024 } else if kind == "point" { 160 } else { 384 }, area: None }),
        flicker: false,
        halo: (!sun).then_some((24.0, 0.35, color)),
        haze_beam: 0.0,
        src: Some(0),
        fitting: 40.0,
    })
}

/// The exposure that shows the floor under a typical lamp of the level (its median, 3 m below
/// it, grey) nearly white. None for a level lit by its sun alone.
fn lamp_exposure(lights: &[Light]) -> Option<f32> {
    let mut cd: Vec<f32> = lights
        .iter()
        .filter(|l| !matches!(l.beam, Beam::Sun { .. }))
        .map(|l| l.color[0].max(l.color[1]).max(l.color[2]))
        .filter(|c| *c > 0.0)
        .collect();
    if cd.is_empty() {
        return None;
    }
    cd.sort_by(f32::total_cmp);
    let floor = 0.214 / std::f32::consts::PI * cd[cd.len() / 2] / 9.0;
    Some(1.5 / floor)
}

/// The level's haze, fill, sky and exposure from the read's `env`, around its lamps.
fn environment(e: &Value, lights: &[Light]) -> Env {
    let sun = lights.iter().find_map(|l| match l.beam {
        Beam::Sun { toward } => Some((toward, l.color)),
        _ => None,
    });
    let sun_lux = sun.map(|(_, c)| c[0].max(c[1]).max(c[2])).unwrap_or(0.0);
    let sky_light = e.get("sky_light").filter(|s| s.is_object());
    let fill = sky_light.and_then(|s| s.get("intensity")).and_then(num).unwrap_or(if sky_light.is_some() { 1.0 } else { 0.0 }) as f32;
    let fill_color = sky_light.and_then(|s| s.get("color")).and_then(color_of).unwrap_or([0.55, 0.62, 0.75]);
    // the sky and the fill it gives: bright with a sun, a dim night otherwise
    let (top, horizon) = match sun {
        Some((toward, _)) if toward[2] > 0.2 => (linear([0.24, 0.43, 0.78]), linear([0.67, 0.75, 0.86])),
        Some(_) => (linear([0.24, 0.20, 0.43]), linear([0.94, 0.55, 0.31])),
        None => ([0.002, 0.003, 0.006], [0.006, 0.008, 0.012]),
    };
    let sky_k = if sun.is_some() { sun_lux * 0.3 } else { 1.0 };
    let base = if sun.is_some() { sun_lux * 0.12 } else { 0.05 };
    let ambient_sky = fill_color.map(|c| c * fill * base);
    let fog = e.get("fog").filter(|f| f.is_object()).map(|f| Fog::Ue {
        color: f.get("color").and_then(color_of).unwrap_or([0.1, 0.12, 0.15]),
        density: f.get("density").and_then(num).unwrap_or(0.02) / 1000.0,
        falloff: f.get("falloff").and_then(num).unwrap_or(0.2) / 1000.0,
        z: f.get("z").and_then(num).unwrap_or(0.0),
        start: f.get("start").and_then(num).unwrap_or(0.0),
        max: f.get("max").and_then(num).unwrap_or(1.0) as f32,
    });
    let ex = e.get("exposure").cloned().unwrap_or(Value::Null);
    let bias = ex.get("bias").and_then(num).unwrap_or(1.0) as f32;
    let (min, max) = (ex.get("min").and_then(num).unwrap_or(-10.0) as f32, ex.get("max").and_then(num).unwrap_or(20.0) as f32);
    let manual = ex.get("method").and_then(Value::as_str) == Some("manual") || (max - min).abs() < 1e-3;
    let ev = if manual { ex.get("ev").and_then(num).map(|v| v as f32).unwrap_or(min) } else { 0.0 };
    Env {
        ambient_sky,
        ambient_ground: ambient_sky.map(|c| c * 0.3),
        sky_top: top.map(|c| c * sky_k),
        sky_horizon: horizon.map(|c| c * sky_k),
        moon: None,
        fog,
        exposure: 2f32.powf(bias) / (1.2 * 2f32.powf(ev)),
        auto_exposure: (!manual).then_some(AutoExposure { bias, min, max, around: lamp_exposure(lights) }),
        bloom: (0.35, 4.0),
        vignette: 0.3,
        flicker: None,
        ground: Some(linear([0.32, 0.32, 0.31])),
        sky_cover: Some(0.002),
        bounce: Some(0.12),
        squared_cone: true,
    }
}

/// Reads the plan. Every piece's source is the plan file (index 0 of the world's sources).
pub(crate) fn parse(v: &Value) -> Level {
    let points = v["points"].as_array().cloned().unwrap_or_default();
    // where the player starts is an editor marker, not something in the game's picture
    let starts: BTreeSet<&str> = points.iter().filter(|p| p["kind"] == "start").filter_map(|p| p["label"].as_str()).collect();
    let meshes = meshes(v);
    let mut lights: Vec<Light> = points.iter().filter(|p| p["kind"] == "light").filter_map(lamp).collect();
    // the sun is kept with the level's light, not among the lamps the blocking map draws
    if let Some(sun) = v.get("env").and_then(|e| e.get("sun")).filter(|s| s.is_object()) {
        lights.extend(lamp(&serde_json::json!({"at": [0.0, 0.0, 0.0], "light": sun})));
    }
    let lit = !lights.is_empty() || v.get("env").is_some_and(Value::is_object);
    let mut b = Builder::default();
    for s in v["shapes"].as_array().into_iter().flatten() {
        if s["label"].as_str().is_some_and(|l| starts.contains(l)) {
            continue;
        }
        let kind = s["kind"].as_str().unwrap_or("prop");
        // the blocking map's colours mark what a piece is; lit, a piece is its own colour or grey
        let fallback = if lit { [0.5, 0.5, 0.5] } else { kind_color(kind) };
        let shown = s.get("color").and_then(color_of).map(|c| c.map(super::mesh::linear_to_srgb)).unwrap_or(fallback);
        let surf = b.surf(Surf::plain(linear(shown)));
        let piece = b.piece(Some(0));
        let mesh = s.get("mesh").and_then(Value::as_str).and_then(|k| meshes.get(k));
        let xf = s.get("xf").filter(|x| x.is_object());
        if let (Some(m), Some(xf)) = (mesh, xf) {
            let at = xf.get("at").and_then(vec3).unwrap_or([0.0; 3]);
            let rot = xf.get("rot").and_then(vec3).map(Rot::from_array).unwrap_or_default();
            let scale = xf.get("scale").and_then(vec3).unwrap_or([1.0; 3]);
            add_mesh(&mut b, m, &Xf::ue(at, rot, scale), surf, piece);
            continue;
        }
        let (Some(center), Some(half)) = (s.get("center").and_then(vec3), s.get("half").and_then(vec3)) else { continue };
        let half = half.map(f64::abs);
        let yaw = s["yaw"].as_f64().unwrap_or(0.0);
        if kind == "character" {
            add_character(&mut b, center, half, yaw, surf, piece);
        } else {
            add_box(&mut b, center, half, yaw, surf, piece);
        }
    }
    let lamps = points.iter().filter(|p| p["kind"] == "light").filter_map(|p| p.get("at").and_then(vec3)).collect();
    let env = lit.then(|| environment(v.get("env").unwrap_or(&Value::Null), &lights));
    Level { built: b, lamps, lights, env }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_mesh_keeps_its_opening_and_faces_out() {
        // a door frame: two posts and a lintel as one mesh (wound outward), the opening between
        let mut v: Vec<f64> = Vec::new();
        let mut i: Vec<u64> = Vec::new();
        for (c, h) in [([-60.0, 0.0, 110.0], [10.0, 10.0, 110.0]), ([60.0, 0.0, 110.0], [10.0, 10.0, 110.0]), ([0.0, 0.0, 215.0], [70.0, 10.0, 5.0])] {
            for axis in 0..3 {
                for sign in [1.0, -1.0] {
                    let (u, w) = ((axis + 1) % 3, (axis + 2) % 3);
                    let base = (v.len() / 3) as u64;
                    let mut corners = vec![(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
                    if sign < 0.0 {
                        corners.reverse();
                    }
                    for (su, sw) in corners {
                        let mut p = c;
                        p[axis] += sign * h[axis];
                        p[u] += su * h[u];
                        p[w] += sw * h[w];
                        v.extend(p);
                    }
                    i.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
        let level = json!({
            "meshes": {"frame": {"v": v, "i": i}},
            "shapes": [{"label": "BP_DoorFrame", "kind": "door", "center": [0, 0, 110], "half": [70, 10, 110], "yaw": 0,
                        "mesh": "frame", "xf": {"at": [500, 0, 0], "rot": [0, 90, 0], "scale": [1, 1, 1]}}],
            "points": [], "floors": [0]});
        let l = parse(&level);
        assert!(l.env.is_none(), "no light was read: graybox");
        assert_eq!(l.built.tris.len(), 36);
        // turned 90 degrees: the posts stand at y = -60 and 60, the opening between them is clear
        let hits = |y: f64| l.built.tris.iter().any(|t| t.p.iter().all(|p| (p[1] - y).abs() <= 11.0 && p[2] < 200.0));
        assert!(hits(-60.0) && hits(60.0));
        assert!(!l.built.tris.iter().any(|t| t.p.iter().all(|p| p[1].abs() < 40.0 && p[2] < 150.0)));
        // every face points away from the middle of its post or lintel
        for t in &l.built.tris {
            let n = t.facing().unwrap();
            let c = g::scale(g::add(g::add(t.p[0], t.p[1]), t.p[2]), 1.0 / 3.0);
            let post = if c[2] > 209.0 { [500.0, 0.0, 215.0] } else { [500.0, c[1].signum() * 60.0, 110.0] };
            assert!(dot(n, g::sub(c, post)) > 0.0, "{c:?} {n:?}");
        }
    }

    #[test]
    fn lamps_and_the_levels_light_make_it_lit() {
        let level = json!({"shapes": [], "floors": [0],
            "points": [{"kind": "light", "label": "Spot", "at": [0, 0, 300],
                        "light": {"type": "spot", "dir": [0, 0, -1], "color": [1, 0.9, 0.8], "cd": 80, "radius": 900, "inner": 20, "outer": 40}}],
            "env": {"exposure": {"method": "manual", "ev": 1.0, "bias": 0}, "fog": {"density": 0.05, "falloff": 0.2, "color": [0.1, 0.1, 0.12]}}});
        let l = parse(&level);
        let env = l.env.expect("lit");
        assert!(env.auto_exposure.is_none());
        assert!((env.exposure - 1.0 / 2.4).abs() < 1e-6);
        assert!(matches!(l.lights[0].beam, Beam::Spot { .. }));
        assert_eq!(l.lights[0].color, [80.0, 72.0, 64.0]);
    }
}
