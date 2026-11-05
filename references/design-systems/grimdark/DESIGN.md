# Design System Inspired by Grimdark

> Category: RPG / Action
> Ruins, ash and cold steel. A world that has already ended once; the hero is small and the odds are absurd.

## 1. Visual Theme & Atmosphere

A heavy, mythic teaser. Vast ruined architecture, fog, a lone figure, a monstrous scale. The camera is slow and reverent until the clash, then it holds on the impact rather than cutting around it. Text is terse and archaic, like an inscription. The tone is despair with a stubborn ember of defiance; the title arrives like a verdict.

## 2. Color & Grade

- Grade: `horror` (desaturated, cold shadows, grain, vignette). It reads as ash and iron here, not as fear.
- Letterbox: 2.39.
- Palette roles: ink `#08090b`; paper `#d9d3c4` (bone); accent `#b8892e` (tarnished gold) for the title and one ember.
- Lighting: overcast, fog, fire as the only warm source.

## 3. Typography Rules

### Font family
- Display: `Kaisei Tokumin` 800: an extra-bold mincho for the title and the short declarations. The system mincho stands in when the face cannot be fetched.
- Body: `Zen Kaku Gothic Antique` 500: captions and the end card.
- Latin: `Grenze Gotisch` 600: a gothic display with real weight, in title case (never all caps).

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 16 | Display 800 |
| Card, rule | 5 | Display 800 |
| Caption | 5 | Body 500 |
| Count | 4 | Body 500 |
| English line | 2.5 | Latin 600 |

### Principles
- Lines are short and declarative: up to 10 Japanese characters or 5 English words.
- Weight carries the menace; no dripping or distressed type.

## 4. Title & Card Stylings

- `card`: inscriptions on black ("王は死んだ。" / "THE KING IS DEAD.").
- `black`: 1 s of silence before the boss and before the title.
- `title`: 4 s, slow fade from black, gold accent.
- No captions over action; text lives only on black.

## 5. Pacing & Transitions

- First half slow (4-6 s shots of scale and ruin), third quarter faster (1.5-2.5 s around a clash), last quarter one long held image before the title.
- Transitions: `cut`; `dip` before the title; one `flash` allowed on the decisive impact.
- Card holds 2.5-3 s.
- Motion: the `rpg` profile (references/motion/profiles.json); energy `calm`; accent `dip`. A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 18-24 mm low angles for scale; 85 mm for the eyes of the enemy.
- Moves: `crane_up` revealing a colossal structure, slow `dolly_in` toward the lone figure, `orbit_left` / `orbit_right` around a standoff. `ease_in_out`, `ease_in` into an impact.
- Put the hero in the lower third; let architecture dominate.

## 7. Sound Character

Choir or low strings, wind, distant bells, metal. Silence right before the impact; one deep hit on the title.

## 8. Do's and Don'ts

Do
- Sell scale first, then threat, then defiance.
- Keep text like an inscription: few words, heavy.
- Hold the decisive image instead of cutting around it.

Don't
- Use bright saturated colour or playful type.
- Cut fast all the way through.
- Put captions over combat.

## 9. Agent Prompt Guide

- Quick reference: grade `horror`, letterbox 2.39, typeface `mincho`, transitions `cut` / `dip`, one `flash`.
- Example shot: "Low 20 mm crane up from a broken sword to the cathedral ruin in fog, 5 s."
- Example card: "王は死んだ。" / "THE KING IS DEAD."
- Iteration: when it feels busy, cut a combat shot and hold the scale shot longer.
