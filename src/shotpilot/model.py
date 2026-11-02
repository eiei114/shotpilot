"""shots.yaml: the definition side of Shotpilot.

The definition is the single source of truth. Everything under render/ and every
engine asset is derived from it, so any shot can be rebuilt from this file alone.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from . import geometry as g
from .yamlio import load_yaml

SCHEMA_VERSION = 1
ID_RE = re.compile(r"^[a-z0-9][a-z0-9_\-]{0,47}$")
PACKAGE_RE = re.compile(r"^/[A-Za-z0-9_]+(/[A-Za-z0-9_\-. ]+)+$")

# Sun elevation in degrees above the horizon. UE DirectionalLight pitch = -elevation.
TIME_OF_DAY = {
    "dawn": 6.0,
    "morning": 25.0,
    "noon": 65.0,
    "afternoon": 35.0,
    "golden_hour": 10.0,
    "dusk": 3.0,
    "night": -18.0,
}

# The teaser shape Shotpilot ships with. Beats are editorial: they never affect pixels.
BEATS = ("hook", "establish", "escalate", "title")
BEAT_GUIDE = {
    "hook": "first 2-5 s: one striking image or question that stops the scroll",
    "establish": "where we are and what kind of game this is",
    "escalate": "tension, stakes or spectacle rising shot by shot",
    "title": "a calm final image with room for the title card",
}

MOVES = {
    "static", "dolly_in", "dolly_out", "truck_left", "truck_right",
    "crane_up", "crane_down", "orbit_left", "orbit_right",
}
CURVES = {"linear", "ease_in", "ease_out", "ease_in_out"}


class ShotpilotError(Exception):
    """User-facing error. The CLI prints the message without a traceback."""


@dataclass
class ProjectCfg:
    name: str
    engine: str = "mock"  # mock | ue5
    uproject: str | None = None
    map: str | None = None
    ue_cmd: str | None = None
    content_root: str = "/Game/Shotpilot"
    sun_actor: str | None = None
    isolate_userdir: bool = True


@dataclass
class TeaserCfg:
    title: str = ""
    pitch: str = ""
    target_duration: float = 45.0
    fps: int = 30
    resolution: tuple[int, int] = (1920, 1080)


@dataclass
class Definition:
    path: Path
    project: ProjectCfg
    teaser: TeaserCfg
    anchors: dict[str, g.Vec3]
    defaults: dict[str, Any]
    qualities: dict[str, dict[str, Any]]
    shots: list[dict[str, Any]]

    @property
    def root(self) -> Path:
        return self.path.parent

    def shot_ids(self) -> list[str]:
        return [s["id"] for s in self.shots]

    def find(self, shot_id: str) -> dict[str, Any]:
        for s in self.shots:
            if s["id"] == shot_id:
                return s
        raise ShotpilotError(f"unknown shot '{shot_id}'. known: {', '.join(self.shot_ids())}")


@dataclass
class ResolvedShot:
    id: str
    index: int
    purpose: str
    duration: float
    fps: int
    frames: int
    start: g.Xform
    end: g.Xform
    curve: str
    lens: dict[str, Any]
    environment: dict[str, Any]
    markers: list[dict[str, Any]] = field(default_factory=list)
    target: g.Vec3 | None = None
    notes: str = ""
    beat: str | None = None

    def camera_at(self, frame: int) -> g.Xform:
        if self.frames <= 1:
            return self.start
        t = g.ease(frame / (self.frames - 1), self.curve)
        return g.interpolate(self.start, self.end, t)

    def engine_payload(self) -> dict[str, Any]:
        """Everything that gets written into the engine's shot sequence."""
        return {
            "frames": self.frames,
            "fps": self.fps,
            "curve": self.curve,
            "start": self.start.to_dict(),
            "end": self.end.to_dict(),
            "lens": self.lens,
            "environment": self.environment,
            "markers": self.markers,
        }


# --------------------------------------------------------------------------- loading


def load(path: str | Path) -> Definition:
    path = Path(path)
    if not path.exists():
        raise ShotpilotError(f"{path} not found. run `shotpilot init` first.")
    data = load_yaml(path)
    return parse(data, path)


