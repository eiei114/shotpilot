"""Claude-assisted shot planning and partial revision.

`plan`   : pitch + anchors -> a first shot list (high-level camera intent only).
`revise` : natural-language note -> edits to the named shots only, plus an optional
           reorder. Everything Claude returns is validated by resolving it before it
           touches shots.yaml, and the edit scope is enforced in code, not by prompt.
"""

from __future__ import annotations

import json
import os
from typing import Any

from . import model
from .model import BEAT_GUIDE, BEATS, MOVES, TIME_OF_DAY, Definition, ShotpilotError
from .yamlio import dumps

MODEL = os.environ.get("SHOTPILOT_MODEL", "claude-opus-5-5")

CONVENTIONS = """\
Unreal Engine conventions: units are centimeters, Z is up, X forward, Y right.
A camera is placed relative to an anchor (a named point in the level) with an
offset [x, y, z] in world space, and looks at that anchor unless look_at names
another anchor. 1.7 m eye height is z=170 relative to the ground. Typical framing
distances: close-up 80-200 cm at 50-85 mm, medium 250-500 cm at 35-50 mm,
wide 800-3000 cm at 18-28 mm.
Moves: static, dolly_in/dolly_out (distance cm along view), truck_left/truck_right
(distance cm sideways, keeps aim on the subject), crane_up/crane_down (distance cm),
orbit_left/orbit_right (degrees around the subject)."""

_SHOT_SCHEMA = {
    "type": "object",
    "properties": {
        "id": {"type": "string", "description": "lowercase id like s01_courtyard"},
        "beat": {"type": "string", "enum": list(BEATS)},
        "purpose": {"type": "string", "description": "what this shot must communicate, one sentence"},
        "duration": {"type": "number"},
        "anchor": {"type": "string"},
        "offset": {"type": "array", "items": {"type": "number"}},
        "look_at": {"type": "string", "description": "anchor name, or empty string to look at `anchor`"},
        "focal_length": {"type": "number"},
        "move": {"type": "string", "enum": sorted(MOVES)},
        "move_amount": {"type": "number", "description": "cm for dolly/truck/crane, degrees for orbit, 0 for static"},
        "time_of_day": {"type": "string", "enum": ["default", *sorted(TIME_OF_DAY)]},
        "notes": {"type": "string"},
    },
    "required": ["id", "beat", "purpose", "duration", "anchor", "offset", "look_at", "focal_length", "move",
                 "move_amount", "time_of_day", "notes"],
    "additionalProperties": False,
}

PLAN_SCHEMA = {
    "type": "object",
    "properties": {
        "rationale": {"type": "string"},
        "shots": {"type": "array", "items": _SHOT_SCHEMA},
    },
    "required": ["rationale", "shots"],
    "additionalProperties": False,
}

REVISE_SCHEMA = {
    "type": "object",
    "properties": {
        "summary": {"type": "string"},
        "edits": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "yaml": {"type": "string", "description": "the complete replacement YAML mapping for this shot"},
                    "reason": {"type": "string"},
                },
                "required": ["id", "yaml", "reason"],
                "additionalProperties": False,
            },
        },
        "order": {"type": "array", "items": {"type": "string"},
                  "description": "full list of shot ids in the new order, or empty to keep the order"},
    },
    "required": ["summary", "edits", "order"],
    "additionalProperties": False,
}


def _client():
    try:
        import anthropic
    except ImportError as e:
        raise ShotpilotError("the `anthropic` package is required: pip install 'shotpilot[ai]'") from e
    return anthropic.Anthropic()


