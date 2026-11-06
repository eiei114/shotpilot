"""AshenPillars (action-adventure): a ruined temple where ash falls at sunset."""

import math

import unreal

from .core import *  # noqa: F401,F403  (the Level spawner, paint, constants)
from .fx import *  # noqa: F401,F403
from .anim import *  # noqa: F401,F403
from .shapes import assets_folder, place, column, lathe, rock, ramp, prism, _make
from . import moves_action as mv

# ---------------------------------------------------------------------------------------------
# AshenPillars (action-adventure): a ruined temple where ash falls at sunset. A broken gate, a
# plaza between two rows of columns (some snapped, their tops lying where they fell), a grand
# stair up to the altar terrace, braziers burning, ruined precinct walls with arched gateways.
# Warriors hold swords: a pair duels in the plaza, another pair thrusts and dodges, one kneels at
# the altar, one rallies, one looks out over the gate, the hero waits at the top of the stair.
# ---------------------------------------------------------------------------------------------

ASHEN_MAP = "/Game/AshenPillars/Maps/Lvl_Temple"

PLAZA = 40.0                 # the plaza's top (cm)
COL_R, COL_H = 60.0, 1000.0  # column radius and height (from the plaza)
COL_X = [-1400.0 + 400.0 * i for i in range(8)]
COL_Y = {"L": 650.0, "R": -650.0}
BROKEN = {("L", 2): 430.0, ("L", 5): 600.0, ("R", 1): 290.0, ("R", 6): 680.0}  # snapped columns: height of the stump
TOP = PLAZA + COL_H          # the underside of the entablature
SPOTS = ((-1000, 430, PLAZA), (-1000, -430, PLAZA), (600, 430, PLAZA), (600, -430, PLAZA),
         (3050, 520, 440.0), (3050, -520, 440.0))   # the braziers (floor height)
DUEL_AT = (500.0, 0.0)       # the middle of the duelling pair
PAIR_AT = (1250.0, 400.0)    # ... and of the thrust-and-dodge pair (off the stair camera's line of sight)


# --------------------------------------------------------------------------- the shapes of this place

def _spun(F, kind, params, prof, segments=32, flutes=0, zmod=None, angle=50.0):
    """A solid of revolution like shapes.lathe, with fluting (a point's third number is the groove depth)
    and a ragged edge (a point's fourth number weights `zmod(j, theta)`, a height added to that ring)."""
    def mod(rr, a, th):
        return rr * (1.0 - a * (0.5 - 0.5 * math.cos(flutes * th)))

    def build(m):
        m.begin()
        rings = []
        for p in prof:
            r, z = p[0], p[1]
            a = p[2] if len(p) > 2 else 0.0
            w = p[3] if len(p) > 3 else 0.0
            ring = []
            for j in range(segments):
                th = 2 * math.pi * j / segments
                rr = mod(r, a, th) if (flutes and a) else r
                zz = z + (w * zmod(j, th) if (zmod and w and r > 0) else 0.0)
                ring.append((rr * math.cos(th), rr * math.sin(th), zz))
            rings.append(ring)
        n = len(prof)
        for i in range(n):
            k = (i + 1) % n
            for j in range(segments):
                q = (j + 1) % segments
                m.quad(rings[i][j], rings[i][q], rings[k][q], rings[k][j])
        m.end(kind)
    return _make(F, kind, params, build, angle)


def _shaft(r, z, s0, s1, taper=0.85):
    u = (z - s0) / (s1 - s0)
    return r * (1 + (taper - 1) * u + 0.025 * math.sin(math.pi * u))