def parse(data: Any, path: Path) -> Definition:
    if not isinstance(data, dict):
        raise ShotpilotError(f"{path}: top level must be a mapping")
    errs: list[str] = []
    version = data.get("version", SCHEMA_VERSION)
    if version != SCHEMA_VERSION:
        errs.append(f"version: expected {SCHEMA_VERSION}, got {version}")

    p = dict(data.get("project") or {})
    project = ProjectCfg(
        name=str(p.get("name") or path.parent.name),
        engine=str(p.get("engine", "mock")),
        uproject=p.get("uproject"),
        map=p.get("map"),
        ue_cmd=p.get("ue_cmd"),
        content_root=str(p.get("content_root", "/Game/Shotpilot")).rstrip("/"),
        sun_actor=p.get("sun_actor"),
        isolate_userdir=bool(p.get("isolate_userdir", True)),
    )
    if project.engine not in ("mock", "ue5"):
        errs.append(f"project.engine: must be 'mock' or 'ue5', got '{project.engine}'")
    if project.engine == "ue5":
        if not project.uproject:
            errs.append("project.uproject: required when engine is ue5")
        if not project.map:
            errs.append("project.map: required when engine is ue5 (e.g. /Game/Maps/Courtyard)")
    if project.map and not PACKAGE_RE.match(str(project.map)):
        errs.append(
            f"project.map: must be a UE package path like /Game/Maps/Courtyard (got '{project.map}'). "
            "Git Bash rewrites /Game/... arguments into Windows paths; set MSYS_NO_PATHCONV=1 or edit shots.yaml")

    t = dict(data.get("teaser") or {})
    res = t.get("resolution", [1920, 1080])
    teaser = TeaserCfg(
        title=str(t.get("title", "")),
        pitch=str(t.get("pitch", "")),
        target_duration=float(t.get("target_duration", 45)),
        fps=int(t.get("fps", 30)),
        resolution=(int(res[0]), int(res[1])),
    )
    if teaser.fps <= 0 or teaser.fps > 120:
        errs.append("teaser.fps: must be 1..120")

    anchors: dict[str, g.Vec3] = {}
    for name, v in dict(data.get("anchors") or {}).items():
        vec = _vec3(v)
        if vec is None:
            errs.append(f"anchors.{name}: must be [x, y, z]")
        else:
            anchors[str(name)] = vec

    defaults = _plain(data.get("defaults") or {})
    qualities = _plain(data.get("qualities") or {})
    shots_raw = data.get("shots") or []
    if not isinstance(shots_raw, list):
        errs.append("shots: must be a list")
        shots_raw = []
    shots = [_plain(s) for s in shots_raw]

    seen: set[str] = set()
    for i, s in enumerate(shots):
        where = f"shots[{i}]"
        if not isinstance(s, dict):
            errs.append(f"{where}: must be a mapping")
            continue
        sid = s.get("id")
        if not isinstance(sid, str) or not ID_RE.match(sid):
            errs.append(f"{where}.id: lowercase letters/digits/_/- (got {sid!r})")
        elif sid in seen:
            errs.append(f"{where}.id: duplicate '{sid}'")
        else:
            seen.add(sid)
        dur = s.get("duration")
        if not isinstance(dur, (int, float)) or dur <= 0:
            errs.append(f"{where}.duration: positive seconds required")
        if s.get("beat") is not None and s.get("beat") not in BEATS:
            errs.append(f"{where}.beat: one of {', '.join(BEATS)} (got {s.get('beat')!r})")

    defn = Definition(path, project, teaser, anchors, defaults, qualities, shots)
    if not errs:
        for i, s in enumerate(shots):
            try:
                resolve_shot(defn, s, i)
            except ShotpilotError as e:
                errs.append(str(e))
    if errs:
        raise ShotpilotError(f"{path} has {len(errs)} problem(s):\n  - " + "\n  - ".join(errs))
    return defn


