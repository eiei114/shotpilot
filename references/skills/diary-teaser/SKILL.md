---
name: diary-teaser
ja_name: 一人称の日記
description: |
  A first-person voice in short lines ("Today, I move in."), like diary entries over the
  places where they happen. Use for cozy life sims, personal stories and slow horror, or when
  the request mentions a diary, a first-person voice, days, or "私".
triggers: ["日記", "一人称", "私", "毎日", "diary", "first person", "day one", "today"]
sp:
  mode: main
  kind: teaser
  genres: [cozy, horror, adventure]
  design_system: {requires: true}
  example_prompt: 田舎に引っ越した主人公の日記のように見せたい。1日目、3日目、7日目…
---

# Diary Teaser

The player character speaks, in text, one short entry at a time. The voice makes the world
personal; the gap between what the entries say and what the shots show creates warmth or dread.

## Resource map

- Brief: who the player character is (`core`, `notes`).
- Stage: the anchors of a daily routine (home, work, a path).
- Active design system: card style; cozy and horror looks use the same structure with opposite
  turns.

## Workflow

### Step 0 — Choose the turn
Cozy: the entries grow warmer (lonely → at home). Horror: the entries stay calm while the shots
grow wrong.

### Step 1 — Beat map
- hook: the first entry over the first place ("今日から、ここに住む。").
- establish: two or three days, `count` "1日目" / "3日目" lower left, one shot each.
- escalate: the entries skip days or repeat; the shots change.
- title: the last entry, `black`, `title`.

### Step 2 — Script
Entries as `caption` (on the shot) or `card` (between shots); days as `count`. First person,
present tense, under 15 Japanese characters.

### Step 3 — Shots
Eye-level, 35-50 mm, like the character's own view. Repeat a place on different days with a
changed `time_of_day` or an `appear`.

### Step 4 — Finish
Design system grade; transitions `cut`, `dip` for skipped days.

## Hard rules

- One voice throughout; never switch to a narrator.
- The entries never explain the mechanics.
