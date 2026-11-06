"""KettleHill (cozy): a tea village on a hill at golden hour."""

import math

import unreal

from .core import *  # noqa: F401,F403  (the Level spawner, paint, constants)
from .fx import *  # noqa: F401,F403
from .anim import *  # noqa: F401,F403
from . import shapes as S
from . import moves_cozy as M

# ---------------------------------------------------------------------------------------------
# KettleHill (cozy): a tea village on a hill at golden hour. No anchors: shotpilot picks the
# places from what the level shows (D-44/D-45), so the landmarks carry plain labels. The teaser's
# anchors (teaser/shots.yaml) are `at:` points on the landmarks below; keep them where they are:
#   crop_row (-2090,-1275 | row 1, y -1250)   grove_tree (-1300, 2100)   market_stall (900, -520)
#   old_oak (-400, -1700)   teahouse (0, 950; 900 x 600 footprint)   windmill (1900, -1500)
# ---------------------------------------------------------------------------------------------

KETTLEHILL_MAP = "/Game/KettleHill/Maps/Lvl_KettleHill"

HILLS = ((-4200, 3600, 3000, 900), (3800, 4200, 3600, 1100), (5200, -2600, 3000, 800),
         (-5000, -3400, 3400, 1000), (600, 6200, 4200, 1300))  # x, y, radius, height of each ellipsoid


def hill_z(x, y):
    """The ground height at (x, y): the highest hill surface there (0 on the flat)."""
    best = 0.0
    for (hx, hy, r, h) in HILLS:
        d = math.hypot(x - hx, y - hy)
        if d < r:
            best = max(best, -0.25 * h + h * math.sqrt(1.0 - (d / r) ** 2))
    return best


