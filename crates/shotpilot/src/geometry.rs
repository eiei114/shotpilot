//! UE-convention camera math (left-handed, Z up, X forward, Y right, rotators in degrees).
//!
//! Dependency-free so it can be unit tested without the engine.

use serde_json::{json, Value};

pub type Vec3 = [f64; 3];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rot {
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
}

impl Rot {
    pub fn new(pitch: f64, yaw: f64, roll: f64) -> Self {
        Rot { pitch, yaw, roll }
    }

    pub fn from_array(a: [f64; 3]) -> Self {
        Rot::new(a[0], a[1], a[2])
    }

    pub fn as_array(&self) -> [f64; 3] {
        [self.pitch, self.yaw, self.roll]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xform {
    pub location: Vec3,
    pub rotation: Rot,
}

impl Xform {
    pub fn new(location: Vec3, rotation: Rot) -> Self {
        Xform { location, rotation }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "location": self.location.iter().map(|v| r3(*v)).collect::<Vec<_>>(),
            "rotation": self.rotation.as_array().iter().map(|v| r3(*v)).collect::<Vec<_>>(),
        })
    }

    pub fn rounded(&self) -> Xform {
        Xform::new(self.location.map(r3), Rot::from_array(self.rotation.as_array().map(r3)))
    }
}

/// Round to `n` decimals the way Python's `round(x, n)` does (correctly rounded on
/// the exact binary value), so fingerprints match takes made by earlier versions.
pub fn round_to(v: f64, n: usize) -> f64 {
    if !v.is_finite() {
        return v;
    }
    let r: f64 = format!("{v:.n$}").parse().unwrap_or(v);
    if r == 0.0 && v.is_sign_negative() {
        -0.0
    } else {
        r
    }
}

/// Fingerprints are stable across float noise: 0.001 cm / deg is far below visible.
pub fn r3(v: f64) -> f64 {
    round_to(v, 3)
}

