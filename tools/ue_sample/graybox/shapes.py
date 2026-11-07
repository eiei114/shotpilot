"""Blockout shapes made with Epic's GeometryScripting: closed solids with right normals, built as
StaticMesh assets and placed like any other prop.

Every mesh is built in this file as plain triangles (own builder: welded vertices, one closed solid
at a time, winding checked), then handed to GeometryScript to compute the split normals (hard edges
above an opening angle, smooth below it) and to write the StaticMesh. Collision is complex-as-simple
(the triangle mesh itself answers both visibility traces and the player's feet), so a gateway stays
open and stairs stay steps. The paint is world-position HLSL, so no UVs are needed (a planar set is
written only so the asset has one, along a direction that keeps tangents valid).

Local frame of every mesh (cm): origin on the ground at the middle of the footprint, +X forward,
+Y right, +Z up. Roofs have their origin on the wall-top plane.

Assets are named `SM_<Kind>_<hash of the parameters>` in the folder the caller gives, and reused when
they already exist: the same call twice makes one asset. Change SHAPES_VERSION when a builder changes
so old assets are not reused.
"""

import hashlib
import json
import math

import unreal

from .core import EAL, load, log, setp, warn

SHAPES_VERSION = 2
EPS = 1e-4

__all__ = [
    "assets_folder", "place", "prism", "gable_outline", "gable_wall", "house_body", "roof", "cone_roof",
    "lathe", "column", "barrel", "well_ring", "kettle", "urn", "arch", "stairs", "ramp", "rock", "canopy",
    "trunk", "house", "tree", "purge", "Mesh",
]


# --------------------------------------------------------------------------- vector helpers

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


def _len(a):
    return math.sqrt(_dot(a, a))


def _unit(a):
    n = _len(a)
    return (a[0] / n, a[1] / n, a[2] / n) if n > 1e-12 else (0.0, 0.0, 1.0)


# --------------------------------------------------------------------------- the triangle builder

def _area2(poly):
    """Twice the signed area of a 2D polygon (positive = counter clockwise)."""
    return sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
               for i in range(len(poly)))


def _clean_outline(pts):
    """A 2D outline, counter clockwise, without repeated or collinear points (collinear points would
    become T-junctions between a cap and its sides)."""
    pts = [tuple(map(float, p)) for p in pts]
    out = []
    for p in pts:
        if not out or math.hypot(p[0] - out[-1][0], p[1] - out[-1][1]) > 1e-6:
            out.append(p)
    if len(out) > 1 and math.hypot(out[0][0] - out[-1][0], out[0][1] - out[-1][1]) <= 1e-6:
        out.pop()
    if _area2(out) < 0:
        out.reverse()
    changed = True
    while changed and len(out) > 3:
        changed = False
        for i in range(len(out)):
            a, b, c = out[i - 1], out[i], out[(i + 1) % len(out)]
            cr = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
            if abs(cr) < 1e-7 * max(1.0, math.hypot(c[0] - a[0], c[1] - a[1]) ** 2):
                del out[i]
                changed = True
                break
    if len(out) < 3 or _area2(out) <= 1e-9:
        raise ValueError("outline has no area")
    return out


def _ear_clip(poly):
    """Triangles (index triples, counter clockwise) of a simple 2D polygon given counter clockwise."""
    idx = list(range(len(poly)))
    tris = []

    def inside(p, a, b, c):
        d1 = (p[0] - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (p[1] - b[1])
        d2 = (p[0] - c[0]) * (b[1] - c[1]) - (b[0] - c[0]) * (p[1] - c[1])
        d3 = (p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1])
        return not ((d1 < -1e-9 or d2 < -1e-9 or d3 < -1e-9) and (d1 > 1e-9 or d2 > 1e-9 or d3 > 1e-9))

    guard = 0
    while len(idx) > 3 and guard < 10000:
        guard += 1
        for k in range(len(idx)):
            i0, i1, i2 = idx[k - 1], idx[k], idx[(k + 1) % len(idx)]
            a, b, c = poly[i0], poly[i1], poly[i2]
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) <= 1e-9:
                continue  # reflex or flat corner: not an ear
            if any(inside(poly[j], a, b, c) for j in idx if j not in (i0, i1, i2)
                   and poly[j] not in (a, b, c)):
                continue
            tris.append((i0, i1, i2))
            del idx[k]
            break
        else:
            raise ValueError("outline cannot be triangulated (does it cross itself?)")
    tris.append(tuple(idx))
    return tris


