"""Engine backends.

Both backends speak the same job/result JSON protocol. `ue5` hands the job to
ue/sp_entry.py inside Unreal; `mock` executes it in-process so the whole loop
(definition -> build -> render -> pull -> cut) runs and is testable without UE.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any

from . import previz
from .model import Definition, ResolvedShot, ShotpilotError, resolve_all

ENTRY_SCRIPT = Path(__file__).parent / "ue" / "sp_entry.py"


def shot_asset(defn: Definition, shot_id: str) -> str:
    return f"{defn.project.content_root}/Shots/SEQ_{shot_id}"


def master_asset(defn: Definition) -> str:
    return f"{defn.project.content_root}/SEQ_Teaser"


def readback_subset(payload: dict[str, Any]) -> dict[str, Any]:
    lens = payload.get("lens", {})
    return {
        "frames": payload["frames"], "fps": payload["fps"],
        "start": payload["start"], "end": payload["end"],
        "lens": {k: lens[k] for k in ("focal_length", "aperture", "focus_distance") if k in lens},
    }


class Engine:
    name = "base"

    def __init__(self, defn: Definition):
        self.defn = defn
        self.root = defn.root
        self.jobs_dir = self.root / ".shotpilot" / "jobs"
        self.jobs_dir.mkdir(parents=True, exist_ok=True)

    def _job_file(self, kind: str, job: dict[str, Any]) -> Path:
        stamp = time.strftime("%Y%m%d-%H%M%S") + f"-{int(time.time() * 1000) % 1000:03d}"
        p = self.jobs_dir / f"{stamp}-{kind}.json"
        job = {"kind": kind, "project": self._project_dict(), **job}
        p.write_text(json.dumps(job, indent=2, ensure_ascii=False), encoding="utf-8")
        return p

    def _project_dict(self) -> dict[str, Any]:
        p = self.defn.project
        return {"name": p.name, "map": p.map, "content_root": p.content_root, "sun_actor": p.sun_actor}

    # interface -------------------------------------------------------------
    def build(self, items: list[dict], master: dict | None, force: bool) -> dict[str, Any]:
        raise NotImplementedError

    def render(self, build_items: list[dict], master: dict | None, render_items: list[dict], force: bool) -> dict:
        raise NotImplementedError

    def pull(self, items: list[dict]) -> dict[str, Any]:
        raise NotImplementedError

    def scan(self, tag: str, prefix: str) -> dict[str, Any]:
        raise NotImplementedError


# --------------------------------------------------------------------------- mock


class MockEngine(Engine):
    """Stands in for UE: 'assets' are JSON files, 'renders' are tinted previz frames."""

    name = "mock"

    def __init__(self, defn: Definition):
        super().__init__(defn)
        self.store = self.root / ".shotpilot" / "mock_engine"
        self.store.mkdir(parents=True, exist_ok=True)

    def _asset_file(self, asset: str) -> Path:
        return self.store / (asset.strip("/").replace("/", "__") + ".json")

    def _launch(self, kind: str, **detail: Any) -> None:
        with open(self.store / "launches.jsonl", "a", encoding="utf-8") as f:
            f.write(json.dumps({"kind": kind, **detail}, ensure_ascii=False) + "\n")

    def edit_asset(self, shot_id: str, mutate) -> None:
        """Simulate a human editing the shot sequence in Sequencer (tests / demo)."""
        p = self._asset_file(shot_asset(self.defn, shot_id))
        data = json.loads(p.read_text(encoding="utf-8"))
        mutate(data)
        p.write_text(json.dumps(data, indent=2), encoding="utf-8")

    def _build(self, items: list[dict], master: dict | None, force: bool) -> dict[str, Any]:
        from .sync import same_payload
        out: dict[str, Any] = {"shots": {}}
        for it in items:
            p = self._asset_file(it["asset"])
            if p.exists() and it.get("base") is not None and not force:
                current = json.loads(p.read_text(encoding="utf-8"))
                if not same_payload(readback_subset(current), it["base"]):
                    out["shots"][it["id"]] = {"status": "diverged", "engine": readback_subset(current)}
                    continue
            p.write_text(json.dumps(it["payload"], indent=2), encoding="utf-8")
            out["shots"][it["id"]] = {"status": "built", "engine": readback_subset(it["payload"])}
        if master:
            self._asset_file(master["asset"]).write_text(json.dumps(master, indent=2), encoding="utf-8")
            out["master"] = {"asset": master["asset"], "frames": sum(s["frames"] for s in master["shots"])}
        return out

    def build(self, items, master, force):
        self._launch("build", shots=[i["id"] for i in items])
        return self._build(items, master, force)

    def render(self, build_items, master, render_items, force):
        self._launch("render", shots=[i["key"] for i in render_items], built=[i["id"] for i in build_items])
        res: dict[str, Any] = {"build": self._build(build_items, master, force) if build_items or master else None,
                               "renders": {}}
        blocked = {k for k, v in ((res["build"] or {}).get("shots") or {}).items() if v["status"] != "built"}
        resolved = {s.id: s for s in resolve_all(self.defn)}
        t0 = time.time()
        for it in render_items:
            if it["id"] in blocked:
                res["renders"][it["key"]] = {"status": "error", "error": "build failed or diverged"}
                continue
            p = self._asset_file(it["asset"])
            if not p.exists():
                res["renders"][it["key"]] = {"status": "error", "error": "sequence asset missing; build first"}
                continue
            shot = _shot_from_payload(resolved[it["id"]], json.loads(p.read_text(encoding="utf-8")))
            w, h = it["resolution"]
            # cap mock resolution: it's a stand-in, not a renderer
            k = min(1.0, 480 / w)
            previz.render_shot(shot, it["frames"], self.defn.anchors, Path(it["out_dir"]),
                               max(2, int(w * k)), max(2, int(h * k)), tint=(255, 170, 60))
            res["renders"][it["key"]] = {"status": "ok"}
        res["seconds"] = round(time.time() - t0, 2)
        return res

    def pull(self, items):
        self._launch("pull", shots=[i["id"] for i in items])
        out: dict[str, Any] = {"shots": {}}
        for it in items:
            p = self._asset_file(it["asset"])
            out["shots"][it["id"]] = readback_subset(json.loads(p.read_text(encoding="utf-8"))) if p.exists() else None
        return out

    def scan(self, tag, prefix):
        self._launch("scan")
        level = self.store / "level.json"
        if level.exists():
            return {"anchors": json.loads(level.read_text(encoding="utf-8"))}
        return {"anchors": {k: list(v) for k, v in self.defn.anchors.items()}}


def _shot_from_payload(base: ResolvedShot, payload: dict[str, Any]) -> ResolvedShot:
    """Rebuild a ResolvedShot from what is actually stored in the engine asset."""
    from . import geometry as g

    def xf(d):
        return g.Xform(tuple(d["location"]), g.Rot(*d["rotation"]))

    return ResolvedShot(
        id=base.id, index=base.index, purpose=base.purpose, duration=payload["frames"] / payload["fps"],
        fps=payload["fps"], frames=payload["frames"], start=xf(payload["start"]), end=xf(payload["end"]),
        curve=payload.get("curve", base.curve), lens={**base.lens, **payload.get("lens", {})},
        environment=payload.get("environment", base.environment), markers=payload.get("markers", []),
        target=base.target,
    )


# --------------------------------------------------------------------------- UE5


class UE5Engine(Engine):
    name = "ue5"

    def __init__(self, defn: Definition):
        super().__init__(defn)
        self.uproject = Path(defn.project.uproject or "")
        if not self.uproject.is_absolute():
            self.uproject = (defn.root / self.uproject).resolve()
        if not self.uproject.exists():
            raise ShotpilotError(f"project.uproject not found: {self.uproject}")
        self.cmd = self._find_cmd()
        self.logs = self.root / ".shotpilot" / "logs"
        self.logs.mkdir(parents=True, exist_ok=True)
        self.timeout = int(os.environ.get("SHOTPILOT_UE_TIMEOUT", "3600"))

    def _find_cmd(self) -> str:
        cand = self.defn.project.ue_cmd or os.environ.get("SHOTPILOT_UE_CMD")
        if cand:
            if not Path(cand).exists():
                raise ShotpilotError(f"UnrealEditor-Cmd not found at {cand}")
            return cand
        try:
            assoc = json.loads(self.uproject.read_text(encoding="utf-8")).get("EngineAssociation", "")
        except (OSError, json.JSONDecodeError):
            assoc = ""
        for base in (Path(r"C:\Program Files\Epic Games"), Path(r"D:\Epic Games")):
            p = base / f"UE_{assoc}" / "Engine" / "Binaries" / "Win64" / "UnrealEditor-Cmd.exe"
            if assoc and p.exists():
                return str(p)
        found = shutil.which("UnrealEditor-Cmd")
        if found:
            return found
        raise ShotpilotError(
            f"could not locate UnrealEditor-Cmd for EngineAssociation '{assoc}'. "
            "set project.ue_cmd in shots.yaml or SHOTPILOT_UE_CMD")

    def _common(self) -> list[str]:
        args = ["-unattended", "-nosplash", "-nop4", "-stdout", "-FullStdOutLogOutput", "-NoSound"]
        if self.defn.project.isolate_userdir:
            args.append(f"-UserDir={self.root / '.shotpilot' / 'ue_userdir'}")
        return args

    def _run(self, kind: str, job: dict[str, Any]) -> dict[str, Any]:
        job_file = self._job_file(kind, job)
        result_file = job_file.with_name(job_file.stem + ".result.json")
        if kind == "render":
            argv = [self.cmd, str(self.uproject), self.defn.project.map or "",
                    f"-ExecutePythonScript={ENTRY_SCRIPT}", "-windowed", "-ResX=1280", "-ResY=720",
                    "-NoLoadingScreen", *self._common()]
        else:
            argv = [self.cmd, str(self.uproject), "-run=pythonscript", f"-script={ENTRY_SCRIPT}",
                    "-NullRHI", *self._common()]
        env = {**os.environ, "SHOTPILOT_JOB": str(job_file)}
        log_file = self.logs / (job_file.stem + ".log")
        t0 = time.time()
        with open(log_file, "w", encoding="utf-8", errors="replace") as lf:
            lf.write(" ".join(f'"{a}"' if " " in a else a for a in argv) + "\n\n")
            lf.flush()
            try:
                proc = subprocess.run(argv, env=env, stdout=lf, stderr=subprocess.STDOUT, timeout=self.timeout)
            except subprocess.TimeoutExpired:
                raise ShotpilotError(f"UE {kind} timed out after {self.timeout}s. log: {log_file}")
        if not result_file.exists():
            raise ShotpilotError(
                f"UE {kind} exited ({proc.returncode}) without a result after {time.time() - t0:.0f}s. log: {log_file}")
        res = json.loads(result_file.read_text(encoding="utf-8"))
        if res.get("fatal"):
            raise ShotpilotError(f"UE {kind} failed:\n{res['fatal']}\nlog: {log_file}")
        res["log"] = str(log_file)
        return res

    def build(self, items, master, force):
        return self._run("build", {"build": items, "master": master, "force": force})

    def render(self, build_items, master, render_items, force):
        return self._run("render", {"build": build_items, "master": master, "render": render_items, "force": force})

    def pull(self, items):
        return self._run("pull", {"pull": items})

    def scan(self, tag, prefix):
        return self._run("scan", {"tag": tag, "prefix": prefix})


def make_engine(defn: Definition) -> Engine:
    if defn.project.engine == "ue5":
        return UE5Engine(defn)
    return MockEngine(defn)
