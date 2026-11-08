//! What previz draws: triangles with a surface each, and the transforms that place them.
//!
//! Shapes from the three game are built in three's frame (metres, Y up, right-handed), as the
//! page builds them, and turned into UE's (cm, Z up, X forward) as they are added. Which side of
//! a triangle faces out comes from its vertex normals, never from its winding, so the change of
//! handedness between the two frames needs no care.

use crate::geometry::{self as g, Rot, Vec3};

pub(crate) type Col = [f32; 3];

pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub(crate) fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// "#rrggbb" as a linear colour.
pub(crate) fn hex(s: &str) -> Option<Col> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    let c = |sh: u32| srgb_to_linear(((v >> sh) & 0xff) as f32 / 255.0);
    Some([c(16), c(8), c(0)])
}

/// What a surface looks like: its colour (linear), tiles of a second colour, and its glow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Surf {
    pub albedo: Col,
    /// The other tile colour and the tile's size (cm) along the face's two axes. Tiles run on
    /// the world grid, as the page lays them, so they continue across pieces.
    pub tiles: Option<(Col, f32, f32)>,
    /// Light it gives off (linear radiance).
    pub emit: Col,
    /// Its glow dips with the flickering lamps.
    pub flicker: bool,
    /// How rough it is (0 a mirror, 1 matt) and what it reflects head-on: the lamps' glints.
    pub rough: f32,
    pub f0: Col,
}

impl Surf {
    pub const fn plain(albedo: Col) -> Surf {
        Surf { albedo, tiles: None, emit: [0.0; 3], flicker: false, rough: 0.8, f0: [0.04; 3] }
    }

    /// The colour at a world point on a face with normal `n` (UE axes).
    pub fn albedo_at(&self, p: [f32; 3], n: [f32; 3]) -> Col {
        let Some((other, su, sv)) = self.tiles else { return self.albedo };
        // three's axes: x = UE y, y = UE z, z = -UE x (the page's placeUV picks the plane)
        let (x3, y3, z3) = (p[1], p[2], -p[0]);
        let (ax, ay, az) = (n[1].abs(), n[2].abs(), n[0].abs());
        let (u, v) = if ay >= ax && ay >= az {
            (x3, z3)
        } else if ax >= az {
            (z3, y3)
        } else {
            (x3, y3)
        };
        let k = (u / su).floor() as i64 + (v / sv).floor() as i64;
        if k.rem_euclid(2) == 1 {
            other
        } else {
            self.albedo
        }
    }
}

/// One triangle in the world (UE cm) with its vertex normals.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Tri {
    pub p: [Vec3; 3],
    pub n: [Vec3; 3],
    pub surf: u32,
    /// Which piece it belongs to: outlines run where pieces meet, and a piece knows its source.
    pub piece: u32,
}

