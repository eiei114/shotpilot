# Typography measured in RPG and STRATEGY/SIMULATION teasers

Third-party works; no frames are stored in the repo. Sizes are % of frame height (1080-line equivalent = value x 10.8 px).

## 1. Method

- 20 videos (10 RPG, 10 strategy/simulation; two of the strategy videos are two uploads of the same game, Winning Post 10 2026, so 9 strategy games). 8 videos are Japanese-language (ja): DQ I&II, MH Iceborne, Galeid, MH Wilds x FF14 (RPG); Winning Post x2, Duel Links, Lonely dev sim (strategy).
- Text moments came from `research_notes/モーションデザイン/corpus_cozy_rpg.md` and `corpus_action_strategy.md` where they existed (P6, DQ, Lair, Warlock, Miniopolis, Neighborhoods, Winning Post, Frostpunk 2). For the other 12 a 0.5 or 1 fps contact sheet (8 columns, 240 px) found the text, then `frame.sh` at 1280 px (1600 for small Japanese) gave one full frame per element, 2-6 frames per teaser.
- Sizes were read in pixels off 1280x720 frames: cap height for Latin, kanji/kana height for Japanese, divided by 720. Tracking is estimated from glyph pitch vs glyph width (em, +-0.05). Reading error is about +-1 point for small text and +-3 for large. The earlier motion pass eyeballed some sizes smaller (e.g. Frostpunk "NEW EXPANSION" 4%, WP date 4%); pixel readings here are 12% and 11%, and are the ones to use.
- Letterboxed sources: Black Myth Zhong Kui (2.37:1 picture; % is of the picture height) and the first 10 s of Unawake.
- "looks like X" is a visual match only, with confidence. In-game UI and HUD are ignored except to note where they are the only text. Legal lines are noted by size only.
- Limits: partner logos and platform rows were not measured individually. Colours are by eye. Per-teaser typeface counts are of designed text only (logos counted as one family each).

## 2. Per teaser

Abbreviations: ja = Japanese, lat = Latin, cap = capital height, k = kanji/kana height, x = tracking in em, band = dark band behind text (座布団).

### RPG

#### R1 Persona 6 (yt-9lwef2jan-Q, 85 s) - https://www.youtube.com/watch?v=9lwef2jan-Q
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title logo "P6" | lat | custom lettering: condensed slab "P6" inside a rough drawn ring (woodtype/stencil slab) | black, caps, tight | ring 40% (end card) to 65% (title hit) | centre-upper, inside margin | neon green with outer glow, ragged edges, on black; end card white, no glow |
| sub-line "PERSONA 6" | lat | extended slab / woodtype caps, same family as logo | black, caps, very wide x ~0.35 | cap 3.5%, 1 line, width 25% | centre, under the ring | green glow with flame wisps; end card white |
| CTA "Wishlist Now" | lat | humanist sans (looks like Segoe UI Semibold, medium) | semibold, title case, normal | cap 4.2% | centre | white on dark green key art |
| platform row | lat | Xbox wordmarks | caps, normal | cap 2.8% | centre | white |
| legal | lat | sans | regular | ~1% | bottom, within 3% of edge | white 60% |
Distinctive: the title is a drawn emblem; the only designed words are a wide-tracked sub-line and a mixed-case CTA. 3 typefaces. Largest : smallest (excl. legal) ~15:1.

#### R2 Dragon Quest I&II HD-2D (yt-3idSGSoMMn8, 80 s, ja) - https://www.youtube.com/watch?v=3idSGSoMMn8
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| lead-in "時を超え" | ja | mincho, light-regular (Shuei-like, medium) | regular, very wide x ~0.55 | k 5.5%, 1 line, 4 chars | centre (y 50%), over key art | off-white, faint soft shadow |
| tagline "新たに紡がれる勇者たちの物語" | ja | mincho | regular, wide x ~0.3 | k 5.5%, 1 line, 14 chars, width 78% | centre, across the emblem | white with soft glow, no band |
| title "DRAGON QUEST" + "I&II" | lat + ja | custom 3D logo (red bevel, gold edge, phoenix); tiny katakana sub-title | black | block 30%; "DRAGON QUEST" cap ~16%; katakana k ~1.5% | upper right of key art | red with gold bevel, metallic gradient, sparkle |
| date "2025年発売予定" | ja + digits | mincho, lining numerals | regular, normal | k 4.5%, 1 line | centre, below logo | pale grey-white |
| platform + legal | lat/ja | sans | - | icons 1.5-2%, legal ~1% | lower left / right edge | white |
Distinctive: the only ja teaser in the set that sets copy in mincho, always very wide-tracked and unboxed. 3 typefaces. ~10:1 (excl. legal).

