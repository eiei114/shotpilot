"""Character animation for the sample levels: moves keyed from a few poses (make_move) and
one-pose bodies (pose_anim), both written as AnimSequence assets for the template's mannequins.

Why not Control Rig + Sequencer (Epic's AnimationAssistantToolset route)? Tried in a commandlet
(UE 5.8): ControlRigSequencerLibrary.find_or_create_control_rig_track asserts in Slate
(FSlateApplication::Get, ControlRigEditor) and takes the process down, so it cannot run in the
-NullRHI build. make_move therefore solves the poses itself (a small FK + two-bone IK on the
mannequin's skeleton, below) and writes the bones with AnimDataController.set_bone_track_keys,
the same writer pose_anim uses.

A move is a few keyed poses. Every value is in the standing mannequin's component space, in cm
and degrees: +Z up, +Y the way it faces, +X its left (its right is -X). The floor is z = 0.

    make_move("/Game/Lanternless/Anims/A_Wave", seconds=2.0, loop=True, base=IDLE, keys=[
        (0.0, {"arm_l": {"hand": (30, 20, 185)}, "arm_r": {"hand": (-30, 20, 160)}}),
        (1.0, {"arm_l": {"hand": (30, 20, 160)}, "arm_r": {"hand": (-30, 20, 185)}}),
    ])

Pose vocabulary (every field optional; a field not keyed at a time is held from its nearest
keys, and a move that starts without it eases in from the base animation):

    pelvis       (dx, dy, dz)       cm from where the base has the pelvis (rest: z = 93.8), so
                                    (0, 0, -41) is a drop of 41 cm. Feet stay planted: the legs bend.
    body         (pitch, yaw, roll) degrees, the whole body turned about the pelvis. pitch > 0
                                    tips forward (90 = torso horizontal, face down), yaw > 0 turns
                                    left, roll > 0 leans toward its left.
    spine        (bend, twist, side) degrees spread over spine_01..05, same signs as body.
    head         (pitch, yaw, roll) degrees spread over neck_01, neck_02 and head; pitch > 0 looks down.
    look         (x, y, z)          a direction the head faces whatever the body does (neck takes the
                                    swing). (0, 1, 0) = straight ahead even on all fours.
    shoulder_l / shoulder_r (raise, forward)  degrees of the clavicle.
    arm_l / arm_r   {"hand": (x, y, z)         where the wrist should be (two-bone IK),
                     "pole": (dx, dy, dz)      the way the elbow points (default down, back, out),
                     "hand_dir": (dx, dy, dz)  which way the fingers point (default: follow the forearm),
                     "aim": ((ux,uy,uz), (lx,ly,lz))   OR the upper arm / forearm directions directly
                                               (FK; use instead of "hand"),
                     None                      let go: fall back to the carried arm}
    leg_l / leg_r   {"foot": (x, y, z)         where the ankle should be (default: planted where the base
                                               has it, so a lower pelvis bends the knees),
                     "pole": ..., "aim": ..., "foot_dir": (dx, dy, dz)  # which way ankle -> toes points
                     None                      plant again}

The rest ankle is at z = 8.1 (the ball of the foot at z = 0.2), the hip is 2.6 cm below the pelvis
and 9 cm to the side, thigh 43 cm, shin 40 cm, upper arm 28 cm, forearm 26 cm.
"""

import math

import unreal

from .core import ANIMS, EAL, log, warn

IDLE = ANIMS + "/Unarmed/MM_Idle"
FPS = 30  # the template's animations run at 30 fps; the editor wants a multiple or a factor of that

# --------------------------------------------------------------------------- vector and quaternion math
# Quaternions are (x, y, z, w), Hamilton order like FQuat: qmul(a, b) turns by b, then by a.

def _add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def _sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def _mul(a, k):
    return (a[0] * k, a[1] * k, a[2] * k)


def _dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def _cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def _len(v):
    return math.sqrt(_dot(v, v))


def _norm(v):
    n = _len(v) or 1.0
    return (v[0] / n, v[1] / n, v[2] / n)


