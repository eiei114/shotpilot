"""The browser UI's HTTP layer: same partial-fix loop, driven over HTTP + SSE."""

import http.client
import json
import threading

import pytest

from shotpilot.server import make_server

from .conftest import launches


@pytest.fixture
def srv(ws):
    s = make_server(ws.path, "127.0.0.1", 0)
    t = threading.Thread(target=s.serve_forever, daemon=True)
    t.start()
    yield s
    s.shutdown()
    s.server_close()


def call(srv, method, path, body=None, headers=None):
    host, port = srv.server_address[:2]
    c = http.client.HTTPConnection(host, port, timeout=60)
    data = json.dumps(body).encode() if body is not None else None
    c.request(method, path, body=data, headers={"Content-Type": "application/json", **(headers or {})})
    r = c.getresponse()
    raw = r.read()
    c.close()
    return r.status, r.getheader("Content-Type", ""), raw


def jcall(srv, method, path, body=None, headers=None):
    code, _, raw = call(srv, method, path, body, headers)
    return code, json.loads(raw)


def sse(raw: bytes) -> list[tuple[str, dict]]:
    events = []
    for block in raw.decode("utf-8").split("\n\n"):
        if not block.strip():
            continue
        lines = dict(line.split(": ", 1) for line in block.splitlines())
        events.append((lines["event"], json.loads(lines["data"])))
    return events


def render(srv, quality="previz", **extra):
    code, ctype, raw = call(srv, "POST", "/api/render", {"quality": quality, **extra})
    assert code == 200 and ctype.startswith("text/event-stream")
    events = sse(raw)
    assert events[-1][0] == "done", events[-1]
    return {d["shot_id"]: d["action"] for e, d in events if e == "shot"}, events


def test_index_and_status(srv):
    code, ctype, raw = call(srv, "GET", "/")
    assert code == 200 and ctype.startswith("text/html") and b"shotpilot" in raw
    code, st = jcall(srv, "GET", "/api/status")
    assert code == 200
    assert len(st["shots"]) == 6 and st["shots"][2]["id"] == "s03_window"
    assert st["shots"][0]["qualities"]["previz"] == "none"
    assert st["shots"][0]["node"]["id"] == "s01_gate"


def test_http_only_changed_shot_rerenders(srv):
    first, events = render(srv)
    assert set(first.values()) == {"rendered"}
    assert events[0][0] == "plan" and len(events[0][1]["render"]) == len(first)
    assert all(d["thumb"].startswith("/files/render/") for e, d in events if e == "shot")

    code, rep = jcall(srv, "POST", "/api/shots/s03_window", {"set": {"camera.lens.focal_length": 85}})
    assert code == 200 and rep["stale"] == ["s03_window"]
    st = jcall(srv, "GET", "/api/status")[1]
    assert {s["id"]: s["qualities"]["previz"] for s in st["shots"]}["s03_window"] == "stale"

    second, events = render(srv)
    assert second.pop("s03_window") == "rendered"
    assert set(second.values()) == {"reused"}
    assert events[-1][1] == {"quality": "previz", "rendered": 1, "reused": len(second), "failed": 0}


def test_http_engine_render_launches_only_changed_shot(srv, ws):
    render(srv, "preview")
    code, rep = jcall(srv, "POST", "/api/shots/3", {"set": {"duration": 3}})  # "3" = third shot
    assert code == 200 and rep["stale"] == ["s03_window"]
    second, _ = render(srv, "preview")
    assert second.pop("s03_window") == "rendered" and set(second.values()) == {"reused"}
    last = [x for x in launches(ws) if x["kind"] == "render"][-1]
    assert last["shots"] == ["s03_window:preview"]


def test_reorder_over_http_never_invalidates(srv, ws):
    render(srv)
    ids = [s.id for s in ws.resolved()]
    code, rep = jcall(srv, "POST", "/api/order", {"order": ids[::-1]})
    assert code == 200 and rep["stale"] == []
    again, _ = render(srv)
    assert set(again.values()) == {"reused"}


def test_thumbnails_are_served_and_immutable(srv):
    _, events = render(srv)
    url = next(d["thumb"] for e, d in events if e == "shot")
    host, port = srv.server_address[:2]
    c = http.client.HTTPConnection(host, port, timeout=10)
    c.request("GET", url)
    r = c.getresponse()
    assert r.status == 200 and r.getheader("Content-Type") == "image/png"
    assert "immutable" in r.getheader("Cache-Control")
    assert r.read()[:8] == b"\x89PNG\r\n\x1a\n"


@pytest.mark.parametrize("path", ["/files/../shots.yaml", "/files/render/../shots.yaml",
                                  "/files/.shotpilot/state.json", "/files/render/%2e%2e/shots.yaml"])
def test_files_stay_inside_project_outputs(srv, path):
    code, _, _ = call(srv, "GET", path)
    assert code == 404


def test_foreign_host_and_origin_are_rejected(srv):
    assert call(srv, "GET", "/api/status", headers={"Host": "evil.example"})[0] == 403
    code, _, _ = call(srv, "POST", "/api/shots/s01_gate", {"set": {"duration": 9}},
                      headers={"Origin": "http://evil.example"})
    assert code == 403


def test_user_errors_are_400_with_message(srv):
    code, body = jcall(srv, "POST", "/api/shots/nope", {"set": {"duration": 2}})
    assert code == 400 and "unknown shot" in body["error"]
    code, body = jcall(srv, "POST", "/api/render", {"quality": "ultra"})
    assert code == 400 and "unknown quality" in body["error"]
    code, body = jcall(srv, "POST", "/api/order", {"order": ["s01_gate"]})
    assert code == 400


def test_cli_edits_show_up_without_restart(srv, ws):
    render(srv)
    ws.set_fields("s02_well", {"duration": 2})  # as if from another terminal
    st = jcall(srv, "GET", "/api/status")[1]
    assert {s["id"]: s["qualities"]["previz"] for s in st["shots"]}["s02_well"] == "stale"