class Mesh:
    """Triangles of closed solids. Each solid is welded on its own (`begin`/`end`), checked to be
    closed and consistently wound, and turned the right way round for Unreal (front faces)"""

    def __init__(self):
        self.v, self.t = [], []
        self._key, self._solid, self._start = {}, 0, 0

    # -- construction
    def begin(self):
        self._solid += 1
        self._start = len(self.t)

    def vert(self, p):
        key = (self._solid, round(p[0], 3), round(p[1], 3), round(p[2], 3))
        i = self._key.get(key)
        if i is None:
            i = self._key[key] = len(self.v)
            self.v.append((float(p[0]), float(p[1]), float(p[2])))
        return i

    def tri(self, a, b, c):
        ia, ib, ic = self.vert(a), self.vert(b), self.vert(c)
        if ia == ib or ib == ic or ia == ic:
            return
        if _len(_cross(_sub(self.v[ib], self.v[ia]), _sub(self.v[ic], self.v[ia]))) < 1e-6:
            return  # a sliver: it would only add a crack
        self.t.append((ia, ib, ic))

    def quad(self, a, b, c, d):
        self.tri(a, b, c)
        self.tri(a, c, d)

    def end(self, what="solid"):
        tris = self.t[self._start:]
        edges = {}
        for (a, b, c) in tris:
            for e in ((a, b), (b, c), (c, a)):
                edges[e] = edges.get(e, 0) + 1
        bad = [e for e, n in edges.items() if n != 1 or (e[1], e[0]) not in edges]
        if bad:
            raise ValueError("%s is not a closed, consistently wound solid (%d bad edges)" % (what, len(bad)))
        vol = sum(_dot(self.v[a], _cross(self.v[b], self.v[c])) for (a, b, c) in tris) / 6.0
        # Unreal is left handed: a front face runs the other way round than right hand maths says, so
        # the signed volume (right hand) of a correctly wound solid is negative
        if vol > 0:
            self.t[self._start:] = [(a, c, b) for (a, b, c) in tris]
        elif vol > -1e-6:
            raise ValueError("%s has no volume" % what)

    # -- solids
    def prism(self, outline, origin, u, w, d, t, mid=False):
        """Extrude a 2D outline (u, w) along `d` by `t`: point = origin + u*U + w*W + s*D."""
        poly = _clean_outline(outline)
        o = _sub(origin, _mul(d, t / 2.0)) if mid else origin

        def at(p, s):
            return _add(_add(_add(o, _mul(u, p[0])), _mul(w, p[1])), _mul(d, s))

        self.begin()
        bot = [at(p, 0.0) for p in poly]
        top = [at(p, t) for p in poly]
        n = len(poly)
        for i in range(n):
            j = (i + 1) % n
            self.quad(bot[i], bot[j], top[j], top[i])
        for (i, j, k) in _ear_clip(poly):
            self.tri(top[i], top[j], top[k])
            self.tri(bot[i], bot[k], bot[j])
        self.end("prism")

    def revolve(self, profile, segments=24, rotate=0.0, mod=None):
        """Revolve a closed (r, z[, a]) profile about the Z axis. The profile is a loop (the last
        point joins the first); points with r = 0 sit on the axis. `a` is a per-point amount handed
        to `mod(r, a, theta)` (flutes), which returns the radius."""
        self.begin()
        n = len(profile)
        rings = []
        for p in profile:
            r, z = p[0], p[1]
            a = p[2] if len(p) > 2 else 0.0
            ring = []
            for j in range(segments):
                th = math.radians(rotate) + 2 * math.pi * j / segments
                rr = mod(r, a, th) if (mod and a) else r
                ring.append((rr * math.cos(th), rr * math.sin(th), z))
            rings.append(ring)
        for i in range(n):
            k = (i + 1) % n
            for j in range(segments):
                m = (j + 1) % segments
                self.quad(rings[i][j], rings[i][m], rings[k][m], rings[k][j])
        self.end("revolve")

    def tube(self, path, radius, sides=10, caps=True):
        """A round tube along a polyline. `radius` is a number or one value per path point."""
        self.begin()
        n = len(path)
        radii = radius if isinstance(radius, (list, tuple)) else [radius] * n
        tans = []
        for i in range(n):
            a, b = path[max(i - 1, 0)], path[min(i + 1, n - 1)]
            tans.append(_unit(_sub(b, a)))
        ref = (0.0, 0.0, 1.0) if abs(tans[0][2]) < 0.9 else (1.0, 0.0, 0.0)
        side = _unit(_cross(tans[0], ref))
        rings = []
        for i in range(n):
            if i:  # parallel transport: keep `side` square to the new tangent
                side = _sub(side, _mul(tans[i], _dot(side, tans[i])))
                side = _unit(side)
            up = _cross(side, tans[i])
            ring = []
            for j in range(sides):
                th = 2 * math.pi * j / sides
                ring.append(_add(path[i], _add(_mul(side, radii[i] * math.cos(th)),
                                               _mul(up, radii[i] * math.sin(th)))))
            rings.append(ring)
        for i in range(n - 1):
            for j in range(sides):
                m = (j + 1) % sides
                self.quad(rings[i][j], rings[i][m], rings[i + 1][m], rings[i + 1][j])
        if caps:
            for j in range(sides):
                m = (j + 1) % sides
                self.tri(path[0], rings[0][m], rings[0][j])
                self.tri(path[-1], rings[-1][j], rings[-1][m])
        self.end("tube")

    def blob(self, radius_at, lon=20, lat=12, flat_bottom=None):
        """A closed star-shaped surface: `radius_at(direction)` -> (x, y, z) offset from the centre.
        Lat-long grid with one vertex at each pole."""
        self.begin()
        rows = []
        for i in range(lat + 1):
            ph = math.pi * i / lat
            ring = []
            if i in (0, lat):
                ring = [radius_at((0.0, 0.0, 1.0 if i == 0 else -1.0))] * lon
            else:
                for j in range(lon):
                    th = 2 * math.pi * j / lon
                    d = (math.sin(ph) * math.cos(th), math.sin(ph) * math.sin(th), math.cos(ph))
                    ring.append(radius_at(d))
            rows.append(ring)
        if flat_bottom is not None:
            rows = [[(p[0], p[1], max(p[2], flat_bottom)) for p in ring] for ring in rows]
        for i in range(lat):
            for j in range(lon):
                m = (j + 1) % lon
                self.quad(rows[i][j], rows[i + 1][j], rows[i + 1][m], rows[i][m])
        self.end("blob")


