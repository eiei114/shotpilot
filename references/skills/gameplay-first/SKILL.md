---
name: gameplay-first
ja_name: ゲームプレイ先行
description: |
  A store-page trailer that shows real play from the first seconds: what you do, how it feels,
  why it is different. Use when the brief kind is gameplay, when it is for a Steam page, or when
  the request mentions gameplay, a store page, or "何をするゲームか".
triggers: ["ゲームプレイ", "ストア", "Steam", "store page", "gameplay", "what you do"]
sp:
  mode: main
  kind: gameplay
  genres: []
  design_system: {requires: true}
  example_prompt: Steam のストアページ用。最初の数秒で何をするゲームかが分かるようにしたい。
---

# Gameplay First

Steam recommends that the first trailer on a store page is mostly gameplay, and many viewers
watch it muted and for less than ten seconds. The opening shot must already be play.

## Resource map

- Brief: `core` and `must_show` name the loop. Props that can appear stand in for enemies and
  pickups.
- Stage: which anchors are play spaces (open floor, paths) rather than set dressing.
- Active design system: text style; this skill keeps the grade light so play stays readable.

## Workflow

### Step 0 — Name the loop in three verbs
"Explore, scavenge, survive." These become captions.

### Step 1 — Beat map
- hook (0-3 s): the most distinctive moment of play, already in motion.
- establish: the loop, one verb per shot, 2-3 s each, captions naming the verb.
- escalate: the loop under pressure (more enemies, a bigger space, a twist on a mechanic).
- title: title and facts only.

### Step 2 — Script
Captions over play, one verb each; at most one `card`. The text must read with the sound off.

### Step 3 — Shots
Frame as the player sees it: player height or the game's camera distance, 28-40 mm. Use
`appear` to put an enemy or a pickup into a shot where the verb needs it.

### Step 4 — Finish
Letterbox 0 even if the design system has bars, so the play area is not cropped. Transitions
`cut`.

## Hard rules

- The first shot is gameplay, not a logo or a landscape.
- No cinematic-only shots in the first third.
- Never invent mechanics the brief does not mention.
