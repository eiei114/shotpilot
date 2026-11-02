"""The D-13 partial-fix loop: per-shot re-render, definition/output separation,
state, cheap previews, and reverse sync."""

import json

import pytest

from shotpilot import fingerprint as fp
from shotpilot.model import ShotpilotError
from shotpilot.yamlio import load_yaml

from .conftest import launches


def actions(outcomes):
    return {o.shot_id: o.action for o in outcomes}


def test_definition_validates_and_resolves(ws):
    shots = ws.resolved()
    assert [s.id for s in shots][:2] == ["s01_gate", "s02_well"]
    s1 = shots[0]
    assert s1.frames == 150
    assert s1.start.location != s1.end.location  # dolly_in moved the camera
    assert s1.lens["focal_length"] == 24
    assert s1.lens["focus_distance"] > 0  # auto-focus on the look target


def test_bad_definition_reports_all_problems(ws):
    ws.path.write_text(ws.path.read_text(encoding="utf-8").replace("duration: 5\n", "duration: -1\n", 1)
                       .replace("anchor: well", "anchor: nowhere", 1), encoding="utf-8")
    with pytest.raises(ShotpilotError) as e:
        ws.reload()
    assert "duration" in str(e.value)


def test_only_changed_shot_rerenders(ws):
    first = ws.render(None, "previz")
    assert set(actions(first).values()) == {"rendered"}

    rep = ws.set_fields("s03_window", {"camera.lens.focal_length": 85})
    assert rep["stale"] == ["s03_window"]

    second = actions(ws.render(None, "previz"))
    assert second.pop("s03_window") == "rendered"
    assert set(second.values()) == {"reused"}


def test_revert_reuses_old_take(ws):
    ws.render(None, "previz")
    ws.set_fields("s02_well", {"duration": 2})
    assert actions(ws.render(["s02_well"], "previz"))["s02_well"] == "rendered"
    ws.set_fields("s02_well", {"duration": 3.5})
    assert actions(ws.render(["s02_well"], "previz"))["s02_well"] == "reused"


def test_order_purpose_notes_never_invalidate(ws):
    ws.render(None, "previz")
    rep = ws.edit_definition(order=["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"])
    assert rep["stale"] == []
    rep = ws.set_fields("s04_lantern", {"purpose": "別の説明", "notes": "memo"})
    assert rep["stale"] == []
    assert set(actions(ws.render(None, "previz")).values()) == {"reused"}
    info = ws.cut("previz", video=False)
    m = json.loads((ws.root / "cut" / "previz" / "manifest.json").read_text(encoding="utf-8"))
    assert [r["id"] for r in m["shots"]][0] == "s06_title"
    assert m["shots"][1]["record_in"] == m["shots"][0]["frames"]
    assert info["shots"] == 6


def test_comments_survive_machine_edits(ws):
    ws.set_fields("s01_gate", {"duration": 6})
    text = ws.path.read_text(encoding="utf-8")
    assert "# Order here is the order of the cut" in text
    assert "isolate_userdir: true            # run UE with its own -UserDir" in text


def test_cut_needs_takes(ws):
    with pytest.raises(ShotpilotError, match="no fresh"):
        ws.cut("preview", video=False)


def test_best_cut_mixes_qualities(ws):
    ws.render(None, "previz")
    ws.render(["s02_well"], "preview")
    picks = {s.id: q for s, _, q in ws.pick_takes("best")}
    assert picks["s02_well"] == "preview"
    assert picks["s01_gate"] == "previz"


def test_engine_builds_only_dirty_shots(ws):
    out = ws.render(None, "preview")
    assert set(actions(out).values()) == {"rendered"}
    assert len(launches(ws)[-1]["built"]) == 6

    ws.set_fields("s05_door", {"camera.move.distance": 300})
    out = actions(ws.render(None, "preview"))
    assert out.pop("s05_door") == "rendered"
    assert set(out.values()) == {"reused"}
    last = launches(ws)[-1]
    assert last["built"] == ["s05_door"]
    assert last["shots"] == ["s05_door:preview"]
    assert all(r.build == "synced" for r in ws.status())


