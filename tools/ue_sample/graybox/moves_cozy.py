"""Moves for the cozy game (KettleHill): the things a village teaser films people doing, made with
anim.make_move from a few keyed poses, plus the props they hold.

    paths = build_moves("/Game/KettleHill/Anims")      # {name: asset path}
    held  = held_props(paths["pour_tea"], "pour_tea")  # what goes in the hands, as attach transforms

Moves (all loop, in place; the walking one is layered over the template's walk):
    pour_tea      right hand pours from an iron kettle over a cup, left hand on the hip
    sip_tea       both hands raise a cup to the face, sip, lower it (standing)
    sit_sip       the same, seated on a 45 cm bench
    sweep         a broom swept side to side with the whole torso
    water_plants  a watering can tipped over the crops, swinging side to side
    wave_hello    a friendly wave
    chat_seated   seated, turned toward the neighbour, hands telling a story (mirror = left-handed)
    stretch       both arms up and back, a long breath, then relaxed
    carry_basket  walking with a basket held in front (needs the walk clip's own rate)
    hang_laundry  bend to the basket, reach up and peg a cloth on the line
    fishing       a rod held out over the water, a tug now and then

Poses are in the mannequin's component space (cm; +Y forward, +X its left). The props are built
with GeometryScripting (shapes.py) with their origin in the middle of the part that is gripped, and
`held_props` works out where the hand bone has to carry them so that, at one reference moment of the
move, the prop has the orientation the move wants (a kettle level with its spout forward). The
hand then moves the prop rigidly through the rest of the move.
"""

import math

import unreal

from . import anim as A
from .core import EAL, ANIMS, log, warn
from . import shapes as S

IDLE = A.IDLE
WALK = ANIMS + "/Unarmed/Walk/MF_Unarmed_Walk_Fwd"
IDLE_SECONDS = 227 / 30.0  # the template idle is this long: a held pose that loops over it never jumps


# --------------------------------------------------------------------------- pose helpers

def hold(pose):
    """A pose held for the length of the idle (one key: the idle keeps breathing underneath)."""
    return dict(seconds=IDLE_SECONDS, keys=[(0.0, pose)], loop=True, base=IDLE)


def seated(**over):
    """The seated base: on a 45 cm seat whose top spans y -35..15 (the pelvis dropped, the feet in front)."""
    pose = {"pelvis": (-2, -9, -40.5),
            "leg_l": {"foot": (13, 36, 8), "pole": (0.1, 1, 0.2)},
            "leg_r": {"foot": (-13, 36, 8), "pole": (-0.1, 1, 0.2)},
            "arm_l": {"hand": (16, 22, 62), "pole": (0.6, -0.5, -0.4)},
            "arm_r": {"hand": (-16, 22, 62), "pole": (-0.6, -0.5, -0.4)}}
    pose.update(over)
    return pose


# --------------------------------------------------------------------------- the moves

def pour_tea():
    def pose(hand, tilt, head, spine):
        return {"arm_r": {"hand": hand, "hand_dir": tilt, "pole": (-0.6, -0.3, -1.0)},
                "arm_l": {"hand": (17, 10, 96), "pole": (0.4, -0.7, -0.5)},
                "spine": (spine, 0, 0), "head": (head, 0, 0), "look": (0.0, 1.0, -0.45)}
    lvl = pose((-15, 38, 106), (0.0, 1.0, 0.0), 8, 5)
    pour = pose((-13, 42, 110), (0.0, 0.8, -0.6), 14, 9)
    return dict(seconds=5.0, loop=True, base=IDLE, keys=[
        (0.0, lvl), (1.0, lvl), (1.9, pour), (3.5, pour), (4.3, lvl)])