def _ask(system: str, user: str, schema: dict[str, Any], client=None) -> dict[str, Any]:
    import anthropic
    client = client or _client()
    try:
        with client.beta.messages.stream(
            model=MODEL,
            max_tokens=64000,
            thinking={"type": "adaptive"},
            output_config={"effort": "medium", "format": {"type": "json_schema", "schema": schema}},
            betas=["server-side-fallback-2026-07-01"],
            fallbacks="default",
            system=system,
            messages=[{"role": "user", "content": user}],
        ) as stream:
            msg = stream.get_final_message()
    except anthropic.AuthenticationError as e:
        raise ShotpilotError("Claude API authentication failed. set ANTHROPIC_API_KEY") from e
    except anthropic.RateLimitError as e:
        raise ShotpilotError("Claude API rate limited; retry shortly") from e
    except anthropic.APIStatusError as e:
        raise ShotpilotError(f"Claude API error {e.status_code}: {e.message}") from e
    except anthropic.APIConnectionError as e:
        raise ShotpilotError("could not reach the Claude API") from e
    if msg.stop_reason == "refusal":
        raise ShotpilotError("Claude declined this request")
    if msg.stop_reason == "max_tokens":
        raise ShotpilotError("Claude's answer was cut off (max_tokens)")
    text = next((b.text for b in msg.content if b.type == "text"), None)
    if not text:
        raise ShotpilotError("Claude returned no text")
    return json.loads(text)


# --------------------------------------------------------------------------- plan


def plan(defn: Definition, pitch: str | None = None, duration: float | None = None, n_shots: int | None = None,
         client=None) -> tuple[list[dict[str, Any]], str]:
    if not defn.anchors:
        raise ShotpilotError("no anchors. run `shotpilot scan` (or add `anchors:`) so shots can be placed")
    pitch = pitch or defn.teaser.pitch
    if not pitch:
        raise ShotpilotError("give a pitch: --pitch '...' or teaser.pitch in shots.yaml")
    duration = duration or defn.teaser.target_duration
    system = (
        "You are a trailer director planning a game teaser shot list for an indie developer. "
        "You plan what to film inside the engine before anything is recorded. Each shot exists "
        "to make the viewer feel one specific thing. Prefer fewer, deliberate shots over many busy ones.\n\n"
        "Every teaser follows this shape, in order, and each shot carries its beat:\n"
        + "\n".join(f"- {b}: {BEAT_GUIDE[b]}" for b in BEATS) + "\n\n" + CONVENTIONS)
    anchors = "\n".join(f"- {k}: {list(v)}" for k, v in defn.anchors.items())
    user = (
        f"Game pitch: {pitch}\n"
        f"Target teaser length: {duration:g} seconds (shot durations must sum to within 10% of this).\n"
        + (f"Number of shots: {n_shots}\n" if n_shots else "")
        + f"Available anchors (only use these names):\n{anchors}\n\n"
        "Write the shot list. Write `purpose` and `notes` in the same language as the pitch.")
    out = _ask(system, user, PLAN_SCHEMA, client)
    shots = [_shot_from_plan(s, i) for i, s in enumerate(out["shots"])]
    _validate_shots(defn, shots)
    return shots, out.get("rationale", "")


def _shot_from_plan(s: dict[str, Any], i: int) -> dict[str, Any]:
    cam: dict[str, Any] = {"anchor": s["anchor"], "offset": [round(float(v), 1) for v in (s["offset"] + [0, 0, 0])[:3]]}
    if s.get("look_at") and s["look_at"] != s["anchor"]:
        cam["look_at"] = s["look_at"]
    if s["move"] != "static":
        key = "degrees" if s["move"].startswith("orbit") else "distance"
        cam["move"] = {"type": s["move"], key: round(float(s["move_amount"]), 1)}
    cam["lens"] = {"focal_length": round(float(s["focal_length"]), 1)}
    shot: dict[str, Any] = {"id": _clean_id(s["id"], i), "beat": s["beat"], "purpose": s["purpose"],
                            "duration": round(float(s["duration"]), 2), "camera": cam}
    if s["time_of_day"] != "default":
        shot["environment"] = {"time_of_day": s["time_of_day"]}
    if s.get("notes"):
        shot["notes"] = s["notes"]
    return shot


def _clean_id(raw: str, i: int) -> str:
    sid = "".join(c if c.isalnum() or c in "_-" else "_" for c in raw.lower()).strip("_-")[:48]
    return sid if sid and model.ID_RE.match(sid) else f"s{i + 1:02d}"