#### R3 Lair of the Leviathan (yt-AZmYsFsGVMk, 80 s) - https://www.youtube.com/watch?v=AZmYsFsGVMk
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title logo | lat | custom lettering: uncial/blackletter-flavoured display with swash, dragon and sparkles ("of the" small) | mixed case, tight | block 35% x 33% of width; "L" cap ~8%; 2 lines | upper left of centre, on key art | cream/silver, dark outline, light sweep |
| studio/publisher marks | lat | custom marks | caps | 2-3% | lower left, lower centre, lower right | full colour |
Distinctive: no designed copy; title plus logos on key art. 1 typeface plus marks. ~12:1.

#### R4 Warlock: Dungeons & Dragons (yt-ayW7oCXggoE, 86 s) - https://www.youtube.com/watch?v=ayW7oCXggoE
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "WARLOCK" + emblem | lat | custom lettering: flared angular display caps with chisel cuts (fantasy display, not Trajan) | bold, caps, tight | cap 17%; emblem above ~13%; block 45% | centre, slightly low | bone beige, grain texture, on black with ruled ornament |
| lockup "DUNGEONS & DRAGONS" | lat | serif small caps, ampersand flourish (Trajan-like, low-medium) | regular, caps, x ~0.1 | cap 2.5% (7:1 to title) | centre, under title | beige |
| date cards "GAMEPLAY REVEAL / SUMMER 2026", "LAUNCH 2027" | lat | roman caps serif (Trajan-like, medium) | regular, caps, very wide x ~0.4 | cap ~3%, 1-2 lines, width 30%, leading ~1.8 | centre, on black ornament | silver-beige, no glow |
| end card URL, partner marks | lat | sans | - | ~1.5% | lower centre | beige/grey |
Distinctive: cards on black with the widest tracking of the RPG set. 3 typefaces. ~7:1.

#### R5 Monster Hunter World: Iceborne (yt-0OJRP0oVrmw, 112 s, ja) - https://www.youtube.com/watch?v=0OJRP0oVrmw
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| "CAPCOM Presents" | lat | transitional serif (Garamond-like, medium) | regular, mixed, normal | cap ~1.5% | centre on black / over picture | white |
| title "MONSTER HUNTER WORLD ICEBORNE" | lat | engraved Trajan-like caps with chiselled M/N (custom); WORLD small spaced caps; ICEBORNE flared | regular-bold, caps, x ~0.1 (WORLD ~0.5) | cap 7% (MONSTER HUNTER), ICEBORNE 6%, WORLD 2%; 3 lines | centre, over snow / on white end card | steel grey with highlight; ICEBORNE ice blue; no outline |
| "Autumn 2019" | lat | transitional serif | regular, mixed | cap 3.5%; "2019年 秋" 1.5% | centre on black | white |
| caption (92-100 s) | ja | gothic medium (Shin Go-like, medium) | medium, normal (x ~0.02) | k 4.2%, 2 lines, 12-22 chars, leading ~1.2 | lower centre 82-90%, centre aligned | white on a full-width dark gradient band (座布団); no 。; uses 、 and "：" |
| sub-heads "ディスク版 / デジタル版", "超大型拡張DLC" | ja | gothic medium | medium | k 3.2% | above each box | white, soft shadow |
| lead "超大型拡張コンテンツ発売決定" | ja | gothic medium | medium, x ~0.05 | k 4.5% | centre-left, 50% height | white, soft shadow |
| end card (white) | ja + lat | katakana reading k 2.8%; "2019年秋 発売予定" k 5% (digits = kanji height); notice k 3%; URL cap 2.2%; SNS 1.5%; legal 1% | gothic medium, normal | 5-6 stacked rows, centre | lower two thirds, bottom row at 90% | dark grey on white |
Distinctive: the ja end card is a stack of 5-6 rows from 5% down to 1%. 4 typefaces. ~5:1 (excl. legal).