def _plain(v: Any) -> Any:
    """ruamel CommentedMap/Seq -> plain dict/list."""
    if isinstance(v, dict):
        return {str(k): _plain(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [_plain(x) for x in v]
    return v


def _vec3(v: Any) -> g.Vec3 | None:
    if isinstance(v, (list, tuple)) and len(v) == 3 and all(isinstance(x, (int, float)) for x in v):
        return (float(v[0]), float(v[1]), float(v[2]))
    return None


# --------------------------------------------------------------------------- resolution


def resolve_all(defn: Definition) -> list[ResolvedShot]:
    return [resolve_shot(defn, s, i) for i, s in enumerate(defn.shots)]


def resolve_shot(defn: Definition, shot: dict[str, Any], index: int | None = None) -> ResolvedShot:
    if index is None:
        index = defn.shot_ids().index(shot["id"])
    where = f"shots[{index}]({shot.get('id')})"
    cam = dict(shot.get("camera") or {})
    fps = int(shot.get("fps", defn.teaser.fps))
    duration = float(shot["duration"])
    frames = max(1, round(duration * fps))

    target = _point(defn, cam.get("look_at"), f"{where}.camera.look_at")
    roll = float(cam.get("roll", 0.0))

    if "anchor" in cam:
        anchor = _point(defn, cam["anchor"], f"{where}.camera.anchor")
        off = _vec3(cam.get("offset", [-600, 0, 150]))
        if off is None:
            raise ShotpilotError(f"{where}.camera.offset: must be [x, y, z]")
        eye = g.add(anchor, off)  # type: ignore[arg-type]
        if target is None:
            target = anchor
    elif "location" in cam:
        eye = _vec3(cam["location"])
        if eye is None:
            raise ShotpilotError(f"{where}.camera.location: must be [x, y, z]")
    else:
        raise ShotpilotError(f"{where}.camera: needs either `location` or `anchor`")

    if target is not None:
        start_rot = g.look_at(eye, target, roll)
    elif "rotation" in cam:
        r = _vec3(cam["rotation"])
        if r is None:
            raise ShotpilotError(f"{where}.camera.rotation: must be [pitch, yaw, roll]")
        start_rot = g.Rot(*r)
    else:
        raise ShotpilotError(f"{where}.camera: needs `rotation`, `look_at` or `anchor`")
    start = g.Xform(eye, start_rot)

    curve = "ease_in_out"
    if "end" in cam:
        end_cfg = dict(cam["end"])
        e_loc = _vec3(end_cfg.get("location", list(eye)))
        if e_loc is None:
            raise ShotpilotError(f"{where}.camera.end.location: must be [x, y, z]")
        e_target = _point(defn, end_cfg.get("look_at"), f"{where}.camera.end.look_at") or target
        if e_target is not None and "rotation" not in end_cfg:
            e_rot = g.look_at(e_loc, e_target, float(end_cfg.get("roll", roll)))
        else:
            r = _vec3(end_cfg.get("rotation", start_rot.as_list()))
            if r is None:
                raise ShotpilotError(f"{where}.camera.end.rotation: must be [pitch, yaw, roll]")
            e_rot = g.Rot(*r)
        end = g.Xform(e_loc, e_rot)
        curve = str(end_cfg.get("curve", curve))
    else:
        move = dict(cam.get("move") or {"type": "static"})
        curve = str(move.get("curve", curve))
        end = _apply_move(start, move, target, f"{where}.camera.move")
    if curve not in CURVES:
        raise ShotpilotError(f"{where}: curve must be one of {sorted(CURVES)}")
    # Unwrap end angles relative to start so a plain numeric interpolation (Sequencer
    # keys) takes the short way round, exactly like the previz does.
    end = g.Xform(end.location, g.Rot(*(
        s + ((e - s + 180.0) % 360.0 - 180.0) for s, e in zip(start.rotation.as_list(), end.rotation.as_list()))))

    lens = {"focal_length": 35.0, "aperture": 2.8, "sensor_width": 36.0}
    lens.update(defn.defaults.get("lens") or {})
    lens.update(cam.get("lens") or {})
    fl = lens.get("focal_length")
    if not isinstance(fl, (int, float)) or not (4 <= fl <= 1200):
        raise ShotpilotError(f"{where}.camera.lens.focal_length: 4..1200 mm")
    if lens.get("focus_distance") is None and target is not None:
        lens["focus_distance"] = round(g.length(g.sub(target, eye)), 1)
    lens = {k: (round(float(v), 3) if isinstance(v, (int, float)) else v) for k, v in lens.items()}

    env = _resolve_env(defn, shot, where)

    markers = []
    for m in shot.get("markers") or []:
        at = float(m.get("at", 0))
        if not 0 <= at <= duration:
            raise ShotpilotError(f"{where}.markers: at={at} outside 0..{duration}")
        markers.append({"frame": min(frames - 1, round(at * fps)), "label": str(m.get("label", "marker"))})

    return ResolvedShot(
        id=shot["id"], index=index, purpose=str(shot.get("purpose", "")),
        duration=duration, fps=fps, frames=frames,
        start=_round_xf(start), end=_round_xf(end), curve=curve,
        lens=lens, environment=env, markers=markers, target=target,
        notes=str(shot.get("notes", "")), beat=shot.get("beat"),
    )


def _round_xf(x: g.Xform) -> g.Xform:
    return g.Xform(tuple(g._r(v) for v in x.location), g.Rot(*(g._r(v) for v in x.rotation.as_list())))  # type: ignore[arg-type]


def _point(defn: Definition, v: Any, where: str) -> g.Vec3 | None:
    if v is None:
        return None
    if isinstance(v, str):
        if v not in defn.anchors:
            known = ", ".join(defn.anchors) or "(none; run `shotpilot scan` or add `anchors:`)"
            raise ShotpilotError(f"{where}: unknown anchor '{v}'. known: {known}")
        return defn.anchors[v]
    vec = _vec3(v)
    if vec is None:
        raise ShotpilotError(f"{where}: anchor name or [x, y, z]")
    return vec


def _apply_move(start: g.Xform, move: dict[str, Any], target: g.Vec3 | None, where: str) -> g.Xform:
    kind = str(move.get("type", "static"))
    if kind not in MOVES:
        raise ShotpilotError(f"{where}.type: one of {sorted(MOVES)}")
    if kind == "static":
        return start
    dist = float(move.get("distance", 150.0))
    eye = start.location
    if kind in ("dolly_in", "dolly_out"):
        d = g.forward(start.rotation)
        eye = g.add(eye, g.scale(d, dist if kind == "dolly_in" else -dist))
    elif kind in ("truck_left", "truck_right"):
        d = g.right(start.rotation)
        eye = g.add(eye, g.scale(d, dist if kind == "truck_right" else -dist))
    elif kind in ("crane_up", "crane_down"):
        eye = g.add(eye, (0.0, 0.0, dist if kind == "crane_up" else -dist))
    else:  # orbit
        deg = float(move.get("degrees", 30.0))
        pivot = target or g.add(eye, g.scale(g.forward(start.rotation), 500.0))
        eye = g.orbit(eye, pivot, deg if kind == "orbit_left" else -deg)
        return g.Xform(eye, g.look_at(eye, pivot, start.rotation.roll))
    keep_aim = bool(move.get("keep_aim", target is not None))
    if keep_aim and target is not None and kind != "dolly_in" and kind != "dolly_out":
        return g.Xform(eye, g.look_at(eye, target, start.rotation.roll))
    return g.Xform(eye, start.rotation)


def _resolve_env(defn: Definition, shot: dict[str, Any], where: str) -> dict[str, Any]:
    env: dict[str, Any] = {"time_of_day": None, "sun_yaw": 45.0}
    env.update(defn.defaults.get("environment") or {})
    env.update(shot.get("environment") or {})
    tod = env.get("time_of_day")
    if tod is not None and tod not in TIME_OF_DAY:
        raise ShotpilotError(f"{where}.environment.time_of_day: one of {sorted(TIME_OF_DAY)}")
    if env.get("sun_elevation") is None and tod is not None:
        env["sun_elevation"] = TIME_OF_DAY[tod]
    out = {k: v for k, v in env.items() if v is not None}
    return dict(sorted(out.items()))


def structure_warnings(shots: list[ResolvedShot], target_duration: float) -> list[str]:
    """Advisory checks against the hook -> establish -> escalate -> title shape."""
    if not shots:
        return []
    out = []
    beats = [s.beat for s in shots]
    if not any(beats):
        return ["no shot has a `beat`; tag shots with hook / establish / escalate / title to check the teaser shape"]
    untagged = [s.id for s in shots if not s.beat]
    if untagged:
        out.append(f"no beat on: {', '.join(untagged)}")
    for b in BEATS:
        if b not in beats:
            out.append(f"no `{b}` shot ({BEAT_GUIDE[b]})")
    if beats[0] and beats[0] != "hook":
        out.append(f"the cut opens on `{beats[0]}`; teasers usually open on the hook")
    if beats[-1] and beats[-1] != "title":
        out.append(f"the cut ends on `{beats[-1]}`; end on the title beat")
    ranks = [BEATS.index(b) for b in beats if b]
    if any(b < a for a, b in zip(ranks, ranks[1:])):
        out.append("beats go backwards (e.g. establish after escalate); intentional?")
    total = sum(s.duration for s in shots)
    hook = sum(s.duration for s in shots if s.beat == "hook")
    if hook > 6:
        out.append(f"hook runs {hook:g}s; keep it under ~5s")
    if target_duration and abs(total - target_duration) > 0.1 * target_duration:
        out.append(f"runtime {total:g}s is more than 10% off the {target_duration:g}s target")
    return out
