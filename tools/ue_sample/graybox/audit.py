"""Structural audit of a built sample level (bounds math only, no collision traces).

Collision does not work in a commandlet, so everything here is arithmetic on each mesh's local
bounds and the actor's world transform (oriented boxes). It reports, with labels, positions and
sizes:

  zfight    two actors with coplanar faces that face the same way and overlap in area (flicker)
  floating  static meshes that touch nothing, or whole clusters not connected to the ground
  support   long spans (beams, lintels) with an end that rests on nothing and is not held at its end
  bodies    mannequins hovering, sunk, inside a solid, inside each other; held props away from the hand
  walkers   the keyed paths of level-sequence walkers swept against the solids (sunk feet, walking through)
  planted   posts and boxes that start 20+ cm inside another solid (trees in hills)
  dupes     identical meshes at one transform, and meshes buried entirely inside another

Round shapes are approximated: a cylinder or cone or sphere is tested by its true profile where
that is cheap (containment, cylinder caps) and by its bounding box otherwise (contact).
Everything geometric is plain Python so it can be unit tested without the engine; `collect`
and `run` are the only parts that touch `unreal`.
"""

import math
import re

try:
    import unreal
except ImportError:  # the geometry below is tested outside the engine
    unreal = None

ZFIGHT_PLANE = 0.5      # cm: faces closer than this to one plane flicker
ZFIGHT_AREA = 1.0       # cm2: overlap smaller than this is not visible
TOUCH = 1.0             # cm: contact tolerance
SAME_FACING = 0.99999   # cosine of the angle under which two normals count as the same way


# ------------------------------------------------------------------------------ vector helpers

def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def mul(a, k):
    return (a[0] * k, a[1] * k, a[2] * k)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def norm(a):
    return math.sqrt(dot(a, a))


def rotate(q, v):
    """Rotate v by the quaternion q = (x, y, z, w) (the same formula Unreal uses)."""
    qv = (q[0], q[1], q[2])
    t = mul(cross(qv, v), 2.0)
    return add(add(v, mul(t, q[3])), cross(qv, t))


class Item:
    """One mesh as an oriented box: centre `c`, unit axes `ax` (the component's local x, y, z in
    the world) and half sizes `h` along them. `kind` says how round it really is."""

    def __init__(self, label, kind, c, ax, h, mesh="", parent=None, folder="", ref=None, quat=None, scale=None):
        self.label, self.kind, self.c, self.ax, self.h = label, kind, c, ax, h
        self.mesh, self.parent, self.folder, self.ref = mesh, parent, folder, ref
        self.quat, self.scale = quat, scale
        ex = [sum(abs(self.ax[i][k]) * self.h[i] for i in range(3)) for k in range(3)]
        self.lo = tuple(self.c[k] - ex[k] for k in range(3))
        self.hi = tuple(self.c[k] + ex[k] for k in range(3))

    @property
    def stem(self):
        return re.sub(r"[_ ]*\d+$", "", self.label) or self.label

    @property
    def size(self):
        return tuple(2 * v for v in self.h)

    def describe(self):
        return "%s at (%.0f, %.0f, %.0f) size %.0f x %.0f x %.0f" % ((self.label,) + tuple(self.c) + self.size)

    def local(self, p):
        d = sub(p, self.c)
        return tuple(dot(d, a) for a in self.ax)

    def contains(self, p, pad=0.0):
        """Is the world point p inside the true shape (box, cylinder along local z, cone apex up, sphere)?"""
        lx, ly, lz = self.local(p)
        hx, hy, hz = self.h[0] + pad, self.h[1] + pad, self.h[2] + pad
        if abs(lz) > hz:
            return False
        if self.kind == "cyl":
            return (lx / hx) ** 2 + (ly / hy) ** 2 <= 1.0
        if self.kind == "cone":
            r = (hz - lz) / (2 * hz)
            return (lx / hx) ** 2 + (ly / hy) ** 2 <= r * r
        if self.kind == "sphere":
            return (lx / hx) ** 2 + (ly / hy) ** 2 + (lz / hz) ** 2 <= 1.0
        return abs(lx) <= hx and abs(ly) <= hy

    def corners(self):
        out = []
        for sx in (-1, 1):
            for sy in (-1, 1):
                for sz in (-1, 1):
                    out.append(add(self.c, add(add(mul(self.ax[0], sx * self.h[0]), mul(self.ax[1], sy * self.h[1])),
                                               mul(self.ax[2], sz * self.h[2]))))
        return out


