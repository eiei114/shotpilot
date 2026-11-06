"""Lanternless (horror): a closed-down ward at night."""

import math

import unreal

from .core import *  # noqa: F401,F403  (the Level spawner, paint, constants)
from .fx import *  # noqa: F401,F403
from .anim import *  # noqa: F401,F403
from . import fx as _fx
from . import shapes
from . import moves_horror

# ---------------------------------------------------------------------------------------------
# Lanternless (horror): a closed-down ward at night. A lobby, one long corridor ending in a dead
# end, three side rooms. Moonlight through barred windows, a few weak lamps, most of them dead.
# Units are cm, X runs down the corridor, floor top at z = 0, ceiling at z = 300.
# ---------------------------------------------------------------------------------------------

LANTERNLESS_MAP = "/Game/Lanternless/Maps/Lvl_Ward"
LANTERNLESS_MATS = "/Game/Lanternless/Materials/"

# the failing tube: two short dips every 2.6 s, the first 1.2 s into any shot (each shot starts
# PIE at 0 s). Under one dip pair per 2.6 s, far below three flashes a second.
FLICKER = """float ph = fmod(T + 1.4, 2.6);
float dip = (ph < 0.08 || (ph > 0.17 && ph < 0.25)) ? 1.0 : 0.0;
float level = lerp(1.0, 0.12, dip);"""

INSIDE = "bool inside = WP.x > -810.0 && WP.x < 2610.0 && abs(WP.y) < 660.0;"


def ward_materials(lv):
    """Paint for the ward: dirty two-tone plaster, glossy checker tiles, dark metal, lamps."""
    m = LANTERNLESS_MATS
    lv.mats["wall"] = make_material(m + "M_Ward_Plaster", base="""
float z = WP.z;
float3 c = z < 105.0 ? float3(0.045, 0.075, 0.065) : float3(0.20, 0.205, 0.185);
if (abs(z - 107.0) < 2.5) c = float3(0.02, 0.025, 0.022);
c *= lerp(0.55, 1.0, saturate(z / 45.0));
c *= lerp(1.0, 0.7, saturate((z - 250.0) / 60.0));
float n = 0.5 + 0.25 * sin(WP.x * 0.013 + WP.z * 0.021) + 0.25 * sin(WP.y * 0.017 - WP.z * 0.011 + 1.7);
return c * lerp(0.78, 1.05, n);""", roughness="return 0.85;")
    lv.mats["floor"] = make_material(m + "M_Ward_Tiles", base=INSIDE + """
if (!inside) return float3(0.025, 0.028, 0.022);
float2 cell = floor(WP.xy / 40.0);
float k = fmod(abs(cell.x + cell.y), 2.0);
float3 c = lerp(float3(0.11, 0.115, 0.105), float3(0.035, 0.04, 0.038), k);
float2 f = frac(WP.xy / 40.0);
float grout = (f.x < 0.03 || f.y < 0.03) ? 0.6 : 1.0;
float n = 0.5 + 0.25 * sin(WP.x * 0.011 + 0.7) + 0.25 * sin(WP.y * 0.009 + 2.1);
return c * grout * lerp(0.75, 1.05, n);""", roughness=INSIDE + """
if (!inside) return 0.95;
return lerp(0.22, 0.45, 0.5 + 0.5 * sin(WP.x * 0.007 + WP.y * 0.013));""")
    lv.mats["prop"] = make_material(m + "M_Ward_Metal", base="return float3(0.05, 0.06, 0.058);",
                                    roughness="return 0.5;")
    dark = "return float3(0.02, 0.02, 0.02);"
    lv.mats["lamp_on"] = make_material(m + "M_Ward_LampOn", base=dark, roughness="return 0.4;",
                                       emissive="return float3(0.85, 0.93, 1.0) * 14.0;")
    lv.mats["lamp_warm"] = make_material(m + "M_Ward_LampWarm", base=dark, roughness="return 0.4;",
                                         emissive="return float3(1.0, 0.72, 0.42) * 10.0;")
    lv.mats["lamp_off"] = make_material(m + "M_Ward_LampOff", base="return float3(0.03, 0.03, 0.03);",
                                        roughness="return 0.35;")
    lv.mats["lamp_flicker"] = make_material(m + "M_Ward_LampFlicker", base=dark, roughness="return 0.4;",
                                            emissive=FLICKER + "\nreturn float3(0.8, 0.95, 0.9) * 14.0 * level;")
    lv.mats["exit"] = make_material(m + "M_Ward_Exit", base=dark, roughness="return 0.4;",
                                    emissive="return float3(1.0, 0.04, 0.02) * 12.0;")
    return make_material(m + "LF_Ward_Flicker", emissive=FLICKER + "\nreturn level;", light_function=True)



