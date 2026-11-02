"""State: what was pushed to the engine, which takes exist, and what changed when.

Layout (relative to the folder holding shots.yaml):

  .shotpilot/state.json            pushed snapshots (base for pull), current takes, history
  .shotpilot/jobs/                 job/result JSON exchanged with the engine
  render/<shot>/<quality>/<fp12>/  one immutable take per fingerprint, with take.json

Takes are content-addressed by fingerprint and never overwritten. Reverting a shot's
definition therefore reuses the old take instead of rendering again.
"""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any

STATE_VERSION = 1


def now_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%S%z")


class State:
    def __init__(self, root: Path):
        self.root = root
        self.dir = root / ".shotpilot"
        self.path = self.dir / "state.json"
        self.render_dir = root / "render"
        if self.path.exists():
            self.data = json.loads(self.path.read_text(encoding="utf-8"))
        else:
            self.data = {"version": STATE_VERSION, "shots": {}, "history": []}

    # ----------------------------------------------------------------- persistence

    def save(self) -> None:
        self.dir.mkdir(parents=True, exist_ok=True)
        tmp = self.path.with_suffix(".tmp")
        tmp.write_text(json.dumps(self.data, indent=2, ensure_ascii=False), encoding="utf-8")
        tmp.replace(self.path)

    def _shot(self, shot_id: str) -> dict[str, Any]:
        return self.data["shots"].setdefault(shot_id, {"current": {}})

    # ----------------------------------------------------------------- takes

    def take_dir(self, shot_id: str, quality: str, fp: str) -> Path:
        return self.render_dir / shot_id / quality / fp[:12]

    def find_take(self, shot_id: str, quality: str, fp: str) -> dict[str, Any] | None:
        d = self.take_dir(shot_id, quality, fp)
        meta = d / "take.json"
        if not meta.exists():
            return None
        try:
            info = json.loads(meta.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            return None
        if info.get("fingerprint") != fp or not info.get("complete"):
            return None
        info["dir"] = d
        return info

    def write_take(self, shot_id: str, quality: str, fp: str, info: dict[str, Any]) -> Path:
        d = self.take_dir(shot_id, quality, fp)
        d.mkdir(parents=True, exist_ok=True)
        meta = {"shot": shot_id, "quality": quality, "fingerprint": fp, "created": now_iso(), **info}
        (d / "take.json").write_text(json.dumps(meta, indent=2, ensure_ascii=False), encoding="utf-8")
        self._shot(shot_id)["current"][quality] = fp
        return d

    def mark_current(self, shot_id: str, quality: str, fp: str) -> None:
        self._shot(shot_id)["current"][quality] = fp

    def current(self, shot_id: str, quality: str) -> str | None:
        return self.data["shots"].get(shot_id, {}).get("current", {}).get(quality)

    def list_takes(self, shot_id: str, quality: str) -> list[dict[str, Any]]:
        base = self.render_dir / shot_id / quality
        out = []
        if base.exists():
            for d in base.iterdir():
                meta = d / "take.json"
                if meta.exists():
                    info = json.loads(meta.read_text(encoding="utf-8"))
                    info["dir"] = d
                    out.append(info)
        return sorted(out, key=lambda x: x.get("created", ""))

    # ----------------------------------------------------------------- engine sync

    def pushed(self, shot_id: str) -> dict[str, Any] | None:
        return self.data["shots"].get(shot_id, {}).get("pushed")

    def set_pushed(self, shot_id: str, fp: str, payload: dict[str, Any]) -> None:
        self._shot(shot_id)["pushed"] = {"fingerprint": fp, "payload": payload, "at": now_iso()}

    def forget(self, shot_id: str) -> None:
        self.data["shots"].pop(shot_id, None)

    # ----------------------------------------------------------------- history

    def log(self, action: str, **detail: Any) -> None:
        self.data["history"].append({"at": now_iso(), "action": action, **detail})
        # keep the file small; the take.json files keep the long-term record
        self.data["history"] = self.data["history"][-500:]