# --------------------------------------------------------------------------- noise (deterministic)

def _hash(ix, iy, iz, seed):
    h = (ix * 374761393 + iy * 668265263 + iz * 2147483629 + seed * 1274126177) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    h ^= h >> 16
    return (h & 0xFFFF) / 65535.0


def _vnoise(x, y, z, seed):
    """Smooth value noise in 0..1."""
    ix, iy, iz = math.floor(x), math.floor(y), math.floor(z)
    fx, fy, fz = x - ix, y - iy, z - iz
    fx, fy, fz = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy), fz * fz * (3 - 2 * fz)

    def lerp(a, b, t):
        return a + (b - a) * t

    c = [[[_hash(ix + dx, iy + dy, iz + dz, seed) for dz in (0, 1)] for dy in (0, 1)] for dx in (0, 1)]
    return lerp(lerp(lerp(c[0][0][0], c[1][0][0], fx), lerp(c[0][1][0], c[1][1][0], fx), fy),
                lerp(lerp(c[0][0][1], c[1][0][1], fx), lerp(c[0][1][1], c[1][1][1], fx), fy), fz)


def _fbm(p, seed, octaves=3):
    total, amp, freq, norm = 0.0, 1.0, 1.0, 0.0
    for o in range(octaves):
        total += amp * (_vnoise(p[0] * freq, p[1] * freq, p[2] * freq, seed + o * 17) - 0.5) * 2
        norm += amp
        amp *= 0.5
        freq *= 2.1
    return total / norm


def _spline(points, per=6):
    """Catmull-Rom through 2D points (ends repeated), `per` samples per span."""
    pts = [points[0]] + list(points) + [points[-1]]
    out = []
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        for s in range(per):
            t = s / per
            t2, t3 = t * t, t * t * t
            out.append(tuple(0.5 * ((2 * p1[k]) + (-p0[k] + p2[k]) * t + (2 * p0[k] - 5 * p1[k] + 4 * p2[k] - p3[k]) * t2
                                    + (-p0[k] + 3 * p1[k] - 3 * p2[k] + p3[k]) * t3) for k in range(len(p1))))
    out.append(tuple(points[-1]))
    return out


# --------------------------------------------------------------------------- assets

def assets_folder(lv, name="Shapes"):
    """Where a level's shape assets live: /Game/<game>/<name>."""
    return "/Game/%s/%s" % (lv.game, name)


def _asset_name(kind, params):
    blob = json.dumps([SHAPES_VERSION, kind, params], sort_keys=True, default=str)
    return "SM_%s_%s" % (kind, hashlib.md5(blob.encode()).hexdigest()[:8])


