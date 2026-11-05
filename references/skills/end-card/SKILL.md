---
name: end-card
ja_name: エンドカード（事実だけ）
description: |
  The facts after the title: release window, platforms, a call to action such as "Wishlist on
  Steam", only as the brief states them. Use whenever the brief contains any of these facts; with
  none, the teaser ends on the title.
triggers: ["エンドカード", "ウィッシュリスト", "対応機種", "end card", "wishlist", "platforms", "call to action"]
sp:
  mode: utility
  kind: any
  genres: []
  design_system: {requires: true}
  example_prompt: 最後に「ウィッシュリスト登録受付中」と出したい。
---

# End Card

The end card is the only part of the teaser that must be literally true. It is copied, not
written.

## Workflow

### Step 1 — Collect the facts
From the brief only: release date or window, platforms, the call to action, a store name. If a
fact is not in the brief, it does not exist.

### Step 2 — Order
Most important first, at most three lines: the call to action or the date, then the platforms.
No social handles; a website at most.

### Step 3 — Placement
A `card` after the `title`, held 2.5-3.5 s, in the design system's body typeface. The English line
is the call to action in English, as the brief gives it.

## Hard rules

- Never invent or guess a date, a window ("2027"), a platform, a price, a rating or an award.
- Never draw platform or store logos; write their names only if the brief has them.
- No facts in the brief: no end card.
