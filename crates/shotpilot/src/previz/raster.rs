//! Drawing a scene into pixels. The triangles go into a z-buffer that keeps each pixel's
//! normal, surface and piece; then the pixels are lit, either by the graybox key light (a level
//! with no light of its own) or by the game's lamps with shadow maps, its fill, haze and
//! exposure. Ink runs where pieces meet or a surface folds.

use std::collections::BTreeSet;

use crate::geometry::{self as g, Vec3, Xform};

use super::mesh::{cross, dot, linear_to_srgb, unit, Builder, Col, Surf, Tri};
use super::scene::{aces, smooth, AutoExposure, Beam, Env, Light};
use super::Canvas;

/// Nothing nearer than this (cm) is drawn: what the camera stands inside is cut away.
pub(crate) const NEAR: f64 = 2.0;
/// Distance (cm) over which the graybox haze closes in.
const HAZE: f64 = 6000.0;
pub(crate) const SKY: u32 = u32::MAX;
pub(crate) const GROUND: u32 = u32::MAX - 1;

/// A pinhole camera in the units of the buffer it draws into. Same projection as
/// `geometry::project`.
#[derive(Clone)]
pub(crate) struct View {
    pub eye: Vec3,
    pub f: Vec3,
    pub r: Vec3,
    pub u: Vec3,
    /// sin and cos of -roll.
    rs: f64,
    rc: f64,
    pub fx: f64,
    pub cx: f64,
    pub cy: f64,
}

impl View {
    pub fn new(cam: &Xform, focal: f64, sensor: f64, w: usize, h: usize) -> View {
        let (rs, rc) = (-cam.rotation.roll).to_radians().sin_cos();
        let fx = (w as f64 / 2.0) / (g::hfov_from_focal(focal, sensor).to_radians() / 2.0).tan();
        View {
            eye: cam.location,
            f: g::forward(cam.rotation),
            r: g::right(cam.rotation),
            u: g::up(cam.rotation),
            rs,
            rc,
            fx,
            cx: w as f64 / 2.0,
            cy: h as f64 / 2.0,
        }
    }

    /// A square view from `eye` along `f` (a lamp's shadow map).
    fn looking(eye: Vec3, f: Vec3, fx: f64, size: usize) -> View {
        let f = unit(f);
        let up = if f[2].abs() > 0.95 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
        let r = unit(cross(up, f));
        let u = cross(f, r);
        View { eye, f, r, u, rs: 0.0, rc: 1.0, fx, cx: size as f64 / 2.0, cy: size as f64 / 2.0 }
    }

    /// Camera space: (right, up, depth).
    pub fn cam(&self, p: Vec3) -> Vec3 {
        let d = g::sub(p, self.eye);
        let (x, y) = (dot(d, self.r), dot(d, self.u));
        [x * self.rc - y * self.rs, x * self.rs + y * self.rc, dot(d, self.f)]
    }

    /// Pixel position and 1/depth of a camera-space point in front of the camera.
    pub fn screen(&self, c: Vec3) -> [f64; 3] {
        [self.cx + self.fx * c[0] / c[2], self.cy - self.fx * c[1] / c[2], 1.0 / c[2]]
    }

    pub fn project(&self, p: Vec3) -> Option<[f64; 3]> {
        let c = self.cam(p);
        (c[2] > NEAR).then(|| self.screen(c))
    }

    /// The world direction through a pixel; its part along the view is 1, so a hit at
    /// `eye + t * ray` lies at depth t.
    pub fn ray(&self, px: f64, py: f64) -> Vec3 {
        let (xr, yr) = ((px - self.cx) / self.fx, (self.cy - py) / self.fx);
        let (x, y) = (xr * self.rc + yr * self.rs, -xr * self.rs + yr * self.rc);
        [self.f[0] + self.r[0] * x + self.u[0] * y, self.f[1] + self.r[1] * x + self.u[1] * y, self.f[2] + self.r[2] * x + self.u[2] * y]
    }
}

/// Sutherland-Hodgman against the near plane in camera space, carrying a normal per corner.
fn clip_near(p: &[(Vec3, Vec3)]) -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::with_capacity(p.len() + 2);
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        let (ina, inb) = (a.0[2] >= NEAR, b.0[2] >= NEAR);
        if ina {
            out.push(a);
        }
        if ina != inb {
            let t = (NEAR - a.0[2]) / (b.0[2] - a.0[2]);
            let l = |x: Vec3, y: Vec3| [x[0] + (y[0] - x[0]) * t, x[1] + (y[1] - x[1]) * t, x[2] + (y[2] - x[2]) * t];
            let mut c = l(a.0, b.0);
            c[2] = NEAR;
            out.push((c, l(a.1, b.1)));
        }
    }
    out
}

/// Visits the pixel centres inside a screen triangle with their barycentric weights.
fn raster(w: usize, h: usize, a: [f64; 3], b: [f64; 3], c: [f64; 3], mut f: impl FnMut(usize, f64, f64, f64)) {
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if area.abs() < 1e-12 || !area.is_finite() {
        return;
    }
    let (wf, hf) = (w as f64, h as f64);
    let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.0);
    let x1 = a[0].max(b[0]).max(c[0]).ceil().min(wf - 1.0);
    let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.0);
    let y1 = a[1].max(b[1]).max(c[1]).ceil().min(hf - 1.0);
    if x0 > x1 || y0 > y1 {
        return;
    }
    let inv_area = 1.0 / area;
    // edge functions: e(p, q) at a pixel is the weight of the vertex opposite p-q, times area
    let e = |p: [f64; 3], q: [f64; 3], x: f64, y: f64| (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
    let (d0, d1, d2) = (-(c[1] - b[1]) * inv_area, -(a[1] - c[1]) * inv_area, -(b[1] - a[1]) * inv_area);
    let eps = -1e-9;
    for y in y0 as usize..=y1 as usize {
        let (py, px) = (y as f64 + 0.5, x0 + 0.5);
        let mut l0 = e(b, c, px, py) * inv_area;
        let mut l1 = e(c, a, px, py) * inv_area;
        let mut l2 = e(a, b, px, py) * inv_area;
        let row = y * w;
        for x in x0 as usize..=x1 as usize {
            if l0 >= eps && l1 >= eps && l2 >= eps {
                f(row + x, l0, l1, l2);
            }
            l0 += d0;
            l1 += d1;
            l2 += d2;
        }
    }
}

