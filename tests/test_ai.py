"""Claude integration, with a fake client: scope enforcement and validation are code, not prompt."""

import json
from types import SimpleNamespace

import pytest

from shotpilot import ai
from shotpilot.model import ShotpilotError


class FakeClient:
    def __init__(self, payload):
        self.payload = payload
        self.calls = []
        self.beta = SimpleNamespace(messages=SimpleNamespace(stream=self._stream))

    def _stream(self, **kw):
        self.calls.append(kw)
        msg = SimpleNamespace(stop_reason="end_turn",
                              content=[SimpleNamespace(type="text", text=json.dumps(self.payload))])

        class Ctx:
            def __enter__(self_inner):
                return SimpleNamespace(get_final_message=lambda: msg)

            def __exit__(self_inner, *a):
                return False
        return Ctx()


S03_CLOSER = """id: s03_window
purpose: 2階の窓を見上げる。人影が一瞬だけ映る
duration: 4
camera:
  anchor: window
  offset: [-400, -300, -280]
  move: {type: crane_up, distance: 80}
  lens: {focal_length: 85}
environment: {time_of_day: night}
"""


def test_revise_edits_only_named_shot(ws):
    fake = FakeClient({"summary": "closer", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}],
                       "order": []})
    edits, order, _ = ai.revise(ws.defn, "3番目はもっと寄って", client=fake)
    assert list(edits) == ["s03_window"] and order is None
    rep = ws.edit_definition(edits, order)
    assert rep["stale"] == ["s03_window"]
    req = fake.calls[0]
    assert req["model"] == "claude-opus-5-5"
    assert req["output_config"]["format"]["type"] == "json_schema"


def test_revise_scope_is_enforced(ws):
    fake = FakeClient({"summary": "x", "edits": [{"id": "s03_window", "yaml": S03_CLOSER, "reason": "r"}],
                       "order": []})
    with pytest.raises(ShotpilotError, match="outside the allowed scope"):
        ai.revise(ws.defn, "note", scope=["s01_gate"], client=fake)


def test_revise_rejects_invalid_shot(ws):
    bad = S03_CLOSER.replace("anchor: window", "anchor: moon")
    fake = FakeClient({"summary": "x", "edits": [{"id": "s03_window", "yaml": bad, "reason": "r"}], "order": []})
    with pytest.raises(ShotpilotError, match="unknown anchor"):
        ai.revise(ws.defn, "note", client=fake)


def test_revise_reorder_must_be_permutation(ws):
    fake = FakeClient({"summary": "x", "edits": [], "order": ["s06_title", "s01_gate"]})
    with pytest.raises(ShotpilotError, match="permutation"):
        ai.revise(ws.defn, "タイトルを先に", client=fake)


def test_plan_builds_valid_shots(ws):
    shot = {"id": "S01 Hook!", "beat": "hook", "purpose": "p", "duration": 4, "anchor": "well", "offset": [-300, 0, 120],
            "look_at": "", "focal_length": 35, "move": "orbit_left", "move_amount": 20,
            "time_of_day": "night", "notes": ""}
    fake = FakeClient({"rationale": "r", "shots": [shot]})
    shots, _ = ai.plan(ws.defn, pitch="test", client=fake)
    assert shots[0]["id"] == "s01_hook"
    assert shots[0]["beat"] == "hook"
    assert shots[0]["camera"]["move"] == {"type": "orbit_left", "degrees": 20.0}
    assert shots[0]["environment"] == {"time_of_day": "night"}
