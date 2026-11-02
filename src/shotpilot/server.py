"""Local daemon for the browser UI: a thin HTTP + Server-Sent Events layer over Workspace.

Shape borrowed from open-design: one local process owns the API, the UI and the CLI
drive the same operations, long work streams progress as SSE, the server binds to
loopback and only serves files from inside the project.

shots.yaml and .shotpilot/state.json stay the source of truth: every request opens a
fresh Workspace, so edits made from the CLI or a text editor show up immediately.
Mutations are serialized with one lock; a second mutation while one runs gets 409.
"""

from __future__ import annotations

import json
import mimetypes
import sys
import threading
import traceback
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib import resources
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs, unquote, urlsplit

from . import fingerprint as fpmod
from . import model
from .model import ShotpilotError
from .workspace import Workspace

LOOPBACK = {"127.0.0.1", "localhost", "::1"}
SERVED_DIRS = ("render", "cut")  # the only project subfolders /files/ may read
MAX_BODY = 1 << 20


class Busy(ShotpilotError):
    pass


class App:
    def __init__(self, shots_path: str | Path):
        self.path = Path(shots_path).resolve()
        self.lock = threading.Lock()
        Workspace(self.path)  # fail fast on a broken definition

    def ws(self) -> Workspace:
        return Workspace(self.path)

    def mutate(self):
        if not self.lock.acquire(blocking=False):
            raise Busy("another operation is running; try again when it finishes")
        return _Release(self.lock)


class _Release:
    def __init__(self, lock: threading.Lock):
        self.lock = lock

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.lock.release()


# --------------------------------------------------------------------------- payloads


def file_url(ws: Workspace, path: Path) -> str:
    return "/files/" + path.resolve().relative_to(ws.root.resolve()).as_posix()


