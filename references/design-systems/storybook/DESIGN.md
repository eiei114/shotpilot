# Design System Inspired by Storybook

> Category: Adventure / Cozy
> A picture book opened page by page. Each shot is a tableau; text reads like the line under an illustration.

## 1. Visual Theme & Atmosphere

The teaser is told like a fairy tale: once there was a place, someone set out, something waited. Shots are composed as tableaux with clear silhouettes and depth, often wide, with the hero small in a large world. Movement is slow and graceful. Text is sparse and literary, set like a caption in a book. The emotion is wonder with a thread of melancholy.

## 2. Color & Grade

- Grade: `warm` for most stories; `none` when the game's own palette is already painterly.
- Letterbox: none (0).
- Palette roles: ink `#2a2622`; paper `#f3eadb` (aged paper); accent `#c46a3a` (terracotta) for the title only.
- Favour dawn, dusk and lantern light; avoid flat noon.

## 3. Typography Rules

### Font family
- Display: `Zen Old Mincho` 700: the title and the told lines. The system mincho stands in when the face cannot be fetched.
- Body: `Zen Maru Gothic` 500: captions and the end card.
- Latin: `Young Serif` 400: an old-style serif for the English line.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 18 | Display 700 |
| Card, rule | 5 | Display 700 |
| Caption | 5 | Body 500 |
| Count | 4 | Body 500 |
| English line | 2.5 | Latin 400 |

### Principles
- Sentences may be full and gentle, but short: up to 14 Japanese characters or 7 English words.
- No all-caps.

## 4. Title & Card Stylings

- `card`: storytelling lines on black, in order ("むかし、灯りの消えた町があった。").
- `caption`: a short line under a tableau, like a page caption.
- `black`: 0.6-0.8 s between pages.
- `title`: 3.5-4 s, faded in, in the accent colour.

## 5. Pacing & Transitions

- Shots 3-5 s, each a page. A gentle build in the third quarter, then a long final image.
- Transitions: `dip` through black between chapters, `cut` within one. No `flash`.
- Card holds 2.5-3 s.
- Motion: the `cozy` profile (references/motion/profiles.json); energy `calm`; card in `fade` 0.6 s; title in `fade` 0.8 s, hold `drift`. A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 24-35 mm wide tableaux; 50 mm for a single object of the story (a key, a lantern).
- Moves: `crane_up` / `crane_down` to open and close a page, slow `truck_left` / `truck_right` across a scene like turning a page. `ease_in_out`.
- Keep horizons level and silhouettes readable.

## 7. Sound Character

Solo piano or strings, a music box, wind. Music carries the story; sound effects are few and soft.

## 8. Do's and Don'ts

Do
- Compose each shot as a page that could stand alone.
- Tell a tiny story with a beginning, a turn and an ending.
- Keep the hero small against the world.

Don't
- Use fast cutting or hard jolts.
- Explain mechanics in the text.
- Crowd the frame with UI.

## 9. Agent Prompt Guide

- Quick reference: grade `warm`, letterbox 0, typeface `mincho`, transitions `dip` / `cut`.
- Example shot: "Dusk, 28 mm, slow crane up from the lantern to the whole sleeping town, 4.5 s."
- Example card: "むかし、灯りの消えた町があった。" / "Once, a town lost its light."
- Iteration: when it feels like a list, connect the cards into one sentence across the cut.
