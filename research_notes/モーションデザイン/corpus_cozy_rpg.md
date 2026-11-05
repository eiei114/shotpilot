# Text, title and transition motion in COZY and RPG teasers (measured)

Scope: 4 cozy and 6 RPG teasers (3 of the RPG ones carry Japanese on-screen text: DQ III HD-2D, RPG Maker, Berserk Musou). No Japanese-language cozy teaser exists in the corpus (checked `_index.json`: no `ja` entry with a cozy category or a cozy/life-sim/farming name that has a video on disk), so the JA comparison is RPG-only. Third-party works; no frames are stored in the repo. All times are seconds in the source video.

## 1. Method

- Overview: 1 fps contact sheet (10 columns, 190 px) per teaser to find every designed text moment.
- Detail: 6-8 fps strips (1-4 s) around each text moment, with the time stamped on each tile.
- Numbers: `luma.sh` at 12-24 fps. A fade of a white logo shows as YMAX ramping to 255; a fade of the whole picture shows as YAVG ramping; a hard cut or flash shows as a YDIF spike (35-100+). Where a logo fades over a changing picture, the duration is read from the strip and marked "approx".
- Accuracy: 1/24 to 1/8 s depending on the pass. In-game UI and HUD are ignored. Diegetic text is noted only when it behaves like a title.
- Sizes are the share of frame height (logo or cap height), eyeballed.

## 2. Per teaser

Abbreviations: HC = hard cut, FI/FO = fade in/out. Durations in seconds.

### COZY

#### 2.1 Brushland (yt-ZLXfWsIWog8, 81 s) - https://www.youtube.com/watch?v=ZLXfWsIWog8
Almost no designed text for 72 s (HUD and build UI only). All the motion design is in the title.

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 72-73.0 | lead-in | slow hazy dissolve (mist) into the castle shot, no cut | - | - |
| 73.0-76.17 | title logo: blackletter-ish serif display, white with soft shadow, centre, ~18% height | FI 0.75 (73.0 to 73.75, ease-in-out feel, logo emerges through mist) | 3.1 s; picture drifts/tilts down slowly (tilt-shift, soft focus); logo static | HC at 76.17 (YDIF 35) to the key-art end card |
| 76.17-~80+ | end card: same logo plus a "wishlist on steam" line and a credit line (small sans, white) | HC-on, no fade | at least 3.8 s, static key art | video end; fade-out not seen (unclear) |

Distinctive: the only cozy teaser with a slow logo fade; the way into the title is a long mist dissolve, not a cut. Camera on the title shot: slow tilt-down/drift, no push.

#### 2.2 Dressmaker (yt-QtGJNp2QWEw, 44 s) - https://www.youtube.com/watch?v=QtGJNp2QWEw

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 0.25-1.0 | title logo (calligraphic script), white with soft glow, centre, ~13% height, over a still room shot | write-on: a thread/needle stroke draws the letters left to right, 0.75, linear; a star glint at the end of the stroke (twinkles to 1.9) | 0.9 (1.0 to 1.9) | HC at 1.917 (YDIF 103) to a UI shot |
| 36.8-37.3 | title again (end card) over a blurred sewing-machine still (HC in at 36.625, YDIF ~40) | write-on 0.5, same stroke, star glint | stays to the end (about 7.3 s, 36.7-44) | none; ends on its last frame |
| 37.75-38.1 | date ribbon (pink, white serif ~3.5% height) under the logo | ribbon wipes open horizontally from the centre, ~0.15; text appears as it opens ("September", then "September 2026") | ~1.3 | text swaps in place by cut: "September 2026", "2026" (~39), "Wishlist Now" (~40), platform icons pop (~41) |

Distinctive: handwriting write-on is the signature (thread/needle for a needlework game): 0.5-0.75 s draw, then a glint. Everything else is a hard cut; the ribbon opens with a centre wipe, not a fade. Camera under the end title: static (blurred still).