def _dynamic(m, angle):
    """A DynamicMesh from the builder, with split normals (hard edges above `angle` degrees)."""
    dm = unreal.new_object(unreal.DynamicMesh)
    buf = unreal.GeometryScriptSimpleMeshBuffers()
    buf.set_editor_property("vertices", [unreal.Vector(*v) for v in m.v])
    buf.set_editor_property("triangles", [unreal.IntVector(*t) for t in m.t])
    # a planar set along a direction no axis-aligned face contains: no degenerate tangent basis
    buf.set_editor_property("uv0", [unreal.Vector2D((0.8 * v[0] - 0.5 * v[1] + 0.3 * v[2]) / 100.0,
                                                    (0.2 * v[0] + 0.7 * v[1] - 0.9 * v[2]) / 100.0) for v in m.v])
    unreal.GeometryScript_MeshEdits.append_buffers_to_mesh(dm, buf)
    unreal.GeometryScript_Normals.compute_split_normals(
        dm, unreal.GeometryScriptSplitNormalsOptions(split_by_opening_angle=True, opening_angle_deg=angle),
        unreal.GeometryScriptCalculateNormalsOptions(angle_weighted=True, area_weighted=True))
    return dm


def _make(folder, kind, params, build, angle=40.0):
    """The cached StaticMesh `kind` for `params`, built by `build(Mesh)` when it does not exist."""
    name = _asset_name(kind, params)
    path = folder.rstrip("/") + "/" + name
    if EAL.does_asset_exist(path):
        return EAL.load_asset(path)
    m = Mesh()
    build(m)
    if not m.t:
        raise ValueError("%s: no triangles" % kind)
    dm = _dynamic(m, angle)
    if not unreal.GeometryScript_MeshQueries.get_is_closed_mesh(dm):
        warn("%s is not closed after welding" % name)
    opts = unreal.GeometryScriptCreateNewStaticMeshAssetOptions()
    setp(opts, enable_recompute_normals=False, enable_recompute_tangents=True, enable_collision=True,
         collision_mode=unreal.CollisionTraceFlag.CTF_USE_COMPLEX_AS_SIMPLE, enable_nanite=False)
    sm, outcome = unreal.GeometryScript_NewAssetUtils.create_new_static_mesh_asset_from_mesh(dm, path, opts)
    if sm is None:
        raise RuntimeError("could not create %s (%s)" % (path, outcome))
    EAL.save_asset(path, only_if_is_dirty=False)
    log("shape %s: %d verts, %d tris" % (name, len(m.v), len(m.t)))
    return sm


def purge(folder):
    """Delete every shape asset in `folder` (a rebuild with changed builders starts from nothing)."""
    if EAL.does_directory_exist(folder):
        EAL.delete_directory(folder)


def place(lv, label, mesh, at, rot=0.0, mat="prop", folder="Props", scale=1.0):
    """Put a shape in the level with its origin at `at` (cm). `rot` is a yaw, or (pitch, yaw, roll).
    `mat` is a name in lv.mats or a material. Same labels and outliner folders as Level.box/shape."""
    p, y, r = (0.0, rot, 0.0) if isinstance(rot, (int, float)) else rot
    a = lv._done(lv._spawn(unreal.StaticMeshActor, unreal.Vector(*at), unreal.Rotator(roll=r, pitch=p, yaw=y)),
                 label, folder)
    if scale != 1.0:
        s = scale if isinstance(scale, (list, tuple)) else (scale,) * 3
        a.set_actor_scale3d(unreal.Vector(*s))
    comp = a.get_component_by_class(unreal.StaticMeshComponent)
    comp.set_static_mesh(mesh)
    material = lv.mats[mat] if isinstance(mat, str) else mat
    for i in range(max(1, comp.get_num_materials())):
        comp.set_material(i, material)
    return a


# --------------------------------------------------------------------------- shapes

def prism(folder, outline, thickness, plane="xz", angle=40.0):
    """An extruded 2D outline (any simple closed polygon, cm). plane "xz": the outline is (x, z) and
    the thickness runs along Y, centred on 0 (a gable wall, a sign, a wedge). plane "xy": the outline
    is (x, y) and rises from z = 0 to `thickness` (a plinth, a pond bank).
    prism(F, [(0, 0), (300, 0), (150, 120)], 20)"""
    pts = [tuple(p) for p in outline]
    if plane == "xz":
        def build(m):
            m.prism(pts, (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), thickness, mid=True)
    elif plane == "xy":
        def build(m):
            m.prism(pts, (0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1), thickness)
    else:
        raise ValueError("plane must be 'xz' or 'xy'")
    return _make(folder, "Prism", [pts, thickness, plane], build, angle)


