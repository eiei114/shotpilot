# Design System Inspired by Quiet Dread

> Category: Horror
> Slow, wide, patient. Nothing jumps; the frame waits until the viewer starts looking for what is wrong.

## 1. Visual Theme & Atmosphere

A teaser that frightens by holding still. Long static or creeping shots of ordinary places (a corridor, a door left ajar, a room at night), framed wide enough that the viewer searches the dark edges. The camera behaves like a witness that does not want to be noticed: low, level, unhurried. Light comes from inside the world (a lamp, a window, a screen), never from a cinematic key. The fear is in what the frame withholds, and in a rule or a line of text that makes an empty room mean something.

## 2. Color & Grade

- Grade: `horror` (cool, desaturated, contrast up, vignette, fine grain).
- Letterbox: 2.39. The bars make the frame feel watched.
- Palette roles: ink `#0b0c0e` for black and cards; paper `#e6e1d6` for text; one accent `#7a1717` used at most once (a sign, a light, one word).
- Exposure stays low. Let windows and practical lights clip; keep faces and corners in shadow.

## 3. Typography Rules

### Font family
- Display: `Shippori Mincho` 500: the title, cards and rules. The system mincho stands in when the face cannot be fetched.
- Body: `Zen Kaku Gothic Antique` 400: the gothic only informs: dates, counts, the end card.
- Latin: `Newsreader` 400: the quiet English line, small.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 16 | Display 500 |
| Card, rule | 5 | Display 500 |
| Caption | 5 | Body 400 |
| Count | 4 | Body 400 |
| English line | 2.5 | Latin 400 |

### Principles
- Cards are small and centred, never bold. Up to 12 Japanese characters or 6 English words per line.
- The title is the only large type in the teaser.
- Mincho carries the voice; the gothic is for facts.

## 4. Title & Card Stylings

- `card`: one plain sentence on black, an English line under it at 60% size.
- `rule`: the device this look is built for. A small `head` ("規則 3") above the rule.
- `black`: 0.8-1.2 s of silence before the reveal and before the title.
- `title`: 3.5-4 s on black, faded in from black, no motion.
- `message`: a last line that turns the premise back on the viewer (a note, a text message).

## 5. Pacing & Transitions

- Shots 3-6 s. The calm is the point; the last quarter is the slowest.
- Transitions: `cut` by default, `dip` through black for a jump in time and before the title. No `flash`, except one jolt at most in the last third.
- Card holds 2.5-3 s.
- Motion: the `horror` profile (references/motion/profiles.json); title in `fade` 1.2 s, hold `still`, out `fade`; never `glitch`. A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 24-35 mm for rooms, 50 mm for a detail that should not be there.
- Moves: `static`, slow `dolly_in` (60-120 cm over the whole shot), `truck_left` / `truck_right` along a corridor. `ease_in_out`.
- Eye level or slightly below. Leave negative space where something could appear.

## 7. Sound Character

Room tone, a distant hum, one sound that does not belong (a knock, a phone). Music, if any, is a low drone that drops out before the reveal.

## 8. Do's and Don'ts

Do
- Hold an empty frame longer than feels comfortable.
- Tie every card to a picture that proves it.
- Keep the threat off screen or at the edge.

Don't
- Use fast montage, whooshes or a flash on every cut.
- Show a monster in full light.
- Use more than one accent colour.

## 9. Agent Prompt Guide

- Quick reference: grade `horror`, letterbox 2.39, typeface `mincho`, transitions `cut` / `dip`.
- Example shot: "Wide static at 28 mm down the corridor at night, door at the far end ajar, 5 s, no caption."
- Example card: "この部屋には、規則がある。" / "This room has rules."
- Iteration: when a cut feels flat, slow it down and remove a card rather than adding motion.
