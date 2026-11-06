"""Moves for an action-adventure teaser (sword fighting), made with anim.make_move.

Every move is a few keyed poses in the standing mannequin's component space (cm, +Y forward,
+X its left, so its right hand, the one that holds the sword, is on the -X side). The sword lies
along the hand's fingers (blade direction = `hand_dir`, 105 cm long from the wrist), so a pose says
where the blade points by the direction of the right hand.

    build_moves("/Game/AshenPillars/Anims") -> {name: asset path}

Moves (all loop, so a mannequin plays them for ever):
    guard         sword up, weight shifting a little
    slash         wind up over the head, strike down, follow through, back to guard
    thrust        pull the sword back, lunge, recover
    block         raise the sword across the face, take the blow, recover
    duel_a/duel_b a pair that must stand `DUEL_GAP` cm apart, facing: A strikes and B blocks at
                  HITS[0], B strikes and A blocks at HITS[1]; the same length, so the hits land
                  every loop at the same point (`DUEL_POINT`, world, between the two)
    thrust_a/dodge_b  a lunge and the sidestep that lets it pass (same length, `DUEL_GAP`
                  apart, lunge at HIT_THRUST)
    kneel_sword   down on one knee, the sword planted tip-down, head bowed
    battle_cry    sword thrust to the sky, chest out, head back, pumping
    lookout       one hand shading the eyes, scanning left and right, sword low
    hero_ready    low guard -> raised guard -> overhead slash -> block -> low guard (the hero)
"""

from .anim import IDLE, make_move
from .core import log

BLADE = 105.0          # sword length from the wrist (cm)
HAND_FINGERS = (-0.993, -0.114, 0.013)  # the direction of hand_r's fingers in the hand's own frame (the sword lies along it)
GRIP_YAW = 6.5         # degrees: turn a held sword by this (yaw, in the hand's frame) so its blade follows the fingers
SPARK_LAG = 0.4        # fx.make_sparks warms up to 0.4 s past a burst: its first burst in play is at interval - 0.4
DUEL_LOOP = 4.0        # seconds: both fighters of a pair loop this long
HITS = (1.6, 3.6)      # when the blades meet (= k * 2.0 - SPARK_LAG, so a spark system with interval 2.0 bursts there)
SPARK_INTERVAL = 2.0
DUEL_GAP = 160.0       # cm between the two fighters of a pair
HIT_THRUST = 1.6       # the lunge of thrust_a (dodge_b steps aside just before it)
PAIR_LOOP = 3.2        # thrust_a / dodge_b


# --------------------------------------------------------------------------- pose helpers

def arm(hand, direction=None, pole=None):
    """An arm field: the wrist at `hand`, the fingers (and the blade) along `direction`."""
    d = {"hand": hand}
    if direction is not None:
        d["hand_dir"] = direction
    if pole is not None:
        d["pole"] = pole
    return d


R_POLE = (-0.45, -0.3, -0.85)   # right elbow: down, back and a little out
L_POLE = (0.45, -0.3, -0.85)
LF = (16.0, 34.0, 8.1)          # left foot forward, right foot back: the sword stance
RF = (-17.0, -18.0, 8.1)


def stance(dz=-10.0, dy=5.0, body=(2.0, 0.0, 0.0), lf=LF, rf=RF, **more):
    """The legs and hips of a sword stance (weight low, left foot forward)."""
    p = {"pelvis": (0.0, dy, dz), "body": body, "leg_l": {"foot": lf}, "leg_r": {"foot": rf},
         "look": (0.0, 1.0, 0.05)}
    p.update(more)
    return p


def pose(base, **more):
    p = dict(base)
    p.update(more)
    return p


GUARD = pose(stance(), arm_r=arm((-18, 30, 120), (0.0, 0.7, 0.71), R_POLE), arm_l=arm((22, 26, 112), None, L_POLE))
GUARD_B = pose(stance(-12.0, 4.0, (3.0, -2.0, 0.0)), arm_r=arm((-18, 33, 124), (0.0, 0.62, 0.78), R_POLE),
               arm_l=arm((22, 24, 116), None, L_POLE))