/// Whether all three corners lie beyond one side of a view's frustum.
fn outside(v: &View, c: &[Vec3; 3], half_w: f64, half_h: f64) -> bool {
    let (tx, ty) = (half_w / v.fx, half_h / v.fx);
    let all = |f: &dyn Fn(&Vec3) -> bool| c.iter().all(f);
    all(&|c| c[2] < NEAR) || all(&|c| c[0] > tx * c[2]) || all(&|c| c[0] < -tx * c[2]) || all(&|c| c[1] > ty * c[2]) || all(&|c| c[1] < -ty * c[2])
}

// --------------------------------------------------------------------------- shadow maps

/// How a shadow map looks at the world.
#[derive(Clone)]
enum Proj {
    Persp(View),
    /// Straight along `f` from far behind `o`: `s` pixels per cm, centred.
    Ortho { o: Vec3, f: Vec3, r: Vec3, u: Vec3, s: f64, c: f64 },
}

#[derive(Clone)]
struct Face {
    proj: Proj,
    size: usize,
    /// The nearest depth (cm) along the light seen at each texel.
    z: Vec<f32>,
}

impl Face {
    fn new(proj: Proj, size: usize) -> Face {
        Face { proj, size, z: vec![f32::INFINITY; size * size] }
    }

    /// Texel position and depth of a world point, and the world size of a texel there.
    fn locate(&self, p: Vec3) -> Option<(f64, f64, f64, f64)> {
        match &self.proj {
            Proj::Persp(v) => {
                let c = v.cam(p);
                (c[2] > NEAR).then(|| {
                    let s = v.screen(c);
                    (s[0], s[1], c[2], c[2] / v.fx)
                })
            }
            Proj::Ortho { o, f, r, u, s, c } => {
                let d = g::sub(p, *o);
                Some((c + dot(d, *r) * s, c - dot(d, *u) * s, dot(d, *f), 1.0 / s))
            }
        }
    }

    fn draw(&mut self, t: &Tri) {
        let size = self.size;
        let pts: Vec<[f64; 3]> = match &self.proj {
            Proj::Persp(v) => {
                let cam = [v.cam(t.p[0]), v.cam(t.p[1]), v.cam(t.p[2])];
                if outside(v, &cam, size as f64 / 2.0, size as f64 / 2.0) {
                    return;
                }
                let poly = clip_near(&[(cam[0], [0.0; 3]), (cam[1], [0.0; 3]), (cam[2], [0.0; 3])]);
                poly.iter().map(|(c, _)| v.screen(*c)).collect()
            }
            Proj::Ortho { o, f, r, u, s, c } => t
                .p
                .iter()
                .map(|p| {
                    let d = g::sub(*p, *o);
                    [c + dot(d, *r) * s, c - dot(d, *u) * s, dot(d, *f)]
                })
                .collect(),
        };
        if pts.len() < 3 {
            return;
        }
        let persp = matches!(self.proj, Proj::Persp(_));
        let z = &mut self.z;
        for i in 1..pts.len() - 1 {
            let (a, b, c) = (pts[0], pts[i], pts[i + 1]);
            raster(size, size, a, b, c, |k, l0, l1, l2| {
                let v = l0 * a[2] + l1 * b[2] + l2 * c[2];
                let d = if persp { 1.0 / v } else { v } as f32;
                if d < z[k] {
                    z[k] = d;
                }
            });
        }
    }

    /// How much of the light reaches `p` (with normal `n`), over a 3x3 of texels.
    fn reach(&self, p: Vec3, n: Vec3) -> Option<f32> {
        // lift the point off its surface by about a texel
        let texel = match &self.proj {
            Proj::Persp(v) => g::length(g::sub(p, v.eye)) / v.fx,
            Proj::Ortho { s, .. } => 1.0 / s,
        };
        let q = g::add(p, g::scale(n, texel * 1.5));
        let (x, y, d, texel) = self.locate(q)?;
        let bias = (texel * 1.5 + 0.5) as f32;
        let (xi, yi) = (x.floor() as i64, y.floor() as i64);
        let size = self.size as i64;
        if xi < -1 || yi < -1 || xi > size || yi > size {
            return None;
        }
        let mut lit = 0.0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (sx, sy) = (xi + dx, yi + dy);
                let open = sx < 0 || sy < 0 || sx >= size || sy >= size || (d as f32) - bias <= self.z[(sy * size + sx) as usize];
                if open {
                    lit += 1.0;
                }
            }
        }
        Some(lit / 9.0)
    }
}

/// A lamp's shadow map: one face for a spot or the sun, six for a point.
#[derive(Clone)]
pub(crate) struct ShadowMap {
    faces: Vec<Face>,
    pub at: Vec3,
    point: bool,
    range: f64,
    fitting: f64,
}

impl ShadowMap {
    /// The map for `light`, drawn from the triangles within its reach. A sun that names no area
    /// covers `around` (a centre and a half width in cm).
    pub fn build(light: &Light, tris: &[Tri], around: (Vec3, f64)) -> Option<ShadowMap> {
        let sh = light.shadow?;
        let size = sh.size.clamp(64, 2048);
        let faces = match light.beam {
            Beam::Spot { dir, cos_outer, .. } => {
                let half = cos_outer.clamp(-1.0, 1.0).acos().min(80f64.to_radians()) + 4f64.to_radians();
                let fx = size as f64 / 2.0 / half.tan();
                vec![Face::new(Proj::Persp(View::looking(light.at, dir, fx, size)), size)]
            }
            Beam::Point => [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]]
                .iter()
                .map(|d| Face::new(Proj::Persp(View::looking(light.at, *d, size as f64 / 2.0, size)), size))
                .collect(),
            Beam::Sun { toward } => {
                let (centre, half) = sh.area.unwrap_or(around);
                let f = g::scale(unit(toward), -1.0);
                let up = if f[2].abs() > 0.95 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
                let r = unit(cross(up, f));
                let u = cross(f, r);
                let o = g::sub(centre, g::scale(f, 20000.0));
                vec![Face::new(Proj::Ortho { o, f, r, u, s: size as f64 / (2.0 * half), c: size as f64 / 2.0 }, size)]
            }
        };
        let range = if matches!(light.beam, Beam::Sun { .. }) { 0.0 } else { light.range };
        let mut map = ShadowMap { faces, at: light.at, point: matches!(light.beam, Beam::Point), range, fitting: light.fitting };
        map.add(tris);
        Some(map)
    }

    /// Draws more triangles into the map (what moves, over a map of what stands still). The
    /// lamp's own fitting (anything within its `fitting` distance) casts no shadow.
    pub fn add(&mut self, tris: &[Tri]) {
        for t in tris {
            if self.range > 0.0 {
                let (lo, hi) = t.bounds();
                let near: Vec3 = std::array::from_fn(|k| self.at[k].clamp(lo[k], hi[k]));
                let d = g::length(g::sub(near, self.at));
                if d > self.range || d < self.fitting {
                    continue;
                }
            }
            for f in &mut self.faces {
                f.draw(t);
            }
        }
    }

    /// How much light reaches `p`: 1 in the open, 0 in shadow.
    fn reach(&self, p: Vec3, n: Vec3) -> f32 {
        let face = if self.point {
            let d = g::sub(p, self.at);
            let (ax, ay, az) = (d[0].abs(), d[1].abs(), d[2].abs());
            if ax >= ay && ax >= az {
                if d[0] > 0.0 { 0 } else { 1 }
            } else if ay >= az {
                if d[1] > 0.0 { 2 } else { 3 }
            } else if d[2] > 0.0 {
                4
            } else {
                5
            }
        } else {
            0
        };
        self.faces[face].reach(p, n).unwrap_or(1.0)
    }
}