def test_preview_and_still_frame_counts(ws):
    s = ws.resolved()[0]
    assert len(fp.frame_list(s, fp.quality(ws.defn, "preview"))) == s.frames
    assert fp.frame_list(s, fp.quality(ws.defn, "still")) == [0, (s.frames - 1) // 2, s.frames - 1]
    assert len(fp.frame_list(s, fp.quality(ws.defn, "previz"))) < s.frames


def _human_edit(ws, shot_id, focal=60.0, dz=40.0):
    def mutate(d):
        d["lens"]["focal_length"] = focal
        d["start"]["location"][2] += dz
    ws.engine.edit_asset(shot_id, mutate)


def test_pull_brings_editor_edits_back(ws):
    ws.build()
    _human_edit(ws, "s02_well")
    before = fp.render_fingerprint(ws.defn, ws.select(["s02_well"])[0], "preview")

    decisions = {d.shot_id: d for d in ws.pull()}
    assert decisions["s02_well"].kind == "engine_edit"
    assert decisions["s01_gate"].kind == "clean"

    s2 = ws.select(["s02_well"])[0]
    assert s2.lens["focal_length"] == 60
    doc = load_yaml(ws.path)
    node = next(n for n in doc["shots"] if n["id"] == "s02_well")
    assert "location" in node["camera"] and "anchor" not in node["camera"]
    assert fp.render_fingerprint(ws.defn, s2, "preview") != before  # needs re-render
    assert all(r.build == "synced" for r in ws.status())  # yaml == engine now
    assert [d.kind for d in ws.pull()].count("engine_edit") == 0  # idempotent


def test_build_refuses_to_clobber_editor_edits(ws):
    ws.build()
    _human_edit(ws, "s03_window")
    ws.set_fields("s03_window", {"duration": 5})
    rep = ws.build()
    assert rep["diverged"] == ["s03_window"]

    decisions = {d.shot_id: d.kind for d in ws.pull()}
    assert decisions["s03_window"] == "conflict"

    ws.pull(prefer="yaml")
    rep = ws.build()
    assert rep["built"] == ["s03_window"]
    assert ws.select(["s03_window"])[0].lens["focal_length"] == 50  # yaml won


def test_conflict_prefer_engine(ws):
    ws.build()
    _human_edit(ws, "s04_lantern", focal=28)
    ws.set_fields("s04_lantern", {"duration": 3})
    ws.pull(prefer="engine")
    s = ws.select(["s04_lantern"])[0]
    assert s.lens["focal_length"] == 28
    assert s.duration == 4  # engine's frame count wins too
    assert ws.build().get("diverged") in (None, [])


def test_render_blocked_when_diverged(ws):
    ws.render(["s01_gate"], "preview")
    _human_edit(ws, "s01_gate")
    ws.set_fields("s01_gate", {"camera.lens.focal_length": 30})
    out = ws.render(["s01_gate"], "preview")
    assert out[0].action == "failed"
    assert "pull" in out[0].detail


def test_gc_keeps_current(ws):
    ws.render(["s02_well"], "previz")
    for d in (2, 2.5, 3):
        ws.set_fields("s02_well", {"duration": d})
        ws.render(["s02_well"], "previz")
    removed = ws.gc(keep=1)
    assert len(removed) == 2
    assert ws.status()[1].qualities["previz"] == "fresh"


def test_history_records_actions(ws):
    ws.render(["s01_gate"], "previz")
    ws.set_fields("s01_gate", {"duration": 4})
    acts = [h["action"] for h in ws.state.data["history"]]
    assert acts[-2:] == ["render", "set"]


def test_beats_are_editorial_and_checked(ws):
    assert ws.structure_warnings() == []
    ws.render(None, "previz")
    rep = ws.set_fields("s02_well", {"beat": "hook"})
    assert rep["stale"] == []
    rep = ws.edit_definition(order=["s06_title", "s01_gate", "s02_well", "s03_window", "s04_lantern", "s05_door"])
    warnings = " ".join(ws.structure_warnings())
    assert "opens on `title`" in warnings and "end on the title beat" in warnings


def test_map_must_be_package_path(ws):
    text = ws.path.read_text(encoding="utf-8").replace("map: /Game/Maps/Courtyard", "map: C:/Program Files/Git/Game/X")
    ws.path.write_text(text, encoding="utf-8")
    with pytest.raises(ShotpilotError, match="MSYS_NO_PATHCONV"):
        ws.reload()
