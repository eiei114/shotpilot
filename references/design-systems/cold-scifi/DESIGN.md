# Design System Inspired by Cold Sci-Fi

> Category: Sci-Fi (Action / Adventure / Strategy)
> Clean geometry, cold light, a vast silence. Technology at a scale that makes people small.

## 1. Visual Theme & Atmosphere

A teaser of precision and isolation. Long corridors of a station, a planet through a viewport, machines larger than the frame. Text reads like system messages or mission logs. The camera glides with mechanical smoothness. The feeling is awe with an undertone of something gone wrong in the system.

## 2. Color & Grade

- Grade: `cool` (blue cast, slightly desaturated).
- Letterbox: 2.39 for cinematic sci-fi; 0 for a systems-heavy game.
- Palette roles: ink `#05080c`; paper `#e6eef5`; accent `#3fd0ff` (interface cyan) and `#ff5a3c` (warning orange) used only for an alarm.
- Lighting: clean white panels, emissive lines, starlight; one red emergency light at the turn.

## 3. Typography Rules

### Font family
- Display: `M PLUS 1` 800: the title and the system messages. The system gothic stands in when the face cannot be fetched.
- Body: `IBM Plex Sans JP` 400: captions, counts and the end card.
- Latin: `Chakra Petch` 500: a squared technical sans, uppercase.

### Hierarchy
| Role | Glyph height (% of frame) | Face |
|---|---|---|
| Title | 14 | Display 800 |
| Card, rule | 5 | Display 800 |
| Caption | 5 | Body 400 |
| Count | 4 | Body 400 |
| English line | 2.5 | Latin 500 |

### Principles
- System-message style: `SIGNAL LOST`, `記録 0417`. Up to 12 Japanese characters or 5 English words.

## 4. Title & Card Stylings

- `card`: mission-log lines ("最後の通信から、40年。" / "40 years since last contact.").
- `count`: log numbers or sectors over detail shots.
- `black`: 0.8 s, like a signal dropping.
- `title`: 3 s, clean fade, cyan.

## 5. Pacing & Transitions

- Shots 3-5 s for scale, 1.5-2 s for system details; one alarm moment in the third quarter.
- Transitions: `cut`; `dip` for signal loss; one `flash` for the alarm.
- Card holds 2.5 s.
- Motion: the `scifi` profile (references/motion/profiles.json). A line may ask for its own entrance (`motion:`) for one or two accents.

## 6. Camera Language

- 18-24 mm for scale, 50 mm for interface details.
- Moves: smooth `truck_left` / `truck_right` along corridors with `linear` curves, `crane_up` past a machine, slow `orbit_left` / `orbit_right` around a vessel.
- Symmetry and vanishing points.

## 7. Sound Character

Low synth pads, electrical hum, beeps of interfaces, a distorted radio voice. Silence in space.

## 8. Do's and Don'ts

Do
- Use symmetry and scale.
- Write text as the machine would say it.
- Save the one warning colour for the turn.

Don't
- Use warm grades or hand-written type.
- Fill the soundtrack with action music.
- Explain the plot.

## 9. Agent Prompt Guide

- Quick reference: grade `cool`, letterbox 2.39, typeface `gothic`, transitions `cut` / `dip`.
- Example shot: "20 mm linear truck along the empty station corridor, panels lit, 4 s."
- Example card: "最後の通信から、40年。" / "40 years since last contact."
- Iteration: when it feels sterile, add one human trace (a photo taped to a console).
