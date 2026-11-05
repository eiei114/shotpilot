---
name: count-run
ja_name: 数え上げ
description: |
  A run of short evidence shots with a number that keeps counting (記録 1, 記録 2 … or days,
  victims, years), building pressure through repetition. Use with horror, mystery, survival or
  strategy teasers, or when the request mentions a count, a list or evidence.
triggers: ["数え上げ", "カウント", "証拠", "記録", "count", "evidence", "list", "day 1"]
sp:
  mode: utility
  kind: any
  genres: [horror, adventure, strategy]
  design_system: {requires: true}
  example_prompt: 住人が消えた記録を、1件ずつ短く見せていきたい。
---

# Count Run

A count turns separate shots into a series and promises that it will go on. It works best in
the escalate beat, after the world is established.

## Workflow

### Step 1 — Choose what is counted
Something the world would log: records, days, rooms, years, people missing. The count is the
`count` overlay, lower left.

### Step 2 — The run
Four to six shots of 1-1.5 s each, every one a different place or detail, each with the next
number. The numbers may skip to show time passing (1, 2, 3, 7, 41).

### Step 3 — Break the run
End the run on a shot that holds longer (3 s+) or on a number that should not exist, then
`black` or the reveal.

## Hard rules

- One `count` per shot, starting at 0.1-0.3 s into the shot.
- Each counted shot is a different framing; repetition is in the number, not the image.
- `flash` on each counted shot only if the design system uses `flash`.