#### 2.3 From Scratch (yt-CP61GovN7ZE, 79 s) - https://www.youtube.com/watch?v=CP61GovN7ZE

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 15.4-17.6 | mid-teaser title card: "From Scratch", white rounded marker-sans, ~20% height, centre-left, over a table top with a wooden prop | HC into the shot at 15.4 (YDIF 22), logo FI 0.35-0.4 (15.8 to 16.2, YMAX 216 to 248) | 1.4 (16.2 to 17.6), static | HC at 17.625 (one-frame blend) to gameplay |
| 21, 47-54, 68-71 | small UI-style captions (teal tab/banner, lower left or top right, ~2.5% height) | appear with the shot (entrance unclear at this sampling) | 1.5-7 s | removed by HC |
| 72.33-77.5 | end card: HC into the same table-top at 72.33 (YDIF 36); logo FI ~0.25 (72.96 to 73.2); "Wishlist on Steam!" types on per character, 0.4 (73.0 to 73.4); Steam icon pops at 73.25 | - | 4.3, static | FO to black 0.55 (77.5 to 78.04, linear), then 0.9 s black |

Distinctive: the same title card is used twice (mid and end), identical look. Fast fade-in (0.25-0.4 s) and a type-on call to action. Exits are cut (mid) or a short linear fade to black (end).

#### 2.4 Waterful (yt-vmVVsImRjHo, 61 s) - https://www.youtube.com/watch?v=vmVVsImRjHo

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 0.7-2.6 | greeting, white rounded handwritten sans, lower left, ~7% height: "Hi!" then "I'm Lenja" | word by word, each word an instant pop (0.7, then +0.6) | 1.4 | HC to the next shot at 2.1 with the text carried across the cut, then FO 0.5 (2.1 to 2.6) over the new picture |
| 3.67-9.1 | title logo "WATERFUL" (chunky condensed rounded sans, white, ~20% height, upper centre) plus a subtitle (small handwritten, lower left) | HC to the shot at 3.67 (YDIF 36); logo FI 0.65 (3.75 to 4.4); subtitle word by word ("a cozy" 5.6, "minimalist" ~6.1, "nature-builder" ~7), ~0.5 per word | logo 1.2 s on the first shot, then carries over an HC at 5.625 onto a new shot until ~8.9 | removed at HC ~9.2 |
| 9.3-20 | keyword overlays DIG, GROW, UNLOCK, DISCOVER: white caps rounded sans, ~12% height, upper centre/right, tilted | present at full alpha on the first frame of each shot (cut-on) | ~2.5 each; slow drift and slight rotation; GROW/DISCOVER show per-letter wobble/ghosting | HC at the next shot |
| 45.6-52.4 | "AT YOUR OWN PACE", same style, ~8% height, centre, over a map | cut-on with the shot | 6.8; slow drift/tilt, letters wobble | HC at 52.4 |
| 53.43-59.75 | end card on a saturated red shot: logo plus "WISHLIST NOW ON STEAM" lower centre | HC (YDIF 50), logo at full alpha on the first frame (no fade); CTA lines appear ~1 s later by cut, a second form "WISHLIST NOW :)" at ~56 | 5.4 | dissolve to white 0.85 (58.9 to 59.75, linear) |

Distinctive: the most text of the cozy set, and it is a "living" overlay: cut-on with the shot, then constant gentle drift/rotation/wobble for the whole hold; text carries across cuts; words pop one by one. The end is a white dissolve.

### RPG

#### 2.5 Persona 6 (yt-9lwef2jan-Q, 85 s) - https://www.youtube.com/watch?v=9lwef2jan-Q

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 0-1.5 | rating card (white bordered card on black, ~35% height) | on from frame 0 | ~1.9 | HC |
| 2.0-2.9 | console logo (green, centre, ~15% height) | FI 0.75 (2.0 to 2.75, ease-in) | 0.1 | brightens 0.1 (2.83 to 2.92), black at 3.0 |
| 3.0-10 | publisher logo (blue metallic condensed, centre, ~12% height) over fog | slow FI 2.0 (3.0 to 5.0, smooth ease-in-out; fog drifts) | 3.9 (5.0 to 8.9), fog drift | FO ~1.2 (8.9 to 10.1) into fog |
| 19, 20, 29, 30, 37-38 | (body) black gaps between shots, ~1 s each (read from the 1 fps sheet, not luma measured) | - | - | the body is a series of cut-to-black beats |
| 65.6-69.75 | black gap 4.1 s before the title | - | - | - |
| 69.75-77.5 | title logo "P6" (green neon ring, custom, ~30% height, centre) | HC-on at 69.75 (mean luma 16 to 41 to 65 in 2 frames): a burst of green energy/smoke around the ring that dissipates over ~2.5 s (mean luma 65 to 52); logo static | 7.8 total | HC to black at 77.58 (YDIF 33) |
| ~72.9 | subtitle "PERSONA 6" (wide-tracked caps, green, ~3% height) under the logo | appears at ~72.9; type unclear (looks like a short tracking/wipe, about 0.5) | ~4.6 | - |
| 77.6-81.5 | end card (white logo, "Wishlist Now", platform icons, tiny legal row) | FI 0.45 (77.6 to 78.05, ease-out) from black | 3.5, static | HC to black at 81.58 (YDIF 34) |
| 81.9-end | console logo (green, ~12% height) | FI 0.5 (81.9 to 82.45) | ~1.5 | end |

