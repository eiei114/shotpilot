"""Gridhold (strategy): a walled outpost on a gridded plain at noon."""

import math

import unreal

from .core import *  # noqa: F401,F403  (the Level spawner, paint, constants)
from .fx import *  # noqa: F401,F403
from .anim import *  # noqa: F401,F403
from .shapes import *  # noqa: F401,F403
from . import moves_strategy as ms
from . import shapes as sh

# ---------------------------------------------------------------------------------------------
# Gridhold (strategy): a walled outpost on a gridded plain at noon. Four lookout towers, a gatehouse
# with a round arch and open doors, a keep with a terrace and a great banner, a command tent with a
# war table, a supply depot, a forge, an archery lane and a drill yard. A drilled formation marches
# on the spot, a patrol goes out of the gate and back, commanders lean over the map. The red camp of
# the enemy is out on the plain beyond the gate (the teaser looks west to it).
# ---------------------------------------------------------------------------------------------

GRIDHOLD_MAP = "/Game/Gridhold/Maps/Lvl_Outpost"

H = 500.0      # wall height
T = 200.0      # wall thickness
HALF = 1800.0  # the wall centre lines are at +-HALF
TOWERS = ((-HALF, -HALF), (HALF, -HALF), (HALF, HALF), (-HALF, HALF))
GATE_X = -HALF


def _materials(lv, m):
    pave = "float2 p = WP.xy;\n"
    ground = (NOISE + GRAIN + pave +
              "float2 cell = frac(p / 400.0);\n"
              "float3 c = lerp(float3(0.20, 0.31, 0.10), float3(0.30, 0.38, 0.13), n) * (0.9 + 0.15 * g);\n"
              "c = (cell.x < 0.006 || cell.y < 0.006) ? c * 1.3 + 0.03 : c;\n"
              # the road out of the gate: packed earth that fades into the grass
              "float road = saturate((230.0 - abs(p.y)) / 70.0) * step(p.x, -1790.0);\n"
              "c = lerp(c, float3(0.40, 0.33, 0.23) * (0.85 + 0.2 * n) * (0.9 + 0.15 * g), road);\n"
              # inside the walls: dirt
              "float fort = (abs(p.x) < 1795.0 && abs(p.y) < 1795.0) ? 1.0 : 0.0;\n"
              "c = lerp(c, float3(0.38, 0.31, 0.22) * (0.85 + 0.2 * n) * (0.9 + 0.15 * g), fort);\n"
              # the drill yard: lighter sand with a 1 m grid
              "float yard = (p.x > 150.0 && p.x < 1250.0 && p.y > -1530.0 && p.y < -630.0) ? 1.0 : 0.0;\n"
              "float2 yc = frac((p - float2(150.0, -1530.0)) / 100.0);\n"
              "float3 sand = float3(0.55, 0.47, 0.33) * (0.93 + 0.1 * g);\n"
              "sand = (yc.x < 0.018 || yc.y < 0.018) ? sand * 0.74 : sand;\n"
              "return lerp(c, sand, yard);")
    paint(lv, m, {
        "ground": dict(base=ground, roughness="return 0.93;"),
        "wall": dict(base=GRAIN + "float2 b = frac(float2((WP.x + WP.y) / 110.0 + floor(WP.z / 55.0) * 0.5, WP.z / 55.0));\n"
                                  "float mortar = (b.x < 0.04 || b.y < 0.06) ? 0.6 : 1.0;\n"
                                  "return float3(0.50, 0.50, 0.48) * mortar * (0.85 + 0.2 * g);", roughness="return 0.9;"),
        "slate": dict(base="float2 b = frac(float2((WP.x + WP.y) / 40.0, WP.z / 30.0)); "
                           "return float3(0.16, 0.18, 0.22) * (b.y < 0.07 ? 0.7 : 1.0);", roughness="return 0.6;"),
        "wood": dict(base="float s = frac(WP.x / 31.0 + WP.y / 31.0); return float3(0.36, 0.24, 0.13) * (s < 0.06 ? 0.6 : 1.0);",
                     roughness="return 0.8;"),
        "canvas": dict(base=NOISE + "return float3(0.72, 0.66, 0.52) * (0.92 + 0.1 * n);", roughness="return 0.9;"),
        "dark": dict(base="return float3(0.07, 0.055, 0.045);", roughness="return 0.95;"),
        "sack": dict(base=NOISE + GRAIN + "return float3(0.60, 0.50, 0.33) * (0.9 + 0.12 * g) * (0.9 + 0.15 * n);",
                     roughness="return 0.95;"),
        "straw": dict(base=GRAIN + "float s = frac(WP.z / 6.0 + WP.y / 9.0); return float3(0.68, 0.56, 0.27) * (0.85 + 0.2 * g) * (s < 0.15 ? 0.8 : 1.0);",
                      roughness="return 0.95;"),
        "target": dict(base="float d = length(float2(WP.y - 1100.0, WP.z - 130.0)); float d2 = length(float2(WP.y - 1300.0, WP.z - 130.0));\n"
                            "float r = min(d, d2); float ring = floor(r / 11.0);\n"
                            "float3 c = ring < 1.0 ? float3(0.8, 0.1, 0.08) : (frac(ring * 0.5) < 0.25 ? float3(0.85, 0.82, 0.7) : float3(0.12, 0.2, 0.5));\n"
                            "return c;", roughness="return 0.9;"),
        "blue": dict(base="float f = 0.92 + 0.08 * sin(WP.z * 0.05 + WP.y * 0.03); return float3(0.08, 0.18, 0.55) * f;",
                     roughness="return 0.8;"),
        "red": dict(base="return float3(0.60, 0.08, 0.07);", roughness="return 0.8;"),
        "map": dict(base="float2 q = float2(WP.x + 600.0, WP.y - 1000.0) / float2(330.0, 210.0) + 0.5;\n"
                         "float3 c = float3(0.80, 0.72, 0.52) * (0.93 + 0.1 * frac(sin(dot(floor(WP.xy / 3.0), float2(12.9898, 78.233))) * 43758.5));\n"
                         "float land = 0.5 + 0.3 * sin(q.x * 9.0 + 1.0) * sin(q.y * 7.0 + 0.5) + 0.2 * sin(q.x * 21.0 + q.y * 13.0);\n"
                         "c = lerp(c, float3(0.55, 0.62, 0.38), smoothstep(0.55, 0.8, land));\n"
                         "float river = abs(q.y - (0.55 + 0.18 * sin(q.x * 7.0)));\n"
                         "c = lerp(c, float3(0.42, 0.56, 0.72), 1.0 - smoothstep(0.012, 0.03, river));\n"
                         "float2 gr = frac(q * float2(11.0, 7.0));\n"
                         "c *= (gr.x < 0.02 || gr.y < 0.02) ? 0.72 : 1.0;\n"
                         "float edge = min(min(q.x, 1.0 - q.x) * 330.0, min(q.y, 1.0 - q.y) * 210.0);\n"
                         "return edge < 6.0 ? c * 0.4 : c;", roughness="return 0.85;"),
        "steel": dict(base="return float3(0.6, 0.6, 0.62);", roughness="return 0.3;"),
        "iron": dict(base="return float3(0.09, 0.09, 0.1);", roughness="return 0.5;"),
        "pine": dict(base=NOISE + "return float3(0.07, 0.18, 0.08) * (0.85 + 0.25 * n);", roughness="return 0.9;"),
        "bark": dict(base="return float3(0.22, 0.15, 0.09);", roughness="return 0.95;"),
        "rock": dict(base=NOISE + GRAIN + "return float3(0.42, 0.41, 0.38) * (0.85 + 0.25 * n) * (0.92 + 0.1 * g);",
                     roughness="return 0.95;"),
        "fire": dict(base="return float3(0.0, 0.0, 0.0);", roughness="return 1.0;", emissive=FIRE),
    })