def sip_tea(seat=False):
    def pose(lift, head):
        r = {"hand": (-5, 22 - lift * 4, 118 + lift * 32), "hand_dir": (0.3, 0.7, 0.6 * lift), "pole": (-0.6, -0.4, -1.0)}
        l = {"hand": (7, 22 - lift * 3, 109 + lift * 30), "hand_dir": (-0.2, 0.8, 0.3), "pole": (0.6, -0.4, -0.8)}
        if seat:  # sitting, the hands start lower
            r["hand"] = (r["hand"][0], r["hand"][1], r["hand"][2] - 14)
            l["hand"] = (l["hand"][0], l["hand"][1], l["hand"][2] - 14)
        p = {"arm_r": r, "arm_l": l, "head": (head, 0, 0), "spine": (4 - lift * 6, 0, 0)}
        return seated(**p) if seat else p
    low, high = pose(0.0, 6), pose(1.0, -9)
    return dict(seconds=6.0, loop=True, base=IDLE, keys=[
        (0.0, low), (0.5, low), (1.8, high), (3.6, high), (4.9, low)])


def sweep():
    def pose(x, twist):
        # the hands hold the shaft ahead of the body; the whole chest follows the stroke
        return {"arm_r": {"hand": (-14 + x, 34, 102), "hand_dir": (0.8, 0.2, -0.2), "pole": (-0.7, -0.3, -0.6)},
                "arm_l": {"hand": (6 + x, 44, 80), "hand_dir": (-0.8, 0.2, -0.2), "pole": (0.7, -0.3, -0.6)},
                "spine": (14, twist, 0), "head": (24, -twist * 0.5, 0), "pelvis": (0, 4, -3)}
    L, R = pose(-18, 16), pose(18, -16)
    return dict(seconds=3.2, loop=True, base=IDLE, ease="cr", keys=[(0.0, L), (0.8, pose(0, 0)), (1.6, R), (2.4, pose(0, 0))])


def water_plants():
    def pose(twist, tip):
        return {"arm_r": {"hand": (-16, 36, 98), "hand_dir": (0.0, 1.0, -tip), "pole": (-0.7, -0.3, -0.8)},
                "arm_l": {"hand": (15, 12, 98), "pole": (0.5, -0.6, -0.6)},
                "spine": (16, twist, 0), "head": (24, twist * 0.4, 0), "pelvis": (0, 6, -4)}
    a, b = pose(-16, 0.3), pose(16, 0.55)
    return dict(seconds=5.0, loop=True, base=IDLE, ease="cr", keys=[
        (0.0, a), (1.4, pose(0, 0.45)), (2.5, b), (3.7, pose(0, 0.45))])


def wave_hello():
    def pose(x):
        return {"arm_r": {"hand": (-26 + x, 18, 178), "pole": (-0.7, -0.1, -1.0)}, "shoulder_r": (10, 0),
                "arm_l": {"hand": (19, 8, 94), "pole": (0.4, -0.7, -0.5)},
                "head": (0, 0, -7), "spine": (-2, -6, 3)}
    return dict(seconds=2.4, loop=True, base=IDLE, ease="cr", keys=[
        (0.0, pose(-16)), (0.6, pose(10)), (1.2, pose(-16)), (1.8, pose(10))])


def chat_seated(mirror=False):
    """Seated and turned toward the neighbour (to its left; mirror = to its right), talking with one hand."""
    s = -1.0 if mirror else 1.0
    talk, rest = ("arm_l", "arm_r") if mirror else ("arm_r", "arm_l")

    sgn = 1 if talk == "arm_r" else -1  # arm_r reaches toward -X
    def g(x, y, z, head=(0, 0, 0), up=0.0):
        pose = {talk: {"hand": (-14 * sgn + x * sgn, y, z), "pole": (-0.5 * sgn, -0.3, -0.9)},
                rest: {"hand": (16 * -sgn, 22, 62), "pole": (0.6 * -sgn, -0.5, -0.4)},
                "spine": (3 + up, 18 * s, 0), "head": (head[0], head[1], head[2])}
        return seated(**pose)
    return dict(seconds=9.0, loop=True, base=IDLE, ease="cr", keys=[
        (0.0, g(0, 24, 66, (4, 6 * s, 0))), (1.2, g(2, 34, 112, (2, 8 * s, 4), 2)), (2.4, g(10, 38, 128, (0, 10 * s, -4), 3)),
        (3.4, g(4, 30, 100, (6, 4 * s, 2))), (4.6, g(-2, 28, 70, (8, 10 * s, 0))), (5.6, g(8, 40, 122, (0, 8 * s, 5), 2)),
        (6.8, g(12, 34, 138, (-2, 10 * s, 3), 4)), (7.9, g(2, 26, 76, (6, 6 * s, 0)))])