/// Python's `round(x)`: half to even.
pub fn round_half_even(v: f64) -> i64 {
    let f = v.floor();
    let diff = v - f;
    let up = diff > 0.5 || (diff == 0.5 && (f as i64) % 2 != 0);
    (if up { f + 1.0 } else { f }) as i64
}

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub fn length(a: Vec3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

pub fn look_at(eye: Vec3, target: Vec3, roll: f64) -> Rot {
    let d = sub(target, eye);
    let horiz = d[0].hypot(d[1]);
    if horiz < 1e-6 && d[2].abs() < 1e-6 {
        return Rot::new(0.0, 0.0, roll);
    }
    let yaw = d[1].atan2(d[0]).to_degrees();
    let pitch = d[2].atan2(horiz).to_degrees();
    Rot::new(pitch, yaw, roll)
}

pub fn forward(r: Rot) -> Vec3 {
    let (sp, cp) = r.pitch.to_radians().sin_cos();
    let (sy, cy) = r.yaw.to_radians().sin_cos();
    [cp * cy, cp * sy, sp]
}

/// Ignores roll on purpose: trucking follows the ground plane, not a tilted horizon.
pub fn right(r: Rot) -> Vec3 {
    let (sy, cy) = r.yaw.to_radians().sin_cos();
    [-sy, cy, 0.0]
}

pub fn up(r: Rot) -> Vec3 {
    let f = forward(r);
    let rt = right(r);
    // up = forward x right (with Y as "right" this yields +Z for a level camera)
    [
        f[1] * rt[2] - f[2] * rt[1],
        f[2] * rt[0] - f[0] * rt[2],
        f[0] * rt[1] - f[1] * rt[0],
    ]
}

/// Rotate eye around pivot about the world Z axis. Positive = counter-clockwise seen from above.
pub fn orbit(eye: Vec3, pivot: Vec3, degrees: f64) -> Vec3 {
    let (s, c) = degrees.to_radians().sin_cos();
    let dx = eye[0] - pivot[0];
    let dy = eye[1] - pivot[1];
    [pivot[0] + dx * c - dy * s, pivot[1] + dx * s + dy * c, eye[2]]
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Python-style modulo (result has the sign of the divisor).
pub fn pymod(a: f64, b: f64) -> f64 {
    let m = a % b;
    if m != 0.0 && (m < 0.0) != (b < 0.0) {
        m + b
    } else {
        m
    }
}

pub fn angle_delta(a: f64, b: f64) -> f64 {
    pymod(b - a + 180.0, 360.0) - 180.0
}

pub fn lerp_angle(a: f64, b: f64, t: f64) -> f64 {
    a + angle_delta(a, b) * t
}

pub fn ease(t: f64, curve: &str) -> f64 {
    match curve {
        "linear" => t,
        "ease_in" => t * t,
        "ease_out" => 1.0 - (1.0 - t) * (1.0 - t),
        // ease_in_out (smoothstep) is the default: it reads as a deliberate camera move.
        _ => t * t * (3.0 - 2.0 * t),
    }
}

pub fn interpolate(a: &Xform, b: &Xform, t: f64) -> Xform {
    let loc = [
        lerp(a.location[0], b.location[0], t),
        lerp(a.location[1], b.location[1], t),
        lerp(a.location[2], b.location[2], t),
    ];
    let rot = Rot::new(
        lerp_angle(a.rotation.pitch, b.rotation.pitch, t),
        lerp_angle(a.rotation.yaw, b.rotation.yaw, t),
        lerp_angle(a.rotation.roll, b.rotation.roll, t),
    );
    Xform::new(loc, rot)
}

pub fn hfov_from_focal(focal_length_mm: f64, sensor_width_mm: f64) -> f64 {
    (2.0 * (sensor_width_mm / (2.0 * focal_length_mm)).atan()).to_degrees()
}

/// Pinhole projection. Returns (px, py, depth) or None when behind the camera.
pub fn project(point: Vec3, cam: &Xform, focal_length_mm: f64, width: u32, height: u32,
               sensor_width_mm: f64) -> Option<(f64, f64, f64)> {
    let rel = sub(point, cam.location);
    let f = forward(cam.rotation);
    let rt = right(cam.rotation);
    let u = up(cam.rotation);
    let depth = rel[0] * f[0] + rel[1] * f[1] + rel[2] * f[2];
    if depth <= 1.0 {
        return None;
    }
    let mut x = rel[0] * rt[0] + rel[1] * rt[1] + rel[2] * rt[2];
    let mut y = rel[0] * u[0] + rel[1] * u[1] + rel[2] * u[2];
    if cam.rotation.roll != 0.0 {
        let (s, c) = (-cam.rotation.roll).to_radians().sin_cos();
        let (nx, ny) = (x * c - y * s, x * s + y * c);
        x = nx;
        y = ny;
    }
    let (w, h) = (width as f64, height as f64);
    let fx = (w / 2.0) / (hfov_from_focal(focal_length_mm, sensor_width_mm).to_radians() / 2.0).tan();
    Some((w / 2.0 + fx * x / depth, h / 2.0 - fx * y / depth, depth))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6 * b.abs().max(1.0)
    }

    #[test]
    fn look_at_yaw_pitch() {
        let r = look_at([0.0; 3], [100.0, 100.0, 0.0], 0.0);
        assert!(approx(r.yaw, 45.0) && approx(r.pitch, 0.0));
        let r = look_at([0.0; 3], [100.0, 0.0, 100.0], 0.0);
        assert!(approx(r.pitch, 45.0));
    }

    #[test]
    fn basis_is_ue_convention() {
        let r = Rot::default();
        let (f, rt, u) = (forward(r), right(r), up(r));
        assert!(approx(f[0], 1.0) && approx(f[1], 0.0) && approx(f[2], 0.0));
        assert!(approx(rt[0], 0.0) && approx(rt[1], 1.0) && approx(rt[2], 0.0));
        assert!(approx(u[0], 0.0) && approx(u[1], 0.0) && approx(u[2], 1.0));
    }

    #[test]
    fn project_center_and_sides() {
        let cam = Xform::new([0.0; 3], Rot::default());
        let (cx, cy, depth) = project([1000.0, 0.0, 0.0], &cam, 35.0, 320, 180, 36.0).unwrap();
        assert!(approx(cx, 160.0) && approx(cy, 90.0) && approx(depth, 1000.0));
        let (x_right, _, _) = project([1000.0, 100.0, 0.0], &cam, 35.0, 320, 180, 36.0).unwrap();
        let (_, y_up, _) = project([1000.0, 0.0, 100.0], &cam, 35.0, 320, 180, 36.0).unwrap();
        assert!(x_right > 160.0); // +Y is screen right
        assert!(y_up < 90.0); // +Z is screen up
        assert!(project([-10.0, 0.0, 0.0], &cam, 35.0, 320, 180, 36.0).is_none());
    }

    #[test]
    fn longer_lens_magnifies() {
        let cam = Xform::new([0.0; 3], Rot::default());
        let wide = project([1000.0, 100.0, 0.0], &cam, 24.0, 320, 180, 36.0).unwrap().0 - 160.0;
        let tele = project([1000.0, 100.0, 0.0], &cam, 85.0, 320, 180, 36.0).unwrap().0 - 160.0;
        assert!(((tele / wide) - 85.0 / 24.0).abs() < 85.0 / 24.0 * 1e-3);
    }

    #[test]
    fn orbit_keeps_radius() {
        let e = orbit([100.0, 0.0, 50.0], [0.0; 3], 90.0);
        assert!(e[0].abs() < 1e-9 && approx(e[1], 100.0) && approx(e[2], 50.0));
    }

    #[test]
    fn smoothstep_ends() {
        assert_eq!(ease(0.0, "ease_in_out"), 0.0);
        assert_eq!(ease(1.0, "ease_in_out"), 1.0);
        assert!(approx(ease(0.5, "ease_in_out"), 0.5));
    }

    #[test]
    fn python_rounding() {
        assert_eq!(round_half_even(2.5), 2);
        assert_eq!(round_half_even(3.5), 4);
        assert_eq!(round_half_even(105.0), 105);
        assert_eq!(r3(1.23456), 1.235);
        assert_eq!(pymod(-190.0, 360.0), 170.0);
    }

    /// Guards the "geometry.rs stays dependency-free" invariant: only std and serde_json.
    #[test]
    fn geometry_uses_nothing_but_std_and_serde_json() {
        let src = include_str!("geometry.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        let mut uses = 0;
        for line in body.lines().map(str::trim).filter(|l| l.starts_with("use ")) {
            uses += 1;
            let path = line.trim_start_matches("use ").trim_start_matches("::");
            let root = path.split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("");
            assert!(root == "std" || root == "serde_json", "geometry.rs must not depend on `{line}`");
        }
        assert!(uses >= 1, "expected at least the serde_json use line");
        assert!(!body.contains("crate::"), "geometry.rs must not reach into other modules");
        assert!(!body.contains("super::"), "geometry.rs must not reach into other modules");
    }

    /// Guards the UE convention: yaw 90 looks along +Y, positive pitch looks up, and the
    /// right vector of a yawed camera follows (left-handed, Z up).
    #[test]
    fn yaw_and_pitch_follow_the_ue_directions() {
        let f = forward(Rot::new(0.0, 90.0, 0.0));
        assert!(approx(f[0], 0.0) && approx(f[1], 1.0) && approx(f[2], 0.0), "{f:?}");
        let r = right(Rot::new(0.0, 90.0, 0.0));
        assert!(approx(r[0], -1.0) && approx(r[1], 0.0), "{r:?}");
        let f = forward(Rot::new(30.0, 0.0, 0.0));
        assert!(f[2] > 0.49 && f[0] > 0.0, "{f:?}");
        // looking at a point on +Y from the origin gives yaw 90; a point above gives positive pitch
        assert!(approx(look_at([0.0; 3], [0.0, 100.0, 0.0], 0.0).yaw, 90.0));
        assert!(look_at([0.0; 3], [100.0, 0.0, 50.0], 0.0).pitch > 0.0);
    }
}