def stump(F, r, h, seed):
    """A snapped column: the same base and fluted shaft as shapes.column (height COL_H), broken off at `h`
    along a tilted, jagged fracture."""
    base, cap = COL_H * 0.06, COL_H * 0.08
    s0, s1 = base * 2, COL_H - cap * 1.6
    ramp = (s1 - s0) * 0.06
    prof = [(0.0, 0.0), (r * 1.55, 0.0), (r * 1.55, base), (r * 1.3, base), (r * 1.3, base * 1.6),
            (r * 1.14, base * 1.6), (r * 1.14, s0)]
    n = max(4, int((h - s0) / 60.0))
    for i in range(n + 1):
        z = s0 + (h - s0) * i / n
        top = i == n
        prof.append((_shaft(r, z, s0, s1), z, 0.08 if (z - s0 > ramp or top) else 0.0, 1.0 if top else 0.0))
    prof.append((0.0, h))
    rnd = [((math.sin(seed * 12.9 + k * 78.2) * 43758.5) % 1.0 - 0.5) for k in range(8)]
    phi = seed * 1.7
    tilt = 38.0 + 10.0 * (seed % 3)

    def jag(j, th, segs=64):
        f = 8.0 * j / segs
        k = int(f)
        e = rnd[k % 8] + (rnd[(k + 1) % 8] - rnd[k % 8]) * (f - k)
        return tilt * math.cos(th - phi) + 34.0 * e

    return _spun(F, "Stump", [r, h, seed], prof, 64, flutes=16, zmod=jag, angle=50.0)


def drum(F, r, length, taper=0.9):
    """A fallen column drum: a fluted cylinder centred on its origin, its axis along Z."""
    h = length / 2.0
    r1 = r * taper
    prof = [(0.0, -h), (r - 4, -h), (r, -h + 5), (r, -h + 14, 0.08), (r1, h - 14, 0.08), (r1, h - 5), (r1 - 4, h), (0.0, h)]
    return _spun(F, "Drum", [r, length, taper], prof, 48, flutes=16)


def capital(F, r=COL_R):
    """A column capital (the shapes.column one, on its own): neck, flared echinus and a square-ish abacus,
    centred on its origin."""
    prof = [(0.0, -40.0), (r * 0.9, -40.0), (r * 1.12, -26.0), (r * 1.28, -12.0), (r * 1.45, 4.0), (r * 1.5, 4.0),
            (r * 1.5, 40.0), (0.0, 40.0)]
    return _spun(F, "Capital", [r], prof, 32)


def banner(F, width=120.0, height=382.0, ragged=0):
    """A torn banner (a flat cloth, 5 cm thick) hanging from z = height; the origin is at its bottom centre."""
    w = width / 2.0
    tails = (((40, 80), (22, 10), (2, 70), (-20, 0), (-42, 60)), ((46, 30), (30, 90), (8, 20), (-14, 80), (-36, 14)))
    pts = [(-w, height), (w, height), (w, 30.0 + 20 * ragged)] + [(x * w / 60.0, z) for x, z in tails[ragged % 2]] + [(-w, 25.0)]
    return prism(F, pts, 5.0, plane="xz")


def sword(F):
    """A sword in the hand's own frame (hand_r): the grip across the fist, the blade out along -X for
    mv.BLADE cm, flat to the camera when the hand hangs. Origin at the wrist."""
    def build(m):
        tip = -mv.BLADE
        m.prism([(-17.0, -3.4), (tip + 12.0, -3.4), (tip, 0.0), (tip + 12.0, 3.4), (-17.0, 3.4)],
                (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), 2.2, mid=True)      # blade
        m.prism([(-14.0, -12.0), (-17.0, -12.0), (-17.0, 12.0), (-14.0, 12.0)],
                (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), 5.0, mid=True)      # crossguard
        m.prism([(5.0, -2.2), (-14.0, -2.2), (-14.0, 2.2), (5.0, 2.2)],
                (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), 4.4, mid=True)      # grip
        m.prism([(5.0, -3.5), (8.5, -3.5), (8.5, 3.5), (5.0, 3.5)],
                (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), 7.0, mid=True)      # pommel
    return _make(F, "Sword", [mv.BLADE], build, 40.0)


def bowl(F):
    """A brazier bowl 110 cm across and 32 high, hollow on top, the origin on its underside."""
    prof = [(0.0, 0.0), (24.0, 0.0), (40.0, 8.0), (55.0, 32.0), (49.0, 32.0), (34.0, 16.0), (0.0, 16.0)]
    return lathe(F, "Bowl", prof, 40)


def pedestal(F):
    """A brazier stand 90 cm tall: a flared foot, a waisted shaft, a collar."""
    prof = [(0.0, 0.0), (36.0, 0.0), (36.0, 10.0), (20.0, 18.0), (14.0, 40.0), (14.0, 66.0), (24.0, 78.0), (30.0, 80.0),
            (30.0, 90.0), (0.0, 90.0)]
    return lathe(F, "Pedestal", prof, 28)