def stretch():
    rest = {"arm_l": None, "arm_r": None, "spine": None, "head": None, "shoulder_l": None, "shoulder_r": None,
            "pelvis": None}
    up = {"arm_l": {"hand": (20, 6, 204), "pole": (0.4, -0.4, -0.3)}, "arm_r": {"hand": (-20, 6, 204), "pole": (-0.4, -0.4, -0.3)},
          "shoulder_l": (14, 0), "shoulder_r": (14, 0), "spine": (-14, 0, 0), "head": (-14, 0, 0),
          "pelvis": (0, -3, 0)}
    out = {"arm_l": {"hand": (44, 6, 168), "pole": (0.6, -0.2, -0.5)}, "arm_r": {"hand": (-44, 6, 168), "pole": (-0.6, -0.2, -0.5)},
           "shoulder_l": (8, 0), "shoulder_r": (8, 0), "spine": (-6, 0, 0), "head": (-6, 0, 0)}
    return dict(seconds=7.0, loop=True, base=IDLE, ease="cr", keys=[
        (0.0, rest), (1.0, out), (2.2, up), (4.2, up), (5.3, out), (6.2, rest)])


def carry_basket():
    pose = {"arm_r": {"hand": (-20, 32, 96), "hand_dir": (0.5, 0.8, 0.0), "pole": (-0.7, -0.2, -0.7)},
            "arm_l": {"hand": (20, 32, 96), "hand_dir": (-0.5, 0.8, 0.0), "pole": (0.7, -0.2, -0.7)},
            "spine": (-3, 0, 0), "head": (3, 0, 0)}
    return dict(seconds=1.5, loop=True, base=WALK, keys=[(0.0, pose)])


def hang_laundry():
    # down to the basket for a cloth, up to the line, peg it, and again
    low = {"arm_r": {"hand": (-14, 30, 56), "pole": (-0.5, -0.5, -0.4)}, "arm_l": {"hand": (14, 32, 52), "pole": (0.5, -0.5, -0.4)},
           "spine": (36, 0, 0), "head": (16, 0, 0), "pelvis": (0, 10, -10)}
    reach = {"arm_r": {"hand": (-20, 30, 186), "pole": (-0.7, -0.1, -0.4)}, "arm_l": {"hand": (16, 26, 172), "pole": (0.5, -0.4, -0.6)},
             "shoulder_r": (12, 0), "spine": (-6, 0, 0), "head": (-10, 0, 0)}
    peg = dict(reach, arm_r={"hand": (-12, 34, 184), "pole": (-0.7, -0.1, -0.4)})
    mid = {"arm_r": {"hand": (-18, 24, 130), "pole": (-0.6, -0.3, -0.6)}, "arm_l": {"hand": (16, 24, 126), "pole": (0.5, -0.4, -0.6)},
           "spine": (8, 0, 0)}
    return dict(seconds=6.0, loop=True, base=IDLE, ease="cr", keys=[
        (0.0, low), (1.0, low), (2.0, mid), (2.8, reach), (3.4, peg), (3.9, reach), (4.6, mid), (5.4, low)])


def fishing():
    def pose(lift, sway):
        return {"arm_r": {"hand": (-13, 36 + lift * 4, 100 + lift * 10), "hand_dir": (0.0, 1.0, 0.25 + lift * 0.3),
                          "pole": (-0.7, -0.3, -0.8)},
                "arm_l": {"hand": (8, 24 + lift * 3, 108 + lift * 8), "pole": (0.6, -0.3, -0.7)},
                "spine": (2 - lift * 4, sway, 0), "head": (-6 + lift * 3, sway * 0.3, 0)}
    wait, tug = pose(0.0, 0), pose(1.0, 2)
    return dict(seconds=8.0, loop=True, base=IDLE, ease="cr", keys=[
        (0.0, wait), (2.5, pose(0.1, 2)), (4.5, wait), (5.2, tug), (5.6, pose(0.3, 0)), (6.4, wait)])


