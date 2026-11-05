# Design System Inspired by VHS Found Footage

> Category: Horror
> A recording somebody should not have kept. Hand-held framing, timestamps, cuts that feel like the tape skipped.

## 1. Visual Theme & Atmosphere

The teaser pretends to be evidence. Shots are framed like a person holding a camera (slightly off level, too close, searching), and the text reads like a label on a tape or an incident log. The world is lit only by what was there: a flashlight, a monitor, a streetlight. Cuts are abrupt, as if the recording stopped and started again. The viewer is put in the position of someone going through footage after something happened.

## 2. Color & Grade

- Grade: `horror` (desaturated, grain, vignette). Use `red` only for a short jolt teaser under 20 s.
- Letterbox: none (0). Full frame, like a camcorder.
- Palette roles: ink `#050505`; paper `#d8d8d0` for log text; accent `#c8ff3c` (an on-screen-display green) for timestamps and counts only.

## 3. Typography Rules

- Japanese: gothic (Noto Sans JP; DotGothic16 for logs). Latin: a monospaced face (VT323 / Space Mono), uppercase.
- Text looks like metadata: `REC 03:12:44`, `TAPE 2 / 1998.08.14`, `記録 3`.
- No serif, no elegant centred cards.

## 4. Title & Card Stylings

- `count`: the main device. Lower left, like a note under a photograph: "記録 1", "記録 2" over short evidence shots.
- `card`: incident-log lines on black ("1998年8月14日 最後の記録").
- `black`: hard cuts to black of 0.3-0.6 s, as if the tape stopped.
- `title`: plain and small, a hard cut in and a hard cut out.
- `message`: the last thing found on the tape.

## 5. Pacing & Transitions

- Evidence shots 1-1.5 s in a run; establishing shots 3-4 s.
- Transitions: `cut`, and `flash` (from white) on each short evidence shot of a count. `dip` only before the title.
- Card holds 1.8-2.2 s; logs are read fast.

## 6. Camera Language

- 18-28 mm, close to the subject, as a hand-held camera would be.
- Moves: `truck_left` / `truck_right` and short `dolly_in` with `linear` curves (a person walking), `static` for the found clip.
- Frame slightly off centre; let the subject leave the frame.

## 7. Sound Character

Tape hiss, mechanical clicks between clips, breathing, one sound off screen. No score; silence after the last clip.

## 8. Do's and Don'ts

Do
- Make every caption read like data, not like a slogan.
- Keep each evidence shot short and specific.
- End on the clip nobody should have seen.

Don't
- Use elegant serif cards or slow fades.
- Show the full threat; a shape at the edge is enough.
- Mix in a cinematic letterbox.

## 9. Agent Prompt Guide

- Quick reference: grade `horror`, letterbox 0, typeface `gothic`, transitions `cut` / `flash`.
- Example run: "Five 1.2 s shots, each a different room at 24 mm, count overlays 記録 1-5, `flash` on each."
- Example card: "1998年8月14日" / "Last recording."
- Iteration: when the run feels long, cut a clip instead of shortening all of them.