// --------------------------------------------------------------------------- the frame

/// The graybox light: a key light from the time of day, a sky-over-ground fill and a little
/// light from the camera, so no face goes black. Colours are as shown (not linear).
pub(crate) struct Graybox {
    pub key: Vec3,
    pub tone: Col,
    pub top: Col,
    pub horizon: Col,
    pub ground: Col,
    pub ground_z: f64,
}

/// One frame being drawn: per pixel 1/depth (0 = nothing), normal, surface and piece, and
/// the colour once lit.
pub(crate) struct Frame {
    pub w: usize,
    pub h: usize,
    pub view: View,
    pub inv: Vec<f32>,
    nrm: Vec<[f32; 3]>,
    surf: Vec<u32>,
    pub piece: Vec<u32>,
    pub col: Vec<Col>,
}

impl Frame {
    pub fn new(view: View, w: usize, h: usize) -> Frame {
        let n = w * h;
        Frame { w, h, view, inv: vec![0.0; n], nrm: vec![[0.0, 0.0, 1.0]; n], surf: vec![0; n], piece: vec![SKY; n], col: vec![[0.0; 3]; n] }
    }

    /// Z-buffers the triangles that face the camera.
    pub fn draw(&mut self, tris: &[Tri]) {
        let (w, h) = (self.w, self.h);
        for t in tris {
            let cam = [self.view.cam(t.p[0]), self.view.cam(t.p[1]), self.view.cam(t.p[2])];
            if outside(&self.view, &cam, self.view.cx, self.view.cy) {
                continue;
            }
            let Some(face) = t.facing() else { continue };
            if dot(face, g::sub(self.view.eye, t.p[0])) <= 0.0 {
                continue;
            }
            let poly = clip_near(&[(cam[0], t.n[0]), (cam[1], t.n[1]), (cam[2], t.n[2])]);
            if poly.len() < 3 {
                continue;
            }
            let s: Vec<[f64; 3]> = poly.iter().map(|(c, _)| self.view.screen(*c)).collect();
            for i in 1..s.len() - 1 {
                let (a, b, c) = (s[0], s[i], s[i + 1]);
                let (na, nb, nc) = (poly[0].1, poly[i].1, poly[i + 1].1);
                let (inv, nrm, surf, piece) = (&mut self.inv, &mut self.nrm, &mut self.surf, &mut self.piece);
                raster(w, h, a, b, c, |k, l0, l1, l2| {
                    let z = (l0 * a[2] + l1 * b[2] + l2 * c[2]) as f32;
                    if z > inv[k] {
                        inv[k] = z;
                        // perspective-correct normal
                        let (w0, w1, w2) = (l0 * a[2], l1 * b[2], l2 * c[2]);
                        let n = [
                            na[0] * w0 + nb[0] * w1 + nc[0] * w2,
                            na[1] * w0 + nb[1] * w1 + nc[1] * w2,
                            na[2] * w0 + nb[2] * w1 + nc[2] * w2,
                        ];
                        let n = unit(n);
                        nrm[k] = [n[0] as f32, n[1] as f32, n[2] as f32];
                        surf[k] = t.surf;
                        piece[k] = t.piece;
                    }
                });
            }
        }
    }

    /// The ground plane at `z` under everything else (a hair farther than the plane, so floors
    /// resting on it win).
    pub fn ground(&mut self, z: f64) {
        if self.view.eye[2] <= z + 0.5 {
            return;
        }
        for y in 0..self.h {
            for x in 0..self.w {
                let i = y * self.w + x;
                let dir = self.view.ray(x as f64 + 0.5, y as f64 + 0.5);
                if dir[2] / g::length(dir) >= -1e-4 {
                    continue;
                }
                let t = (z - self.view.eye[2]) / dir[2];
                let inv = (1.0 / (t * 1.0005)) as f32;
                if inv > self.inv[i] {
                    self.inv[i] = inv;
                    self.nrm[i] = [0.0, 0.0, 1.0];
                    self.piece[i] = GROUND;
                }
            }
        }
    }

    /// The world point under a pixel.
    fn point(&self, i: usize) -> Vec3 {
        let (x, y) = (i % self.w, i / self.w);
        let d = 1.0 / self.inv[i] as f64;
        g::add(self.view.eye, g::scale(self.view.ray(x as f64 + 0.5, y as f64 + 0.5), d))
    }

    /// The level sources the frame showed.
    pub fn seen(&self, pieces: &[Option<usize>]) -> BTreeSet<usize> {
        let mut shown = vec![false; pieces.len()];
        for &p in &self.piece {
            if let Some(s) = shown.get_mut(p as usize) {
                *s = true;
            }
        }
        pieces.iter().zip(shown).filter(|(_, on)| *on).filter_map(|(s, _)| *s).collect()
    }

    // ----------------------------------------------------------------------- graybox