def wall_run(F, name, width, top, door=None):
    """A ruined wall 100 cm thick lying along X from 0 to `width`: `top` is its broken skyline [(x, height)]
    left to right, `door` = (centre x, width, height) cuts a round-headed gateway to the ground."""
    pts = [(0.0, 0.0)]
    if door:
        ax, aw, ah = door
        hw, spring = aw / 2.0, ah - aw / 2.0
        pts += [(ax - hw, 0.0), (ax - hw, spring)]
        for i in range(1, 14):
            a = math.pi - math.pi * i / 14.0
            pts.append((ax + hw * math.cos(a), spring + hw * math.sin(a)))
        pts += [(ax + hw, spring), (ax + hw, 0.0)]
    pts += [(width, 0.0)] + [tuple(p) for p in reversed(top)]
    return prism(F, pts, 100.0, plane="xz")


def tilted_beam(lv, label, p1, p2, x, depth, thick, low_end=100.0, high_end=60.0, mat="stone", folder="Gate"):
    """A box leaning between two supports in the y-z plane at world x: its underside touches the high support
    `p1` and the low one `p2` ((y, z) each), reaching `high_end` past the first and `low_end` into the second."""
    dy, dz = p2[0] - p1[0], p2[1] - p1[1]
    ln = math.hypot(dy, dz)
    uy, uz = dy / ln, dz / ln
    ny, nz = -uz if uz < 0 else uz, (uy if uz < 0 else -uy)   # the normal that points up
    if nz < 0:
        ny, nz = -ny, -nz
    length = ln + low_end + high_end
    mid = ((p1[0] + p2[0]) / 2 + uy * (low_end - high_end) / 2 + ny * thick / 2,
           (p1[1] + p2[1]) / 2 + uz * (low_end - high_end) / 2 + nz * thick / 2)
    roll = -math.degrees(math.atan2(-uz, -uy)) if uy < 0 else -math.degrees(math.atan2(uz, uy))
    return lv.shape(label, (x, mid[0], mid[1]), (depth, length, thick), mat=mat, rot=(0.0, 0.0, roll), folder=folder)


def fill_light(lv, label, at, candela, color, radius):
    """A soft light that casts no shadows, so the stand and bowl of a brazier do not leave a black disc under them."""
    a = lv.point(label, at, candela, color, radius=radius, source=40.0)
    setp(a.get_component_by_class(unreal.PointLightComponent), cast_shadows=False)
    return a


def hold_mesh(lv, holder, label, mesh, mat="steel", socket="hand_r", rot=(0.0, mv.GRIP_YAW, 0.0)):
    """A mesh in a mannequin's hand: snapped to the socket, then turned by `rot` in the hand's own frame."""
    a = place(lv, label, mesh, (0.0, 0.0, 0.0), mat=mat, folder="Characters")
    comp = a.get_component_by_class(unreal.StaticMeshComponent)
    movable(comp)
    skel = holder.get_component_by_class(unreal.SkeletalMeshComponent)
    snap, world = unreal.AttachmentRule.SNAP_TO_TARGET, unreal.AttachmentRule.KEEP_WORLD
    a.attach_to_component(skel, socket, snap, snap, world, False)
    comp.set_editor_property("relative_location", unreal.Vector(0.0, 0.0, 0.0))
    comp.set_editor_property("relative_rotation", unreal.Rotator(roll=rot[2], pitch=rot[0], yaw=rot[1]))
    return a


def walker3(lv, life, label, path, clips, mesh=MANNY):
    """Like Ambient.walker, but the path keys carry a height: (seconds, x, y, z, facing). `clips` may name
    a full /Game path."""
    t0, x0, y0, z0, f0 = path[0]
    a = lv.mannequin((x0, y0, z0), unreal.Rotator(roll=0.0, pitch=0.0, yaw=f0 - 90.0), mesh, clips[0][2])
    lv._done(a, label, "Characters")
    b = life.seq.add_possessable(a)
    sec = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
    sec.set_range(0, life.frames)
    ch = transform_channels(sec)
    lin = unreal.MovieSceneKeyInterpolation.LINEAR
    for (t, x, y, z, facing) in path:
        for name, value in (("Location.X", x), ("Location.Y", y), ("Location.Z", z), ("Rotation.X", 0.0),
                            ("Rotation.Y", 0.0), ("Rotation.Z", facing - 90.0)):
            ch[name].add_key(unreal.FrameNumber(life._f(t)), float(value), 0.0, unreal.MovieSceneTimeUnit.DISPLAY_RATE, lin)
    for name in ("Scale.X", "Scale.Y", "Scale.Z"):
        try:
            ch[name].set_default(1.0)
        except Exception:
            pass
    track = b.add_track(unreal.MovieSceneSkeletalAnimationTrack)
    for (c0, c1, anim, rate) in clips:
        s = track.add_section()
        s.set_range(life._f(c0), life._f(c1))
        p = s.get_editor_property("params")
        p.set_editor_property("animation", load(anim if anim.startswith("/") else ANIMS + "/" + anim))
        set_play_rate(p, rate)
        s.set_editor_property("params", p)
    return a