Distinctive: a ritual of logo cards, each separated by hard cuts through black; slow, long fades for the publisher logo (2.0 in, 1.2 out); the title itself arrives with a hit (cut-on plus a particle burst), not a fade. Camera: none (logo on black).

#### 2.6 Dragon Quest III HD-2D (yt-3idSGSoMMn8, 80 s, Japanese text) - https://www.youtube.com/watch?v=3idSGSoMMn8

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 3.0-10.3 | two Japanese lines (small white mincho serif, ~3% height, centre, over key art) | line 1 FI ~0.4 (approx, 3.0); line 2 appears at ~7.0 after a cross-dissolve (~0.7) | line 1 3.3, line 2 ~3.3; the art behind changes slowly (sketch to colour) | white-out at 10.3 |
| 21.0-24.4 | emblem plus Japanese tagline (small white mincho, ~2.5% height, under the emblem) | emblem emerges from white: white to black background over ~1.0 with bloom/lens flare; emblem resolved ~21.5; tagline FI ~0.3 (21.6 to 21.9) with a sparkle | ~2.5 | HC at 24.4 to picture (no fade) |
| 69.8-78.5 | title: phoenix FI 0.4 (69.8 to 70.2); "DRAGON QUEST" 3D red/gold logo; "III" numeral; date line and platform icons | logo slam-from-large: starts cropped at ~150% frame width with a blur streak at 70.8, shrinks to fit by 72.2 (about 1.4, decelerating ease-out); "III" drops in from below at ~72.0 (gold shine); "III" morphs to "I＆II" with a light sweep 72.5 to 73.3 (about 0.8); the date line (~1.5% height) FI at ~74 | ~5 s lock-up, static | FO to black 1.5 (78.5 to 80.0, linear) |

Transitions (mean luma): body shots are joined by white-outs. Shape: rise to white 0.2-0.5 (a 0.17 s spike at 48.1), hold at full white 0-0.5, decay 0.7-1.0 (ease-out, slower than the rise). Examples: 10.3 to 12.3 (rise 0.5, hold 0.5, decay at least 1.0), 52.6 to 54.4 (rise 0.3, hold 0.5, decay 0.9), 59.6 to 61.4 (rise 0.5, hold 0.4). 15-17 s is a dark dip.

Distinctive: slow fades for subtitles, the white-out as the signature transition, a slam-with-blur logo that decelerates over 1.4 s, a numeral morph, a final long linear fade to black.

#### 2.7 Lair of the Leviathan (yt-AZmYsFsGVMk, 80 s) - https://www.youtube.com/watch?v=AZmYsFsGVMk
Designed text is rare (in-game dialogue boxes, stat screens, and an in-game "Battle Begins" splash at 9.5 that is game UI: pop-in 0.15, hold 0.9, cut).

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 71.5-80 | title over key art, logo upper right ~12% height (silver fantasy serif); small credits row lower right | a white flash (2 frames at 71.6) cuts to the key art; the logo gathers from blue-white magic sparks 72.2 to 72.75 (about 0.55; solid at ~72.5, sparks fade by 73.4): light sweep/shine | 8.3 s; key art static, logo sparkles for ~1 s then none | end of video (fade-out unclear) |