def aabb_near(a, b, tol):
    return all(a.lo[k] - tol <= b.hi[k] and b.lo[k] - tol <= a.hi[k] for k in range(3))


def obb_gap(a, b):
    """A lower bound on the distance between two oriented boxes (separating-axis test; 0 means they
    touch or overlap)."""
    axes = list(a.ax) + list(b.ax)
    for i in range(3):
        for j in range(3):
            c = cross(a.ax[i], b.ax[j])
            n = norm(c)
            if n > 1e-6:
                axes.append(mul(c, 1.0 / n))
    d = sub(b.c, a.c)
    best = 0.0
    for ax in axes:
        ra = sum(abs(dot(ax, a.ax[i])) * a.h[i] for i in range(3))
        rb = sum(abs(dot(ax, b.ax[i])) * b.h[i] for i in range(3))
        best = max(best, abs(dot(d, ax)) - ra - rb)
    return best


def touching(a, b, tol=TOUCH):
    return aabb_near(a, b, tol) and obb_gap(a, b) <= tol


# ---------------------------------------------------------------------------------- polygons

def poly_area(p):
    s = 0.0
    for i in range(len(p)):
        x0, y0 = p[i]
        x1, y1 = p[(i + 1) % len(p)]
        s += x0 * y1 - x1 * y0
    return s / 2.0


def clip(subject, clipper):
    """Sutherland-Hodgman: the part of convex polygon `subject` inside convex `clipper`."""
    if poly_area(clipper) < 0:
        clipper = clipper[::-1]
    out = subject
    for i in range(len(clipper)):
        if not out:
            break
        (ax, ay), (bx, by) = clipper[i], clipper[(i + 1) % len(clipper)]

        def inside(p):
            return (bx - ax) * (p[1] - ay) - (by - ay) * (p[0] - ax) >= -1e-9

        def meet(p, q):
            d1 = (bx - ax) * (p[1] - ay) - (by - ay) * (p[0] - ax)
            d2 = (bx - ax) * (q[1] - ay) - (by - ay) * (q[0] - ax)
            t = d1 / (d1 - d2)
            return (p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t)

        inp, out = out, []
        for j in range(len(inp)):
            p, q = inp[j], inp[(j + 1) % len(inp)]
            if inside(q):
                if not inside(p):
                    out.append(meet(p, q))
                out.append(q)
            elif inside(p):
                out.append(meet(p, q))
    return out