def gable_outline(span, wall_h, pitch=35.0):
    """The (y, z) pentagon of a gable end: walls of height `wall_h`, ridge at span/2 * tan(pitch)."""
    ridge = span / 2.0 * math.tan(math.radians(pitch))
    return [(-span / 2, 0.0), (span / 2, 0.0), (span / 2, wall_h), (0.0, wall_h + ridge), (-span / 2, wall_h)]


def gable_wall(folder, span, wall_h, pitch=35.0, thick=20.0):
    """A closed gable end wall (a pentagon prism `thick` along X, centred), for houses built from
    walls with openings. Local Y runs across the span.
    gable_wall(F, 600, 280, 35, 20)"""
    pts = gable_outline(span, wall_h, pitch)

    def build(m):
        m.prism(pts, (0, 0, 0), (0, 1, 0), (0, 0, 1), (1, 0, 0), thick, mid=True)
    return _make(folder, "GableWall", [span, wall_h, pitch, thick], build)


def house_body(folder, length, span, wall_h, pitch=35.0):
    """A solid house body, ridge along X: `length` x `span` footprint, walls `wall_h` high, both
    gable ends closed up to the ridge. Put a `roof` of the same `span`/`pitch` on top.
    house_body(F, 800, 600, 280, 35)"""
    pts = gable_outline(span, wall_h, pitch)

    def build(m):
        m.prism(pts, (0, 0, 0), (0, 1, 0), (0, 0, 1), (1, 0, 0), length, mid=True)
    return _make(folder, "HouseBody", [length, span, wall_h, pitch], build)


def roof(folder, length, span, pitch=35.0, thick=14.0, overhang=30.0, gable_overhang=None):
    """A pitched roof with real thickness: two slopes that meet in a ridge along X, eaves running
    `overhang` past the walls (across the span) and `gable_overhang` (default the same) past the gable
    ends. Origin on the wall-top plane: the underside touches the wall tops and the ridge of a
    `house_body` with the same span and pitch exactly.
    roof(F, 800, 600, 35, 14, 30)"""
    go = overhang if gable_overhang is None else gable_overhang
    tn = math.tan(math.radians(pitch))
    tv = thick / math.cos(math.radians(pitch))
    hs = span / 2.0
    ye = hs + overhang
    section = [(0.0, hs * tn), (ye, -overhang * tn), (ye, -overhang * tn + tv), (0.0, hs * tn + tv),
               (-ye, -overhang * tn + tv), (-ye, -overhang * tn)]
    if pitch < 1.0:
        raise ValueError("roof pitch must be at least 1 degree (use prism for a flat slab)")

    def build(m):
        m.prism(section, (0, 0, 0), (0, 1, 0), (0, 0, 1), (1, 0, 0), length + 2 * go, mid=True)
    return _make(folder, "Roof", [length, span, pitch, thick, overhang, go], build)


def lathe(folder, name, profile, segments=24, rotate=0.0, angle=40.0):
    """A solid of revolution about Z from a (radius, height) profile: a closed loop (the last point
    joins the first); points with radius 0 are on the axis. `name` tells assets apart in the browser.
    lathe(F, "Pawn", [(0, 0), (20, 0), (8, 50), (14, 60), (0, 70)], 32)"""
    prof = [tuple(p) for p in profile]

    def build(m):
        m.revolve(prof, segments, rotate)
    return _make(folder, "Lathe_" + name, [prof, segments, rotate], build, angle)


def cone_roof(folder, radius, height, thick=12.0, overhang=25.0, segments=32, rotate=0.0):
    """A conical (segments 4, rotate 45 = pyramid) roof for a round tower or a well: its underside
    touches the wall-top circle `radius` and the apex at `height`, with eaves `overhang` beyond.
    Origin on the wall-top plane.
    cone_roof(F, 200, 160, 12, 25, 32)"""
    slope = height / radius
    ro = radius + overhang
    zo = -overhang * slope
    tv = thick * math.sqrt(1 + slope * slope)
    prof = [(0.0, height + tv), (ro, zo + tv), (ro, zo), (0.0, height)]

    def build(m):
        m.revolve(prof, segments, rotate)
    return _make(folder, "ConeRoof", [radius, height, thick, overhang, segments, rotate], build, 50.0)


