//! The light a lit previz frame is drawn with: the level's lamps, its sky and fill, its haze
//! and how bright the picture is made (exposure), as the game itself has them.

use crate::geometry::Vec3;

use super::mesh::{unit, Col};

/// How a lamp throws its light.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Beam {
    Point,
    /// `dir` is where it points; the light fades from `cos_inner` to nothing at `cos_outer`.
    Spot { dir: Vec3, cos_outer: f64, cos_inner: f64 },
    /// From far away along `toward` (the direction to the light): the sun or the moon.
    Sun { toward: Vec3 },
}

/// A shadow map's size and, for a sun, the part of the world it covers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shadow {
    pub size: usize,
    /// (center, half width) in cm, for a sun; None follows the camera.
    pub area: Option<(Vec3, f64)>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Light {
    pub at: Vec3,
    pub beam: Beam,
    /// Linear colour times intensity: candela for lamps (at metres), lux for a sun.
    pub color: Col,
    /// Distance (cm) where its light ends; 0 for none.
    pub range: f64,
    pub shadow: Option<Shadow>,
    pub flicker: bool,
    /// A soft glow around the lamp: (size in cm, opacity, its colour).
    pub halo: Option<(f64, f32, Col)>,
    /// A cone of light seen in the haze (spots only): its opacity.
    pub haze_beam: f32,
    /// The source it comes from (an index into the world's sources): a frame it lights
    /// depends on it.
    pub src: Option<usize>,
    /// How near the lamp (cm) its fitting is, which casts no shadow (UE's shades and bulbs;
    /// three's shades do cast theirs, so 0 there).
    pub fitting: f64,
}

/// Haze that thickens with distance and toward the floor.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Fog {
    /// three's: a thin haze everywhere (`density` per metre) and a thicker layer `ground` on
    /// the floor at `floor` cm that thins out over `height` metres.
    Three { color: Col, density: f64, ground: f64, height: f64, floor: f64 },
    /// UE's exponential height fog: `density` per cm at height `z`, falling off by `falloff`
    /// per cm (base 2, as the engine's shader), nothing nearer than `start`, at most `max`.
    Ue { color: Col, density: f64, falloff: f64, z: f64, start: f64, max: f32 },
}

impl Fog {
    pub fn color(&self) -> Col {
        match self {
            Fog::Three { color, .. } | Fog::Ue { color, .. } => *color,
        }
    }

    /// How much of the haze colour covers a point `dist` cm away seen from `eye_z` to `z`.
    pub fn amount(&self, dist: f64, eye_z: f64, z: f64) -> f32 {
        match *self {
            Fog::Three { density, ground, height, floor, .. } => {
                let (y0, y1) = (((eye_z - floor) / 100.0).max(0.0), ((z - floor) / 100.0).max(0.0));
                let h = height.max(1e-3);
                let (e0, e1) = ((-y0 / h).exp(), (-y1 / h).exp());
                let layer = if (y1 - y0).abs() > 1e-3 { h * (e0 - e1) / (y1 - y0) } else { e0 };
                (1.0 - (-(dist / 100.0) * (density + ground * layer)).exp()) as f32
            }
            Fog::Ue { density, falloff, z: fz, start, max, .. } => {
                let len = (dist - start).max(0.0);
                if len <= 0.0 || dist <= 0.0 {
                    return 0.0;
                }
                let origin = density * (-falloff * (eye_z - fz)).exp2();
                let f = (falloff * (z - eye_z)).max(-127.0);
                let line = if f.abs() > 1e-6 { (1.0 - (-f).exp2()) / f } else { std::f64::consts::LN_2 };
                let integral = origin * line * len;
                ((1.0 - (-integral).exp2()) as f32).min(max)
            }
        }
    }
}

/// Exposure the engine picks for itself: the frame's average brightness brought to middle
/// grey (log2 EV100, kept between `min` and `max`), then `bias` stops over.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AutoExposure {
    pub bias: f32,
    pub min: f32,
    pub max: f32,
    /// The exposure the level's lamps call for: a frame's own stays within 1.5 stops of it, as
    /// the engine's eye adaptation only gets so far in a shot's warm-up (a dark corridor with
    /// no lamp in view stays dark).
    pub around: Option<f32>,
}

