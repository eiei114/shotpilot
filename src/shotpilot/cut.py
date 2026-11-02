"""Assemble the teaser in definition order from existing takes.

Nothing is rendered here. Each take is encoded once into a cached segment.mp4 inside
its own take folder; the cut is a stream-copy concat of those segments. Reordering
shots or swapping one take therefore costs a concat, not a render.
"""

from __future__ import annotations

import html
import json
import os
from pathlib import Path
from typing import Any

from . import fingerprint as fpmod
from .model import Definition, ResolvedShot
from .state import now_iso


def _segment(take: dict[str, Any], w: int, h: int, fps: int) -> Path:
    from .workspace import run_ffmpeg
    d: Path = take["dir"]
    seg = d / f"segment_{w}x{h}_{fps}.mp4"
    if seg.exists():
        return seg
    frames: list[int] = take["frames"]
    files: list[str] = take["files"]
    total = int(take["shot_frames"])
    lines = []
    for i, (f, name) in enumerate(zip(frames, files)):
        nxt = frames[i + 1] if i + 1 < len(frames) else total
        lines.append(f"file '{name}'")
        lines.append(f"duration {(nxt - f) / take['fps']:.6f}")
    lines.append(f"file '{files[-1]}'")  # concat demuxer needs the last file twice for its duration
    lst = d / "frames.txt"
    lst.write_text("\n".join(lines) + "\n", encoding="utf-8")
    vf = (f"scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,"
          f"fps={fps},format=yuv420p")
    tmp = seg.with_suffix(".tmp.mp4")
    run_ffmpeg(["-f", "concat", "-safe", "0", "-i", str(lst), "-vf", vf, "-frames:v", str(total),
                "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-r", str(fps), str(tmp)])
    tmp.replace(seg)
    return seg


