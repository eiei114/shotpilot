# references/

Reference material for planning teasers, laid out after open-design
(`design-systems/` + `craft/` + `design-templates/`). Claude reads these before it
writes a shot list; nothing here is checked at runtime yet.

| Axis | open-design | here | holds |
|---|---|---|---|
| Style package | `design-systems/<brand>/` | `teasers/<id>/` | one real teaser: "Inspired by" prose + measured numbers |
| Universal rules | `craft/*.md` | `craft/` (not yet) | trailer rules true for any title (now in `craft.rs` and `research_notes/`) |
| Shape | `design-templates/` | `templates.rs` skeletons | `hook → establish → escalate → title` |

## teasers/<id>/

```
manifest.json        machine entry: id, name, category, language, source, files, status
TEASER.md            read first. "Teaser Inspired by X": tone, beats with timecodes, rhythm,
                     camera, title cards, sound, and Do / Avoid for shotpilot
tokens.json          measured numbers (ffmpeg): shots, ASL, cuts per quarter, blacks, loudness
source/evidence.md   URL, channel, how it was found and measured
preview/sheet.jpg    8x6 contact sheet. Gitignored (frames of a third-party work)
```

`status` moves `catalogued → downloaded → measured → analyzed`. A package is
`analyzed` once the `> 分析: 未` line is gone from `TEASER.md`.

Worked example: [`teasers/yt-rsDR9xaPIJE`](teasers/yt-rsDR9xaPIJE/TEASER.md).
Catalogue: [`teasers/INDEX.md`](teasers/INDEX.md).

## Regenerating

```bash
sh research_data/tools/download.sh            # resumable; videos stay in research_data/ (not in git)
python -I research_data/tools/build_refs.py research_data/teasers references/teasers --measure
```

`build_refs.py` rewrites `manifest.json`, `tokens.json` (only when missing),
`evidence.md`, the indexes, and the block between `<!-- auto:begin -->` and
`<!-- auto:end -->` in `TEASER.md`. Everything else in `TEASER.md` is hand
(or Claude) written and is never overwritten.

## Rules

- Videos and frames stay local and are used for analysis only (Japan Copyright Act
  Art. 30-4). Do not commit them or ship them in the submission.
- Write what you saw. Mark what you inferred without watching/listening as 推定.
- Measured cut counts are approximate: the scene detector misses dissolves and
  counts flashes; check against the contact sheet before quoting a number.
