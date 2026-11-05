# references/

Reference material for planning teasers, laid out after open-design
(`design-systems/` + `craft/` + `design-templates/` + `skills/`). `craft/` and `genres/` are
embedded in the binary (`crates/shotpilot/src/refs.rs`) and handed to Claude on every `plan`;
`design-systems/` and `skills/` are embedded too (`designsys.rs`, `skills.rs`) and handed to
Claude when the planning turn picked them.

| Axis | open-design | here | holds |
|---|---|---|---|
| Universal rules | `craft/*.md` | `craft/` | what holds for any game teaser, measured over every teaser (D-29) |
| Genre package | `design-systems/<brand>/` | `genres/<id>/` | one genre: prose + measured numbers + aliases (D-29) |
| Style package | `design-systems/<brand>/` | `teasers/<id>/` | one real teaser: "Inspired by" prose + measured numbers |
| Shape | `design-templates/` | `craft.rs` skeleton | `hook → establish → escalate → title` |
| Look | `design-systems/<brand>/` | `design-systems/<id>/` | one look a teaser wears: `DESIGN.md` + `tokens.json` + `manifest.json` (D-45) |
| Playbook | `skills/<id>/SKILL.md` | `skills/<id>/SKILL.md` | one kind of teaser, or one part of it (D-45) |

## Design systems and skills (D-45)

