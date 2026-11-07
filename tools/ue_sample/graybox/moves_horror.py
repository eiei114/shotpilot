"""Moves for the ward (horror): what the patients and the nurse do while a teaser looks at them.

build_moves(folder) writes the AnimSequences (anim.make_move) under `folder` and returns
{name: asset path}. Every move loops, because a mannequin plays one animation forever; the ones that
tell a small story (a head that turns, a lean out of a doorway) hold still first, since a teaser
shot starts the level at 0 s and sees only its first few seconds.

Frame of every pose: the standing mannequin's component space, +Y the way it faces, +X its left.
"""

from .anim import IDLE, make_move
from .core import ANIMS, EAL

WALK = ANIMS + "/Unarmed/Walk/MF_Unarmed_Walk_Fwd"
IDLE_SECONDS = 227 / 30.0  # the template idle's length: a "stay" move that keeps its breathing


def _sway():
    """Standing close to a wall, head down, rocking slowly from foot to foot."""
    def pose(s):
        return {"pelvis": (7 * s, 0, 0), "body": (3, 0, 5 * s), "spine": (7, 0, -4 * s),
                "head": (24, 4 * s, 9 * s),
                "arm_l": {"hand": (22 + 3 * s, 8, 92), "pole": (0.4, -0.6, -0.4)},
                "arm_r": {"hand": (-22 + 3 * s, 8, 92), "pole": (-0.4, -0.6, -0.4)}}
    return dict(seconds=5.0, loop=True, base=IDLE, ease="cr",
                keys=[(0.0, pose(1)), (1.25, pose(0)), (2.5, pose(-1)), (3.75, pose(0))])


def _head_turn():
    """Stands with its back to the room, then, slowly, the head comes around over the shoulder."""
    away = {"head": (6, 0, 0), "spine": (4, 0, 0)}
    around = {"head": (-4, 78, 10), "spine": (4, 38, 0), "body": (0, 14, 0),
              "arm_l": {"hand": (26, 10, 92), "pole": (0.5, -0.6, -0.4)},
              "arm_r": {"hand": (-26, 4, 90), "pole": (-0.5, -0.6, -0.4)}}
    return dict(seconds=12.0, loop=True, base=IDLE,
                keys=[(0.0, away), (1.2, away), (4.6, around), (9.5, around)])


def _crawl():
    """Dragging itself along the floor on hands and knees, one diagonal pair at a time (slow)."""
    def p(hl_y, hl_z, hr_y, hr_z, kl_y, kl_z, kr_y, kr_z):
        return {"pelvis": (0, 0, -42), "body": (82, 0, 0), "look": (0, 1, 0.15), "head": (-10, 0, 0),
                "arm_l": {"hand": (17, hl_y, hl_z), "hand_dir": (0, 1, 0), "pole": (0.5, -0.3, -0.2)},
                "arm_r": {"hand": (-17, hr_y, hr_z), "hand_dir": (0, 1, 0), "pole": (-0.5, -0.3, -0.2)},
                "leg_l": {"foot": (10, kl_y, kl_z), "foot_dir": (0, -0.85, -0.5), "pole": (0, 0, -1)},
                "leg_r": {"foot": (-10, kr_y, kr_z), "foot_dir": (0, -0.85, -0.5), "pole": (0, 0, -1)}}
    return dict(seconds=2.4, loop=True, base=None, ease="cr",
                keys=[(0.0, p(58, 6, 40, 6, -30, 9, -38, 9)),
                      (0.6, p(66, 22, 46, 6, -30, 9, -30, 22)),
                      (1.2, p(40, 6, 58, 6, -38, 9, -30, 9)),
                      (1.8, p(46, 6, 66, 22, -30, 22, -30, 9))])


def _twitch():
    """Standing, then sudden jerks at uneven times: the head snaps, a shoulder and an arm flinch."""
    rest = {"head": (10, 0, 0), "spine": (3, 0, 0)}
    a = {"head": (4, -28, 16), "spine": (6, -10, 6), "shoulder_r": (16, 4),
         "arm_r": {"hand": (-30, 22, 112), "pole": (-0.6, -0.4, 0.3)}}
    b = {"head": (22, 18, -14), "body": (4, 6, -5), "shoulder_l": (14, 0),
         "arm_l": {"hand": (34, 14, 100), "pole": (0.6, -0.3, -0.3)}}
    return dict(seconds=4.0, loop=True, base=IDLE, ease="linear",
                keys=[(0.0, rest), (0.8, rest), (0.88, a), (1.1, a), (1.2, rest), (2.5, rest), (2.56, b),
                      (2.64, rest), (2.72, b), (2.95, rest)])


