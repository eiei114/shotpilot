# Design System Inspired by Neon Action

> Category: Action
> Speed, light and impact. Every cut lands on a beat; the screen hums with colour.

## 1. Visual Theme & Atmosphere

A high-energy teaser built on rhythm. Saturated neon against deep blacks, motion in every frame, impacts timed to the music. Text is short, loud and kinetic, often one word per beat. The cut accelerates through the third quarter and stops dead for the title. The feeling is adrenaline and style.

## 2. Color & Grade

- Grade: `vivid` (contrast and saturation up, a little sharper).
- Letterbox: none (0); full frame keeps the energy.
- Palette roles: ink `#06060a`; paper `#f2f2f7`; accent `#ff2e88` (magenta) and `#22e4ff` (cyan), one per card.
- Lighting: night, neon signs, muzzle flashes, emissive materials.

## 3. Typography Rules

### Font family
- Display: `Dela Gothic One` 400: the title and the one-word punches. The system gothic stands in when the face cannot be fetched.
- Body: `M PLUS 1` 700: captions, counts and the end card.
- Latin: `Big Shoulders` 800: condensed, all caps.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 15 | Display 400 |
| Card, rule | 5 | Display 400 |
| Caption | 5 | Body 700 |
| Count | 4 | Body 700 |
| English line | 2.5 | Latin 800 |

### Principles
- One to three words per card. Up to 6 Japanese characters or 3 English words.

## 4. Title & Card Stylings

- `card`: one-word punches (速く。強く。 / FASTER. HARDER.), each 0.8-1.2 s.
- `caption`: verbs over action, one per shot.
- `black`: 0.3-0.5 s right before the title (the drop).
- `title`: 2.5-3 s, hard cut in with a `flash`.

## 5. Pacing & Transitions

- Shots 0.8-2 s in the build, 2.5-3 s for the hook and the last action image.
- Transitions: `cut` on the beat; `flash` on impacts and on the title (sparingly: at most one in four cuts). No `dip` except the drop.
- Card holds 0.8-1.2 s.
- Motion: the `action` profile (references/motion/profiles.json); card in `slam` 0.15 s; title in `slam` 0.15 s. A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 18-28 mm close to the action; 50 mm for a hero pose.
- Moves: fast `dolly_in`, `orbit_left` / `orbit_right` around a fighter, `truck_left` / `truck_right` alongside a chase. `ease_in` into hits, `linear` for chases.
- Diagonal compositions; subject moving across the frame.

## 7. Sound Character

Synthwave or electronic with a strong kick; risers into the drop; one big hit on the title. Whooshes on cuts are allowed here.

## 8. Do's and Don'ts

Do
- Cut on the beat.
- Keep text to single punches.
- Stop dead before the title.

Don't
- Hold quiet shots in the middle.
- Use serif type or long sentences.
- Flash on every cut.

## 9. Agent Prompt Guide

- Quick reference: grade `vivid`, letterbox 0, typeface `gothic`, transitions `cut` / `flash`.
- Example run: "Four 1 s shots of a chase at 24 mm with truck moves, captions 走る / 撃つ / 跳ぶ / 壊す."
- Example card: "速く。" / "FASTER."
- Iteration: when it blurs together, insert one held 2.5 s shot before the drop.