impl Tri {
    /// The face normal, turned to agree with the vertex normals.
    pub fn facing(&self) -> Option<Vec3> {
        let c = cross(g::sub(self.p[1], self.p[0]), g::sub(self.p[2], self.p[0]));
        let l = g::length(c);
        if l < 1e-9 {
            return None;
        }
        let n = g::scale(c, 1.0 / l);
        let avg = g::add(g::add(self.n[0], self.n[1]), self.n[2]);
        Some(if dot(n, avg) < 0.0 { g::scale(n, -1.0) } else { n })
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = self.p[0];
        let mut hi = self.p[0];
        for p in &self.p[1..] {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }
}

pub(crate) fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

pub(crate) fn unit(a: Vec3) -> Vec3 {
    let l = g::length(a);
    if l > 1e-12 {
        g::scale(a, 1.0 / l)
    } else {
        [0.0, 0.0, 1.0]
    }
}

type M3 = [[f64; 3]; 3];

fn mul3(a: &M3, b: &M3) -> M3 {
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

fn apply3(m: &M3, v: Vec3) -> Vec3 {
    [dot(m[0], v), dot(m[1], v), dot(m[2], v)]
}

fn rot_x(a: f64) -> M3 {
    let (s, c) = a.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

fn rot_y(a: f64) -> M3 {
    let (s, c) = a.sin_cos();
    [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
}

fn rot_z(a: f64) -> M3 {
    let (s, c) = a.sin_cos();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

/// three's Euler orders that the page uses.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Order {
    Xyz,
    Yxz,
}

/// An affine transform: `r * v + t`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Xf {
    r: M3,
    t: Vec3,
}

impl Xf {
    pub const ID: Xf = Xf { r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] };
    /// three's metres (x, y up, z toward the viewer) into UE's cm: x_ue = -z, y_ue = x, z_ue = y.
    pub const THREE_TO_UE: Xf = Xf { r: [[0.0, 0.0, -100.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]], t: [0.0; 3] };

    pub fn then(&self, child: &Xf) -> Xf {
        Xf { r: mul3(&self.r, &child.r), t: g::add(apply3(&self.r, child.t), self.t) }
    }

    pub fn point(&self, v: Vec3) -> Vec3 {
        g::add(apply3(&self.r, v), self.t)
    }

    /// A normal carried by the inverse transpose, of unit length.
    pub fn normal(&self, n: Vec3) -> Vec3 {
        let m = &self.r;
        // cofactors; dividing by the determinant keeps the side when the frame flips
        let c = [
            [m[1][1] * m[2][2] - m[1][2] * m[2][1], m[1][2] * m[2][0] - m[1][0] * m[2][2], m[1][0] * m[2][1] - m[1][1] * m[2][0]],
            [m[0][2] * m[2][1] - m[0][1] * m[2][2], m[0][0] * m[2][2] - m[0][2] * m[2][0], m[0][1] * m[2][0] - m[0][0] * m[2][1]],
            [m[0][1] * m[1][2] - m[0][2] * m[1][1], m[0][2] * m[1][0] - m[0][0] * m[1][2], m[0][0] * m[1][1] - m[0][1] * m[1][0]],
        ];
        let det = m[0][0] * c[0][0] + m[0][1] * c[0][1] + m[0][2] * c[0][2];
        let s = if det < 0.0 { -1.0 } else { 1.0 };
        unit(g::scale(apply3(&c, n), s))
    }

    pub fn translate(t: Vec3) -> Xf {
        Xf { t, ..Xf::ID }
    }

    pub fn scale(s: Vec3) -> Xf {
        Xf { r: [[s[0], 0.0, 0.0], [0.0, s[1], 0.0], [0.0, 0.0, s[2]]], t: [0.0; 3] }
    }

    /// three's Object3D: position, Euler angles (radians) in `order`, then scale.
    pub fn three(pos: Vec3, rot: Vec3, order: Order, scale: Vec3) -> Xf {
        let (x, y, z) = (rot_x(rot[0]), rot_y(rot[1]), rot_z(rot[2]));
        let r = match order {
            Order::Xyz => mul3(&mul3(&x, &y), &z),
            Order::Yxz => mul3(&mul3(&y, &x), &z),
        };
        Xf { r: mul3(&r, &[[scale[0], 0.0, 0.0], [0.0, scale[1], 0.0], [0.0, 0.0, scale[2]]]), t: pos }
    }

    /// A UE component: location, rotator (with roll) and scale, as FRotationMatrix turns the
    /// local axes.
    pub fn ue(at: Vec3, rot: Rot, scale: Vec3) -> Xf {
        let (sp, cp) = rot.pitch.to_radians().sin_cos();
        let (sy, cy) = rot.yaw.to_radians().sin_cos();
        let (sr, cr) = rot.roll.to_radians().sin_cos();
        let axes = [
            [cp * cy, cp * sy, sp],
            [sr * sp * cy - cr * sy, sr * sp * sy + cr * cy, -sr * cp],
            [-(cr * sp * cy + sr * sy), cy * sr - cr * sp * sy, cr * cp],
        ];
        let c: [Vec3; 3] = std::array::from_fn(|k| g::scale(axes[k], scale[k]));
        Xf { r: [[c[0][0], c[1][0], c[2][0]], [c[0][1], c[1][1], c[2][1]], [c[0][2], c[1][2], c[2][2]]], t: at }
    }
}

/// An indexed triangle mesh in a local frame.
#[derive(Debug, Clone, Default)]
pub(crate) struct Geo {
    pub v: Vec<Vec3>,
    pub n: Vec<Vec3>,
    pub i: Vec<[u32; 3]>,
}

impl Geo {
    fn quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.i.push([a, b, c]);
        self.i.push([a, c, d]);
    }

    /// A box of the given size around the origin (three's BoxGeometry).
    pub fn cube(size: Vec3) -> Geo {
        let h = g::scale(size, 0.5);
        let mut out = Geo::default();
        for axis in 0..3 {
            for sign in [1.0, -1.0] {
                let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                let mut n = [0.0; 3];
                n[axis] = sign;
                let base = out.v.len() as u32;
                for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                    let mut p = [0.0; 3];
                    p[axis] = sign * h[axis];
                    p[u] = su * h[u];
                    p[v] = sv * h[v];
                    out.v.push(p);
                    out.n.push(n);
                }
                out.quad(base, base + 1, base + 2, base + 3);
            }
        }
        out
    }