# ------------------------------------------------------------------------------ small helpers

def _spaced(lo, hi, w, gap):
    """Centres of merlons of width w along lo..hi, evenly spaced with gaps near `gap`."""
    n = max(1, int((hi - lo - w) // (w + gap)) + 1)
    if n == 1:
        return [(lo + hi) / 2.0]
    return [lo + w / 2.0 + (hi - lo - w) * i / (n - 1) for i in range(n)]


def timed_loop(points, period):
    """A closed walk through `points` [(x, y)] as loop_path wants it, the time spent on each leg in
    proportion to its length. Returns (path keys, speed in cm/s)."""
    pts = list(points) + [points[0]]
    lens = [math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in zip(pts, pts[1:])]
    total = sum(lens)
    t, keyed = 0.0, []
    for (x, y), ln in zip(pts, [0.0] + lens):
        t += period * ln / total
        keyed.append((t, x, y))
    keyed[0] = (0.0, pts[0][0], pts[0][1])
    return keyed, total / period


def build_gridhold(lv):
    m = "/Game/Gridhold/Materials/"
    F = assets_folder(lv)
    FXF = "/Game/Gridhold/FX"
    _materials(lv, m)
    moves = ms.build_moves("/Game/Gridhold/Anims")
    fx_ok = hasattr(unreal, "ToolsetRegistry") and unreal.ToolsetRegistry.is_available()

    def shape(label, mesh, at, rot=0.0, mat="wall", folder="Props", scale=1.0):
        return place(lv, label, mesh, at, rot, mat, folder, scale)

    def figure(label, at, facing, move, mesh=MANNY, rate=1.0):
        return lv.figure(label, at, facing, mesh=mesh, anim=moves[move], rate=rate)

    def ring(label, cx, cy, hx, hy, z, folder, w=60.0, t=40.0, h=70.0, gap=50.0):
        """A parapet of merlons around the top of a rectangle: the corners belong to the long edges."""
        for x in _spaced(cx - hx, cx + hx, w, gap):
            for s in (-1, 1):
                lv.box(label, (x, cy + s * (hy - t / 2), z + h / 2), (w, t, h), mat="wall", folder=folder)
        lo, hi = cy - hy + t, cy + hy - t
        if hi - lo >= w:
            for y in _spaced(lo, hi, w, gap):
                for s in (-1, 1):
                    lv.box(label, (cx + s * (hx - t / 2), y, z + h / 2), (t, w, h), mat="wall", folder=folder)

    # ------------------------------------------------------------------ the plain
    # one ground slab: the dirt of the fort, the yard, the road and the 4 m grid are all paint
    lv.box("Ground", (0, 0, -10), (24000, 24000, 20), mat="ground", folder="Geometry")

    # ------------------------------------------------------------------ walls
    def wall_run(label, a, b, out):
        """A straight wall a -> b (axis aligned) with merlons along its outer edge (towards `out`)."""
        (x0, y0), (x1, y1) = a, b
        length = math.hypot(x1 - x0, y1 - y0)
        ux, uy = (x1 - x0) / length, (y1 - y0) / length
        size = (length, T, H) if y0 == y1 else (T, length, H)
        lv.box(label, ((x0 + x1) / 2, (y0 + y1) / 2, H / 2), size, mat="wall", folder="Walls")
        k = 0
        while 50 + 190 * k + 50 <= length + 1e-6:
            u = 50 + 190 * k
            cx, cy = x0 + ux * u + out[0] * (T / 2 - 25), y0 + uy * u + out[1] * (T / 2 - 25)
            sz = (100.0, 50.0, 70.0) if y0 == y1 else (50.0, 100.0, 70.0)
            lv.box("Merlon", (cx, cy, H + 35), sz, mat="wall", folder="Walls")
            k += 1

    edge = HALF - 210  # the towers are 420 wide: walls run from face to face
    wall_run("Fort_Wall_N", (-edge, HALF), (edge, HALF), (0, 1))
    wall_run("Fort_Wall_S", (-edge, -HALF), (edge, -HALF), (0, -1))
    wall_run("Fort_Wall_E", (HALF, -edge), (HALF, edge), (1, 0))
    wall_run("Fort_Wall_W", (-HALF, -edge), (-HALF, -570.0), (-1, 0))
    wall_run("Fort_Wall_W", (-HALF, 570.0), (-HALF, edge), (-1, 0))

    # ------------------------------------------------------------------ the four towers
    # stone body, a corbelled crown with a parapet, a pavilion roof on four posts, a banner on its tip
    R, RH, RT, RO = 260.0, 240.0, 26.0, 45.0
    cone = cone_roof(F, R, RH, RT, RO, 32)
    z0 = 1280.0  # the roof's wall-top plane
    pr = 12.0    # post radius
    for i, (x, y) in enumerate(TOWERS):
        lv.box("SP_watchtower" if i == 0 else "Watchtower_%d" % (i + 1), (x, y, 450), (420, 420, 900), mat="wall", folder="Towers")
        lv.box("Tower_Crown", (x, y, 920), (480, 480, 40), mat="wall", folder="Towers")
        ring("Tower_Merlon", x, y, 240, 240, 940, "Towers")
        # the roof's underside at the posts' inner edge fixes how tall they are (they end inside the shell)
        d_in = 140 * math.sqrt(2) - pr
        top = z0 + RH * (1.0 - d_in / R) + 1.5
        for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            lv.post("Tower_Post", (x + sx * 140, y + sy * 140, 940), 2 * pr, top - 940, mat="wood", folder="Towers")
        shape("Tower_Roof", cone, (x, y, z0), 0.0, "slate", "Towers")
        apex = z0 + RH
        lv.post("Tower_Flagpole", (x, y, apex + 10), 8, 170, mat="wood", folder="Towers")
        lv.box("Tower_Banner", (x, y + 61, apex + 10 + 125), (4, 120, 70), mat="blue", folder="Towers")

    # ------------------------------------------------------------------ the gatehouse
    # two blocks with a parapet, between them a vaulted passage under the lintel (SP_gate)
    for s in (-1, 1):
        lv.box("Gatehouse", (GATE_X, s * 420, 360), (320, 300, 720), mat="wall", folder="Gate")
        ring("Gatehouse_Merlon", GATE_X, s * 420, 160, 150, 720, "Gate")
    arch_mesh = arch(F, 400.0, 440.0, depth=320.0, pier=70.0, segments=20)
    shape("Gate_Arch", arch_mesh, (GATE_X, 0, 0), 90.0, "wall", "Gate")
    lv.box("SP_gate", (GATE_X, 0, 580), (320, 540, 140), mat="wall", folder="Gate")
    for x in (GATE_X - 140, GATE_X + 140):  # merlons along the two edges of the lintel
        for y in _spaced(-270, 270, 60, 50):
            lv.box("Gate_Merlon", (x, y, 650 + 35), (40, 60, 70), mat="wall", folder="Gate")
    # the doors stand open, swung inwards against the passage's piers
    for s in (-1, 1):
        lv.box("Gate_Door", (GATE_X + 255, s * 212, 200), (190, 16, 400), mat="wood", folder="Gate")
        for z in (80, 320):  # iron straps
            lv.box("Gate_Strap", (GATE_X + 255, s * (212 + 8 + 1.5), z), (190, 3, 22), mat="iron", folder="Gate")

    # ------------------------------------------------------------------ the keep
    lv.box("SP_keep", (500, 0, 600), (1000, 1000, 1200), mat="wall", folder="Keep")
    lv.box("Keep_Course", (500, 0, 600), (1040, 1040, 30), mat="wall", folder="Keep")        # a string course
    lv.box("Keep_Course_Top", (500, 0, 1175), (1040, 1040, 30), mat="wall", folder="Keep")  # ends under the roof line
    ring("Keep_Merlon", 500, 0, 500, 500, 1200, "Keep", w=100.0, t=50.0, h=80.0, gap=60.0)
    tower_top = lathe(F, "KeepTop", [(0.0, 0.0), (283.0, 0.0), (283.0, 440.0), (0.0, 740.0)], 4, 45.0)
    ext = tower_top.get_bounds().box_extent
    log("keep top bounds %.0f x %.0f x %.0f" % (2 * ext.x, 2 * ext.y, 2 * ext.z))
    shape("Keep_Top", tower_top, (700, 0, 1200), 0.0, "wall", "Keep")
    lv.post("Keep_Flagpole", (700, 0, 1925), 10, 330, mat="wood", folder="Keep")
    lv.box("Keep_Banner", (700, 115, 2160), (4, 220, 150), mat="blue", folder="Keep")
    # a door at the top of three steps, arrow slits, hanging banners
    shape("Keep_Steps", stairs(F, 3, 15, 40, 300), (-120, 0, 0), 0.0, "wall", "Keep")
    lv.box("Keep_Door", (-3, 0, 45 + 160), (6, 200, 320), mat="wood", folder="Keep")
    lv.box("Keep_Door_Strap", (-7, 0, 45 + 60), (2, 200, 22), mat="iron", folder="Keep")
    lv.box("Keep_Door_Strap", (-7, 0, 45 + 250), (2, 200, 22), mat="iron", folder="Keep")
    lv.box("Keep_Lintel", (-8, 0, 45 + 340), (16, 300, 40), mat="wall", folder="Keep")
    for s in (-1, 1):
        lv.box("Keep_Jamb", (-8, s * 120, 45 + 160), (16, 40, 320), mat="wall", folder="Keep")
        lv.box("Keep_Hanging", (-2, s * 340, 760), (4, 90, 330), mat="blue", folder="Keep")
        lv.box("Keep_Hanging_Bar", (-4, s * 340, 760 + 175), (8, 110, 10), mat="wood", folder="Keep")
        for z in (350, 900, 1060):
            lv.box("Keep_Slit", (-2, s * 220, z), (4, 24, 110), mat="dark", folder="Keep")
    for x in (180, 500, 820):  # the south face, over the drill yard
        for z in (420, 900):
            lv.box("Keep_Slit", (x, -502, z), (24, 4, 110), mat="dark", folder="Keep")

    # ------------------------------------------------------------------ stairs to the wall walk
    walk_steps = stairs(F, 20, 25, 30, 200)
    shape("Wall_Stairs_S", walk_steps, (-400, -1100, 0), -90.0, "wall", "Walls")
    shape("Wall_Stairs_N", walk_steps, (300, 1100, 0), 90.0, "wall", "Walls")

    # ------------------------------------------------------------------ the command pavilion and the war table
    # the tent stands behind the table so the table is in the sun; its front awning shades only the back
    tent_len, tent_span, tent_wall, tent_pitch = 420.0, 460.0, 200.0, 30.0
    shape("Command_Tent_Body", house_body(F, tent_len, tent_span, tent_wall, tent_pitch), (-600, 1350, 0), 90.0, "canvas",
          "Command")
    shape("Command_Tent_Roof", sh.roof(F, tent_len, tent_span, tent_pitch, 10.0, 30.0, 80.0), (-600, 1350, tent_wall), 90.0, "canvas", "Command")
    lv.box("Tent_Door", (-600, 1350 - tent_len / 2 - 2, 75), (90, 4, 150), mat="dark", folder="Command")
    for x in (-1010, -190):
        lv.post("Banner_Pole", (x, 1130, 0), 8, 360, mat="wood", folder="Command")
        lv.box("Command_Banner", (x + (-0.0), 1130 + 4 + 50 - 3, 300), (4, 100, 100), mat="blue", folder="Command")
    lv.box("SP_war_table", (-600, 1000, 45), (320, 200, 90), mat="wood", folder="Command")
    lv.box("War_Map", (-600, 1000, 92), (330, 210, 4), mat="map", folder="Command")
    pawn = lathe(F, "Pawn", [(0.0, 0.0), (9.0, 0.0), (9.0, 3.0), (5.0, 7.0), (4.0, 19.0), (8.0, 22.0), (8.0, 27.0),
                             (4.0, 31.0), (0.0, 33.0)], 20)
    blues = [(-735, 975), (-705, 975), (-675, 975), (-645, 1010), (-690, 1040), (-740, 1030), (-720, 950), (-660, 940)]
    reds = [(-500, 1040), (-470, 1010), (-520, 990), (-480, 965), (-545, 1060)]
    for (x, y) in blues:
        shape("Map_Pawn", pawn, (x, y, 94), 0.0, "blue", "Command")
    for (x, y) in reds:
        shape("Map_Pawn", pawn, (x, y, 94), 0.0, "red", "Command")
    lv.shape("Map_Scroll", (-590, 1068, 99), (9, 9, 80), mesh=CYLINDER, mat="canvas", rot=(0.0, 0.0, 90.0), folder="Command")
    figure("Commander", (-730, 850, 0), 90.0, "point_map")
    figure("Captain", (-812, 1010, 0), 0.0, "study_map")
    strategist = figure("Strategist", (-388, 1000, 0), 180.0, "place_piece", mesh=QUINN)
    ms.give_piece(lv, strategist, "Strategist_Piece", moves["place_piece"])

    # ------------------------------------------------------------------ the supply depot
    heights = (90.0, 84.0, 96.0, 88.0, 93.0, 86.0)
    for i in range(6):
        x, y = -1000 + (i % 3) * 96, -1100 - (i // 3) * 96
        h = heights[i]
        lv.box("SP_supply" if i == 0 else "Supply_Crate", (x, y, h / 2), (90, 90, h), mat="wood", folder="Depot")
    for (x, y, base, h) in ((-1000, -1100, 90, 78), (-808, -1100, 96, 74), (-904, -1196, 93, 70)):
        lv.box("Supply_Crate_Top", (x, y, base + h / 2), (72, 72, h), mat="wood", yaw=14.0, folder="Depot")
    barrel_mesh = barrel(F, 34.0, 92.0)
    for i, (x, y) in enumerate(((-1290, -1000), (-1190, -980), (-1250, -1090), (-1330, -1130))):
        shape("Barrel", barrel_mesh, (x, y, 0), 0.0, "wood", "Depot")
    sack = lathe(F, "Sack", [(0.0, 0.0), (26.0, 0.0), (32.0, 14.0), (29.0, 38.0), (17.0, 50.0), (9.0, 58.0), (0.0, 56.0)], 14)
    for i, (x, y, yaw) in enumerate(((-1150, -1230, 20), (-1090, -1250, 70), (-1210, -1260, 130), (-1120, -1180, 200))):
        shape("Sack", sack, (x, y, 0), yaw, "sack", "Depot")
    # a cart, its shafts propped on a stick
    cx, cy = -960.0, -900.0
    lv.box("Cart_Bed", (cx, cy, 48), (190, 100, 14), mat="wood", folder="Depot")
    lv.box("Cart_Axle", (cx, cy, 40), (8, 124, 8), mat="iron", folder="Depot")
    for s in (-1, 1):
        lv.shape("Cart_Wheel", (cx, cy + s * 62, 40), (80, 80, 8), mesh=CYLINDER, mat="wood", rot=(0.0, 0.0, 90.0),
                 folder="Depot")
        lv.box("Cart_Side", (cx, cy + s * 46, 67), (190, 8, 24), mat="wood", folder="Depot")
    lv.box("Cart_Shaft", (cx - 140, cy, 48), (90, 8, 8), mat="wood", folder="Depot")
    lv.post("Cart_Prop", (cx - 170, cy, 0), 8, 44, mat="wood", folder="Depot")

    # ------------------------------------------------------------------ the forge
    lv.box("Forge_Hearth", (-1620, -700, 42.5), (140, 170, 85), mat="wall", folder="Forge")
    lv.box("Forge_Chimney", (-1650, -700, 85 + 130), (60, 60, 260), mat="wall", folder="Forge")
    lv.box("Forge_Coals", (-1580, -700, 85 + 3), (70, 100, 6), mat="fire", folder="Forge")
    lv.post("Anvil_Stump", (-1458, -734, 0), 50, 52, mat="wood", folder="Forge")
    lv.box("Anvil", (-1458, -734, 52 + 18), (64, 24, 36), mat="iron", folder="Forge")
    lv.box("Anvil_Horn", (-1458 - 44, -734, 52 + 26), (24, 16, 20), mat="iron", folder="Forge")
    lv.box("Forge_Bench", (-1470, -610, 40), (130, 40, 80), mat="wood", folder="Forge")
    smith = figure("Smith", (-1440, -796, 0), 90.0, "hammer")
    ms.give_hammer(lv, smith, "Smith_Hammer", moves["hammer"])

    # ------------------------------------------------------------------ the drill yard
    # the standard in front of the ranks (the anchor: tall enough to frame them)
    lv.post("SP_formation", (360, -1080, 0), 10, 420, mat="wood", folder="Drill")
    lv.box("Formation_Standard", (360, -1080 + 61, 330), (6, 120, 150), mat="blue", folder="Drill")
    for r, x in enumerate((520, 720, 920, 1120)):
        for c, y in enumerate((-1380, -1180, -980, -780)):
            s = figure("Soldier_%d%d" % (r + 1, c + 1), (x, y, 0), 180.0, "drill_march")
            ms.give_spear(lv, s, "Spear_%d%d" % (r + 1, c + 1), moves["drill_march"], grip=96.0)
    officer = figure("Officer", (330, -1280, 0), 0.0, "salute")

    # ------------------------------------------------------------------ guards, lookouts and the wall walk
    for s, facing in ((-1, 90.0), (1, 270.0)):
        g = figure("Gate_Guard", (GATE_X + 260, s * 340, 0), facing, "sentry_spear")
        ms.give_spear(lv, g, "Gate_Spear", moves["sentry_spear"])
    deck = 940.0
    look = figure("Lookout", (TOWERS[0][0], TOWERS[0][1], deck), 225.0, "lookout_shade")
    figure("Lookout", (TOWERS[2][0], TOWERS[2][1], deck), 45.0, "lookout_shade", mesh=QUINN)
    sig = figure("Signaller", (TOWERS[1][0], TOWERS[1][1], deck), 315.0, "wave_flag")
    ms.give_flag(lv, sig, "Signal_Flag", moves["wave_flag"])
    g = figure("Tower_Sentry", (TOWERS[3][0], TOWERS[3][1], deck), 135.0, "sentry_spear", mesh=QUINN)
    ms.give_spear(lv, g, "Tower_Spear", moves["sentry_spear"])

    def walk_gap(wall_start, k):
        """The middle of the k-th gap between merlons along a wall, from its start."""
        return wall_start + 145.0 + 190.0 * k

    g = figure("Wall_Sentry", (-1785, walk_gap(-edge, 2), H), 180.0, "sentry_spear")
    ms.give_spear(lv, g, "Wall_Spear", moves["sentry_spear"])
    a = figure("Wall_Archer", (-1785, walk_gap(-edge, 4), H), 180.0, "draw_bow", mesh=QUINN)
    ms.give_bow(lv, a, "Wall_Bow", moves["draw_bow"])
    a = figure("Wall_Archer", (walk_gap(-edge, 8), -1785, H), 270.0, "draw_bow")
    ms.give_bow(lv, a, "Wall_Bow", moves["draw_bow"])
    g = figure("Wall_Sentry", (walk_gap(-edge, 3), -1785, H), 270.0, "sentry_spear", mesh=QUINN)
    ms.give_spear(lv, g, "Wall_Spear", moves["sentry_spear"])
    g = figure("Wall_Sentry", (walk_gap(-edge, 12), 1785, H), 90.0, "sentry_spear")
    ms.give_spear(lv, g, "Wall_Spear", moves["sentry_spear"])

    # ------------------------------------------------------------------ the archery lane
    for y in (1100, 1300):
        lv.box("Straw_Butt", (1640, y, 130), (30, 130, 130), mat="straw", folder="Archery")
        for s in (-1, 1):
            lv.post("Butt_Post", (1640, y + s * 45, 0), 12, 130, mat="wood", folder="Archery")
        lv.box("Target_Face", (1624, y, 130), (4, 110, 110), mat="target", folder="Archery")
    for (x, y, yaw) in ((860, 1010, 5), (860, 1394, -8), (1010, 1230, 40)):
        lv.box("Hay_Bale", (x, y, 25), (110, 55, 50), mat="straw", yaw=float(yaw), folder="Archery")
    for y, mesh in ((1100, MANNY), (1300, QUINN)):
        a = figure("Range_Archer", (1010, y, 0), 0.0, "draw_bow", mesh=mesh)
        ms.give_bow(lv, a, "Range_Bow", moves["draw_bow"])

    # ------------------------------------------------------------------ the well
    shape("Well_Ring", well_ring(F, 75.0, 52.0, 70.0), (-80, 760, 0), 0.0, "wall", "Courtyard")
    for s in (-1, 1):
        lv.post("Well_Post", (-80 + s * 92, 760, 0), 16, 190, mat="wood", folder="Courtyard")
    lv.box("Well_Beam", (-80, 760, 190), (200, 12, 12), mat="wood", folder="Courtyard")
    lv.box("Well_Rope", (-80, 760, 142), (2, 2, 100), mat="dark", folder="Courtyard")
    lv.post("Well_Bucket", (-80, 760, 90), 24, 22, mat="wood", folder="Courtyard")

    # ------------------------------------------------------------------ workers and the patrol
    life = Ambient(lv, "/Game/Gridhold/Cinematics/LS_OutpostLife", seconds=12.0)
    carry = [(-760.0, -880.0), (-150.0, -880.0), (-150.0, -330.0), (-760.0, -330.0)]
    path, speed = timed_loop(carry, 12.0)
    porter = ms.walker(life, "Porter", loop_path(path, 12.0, 0.0), [(0.0, 12.0, moves["carry_crate"], speed / 300.0)])
    ms.give_crate(lv, porter, "Porter_Crate", moves["carry_crate"])
    route = [(-1100.0, -130.0), (-2300.0, -130.0), (-2300.0, 130.0), (-1100.0, 130.0)]
    path, speed = timed_loop(route, 12.0)
    for k in range(4):
        p = ms.walker(life, "Patrol_%d" % (k + 1), loop_path(path, 12.0, k * 1.15), [(0.0, 12.0, moves["march_spear"], speed / 300.0)],
                      mesh=QUINN if k % 2 else MANNY)
        ms.give_spear(lv, p, "Patrol_Spear_%d" % (k + 1), moves["march_spear"], grip=96.0)
    life.place()

    # ------------------------------------------------------------------ outside: the enemy camp, pines, rocks
    camp = (-4650.0, 950.0)
    tent_mesh_body = house_body(F, 380.0, 330.0, 40.0, 50.0)
    tent_mesh_roof = sh.roof(F, 380.0, 330.0, 50.0, 6.0, 14.0)
    for i, a in enumerate((20.0, 110.0, 200.0, 290.0)):
        x, y = camp[0] + 520 * math.cos(math.radians(a)), camp[1] + 520 * math.sin(math.radians(a))
        shape("Enemy_Tent_Body", tent_mesh_body, (x, y, 0), a + 90.0, "red", "Enemy")
        shape("Enemy_Tent_Roof", tent_mesh_roof, (x, y, 40), a + 90.0, "red", "Enemy")
    for a in (65.0, 155.0, 245.0):
        bx, by = camp[0] + 800 * math.cos(math.radians(a)), camp[1] + 800 * math.sin(math.radians(a))
        lv.post("Enemy_Banner_Pole", (bx, by, 0), 8, 420, mat="wood", folder="Enemy")
        lv.box("Enemy_Banner", (bx, by + 4 + 56, 330), (4, 120, 170), mat="red", folder="Enemy")
    stone = sh.rock(F, 18.0, 3, 0.6, 0.2)
    for i in range(9):  # stones round the fire
        a = i * 40.0
        shape("Fire_Stone", stone, (camp[0] + 70 * math.cos(math.radians(a)), camp[1] + 70 * math.sin(math.radians(a)), 0),
              a * 3.0, "rock", "Enemy")
    for i, (a, yaw) in enumerate(((0, 20), (130, 70), (250, 150))):
        d = 14.0 + 2.0 * i  # different thicknesses: no two tops share a plane
        lv.box("Fire_Log", (camp[0] + 15 * math.cos(math.radians(a)), camp[1] + 15 * math.sin(math.radians(a)), d / 2), (90, d, d),
               mat="bark", yaw=float(yaw), folder="Enemy")
    for i, (r, a, h) in enumerate(((400, 150, 70.0), (440, 160, 62.0))):
        lv.box("Enemy_Crate", (camp[0] + r * math.cos(math.radians(a)), camp[1] + r * math.sin(math.radians(a)), h / 2),
               (70, 70, h), mat="wood", yaw=10.0 * i, folder="Enemy")
    # hedgehogs on the road, a line the gate looks out at: three logs crossing at the middle
    for i, y in enumerate((-640, -330, 330, 640)):
        for k in range(3):
            lv.shape("Hedgehog", (-2900, y, 70), (230, 14, 14), mesh=CUBE, mat="wood",
                     rot=(35.0, 60.0 * k + i * 11.0, 0.0), folder="Outside")

    pine_mesh = lathe(F, "Pine", [(0.0, 0.0), (150.0, 0.0), (60.0, 240.0), (115.0, 240.0), (43.4, 464.0), (80.0, 464.0),
                                  (0.0, 800.0)], 16)
    boulders = [sh.rock(F, 90.0, s) for s in (1, 2, 3)]
    keep_out = lambda x, y: ((x < -2400 and abs(y) < 900) or (-5900 < x < -3700 and 200 < y < 2300)
                             or (abs(x) < 2300 and abs(y) < 2300))
    pines = [(-3500.0, -2100.0, 1.55)]
    for i in range(30):
        a = i * 2.39996
        rr = 3000 + (i * 263) % 3400
        x, y = rr * math.cos(a), rr * math.sin(a)
        if not keep_out(x, y):
            pines.append((x, y, 0.8 + ((i * 37) % 60) / 100.0))
    for i, (x, y, s) in enumerate(pines):
        lv.post("Pine_Trunk", (x, y, 0), 38 * s, 250 * s, mat="bark", folder="Outside")
        shape("Pine", pine_mesh, (x, y, 200 * s), i * 47.0, "pine", "Outside", scale=s)
    for i in range(12):
        a = 1.1 + i * 1.9
        rr = 2700 + (i * 421) % 3000
        x, y = rr * math.cos(a), rr * math.sin(a)
        if not keep_out(x, y):
            shape("Rock", boulders[i % 3], (x, y, 0), i * 61.0, "rock", "Outside", scale=0.6 + (i % 4) * 0.3)

    # ------------------------------------------------------------------ the air: fire, smoke, sparks, dust
    if fx_ok:
        try:
            lf = brazier_light_function(m + "LF_Fire", 1.3)
            fire = make_fire(FXF, "NS_Forge", size=0.7, power=1.0)
            lv.fx("Forge_Fire", (-1580, -700, 88), fire)
            lv.point("Forge_Light", (-1400, -700, 170), 220.0, (1.0, 0.52, 0.22), radius=600, source=14.0, light_function=lf)
            lv.fx("Forge_Smoke", (-1650, -700, 85 + 262), make_smoke(FXF, "NS_ForgeSmoke", scale=0.7, wind=(26.0, 8.0)))
            lv.fx("Anvil_Sparks", (-1458, -734, 92), make_sparks(FXF, "NS_AnvilSparks", interval=2.4, count=16, speed=(120.0, 320.0),
                                                                   angle=45.0))
            lv.fx("Camp_Fire", (camp[0], camp[1], 12), make_fire(FXF, "NS_Campfire", size="campfire"))
            lv.point("Camp_Light", (camp[0], camp[1], 130), 1200.0, (1.0, 0.5, 0.2), radius=900, source=20.0, light_function=lf)
            lv.fx("Camp_Smoke", (camp[0], camp[1], 120), make_smoke(FXF, "NS_CampSmoke", scale=1.1, wind=(40.0, -10.0)))
            dust = make_dust(FXF + "/NS_YardDust", make_dust_material(m + "M_YardDust"), rate=14.0, size=(1.6, 3.0),
                             color=(0.9, 0.82, 0.62, 0.7), wind=(14.0, 5.0, 1.0), box=(1100.0, 900.0, 230.0))
            lv.fx("Yard_Dust", (700, -1080, 120), dust)
        except Exception as e:  # cosmetic: the level stands without them
            warn("effects not made (%s)" % e)
    else:
        warn("no effects: enable the ToolsetRegistry and NiagaraToolsets plugins in the project")

    # ------------------------------------------------------------------ light: high clear noon, a little haze for depth
    lv.sun("Sun", pitch=-50.0, yaw=35.0, lux=10.0, color=(1.0, 0.97, 0.92))
    lv.sky(intensity=1.4)
    lv.fog(0.006, (0.55, 0.65, 0.85))
    lv.exposure(1.8)
    lv.player_start((-2600, 0, 0), yaw=0.0)
