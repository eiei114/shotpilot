---
name: threat-reveal
ja_name: 脅威の登場
description: |
  Reveal what hunts the player (a monster, a boss, an entity) by withholding it: traces first,
  a glimpse second, the full presence last or never. Use when the request names an enemy, a
  boss, a monster, or "何かがいる".
triggers: ["敵", "ボス", "怪物", "何かがいる", "monster", "boss", "enemy", "creature", "stalker"]
sp:
  mode: main
  kind: teaser
  genres: [horror, action, rpg]
  design_system: {requires: true}
  example_prompt: 屋敷に何かが住み着いている。最後にだけ、廊下の奥に姿を見せたい。
---

# Threat Reveal

Fear grows with distance from the answer. The threat is told through its effects on the world,
then a partial view, then one held image. The reveal is the climax, not the opening.

## Resource map

- Props: the threat's mesh in `props`, made to `appear` at the right frame.
- Stage: corridors, doorways and dark corners where something can stand at the edge of frame.
- Active design system: the jolt rules (whether one `flash` is allowed).

## Workflow

### Step 0 — Decide how much is shown
Pick one: never shown, glimpsed, or shown once in full. Write it in the plan.

### Step 1 — Beat map
- hook: a trace (a door ajar, a mark, a sound off screen).
- establish: the place the threat lives, empty, slow.
- escalate: the same place again, now with the prop `appear`ing late in the shot, at the edge
  or in the depth of the frame.
- title: black, silence, title; a last `message` if the design system uses one.

### Step 2 — Script
Lines are warnings or rules ("振り返らないで。"). Never name or describe the creature.

### Step 3 — Shots
Repeat a framing so the viewer notices the difference the second time. Use `appear` at 60-80%
of the shot's length; hold after it appears rather than cutting away.

### Step 4 — Finish
One `flash` at most, on the reveal, only if the design system allows it.

## Hard rules

- The threat is never shown in the first half.
- No close-up of a prop that cannot animate.
