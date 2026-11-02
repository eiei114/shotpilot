import json
from pathlib import Path

import pytest

from shotpilot.templates import DEMO
from shotpilot.workspace import Workspace


@pytest.fixture
def ws(tmp_path: Path) -> Workspace:
    p = tmp_path / "shots.yaml"
    p.write_text(DEMO.format(name="t", engine="mock", uproject="null", map="/Game/Maps/Courtyard"), encoding="utf-8")
    return Workspace(p)


def launches(ws: Workspace) -> list[dict]:
    f = ws.root / ".shotpilot" / "mock_engine" / "launches.jsonl"
    if not f.exists():
        return []
    return [json.loads(line) for line in f.read_text(encoding="utf-8").splitlines() if line.strip()]