Distinctive: the minimal indie pattern: flash into key art, then one shine on the logo.

#### 2.8 Warlock (yt-ayW7oCXggoE, 86 s) - https://www.youtube.com/watch?v=ayW7oCXggoE

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 0-3 | rating badge lower right (~6% height) | on from frame 0 | 3 | HC with the shot |
| 74.0-76.8 | logo (metallic serif, emblem above, centre, ~35% height for the group) on black with ornament background | HC from a face close-up to black (YDIF 46): logo present on the first frame, then the ornament/mist background fades up behind it, about 1.4 (mean luma 25 to 43) | 2.8, logo static | dissolves out 0.3-0.5 (76.7 to 77.0) into the next card |
| 77.0-79.0 | "GAMEPLAY REVEAL / SUMMER 2026" (very wide-tracked caps, silver serif, ~2% height, two lines, centre) | soft dissolve in, about 0.4 | 2.0-2.3, static | cross-dissolve 0.4 (79.0 to 79.4) |
| 79.4-81.1 | "LAUNCH 2027" (same style) | cross-dissolve | 1.7 | cross-dissolve 0.3 to the logo |
| 81.3-86 | end card: logo plus a legal/platform row below | dissolve 0.3-0.4 | ~4.5, static | end |

Distinctive: cards on black in sequence, joined by 0.3-0.5 s cross-dissolves; wide-tracked small caps for the date; the title is cut-on with the picture fading up behind it. No flicker, no flash on the cards.

#### 2.9 RPG Maker (yt-V0xhZUk74e4, 66 s, Japanese) - https://www.youtube.com/watch?v=V0xhZUk74e4

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 1.0-7.1 | studio logo (mark plus wordmark, ~12% height, centre) on black | linear FI 1.6 (1.0 to 2.6) | 2.4 | linear FO 2.1 (5.0 to 7.1), then 3 s black |
| 20.0-26.3 | (picture) framed game screens in a museum | slow dip: mean luma 100 to 16 over 1.7 (20.0 to 21.5), 1.9 s black, HC-on at 23.375, HC to black at 26.25 | - | - |
| 50.0-60.4 | title "RPGツクール" (condensed grey-metal katakana wordmark, ~10% height, centre) over rising embers | FI ~2.0 (51.0 to 53.0, linear) after the previous painting fades to black (49 to 50.4) | 5.3 (53 to 58.25); embers drift upward | FO 2.1 (58.25 to 60.4), then 5.6 s black |

Distinctive: essentially no designed text except the studio mark and the title; both fade in and out slowly (1.6-2.1 s) and sit in long black; the museum conceit uses slow dips to black.

#### 2.10 Berserk Musou (yt-m5w1UlJb-ZY, 52 s, Japanese) - https://www.youtube.com/watch?v=m5w1UlJb-ZY

| time | kind | entrance | hold | exit / transitions |
|---|---|---|---|---|
| 30-38.5 | title: a blood smear forms a red brush wordmark (33-37), then "無双" in chrome kanji, ~60% height | brush/ink smear forms the wordmark over ~2 (35.5 to 37.5); "無双" blur-to-focus plus shrink: enters blurred at ~150% scale (36.8), decelerates to sharp in about 0.95 (to 37.75), a hit flash/sparks at 38.375 (1 frame), then snaps to the small lock-up (~15% height) in 0.15 with red dust/smoke | 4.1 (38.5 to 42.6); slow smoke/ember drift | glitch/smear out (42.5 to 42.67) |
| 42.67-44.8 | date card "2016年発売" (white serif, ~7% height, centre) | red/white RGB smear/glitch settles in about 0.2 (42.67 to 42.83) | 1.7, static | horizontal motion-blur streak 0.2 (44.8 to 45.0) |
| 45.0-47.3 | credits card: lock-up top, dense tiny white credits, platform logos, URL (~1% height) | HC | 2.3 | HC |
| 47.3-52 | studio logo (white, ~12% height) | HC | 4.7 | end (fade unclear) |

Distinctive: the only teaser with a real impact title (slam through blur with a hit frame), followed by a text-dense end sequence of three cards separated by hard cuts and smears.

## 3. Genre synthesis