def _validate_shots(defn: Definition, shots: list[dict[str, Any]]) -> None:
    trial = model.Definition(defn.path, defn.project, defn.teaser, defn.anchors, defn.defaults, defn.qualities, shots)
    for i, s in enumerate(shots):
        model.resolve_shot(trial, s, i)
    ids = [s["id"] for s in shots]
    if len(set(ids)) != len(ids):
        raise ShotpilotError(f"duplicate shot ids from Claude: {ids}")


# --------------------------------------------------------------------------- revise


def revise(defn: Definition, instruction: str, scope: list[str] | None = None, client=None
           ) -> tuple[dict[str, dict[str, Any]], list[str] | None, str]:
    """Returns (replacement shots by id, new order or None, summary). Nothing is written here."""
    from ruamel.yaml import YAML

    ids = defn.shot_ids()
    if scope:
        unknown = [s for s in scope if s not in ids]
        if unknown:
            raise ShotpilotError(f"unknown shot(s): {', '.join(unknown)}")
    resolved = {s.id: s for s in model.resolve_all(defn)}
    lines = []
    for i, s in enumerate(defn.shots, 1):
        r = resolved[s["id"]]
        lines.append(f"# shot {i}: resolves to start {r.start.to_dict()} end {r.end.to_dict()} "
                     f"lens {r.lens['focal_length']}mm")
        lines.append(dumps([s]).rstrip())
    anchors = "\n".join(f"- {k}: {list(v)}" for k, v in defn.anchors.items()) or "(none)"
    system = (
        "You revise a game teaser shot list defined in YAML. You change as little as possible: "
        "only the shots the note is about, only the fields that need to change. You never rename ids. "
        "Shots are numbered from 1 in the order listed (\"the third shot\" = shot 3). Each shot has an "
        "editorial `beat` (" + " -> ".join(BEATS) + "); keep it unless the note changes the shot's role.\n\n"
        + CONVENTIONS
        + "\nA shot's camera can be either `anchor`+`offset` (+ optional `move`, `look_at`, `lens`), or "
        "explicit `location` + `rotation` [pitch, yaw, roll] / `look_at`, with optional `end` "
        "{location, rotation|look_at, curve}. `environment.time_of_day` is one of "
        + ", ".join(sorted(TIME_OF_DAY)) + ".")
    scope_line = (f"You may only edit these shots: {', '.join(scope)}. Leave `order` empty.\n" if scope else
                  "Only reorder if the note asks for it.\n")
    user = (f"Anchors:\n{anchors}\n\nCurrent shots:\n" + "\n".join(lines)
            + f"\n\nNote from the director: {instruction}\n" + scope_line
            + "Return each changed shot as a complete YAML mapping (without the leading '- ').")
    out = _ask(system, user, REVISE_SCHEMA, client)

    edits: dict[str, dict[str, Any]] = {}
    y = YAML(typ="safe")
    for e in out.get("edits", []):
        sid = e["id"]
        if sid not in ids:
            raise ShotpilotError(f"Claude edited an unknown shot '{sid}'")
        if scope and sid not in scope:
            raise ShotpilotError(f"Claude tried to edit '{sid}' outside the allowed scope {scope}; nothing applied")
        new = y.load(e["yaml"])
        if isinstance(new, list) and len(new) == 1:
            new = new[0]
        if not isinstance(new, dict):
            raise ShotpilotError(f"edit for '{sid}' is not a mapping")
        new["id"] = sid
        edits[sid] = new
    order = out.get("order") or None
    if order:
        if scope:
            raise ShotpilotError("Claude tried to reorder while scoped to specific shots; nothing applied")
        if sorted(order) != sorted(ids):
            raise ShotpilotError(f"Claude's new order {order} is not a permutation of {ids}")
        if order == ids:
            order = None
    trial = [edits.get(s["id"], s) for s in defn.shots]
    _validate_shots(defn, trial)
    return edits, order, out.get("summary", "")