#### R6 Galeid. prologue (yt-tVuGDJOX2GM, 67 s, ja) - https://www.youtube.com/watch?v=tVuGDJOX2GM
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "Galeid." | lat | pixel display (outlined bitmap type) | bold, mixed, normal | cap 14%; "prologue" 5% (3:1) | upper right, on key art | white, black stencil outline, glitch edge |
| date "2024 12.24" | lat digits | pixel | bold, normal | cap 5% | right of centre, below | white, outline |
| "steam & ふりーむ で配信予定!" | ja + lat | bitmap pixel face | regular | k 3.5%, 2 lines | right of centre | white |
| callouts "敵をやっつけよう！", "[属性浸り]を使いこなし、" | ja | pixel gothic (bitmap, bold) | bold, x ~0.02 | k 9-10%, 1 line, 8-16 chars | left-lower and right-lower diagonally; reaches 4% from left and 98% on the right (outside the 5% margin) | white fill, black outer stroke ~0.6%, over dark screen, no band |
| disclaimer | ja | gothic | regular | 2% | lower right, 2% from edges | white |
Distinctive: huge outlined pixel-gothic callouts, edge to edge, full-width "！" and 、 but no 。. 2 typefaces. ~5:1.

#### R7 Monster Hunter Wilds x FINAL FANTASY XIV (yt-8ZjctQ4exrk, 65 s, ja) - https://www.youtube.com/watch?v=8ZjctQ4exrk
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| collab lockup (two logos) | lat | MH Wilds: Trajan-like caps with gold WILDS; FF XIV: condensed Trajan-like grey caps | regular, caps, x ~0.1 | cap 4-5% each | upper left / upper right | metal, gradient |
| "SPECIAL COLLABORATION" | lat | roman caps with bevel (looks like Trajan, medium-high) | bold, caps, x ~0.05 | cap 7.5%, width 80% | lower third, centred | gold gradient, bevel, soft shadow, over sky |
| "LATE SEPTEMBER" | lat | flared roman caps (Trajan-like) | regular, caps, x ~0.05 | cap 6% | lower left of centre, 60-70% | cyan with glow |
| card head "タイトルアップデート第3弾 / 追加モンスター" | ja | gothic medium-bold | bold, x ~0.03 | k 4.5% / 4%, 2 lines | centre, upper third of card | warm gold with glow, on dark ornamental card |
| monster name "オメガ・プラネテス" | ja (katakana) | rounded-bold gothic (maru-leaning, medium) | black, x ~0.03 | k 11%, 1 line, 9 chars with ・ | centre, y 55% | gold gradient, bevel, glow |
| "好評発売中!" | ja | gothic | bold | k 2.5% | right of centre, 70% | gold/white |
| roadmap table | ja | gothic | regular | 1.5-2% | full frame | table text on dark |
Distinctive: ja headline blocks in gold gradient with glow; EN in Trajan-like gold. 4 typefaces. ~6:1.

#### R8 Black Myth: Zhong Kui (yt-4r1hl2nvAEY, 116 s) - https://www.youtube.com/watch?v=4r1hl2nvAEY
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| studio mark "GAME SCIENCE" | lat | sans caps | regular, x ~0.2 | ~1.2% | centre on black | grey-white |
| title "ZHONG KUI" | lat | custom brush lettering, angular strokes (drawn, not a font) | medium, caps, normal | cap 13% of picture height; 2 lines (ZHONG / KUI) | centre | deep red ink, rough edges, over a large faded brush character and red seal, on black |
| "BLACK MYTH" | lat | sans caps | regular, very wide x ~1.0 | cap 1.3% | centre, between title lines | dark red |
| URL "gamesci.cn / zhongkui" | lat | sans | regular | ~1% | centre, 60% | grey + red |
Distinctive: only ink lettering plus a hairline sub-line. 2 typefaces. 10:1.

#### R9 Unawake (yt-Rk22tn8yDLc, 77 s) - https://www.youtube.com/watch?v=Rk22tn8yDLc
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "UNAWAKE" | lat | custom engraved caps with cracked/scale texture, A without crossbar (Trajan-like skeleton, medium) | regular-bold, caps, x ~0.15 | cap 9%, width 55% | upper centre, 22% | bronze/stone, texture, no glow |
| CTA "WISHLIST NOW" | lat | flared humanist caps (Trajan-like) | medium, caps, x ~0.15 | cap 4% | centre, 47% | warm gold, soft glow |
| logo row | lat | studio, engine, Steam | - | cap 2% | lower centre, 82-86% | white |
| legal | lat | sans | - | ~1.2%, 2 lines | bottom 94-96% | grey |
Distinctive: textured title plus a gold CTA. 3 typefaces. ~8:1.