def faces(it):
    """The flat faces that can flicker: the six faces of a box, the two caps of a cylinder. Each is
    (normal, plane offset, polygon of 3D points)."""
    if it.kind == "box":
        spec = [(k, s) for k in range(3) for s in (-1, 1)]
    elif it.kind == "cyl":
        spec = [(2, -1), (2, 1)]
    else:
        return []
    out = []
    for k, s in spec:
        n = mul(it.ax[k], s)
        fc = add(it.c, mul(n, it.h[k]))
        i, j = [x for x in range(3) if x != k]
        if it.kind == "cyl":
            pts = [add(fc, add(mul(it.ax[i], it.h[i] * math.cos(t)), mul(it.ax[j], it.h[j] * math.sin(t))))
                   for t in [2 * math.pi * m / 24 for m in range(24)]]
        else:
            pts = [add(fc, add(mul(it.ax[i], si * it.h[i]), mul(it.ax[j], sj * it.h[j])))
                   for si, sj in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
        out.append((n, dot(n, fc), pts))
    return out


def zfight(items):
    """Pairs of different actors with coplanar, same-facing, overlapping faces."""
    hits = []
    fcache = {}
    for ai in range(len(items)):
        a = items[ai]
        if a.kind not in ("box", "cyl"):
            continue
        for bi in range(ai + 1, len(items)):
            b = items[bi]
            if b.kind not in ("box", "cyl") or not aabb_near(a, b, ZFIGHT_PLANE):
                continue
            fa = fcache.setdefault(ai, faces(a))
            fb = fcache.setdefault(bi, faces(b))
            for na, da, pa in fa:
                for nb, db, pb in fb:
                    if dot(na, nb) < SAME_FACING or abs(da - db) > ZFIGHT_PLANE:
                        continue
                    e1 = sub(pa[1], pa[0])
                    e1 = mul(e1, 1.0 / norm(e1))
                    e2 = cross(na, e1)
                    o = pa[0]

                    def flat(pts):
                        return [(dot(sub(p, o), e1), dot(sub(p, o), e2)) for p in pts]
                    poly = clip(flat(pb), flat(pa))
                    area = abs(poly_area(poly))
                    if area > ZFIGHT_AREA:
                        # a face with a solid right in front of it (the ground, a wall) cannot be seen
                        cx, cy = sum(p[0] for p in poly) / len(poly), sum(p[1] for p in poly) / len(poly)
                        mid = add(o, add(mul(e1, cx), mul(e2, cy)))
                        front = add(mid, mul(na, 2.0))
                        hidden = some_item_contains(items, front, None, skip2=(a, b)) is not None
                        hits.append((area, a, b, na, hidden))
    return hits


# ----------------------------------------------------------------------------- the checks

def is_root(it):
    """Ground slabs: wide, thin, near z = 0. Everything must hang off one of these."""
    return max(it.h[0], it.h[1], it.h[2]) >= 1000 and min(it.h) <= 60 and it.lo[2] <= 60


def contacts(items):
    """Adjacency lists of touching static items."""
    adj = [[] for _ in items]
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            if touching(items[i], items[j]):
                adj[i].append(j)
                adj[j].append(i)
    return adj


def floating(items):
    """(isolated, clusters): items touching nothing, and groups not connected to any ground slab.
    Parts attached to one parent actor (a spinning group) count as one rigid thing."""
    adj = contacts(items)
    by_parent = {}
    for i, it in enumerate(items):
        if it.parent is not None:
            by_parent.setdefault(it.parent, []).append(i)
    for members in by_parent.values():
        for i in members[1:]:
            adj[members[0]].append(i)
            adj[i].append(members[0])
    seen, comps = set(), []
    for s in range(len(items)):
        if s in seen:
            continue
        comp, stack = [], [s]
        seen.add(s)
        while stack:
            u = stack.pop()
            comp.append(u)
            for v in adj[u]:
                if v not in seen:
                    seen.add(v)
                    stack.append(v)
        comps.append(comp)
    isolated, clusters = [], []
    for comp in comps:
        if any(is_root(items[i]) for i in comp):
            continue
        if len(comp) == 1 and items[comp[0]].parent is None:
            isolated.append(items[comp[0]])
        else:
            clusters.append([items[i] for i in comp])
    return isolated, clusters


def some_item_contains(items, p, skip, pad=0.0, skip2=()):
    for o in items:
        if o is skip or o in skip2:
            continue
        if all(o.lo[k] - pad <= p[k] <= o.hi[k] + pad for k in range(3)) and o.contains(p, pad):
            return o
    return None


def hovering(items, reach=80.0):
    """Items whose underside is level and hangs above a solid with a gap (a roof lifted off its walls,
    a lantern with no chain): (item, gap, item below). An item is held when any of five points of its
    underside touches a solid, or (for boxes and cylinders) when a prism-shaped solid reaches its
    underside; parts of a moving group are skipped."""
    out = []
    for it in items:
        if it.kind not in ("box", "cyl", "cone", "mesh") or is_root(it) or it.parent is not None:
            continue
        v = max(range(3), key=lambda k: abs(it.ax[k][2]))
        if abs(it.ax[v][2]) < 0.999:
            continue
        hz = [k for k in range(3) if k != v]
        z0 = it.lo[2]
        pts = [it.c[:2]] + [(it.c[0] + sx * 0.7 * it.h[hz[0]] * it.ax[hz[0]][0] + sy * 0.7 * it.h[hz[1]] * it.ax[hz[1]][0],
                             it.c[1] + sx * 0.7 * it.h[hz[0]] * it.ax[hz[0]][1] + sy * 0.7 * it.h[hz[1]] * it.ax[hz[1]][1])
                            for sx, sy in ((1, 1), (1, -1), (-1, 1), (-1, -1))]
        gaps, held = [], False
        for (x, y) in pts:
            hit = surface_below(items, x, y, z0 + 0.5, z0 - reach, skip=(it,))
            if hit is None:
                continue
            g = z0 - hit[0]
            if g <= 1.5:
                held = True
                break
            gaps.append((g, hit[1]))
        if not held and it.kind in ("box", "cyl"):
            for o in items:
                if o is it or o.kind not in ("box", "cyl", "mesh") or o.lo[2] >= z0 or o.hi[2] < z0 - 1.5:
                    continue
                if touching(it, o):
                    held = True
                    break
        if not held and gaps:
            g, below = min(gaps, key=lambda r: r[0])
            out.append((it, g, below))
    return out


def planted(items, least=20.0):
    """Items whose level underside starts inside another solid: (item, depth in cm, solid). Posts in
    the ground are normal, so only a depth of `least` cm or more counts."""
    out = []
    for it in items:
        if it.kind not in ("box", "cyl", "cone", "mesh") or is_root(it) or it.parent is not None:
            continue
        v = max(range(3), key=lambda k: abs(it.ax[k][2]))
        if abs(it.ax[v][2]) < 0.999:
            continue
        hz = [k for k in range(3) if k != v]
        z0 = it.lo[2]
        pts = [it.c[:2]] + [(it.c[0] + sx * 0.7 * it.h[hz[0]] * it.ax[hz[0]][0] + sy * 0.7 * it.h[hz[1]] * it.ax[hz[1]][0],
                             it.c[1] + sx * 0.7 * it.h[hz[0]] * it.ax[hz[0]][1] + sy * 0.7 * it.h[hz[1]] * it.ax[hz[1]][1])
                            for sx, sy in ((1, 1), (1, -1), (-1, 1), (-1, -1))]
        depths, solid = [], None
        for (x, y) in pts:
            hit = surface_below(items, x, y, z0 + 0.5, z0 + 0.5, skip=(it,))
            if hit is None:
                break
            solid = solid or hit[1]
            z = z0 + 0.5
            while z < z0 + 600.0 and surface_below(items, x, y, z, z, skip=(it,)) is not None:
                z += 1.0
            depths.append(z - z0)
        if len(depths) < len(pts):
            continue
        z = z0 + min(depths)
        hit = (0, solid)
        if z - z0 >= least:
            out.append((it, z - z0, hit[1]))
    return out


def spans(items):
    """Long, level spans (beams, lintels, slabs on posts) and how each end is carried.
    Yields (item, [(end, state)]) with state 'below' / 'held' (touching something at its end) / 'none'."""
    out = []
    for it in items:
        if it.kind not in ("box", "mesh") or is_root(it) or it.parent is not None:
            continue
        # which local axis is the vertical one, and which is the long horizontal one
        v = max(range(3), key=lambda k: abs(it.ax[k][2]))
        if abs(it.ax[v][2]) < 0.999:
            continue
        hz = [k for k in range(3) if k != v]
        long_k = max(hz, key=lambda k: it.h[k])
        short = max(it.h[k] for k in range(3) if k != long_k)
        if 2 * it.h[long_k] < 150 or it.h[long_k] < 3 * short * 0.8:
            continue
        down = mul(it.ax[v], -1 if it.ax[v][2] > 0 else 1)
        base = add(it.c, mul(down, it.h[v]))
        ends = []
        other = [k for k in hz if k != long_k][0]
        for sgn in (-1, 1):
            along = mul(it.ax[long_k], sgn)
            state = "none"
            reach_len = min(60.0, 0.3 * it.h[long_k])
            d = 5.0
            while d <= reach_len and state != "below":
                for lat in (0.0, 0.6, -0.6, 0.9, -0.9):
                    p = add(add(base, mul(along, it.h[long_k] - d)), add(mul(it.ax[other], lat * it.h[other]), mul(down, 1.0)))
                    if some_item_contains(items, p, it):
                        state = "below"
                        break
                d += 5.0
            if state == "none":
                beyond = add(it.c, mul(along, it.h[long_k] + 1.0))
                if some_item_contains(items, beyond, it):
                    state = "held"
            ends.append((sgn, state))
        out.append((it, ends))
    return out


class Body:
    def __init__(self, label, c, h, lying, foot=None, ref=None, holder=None, held_dist=None):
        self.label, self.c, self.h, self.lying = label, c, h, lying
        self.foot = foot  # z of the soles: the actor's origin for a standing mannequin (bounds are padded)
        self.ref, self.holder, self.held_dist = ref, holder, held_dist
        self.lo = tuple(c[k] - h[k] for k in range(3))
        self.hi = tuple(c[k] + h[k] for k in range(3))
        self.props = []

    def describe(self):
        return "%s at (%.0f, %.0f, %.0f) bounds %.0f x %.0f x %.0f" % ((self.label,) + tuple(self.c) + tuple(2 * v for v in self.h))


def surface_below(items, x, y, z_from, z_to, skip=()):
    """The highest solid z at (x, y) between z_from and z_to (1 cm steps), or None."""
    z = z_from
    while z >= z_to:
        for o in items:
            if o in skip or not (o.lo[0] <= x <= o.hi[0] and o.lo[1] <= y <= o.hi[1] and o.lo[2] <= z <= o.hi[2]):
                continue
            if o.contains((x, y, z)):
                return z, o
        z -= 1.0
    return None


def body_findings(items, bodies):
    """Per body: ('hover'|'sunk'|'unsupported', gap, surface item) and the solids it stands inside."""
    hover, inside, overlap = [], [], []
    for b in bodies:
        if b.holder is not None:
            continue
        foot = b.lo[2] if b.foot is None else b.foot
        probe = surface_below(items, b.c[0], b.c[1], foot + 0.5, foot + 0.5)
        if probe is not None:  # the feet start inside a solid: climb to the free air above it
            z = foot + 0.5
            while z < foot + 400.0 and surface_below(items, b.c[0], b.c[1], z, z) is not None:
                z += 0.5
            if z - foot > 2.0:
                hover.append((b, -(z - foot), probe[1]))
        else:
            found = surface_below(items, b.c[0], b.c[1], foot, foot - 400.0)
            if found is None:
                hover.append((b, None, None))
            elif foot - found[0] > 2.0:
                hover.append((b, foot - found[0], found[1]))
        if b.lying:
            continue
        r = 0.5 * min(b.h[0], b.h[1])
        cols = [(b.c[0] + dx, b.c[1] + dy) for dx, dy in ((0, 0), (r, 0), (-r, 0), (0, r), (0, -r))]
        z = foot + 10.0
        samples = []
        while z <= b.hi[2] - 5.0:
            for (x, y) in cols:
                samples.append((x, y, z))
            z += 12.0
        counts = {}
        for p in samples:
            for o in items:
                if o.hi[2] <= foot + 12.0 or o.kind == "mesh" and is_root(o):
                    continue  # floors and yards: sunk feet are reported above
                if not all(o.lo[k] <= p[k] <= o.hi[k] for k in range(3)):
                    continue
                if o.contains(p):
                    counts.setdefault(o, []).append(p[2])
        for o, zs in counts.items():
            inside.append((len(zs) / float(len(samples)), b, o, min(zs), max(zs)))
    standing = [b for b in bodies if not b.lying and b.holder is None]
    for i in range(len(standing)):
        for j in range(i + 1, len(standing)):
            a, c = standing[i], standing[j]
            d = math.hypot(a.c[0] - c.c[0], a.c[1] - c.c[1])
            if d < 35.0 and a.lo[2] < c.hi[2] and c.lo[2] < a.hi[2]:
                overlap.append((d, a, c))
    return hover, sorted(inside, key=lambda r: -r[0]), overlap


def lerp_keys(keys, t):
    """Linear value of sorted (time, value) keys at time t (clamped at the ends)."""
    if t <= keys[0][0]:
        return keys[0][1]
    for (t0, v0), (t1, v1) in zip(keys, keys[1:]):
        if t0 <= t <= t1:
            return v0 + (v1 - v0) * ((t - t0) / (t1 - t0) if t1 > t0 else 0.0)
    return keys[-1][1]


def walk_findings(items, walkers, period, step=0.25, radius=28.0, height=170.0):
    """Sweep each walker's body column along its keyed path. `walkers` is [(label, {channel: keys})]
    with channels 'x', 'y', 'z' (cm). Returns (label, kind, time, (x, y, z), item, depth) rows:
    'sunk' (feet below the surface they walk on), 'hover' (feet above it) and 'through' (the body
    column inside a solid that is not the floor under it). One row per walker and solid."""
    rows = []
    for label, ch in walkers:
        seen = set()
        t = 0.0
        while t <= period + 1e-6:
            x, y, z = (lerp_keys(ch[k], t) for k in ("x", "y", "z"))
            probe = surface_below(items, x, y, z + 0.5, z + 0.5)
            if probe is not None:
                top = z + 0.5
                while top < z + 400.0 and surface_below(items, x, y, top, top) is not None:
                    top += 0.5
                if top - z > 2.0 and ("sunk", probe[1]) not in seen:
                    seen.add(("sunk", probe[1]))
                    rows.append((label, "sunk", t, (x, y, z), probe[1], top - z))
            else:
                found = surface_below(items, x, y, z, z - 400.0)
                if found is not None and z - found[0] > 2.0 and ("hover", found[1]) not in seen:
                    seen.add(("hover", found[1]))
                    rows.append((label, "hover", t, (x, y, z), found[1], z - found[0]))
            for dx, dy in ((0, 0), (radius, 0), (-radius, 0), (0, radius), (0, -radius)):
                for dz in (30.0, 80.0, 130.0, height - 8.0):
                    p = (x + dx, y + dy, z + dz)
                    for o in items:
                        if o in seen or ("sunk", o) in seen or not all(o.lo[k] <= p[k] <= o.hi[k] for k in range(3)) or not o.contains(p):
                            continue
                        if o.hi[2] <= z + 12.0:
                            continue  # the floor under the feet
                        seen.add(o)
                        rows.append((label, "through", t, (x, y, z), o, 0.0))
            t += step
    return rows


def duplicates(items):
    """Same mesh at one transform, and meshes entirely inside another."""
    same, buried = [], []
    for i in range(len(items)):
        a = items[i]
        for j in range(i + 1, len(items)):
            b = items[j]
            if not aabb_near(a, b, 0.5):
                continue
            if (a.mesh == b.mesh and norm(sub(a.c, b.c)) < 0.5 and
                    all(abs(a.h[k] - b.h[k]) < 0.5 for k in range(3)) and
                    all(abs(abs(dot(a.ax[k], b.ax[k])) - 1.0) < 1e-4 for k in range(3))):
                same.append((a, b))
        if is_root(a):
            continue
        cs = a.corners()
        for o in items:
            if o is a or o.kind not in ("box", "cyl", "sphere", "cone"):
                continue
            if a.h[0] * a.h[1] * a.h[2] >= o.h[0] * o.h[1] * o.h[2]:
                continue
            if all(o.contains(c, 0.1) for c in cs):
                buried.append((a, o))
                break
    return same, buried


# ----------------------------------------------------------------------------------- engine

def classify(mesh_path):
    name = mesh_path.rsplit("/", 1)[-1].split(".")[0]
    return {"SM_Cube": "box", "Cube": "box", "SM_Cylinder": "cyl", "Cylinder": "cyl",
            "Cone": "cone", "SM_Cone": "cone", "Sphere": "sphere", "SM_Sphere": "sphere"}.get(name, "mesh")


def vec(v):
    return (float(v.x), float(v.y), float(v.z))


def collect():
    """(items, bodies, others): static meshes as oriented boxes, mannequins as bounds, and how many
    other actors were skipped."""
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
    items, bodies, byref = [], [], {}
    skipped = 0
    for a in actors:
        label = a.get_actor_label()
        if isinstance(a, unreal.StaticMeshActor):
            comp = a.get_component_by_class(unreal.StaticMeshComponent)
            mesh = comp.get_editor_property("static_mesh") if comp else None
            if mesh is None:
                skipped += 1  # a group hub
                continue
            t = comp.get_world_transform()
            q = (t.rotation.x, t.rotation.y, t.rotation.z, t.rotation.w)
            s = vec(t.scale3d)
            b = mesh.get_bounds()
            o, e = vec(b.origin), vec(b.box_extent)
            ax = tuple(rotate(q, v) for v in ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)))
            c = add(vec(t.translation), rotate(q, (s[0] * o[0], s[1] * o[1], s[2] * o[2])))
            h = (abs(s[0]) * e[0], abs(s[1]) * e[1], abs(s[2]) * e[2])
            par = a.get_attach_parent_actor()
            folder = str(a.get_folder_path())
            it = Item(label, classify(mesh.get_path_name()), c, ax, h, mesh.get_path_name().split(".")[0],
                      parent=par.get_path_name() if par else None, folder=folder, ref=a, quat=q, scale=s)
            it.holder_ref = par if isinstance(par, unreal.SkeletalMeshActor) else None
            items.append(it)
        elif isinstance(a, unreal.SkeletalMeshActor):
            o, e = a.get_actor_bounds(False)
            rot = a.get_actor_rotation()
            lying = abs(rot.roll) > 45.0 or abs(rot.pitch) > 45.0
            body = Body(label, vec(o), vec(e), lying, foot=None if lying else float(a.get_actor_location().z), ref=a)
            bodies.append(body)
            byref[a.get_path_name()] = body
        else:
            skipped += 1
    props = []
    for it in list(items):
        h = getattr(it, "holder_ref", None)
        if h is None:
            continue
        body = byref.get(h.get_path_name())
        if body is not None:
            sock = it.ref.get_attach_parent_socket_name() if hasattr(it.ref, "get_attach_parent_socket_name") else None
            dist = None
            try:
                comp = body.ref.get_component_by_class(unreal.SkeletalMeshComponent)
                loc = comp.get_socket_location(sock) if sock else None
                if loc is not None:
                    dist = norm(sub(it.c, vec(loc)))
            except Exception:
                pass
            body.props.append((it, str(sock), dist))
            props.append(it)
    items = [i for i in items if i not in props]
    return items, bodies, skipped, len(props)


