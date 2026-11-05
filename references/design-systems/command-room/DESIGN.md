# Design System Inspired by Command Room

> Category: Strategy / Simulation
> From one small decision to an empire. Clear, confident, systemic: the viewer sees the loop and the scale it grows into.

## 1. Visual Theme & Atmosphere

A teaser that sells a system. It starts with one unit, one building or one choice, and zooms out as the consequences multiply. The camera rises from ground level to a commander's view. Text names the verbs of the loop and the stakes. The feeling is ambition and control, with a hint of the chaos the player will have to manage.

## 2. Color & Grade

- Grade: `none` (keep readability of units and terrain); `vivid` for a colourful, toy-like game.
- Letterbox: none (0).
- Palette roles: ink `#0d1117`; paper `#f4f6f8`; accent `#3a7bd5` (command blue) and `#e0a526` (resource gold).
- Lighting: clear daylight for readability, dusk for the final scale shot.

## 3. Typography Rules

- Japanese: solid gothic (Noto Sans JP Bold). Latin: a sturdy condensed sans (Oswald / Barlow Condensed), uppercase.
- Verbs and numbers: "築く。広げる。守る。" / "BUILD. EXPAND. DEFEND." Up to 8 Japanese characters or 4 English words.

## 4. Title & Card Stylings

- `card`: one verb per card in a series, or the stakes ("100年、生き延びろ。" / "Survive a hundred years.").
- `count`: years, population or turns over shots that grow ("1年目", "50年目", "100年目").
- `title`: 3 s, centred, after the widest shot.

## 5. Pacing & Transitions

- Start close and slow (3-4 s), speed up as the system grows (1.5-2.5 s), end on the widest view held long.
- Transitions: `cut`; `dip` for a jump in time. `flash` only for a disaster.
- Card holds 2 s.

## 6. Camera Language

- 35-50 mm at ground level at the start; 18-24 mm high views later.
- Moves: `crane_up` from a single unit to the whole settlement (the signature move), `truck_left` / `truck_right` over a front line, `orbit_left` / `orbit_right` around a capital.
- Keep units readable: avoid extreme close-ups on low-detail models.

## 7. Sound Character

Building percussion and strings that grow layer by layer; crowd and construction foley; a horn on the title.

## 8. Do's and Don'ts

Do
- Show the loop in verbs, then the scale it creates.
- Use counts to show time and growth.
- End on the biggest view.

Don't
- Show menus, tooltips or spreadsheets.
- Start wide; the growth needs a small beginning.
- Use horror grades.

## 9. Agent Prompt Guide

- Quick reference: grade `none`, letterbox 0, typeface `gothic`, transitions `cut` / `dip`.
- Example shot: "40 mm at ground level on the first hut, crane up to the whole valley full of buildings, 4 s, count 100年目."
- Example card: "築く。" / "BUILD."
- Iteration: when the growth is unclear, add a count to each step.