### COZY (4 teasers)
- Cards on black: none. Cozy teasers carry their text over the picture; there are no in-body script cards.
- Overlays: enter by hard cut together with the shot (Waterful keywords, "AT YOUR OWN PACE", the From Scratch mid title) or word-by-word pop (about 0.6 s per word; subtitle words about 0.5 s apart). Secondary lines type on per character (0.4 s). Hold: 1.4-2.5 s for keywords, 6.8 s for a motto, with continuous gentle drift, slight rotation and letter wobble. Exit: carried across the cut and then FO 0.5 s over the new picture, or removed at the next HC. Placement: lower-left for greetings and subtitles, upper-centre for keywords.
- Title: three kinds: FI 0.25-0.4 s after an HC into the shot (From Scratch); FI 0.65-0.75 s (Waterful, Brushland); handwriting write-on 0.5-0.75 s (Dressmaker). Median fade-in about 0.5 s. One end card cuts on with no fade (Waterful). Hold: mid-body title 0.9-1.4 s; end card 3.9-7.3 s (median about 5 s). Motion during the hold: static, a slow tilt (Brushland), or a twinkle glint (Dressmaker). Exit: HC (mid-body, 2 of 2), FO to black 0.55 s (From Scratch), dissolve to white 0.85 s (Waterful), none (Dressmaker).
- End card: logo plus CTA; the CTA types on (0.4 s) or appears by cut; a date ribbon opens with a centre wipe of 0.15 s.
- Transitions: nearly all hard cuts. The only softer ones are a mist dissolve into the title (Brushland), and at the very end a dissolve to white (0.85 s) or FO to black (0.55 s). No flashes and no dips seen. Text across a cut is allowed.

### RPG (6 teasers)
- Cards on black are the genre's main device (Warlock: 3 cards of 2-2.3 s; Berserk: date, credits, studio; Persona: platform and publisher logos). They are joined by 0.3-0.5 s cross-dissolves (Warlock) or hard cuts through black (Persona, Berserk). Wide-tracked small caps for dates (about 2% height).
- Overlays on the picture are rare: only the Japanese DQ lines (small mincho serif, 3% height, FI about 0.4 s, hold 3.3 s, cross-dissolve between lines 0.7 s).
- Studio and publisher logos fade slowly (EN: 0.75 s in; Persona publisher 2.0 in and 1.2 out; JA studio mark 1.6 in and 2.1 out), hold 2.4-3.9 s.
- Title: cut-on with a burst or hit and then a settle (Persona neon ring and smoke settling over 2.5 s; Berserk blur-to-focus slam 0.95 s, hit flash, snap 0.15 s; DQ slam-from-large 1.4 s ease-out, numeral drop, light-sweep morph 0.8 s; Lair a flash then a light sweep 0.55 s; Warlock logo cut-on, then background fading up 1.4 s; RPG Maker a plain slow FI 2.0 s). Hold 2.8-8 s; end card 3.5-8.3 s (median about 4 s). During the hold: smoke, embers, sparkle, or a static lock-up. Exit: FO 1.2-2.1 s (publisher, RPG Maker, DQ), HC to black (Persona), dissolve 0.3-0.5 s into the next card (Warlock), smear/glitch 0.2 s (Berserk).
- End card: dense small text (platform icons, legal, credits) after the logo; fades in from black over 0.45-0.5 s; static 3.5-4.7 s.
- Transitions: white-out (DQ: rise 0.2-0.5, hold 0-0.5, decay 0.7-1.0), a 2-frame flash into a key-art end card (Lair), 1 s black beats between shots (Persona), 4.1 s black before the title (Persona), slow dips to black (RPG Maker: 1.7 s out, 1.9 s black), dissolves between cards (Warlock). Hard cuts remain the default inside action montages.

### What differs between the genres
Cozy puts the motion on the words (pop per word, type-on, write-on, constant gentle wobble), titles arrive quickly (0.25-0.75 s), transitions are hard cuts, and nothing happens in black. RPG puts the motion on the lock-up (slam, burst, light sweep, morph), uses many text cards on black, titles are either violent or slow (never a quick plain fade), and transitions are fade-through-black, white-out or dissolve.

