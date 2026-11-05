# Design System Inspired by Arthouse Minimal

> Category: Adventure / Any
> Few shots, long holds, lots of space. Confidence through restraint; the teaser feels like the opening of a film.

## 1. Visual Theme & Atmosphere

A teaser that says almost nothing and makes the viewer lean in. Five to eight shots, each a precise composition with a lot of negative space: a figure in a field, a table in an empty room, light moving across a wall. Text is rare, small and exact. The meaning comes from the order of images and one line that reframes them. The feeling is curiosity and quiet weight.

## 2. Color & Grade

- Grade: `none` to keep the game's look; `cool` when the game is muted already.
- Letterbox: 2.39.
- Palette roles: ink `#111111`; paper `#f0eee9`; no accent. Colour comes from the images.
- Lighting: natural, soft, directional; no effects.

## 3. Typography Rules

- Japanese: a light mincho (Noto Serif JP Light). Latin: a clean grotesque (Inter / Space Grotesk), lowercase or sentence case, small.
- One line per card, up to 14 Japanese characters or 8 English words. Text is set small.

## 4. Title & Card Stylings

- `card`: at most two in the whole teaser. Placed where the images need context.
- `caption`: none.
- `black`: 1-1.5 s, used as punctuation.
- `title`: 3.5-4 s, small, centred, faded in.

## 5. Pacing & Transitions

- Shots 4-7 s. The rhythm is set by the images, not by music.
- Transitions: `cut` only, plus `dip` before the title. Never `flash`.
- Card holds 3 s.

## 6. Camera Language

- 35-50 mm, level, symmetrical or deliberately off-centre compositions.
- Moves: mostly `static`; one slow `dolly_in` at the turning point.
- Generous headroom and empty space; the subject may be small.

## 7. Sound Character

Natural ambience, a single sustained tone, or silence. Music, if any, enters late and sparse.

## 8. Do's and Don'ts

Do
- Choose fewer, better shots.
- Let one line change how the images read.
- Trust silence.

Don't
- Fill the frame with action or UI.
- Use captions or counts.
- Cut to the beat of a track.

## 9. Agent Prompt Guide

- Quick reference: grade `none`, letterbox 2.39, typeface `mincho`, transitions `cut` / `dip`.
- Example shot: "Static 40 mm, an empty dining table, morning light moving across it, 6 s."
- Example card: "誰も、帰ってこなかった。" / "No one came back."
- Iteration: when it feels thin, hold longer rather than adding a shot.
