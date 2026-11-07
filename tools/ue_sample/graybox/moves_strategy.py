"""Moves for the strategy sample (Gridhold): a commander over the war table, a guard at attention,
a drill that marches in place, an archer, a smith, a crate carrier, a lookout and a flag signaller.

Each move is keyed from a few poses with anim.make_move (the vocabulary is at the top of anim.py:
cm and degrees in the standing mannequin's space, +X its left, +Y forward, +Z up). `build_moves(folder)`
writes them all as /Game assets and returns {name: asset path}. Things carried in a hand (a spear, a
bow, a crate) are placed with `hold_part`, which reads the finished animation, so the prop is put where
the hand really is whatever the hand bone's own axes are.
"""

import unreal

from . import anim as A
from .anim import IDLE, make_move
from .core import ANIMS, EAL, MANNY, load, log, set_play_rate, transform_channels, warn

WALK = ANIMS + "/Unarmed/Walk/MF_Unarmed_Walk_Fwd"
IDLE_SECONDS = 227 / 30.0  # a "stay" move: one pose held as a loop over the idle's own length


def _stay(pose):
    """A pose held for ever (the idle keeps breathing under it)."""
    return dict(seconds=IDLE_SECONDS, keys=[(0.0, pose)], loop=True, base=IDLE)


WALK_BASE = {"path": WALK}  # the in-place copy of the walk, once build_moves has made it


def _walk_seconds():
    return float(EAL.load_asset(WALK).get_editor_property("sequence_length"))


def inplace_walk(folder):
    """The template's walk with its root motion taken out (the root bone pinned at the origin), so a
    move layered on it stays on the spot and a level sequence supplies the travel."""
    path = folder.rstrip("/") + "/A_walk_inplace"
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    seq = EAL.duplicate_asset(WALK, path)
    X = unreal.AnimPoseExtensions
    tr = X.get_bone_pose(X.get_anim_pose_at_time(seq, 0.0, unreal.AnimPoseEvaluationOptions()), "root",
                         unreal.AnimPoseSpaces.LOCAL)
    n = unreal.AnimationLibrary.get_num_frames(seq) + 1
    ctrl = seq.get_editor_property("controller")
    ctrl.open_bracket("walk in place")
    ok = ctrl.set_bone_track_keys("root", [tr.translation] * n, [tr.rotation] * n, [tr.scale3d] * n)
    ctrl.close_bracket()
    if not ok:
        warn("walk in place: the root track was not written")
    try:
        seq.set_editor_property("enable_root_motion", False)
    except Exception as e:
        warn("root motion flag not cleared (%s)" % e)
    EAL.save_loaded_asset(seq)
    WALK_BASE["path"] = path
    return path


# ------------------------------------------------------------------------------ the moves
# Right arm holds a spear upright at its side: the hand at hip height, the elbow out and back.
SPEAR_ARM = {"hand": (-23.0, 16.0, 108.0), "pole": (-0.5, -0.6, -0.4)}


def sentry_spear():
    """At attention with a spear grounded beside the right foot, chest up, eyes front."""
    return _stay({"arm_r": SPEAR_ARM, "shoulder_r": (-2.0, 3.0), "spine": (-3.0, 0.0, 0.0),
                  "head": (-2.0, 0.0, 0.0)})


def drill_march():
    """Marching in place at two steps a second, spear upright in the right hand, left arm swinging."""
    up = {"foot": (13.0, 20.0, 38.0), "pole": (0.0, 1.0, 0.25)}
    upr = {"foot": (-13.0, 20.0, 38.0), "pole": (0.0, 1.0, 0.25)}
    back, fwd = {"hand": (27.0, -12.0, 92.0), "pole": (0.5, -1.0, -0.2)}, {"hand": (25.0, 26.0, 108.0), "pole": (0.5, 0.2, -1.0)}
    keys = [(0.0, {"leg_l": None, "leg_r": None, "pelvis": (0, 0, -1.8), "arm_l": back, "arm_r": SPEAR_ARM}),
            (0.25, {"leg_l": up, "pelvis": (0, 0, 0.5)}),
            (0.5, {"leg_l": None, "leg_r": None, "pelvis": (0, 0, -1.8), "arm_l": back}),
            (0.75, {"leg_r": upr, "pelvis": (0, 0, 0.5), "arm_l": fwd})]
    return dict(seconds=1.0, keys=keys, loop=True, base=IDLE, ease="cr")