MOVES = {
    "pour_tea": ("A_PourTea", pour_tea),
    "sip_tea": ("A_SipTea", sip_tea),
    "sit_sip": ("A_SitSip", lambda: sip_tea(True)),
    "sweep": ("A_Sweep", sweep),
    "water_plants": ("A_WaterPlants", water_plants),
    "wave_hello": ("A_WaveHello", wave_hello),
    "chat_seated": ("A_ChatSeated", chat_seated),
    "chat_seated_m": ("A_ChatSeatedM", lambda: chat_seated(True)),
    "stretch": ("A_Stretch", stretch),
    "carry_basket": ("A_CarryBasket", carry_basket),
    "hang_laundry": ("A_HangLaundry", hang_laundry),
    "fishing": ("A_Fishing", fishing),
}

# the seat the seated moves are made for (cm above the floor the mannequin stands on)
SEAT_HEIGHT = 45.0


def build_moves(folder):
    """Write every move under `folder`; returns {name: asset path}."""
    out = {}
    for name, (asset, fn) in MOVES.items():
        spec = fn()
        path = folder.rstrip("/") + "/" + asset
        A.make_move(path, spec["seconds"], spec["keys"], loop=spec.get("loop", False), base=spec.get("base"),
                    ease=spec.get("ease", "smooth"))
        out[name] = path
    return out


# --------------------------------------------------------------------------- props

def _solid(folder, kind, params, build, angle=45.0):
    return S._make(folder, kind, params, build, angle)


def cup_mesh(F):
    prof = [(0, 0), (2.6, 0), (3.0, 0.8), (3.9, 6.4), (3.9, 6.8), (3.4, 6.8), (3.1, 1.6), (0, 1.6)]
    return _solid(F, "Cup", [1], lambda m: m.revolve(prof, 16))


def kettle_mesh(F):
    return S.kettle(F, 11, 15)


def teapot_mesh(F):
    return S.kettle(F, 9, 11)


def broom_mesh(F):
    """The shaft, grip at the origin (shaft 40 cm above it, the head's tip 115 cm below)."""
    def build(m):
        m.tube([(0, 0, -92), (0, 0, 40)], 1.5, 8)
    return _solid(F, "BroomShaft", [1], build)


def broom_head_mesh(F):
    prof = [(0, -115), (8.5, -115), (10.5, -100), (2.4, -86), (0, -86)]
    return _solid(F, "BroomHead", [1], lambda m: m.revolve(prof, 14))


def can_mesh(F):
    """A watering can, spout toward +X, the grip on the back handle at x -14."""
    body = [(0, 0), (9.5, 0), (10, 1), (10, 20), (9.5, 22), (0, 22)]

    def build(m):
        m.revolve(body, 18)
        sp = [(7, 0, 5), (20, 0, 14), (34, 0, 26), (46, 0, 34)]
        m.tube(sp, [2.4, 2.1, 1.9, 2.0], 8)
        m.tube([(46, 0, 34), (52, 0, 37)], [2.0, 5.0], 10)  # the rose
        m.tube([(-8, 0, 20), (-16, 0, 19), (-18, 0, 9), (-10, 0, 3)], 1.4, 6)
        m.tube([(-8, 0, 21), (0, 0, 31), (8, 0, 21)], 1.2, 6)  # the carrying handle over the top
    return _solid(F, "Can", [1], build)


def basket_mesh(F):
    bowl = [(0, 0), (14, 0), (18, 2), (21, 14), (19.5, 14), (17, 3), (0, 3)]

    def build(m):
        m.revolve(bowl, 20)
        arc = [(20 * math.cos(math.radians(a)), 0.0, 13 + 17 * math.sin(math.radians(a))) for a in range(180, -1, -20)]
        m.tube(arc, 1.2, 6)
    return _solid(F, "Basket", [1], build)


def basket_goods_mesh(F):
    dome = [(0, 12), (16, 12), (14, 17), (8, 21), (0, 22)]
    return _solid(F, "BasketGoods", [1], lambda m: m.revolve(dome, 14))


