# Typography in ADVENTURE and COZY teasers (measured)

Scope: 10 adventure teasers (3 Japanese-language) and 10 cozy teasers (none Japanese; no JA cozy teaser exists in the corpus). Third-party works, no frames stored in the repo. Sizes are % of frame height (H). "cap" = Latin capital height, "kana" = height of a kanji/kana. Eyeballed on 1280x720 / 1920x1080 frames (error about +/-1 point of H; small text +/-0.5). "block" = whole title lockup height.

## 1. Method

- Text moments were taken from the earlier motion notes (`モーションデザイン/corpus_horror_adventure.md`, `corpus_cozy_rpg.md`) where they existed; for the other 11 teasers a 1 fps overview sheet (200-240 px tiles) was used to find every designed text moment.
- Then 1-2 full frames per teaser (1280 px; 1920 px for small text) at the title, the end card and one card/overlay. Heights were read in pixels and divided by the frame height. Leading = baseline-to-baseline distance divided by the font size (font size taken as ~1.4 x cap for caps text, ~1.0 x kana for Japanese).
- Typeface class and "looks like" are judged by eye; confidence is low unless stated. Tracking estimates are visual (em).
- Ignored: in-game UI/HUD, dialogue boxes, platform/publisher logos (size noted only), legal lines (1.2-1.8% H, grey/white, bottom, 2-4 lines).
- "unclear" = not readable at the sampling used. Where a teaser has many kinetic words (Danganronpa), only the class is recorded.

## 2. Per teaser

### Adventure