LOW = pose(stance(-7.0, 4.0, (0.0, 0.0, 0.0)), arm_r=arm((-28, 16, 100), (-0.4, 0.35, -0.85), R_POLE),
           arm_l=arm((22, 16, 96), None, L_POLE))
WINDUP = pose(stance(-8.0, 2.0, (-6.0, 0.0, 0.0), spine=(-12.0, 0.0, 0.0)),
              arm_r=arm((-16, 10, 190), (0.0, -0.5, 0.86), (-0.6, -0.5, -0.1)), arm_l=arm((24, 34, 130), None, L_POLE))
STRIKE = pose(stance(-22.0, 18.0, (10.0, 0.0, 0.0), lf=(16.0, 52.0, 8.1), spine=(14.0, 0.0, 0.0)),
              arm_r=arm((-14, 52, 108), (0.0, 0.97, -0.25), R_POLE), arm_l=arm((20, 12, 125), None, L_POLE))
FOLLOW = pose(stance(-22.0, 18.0, (14.0, 0.0, 0.0), lf=(16.0, 52.0, 8.1), spine=(18.0, 0.0, 0.0)),
              arm_r=arm((-14, 46, 80), (0.0, 0.65, -0.76), R_POLE), arm_l=arm((22, 10, 120), None, L_POLE))
PULL = pose(stance(-12.0, 0.0, (0.0, -8.0, 0.0)), arm_r=arm((-20, 6, 112), (0.0, 1.0, 0.05), R_POLE),
            arm_l=arm((30, -4, 126), None, L_POLE))
LUNGE = pose(stance(-24.0, 20.0, (8.0, -8.0, 0.0), lf=(16.0, 56.0, 8.1), rf=(-17.0, -22.0, 8.1)),
             arm_r=arm((-16, 52, 120), (0.0, 1.0, 0.08), R_POLE), arm_l=arm((28, -6, 130), None, L_POLE))
BLOCK = pose(stance(-12.0, 4.0, (2.0, 0.0, 0.0)), arm_r=arm((-12, 34, 150), (0.8, 0.35, 0.5), R_POLE),
             arm_l=arm((22, 26, 130), None, L_POLE))
BLOCK_HIT = pose(stance(-14.0, 0.0, (-6.0, 0.0, 0.0)), arm_r=arm((-12, 32, 152), (0.8, 0.35, 0.5), R_POLE),
                 arm_l=arm((22, 22, 128), None, L_POLE))


def guard():
    return dict(seconds=4.0, keys=[(0.0, GUARD), (2.0, GUARD_B)], loop=True, ease="cr")


def slash():
    return dict(seconds=3.2, keys=[(0.0, GUARD), (0.9, WINDUP), (1.2, STRIKE), (1.5, FOLLOW), (2.4, GUARD_B)],
                loop=True)


def thrust():
    return dict(seconds=2.8, keys=[(0.0, GUARD), (0.8, PULL), (1.1, LUNGE), (1.6, LUNGE), (2.2, GUARD_B)], loop=True)


def block():
    return dict(seconds=2.4, keys=[(0.0, GUARD), (0.6, BLOCK), (0.85, BLOCK_HIT), (1.4, BLOCK_HIT), (2.0, GUARD_B)],
                loop=True)


# --------------------------------------------------------------------------- the duel
# Two fighters face each other DUEL_GAP apart (A at world (0, -GAP/2) facing +Y, B at (0, +GAP/2) facing -Y).
# Both are posed in their own space, and the blades cross at the same point in either: the middle
# between them, (0, GAP/2, 162). Roles swap in the second exchange, so one strike pose and one block
# pose serve both fighters.

def _toward(hand, goal):
    d = tuple(g - h for g, h in zip(goal, hand))
    n = sum(c * c for c in d) ** 0.5
    return tuple(round(c / n, 3) for c in d)