#### R10 Full Circle (yt-VR0Fo8DldfI, 66 s) - https://www.youtube.com/watch?v=VR0Fo8DldfI
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "FULL CIRCLE" in an enso ring | lat | bold geometric sans (looks like Montserrat ExtraBold, medium); ring custom brush | bold, caps, x ~0.03 | cap 7.5%, width 37%; ring 40% | upper centre | white on black; first fades up over a sunset picture |
| "SEE YOU AT GAMESCOM" | lat | condensed grotesque (looks like Bebas Neue / DIN Condensed, medium) | bold, caps, x ~0.03 | cap 4% | centre, 70% | white |
| marks | lat | studio/publisher | - | 2-4% | lower row | white |
Distinctive: a plain centred stack on black. 2 typefaces. ~4:1.

### STRATEGY / SIMULATION

#### S1 Miniopolis (yt-fGgsNvxaA04, 90 s) - https://www.youtube.com/watch?v=fGgsNvxaA04
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "miniopolis" | lat | rounded geometric sans as 3D letters (looks like Quicksand / Gotham Rounded, medium); diegetic | medium, lowercase, tight | x-height ~20%, ascender 28%; bleeds past both frame sides | centre-lower, 33-60% | matte grey 3D, rack focus |
| CTA "WISHLIST NOW" | lat | humanist/neo-grotesque sans bold (looks like Open Sans Bold, medium) | bold, caps, x ~0.03 | cap 8% | lower centre, 75-83% | white, over dark water, no band |
| "Demo Coming Soon" | lat | same sans, semibold | semibold, title case | cap 3% | lower right, 93% | grey 60% |
| Steam mark | lat | wordmark | - | cap 3% | lower left, 93% | grey |
Distinctive: three elements only; the title is scene geometry. 2 typefaces. ~9:1.

#### S2 Neighborhoods (yt-UpH6HB-Km_0, 60 s) - https://www.youtube.com/watch?v=UpH6HB-Km_0
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "NEIGHBORHOODS" | lat | custom clay lettering: hand-cut rough caps, teal drop-cap N, house tiles beneath | caps, tight | cap 18%, width 85% | centre, upper | red and teal clay, thick white sticker outline (~1.5%), shadow |
| "Wishlist Now!" | lat | hand-drawn condensed sans (marker-like, low) | bold, title case, tight | cap 8% | centre, 62% | white, dark outline ~0.5% |
| URL "NeighborhoodsGame.com" | lat | same hand-drawn | bold, mixed | cap 5.5% | centre, 75% | white, outline |
| publisher mark | lat | logo | - | 4% | bottom centre | orange |
Distinctive: all type shares one outlined hand-drawn feel; no band. 2 typefaces. ~3:1.

#### S3 Winning Post 10 2026 (yt--uDHQlmGstM 82 s and yt-LeS4jr4cWWM 85 s, both ja) - https://www.youtube.com/watch?v=-uDHQlmGstM and https://www.youtube.com/watch?v=LeS4jr4cWWM
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| archival date "1993.12.26" | digits | neo-grotesque (looks like Helvetica Neue Medium, medium) | regular, x ~0.03 | digit cap 11%, width 40% | centre | grey-white, soft bloom, on black |
| card sentences "それは、奇跡の復活。" (4 of them) | ja | heavy italic gothic with worn texture (display ultra-black gothic, slant ~12 deg) | black, x ~ -0.03 (glyphs nearly touching) | k 15%, 1 line, 8-10 chars, width 80% | centre, 50% | black on white, grunge, motion blur on entry; 、 and 。 |
| card "その蹄跡を、歴史に刻め。" | ja | same | black, tight | k 21%, 2 lines, leading ~1.3; ruby "ていせき" 4.5% (0.28x body) | centre-left, stepped | white on black, grunge |
| race tag / horse name | ja | italic heavy gothic | black | k 3% / 8% | top left / lower right | white, dark shadow |
| title | lat + digits | custom logo: black slab italic "Winning Post" over gold arc with horse head; "2026" red/white bevel | - | cap 11%; "2026" 12%; block 55% | centre, upper | black, gold, red gradient, white outline |
| date "2026.3.26発売" | ja + digits | bold gothic; digits larger than kanji | bold, normal | "3.26" 10%, "2026." 7%, "発売" 5% | lower left | white, soft shadow, on green gradient |
| price / platform / notice | ja | gothic | regular | 1.5-2% | lower area | white |
| legal ribbon | ja | gothic | - | 1.3% | bottom 4 rows to 98% | white on dark / on a yellow bar |
| bonus slides | ja | gothic with yellow bar heads | bold/regular | 2-3.5% | full frame | dark on light |
Distinctive: the most text-forward ja teaser: white cards with black heavy italic type, then an end card with digits larger than kanji, then info slides. 4 typefaces. ~8:1 (excl. legal).

