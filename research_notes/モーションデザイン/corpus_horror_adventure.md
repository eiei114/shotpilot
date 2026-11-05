# Motion of text, titles and transitions in horror and adventure teasers (measured)

Scope: 6 horror and 6 adventure teasers (2 Japanese-language in each genre) from `research_data/teasers/videos/`. Only motion and timing are recorded; no frames are copied into the repo.

## 1. Method

- Overview contact sheet per teaser at 1 fps (240 px tiles, time stamped) to find every designed text moment.
- Dense strips (4-16 fps, 1-5 s) around the text moments to name the entrance/exit type.
- Per-frame luma (`luma.sh`: mean YAVG, max YMAX, mean frame difference YDIF) at 24 fps for windows up to 2.5 s, 12 or 8 fps for longer windows. Durations were read as follows. A fade is the span over which YMAX (or YAVG on black) ramps between its floor and plateau (accuracy about +/-0.08 s at 12 fps, +/-0.04 s at 24 fps). A hard cut is a YDIF spike with a step in YAVG on one frame. A flash is a bright (YAVG > 150) run of a few frames. A flicker is YMAX changing by more than 10% from frame to frame.
- Source frame rate is about 24 fps, so "cut" means <= 1 frame (0.04 s).
- Ignored: in-game UI/HUD and the games' own in-world dialogue boxes (Maskborne 12-20 s, Become speech bubbles). Included: the Mercurius captions and name plates, which are edited-in design text.
- "unclear" = could not be told from luma and the strips used.
- Sizes are the share of frame height taken by the text block (cap height of one line for small text).
- Luma cannot see a text ramp on top of a busy picture, so overlay entrances on live picture were judged from strips (cut = appears within one strip frame).

## 2. Per teaser

Notation: t = seconds in the video; dur = seconds.

### Horror

#### yt-fAXzLmsbrd8 (67 s) - "Escape From Playtime" release date trailer (https://www.youtube.com/watch?v=fAXzLmsbrd8)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 5.0-9.0 | studio logo (custom, centre, ~30% h, white-cyan glow) | TV-static burst 0.9 s (5.04-5.96: luma jumps 44/88/100 every 3-6 frames, YDIF 25-90), then hard cut onto the logo at 5.96 | 3.0 s, none | slow change over ~0.4 s from 9.0 (dissolve-like; exact type unclear) | strobing static in, soft out |
| 56.04-63.7 | title logo (custom bubble logo, centre, ~38% h, white with soft glow) | cut to black at 56.04 (0.3 s black), then **flicker-in 0.8 s** (56.33-57.1, YMAX oscillating 28/243/199/28/197/214), glow-up complete ~57.4 (total ~1.1 s) | 6.3 s (57.4-63.7); mean luma 75 -> 72 (glow decays 4% in 6 s), otherwise static; logo on black, no camera | hard cut at 63.71 | cut-to-black, flicker, hold, cut |
| 63.7-67.4 | end card: date + Steam icon + URL (monospace, white, 4 lines, ~5% h each, centre) | cut | 3.7 s static | cut to black at 67.38 (file end) | cut / cut |

Distinct: strobing static as the logo entrance; the title flicker is the closest real match to the current flicker-in, but lasts 0.8 s, twice the current ~0.4 s.

#### yt-yd6oGFqE1F8 (58 s) - "The Cat and the Hatchet" teaser (https://www.youtube.com/watch?v=yd6oGFqE1F8)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 11.12-15.1 | card on black: 1 line (red distressed display, tracked caps, centre, ~5% h) | picture hard-cuts to black at 11.12, **linear fade-in ~1.0-1.1 s** (YMAX 17 -> 44) | 3.1 s (12.1-15.08), flat | **hard cut** at 15.12; the next picture then fades up from black 0.45 s (YMAX 17 -> 82) | cut-to-black in, cut out |
| 26.46-29.8 | card on black: 1 line (red, ~3% h, dim) | cut to black at 26.46, **linear fade-in ~1.0 s** (YMAX 18 -> 41) | ~2.3 s flat | hard cut at 29.83 to a bright picture | same pattern |
| 47.58-50.9 | title card (stacked 3-line red logo, centre, ~20% h) | **cut** (0 s; YMAX 47 on the first frame) after a cut to black | 3.3 s, tiny flicker (YMAX 43-47, +/-3) | hard cut to picture at 50.92 | cut / cut |
| 51.58-58.2 | "COMING SOON" (red, ~3% h, centre) | cut to black at 51.58, fade-in ~1.15 s (YMAX 17 -> 43) | ~3.5 s flat (52.8-56.2) | **linear fade-out 1.3 s** (56.3-57.6), black to end | the only fade-out of the teaser |

