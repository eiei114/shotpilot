---
name: mood-teaser
ja_name: 空気で見せる
description: |
  Atmosphere only: places, light and sound make the viewer feel the game before they know what
  it is. Use for horror, mystery or art games with little to show yet, or when the brief asks
  for mood, atmosphere or "雰囲気".
triggers: ["雰囲気", "空気感", "mood", "atmosphere", "vibe", "tone piece"]
sp:
  mode: main
  kind: teaser
  genres: [horror, adventure, cozy]
  design_system: {requires: true}
  example_prompt: 廃校を舞台にしたホラー。まだ遊べる部分は少ないので、空気だけで怖がらせたい。
---

# Mood Teaser

No mechanics, no characters explained, almost no text. The teaser is a walk through the world
at the speed of feeling. It works when the places are strong and the edit is patient.

## Resource map

- Stage: lighting and what each anchor looks like; the scout photos decide the order.
- Mood board: palette and time of day.
- Active design system: grade and pacing; this skill favours its slowest holds.

## Workflow

### Step 0 — Choose the arc of light
Pick one change that runs through the cut: day to night, warm to cold, quiet to wrong.

### Step 1 — Beat map
- hook: a single detail, still, with a sound that does not belong.
- establish: three or four places, 4-6 s each, the camera barely moving.
- escalate: the same kind of place, but something is off (a light, a door, a shape).
- title: black, silence, title.

### Step 2 — Script
At most two lines in the whole teaser, or none. If there is one, it names the place or the
feeling, never the plot.

### Step 3 — Shots
`static` and slow `dolly_in` / `truck_left` / `truck_right`. Every shot leaves empty space. Use
`appear` once at most, late, at the edge of frame.

### Step 4 — Finish
Grade from the design system; transitions `cut`, `dip` before the title. No `flash`.

## Hard rules

- Shots at least 3 s, except one short jolt if the design system allows it.
- No feature claims in text.
- Do not show the threat in full.
