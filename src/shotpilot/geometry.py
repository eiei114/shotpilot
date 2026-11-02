"""UE-convention camera math (left-handed, Z up, X forward, Y right, rotators in degrees).

Kept dependency-free so it can be unit tested without the engine.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

Vec3 = tuple[float, float, float]


@dataclass(frozen=True)
class Rot:
    pitch: float = 0.0
    yaw: float = 0.0
    roll: float = 0.0

    def as_list(self) -> list[float]:
        return [self.pitch, self.yaw, self.roll]


@dataclass(frozen=True)
class Xform:
    location: Vec3
    rotation: Rot

    def to_dict(self) -> dict:
        return {"location": [_r(v) for v in self.location], "rotation": [_r(v) for v in self.rotation.as_list()]}


def _r(v: float) -> float:
    # Round so fingerprints are stable across float noise (0.01 cm / 0.001 deg is far below visible).
    return round(float(v), 3)


def add(a: Vec3, b: Vec3) -> Vec3:
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def sub(a: Vec3, b: Vec3) -> Vec3:
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def scale(a: Vec3, s: float) -> Vec3:
    return (a[0] * s, a[1] * s, a[2] * s)


def length(a: Vec3) -> float:
    return math.sqrt(a[0] ** 2 + a[1] ** 2 + a[2] ** 2)


def look_at(eye: Vec3, target: Vec3, roll: float = 0.0) -> Rot:
    d = sub(target, eye)
    horiz = math.hypot(d[0], d[1])
    if horiz < 1e-6 and abs(d[2]) < 1e-6:
        return Rot(0.0, 0.0, roll)
    yaw = math.degrees(math.atan2(d[1], d[0]))
    pitch = math.degrees(math.atan2(d[2], horiz))
    return Rot(pitch, yaw, roll)


def forward(r: Rot) -> Vec3:
    cp, sp = math.cos(math.radians(r.pitch)), math.sin(math.radians(r.pitch))
    cy, sy = math.cos(math.radians(r.yaw)), math.sin(math.radians(r.yaw))
    return (cp * cy, cp * sy, sp)


def right(r: Rot) -> Vec3:
    # Ignores roll on purpose: trucking follows the ground plane, not a tilted horizon.
    cy, sy = math.cos(math.radians(r.yaw)), math.sin(math.radians(r.yaw))
    return (-sy, cy, 0.0)


def up(r: Rot) -> Vec3:
    f, rt = forward(r), right(r)
    # up = forward x right (with Y as "right" this yields +Z for a level camera)
    return (
        f[1] * rt[2] - f[2] * rt[1],
        f[2] * rt[0] - f[0] * rt[2],
        f[0] * rt[1] - f[1] * rt[0],
    )


def orbit(eye: Vec3, pivot: Vec3, degrees: float) -> Vec3:
    """Rotate eye around pivot about the world Z axis. Positive = counter-clockwise seen from above."""
    a = math.radians(degrees)
    dx, dy = eye[0] - pivot[0], eye[1] - pivot[1]
    return (
        pivot[0] + dx * math.cos(a) - dy * math.sin(a),
        pivot[1] + dx * math.sin(a) + dy * math.cos(a),
        eye[2],
    )


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def lerp_angle(a: float, b: float, t: float) -> float:
    d = (b - a + 180.0) % 360.0 - 180.0
    return a + d * t


def ease(t: float, curve: str) -> float:
    if curve == "linear":
        return t
    if curve == "ease_in":
        return t * t
    if curve == "ease_out":
        return 1 - (1 - t) * (1 - t)
    # ease_in_out (smoothstep) is the default: it reads as a deliberate camera move.
    return t * t * (3 - 2 * t)


def interpolate(a: Xform, b: Xform, t: float) -> Xform:
    loc = tuple(lerp(a.location[i], b.location[i], t) for i in range(3))
    rot = Rot(
        lerp_angle(a.rotation.pitch, b.rotation.pitch, t),
        lerp_angle(a.rotation.yaw, b.rotation.yaw, t),
        lerp_angle(a.rotation.roll, b.rotation.roll, t),
    )
    return Xform(loc, rot)  # type: ignore[arg-type]


def hfov_from_focal(focal_length_mm: float, sensor_width_mm: float = 36.0) -> float:
    return math.degrees(2 * math.atan(sensor_width_mm / (2 * focal_length_mm)))


def project(point: Vec3, cam: Xform, focal_length_mm: float, width: int, height: int,
            sensor_width_mm: float = 36.0) -> tuple[float, float, float] | None:
    """Pinhole projection. Returns (px, py, depth) or None when behind the camera."""
    rel = sub(point, cam.location)
    f, rt, u = forward(cam.rotation), right(cam.rotation), up(cam.rotation)
    depth = rel[0] * f[0] + rel[1] * f[1] + rel[2] * f[2]
    if depth <= 1.0:
        return None
    x = rel[0] * rt[0] + rel[1] * rt[1] + rel[2] * rt[2]
    y = rel[0] * u[0] + rel[1] * u[1] + rel[2] * u[2]
    if cam.rotation.roll:
        a = math.radians(-cam.rotation.roll)
        x, y = x * math.cos(a) - y * math.sin(a), x * math.sin(a) + y * math.cos(a)
    fx = (width / 2) / math.tan(math.radians(hfov_from_focal(focal_length_mm, sensor_width_mm)) / 2)
    return (width / 2 + fx * x / depth, height / 2 - fx * y / depth, depth)
