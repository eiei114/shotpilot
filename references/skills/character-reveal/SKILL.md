---
name: character-reveal
ja_name: キャラクター紹介
description: |
  Introduce one character (a hero, a companion, a villain): who they are, what they want, what
  they can do. Use when the request names a character or asks for a character trailer or
  "キャラクター紹介".
triggers: ["キャラクター", "主人公", "紹介", "character", "hero", "villain", "introducing"]
sp:
  mode: main
  kind: teaser
  genres: [rpg, action, adventure]
  design_system: {requires: true}
  example_prompt: 新しい仲間キャラクターの紹介ティザー。無口な鍛冶屋で、戦うと強い。
---

# Character Reveal

The character is the hook and the title is their name (or the game's, with their name before
it). Without facial animation, the character is told through place, silhouette, objects and
the reactions of the world around them.

## Resource map

- Props: the character's mesh, if it is in `props` (it stands in its reference pose; frame
  around that).
- Stage: the anchors that belong to this character (their home, their workplace).
- Active design system: card and title style.

## Workflow

### Step 0 — Write the character in one line
"A silent blacksmith who only speaks through what she forges."

### Step 1 — Beat map
- hook: an object or a place that belongs to them, before we see them.
- establish: their world, then a silhouette or partial view (`appear` at the edge).
- escalate: the full figure, held, with the line that defines them.
- title: their name as a `card`, then the game's `title`.

### Step 2 — Script
Two or three `card` lines in their voice or about them; the name as a `card` before the title.

### Step 3 — Shots
Since the prop does not move, sell it through the camera: a slow `dolly_in` or `orbit_left` /
`orbit_right` around the figure, 50-85 mm. Keep the reveal of the full figure for the third
quarter.

### Step 4 — Finish
Design system grade; transitions `cut`, `dip` before the name.

## Hard rules

- Do not describe animation, dialogue or abilities the brief does not list.
- Never animate or pose the prop; it appears as it is.