def _lerp(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def _qmul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return (aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz)


def _qinv(q):
    return (-q[0], -q[1], -q[2], q[3])


def _qrot(q, v):
    x, y, z, w = q
    t = (2 * (y * v[2] - z * v[1]), 2 * (z * v[0] - x * v[2]), 2 * (x * v[1] - y * v[0]))
    return (v[0] + w * t[0] + y * t[2] - z * t[1], v[1] + w * t[1] + z * t[0] - x * t[2],
            v[2] + w * t[2] + x * t[1] - y * t[0])


def _qaxis(axis, degrees):
    x, y, z = _norm(axis)
    h = math.radians(degrees) / 2
    return (x * math.sin(h), y * math.sin(h), z * math.sin(h), math.cos(h))


def _qbetween(u, v):
    """The shortest turn taking direction u to direction v."""
    u, v = _norm(u), _norm(v)
    c = _cross(u, v)
    s, d = _len(c), _dot(u, v)
    if s < 1e-6:
        if d > 0:
            return (0.0, 0.0, 0.0, 1.0)
        # opposite directions: half a turn about any axis across u
        side = _cross(u, (1.0, 0.0, 0.0) if abs(u[0]) < 0.9 else (0.0, 1.0, 0.0))
        return _qaxis(side, 180.0)
    return _qaxis(c, math.degrees(math.atan2(s, d)))


def _qslerp(a, b, t):
    d = sum(x * y for x, y in zip(a, b))
    if d < 0:
        b, d = tuple(-x for x in b), -d
    if d > 0.9995:
        q = tuple(x + (y - x) * t for x, y in zip(a, b))
    else:
        th = math.acos(d)
        s = math.sin(th)
        wa, wb = math.sin((1 - t) * th) / s, math.sin(t * th) / s
        q = tuple(wa * x + wb * y for x, y in zip(a, b))
    n = math.sqrt(sum(x * x for x in q)) or 1.0
    return tuple(x / n for x in q)


def _turn(pitch=0.0, yaw=0.0, roll=0.0):
    """A turn from the named angles (see the module doc for the signs), applied pitch, roll, yaw."""
    return _qmul(_qaxis((0, 0, 1), -yaw), _qmul(_qaxis((0, 1, 0), roll), _qaxis((1, 0, 0), -pitch)))


# --------------------------------------------------------------------------- the rig
# Only these bones are solved; every other bone (twists, correctives, fingers) keeps its local
# transform from the base, so it follows its parent.

PARENT = {"root": None, "pelvis": "root"}
for _i in range(1, 6):
    PARENT["spine_0%d" % _i] = "pelvis" if _i == 1 else "spine_0%d" % (_i - 1)
PARENT.update({"neck_01": "spine_05", "neck_02": "neck_01", "head": "neck_02",
               "ik_foot_root": "root", "ik_hand_root": "root", "ik_hand_gun": "ik_hand_root"})
for _s in "lr":
    PARENT.update({"clavicle_" + _s: "spine_05", "upperarm_" + _s: "clavicle_" + _s,
                   "lowerarm_" + _s: "upperarm_" + _s, "hand_" + _s: "lowerarm_" + _s,
                   "middle_metacarpal_" + _s: "hand_" + _s, "middle_01_" + _s: "middle_metacarpal_" + _s,
                   "thigh_" + _s: "pelvis", "calf_" + _s: "thigh_" + _s, "foot_" + _s: "calf_" + _s,
                   "ball_" + _s: "foot_" + _s, "ik_foot_" + _s: "ik_foot_root", "ik_hand_" + _s: "ik_hand_gun"})
ORDER = []  # parents before children
for _b in PARENT:
    chain = []
    while _b is not None and _b not in ORDER:
        chain.append(_b)
        _b = PARENT[_b]
    ORDER.extend(reversed(chain))

def _is_below(c, b):
    while c is not None:
        if c == b:
            return True
        c = PARENT[c]
    return False


SUBTREE = {b: [c for c in ORDER if _is_below(c, b)] for b in ORDER}
# the bones that mirror a hand or foot (they follow it, with the offset the base has)
FOLLOW = {"ik_foot_l": "foot_l", "ik_foot_r": "foot_r", "ik_hand_l": "hand_l", "ik_hand_r": "hand_r",
          "ik_hand_gun": "hand_r"}
SPINE = ("spine_01", "spine_02", "spine_03", "spine_04", "spine_05")
SPINE_W = (0.15, 0.2, 0.25, 0.25, 0.15)
NECK = (("neck_01", 0.25), ("neck_02", 0.25), ("head", 0.5))
SIGN = {"l": 1.0, "r": -1.0}  # +X is the mannequin's left


class Body:
    """World (component space) transforms of the rig bones, built from local ones. Turning or
    moving a bone takes its whole subtree with it, so the children keep their local transforms
    (the way AnimPose.set_bone_pose does it)."""

    def __init__(self, local):
        self.pos, self.rot = {}, {}
        for b in ORDER:
            p, (lp, lq) = PARENT[b], local[b]
            if p is None:
                self.pos[b], self.rot[b] = tuple(lp), tuple(lq)
            else:
                self.pos[b] = _add(self.pos[p], _qrot(self.rot[p], lp))
                self.rot[b] = _qmul(self.rot[p], lq)

    def copy(self):
        c = object.__new__(Body)
        c.pos, c.rot = dict(self.pos), dict(self.rot)
        return c

    def turn(self, bone, q):
        """Turn `bone` and everything below it by the world turn q, about the bone's own position."""
        o = self.pos[bone]
        for b in SUBTREE[bone]:
            self.rot[b] = _qmul(q, self.rot[b])
            if b != bone:
                self.pos[b] = _add(o, _qrot(q, _sub(self.pos[b], o)))

    def shift(self, bone, d):
        for b in SUBTREE[bone]:
            self.pos[b] = _add(self.pos[b], d)

    def aim(self, bone, child, direction, amount=1.0):
        """Turn `bone` so that the way to `child` points along `direction` (amount < 1: part of the way)."""
        q = _qbetween(_sub(self.pos[child], self.pos[bone]), direction)
        self.turn(bone, _qslerp((0.0, 0.0, 0.0, 1.0), q, amount))

    def local(self):
        out = {}
        for b in ORDER:
            p = PARENT[b]
            if p is None:
                out[b] = (self.pos[b], self.rot[b])
            else:
                ip = _qinv(self.rot[p])
                out[b] = (_qrot(ip, _sub(self.pos[b], self.pos[p])), _qmul(ip, self.rot[b]))
        return out


def _two_bone(root, target, l1, l2, pole):
    """Where the middle joint goes for a chain of lengths l1, l2 from `root` reaching `target`
    (clamped to what the chain can reach), bending toward `pole`. Returns (mid, end)."""
    d = _sub(target, root)
    dist = _len(d)
    u = _mul(d, 1.0 / dist) if dist > 1e-6 else (0.0, 0.0, -1.0)
    dist = min(max(dist, abs(l1 - l2) + 1e-3), l1 + l2 - 1e-3)
    cos_a = max(-1.0, min(1.0, (l1 * l1 + dist * dist - l2 * l2) / (2 * l1 * dist)))
    n = _sub(pole, _mul(u, _dot(pole, u)))
    if _len(n) < 1e-6:
        n = _cross(u, (1.0, 0.0, 0.0) if abs(u[0]) < 0.9 else (0.0, 1.0, 0.0))
    n = _norm(n)
    mid = _add(_add(root, _mul(u, l1 * cos_a)), _mul(n, l1 * math.sqrt(1.0 - cos_a * cos_a)))
    return mid, _add(root, _mul(u, dist))


# --------------------------------------------------------------------------- keyed channels

# name -> (kind, size). "off": an amount added to the base (nothing = zero). "tgt": a place or
# direction that replaces the base's (nothing = carry the base's, blended in and out).
VOCAB = {"pelvis": ("off", 3), "body": ("off", 3), "spine": ("off", 3), "head": ("off", 3),
         "shoulder_l": ("off", 2), "shoulder_r": ("off", 2), "look": ("tgt", 3)}
LIMB_FIELDS = {"arm": {"hand": 3, "pole": 3, "hand_dir": 3, "aim": 6},
               "leg": {"foot": 3, "pole": 3, "foot_dir": 3, "aim": 6}}


def _ease(kind, u):
    if kind == "linear":
        return u
    return u * u * (3.0 - 2.0 * u)


def _hermite(p0, p1, m0, m1, u):
    u2, u3 = u * u, u * u * u
    return (2 * u3 - 3 * u2 + 1) * p0 + (u3 - 2 * u2 + u) * m0 + (-2 * u3 + 3 * u2) * p1 + (u3 - u2) * m1


class Track:
    """The keys of one channel: [(seconds, value or None)]. None releases a "tgt" channel (and
    means zero for an "off" one). at() gives (value, weight): the weight is how far a "tgt"
    channel overrides what the base would do."""

    def __init__(self, name, keys, kind, size, period, loop, ease):
        self.name, self.kind, self.size = name, kind, size
        self.period, self.loop, self.ease = period, loop, ease
        ks = sorted(keys, key=lambda k: k[0])
        if kind == "off":
            zero = (0.0,) * size
            ks = [(t, zero if v is None else v) for t, v in ks]
        if loop:
            if len({round(t % period, 6) for t, _ in ks}) != len(ks):
                raise ValueError("%s: two keys at the same time of the loop (the end of a loop is its start)" % name)
            ks = [(t - period, v) for t, v in ks] + ks + [(t + period, v) for t, v in ks]
        elif ks[0][0] > 1e-9:  # the move starts from the base
            ks.insert(0, (0.0, (0.0,) * size if kind == "off" else None))
        self.keys = ks

    def at(self, t):
        ks = self.keys
        if self.loop:
            t %= self.period
        elif t >= ks[-1][0] or len(ks) == 1:
            v = ks[-1][1]
            return (v, 1.0) if v is not None else (None, 0.0)
        i = max(j for j in range(len(ks) - 1) if ks[j][0] <= t)
        (t0, v0), (t1, v1) = ks[i], ks[i + 1]
        u = min(max((t - t0) / max(t1 - t0, 1e-9), 0.0), 1.0)
        if v0 is not None and v1 is not None:
            if self.ease == "cr":
                (ta, va), (tb, vb) = ks[max(i - 1, 0)], ks[min(i + 2, len(ks) - 1)]
                if va is not None and vb is not None:
                    span = t1 - t0
                    ma, mb = span / max(t1 - ta, 1e-9), span / max(tb - t0, 1e-9)
                    return tuple(_hermite(v0[k], v1[k], (v1[k] - va[k]) * ma, (vb[k] - v0[k]) * mb, u)
                                 for k in range(self.size)), 1.0
            return _lerp(v0, v1, _ease(self.ease, u)), 1.0
        e = _ease(self.ease, u)
        if v0 is None and v1 is None:
            return None, 0.0
        return (v1, e) if v0 is None else (v0, 1.0 - e)


def _tracks(keys, period, loop, ease):
    """Split [(t, pose dict)] into one Track per channel, checking the names as it goes."""
    raw = {}  # channel -> {"kind", "size", "keys"}

    def put(name, kind, size, t, value):
        if value is not None and len(value) != size:
            raise ValueError("%s needs %d numbers, got %r" % (name, size, value))
        ch = raw.setdefault(name, {"kind": kind, "size": size, "keys": []})
        ch["keys"].append((float(t), None if value is None else tuple(float(x) for x in value)))

    for t, pose in keys:
        if t < 0 or t > period + 1e-9:
            raise ValueError("key at %.2f s is outside the move (0 to %.2f s)" % (t, period))
        for name, value in pose.items():
            limb = name.split("_")[0]
            if name in VOCAB:
                kind, size = VOCAB[name]
                put(name, kind, size, t, value)
            elif limb in LIMB_FIELDS and name[-2:] in ("_l", "_r") and name.count("_") == 1:
                fields = LIMB_FIELDS[limb]
                if value is None:  # let go of every field this move ever keys on the limb
                    for f in fields:
                        if (name + "." + f) in raw or f == ("hand" if limb == "arm" else "foot"):
                            put(name + "." + f, "tgt", 3 if f != "aim" else 6, t, None)
                    continue
                if "aim" in value and ("hand" in value or "foot" in value):
                    raise ValueError("%s: 'aim' and a target together" % name)
                for f, v in value.items():
                    if f not in fields:
                        raise ValueError("%s: unknown field %r (use %s)" % (name, f, ", ".join(sorted(fields))))
                    if f == "aim":
                        v = None if v is None else tuple(v[0]) + tuple(v[1])
                    put(name + "." + f, "tgt", fields[f], t, v)
            else:
                raise ValueError("unknown pose field %r (see the vocabulary at the top of anim.py)" % name)
    return {n: Track(n, c["keys"], c["kind"], c["size"], period, loop, ease) for n, c in raw.items()}


# --------------------------------------------------------------------------- solving a frame

def _euler(v):
    return _turn(v[0], v[1], v[2])


def _solve(local, ch, floor):
    """The rig's local transforms for one frame: `local` is the base's, `ch` the channel values at
    this time ({name: (value, weight)})."""
    rest = Body(local)
    b = rest.copy()

    def val(name):
        return ch.get(name, (None, 0.0))

    # pelvis, body and spine
    v, _ = val("pelvis")
    if v is not None:
        b.shift("pelvis", v)
    v, _ = val("body")
    if v is not None:
        b.turn("pelvis", _euler(v))
    v, _ = val("spine")
    if v is not None:
        for bone, w in zip(SPINE, SPINE_W):
            b.turn(bone, _euler(_mul(v, w)))
    # shoulders: raise the clavicle's tip (about the front axis) and swing it forward
    for s in "lr":
        v, _ = val("shoulder_" + s)
        if v is not None:
            sg = SIGN[s]
            b.turn("clavicle_" + s, _qmul(_qaxis((0, 0, 1), sg * v[1]), _qaxis((0, 1, 0), -sg * v[0])))
    # the head: aim it, then add the offsets
    fwd = _qrot(_qinv(rest.rot["head"]), (0.0, 1.0, 0.0))
    v, w = val("look")
    if v is not None and w > 0:
        rem = 1.0
        for bone, share in NECK:
            cur = _qrot(b.rot["head"], fwd)
            goal = _norm(_lerp(cur, _norm(v), w))
            q = _qbetween(cur, goal)
            b.turn(bone, _qslerp((0.0, 0.0, 0.0, 1.0), q, share / rem))
            rem -= share
    v, _ = val("head")
    if v is not None:
        for bone, share in NECK:
            b.turn(bone, _euler(_mul(v, share)))
    # arms
    for s in "lr":
        _limb(b, rest, ch, "arm_" + s, "upperarm_" + s, "lowerarm_" + s, "hand_" + s, "hand",
              (0.35 * SIGN[s], -0.35, -1.0), plant=False, tip="middle_01_" + s, floor=None)
    # legs: planted where the base has the ankles unless told otherwise
    for s in "lr":
        _limb(b, rest, ch, "leg_" + s, "thigh_" + s, "calf_" + s, "foot_" + s, "foot",
              None, plant=True, tip="ball_" + s, floor=floor)
    # the IK helper bones ride on the hands and feet as they do in the base
    for ik, bone in FOLLOW.items():
        off_p = _qrot(_qinv(rest.rot[bone]), _sub(rest.pos[ik], rest.pos[bone]))
        off_q = _qmul(_qinv(rest.rot[bone]), rest.rot[ik])
        b.pos[ik] = _add(b.pos[bone], _qrot(b.rot[bone], off_p))
        b.rot[ik] = _qmul(b.rot[bone], off_q)
    out = b.local()
    for name, (pos, rot) in out.items():  # keep the sign of the base's quaternion (no flips in playback)
        if sum(x * y for x, y in zip(rot, local[name][1])) < 0:
            out[name] = (pos, tuple(-x for x in rot))
    return out


def _limb(b, rest, ch, name, top, mid, end, goal, default_pole, plant, tip, floor):
    """One arm or leg: IK to a target (or the plant), or FK from two directions, then the hand or foot.
    A planted foot is lifted, if need be, until its toes clear `floor` (z in cm)."""
    def val(f):
        return ch.get(name + "." + f, (None, 0.0))

    target, wt = val(goal)
    up, wu = val("aim")
    aimed = up is not None and wu > 0
    l1 = _len(_sub(rest.pos[mid], rest.pos[top]))
    l2 = _len(_sub(rest.pos[end], rest.pos[mid]))
    root = b.pos[top]
    pole, _ = val("pole")
    if pole is None:
        if default_pole is None:  # the way the base bends its knee (forward if it is straight)
            line = _norm(_sub(rest.pos[end], rest.pos[top]))
            d = _sub(rest.pos[mid], _add(rest.pos[top], _mul(line, _dot(_sub(rest.pos[mid], rest.pos[top]), line))))
            pole = _norm(d) if _len(d) > 0.5 else (0.0, 1.0, 0.0)
        else:
            pole = default_pole
    home = rest.pos[end] if plant else b.pos[end]
    snap = (dict(b.pos), dict(b.rot))

    def run(lift):
        if aimed:
            cur_u = _norm(_sub(b.pos[mid], b.pos[top]))
            cur_l = _norm(_sub(b.pos[end], b.pos[mid]))
            b.aim(top, mid, _norm(_lerp(cur_u, _norm(up[:3]), wu)))
            b.aim(mid, end, _norm(_lerp(cur_l, _norm(up[3:]), wu)))
        elif target is not None or plant:
            goal_pos = home if target is None else _lerp(home, target, wt)
            m, e = _two_bone(root, (goal_pos[0], goal_pos[1], goal_pos[2] + lift), l1, l2, pole)
            b.aim(top, mid, _sub(m, root))
            b.aim(mid, end, _sub(e, m))
        # the hand follows the forearm; a planted foot keeps the flatness the base has; either may be turned
        if plant:
            b.turn(end, _qmul(rest.rot[end], _qinv(b.rot[end])))
        d, wd = val(goal + "_dir")
        if d is not None and wd > 0:
            b.aim(end, tip, _norm(_lerp(_norm(_sub(b.pos[tip], b.pos[end])), _norm(d), wd)))

    run(0.0)
    lift = 0.0
    for _ in range(3):
        if not plant or floor is None or b.pos[tip][2] >= floor - 0.05:
            break
        lift += floor - b.pos[tip][2]
        b.pos, b.rot = dict(snap[0]), dict(snap[1])
        run(lift)


# --------------------------------------------------------------------------- bases

class Base:
    """The animation a move is layered over, as local transforms of every bone: a still frame
    (the standing pose) or a looping animation such as the idle, sampled by time."""

    def __init__(self, names, sample, length=0.0):
        self.names, self.sample, self.length = names, sample, length  # sample(t) -> [(pos, rot, scl)] per name
        self._still = None

    def at(self, t, rate=1.0):
        if self.length <= 0:
            if self._still is None:
                self._still = self.sample(0.0)
            return self._still
        return self.sample((t * rate) % self.length)


def _ue_base(path, still):
    X = unreal.AnimPoseExtensions
    L = unreal.AnimPoseSpaces.LOCAL
    seq = EAL.load_asset(path)
    if seq is None:
        raise ValueError("base animation not found: " + path)
    opts = unreal.AnimPoseEvaluationOptions()
    names = [str(n) for n in X.get_bone_names(X.get_anim_pose_at_time(seq, 0.0, opts))]

    def sample(t):
        pose = X.get_anim_pose_at_time(seq, t, opts)
        out = []
        for n in names:
            tr = X.get_bone_pose(pose, n, L)
            tt, r, sc = tr.translation, tr.rotation, tr.scale3d
            out.append(((tt.x, tt.y, tt.z), (r.x, r.y, r.z, r.w), (sc.x, sc.y, sc.z)))
        return out

    return Base(names, sample, 0.0 if still else float(seq.get_editor_property("sequence_length")))


def _frames(base, tracks, seconds, loop, rate, floor=0.2):
    """Every bone's local transform at each frame 0..n (n = seconds * FPS; the last equals the first for a loop)."""
    n = max(1, int(round(seconds * FPS)))
    names = base.names
    idx = {nm: i for i, nm in enumerate(names)}
    missing = [b for b in ORDER if b not in idx]
    if missing:
        raise ValueError("the base animation lacks bones: %s" % ", ".join(missing))
    first = base.at(0.0, rate)
    last = base.at(n / FPS, rate)
    frames = []
    for f in range(n + 1):
        t = f / FPS
        cur = base.at(t, rate)
        if loop and base.length > 0:  # ramp the base's seam out, so the loop closes
            k = f / n
            cur = [((c[0][0] + (a[0][0] - z[0][0]) * k, c[0][1] + (a[0][1] - z[0][1]) * k,
                     c[0][2] + (a[0][2] - z[0][2]) * k),
                    _qmul(_qslerp((0.0, 0.0, 0.0, 1.0), _qmul(a[1], _qinv(z[1])), k), c[1]), c[2])
                   for c, a, z in zip(cur, first, last)]
        local = {b: (cur[idx[b]][0], cur[idx[b]][1]) for b in ORDER}
        ch = {name: tr.at(t) for name, tr in tracks.items()}
        solved = _solve(local, ch, floor)
        row = list(cur)
        for b, (p, q) in solved.items():
            row[idx[b]] = (p, q, cur[idx[b]][2])
        frames.append(row)
    if loop:
        frames[-1] = frames[0]
    return names, frames


# --------------------------------------------------------------------------- writing assets

def _write(path, names, frames, skeleton_src=IDLE):
    """An AnimSequence at `path` holding `frames` (per frame, a (pos, rot, scale) per bone name)."""
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    seq = EAL.duplicate_asset(skeleton_src, path)  # same skeleton and bone tracks as the template's idle
    ctrl = seq.get_editor_property("controller")
    n = len(frames) - 1
    ctrl.open_bracket("make_move")
    ctrl.set_number_of_frames(unreal.FrameNumber(n))  # n frames = n + 1 keys, n / 30 seconds
    failed = []
    for i, name in enumerate(names):
        pos = [unreal.Vector(*f[i][0]) for f in frames]
        rot = [unreal.Quat(*f[i][1]) for f in frames]
        scl = [unreal.Vector(*f[i][2]) for f in frames]
        if not ctrl.set_bone_track_keys(name, pos, rot, scl):
            try:
                ctrl.add_bone_track(name)
                ok = ctrl.set_bone_track_keys(name, pos, rot, scl)
            except Exception:
                ok = False
            if not ok:
                failed.append(name)
    ctrl.close_bracket()
    if failed:
        warn("%s: %d bone tracks not written (%s...)" % (path, len(failed), ", ".join(failed[:5])))
    EAL.save_loaded_asset(seq)
    return seq


def make_move(path, seconds, keys, loop=False, base=None, base_rate=None, ease="smooth", floor=0.2):
    """Write the AnimSequence `path` (a /Game path) for a mannequin: `keys` is [(seconds, pose)]
    (the vocabulary is at the top of this file). loop=True makes the last frame equal the first and
    wraps the keys around, so the move repeats without a seam. `base` is an animation to layer
    over (IDLE keeps the idle's breathing alive; None = the standing pose, still); `base_rate`
    speeds it up or slows it down. ease: "smooth" (eases in and out of every key), "linear" or
    "cr" (smooth through the keys, for repeating motion like a crawl). floor: the z the toes of a
    planted foot are kept above (a foot turning on the spot never dips through; None = no guard)."""
    if ease not in ("smooth", "linear", "cr"):
        raise ValueError("ease must be smooth, linear or cr")
    if not keys:
        raise ValueError("a move needs at least one key")
    tracks = _tracks(keys, float(seconds), loop, ease)
    b = _ue_base(base or IDLE, still=base is None)
    names, frames = _frames(b, tracks, float(seconds), loop, base_rate or 1.0, floor)
    _write(path, names, frames)
    log("move %s: %d frames, %.2f s%s" % (path, len(frames) - 1, seconds, ", loop" if loop else ""))
    return path


def make_pose(path, pose, base=None):
    """A one-frame-long move that just holds `pose` (same vocabulary as a key)."""
    return make_move(path, 1.0 / FPS, [(0.0, pose)], loop=False, base=base)


# --------------------------------------------------------------------------- posed bodies
# The template's death animations stop mid-fall (games hand over to ragdoll physics there), so a
# body on the floor gets its own one-pose animation: the idle pose with its limbs turned joint by
# joint. Directions are in the standing mannequin's component space: +Z up, +Y the way it faces,
# +X its left. `floor` is the side that will touch the ground once the body is laid down (-1: its
# back, lying face up; +1: its front, face down), so limbs can rest on the ground.

def body_pose(floor, kind="fallen"):
    """(aim, turn) for a body. "fallen": sprawled where it fell; "laid": laid out on a bed, one hand
    on its stomach. aim: (bone, child, direction) points the bone at its child along a direction;
    turn: (bone, axis, degrees) spins a bone where it is."""
    f = floor
    if kind == "laid":
        aim = [
            ("upperarm_r", "lowerarm_r", (-0.28, 0.15 * f, -0.95)),  # right arm along its side
            ("lowerarm_r", "hand_r", (-0.2, 0.2 * f, -0.96)),
            ("upperarm_l", "lowerarm_l", (0.3, 0.1 * f, -0.95)),     # left hand on its stomach
            ("lowerarm_l", "hand_l", (-0.75, -0.55 * f, 0.35)),
            ("thigh_l", "calf_l", (0.08, 0.0, -1.0)),
            ("calf_l", "foot_l", (0.06, 0.05 * f, -1.0)),
            ("thigh_r", "calf_r", (-0.08, 0.0, -1.0)),
            ("calf_r", "foot_r", (-0.06, 0.05 * f, -1.0)),
        ]
        turn = [("head", (0.0, 0.0, 1.0), -35.0), ("foot_l", (0.0, 0.0, 1.0), 20.0),
                ("foot_r", (0.0, 0.0, 1.0), -20.0)]
        return aim, turn
    aim = [
        ("spine_03", "spine_05", (0.12, 0.0, 1.0)),            # a slight sideways slump
        ("upperarm_l", "lowerarm_l", (0.6, 0.25 * f, 0.75)),   # left arm flung up beside the head
        ("lowerarm_l", "hand_l", (0.15, 0.3 * f, 0.95)),
        ("upperarm_r", "lowerarm_r", (-0.85, 0.3 * f, -0.4)),  # right arm out to the side
        ("lowerarm_r", "hand_r", (-0.55, 0.45 * f, -0.7)),
        # left knee: raised when face up; drawn out to the side along the ground when face down
        ("thigh_l", "calf_l", (0.45, 0.35 if f < 0 else 0.1, -0.82)),
        ("calf_l", "foot_l", (0.05, -0.45 if f < 0 else 0.05, -0.9)),
        ("thigh_r", "calf_r", (-0.2, 0.08 * f, -0.98)),        # right leg straight, apart
        ("calf_r", "foot_r", (-0.18, 0.1 * f, -0.98)),
    ]
    turn = [("head", (0.0, 0.0, 1.0), 55.0), ("head", (0.0, 1.0, 0.0), -12.0),
            ("foot_l", (0.0, 0.0, 1.0), 25.0), ("foot_r", (0.0, 0.0, 1.0), -30.0)]
    return aim, turn


def pose_anim(path, floor, kind="fallen"):
    """A one-pose animation of a body (see body_pose), written over a copy of MM_Idle."""
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    seq = EAL.duplicate_asset(ANIMS + "/Unarmed/MM_Idle", path)
    X = unreal.AnimPoseExtensions
    W, L = unreal.AnimPoseSpaces.WORLD, unreal.AnimPoseSpaces.LOCAL
    pose = X.get_anim_pose_at_time(seq, 0.0, unreal.AnimPoseEvaluationOptions())

    def where(bone):
        v = X.get_bone_pose(pose, bone, W).translation
        return (v.x, v.y, v.z)

    def rotate(bone, q):
        nonlocal pose
        tr = X.get_bone_pose(pose, bone, W)
        r = tr.rotation
        n = _qmul(q, (r.x, r.y, r.z, r.w))
        tr.set_editor_property("rotation", unreal.Quat(n[0], n[1], n[2], n[3]))
        out = X.set_bone_pose(pose, tr, bone, W)  # children keep their local transforms: they follow
        if isinstance(out, unreal.AnimPose):
            pose = out

    aim, turn = body_pose(floor, kind)
    for bone, child, direction in aim:
        a, b = where(bone), where(child)
        rotate(bone, _qbetween((b[0] - a[0], b[1] - a[1], b[2] - a[2]), direction))
    for bone, axis, degrees in turn:
        rotate(bone, _qaxis(axis, degrees))

    keys = unreal.AnimationLibrary.get_num_frames(seq) + 1
    ctrl = seq.get_editor_property("controller")
    ctrl.open_bracket("Fallen body pose")
    failed = []
    for bone in X.get_bone_names(pose):
        t = X.get_bone_pose(pose, bone, L)
        pos, rot, scl = [t.translation] * keys, [t.rotation] * keys, [t.scale3d] * keys
        if not ctrl.set_bone_track_keys(bone, pos, rot, scl):
            try:
                ctrl.add_bone_track(bone)
                ok = ctrl.set_bone_track_keys(bone, pos, rot, scl)
            except Exception:
                ok = False
            if not ok:
                failed.append(str(bone))
    ctrl.close_bracket()
    if failed:
        warn("pose %s: %d bone tracks not written (%s...)" % (path, len(failed), ", ".join(failed[:5])))
    EAL.save_loaded_asset(seq)
    return path