    pub fn shade_graybox(&mut self, b: &Builder, gb: &Graybox) {
        let eye = self.view.eye;
        let lit = |n: Vec3, at: Vec3| {
            let key = dot(n, gb.key).max(0.0);
            let hemi = 0.5 + 0.5 * n[2];
            let head = dot(n, unit(g::sub(eye, at))).max(0.0);
            let s = (0.26 + 0.24 * hemi + 0.36 * key + 0.20 * head) as f32;
            [s * gb.tone[0], s * gb.tone[1], s * gb.tone[2]]
        };
        let lg = lit([0.0, 0.0, 1.0], [eye[0], eye[1], gb.ground_z]);
        let ground = [gb.ground[0] * lg[0] * 1.6, gb.ground[1] * lg[1] * 1.6, gb.ground[2] * lg[2] * 1.6];
        let line = lerp(ground, [0.92, 0.92, 0.90], 0.22);
        let major = lerp(ground, [0.95, 0.95, 0.93], 0.42);
        for i in 0..self.w * self.h {
            let (x, y) = (i % self.w, i / self.w);
            let c = match self.piece[i] {
                SKY => {
                    let dir = self.view.ray(x as f64 + 0.5, y as f64 + 0.5);
                    let ez = dir[2] / g::length(dir);
                    let s = (ez / 0.55).clamp(0.0, 1.0).powf(0.7) as f32;
                    if ez >= 0.0 {
                        lerp(gb.horizon, gb.top, s)
                    } else {
                        gb.horizon
                    }
                }
                GROUND => {
                    let dir = self.view.ray(x as f64 + 0.5, y as f64 + 0.5);
                    let len = g::length(dir);
                    let t = (gb.ground_z - eye[2]) / dir[2];
                    let (px, py) = (eye[0] + dir[0] * t, eye[1] + dir[1] * t);
                    // cm of ground per pixel, stretched where the ray grazes it
                    let foot = t * len / self.view.fx / (-dir[2] / len).max(0.03);
                    let grid = |step: f64| {
                        let fade = (1.0 - foot / (step * 0.3)).clamp(0.0, 1.0);
                        if fade <= 0.0 {
                            return 0.0;
                        }
                        let near = |v: f64| (v / step - (v / step).round()).abs() * step;
                        let d = near(px).min(near(py));
                        (1.0 - d / (0.8 * foot)).clamp(0.0, 1.0) * fade
                    };
                    let (m, n) = (grid(500.0), grid(100.0));
                    lerp(lerp(ground, line, n as f32), major, m as f32)
                }
                _ => {
                    let at = self.point(i);
                    let n = self.nrm[i];
                    let s = b.surfs.get(self.surf[i] as usize).copied().unwrap_or(Surf::plain([0.5; 3]));
                    let base = shown(s.albedo_at([at[0] as f32, at[1] as f32, at[2] as f32], n));
                    let l = lit([n[0] as f64, n[1] as f64, n[2] as f64], at);
                    let e = shown(s.emit);
                    [(base[0] * l[0] + e[0]).min(1.0), (base[1] * l[1] + e[1]).min(1.0), (base[2] * l[2] + e[2]).min(1.0)]
                }
            };
            self.col[i] = c;
        }
        // haze toward the horizon colour with distance
        for (c, &inv) in self.col.iter_mut().zip(&self.inv) {
            if inv > 0.0 {
                let f = ((1.0 - (-(1.0 / inv as f64) / HAZE).exp()) * 0.8) as f32;
                *c = lerp(*c, gb.horizon, f);
            }
        }
    }

    // ----------------------------------------------------------------------- lit

