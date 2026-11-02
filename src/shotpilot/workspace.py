"""Workspace: ties definition, state, engine and outputs together.

Every operation is incremental by construction: it computes fingerprints from the
current definition, compares them with what exists, and only does the missing work.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable

from . import fingerprint as fpmod
from . import model, previz, sync
from .engines import Engine, make_engine, master_asset, readback_subset, shot_asset
from .model import Definition, ResolvedShot, ShotpilotError
from .state import State
from .yamlio import dump_yaml, load_yaml, to_rt

IMAGE_EXTS = (".png", ".jpg", ".jpeg", ".exr", ".bmp")
QUALITY_RANK = ["final", "preview", "still", "previz"]

Progress = Callable[[str, dict[str, Any]], None]


@dataclass
class RenderOutcome:
    shot_id: str
    quality: str
    action: str  # reused | rendered | failed
    fingerprint: str
    seconds: float = 0.0
    detail: str = ""


@dataclass
class ShotStatus:
    index: int
    id: str
    duration: float
    purpose: str
    beat: str | None
    build: str  # synced | dirty | new
    qualities: dict[str, str] = field(default_factory=dict)  # fresh | stale | none


class Workspace:
    def __init__(self, shots_path: str | Path = "shots.yaml"):
        self.path = Path(shots_path).resolve()
        self.defn: Definition = model.load(self.path)
        self.root = self.defn.root
        self.state = State(self.root)
        self._engine: Engine | None = None

    @property
    def engine(self) -> Engine:
        if self._engine is None:
            self._engine = make_engine(self.defn)
        return self._engine

    def reload(self) -> None:
        self.defn = model.load(self.path)
        self._engine = None

    def resolved(self) -> list[ResolvedShot]:
        return model.resolve_all(self.defn)

    def structure_warnings(self) -> list[str]:
        return model.structure_warnings(self.resolved(), self.defn.teaser.target_duration)

    def select(self, ids: list[str] | None) -> list[ResolvedShot]:
        shots = self.resolved()
        if not ids:
            return shots
        by_id = {s.id: s for s in shots}
        out = []
        for i in ids:
            if i.isdigit() and 1 <= int(i) <= len(shots):  # allow "3" = third shot
                out.append(shots[int(i) - 1])
            elif i in by_id:
                out.append(by_id[i])
            else:
                raise ShotpilotError(f"unknown shot '{i}'. known: {', '.join(by_id)}")
        return out

    # ------------------------------------------------------------------ status

    def status(self) -> list[ShotStatus]:
        rows = []
        for s in self.resolved():
            pushed = self.state.pushed(s.id)
            bfp = fpmod.build_fingerprint(self.defn, s)
            build = "new" if pushed is None else ("synced" if pushed["fingerprint"] == bfp else "dirty")
            row = ShotStatus(s.index, s.id, s.duration, s.purpose, s.beat, build)
            for q in fpmod.quality_names(self.defn):
                rfp = fpmod.render_fingerprint(self.defn, s, q)
                if self.state.find_take(s.id, q, rfp):
                    row.qualities[q] = "fresh"
                elif self.state.list_takes(s.id, q):
                    row.qualities[q] = "stale"
                else:
                    row.qualities[q] = "none"
            rows.append(row)
        return rows

    # ------------------------------------------------------------------ build

    def _master(self, shots: list[ResolvedShot]) -> dict[str, Any]:
        return {"asset": master_asset(self.defn), "fps": self.defn.teaser.fps,
                "shots": [{"id": s.id, "asset": shot_asset(self.defn, s.id), "frames": s.frames} for s in shots]}

    def _master_fp(self, master: dict[str, Any]) -> str:
        return fpmod._digest(master)

    def _build_items(self, shots: list[ResolvedShot], force: bool) -> list[dict[str, Any]]:
        items = []
        for s in shots:
            bfp = fpmod.build_fingerprint(self.defn, s)
            pushed = self.state.pushed(s.id)
            if not force and pushed and pushed["fingerprint"] == bfp:
                continue
            items.append({
                "id": s.id, "asset": shot_asset(self.defn, s.id), "fingerprint": bfp,
                "payload": s.engine_payload(),
                "base": readback_subset(pushed["payload"]) if pushed else None,
            })
        return items

    def _master_if_changed(self, force: bool) -> dict[str, Any] | None:
        master = self._master(self.resolved())
        if force or self.state.data.get("master_fp") != self._master_fp(master):
            return master
        return None

    def _absorb_build(self, items: list[dict[str, Any]], result: dict[str, Any] | None,
                      master: dict[str, Any] | None) -> dict[str, list[str]]:
        report: dict[str, list[str]] = {"built": [], "diverged": [], "error": []}
        shots = (result or {}).get("shots", {})
        for it in items:
            r = shots.get(it["id"], {"status": "error", "error": "no result"})
            report.setdefault(r["status"], []).append(it["id"])
            if r["status"] == "built":
                self.state.set_pushed(it["id"], it["fingerprint"], it["payload"])
        if master and result and result.get("master") and not result["master"].get("error"):
            self.state.data["master_fp"] = self._master_fp(master)
        if report["built"] or report["diverged"]:
            self.state.log("build", built=report["built"], diverged=report["diverged"])
        return report

    def build(self, ids: list[str] | None = None, force: bool = False) -> dict[str, list[str]]:
        shots = self.select(ids)
        items = self._build_items(shots, force)
        master = self._master_if_changed(force)
        if not items and not master:
            return {"built": [], "diverged": [], "error": [], "skipped": [s.id for s in shots]}
        result = self.engine.build(items, master, force)
        report = self._absorb_build(items, result, master)
        self.state.save()
        return report

    # ------------------------------------------------------------------ render

    def render(self, ids: list[str] | None, quality: str, force: bool = False,
               on_progress: Progress | None = None) -> list[RenderOutcome]:
        """on_progress(event, data) is told the plan ("plan") and each finished shot ("shot")."""
        emit: Progress = on_progress or (lambda event, data: None)
        q = fpmod.quality(self.defn, quality)
        shots = self.select(ids)
        outcomes: list[RenderOutcome] = []
        todo: list[tuple[ResolvedShot, str]] = []
        for s in shots:
            rfp = fpmod.render_fingerprint(self.defn, s, quality)
            take = None if force else self.state.find_take(s.id, quality, rfp)
            if take:
                self.state.mark_current(s.id, quality, rfp)
                outcomes.append(RenderOutcome(s.id, quality, "reused", rfp, detail=str(take["dir"])))
            else:
                todo.append((s, rfp))

        emit("plan", {"quality": quality, "render": [s.id for s, _ in todo],
                      "reused": [o.shot_id for o in outcomes]})
        for o in outcomes:
            emit("shot", asdict(o))

        if todo and q["runner"] == "previz":
            outcomes += self._render_previz(todo, quality, q, emit)
        elif todo:
            engine_outcomes = self._render_engine(todo, quality, q, force)
            for o in engine_outcomes:  # one engine launch renders the whole batch
                emit("shot", asdict(o))
            outcomes += engine_outcomes

        if any(o.action != "reused" for o in outcomes):
            self.state.log("render", quality=quality,
                           rendered=[o.shot_id for o in outcomes if o.action == "rendered"],
                           reused=[o.shot_id for o in outcomes if o.action == "reused"],
                           failed=[o.shot_id for o in outcomes if o.action == "failed"])
        self.state.save()
        order = {s.id: i for i, s in enumerate(shots)}
        return sorted(outcomes, key=lambda o: order[o.shot_id])

    def _take_info(self, s: ResolvedShot, quality: str, q: dict[str, Any], frames: list[int],
                   files: list[str], seconds: float, engine: str) -> dict[str, Any]:
        return {"complete": True, "engine": engine, "frames": frames, "files": files,
                "fps": s.fps, "shot_frames": s.frames, "resolution": list(fpmod.resolution(self.defn, q)),
                "seconds": round(seconds, 2), "payload": s.engine_payload(), "quality_preset": q}

    def _render_previz(self, todo, quality, q, emit: Progress) -> list[RenderOutcome]:
        out = []
        w, h = fpmod.resolution(self.defn, q)
        ground_z = float((self.defn.defaults.get("previz") or {}).get("ground_z", 0.0))
        for s, rfp in todo:
            t0 = time.time()
            d = self.state.take_dir(s.id, quality, rfp)
            if d.exists():
                shutil.rmtree(d)  # forced re-render: drop old frames and cached segments
            frames = fpmod.frame_list(s, q)
            files = previz.render_shot(s, frames, self.defn.anchors, d, w, h, ground_z)
            dt = time.time() - t0
            self.state.write_take(s.id, quality, rfp, self._take_info(s, quality, q, frames, files, dt, "previz"))
            out.append(RenderOutcome(s.id, quality, "rendered", rfp, dt, str(d)))
            emit("shot", asdict(out[-1]))
        return out

    def _render_engine(self, todo, quality, q, force) -> list[RenderOutcome]:
        shots = [s for s, _ in todo]
        build_items = self._build_items(shots, force=False)
        master = self._master_if_changed(False)
        res = fpmod.resolution(self.defn, q)
        items = []
        for s, rfp in todo:
            d = self.state.take_dir(s.id, quality, rfp)
            if d.exists():
                shutil.rmtree(d)  # incomplete leftovers from a failed attempt
            d.mkdir(parents=True)
            items.append({"key": f"{s.id}:{quality}", "id": s.id, "asset": shot_asset(self.defn, s.id),
                          "quality_name": quality, "quality": q, "resolution": list(res),
                          "frames": fpmod.frame_list(s, q), "out_dir": str(d)})
        t0 = time.time()
        result = self.engine.render(build_items, master, items, force)
        build_report = self._absorb_build(build_items, result.get("build"), master)
        total = time.time() - t0
        out = []
        for (s, rfp), it in zip(todo, items):
            r = (result.get("renders") or {}).get(it["key"], {"status": "error", "error": "no result"})
            d = Path(it["out_dir"])
            if r["status"] != "ok":
                why = r.get("error", "")
                if s.id in build_report.get("diverged", []):
                    why = "sequence was edited in the editor; run `shotpilot pull` first (or build --force)"
                out.append(RenderOutcome(s.id, quality, "failed", rfp, detail=why))
                continue
            try:
                files = _normalize_frames(d, it["frames"])
            except ShotpilotError as e:
                out.append(RenderOutcome(s.id, quality, "failed", rfp, detail=str(e)))
                continue
            share = total / max(1, len(items))
            self.state.write_take(s.id, quality, rfp,
                                  self._take_info(s, quality, q, it["frames"], files, share, self.engine.name))
            out.append(RenderOutcome(s.id, quality, "rendered", rfp, share, str(d)))
        return out

    # ------------------------------------------------------------------ pull

    def pull(self, prefer: str | None = None, dry_run: bool = False) -> list[sync.PullDecision]:
        """Bring Sequencer edits back into shots.yaml. prefer: None | 'engine' | 'yaml' for conflicts."""
        shots = [s for s in self.resolved() if self.state.pushed(s.id)]
        if not shots:
            return []
        result = self.engine.pull([{"id": s.id, "asset": shot_asset(self.defn, s.id)} for s in shots])
        decisions = []
        for s in shots:
            pushed = self.state.pushed(s.id)
            decisions.append(sync.decide(s.id, result["shots"].get(s.id), pushed["payload"], s))
        if dry_run:
            return decisions

        take_engine = [d for d in decisions if d.kind == "engine_edit" or (d.kind == "conflict" and prefer == "engine")]
        if take_engine:
            doc = load_yaml(self.path)
            nodes = {n["id"]: n for n in doc["shots"]}
            by_id = {s.id: s for s in shots}
            for d in take_engine:
                sync.apply_engine_to_shot(nodes[d.shot_id], d.engine, by_id[d.shot_id])
            dump_yaml(doc, self.path)
            self.reload()

        for d in decisions:
            if d in take_engine or d.kind == "agree":
                # yaml now resolves to what the engine holds: that is the new base
                s = model.resolve_shot(self.defn, self.defn.find(d.shot_id))
                self.state.set_pushed(s.id, fpmod.build_fingerprint(self.defn, s), s.engine_payload())
            elif d.kind == "conflict" and prefer == "yaml" and d.engine:
                # accept the engine asset as the base so the next build overwrites it with yaml
                pushed = self.state.pushed(d.shot_id)
                self.state.set_pushed(d.shot_id, "superseded", {**pushed["payload"], **d.engine})

        conflicts = [d.shot_id for d in decisions if d.kind == "conflict"]
        if take_engine or conflicts:
            self.state.log("pull", pulled=[d.shot_id for d in take_engine], conflicts=conflicts, prefer=prefer)
        self.state.save()
        return decisions

    # ------------------------------------------------------------------ scan

    def scan(self, tag: str = "shotpilot", prefix: str = "SP_") -> dict[str, list[str]]:
        found = self.engine.scan(tag, prefix).get("anchors", {})
        doc = load_yaml(self.path)
        anchors = doc.get("anchors")
        if anchors is None:
            doc["anchors"] = to_rt({})
            anchors = doc["anchors"]
        report: dict[str, list[str]] = {"added": [], "moved": [], "same": []}
        for name, pos in sorted(found.items()):
            new = to_rt([round(float(v), 1) for v in pos])
            if name not in anchors:
                report["added"].append(name)
                anchors[name] = new
            elif [float(v) for v in anchors[name]] != [float(v) for v in pos]:
                report["moved"].append(name)
                anchors[name] = new
            else:
                report["same"].append(name)
        if report["added"] or report["moved"]:
            dump_yaml(doc, self.path)
            self.reload()
            self.state.log("scan", **{k: v for k, v in report.items() if k != "same"})
            self.state.save()
        return report

    # ------------------------------------------------------------------ cut / export

    def pick_takes(self, quality: str) -> list[tuple[ResolvedShot, dict[str, Any] | None, str | None]]:
        """Per shot in definition order: (shot, take, quality used). quality='best' mixes."""
        out = []
        for s in self.resolved():
            ladder = QUALITY_RANK if quality == "best" else [quality]
            chosen = (None, None)
            for qn in ladder:
                t = self.state.find_take(s.id, qn, fpmod.render_fingerprint(self.defn, s, qn))
                if t:
                    chosen = (t, qn)
                    break
            out.append((s, chosen[0], chosen[1]))
        return out

    def cut(self, quality: str = "best", video: bool = True) -> dict[str, Any]:
        from .cut import write_cut
        picks = self.pick_takes(quality)
        missing = [s.id for s, t, _ in picks if t is None]
        if missing:
            hint = "previz" if quality == "best" else quality
            raise ShotpilotError(
                f"no fresh {quality} take for: {', '.join(missing)}. run `shotpilot render {hint}` first")
        out_dir = self.root / "cut" / quality
        info = write_cut(self.defn, picks, out_dir, video=video)
        self.state.log("cut", quality=quality, order=[s.id for s, _, _ in picks],
                       takes={s.id: f"{qn}:{t['fingerprint'][:12]}" for s, t, qn in picks})
        self.state.save()
        return info

    # ------------------------------------------------------------------ definition edits

    def edit_definition(self, edits: dict[str, dict[str, Any]] | None = None, order: list[str] | None = None,
                        action: str = "edit", note: str = "") -> dict[str, list[str]]:
        """Replace whole shots and/or reorder, then report which renders went stale.

        Untouched shots keep their nodes (and comments) as they are in the file.
        """
        before = {s.id: {q: fpmod.render_fingerprint(self.defn, s, q) for q in fpmod.quality_names(self.defn)}
                  for s in self.resolved()}
        doc = load_yaml(self.path)
        seq = doc["shots"]
        for i, node in enumerate(list(seq)):
            if edits and node["id"] in edits:
                seq[i] = to_rt(edits[node["id"]])
        if order:
            by_id = {n["id"]: n for n in seq}
            seq[:] = [by_id[i] for i in order]
        # validate before writing anything
        model.parse(_plain_doc(doc), self.path)
        dump_yaml(doc, self.path)
        self.reload()
        stale = []
        for s in self.resolved():
            if any(fpmod.render_fingerprint(self.defn, s, q) != fp for q, fp in before.get(s.id, {}).items()):
                stale.append(s.id)
        self.state.log(action, edited=sorted(edits or {}), reordered=bool(order), stale=stale, note=note)
        self.state.save()
        return {"edited": sorted(edits or {}), "stale": stale, "order": order or []}

    def set_fields(self, shot_id: str, assignments: dict[str, Any]) -> dict[str, list[str]]:
        node = dict(self.defn.find(shot_id))
        node = json.loads(json.dumps(node))  # deep copy
        for path, value in assignments.items():
            cur = node
            parts = path.split(".")
            for p in parts[:-1]:
                if not isinstance(cur.get(p), dict):
                    cur[p] = {}
                cur = cur[p]
            if value is None:
                cur.pop(parts[-1], None)
            else:
                cur[parts[-1]] = value
        # switching camera form: an explicit location drops the anchor form and vice versa
        cam = node.get("camera", {})
        if "location" in assignments.get("camera", {}) or "camera.location" in assignments:
            for k in ("anchor", "offset"):
                cam.pop(k, None)
        return self.edit_definition({shot_id: node}, action="set", note=", ".join(assignments))

    # ------------------------------------------------------------------ gc

    def gc(self, keep: int = 2, dry_run: bool = False) -> list[Path]:
        """Delete old takes, keeping the current one plus `keep` most recent per shot/quality."""
        removed = []
        live = {s.id for s in self.resolved()}
        for shot_dir in (self.state.render_dir.iterdir() if self.state.render_dir.exists() else []):
            for qdir in (shot_dir.iterdir() if shot_dir.is_dir() else []):
                takes = self.state.list_takes(shot_dir.name, qdir.name)
                cur = self.state.current(shot_dir.name, qdir.name)
                old = [t for t in takes if t["fingerprint"] != cur]
                victims = old if shot_dir.name not in live else old[:-keep] if keep else old
                for t in victims:
                    removed.append(t["dir"])
                    if not dry_run:
                        shutil.rmtree(t["dir"])
        if removed and not dry_run:
            self.state.log("gc", removed=len(removed))
            self.state.save()
        return removed


def _plain_doc(doc: Any) -> Any:
    return model._plain(doc)


def _normalize_frames(d: Path, frames: list[int]) -> list[str]:
    """Engines name files their own way; rename to frame.NNNN.ext matching the planned frame list."""
    files = sorted(p for p in d.iterdir() if p.suffix.lower() in IMAGE_EXTS)
    if len(files) != len(frames):
        raise ShotpilotError(f"expected {len(frames)} frames in {d}, found {len(files)}")
    names = []
    tmp = []
    for p, f in zip(files, frames):
        t = p.with_name(f"__tmp_{f:04d}{p.suffix.lower()}")
        p.rename(t)
        tmp.append((t, f))
    for t, f in tmp:
        final = t.with_name(f"frame.{f:04d}{t.suffix}")
        t.rename(final)
        names.append(final.name)
    return names


def ffmpeg_available() -> str | None:
    return shutil.which("ffmpeg")


def run_ffmpeg(args: list[str]) -> None:
    exe = ffmpeg_available()
    if not exe:
        raise ShotpilotError("ffmpeg not found on PATH")
    p = subprocess.run([exe, "-hide_banner", "-loglevel", "error", "-y", *args], capture_output=True, text=True)
    if p.returncode != 0:
        raise ShotpilotError(f"ffmpeg failed: {p.stderr.strip()[:2000]}")


def take_summary(t: dict[str, Any]) -> str:
    return json.dumps({k: t[k] for k in ("quality", "fingerprint", "created") if k in t})