def collect_walkers():
    """[(label, {'x': keys, 'y': keys, 'z': keys})] and the sequence length in seconds, read from the
    transform tracks of every level sequence in the level (keys as (seconds, cm))."""
    out, period = [], 12.0
    actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
    for a in actors:
        if not isinstance(a, unreal.LevelSequenceActor):
            continue
        try:
            seq = a.get_sequence()
            rate = seq.get_display_rate()
            fps = float(rate.numerator) / float(rate.denominator)
            period = (seq.get_playback_end() - seq.get_playback_start()) / fps
            for b in seq.get_bindings():
                chans = {}
                for tr in b.get_tracks():
                    if not isinstance(tr, unreal.MovieScene3DTransformTrack):
                        continue
                    for sec in tr.get_sections():
                        for c in sec.get_all_channels():
                            name = str(c.get_name())
                            for want, key in (("Location.X", "x"), ("Location.Y", "y"), ("Location.Z", "z")):
                                if name == want or name.startswith(want + "_"):
                                    ks = []
                                    for k in c.get_keys():
                                        t = k.get_time()
                                        frame = t.frame_number.value if hasattr(t, "frame_number") else t.value
                                        ks.append((frame / fps, float(k.get_value())))
                                    chans[key] = sorted(ks)
                if len(chans) == 3 and all(chans[k] for k in chans):
                    out.append((str(b.get_name()), chans))
        except Exception as e:  # noqa: BLE001
            unreal.log_warning("[graybox] audit: could not read %s (%s)" % (a.get_actor_label(), e))
    return out, period