FX_FOLDER = "/Game/Lanternless/FX"
ANIM_FOLDER = "/Game/Lanternless/Anims"


class WardLife(Ambient):
    """Ambient whose walkers may play an animation made in the project (a /Game path)."""

    def walker(self, label, path, clips, mesh=MANNY):
        t0, x0, y0, f0 = path[0]
        a = self.lv.mannequin((x0, y0, 0.0), unreal.Rotator(roll=0.0, pitch=0.0, yaw=f0 - 90.0), mesh, clips[0][2])
        self.lv._done(a, label, "Characters")
        b = self.seq.add_possessable(a)
        sec = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
        sec.set_range(0, self.frames)
        ch = transform_channels(sec)
        lin = unreal.MovieSceneKeyInterpolation.LINEAR
        for (t, x, y, facing) in path:
            for name, value in (("Location.X", x), ("Location.Y", y), ("Location.Z", 0.0), ("Rotation.X", 0.0),
                                ("Rotation.Y", 0.0), ("Rotation.Z", facing - 90.0)):
                ch[name].add_key(unreal.FrameNumber(self._f(t)), float(value), 0.0,
                                 unreal.MovieSceneTimeUnit.DISPLAY_RATE, lin)
        for name in ("Scale.X", "Scale.Y", "Scale.Z"):
            try:
                ch[name].set_default(1.0)
            except Exception:
                pass
        track = b.add_track(unreal.MovieSceneSkeletalAnimationTrack)
        for (c0, c1, anim, rate) in clips:
            s = track.add_section()
            s.set_range(self._f(c0), self._f(c1))
            p = s.get_editor_property("params")
            p.set_editor_property("animation", load(anim if anim.startswith("/") else ANIMS + "/" + anim))
            set_play_rate(p, rate)
            s.set_editor_property("params", p)
        return a


def along(a, b, u):
    """The point u cm along the wall a -> b."""
    length = math.hypot(b[0] - a[0], b[1] - a[1])
    return a[0] + (b[0] - a[0]) * u / length, a[1] + (b[1] - a[1]) * u / length


def doorway(lv, a, b, u0, u1, top, depth=26.0, jamb=5.0):
    """Door jambs inside an opening of the wall a -> b (the head is the wall above it)."""
    yaw = math.degrees(math.atan2(b[1] - a[1], b[0] - a[0]))
    for u in (u0 + jamb / 2, u1 - jamb / 2):
        x, y = along(a, b, u)
        lv.box("Door_Jamb", (x, y, top / 2), (jamb, depth, top), mat="prop", yaw=yaw, folder="Geometry")


def window(lv, a, b, u0, u1, bottom, top, thick=20.0):
    """Barred window: the bars, a sill that juts into the room and a head rail."""
    lv.bars(a, b, u0, u1, bottom, top)
    yaw = math.degrees(math.atan2(b[1] - a[1], b[0] - a[0]))
    x, y = along(a, b, (u0 + u1) / 2)
    lv.box("Window_Sill", (x, y, bottom + 2), (u1 - u0, thick + 12, 4), mat="prop", yaw=yaw, folder="Geometry")


def dead_tree(lv, label, at, height, seed):
    """A bare tree: a tapered trunk with a few thin boughs reaching out and up."""
    f = shapes.assets_folder(lv)
    r = height * 0.035
    lean = height * 0.05
    body = shapes.trunk(f, r, height, top=0.22, lean=lean, flare=0.6, seed=seed)
    shapes.place(lv, label + "_Trunk", body, at, seed * 53.0 % 360, "prop", "Exterior")
    for i, (frac, yaw, size) in enumerate(((0.45, 20, 0.5), (0.58, 140, 0.45), (0.7, 250, 0.4), (0.82, 60, 0.3))):
        yaw = yaw + seed * 41.0
        bough = shapes.trunk(f, r * 0.5, height * size, top=0.12, lean=height * 0.04, flare=0.0, seed=seed + i)
        shapes.place(lv, "%s_Bough_%d" % (label, i + 1), bough, (at[0], at[1], at[2] + height * frac),
                     (-52.0 + 8 * i, yaw, 0.0), "prop", "Exterior")


