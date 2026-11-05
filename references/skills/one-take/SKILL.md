---
name: one-take
ja_name: ワンカット
description: |
  The whole teaser as one continuous camera journey, broken only by cards on black. Use when the
  level is a connected space (a house, a station, a street) and the request mentions a long
  take, one shot, continuous, or "ワンカット".
triggers: ["ワンカット", "長回し", "一続き", "one take", "long take", "continuous", "oner"]
sp:
  mode: main
  kind: teaser
  genres: [horror, adventure]
  design_system: {requires: true}
  example_prompt: 玄関から七号室の奥まで、カメラが一続きに進んでいくワンカットのティザー。
---

# One Take

The camera travels through the space without cutting, so the viewer feels trapped in real time.
In shotpilot each shot is one move from an anchor, so the "one take" is a chain of shots whose
end and start match: the next shot begins where the last one ended, in the same direction.

## Resource map

- Stage and anchors: a path through connected anchors (entrance → corridor → door → room).
- Active design system: card style; cards on black are the only breaks.

## Workflow

### Step 0 — Draw the path
Order the anchors along a walkable path. Four to six stops.

### Step 1 — Beat map
- hook: the threshold, the camera already moving.
- establish: the path through two stops, steady speed.
- escalate: the path narrows or darkens; the move slows.
- title: the camera stops on the last thing; cut to black; title.

### Step 2 — Script
Two or three `card` lines between legs of the journey, or none. The cards are the only places the
take is allowed to break.

### Step 3 — Shots
Every shot uses the same lens (28-35 mm) and a forward move (`dolly_in` or `truck_left` /
`truck_right`) with `linear` curves, so the speed carries across cuts. Each shot starts where the
previous one ended and keeps its direction.

### Step 4 — Finish
Transitions `cut` only; `dip` only before the title.

## Hard rules

- Never reverse direction between consecutive shots.
- No lens change across the take.
