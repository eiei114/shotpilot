# Motion measured from luma over every in-scope teaser (D-53)

Provenance: [M] measured by `research_data/tools/measure_motion.py` (not in git) over the 736
in-scope teasers in `research_data/teasers/videos/` on 2026-11-05. Raw rows:
`research_data/motion/motion.jsonl`, summary: `research_data/motion/summary.json` (both local).

## Method

Every video is read at 12 fps on a 320 px wide picture; per frame, mean luma (YAVG) and max luma
(YMAX). Resolution of every time below: 1/12 s (0.083 s).

- **Text on black**: a run of dark frames (YAVG < 28, at least 0.5 s) whose YMAX rises above the
  black level. Kept only when the text is held at least 1 s, steady (the held YMAX varies less
  than 12% of its contrast) and at least 12 levels brighter than the black. The entrance is the
  time from the first lit frame to 90% of the plateau; text already lit when the dark run
  begins came in by a cut from the picture (0 s). The exit is the same backwards; text still lit
  when the dark run ends left by a cut to the picture (0 s).
- **Black gaps**: runs of at least two frames with nothing on screen (YMAX < 35), not at the
  start or the end. The picture reaches the gap by a cut (one frame) or by steady dimming
  (its length); it comes back the same way.
- **White flashes**: frames with YAVG > 190 that last 0.25 s or less between darker frames. The
  most in any one second is the photosensitivity number (WCAG 2.3.1 allows 3). Longer white is
  a whiteout.

Caveats: dark gameplay with a steady bright element can pass for text on black (the hold and
steadiness filters remove most of it); logos count as text; a title fading in over a picture
that is not dark is not seen. The frame-by-frame notes (`corpus_*.md`) say which kind of entrance
it was; this pass says how often and how long, over many more teasers.

## Results

| group | teasers | text on black | in: cut | in: fade p25–median–p75 (s) | out: cut | out: fade median (s) | hold median (s) | black gap median (s) | into black: cut / fade (s) | out of black: cut / fade (s) | most flashes in 1 s | teasers with a whiteout |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| all | 736 | 689 | 52% | 0.333–0.5–0.917 | 60% | 0.417 | 2.25 | 0.583 | 72% / 0.333 | 68% / 0.333 | 3 | 23% |
| all-ja | 168 | 166 | 43% | 0.333–0.417–0.75 | 51% | 0.417 | 2.333 | 0.667 | 65% / 0.417 | 60% / 0.333 | 2 | 39% |
| horror | 168 | 261 | 52% | 0.25–0.5–0.917 | 63% | 0.417 | 2.167 | 0.583 | 79% / 0.333 | 77% / 0.333 | 2 | 10% |
| horror-ja | 35 | 50 | 44% | 0.25–0.417–0.583 | 52% | 0.333 | 2.333 | 0.709 | 78% / 0.333 | 79% / 0.333 | 2 | 9% |
| adventure | 51 | 34 | 59% | 0.333–0.459–0.646 | 71% | 0.541 | 2 | 0.5 | 76% / 0.333 | 69% / 0.417 | 1 | 25% |
| adventure-ja | 9 | 13 | 46% | 0.291–0.417–0.625 | 54% | 0.458 | 1.417 | 0.333 | 68% / 0.25 | 73% / 0.417 | 0 | 56% |
| cozy | 60 | 7 | 43% | 0.312–0.333–0.354 | 86% | 0.583 | 3.25 | 0.167 | 22% / 0.417 | 33% / 0.375 | 1 | 40% |
| rpg | 51 | 52 | 57% | 0.417–0.75–1.458 | 75% | 0.667 | 2.292 | 0.583 | 62% / 0.417 | 62% / 0.417 | 2 | 41% |
| rpg-ja | 18 | 19 | 57% | 0.667–0.791–1.375 | 68% | 0.5 | 2 | 0.583 | 57% / 0.667 | 48% / 0.583 | 2 | 56% |
| action | 99 | 76 | 55% | 0.271–0.375–0.75 | 55% | 0.459 | 2.333 | 0.583 | 63% / 0.25 | 65% / 0.417 | 1 | 25% |
| action-ja | 20 | 20 | 30% | 0.333–0.375–0.708 | 30% | 0.583 | 4.25 | 0.75 | 55% / 0.333 | 67% / 0.333 | 1 | 50% |
| strategy | 27 | 8 | 12% | 0.417–0.583–1.042 | 75% | 0.292 | 1.75 | 0.417 | 32% / 0.583 | 28% / 0.541 | 1 | 26% |
| strategy-ja | 10 | 5 | 20% | 0.375–0.417–0.667 | 100% | - | 1.5 | 0.417 | 24% / 0.583 | 14% / 0.541 | 1 | 60% |
| scifi | 15 | 14 | 56% | 0.312–0.584–0.791 | 50% | 0.417 | 1.708 | 0.667 | 75% / 0.291 | 70% / 0.25 | 1 | 40% |
| general | 265 | 237 | 51% | 0.333–0.541–0.938 | 53% | 0.417 | 2.333 | 0.5 | 73% / 0.333 | 66% / 0.333 | 3 | 23% |
| general-ja | 75 | 59 | 44% | 0.333–0.5–0.75 | 46% | 0.333 | 2.333 | 0.667 | 69% / 0.333 | 53% / 0.333 | 1 | 41% |

## Takeaways

- Text on black enters by a cut about half the time; faded, the median is 0.5 s (0.33–0.92).
  It leaves by a cut 60% of the time. The base profile's 0.45 s fade in is inside the range;
  its fade out (0.5 s) is the minority move.
- RPG fades are the slowest (median 0.75 s; Japanese RPG 0.79 s, p75 about 1.4 s). Strategy
  text fades in almost always (cuts 12%). Cozy rarely puts text on black (7 events in 60).
- Into black: horror cuts to black 79% of the time (the smash to black); cozy (22% cuts) and
  strategy (32%) fade into it over about 0.4–0.6 s. The profiles' `dip` lengths follow this.
- No teaser flashes more than 3 times in a second (the most is 3, once). A cap of 3 a second
  (`flash.rate`, P0) rules out nothing teasers do.
- Whiteouts appear in 23% of teasers, 41% of RPG and 39% of Japanese teasers: the `whiteout`
  hand-over and the RPG profile's accent.