def point_map():
    """Leaning over the war table, one hand on the hip, the other sweeping out over the map."""
    def pose(x, y, z):
        return {"arm_r": {"hand": (x, y, z), "pole": (-0.6, -0.3, -0.5)}}
    base = {"body": (14.0, 0.0, 0.0), "spine": (10.0, 0.0, 0.0), "look": (0.0, 1.0, -0.85),
            "arm_l": {"hand": (30.0, 4.0, 99.0), "pole": (1.0, -0.4, 0.0)}}
    keys = [(0.0, dict(base, **pose(-14.0, 62.0, 104.0))),
            (1.1, pose(4.0, 70.0, 100.0)),
            (2.2, pose(-30.0, 58.0, 102.0))]
    return dict(seconds=3.3, keys=keys, loop=True, base=IDLE, ease="cr")


def study_map():
    """Leaning on the table with the left hand, the right hand at the chin, the head turning over the map."""
    base = {"body": (18.0, 0.0, 0.0), "spine": (8.0, 0.0, 0.0),
            "arm_l": {"hand": (26.0, 46.0, 93.0), "pole": (0.6, -0.8, -0.3)},
            "arm_r": {"hand": (-6.0, 22.0, 146.0), "pole": (-0.5, 0.2, -1.0)}}
    keys = [(0.0, dict(base, look=(0.35, 1.0, -0.8))), (2.0, {"look": (-0.35, 1.0, -0.8)})]
    return dict(seconds=4.0, keys=keys, loop=True, base=IDLE, ease="cr")


def place_piece():
    """Picking a game piece up and setting it down on the map, the left hand steady on the table."""
    def hand(x, y, z):
        return {"arm_r": {"hand": (x, y, z), "pole": (-0.6, -0.4, -0.5)}}
    base = {"body": (16.0, 0.0, 0.0), "spine": (6.0, 0.0, 0.0), "look": (0.0, 1.0, -0.75),
            "arm_l": {"hand": (26.0, 46.0, 93.0), "pole": (0.6, -0.8, -0.3)}}
    keys = [(0.0, dict(base, **hand(-20.0, 32.0, 124.0))), (0.9, hand(-14.0, 58.0, 108.0)),
            (1.5, hand(-14.0, 60.0, 95.0)), (2.0, hand(-14.0, 58.0, 101.0)), (2.8, hand(-22.0, 34.0, 122.0))]
    return dict(seconds=3.6, keys=keys, loop=True, base=IDLE, ease="cr")


def salute():
    """A fist to the brow, elbow out, standing tall."""
    return _stay({"arm_r": {"hand": (-9.0, 14.0, 163.0), "pole": (-1.0, -0.3, 0.2)}, "shoulder_r": (6.0, 0.0),
                  "spine": (-3.0, 0.0, 0.0), "head": (-2.0, 0.0, 0.0)})


def lookout_shade():
    """A flat hand above the eyes against the glare, the head slowly sweeping the horizon."""
    base = {"arm_r": {"hand": (-7.0, 17.0, 170.0), "pole": (-1.0, -0.2, 0.3)}, "shoulder_r": (6.0, 0.0),
            "arm_l": {"hand": (29.0, 4.0, 98.0), "pole": (1.0, -0.4, 0.0)}, "spine": (3.0, 0.0, 0.0)}
    keys = [(0.0, dict(base, look=(-0.45, 1.0, -0.12))), (2.5, {"look": (0.35, 1.0, -0.1)})]
    return dict(seconds=5.0, keys=keys, loop=True, base=IDLE, ease="cr")


FLAG_DIR = ((-0.55, 0.1, 0.85), (0.55, 0.1, 0.85))


def wave_flag():
    """Swinging a signal flag from side to side over the head, the left fist on the hip."""
    def arm(x, z, d):
        return {"arm_r": {"hand": (x, 26.0, z), "hand_dir": d, "pole": (-0.7, -0.2, -0.3)}}
    keys = [(0.0, dict(arm(-30.0, 152.0, FLAG_DIR[0]), arm_l={"hand": (29.0, 4.0, 98.0), "pole": (1.0, -0.4, 0.0)},
                       shoulder_r=(8.0, 0.0))),
            (0.6, arm(-2.0, 168.0, FLAG_DIR[1]))]
    return dict(seconds=1.2, keys=keys, loop=True, base=IDLE, ease="smooth")


def draw_bow():
    """An archer: nock, draw to the cheek, hold, loose, nock again."""
    bow_arm = {"hand": (9.0, 58.0, 150.0), "pole": (0.6, 0.0, -1.0)}

    def string(x, y, z, pole=(-0.3, -1.0, 0.1)):
        return {"arm_r": {"hand": (x, y, z), "pole": pole}}
    keys = [(0.0, dict(string(-1.0, 46.0, 148.0, (-0.4, -0.3, -1.0)), arm_l=bow_arm, shoulder_l=(4.0, 6.0),
                       shoulder_r=(4.0, -4.0), look=(0.1, 1.0, 0.0), body=(0.0, 0.0, 0.0))),
            (1.4, string(-6.0, 11.0, 152.0)),
            (2.6, string(-6.5, 10.0, 152.5)),
            (2.75, string(-24.0, 2.0, 148.0, (-0.6, -1.0, 0.0))),
            (3.3, string(-8.0, 28.0, 146.0, (-0.4, -0.6, -0.8)))]
    return dict(seconds=4.0, keys=keys, loop=True, base=IDLE, ease="smooth")