#### S4 Frostpunk 2: The Surge (yt-QW_7oiMLM6o, 61 s) - https://www.youtube.com/watch?v=QW_7oiMLM6o
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| "11 BIT STUDIOS presents" | lat | sans caps | regular, x ~0.25 | 1.5% | centre | grey-white |
| "NEW EXPANSION", "COMING", "DECEMBER 14" | lat | distressed condensed grotesque (Impact/Bebas-like, medium) | bold, caps, x ~0.05 | cap 12%, width 61% | centre, 50% | steel grey-blue, cracked texture, on smoke |
| title "FROSTPUNK 2" | lat | industrial heavy grotesque, wide caps with metal bevel (custom-like) | bold, caps, x ~0.02 | cap 10.7% | left-centre, 25-35% | metal/stone, shadow |
| "THE SURGE" | lat | thin roman caps (Trajan-like, low) | regular, caps, x ~0.1 | cap 5% (2:1 to title) | under title, left aligned | pale blue |
| "COMING DEC 14TH" | lat | serif caps with superscript TH (Trajan-like, medium) | semibold, caps, x ~0.1 | cap 3% | left, 58%, inside an outlined pill | white |
| platform rows, legal | lat | sans | - | 2.3% / 1% | left, 74-80% / bottom | white |
Distinctive: text sits on the picture's dark side, left-aligned. 4 typefaces. ~12:1 (excl. legal).

#### S5 Lonely indie developer's life (yt-B-4GA16mPYs, 25 s, ja) - https://www.youtube.com/watch?v=B-4GA16mPYs
No designed title, card or end card exists; the only text is in-game UI (a JRPG message window: gothic regular k 3.5%, a second line in dark red k 5%; 。 and …… used). Designed typefaces: 0. Excluded from the syntheses.

#### S6 Yu-Gi-Oh! Duel Links GO RUSH (yt-Tr_exHsPco8, 49 s, ja) - https://www.youtube.com/watch?v=Tr_exHsPco8
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| anniversary mark "25th" | lat | custom logo | - | 8% | centre, on black | red/white |
| series logos | ja + lat | custom logos | - | 10-25% | varied | full colour |
| captions "フュージョン召喚実装", "ゲーム画面をアップデート!", "新機能クロニクルカード登場!" | ja | heavy gothic with rounded-leaning shapes (maru-leaning, low-medium) | bold-black, x ~0.02 | k 10%, 1 line, 10-15 chars, width 75% | lower centre, 82-95% (touches the 5% bottom margin) | pale green-cream gradient, dark edge, drop shadow, over picture, no band; no 。; half-width "!" |
| date "2024.10.01" | digits | humanist sans (Gill Sans / Optima-like, low) | semibold, x ~0.03 | digit cap 22%, width 87% | centre | pale sage metallic gradient, on black |
| publisher mark | lat | KONAMI red wordmark | - | 3% | top right | red; end card white on red |
Distinctive: large low captions; the date is the second-biggest text in the set. 3 typefaces. ~4:1.

#### S7 Culdcept BEGINS (yt-GiZ1OLoUb4Q, 30 s) - https://www.youtube.com/watch?v=GiZ1OLoUb4Q
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| title "Culdcept BEGINS" | lat | calligraphic serif logo (custom, swash lowercase) with roman caps BEGINS | bold, mixed, tight | "C" cap 10%, BEGINS cap 5% | upper left (end card) / top left (start) | cream, shadow |
| feature lines "Summon Magic Spells & Creatures", "A Beloved Classic Reborn", "Master the Culdcept", "And Claim the Power of Gods" | lat | decorative fantasy display (swash serif, Cooper-adjacent, low) | black, mixed case, tight | cap 8%, 2 lines, leading ~1.1, width 70% | centre, 50%, over a radial light burst | gold-yellow, black outline ~0.5%, outer glow |
| "FIGHT" | lat | high-contrast roman caps (Trajan-like, medium) | bold, caps, normal | cap 11% | centre | white-to-cream gradient |
| "Release Date:" / "October 28, 2026" | lat | old-style serif / decorative gold | regular / bold | 3% / cap 5.5%, 2 lines | left, 62-80% | cream; gold with glow |
| Steam badge, legal | lat | marks | - | 6% / 1.5% (legal to 96%) | left / bottom | white on black pill |
Distinctive: each feature line is a gold fantasy card; the end card is a left-aligned stack. 3 typefaces. ~7:1.

