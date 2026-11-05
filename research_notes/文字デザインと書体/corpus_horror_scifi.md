# Typography of text in horror and sci-fi teasers (measured)

Scope: 10 horror and 5 sci-fi teasers (3 Japanese-language horror, 1 Japanese sci-fi game, 1 Japanese anime PV) from `research_data/teasers/videos/`. No frames are copied into the repo. Text timing and motion are in `research_notes/モーションデザイン/corpus_horror_adventure.md` and are not repeated here.

## 1. Method

- Overview contact sheet per teaser (1 fps for clips up to 60 s, 0.5 fps for longer; 240 px tiles) to find every designed text moment. For fAXzLmsbrd8, yd6oGFqE1F8, IkXbe0zTAck and LnydjhGRH1s the text times from the motion corpus were reused.
- One to three full frames per teaser at 1280 or 1920 px wide, read by eye. Sizes are pixel extents on the 1080-line frame (720-line frames scaled), as a % of frame height: cap height for Latin, one kanji/kana square for Japanese; "block" is the height of the whole lockup. Accuracy about +/-1 point for small text, +/-3 points for logos. Tracking in em is an eye estimate from letter gaps (+/-0.05 em).
- Typeface class is a visual classification. "looks like X" appears only where a known face is plausible, with confidence. Third-party logos (platform, engine, publisher) are ignored; legal lines are listed only for size.
- Not measured: exact hex colours (named approximately), numeric weight. "unclear" = could not be told from the frames used. Some mid-teaser captions (DH3O9ouf5Yo 40-48 s, Lmikfxhi8E0 kinetic names) were judged from tiles only.
- 5% margin = 54 px vertical, 96 px horizontal.

## 2. Per teaser

Abbrev: ja = Japanese; 座布団 = dark box behind text.

### Horror

#### yt-fAXzLmsbrd8 - Escape From Playtime release date trailer (https://www.youtube.com/watch?v=fAXzLmsbrd8)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title logo (custom) | Latin | custom hand-lettered caps bent along the mouth of a grinning-cat badge | black / ALL CAPS / tight | wordmark cap ~14%, badge ~85-90% of frame h, 2 lines + tiny "FROM" | centre | white on black, strong white glow, CRT scan/chroma jitter | the logo is the picture |
| end card, 4 lines | Latin | pixel/bitmap CRT font (VT323-like, low confidence), mixed case ("Releasing October 21st") | regular / mixed / normal (~0.05) | cap ~6-7%, leading ~1.45, 14-22 chars/line | centre, last line ends at 94% h (at the bottom margin edge) | off-white, soft glow, horizontal scanline blur | Steam icon between lines |
| studio logo | Latin | custom | - | ~30% (per motion note) | centre | white-cyan glow | |

Typefaces: 2 (custom lettering, pixel font). Largest:smallest about 2:1 (no small print). Distinct: one CRT/VHS treatment applied to all text.

#### yt-yd6oGFqE1F8 - The Cat and the Hatchet (https://www.youtube.com/watch?v=yd6oGFqE1F8)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title, 3 lines "THE CAT / AND THE / HATCHET" | Latin | distressed brush/drip display, set in a font (looks like a free horror/dripping font, low) | bold / ALL CAPS / tight (~0) | cap ~14% (lines 1 and 3), "AND THE" ~8%, block ~50% | centre | dark blood red (#8a0f14), fuzzy edges, drips, on black | hierarchy by size only |
| cards "THIS HALLOWEEN", "FUN COMES WITH A PRICE", "COMING SOON" | Latin | same distressed face | bold / ALL CAPS / wide (~0.1) | cap ~3% | centre | dim red (lower luminance than title), on black | 14-21 chars |

Typefaces: 1. Ratio ~5:1. Distinct: low-key red on black; cards deliberately dim.