class Life(Ambient):
    """Ambient with walkers that stand on a surface above z = 0 and use the game's own clips."""

    def walker(self, label, path, clips, mesh=MANNY, z=0.0):
        t0, x0, y0, f0 = path[0]
        a = self.lv.mannequin((x0, y0, z), unreal.Rotator(roll=0.0, pitch=0.0, yaw=f0 - 90.0), mesh, clips[0][2])
        self.lv._done(a, label, "Characters")
        b = self.seq.add_possessable(a)
        sec = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
        sec.set_range(0, self.frames)
        ch = transform_channels(sec)
        lin = unreal.MovieSceneKeyInterpolation.LINEAR
        for (t, x, y, facing) in path:
            for name, value in (("Location.X", x), ("Location.Y", y), ("Location.Z", z), ("Rotation.X", 0.0),
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


def hold_prop(lv, holder, label, mesh, mat, bone, loc, rot):
    """A prop mesh carried by a mannequin's bone, moved by `loc` and turned by `rot` in the bone's frame."""
    a = S.place(lv, label, mesh, (0.0, 0.0, 0.0), 0.0, mat, "Characters")
    comp = a.get_component_by_class(unreal.StaticMeshComponent)
    movable(comp)
    skel = holder.get_component_by_class(unreal.SkeletalMeshComponent)
    snap, world = unreal.AttachmentRule.SNAP_TO_TARGET, unreal.AttachmentRule.KEEP_WORLD
    a.attach_to_component(skel, bone, snap, snap, world, False)
    comp.set_editor_property("relative_location", loc)
    comp.set_editor_property("relative_rotation", rot)
    return a


def build_kettlehill(lv):
    m = "/Game/KettleHill/Materials/"
    F = S.assets_folder(lv)
    paint(lv, m, {
        "grass": dict(base=NOISE + GRAIN + "return lerp(float3(0.16, 0.30, 0.06), float3(0.33, 0.43, 0.11), n) * (0.88 + 0.18 * g);",
                      roughness="return 0.92;"),
        "path": dict(base=NOISE + GRAIN + "return float3(0.36, 0.27, 0.17) * (0.85 + 0.2 * n) * (0.9 + 0.15 * g);",
                     roughness="return 0.95;"),
        "soil": dict(base=NOISE + "return float3(0.13, 0.085, 0.05) * (0.8 + 0.3 * n);", roughness="return 0.98;"),
        "plaster": dict(base=NOISE + "return float3(0.78, 0.70, 0.56) * (0.9 + 0.12 * n);", roughness="return 0.85;"),
        "wood": dict(base="float s = frac(WP.x / 27.0 + WP.y / 27.0); "
                          "return float3(0.34, 0.21, 0.11) * (s < 0.06 ? 0.55 : 0.95 + 0.1 * sin(WP.z * 0.3));",
                     roughness="return 0.75;"),
        "roof": dict(base="float r = frac(WP.z / 18.0); return float3(0.58, 0.24, 0.13) * (r < 0.15 ? 0.6 : 1.0);",
                     roughness="return 0.7;"),
        "copper": dict(base="return float3(0.72, 0.40, 0.20);", roughness="return 0.3;"),
        "iron": dict(base="return float3(0.10, 0.10, 0.11);", roughness="return 0.45;"),
        "ceramic": dict(base="return float3(0.86, 0.84, 0.78);", roughness="return 0.25;"),
        "tin": dict(base="return float3(0.30, 0.38, 0.46);", roughness="return 0.35;"),
        "bamboo": dict(base="return float3(0.62, 0.52, 0.22);", roughness="return 0.6;"),
        "wicker": dict(base="float w = 0.75 + 0.25 * sin(WP.z * 1.6) * sin(WP.x * 1.4 + WP.y * 1.4); "
                            "return float3(0.40, 0.26, 0.11) * w;", roughness="return 0.9;"),
        "straw": dict(base=GRAIN + "return float3(0.72, 0.6, 0.28) * (0.85 + 0.25 * g);", roughness="return 0.95;"),
        "cloth_white": dict(base="return float3(0.88, 0.86, 0.80);", roughness="return 0.95;"),
        "cloth_pink": dict(base="return float3(0.86, 0.52, 0.58);", roughness="return 0.95;"),
        "cloth_blue": dict(base="return float3(0.35, 0.50, 0.70);", roughness="return 0.95;"),
        "line": dict(base="return float3(0.9, 0.88, 0.8);", roughness="return 0.9;"),
        "rope": dict(base="return float3(0.5, 0.4, 0.25);", roughness="return 0.95;"),
        "stone": dict(base=NOISE + GRAIN + "return float3(0.47, 0.45, 0.41) * (0.8 + 0.25 * g);", roughness="return 0.9;"),
        "brick": dict(base="float row = floor(WP.z / 7.5); float col = frac((WP.x + WP.y) / 22.0 + row * 0.5); "
                           "return float3(0.50, 0.26, 0.19) * (col < 0.06 || frac(WP.z / 7.5) < 0.08 ? 0.55 : 1.0);",
                      roughness="return 0.9;"),
        "crop": dict(base=NOISE + "return lerp(float3(0.20, 0.42, 0.08), float3(0.42, 0.52, 0.12), n);", roughness="return 0.8;"),
        "leaf": dict(base=NOISE + GRAIN + "return lerp(float3(0.13, 0.30, 0.07), float3(0.30, 0.42, 0.10), n * g);",
                     roughness="return 0.85;"),
        "blossom": dict(base=NOISE + GRAIN + "return lerp(float3(0.82, 0.50, 0.58), float3(0.95, 0.72, 0.74), n * g);",
                        roughness="return 0.85;"),
        "bark": dict(base="return float3(0.20, 0.13, 0.08);", roughness="return 0.95;"),
        "water": dict(base="return float3(0.015, 0.04, 0.05);", roughness="return 0.04;"),
        "awning_red": dict(base="return float3(0.62, 0.10, 0.08);", roughness="return 0.8;"),
        "awning_white": dict(base="return float3(0.86, 0.83, 0.74);", roughness="return 0.8;"),
        "flower_pink": dict(base="return float3(0.85, 0.35, 0.5);", roughness="return 0.7;"),
        "flower_yellow": dict(base="return float3(0.95, 0.75, 0.15);", roughness="return 0.7;"),
        "lamp": dict(base="return float3(0.02, 0.02, 0.02);", roughness="return 0.4;",
                     emissive="return float3(1.0, 0.68, 0.36) * 9.0;"),
        "window": dict(base="return float3(0.05, 0.04, 0.03);", roughness="return 0.2;",
                       emissive="return float3(1.0, 0.62, 0.30) * 1.3;"),
        "produce": dict(base=GRAIN + "return lerp(float3(0.75, 0.2, 0.08), float3(0.9, 0.6, 0.1), g);", roughness="return 0.6;"),
    })
    lv.box("Ground", (0, 0, -10), (14000, 14000, 20), mat="grass", folder="Geometry")
    for i, (x, y, r, h) in enumerate(HILLS):
        lv.shape("Hill_%d" % (i + 1), (x, y, -h * 0.25), (r * 2, r * 2, h * 2), mesh=SPHERE, mat="grass", folder="Geometry")

    # the path through the village (1 cm proud of the grass), and its branch to the tea house steps
    lv.box("Village_Path", (0, 0, 0.5), (6000, 260, 1.0), mat="path", folder="Geometry")
    lv.box("Teahouse_Path", (0, 275, 0.5), (240, 290, 1.0), mat="path", folder="Geometry")
    for i, (x, y) in enumerate(((-300, 330), (330, 300), (-90, 205), (110, 360))):  # stepping stones by the path
        S.place(lv, "Path_Stone_%d" % (i + 1), S.rock(F, 20 + 4 * i, seed=20 + i, squash=0.45, rough=0.16), (x + 200 * (i % 2 - 0.5), y, 0),
                30.0 * i, "stone", "Geometry")

    # ------------------------------------------------------------------ the tea house (900 x 600, ridge along X)
    # front (south) face y 650, back y 1250, west x -450, east x 450; walls 350, roof pitch 32
    th_x, th_y, th_wall, th_pitch = 0.0, 950.0, 350.0, 32.0
    S.place(lv, "Teahouse_Plinth", S.prism(F, [(-460, 640), (460, 640), (460, 1260), (-460, 1260)], 22.0, "xy"), (0, 0, 0), 0.0,
            "stone", "Teahouse")
    S.place(lv, "Teahouse", S.house_body(F, 900, 600, th_wall, th_pitch), (th_x, th_y, 0.0), 0.0, "plaster", "Teahouse")
    S.place(lv, "Teahouse_Roof", S.roof(F, 900, 600, th_pitch, 14.0, 150.0, gable_overhang=40.0), (th_x, th_y, th_wall), 0.0,
            "roof", "Teahouse")
    # the porch: a deck, steps, and four posts that carry the long front eave
    lv.box("Teahouse_Porch", (0, 575, 15), (940, 150, 30), mat="wood", folder="Teahouse")
    S.place(lv, "Teahouse_Steps", S.stairs(F, 2, 15.0, 40.0, 220), (0, 420, 0), 90.0, "stone", "Teahouse")

    def eave_under(y):  # the underside of the front eave over the porch at world y (z)
        return th_wall - (650.0 - y) * math.tan(math.radians(th_pitch))
    for i, x in enumerate((-440, -200, 200, 440)):
        zt = eave_under(512.0) + 3.0
        lv.box("Porch_Post_%d" % (i + 1), (x, 512, (30 + zt) / 2), (12, 12, zt - 30), mat="wood", folder="Teahouse")
    # door and windows stand proud of the front face (y 650); nothing is coplanar with the wall
    lv.box("Teahouse_Door", (0, 647, 112), (130, 8, 220), mat="wood", folder="Teahouse")
    lv.box("Door_Lintel", (0, 645, 232), (170, 12, 16), mat="wood", folder="Teahouse")
    for sx in (-80, 80):
        lv.box("Door_Jamb", (sx, 645, 120), (14, 10, 236), mat="wood", folder="Teahouse")
    lv.box("Door_Step", (0, 635, 33), (150, 30, 6), mat="stone", folder="Teahouse")
    for x in (-250, 250):
        lv.box("Teahouse_WindowFrame", (x, 648, 175), (170, 4, 130), mat="wood", folder="Teahouse")
        lv.box("Teahouse_Window", (x, 646.5, 175), (150, 3, 110), mat="window", folder="Teahouse")
        lv.box("Window_Mullion", (x, 644.5, 175), (7, 3, 104), mat="wood", folder="Teahouse")
        lv.box("Window_Transom", (x, 643.7, 175), (144, 4, 7), mat="wood", folder="Teahouse")
        lv.box("Window_Sill", (x, 643, 106), (186, 16, 6), mat="wood", folder="Teahouse")
        lv.box("Window_Planter", (x, 639, 41), (150, 22, 22), mat="wood", folder="Teahouse")  # on the deck (z 30)
        for k in range(7):
            lv.shape("Planter_Flower", (x - 66 + k * 22, 639, 52), (18, 18, 16), mesh=SPHERE,
                     mat="flower_pink" if k % 2 else "flower_yellow", folder="Teahouse")
    # lanterns on short chains from the eave
    for i, x in enumerate((-300, 300)):
        zu = eave_under(540.0)
        lv.post("Lantern_Chain", (x, 540, zu - 14.0), 3.0, 15.0, mat="iron", folder="Teahouse")
        lv.shape("Porch_Lantern", (x, 540, zu - 14.0 - 21.0), (34, 34, 42), mesh=SPHERE, mat="lamp", folder="Teahouse")
        lv.point("Porch_Lamp", (x, 540, zu - 35.0), 25.0, (1.0, 0.7, 0.4), radius=700, source=8.0)
    # the west gable end: a window, a rain barrel, a wood pile
    lv.box("Gable_WindowFrame", (-452, 950, 215), (4, 130, 110), mat="wood", folder="Teahouse")
    lv.box("Gable_Window", (-453.5, 950, 215), (3, 112, 92), mat="window", folder="Teahouse")
    lv.box("Gable_Mullion", (-455.5, 950, 215), (3, 7, 86), mat="wood", folder="Teahouse")
    lv.box("Gable_Transom", (-456.3, 950, 215), (4, 106, 7), mat="wood", folder="Teahouse")
    for sx in (-1, 1):  # timber on both gables: a belt, corner posts, a king post to the ridge
        lv.box("Gable_Belt", (sx * 452.5, 950, 350), (5, 600, 12), mat="wood", folder="Teahouse")
        for y in (657, 1243):
            lv.box("Gable_Post", (sx * 453.5, y, 175), (7, 14, 350), mat="wood", folder="Teahouse")
        lv.box("Gable_KingPost", (sx * 453, 950, 350 + 90), (6, 14, 180), mat="wood", folder="Teahouse")
    lv.box("Gable_WindowFrame_E", (452, 950, 215), (4, 130, 110), mat="wood", folder="Teahouse")
    lv.box("Gable_Window_E", (453.5, 950, 215), (3, 112, 92), mat="window", folder="Teahouse")
    S.place(lv, "Rain_Barrel", S.barrel(F, 30, 78, 0.1), (-500, 760, 0), 0.0, "wood", "Teahouse")
    for r, n in enumerate((4, 3, 2)):  # logs, axes along Y, stacked against the wall (the plinth ends at x -460)
        for j in range(n):
            lv.shape("Log", (-476 - r * 8.5 - j * 17, 1060, 7.5 + r * 13.5), (15, 15, 110), mesh=CYLINDER, mat="wood",
                     rot=(0.0, 0.0, 90.0), folder="Teahouse")
    # the chimney through the back slope, and a big kettle on the ridge (the hill's name)
    lv.box("Teahouse_Chimney", (-250, 1085, 470), (64, 64, 190), mat="brick", folder="Teahouse")
    lv.box("Chimney_Cap", (-250, 1085, 571), (80, 80, 12), mat="stone", folder="Teahouse")
    ridge = th_wall + 300.0 * math.tan(math.radians(th_pitch)) + 14.0 / math.cos(math.radians(th_pitch))
    S.place(lv, "Kettle_Sign", S.kettle(F, 80, 120), (0, 950, ridge - 46.0), 0.0, "copper", "Teahouse")
    # the porch table: one cup under the pouring kettle, a pot, a tray
    table_top = 30 + 72.0
    lv.box("Porch_Table", (-40, 541, table_top - 2.5), (130, 70, 5), mat="wood", folder="Teahouse")
    for dx, dy in ((-55, -28), (55, -28), (-55, 28), (55, 28)):
        lv.box("Table_Leg", (-40 + dx, 541 + dy, 30 + (table_top - 5 - 30) / 2), (6, 6, table_top - 5 - 30), mat="wood", folder="Teahouse")
    cup = M.cup_mesh(F)
    for i, (cx, cy) in enumerate(((-40, 523.5), (25, 549), (-8, 562))):
        S.place(lv, "Table_Cup_%d" % (i + 1), cup, (cx, cy, table_top), 20.0 * i, "ceramic", "Teahouse")
    S.place(lv, "Table_Teapot", S.kettle(F, 9, 11), (-80, 548, table_top), 0.0, "ceramic", "Teahouse")
    for i, x in enumerate((-650, 650)):  # benches by the steps
        by = 520.0
        lv.box("Teahouse_Bench_%d" % (i + 1), (x, by, 42), (220, 50, 6), mat="wood", folder="Teahouse")
        for dx in (-90, 90):
            lv.box("Bench_Leg", (x + dx, by, 19.5), (8, 44, 39), mat="wood", folder="Teahouse")
    for i in range(12):  # flower beds along the end walls
        x = (1 if i % 2 else -1) * (500 + (i // 2 % 3) * 26)
        lv.shape("Flower", (x, 670 + (i // 6) * 28 + (i % 3) * 6, 8.5), (18, 18, 18), mesh=SPHERE,
                 mat="flower_pink" if i % 2 else "flower_yellow", folder="Teahouse")

    # ------------------------------------------------------------------ cottages: houses with their own chimney smoke
    def cottage(label, cx, cy, sg):
        """A cottage whose front (door, window) faces +Y when sg is +1, -Y when it is -1."""
        length, span, wall = 520.0, 400.0, 230.0
        S.place(lv, label + "_Plinth", S.prism(F, [(cx - 270, cy - span / 2 - 10), (cx + 270, cy - span / 2 - 10),
                                                    (cx + 270, cy + span / 2 + 10), (cx - 270, cy + span / 2 + 10)], 14.0, "xy"),
                (0, 0, 0), 0.0, "stone", label)
        S.place(lv, label, S.house_body(F, length, span, wall, 36.0), (cx, cy, 0.0), 0.0, "plaster", label)
        S.place(lv, label + "_Roof", S.roof(F, length, span, 36.0, 12.0, 35.0, gable_overhang=30.0), (cx, cy, wall), 0.0, "roof", label)
        fy = cy + sg * span / 2  # the front face
        lv.box(label + "_Door", (cx + 110, fy + sg * 3, 92), (88, 8, 176), mat="wood", folder=label)
        lv.box(label + "_Lintel", (cx + 110, fy + sg * 5, 184), (112, 12, 12), mat="wood", folder=label)
        lv.box(label + "_Step", (cx + 110, fy + sg * 20, 10), (120, 34, 20), mat="stone", folder=label)
        lv.box(label + "_WindowFrame", (cx - 120, fy + sg * 2, 140), (112, 4, 92), mat="wood", folder=label)
        lv.box(label + "_Window", (cx - 120, fy + sg * 3.5, 140), (96, 3, 76), mat="window", folder=label)
        lv.box(label + "_Mullion", (cx - 120, fy + sg * 5.5, 140), (6, 3, 70), mat="wood", folder=label)
        lv.box(label + "_Transom", (cx - 120, fy + sg * 6.3, 140), (90, 4, 6), mat="wood", folder=label)
        lv.box(label + "_Sill", (cx - 120, fy + sg * 7, 90), (128, 14, 6), mat="wood", folder=label)
        lv.box(label + "_Chimney", (cx - 170, cy - sg * 80, 320), (56, 56, 170), mat="brick", folder=label)
        lv.box(label + "_ChimneyCap", (cx - 170, cy - sg * 80, 410), (70, 70, 10), mat="stone", folder=label)
        lv.point(label + "_Glow", (cx - 120, fy + sg * 40, 140), 12.0, (1.0, 0.7, 0.4), radius=400, source=6.0)
        return (cx - 170, cy - sg * 80, 420)

    smoke_a = cottage("Cottage_West", -3000.0, -560.0, 1.0)
    smoke_b = cottage("Cottage_East", 2500.0, 700.0, -1.0)

    # ------------------------------------------------------------------ the vegetable garden: five rows, four aisles
    lv.box("Garden_Soil", (-1650, -950, 1), (1000, 800, 2), mat="soil", folder="Garden")
    for i in range(5):
        y = -1250 + i * 150
        lv.box("Crop_Row_%d" % (i + 1), (-1650, y, 18), (880, 50, 32), mat="crop", folder="Garden")
        if i in (2, 3):  # cabbages and lettuce sitting on the rows
            for k in range(8):
                lv.shape("Cabbage", (-2040 + k * 112, y, 44), (30, 30, 24), mesh=SPHERE,
                         mat="leaf" if i == 2 else "produce", folder="Garden")
        if i == 4:  # bean poles
            for k in range(8):
                lv.post("Bean_Pole", (-2040 + k * 112, y, 32), 3.0, 95.0, mat="wood", folder="Garden")
            lv.box("Bean_Line", (-1650, y, 120), (860, 1.5, 1.5), mat="rope", folder="Garden")
    for i in range(11):
        lv.post("Fence_Post", (-2160 + i * 102, -1360, 0), 10, 100, mat="wood", folder="Garden")
    for z in (75, 40):
        lv.box("Fence_Rail", (-1650, -1360, z), (1020, 6, 10), mat="wood", folder="Garden")
    S.place(lv, "Watering_Barrel", S.barrel(F, 28, 70, 0.1), (-2160, -1180, 0), 0.0, "wood", "Garden")

    # ------------------------------------------------------------------ the windmill: a tapered tower, a cone cap, four sails
    wx, wy = 1900.0, -1500.0
    tower = [(0, 0), (190, 0), (190, 22), (170, 28), (152, 440), (124, 872), (0, 872)]
    S.place(lv, "Windmill", S.lathe(F, "MillTower", tower, 36), (wx, wy, 0), 0.0, "plaster", "Windmill")
    S.place(lv, "Windmill_Cap", S.cone_roof(F, 132, 240, 12.0, 30.0, 36), (wx, wy, 872), 0.0, "roof", "Windmill")
    S.place(lv, "Windmill_Door", S.arch(F, 84, 176, 50.0, 16.0), (wx, wy + 165 + 6, 0), 0.0, "stone", "Windmill")
    lv.box("Windmill_DoorLeaf", (wx, wy + 165 + 3, 86), (84, 8, 172), mat="wood", folder="Windmill")
    lv.box("Windmill_Window", (wx, wy + 136, 520), (40, 6, 70), mat="window", folder="Windmill")
    lv.box("Windmill_WindowFrame", (wx, wy + 134, 520), (52, 4, 82), mat="wood", folder="Windmill")
    hub = (wx, wy + 122.0 + 46.0, 800.0)
    lv.shape("Windmill_Axle", (wx, wy + 122.0 + 14.0, 800), (22, 22, 70), mesh=CYLINDER, mat="wood", rot=(0.0, 0.0, 90.0), folder="Windmill")
    parts = [lv.shape("Windmill_Hub", hub, (56, 56, 56), mesh=SPHERE, mat="wood", folder="Windmill")]
    for i in range(4):  # each sail: one spar and its cloth, a single outline, a hair thicker each so none is coplanar
        outline = [(-6, 14), (6, 14)]  # a spar with seven slats on one side (a lattice sail)
        for k in range(7):
            a = 110.0 + k * 76.0
            outline += [(6, a), (78, a), (78, a + 50.0), (6, a + 50.0)]
        outline += [(6, 690), (-6, 690)]
        sail = S.prism(F, outline, 8.0 + 0.6 * i, "xz")
        parts.append(S.place(lv, "Windmill_Sail_%d" % (i + 1), sail, hub, (i * 90.0, 0.0, 0.0), "awning_white", "Windmill"))
    sails = lv.group("Windmill_Sails", hub, parts, folder="Windmill")

    # ------------------------------------------------------------------ the market stall: counter, awning that falls to the front (+Y)
    sx, sy = 900.0, -520.0
    lv.box("Market_Stall", (sx, sy, 50), (420, 150, 100), mat="wood", folder="Market")
    lv.box("Counter_Top", (sx, sy + 2, 102), (440, 160, 4), mat="wood", folder="Market")
    tan12 = math.tan(math.radians(12.0))
    zc = 254.5  # slab centre height at its middle (y = sy - 40)
    def awning_under(y):
        return zc - (y - (sy - 40.0)) * tan12 - 3.07
    for yy, name in ((sy + 90, "Stall_Pole_Front"), (sy - 90, "Stall_Pole_Back")):
        for dx in (-215, 215):
            top = awning_under(yy) + 2.0
            lv.post(name, (sx + dx, yy, 0), 10, top, mat="wood", folder="Market")
    for i in range(6):
        lv.shape("Stall_Awning_%d" % (i + 1), (sx - 185 + i * 74, sy - 40, zc), (74, 262, 6),
                 mat="awning_red" if i % 2 == 0 else "awning_white", rot=(0.0, 0.0, 12.0), folder="Market")
    for i in range(8):
        lv.shape("Produce", (sx - 140 + (i % 4) * 90, sy + 22 - (i // 4) * 44, 114), (32, 32, 28), mesh=SPHERE, mat="produce", folder="Market")
    S.place(lv, "Stall_Basket", S.prism(F, [(-18, 0), (18, 0), (22, 26), (-22, 26)], 40.0, "xz"), (sx + 120, sy + 92, 102.0), 0.0, "wicker", "Market")
    for i, (cx, cy) in enumerate(((sx - 260, sy + 40), (sx + 260, sy + 50))):  # crates by the stall
        lv.box("Crate_%d" % (i + 1), (cx, cy, 24), (70, 56, 48), mat="wood", folder="Market")
        lv.shape("Crate_Goods", (cx, cy, 52), (56, 44, 20), mesh=SPHERE, mat="produce" if i else "leaf", folder="Market")
    S.place(lv, "Market_Barrel", S.barrel(F, 28, 70, 0.1), (sx - 300, sy - 60, 0), 0.0, "wood", "Market")

    # ------------------------------------------------------------------ the village well: a ring, two posts, a beam, a little roof, a bucket
    wl = (-700.0, -420.0)
    S.place(lv, "Well", S.well_ring(F, 90, 62, 80), (wl[0], wl[1], 0), 0.0, "stone", "Well")
    lv.post("Well_Water", (wl[0], wl[1], 0), 118, 52, mat="water", folder="Well")
    roof_z0 = 218.0
    S.place(lv, "Well_Roof", S.roof(F, 240, 200, 35.0, 8.0, 22.0, gable_overhang=16.0), (wl[0], wl[1], roof_z0), 0.0, "roof", "Well")
    post_top = roof_z0 + (100.0 - 80.0) * math.tan(math.radians(35.0)) + 4.5
    for dy in (-80, 80):
        lv.box("Well_Post", (wl[0], wl[1] + dy, post_top / 2), (12, 12, post_top), mat="wood", folder="Well")
    lv.box("Well_Beam", (wl[0], wl[1], 196), (8, 168, 8), mat="wood", folder="Well")
    lv.post("Well_Rope", (wl[0], wl[1], 124), 2.0, 70.0, mat="rope", folder="Well")
    S.place(lv, "Well_Bucket", S.barrel(F, 16, 26, 0.1), (wl[0], wl[1], 100), 0.0, "wood", "Well")

    # ------------------------------------------------------------------ the pond with a dock
    pc, rx, ry = (-1900.0, 1000.0), 750.0, 500.0
    def ring(grow):
        pts = []
        for k in range(32):
            a = 2 * math.pi * k / 32
            f = 1.0 + 0.07 * math.sin(3 * a + 1.0) + 0.04 * math.sin(5 * a + 0.3)
            pts.append((pc[0] + (rx * f + grow) * math.cos(a), pc[1] + (ry * f + grow) * math.sin(a)))
        return pts
    S.place(lv, "Pond_Shore", S.prism(F, ring(90.0), 0.8, "xy"), (0, 0, 0), 0.0, "path", "Pond")
    S.place(lv, "Pond", S.prism(F, ring(0.0), 1.6, "xy"), (0, 0, 0), 0.0, "water", "Pond")
    lv.box("Dock", (-1500, 760, 24), (160, 560, 14), mat="wood", folder="Pond")
    for dy in (-240, 0, 240):
        for dx in (-70, 70):
            lv.post("Dock_Post", (-1500 + dx, 760 + dy, 0), 14, 18, mat="wood", folder="Pond")
    for i, (x, y, rad, sd) in enumerate(((-1010, 880, 60, 3), (-1180, 1480, 44, 5), (-2650, 1300, 70, 8), (-2480, 450, 40, 9),
                                         (-1150, 640, 34, 12))):
        S.place(lv, "Pond_Rock_%d" % (i + 1), S.rock(F, rad, seed=sd), (x, y, 0), 40.0 * i, "stone", "Pond")

    # ------------------------------------------------------------------ trees: planted on the hill that lifts them
    def tree(label, x, y, h, seed, leaf="leaf", folder="Trees"):
        z0, base = hill_z(x, y), 0.46 * h  # the canopy starts at 46% of the height, taller than a sphere
        S.place(lv, label + "_Trunk", S.trunk(F, h * 0.045, base + 50.0, 0.5, seed=seed), (x, y, z0), 0.0, "bark", folder)
        S.place(lv, label + "_Canopy", S.canopy(F, h * 0.3, seed), (x, y, z0 + base), seed * 37.0 % 360, leaf, folder,
                scale=(1.0, 1.0, 1.6))
    # the old oak the children run around: a leaning trunk, a heavy canopy (canopy centre near z 714)
    S.place(lv, "Old_Oak_Trunk", S.trunk(F, 66, 560, 0.5, lean=24.0, flare=1.0, seed=3), (-400, -1700, 0), 0.0, "bark", "Trees")
    S.place(lv, "Old_Oak", S.canopy(F, 300, 7, lobes=6), (-400 + 22, -1700, 450), 40.0, "leaf", "Trees", scale=(1.0, 1.0, 1.4))
    for k, (dx, dy, dz, r, sd) in enumerate(((-190, 80, 470, 230, 11), (200, -90, 490, 220, 12), (20, 190, 520, 200, 13))):
        S.place(lv, "Old_Oak_Lobe_%d" % (k + 1), S.canopy(F, r, sd, lobes=5), (-400 + dx, -1700 + dy, dz), 25.0 * k, "leaf", "Trees",
                scale=(1.0, 1.0, 1.25))
    for k, (dx, dy, a) in enumerate(((-90, -30, 200), (70, 60, 40))):  # two heavy limbs into the crown
        S.place(lv, "Old_Oak_Limb_%d" % (k + 1), S.trunk(F, 26, 240, 0.7, lean=140.0 * (1 if k else -1), flare=0.0, seed=5 + k),
                (-400 + dx * 0.3, -1700 + dy * 0.3, 380), a, "bark", "Trees")
    for i, (x, y) in enumerate(((-1300, 2100), (-500, 2300), (400, 2200), (1300, 2000), (2200, 1600), (2900, 400))):
        tree("Grove_Tree_%d" % (i + 1), x, y, 650 + (i % 3) * 90, 3 + i * 5, "blossom" if i == 1 else "leaf")
    for i, (x, y, r) in enumerate(((-3400, -900, 120), (-2400, -400, 90), (3300, 300, 130), (2900, 1200, 100), (-1100, -1500, 80),
                                   (1400, -2500, 120), (-2700, 1900, 110))):  # bushes
        S.place(lv, "Bush_%d" % (i + 1), S.canopy(F, r, 40 + i, lobes=4, lumps=0.25), (x, y, hill_z(x, y) - 6), 17.0 * i, "leaf", "Trees")

    # ------------------------------------------------------------------ the laundry line behind the east cottage
    ly = 400.0
    for dx in (-250, 250):
        lv.post("Line_Post", (2500 + dx, ly, 0), 8, 205, mat="wood", folder="Laundry")
    lv.box("Laundry_Line", (2500, ly, 198), (500, 1.4, 1.4), mat="rope", folder="Laundry")
    for i, (mat, dx) in enumerate((("cloth_white", -190), ("cloth_pink", -130), ("cloth_blue", -70), ("cloth_white", 60),
                                    ("cloth_pink", 110), ("cloth_white", 180))):
        h = 62.0 if i % 2 else 78.0
        lv.box("Laundry_%d" % (i + 1), (2500 + dx, ly, 198 - h / 2 - 1), (40, 1.5, h), mat=mat, folder="Laundry")
    S.place(lv, "Laundry_Basket", M.basket_mesh(F), (2500 + 85, ly - 45, 0), 20.0, "wicker", "Laundry")
    S.place(lv, "Laundry_Pile", M.basket_goods_mesh(F), (2500 + 85, ly - 45, 0), 20.0, "cloth_white", "Laundry")

    # ------------------------------------------------------------------ the people
    paths = M.build_moves("/Game/KettleHill/Anims")
    pf = "/Game/KettleHill/Props"

    def person(label, at, facing, move, mesh=MANNY):
        a = lv.figure(label, at, facing, mesh=mesh, anim=paths[move])
        for bone, key, loc, rot in M.held_props(paths[move], move):
            maker, mat = M.PROPS[key]
            hold_prop(lv, a, "%s_%s" % (label, key.capitalize()), maker(pf), mat, bone, loc, rot)
        return a

    # tea keeper pours over the table (comp +Y is world -Y: the cup is 72 cm ahead of the keeper)
    person("Tea_Keeper", (-52.6, 598.0, 30.0), 270.0, "pour_tea", QUINN)
    person("Sweeper", (330.0, 600.0, 30.0), 270.0, "sweep")
    # the benches face the street (-Y): the seat is 10 cm behind the pelvis
    person("Guest_Chat_A", (-690.0, 510.0, 0.0), 270.0, "chat_seated_m", QUINN)
    person("Guest_Chat_B", (-610.0, 510.0, 0.0), 270.0, "chat_seated")
    person("Guest_Sip", (650.0, 510.0, 0.0), 270.0, "sit_sip", QUINN)
    person("Stall_Keeper", (900.0, -655.0, 0.0), 90.0, "wave_hello")
    person("Gardener", (-1760.0, -1175.0, 2.0), 90.0, "water_plants", QUINN)
    person("Fisher", (-1500.0, 1000.0, 31.0), 90.0, "fishing")
    person("Laundress", (2500.0 + 30.0, ly - 30.0, 0.0), 90.0, "hang_laundry", QUINN)
    person("Stretcher", (-2050.0, 400.0, 0.0), 90.0, "stretch")
    person("Greeter", (-3000.0 + 110.0, -560.0 + 200.0 + 100.0, 0.0), 90.0, "wave_hello", QUINN)

    life = Life(lv, "/Game/KettleHill/Cinematics/LS_VillageLife", seconds=12.0)
    carry = paths["carry_basket"]
    life.walker("Farmer", [(0.0, -2050, -1025, 0), (5.4, -1250, -1025, 0), (6.0, -1250, -1025, 180),
                           (11.4, -2050, -1025, 180), (12.0, -2050, -1025, 360)],
                [(0.0, 5.4, carry, 0.7), (5.4, 6.0, "Unarmed/MM_Idle", 1.0),
                 (6.0, 11.4, carry, 0.7), (11.4, 12.0, "Unarmed/MM_Idle", 1.0)], z=2.0)
    ring_pts = [(i * 1.5, -400 + 330 * math.cos(math.radians(i * 45.0)), -1700 + 330 * math.sin(math.radians(i * 45.0)))
                for i in range(9)]  # one lap every 12 s, counter-clockwise
    for k, phase in enumerate((0.0, 6.0)):
        life.walker("Child_%d" % (k + 1), loop_path(ring_pts, 12.0, phase),
                    [(0.0, 12.0, "Unarmed/Jog/MF_Unarmed_Jog_Fwd", 0.85)], mesh=QUINN if k else MANNY, z=hill_z(-400, -1700))
    life.walker("Traveller", [(0.0, 2700, -40, 180), (12.0, 700, -40, 180)], [(0.0, 12.0, carry, 0.8)], z=1.0)
    life.spin(sails, turns=0.5, axis="Rotation.Y")
    life.place()

    # ------------------------------------------------------------------ light: low golden sun, warm haze, lanterns and windows already lit
    lv.sun("Sun", pitch=-24.0, yaw=215.0, lux=10.0, color=(1.0, 0.84, 0.62))
    lv.sky(intensity=1.5)
    lv.fog(0.005, (0.85, 0.70, 0.52), volumetric=True, forward=0.6, extinction=0.5)

    # ------------------------------------------------------------------ atmosphere: Epic's Niagara only (fx.py)
    if hasattr(unreal, "ToolsetRegistry") and unreal.ToolsetRegistry.is_available():
        fxf = "/Game/KettleHill/FX"
        def system(maker, **kw):
            try:
                return maker(fxf, **kw)
            except Exception as e:  # an effect that fails must not fail the level
                warn("%s failed (%s)" % (maker.__name__, e))
                return None

        def put(label, at, sys):
            if sys is not None:
                lv.fx(label, at, sys)
        kettle_steam = system(make_steam, name="NS_KettleSteam", scale=2.0, opacity=0.4)
        cup_steam = system(make_steam, name="NS_CupSteam", scale=0.4, opacity=0.34, seed=33)
        smoke = system(make_smoke, name="NS_Smoke", scale=0.6, wind=(14.0, 4.0), color=(0.62, 0.58, 0.52), opacity=0.4)
        leaves = system(make_leaves, name="NS_Leaves", kind="leaf", area=(800.0, 800.0), height=520.0, count=26)
        petals = system(make_leaves, name="NS_Petals", kind="petal", area=(420.0, 420.0), height=360.0, count=22, seed=53)
        # the kettle on the ridge steams from its spout (+X, about 138 cm out and 70 cm up the body)
        put("Steam_KettleSign", (138.0, 950.0, ridge - 46.0 + 96.0), kettle_steam)
        put("Steam_Cup", (-40.0, 523.5, table_top + 7.5), cup_steam)
        put("Steam_Teapot", (-80.0 + 15.5, 548.0, table_top + 9.0), cup_steam)
        put("Smoke_Teahouse", (-250.0, 1085.0, 585.0), smoke)
        put("Smoke_Cottage_West", (smoke_a[0], smoke_a[1], smoke_a[2] + 5.0), smoke)
        put("Smoke_Cottage_East", (smoke_b[0], smoke_b[1], smoke_b[2] + 5.0), smoke)
        put("Leaves_Old_Oak", (-400.0, -1700.0, 640.0), leaves)
        put("Petals_Blossom_Tree", (-500.0, 2300.0, hill_z(-500, 2300) + 420.0), petals)
        try:
            pollen = make_dust("/Game/KettleHill/FX/NS_Pollen", make_dust_material(m + "M_Pollen"), rate=14.0, size=(1.4, 2.6),
                               color=(1.0, 0.9, 0.62, 0.85), wind=(10.0, 5.0, 4.0), box=(900.0, 900.0, 360.0))
            for i, at in enumerate(((0, 300, 200), (-700, 300, 200), (-1650, -950, 200), (-400, -1700, 200),
                                    (900, -500, 200), (-1500, 760, 200), (1900, -1100, 300))):
                lv.fx("Pollen_%02d" % (i + 1), at, pollen)
        except Exception as e:
            warn("pollen failed (%s)" % e)
    else:
        warn("no Niagara: enable the ToolsetRegistry and NiagaraToolsets plugins in the project")
    lv.exposure(0.9)
    lv.player_start((0, -300, 0), yaw=90.0)