Distinct: text exits are hard cuts; entrances are always a ~1.0-1.15 s linear fade from a freshly cut black card. Text is dark red and dim (plateau YMAX 41-46, about 17% of full white).

#### yt-IkXbe0zTAck (60 s) - "Indigo Park: Chapter 2" teaser (https://www.youtube.com/watch?v=IkXbe0zTAck)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 0-2 | none (picture) | - | - | - | picture fades up from black over ~2 s (YMAX 18 -> 177) |
| 50.6-54.7 | title logo (purple neon script + emblem, centre, ~22% h) | **white-out wipe 0.25 s** (50.58-50.83: YAVG 50 -> 119, picture bleaches), then letters draw/light on ("I", "In", "Ind") like neon-tube ignition for ~1.2-1.5 s while the white bloom decays (YAVG 119 -> 33 by 52.25) | 2.4 s (52.3-54.7), static glow (YMAX 246-248) | **fade-out 0.9 s** (54.75-55.6) | picture -> white flash-wipe -> logo |
| 55.9-59.8 | "CHAPTER 2" (wide-tracked thin sans, grey-blue, ~4% h, centre) | fade-in ~1.1 s (55.9-57.0, YMAX 60 -> 225) with a **light glint** at 56.9-57.2 (peak 225, then 158) | none (already decaying) | **slow fade 2.5 s** (57.3-59.8; YMAX 168 -> 30, linear) | no text in the body of the teaser |

Distinct: the logo ignites out of a white bloom (the flash is the reveal itself), then a sub-title with a glint, then a very slow fade-out.

#### yt-LnydjhGRH1s (77 s) - "Farsight" reveal trailer (https://www.youtube.com/watch?v=LnydjhGRH1s)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 66.58-70.5 | title logo (condensed dripping display, white, centre ~22% h) + small studio logos below | cut to black at 66.58 (0.17 s black), **cut-in** at 66.75 with a 0.3 s settle (YMAX 233 -> 255) | 3.7 s static | hard cut at 70.50 (layout changes) | cut |
| 70.5-74.3 | end card "COMING 2026 / WISHLIST ON STEAM" (small caps sans ~2.5% h, centre, Steam icon, studio logos) | cut | 3.8 s static | hard cut to black 74.33, black 0.75 s, cut to picture at 75.08 | cut, black 0.75 s |

Body transitions: nearly all cuts-to-black are hard (5.12 has a 0.1 s ramp; 42.25 and 74.33 single-frame). At 55.46-55.58 there is a **white flash of 0.12 s** (YAVG 209 for 3 frames) directly before a black frame. Opening: black to picture in 0.1 s.
Distinct: zero fades; every text moment is a cut; titles are held ~3.7 s on black.

#### yt-p9sgg9u8ZXg (36 s, Japanese) - "OBSERVE" Steam wishlist trailer (https://www.youtube.com/watch?v=p9sgg9u8ZXg)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 28.4-32.4 | title logo over picture, then on black (bold monospace-like sans with slashed O + kanji line, white bloom, centre, ~11% h) | **cut-in** over the running (static-camera) tunnel shot at 28.4 | 4.0 s; the picture hard-cuts to black at 28.92 (0.5 s after the logo), logo stays 3.5 s more; static bloom | hard cut at 32.46 | logo straddles the picture/black cut |
| 32.5-35.9 | end card: terminal-style monospace lines (blue-white, ~3% h, centre), then a Japanese version at ~34 s | cut | 3.5 s; YMAX jitter +/-8 (scanline/noise) | file ends | cut |

Body: picture-to-black cuts are single frames; red-tinted shots are single cuts.
Distinct: logo is superimposed on the last picture, then the picture drops out beneath it; end card is a fake terminal text (typed look, but the entrance is a cut).