#### yt-IkXbe0zTAck - Indigo Park: Chapter 2 (https://www.youtube.com/watch?v=IkXbe0zTAck)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title logo (custom) | Latin | brush-script wordmark "Indigo" + bold sans caps "PARK" + ring emblem | bold / mixed + caps / normal | block ~44% h, script cap ~18% | centre | lavender fill, purple outline, pale-blue moon disc, neon glow | custom lettering |
| "CHAPTER 2" | Latin | thin geometric sans (looks like Montserrat Light, medium) | light / ALL CAPS / wide (~0.1) | cap ~5.5%, 1 line | centre | pale grey, "2" in blue; light glint | "IN DEVELOPMENT" below, cap ~2.5%, tracking ~0.08, dimmer |
| copyright | Latin | same sans | regular / caps / ~0.03 | cap ~1.7% | bottom centre, y~91% | grey, ~45% luminance | persists on screen |

Typefaces: 3. Ratio ~26:1 with the copyright line.

#### yt-LnydjhGRH1s - Farsight reveal (https://www.youtube.com/watch?v=LnydjhGRH1s)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title logo (custom) | Latin | heavy condensed grotesque with distressed edge; silhouettes of figures bleed out of the bottom of the letters | black / ALL CAPS / tight | cap ~35%, width ~64% | upper centre (y 19-54%) | white on black, distress texture | 1 line, 8 chars |
| end card "COMING 2026 / WISHLIST ON" | Latin | wide-ish grotesque (Neue-Montreal/Satoshi-like, low) | bold line 1, regular line 2 / ALL CAPS / slightly tight (~-0.02) | cap ~3.5%, line pitch ~1.75x cap | centre, y 33-43% | white | Steam logo, studio logos at ~76% |
| credits/legal | Latin | same sans | bold / regular | ~1.2-1.5% | centre, y 85-94% | white | |

Typefaces: 2. Ratio ~30:1 with legal. Distinct: the biggest title measured (35% h).