CLASH = (0.0, DUEL_GAP / 2.0, 162.0)
D_STRIKE_HAND, D_BLOCK_HAND = (-14.0, 33.0, 176.0), (-12.0, 54.0, 128.0)
D_STRIKE = pose(stance(-14.0, 6.0, (6.0, 0.0, 0.0)), arm_r=arm(D_STRIKE_HAND, _toward(D_STRIKE_HAND, CLASH), R_POLE),
                arm_l=arm((20, 12, 126), None, L_POLE))
D_PRESS = pose(D_STRIKE, arm_r=arm((-14, 37, 172), _toward((-14, 37, 172), CLASH), R_POLE))
D_BLOCK = pose(stance(-12.0, 6.0, (4.0, 0.0, 0.0)), arm_r=arm(D_BLOCK_HAND, _toward(D_BLOCK_HAND, CLASH), R_POLE),
               arm_l=arm((22, 24, 130), None, L_POLE))
D_BLOCK_PRESS = pose(D_BLOCK, pelvis=(0.0, 2.0, -15.0), body=(-2.0, 0.0, 0.0))


def duel_a():
    keys = [(0.0, GUARD), (0.8, GUARD_B), (1.2, WINDUP), (HITS[0], D_STRIKE), (2.0, D_PRESS), (2.5, GUARD_B),
            (3.0, BLOCK), (3.45, D_BLOCK), (HITS[1], D_BLOCK)]
    return dict(seconds=DUEL_LOOP, keys=keys, loop=True)


def duel_b():
    keys = [(0.0, GUARD), (0.7, GUARD_B), (1.0, BLOCK), (1.45, D_BLOCK), (HITS[0], D_BLOCK), (2.0, D_BLOCK_PRESS),
            (2.4, GUARD_B), (2.8, WINDUP), (3.2, WINDUP), (HITS[1], D_STRIKE)]
    return dict(seconds=DUEL_LOOP, keys=keys, loop=True)


def thrust_a():
    keys = [(0.0, GUARD), (0.7, PULL), (HIT_THRUST, LUNGE), (2.2, LUNGE), (2.8, GUARD_B)]
    return dict(seconds=PAIR_LOOP, keys=keys, loop=True)


DODGE = pose(stance(-16.0, 0.0, (4.0, 0.0, -10.0), lf=(-14.0, 30.0, 8.1), rf=(-48.0, -16.0, 8.1)),
             pelvis=(-32.0, 0.0, -16.0), arm_r=arm((-24, 24, 112), (0.0, 0.7, 0.7), R_POLE),
             arm_l=arm((6, 20, 110), None, L_POLE))


def dodge_b():
    mid_r = pose(GUARD, pelvis=(-16.0, 4.0, -14.0), leg_r={"foot": (-40.0, -18.0, 26.0)})
    mid_l = pose(DODGE, pelvis=(-30.0, 2.0, -16.0), leg_l={"foot": (0.0, 34.0, 26.0)})
    t = HIT_THRUST
    keys = [(0.0, GUARD), (t - 0.8, GUARD), (t - 0.55, mid_r), (t - 0.35, mid_l), (t - 0.15, DODGE), (t + 0.9, DODGE),
            (t + 1.25, pose(GUARD, pelvis=(-14.0, 4.0, -12.0))), (t + 1.5, GUARD_B)]
    return dict(seconds=PAIR_LOOP, keys=[k for k in keys if k[0] >= 0], loop=True)


# --------------------------------------------------------------------------- one-person moves

KNEEL = {"pelvis": (0.0, 4.0, -42.0), "spine": (8.0, 0.0, 0.0), "head": (18.0, 0.0, 0.0),
         "leg_l": {"foot": (13.0, 40.0, 8.0), "pole": (0.0, 1.0, 0.3)},
         "leg_r": {"foot": (-9.0, -36.0, 10.0), "foot_dir": (0.0, -0.85, -0.5), "pole": (0.0, 0.0, -1.0)},
         "arm_r": arm((-8, 40, 94), (0.0, 0.05, -1.0), R_POLE), "arm_l": arm((6, 40, 100), None, L_POLE)}