# ----------------------------------------------------------------------------------- report

def group_by(rows, key):
    out = {}
    for r in rows:
        out.setdefault(key(r), []).append(r)
    return out


def report(items, bodies, nprops=0, skipped=0, top=12):
    lines = []

    def say(s=""):
        lines.append(s)

    n_floor = sum(1 for i in items if is_root(i))
    say("structural audit: %d static meshes (%d ground slabs), %d mannequins, %d held props, %d other actors skipped"
        % (len(items), n_floor, len(bodies), nprops, skipped))

    # 1 z-fighting
    zf_all = zfight(items)
    zf = [r for r in zf_all if not r[4]]
    grp = group_by(zf, lambda r: tuple(sorted((r[1].stem, r[2].stem))))
    say("")
    say("[1] Z-FIGHTING: %d visible coplanar same-facing overlaps in %d pairs of kinds (%d more are buried under another solid and cannot be seen)"
        % (len(zf), len(grp), len(zf_all) - len(zf)))
    for key, rows in sorted(grp.items(), key=lambda kv: -max(r[0] for r in kv[1]))[:top]:
        area, a, b, n, _ = max(rows, key=lambda r: r[0])
        say("  %s <-> %s  x%d  largest %.0f cm2, facing (%.0f, %.0f, %.0f)" % (key[0], key[1], len(rows), area, n[0], n[1], n[2]))
        say("      %s | %s" % (a.describe(), b.describe()))

    # 2 floating
    iso, clusters = floating(items)
    say("")
    say("[2] FLOATING: %d static meshes touch nothing, %d clusters are not connected to the ground"
        % (len(iso), len(clusters)))
    for it in sorted(iso, key=lambda i: -(i.h[0] * i.h[1] * i.h[2]))[:top]:
        say("  isolated  %s  (lowest point z %.0f)" % (it.describe(), it.lo[2]))
    for comp in sorted(clusters, key=lambda c: -len(c))[:top]:
        low = min(i.lo[2] for i in comp)
        say("  cluster of %d (lowest z %.0f): %s" % (len(comp), low, "; ".join(i.describe() for i in comp[:3])))
    hv = hovering(items)
    say("  hovering: %d items whose level underside hangs above a solid without touching" % len(hv))
    for it, g, below in sorted(hv, key=lambda r: -r[0].h[0] * r[0].h[1])[:top]:
        say("    %s  %.0f cm above %s" % (it.describe(), g, below.label))
    pl = planted(items)
    say("  planted: %d items start 20 cm or more inside another solid" % len(pl))
    for it, depth, o in sorted(pl, key=lambda r: -r[1])[:top]:
        say("    %s  %.0f cm inside %s" % (it.describe(), depth, o.label))
    sp = spans(items)
    bad = [(it, ends) for it, ends in sp if any(s == "none" for _, s in ends)]
    say("  spans: %d long level spans, %d with an end that rests on nothing and is not held" % (len(sp), len(bad)))
    for it, ends in sorted(bad, key=lambda r: -r[0].h[0] * r[0].h[1] * r[0].h[2])[:top]:
        say("    %s  ends: %s" % (it.describe(), ", ".join("%s=%s" % ("-+"[s > 0], st) for s, st in ends)))

    # 3 bodies
    hover, inside, overlap = body_findings(items, bodies)
    say("")
    say("[3] BODIES: %d off their surface, %d inside a solid, %d overlapping each other, %d held props away from the hand"
        % (len(hover), len(inside), len(overlap), sum(1 for b in bodies for p in b.props if p[2] is not None and p[2] > 60.0)))
    for b, gap, o in hover[:top * 2]:
        if gap is None:
            say("  no surface under  %s%s" % (b.describe(), "  (lying)" if b.lying else ""))
        else:
            say("  %s %.1f cm %s %s%s" % ("hovers" if gap > 0 else "sunk", abs(gap),
                                          "above" if gap > 0 else "into", o.label, "  [" + b.describe() + "]" + ("  (lying)" if b.lying else "")))
    for frac, b, o, z0, z1 in inside[:top]:
        say("  %.0f%% of %s is inside %s (z %.0f..%.0f)  [%s]" % (100 * frac, b.label, o.label, z0, z1, o.describe()))
    for d, a, c in overlap[:top]:
        say("  %s and %s stand %.0f cm apart" % (a.label, c.label, d))
    held = [(p[2], it.label, b.label) for b in bodies for it, p in ((q[0], q) for q in b.props) if p[2] is not None]
    if held:
        far = max(held)
        say("  held props: %d, farthest from its hand socket: %s of %s at %.0f cm" % (len(held), far[1], far[2], far[0]))
    for b in bodies:
        for it, sock, dist in b.props:
            if dist is not None and dist > 60.0:
                say("  %s is %.0f cm from %s's %s" % (it.label, dist, b.label, sock))

    walkers, period = collect_walkers() if unreal else ([], 12.0)
    names = set(b.label for b in bodies)
    walkers = [w for w in walkers if w[0] in names]
    wf = walk_findings(items, walkers, period)
    bad_w = group_by(wf, lambda r: r[0])
    say("")
    say("[3b] WALKERS: %d walkers keyed in level sequences, %d with a problem along the path (%.0f s loop)"
        % (len(walkers), len(bad_w), period))
    for label, rows in bad_w.items():
        for (_, kind, t, p, o, depth) in rows[:top]:
            if kind == "through":
                say("  %s walks through %s at t=%.2f s (%.0f, %.0f)  [%s]" % (label, o.label, t, p[0], p[1], o.describe()))
            else:
                say("  %s feet %s %s by %.1f cm at t=%.2f s (%.0f, %.0f, z %.0f)" % (
                    label, "inside" if kind == "sunk" else "above", o.label, depth, t, p[0], p[1], p[2]))

    # 4 duplicates
    same, buried = duplicates(items)
    say("")
    say("[4] DUPLICATES: %d identical meshes at one transform, %d meshes buried inside a bigger one" % (len(same), len(buried)))
    for a, b in same[:top]:
        say("  same  %s = %s" % (a.describe(), b.label))
    g = group_by(buried, lambda r: (r[0].stem, r[1].stem))
    for key, rows in sorted(g.items(), key=lambda kv: -len(kv[1]))[:top]:
        a, o = rows[0]
        say("  buried  %s inside %s  x%d, e.g. %s" % (key[0], key[1], len(rows), a.describe()))
    return lines


def run(write_to=None):
    """Audit the editor world and log the report. Never raises: an audit must not fail a build."""
    log = (lambda m: unreal.log("[graybox] " + m)) if unreal else print
    try:
        world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
        items, bodies, skipped, nprops = collect()
        lines = ["map: " + world.get_path_name()] + report(items, bodies, nprops, skipped)
    except Exception as e:  # noqa: BLE001
        import traceback
        log("audit failed: %s" % e)
        log(traceback.format_exc())
        return None
    for line in lines:
        log("audit| " + line)
    if write_to:
        try:
            with open(write_to, "w", encoding="utf-8") as f:
                f.write("\n".join(lines) + "\n")
        except OSError as e:
            log("audit report not written: %s" % e)
    return lines
