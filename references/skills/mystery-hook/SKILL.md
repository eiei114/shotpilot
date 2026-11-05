---
name: mystery-hook
ja_name: 謎で引っぱる
description: |
  Built around one question the viewer cannot stop thinking about. Every shot adds a clue and
  none answers it. Use for mystery, narrative and puzzle games, or when the request mentions a
  secret, a question, a disappearance or "謎".
triggers: ["謎", "秘密", "失踪", "なぜ", "mystery", "secret", "missing", "who", "why"]
sp:
  mode: main
  kind: teaser
  genres: [adventure, horror]
  design_system: {requires: true}
  example_prompt: 前の住人はどこへ消えたのか。答えは見せずに、手がかりだけで引っ張りたい。
---

# Mystery Hook

The teaser asks one question early and makes it sharper with every shot. The title arrives as
the only answer the viewer gets: play to find out.

## Resource map

- Brief: the central question (`core`), the clues (`must_show`).
- Stage: detail anchors that can hold a clue (a desk, a photo, a door).
- Props: an object that should not be there, made to `appear`.

## Workflow

### Step 0 — Write the question
One sentence the viewer would type into a search box ("前の住人はどこへ消えた？").

### Step 1 — Beat map
- hook: the question as a `card`, or an image that asks it.
- establish: the place where it happened, empty.
- escalate: three or four clues, each a detail shot, each changing the possible answer.
- title: a last clue that reframes all of them, `black`, `title`.

### Step 2 — Script
The question once, near the start. Clues may carry a `count` ("手がかり 1"). The last line may be
a `message` that deepens the question.

### Step 3 — Shots
Detail shots at 50-85 mm with slow `dolly_in`; one wide of the place at 24-28 mm. Clues are
things in the level; never invent objects the stage does not have, except props that `appear`.

### Step 4 — Finish
Transitions `cut`; `dip` for time jumps.

## Hard rules

- Never answer the question.
- Every clue is specific and visible.
