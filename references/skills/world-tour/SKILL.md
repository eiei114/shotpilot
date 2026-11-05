---
name: world-tour
ja_name: 世界をめぐる
description: |
  A tour of the world's places, each with its own light and feeling, building to the most
  impressive one. Use for open worlds, exploration, adventure and RPGs, or when the request
  mentions places, the world, regions or "世界観".
triggers: ["世界観", "舞台", "場所", "world", "places", "regions", "explore", "open world"]
sp:
  mode: main
  kind: teaser
  genres: [adventure, rpg, cozy, strategy]
  design_system: {requires: true}
  example_prompt: 島を巡る冒険ゲーム。森、港町、火山の3つの場所を見せたい。
---

# World Tour

The world is the main character. The teaser visits places in an order that rises: familiar,
then wondrous, then the one that makes people say "I want to go there".

## Resource map

- Stage and scout photos: all anchors, their light, and which one is the most striking.
- Mood board: the colour journey.
- Active design system: pacing; vistas get its longest holds.

## Workflow

### Step 0 — Order the places
Sort the anchors by how much they impress; the most striking is last before the title. Two
places can share a beat if they contrast (forest then fire).

### Step 1 — Beat map
- hook: a small, intimate detail of the first place.
- establish: two places, wide, 4-5 s each, with time of day changing.
- escalate: faster visits (2-3 s) to three or four places, rising in scale.
- title: the most striking place held long, then black, then the title.

### Step 2 — Script
Optionally a `caption` naming each place (two to four words). One `card` that invites ("この島の
どこへでも。").

### Step 3 — Shots
`crane_up` and wide `truck_left` / `truck_right` for vistas, 18-28 mm. Change `time_of_day`
between places to keep the tour from feeling flat.

### Step 4 — Finish
Transitions `cut`; `dip` between distant places.

## Hard rules

- Every place shown must be an anchor; do not describe places that are not in the level.
- No two consecutive shots of the same place from a similar angle.