# --------------------------------------------------------------------------- the level

def build_ashenpillars(lv):
    m = "/Game/AshenPillars/Materials/"
    F = assets_folder(lv)
    FX = "/Game/AshenPillars/FX"
    paint(lv, m, {
        "ash": dict(base=NOISE + GRAIN + "return lerp(float3(0.06, 0.055, 0.05), float3(0.26, 0.25, 0.24), n * n) * (0.85 + 0.25 * g);",
                    roughness="return 0.97;"),
        "stone": dict(base=NOISE + GRAIN + "float grime = saturate(1.0 - WP.z / 220.0);\n"
                                           "return float3(0.46, 0.41, 0.34) * (0.82 + 0.22 * g) * (1.0 - 0.45 * grime) * (0.9 + 0.15 * n);",
                      roughness="return 0.88;"),
        "dark_stone": dict(base=NOISE + GRAIN + "return float3(0.20, 0.18, 0.16) * (0.8 + 0.3 * g) * (0.9 + 0.2 * n);",
                           roughness="return 0.9;"),
        # flagstones laid in a running bond, 2 m a side, each a little different, with dark joints
        "flags": dict(base=NOISE + "float2 q = WP.xy / 200.0;\nq.x += floor(q.y) * 0.37;\nfloat2 id = floor(q);\nfloat2 f = frac(q);\n"
                                   "float h = frac(sin(dot(id, float2(12.9898, 78.233))) * 43758.5453);\n"
                                   "float e = min(min(f.x, 1.0 - f.x), min(f.y, 1.0 - f.y));\n"
                                   "float seam = 1.0 - smoothstep(0.0, 0.03, e);\n"
                                   "float3 c = lerp(float3(0.17, 0.15, 0.13), float3(0.28, 0.25, 0.21), h);\n"
                                   "return c * (1.0 - 0.6 * seam) * (0.85 + 0.2 * n) * (0.9 + 0.1 * frac(h * 9.7));",
                      roughness="return 0.9;"),
        "gold": dict(base="return float3(0.75, 0.54, 0.20);", roughness="return 0.28;"),
        "coal": dict(base="return float3(0.02, 0.015, 0.01);", roughness="return 1.0;",
                     emissive="return float3(1.0, 0.3, 0.05) * (2.2 + 1.2 * sin(T * 3.1 + WP.x * 0.2) * sin(T * 1.7 + WP.y * 0.3));"),
        "relic": dict(base="return float3(0.1, 0.03, 0.0);", roughness="return 0.3;",
                      emissive="return float3(1.0, 0.45, 0.15) * (26.0 + 6.0 * sin(T * 1.7));"),
        "steel": dict(base="return float3(0.62, 0.62, 0.64);", roughness="return 0.22;"),
        "cloth": dict(base=NOISE + "return float3(0.35, 0.05, 0.04) * (0.8 + 0.3 * n);", roughness="return 0.9;"),
        "mountain": dict(base=NOISE + "return float3(0.10, 0.09, 0.10) * (0.8 + 0.3 * n);", roughness="return 1.0;"),
    })
    lv.box("Ground", (0, 0, -10), (80000, 80000, 20), mat="ash", folder="Geometry")
    ridge = [rock(F, 1000.0, seed=3 + k, squash=0.75, rough=0.14, lon=22, lat=12) for k in range(4)]
    for i in range(16):  # low overlapping ridges far off, sitting on the ground
        a = i * 0.41 + 0.3
        rr = 13000 + (i * 977) % 4000
        w, h = 5000 + (i * 613) % 3000, 1500 + (i * 421) % 1600
        # a rock 1000 across is about 1160 tall: scale it to a ridge of height ~0.55 h (low enough to leave the sun's glow)
        place(lv, "Distant_Ridge_%d" % (i + 1), ridge[i % 4], (rr * math.cos(a * 2.6), rr * math.sin(a * 2.6), -h * 0.08),
              rot=(i * 37) % 180, mat="mountain", folder="Backdrop", scale=(w / 2000.0, w * 0.6 / 2000.0, 0.55 * h / 1160.0))

    # the plaza: flagstones, with a worn ramp up from the ground at the gate
    lv.box("Plaza", (10, 0, 20), (3780, 2100, 40), mat="flags", folder="Geometry")
    place(lv, "Gate_Ramp", ramp(F, 160.0, 500.0, 40.0), (-2040, 0, 0), mat="stone", folder="Gate")

    # the colonnade: two rows of eight fluted columns, four of them snapped off
    full = column(F, COL_R, COL_H, 0.85, flutes=16)
    stumps = {}
    for side, y in COL_Y.items():
        for i, x in enumerate(COL_X):
            h = BROKEN.get((side, i))
            if h is None:
                label = "SP_colonnade" if (side, i) == ("L", 4) else "Column_%s%d" % (side, i + 1)
                place(lv, label, full, (x, y, PLAZA), mat="stone", folder="Colonnade")
            else:
                stumps[(side, i)] = stump(F, COL_R, h, 2 + i + (3 if side == "R" else 0))
                place(lv, "Column_%s%d_Stump" % (side, i + 1), stumps[(side, i)], (x, y, PLAZA), rot=17.0 * i, mat="stone",
                      folder="Colonnade")
    # the entablature survives only above runs of standing columns: one block per bay, a cornice over each run
    runs = {"L": [(0, 1), (3, 4), (6, 7)], "R": [(0, 0), (2, 5), (7, 7)]}
    for side, spans in runs.items():
        y = COL_Y[side]
        for (a, b) in spans:
            if a == b:  # a lone column carries nothing
                continue
            for i in range(a, b):
                lv.box("Architrave", ((COL_X[i] + COL_X[i + 1]) / 2, y, TOP + 35), (396, 150, 70), mat="stone", folder="Colonnade")
            x0, x1 = COL_X[a] - 90.0, COL_X[b] + 90.0
            lv.box("Cornice", ((x0 + x1) / 2, y, TOP + 70 + 12), (x1 - x0 + 20, 176, 24), mat="stone", folder="Colonnade")
            # the end blocks reach the outer edge of the end capitals
            lv.box("Architrave_Cap", (x0 + 45, y, TOP + 35), (90, 150, 70), mat="stone", folder="Colonnade")
            lv.box("Architrave_Cap", (x1 - 45, y, TOP + 35), (90, 150, 70), mat="stone", folder="Colonnade")
    cloth = [banner(F, 120.0, 382.0, 0), banner(F, 120.0, 382.0, 1)]
    for k, (x, y) in enumerate(((0, 650), (-400, -650), (400, -650))):
        place(lv, "Torn_Banner_%d" % (k + 1), cloth[k % 2], (x, y, TOP - 380), mat="cloth", folder="Colonnade")

    # snapped tops lie where they fell, on the plaza
    d = drum(F, 55.0, 170.0)
    cap = capital(F)
    lie = [(-600, 465, 88), (-632, 285, 76), (800, 520, 100), (400, 545, 30), (-1200, -500, 100), (-1130, -790, 20),
           (1000, -480, 104), (1220, -560, 15)]
    for k, (x, y, yaw) in enumerate(lie):
        place(lv, "Fallen_Drum_%d" % (k + 1), d, (x, y, PLAZA + 55.0), rot=(90.0, yaw, 0.0), mat="stone", folder="Colonnade")
    for k, (x, y, yaw) in enumerate(((-420, 480, 30), (-1330, -520, 200), (1380, -560, 80))):
        place(lv, "Fallen_Capital_%d" % (k + 1), cap, (x, y, PLAZA + 40.0), rot=(180.0, yaw, 0.0), mat="stone", folder="Colonnade")
    for k, (x, y, yaw) in enumerate(((-820, 600, 12), (760, 580, 160))):  # a block of the entablature that came down
        lv.shape("Fallen_Block_%d" % (k + 1), (x, y, PLAZA + 35.0), (396, 150, 70), mat="stone", rot=(0.0, yaw, 0.0),
                 folder="Colonnade")

    # rubble, scattered the same way every build, clear of everything that stands or walks
    keep = [(x, y, 130.0) for (x, y, _) in lie] + [(-420, 480, 130.0), (-1330, -520, 130.0), (1380, -560, 130.0)]
    keep += [(x, COL_Y[s], 130.0) for s in COL_Y for x in COL_X]
    keep += [(DUEL_AT[0], DUEL_AT[1], 200.0), (PAIR_AT[0], PAIR_AT[1], 200.0), (-350, 260, 160.0), (-1200, 150, 90.0)]
    stones = [rock(F, 40.0, seed=s, squash=0.65, rough=0.3) for s in (1, 2, 3, 4)]
    placed = 0
    for i in range(80):
        rx, ry = -1750 + (i * 137) % 3500, -760 + (i * 311) % 1520
        sz = 0.5 + ((i * 37) % 45) / 40.0
        if any(math.hypot(rx - kx, ry - ky) < kr + 30 * sz for (kx, ky, kr) in keep) or abs(ry + 300) < 100 or placed >= 26:
            continue
        place(lv, "Rubble", stones[i % 4], (rx, ry, PLAZA - 3), rot=((i * 23) % 40, (i * 71) % 360, (i * 13) % 30), mat="stone",
              folder="Colonnade", scale=(sz * 1.2, sz, sz))
        placed += 1
    for i, (x, y, s) in enumerate(((-2300, 20, 2.0), (-2420, 330, 1.4), (-2180, 480, 1.1), (-2580, -80, 2.4),
                                   (2300, 1100, 2.2), (1500, -1180, 1.6))):
        place(lv, "Outer_Rubble_%d" % (i + 1), stones[i % 4], (x, y, -4), rot=(0.0, i * 61.0, 0.0), mat="stone",
              folder="Geometry", scale=(s * 1.3, s, s))

    # ruined precinct walls with arched gateways, far enough from the axis to frame it
    walls = (("North_Wall", -1800, 1600, 3200, [(0, 380), (260, 470), (260, 400), (620, 430), (620, 560), (1100, 520), (1100, 400),
                                                (1380, 520), (1620, 470), (1900, 540), (1900, 300), (2300, 330), (2300, 450),
                                                (2700, 380), (3200, 260)], (1500, 240, 320)),
             ("South_Wall", -1500, -1600, 2900, [(0, 300), (350, 420), (350, 520), (800, 480), (1000, 380), (1000, 300),
                                                 (1780, 480), (2020, 540), (2400, 460), (2400, 340), (2900, 420)], (1900, 240, 320)))
    for name, x0, y0, width, top, door in walls:
        place(lv, name, wall_run(F, name, width, top, door), (x0, y0, 0), mat="stone", folder="Precinct")

    # the broken gate: the tall post is the anchor, the other one snapped; the lintel rests on both
    lv.box("SP_gate", (-2000, 380, 550), (220, 220, 1100), mat="stone", folder="Gate")
    lv.box("Gate_Post", (-2000, -380, 320), (220, 220, 640), mat="stone", folder="Gate")
    tilted_beam(lv, "Gate_Lintel_Leaning", (270.0, 1100.0), (-270.0, 640.0), -2000, 240, 170)
    lv.shape("Gate_Lintel_Fallen", (-2300, 260, 85), (240, 480, 170), mat="stone", rot=(0.0, 28.0, 0.0), folder="Gate")

    # the grand stair (the first step is the anchor) and the terrace with the altar
    lv.box("SP_stairs", (1940, 0, 40), (80, 1000, 80), mat="stone", folder="Altar")
    pts = [(0.0, 0.0), (720.0, 0.0)]
    for k in range(9, 0, -1):
        pts += [(k * 80.0, 80.0 + 40.0 * k), ((k - 1) * 80.0, 80.0 + 40.0 * k)]
    place(lv, "Grand_Stair", prism(F, pts, 1000.0, plane="xz"), (1980, 0, 0), mat="stone", folder="Altar")
    cheek = [(0.0, 0.0), (720.0, 0.0), (720.0, 520.0), (0.0, 140.0)]
    for k, y in enumerate((530, -530)):
        place(lv, "Stair_Cheek_%d" % (k + 1), prism(F, cheek, 60.0, plane="xz"), (1980, y, 0), mat="dark_stone", folder="Altar")
        lv.box("Stair_Cheek_Low_%d" % (k + 1), (1940, y, 70), (80, 60, 140), mat="dark_stone", folder="Altar")
    lv.box("Altar_Terrace", (3300, 0, 220), (1200, 1800, 440), mat="dark_stone", folder="Altar")
    lv.box("SP_altar", (3400, 0, 500), (300, 200, 120), mat="stone", folder="Altar")
    lv.box("Altar_Slab", (3400, 0, 566), (330, 230, 12), mat="gold", folder="Altar")
    lv.shape("Altar_Relic", (3400, 0, 607), (56, 56, 70), mesh=SPHERE, mat="relic", folder="Altar")
    lv.point("Relic_Light", (3400, 0, 700), 25.0, (1.0, 0.5, 0.2), radius=700, source=10.0, scatter=1.0)

    # braziers: a stand, a bowl of coals, a Niagara flame, and the light that breathes with it
    stand, dish = pedestal(F), bowl(F)
    lfs = [brazier_light_function(m + "LF_Fire_%d" % k, k * 2.1) for k in range(3)]
    fire = make_fire(FX, "NS_BrazierFire", size="brazier") if hasattr(unreal, "ToolsetRegistry") and unreal.ToolsetRegistry.is_available() else None
    for i, (x, y, z) in enumerate(SPOTS):
        place(lv, "Brazier_Stand_%d" % (i + 1), stand, (x, y, z), mat="stone", folder="Braziers")
        place(lv, "SP_brazier" if i == 3 else "Brazier_%d" % (i + 1), dish, (x, y, z + 90), mat="steel", folder="Braziers")
        lv.shape("Brazier_Coals", (x, y, z + 90 + 17), (86, 86, 8), mesh=SPHERE, mat="coal", folder="Braziers")
        lv.point("Brazier_Light", (x, y, z + 230), 60.0, (1.0, 0.52, 0.22), radius=1100, source=20.0, scatter=2.5,
                 light_function=lfs[i % 3])
        fill_light(lv, "Brazier_Fill", (x, y, z + 90 + 60), 14.0, (1.0, 0.5, 0.2), 500)
        if fire:
            lv.fx("Brazier_Fire_%d" % (i + 1), (x, y, z + 90 + 18), fire)

    # the warriors
    moves = mv.build_moves("/Game/AshenPillars/Anims")
    blade = sword(F)

    def armed(label, at, facing, move, mesh=MANNY):
        w = lv.figure(label, at, facing, mesh=mesh, anim=moves[move])
        hold_mesh(lv, w, label.replace("SP_", "Hero_") + "_Sword", blade)   # (an SP_ label would make an anchor)
        return w

    armed("SP_warrior", (2720, 0, 440), 180.0, "hero_ready")                                     # waits at the top of the stair
    armed("Kneeler", (3200, -620, 440), 72.0, "kneel_sword", QUINN)                               # before the altar, sword planted
    gap = mv.DUEL_GAP / 2.0
    armed("Duelist_A", (DUEL_AT[0], DUEL_AT[1] - gap, PLAZA), 90.0, "duel_a")                     # two fighters trading blows
    armed("Duelist_B", (DUEL_AT[0], DUEL_AT[1] + gap, PLAZA), 270.0, "duel_b", QUINN)
    armed("Lunger", (PAIR_AT[0], PAIR_AT[1] - gap, PLAZA), 90.0, "thrust_a")                      # a thrust ...
    armed("Dodger", (PAIR_AT[0], PAIR_AT[1] + gap, PLAZA), 270.0, "dodge_b", QUINN)               # ... and the sidestep that lets it pass
    armed("Rallier", (-1200, 150, PLAZA), 0.0, "battle_cry")
    armed("Lookout", (-1750, -700, PLAZA), 180.0, "lookout", QUINN)
    fallen = pose_anim("/Game/AshenPillars/Anims/A_Fallen_FaceDown", floor=1)
    lv.lying("Fallen_Warrior", (-350, 260), along=35.0, pose=fallen, surface=PLAZA, face_up=False)
    lv.shape("Fallen_Sword", (-180, 362, PLAZA + 2.3), (110, 9, 4), mat="steel", rot=(0.0, 60.0, 0.0), folder="Characters")

    life = Ambient(lv, "/Game/AshenPillars/Cinematics/LS_TempleLife", seconds=12.0)
    for k, y in enumerate((1000, -1000)):
        sgn = 1 if k == 0 else -1
        f0, f1 = (0 if sgn > 0 else 180), (180 if sgn > 0 else 360)
        p = walker3(lv, life, "Patrol_%d" % (k + 1),
                    [(0.0, -1300 * sgn, y, PLAZA, f0), (5.4, 1300 * sgn, y, PLAZA, f0), (6.0, 1300 * sgn, y, PLAZA, f1),
                     (11.4, -1300 * sgn, y, PLAZA, f1), (12.0, -1300 * sgn, y, PLAZA, f0 + 360)],
                    [(0.0, 5.4, "Unarmed/Walk/MF_Unarmed_Walk_Fwd", 0.8), (5.4, 6.0, "Unarmed/MM_Idle", 1.0),
                     (6.0, 11.4, "Unarmed/Walk/MF_Unarmed_Walk_Fwd", 0.8), (11.4, 12.0, "Unarmed/MM_Idle", 1.0)],
                    mesh=QUINN if k else MANNY)
        hold_mesh(lv, p, "Patrol_%d_Sword" % (k + 1), blade)
    # the runner comes in through the gate: up the ramp onto the plaza, then waits at the foot of the stair
    ry = -130   # through the gap between the posts and clear of the fallen lintel, then out to the side of the duel
    runner = walker3(lv, life, "Runner", [(0.0, -2400, ry, 0, 0), (0.527, -2040, ry, 0, 0), (0.761, -1880, ry, PLAZA, 0), (1.32, -1500, -300, PLAZA, 0), (6.0, 1700, -300, PLAZA, 0),
                                          (12.0, 1700, -300, PLAZA, 0)],
                     [(0.0, 6.0, "Unarmed/Jog/MF_Unarmed_Jog_Fwd", 1.1), (6.0, 12.0, "Unarmed/MM_Idle", 1.0)])
    hold_mesh(lv, runner, "Runner_Sword", blade)
    life.place()

    # light: a low sun behind the altar, smoky air with beams between the columns
    lv.sun("Sun", pitch=-9.0, yaw=170.0, lux=6.0, color=(1.0, 0.52, 0.28))
    lv.sky(intensity=0.7)
    lv.fog(0.035, (0.55, 0.38, 0.28), volumetric=True, forward=0.75, extinction=1.2)
    if hasattr(unreal, "ToolsetRegistry") and unreal.ToolsetRegistry.is_available():
        ash = make_dust("/Game/AshenPillars/FX/NS_Ash", make_dust_material(m + "M_Ash"), rate=60.0, size=(1.5, 3.0),
                        color=(0.55, 0.53, 0.5, 0.9), wind=(25.0, 10.0, -40.0), box=(1600.0, 1600.0, 900.0))
        for i, at in enumerate(((-2400, 0, 500), (-1200, 0, 500), (400, 0, 500), (1900, 0, 700), (3200, 0, 900),
                                (-400, 1300, 500), (-400, -1300, 500))):
            lv.fx("Ash_%02d" % (i + 1), at, ash)
        smoke = make_smoke(FX, "NS_RuinSmoke", scale=0.8)
        for i, (x, y, z) in enumerate(((-600, 650, PLAZA + 436), (-1000, -650, PLAZA + 296), (1000, -650, PLAZA + 686))):
            lv.fx("Smoke_%02d" % (i + 1), (x, y, z), smoke)
        sparks = make_sparks(FX, "NS_Clash", interval=mv.SPARK_INTERVAL)   # bursts at 1.6 s and 3.6 s: the duel's two hits
        lv.fx("Clash_Sparks", (DUEL_AT[0], DUEL_AT[1], PLAZA + 158.0), sparks)
    else:
        warn("no ash: enable the ToolsetRegistry and NiagaraToolsets plugins in the project")
    lv.exposure(1.3)
    lv.player_start((-2600, 0, 0), yaw=0.0)
