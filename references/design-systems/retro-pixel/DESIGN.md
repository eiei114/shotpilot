# Design System Inspired by Retro Pixel

> Category: Action / Adventure
> Crisp pixels, bright palettes, chiptune energy. Charm first; the game's own look is the star.

## 1. Visual Theme & Atmosphere

A teaser that trusts the game's art. Pixel-art or low-poly scenes framed so the pixels stay crisp, cut to a chiptune rhythm. Text is playful and arcade-like. The structure is often a quick premise, a montage of mechanics, a boss tease and the title. The feeling is nostalgia plus surprise.

## 2. Color & Grade

- Grade: `none`. Keep the game's palette exactly as drawn.
- Letterbox: none (0).
- Palette roles: ink `#111022`; paper `#fafaf0`; accent `#ffcc33` (coin yellow) for the title and counts.
- Prefer framings where the art reads at a glance; avoid heavy fog or post effects.

## 3. Typography Rules

### Font family
- Display: `DotGothic16` 400: everything Japanese. The system gothic stands in when the face cannot be fetched.
- Body: `DotGothic16` 400: the same face.
- Latin: `Pixelify Sans` 500: a pixel face, uppercase.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 14 | Display 400 |
| Card, rule | 5 | Display 400 |
| Caption | 5 | Body 400 |
| Count | 4 | Body 400 |
| English line | 2.5 | Latin 500 |

### Principles
- Very short lines: up to 8 Japanese characters or 4 English words.
- Pixel faces stay crisp only at whole multiples of their grid.

## 4. Title & Card Stylings

- `card`: arcade-style lines ("ここから出られるか？" / "CAN YOU ESCAPE?").
- `count`: stages or lives as counts ("STAGE 1", "STAGE 2").
- `caption`: verbs over mechanics.
- `title`: 2.5-3 s, `flash` in.

## 5. Pacing & Transitions

- Shots 1.5-2.5 s; a montage of mechanics in the middle; boss tease in the last quarter.
- Transitions: `cut`; `flash` (white) for a new stage or the title.
- Card holds 1.5-2 s.
- Motion: the `action` profile (references/motion/profiles.json); caption in `type`; title in `flicker` 0.5 s. A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- Keep camera moves simple and steady so pixels do not shimmer: `static`, slow `truck_left` / `truck_right`.
- 35-50 mm to avoid wide-angle distortion of 2D-style art.
- Frame the playfield as the player sees it.

## 7. Sound Character

Chiptune or chip-influenced music, coin and jump sounds placed on actions, one jingle on the title.

## 8. Do's and Don'ts

Do
- Show the game's art as it is.
- Name mechanics with single verbs.
- End with a boss or a twist before the title.

Don't
- Apply colour grades or bars.
- Use realistic serif cinematic cards.
- Use fast orbit moves that smear pixels.

## 9. Agent Prompt Guide

- Quick reference: grade `none`, letterbox 0, typeface `gothic`, transitions `cut` / `flash`.
- Example shot: "Static 40 mm on the first room, the hero jumps across, 2 s, caption 跳ぶ / JUMP."
- Example card: "ここから出られるか？" / "CAN YOU ESCAPE?"
- Iteration: when it feels generic, replace a montage shot with the game's strangest mechanic.
