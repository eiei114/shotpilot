---
name: verb-montage
ja_name: 動詞のモンタージュ
description: |
  The game's loop named one verb at a time (DIG / GROW / BUILD), each over the play it names,
  accelerating to the title. Use for cozy, crafting, simulation and strategy games, or when the
  request lists what the player does.
triggers: ["できること", "遊び方", "作る", "育てる", "verbs", "build", "craft", "grow", "loop"]
sp:
  mode: main
  kind: any
  genres: [cozy, strategy, action]
  design_system: {requires: true}
  example_prompt: 掘って、育てて、売って、町を大きくする。やれることを順に見せたい。
---

# Verb Montage

Each verb is a promise of play, proven by a shot. The rhythm of verbs is the rhythm of the cut:
slow at first, then faster, then one final verb held before the title.

## Resource map

- Brief: the verbs (`core`, `must_show`).
- Stage: one anchor per verb, where that action happens.
- Props: things a verb produces, made to `appear` (a crop, a building).
- Active design system: caption typeface and the accent colour for verbs.

## Workflow

### Step 0 — List four to six verbs
In the order the player meets them. The last verb is the biggest (BUILD A TOWN, DEFEND).

### Step 1 — Beat map
- hook: the first verb, 3 s, on a small action.
- establish: two verbs, 2.5 s each.
- escalate: two or three verbs, 1.5-2 s each, rising in scale.
- title: the final verb held 3 s on the widest view, then the title.

### Step 2 — Script
Each verb is a `caption` on its shot (Japanese + English line). No `card` between verbs; the
pace comes from the shots.

### Step 3 — Shots
Each shot shows its verb's result appearing (`appear` at 40-60% of the shot) or the camera
revealing it (`crane_up`). The last shot is the widest of the teaser.

### Step 4 — Finish
Transitions `cut`; `flash` only if the design system uses it.

## Hard rules

- One verb per shot; never two captions on one shot.
- Only verbs the brief supports.
