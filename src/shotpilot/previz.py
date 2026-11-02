"""Local previz: a wireframe preview rendered without launching the engine.

It draws the ground grid, the anchors (as human-scale pillars) and the look target
from the exact camera the engine will use. That is enough to judge framing, lens and
movement in seconds, before paying for an engine render.
"""

from __future__ import annotations

import hashlib
import math
import struct
import zlib
from pathlib import Path

from . import geometry as g
from .model import ResolvedShot

# 5x7 bitmap font, rows top->bottom, 5 bits each (MSB = left).
_FONT: dict[str, tuple[int, ...]] = {
    "0": (0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E), "1": (0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E),
    "2": (0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F), "3": (0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E),
    "4": (0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02), "5": (0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E),
    "6": (0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E), "7": (0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08),
    "8": (0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E), "9": (0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C),
    "A": (0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11), "B": (0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E),
    "C": (0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E), "D": (0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C),
    "E": (0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F), "F": (0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10),
    "G": (0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F), "H": (0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11),
    "I": (0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E), "J": (0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C),
    "K": (0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11), "L": (0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F),
    "M": (0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11), "N": (0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11),
    "O": (0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E), "P": (0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10),
    "Q": (0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D), "R": (0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11),
    "S": (0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E), "T": (0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04),
    "U": (0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E), "V": (0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04),
    "W": (0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A), "X": (0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11),
    "Y": (0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04), "Z": (0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F),
    " ": (0,) * 7, ".": (0, 0, 0, 0, 0, 0x0C, 0x0C), ":": (0, 0x0C, 0x0C, 0, 0x0C, 0x0C, 0),
    "-": (0, 0, 0, 0x1F, 0, 0, 0), "_": (0, 0, 0, 0, 0, 0, 0x1F), "/": (0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10),
    "#": (0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A), "+": (0, 0x04, 0x04, 0x1F, 0x04, 0x04, 0),
    "?": (0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04),
}

RGB = tuple[int, int, int]


class Canvas:
    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.px = bytearray(w * h * 3)

    def row_fill(self, y: int, color: RGB) -> None:
        if 0 <= y < self.h:
            i = y * self.w * 3
            self.px[i:i + self.w * 3] = bytes(color) * self.w

    def set(self, x: int, y: int, c: RGB) -> None:
        if 0 <= x < self.w and 0 <= y < self.h:
            i = (y * self.w + x) * 3
            self.px[i:i + 3] = bytes(c)

    def rect(self, x0: int, y0: int, x1: int, y1: int, c: RGB) -> None:
        x0, x1 = max(0, x0), min(self.w, x1)
        if x1 <= x0:
            return
        for y in range(max(0, y0), min(self.h, y1)):
            i = (y * self.w + x0) * 3
            self.px[i:i + (x1 - x0) * 3] = bytes(c) * (x1 - x0)

    def line(self, x0: float, y0: float, x1: float, y1: float, c: RGB) -> None:
        # Liang-Barsky clip to the canvas so off-screen spans cost nothing
        t0, t1 = 0.0, 1.0
        ddx, ddy = x1 - x0, y1 - y0
        for p, q in ((-ddx, x0), (ddx, self.w - 1 - x0), (-ddy, y0), (ddy, self.h - 1 - y0)):
            if p == 0:
                if q < 0:
                    return
            else:
                r = q / p
                if p < 0:
                    t0 = max(t0, r)
                else:
                    t1 = min(t1, r)
        if t0 > t1:
            return
        x0, y0, x1, y1 = x0 + ddx * t0, y0 + ddy * t0, x0 + ddx * t1, y0 + ddy * t1
        cb = bytes(c)
        w, h, px = self.w, self.h, self.px
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        dx, dy = (x1 - x0) / n, (y1 - y0) / n
        x, y = x0, y0
        for _ in range(n + 1):
            xi, yi = int(x), int(y)
            if 0 <= xi < w and 0 <= yi < h:
                i = (yi * w + xi) * 3
                px[i:i + 3] = cb
            x += dx
            y += dy

    def text(self, x: int, y: int, s: str, c: RGB, size: int = 1) -> None:
        for ch in s.upper():
            glyph = _FONT.get(ch, _FONT["?"])
            for ry, bits in enumerate(glyph):
                for rx in range(5):
                    if bits & (0x10 >> rx):
                        self.rect(x + rx * size, y + ry * size, x + (rx + 1) * size, y + (ry + 1) * size, c)
            x += 6 * size

    def png(self) -> bytes:
        raw = bytearray()
        stride = self.w * 3
        for y in range(self.h):
            raw.append(0)
            raw += self.px[y * stride:(y + 1) * stride]

        def chunk(tag: bytes, data: bytes) -> bytes:
            return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

        return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", self.w, self.h, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(bytes(raw), 6)) + chunk(b"IEND", b""))


def _mix(a: RGB, b: RGB, t: float) -> RGB:
    t = max(0.0, min(1.0, t))
    return (int(a[0] + (b[0] - a[0]) * t), int(a[1] + (b[1] - a[1]) * t), int(a[2] + (b[2] - a[2]) * t))


