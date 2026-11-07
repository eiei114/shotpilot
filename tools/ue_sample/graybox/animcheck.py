"""Look at an animation without a GPU: evaluate an AnimSequence with AnimPose at a few times and
draw stick figures (front and side views, the ground line) into a PNG.

    from graybox import animcheck
    animcheck.sheet("/Game/Lanternless/Anims/A_Wave", "C:/tmp/wave.png", frames=8)
    animcheck.report("/Game/Lanternless/Anims/A_Wave")     # lowest points, bone lengths

The drawing part (draw_sheet, write_png) is plain Python and takes bone positions in the
mannequin's component space (+X its left, +Y forward, +Z up, cm).
"""

import math
import struct
import zlib

# bones that make up the figure: (from, to, colour)
BLUE, RED, GREY, GREEN = (60, 110, 230), (230, 70, 60), (60, 60, 60), (40, 150, 70)
SEGMENTS = (
    [("pelvis", "spine_01", GREY), ("spine_01", "spine_02", GREY), ("spine_02", "spine_03", GREY),
     ("spine_03", "spine_04", GREY), ("spine_04", "spine_05", GREY), ("spine_05", "neck_01", GREY),
     ("neck_01", "neck_02", GREY), ("neck_02", "head", GREY)]
    + [(a.replace("%s", s), b.replace("%s", s), c) for s, c in (("l", BLUE), ("r", RED)) for a, b in (
        ("spine_05", "clavicle_%s"), ("clavicle_%s", "upperarm_%s"), ("upperarm_%s", "lowerarm_%s"),
        ("lowerarm_%s", "hand_%s"), ("hand_%s", "middle_01_%s"), ("pelvis", "thigh_%s"),
        ("thigh_%s", "calf_%s"), ("calf_%s", "foot_%s"), ("foot_%s", "ball_%s"))])
KEY_BONES = sorted({b for a, c, _ in SEGMENTS for b in (a, c)})
SKIN = 4.5  # cm: the mannequin's limbs are about this far from their bones' centre lines
FLOOR_BONES = ("ball_l", "ball_r", "foot_l", "foot_r", "hand_l", "hand_r", "calf_l", "calf_r")


# --------------------------------------------------------------------------- a tiny PNG writer
def write_png(path, width, height, rgb):
    """rgb: a bytearray of width * height * 3."""
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + bytes(rgb[y * width * 3:(y + 1) * width * 3]) for y in range(height))
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
                + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


class Canvas:
    def __init__(self, w, h, bg=(250, 250, 247)):
        self.w, self.h = w, h
        self.px = bytearray(bytes(bg) * (w * h))

    def dot(self, x, y, c, r=0):
        for yy in range(int(y) - r, int(y) + r + 1):
            for xx in range(int(x) - r, int(x) + r + 1):
                if 0 <= xx < self.w and 0 <= yy < self.h:
                    i = (yy * self.w + xx) * 3
                    self.px[i:i + 3] = bytes(c)

    def line(self, x0, y0, x1, y1, c, r=0):
        n = int(max(abs(x1 - x0), abs(y1 - y0))) + 1
        for i in range(n + 1):
            t = i / n
            self.dot(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, c, r)

    def rect(self, x0, y0, x1, y1, c):
        for yy in range(int(y0), int(y1) + 1):
            for xx in range(int(x0), int(x1) + 1):
                self.dot(xx, yy, c)

    def circle(self, cx, cy, rad, c):
        for i in range(48):
            a = i / 48 * 2 * math.pi
            self.dot(cx + rad * math.cos(a), cy + rad * math.sin(a), c, 0)

    def text(self, x, y, s, c=(40, 40, 40), k=2):
        for ch in s:
            for ry, row in enumerate(FONT.get(ch, FONT[" "])):
                for rx, bit in enumerate(row):
                    if bit == "#":
                        for a in range(k):
                            for b in range(k):
                                self.dot(x + rx * k + a, y + ry * k + b, c)
            x += 4 * k


FONT = {" ": ["   "] * 5, ".": ["   ", "   ", "   ", "   ", " # "], "-": ["   ", "   ", "###", "   ", "   "],
        "=": ["   ", "###", "   ", "###", "   "], ":": ["   ", " # ", "   ", " # ", "   "],
        "0": ["###", "# #", "# #", "# #", "###"], "1": [" # ", "## ", " # ", " # ", "###"],
        "2": ["###", "  #", "###", "#  ", "###"], "3": ["###", "  #", "###", "  #", "###"],
        "4": ["# #", "# #", "###", "  #", "  #"], "5": ["###", "#  ", "###", "  #", "###"],
        "6": ["###", "#  ", "###", "# #", "###"], "7": ["###", "  #", "  #", "  #", "  #"],
        "8": ["###", "# #", "###", "# #", "###"], "9": ["###", "# #", "###", "  #", "###"],
        "s": ["   ", " ##", "## ", "  #", "## "], "t": [" # ", "###", " # ", " # ", " ##"],
        "f": [" ##", "#  ", "## ", "#  ", "#  "], "r": ["   ", "## ", "#  ", "#  ", "#  "],
        "o": ["   ", "###", "# #", "# #", "###"], "n": ["   ", "## ", "# #", "# #", "# #"],
        "i": [" # ", "   ", " # ", " # ", " # "], "d": ["  #", "  #", "###", "# #", "###"],
        "e": ["###", "# #", "###", "#  ", "###"], "m": ["   ", "###", "###", "# #", "# #"],
        "z": ["###", "  #", " # ", "#  ", "###"], "x": ["# #", "# #", " # ", "# #", "# #"],
        "y": ["# #", "# #", " ##", "  #", "## "], "w": ["# #", "# #", "###", "###", "# #"],
        "a": ["###", "# #", "###", "# #", "# #"], "l": ["#  ", "#  ", "#  ", "#  ", "###"]}