#### yt--a02BYwRrjg (19 s, Japanese doujin) - "Dreaming Cortex" teaser (https://www.youtube.com/watch?v=-a02BYwRrjg)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 0.0-1.0 | credit: "Presented by" + kanji group name (serif white, red English line, top-centre, ~9% h) | **fade-in 0.25 s** (YMAX 18 -> 255) | 0.75 s static | hard cut | |
| 1.0-2.0 | quote card: 2 serif lines (~2.5% h, left-centre) on a grainy grey plate | cut | 1.0 s, film grain | hard cut to a white-out picture at 2.0 (YAVG 47 -> 93) | cut |
| 4-9 | kinetic text montage: tall vertical kanji columns, a spiral ring of kanji, red kanji line (each 0.17-0.5 s) | **cut with glitch**: RGB split at 4.67-4.83, horizontal jitter, scanline tear at 8.33; ring spins 6.3-6.8 | 0.17-0.4 s each | cut | 10+ text events in 5 s |
| 10.2-12.9 | title card: red outlined title (kanji, ~9% h) + genre line + price/platform line (small), centre | **strobe-in**: 2 pulses at 10.17 and 10.75 (YMAX 249 -> 198 -> 255, ~0.6 s total) | 2.1 s static | hard cut at 12.92 | |
| 12.9-18.5 | date card: white sans text (3.5% h, left-centre) over a faint moon image, tiny credits at the bottom | cut | 5.6 s static | **fade-out 0.5 s** (18.5-19.0, YMAX 252 -> 49) | |

Distinct: the densest text of the horror set (credit, quote, kinetic type, title, date in 19 s); entrances are cuts and glitches; the only fades are the first (0.25 s) and the last (0.5 s).

### Adventure

#### yt-qDFEeeLy6ws (41.5 s) - "Hollow Knight: Silksong - Sea of Sorrow" teaser (https://www.youtube.com/watch?v=qDFEeeLy6ws)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 26.7-31.9 | title logo (ornamented serif display, white + teal "SEA OF SORROW", centre, ~22% h) | picture **fades to black 1.3 s** (25.3-26.6), 0.2 s black, then **blur-to-focus + fade-up 1.8 s** (26.75-28.5; starts heavily blurred, YMAX 21 -> 217) | 3.4 s (28.5-31.9), slight glow settle (YMAX 224 -> 255); no camera | **fade-out 0.8 s** (31.9-32.7) | picture -> fade through black -> blur-focus title |
| 33.0-34.8 | "2026" (serif, teal, ~7% h, centre-low) | **fast fade 0.25 s** (32.83-33.08, YMAX 77 -> 191, ease-out) | 1.7 s flat | **fade-out 1.2 s** (34.8-36.0) | follows the title 0.4 s after its fade-out |
| 36.4-40.0 | "FREE FOR ALL PLAYERS" (small caps sans, white, ~2.5% h, centre) | fade-in 0.45 s (36.4-36.85, ease-out) | 1.9 s flat | **fade-out 1.3 s** (38.7-40.0) | ends on black 1.5 s |

Distinct: the only blur-to-focus; asymmetric timings (in 0.25-0.45 s on small lines, out 0.8-1.3 s); 0.2-0.4 s black gaps between cards.

#### yt-GUnZa0ghIow (74 s) - "Become" announcement trailer (https://www.youtube.com/watch?v=GUnZa0ghIow)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 0.0-1.8 | diegetic 3D letters in a lit cave ("DEEP INSIDE THE ...", heavy rounded sans, ~12% h per line, teal-grey) | present at t=0; fog/light reveals it by exposure rise over ~1 s | 1.8 s, slow camera push/drift (lines rise and scale up ~15%) | cut at 1.8 to a flare shot | diegetic |
| 20.2-22.0 | diegetic 3D title letters ("TINY INDIE ADVENTURE", chunky 3D sans, teal-green, ~12% h) | camera swoops past: a foreground letter group races across 20.4-20.9 (**fly-through, ~0.5 s**) | ~1.1 s; perspective keeps changing (tilt ~8 deg) | hard cut at 22.0 | diegetic |
| 46.3-48.4 | diegetic text on a sock (chunky sans, ~9% h, teal-white) | camera pan reveals it 46.3-46.6; no entrance animation | 1.0 s static in world | **exposure darkens to black over 1.0 s** (47.4-48.4) | text is a joke in the scene |
| 66.25-69.9 | title "BECOME" (rounded logo with a sperm-shaped O, ~15% h, cream with dark edge, centre-right) | **emerges from the burst/blur ~0.5 s** (66.25-66.75: blurred and dark, then sharp) | 3.2 s, slow camera drift; cells drift behind | hard cut at 70.0 | |
| 70.0-73.9 | "COMING 2026" (3D letters on speed streaks, ~12% h, teal-white, angled) | cut; letters tilt/whip with motion blur | 3.9 s, strong camera shake/vibration | file ends on the shot | whip/shake |