def _cower():
    """On the floor in a corner, knees drawn up, arms round the shins, rocking and shaking a little."""
    def pose(s):
        return {"pelvis": (0, -8, -81), "spine": (22 + 7 * s, 0, 0), "head": (24 + 4 * s, 0, 0),
                "leg_l": {"foot": (11, 22, 8), "pole": (0, 0.2, 1)},
                "leg_r": {"foot": (-11, 22, 8), "pole": (0, 0.2, 1)},
                "arm_l": {"hand": (12, 34, 40), "pole": (0.7, 0.1, 0.2)},
                "arm_r": {"hand": (-12, 34, 40), "pole": (-0.7, 0.1, 0.2)}}
    return dict(seconds=3.0, loop=True, base=None, ease="cr",
                keys=[(0.0, pose(0)), (0.75, pose(1)), (1.5, pose(0)), (2.25, pose(-1))])


def _peek():
    """Behind a door frame: lean out with a hand on the frame, hold, ease back out of sight."""
    hide = {"head": (0, -25, 0)}
    out = {"body": (0, 0, 22), "pelvis": (14, 0, 0), "spine": (0, 0, 8), "head": (6, 24, -24),
           "arm_l": {"hand": (36, 14, 118), "pole": (0.6, -0.4, -0.3), "hand_dir": (0, 0, 1)},
           "shoulder_l": (10, 6)}
    return dict(seconds=9.0, loop=True, base=IDLE,
                keys=[(0.0, hide), (1.4, hide), (3.4, out), (6.6, out), (8.4, hide)])


def _reach():
    """Flat on its back, one arm slowly reaching up from the bed, fingers pointing at the ceiling."""
    def pose(a, b):
        return {"pelvis": (0, 0, -81), "body": (-90, 0, 0), "head": (0, 22, 0),
                "leg_l": {"foot": (10, 82, 10), "foot_dir": (0, 0.4, 1), "pole": (0, 0, 1)},
                "leg_r": {"foot": (-10, 84, 10), "foot_dir": (0, 0.4, 1), "pole": (0, 0, 1)},
                "arm_l": {"hand": (16, 20, 11), "pole": (0.5, 0, 1)},
                "arm_r": {"hand": (-22 + a, -30 + 4 * a, 12 + 64 + b), "hand_dir": (0, 0, 1),
                          "pole": (-0.6, 0.2, 0.2)}}
    return dict(seconds=6.0, loop=True, base=None, ease="cr",
                keys=[(0.0, pose(-4, -10)), (1.5, pose(3, 0)), (3.0, pose(7, 8)), (4.5, pose(3, 0))])


def _shamble():
    """A stiff, slow walk: stooped, head hung to one side, one arm dragging, the other held up."""
    seconds = float(EAL.load_asset(WALK).get_editor_property("sequence_length")) / 0.62
    pose = {"body": (14, 0, 0), "spine": (10, 6, 3), "head": (26, 8, 20),
            "arm_l": {"hand": (22, 6, 86), "pole": (0.5, -0.5, -0.3)},
            "arm_r": {"hand": (-26, 34, 120), "pole": (-0.5, -0.3, 0.2)}}
    return dict(seconds=seconds, loop=True, base=WALK, base_rate=0.62, keys=[(0.0, pose)])


def _nurse():
    """A nurse, still, writing on a clipboard held against the chest; now and then the head lifts."""
    note = {"head": (26, 0, 0), "spine": (4, 0, 0),
            "arm_l": {"hand": (10, 32, 112), "pole": (0.7, -0.2, -0.4), "hand_dir": (0, 1, -0.1)},
            "arm_r": {"hand": (-14, 26, 124), "pole": (-0.7, -0.4, -0.2), "hand_dir": (0, 1, -0.2)},
            "shoulder_l": (4, 0), "shoulder_r": (4, 0)}
    up = dict(note, head=(2, -10, 0))
    return dict(seconds=IDLE_SECONDS, loop=True, base=IDLE,
                keys=[(0.0, note), (2.4, note), (3.1, up), (4.7, up), (5.4, note), (IDLE_SECONDS - 0.3, note)])


MOVES = {"sway": _sway, "head_turn": _head_turn, "crawl": _crawl, "twitch": _twitch, "cower": _cower,
         "peek": _peek, "reach": _reach, "shamble": _shamble, "nurse": _nurse}


def build_moves(folder, only=None):
    """Write the moves under `folder` (a /Game path); returns {name: asset path}."""
    out = {}
    for name, make in MOVES.items():
        if only and name not in only:
            continue
        out[name] = make_move("%s/A_%s" % (folder, name.title().replace("_", "")), **make())
    return out