#### yt-qDFEeeLy6ws - Hollow Knight: Silksong - Sea of Sorrow (https://www.youtube.com/watch?v=qDFEeeLy6ws)
| role | script | class / looks like | weight, case, tracking, size | position | colour, treatment, bg |
|---|---|---|---|---|---|
| title lockup | Latin | custom lockup, serif display with flared strokes (looks like Albertus/Trajan-ish, low), small-cap "of" | bold, ALL CAPS, normal (~0.02em); SILKSONG cap 16%, SEA OF SORROW cap 13%, HOLLOW KNIGHT cap 3.8%; block 47%, width 64%; 3 lines | centre | white + teal (#5cc8c4-ish), floral ornaments above/below, soft glow; dark teal fog |
| date | Latin | serif, same family | regular, numerals "2026", cap 7% | centre | teal, none |
| CTA line | Latin | small-caps sans (unclear) | regular, ALL CAPS, very wide (~0.2em); cap 2.5%; 1 line | centre | white, on black |
Distinctive: 3 text levels with big steps (16 / 7 / 2.5%), one accent colour, no CTA button. Faces 2-3. Ratio largest/smallest ~6.

#### yt-GUnZa0ghIow - Become
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title | Latin | custom lettering, chunky rounded sans (O is a drawn mark) | black, ALL CAPS, very tight (~-0.02em); cap 21%, width 56% | centre-right, tilt ~3 deg | cream (#f3ecd0), thin dark edge, over burst art |
| in-scene words ("DEEP INSIDE...", "COMING 2026") | Latin | 3D chunky rounded sans, diegetic | black, caps; cap 9-12% | centre / angled | teal-white, 3D bevel |
Distinctive: all text is in the scene or a logo, no designed overlay. Faces 1. Credits/Steam tags ~2%.

#### yt-q21jZaUOa4Q - Mighty Cuphead Adventure
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| heading "Currently in Development" | Latin | transitional serif (looks like Miller/Chronicle bold, low) | bold, mixed, normal; cap ~5%; 2 lines, leading ~1.0 | upper left, left-aligned block | off-white with glow/VHS chroma bleed, on picture |
| list lines (3) | Latin | bold grotesque/humanist sans | bold, Title Case, normal; cap ~2.5% | under heading, check-box icons | white, glow |
| game logo (on a TV) | Latin | custom hand lettering, bouncy slab/brush | black, mixed, tight; ~25% of frame (inside TV) | right | yellow-orange, dark outline |
Distinctive: retro-ad pastiche (4:3 pillarbox); heading/list ratio 2:1. Faces 3.

#### yt-7SFCD_MMQ6Q - Maskborne
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title | Latin | custom hand-drawn lettering (rune-like, uneven) | bold (stroke ~1% H), ALL CAPS, wide (~0.08em); cap 19% per line, 2 lines split by a rule with a circle; block 49% | left third, left-aligned | white, soft glow; dark teal key art, glowing emblem right |
| CTA "Wishlist on Steam" | Latin | handwriting-like humanist sans | regular, mixed, normal; cap ~5% | under title, left | white |
Distinctive: logo and CTA share one drawn hand; title left, emblem right. Faces 2.

#### yt-sPfQoHlb4ZI (ja) - Shadow mate ~小さな竜と適合者~ (https://www.youtube.com/watch?v=sPfQoHlb4ZI)
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title | Latin | custom lettering on a high-contrast serif (Didone-leaning), distressed texture, flame flourishes, red eye in the "o", oversized S and M | bold contrast, mixed with swash caps; S cap 19%, x-height 11%; width 58% | left of centre, left-aligned | black, rough-edge texture, on white key art (character right) |
| sub-title | Japanese | mincho, medium | regular, kana 4.2%, wide (~0.1em), 1 line, 8 chars | under "ow Mate" | black |
Distinctive: sub-line is 22% of title cap; mincho paired with serif Latin logo; light (white) end card is unusual. Faces 2.

#### yt-pL5c8WYfqv8 (ja) - メルクリウスの青い砂 ReCollection (https://www.youtube.com/watch?v=pL5c8WYfqv8)
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title | Japanese | mincho with brush-like entry strokes (looks like a Shippori/Tsukushi-type mincho, low); one kanji (青) in blue | bold-ish, tight; kana 9.7%; 1 line, 10 chars | centre | white + blue accent, on black |
| title bar | Latin | pixel-cut bar with rounded bold sans "ReCollection" | bold, mixed, wide (~0.1em); cap ~3% | under title | cyan bar, dark text |
| date line "2024 Back" | Latin | serif | regular; cap ~4% | centre | grey-white |
| captions | Japanese | gothic, medium (looks like Noto Sans JP, medium) | regular, kana 3.7%, normal, 2 lines, leading ~1.75 | lower centre, left-aligned block in black letterbox, bottom margin ~7% | white, no stroke or shadow, on black bar |
| card lines on black | Japanese | mincho | regular, kana ~2.5%, 1 line | centre | white |
Distinctive: mincho title, gothic captions, serif card lines. Captions end with ー or ？, no 。; no 「」 in the lines read. Faces 4.

#### yt-irCp04t09DY (ja) - Erase -missing link- ティザーPV 1st (https://www.youtube.com/watch?v=irCp04t09DY)
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title logo | Latin | custom lettering: wide tilted display caps, final E drawn as a 3 | black, ALL CAPS, tight; block 32% (cap ~30%), width 40% | left-centre, tilt ~-4 deg | purple-magenta gradient, yellow outline ~1% H; sunset art |
| sub-title "missing link" | Latin | high-contrast serif with odd-cased caps | light, lowercase + swash S, wide (~0.08em); ~3.5% | under logo, dashes either side | pale lavender |
| genre tagline | Japanese | gothic bold | bold, kana ~3%, normal | above logo, left | yellow (#f2e03a) |
| date "2026年 配信予定" | both | gothic extra-bold | black weight, kana 5.3%, normal; 1 line | lower left, left-aligned | white, soft dark shadow |
| narration card on black | Japanese | handwriting (hand-drawn gothic) | regular, kana 5%, 2 lines, wide (~0.05em), leading ~1.4; ruby 2% over "最高ランク" | centre | cream, on black |
| platform logos, legal | - | - | logos 5%, legal 1.4-1.7% | top bar, bottom | white |
Distinctive: 4+ tiers on the end card (30 / 5.3 / 3.5 / 3%); the card uses handwriting + ruby. Punctuation: 、 and ！; no 。; no 「」. Faces 4-5.

#### yt-rgBJoM_k4CA - Iron Bramble
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title | Latin | custom blackletter (pixel-sharpened Fraktur) | bold, lowercase, tight; x-height 25%, ascender to baseline 46%; width 98% of frame (breaks the 5% margin) | upper centre | red (#e8231f), CRT scanline texture, near-black bg |
| CTA | Latin | condensed grotesque (looks like Bebas Neue / League Gothic, medium) | bold, ALL CAPS, normal (~0.02em); cap 9%; 1 line, 20 chars | lower centre | same red |
| platform | Latin | Steam logo + wordmark | cap 4.5% | centre | white |
Distinctive: logo as full-bleed banner; one colour; size ratio 5:1. Faces 2.

#### yt-j13VaZ4Xn-4 - Scholar Adventure: Lost Night
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| title | Latin | custom geometric sans (A is a triangle in a circle; looks like Futura/Gotham bold, low) | bold, ALL CAPS, normal (~0.03em); cap 13%, width 42% | centre | white, on key art |
| genre line "Scholar Adventure" | Latin | transitional serif | regular, mixed; cap 2.8% | above title | white |
| CTA | Latin | condensed sans (looks like Bebas Neue/Oswald, medium) | bold, ALL CAPS, wide (~0.04em); cap 6% | below title | pale blue (#8ec1e6) |
| text cards on black | Latin | same condensed sans | bold, ALL CAPS, normal; cap 10.4%, 2 lines, leading ~1.1, width 60% | centre | white |
| in-scene subtitles | Latin | small serif | cap ~1.8% | on picture | white |
Distinctive: condensed bold caps for cards and CTA, serif descriptor; ratio 13:2.8. Faces 3.

#### yt-M8LGot_GCEE - Danganronpa 2x2
| role | script | class | weight, case, tracking, size | position | colour, treatment |
|---|---|---|---|---|---|
| logo | Latin | custom italic lettering with a flower ornament | black, mixed, tight; block 33%, width 76% | centre-upper | white, grey 3D extrusion + black outline, on black |
| "COMING 2026" | Latin | geometric/neo-grotesque (looks like Gotham/Avenir bold, medium) | bold, ALL CAPS, wide (~0.04em); cap 6.3% | centre | white |
| kinetic words (THRILLING, SCHOOL TRIP, IS NOT OVER YET!, TRIA, ALLRISE!, COUNTER) | Latin | custom per word: pop, brush, graffiti, comic | black, caps, tight, tilt -6 to +8 deg; cap 14-35% | centre / fills | colour-block backgrounds, outlines + shadows |
Distinctive: a one-off typeface per beat, tied together by outline and colour; logo gets 3D extrusion. Faces 6+ (3 systems).

### Cozy

#### yt-ZLXfWsIWog8 - Brushland
Title: custom swash blackletter-flavoured serif, cream (#fbf7d0) with thin navy outline (~0.4% H), mixed, tight; B cap 28%, x-height 15%, width 58%; centre; over blurred castle art. End card: same logo + "WISHLIST ON STEAM" tiny sans (~2.5%, caps, wide) + credit line. Faces 2. No other designed text.

#### yt-QtGJNp2QWEw - Dressmaker
Title: calligraphic script, white with soft shadow; D cap 22%, block 28%, width 76%; mixed, loose. Ribbon: pink (#e8286a) banner (width 48%) with white serif small-caps "Wishlist Now" (cap ~6%, "w" raised), centred below. Date "September 2026" serif ~3.5%. Faces 2. Ribbon = sticker/label shape.

#### yt-CP61GovN7ZE - From Scratch
Title: marker-brush rounded sans (custom), white with hard offset darker shadow (~0.7% H), mixed, tight; 2 lines, cap 19-25%, leading ~0.9 of cap (lines nearly touch); centre; blurred table. UI-style tab captions ~2.5% (teal). CTA "Wishlist on Steam!" ~5% same hand sans. Faces 2.

#### yt-vmVVsImRjHo - Waterful
Title: chunky hand-drawn rounded caps, white, cap 27%, width 78%, water-drop mark above. CTA "WISHLIST NOW / ON STEAM": handwriting caps ~6%, wide (~0.08em), 2 lines, centred. Keyword overlays DIG / GROW / UNLOCK / DISCOVER: hand caps, cap 12-19%, cream + pale offset outline, tilt ~-8 deg, upper centre; "AT YOUR OWN PACE" 8%; greeting ~7% lower-left. Sponsor logos 3%. Faces 3.

#### yt-1c628o2iQ7A - Spiritstead
Title: bubbly rounded display, yellow (#ffd84a) with dark-magenta edge, mixed, tight; cap 17% (x 12%); upper centre, sky. "PLAY THE DEMO": rounded bold caps, pale blue-white, cap 4.2%, wide (~0.08em). Cards between shots ("BUILD YOUR VILLAGE", "UNLOCK QUIRKY / YET HELPFUL SPIRITS"): rounded hand-drawn caps, white, cap 4.2%, 2 lines, leading ~1.25, centred, on a flat saturated blue card with clouds (not black). Faces 2.

#### yt-OCeemWKL3bY - Station To Station
Title: badge lockup (steam-loco crest + ornate Victorian display "STATION to STATION"), copper gradient with outline; block 40% at left. CTA "PLAY THE DEMO & / WISHLIST NOW": condensed embossed copper caps (cap 3.5% / 5.1%), lower centre; STEAM logo 4%; publisher logos 6%; legal 1.2%; over a white fade at the bottom. Faces 2-3.

#### yt-EDjk39k-BHM - A Little to the Left: Cupboards & Drawers
Title: small badge of hand lettering on a card tilted ~-5 deg, ~17% tall, lower left. Info block lower left: turquoise (#4fd1b8) humanist sans 4.2%, grey-white regular sans 3.2%, date 2%; on a dark gradient band. Overlay "Coming June 27th": white sans 3.5% on a dark box (座布団), top. Faces 3.

#### yt-m-PE7BPskpw - The Wandering Village: The Last Leviathan
Title: flared serif display lockup (looks like Cinzel/Trajan-ish with swashes, medium), white on a dark monster silhouette; "THE WANDERING VILLAGE" cap 7-8%, "The Last LEVIATHAN" teal gradient cap 6-11%; block 56%; left. CTA "Wishlist DLC now": bold serif (looks like Lora/Merriweather bold, low), cap 5.5%, white with dark teal outline ~0.4% H, lower left-centre. Platform row top right ~4%. Faces 3.

#### yt-0ZWdQi78Rg4 - My Ransomware Can't Be This Cute?!
Title: pixel-art chunky caps, cream with dark-purple outline; "MY" and "CAN'T BE THIS" on dark labels (座布団); cap 14% (RANSOMWARE), 11% (CUTE!); upper left, tilt ~-3 deg. CTA ">wishlist on steam<": pixel sans ~3%, white, over art. Studio logos 4%. Faces 2-3.

#### yt-mE5XKbbLE34 - Ooblets (1.0 launch)
Title: bubbly rounded lettering, each letter a different pastel with navy outline ~1.5% H and shadow; O cap 23%; centre-upper. "LEAVING EARLY ACCESS": italic rounded bold caps, navy, 4.5%. "1.0 LAUNCH": white bold rounded caps, soft shadow, cap 9%. "THIS SUMMER": yellow bold caps 5%. Platform badges 9%; legal 1.2%. Flat mint (#8de9e5) bg. Faces 2-3.

## 3. Genre synthesis

### Adventure (10)
- Title: custom lettering 10/10 (serif-based 3: Silksong, Shadow mate, Mercurius (mincho); blackletter 1; hand-drawn 2: Maskborne, Erase; chunky rounded 1; geometric 1; italic 3D 1; hand-lettered slab 1 for Cuphead). No title was a plain system font.
- Title cap height: Silksong 16, Become 21, Shadow mate 19, Erase ~30, Maskborne 19 per line, Lost Night 13, Iron Bramble x-height 25, Mercurius kana 9.7. Median cap ~19% H; Latin block heights 32-49% (median ~46%). Japanese mincho title is the smallest (9.7%).
- Secondary lines (CTA, date, descriptor): condensed or neo-grotesque sans in ALL CAPS in 5/10 (Iron Bramble, Lost Night, Danganronpa, Silksong small caps, Erase numerals bold gothic); handwriting 2/10; serif 3/10. CTA cap median ~6% (2.5-9); caps tracking normal to +0.04em, very wide (~0.2em) only on 2.5% lines.
- Weight: titles bold/black 9/10 (Mercurius regular-ish); CTA bold 6/10.
- Colour: white 7/10; one accent 6/10 (teal x2, red, pale blue, yellow, purple+yellow). Title treatment: none 4, outline 1, glow 2, 3D extrusion 1, texture 2 (distress, CRT).
- Cards on black: Lost Night (condensed caps, cap 7-10%, leading ~1.1), Silksong (2.5%), Mercurius (mincho 2.5%), Erase (handwriting kana 5%).
- Placement: centred title 6/10, left 3/10, other 1; CTA directly under; margin breaker: Iron Bramble banner (98% width). Legal lines 1.2-1.8% at the bottom 3/10.
- Faces per teaser: 1-6, median 3. Largest/smallest ratio (excluding legal): ~4 to 15, median ~6.
- Number styling: year as its own line 4/10; Erase uses bold gothic with Latin numerals, 5.3%.
- Clichés: condensed bold caps for cards/CTA 3/10; blackletter 1/10; flourish above/below title 1/10; Steam row under CTA 5/10.
- Japanese (3): title Latin custom 2/3, kanji mincho 1/3. Sub-lines mincho (Shadow mate, Mercurius) or bold gothic (Erase). Captions: gothic regular, kana 3.7%, bottom 7%, leading ~1.75, no stroke, on a black bar. Handwriting narration card with ruby (kana 5%, ruby 2%). Vertical text 0/3. 和欧 pairing: sub-line 4.2% vs cap 19% (0.22) in Shadow mate. Punctuation: 、 and ！ used, ？ at line end, 。 not seen, 「」 not seen in designed text.

### Cozy (10)
- Title: custom lettering 10/10 (rounded/bubbly 4: Spiritstead, Ooblets, Waterful, From Scratch; script 1; swash blackletter-serif 1; badge/Victorian 2; pixel 1; flared serif 1). Rounded or hand-drawn forms 6/10.
- Title cap height: Brushland 28, Dressmaker 22, From Scratch ~22, Waterful 27, Spiritstead 17, Ooblets 23, Ransomware 11-14, Wandering Village 8-11 per line (block 56%). Median cap ~22% H; width 40-78%. Block: Station 40%, Little Left badge 17%.
- Case: mixed 8/10, ALL CAPS 2/10. Tracking: tight on titles 10/10.
- CTA and info lines: ALL CAPS bold rounded or condensed 5/10; serif 2/10; handwriting 2/10. CTA cap median ~5% (3.5-9.7); caps tracking +0.04 to +0.08em.
- Colour: pastel or saturated multi-colour 6/10; white 5/10. Outline 6/10 (0.4-1.5% H, dark hue); hard offset shadow 2/10; soft glow or shadow 3/10; ribbon/label/box (座布団) 3/10.
- Background: over blurred key art 5/10, flat colour card 3/10, on black 0/10 (cards go on saturated colour instead).
- Cards/overlays: Spiritstead cards (cap 4.2%, centred), Waterful tilted overlays (cap 12-19%, -8 deg), From Scratch UI tags (2.5%).
- Faces per teaser: 2-3 (median 2.5); largest/smallest ratio ~4-5 excluding legal (Brushland ~11).
- Clichés: bubbly rounded title 6/10; dark-outline pastel 6/10; Steam icon + "Wishlist" line 8/10; tilted keyword overlays 1/10.
- No Japanese cozy teaser is available; Japanese cozy typography stays inferred.

## 4. Proposed typography profile (measured; "(inferred)" = not measured)

### Adventure
- Display: Latin = custom-looking serif display, blackletter, or hand-drawn caps, bold; Japanese = mincho (title), hand/brush for sub-lines.
- Body/card: Latin = condensed or neo-grotesque sans, bold, ALL CAPS; Japanese = gothic regular for captions, mincho for quiet cards.
- Weight: title bold/black; cards bold; captions regular.
- Case/tracking: title mixed or caps, tight (0 to -0.02em); CTA caps +0.03 to +0.04em; tiny lines up to +0.2em.
- Size (% of H): title cap 13-21 (median 19), Latin block 32-49; Japanese title kanji ~10 (inferred 8-12); card 7-10 on black (2 lines, leading ~1.1 for caps); CTA cap 6 (2.5-9); date 7; descriptor 2.8-3; JA caption kana 3.7, leading 1.75, bottom margin 7%; JA sub-line 4.2 (0.22 x title cap); ruby 40% of base.
- Colour/treatment: white on black/dark; one accent colour; no stroke on cards; glow or 3D on logos only; band only as letterbox black bar.
- Placement: title centre (6/10) or left third (3/10); CTA directly under; captions lower centre; margin >=5% except a banner logo.
- Faces per teaser 2-3; ratio ~6.

### Cozy
- Display: Latin = rounded/hand-drawn bubbly or script display, mixed case, tight; Japanese (inferred) = maru gothic or handwriting.
- Body/card: Latin = rounded bold sans or handwriting caps, ALL CAPS; Japanese (inferred) = bold maru gothic.
- Weight: bold/black for title and CTA.
- Case/tracking: title mixed, tight; caps lines +0.04 to +0.08em.
- Size: title cap 17-28 (median 22), block 28-56; CTA cap 5 (3.5-9.7); card 4.2 (2 lines, leading ~1.25); overlay keyword cap 12-19 (tilt -8 deg); info 3-4; legal 1.2.
- Colour/treatment: white or pastel/saturated accent; dark-hue outline 0.4-1.5% H; hard offset or soft shadow; ribbon/label for date or CTA (3/10); cards on a flat saturated colour rather than black.
- Placement: centre / upper-centre title; CTA below; info block lower-left (1/10) or lower centre.
- Faces per teaser 2-3; ratio ~5.
