"""shotpilot command line."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from . import __version__
from . import fingerprint as fpmod
from .model import ShotpilotError


def _ws(args):
    from .workspace import Workspace
    return Workspace(args.file)


def _out(msg: str = "") -> None:
    print(msg)


# --------------------------------------------------------------------------- commands


def cmd_init(args) -> int:
    from .templates import DEMO, EMPTY
    path = Path(args.file)
    if path.exists() and not args.force:
        raise ShotpilotError(f"{path} already exists (use --force to overwrite)")
    tpl = EMPTY if args.empty else DEMO
    text = tpl.format(
        name=args.name or path.resolve().parent.name,
        engine=args.engine,
        uproject=json.dumps(args.uproject) if args.uproject else "null",
        map=args.map or "null",
    )
    path.write_text(text, encoding="utf-8")
    gi = path.parent / ".gitignore"
    if not gi.exists():
        gi.write_text("render/\ncut/\n.shotpilot/jobs/\n.shotpilot/logs/\n.shotpilot/ue_userdir/\n"
                      ".shotpilot/mock_engine/\n", encoding="utf-8")
    from .model import load
    load(path)  # validate
    _out(f"wrote {path}. next: shotpilot render previz && shotpilot cut")
    return 0


def cmd_validate(args) -> int:
    ws = _ws(args)
    shots = ws.resolved()
    total = sum(s.duration for s in shots)
    _out(f"ok: {len(shots)} shots, {total:g}s (target {ws.defn.teaser.target_duration:g}s), engine={ws.defn.project.engine}")
    for w in ws.structure_warnings():
        _out(f"  note: {w}")
    return 0


def cmd_status(args) -> int:
    ws = _ws(args)
    rows = ws.status()
    qs = fpmod.quality_names(ws.defn)
    if args.json:
        _out(json.dumps([r.__dict__ for r in rows], ensure_ascii=False, indent=2))
        return 0
    idw = max([4] + [len(r.id) for r in rows])
    head = (f"{'#':>2}  {'shot':<{idw}}  {'beat':<9}  {'sec':>5}  {'engine':<6}  "
            + "  ".join(f"{q:<7}" for q in qs))
    _out(head)
    mark = {"fresh": "fresh", "stale": "STALE", "none": "-"}
    for r in rows:
        _out(f"{r.index + 1:>2}  {r.id:<{idw}}  {r.beat or '-':<9}  {r.duration:>5g}  {r.build:<6}  "
             + "  ".join(f"{mark[r.qualities[q]]:<7}" for q in qs))
    total = sum(r.duration for r in rows)
    target = ws.defn.teaser.target_duration
    _out(f"\ntotal {total:g}s / target {target:g}s ({total - target:+g}s)")
    _out("engine: synced = sequence asset matches shots.yaml, dirty = needs build, new = never built")
    _out("takes:  fresh = take exists for the current definition, STALE = only older takes exist")
    for w in ws.structure_warnings():
        _out(f"note: {w}")
    return 0


def cmd_build(args) -> int:
    ws = _ws(args)
    rep = ws.build(args.shots or None, force=args.force)
    if rep.get("skipped") is not None and not rep["built"]:
        _out("nothing to build: engine already matches shots.yaml")
        return 0
    _out(f"built: {', '.join(rep['built']) or '-'}")
    if rep.get("diverged"):
        _out(f"NOT overwritten (edited in the editor): {', '.join(rep['diverged'])}. "
             "run `shotpilot pull` to keep those edits, or `build --force` to discard them")
    if rep.get("error"):
        _out(f"errors: {', '.join(rep['error'])}")
        return 1
    return 0


def cmd_render(args) -> int:
    ws = _ws(args)
    targets = list(args.targets)
    quality = "previz"
    if targets and targets[0] in fpmod.quality_names(ws.defn):
        quality = targets.pop(0)
    outcomes = ws.render(targets or None, quality, force=args.force)
    rendered = [o for o in outcomes if o.action == "rendered"]
    for o in outcomes:
        extra = f"{o.seconds:.1f}s" if o.action == "rendered" else o.detail if o.action == "failed" else ""
        _out(f"  {o.action:<8} {o.shot_id:<20} {quality}:{o.fingerprint[:8]}  {extra}")
    _out(f"{quality}: {len(rendered)} rendered, {sum(o.action == 'reused' for o in outcomes)} reused, "
         f"{sum(o.action == 'failed' for o in outcomes)} failed")
    return 1 if any(o.action == "failed" for o in outcomes) else 0


def cmd_pull(args) -> int:
    ws = _ws(args)
    decisions = ws.pull(prefer=args.prefer, dry_run=args.dry_run)
    if not decisions:
        _out("nothing has been pushed to the engine yet")
        return 0
    labels = {
        "clean": "unchanged", "engine_edit": "PULLED (edited in engine)", "agree": "in sync",
        "conflict": "CONFLICT (changed in both)", "missing": "missing in engine", "never_pushed": "never pushed",
    }
    for d in decisions:
        lab = labels[d.kind]
        if d.kind == "engine_edit" and args.dry_run:
            lab = "would pull (edited in engine)"
        if d.kind == "conflict" and args.prefer:
            lab += f" -> kept {args.prefer}"
        _out(f"  {d.shot_id:<20} {lab}")
        for c in d.changes:
            _out(f"      {c}")
    if any(d.kind == "conflict" for d in decisions) and not args.prefer:
        _out("resolve conflicts with `shotpilot pull --prefer engine` or `--prefer yaml`")
        return 1
    return 0


def cmd_scan(args) -> int:
    ws = _ws(args)
    rep = ws.scan(tag=args.tag, prefix=args.prefix)
    _out(f"anchors added: {', '.join(rep['added']) or '-'}; moved: {', '.join(rep['moved']) or '-'}; "
         f"unchanged: {len(rep['same'])}")
    return 0


def cmd_cut(args) -> int:
    ws = _ws(args)
    info = ws.cut(args.quality, video=not args.no_video)
    _out(f"cut: {info['shots']} shots, {info['seconds']}s -> {info['dir']}")
    _out(f"  storyboard: {Path(info['dir']) / 'index.html'}")
    _out(f"  spec:       {Path(info['dir']) / 'spec.md'}")
    if info["video"]:
        _out(f"  video:      {info['video']}")
    elif not args.no_video:
        _out("  video:      skipped (ffmpeg not found)")
    return 0


def _print_edit_report(rep) -> None:
    if rep["order"]:
        _out(f"new order: {' > '.join(rep['order'])}")
    _out(f"edited: {', '.join(rep['edited']) or '-'}")
    _out(f"now stale (need re-render): {', '.join(rep['stale']) or 'nothing'}")


def cmd_set(args) -> int:
    from ruamel.yaml import YAML
    y = YAML(typ="safe")
    assignments = {}
    for a in args.assignments:
        if "=" not in a:
            raise ShotpilotError(f"expected key=value, got '{a}'")
        k, v = a.split("=", 1)
        assignments[k.strip()] = y.load(v) if v.strip() else None
    ws = _ws(args)
    sid = ws.select([args.shot])[0].id
    _print_edit_report(ws.set_fields(sid, assignments))
    return 0


def cmd_order(args) -> int:
    ws = _ws(args)
    ids = ws.defn.shot_ids()
    if args.move:
        sid = ws.select([args.move])[0].id
        rest = [i for i in ids if i != sid]
        pos = max(1, min(len(ids), args.to)) - 1
        new = rest[:pos] + [sid] + rest[pos:]
    else:
        new = [s.id for s in ws.select(args.ids)]
        new += [i for i in ids if i not in new]
    _print_edit_report(ws.edit_definition(order=new, action="order"))
    return 0


def cmd_plan(args) -> int:
    from . import ai
    from .yamlio import dumps, load_yaml, dump_yaml, to_rt
    ws = _ws(args)
    shots, rationale = ai.plan(ws.defn, pitch=args.pitch, duration=args.duration, n_shots=args.shots)
    total = sum(s["duration"] for s in shots)
    _out(f"# {len(shots)} shots, {total:g}s\n# {rationale}\n")
    _out(dumps(to_rt({"shots": shots})))
    if args.write:
        if ws.defn.shots and not args.replace:
            raise ShotpilotError("shots.yaml already has shots; use --replace to overwrite them")
        doc = load_yaml(ws.path)
        doc["shots"] = to_rt(shots)
        dump_yaml(doc, ws.path)
        ws.state.log("plan", shots=[s["id"] for s in shots])
        ws.state.save()
        _out(f"wrote {len(shots)} shots to {ws.path}")
    else:
        _out("(preview only; add --write to put these in shots.yaml)")
    return 0


def cmd_revise(args) -> int:
    from . import ai
    from .yamlio import dumps, to_rt
    ws = _ws(args)
    scope = [s.id for s in ws.select(args.shot)] if args.shot else None
    edits, order, summary = ai.revise(ws.defn, args.note, scope)
    if not edits and not order:
        _out(f"no change: {summary}")
        return 0
    _out(f"# {summary}")
    for sid, node in edits.items():
        _out(f"\n## {sid}\n- before:\n{dumps(to_rt(ws.defn.find(sid)))}- after:\n{dumps(to_rt(node))}")
    if order:
        _out(f"order: {' > '.join(order)}")
    if not args.yes:
        if not sys.stdin.isatty():
            _out("(not applied; pass --yes to apply non-interactively)")
            return 0
        if input("apply? [y/N] ").strip().lower() not in ("y", "yes"):
            _out("not applied")
            return 0
    _print_edit_report(ws.edit_definition(edits, order, action="revise", note=args.note))
    return 0


def cmd_log(args) -> int:
    ws = _ws(args)
    for h in ws.state.data["history"][-args.n:]:
        detail = {k: v for k, v in h.items() if k not in ("at", "action") and v not in (None, [], "", False)}
        _out(f"{h['at']}  {h['action']:<8} {json.dumps(detail, ensure_ascii=False)}")
    return 0


def cmd_takes(args) -> int:
    ws = _ws(args)
    s = ws.select([args.shot])[0]
    for q in fpmod.quality_names(ws.defn):
        cur = fpmod.render_fingerprint(ws.defn, s, q)
        for t in ws.state.list_takes(s.id, q):
            flag = "*" if t["fingerprint"] == cur else " "
            _out(f" {flag} {q:<8} {t['fingerprint'][:12]}  {t.get('created', '')}  {len(t.get('files', []))} frames"
                 f"  {t.get('seconds', 0)}s  {t['dir']}")
    _out("(* = matches the current definition)")
    return 0


def cmd_gc(args) -> int:
    ws = _ws(args)
    removed = ws.gc(keep=args.keep, dry_run=args.dry_run)
    verb = "would remove" if args.dry_run else "removed"
    _out(f"{verb} {len(removed)} old take(s)")
    for p in removed:
        _out(f"  {p}")
    return 0


# --------------------------------------------------------------------------- parser


def cmd_serve(args) -> int:
    from .server import LOOPBACK, make_server
    try:
        srv = make_server(args.file, args.host, args.port)
    except OSError as e:
        raise ShotpilotError(f"cannot listen on {args.host}:{args.port}: {e}") from None
    if args.host.lower() not in LOOPBACK:
        _out(f"warning: listening on {args.host}; there is no authentication, anyone who can reach it can edit shots")
    _out(f"shotpilot UI: {srv.url}  (Ctrl+C to stop)")
    if args.open:
        import webbrowser
        webbrowser.open(srv.url)
    try:
        srv.serve_forever()
    finally:
        srv.server_close()
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="shotpilot", description="Shot design and capture lane for UE5 teasers.")
    p.add_argument("-f", "--file", default="shots.yaml", help="shot definition (default: shots.yaml)")
    p.add_argument("--version", action="version", version=f"shotpilot {__version__}")
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("init", help="write a starter shots.yaml")
    s.add_argument("--engine", choices=["mock", "ue5"], default="mock")
    s.add_argument("--uproject")
    s.add_argument("--map", help="e.g. /Game/Maps/Courtyard")
    s.add_argument("--name")
    s.add_argument("--empty", action="store_true", help="no sample shots")
    s.add_argument("--force", action="store_true")
    s.set_defaults(fn=cmd_init)

    sub.add_parser("validate", help="check shots.yaml").set_defaults(fn=cmd_validate)

    s = sub.add_parser("status", help="what is fresh, stale or out of sync")
    s.add_argument("--json", action="store_true")
    s.set_defaults(fn=cmd_status)

    s = sub.add_parser("build", help="push changed shots into engine sequences")
    s.add_argument("shots", nargs="*")
    s.add_argument("--force", action="store_true", help="rebuild even if unchanged / overwrite editor edits")
    s.set_defaults(fn=cmd_build)

    s = sub.add_parser("render", help="render stale shots: render [previz|still|preview|final] [shot ...]")
    s.add_argument("targets", nargs="*", metavar="quality/shot")
    s.add_argument("--force", action="store_true", help="re-render even if a matching take exists")
    s.set_defaults(fn=cmd_render)

    s = sub.add_parser("pull", help="bring Sequencer edits back into shots.yaml")
    s.add_argument("--prefer", choices=["engine", "yaml"], help="how to resolve conflicts")
    s.add_argument("--dry-run", action="store_true")
    s.set_defaults(fn=cmd_pull)

    s = sub.add_parser("scan", help="import anchors from tagged level actors")
    s.add_argument("--tag", default="shotpilot")
    s.add_argument("--prefix", default="SP_")
    s.set_defaults(fn=cmd_scan)

    s = sub.add_parser("cut", help="assemble the teaser from existing takes (no rendering)")
    s.add_argument("quality", nargs="?", default="best", help="best (default) | previz | still | preview | final")
    s.add_argument("--no-video", action="store_true")
    s.set_defaults(fn=cmd_cut)

    s = sub.add_parser("set", help="edit fields of one shot: set s03 camera.lens.focal_length=50 duration=5")
    s.add_argument("shot")
    s.add_argument("assignments", nargs="+")
    s.set_defaults(fn=cmd_set)

    s = sub.add_parser("order", help="reorder shots: order s06 s01 ...  |  order --move s06 --to 1")
    s.add_argument("ids", nargs="*")
    s.add_argument("--move")
    s.add_argument("--to", type=int, default=1)
    s.set_defaults(fn=cmd_order)

    s = sub.add_parser("plan", help="draft a shot list with Claude from the pitch and anchors")
    s.add_argument("--pitch")
    s.add_argument("--duration", type=float)
    s.add_argument("--shots", type=int)
    s.add_argument("--write", action="store_true")
    s.add_argument("--replace", action="store_true")
    s.set_defaults(fn=cmd_plan)

    s = sub.add_parser("revise", help='apply a note with Claude: revise "3番目はもっと寄って"')
    s.add_argument("note")
    s.add_argument("--shot", action="append", help="limit edits to this shot (repeatable)")
    s.add_argument("--yes", action="store_true")
    s.set_defaults(fn=cmd_revise)

    s = sub.add_parser("takes", help="list takes of one shot")
    s.add_argument("shot")
    s.set_defaults(fn=cmd_takes)

    s = sub.add_parser("log", help="history of builds, renders, pulls and edits")
    s.add_argument("-n", type=int, default=30)
    s.set_defaults(fn=cmd_log)

    s = sub.add_parser("gc", help="delete old takes")
    s.add_argument("--keep", type=int, default=2)
    s.add_argument("--dry-run", action="store_true")
    s.set_defaults(fn=cmd_gc)

    s = sub.add_parser("serve", help="local browser UI over the same operations (loopback only by default)")
    s.add_argument("--host", default="127.0.0.1")
    s.add_argument("--port", type=int, default=7460)
    s.add_argument("--open", action="store_true", help="open the UI in the default browser")
    s.set_defaults(fn=cmd_serve)
    return p


def main(argv: list[str] | None = None) -> int:
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
        except (AttributeError, ValueError):
            pass
    args = build_parser().parse_args(argv)
    try:
        return int(args.fn(args) or 0)
    except ShotpilotError as e:
        print(f"shotpilot: {e}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())