#### S8 Trucker 2056 (yt-TfS472qZUL4, 76 s) - https://www.youtube.com/watch?v=TfS472qZUL4
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| "STEAM NEXT FEST / WINTER 1993 / Exclusive Teaser" | lat | bevelled pixel/blocky display | bold, caps, normal | cap 8% / 5% / 1.3% | upper centre, 32% | blue gradient, thick glow outline |
| title "TRUCKER 2056" | lat | same bevelled pixel display | bold, caps | cap 10%, width 55% | centre, 33% | blue gradient + glow outline |
| end copy ("Coming to Steam Early Access in 2026", URL) | lat | bitmap pixel body | bold, mixed | cap 3%, 4 lines with a blank line, leading ~1.7 | centre, 52-68% | white on black |
Distinctive: all text pixel; one family at three sizes. 1 typeface. ~8:1.

#### S9 Heretic's Fork (yt-jhK21JNrPyI, 29 s) - https://www.youtube.com/watch?v=jhK21JNrPyI
| role | script | face / class | weight, case, tracking | size, lines | position | colour, treatment |
|---|---|---|---|---|---|---|
| "STEAM NEXT FEST" | lat | transitional serif caps (looks like Times New Roman, high) | regular, caps, x ~0.05 | cap 5% | centre | white, glitch-sliced |
| "DEMO" | lat | transitional serif bold (looks like Times New Roman Bold, high) | bold, caps, tight | cap 28%, width 72% | centre, 42% | crimson, flat, glitch lines |
| "MUSIC OF" / "OCCAMS LASER" | lat | serif caps / heavy brush-italic display | regular / black, caps | cap 3% / 11% | centre | white / red |
| title "HERETIC'S FORK" | lat | light Didone-like roman caps with red strike-through lines and red initials | light, caps, x ~0.05 | cap 5.3%, 2 lines, left aligned | upper left, 25-40% | white + red, no glow |
| "wishlist now" | lat | roman lowercase, w in red | regular, x ~0.1 | x-height 3.5% | left, 63% | grey-white |
Distinctive: system-default serif at extreme sizes plus red; DEMO is 28% of height. 4 typefaces. ~9:1.

## 3. Syntheses

### 3.1 RPG (10 teasers)

- Title: custom lettering or drawn logo 9/10 (P6, DQ, Lair, Warlock, MH Iceborne, MH Wilds, Black Myth, Unawake, Full Circle); set in a font 1/10 (Galeid, pixel). Trajan-like (engraved/flared roman caps) in 4/10 (MH Iceborne, MH Wilds, Unawake, Warlock cards); blackletter/uncial 1/10 (Lair); brush/ink 2/10 (Black Myth, Full Circle ring). Cinzel and Cormorant could not be confirmed anywhere; nothing looked like Inter or Noto in display use.
- Latin secondary lines (date, tagline, CTA), 8 teasers with any: Trajan-like roman caps 4, humanist/neo-grotesque sans 2 (P6, Black Myth), condensed sans 1 (Full Circle), pixel 1 (Galeid).
- Japanese copy (4 ja): mincho 1 (DQ), gothic 3 (MH Iceborne, MH Wilds, Galeid pixel gothic); maru-leaning 1 line (MH Wilds monster name). Vertical text 0/4. 和欧: a Latin logo with a small katakana reading under it in MH (reading ~2.8% vs logo cap 7%); DQ has a tiny katakana sub-title only.
- Size by role (% height): title cap 7-17% (median 11%; block 25-65% with emblem); ja headline 11% (1 sample); ja caption/tagline 4-5.5% (median 4.5%); Latin tagline/date caps 3-7.5% (median 4); CTA cap 4-4.2%; platform/URL 1.5-3%; legal ~1%. Largest : smallest (excl. legal) 3:1 to 15:1, median ~7:1.
- Case and tracking: caps with wide tracking for secondary lines (x 0.1-0.4; 0.4 on Warlock date cards, 1.0 on the Black Myth sub-line); titles x 0 to 0.15. ja mincho tagline tracking is the widest ja type (x 0.3-0.55); ja gothic x 0.02-0.05.
- Colour and treatment: white/off-white for secondary text 8/10; gold or warm metal 3/10; accent (green, red, ice blue, cyan) on a title 4/10. Glow 4/10, gradient/bevel 3/10, texture 2/10, heavy outline 1/10 (Galeid), band 1/10 (MH caption). Cards on black 7/10.
- Placement: title centre 6/10, upper 3/10; CTA centre or lower third; ja captions lower centre at 82-90%, inside the 5% margin except Galeid. Alignment: centred 8/10, upper right 2/10 (Lair, Galeid), left 0.
- Typefaces per teaser (designed text): median 3, range 1-4 (2-3 for most). Largest : smallest median ~7:1.