Distinct: all text is diegetic or pseudo-diegetic; the camera, not an overlay layer, performs the motion. Small credits/Steam tags at the bottom are static.

#### yt-q21jZaUOa4Q (28.7 s) - "Mighty Cuphead Adventure" teaser (https://www.youtube.com/watch?v=q21jZaUOa4Q)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 17.4-22.7 | overlay "Currently in Development" + 3 check-list lines (retro ad serif/sans, white, upper-left, ~3.5% h per line) | **staggered build**: heading blurs into focus 17.375-17.6 (0.25 s), then 3 list lines appear in 0.12-0.25 s steps (17.5, 17.625, 17.75); total ~0.6 s | 5.0 s static | cut at 22.75 to the logo sting | overlay on an ad-style shot |
| 22.75-27.7 | studio logo coin + "Studio MDHR" wordmark | coin spins in with VHS-line glitch, 22.75-24.4 (**spin + scanline jitter ~1.6 s**); wordmark **blur-to-focus 0.6 s** (24.5-25.1) with a light pulse (YAVG 52 -> 81 at 24.58-24.75) | 2.6 s (25.1-27.7), glow settles (YAVG 75 -> 59) | **hard cut to black at 27.75** | |

Distinct: retro VHS pastiche; the multi-line stagger; a light pulse on the wordmark; the final logo cuts out rather than fades. The title-on-TV and the opening box fade (0-3 s) were not measured (unclear).

#### yt-7SFCD_MMQ6Q (46 s) - "Maskborne" teaser (https://www.youtube.com/watch?v=7SFCD_MMQ6Q)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 35.0-36.5 | picture -> white flash -> black | **white flash 0.1 s** (35.0-35.17: YAVG 104, 203, then 16), no fade | black ~1.4 s | | |
| 36.5-42.2 | title card: runic logo (hand-cut display, white, left, ~28% h) + glowing mask emblem (right) + "Wishlist on Steam" (small sans ~3% h) + Steam icon | **slow fade-up 2.3 s+** (36.5-38.8+, YAVG 16 -> 68, still rising); logo letters assemble **jittery/fragmented** (glyph pieces flicker at 36.9-37.2 before closing up) | 3.4 s, emblem glow brightens slowly | **fade-out 1.1 s** (42.25-43.33, linear) | black -> logo -> black |

Distinct: the slowest fade-up in the set; fragmented flicker-assemble of the logo; title and CTA are one card.

#### yt-9OTotaEBjpk (43 s, Japanese) - "Shiin" PV (https://www.youtube.com/watch?v=9OTotaEBjpk)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 0-1.0 | rating notice (tiny white sans, ~2% h) on black | present from frame 0 | 1.0 s | cut to the platform splash (0.25 s ramp) | |
| 19.58-22.7 | title (vertical Mincho kanji with small furigana, white, centre, ~15% h) | picture -> **white flash 0.17 s** (19.58-19.75) -> black 0.65 s -> **fade-in 0.35 s** (20.4-20.75, ease-out) | 1.4 s (20.75-22.17) | **fade-out 0.45 s** (22.2-22.65) | flash, black, title |
| 22.7-28.1 | URL (serif, tiny ~1.5% h, centre) | fade-in 0.4 s (22.7-23.1) | 5.0 s static | strobe/hard cut at 28.17-28.58 (0.4 s flash/glitch burst, YAVG 52/166/70/60/118), then black | |
| 30.8-33.0 | platform logo (white, ~5% h, on blue) | cut-in with 0.2 s settle | 1.7 s | **cross-dissolve 0.4 s** (32.55-33.0) into the CTA card | |
| 33.0-43.4 | CTA card "subscribe" + button (sans ~3% h, white on blue) | fade-in 0.4 s (33.4-33.8) | 9.6 s static | file ends | |

