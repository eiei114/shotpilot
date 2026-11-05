---
name: release-date
ja_name: 発売日の告知
description: |
  A date reveal: remind people the game exists, raise the stakes once, land the date. Use when
  the brief gives a release date or window and the request mentions launch, release, date or
  "発売日". Without a date in the brief, do not use this skill.
triggers: ["発売日", "発売", "リリース", "release date", "launch", "coming"]
sp:
  mode: main
  kind: teaser
  genres: []
  design_system: {requires: true}
  example_prompt: 発売日の告知ティザー。2027年2月14日、Steam で発売。
---

# Release Date

The audience may already know the game. The teaser re-ignites the premise in a few strong
images and ends on the date, which is the message.

## Resource map

- Brief: the date, platforms and call to action exactly as written (`notes`, `must_show`).
- The previous teaser's premise, if the plan mentions one.
- Active design system: title and card style.

## Workflow

### Step 0 — Copy the facts
Write the date and platforms once, exactly as the brief states them. These are the only facts
the end card may show.

### Step 1 — Beat map
- hook: the most recognisable image of the game.
- establish: two or three images of the premise, faster than an announce teaser.
- escalate: one new image the audience has not seen.
- title: title, then a `card` with the date.

### Step 2 — Script
One premise line, then the title, then the date as its own `card` held 2.5-3 s. A countdown
(`count` "あと 3 日") only if the brief asks for one.

### Step 3 — Shots
Reuse the strongest anchors; one shot from an anchor not used before.

### Step 4 — Finish
Transitions `cut`; `dip` before the title; date card after the title.

## Hard rules

- The date and platforms are copied, never written from memory or guessed.
- No date in the brief: say so in the plan's assumptions and use `announce-teaser` instead.