### 3.2 Strategy / simulation (9 games, 10 videos; the Lonely dev sim has no designed type)

- Title (8 games with designed type): custom lettering/logo 7 (Miniopolis diegetic rounded sans, Neighborhoods clay, Winning Post, Duel Links, Culdcept, Frostpunk 2, Trucker 2056 bevelled pixel); set in a font with small edits 1 (Heretic's Fork serif with strikes). Rounded/hand-drawn 2, pixel 1, serif 2 (Culdcept subline, Heretic's Fork), metal grotesque 1.
- Cards, CTA, captions: bold sans caps CTA 3 (Miniopolis, Neighborhoods hand-drawn, WP end card), Trajan-like roman caps 3 (Frostpunk, Culdcept FIGHT, Heretic's), pixel 1, distressed grotesque 1 (Frostpunk NEW EXPANSION), fantasy display 1 (Culdcept).
- Japanese copy (3 ja games with designed type): gothic 3/3 (WP heavy italic gothic, WP info gothic, Duel Links heavy maru-leaning); mincho 0/3; vertical 0/3; ruby 1 (WP 0.28x). WP cards: heavy slanted gothic with grunge at k 15-21%, black on white or white on black, every card sentence ends with 。 and has 、 (5/5). Duel Links captions have no 。 and a half-width "!". Numerals: WP date digits larger than the kanji (3.26 cap 10% vs 発売 5%); Duel Links date digits 22%.
- Size by role (% height): title cap 5-25% (median 10.7%; block 20-55%); ja card sentence 15-21%; ja caption 10%; date digits 5-22% (median 11%); CTA cap 4-8% (median 6%); secondary lines 3-5%; platform/URL 3-5.5%; legal 1-1.5%. Largest : smallest (excl. legal) 3:1 to 12:1, median ~8:1. Extremes: Heretic's DEMO 28%, Duel Links date 22% (single word/number on black).
- Case and tracking: caps 7/9; tracking 0 to +0.1 on titles and +0.1 to +0.25 on secondary caps; lowercase/mixed for friendly styles (Miniopolis, Neighborhoods).
- Colour and treatment: white secondary 5/9; gradient/metal 4/9 (Frostpunk, Duel Links, Trucker, Culdcept); outline 3/9 (Neighborhoods, Culdcept, Trucker glow); band behind text 0/9; glow 3/9; texture 3/9 (WP grunge, Frostpunk distress, Culdcept). Cards on black 5/9, on white 1/9 (WP).
- Placement: title centre 5/9, left 3/9 (Frostpunk, Culdcept, Heretic's); CTA lower centre or centre; Duel Links captions reach the 5% bottom margin; WP card text 80% of width, centred.
- Typefaces per teaser: median 3, range 1-4. Largest : smallest median ~8:1.

### 3.3 Japanese vs Western teasers (all 8 ja videos)

- ja copy faces: gothic 6/8 (MH Iceborne, MH Wilds, Galeid pixel gothic, WP x2, Duel Links), mincho 1/8 (DQ), no designed ja 1/8 (Lonely dev, UI only). Mincho appears only on soft tagline lines of the classic RPG; gothic carries captions, cards, dates and notices.
- Vertical text 0/8; maru-leaning gothic 2/8 (Duel Links, MH Wilds monster name); brush/calligraphy ja 0/8; ruby 1/8.
- Punctuation: card sentences use 、 and 。 (WP 5/5); taglines and captions drop 。 (DQ 2/2, MH 2/2, Duel Links 3/3). "!" half-width in Duel Links and MH Wilds (好評発売中!), full-width in Galeid (！). Brackets: Galeid uses [ ] around a term; 「」 not seen.
- Latin vs ja pairing: the Latin title stays Latin with a small katakana reading (MH ~2.8% vs cap 7%; DQ ~1.5%); ja end cards are 5-6 stacked rows (5% date, 3% notice, 2% URL, 1.5% SNS, 1% legal). Digits inside ja lines equal the kanji in MH and are larger in WP.
- ja teasers carry more text elements (6-10, plus price rows, legal ribbons and info slides) than Western ones (3-6, ending in title + CTA + platform row).

## 4. Proposed typography profile per genre

Measured unless marked (inferred). Sizes are % of frame height.

### RPG / fantasy
- Display (latin): engraved/flared roman caps (Trajan-like) or custom lettering; weight regular to bold; ALL CAPS; tracking 0 to +0.1 em. Title cap 7-17% (default 11%); sub-line cap 2-3% with +0.35 to +0.5 em tracking.
- Display (ja): mincho for taglines (DQ): k 5.5%, tracking +0.3 to +0.55 em, white/off-white, soft glow, no band, 1 line, centre. Gothic bold for end-card rows and captions.
- Body/card (latin): roman caps, regular, +0.1 to +0.4 em, cap 3-4%; CTA cap 4-4.5% at +0.15 em or title case.
- Body/card (ja): gothic medium, k 4-5%, leading ~1.2-1.3, tracking 0 to +0.05 em, centred, lower centre at 82-90% height, dark gradient band optional, no 。 (2/2).
- Colour: off-white or bone beige default; one accent (gold, ice blue, cyan, or the game colour) on the title or one line; soft glow or bevel optional; outline only for busy pictures (Galeid).
- Placement: title centre, inside the 5% margin; cards on black, centre.
- Cliché seen: Trajan-like roman caps in 4/10 teasers (gold or steel grey), dark ornamental backgrounds 3/10, gold gradient with bevel 3/10. Cinzel itself not confirmed; the nearest look is Trajan/Cinzel-like (MH Iceborne, Unawake, medium).
- Typefaces 2-3 (inferred cap of 3 sizes); largest : smallest ~7:1.

### Strategy / simulation
- Display (latin): rounded or hand-drawn for cosy city/management (Miniopolis, Neighborhoods); heavy metal grotesque for grim (Frostpunk); serif caps for dark (Heretic's Fork); pixel for retro (Trucker). Weight bold; mixed case for friendly styles, otherwise caps; tracking 0 to +0.05 em. Title cap 5-25% (default 11%).
- Display (ja): heavy gothic, italic slant ~12 degrees, rough texture, tracking -0.03 to 0 em, k 15-21% for 8-10 char card sentences, black on white or white on black, centred, 1-2 lines, leading 1.3, ending 、 and 。; ruby 0.28x body. Captions on picture: heavy maru-leaning gothic k 10% with gradient and dark edge.
- Body/card (latin): bold sans caps CTA cap 4-8% at +0.03 em; secondary caps 3-5% at +0.1 to +0.25 em; platform/URL 3-5.5%.
- Body/card (ja): gothic regular for end-card rows 2-4%; date digits 5-10% larger than the kanji (10% / 5%); tracking 0 to +0.05 em.
- Colour: white default; one gradient/metal title per teaser; outline for friendly styles (sticker ~1.5%, text outline ~0.5%); no band behind text (0/9).
- Placement: CTA lower centre at 75-83% height; corner notes at 93%; ja card sentences 80% of width, centred.
- Cliché seen: a bold sans "Wishlist Now" CTA at cap 4-8% (4/9); left-aligned stacks for end cards (3/9); one giant word or number at 22-28% on black (2/9).
- Typefaces 2-4, median 3; largest : smallest ~8:1.

### Notes for shotpilot (inferred)
- Negative tracking appeared only on heavy italic ja cards (-0.03 em); wide tracking (+0.3 em or more) only on small caps lines and mincho taglines.
- At 1080 lines: title cap 11% is about 119 px, card/caption kana 4.5% about 49 px, CTA cap 4.5% about 49 px, secondary caps 3% about 32 px, legal 1% about 11 px. The current shotpilot sizes (card 64, title 150, caption 54, English sub-line 22) sit inside measured ranges for title and caption; the 22 px sub-line (2%) matches Warlock/MH secondary rows.