Distinct: console-publisher end sequence (logo + CTA, ~12 s, about 28% of the length, with a crossfade); the only designed motion on the title is a short fade between a flash and black.

#### yt-pL5c8WYfqv8 (90 s, Japanese, visual-novel RPG) - "Mercurius' Blue Sand ReCollection" teaser (https://www.youtube.com/watch?v=pL5c8WYfqv8)

| t | kind | entrance + dur | hold + motion | exit + dur | transitions / notes |
|---|---|---|---|---|---|
| 6.3-8.8 | studio logo (wireframe strokes + wordmark over a perspective grid, white-blue, ~8% h) | **draw-on/assemble 0.4 s** (6.35-6.8), strokes fly in | 2.0 s, shimmer (YDIF 5-8 per frame) | hard cut at 8.8 | |
| 10-16 | fake terminal UI + "OMEGA-NET" logo | typed/blinking; unclear | ~5 s | unclear | diegetic UI |
| 17.33-18.7 | card line on black (Mincho serif, white, ~2.5% h, centre) | **cut** (0 s) | 1.4 s static | **cut to black at 18.7** (0.2 s black) | |
| 18.9-19.8+ | next line on black | **scramble-decode ~0.12 s** (garbled glyphs at 18.875, correct at 19.0) | 1+ s | cut | line after line, each separated by cut-to-black |
| 20.2-20.8 | panel/frame lines | **mask/wipe: thin lines draw a panel, picture revealed inside**, 0.6 s | | | |
| 21.4-25.0 etc. | captions (lower centre, 2 lines white sans, ~2.5% h each, on black letterbox) | **cut** with the picture (0 s) | 3.5 s static | cut with the next shot | recurs 22-35 s, 36-45 s, 48-60 s |
| 29.5-34.8 | dialogue lines right of a portrait (white sans, ~2.5% h, 2 lines) | cut | 2.5 s | cut | |
| 32.2-35.0 | character name plate (katakana + small Latin, white sans, ~4% h, left-centre lower) | **per-character type-on with scramble 0.6 s** (32.17-32.8) | 2.0 s static | **fade-out 0.3 s** with the picture (34.8-35.1) | |
| 60.75-64+ | line "Found it" (white serif ~2% h, centre) | the picture **dissolves into particles 1.3 s** (60.75-62.0) while the line **fades in ~1.0 s** (61.0-62.0); the second half is **typed on** at 64.0 | 2-4 s | cut | particle dissolve is the transition |
| 75.4-85.5 | title logo (Mincho wordmark ~6% h + subtitle bar + small line) | after a hard cut to black at 74.92 (0.5 s black): **left-to-right mask wipe 0.1 s** (75.42-75.5); at 76.83-77.0 a **light-sweep/lens-flare burst 0.25 s**, then the blue bar and subtitle wipe in with streaks (77.0-77.3) | 7.0 s (77.3-84.3); small bubbles drift on the right | **gradual fade-out 1.2 s** (84.4-85.6), 1.3 s black | |
| 86.9-90 | studio logo end | cut | 3.1 s | file ends | |

Distinct: visual-novel grammar. Text drives the edit; captions are lower-centre and cut in/out with the picture; text lines are cut to black individually; decode/type-on for names and new lines; a light sweep on the title; the 7 s hold.

## 3. Genre synthesis

### Horror (6 teasers)

