# Design System Inspired by Cozy Pastel

> Category: Cozy
> Warm morning light, soft colour, small pleasures one at a time. The viewer should want to live there.

## 1. Visual Theme & Atmosphere

A teaser that invites rather than impresses. Each shot shows one small, satisfying thing (a garden bed, a shop counter, a cat on a step) in warm daylight. The camera moves gently and stays at a friendly distance, as if visiting. Text is friendly and short, often a first-person voice or a verb that names the play. The cut feels like a day passing: morning, work, evening, a light in a window.

## 2. Color & Grade

- Grade: `warm` (brighter colour, a sunny cast).
- Letterbox: none (0).
- Palette roles: ink `#3a2f2a` (soft brown instead of black) for text; paper `#fff7ea` (cream); accent `#f29e8e` (peach) and `#8cc7a1` (mint) for one word or a shape.
- Time of day: morning and golden hour; night only for the last cosy shot.

## 3. Typography Rules

### Font family
- Display: `Zen Maru Gothic` 900: the title and the friendly single words. The system gothic stands in when the face cannot be fetched.
- Body: `M PLUS Rounded 1c` 500: captions and the end card.
- Latin: `Gabarito` 700: mixed case; never all caps on the title.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 20 | Display 900 |
| Card, rule | 5 | Display 900 |
| Caption | 5 | Body 500 |
| Count | 4 | Body 500 |
| English line | 2.5 | Latin 700 |

### Principles
- Big, friendly single words work; long sentences do not.
- Up to 10 Japanese characters or 4 English words per card.

## 4. Title & Card Stylings

- `card`: one warm line ("今日から、ここに住む。" / "Today, I move in.").
- Verbs one at a time over the play they name (植える / 育てる / 売る; PLANT / GROW / SELL) as a `caption` on each shot.
- `title`: 3-3.5 s, centred, after a short `black` of 0.5 s or straight from the last shot.
- End with a quiet evening image and a gentle `message` ("また明日。").

## 5. Pacing & Transitions

- Shots 2.5-4 s; even, unhurried rhythm. No sudden accelerations.
- Transitions: `cut`; `dip` through black only for the day turning to night. No `flash`.
- Card holds 2-2.5 s.
- Motion: the `cozy` profile (references/motion/profiles.json). A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 35-50 mm, eye level or a little above, like a visitor.
- Moves: slow `orbit_left` / `orbit_right` around a small scene, `crane_up` to reveal the village, gentle `dolly_in` to a detail. `ease_in_out`.
- Frame with breathing room; keep the subject in the lower two thirds.

## 7. Sound Character

Acoustic guitar, light percussion, birds, small foley for each action (a seed in soil, a door bell). One soft chime on the title.

## 8. Do's and Don'ts

Do
- Show one pleasure per shot.
- Let the day progress through the cut.
- Keep text kind and concrete.

Don't
- Use dark grades, bars or jolts.
- Rush; fast montage reads as stress.
- Write generic claims ("Relaxing gameplay!").

## 9. Agent Prompt Guide

- Quick reference: grade `warm`, letterbox 0, typeface `gothic`, transitions `cut` / `dip`.
- Example shot: "Morning, 40 mm, slow orbit around the vegetable bed, 3.5 s, caption 育てる / GROW."
- Example card: "今日から、ここに住む。" / "Today, I move in."
- Iteration: when the cut feels busy, drop a shot and let the remaining ones breathe.