# --------------------------------------------------------------------------- the figure
def draw_figure(cv, pose, view, ox, oy, scale, props=()):
    """Draw one pose into `cv`. view "front" looks at the figure (its left on screen right),
    "side" is from its right (it faces right). (ox, oy) is the screen point of x = 0, z = 0."""
    def p(v):
        h = v[0] if view == "front" else v[1]
        return ox + h * scale, oy - v[2] * scale

    cv.line(ox - 85 * scale, oy, ox + 85 * scale, oy, (150, 150, 140), 0)  # the ground
    for pr in props:  # ("box", x0, x1, y0, y1, z0, z1)
        a, b = (pr[1], pr[2]) if view == "front" else (pr[3], pr[4])
        cv.rect(ox + a * scale, oy - pr[6] * scale, ox + b * scale, oy - pr[5] * scale, (205, 190, 160))
    # far side first (the figure's left in the side view is behind)
    for a, b, c in sorted(SEGMENTS, key=lambda s: (s[2] == BLUE) if view == "side" else 0):
        if a in pose and b in pose:
            x0, y0 = p(pose[a])
            x1, y1 = p(pose[b])
            cv.line(x0, y0, x1, y1, c, 1)
    if "head" in pose and "neck_02" in pose:
        hx, hy = p(pose["head"])
        cv.circle(hx, hy, 9 * scale, GREY)
    for b in ("hand_l", "hand_r", "ball_l", "ball_r"):
        if b in pose:
            x, y = p(pose[b])
            cv.dot(x, y, GREEN, 2)


def draw_sheet(poses, labels, png, props=(), cols=4, cell=(190, 250), scale=1.05):
    """poses: a list of {bone: (x, y, z)}. Each gets a front and a side view, stacked."""
    cw, ch = cell
    rows = (len(poses) + cols - 1) // cols
    cv = Canvas(cw * cols, ch * rows * 2 + 4)
    for i, (pose, label) in enumerate(zip(poses, labels)):
        col, row = i % cols, i // cols
        for k, view in enumerate(("front", "side")):
            x0, y0 = col * cw, (row * 2 + k) * ch
            cv.rect(x0, y0, x0 + cw - 1, y0, (220, 220, 215))
            cv.rect(x0, y0, x0, y0 + ch - 1, (220, 220, 215))
            draw_figure(cv, pose, view, x0 + cw / 2, y0 + ch - 22, scale, props)
            cv.text(x0 + 6, y0 + 5, label if k == 0 else "side", (30, 30, 30))
    write_png(png, cv.w, cv.h, cv.px)
    return png


# --------------------------------------------------------------------------- reading an AnimSequence
def poses_of(anim_path, times=None, frames=8):
    """({bone: (x, y, z)} per time, the times) of a /Game AnimSequence in component space."""
    import unreal
    seq = unreal.EditorAssetLibrary.load_asset(anim_path)
    if seq is None:
        raise ValueError("animation not found: " + anim_path)
    length = float(seq.get_editor_property("sequence_length"))
    if times is None:
        times = [length * i / frames for i in range(frames)]
    X, W = unreal.AnimPoseExtensions, unreal.AnimPoseSpaces.WORLD
    out = []
    for t in times:
        pose = X.get_anim_pose_at_time(seq, min(t, length), unreal.AnimPoseEvaluationOptions())
        d = {}
        for b in KEY_BONES:
            v = X.get_bone_pose(pose, b, W).translation
            d[b] = (v.x, v.y, v.z)
        out.append(d)
    return out, times


def sheet(anim_path, png_path, frames=8, times=None, props=()):
    """Draw `anim_path` at `frames` evenly spaced times (or at `times`, in seconds) into a PNG.
    `props` are boxes to draw behind the figure: ("box", x0, x1, y0, y1, z0, z1) in cm."""
    poses, ts = poses_of(anim_path, times, frames)
    draw_sheet(poses, ["%.2fs" % t for t in ts], png_path, props)
    return png_path


def report(anim_path, frames=12):
    """Numbers that catch a broken pose: the lowest point of the feet, hands and shins, and how much
    any bone length changes over the move (a stretched limb)."""
    poses, ts = poses_of(anim_path, None, frames)
    low = {b: min(p[b][2] for p in poses) for b in FLOOR_BONES}
    stretch = {}
    for a, b, _ in SEGMENTS:
        lens = [math.dist(p[a], p[b]) for p in poses]
        if max(lens) - min(lens) > 0.5:
            stretch[a + "-" + b] = round(max(lens) - min(lens), 2)
    return {"lowest_z": {k: round(v, 1) for k, v in low.items()}, "stretched": stretch}