def column(folder, radius, height, taper=0.85, flutes=0, flute_depth=0.08, segments=None):
    """A column with a stepped base and a flared capital with an abacus. `taper` is the top radius
    over the bottom radius of the shaft; `flutes` > 0 cuts that many grooves (smooth normals).
    column(F, 40, 500, 0.85, flutes=16)"""
    segments = segments or (max(32, flutes * 4) if flutes else 28)
    r, h = radius, height
    base, cap = h * 0.06, h * 0.08
    s0, s1 = base * 2, h - cap * 1.6  # shaft bottom / top
    ramp = (s1 - s0) * 0.06

    def shaft(z):
        u = (z - s0) / (s1 - s0)
        return r * (1 + (taper - 1) * u + 0.025 * math.sin(math.pi * u))

    shaft_pts = []
    n = 10
    for i in range(n + 1):
        z = s0 + (s1 - s0) * i / n
        a = flute_depth if (flutes and ramp < z - s0 < s1 - s0 - ramp) else 0.0
        shaft_pts.append((shaft(z), z, a))
    top_r = shaft(s1)
    prof = [(0.0, 0.0), (r * 1.55, 0.0), (r * 1.55, base), (r * 1.3, base), (r * 1.3, base * 1.6),
            (r * 1.14, base * 1.6), (r * 1.14, s0)]
    prof += shaft_pts
    prof += [(top_r * 1.12, s1), (top_r * 1.28, h - cap * 0.9), (top_r * 1.45, h - cap * 0.55),
             (r * 1.5, h - cap * 0.55), (r * 1.5, h), (0.0, h)]

    def mod(rr, a, th):
        return rr * (1 - a * (0.5 - 0.5 * math.cos(flutes * th)))

    def build(m):
        m.revolve(prof, segments, 0.0, mod if flutes else None)
    return _make(folder, "Column", [radius, height, taper, flutes, flute_depth, segments], build, 50.0)


def barrel(folder, radius, height, bulge=0.12, segments=24):
    """A barrel with a bulging belly, a lip at both ends and a recessed head.
    barrel(F, 28, 70, 0.12)"""
    r, h = radius, height
    lip = h * 0.04
    prof = [(0.0, lip * 1.4), (r * 0.82, lip * 1.4), (r * 0.82, 0.0), (r * 0.94, 0.0)]
    n = 10
    for i in range(n + 1):
        z = lip + (h - 2 * lip) * i / n
        prof.append((r * (0.97 + bulge * math.sin(math.pi * i / n)), z))
    prof += [(r * 0.94, h), (r * 0.82, h), (r * 0.82, h - lip * 1.4), (0.0, h - lip * 1.4)]

    def build(m):
        m.revolve(prof, segments)
    return _make(folder, "Barrel", [radius, height, bulge, segments], build, 45.0)


def well_ring(folder, outer, inner, height, lip=True, segments=32):
    """A hollow ring (a well, a fire pit, a planter): wall between `inner` and `outer` radius, with a
    wider lip at the top when `lip`. The hole goes all the way through.
    well_ring(F, 90, 65, 80)"""
    h = height
    prof = [(inner, 0.0), (outer, 0.0), (outer, h * 0.82)]
    if lip:
        prof += [(outer * 1.07, h * 0.82), (outer * 1.07, h), (inner * 0.97, h), (inner * 0.97, h * 0.82)]
        prof += [(inner, h * 0.82)]
    else:
        prof += [(outer, h), (inner, h)]

    def build(m):
        m.revolve(prof, segments)
    return _make(folder, "WellRing", [outer, inner, height, lip, segments], build, 50.0)


def urn(folder, radius, height, segments=28):
    """A smooth urn: foot, round belly, narrow neck, flared lip, shallow mouth.
    urn(F, 30, 90)"""
    r, h = radius, height
    pts = [(r * 0.55, 0.0), (r * 0.62, h * 0.06), (r * 0.95, h * 0.30), (r * 1.0, h * 0.45), (r * 0.8, h * 0.68),
           (r * 0.45, h * 0.82), (r * 0.42, h * 0.9), (r * 0.62, h * 0.97), (r * 0.6, h)]
    prof = [(0.0, 0.0)] + _spline(pts, 5) + [(r * 0.48, h), (r * 0.46, h * 0.95), (0.0, h * 0.95)]

    def build(m):
        m.revolve(prof, segments)
    return _make(folder, "Urn", [radius, height, segments], build, 55.0)