    /// A cylinder (or cone) along Y around the origin (three's CylinderGeometry).
    pub fn cylinder(top: f64, bottom: f64, height: f64, segs: usize, caps: bool) -> Geo {
        let mut out = Geo::default();
        let hh = height / 2.0;
        let slope = (bottom - top) / height.max(1e-9);
        for k in 0..=segs {
            let a = k as f64 / segs as f64 * std::f64::consts::TAU;
            let (s, c) = a.sin_cos();
            let n = unit([s, slope, c]);
            out.v.push([top * s, hh, top * c]);
            out.n.push(n);
            out.v.push([bottom * s, -hh, bottom * c]);
            out.n.push(n);
        }
        for k in 0..segs as u32 {
            out.quad(2 * k, 2 * k + 1, 2 * k + 3, 2 * k + 2);
        }
        if caps {
            for (y, r, ny) in [(hh, top, 1.0), (-hh, bottom, -1.0)] {
                if r <= 1e-9 {
                    continue;
                }
                let centre = out.v.len() as u32;
                out.v.push([0.0, y, 0.0]);
                out.n.push([0.0, ny, 0.0]);
                for k in 0..segs {
                    let a = k as f64 / segs as f64 * std::f64::consts::TAU;
                    out.v.push([r * a.sin(), y, r * a.cos()]);
                    out.n.push([0.0, ny, 0.0]);
                }
                for k in 0..segs as u32 {
                    out.i.push([centre, centre + 1 + k, centre + 1 + (k + 1) % segs as u32]);
                }
            }
        }
        out
    }

    /// A sphere around the origin (three's SphereGeometry).
    pub fn sphere(r: f64, w: usize, h: usize) -> Geo {
        let mut out = Geo::default();
        for i in 0..=h {
            let th = std::f64::consts::PI * i as f64 / h as f64;
            for j in 0..=w {
                let ph = std::f64::consts::TAU * j as f64 / w as f64;
                let n = [-ph.cos() * th.sin(), th.cos(), ph.sin() * th.sin()];
                out.v.push(g::scale(n, r));
                out.n.push(n);
            }
        }
        let row = (w + 1) as u32;
        for i in 0..h as u32 {
            for j in 0..w as u32 {
                let (a, b) = (i * row + j, (i + 1) * row + j);
                out.quad(a, b, b + 1, a + 1);
            }
        }
        out
    }