def _palette(elev: float | None) -> tuple[RGB, RGB, RGB]:
    """(sky_top, sky_horizon, ground) for a sun elevation in degrees."""
    if elev is None:
        return (70, 110, 160), (170, 190, 210), (60, 66, 58)
    if elev < -5:
        return (6, 8, 20), (24, 30, 58), (14, 16, 20)
    if elev < 12:
        return (60, 50, 110), (240, 140, 80), (55, 42, 40)
    if elev < 40:
        return (60, 120, 200), (200, 210, 220), (70, 80, 60)
    return (40, 110, 220), (170, 205, 240), (90, 100, 75)


def _anchor_color(name: str) -> RGB:
    h = hashlib.md5(name.encode()).digest()
    return (90 + h[0] % 150, 90 + h[1] % 150, 90 + h[2] % 150)


def render_frame(shot: ResolvedShot, frame: int, anchors: dict[str, g.Vec3], w: int, h: int,
                 ground_z: float = 0.0, tint: RGB | None = None) -> Canvas:
    cam = shot.camera_at(frame)
    fl = float(shot.lens["focal_length"])
    sensor = float(shot.lens.get("sensor_width", 36.0))
    cv = Canvas(w, h)
    top, horizon, ground = _palette(shot.environment.get("sun_elevation"))
    if tint:
        top, horizon, ground = _mix(top, tint, 0.35), _mix(horizon, tint, 0.35), _mix(ground, tint, 0.35)

    fx = (w / 2) / math.tan(math.radians(g.hfov_from_focal(fl, sensor)) / 2)
    hy = h / 2 + fx * math.tan(math.radians(max(-89.0, min(89.0, cam.rotation.pitch))))
    for y in range(h):
        if y < hy:
            cv.row_fill(y, _mix(top, horizon, (y / max(1.0, hy)) ** 1.5 if hy > 0 else 1.0))
        else:
            cv.row_fill(y, _mix(_mix(horizon, ground, 0.6), ground, (y - hy) / max(1.0, h - hy) * 3))

    # ground grid around the camera
    grid_c = _mix(ground, (230, 230, 230), 0.25)
    step, span = 250.0, 4000.0
    cx = round(cam.location[0] / step) * step
    cy = round(cam.location[1] / step) * step
    ticks = [i * step for i in range(-int(span / step), int(span / step) + 1)]
    for axis in (0, 1):
        for off in ticks:
            prev = None
            for along in ticks:
                p = (cx + off, cy + along, ground_z) if axis == 0 else (cx + along, cy + off, ground_z)
                q = g.project(p, cam, fl, w, h, sensor)
                if q and prev:
                    cv.line(prev[0], prev[1], q[0], q[1], grid_c)
                prev = q

    # anchors as human-scale wireframe boxes (120 x 120 x 180 cm), far to near
    size = 2 if w >= 640 else 1
    drawn = []
    for name, pos in anchors.items():
        c = g.project(pos, cam, fl, w, h, sensor)
        if c:
            drawn.append((c[2], name, pos))
    edges = [(0, 1), (1, 3), (3, 2), (2, 0), (4, 5), (5, 7), (7, 6), (6, 4), (0, 4), (1, 5), (2, 6), (3, 7)]
    for _, name, pos in sorted(drawn, reverse=True):
        col = _anchor_color(name)
        corners = [g.project((pos[0] + sx * 60, pos[1] + sy * 60, pos[2] + sz * 90), cam, fl, w, h, sensor)
                   for sz in (-1, 1) for sy in (-1, 1) for sx in (-1, 1)]
        for a, b in edges:
            if corners[a] and corners[b]:
                cv.line(corners[a][0], corners[a][1], corners[b][0], corners[b][1], col)
        top = [p for p in corners[4:] if p]
        if top:
            lx = int(min(p[0] for p in top))
            ly = int(min(p[1] for p in top)) - 9 * size
            cv.text(max(0, min(w - 6 * size * len(name), lx)), max(10 * size, ly), name[:16], col, size)

    if shot.target is not None:
        t = g.project(shot.target, cam, fl, w, h, sensor)
        if t:
            r = 4 * size
            cv.line(t[0] - r, t[1], t[0] + r, t[1], (255, 60, 60))
            cv.line(t[0], t[1] - r, t[0], t[1] + r, (255, 60, 60))

    # rule-of-thirds guides
    guide = (255, 255, 255)
    for i in (1, 2):
        for y in range(0, h, 4):
            cv.set(int(w * i / 3), y, guide)
        for x in range(0, w, 4):
            cv.set(x, int(h * i / 3), guide)

    # overlay
    cv.rect(0, 0, w, 10 * size, (0, 0, 0))
    cv.text(2 * size, 2 * size, f"{shot.id}  F{frame + 1}/{shot.frames}  {int(round(fl))}MM", (255, 255, 255), size)
    for m in shot.markers:
        if abs(m["frame"] - frame) <= max(2, shot.fps // 4):
            cv.rect(0, h - 10 * size, w, h, (200, 30, 30))
            cv.text(2 * size, h - 8 * size, m["label"][:40], (255, 255, 255), size)
    return cv


def render_shot(shot: ResolvedShot, frames: list[int], anchors: dict[str, g.Vec3], out_dir: Path,
                w: int, h: int, ground_z: float = 0.0, tint: RGB | None = None) -> list[str]:
    out_dir.mkdir(parents=True, exist_ok=True)
    written = []
    for f in frames:
        name = f"frame.{f:04d}.png"
        (out_dir / name).write_bytes(render_frame(shot, f, anchors, w, h, ground_z, tint).png())
        written.append(name)
    return written