def hammer():
    """A smith's swing: up, down on the anvil, a short rebound, up again. The left hand steadies the work."""
    def hand(x, y, z, p=(-0.6, -0.5, -0.4)):
        return {"arm_r": {"hand": (x, y, z), "pole": p}}
    base = {"body": (10.0, 0.0, 0.0), "look": (0.0, 1.0, -0.6),
            "arm_l": {"hand": (22.0, 42.0, 90.0), "pole": (0.6, -0.4, -0.5)}}
    keys = [(0.0, dict(base, **hand(-18.0, 20.0, 176.0, (-0.5, -0.8, 0.2)))),
            (0.45, hand(-18.0, 46.0, 94.0)),
            (0.58, hand(-18.0, 44.0, 108.0)),
            (0.8, hand(-18.0, 26.0, 150.0, (-0.5, -0.7, -0.1)))]
    return dict(seconds=1.2, keys=keys, loop=True, base=IDLE, ease="cr")


def carry_crate():
    """Walking with a crate held against the chest in both hands (play it on the walk's own speed)."""
    arm = lambda s: {"hand": (s * 21.0, 40.0, 104.0), "pole": (s * 0.9, -0.3, -0.7)}
    keys = [(0.0, {"arm_l": arm(1), "arm_r": arm(-1), "body": (-3.0, 0.0, 0.0)})]
    return dict(seconds=_walk_seconds(), keys=keys, loop=True, base="walk", ease="smooth")


def march_spear():
    """Walking with a spear upright in the right hand, the butt just clear of the ground."""
    keys = [(0.0, {"arm_r": SPEAR_ARM, "shoulder_r": (-2.0, 3.0)})]
    return dict(seconds=_walk_seconds(), keys=keys, loop=True, base="walk", ease="smooth")


MOVES = {"sentry_spear": sentry_spear, "drill_march": drill_march, "point_map": point_map, "study_map": study_map,
         "place_piece": place_piece, "salute": salute, "lookout_shade": lookout_shade, "wave_flag": wave_flag,
         "draw_bow": draw_bow, "hammer": hammer, "carry_crate": carry_crate, "march_spear": march_spear}


def build_moves(folder):
    """Write every move into `folder` (a /Game path) as A_<name>; returns {name: asset path}."""
    out = {"walk_inplace": inplace_walk(folder)}
    for name, fn in MOVES.items():
        spec = fn()
        path = folder.rstrip("/") + "/A_" + name
        base = WALK_BASE["path"] if spec.get("base") == "walk" else spec.get("base", IDLE)
        make_move(path, spec["seconds"], spec["keys"], loop=spec.get("loop", True), base=base,
                  ease=spec.get("ease", "smooth"))
        out[name] = path
    return out


# ------------------------------------------------------------------------------ things in hands

def hand_frame(move_path, bone, t=0.0):
    """((x, y, z), quaternion) of `bone` in the mannequin's space at time t of the animation."""
    X = unreal.AnimPoseExtensions
    seq = EAL.load_asset(move_path)
    pose = X.get_anim_pose_at_time(seq, t, unreal.AnimPoseEvaluationOptions())
    tr = X.get_bone_pose(pose, bone, unreal.AnimPoseSpaces.WORLD)
    p, q = tr.translation, tr.rotation
    return (p.x, p.y, p.z), (q.x, q.y, q.z, q.w)


def hold_part(lv, holder, label, size, mat, move_path, bone, center, axis=(1.0, 0.0, 0.0), t=0.0, at_hand=True):
    """A box of `size` (its long side first) in a mannequin's hand. `center` is where its middle is, in
    the mannequin's space at time t of `move_path`, relative to the hand when `at_hand` (else absolute);
    `axis` is where its long side points in that space. Works out the attachment from the real bone."""
    p, q = hand_frame(move_path, bone, t)
    c = center if not at_hand else (p[0] + center[0], p[1] + center[1], p[2] + center[2])
    iq = A._qinv(q)
    loc = A._qrot(iq, A._sub(c, p))
    rel = A._qmul(iq, A._qbetween((1.0, 0.0, 0.0), axis))
    r = unreal.Quat(*rel).rotator()
    return lv.hold(holder, label, size, mat=mat, socket=bone, loc=loc, rot=(r.pitch, r.yaw, r.roll))