#### yt-7naiEFFSjcI - HOTEL MIROIR (TWINLIGHT: THE LAST BULB), Japanese (https://www.youtube.com/watch?v=7naiEFFSjcI)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| narration caption, 2 lines (3-7 s) | ja | mincho (Yu Mincho / Hiragino Mincho class) | regular / - / very wide ~0.3 em | kana ~5.1%, 8-9 chars/line, leading ~1.4 | lower right, box right edge at 95.3% (inside margin) | off-white; 座布団: near-black box with gold hairline | sentence ends with 。 |
| name-plate lines "『ノア様 ――ご新規のお客様』" | ja | mincho | regular / - / wide ~0.2 | kana ~3.9%, 2 lines of 16-17 chars incl. 『』 ―― | right half, vertical centre, framed box | gold (#d9c58a) | Latin label "GUEST BOOK": roman small caps, cap ~1.9%, tracking ~0.15, gold |
| lower captions (~20 s, 33 s) | ja | mincho | light / - / wide ~0.3 | kana ~2.4%, 1-2 lines of ~20 chars | lower centre (y~70%) | pale grey-gold, on picture, no box | ends with 。 |
| title "HOTEL MIROIR" | Latin | Roman inscriptional caps (Cinzel/Trajan-type, medium), set in a font | regular / ALL CAPS / wide ~0.08 | cap ~5.7% | centre, upper-mid | gold gradient, thin rule + diamond under; drawn flourish above | |
| katakana reading above title | ja | gothic, medium | regular / - / ~0.1 | kana ~3.1% | centre | gold | ratio to title cap 0.55 |
| sub-title logo "TWINLIGHT! THE LAST BULB" | Latin | ornamented Roman caps logo (custom) + small caps line | bold / caps | logo cap ~5.5% incl. ornaments; small line ~2% | centre | gold, bevel | |
| end line "ブラウザで、チェックインできます。 URL" | ja + Latin | gothic + humanist sans for the URL | regular / - / normal | ~3.1% | lower centre y~84% | gold | JP and Latin same size; 、。 used |

Typefaces: 5 (mincho, gothic, Roman caps, ornamental logo, URL sans). Ratio ~3:1. Distinct: all gold on oxblood; very wide mincho; 『』 and ―― punctuation; 座布団 with hairline frames; thin gold frame with corner flourishes on the end card.

#### yt-TGGJmrgHOY0 - 蚕代 -Koshiro-, Japanese (https://www.youtube.com/watch?v=TGGJmrgHOY0)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title "蚕代" (custom) | ja | drawn brush/knife-cut distressed kanji, slanted strokes | black / - / tight | kanji ~27% each; block 27% h x 33% w | left, vertical centre (x 7-43%) | white with blood-red splatter and thread lines, distress; over picture | horizontal |
| "-Koshiro-" | Latin | roman serif (Trajan/Cinzel-like, low) | regular / mixed / very wide ~0.35 | cap ~3.1% | under the title | off-white; hyphens as ornaments | |
| "Coming soon" | Latin | light humanist sans (Segoe UI Light-like, medium) | light / mixed / normal | cap ~5.1% | lower centre, y~85% | white, no treatment | text bottom at 89% |

Typefaces: 3. Ratio ~9:1. Distinct: title only at the end, on the key art; nothing else in the 22 s.

#### yt-o2r-Z-NPn6M - Nightmare: The Lunatic (Switch), Japanese (https://www.youtube.com/watch?v=o2r-Z-NPn6M)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title logo "NIGHTMARE" (custom) | Latin | decorative pointed display, cross as the T | bold / ALL CAPS / tight | cap ~16% | left, y 27-45% | off-white, black outline, grey glow | "THE LUNATIC" cap ~5.5%, same family |
| katakana reading | ja | gothic, medium | regular / - / ~0.05 | kana ~2.3% | under logo, left | white-lilac | width about 0.5 of logo |
| date "2025年12月4日(木) 発売予定" | ja + digits | rounded bold gothic | bold / - / normal | digits ~3.7%, 年月日 ~80% of digit size | left, y 63% | white + dark outline ~3 px | numerals larger than units |
| price lines | ja | bold gothic | bold | ~2.6%; (税込) ~65% | left, y 69% | label teal and orange, price white, dark outline | colour-coded labels |
| legal / rating | ja + Latin | sans | regular | ~1.0-1.2% | bottom-left y 94% | white ~70% | in-game caption "画像は開発中のものです" ~1.4% on a bottom letterbox |

Typefaces: 3. Ratio ~16:1. Distinct: left-aligned stack (logo, reading, date, price) inside the left 40%; coloured price labels.

#### yt-nKu6NuC2LvA - OOTY: Silence Remains (https://www.youtube.com/watch?v=nKu6NuC2LvA)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title "OOTY" (custom) | Latin | stencil-cut lettering with a forest and lookout tower carved into the letters | black / ALL CAPS | cap ~24% | left, y 26-50% | off-white, snow texture, on picture | |
| "SILENCE REMAINS" | Latin | geometric/humanist sans (Gotham-like, low) | bold / ALL CAPS / wide ~0.25 | cap ~2.0% | under logo, left-aligned | off-white | |
| "WISHLIST ON / STEAM" | Latin | bold sans | bold / caps | 1.6% / 3.3% | left, y 66% | white | |
| "HAUNTING SOON" | Latin | bold sans | bold / caps / very wide ~0.3 | cap ~1.2% | centre on black | white ~85% | tiny relative to the frame |
| studio logo "InZeros" | Latin | extended sans, red Z | bold / caps | cap ~3.5% | centre | white + red | |
| warning card | Latin | sans in a white box | | ~1% | top-left | | |

Typefaces: 4. Ratio ~20:1. Distinct: very small taglines under a big custom logo.

#### yt-zDVBbHUwpDE - Terrible Lizards (https://www.youtube.com/watch?v=zDVBbHUwpDE)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title logo (custom) | Latin | hand-lettered chunky caps with fang-shaped I/L, 2 lines | black / ALL CAPS / tight | cap ~18% per line, block ~43% h | right half, y 12-55% | teal-green vertical gradient, dark outline, speckle | on picture |
| "WISHLIST ON STEAM" | Latin | distressed bold condensed grotesque caps | black / caps / normal | cap ~8.8% | right, y 63-75%, right edge at 97.9% (outside 5% margin) | off-white with speckle | |
| platform logos | - | logos | | ~3% | y 83% | | |
| legal, 3 lines | Latin | sans | regular | ~1% | y 91-97% | white ~70% | |

Typefaces: 2. Ratio ~18:1. Distinct: CTA nearly as wide as the title; torn teal paper edges.

#### yt-hmil2xt7yV0 - MISTWATER (https://www.youtube.com/watch?v=hmil2xt7yV0)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title "MISTWATER" | Latin | tall condensed sans, light-regular (Bebas Neue / Trade Gothic Condensed-like, low-medium), set in a font | light / ALL CAPS / wide ~0.12 | cap ~20%, width ~53% | lower centre (y 50-72%) | off-white to grey vertical gradient with grain, on picture | |
| "WISHLIST ON" | Latin | same condensed sans | regular / caps / ~0.08 | cap ~7.4% | below, centre | same gradient | |
| Steam logo | - | | | ~3.5% | y 91% | white | |

Typefaces: 1 (plus logo). Ratio ~6:1. Distinct: one condensed light face, gradient fill, no outline or glow; no text before the end.

### Sci-fi

#### yt-jSvjmO2Yess - ハートレジャー Heartreasure: Stellar Journey, Japanese (https://www.youtube.com/watch?v=jSvjmO2Yess)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title (custom) | Latin | hand-drawn bubble/doodle lettering, 2 lines | bold / mixed / normal | cap ~14% (line 1), ~11% (line 2), block 40% | centre, y 28-58% | white fill, thick black outline (~1% h), on white paper | drawn like the art |
| ja reading "ハートレジャー: ステラージャーニー" | ja | thin gothic | light / - / ~0.1 | kana ~3.2% | above title | grey (#555) | |
| date "2025年12月 Steam配信予定" | ja + Latin | gothic regular; Latin "Steam" at the same size | regular / - / normal | ~4.2% | centre, y 70% | black on white | |
| in-game guide pill "絵の世界にかくれた♥を あつめよう!" | ja | rounded gothic (maru-like, low) | regular | kana ~4.2%, 18 chars | lower centre, pill y 85-95%, bottom edge at 96% (outside margin) | dark ink on a cream pill with black outline; ! used, no 。 | repeats at 26, 36, 44 s |
| credits, 3 lines | Latin | light sans | regular / mixed | cap ~1.7% | lower centre, y 85-96% | black ~70% | |

Typefaces: 4. Ratio ~8:1. Distinct: the only light-background teaser; text drawn in the same line style as the art.

#### yt-n4RiVblVQ90 - BELTFED (https://www.youtube.com/watch?v=n4RiVblVQ90)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title (custom) | Latin | riveted rusted metal-plate letters with bullet belt, gears either side | black / ALL CAPS / tight | cap ~17%, width ~55% | upper centre (y 22-40%) | rust orange, bevel, texture | |
| tagline "ROGUELIKE FACTORY DEFENSE" | Latin | monospace (JetBrains/DejaVu Sans Mono class, medium) | regular / ALL CAPS / very wide ~0.5 | cap ~2.0% | centre, y 44% | white ~90% | |
| CTA "WISHLIST NOW ON STEAM" | Latin | bold monospace | bold / caps / ~0.05 | cap ~5.6% | centre, y 66%, inside a full-width black band 16% h with orange hairlines | white, cyan/red chromatic offset on edges | Steam icon left |
| URL | Latin | monospace | regular / caps / ~0.2 | cap ~1.6% | centre, y 95% | grey | at the bottom margin edge |
| captions "AUTOMATE DEATH", "THE FACTORY IS YOUR WEAPON", "THE NEXUS MUST SURVIVE" | Latin | bold monospace | bold / caps / ~0.05 | cap ~6%, 1 line, 20-27 chars | lower third, band y 70-85%, centred | white, slight chroma offset, full-width dark-brown band + orange hairlines | same band style as the CTA |

Typefaces: 2 (custom plate, monospace). Ratio ~11:1.

#### yt-DH3O9ouf5Yo - Mobile Suit Gundam Battle Operation 3 (https://www.youtube.com/watch?v=DH3O9ouf5Yo)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title logo (custom) | Latin | heavy extended sans with cracked edges; green double-slash mark | black / ALL CAPS / tight | "GUNDAM" cap ~10%; "BATTLE OPERATION 3" ~4.2%; "MOBILE SUIT" ~3.3% (tracking ~0.15) | left, y 40-62% (block ~22%) | white, subtle shadow, green accent | |
| "Play for free" | Latin | humanist sans (Frutiger/Noto-like, medium) | medium / mixed / normal | cap ~3.7%; sub-line ~2.4% | left, y 70% | white, soft dark shadow | |
| in-trailer lines "A single strike pierces the front lines" etc. | Latin | bold sans | bold / mixed | ~3% (from tiles; unclear) | lower left to centre, y ~58-65% | red fill, white outline | unclear |
| "A New Battle Operation Series" | Latin | light sans | light / mixed / ~0.03 | ~2% | centre on black | white | |
| legal 4 lines; ESRB | Latin | sans | | ~1% | bottom-left | | |

Typefaces: 3. Ratio ~10:1. Distinct: the end card is the only designed block; left-aligned at 5%.

#### yt-xMbvzuZnYJA - Bay 12: Mech Repair (https://www.youtube.com/watch?v=xMbvzuZnYJA)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title "BAY12" (custom) | Latin | military stencil block caps | black / ALL CAPS / tight | cap ~31% | upper centre (y 20-50%) | light grey gradient with chipped-paint speckle, bar above | |
| "MECH REPAIR" | Latin | heavy extended sans (Bungee-class, low) | black / caps / ~0.02 | cap ~5%, inside a light plate 11% h with a slash | centre, y 53-64% | black on light-grey plate | |
| "WISHLIST NOW!" | Latin | bold monospace | bold / caps | cap ~2.8% | centre, y 74% | yellow (#f5e12a) on near-black box | |
| Steam + studio logos | | | | ~3% | y 90% | white | |

Typefaces: 3. Ratio ~11:1. Distinct: stencil block on black; yellow monospace CTA.

#### yt-Lmikfxhi8E0 - Mobile Suit Gundam RG: XARX-ZERO teaser PV, Japanese (https://www.youtube.com/watch?v=Lmikfxhi8E0)

| role | script | class | weight / case / tracking | size | position | colour / treatment | notes |
|---|---|---|---|---|---|---|---|
| title "XARX-ZERO" (custom) | Latin | heavy extended grotesque, outlined X | black / ALL CAPS / ~0 | cap ~12%, width ~80% | centre, y 47-60% | white on black | |
| title ja "機動戦士ガンダムRG" | ja + Latin | gothic medium | medium / - / wide ~0.25 | kanji ~3.7% | above title | white | Latin "MOBILE SUIT GUNDAM RG" micro line cap ~0.8%; reading "アレックスゼロ" ~2.6% below, tracking ~0.3 |
| story lines, 2 lines (e.g. 地球の放棄を / 余儀なくされた人類の) | ja | gothic medium (geometric-leaning, Shin Go/Noto-like, low) | medium / - / wide ~0.2 | kanji ~6%, 6-9 chars/line, leading ~1.7 | centre, y 43-60% | cool white-cyan (#dceae6), faint glow, on a blue grid texture | first line no 。; "…上がる。" ends with 。 |
| year "2027年" | digits + ja | geometric sans | regular | digits ~5%, 年 ~55% of digits | centre | white | |
| staff names (kinetic, ~50-56 s) | ja | gothic bold | bold | over 50% of frame (cropped) | full frame | white with lime cutout and glitch | |
| end credits | ja + Latin | gothic + geometric sans | label ~1.2% (grey), name ~4% | | left column x 5%, y 8-92% | white on black over line art; labels grey ~55% | label:name 1:3.3 |

Typefaces: 3. Ratio ~50:1 (cropped names vs labels). Distinct: wide-tracked gothic story lines; huge cropped kanji as a graphic; credit block on the left margin.

## 3. Genre synthesis

### Horror (n=10)

- Title: custom or drawn lettering 8/10; set in a face that looks off-the-shelf 2/10 (MISTWATER condensed sans; HOTEL MIROIR Roman caps). Classes: distressed/grunge/drip 5 (yd6, Lny, TGG, nKu, zDV), script/bubble logo 2 (Indigo, fAX), decorative pointed display 1 (o2r), Roman inscriptional 1, condensed sans 1. Blackletter 0. Pixel 1 (fAX end card).
- Title size (cap or kanji height, % of frame): 5.7, 14, 14, 16, 18, 18, 20, 24, 27, 35; median 18%, range 6-35%. Lockup block 25-50% h in 7/10.
- Case: ALL CAPS in 9/10 Latin titles; mixed only in the script logo and "Coming soon". Tracking: titles tight (0 to -0.02 em) in 7/10, wide (0.08 or more) in 2 (MISTWATER, HOTEL MIROIR); small lines wide, 0.08-0.35 em in 7/9.
- Secondary lines (date / COMING / WISHLIST / CHAPTER): cap 2.0-8.8%, median 4.4%; legal 1.0-1.7%. Ratio largest:smallest designed text 5-30:1, median about 10:1.
- Cards on black: 3/10 have a series of cards or a tagline on black (yd6 three, Ikx one subtitle, nKu one); style: single line, centre, caps, red or grey-white, cap 2-5%, dim (40-60% luminance) except titles. Overlay text over picture: 7naiE (座布団 boxes) only; most horror teasers have no overlay text.
- Colour: white or off-white on 6 titles; dark red on 3 (yd6, TGG splatter, red accents in 7nai art); gold 1; teal gradient 1; lavender 1. Treatments: distress/texture 6, glow 4, outline or shadow 2, gradient 3, box 1, bevel 1, chromatic/scan 1.
- Placement: centre 6/10, left 3 (TGG, o2r, nKu), right 1 (zDV). All titles inside the 5% margin except zDV's CTA (2% from right) and fAX's last line (at the edge). Bottom lines (copyright, URL, "Coming soon") at 84-94% h.
- Typefaces per teaser: 1 (yd6, hmil), 2 (fAX, Lny, zDV), 3 (Ikx, TGG, o2r), 4 (nKu), 5 (7nai); median 2.5.
- Clichés: blood-red or dripping/distressed type 5/10 (drip or splatter proper 3: yd6, TGG, Lny); glow 4/10; pixel/CRT card 1/10. The clean options (light condensed, Roman caps) appear in 2/10.
- Japanese horror (n=3): 蚕代 uses a drawn brush kanji title (~27% kanji height) with a tracked Latin serif reading; Nightmare uses a Latin display title with katakana reading and a bold gothic date; HOTEL MIROIR sets all running text in mincho. Mincho 1/3 (the narrative one), gothic 2/3 and for all dates and prices. Vertical text 0/3 (0/15 overall). Punctuation: 。 at the end of full sentences in HOTEL MIROIR (3/3 lines), 『』 and ―― in plates, none in the Nightmare lockup. Reading line: a small katakana reading under or above the title in 2/3 (ratio to title cap 0.14-0.55). 和欧: Latin title plus ja reading is the norm; in the date line digits are larger than 年月日 (80%).

### Sci-fi (n=5)

- Title: custom logo or drawn lettering 5/5. Classes: heavy industrial/stencil 3 (BELTFED plate, BAY12 stencil, GBO3 cracked extended), extended black grotesque 1 (XARX), hand-drawn doodle 1 (Heartreasure). No thin clean sci-fi sans title in this sample.
- Title size (cap): 10, 12, 14, 17, 31%; median 14%.
- Case: ALL CAPS 4/5 (mixed only in Heartreasure). Titles tight; taglines in monospace or extended sans at 0.15-0.5 em (4/5).
- Secondary text: monospace 2/5 (BELTFED, BAY12; bold CTA at 2.8-5.6%); humanist sans 1 (GBO3); gothic 2 (ja). CTA/tagline cap 2.0-5.6%, median 3.7%; legal/credits 0.8-1.7%. Ratio 8-15:1, 50:1 for the XARX credits.
- Overlay text on picture: BELTFED captions (bold monospace caps 6% in a full-width dark band with orange hairlines, y 70-85%), GBO3 red-fill white-outline lines (~3%), Heartreasure pill. Cards on black: XARX story lines (ja gothic 6%), GBO3 "A New Battle Operation Series" (~2%).
- Colour: white or cool white for 3/5 titles; BELTFED rust orange, BAY12 light grey, Heartreasure black outline on white. Accents: orange hairlines, yellow CTA, green slash, cyan-white tint. Treatments: chromatic offset 1 (BELTFED), grain/chip texture 3, bevel 1, black band/box 3, glow 1.
- Placement: centre 4/5, left 1 (GBO3); left-aligned at x=5% in GBO3 and the XARX credits; CTA bands full width.
- Typefaces per teaser: 2 (BELTFED), 3 (BAY12, GBO3, XARX), 4 (Heartreasure); median 3.
- Clichés: grit and stencil on titles 3/5, monospace/terminal CTA 2/5, chroma split 1/5. Japanese: XARX story lines wide-tracked gothic (0.2 em), 2 lines, credits label:name 1:3 with grey labels; Heartreasure uses a rounded gothic and a hand-drawn title. Mincho 0/2.

## 4. Proposed typography profiles (plain values, % of 1080 frame height)

### Horror profile
- Display (title): Latin ALL CAPS in a condensed or distressed display, or a Roman inscriptional face for the refined variant; ja brush/distressed kanji or mincho. Weight bold-black for distressed, light for condensed. Tracking 0 to -0.02 em for bold distressed; about 0.1 em for condensed light or Roman (measured).
- Title size: cap height 14-24% (median 18%); lockup 25-45%; single-line title up to 35%.
- Card/body: Latin sans or serif caps, regular to bold, tracking 0.1-0.25 em, cap 3-5% (median ~4.4%). ja mincho or gothic regular, kana 4-5% for cards, 2.4-4% in captions, tracking 0.2-0.3 em (HOTEL MIROIR), leading 1.4-1.5.
- Colour: off-white to white for titles; dark red #8a0f14 for the red variant (3/10); gold #d9c58a for the gothic-romantic variant. Treatment: low-key; grain/distress or soft glow; no box except a 座布団 for narrated captions (near-black box, gold hairline). Cards on black 2-4% cap at 40-60% luminance.
- Placement: title centre or left third; secondary lines centred; bottom lines at y <= 92%, inside the 5% margin. Max 2-3 typefaces (median 2.5). Largest:smallest about 5-10:1.
- Clichés: dripping/splatter type 3/10, heavy glow 4/10, pixel/CRT card 1/10. Present and measured, but not necessary: the clean light-condensed and Roman options also appear in 2/10.
- (inferred) For a tool without custom lettering: set the title in a condensed or Roman face at about 18-20% cap with 0.08-0.12 em tracking and a white-to-grey or gold gradient; add red only as a single small card line.

### Sci-fi profile
- Display: Latin ALL CAPS, heavy industrial or extended grotesque (stencil/plate/heavy), tracking 0 to +0.02 em; ja gothic bold (no mincho was measured).
- Title size: cap 10-17% (median 14%); lockup 17-30%.
- Sub-lines/CTA/tagline: monospace or extended sans, ALL CAPS, tracking 0.05 em for CTA and 0.2-0.5 em for tagline; cap 2-6% (median 3.7%). ja story lines gothic medium, kana about 6%, tracking 0.2 em, leading 1.7, 6-9 chars per line, 2 lines.
- Colour: white or cool white (#dceae6); one accent from orange (#e08a2c), yellow (#f5e12a) or cyan. Treatment: chipped/grain texture on titles, hairline rules (orange), full-width black band (16% h) behind captions and CTA on live picture; chromatic offset only on CTA (1/5).
- Placement: title upper centre (y 20-50%); CTA band centre y 66-74%; credits and left-aligned blocks start at x=5%; captions lower third y 75-85%.
- Typefaces: 2-3 (median 3). Ratio 8-15:1.
- Clichés: grit and stencil on titles 3/5, monospace/terminal CTA 2/5, chroma split 1/5.
- (inferred) A light, clean futurist sans (thin extended) was not seen in this sample; do not assume it from here.