    /// Lights the frame as the game does: its lamps (shadowed where the map says), its fill,
    /// haze, glow, exposure and tone. Says which lamps changed a pixel.
    pub fn shade_lit(&mut self, b: &Builder, lights: &[Light], maps: &[Option<ShadowMap>], sky: Option<&ShadowMap>, env: &Env, t: f64)
        -> Vec<bool> {
        let mut lit_by = vec![false; lights.len()];
        let n_px = self.w * self.h;
        let level = env.level(t);
        let mut alb = vec![[0.0f32; 3]; n_px];
        let mut irr = vec![[0.0f32; 3]; n_px];
        let mut hdr = vec![[0.0f32; 3]; n_px];
        let mut pos = vec![[0.0f64; 3]; n_px];
        let mut gloss = vec![(1.0f32, [0.04f32; 3]); n_px];
        let ground = Surf::plain(env.ground.unwrap_or([0.12; 3]));
        for i in 0..n_px {
            let (x, y) = (i % self.w, i / self.w);
            if self.piece[i] == SKY {
                hdr[i] = env.sky(self.view.ray(x as f64 + 0.5, y as f64 + 0.5));
                continue;
            }
            let p = self.point(i);
            pos[i] = p;
            let n = self.nrm[i];
            let s = if self.piece[i] == GROUND { ground } else { b.surfs.get(self.surf[i] as usize).copied().unwrap_or(ground) };
            alb[i] = s.albedo_at([p[0] as f32, p[1] as f32, p[2] as f32], n);
            gloss[i] = (s.rough.max(0.05), s.f0);
            hdr[i] = s.emit;
            irr[i] = env.ambient(n);
            // under a roof the sky's fill is mostly gone
            if let (Some(cover), Some(m)) = (env.sky_cover, sky) {
                let open = m.reach(p, [n[0] as f64, n[1] as f64, n[2] as f64]);
                irr[i] = irr[i].map(|v| v * (cover + (1.0 - cover) * open));
            }
        }
        // flickering glow dims with its lamps
        if level < 1.0 {
            for ((h, &piece), &surf) in hdr.iter_mut().zip(&self.piece).zip(&self.surf) {
                if piece < GROUND && b.surfs.get(surf as usize).is_some_and(|s| s.flicker) {
                    *h = h.map(|v| v * level);
                }
            }
        }
        for ((l, map), shows) in lights.iter().zip(maps).zip(lit_by.iter_mut()) {
            let k = if l.flicker { level } else { 1.0 };
            if k <= 0.0 || l.color == [0.0; 3] {
                continue;
            }
            let (x0, y0, x1, y1) = self.reach_rect(l);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = y * self.w + x;
                    if self.piece[i] == SKY {
                        continue;
                    }
                    let p = pos[i];
                    let nf = self.nrm[i];
                    let n = [nf[0] as f64, nf[1] as f64, nf[2] as f64];
                    // light that bounces off what the lamp lights, around it (the engine's GI)
                    if let (Some(b), Beam::Point | Beam::Spot { .. }) = (env.bounce, l.beam) {
                        let dist = g::length(g::sub(l.at, p));
                        if l.range <= 0.0 || dist < l.range {
                            let m = dist / 100.0;
                            let r = if l.range > 0.0 { dist / l.range } else { 0.0 };
                            let f = (1.0 - r.powi(4)).clamp(0.0, 1.0).powi(2) / (m * m).max(1.0);
                            let e = b * f as f32 * k;
                            if e * l.color[0].max(l.color[1]).max(l.color[2]) > 1e-4 {
                                *shows = true;
                            }
                            for (v, c) in irr[i].iter_mut().zip(l.color) {
                                *v += c * e;
                            }
                        }
                    }
                    let (dir, falloff) = match l.beam {
                        Beam::Sun { toward } => (unit(toward), 1.0),
                        _ => {
                            let d = g::sub(l.at, p);
                            let dist = g::length(d);
                            if dist < 1e-6 || (l.range > 0.0 && dist >= l.range) {
                                continue;
                            }
                            let m = dist / 100.0;
                            let mut f = 1.0 / (m * m).max(0.01);
                            if l.range > 0.0 {
                                let r = dist / l.range;
                                f *= (1.0 - r * r * r * r).clamp(0.0, 1.0).powi(2);
                            }
                            if let Beam::Spot { dir: sd, cos_outer, cos_inner } = l.beam {
                                let c = -dot(g::scale(d, 1.0 / dist), sd);
                                f *= if env.squared_cone {
                                    ((c - cos_outer) / (cos_inner - cos_outer).max(1e-4)).clamp(0.0, 1.0).powi(2)
                                } else {
                                    smooth(cos_outer, cos_inner, c)
                                };
                            }
                            (g::scale(d, 1.0 / dist), f)
                        }
                    };
                    let ndl = dot(n, dir);
                    if ndl <= 0.0 || falloff <= 0.0 {
                        continue;
                    }
                    let shadow = map.as_ref().map(|m| m.reach(p, n)).unwrap_or(1.0);
                    if shadow <= 0.0 {
                        continue;
                    }
                    let e = (ndl * falloff) as f32 * shadow * k;
                    if e * l.color[0].max(l.color[1]).max(l.color[2]) > 1e-4 {
                        *shows = true;
                    }
                    for (v, c) in irr[i].iter_mut().zip(l.color) {
                        *v += c * e;
                    }
                    // the lamp's glint (three's GGX with a Schlick Fresnel)
                    let (rough, f0) = gloss[i];
                    let v = unit(g::sub(self.view.eye, p));
                    let hv = unit(g::add(dir, v));
                    let (ndv, ndh, vdh) = (dot(n, v).max(1e-3) as f32, dot(n, hv).max(0.0) as f32, dot(v, hv).max(0.0) as f32);
                    let a2 = (rough * rough).powi(2);
                    let d = a2 / (std::f32::consts::PI * (ndh * ndh * (a2 - 1.0) + 1.0).powi(2));
                    let nl = ndl as f32;
                    let vis = 0.5 / (nl * (ndv * ndv * (1.0 - a2) + a2).sqrt() + ndv * (nl * nl * (1.0 - a2) + a2).sqrt()).max(1e-6);
                    let fr = (1.0 - vdh).powi(5);
                    for c in 0..3 {
                        hdr[i][c] += l.color[c] * e * (f0[c] + (1.0 - f0[c]) * fr) * vis * d;
                    }
                }
            }
        }
        let inv_pi = 1.0 / std::f32::consts::PI;
        for i in 0..n_px {
            if self.piece[i] == SKY {
                continue;
            }
            for c in 0..3 {
                hdr[i][c] += alb[i][c] * inv_pi * irr[i][c];
            }
            if let Some(fog) = &env.fog {
                let p = pos[i];
                let f = fog.amount(g::length(g::sub(p, self.view.eye)), self.view.eye[2], p[2]);
                hdr[i] = lerp(hdr[i], fog.color(), f);
            }
        }
        for (l, shows) in lights.iter().zip(lit_by.iter_mut()) {
            let k = if l.flicker { level } else { 1.0 };
            if l.haze_beam > 0.0 && self.beam(&mut hdr, l, k) {
                *shows = true;
            }
            if let Some((size, opacity, color)) = l.halo {
                if self.halo(&mut hdr, l.at, size, opacity * k, color) {
                    *shows = true;
                }
            }
        }
        // bloom, vignette, exposure and tone, as the page finishes a frame
        if env.bloom.0 > 0.0 {
            let glow = bloom(&hdr, self.w, self.h, env.bloom.1);
            for (c, b) in hdr.iter_mut().zip(glow) {
                for k in 0..3 {
                    c[k] += b[k] * env.bloom.0;
                }
            }
        }
        let exposure = match env.auto_exposure {
            Some(ae) => auto_exposure(&hdr, &self.piece, self.w, self.h, ae),
            None => env.exposure,
        };
        let aspect = self.w as f64 / self.h as f64;
        for (i, (out, c)) in self.col.iter_mut().zip(&hdr).enumerate() {
            let (x, y) = (i % self.w, i / self.w);
            let (dx, dy) = ((x as f64 + 0.5) / self.w as f64 - 0.5, (y as f64 + 0.5) / self.h as f64 - 0.5);
            let v = ((dx * aspect).powi(2) + dy * dy).sqrt() * 1.1;
            let vig = 1.0 + (smooth(1.15, 0.2, v) as f32 - 1.0) * env.vignette;
            let t = aces(c.map(|x| x * vig * exposure));
            *out = t.map(linear_to_srgb);
        }
        lit_by
    }

    /// Whether a lamp can light anything in the frame.
    pub fn reaches(&self, l: &Light) -> bool {
        let (x0, y0, x1, y1) = self.reach_rect(l);
        x0 < x1 && y0 < y1
    }

    /// The pixels a lamp can reach: the screen box of what it can light (the ball of its range,
    /// or a spot's cone), or all of it.
    fn reach_rect(&self, l: &Light) -> (usize, usize, usize, usize) {
        let all = (0, 0, self.w, self.h);
        if l.range <= 0.0 || matches!(l.beam, Beam::Sun { .. }) {
            return all;
        }
        let r = l.range;
        let (lo, hi): (Vec3, Vec3) = match l.beam {
            // the cone up to `range` along its axis lies within the apex and the disc there
            Beam::Spot { dir, cos_outer, .. } if cos_outer > 80f64.to_radians().cos() => {
                let rb = cos_outer.acos().tan() * r;
                let c = g::add(l.at, g::scale(dir, r));
                let ext: Vec3 = std::array::from_fn(|k| rb * (1.0 - dir[k] * dir[k]).max(0.0).sqrt());
                (std::array::from_fn(|k| l.at[k].min(c[k] - ext[k])), std::array::from_fn(|k| l.at[k].max(c[k] + ext[k])))
            }
            _ => (g::sub(l.at, [r; 3]), g::add(l.at, [r; 3])),
        };
        let e = self.view.eye;
        if (0..3).all(|k| e[k] >= lo[k] - NEAR && e[k] <= hi[k] + NEAR) {
            return all;
        }
        let mut cam = Vec::with_capacity(8);
        for x in [lo[0], hi[0]] {
            for y in [lo[1], hi[1]] {
                for z in [lo[2], hi[2]] {
                    cam.push(self.view.cam([x, y, z]));
                }
            }
        }
        if cam.iter().all(|c| c[2] < NEAR) {
            return (0, 0, 0, 0);
        }
        if cam.iter().any(|c| c[2] < NEAR) {
            return all;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for c in &cam {
            let p = self.view.screen(*c);
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        let cl = |v: f64, m: usize| v.clamp(0.0, m as f64) as usize;
        (cl(x0.floor(), self.w), cl(y0.floor(), self.h), cl(x1.ceil() + 1.0, self.w), cl(y1.ceil() + 1.0, self.h))
    }

    /// The cone of a spot seen in the haze: brightest at the lamp and toward its axis, behind
    /// whatever stands in front of it (the page's `beam`).
    fn beam(&self, hdr: &mut [Col], l: &Light, k: f32) -> bool {
        let Beam::Spot { dir, cos_outer, .. } = l.beam else { return false };
        let mut drew = false;
        let len = if l.range > 0.0 { l.range.min(1200.0) } else { 600.0 };
        let r1 = cos_outer.clamp(-1.0, 1.0).acos().tan() * len;
        const SEG: usize = 24;
        let side = unit(cross(if dir[2].abs() > 0.95 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] }, dir));
        let up = cross(dir, side);
        let ring = |j: usize, r: f64, along: f64| {
            let a = j as f64 / SEG as f64 * std::f64::consts::TAU;
            let off = g::add(g::scale(side, a.cos() * r), g::scale(up, a.sin() * r));
            let p = g::add(g::add(l.at, g::scale(dir, along)), off);
            let n = unit(g::sub(off, g::scale(dir, r)));
            (p, n)
        };
        let color = l.halo.map(|h| h.2).unwrap_or([1.0; 3]);
        for j in 0..SEG {
            let quad = [ring(j, 0.3, 0.0), ring(j + 1, 0.3, 0.0), ring(j + 1, r1, len), ring(j, r1, len)];
            let along = [0.0, 0.0, 1.0, 1.0];
            for (ia, ib, ic) in [(0, 1, 2), (0, 2, 3)] {
                let cs = [self.view.cam(quad[ia].0), self.view.cam(quad[ib].0), self.view.cam(quad[ic].0)];
                if outside(&self.view, &cs, self.view.cx, self.view.cy) || cs.iter().any(|c| c[2] < NEAR) {
                    continue;
                }
                let s = [self.view.screen(cs[0]), self.view.screen(cs[1]), self.view.screen(cs[2])];
                let ns = [quad[ia].1, quad[ib].1, quad[ic].1];
                let al = [along[ia], along[ib], along[ic]];
                let ps = [quad[ia].0, quad[ib].0, quad[ic].0];
                let (w, h) = (self.w, self.h);
                raster(w, h, s[0], s[1], s[2], |i, l0, l1, l2| {
                    let z = l0 * s[0][2] + l1 * s[1][2] + l2 * s[2][2];
                    if (z as f32) < self.inv[i] {
                        return;
                    }
                    let (w0, w1, w2) = (l0 * s[0][2] / z, l1 * s[1][2] / z, l2 * s[2][2] / z);
                    let n = unit(std::array::from_fn(|c| ns[0][c] * w0 + ns[1][c] * w1 + ns[2][c] * w2));
                    let p: Vec3 = std::array::from_fn(|c| ps[0][c] * w0 + ps[1][c] * w1 + ps[2][c] * w2);
                    let v = unit(g::sub(self.view.eye, p));
                    let along = al[0] * w0 + al[1] * w1 + al[2] * w2;
                    let edge = dot(n, v).abs().powf(2.2);
                    let a = l.haze_beam as f64 * edge * (1.0 - along).powf(1.4) * smooth(0.0, 0.06, along) * 0.775 * k as f64;
                    if a > 1e-4 {
                        drew = true;
                    }
                    for c in 0..3 {
                        hdr[i][c] += color[c] * a as f32;
                    }
                });
            }
        }
        drew
    }

    /// A soft round glow around a lamp, in front of what it is in front of (the page's halo).
    fn halo(&self, hdr: &mut [Col], at: Vec3, size: f64, opacity: f32, color: Col) -> bool {
        let Some([sx, sy, inv]) = self.view.project(at) else { return false };
        let r = size / 2.0 * self.view.fx * inv;
        if r < 0.5 || opacity <= 0.0 {
            return false;
        }
        let mut drew = false;
        let (x0, x1) = ((sx - r).floor().max(0.0) as usize, ((sx + r).ceil().max(0.0) as usize).min(self.w));
        let (y0, y1) = ((sy - r).floor().max(0.0) as usize, ((sy + r).ceil().max(0.0) as usize).min(self.h));
        for y in y0..y1 {
            for x in x0..x1 {
                let i = y * self.w + x;
                if (inv as f32) < self.inv[i] {
                    continue;
                }
                let d = (((x as f64 + 0.5 - sx).powi(2) + (y as f64 + 0.5 - sy).powi(2)).sqrt() / r) as f32;
                if d >= 1.0 {
                    continue;
                }
                // the page's radial(0.18): 1 at the centre, 0.5 at 0.18, 0 at the rim
                let a = if d < 0.18 { 1.0 - d / 0.18 * 0.5 } else { 0.5 * (1.0 - (d - 0.18) / 0.82) };
                drew = true;
                for c in 0..3 {
                    hdr[i][c] += color[c] * a * opacity;
                }
            }
        }
        drew
    }

    // ----------------------------------------------------------------------- ink

    /// Ink where one piece meets another (or the sky) and where a surface folds, two pixels
    /// wide: dark on lit surfaces, light in the dark so shapes still read at night.
    pub fn outlines(&mut self) {
        let (w, h) = (self.w, self.h);
        let mut ink = vec![false; w * h];
        let edge = |a: usize, b: usize| {
            let (pa, pb) = (self.piece[a], self.piece[b]);
            if pa >= GROUND && pb >= GROUND {
                return false;
            }
            if pa != pb {
                return true;
            }
            let (na, nb) = (self.nrm[a], self.nrm[b]);
            if na[0] * nb[0] + na[1] * nb[1] + na[2] * nb[2] < 0.8 {
                return true;
            }
            let (ia, ib) = (self.inv[a], self.inv[b]);
            (ia - ib).abs() > 0.2 * ia.max(ib)
        };
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if x + 1 < w && edge(i, i + 1) {
                    ink[i] = true;
                    ink[i + 1] = true;
                }
                if y + 1 < h && edge(i, i + w) {
                    ink[i] = true;
                    ink[i + w] = true;
                }
            }
        }
        for (c, on) in self.col.iter_mut().zip(ink) {
            if on {
                let lum = 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
                *c = if lum < 0.22 { lerp(*c, [0.62, 0.66, 0.72], 0.32) } else { lerp(*c, [0.05, 0.05, 0.07], 0.72) };
            }
        }
    }

    /// A soft warm glow where a lamp is in sight (the graybox's lamps).
    pub fn glow(&mut self, at: Vec3, ss: f64) {
        let Some([sx, sy, inv]) = self.view.project(at) else { return };
        let (xi, yi) = (sx.floor(), sy.floor());
        if xi < 0.0 || yi < 0.0 || xi >= self.w as f64 || yi >= self.h as f64 {
            return;
        }
        // in front of what covers its pixel, or just behind it (the lamp inside its fitting)
        let cover = self.inv[yi as usize * self.w + xi as usize] as f64;
        if inv < cover * 0.9 {
            return;
        }
        let r = (self.view.fx * 35.0 * inv).clamp(2.5 * ss, 28.0 * ss);
        let warm = [1.0f32, 0.86, 0.55];
        let (x0, x1) = ((sx - r).floor().max(0.0) as usize, ((sx + r).ceil() as usize).min(self.w - 1));
        let (y0, y1) = ((sy - r).floor().max(0.0) as usize, ((sy + r).ceil() as usize).min(self.h - 1));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f64 + 0.5 - sx).powi(2) + (y as f64 + 0.5 - sy).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let a = ((1.0 - d) * (1.0 - d) * 0.9) as f32;
                    let c = &mut self.col[y * self.w + x];
                    for k in 0..3 {
                        c[k] = (c[k] + warm[k] * a).min(1.0);
                    }
                }
            }
        }
    }

    /// A world-space segment, solid where it is in sight and dashed and faint where something
    /// stands in front of it.
    pub fn wire(&mut self, a: Vec3, b: Vec3, color: Col, ss: usize) {
        let (mut ca, mut cb) = (self.view.cam(a), self.view.cam(b));
        if ca[2] < NEAR && cb[2] < NEAR {
            return;
        }
        if ca[2] < NEAR {
            let t = (NEAR - ca[2]) / (cb[2] - ca[2]);
            ca = [ca[0] + (cb[0] - ca[0]) * t, ca[1] + (cb[1] - ca[1]) * t, NEAR];
        } else if cb[2] < NEAR {
            let t = (NEAR - cb[2]) / (ca[2] - cb[2]);
            cb = [cb[0] + (ca[0] - cb[0]) * t, cb[1] + (ca[1] - cb[1]) * t, NEAR];
        }
        let (sa, sb) = (self.view.screen(ca), self.view.screen(cb));
        let Some((t0, t1)) = super::clip_t(sa[0], sa[1], sb[0], sb[1], self.w as f64, self.h as f64) else { return };
        let len = ((sb[0] - sa[0]).powi(2) + (sb[1] - sa[1]).powi(2)).sqrt() * (t1 - t0);
        let n = len.ceil().max(1.0) as usize;
        let dash = 4 * ss;
        for k in 0..=n {
            let t = t0 + (t1 - t0) * k as f64 / n as f64;
            let (x, y) = (sa[0] + (sb[0] - sa[0]) * t, sa[1] + (sb[1] - sa[1]) * t);
            let inv = sa[2] + (sb[2] - sa[2]) * t;
            let (xi, yi) = (x.floor().clamp(0.0, self.w as f64 - 1.0) as usize, y.floor().clamp(0.0, self.h as f64 - 1.0) as usize);
            let shown = inv as f32 >= self.inv[yi * self.w + xi] * 0.985;
            if !shown && (k / dash) % 2 == 1 {
                continue;
            }
            let a = if shown { 1.0 } else { 0.3 };
            for dy in 0..ss {
                for dx in 0..ss {
                    let (px, py) = (xi + dx, yi + dy);
                    if px < self.w && py < self.h {
                        let c = &mut self.col[py * self.w + px];
                        *c = lerp(*c, color, a);
                    }
                }
            }
        }
    }

    /// Lays a colour over the whole frame (the engine stand-in's tint).
    pub fn tint(&mut self, c: Col, a: f32) {
        for p in &mut self.col {
            *p = lerp(*p, c, a);
        }
    }

    /// Averages each ss x ss block into the canvas.
    pub fn resolve(&self, ss: usize) -> Canvas {
        let (w, h) = (self.w / ss, self.h / ss);
        let mut cv = Canvas::new(w as u32, h as u32);
        let k = 1.0 / (ss * ss) as f32;
        for y in 0..h {
            for x in 0..w {
                let mut s = [0.0f32; 3];
                for dy in 0..ss {
                    let row = (y * ss + dy) * self.w + x * ss;
                    for c in &self.col[row..row + ss] {
                        s[0] += c[0];
                        s[1] += c[1];
                        s[2] += c[2];
                    }
                }
                let i = (y * w + x) * 3;
                for (p, v) in cv.px[i..i + 3].iter_mut().zip(s) {
                    *p = ((v * k).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
            }
        }
        cv
    }
}

/// The scale UE's eye adaptation settles on for a frame: the centre-weighted mean of log
/// luminance between the darkest 10% and the brightest 10%, brought to EV100 (K = 12.5) and
/// clamped, then exposure = 2^bias / (1.2 * 2^EV100).
fn auto_exposure(hdr: &[Col], piece: &[u32], w: usize, h: usize, ae: AutoExposure) -> f32 {
    let mut logs: Vec<(f32, f32)> = Vec::with_capacity(hdr.len() / 4);
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let i = y * w + x;
            let c = hdr[i];
            let l = (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]).max(1e-4);
            let (dx, dy) = (x as f32 / w as f32 - 0.5, y as f32 / h as f32 - 0.5);
            let weight = if piece[i] == SKY { 0.5 } else { 1.0 } * (1.0 - (dx * dx + dy * dy).sqrt()).max(0.2);
            logs.push((l.log2(), weight));
        }
    }
    if logs.is_empty() {
        return 1.0;
    }
    logs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (lo, hi) = (logs.len() / 10, logs.len() - logs.len() / 10);
    let (s, wsum) = logs[lo..hi.max(lo + 1)].iter().fold((0.0, 0.0), |(s, ws), (l, w)| (s + l * w, ws + w));
    let mean = s / wsum.max(1e-6);
    let ev = (mean + (100.0f32 / 12.5).log2()).clamp(ae.min, ae.max);
    let own = 2f32.powf(ae.bias) / (1.2 * 2f32.powf(ev));
    match ae.around {
        Some(a) => own.clamp(a / 2.83, a * 2.83),
        None => own,
    }
}