def kneel_sword():
    """Held (a loop of one pose over the idle's breathing)."""
    return dict(seconds=227 / 30.0, keys=[(0.0, KNEEL)], loop=True)


CRY_LOW = pose(stance(-10.0, 2.0, (0.0, 0.0, 0.0), lf=(24.0, 28.0, 8.1), rf=(-26.0, -14.0, 8.1)),
               arm_r=arm((-24, 40, 150), (0.1, 0.4, 0.9), R_POLE), arm_l=arm((24, 28, 120), None, L_POLE),
               head=(-8.0, 0.0, 0.0))
CRY_UP = pose(CRY_LOW, pelvis=(0.0, -2.0, -8.0), spine=(-14.0, 0.0, 0.0), head=(-24.0, 0.0, 0.0),
              arm_r=arm((-20, 14, 202), (0.05, 0.1, 1.0), (-0.6, -0.3, -0.2)), arm_l=arm((22, 24, 152), None, L_POLE),
              look=(0.0, 0.5, 1.0))
CRY_PUMP = pose(CRY_UP, arm_r=arm((-20, 16, 210), (0.05, 0.1, 1.0), (-0.6, -0.3, -0.2)), spine=(-18.0, 0.0, 0.0),
                look=(0.0, 0.4, 1.0))


def battle_cry():
    return dict(seconds=4.0, keys=[(0.0, CRY_LOW), (1.0, CRY_UP), (1.8, CRY_PUMP), (2.4, CRY_UP), (3.0, CRY_PUMP),
                                   (3.5, CRY_UP)], loop=True, ease="cr")


def lookout():
    base = pose(stance(-6.0, 3.0, (4.0, 0.0, 0.0)), arm_r=arm((-22, 22, 100), (0.05, 0.6, -0.8), R_POLE),
                arm_l=arm((4, 20, 166), None, (0.9, 0.0, 0.35)))

    def at(yaw, x):
        return pose(base, body=(4.0, yaw, 0.0), look=(x, 1.0, 0.0))
    return dict(seconds=8.0, keys=[(0.0, at(0.0, 0.0)), (2.0, at(-14.0, -0.8)), (4.0, at(-14.0, -0.8)),
                                   (5.0, at(8.0, 0.7)), (7.0, at(8.0, 0.7))], loop=True, ease="cr")


def hero_ready():
    """The hero's loop: re-set the sword from low, wind up, strike, take a blow, back to low."""
    keys = [(0.0, LOW), (0.8, LOW), (1.5, GUARD), (2.0, GUARD_B), (2.6, WINDUP), (2.95, STRIKE), (3.3, FOLLOW),
            (4.2, GUARD), (5.0, BLOCK), (5.25, BLOCK_HIT), (5.8, BLOCK_HIT), (6.6, GUARD_B), (7.4, LOW)]
    return dict(seconds=8.0, keys=keys, loop=True)


MOVES = {"guard": guard, "slash": slash, "thrust": thrust, "block": block, "duel_a": duel_a, "duel_b": duel_b,
         "thrust_a": thrust_a, "dodge_b": dodge_b, "kneel_sword": kneel_sword, "battle_cry": battle_cry,
         "lookout": lookout, "hero_ready": hero_ready}


def build_moves(folder):
    """Write every move under `folder` (A_<name>) and return {name: asset path}."""
    out = {}
    for name, fn in MOVES.items():
        m = fn()
        out[name] = make_move("%s/A_%s" % (folder, name.title().replace("_", "")), m["seconds"], m["keys"],
                              loop=m.get("loop", False), base=IDLE, ease=m.get("ease", "smooth"))
    log("moves: " + ", ".join(out))
    return out