The planning turn (`ai::discover`, after open-design's OD Next planning turn) reads two
catalogues and picks from them; the picks go to `brief.design` and `brief.skills`. When the
shot list is written, the planner reads the picked `DESIGN.md` whole ("## Active design
system", treated as authoritative), its `tokens.json`, and the whole body of every picked
skill ("## Active skill"). The design system's tokens also set `finish.grade`,
`finish.letterbox` and `finish.typeface` in code.

- `design-systems/<id>/`: twelve generic looks (quiet-dread, vhs-found-footage,
  cozy-pastel, storybook, grimdark, epic-fantasy, neon-action, retro-pixel,
  arthouse-minimal, noir-mystery, cold-scifi, command-room). `DESIGN.md` keeps the nine
  sections of the DESIGN.md convention (VoltAgent / Google Stitch), adapted to teasers:
  1 Visual Theme & Atmosphere, 2 Color & Grade, 3 Typography Rules, 4 Title & Card
  Stylings, 5 Pacing & Transitions, 6 Camera Language, 7 Sound Character, 8 Do's and
  Don'ts, 9 Agent Prompt Guide. None copies a studio's identity: no names, logos or
  trademark colours of real games.
- A project may keep its own `DESIGN.md` next to shots.yaml (the game's brand). It wins
  for the prose; the numbers still come from `brief.design`. `shotpilot design <id> --fork`
  copies a look there to start from.
- `skills/<id>/SKILL.md`: eighteen playbooks, after open-design's SKILL.md (front matter
  `name`, `ja_name`, `description`, `triggers`, `sp: {mode, kind, genres, design_system,
  example_prompt}`; body: Resource map, Workflow, Hard rules). `main` skills shape the
  whole teaser (announce, mood, gameplay-first, release-date, demo, character, world tour,
  threat, rules, diary, verb montage, one take, update, mystery); `utility` skills shape a
  part (cold open, title drop, end card, count run). With no main skill picked, the kind's
  default applies (gameplay-first or announce-teaser).
- To add one: write the package, add a `ds!("<id>")` line in `designsys.rs` or a
  `skill!("<id>")` line in `skills.rs`; `cargo test --test planning` checks the sections,
  the grade and typeface names, and the D-22 words.

## How plan uses them

`plan` (and `produce`) always reads `craft/TEASER-CRAFT.md`. When `brief.genre` contains
one of a genre's aliases (`genres/<id>/manifest.json`, e.g. "一人称ホラー" → horror), it
also reads that `GENRE.md`. Claude gets the prose below the generated tables plus one
line of medians from `tokens.json`. Code takes the hook length (median first cut) and the
title length (median final shot) for the skeleton from the same `tokens.json`.
`shotpilot plan --spec` prints exactly what Claude reads. Genres are matched in the
order horror, cozy, strategy, rpg, action, adventure ("tactics RPG" is strategy, "action
RPG" is rpg, "life sim" stays cozy).

The skeleton also follows where the cutting is dense: each middle slot's length is its
beat's weight divided by the measured edit density of the quarter it falls in
(`profiles.edits_by_quarter`), so a busy quarter gets shorter shots and the calm last
quarter longer ones.

## Styles: recent popular teasers to imitate (D-39)

Twenty analysed packages in `teasers/<id>/` are embedded as styles, four per genre package
(`STYLES` in `crates/shotpilot/src/refs.rs`): game teasers uploaded in 2025 or later,
20-100 s, among the most viewed of their genre, a mix of indie and well-known titles
(films, TV and live action left out). Their `TEASER.md` sections 1-7 were written from the
contact sheet and the measured numbers; the first sentence of section 1 is the one-line
summary the app shows. When `brief.style` names one, the planner also reads its analysis
and measured rhythm ("Inspired by ..."), and the skeleton takes its hook (first cut),
title (final shot) and quarter densities instead of the genre's. Structure, rhythm and
technique are imitated; never the story, characters or words. To add a style: analyse the
package (sections 1-7, remove the `> 分析: 未` line) and add one `style!("<genre>", "<id>")`
line; `cargo test --test styles` checks it.

## craft/ and genres/<id>/

```
craft/TEASER-CRAFT.md   the universal shape. Tables between auto markers are generated;
                        the prose below them is hand written and goes to the planner
craft/tokens.json       distributions over every in-scope teaser (also by length, language,
                        and for gameplay trailers)
genres/<id>/GENRE.md    same layout for one genre: shape, Do, Avoid, in shotpilot's terms
genres/<id>/tokens.json distributions, quarter profiles, exemplars (nearest the medians)
genres/<id>/manifest.json  id, display name, aliases, sample size, member teasers
genres/_index.json      one line per genre
```

A genre gets a package once it has 20 teasers. The keyword scoring left 305 in-scope
videos as `general`; Claude read their titles, tags and descriptions (D-40), and the ones
it put in a genre that had no package yet are kept in `teasers/_genre_overrides.json`
(applied only where the keywords found nothing, so the five earlier packages keep their
members and numbers). That gave strategy / simulation 26 teasers (trading-card product
teasers left out). Still short of 20 once non-games are left out: sci-fi (about 15 games;
the rest are TV series and tech demos), party / co-op (21, but mixed with rhythm and idol
games), racing / sports (17), survival (13).

## teasers/<id>/

```
manifest.json        machine entry: id, name, category (genre), kind, scope, language, source, files, status
TEASER.md            read first. "Teaser Inspired by X": tone, beats with timecodes, rhythm,
                     camera, title cards, sound, and Do / Avoid for shotpilot
tokens.json          measured numbers (ffmpeg): cuts, edits, picture, loudness
source/evidence.md   URL, channel, how it was found and measured
preview/sheet.jpg    8x6 contact sheet. Gitignored (frames of a third-party work)
```

`status` moves `catalogued → downloaded → measured → analyzed`. A package is
`analyzed` once the `> 分析: 未` line is gone from `TEASER.md`.

`category` is scored from the title (x3), YouTube tags (x2), description and search query
(x1); the highest-scoring genre wins, `general` when nothing scores. `scope` is `in`,
`non-game` (anime PVs, films, TV, VTubers, pachislot) or `out-of-range` (under 10 s or over
150 s). Only `scope: in` teasers are aggregated. `kind` is `gameplay` when the title says
so; aggregates use `teaser` only (gameplay trailers are summarised separately in
`craft/tokens.json`).

Worked example: [`teasers/yt-rsDR9xaPIJE`](teasers/yt-rsDR9xaPIJE/TEASER.md).
Catalogue: [`teasers/INDEX.md`](teasers/INDEX.md).

## Reading the measurements

- **Cuts** are peaks of ffmpeg's per-frame scene score that are >= 0.1 and at least 3x the
  median of the 24 surrounding frames. A fixed threshold (0.3) missed most cuts in dark
  footage, where real cuts score 0.15-0.25. Dissolves are not cuts; flashes are.
- **Edits** merge cuts closer than 0.25 s into one edit point (a flash is one edit).
  `edits_per_min`, `median_shot`, `first_cut`, `last_shot` use edits. A **flash burst** is
  3+ cuts each within 0.35 s.
- **Quarter profiles** divide each teaser into four equal parts and divide by the teaser's
  own mean, then take the median over teasers: 1.2 in Q3 means Q3 is 20% denser than average.
- **Brightness** is signalstats YAVG (16-235); a frame under 40 counts as dark.
  **Saturation** is SATAVG.
- **Loudness** is ebur128 integrated LUFS and LRA. "First sound" bottoms out at 0.4 s
  (the momentary window), so 0.4 s means sound from the first frame.
- Source videos are YouTube 360p; numbers are roughly +/-20 % per teaser, stable in aggregate.

## Regenerating

```bash
sh research_data/tools/download.sh            # resumable; videos stay in research_data/ (not in git)
python -I research_data/tools/build_refs.py research_data/teasers references/teasers --measure -j 10
python -I research_data/tools/aggregate_refs.py references
cargo test --test ai                          # the embedded references still parse and match
```

`build_refs.py` rewrites `manifest.json`, `tokens.json` (when missing or from an older
method; `--remeasure` forces it), `evidence.md`, the indexes, and the block between
`<!-- auto:begin -->` and `<!-- auto:end -->` in `TEASER.md`. `aggregate_refs.py` rewrites
`craft/` and `genres/` the same way. Everything outside the auto markers is hand (or
Claude) written and is never overwritten. Adding a genre: give it aliases in
`aggregate_refs.py`, write the prose in its `GENRE.md`, and add one `genre!("<id>")` line
in `refs.rs`.

## Rules

- Videos and frames stay local and are used for analysis only (Japan Copyright Act
  Art. 30-4). Do not commit them or ship them in the submission.
- Write what you saw. Mark what you inferred without watching/listening as 推定.
- Prose that reaches the planner states the shape to write toward. No self-review,
  checklists or scoring (D-22); `references_never_ask_claude_to_inspect_its_own_output`
  keeps it that way.