def take_thumb(ws: Workspace, take: dict[str, Any] | None) -> str | None:
    """Middle frame of a take, the same frame the storyboard uses."""
    if not take or not take.get("files"):
        return None
    files = take["files"]
    return file_url(ws, Path(take["dir"]) / files[len(files) // 2])


def status_payload(ws: Workspace) -> dict[str, Any]:
    qualities = fpmod.quality_names(ws.defn)
    resolved = {s.id: s for s in ws.resolved()}
    shots = []
    for row in ws.status():
        s = resolved[row.id]
        thumbs: dict[str, dict[str, Any] | None] = {}
        for q in qualities:
            fp = fpmod.render_fingerprint(ws.defn, s, q)
            take = ws.state.find_take(s.id, q, fp)
            fresh = take is not None
            if take is None:
                older = ws.state.list_takes(s.id, q)
                take = older[-1] if older else None
            thumbs[q] = {"url": take_thumb(ws, take), "fresh": fresh, "fingerprint": fp[:12]} if take else \
                {"url": None, "fresh": False, "fingerprint": fp[:12]}
        shots.append({
            **row.__dict__,
            "lens": s.lens.get("focal_length"),
            "thumbs": thumbs,
            "node": model._plain(ws.defn.find(row.id)),
        })
    return {
        "project": ws.defn.project.name,
        "engine": ws.defn.project.engine,
        "title": ws.defn.teaser.title,
        "target_duration": ws.defn.teaser.target_duration,
        "total_duration": round(sum(s["duration"] for s in shots), 3),
        "qualities": qualities,
        "warnings": ws.structure_warnings(),
        "shots": shots,
    }


def cut_payload(ws: Workspace, info: dict[str, Any]) -> dict[str, Any]:
    out_dir = Path(info["dir"])
    return {**info,
            "storyboard": file_url(ws, out_dir / "index.html"),
            "video_url": file_url(ws, Path(info["video"])) if info.get("video") else None}


# --------------------------------------------------------------------------- HTTP


class Handler(BaseHTTPRequestHandler):
    server: "Server"
    server_version = "shotpilot"

    # quiet: the UI polls /api/status; errors still go to stderr
    def log_message(self, format: str, *args: Any) -> None:
        pass

    @property
    def app(self) -> App:
        return self.server.app

    # ---------------------------------------------------------------- plumbing

    def _host_ok(self) -> bool:
        allowed = LOOPBACK | {self.server.bind_host}
        host = _hostname(self.headers.get("Host", ""))
        if host not in allowed:
            return False
        origin = self.headers.get("Origin")
        if origin and origin != "null" and _hostname(urlsplit(origin).netloc) not in allowed:
            return False
        return True

    def _send(self, code: int, body: bytes, ctype: str, extra: dict[str, str] | None = None) -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def _json(self, code: int, data: Any) -> None:
        self._send(code, json.dumps(data, ensure_ascii=False).encode("utf-8"),
                   "application/json; charset=utf-8", {"Cache-Control": "no-store"})

    def _body(self) -> dict[str, Any]:
        n = int(self.headers.get("Content-Length") or 0)
        if n > MAX_BODY:
            raise ShotpilotError("request body too large")
        raw = self.rfile.read(n) if n else b""
        if not raw:
            return {}
        try:
            data = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as e:
            raise ShotpilotError(f"request body is not JSON: {e}") from None
        if not isinstance(data, dict):
            raise ShotpilotError("request body must be a JSON object")
        return data

    def _dispatch(self, routes: dict[str, Any]) -> None:
        if not self._host_ok():
            self._json(403, {"error": "forbidden host or origin"})
            return
        url = urlsplit(self.path)
        try:
            for route, fn in routes.items():
                is_prefix = len(route) > 1 and route.endswith("/")
                if url.path == route or (is_prefix and url.path.startswith(route)):
                    fn(url.path[len(route):] if is_prefix else "", parse_qs(url.query))
                    return
            self._json(404, {"error": f"no route for {self.command} {url.path}"})
        except Busy as e:
            self._json(409, {"error": str(e)})
        except ShotpilotError as e:
            self._json(400, {"error": str(e)})
        except (BrokenPipeError, ConnectionResetError):
            pass
        except Exception as e:  # noqa: BLE001 - report, keep serving
            traceback.print_exc(file=sys.stderr)
            try:
                self._json(500, {"error": f"{type(e).__name__}: {e}"})
            except OSError:
                pass

    def do_GET(self) -> None:
        self._dispatch({
            "/": self.get_index,
            "/api/status": self.get_status,
            "/api/log": self.get_log,
            "/files/": self.get_file,
        })

    def do_HEAD(self) -> None:
        self.do_GET()

    def do_POST(self) -> None:
        self._dispatch({
            "/api/shots/": self.post_shot,
            "/api/order": self.post_order,
            "/api/render": self.post_render,
            "/api/cut": self.post_cut,
        })

    # ---------------------------------------------------------------- GET

    def get_index(self, _rest: str, _q: dict) -> None:
        body = resources.files("shotpilot").joinpath("web/index.html").read_bytes()
        self._send(200, body, "text/html; charset=utf-8", {"Cache-Control": "no-store"})

    def get_status(self, _rest: str, _q: dict) -> None:
        self._json(200, status_payload(self.app.ws()))

    def get_log(self, _rest: str, q: dict) -> None:
        n = int((q.get("n") or ["30"])[0])
        self._json(200, self.app.ws().state.data.get("history", [])[-n:])

    def get_file(self, rest: str, _q: dict) -> None:
        rel = unquote(rest)
        top = rel.split("/", 1)[0]
        root = self.app.path.parent
        if top not in SERVED_DIRS:
            self._json(404, {"error": "not found"})
            return
        base = (root / top).resolve()
        target = (root / rel).resolve()
        if not target.is_relative_to(base) or not target.is_file():
            self._json(404, {"error": "not found"})
            return
        ctype = mimetypes.guess_type(target.name)[0] or "application/octet-stream"
        # takes live under their fingerprint and are never overwritten, so they cache forever
        cache = "public, max-age=31536000, immutable" if top == "render" else "no-cache"
        self._send(200, target.read_bytes(), ctype, {"Cache-Control": cache})

    # ---------------------------------------------------------------- POST

    def post_shot(self, shot: str, _q: dict) -> None:
        body = self._body()
        assignments = body.get("set")
        if not isinstance(assignments, dict) or not assignments:
            raise ShotpilotError('expected {"set": {"field.path": value, ...}}')
        with self.app.mutate():
            ws = self.app.ws()
            sid = ws.select([unquote(shot)])[0].id
            self._json(200, ws.set_fields(sid, assignments))

    def post_order(self, _rest: str, _q: dict) -> None:
        order = self._body().get("order")
        with self.app.mutate():
            ws = self.app.ws()
            ids = ws.defn.shot_ids()
            if not isinstance(order, list) or sorted(order) != sorted(ids):
                raise ShotpilotError(f"order must list every shot exactly once: {', '.join(ids)}")
            self._json(200, ws.edit_definition(order=order, action="order"))

    def post_cut(self, _rest: str, _q: dict) -> None:
        body = self._body()
        with self.app.mutate():
            ws = self.app.ws()
            info = ws.cut(str(body.get("quality") or "best"), video=bool(body.get("video", True)))
            self._json(200, cut_payload(ws, info))

    def post_render(self, _rest: str, _q: dict) -> None:
        body = self._body()
        quality = str(body.get("quality") or "previz")
        ids = body.get("shots") or None
        force = bool(body.get("force", False))
        with self.app.mutate():
            ws = self.app.ws()
            fpmod.quality(ws.defn, quality)  # bad input -> plain 400 before streaming starts
            ws.select(ids)
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache, no-transform")
            self.send_header("X-Accel-Buffering", "no")
            self.end_headers()
            stream = _Stream(self.wfile)
            counts = {"rendered": 0, "reused": 0, "failed": 0}

            def on_progress(event: str, data: dict[str, Any]) -> None:
                if event == "shot":
                    counts[data["action"]] = counts.get(data["action"], 0) + 1
                    take = ws.state.find_take(data["shot_id"], data["quality"], data["fingerprint"])
                    data = {**data, "thumb": take_thumb(ws, take)}
                stream.send(event, data)

            try:
                ws.render(ids, quality, force=force, on_progress=on_progress)
                stream.send("done", {"quality": quality, **counts})
            except ShotpilotError as e:
                stream.send("error", {"error": str(e)})
            except Exception as e:  # noqa: BLE001
                traceback.print_exc(file=sys.stderr)
                stream.send("error", {"error": f"{type(e).__name__}: {e}"})


class _Stream:
    """SSE writer that keeps going if the browser goes away: the render must still finish."""

    def __init__(self, wfile: Any):
        self.wfile = wfile
        self.alive = True

    def send(self, event: str, data: dict[str, Any]) -> None:
        if not self.alive:
            return
        try:
            self.wfile.write(f"event: {event}\ndata: {json.dumps(data, ensure_ascii=False)}\n\n".encode("utf-8"))
            self.wfile.flush()
        except OSError:
            self.alive = False


def _hostname(netloc: str) -> str:
    netloc = netloc.strip().lower()
    if netloc.startswith("["):
        return netloc[1:netloc.find("]")] if "]" in netloc else netloc
    return netloc.rsplit(":", 1)[0] if ":" in netloc else netloc


class Server(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, app: App, host: str, port: int):
        self.app = app
        self.bind_host = host.lower()
        super().__init__((host, port), Handler)

    @property
    def url(self) -> str:
        host, port = self.server_address[:2]
        shown = "127.0.0.1" if host in ("0.0.0.0", "") else host
        return f"http://{shown}:{port}/"


def make_server(shots_path: str | Path, host: str = "127.0.0.1", port: int = 7460) -> Server:
    return Server(App(shots_path), host, port)
