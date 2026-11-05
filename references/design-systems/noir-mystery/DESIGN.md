# Design System Inspired by Noir Mystery

> Category: Adventure / Mystery
> Rain, lamplight and secrets. Each shot is a clue; the text asks the question the viewer now wants answered.

## 1. Visual Theme & Atmosphere

A teaser built as an investigation. Night streets, interiors with hard light and deep shadow, objects that look like evidence (a letter, a photograph, a key). The camera lingers on details and moves like someone searching a room. Text is a question or a line from a witness. The feeling is suspicion and the itch to know.

## 2. Color & Grade

- Grade: `cool` (blue cast, slightly desaturated).
- Letterbox: 2.39.
- Palette roles: ink `#0a0d12`; paper `#e8e2d2`; accent `#d08a2e` (amber lamplight) for one clue and the title.
- Lighting: practical lamps, window blinds, neon in rain; strong shadows.

## 3. Typography Rules

- Japanese: mincho (Noto Serif JP). Latin: a typewriter face (Courier Prime / Special Elite) for clues, a high-contrast serif (Playfair Display) for the title.
- Questions and statements of fact; up to 14 Japanese characters or 7 English words.

## 4. Title & Card Stylings

- `card`: the question ("あの夜、誰が嘘をついた？" / "Who lied that night?").
- `count`: clues numbered ("証拠 1", "証拠 2") over detail shots.
- `message`: the last line is a note or a phone message found at the scene.
- `title`: 3.5 s, fade in, amber.

## 5. Pacing & Transitions

- Detail shots 1.5-2.5 s; establishing shots 3-4 s; the reveal held 3 s.
- Transitions: `cut`; `dip` for a jump in time. No `flash` unless the twist is a jolt.
- Card holds 2.5 s.

## 6. Camera Language

- 50-85 mm on details, 28-35 mm for rooms and streets.
- Moves: slow `dolly_in` on a clue, `truck_left` / `truck_right` past a window, `static` on a face in shadow. `ease_in_out`.
- Frame through doorways and blinds; partial views.

## 7. Sound Character

Rain, a ticking clock, muted jazz or a solo saxophone, footsteps. A sting on the twist.

## 8. Do's and Don'ts

Do
- Make every detail shot a clue that pays off.
- Pose one question and do not answer it.
- Use shadow to hide as much as it shows.

Don't
- Show the culprit.
- Use bright, flat lighting.
- Rush the details.

## 9. Agent Prompt Guide

- Quick reference: grade `cool`, letterbox 2.39, typeface `mincho`, transitions `cut` / `dip`.
- Example shot: "85 mm slow dolly in on a torn photograph under a desk lamp, 2.5 s, count 証拠 1."
- Example card: "あの夜、誰が嘘をついた？" / "Who lied that night?"
- Iteration: when it feels vague, make the clues more specific and fewer.