def kettle(folder, radius, height, segments=28):
    """A kettle: round body, lid with a knob, a curved spout toward +X and a loop handle toward -X.
    The spout and handle are separate closed solids that run into the body.
    kettle(F, 14, 18)"""
    r, h = radius, height
    body = [(r * 0.8, 0.0), (r * 0.98, h * 0.12), (r * 1.0, h * 0.35), (r * 0.82, h * 0.62),
            (r * 0.58, h * 0.72), (r * 0.5, h * 0.76)]
    lid = [(r * 0.5, h * 0.8), (r * 0.42, h * 0.86), (r * 0.26, h * 0.92), (r * 0.12, h * 0.95)]
    prof = [(0.0, 0.0)] + _spline(body, 4) + [(r * 0.5, h * 0.8)] + _spline(lid, 4)[1:] + [(r * 0.14, h * 1.02), (0.0, h * 1.02)]
    spout = [(r * 0.75, 0.0, h * 0.28), (r * 1.15, 0.0, h * 0.36), (r * 1.5, 0.0, h * 0.6), (r * 1.72, 0.0, h * 0.78)]
    handle = [(-r * 0.8, 0.0, h * 0.7), (-r * 1.3, 0.0, h * 0.76), (-r * 1.55, 0.0, h * 0.52),
              (-r * 1.3, 0.0, h * 0.26), (-r * 0.9, 0.0, h * 0.2)]

    def build(m):
        m.revolve(prof, segments)
        sp = _spline3(spout, 5)
        m.tube(sp, _lerp_radii(len(sp), r * 0.2, r * 0.1), 8)
        m.tube(_spline3(handle, 5), r * 0.07, 8)
    return _make(folder, "Kettle", [radius, height, segments], build, 50.0)


def _spline3(points, per=5):
    cols = [_spline([(p[k],) for p in points], per) for k in range(3)]
    return [(cols[0][i][0], cols[1][i][0], cols[2][i][0]) for i in range(len(cols[0]))]


def _lerp_radii(n, a, b):
    return [a + (b - a) * i / (n - 1) for i in range(n)]


def arch(folder, width, height, depth=60.0, pier=60.0, segments=16):
    """An arched gateway (a wall with a round-headed opening, open to the ground): clear opening
    `width` x `height` (the round head included), `pier` of wall on each side and above the crown,
    `depth` thick (Y). The opening runs along X; stand in it facing Y.
    arch(F, 260, 340, 70, 70)"""
    hw = width / 2.0
    spring = height - hw
    if spring <= 0:
        raise ValueError("arch height must exceed half the width")
    top = height + pier
    pts = [(-hw - pier, 0.0), (-hw - pier, top), (hw + pier, top), (hw + pier, 0.0), (hw, 0.0), (hw, spring)]
    for i in range(1, segments):
        a = math.pi * i / segments
        pts.append((hw * math.cos(a), spring + hw * math.sin(a)))
    pts += [(-hw, spring), (-hw, 0.0)]

    def build(m):
        m.prism(pts, (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), depth, mid=True)
    return _make(folder, "Arch", [width, height, depth, pier, segments], build, 30.0)


def stairs(folder, steps, rise, run, width):
    """Solid linear stairs rising along +X from z = 0: `steps` treads (the first is `rise` high,
    the last at steps * rise), each `run` deep, `width` across Y (centred). Solid down to the ground.
    stairs(F, 8, 18, 30, 200)"""
    pts = [(0.0, 0.0), (steps * run, 0.0)]
    for k in range(steps, 0, -1):
        pts += [(k * run, k * rise), ((k - 1) * run, k * rise)]

    def build(m):
        m.prism(pts, (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0), width, mid=True)
    return _make(folder, "Stairs", [steps, rise, run, width], build, 30.0)


def ramp(folder, length, width, height):
    """A wedge ramp rising along +X from 0 to `height`, `width` across Y (centred).
    ramp(F, 400, 200, 60)"""
    def build(m):
        m.prism([(0.0, 0.0), (length, 0.0), (length, height)], (0, 0, 0), (1, 0, 0), (0, 0, 1), (0, 1, 0),
                width, mid=True)
    return _make(folder, "Ramp", [length, width, height], build, 30.0)