def cloth_mesh(F):
    return S.prism(F, [(-14, -38), (14, -38), (14, 0), (-14, 0)], 0.8)


def rod_mesh(F):
    """A rod from the butt (x -25) out along +X to the tip (x 270)."""
    n = 8
    path = [(-25 + 295 * i / n, 0.0, 0.0) for i in range(n + 1)]
    radii = [1.8 - 1.3 * i / n for i in range(n + 1)]
    return _solid(F, "Rod", [1], lambda m: m.tube(path, radii, 6))


LINE_LENGTH = 280.0  # from the rod tip down to the water (the dock is 31 cm up, the rod tip about 3 m)


def line_mesh(F):
    """The fishing line from the rod tip, hanging straight down in the world as the rod stands at its
    reference moment, with a float at the end (in the rod's frame: the rod is along +X)."""
    fwd, up = HOLDS["fishing"][0][3:5]
    qp = _quat_from_axes(fwd, up)
    down = tuple(round(v, 4) for v in A._qrot(A._qinv(qp), (0, 0, -1)))
    tip = (270.0, 0.0, 0.0)
    end = tuple(tip[i] + down[i] * LINE_LENGTH for i in range(3))
    bob = tuple(end[i] + down[i] * 6.0 for i in range(3))

    def build(m):
        m.tube([tip, end], 0.35, 4)
        m.tube([end, bob], 2.4, 10)
    return _solid(F, "FishLine", [list(down), LINE_LENGTH], build)


PROPS = {"cup": (cup_mesh, "ceramic"), "kettle": (kettle_mesh, "iron"), "broom": (broom_mesh, "wood"),
         "broom_head": (broom_head_mesh, "straw"), "can": (can_mesh, "tin"), "basket": (basket_mesh, "wicker"),
         "goods": (basket_goods_mesh, "produce"), "cloth": (cloth_mesh, "cloth_white"), "rod": (rod_mesh, "bamboo"),
         "line": (line_mesh, "line")}

# the grip point inside each mesh (where the palm closes), in the mesh frame
GRIP = {"cup": (0, 0, 3.4), "kettle": (-16.0, 0, 7.5), "broom": (0, 0, 0), "broom_head": (0, 0, 0),
        "can": (-14, 0, 9), "basket": (0, 0, 8), "goods": (0, 0, 8), "cloth": (0, 0, -2), "rod": (0, 0, 0), "line": (0, 0, 0)}


# --------------------------------------------------------------------------- holding

# What each move puts in which hand, and how the prop should stand at one moment of the move:
# (bone, [mesh keys], time, forward axis of the mesh in component space, up axis, palm offset (bone frame))
HOLDS = {
    "pour_tea": [("hand_r", ["kettle"], 0.5, (0, 1, 0), (0, 0, 1))],
    "sip_tea": [("hand_r", ["cup"], 0.0, (0, 1, 0), (0, 0, 1))],
    "sit_sip": [("hand_r", ["cup"], 0.0, (0, 1, 0), (0, 0, 1))],
    "sweep": [("hand_r", ["broom", "broom_head"], 0.8, (1, 0, 0), (0, -0.4, 0.92))],
    "water_plants": [("hand_r", ["can"], 1.4, (0, 1, -0.25), (0, 0.25, 0.97))],
    # the basket rides on the chest, not in a hand: the walk swings the hands through every roll
    "carry_basket": [("spine_03", ["basket", "goods"], 0.0, (1, 0, 0), (0, 0, 1), (0.0, 38.0, 94.0))],
    "fishing": [("hand_r", ["rod", "line"], 0.0, (0, 0.8, 0.6), (0, -0.6, 0.8))],
}

# a palm sits about this far along the fingers from the wrist bone, in the bone's frame
PALM = {"hand_r": (-6.0, 1.2, 0.0), "hand_l": (6.0, -1.2, 0.0)}


def _norm(v):
    n = math.sqrt(sum(x * x for x in v)) or 1.0
    return tuple(x / n for x in v)


