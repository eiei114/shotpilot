"""Comment-preserving YAML I/O.

Humans edit shots.yaml by hand, and `pull` / `revise` edit it by machine. Round-trip
loading keeps the human's comments and key order intact through machine edits.
"""

from __future__ import annotations

import io
from pathlib import Path
from typing import Any

from ruamel.yaml import YAML
from ruamel.yaml.comments import CommentedMap, CommentedSeq


def _yaml() -> YAML:
    y = YAML(typ="rt")
    y.preserve_quotes = True
    y.width = 120
    y.indent(mapping=2, sequence=4, offset=2)
    return y


def load_yaml(path: Path) -> Any:
    with open(path, encoding="utf-8") as f:
        return _yaml().load(f)


def dump_yaml(data: Any, path: Path) -> None:
    buf = io.StringIO()
    _yaml().dump(data, buf)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(buf.getvalue(), encoding="utf-8")
    tmp.replace(path)


def dumps(data: Any) -> str:
    buf = io.StringIO()
    _yaml().dump(data, buf)
    return buf.getvalue()


def to_rt(v: Any) -> Any:
    """Plain python -> ruamel round-trip nodes, with short numeric lists kept inline."""
    if isinstance(v, dict):
        m = CommentedMap()
        for k, x in v.items():
            m[k] = to_rt(x)
        return m
    if isinstance(v, (list, tuple)):
        s = CommentedSeq([to_rt(x) for x in v])
        if len(v) <= 4 and all(isinstance(x, (int, float)) for x in v):
            s.fa.set_flow_style()
        return s
    return v
