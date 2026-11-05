---
name: update-teaser
ja_name: アップデート・DLC
description: |
  New content for a game people already know (an update, a DLC, a new area or mode): a quick
  reminder of the familiar, then the new. Use when the request mentions an update, DLC, a new
  area, a season or "アップデート".
triggers: ["アップデート", "DLC", "追加", "新エリア", "update", "expansion", "new area", "season"]
sp:
  mode: main
  kind: any
  genres: []
  design_system: {requires: true}
  example_prompt: 新しいエリア「地下」を追加するアップデートのティザー。
---

# Update Teaser

The audience knows the base game. Spend a few seconds on the familiar so the new stands out,
then give the new content most of the runtime.

## Resource map

- Brief: what is new (`must_show`, `notes`) and its name exactly as written.
- Stage: which anchors are new; familiar anchors are used only in the first quarter.

## Workflow

### Step 0 — Split the anchors
Familiar (at most two shots) and new (everything else).

### Step 1 — Beat map
- hook: a familiar image, held short.
- establish: a `card` that turns ("まだ、下がある。"), then the first new place.
- escalate: the new content, rising in scale.
- title: the game's `title`, then the update's name as a `card` exactly as the brief writes it.

### Step 2 — Script
One turning `card`, the update's name, and a date only if the brief has one.

### Step 3 — Shots
New places get the longer holds and the stronger moves.

### Step 4 — Finish
Same design system as the base game's teasers; transitions `cut`, `dip` at the turn.

## Hard rules

- Never present base-game content as new.
- The update name and date are copied from the brief.