/// The over-bright part of the frame, spread wide (the page's bloom, cheaply).
fn bloom(hdr: &[Col], w: usize, h: usize, threshold: f32) -> Vec<Col> {
    const D: usize = 4;
    let (bw, bh) = (w.div_ceil(D), h.div_ceil(D));
    let mut small = vec![[0.0f32; 3]; bw * bh];
    for y in 0..h {
        for x in 0..w {
            let c = hdr[y * w + x];
            let l = c[0].max(c[1]).max(c[2]).min(60.0);
            if l <= threshold {
                continue;
            }
            let k = (l - threshold) / l / (D * D) as f32;
            let s = &mut small[(y / D) * bw + x / D];
            for i in 0..3 {
                s[i] += c[i].min(60.0) * k;
            }
        }
    }
    // a wide soft blur: three box passes each way
    for _ in 0..3 {
        small = blur(&small, bw, bh, 3, true);
        small = blur(&small, bw, bh, 3, false);
    }
    (0..w * h).map(|i| small[(i / w / D) * bw + (i % w) / D]).collect()
}

fn blur(src: &[Col], w: usize, h: usize, r: usize, across: bool) -> Vec<Col> {
    let mut out = vec![[0.0f32; 3]; w * h];
    let k = 1.0 / (2 * r + 1) as f32;
    for y in 0..h {
        for x in 0..w {
            let mut s = [0.0f32; 3];
            for d in 0..=2 * r {
                let (sx, sy) = if across { ((x + d).saturating_sub(r).min(w - 1), y) } else { (x, (y + d).saturating_sub(r).min(h - 1)) };
                let c = src[sy * w + sx];
                for i in 0..3 {
                    s[i] += c[i];
                }
            }
            out[y * w + x] = [s[0] * k, s[1] * k, s[2] * k];
        }
    }
    out
}

