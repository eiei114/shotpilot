# Design System Inspired by Epic Fantasy

> Category: RPG / Adventure
> Sweeping vistas, a call to adventure, a world worth saving. Hope first, danger second, the journey always.

## 1. Visual Theme & Atmosphere

A teaser that opens a map. Grand landscapes in golden light, a party or a lone traveller against the horizon, cities and creatures that promise a long journey. The camera soars and glides; the cut builds like an overture. Text names the stakes in a heroic register. The feeling is longing to go there.

## 2. Color & Grade

- Grade: `warm` (golden, saturated) for most; `vivid` when the game is bright and colourful.
- Letterbox: 2.39.
- Palette roles: ink `#0e0c0a`; paper `#f5ecd8`; accent `#d9a441` (gold) for the title.
- Lighting: sunrise and sunset, god rays, lantern-lit towns at night.

## 3. Typography Rules

### Font family
- Display: `Shippori Mincho` 800: the title and the call (for a Japanese-styled world, Yuji Syuku instead). The system mincho stands in when the face cannot be fetched.
- Body: `Zen Kaku Gothic Antique` 500: captions and the end card.
- Latin: `Marcellus` 400: flared Roman capitals, calmer than the common Trajan look.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 13 | Display 800 |
| Card, rule | 5 | Display 800 |
| Caption | 5 | Body 500 |
| Count | 4 | Body 500 |
| English line | 2.5 | Latin 400 |

### Principles
- Lines are big promises, short: up to 12 Japanese characters or 6 English words.
- Avoid gold Trajan-style capitals: the most common fantasy title look (4 of 10 measured RPG teasers).

## 4. Title & Card Stylings

- `card`: the call ("世界の果てへ。" / "To the edge of the world.").
- `caption`: rarely; let vistas speak.
- `black`: 0.8 s before the title.
- `title`: 4 s, fade from black, gold.
- End card: facts only, under the title.

## 5. Pacing & Transitions

- Opening vistas 4-5 s; middle builds with 2-3 s shots of places and creatures; third quarter peaks with motion; final image long.
- Transitions: `cut`; `dip` between "chapters" and before the title. No `flash` unless the game is action-heavy.
- Card holds 2.5 s.
- Motion: the `rpg` profile (references/motion/profiles.json). A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 18-24 mm for vistas, 35 mm for the party, 85 mm for a face looking out.
- Moves: `crane_up` over a ridge to reveal the valley, long `truck_left` / `truck_right` along a caravan, `orbit_left` / `orbit_right` around a landmark. `ease_in_out`.
- Horizon on a third; subject facing into the frame.

## 7. Sound Character

Orchestral swell, horns, choir on the title; wind and birds under vistas. The music decides the cut points.

## 8. Do's and Don'ts

Do
- Open with the world, then the traveller.
- Make each place distinct (biome, light, scale).
- Build to one peak, then breathe before the title.

Don't
- Show menus or stats.
- Use horror grades or cold palettes.
- Cut vistas shorter than 3 s.

## 9. Agent Prompt Guide

- Quick reference: grade `warm`, letterbox 2.39, typeface `mincho`, transitions `cut` / `dip`.
- Example shot: "Sunrise, 20 mm crane up over the ridge revealing the river valley and a distant city, 5 s."
- Example card: "世界の果てへ。" / "To the edge of the world."
- Iteration: when the middle drags, swap a vista for a creature or a city.