    /// A profile of (radius, y) points turned about Y (three's LatheGeometry).
    pub fn lathe(profile: &[(f64, f64)], segs: usize) -> Geo {
        let mut out = Geo::default();
        let m = profile.len();
        for j in 0..=segs {
            let a = j as f64 / segs as f64 * std::f64::consts::TAU;
            let (s, c) = a.sin_cos();
            for i in 0..m {
                let (r, y) = profile[i];
                let (p0, p1) = (profile[i.saturating_sub(1)], profile[(i + 1).min(m - 1)]);
                let (dr, dy) = (p1.0 - p0.0, p1.1 - p0.1);
                let l = (dr * dr + dy * dy).sqrt().max(1e-9);
                let (nr, ny) = (dy / l, -dr / l);
                out.v.push([r * s, y, r * c]);
                out.n.push(unit([nr * s, ny, nr * c]));
            }
        }
        for j in 0..segs as u32 {
            for i in 0..m as u32 - 1 {
                let (a, b) = (j * m as u32 + i, (j + 1) * m as u32 + i);
                out.quad(a, a + 1, b + 1, b);
            }
        }
        out
    }
}

/// Collects the triangles of a scene, the surfaces they use and the pieces they belong to.
#[derive(Debug, Clone, Default)]
pub(crate) struct Builder {
    pub tris: Vec<Tri>,
    pub surfs: Vec<Surf>,
    /// The source (an index into the world's sources) behind each piece.
    pub pieces: Vec<Option<usize>>,
}

impl Builder {
    pub fn surf(&mut self, s: Surf) -> u32 {
        if let Some(i) = self.surfs.iter().position(|x| *x == s) {
            return i as u32;
        }
        self.surfs.push(s);
        (self.surfs.len() - 1) as u32
    }

    pub fn piece(&mut self, src: Option<usize>) -> u32 {
        self.pieces.push(src);
        (self.pieces.len() - 1) as u32
    }

    pub fn add(&mut self, geo: &Geo, xf: &Xf, surf: u32, piece: u32) {
        let p: Vec<Vec3> = geo.v.iter().map(|v| xf.point(*v)).collect();
        let n: Vec<Vec3> = geo.n.iter().map(|v| xf.normal(*v)).collect();
        for t in &geo.i {
            let [a, b, c] = t.map(|k| k as usize);
            self.tris.push(Tri { p: [p[a], p[b], p[c]], n: [n[a], n[b], n[c]], surf, piece });
        }
    }

    /// Adds another builder's triangles, surfaces and pieces after these.
    pub fn append(&mut self, other: &Builder) {
        let surf_map: Vec<u32> = other.surfs.iter().map(|s| self.surf(*s)).collect();
        let base = self.pieces.len() as u32;
        self.pieces.extend_from_slice(&other.pieces);
        self.tris.extend(other.tris.iter().map(|t| Tri { surf: surf_map[t.surf as usize], piece: base + t.piece, ..*t }));
    }
}

/// Vertex normals for a bare triangle soup: each corner averages the faces that share its
/// position and bend from it by less than `crease` degrees (curved parts read smooth, edges
/// stay sharp).
pub(crate) fn smooth_normals(v: &[Vec3], tris: &[[u32; 3]], crease: f64) -> Vec<[Vec3; 3]> {
    use std::collections::HashMap;
    let face: Vec<Vec3> = tris
        .iter()
        .map(|t| unit(cross(g::sub(v[t[1] as usize], v[t[0] as usize]), g::sub(v[t[2] as usize], v[t[0] as usize]))))
        .collect();
    let key = |p: Vec3| [(p[0] * 10.0).round() as i64, (p[1] * 10.0).round() as i64, (p[2] * 10.0).round() as i64];
    let mut at: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (fi, t) in tris.iter().enumerate() {
        for &k in t {
            at.entry(key(v[k as usize])).or_default().push(fi);
        }
    }
    let cos = crease.to_radians().cos();
    tris.iter()
        .enumerate()
        .map(|(fi, t)| {
            t.map(|k| {
                let mut s = [0.0; 3];
                for &o in &at[&key(v[k as usize])] {
                    if dot(face[o], face[fi]) >= cos {
                        s = g::add(s, face[o]);
                    }
                }
                unit(s)
            })
        })
        .collect()
}