def give_spear(lv, holder, label, move_path, grip=108.0, length=250.0):
    """A spear upright in the right hand. `grip` is the height of the hand above the butt: 108 stands the
    butt on the floor for a hand at hip height, a walker carries it a little higher."""
    mid = length / 2 - grip
    hold_part(lv, holder, label, (length, 4.5, 4.5), "wood", move_path, "hand_r", (0.0, 0.0, mid), axis=(0.0, 0.0, 1.0))
    hold_part(lv, holder, label + "_Tip", (28.0, 7.0, 3.0), "steel", move_path, "hand_r",
              (0.0, 0.0, length - grip + 14.0), axis=(0.0, 0.0, 1.0))


def give_bow(lv, holder, label, move_path):
    """A stave in the left hand, its string just behind it, and an arrow on the right hand at full draw."""
    hold_part(lv, holder, label, (140.0, 4.0, 6.0), "wood", move_path, "hand_l", (0.0, 0.0, 0.0), axis=(0.0, 0.0, 1.0))
    hold_part(lv, holder, label + "_String", (132.0, 1.0, 1.0), "canvas", move_path, "hand_l", (0.0, -7.0, 0.0),
              axis=(0.0, 0.0, 1.0))
    hold_part(lv, holder, label + "_Arrow", (74.0, 1.4, 1.4), "wood", move_path, "hand_r", (0.0, 36.0, 0.0),
              axis=(0.2, 1.0, 0.0), t=1.9)


def give_hammer(lv, holder, label, move_path):
    """A smith's hammer: a short haft and a head, held at the swing's top."""
    hold_part(lv, holder, label, (42.0, 3.5, 3.5), "wood", move_path, "hand_r", (0.0, 8.0, 18.0), axis=(0.0, 0.3, 1.0))
    hold_part(lv, holder, label + "_Head", (7.0, 7.0, 16.0), "steel", move_path, "hand_r", (0.0, 14.0, 38.0),
              axis=(0.0, 0.3, 1.0))


def give_crate(lv, holder, label, move_path, size=(44.0, 40.0, 40.0)):
    """A crate against the chest, in both hands (it is fixed to the right hand, which holds it still)."""
    return hold_part(lv, holder, label, size, "wood", move_path, "hand_r", (0.0, 40.0, 106.0), at_hand=False)


def give_flag(lv, holder, label, move_path, cloth="blue"):
    """A signal flag: a pole upright in the right hand and a cloth near its top, held mid-swing."""
    t = 0.3
    hold_part(lv, holder, label, (210.0, 4.0, 4.0), "wood", move_path, "hand_r", (0.0, 0.0, 70.0), axis=(0.0, 0.0, 1.0), t=t)
    hold_part(lv, holder, label + "_Cloth", (60.0, 3.0, 85.0), cloth, move_path, "hand_r", (-44.0, 0.0, 120.0),
              axis=(0.0, 0.0, 1.0), t=t)


def give_piece(lv, holder, label, move_path, mat="blue"):
    """A game piece in the right hand."""
    hold_part(lv, holder, label, (13.0, 7.0, 7.0), mat, move_path, "hand_r", (0.0, 0.0, 6.0), axis=(0.0, 0.0, 1.0))


# ------------------------------------------------------------------------------ walkers

def walker(life, label, path, clips, mesh=MANNY):
    """Ambient.walker for clips that may be full /Game paths (the moves built here): a mannequin moving
    through `path`, [(seconds, x, y, facing yaw)], playing `clips`, [(from, to, animation, play rate)]."""
    lv = life.lv
    t0, x0, y0, f0 = path[0]
    first = clips[0][2]
    a = lv.mannequin((x0, y0, 0.0), unreal.Rotator(roll=0.0, pitch=0.0, yaw=f0 - 90.0), mesh,
                     first if first.startswith("/") else first)
    lv._done(a, label, "Characters")
    b = life.seq.add_possessable(a)
    sec = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
    sec.set_range(0, life.frames)
    ch = transform_channels(sec)
    lin = unreal.MovieSceneKeyInterpolation.LINEAR
    for (t, x, y, facing) in path:
        for name, value in (("Location.X", x), ("Location.Y", y), ("Location.Z", 0.0), ("Rotation.X", 0.0),
                            ("Rotation.Y", 0.0), ("Rotation.Z", facing - 90.0)):
            ch[name].add_key(unreal.FrameNumber(life._f(t)), float(value), 0.0,
                             unreal.MovieSceneTimeUnit.DISPLAY_RATE, lin)
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