### Japanese teasers vs the English ones (RPG set only)
- Slower, more formal fades for studio mark and title (1.6-2.1 s in and out in RPG Maker, against 0.45-0.75 s in the EN end cards).
- More text events at the end: date, credits and studio (Berserk: three cards in 9 s, with ~1% height legal text); a Japanese tagline under the emblem; small mincho subtitles that cross-dissolve line by line (DQ).
- Small serif text paired with one huge impact word (無双 blurs in at ~150% scale then shrinks), the supporting text stays small and calm.
- Two of the three JA teasers have no on-picture text except logos; text lives on black or on key art.
- Caveat: only three JA teasers, and none for cozy.

## 4. Proposed motion profiles (measured values only; "(inferred)" marks anything else)

### cozy
- text_in (overlay): `pop-per-word` at 0.6 s steps, or `cut` with the shot; `type-on` 0.4 s for secondary lines; `fade` 0.25-0.4 s as the plain fallback.
- text_hold_motion: slow drift plus at most about 3 degrees of rotation and per-letter wobble over the hold (Waterful). For plain text: none (inferred).
- text_out: carried over the cut then `fade` 0.5 s, or `cut` with the shot.
- title_in: `fade` 0.4-0.75 s (median about 0.5), or `write-on` 0.5-0.75 s for script logos; a hard-cut-on end card means `cut`.
- title_hold: mid-body 0.9-1.4 s; end 4-7 s (median 5). Hold motion: none, or a twinkle glint (non-script logos: none, inferred).
- title_out: `cut` (mid-body); `fade to black` 0.55 s or `dissolve to white` 0.85 s (end).
- end_card_cta: `type-on` 0.4 s or `cut`; date ribbon `wipe` from the centre 0.15 s.
- transition_default: `cut`. transition_accent: `dip` to black or white 0.5-0.9 s only at the very end; a mist `dissolve` 0.75 s into the title (rare). No flash.
- overlay placement: greeting/subtitle lower-left, ~7% height; keywords upper-centre, ~8-12% height, slightly tilted; title centre, ~15-20% height.

### rpg
- text_in (card on black): `fade` 0.3-0.5 s, or a `dissolve` of 0.3-0.5 s between cards; wide-tracked small caps (2% height) for dates; Japanese serif lines FI 0.4 s.
- text_hold: static, 2.0-3.3 s per card (Japanese lines 3.3 s).
- text_out: `dissolve` 0.3-0.5 s into the next card; `cut` at the end of a block.
- studio_logo: `fade` 0.75-2.0 s in, hold 2.4-3.9 s, `fade` 1.2-2.1 s out.
- title_in: `cut-on plus burst/settle` (about 2.5 s settle), or `slam` (blur-to-focus from 150% scale, ease-out) 0.95-1.4 s with a hit frame; a quiet title uses `fade` 1.4-2.0 s (RPG Maker) or a `light sweep` 0.55 s after a 2-frame flash (indie).
- title_hold: 3-8 s (end card 3.5-8.3 s, median about 4); hold motion: smoke/embers/sparkle, or none.
- title_out: `fade to black` 1.2-2.1 s (median 1.5), or a `dissolve` of 0.3-0.5 s to the next card, or `cut` to black.
- end_card: `fade` in 0.45-0.5 s from black, static 3.5-4.7 s, small legal rows.
- transition_default: `cut`. transition_accent: `white-out` (rise 0.3, hold 0.4, decay 0.9), 2-frame flash 0.1 s, `dip` to black 0.5-1.7 s (RPG Maker, Persona); a longer black before the title (Persona 4.1 s; for shotpilot 1.0-2.0 s is inferred).
- overlay placement: JA subtitles centre, ~3% height; dates centre, 2-7% height; title centre, ~30% height; logos on black.

### Mapping notes for shotpilot (inferred)
- Today's `dip` 0.25 s and `flash` 0.12 s are close to the cozy end-of-teaser values (shorter) and the RPG 2-frame flash (0.1 s). An RPG-style `white-out` needs a longer white (0.3 s rise, 0.4 s hold, 0.9 s decay), which changes the cut's length only if the white is inserted rather than overlaid.
- Per-word pop and type-on can be done with `drawtext` enable windows. Write-on and slam need an alpha mask or a scale expression and are the hardest to render.
