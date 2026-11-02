"""Reverse sync: bring edits made in Sequencer back into shots.yaml without losing them.

Three-way comparison per shot:
  base   = what Shotpilot last pushed (state.json)
  engine = what the sequence asset holds now
  ours   = what shots.yaml resolves to now

  engine == base               -> nobody touched the asset; nothing to pull
  engine != base, ours == base -> human edited in the editor: write it into shots.yaml
  engine != base, ours == engine -> both sides agree already; just record it
  engine != base, ours != base -> conflict: both changed. user picks with --prefer
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from .engines import readback_subset
from .model import ResolvedShot
from .state import now_iso
from .yamlio import to_rt

TOL_LOC, TOL_ROT, TOL_LENS = 0.5, 0.05, 0.01


def _angle_diff(a: float, b: float) -> float:
    return abs((a - b + 180.0) % 360.0 - 180.0)


def same_payload(a: dict[str, Any] | None, b: dict[str, Any] | None) -> bool:
    if a is None or b is None:
        return a is b
    if int(a.get("frames", -1)) != int(b.get("frames", -2)):
        return False
    for k in ("start", "end"):
        if k not in a or k not in b:
            return False
        for i in range(3):
            if abs(float(a[k]["location"][i]) - float(b[k]["location"][i])) > TOL_LOC:
                return False
            if _angle_diff(float(a[k]["rotation"][i]), float(b[k]["rotation"][i])) > TOL_ROT:
                return False
    la, lb = a.get("lens", {}), b.get("lens", {})
    for k in ("focal_length", "aperture"):
        if abs(float(la.get(k, 0)) - float(lb.get(k, 0))) > TOL_LENS:
            return False
    return True


def describe_diff(old: dict[str, Any], new: dict[str, Any]) -> list[str]:
    out = []
    if old.get("frames") != new.get("frames"):
        out.append(f"frames {old.get('frames')} -> {new.get('frames')}")
    for k in ("start", "end"):
        for part, tol in (("location", TOL_LOC), ("rotation", TOL_ROT)):
            a, b = old.get(k, {}).get(part), new.get(k, {}).get(part)
            if a and b and any(abs(float(x) - float(y)) > tol for x, y in zip(a, b)):
                out.append(f"{k}.{part} {_fmt(a)} -> {_fmt(b)}")
    for k in ("focal_length", "aperture"):
        a, b = old.get("lens", {}).get(k), new.get("lens", {}).get(k)
        if a is not None and b is not None and abs(float(a) - float(b)) > TOL_LENS:
            out.append(f"lens.{k} {a:g} -> {b:g}")
    return out


def _fmt(v: list[float]) -> str:
    return "[" + ", ".join(f"{float(x):.1f}" for x in v) + "]"


@dataclass
class PullDecision:
    shot_id: str
    kind: str  # clean | engine_edit | agree | conflict | missing | never_pushed
    engine: dict[str, Any] | None
    changes: list[str]


def decide(shot_id: str, engine: dict[str, Any] | None, base: dict[str, Any] | None,
           ours: ResolvedShot) -> PullDecision:
    ours_rb = readback_subset(ours.engine_payload())
    if engine is None:
        return PullDecision(shot_id, "missing", None, [])
    if base is None:
        return PullDecision(shot_id, "never_pushed", engine, [])
    base_rb = readback_subset(base)
    if same_payload(engine, base_rb):
        return PullDecision(shot_id, "clean", engine, [])
    changes = describe_diff(base_rb, engine)
    if same_payload(ours_rb, base_rb):
        return PullDecision(shot_id, "engine_edit", engine, changes)
    if same_payload(ours_rb, engine):
        return PullDecision(shot_id, "agree", engine, changes)
    return PullDecision(shot_id, "conflict", engine, changes)


def apply_engine_to_shot(node: Any, engine: dict[str, Any], ours: ResolvedShot) -> None:
    """Rewrite one shot node (ruamel map) so it resolves to exactly what the engine holds.

    High-level camera intent (anchor/offset/move/look_at) can't express an arbitrary
    hand-placed camera, so the camera becomes explicit start/end transforms. The old
    intent is kept in a comment so it's not silently lost.
    """
    cam = node.get("camera") or {}
    was = {k: cam[k] for k in ("anchor", "offset", "look_at", "move") if k in cam}
    lens_old = dict(cam.get("lens") or {})

    def r(v: list[float], n: int = 2) -> list[float]:
        return [round(float(x), n) for x in v]

    new_cam: dict[str, Any] = {
        "location": r(engine["start"]["location"]),
        "rotation": r(engine["start"]["rotation"], 3),
    }
    moved = not same_payload({**engine, "end": engine["start"]}, engine)
    if moved:
        new_cam["end"] = {"location": r(engine["end"]["location"]), "rotation": r(engine["end"]["rotation"], 3),
                          "curve": ours.curve}
    lens = dict(lens_old)
    elens = engine.get("lens", {})
    for k in ("focal_length", "aperture"):
        if k in elens and abs(float(elens[k]) - float(ours.lens.get(k, 0))) > TOL_LENS:
            lens[k] = round(float(elens[k]), 2)
    if "focus_distance" in elens and (was or "focus_distance" in lens_old):
        lens["focus_distance"] = round(float(elens["focus_distance"]), 1)
    if lens:
        new_cam["lens"] = lens
    if "roll" in cam:
        pass  # roll now lives in rotation[2]
    node["camera"] = to_rt(new_cam)
    if was:
        summary = ", ".join(f"{k}={v}" for k, v in was.items())
        node.yaml_add_eol_comment(f"pulled from engine {now_iso()[:16]}; was {summary}", "camera")
    if int(engine["frames"]) != ours.frames:
        node["duration"] = round(int(engine["frames"]) / float(engine.get("fps") or ours.fps), 3)