- **Cards on black**: cut to black from the picture, then a **linear fade-in 1.0-1.15 s** (yd6o x3, Indigo "Chapter 2" 1.1 s); hold 2.3-3.5 s; exit either a **hard cut** (yd6o x2, 0 s) or a **slow fade-out** (1.3 s yd6o "coming soon", 2.5 s Indigo chapter). Text is dim (red at ~17% of white).
- **Overlays on picture**: none on the Western horror teasers (text sits on black or on a logo card). Only OBSERVE lays the logo over running picture (cut-in, then the picture drops out beneath it).
- **Title**: entrance is a cut (yd6o, Farsight, OBSERVE: 3 of 6), a flicker/strobe 0.6-0.8 s (fAXz 0.8, doujin 0.6), or a neon ignition after a 0.25 s white-out, ~1.5 s (Indigo). Range 0-1.5 s, median ~0.3 s. Hold 2.1-6.6 s, median 3.5 s, mostly static (glow decay -4% over 6 s; tiny flicker +/-3 YMAX in yd6o). Exit a hard cut (fAXz, yd6o, Farsight, OBSERVE) or a fade of 0.9 s (Indigo). Typeface: custom/distressed display (dripping, brush), white or blood red.
- **End card** (date/URL/platform): cut in, 3.5-3.8 s static, cut out (fAXz, Farsight, OBSERVE); small mono/sans type 2.5-5% h. One of four fades out (doujin, 0.5 s).
- **Transitions**: the dominant move is a **hard cut to black**, black held 0.2-0.75 s, then a cut to the next picture. Fade from black is used for picture only (0.45-2 s, at the start or after a card). **White flash 0.10-0.17 s** appears as an accent before black (Farsight 55.46-55.58; doujin white-out at 2.0) and as the reveal (Indigo, 0.25 s). No crossfade/dip measured, except the soft ~0.4 s change at the fAXz studio-logo exit (type unclear).
- **Camera on title shot**: no horror title sits on a moving picture (all on black; OBSERVE's tunnel is static).
- Versus adventure: horror uses cuts, flickers and long linear fades; fades appear only on the quiet lines and the very last line; text is never blurred, wiped or swept.

### Adventure (6 teasers)

- **Cards on black**: Silksong tag lines fade in 0.25-0.45 s (ease-out), hold 1.7-1.9 s, fade out 1.2-1.3 s. Shiin: fade-in 0.35-0.4 s, fade-out 0.45 s. Mercurius lines: **cut** in, 1.4 s, cut out. Fades are asymmetric: **in 0.25-0.45 s, out 0.45-1.3 s**.
- **Overlays on picture**: Mercurius captions (lower centre, 2 lines, ~2.5% h, cut in/out with the picture, 2.5-3.5 s) and Cuphead's retro overlay (staggered build 0.6 s, hold 5 s). Become's credits are static.
- **Title**: entrance 0.1-2.3 s (Silksong blur-to-focus 1.8 s; Maskborne slow fade 2.3+ s with fragmented flicker; Become emerge 0.5 s; Cuphead wordmark blur 0.6 s; Shiin fade 0.35 s; Mercurius wipe 0.1 s + light sweep 0.25 s); median ~0.6 s. Hold 1.4-7.0 s, median ~3.4 s, with slow engine-camera drift/push, particles, or a glow settle. Exit: fade 0.45-1.3 s (median ~1.1 s) or hard cut (Become, Cuphead).
- **End card**: Silksong "2026" and the free line (fade in 0.25-0.45 s, out 1.2-1.3 s); Maskborne's title doubles as the card; Become's in-world "COMING 2026" (3.9 s, shake); Shiin's logo + CTA (12 s).
- **Transitions**: fade to black 1.3 s (Silksong), dissolve into particles 1.3 s (Mercurius), white flash 0.1-0.17 s (Maskborne, Shiin), cross-dissolve 0.4 s (Shiin end), hard cuts elsewhere; black gaps 0.2-0.65 s before a title.
- **Camera on title shot**: the engine camera keeps moving in Become (push/shake) and Silksong (slow settle); Mercurius' title is on black with drifting particles.
- Versus horror: more blur, wipe and light-sweep language, longer holds (up to 7 s), diegetic 3D-in-world titles, a gentler text exit.

### Japanese teasers vs Western ones

- More text per second: the doujin horror has 10+ text events in 19 s; Mercurius ~20 in 90 s (Western teasers: 2-4).
- Text is used as cut rhythm: individual lines cut in and out within 1-3.5 s; glitch/RGB split and jitter for horror (4-9 s of the doujin), decode/scramble 0.12 s and type-on 0.6 s for adventure (Mercurius).
- Captions and dialogue sit lower-centre or beside a portrait (Western teasers have no such overlays).
- Publisher/CTA scaffolding is long (Shiin 12 s, 28%), with a cross-dissolve 0.4 s and a button.
- Vertical text and Mincho serif for titles; tiny URL cards (1.5% h) held 5 s.
- Title entrance, JP horror: cut/strobe (0-0.6 s). JP adventure: short fade 0.35 s (Shiin) or a 0.1 s wipe plus a 0.25 s light sweep (Mercurius). Both are faster than the Western Silksong/Maskborne.

## 4. Proposed motion profiles (plain values)

Values are measured unless marked "(inferred)".

### horror

- text_in (card on black): `fade`, 1.0 s, linear (range 1.0-1.15), after a cut to black.
- text_in (overlay): none measured on Western horror; for JP horror `cut` 0 s, glitch 0.17-0.4 s for kinetic lines (from the doujin; inferred for general use).
- text_out: `cut` 0 s (2 of 3 measured); `fade` 1.3 s for the final line; chapter-style line `fade` 2.5 s.
- title_in: `cut` 0-0.2 s (3 of 6); option `flicker` 0.8 s (fAXz 0.8, doujin 0.6); option `ignite` ~1.5 s after a 0.25 s white wipe (Indigo).
- title_hold: 3.5 s (range 2.1-6.6), motion `none` or `slow glow decay -4%`, optional `flicker +/-3%` (yd6o).
- title_out: `cut` (4 of 6); else `fade` 0.9 s.
- end_card: `cut` in 0 s, hold 3.7 s, `cut` out; type small centred mono/sans.
- transition_default: hard cut via black: `cut` to black, black gap 0.2-0.75 s (about 0.4 s), `cut` in (maps to a `dip` with ~0 s fade and ~0.4 s black: inferred mapping).
- transition_accent: `flash` white 0.12 s (measured 0.10-0.17 s) before black; white-out wipe 0.25 s for the title reveal.
- overlay placement: none in the body; cards centred, 3-5% h; title centred, 20-38% h; end card centred, 2.5-5% h.
- JP horror variant: kinetic lines `cut`/`glitch` 0.17-0.4 s; title `strobe` 0.6 s; date card hold 5.6 s then `fade` 0.5 s.

### adventure

- text_in (card on black): `fade` 0.4 s, ease-out (range 0.25-0.45); short lines may `cut`.
- text_in (overlay): lower-centre caption `cut` 0 s; multi-line list `stagger` 0.6 s, 0.12-0.25 s per line.
- text_out: `fade` 1.2 s (range 0.45-1.3); overlay exit `cut` with the picture.
- title_in: `blur_to_focus + fade` 1.8 s (Silksong), `fade` 2.3 s (Maskborne); median across the six is ~0.6 s, so a plain `fade` 0.5 s is the short form (inferred mid-range).
- title_hold: 3.4 s (range 1.4-7.0), motion `slow drift/push` or `particles`, glow settle.
- title_out: `fade` 1.1 s (range 0.45-1.3).
- end_card: tag line `fade` in 0.4 s, hold 1.9 s, `fade` out 1.3 s; platform/CTA cards `cut` in, hold 3.5-10 s.
- transition_default: `dip` through black: fade to black 1.3 s, black gap 0.2-0.3 s (Silksong), otherwise `cut`.
- transition_accent: `flash` white 0.1-0.17 s then black (Maskborne, Shiin); `light_sweep` 0.25 s on the title.
- overlay placement: captions lower centre, 2 lines, ~2.5% h each; retro/UI overlays upper left; title centre ~20% h; tag lines centre-low ~2.5% h.
- JP adventure variant: lines cut in/out per 1.4-3.5 s; names `type_on` 0.6 s; a new line `scramble` 0.12 s; title `wipe` 0.1 s + `light_sweep` 0.25 s, hold 7 s, `fade` out 1.2 s.

### Mapping notes for shotpilot (inferred)

- The current 0.35-0.45 s fade-in suits adventure small lines but is about 2.5x too fast for horror cards (1.0-1.15 s) and too short for adventure titles.
- Horror cards need a fade-in up to ~1.1 s and a hard-cut exit; adventure needs asymmetric fades (about 0.4 s in, 1.2 s out).
- A title-entrance vocabulary of `cut`, `flicker` (0.6-0.8 s), `fade` (0.5-2.3 s), `blur_to_focus` and `light_sweep` (0.25 s) covers 11 of the 12 measured teasers; glitch/RGB split is the one extra, for JP horror.