pub(crate) fn lerp(a: Col, b: Col, t: f32) -> Col {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// A linear colour as shown.
pub(crate) fn shown(c: Col) -> Col {
    [linear_to_srgb(c[0].max(0.0)), linear_to_srgb(c[1].max(0.0)), linear_to_srgb(c[2].max(0.0))]
}

#[cfg(test)]
mod tests {
    use super::super::mesh::{Geo, Xf};
    use super::super::scene::Shadow;
    use super::*;
    use crate::geometry::Rot;

    /// A floor 6 m square, a block hanging 1.25 m over its middle, and a spot 3 m up.
    fn room() -> (Builder, Light) {
        let mut b = Builder::default();
        let s = b.surf(Surf::plain([0.5; 3]));
        let floor = b.piece(None);
        b.add(&Geo::cube([600.0, 600.0, 2.0]), &Xf::ue([0.0, 0.0, -1.0], Rot::default(), [1.0; 3]), s, floor);
        let block = b.piece(None);
        b.add(&Geo::cube([60.0, 60.0, 60.0]), &Xf::ue([0.0, 0.0, 125.0], Rot::default(), [1.0; 3]), s, block);
        let lamp = Light {
            at: [0.0, 0.0, 300.0],
            beam: Beam::Spot { dir: [0.0, 0.0, -1.0], cos_outer: 40f64.to_radians().cos(), cos_inner: 30f64.to_radians().cos() },
            color: [100.0; 3],
            range: 1000.0,
            shadow: Some(Shadow { size: 256, area: None }),
            flicker: false,
            halo: None,
            haze_beam: 0.0,
            src: None,
            fitting: 0.0,
        };
        (b, lamp)
    }

    #[test]
    fn a_block_under_a_lamp_shades_the_floor_below_it_and_nowhere_else() {
        let (b, lamp) = room();
        let map = ShadowMap::build(&lamp, &b.tris, ([0.0; 3], 1000.0)).unwrap();
        let up = [0.0, 0.0, 1.0];
        assert_eq!(map.reach([0.0, 0.0, 0.0], up), 0.0, "under the block");
        assert_eq!(map.reach([150.0, 0.0, 0.0], up), 1.0, "beside it");
        assert_eq!(map.reach([0.0, 0.0, 155.0], up), 1.0, "on top of the block");
    }

    #[test]
    fn a_shade_just_under_a_ue_lamp_casts_no_shadow_but_one_in_three_does() {
        let (mut b, lamp) = room();
        // a disc-like shade 10 cm under the lamp, wider than the block's shadow
        let s = b.surf(Surf::plain([0.5; 3]));
        let shade = b.piece(None);
        b.add(&Geo::cube([60.0, 60.0, 4.0]), &Xf::ue([0.0, 0.0, 290.0], Rot::default(), [1.0; 3]), s, shade);
        let up = [0.0, 0.0, 1.0];
        let three = ShadowMap::build(&lamp, &b.tris, ([0.0; 3], 1000.0)).unwrap();
        assert_eq!(three.reach([150.0, 0.0, 0.0], up), 0.0, "three's shade shadows the floor");
        let ue = ShadowMap::build(&Light { fitting: 40.0, ..lamp }, &b.tris, ([0.0; 3], 1000.0)).unwrap();
        assert_eq!(ue.reach([150.0, 0.0, 0.0], up), 1.0, "a UE fitting does not");
        assert_eq!(ue.reach([0.0, 0.0, 0.0], up), 0.0, "the block still does");
    }

    #[test]
    fn the_lamp_lights_its_pool_and_the_frame_outside_its_cone_stays_dark() {
        let (b, lamp) = room();
        // looking down at the floor from one side, the block out of the way of the view's middle
        let cam = Xform::new([-500.0, 0.0, 400.0], Rot::new(-50.0, 0.0, 0.0));
        let (w, h) = (96, 54);
        let mut fr = Frame::new(View::new(&cam, 24.0, 36.0, w, h), w, h);
        fr.draw(&b.tris);
        let env = Env {
            ambient_sky: [0.0; 3],
            ambient_ground: [0.0; 3],
            sky_top: [0.0; 3],
            sky_horizon: [0.0; 3],
            moon: None,
            fog: None,
            exposure: 1.0,
            auto_exposure: None,
            bloom: (0.0, 1.0),
            vignette: 0.0,
            flicker: None,
            ground: None,
            sky_cover: None,
            bounce: None,
            squared_cone: false,
        };
        let map = ShadowMap::build(&lamp, &b.tris, ([0.0; 3], 1000.0));
        fr.shade_lit(&b, &[lamp], &[map], None, &env, 0.0);
        let lum = |p: Vec3| {
            let s = fr.view.project(p).unwrap();
            let c = fr.col[s[1] as usize * w + s[0] as usize];
            c[0] + c[1] + c[2]
        };
        let pool = lum([-150.0, 120.0, 0.0]);
        assert!(pool > 0.5, "lit floor {pool}");
        assert!(lum([-150.0, 0.0, 0.0]) > 0.5, "lit floor in front of the block");
        assert!(lum([200.0, 200.0, 0.0]) < 0.05 * pool, "outside the cone");
    }
}