def bed(lv, tag, x, y, mirror=1):
    """An iron hospital bed, head at the west end: legs, frame, mattress and a headboard."""
    for dx in (-90, 90):
        for dy in (-40, 40):
            lv.post("Bed_Leg%s" % tag, (x + dx, y + dy, 0), 6, 30, mat="prop", folder="Props")
    lv.box("Bed_Frame%s" % tag, (x, y, 33), (200, 90, 6), mat="prop", folder="Props")
    lv.box("Bed_Mattress%s" % tag, (x, y, 43), (196, 84, 14), mat="accent", folder="Props")
    lv.box("Bed_Head%s" % tag, (x - 105, y, 60), (10, 94, 120), mat="prop", folder="Props")


def build_lanternless(lv):
    flicker = ward_materials(lv)
    moves = moves_horror.build_moves(ANIM_FOLDER)

    H = 300.0
    # ground (inside and out) and one roof over the whole building
    lv.box("Ground", (0, 0, -10), (8000, 6000, 20), mat="floor")
    lv.box("Roof", (900, 0, H + 10), (3440, 1340, 20), mat="wall")

    # Every outer corner is owned by one wall (it runs thick/2 past the corner); the other stops at the
    # owner's inner face, so there is no notch and no pair of coplanar faces.
    # lobby x -800..0, y -650..650: two barred front windows, a locked entrance, one side window
    front_a, front_b = (-800, 650), (-800, -650)
    lv.wall(front_a, front_b, H, holes=[(150, 450, 100, 230), (850, 1150, 100, 230)], ends=(10, 10))
    window(lv, front_a, front_b, 150, 450, 100, 230)
    window(lv, front_a, front_b, 850, 1150, 100, 230)
    lv.box("Entrance_Door_Locked", (-785, 0, 120), (10, 200, 240), mat="accent")
    side_a, side_b = (-800, -650), (0, -650)
    lv.wall(side_a, side_b, H, holes=[(300, 600, 100, 230)], ends=(-10, -10))
    window(lv, side_a, side_b, 300, 600, 100, 230)
    lv.wall((-800, 650), (0, 650), H, ends=(-10, -10))
    # into the corridor: the opening is as wide as the corridor's inner faces (y -140..140)
    lv.wall((0, -650), (0, 650), H, holes=[(510, 790, 0, 260)], ends=(10, 10))

    # corridor x 0..2600, y -150..150, ending in a blank wall
    south_a, south_b = (0, -150), (2600, -150)
    north_a, north_b = (0, 150), (2600, 150)
    lv.wall(south_a, south_b, H, holes=[(1100, 1220, 0, 220)], ends=(-10, 10))  # door to the left room
    lv.wall(north_a, north_b, H, holes=[(600, 720, 0, 220), (1550, 1670, 0, 220)], ends=(-10, 10))
    lv.wall((2600, -150), (2600, 150), H, ends=(-10, -10))
    doorway(lv, south_a, south_b, 1100, 1220, 220)
    for (u0, u1) in ((600, 720), (1550, 1670)):
        doorway(lv, north_a, north_b, u0, u1, 220)

    # side rooms: R1 (dark, a bed), R2 (the fallen patient), L1 (the empty chair)
    for (x0, x1, y_out) in ((400, 1000, 650), (1400, 2000, 650), (900, 1500, -650)):
        y_in = 150 if y_out > 0 else -150
        a, b = (x0, y_out), (x1, y_out)
        lv.wall(a, b, H, holes=[(200, 400, 110, 220)], ends=(10, 10))
        window(lv, a, b, 200, 400, 110, 220)
        lv.wall((x0, y_in), (x0, y_out), H, ends=(0, -10))
        lv.wall((x1, y_in), (x1, y_out), H, ends=(0, -10))

    # pipes along the ceiling (they are where the water comes from)
    lv.shape("Pipe_Lobby", (-400, -120, H - 5), (10, 10, 780), mesh=CYLINDER, mat="prop", rot=(90, 0, 0), folder="Props")
    lv.shape("Pipe_Corridor", (1300, 100, H - 5), (10, 10, 2540), mesh=CYLINDER, mat="prop", rot=(90, 0, 0), folder="Props")

    # lobby furniture (the nurse stands between the desk and the wall)
    lv.box("SP_reception_desk", (-300, 380, 55), (120, 300, 110), mat="prop", folder="Props")
    lv.box("Reception_Counter", (-300, 380, 113), (140, 320, 6), mat="accent", folder="Props")
    lv.box("Reception_Monitor", (-300, 330, 130), (4, 30, 24), mat="prop", folder="Props")
    lv.box("Reception_Monitor_Foot", (-300, 330, 119), (14, 14, 4), mat="prop", folder="Props")
    for i, x in enumerate((-650, -450)):
        for dx in (-70, 70):
            lv.box("Waiting_Bench_Leg", (x + dx, -560, 19), (8, 44, 38), mat="prop", folder="Props")
        lv.box("Waiting_Bench_%d" % (i + 1), (x, -560, 41), (160, 50, 6), mat="prop", folder="Props")
        lv.box("Waiting_Bench_Back_%d" % (i + 1), (x, -583, 69), (160, 6, 50), mat="prop", folder="Props")
    lv.box("Notice_Board", (-12, 350, 160), (4, 160, 100), mat="accent", folder="Props")  # on the lobby's back wall

    # the stretcher left askew in the corridor, and an IV stand beside it
    lv.box("SP_stretcher", (1300, 50, 80), (200, 70, 8), mat="accent", yaw=12.0, folder="Props")
    for (lx, ly) in ((-96, -28), (-96, 28), (96, -28), (96, 28)):
        c, s = math.cos(math.radians(12)), math.sin(math.radians(12))
        lv.post("Stretcher_Leg", (1300 + lx * c - ly * s, 50 + lx * s + ly * c, 0), 4, 76)
    lv.post("IV_Stand_Base", (1190, 105, 0), 36, 3, mat="prop", folder="Props")
    lv.post("IV_Stand_Pole", (1190, 105, 3), 3, 170, mat="prop", folder="Props")
    lv.box("IV_Stand_Arm", (1190, 112, 167), (3, 28, 3), mat="prop", folder="Props")
    lv.box("IV_Bag", (1190, 124, 160), (6, 12, 20), mat="accent", folder="Props")

    # ceiling lamp fittings every 600 cm; the first still lights, the third is failing, two are dead
    for i, (x, mat) in enumerate(((300, "lamp_on"), (900, "lamp_off"), (1500, "lamp_flicker"), (2100, "lamp_off"))):
        lv.box("Ceiling_Lamp_%d" % (i + 1), (x, 0, H - 4), (60, 20, 8), mat=mat, folder="Props")
    # fittings for the other lamps, and the one red exit sign over the door to L1
    lv.box("Ceiling_Lamp_Lobby", (-400, 0, H - 4), (90, 25, 8), mat="lamp_warm", folder="Props")
    lv.box("Ceiling_Lamp_Nurse", (-300, 560, H - 4), (50, 20, 8), mat="lamp_warm", folder="Props")
    lv.box("Ceiling_Lamp_R1", (700, 400, H - 4), (50, 20, 8), mat="lamp_on", folder="Props")
    lv.box("Ceiling_Lamp_L1", (1300, -300, H - 4), (50, 20, 8), mat="lamp_on", folder="Props")
    lv.box("Ceiling_Lamp_DeadEnd", (2540, 0, H - 4), (40, 60, 8), mat="lamp_on", folder="Props")
    lv.box("Exit_Sign", (1160, -138, 245), (36, 4, 14), mat="exit", folder="Props")

    # R1: a bed with someone in it, reaching up from the mattress
    bed(lv, "", 700, 520)
    lv.figure("Bed_Patient", (703, 520, 51), facing=0.0, mesh=QUINN, anim=moves["reach"])
    # R1: one at the west wall swaying, one hiding in the far corner, one leaning out of the doorway
    lv.figure("Swaying_Patient", (440, 560, 0), facing=180.0, mesh=MANNY, anim=moves["sway"])
    lv.figure("Cowering_Patient", (880, 600, 0), facing=90.0, mesh=QUINN, anim=moves["cower"])
    lv.figure("Peeking_Patient", (748, 192, 0), facing=270.0, mesh=MANNY, anim=moves["peek"])
    # R2: the fallen patient, one twitching in the doorway, one dragging itself across the floor, one at the wall
    face_down = pose_anim(ANIM_FOLDER + "/A_Fallen_FaceDown", floor=1)
    lv.box("Fallen_Lamp", (1640, 470, 8), (60, 16, 16), mat="lamp_warm", yaw=30.0, folder="Props")
    lv.lying("SP_fallen_patient", (1600, 340), along=20.0, pose=face_down, face_up=False)
    lv.figure("Twitching_Patient", (1610, 215, 0), facing=270.0, anim=moves["twitch"])
    lv.figure("Standing_Patient", (1950, 612, 0), facing=90.0, mesh=QUINN, anim=moves["head_turn"])
    # L1: one chair facing the window
    lv.box("SP_empty_chair", (1200, -460, 23), (45, 45, 46), mat="prop", folder="Props")
    lv.box("Chair_Back", (1200, -437, 75), (42, 6, 60), mat="prop", folder="Props")
    lv.box("Cabinet", (960, -560, 90), (60, 120, 180), mat="prop", folder="Props")
    # the night nurse beside the reception desk, writing on a clipboard
    nurse = lv.figure("Night_Nurse", (-300, 590, 0), facing=270.0, mesh=QUINN, anim=moves["nurse"])
    lv.hold(nurse, "Clipboard", (22, 2, 30), mat="accent", socket="hand_l", loc=(0.0, 0.0, 0.0))

    # the ones that move (one 12 s loop; every shot starts at 0 s)
    life = WardLife(lv, "/Game/Lanternless/Cinematics/LS_WardLife", seconds=12.0)
    # at the dead end: stands still, then shambles slowly down the corridor toward the lobby
    life.walker("SP_figure", [(0.0, 2450, 0, 180), (1.5, 2450, 0, 180), (12.0, 1550, 0, 180)],
                [(0.0, 1.5, "Unarmed/MM_Idle", 1.0), (1.5, 12.0, moves["shamble"], 1.0)], mesh=QUINN)
    # in R1: paces from wall to wall past the open door, a closed loop
    life.walker("Pacing_Patient",
                [(0.0, 500, 300, 0), (5.0, 920, 300, 0), (6.0, 920, 300, 180), (11.0, 500, 300, 180),
                 (12.0, 500, 300, 360)],
                [(0.0, 5.0, moves["shamble"], 0.9), (5.0, 6.0, "Unarmed/MM_Idle", 1.0),
                 (6.0, 11.0, moves["shamble"], 0.9), (11.0, 12.0, "Unarmed/MM_Idle", 1.0)])
    # in R2: crawls out of the far corner toward the fallen one, very slowly
    life.walker("Crawling_Patient", [(0.0, 1850, 480, 233), (12.0, 1770, 375, 233)],
                [(0.0, 12.0, moves["crawl"], 1.0)], mesh=MANNY)
    life.place()

    # dead trees outside the windows
    for i, (x, y, h) in enumerate(((-1300, 300, 650), (-1550, -250, 800), (-1200, -900, 550),
                                   (700, 1200, 700), (1700, 1350, 600), (1200, -1250, 750))):
        dead_tree(lv, "Dead_Tree_%d" % (i + 1), (x, y, 0), h, i + 1)

    # light: moon through the front and left windows, weak lamps, cold light behind the figure
    lv.sun("Moon", pitch=-25.0, yaw=35.0, lux=2.0, color=(0.55, 0.65, 1.0))
    lv.sky(intensity=1.0)
    lv.fog(0.05, (0.01, 0.012, 0.02), volumetric=True, forward=0.7, extinction=2.0)
    lv.point("Lamp_Lobby", (-400, 0, H - 30), 25.0, (1.0, 0.82, 0.6), radius=1200, scatter=1.5)
    lv.point("Lamp_Corridor_1", (300, 0, H - 20), 12.0, (0.9, 0.95, 1.0), scatter=1.5)
    lv.point("Lamp_Corridor_3", (1500, 0, H - 20), 10.0, (0.85, 1.0, 0.85), scatter=1.5, light_function=flicker)
    lv.point("Lamp_Fallen", (1660, 470, 25), 6.0, (1.0, 0.75, 0.5), radius=600, source=2.0)
    lv.point("Light_DeadEnd", (2560, 0, H - 30), 15.0, (0.7, 0.8, 1.0), radius=700, scatter=2.0)
    lv.point("Light_Exit", (1160, -95, 238), 3.0, (1.0, 0.05, 0.03), radius=450, source=2.0, scatter=2.0)
    # moonlight through each barred window: a narrow beam from outside, seen in the fog
    moon = (0.55, 0.68, 1.0)
    for label, at, toward in (("Moon_Lobby_Front_1", (-1050, 350, 330), (-600, 350, 0)),
                              ("Moon_Lobby_Front_2", (-1050, -350, 330), (-600, -350, 0)),
                              ("Moon_Lobby_Side", (-350, -900, 330), (-350, -450, 0)),
                              ("Moon_R1", (700, 900, 330), (700, 450, 0)),
                              ("Moon_R2", (1700, 900, 330), (1700, 450, 0)),
                              ("Moon_L1", (1200, -900, 330), (1200, -450, 0))):
        lv.spot(label, at, toward, 120.0, moon, cone=18.0, scatter=8.0)
    # a cold light over the yard in front of the lobby: it lights the rain and the mist seen through the windows
    lv.point("Light_Yard", (-980, 0, 400), 90.0, moon, radius=700, source=6.0)
    lv.point("Lamp_R1", (700, 400, H - 20), 5.0, (0.8, 0.95, 0.85), radius=700)
    lv.point("Lamp_L1", (1300, -300, H - 20), 12.0, (0.75, 0.85, 1.0), radius=800)
    lv.point("Lamp_Nurse", (-300, 560, H - 40), 6.0, (1.0, 0.85, 0.65), radius=600)
    # atmosphere (Niagara through NiagaraToolsets): dust in the beams, drips under the pipes, rain seen through
    # the front windows, low mist at the dead end and over the ground outside
    if hasattr(unreal, "ToolsetRegistry") and unreal.ToolsetRegistry.is_available():
        dust = make_dust(FX_FOLDER + "/NS_Ward_Dust", make_dust_material(LANTERNLESS_MATS + "M_Ward_Dust"))
        spots = [("Corridor", (x, 0, 130)) for x in (200, 600, 1000, 1400, 1800, 2200, 2450)]
        spots += [("Lobby", (x, y, 130)) for x in (-600, -200) for y in (-400, 0, 400)]
        spots += [("L1", (1050, -400, 130)), ("L1", (1350, -400, 130)), ("R1", (550, 400, 130)), ("R1", (850, 400, 130)),
                  ("R2", (1550, 400, 130)), ("R2", (1850, 400, 130))]
        for i, (room, at) in enumerate(spots):
            lv.fx("Dust_%s_%02d" % (room, i + 1), at, dust)
        drips = make_drips(FX_FOLDER, "NS_Ward_Drips", drop=H - 10.0)
        lv.fx("Drips_Lobby", (-420, -120, H - 12), drips)
        lv.fx("Drips_Corridor", (1450, 100, H - 12), drips)
        # `line` is a reserved word in HLSL: the drizzle's streak shader fails to compile (and takes the
        # renderer down) until fx.STREAK stops using it
        _fx.STREAK = _fx.STREAK.replace("float line", "float streak").replace("line *", "streak *")
        rain = make_drizzle(FX_FOLDER, "NS_Ward_Rain", area=(260.0, 1000.0), height=330.0, rate=780.0)
        lv.fx("Rain_Front", (-960, 0, 165), rain)
        mist = make_mist(FX_FOLDER, "NS_Ward_Mist_DeadEnd", area=(240.0, 270.0), thickness=45.0, opacity=0.14, count=10)
        lv.fx("Mist_DeadEnd", (2470, 0, 0), mist)
        fog = make_mist(FX_FOLDER, "NS_Ward_Mist_Yard", area=(1400.0, 1500.0), thickness=90.0, opacity=0.2, count=26)
        lv.fx("Mist_Yard_Front", (-1500, 0, 0), fog)
        lv.fx("Mist_Yard_North", (1200, 1300, 0), fog)
    else:
        warn("no atmosphere: enable the ToolsetRegistry and NiagaraToolsets plugins in the project")
    lv.exposure(1.0)  # tune by eye in the editor if the lit spots read too dark or too bright

    lv.player_start((-600, 0, 0), yaw=0.0)