/// When the flickering lamps dip: the page's `level(t)`.
#[derive(Debug, Clone)]
pub(crate) struct Flicker {
    pub offset: f64,
    pub period: f64,
    pub dips: Vec<(f64, f64)>,
    pub low: f32,
}

impl Flicker {
    pub fn level(&self, t: f64) -> f32 {
        let ph = ((t + self.offset) % self.period + self.period) % self.period;
        if self.dips.iter().any(|&(a, b)| ph >= a && ph < b) {
            self.low
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Env {
    /// Fill from the sky above and the ground below (irradiance).
    pub ambient_sky: Col,
    pub ambient_ground: Col,
    pub sky_top: Col,
    pub sky_horizon: Col,
    /// The moon in the sky: direction to it, colour, angular size (radians).
    pub moon: Option<(Vec3, Col, f64)>,
    pub fog: Option<Fog>,
    pub exposure: f32,
    /// When set, the exposure follows each frame's brightness instead (UE's eye adaptation).
    pub auto_exposure: Option<AutoExposure>,
    /// Bloom strength and the radiance it starts from.
    pub bloom: (f32, f32),
    pub vignette: f32,
    pub flicker: Option<Flicker>,
    /// What the ground plane under the level is made of, when there is one (UE levels).
    pub ground: Option<Col>,
    /// What is left of the sky's fill under a roof, when the engine shades it there (UE's sky
    /// light is occluded indoors; three's hemisphere light is not).
    pub sky_cover: Option<f32>,
    /// How much of a lamp's light comes back off the room around it (UE's global illumination;
    /// three has none).
    pub bounce: Option<f32>,
    /// A spot fades as UE's do (the square of a straight ramp between its cones) rather than
    /// three's smoothstep.
    pub squared_cone: bool,
}

impl Env {
    pub fn level(&self, t: f64) -> f32 {
        self.flicker.as_ref().map(|f| f.level(t)).unwrap_or(1.0)
    }

    pub fn ambient(&self, n: [f32; 3]) -> Col {
        let w = 0.5 + 0.5 * n[2];
        let (s, g) = (self.ambient_sky, self.ambient_ground);
        [g[0] + (s[0] - g[0]) * w, g[1] + (s[1] - g[1]) * w, g[2] + (s[2] - g[2]) * w]
    }

    /// The sky's radiance along a (UE) direction.
    pub fn sky(&self, d: Vec3) -> Col {
        let d = unit(d);
        let up = d[2];
        let s = smooth(-0.05, 0.55, up) as f32;
        let (t, h) = (self.sky_top, self.sky_horizon);
        let mut c = [h[0] + (t[0] - h[0]) * s, h[1] + (t[1] - h[1]) * s, h[2] + (t[2] - h[2]) * s];
        if let Some((dir, mc, size)) = self.moon {
            let m = super::mesh::dot(d, dir);
            let disc = smooth(size.cos(), (size * 0.8).cos(), m) * 6.0;
            let glow = m.max(0.0).powi(400) * 0.6 + m.max(0.0).powi(24) * 0.06;
            for k in 0..3 {
                c[k] += mc[k] * (disc + glow) as f32;
            }
        }
        c
    }
}

pub(crate) fn smooth(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// ACES as the page tone maps (its `aces()`, which divides by 0.6 first).
pub(crate) fn aces(c: Col) -> Col {
    let c = [c[0] / 0.6, c[1] / 0.6, c[2] / 0.6];
    // GLSL mat3 columns: I * c
    let i = [
        0.59719 * c[0] + 0.35458 * c[1] + 0.04823 * c[2],
        0.07600 * c[0] + 0.90834 * c[1] + 0.01566 * c[2],
        0.02840 * c[0] + 0.13383 * c[1] + 0.83777 * c[2],
    ];
    let f = |x: f32| (x * (x + 0.024_578_6) - 0.000_090_537) / (x * (0.983_729 * x + 0.432_951) + 0.238_081);
    let r = [f(i[0]), f(i[1]), f(i[2])];
    [
        (1.60475 * r[0] - 0.53108 * r[1] - 0.07367 * r[2]).clamp(0.0, 1.0),
        (-0.10208 * r[0] + 1.10813 * r[1] - 0.00605 * r[2]).clamp(0.0, 1.0),
        (-0.00327 * r[0] - 0.07276 * r[1] + 1.07602 * r[2]).clamp(0.0, 1.0),
    ]
}