def _thumb(take: dict[str, Any]) -> str:
    files = take["files"]
    return files[len(files) // 2]


def write_cut(defn: Definition, picks: list[tuple[ResolvedShot, dict[str, Any], str]], out_dir: Path,
              video: bool = True) -> dict[str, Any]:
    from .workspace import ffmpeg_available, run_ffmpeg
    out_dir.mkdir(parents=True, exist_ok=True)
    fps = defn.teaser.fps
    w, h = fpmod.resolution(defn, fpmod.quality(defn, "preview"))
    qualities = {qn for _, _, qn in picks}
    if qualities == {"final"}:
        w, h = defn.teaser.resolution

    t = 0
    rows = []
    for s, take, qn in picks:
        frames = s.frames
        rows.append({
            "order": len(rows) + 1, "id": s.id, "beat": s.beat, "purpose": s.purpose, "notes": s.notes,
            "duration": s.duration, "frames": frames, "record_in": t, "record_out": t + frames,
            "quality": qn, "fingerprint": take["fingerprint"],
            "take_dir": os.path.relpath(take["dir"], out_dir).replace("\\", "/"),
            "files": take["files"], "frame_numbers": take["frames"],
            "camera": {"start": s.start.to_dict(), "end": s.end.to_dict(), "curve": s.curve},
            "lens": s.lens, "environment": s.environment, "markers": s.markers,
        })
        t += frames

    manifest = {
        "generated": now_iso(), "project": defn.project.name, "title": defn.teaser.title,
        "fps": fps, "total_frames": t, "total_seconds": round(t / fps, 3),
        "target_seconds": defn.teaser.target_duration, "shots": rows,
    }
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")
    (out_dir / "spec.md").write_text(_spec_md(defn, manifest), encoding="utf-8")
    (out_dir / "index.html").write_text(_storyboard_html(defn, manifest), encoding="utf-8")

    info = {"dir": str(out_dir), "seconds": manifest["total_seconds"], "shots": len(rows), "video": None}
    if video and ffmpeg_available():
        segs = [_segment(take, w, h, fps) for _, take, _ in picks]
        lst = out_dir / "segments.txt"
        lst.write_text("".join(f"file '{p.as_posix()}'\n" for p in segs), encoding="utf-8")
        run_ffmpeg(["-f", "concat", "-safe", "0", "-i", str(lst), "-c", "copy", str(out_dir / "cut.mp4")])
        info["video"] = str(out_dir / "cut.mp4")
    return info


def _tc(frame: int, fps: int) -> str:
    s, f = divmod(frame, fps)
    m, s = divmod(s, 60)
    return f"{m:02d}:{s:02d}:{f:02d}"


def _spec_md(defn: Definition, m: dict[str, Any]) -> str:
    lines = [
        f"# {defn.teaser.title or defn.project.name} — shot spec",
        "",
        f"- generated: {m['generated']}",
        f"- runtime: {m['total_seconds']}s (target {m['target_seconds']}s), {m['fps']} fps, {len(m['shots'])} shots",
        f"- map: {defn.project.map or '-'}",
        "",
        "| # | shot | beat | in | out | sec | lens | env | take | purpose |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    for r in m["shots"]:
        env = r["environment"].get("time_of_day", "-")
        lines.append(
            f"| {r['order']} | `{r['id']}` | {r['beat'] or '-'} | {_tc(r['record_in'], m['fps'])} | {_tc(r['record_out'], m['fps'])} "
            f"| {r['duration']:g} | {r['lens']['focal_length']:g}mm f/{r['lens'].get('aperture', 0):g} | {env} "
            f"| {r['quality']}:{r['fingerprint'][:8]} | {r['purpose']} |")
    lines.append("")
    for r in m["shots"]:
        c = r["camera"]
        lines += [
            f"## {r['order']}. {r['id']}",
            "",
            f"- beat: {r['beat'] or '-'}",
            f"- purpose: {r['purpose'] or '-'}",
            f"- camera start: loc {c['start']['location']} rot {c['start']['rotation']}",
            f"- camera end:   loc {c['end']['location']} rot {c['end']['rotation']} ({c['curve']})",
            f"- lens: {json.dumps(r['lens'])}",
            f"- environment: {json.dumps(r['environment'], ensure_ascii=False)}",
            f"- frames: {r['take_dir']}/ ({len(r['files'])} files)",
        ]
        for mk in r["markers"]:
            lines.append(f"- marker @{mk['frame']}: {mk['label']}")
        if r["notes"]:
            lines.append(f"- notes: {r['notes']}")
        lines.append("")
    return "\n".join(lines)


def _storyboard_html(defn: Definition, m: dict[str, Any]) -> str:
    esc = html.escape
    cards = []
    for r in m["shots"]:
        thumb = f"{r['take_dir']}/{r['files'][len(r['files']) // 2]}"
        first, last = f"{r['take_dir']}/{r['files'][0]}", f"{r['take_dir']}/{r['files'][-1]}"
        cards.append(f"""
<article class="card">
  <div class="frames"><img src="{esc(first)}" alt=""><img class="mid" src="{esc(thumb)}" alt=""><img src="{esc(last)}" alt=""></div>
  <div class="meta">
    <div class="head"><span class="n">{r['order']}</span><code>{esc(r['id'])}</code>
      <span class="beat">{esc(r['beat'] or '')}</span>
      <span class="q q-{esc(r['quality'])}">{esc(r['quality'])}</span></div>
    <p class="purpose">{esc(r['purpose'])}</p>
    <p class="facts">{r['duration']:g}s · {r['lens']['focal_length']:g}mm · {esc(str(r['environment'].get('time_of_day', '')))}
      · {_tc(r['record_in'], m['fps'])}</p>
  </div>
</article>""")
    title = esc(defn.teaser.title or defn.project.name)
    over = m["total_seconds"] - m["target_seconds"]
    return f"""<!doctype html>
<html lang="ja"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} storyboard</title>
<style>
:root {{ --bg:#111; --fg:#eee; --mute:#999; --card:#1b1b1b; --line:#2a2a2a; }}
@media (prefers-color-scheme: light) {{ :root {{ --bg:#f6f6f4; --fg:#111; --mute:#666; --card:#fff; --line:#ddd; }} }}
body {{ margin:0; background:var(--bg); color:var(--fg); font:14px/1.5 system-ui, sans-serif; }}
header {{ padding:20px 16px 8px; }} h1 {{ margin:0; font-size:20px; }}
.sub {{ color:var(--mute); }}
main {{ display:grid; gap:12px; padding:16px; grid-template-columns:repeat(auto-fill,minmax(320px,1fr)); }}
.card {{ background:var(--card); border:1px solid var(--line); border-radius:8px; overflow:hidden; }}
.frames {{ display:grid; grid-template-columns:1fr 2fr 1fr; gap:2px; background:#000; }}
.frames img {{ width:100%; aspect-ratio:16/9; object-fit:cover; display:block; }}
.meta {{ padding:8px 10px; }} .head {{ display:flex; gap:8px; align-items:center; }}
.n {{ font-weight:700; }} .beat {{ color:var(--mute); font-size:12px; text-transform:uppercase; }} .purpose {{ margin:4px 0; }} .facts {{ margin:0; color:var(--mute); font-size:12px; }}
.q {{ margin-left:auto; font-size:11px; padding:1px 6px; border-radius:4px; border:1px solid var(--line); }}
.q-final {{ background:#1f6f3f; color:#fff; }} .q-preview {{ background:#2d4f8f; color:#fff; }}
.q-still {{ background:#6b4f9f; color:#fff; }} .q-previz {{ background:#555; color:#fff; }}
</style></head><body>
<header><h1>{title}</h1>
<div class="sub">{m['total_seconds']}s / target {m['target_seconds']}s ({'+' if over >= 0 else ''}{over:.1f}s) · {len(m['shots'])} shots · {m['fps']} fps · {esc(m['generated'])}</div>
</header>
<main>{''.join(cards)}</main>
</body></html>
"""
