"""Fingerprints decide what is stale.

A shot's render fingerprint covers only what changes pixels for that shot. Order,
purpose and notes are deliberately excluded, so reordering the cut or rewording a
note never triggers a re-render.
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from .model import Definition, ResolvedShot

RENDERER_VERSION = 1  # bump when the engine-side scripts change output for the same input

BUILTIN_QUALITIES: dict[str, dict[str, Any]] = {
    # Local wireframe previz. No engine launch at all: seconds per shot.
    "previz": {"runner": "previz", "scale": 1 / 6, "frame_step": 3, "format": "png"},
    # Engine, three frames per shot (first/mid/last) to check framing and light.
    "still": {"runner": "engine", "scale": 0.5, "frames": "keys", "spatial_samples": 1,
              "temporal_samples": 1, "screen_percentage": 50, "warmup_frames": 8, "format": "jpg"},
    # Engine, every frame at half res, no AA accumulation.
    "preview": {"runner": "engine", "scale": 0.5, "frames": "all", "spatial_samples": 1,
                "temporal_samples": 1, "screen_percentage": 50, "warmup_frames": 8, "format": "jpg"},
    # Delivery frames.
    "final": {"runner": "engine", "scale": 1.0, "frames": "all", "spatial_samples": 1,
              "temporal_samples": 8, "screen_percentage": 100, "warmup_frames": 32, "format": "png"},
}


def quality(defn: Definition, name: str) -> dict[str, Any]:
    base = dict(BUILTIN_QUALITIES.get(name, {}))
    base.update(defn.qualities.get(name, {}))
    if not base:
        known = sorted(set(BUILTIN_QUALITIES) | set(defn.qualities))
        from .model import ShotpilotError
        raise ShotpilotError(f"unknown quality '{name}'. known: {', '.join(known)}")
    base.setdefault("runner", "engine")
    base.setdefault("frames", "all")
    base.setdefault("format", "png")
    base.setdefault("scale", 1.0)
    return base


def quality_names(defn: Definition) -> list[str]:
    order = list(BUILTIN_QUALITIES)
    return order + [q for q in defn.qualities if q not in order]


def resolution(defn: Definition, q: dict[str, Any]) -> tuple[int, int]:
    w, h = defn.teaser.resolution
    s = float(q.get("scale", 1.0))
    # even numbers keep video encoders happy
    return (max(2, int(round(w * s / 2)) * 2), max(2, int(round(h * s / 2)) * 2))


def frame_list(shot: ResolvedShot, q: dict[str, Any]) -> list[int]:
    n = shot.frames
    mode = q.get("frames", "all")
    if mode == "keys":
        return sorted({0, (n - 1) // 2, n - 1})
    if mode == "mid":
        return [(n - 1) // 2]
    step = max(1, int(q.get("frame_step", 1)))
    frames = list(range(0, n, step))
    if frames[-1] != n - 1:
        frames.append(n - 1)
    return frames


def _digest(obj: Any) -> str:
    blob = json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(blob.encode("utf-8")).hexdigest()


def build_fingerprint(defn: Definition, shot: ResolvedShot) -> str:
    """What the engine's shot sequence must contain."""
    return _digest({
        "v": RENDERER_VERSION,
        "map": defn.project.map,
        "content_root": defn.project.content_root,
        "sun_actor": defn.project.sun_actor,
        "shot": shot.engine_payload(),
    })


def render_fingerprint(defn: Definition, shot: ResolvedShot, quality_name: str) -> str:
    q = quality(defn, quality_name)
    payload = shot.engine_payload()
    payload.pop("markers", None)  # markers are editorial metadata, not pixels
    body: dict[str, Any] = {
        "v": RENDERER_VERSION,
        "shot": payload,
        "quality": q,
        "resolution": resolution(defn, q),
    }
    if q["runner"] == "previz":
        body["anchors"] = {k: list(v) for k, v in sorted(defn.anchors.items())}
    else:
        body["map"] = defn.project.map
        body["sun_actor"] = defn.project.sun_actor
        body["engine"] = defn.project.engine
    return _digest(body)