def _quat_from_axes(x, z):
    """The rotation taking the local +X to `x` and +Z to `z` (z is made square to x)."""
    x = _norm(x)
    d = sum(a * b for a, b in zip(x, z))
    z = _norm(tuple(z[i] - x[i] * d for i in range(3)))
    y = A._cross(z, x)
    m = ((x[0], y[0], z[0]), (x[1], y[1], z[1]), (x[2], y[2], z[2]))
    tr = m[0][0] + m[1][1] + m[2][2]
    if tr > 0:
        s = math.sqrt(tr + 1.0) * 2
        return ((m[2][1] - m[1][2]) / s, (m[0][2] - m[2][0]) / s, (m[1][0] - m[0][1]) / s, 0.25 * s)
    if m[0][0] > m[1][1] and m[0][0] > m[2][2]:
        s = math.sqrt(1.0 + m[0][0] - m[1][1] - m[2][2]) * 2
        return (0.25 * s, (m[0][1] + m[1][0]) / s, (m[0][2] + m[2][0]) / s, (m[2][1] - m[1][2]) / s)
    if m[1][1] > m[2][2]:
        s = math.sqrt(1.0 + m[1][1] - m[0][0] - m[2][2]) * 2
        return ((m[0][1] + m[1][0]) / s, 0.25 * s, (m[1][2] + m[2][1]) / s, (m[0][2] - m[2][0]) / s)
    s = math.sqrt(1.0 + m[2][2] - m[0][0] - m[1][1]) * 2
    return ((m[0][2] + m[2][0]) / s, (m[1][2] + m[2][1]) / s, 0.25 * s, (m[1][0] - m[0][1]) / s)


def bone_at(anim_path, bone, t):
    """(position, rotation quaternion) of a bone in component space at time t of an animation."""
    X = unreal.AnimPoseExtensions
    seq = EAL.load_asset(anim_path)
    pose = X.get_anim_pose_at_time(seq, min(t, float(seq.get_editor_property("sequence_length"))),
                                   unreal.AnimPoseEvaluationOptions())
    tr = X.get_bone_pose(pose, bone, unreal.AnimPoseSpaces.WORLD)
    return ((tr.translation.x, tr.translation.y, tr.translation.z),
            (tr.rotation.x, tr.rotation.y, tr.rotation.z, tr.rotation.w))


def grip_transform(anim_path, bone, t, mesh_key, fwd, up, at=None):
    """(location, rotator) of a mesh relative to `bone` such that at time t the palm is on the mesh's
    grip point and the mesh's +X points along `fwd`, its +Z along `up` (component space)."""
    p, q = bone_at(anim_path, bone, t)
    qp = _quat_from_axes(fwd, up)
    palm = A._add(p, A._qrot(q, PALM[bone])) if at is None else at
    g = GRIP[mesh_key]
    origin = A._sub(palm, A._qrot(qp, g))
    inv = A._qinv(q)
    loc = A._qrot(inv, A._sub(origin, p))
    rel = A._qmul(inv, qp)
    return unreal.Vector(*loc), unreal.Quat(*rel).rotator()


def prop_axes(anim_path, bone, loc, rot, times):
    """Where a held mesh's +X and +Z point (component space) at each time: a check that it stays upright."""
    out = []
    qrel = unreal.Rotator(roll=rot.roll, pitch=rot.pitch, yaw=rot.yaw).quaternion()
    qrel = (qrel.x, qrel.y, qrel.z, qrel.w)
    for t in times:
        p, q = bone_at(anim_path, bone, t)
        qw = A._qmul(q, qrel)
        out.append((round(t, 2), tuple(round(v, 2) for v in A._qrot(qw, (1, 0, 0))),
                    tuple(round(v, 2) for v in A._qrot(qw, (0, 0, 1)))))
    return out


def held_props(anim_path, move):
    """[(bone, mesh key, location, rotator)] for what `move` holds (empty when it holds nothing)."""
    out = []
    for bone, keys, t, fwd, up, *at in HOLDS.get(move, []):
        for key in keys:
            loc, rot = grip_transform(anim_path, bone, t, key, fwd, up, at[0] if at else None)
            out.append((bone, key, loc, rot))
    return out


def prop_mesh(folder, key):
    """The StaticMesh of a prop, built under `folder`."""
    return PROPS[key][0](folder)