def rock(folder, radius, seed=1, squash=0.7, rough=0.28, lon=18, lat=11):
    """A rock: a sphere pushed in and out by deterministic noise, squashed, sitting flat on z = 0
    (origin at the middle of the bottom). `rough` is the size of the lumps (0.1 pebble ... 0.4 crag).
    rock(F, 80, seed=3)"""
    cx = (1.7 * seed % 5, 2.3 * seed % 7, 1.1 * seed % 3)

    def at(d):
        p = (d[0] * 1.3 + cx[0], d[1] * 1.3 + cx[1], d[2] * 1.3 + cx[2])
        k = 1.0 + rough * _fbm(p, seed) + 0.06 * _fbm(_mul(p, 3.1), seed + 5)
        return (d[0] * radius * k, d[1] * radius * k * (0.85 + 0.2 * _vnoise(cx[0], cx[1], 3, seed)),
                d[2] * radius * k * squash)

    cut = -radius * squash * 0.55

    def build(m):
        m.blob(at, lon, lat, flat_bottom=cut)
        m.v[:] = [(x, y, z - cut) for (x, y, z) in m.v]
    return _make(folder, "Rock", [radius, seed, squash, rough, lon, lat], build, 60.0)


def canopy(folder, radius, seed=1, lobes=5, lumps=0.38, lon=22, lat=14):
    """A tree canopy: one closed lumpy surface (a sphere pulled out toward `lobes` random directions
    and roughened by noise), flatter underneath. Origin at the centre of the bottom.
    canopy(F, 220, seed=2)"""
    dirs = []
    for i in range(lobes):
        a = _hash(i, 1, 0, seed) * 2 * math.pi
        e = (_hash(i, 2, 0, seed) - 0.3) * 1.1
        dirs.append((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))

    def at(d):
        bump = sum(math.exp(-((1 - _dot(d, u)) * 5.0)) for u in dirs) / max(1, lobes)
        k = 0.78 + lumps * bump * 2.2 + 0.1 * _fbm((d[0] * 2.2, d[1] * 2.2, d[2] * 2.2 + seed), seed)
        z = d[2] * k * radius * (0.62 if d[2] < 0 else 0.9)
        return (d[0] * k * radius, d[1] * k * radius, z)

    def build(m):
        m.blob(at, lon, lat)
        zmin = min(v[2] for v in m.v)
        m.v[:] = [(x, y, z - zmin) for (x, y, z) in m.v]
    return _make(folder, "Canopy", [radius, seed, lobes, lumps, lon, lat], build, 70.0)


def trunk(folder, radius, height, top=0.55, lean=0.0, flare=0.5, seed=1, sides=12):
    """A tapered trunk with a root flare, leaning `lean` cm toward +X at the top; `top` is the top
    radius over the bottom radius. Closed at both ends; origin at the middle of the bottom.
    trunk(F, 22, 380, 0.5, lean=30)"""
    n = 12
    path, radii = [], []
    for i in range(n + 1):
        u = i / n
        sway = math.sin(u * math.pi * 1.5 + seed) * radius * 0.25
        path.append((lean * u * u + sway * u, sway * 0.6 * u, height * u))
        radii.append(radius * ((1 - u) + top * u) * (1 + flare * math.exp(-u * 14)))

    def build(m):
        m.tube(path, radii, sides)
    return _make(folder, "Trunk", [radius, height, top, lean, flare, seed, sides], build, 50.0)


# --------------------------------------------------------------------------- composites

def house(lv, label, at, yaw, length, span, wall_h, pitch=35.0, body_mat="plaster", roof_mat="roof",
          thick=14.0, overhang=30.0, folder="Buildings", assets=None):
    """A solid house with a roof sitting exactly on it: returns (body, roof). `at` is the middle of
    the footprint on the ground, `yaw` turns the ridge (0 = ridge along X).
    house(lv, "Cottage", (0, 0, 0), 90, 800, 600, 280)"""
    f = assets or assets_folder(lv)
    body = place(lv, label + "_Body", house_body(f, length, span, wall_h, pitch), at, yaw, body_mat, folder)
    top = roof(f, length, span, pitch, thick, overhang)
    rf = place(lv, label + "_Roof", top, (at[0], at[1], at[2] + wall_h), yaw, roof_mat, folder)
    return body, rf


def tree(lv, label, at, height, seed=1, trunk_mat="wood", leaf_mat="grass", folder="Trees", assets=None):
    """A trunk with a canopy sitting on it: returns (trunk, canopy). `height` is the whole tree.
    tree(lv, "Oak", (0, 0, 0), 500, seed=2)"""
    f = assets or assets_folder(lv)
    th = height * 0.62
    tk = trunk(f, height * 0.045, th, 0.5, lean=0.0, seed=seed)
    cr = height * 0.3
    cn = canopy(f, cr, seed)
    a = place(lv, label + "_Trunk", tk, at, 0.0, trunk_mat, folder)
    b = place(lv, label + "_Canopy", cn, (at[0], at[1], at[2] + th * 0.78), seed * 37.0 % 360, leaf_mat, folder)
    return a, b
