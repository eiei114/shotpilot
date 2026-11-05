---
name: rules-teaser
ja_name: 規則で見せる
description: |
  The world has rules, shown one at a time as cards, each proven by a shot, until the last rule
  is broken. Use for horror and mystery built on rules or a routine, or when the request mentions
  rules, "規則", "ルール" or a list of instructions.
triggers: ["規則", "ルール", "決まり", "禁止", "rules", "instructions", "do not"]
sp:
  mode: main
  kind: teaser
  genres: [horror, adventure]
  design_system: {requires: true}
  example_prompt: 七号室には住人だけが知る規則がある。規則を1つずつ見せて、最後に破られる。
---

# Rules Teaser

A rule is a promise of what happens if it is broken. Three or four rules, each a `rule` card
followed by the picture that makes it real, build a world faster than any explanation. The last
rule is broken on screen, or the teaser ends just before it is.

## Resource map

- Brief: the rules, if the game has them (`notes`, `must_show`); otherwise write rules that fit
  the stage and say so in the plan's assumptions.
- Stage: the anchor each rule points at (a door, a window, a clock).
- Active design system: `rule` card styling and holds.

## Workflow

### Step 0 — Write the rules
Three or four short imperatives tied to visible places ("夜 2時、窓を見ないこと。"). Number them
with `head` ("規則 1").

### Step 1 — Beat map
- hook: the place of the last rule, quiet.
- establish: rule 1 → its place; rule 2 → its place. Each rule `card` 2.5-3 s, each shot 3-4 s.
- escalate: rule 3 → the place, now slightly wrong; the last rule cut short or broken.
- title: `black` 1 s, `title`, then a `message` that turns the rules on the viewer.

### Step 2 — Script
Rules are `rule` slots with `head`. Keep each under 15 Japanese characters. The final `message`
is from inside the world (a note left by the previous tenant).

### Step 3 — Shots
One shot per rule, from the anchor it names. The broken rule's shot repeats an earlier framing
with one change (`appear`, a light, `time_of_day`).

### Step 4 — Finish
Transitions `cut`; `dip` before the title; one `flash` on the broken rule only if the design
system allows it.

## Hard rules

- Each rule points at something visible in its shot.
- At most four rules; a fifth weakens all of them.
