# Typography in ACTION teasers and Japanese-language game PVs (measured)

## 1. Method

- Corpus: 10 action teasers (7 Western, 3 Japanese: SHIKA-Q, SF6 Sagat, NARUTO mobile) and 12 Japanese-language PVs of mixed genre. Videos are third-party; no frames were copied into the repo.
- Pass 1: one overview strip per teaser (every 2 s, 240 px tiles, time-stamped) to find every designed text moment. Text times for the action set were reused from `research_notes/モーションデザイン/corpus_action_strategy.md`.
- Pass 2: full frames (960 or 1280 px wide) at the title, the longest text holds and the end card; for the Japanese PVs, 2x2 montages of 960 px frames. Sizes are read off the frame in pixels and divided by frame height (540 or 720). Latin = cap height, Japanese = kanji/kana body height. Accuracy is about +/-1 percentage point; letterboxed trailers are measured against the full 16:9 frame including bars, and the visible picture is smaller than that.
- Typeface class is judged by eye. "Looks like X" is marked with a confidence; anything I could not tell is "unclear". Tracking in em is estimated from letter gaps. In-game HUD and legal lines are ignored except for their size.
- Limits: 3-5 frames per teaser, so a callout that only appears between sampled frames is missed (Defilade has about 14 callouts; only 2 were measured up close). Text animation is out of scope here (see the motion note).
- Three of the "Japanese PVs" are not game PVs: `yt-Ff944nqaJPg` (VTuber agency debut PV, from a game company), `yt-evU8B13S9T8` (Pokemon Card documentary teaser), `yt-_IvkZ1VlUe4` (a 41 s still of a game's title screen; no ad cuts). They are kept for Japanese type habits but are excluded from the "game PV" counts where marked.

## 2. Per teaser

Abbreviations: JP = Japanese, EN = Latin. "%H" = share of frame height.

### 2.1 Action

#### yt-luWunttkCgY, Dear Passengers (36 s). https://www.youtube.com/watch?v=luWunttkCgY
| role | script | face / weight / case / tracking | size | position / colour / treatment |
|---|---|---|---|---|
| title logo, custom lettering | EN | chunky rounded display with bevel, black weight, ALL CAPS, tight (about -0.02 em), slight upward-curved baseline | DEAR cap 18 %H, PASSENGERS cap 13 %H (block 42 %H), 2 lines | upper centre, off-white over yellow, dark purple thick outline (about 1.5 %H) and drop shadow, over picture |
Distinctive: the only designed text in 36 s; logo is huge and a sticker, no other type. 1 typeface.

#### yt-0lEO0u1_ooY, Moylo (33 s). https://www.youtube.com/watch?v=0lEO0u1_ooY
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| title logo, custom lettering | EN | heavy rounded slab-ish cartoon caps, black weight, ALL CAPS, tight | cap 16 %H, 1 line | centre-left, red, black outline about 0.8 %H plus offset shadow, over key art |
| CTA "Wishlist On" | EN | bold humanist/rounded sans (looks like a Fredoka/Baloo-type bold, low confidence), mixed case, normal | cap 5.5 %H | below logo, black, no treatment |
| platform logo (Steam wordmark) | EN | logo | 4 %H | below CTA, black |
Distinctive: cartoon ink-black type that matches the line art. 2 typefaces (logo + CTA sans).

#### yt-h8AcT7N-OoU, Defilade (76 s). https://www.youtube.com/watch?v=h8AcT7N-OoU
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| feature callout (e.g. "GRAD ROCKETS & ...") | EN | extended heavy grotesque, bold/black, ALL CAPS, slightly open (about +0.05 em) (looks like Gotham Black / Druk Wide class, low confidence) | cap 5 %H, 1-3 lines, up to 38 chars | lower left (x about 7 %), y about 69 %H, left-aligned, white, no outline/shadow, over picture; red 1 px vertical tick at left |
| callout label above | EN | mono-ish small caps, wide (about +0.2 em), grey | cap 1.4 %H | above callout, 60 % white |
| title "DEFILADE" | EN | extended bold grotesque (geometric-ish; looks like Montserrat Bold / Gotham Bold, low), ALL CAPS, very wide tracking about +0.5 em | cap 13 %H, 1 line, 78 % of frame width | upper centre (y 25-38 %), white with slight top-to-bottom grey gradient, on near-black, corner brackets in the frame corners |
| CTA "WISHLIST NOW ON STEAM" | EN | same wide caps | cap about 3 %H | lower right, white |
Distinctive: one wide-grotesque voice for everything, including a tactical-readout look. 1 family.

#### yt-olOTdoW6XsQ, Tomb Raider: Catalyst (91 s). https://www.youtube.com/watch?v=olOTdoW6XsQ
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| title logo, custom lettering | EN | flared/engraved serif caps, bold, ALL CAPS, tight; "RAIDER" crack texture | TOMB / RAIDER cap 14 %H each (block 45 %H), 2 lines + small line | centre, gold gradient with cracked metal texture, over picture |
| subtitle "CATALYST" | EN | thin serif/sans caps, very wide tracking about +0.4 em | cap 2.2 %H | under logo with ornament rule, gold |
| date "2027" | EN | semibold extended sans, lining figures | figure height 8 %H | centre (y about 45 %H), white to grey vertical gradient, on black |
| platform row + legal | EN | logos 2 %H; legal paragraph 1 %H (3 lines) | bottom centre |
Distinctive: logo plus a year and nothing else; legal small print is 1 %H. 3 typefaces (logo, tracked caps, date sans).

#### yt-zH6ymTkaOso, SHIKA-Q (40 s, Japanese). https://www.youtube.com/watch?v=zH6ymTkaOso
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| tagline "I'M SPECIAL." | EN | condensed bold caps, ALL CAPS, normal | cap 3 %H | lower centre, white, on black |
| title logo, custom lettering | EN | spiky italic sticker logo with hexagon badge | block about 40 %H | centre, magenta with cyan outline, sparks |
| "PUZZLE GAME" | EN | heavy italic extended-condensed display, ALL CAPS, tight | cap 18 %H, 2 lines, leading about 1.0 | centre, cream (#e8e4c8), navy outline about 1 %H |
| lead-in line "戦略×超高速×対戦" | JP | extra-bold italic gothic, yellow | glyph 9 %H, 11 chars | above PUZZLE, yellow gradient with dark outline; "×" as separator |
| callouts (繋げて破壊! / 判断力が試される一瞬の攻防 / 世界相手にランクマッチ / ハイスコアランキング / フルボーカル30曲以上収録) | JP | bold italic gothic, black weight, slanted about 10 degrees | glyph 13 %H (single line), 2-line ones about 11-12 %H; 5-9 chars per line | centre to upper centre, white-yellow gradient with teal-black outline about 1 %H, over gameplay |
| end card info | JP | light mincho-like (unclear: could be Yu Mincho Light), 2 lines | glyph 5.5 %H, 14 and 16 chars | centre, cream, on black; full-width spaces as separators, no punctuation |
| platform logos | | | 5 %H | below |
Distinctive: callouts are the loudest JP type in the set (9-13 %H, italic, outlined) and are the only text over gameplay. 4 typefaces (logo, EN display italic, JP italic gothic, JP mincho for the information card).

#### yt-MKMYcioGj0I, Street Fighter 6, Sagat (98 s, Japanese market). https://www.youtube.com/watch?v=MKMYcioGj0I
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| date card "SAGAT IS COMING / AUGUST 5, 2025" | EN | wide geometric/neo-grotesque, medium, ALL CAPS, slightly open (about +0.03 em) (looks like Helvetica Neue Extended / Aeonik, low) | cap 5 %H, 2 lines | centre (y 30-45 %H), white with faint cyan outer glow, on black |
| sub-line "TO WORLD TOUR, BATTLE HUB..." | EN | same family, ALL CAPS | cap 1.6 %H (ratio to main 0.32) | below, white |
| feature badges (3 parallelograms) | | art | 6 %H | |
| end card logo "STREET FIGHTER 6" | EN | bold squared grotesque caps, white | cap 6.5 %H | left, white; "FIGHTERS EDITION" brush script, orange, 5 %H |
| "好評発売中" | JP | bold gothic | glyph 5 %H, 5 chars | under logo, white, wide tracking about +0.1 em |
| platform row, publisher mark, legal | | | 2-3 %H; legal 1 %H | |
| opening CERO rating, "CAPCOM" graffiti title | | logos | | |
Distinctive: no designed text for 80 s; EN date card in a localised Japanese upload; JP appears only as "好評発売中". 3 typefaces (extended sans, brush script, JP gothic).

#### yt-pfY-EoYpIRE, NARUTO mobile action teaser (79 s, Japanese). https://www.youtube.com/watch?v=pfY-EoYpIRE
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| catch copy 1 "火の意志を" / 2 "継ぐ者たちへ" | JP | mincho, bold-ish, one phrase per card | glyph 7 %H, 5 and 6 chars | centre, off-white with orange fire glow, wide tracking about +0.5 em (full-width gaps), over picture |
| title logo "NARUTO MOBILE 火影忍者" | EN+JP | custom comic-style logo | block about 35 %H | centre, red/white with outlines |
| kicker "FORGE NEW BONDS" | EN | classical serif caps (looks like Cinzel-class, low) | cap 2.5 %H, tracking about +0.3 em | above main line, white |
| main "COMING WORLDWIDE" | EN | ultra-condensed black grotesque (looks like Impact/Anton class), ALL CAPS, tight | cap 19 %H, 1 line, 90 % of width | centre; COMING white-grey metal gradient, WORLDWIDE orange-red with brush texture |
| JP sub "全世界配信決定" | JP | mincho regular | glyph 4.5 %H (ratio to EN 0.24), 7 chars | below, white, tracking about +0.1 em |
| end card "続報はジャンプフェスタ2027にて" | JP | mincho bold with gold gradient | glyph 8 %H, 17 chars, 1 line | upper centre, gold with glow |
| date | JP | mincho/gothic regular | 3.5 %H | below, white |
| social handle | EN | gothic | 5 %H | |
| legal | JP | | 1 %H, 4 lines | |
Distinctive: mincho for the solemn lines, condensed black for the announcement, tracked serif for the kicker: 4 typefaces. 2 JP catch phrases, one JP+EN announcement stack.

#### yt-RhjtJIHe_v0, DOOM (61 s). https://www.youtube.com/watch?v=RhjtJIHe_v0
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| tagline "FIGHT LIKE HELL" | EN | thin extended sans caps, tracking about +0.8 em | cap about 1.2 %H (unclear, very small), 1 line | centre, grey-white, faint, in the dark picture |
| title logo, custom lettering | EN | angular fractured caps | block about 33 %H | centre, white, distressed edges, on red-black |
| date "MAY 13, 2016" | EN | extended light sans, ALL CAPS, tracking about +0.2 em | cap 3.9 %H | lower centre (y 77 %H), off-white |
| rating (PEGI 18) | | | 8 %H, lower left | |
Distinctive: logo plus a box-art end card; the tagline is nearly invisible. 3 typefaces.

#### yt-Dd9tLg9Z5Rw, God of War: Laufey (71 s). https://www.youtube.com/watch?v=Dd9tLg9Z5Rw
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| series logo "GOD OF WAR" | EN | custom carved serif caps with small-cap "of" | cap 5.5 %H | upper centre, silver |
| title "LAUFEY" | EN | custom angular runic-ish serif caps, wide spacing about +0.15 em | cap 18.5 %H, 1 line | centre, violet with vein texture and diamond over the A |
| end card "PRE-ORDER NOW / 02.16.2027" | EN | flared Trajan-class caps, tracking about +0.1 em; date old-style-like lining numerals | cap 4.2 %H and 3.5 %H | left, y 42-55 %H; silver metallic gradient, thin rules with a star above/below |
| platform logo PS5 | | | 4 %H | |
| studio logos | | | 5 %H, lower right | |
| rating box | | | 6 %H, lower left, opening | |
Distinctive: tidy end card with rule lines and metal type; title card is a bespoke wordmark. 3 typefaces.

#### yt-DSFXYBy2jMk, UE5 game teaser (21 s). https://www.youtube.com/watch?v=DSFXYBy2jMk
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| opening "UNREAL ENGINE" logo | EN | logo | about 8 %H, centre | |
| date "2023" | EN | geometric bold sans (looks like Poppins/Gilroy Bold, medium) | figure height 7 %H | centre, white, on black |
| platform glyphs | | | 6 %H | lower centre, white |
| creator watermark | | | 1.5 %H, lower left | |
Distinctive: no title and no copy; just a year. 1 typeface.

### 2.2 Japanese-language PVs

#### yt-0OmH7ix1dEE, Mafia: The Old Country, JP trailer (59 s). https://www.youtube.com/watch?v=0OmH7ix1dEE
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| burned-in JP subtitles ("我々はー ..." etc.) | JP | gothic regular | about 3 %H (unclear) | lower centre in the picture, white, soft shadow, 1 line, 8-12 chars |
| notice (ゲーム内ムービー…) | JP | gothic regular | 2.2 %H | lower centre |
| title "MAFIA / THE OLD COUNTRY" | EN | high-contrast flared serif caps (custom logo), bold | MAFIA cap 13 %H; THE OLD COUNTRY 3.7 %H | centre, silver gradient |
| JP title "マフィア：オリジン〜裏切りの祖国" | JP | gothic medium, 2 lines, centred | glyph 5.5 %H (ratio to EN cap 0.42), 8+8 chars; line pitch 1.6 | under the logo, white; full-width colon and wave dash; straddles the letterbox edge |
| CTA "ウィッシュリストに追加しよう" + "mafiagame.com" | JP+EN | gothic medium / humanist sans | 5 %H and 3 %H | centre, white |
| date "2024年12月 詳細公開" | JP | gothic bold | 5 %H | centre, white, blur-in |
| socials, platform logo, legal | | | 2.5 %H, legal 1 %H | |
Distinctive: a Western EN logo with a plain gothic JP title tucked under it; the JP date and CTA are gothic medium at 5 %H. 3 typefaces.

#### yt-Ei5Po1xKWig, Attack on Titan (game, 2016) (57 s). https://www.youtube.com/watch?v=Ei5Po1xKWig
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| quote lines ("That day, the human race remembered ...") | EN | wide-set old-style serif, regular, mixed case, open (about +0.05 em) (looks like Cinzel-lower/Optima-ish, unclear) | x/cap about 7 %H, 2 lines with a hanging dash and indent | upper left, sepia (#b08e6a), on dark paper |
| JP sub-line | JP | gothic light | glyph 5 %H, 11 chars, 1 line | centre, under the EN; sepia |
| kinetic burn-in line ("I am going to destroy them") | EN | serif, gold, flame texture | cap about 8 %H | centre |
| title lockup "進撃の巨人 / attack on titan / Attack on Titan (Working Title)" | JP+EN | JP: brush-cut distressed gothic, white with a red slash (custom logo); EN: wedge serif | kanji 19 %H; EN 8 %H (ratio 0.42); "(Working Title)" 2.8 %H | left-centre column, centred, white |
| year "2016" | EN | bold serif | figure 6.5 %H on the card; 16 %H in the closing frame | centre, sepia on blood-spattered black |
| "Prepare to make humanity's last stand!" + JP "人類最後の抵抗の日は近い…" | EN/JP | serif bold / gothic light | 7 %H / 5 %H | centre, sepia |
| social handles | EN | humanist sans | 4 %H | |
| copyright | | | 1.5 %H, lower right | |
Distinctive: bilingual lines are stacked EN above JP at about 0.7:1 JP/EN; the JP never leads. The "…" and "——" are used in the JP. 4 typefaces.

#### yt--ThjI3cn83U, LUCK SEVEN (33 s, indie). https://www.youtube.com/watch?v=-ThjI3cn83U
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| title logo, custom lettering | EN | stencil-cut condensed black caps in a red square frame | block 62 %H, LUCK cap 24 %H | centre, dark red on black, chromatic aberration |
| credits "【 LUCK SEVEN 】 制作 「茸の里」 ..." | JP+EN | mincho (looks like a bold mincho) | title cap 7 %H; ruby ラック・セブン 3 %H; roles 3.5 %H; names 5 %H | centre column, white, 8 lines, tightly stacked; 「」 around the name, 【】 around the title |
| vertical line "この神話は繰り返す" | JP | mincho, vertical (縦書き) | glyph 4 %H, 9 chars | centre (x 50 %, y 28-72 %), white-grey (one frame in red) |
| CTA "続報をお待ちください" | JP | mincho | 4.5 %H | lower right, white (margin 2 %) |
Distinctive: all text is mincho; one vertical line; ruby on the title; red chromatic fringing. 2 typefaces (logo, mincho).

#### yt-Ff944nqaJPg, Uraroji Game Company VTuber PV (40 s; not a game PV). https://www.youtube.com/watch?v=Ff944nqaJPg
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| copy "新しい物語をここから紡ぐ" | JP | light mincho, tracking about +0.5 em | glyph 4 %H, 12 chars | centre, dark grey on white |
| date line "2026年8月19日・20日にVTuberデビュー" | JP | mincho with small numerals | 2.8 %H | centre, green (#7ac36a) |
| company logo "URAROJI GAME COMPANY" | EN | DIN-like bold grotesque caps, wide-ish | cap 5.5 %H | centre, black, with icon |
| credits (EN roles right-aligned, names left, mixed JP/EN) | | serif/sans mix | 1.5 %H | centre, white on dark |
Distinctive: white cards with ultra-light, widely tracked mincho; the only light-background card in the set. 3 typefaces.

#### yt-_IvkZ1VlUe4, Chachamopa no Mori title screen (41 s; a still, 518x360, 1.44:1). https://www.youtube.com/watch?v=_IvkZ1VlUe4
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| title チャチャモパの森, vertical | JP | hand-cut blocky katakana, uneven baseline (custom) | glyph 8 %H, block 78 %H, 8 chars | centre-top, cream, stacked vertical (縦書き) |
| menu "はじめから / つづきから / おぷしょん" | JP | hiragana, light, pixel-soft | 3 %H | lower row, cream |
Distinctive: a vertical hand-drawn title; no ad text at all. 2 typefaces.

#### yt-evU8B13S9T8, Pokemon Card x Iwai Shunji documentary teaser (32 s; not a game PV). https://www.youtube.com/watch?v=evU8B13S9T8
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| credit "エグゼクティブプロデューサー / 岩井俊二" | JP | gothic medium; role smaller | role 2 %H, name 3.5 %H (ratio 0.57) | centre, white, soft shadow, over aerial shot |
| statement "中国カードゲーム界に衝撃を与えた青年" | JP | gothic bold, tracking about +0.05 em | glyph 5 %H, 19 chars, 1 line | mid-height (y 54 %H), centre-right, white, drop shadow, no band |
| date "2024年秋、Nii.の軌跡、公開" | JP | light mincho, tracking about +0.15 em | 3.5 %H | lower centre (y 88 %H), white; 、 used as separator |
| tournament logo | | | 14 %H | |
| end credits | EN/JP | | 1.5 %H | black |
Distinctive: only text over picture with no band; the date line is mincho. 2 typefaces.

#### yt-JjXngOfw_YQ, Metaphor: ReFantazio launch trailer (81 s). https://www.youtube.com/watch?v=JjXngOfw_YQ
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| title logo (corner bug) + "好評発売中!" | EN+JP | custom logo (white on black) / bold gothic | logo 16 %H; JP 3.5 %H | upper right, white |
| catch copy "『ペルソナ5』クリエイターによる / 新たなファンタジーRPG" | JP+EN | gothic bold, 2 lines; "RPG" in the same family; second line 1.45x larger | 7 %H and 10 %H; 14 and 11 chars | centre, white with an iridescent gradient and soft shadow, over dark texture; 『』 around the title |
| subtitle band "幼馴染の王子を死の呪いから救うため / 主人公は、波乱に満ちた冒険の旅へ——" | JP | gothic bold | 5.2 %H, 2 lines of 18-19 chars, pitch 1.4 | lower centre, 80 % width, white on a pink brush-stroke band with teal flecks; 、 and "——"; no 。 |
| callout "戦闘は緊張感と爽快感あふれるアクション&コマンドバトル!" etc. | JP | same | 5.2 %H | same band |
| CTA "プロローグ体験版 / 無料配信中!" | JP | bold gothic with a mincho feel, silver gradient (unclear) | 15 %H and 13 %H | centre, silver with ring graphic; half-width "!" |
| ribbon "序盤まるごと遊べて引き継げる!" | JP | gothic bold | 6 %H | pink ribbon above CTA |
Distinctive: the same pink-band subtitle style is used for six captions; there is a size jump of 1.45x inside a catch copy. 3 typefaces.

#### yt-5GOtzYKXWVk, Devil Connection (70 s). https://www.youtube.com/watch?v=5GOtzYKXWVk
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| subtitles (そこには今にも悪魔を呼び寄せようと ...) | JP | maru gothic, light, white | 3.3 %H, 17 chars | upper-middle (y 35 %H), centred, soft shadow |
| title logo, custom lettering | JP+EN | bubble hiragana/katakana, 3D with navy outline, pink-to-teal gradient | 2 lines, block 53 %H | centre, on white |
| "Devil connection" | EN | rounded bold sans | 3.5 %H (ratio to JP line 0.13) | under logo, purple |
| callout "邪眼発動" | JP | brush calligraphy (筆), rough-edge, black weight | glyph 18 %H, 2+2 chars | centre on a skewed panel, white |
| end card "2025年 リリース予定！ / お楽しみに！" | JP | rounded bold gothic | 4 %H | below the cast art, purple; ！ full-width, space after 年 |
| credit labels (handwriting) | JP | handwriting | 1.5-2.5 %H | around cast |
| in-game dialogue | JP | gothic small | 2.5 %H | |
Distinctive: maru/handwritten tone; brush for the one hit moment; no EN except a small logo subline. 4 typefaces.

#### yt-rDHPFX5Xh6M, Meg and the Monster (61 s). https://www.youtube.com/watch?v=rDHPFX5Xh6M
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| narration subtitles ("魔界に迷いこんだ / 一人の少女、メグ。") | JP | gothic bold | 6.5 %H, 2 lines of 8-11 chars | upper centre (y 28 %H), white with dark shadow; ends with 。 |
| in-game "Game Over" | EN | pixel/rounded heavy | 14 %H | upper centre, grey on white |
| title メグとばけもの | JP | hand-drawn brush katakana (custom), white outline | 9 %H on the black card; 15 %H per line on the key art | lower right (card), top right (key art) |
| date "2023/03/02 発売" | JP | gothic bold | 5 %H | right-aligned, white; slashes in the date |
| platform logos | | | 4-5 %H | right |
Distinctive: narration shown as subtitles, ending in 。, the only JP PV here that does so; pixel game. 3 typefaces.

#### yt-miwP0svg1hg, Forza Horizon 6 (JP, 48 s). https://www.youtube.com/watch?v=miwP0svg1hg
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| callouts "日本が待っている / Horizon 最大級の都市を体験 / 日本の車文化に浸り / レーシング界の伝説になれ" | JP (+EN word) | gothic semibold; Latin "Horizon" in the same family at the same size | 7-8 %H, 1-2 lines, 8-13 chars | centre of a dark band between teal top and bottom bars; white; second line in magenta; drop shadow |
| corner logo | EN | custom logo | 7 %H | top left |
| CTA "2026 年 5 月 19 日発売 / 今すぐ予約注文" | JP | gothic bold | 3.5 %H and 5.5 %H | white on a magenta rounded box, with a rule between; half-width spaces around the digits |
| platform row | EN | | 2.5 %H | |
| notice "ゲーム内映像" | JP | | 1.5 %H | lower left |
Distinctive: a branded frame with teal/magenta bars; JP and EN share one family; no outline. 2 typefaces (logo + gothic).

#### yt--PDHlviyssc, Attack on Titan 2 (65 s). https://www.youtube.com/watch?v=-PDHlviyssc
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| EN line "THIS WORLD IS CRUEL." / "AND ALSO VERY BEAUTIFUL." | EN | Trajan-class serif caps with stone texture and shadow | cap 7.4 %H, 1 line | centre, grey-green, over cloud texture |
| JP sub "この世界は残酷だ…" | JP | gothic light | 4 %H | lower centre, white |
| title logo "進撃の巨人2" + "attack on titan" | JP+EN | custom brush-cut kanji with a red slash; red "2" with glow | kanji 22 %H; "2" 26 %H; EN 3 %H | left-centre, white/red |
| date "Early 2018" | EN | humanist sans (looks like DIN/Ropa), flanked by em rules | cap 10 %H | below logo, white |
| copyright | JP | | 1.5 %H | lower right |
| warning card (opening) | JP | gothic | 3 %H | centre |
Distinctive: EN leads, JP is the quiet gloss; date is a different sans from the serif body. 4 typefaces.

#### yt-hn_5gTAJ1cc, Girls Band Cry: First Riff (43 s). https://www.youtube.com/watch?v=hn_5gTAJ1cc
| role | script | face | size | position / colour / treatment |
|---|---|---|---|---|
| teaser card "オリジナルゲーム / 鋭意開発中" | JP | extra-black display gothic (looks like Dela Gothic-class, medium), tight | glyph 20 %H, 2 lines, 8 and 5 chars, leading about 1.1 | centre, tilted about -4 degrees, white with soft dark edge, on colour splashes |
| title logo "GIRLS BAND CRY / FIRST RIFF" | EN | custom heavy extended caps, angular; sub in the same font | cap 6.5 %H; sub 4.5 %H | left, white on red; JP "ガールズバンドクライ" 2.5 %H on a pick graphic |
| CTA "公式X&ティザーサイト" | JP | gothic bold | 5.5 %H | white on red |
| "OPEN!" | EN | heavy sans | 6 %H | red on a white box |
| search box "ガルリフ ゲーム 検索" | JP | gothic | 4.5 %H | in pill |
| producer logos | EN | DIN-like caps, tracking +0.15 em | cap 9 %H ("DONUTS GAMES") | centre, white, on black |
Distinctive: the loudest JP type in the PVs is a 20 %H extra-black display on colour splashes. 4 typefaces.

## 3. Synthesis

### 3.1 Action (10 teasers: 7 Western, 3 Japanese)

- Designed-text counts: Western teasers are sparse. Dear Passengers 1, Moylo 2, Tomb Raider 4, DOOM 3, God of War 4-5, DSFX 2; Defilade is the exception with 14+ callouts. Japanese action teasers carry 5-10 (SHIKA-Q 10, NARUTO 8, SF6 3 designed cards).
- Title: 9 of 10 use custom lettering / a drawn logo (only Defilade sets the title in a font; DSFX has none). Title cap or main line height: 13-19 %H, median about 15 %H; whole block 33-45 %H when it is a 2-3 line logo. Colour is metallic gold/silver, outlined cartoon or white; centre or upper centre in 7 of 9.
- Date / CTA: 3.5-8 %H, median 4.5 %H (Tomb Raider 8, DSFX 7, SF6 5, DOOM 3.9, God of War 3.5-4.2, SHIKA-Q 5.5). ALL CAPS, tracking +0.1 to +0.2 em in the three Western ones, white or metallic, centre or left, on black or box art. Platform glyph rows: 2-6 %H. Legal: 1-1.5 %H.
- Callouts (over gameplay): 5 %H (Defilade, ALL CAPS wide grotesque, lower left) against 9-13 %H (SHIKA-Q, JP italic with outline, centre). Western ALL CAPS grotesque is the default; the three JP action teasers add mincho or gothic JP lines with Latin.
- Case: ALL CAPS in 25 of 27 Latin elements; mixed case only for "Wishlist On", the "Prepare to make humanity's last stand!" style quote, and "Early 2018". Tracking: titles and dates are wide (+0.1 to +0.5 em); heavy display logos are tight or custom.
- Colour: white or off-white in 70 %; metallic gold/silver gradients 4; saturated accent 4 (red, yellow, magenta, violet). Outlines are used on cartoon/pop titles and on JP callouts (about 1 %H); glow on SF6, NARUTO. Bands/boxes only on SHIKA-Q end card (none), none of the Western action teasers.
- Typefaces per teaser: 1-4, median 3. Largest/smallest designed text ratio: roughly 6:1 to 15:1 (e.g. 19 %H main line vs 2.5 %H kicker; 13 %H title vs 3 %H CTA).
- Western vs Japanese: Western action puts almost no body copy over picture; Japanese action puts mincho/gothic calls at 7-13 %H with outlines and uses full-width spaces for gaps (SHIKA-Q, 2025年　秋　発売予定).
- Clichés seen: gold metal logo (Tomb Raider, SF6 edition line); wide-tracked ALL CAPS date on black (Tomb Raider, DOOM, SF6, DSFX: 4 of 10); classical serif caps kicker over a title (NARUTO, GoW, AoT2: 3); corner brackets (Defilade 1); icon platform row in the end card (8 of 10).

### 3.2 Japanese PV typography (12 PVs + 3 JP action; 15 total, 3 PVs are not game ads)

- Mincho vs gothic. Mincho: solemn or narrative lines (NARUTO 火の意志を / 続報は…, LUCK SEVEN credits and vertical line, Uraroji copy, Pokemon-doc date line, SHIKA-Q end card): 6 of 15, always light/regular weight and tracked +0.1 to +0.5 em. Gothic: everything that sells or informs (callouts, subtitles, CTA, date, platform): in 14 of 15 PVs, medium to black weight. Extra-black or brush display: GBC 20 %H, 邪眼発動 18 %H, SHIKA-Q italic 13 %H, Metaphor CTA 15 %H. Maru: Devil Connection subtitles and logo (1 of 15). Pixel/hand lettering: Meg title, Chachamopa title, Game Over.
- Vertical 縦書き: 2 of 15 (LUCK SEVEN 9-char line, Chachamopa title). Everything else is horizontal.
- Outlines and bands. Thick outline (about 1 %H) or heavy edge: SHIKA-Q, Devil Connection brush, GBC (soft). Band behind text (座布団): Metaphor pink brush band for 2-line captions (5.2 %H), Forza dark band plus teal bars (7-8 %H), Forza CTA box. Soft drop shadow only: Mafia, AoT, Meg, Pokemon doc, 5GO (about 7 of 15). No plate and no shadow: NARUTO, LUCK SEVEN, Uraroji.
- 和欧 pairing. EN leads, JP is smaller below: AoT (JP/EN 0.7 for the quote sub-lines, 0.42 for the title lockup), Mafia (JP title 0.42 of the EN cap), NARUTO (JP 0.24 of EN), GBC (JP 0.38), SHIKA-Q (EN 18 %H under a 9 %H JP, i.e. EN twice JP). Latin inside a JP line is the same size and family (Forza "Horizon", Metaphor "RPG"). The English sub-line under a JP title is rare (Devil Connection 0.13). Latin classes: classical serif caps for drama (AoT, NARUTO kicker), ultra-condensed black for announcements (NARUTO), DIN-like geometric for corporate (Donuts, Uraroji), humanist sans for dates (AoT2).
- Punctuation. Catch copy has no 。 (all headline lines), narration subtitles do (Meg "少女、メグ。"). Used: ！ at the end of CTA and fun lines (Devil Connection full-width, Metaphor and SHIKA-Q half-width), …… as a trailing "…" (AoT, AoT2), "——" (Metaphor), 、 as a separator even in date lines (Pokemon doc: 2024年秋、Nii.の軌跡、公開), full-width spaces as separators (SHIKA-Q, Devil Connection), 『』 for a game name (Metaphor), 「」 for a name (LUCK SEVEN), 【】 for a title (LUCK SEVEN), full-width colon and 〜 in a title (Mafia). Numbers are half-width; Forza and Microsoft style put half-width spaces around the units ("2026 年 5 月 19 日"). Ruby: 1 of 15 (LUCK SEVEN, ruby at 0.4 of base).
- Catch-copy length per line: 5-19 chars, median 9-11; typical card is 2 lines of 5-9 chars; sentences up to 19 chars appear only as 2-line subtitle bands (Metaphor) or small mincho cards (Pokemon doc). Single-glyph emphasis by enlarging: none seen; a whole second line enlarged 1.45x (Metaphor) and a coloured second line (Forza magenta) are the usual ways.
- Sizes (kanji/kana body height as %H): catch copy over picture 7-13 %H (median 8); 2-line subtitle bands 5-6.5 %H; translated narration 3-4 %H; end-card date or CTA 3.5-5.5 %H (median 5); platform logos 2.5-5 %H; JP sub-line under EN title 4-5.5 %H; legal 1-2 %H; big pop card 18-20 %H.
- Margins: bottom captions sit at y about 80-92 %H with at least 5 % side margins; corner logos at 2-4 %. LUCK SEVEN's CTA at 2 %H from the edge and Mafia's JP title crossing the letterbox are the only violations of the 5 % margin.
- How titles are built. A drawn logo in 11 of 15 (Kanji logos with a red slash, bubble kana, brush katakana). Two of them are a Western EN logo with a plain gothic JP title set under it (Mafia), or a kanji logo with the EN romanisation as a sub-line (AoT 1 and 2). Pure set-in-font titles: Uraroji (company). Typefaces per PV: 2-4, median 3.
- Clichés seen: pink brush-stroke band under subtitles (Metaphor 6 times); big "2026年5月19日発売 + 今すぐ予約注文" CTA box (Forza); "続報はX" end line (NARUTO, LUCK SEVEN, Pokemon doc date); vertical text for a myth line (LUCK SEVEN); sepia EN quote over JP gloss (AoT 1 and 2, 2 of 15).

## 4. Proposed typography profiles

Sizes are % of a 1080-line frame; "(inferred)" marks values not directly measured.

### 4.1 action (Western)
- Title: custom-lettering logo; if set in a font: extended heavy grotesque, bold, ALL CAPS, tracking +0.3 to +0.5 em for a single word (Defilade) or tight for a chunky sticker (Dear Passengers); cap 13-16 %H (measured median about 15), upper centre or centre, white or off-white; 1 line (7 of 9) or 2 lines.
- Callout over picture: wide bold grotesque, ALL CAPS, tracking +0.05 em, cap 5 %H (measured, 1 teaser), lower left at x 7 %, white, no outline/band; line length up to 38 chars, 1-3 lines.
- Kicker/sub-line: classical or thin caps, tracking +0.3 em, cap 2-2.5 %H.
- Date / CTA end card: ALL CAPS, wide (+0.1 to +0.2 em), cap 4-5 %H (date alone on black may be 7-8 %H), centre, white or metallic, on black; platform row 4-5 %H; legal 1 %H.
- Faces: 1-3 per video; avoid more than 3 (measured median 3).

### 4.2 action (Japanese)
- Title: custom logo or JP gothic black italic with outline; block 35-40 %H.
- Catch copy / callout: gothic bold or black weight, slanted (about 10 degrees) when kinetic, 9-13 %H, 5-9 chars per line, 1-2 lines, centre to upper centre, white-yellow gradient, dark or teal outline about 1 %H; no 。; ！ allowed.
- Solemn lines (inferred for 3 teasers): mincho, 7 %H, tracking +0.5 em (full-width gaps), glow.
- Announcement: ultra-condensed black Latin cap 19 %H with a JP mincho sub-line 4.5 %H (ratio 0.24).
- End card: JP info 5.5 %H in a light mincho or gothic, 2 lines of 14-16 chars, full-width spaces for separators, cream or white, centre; platform logos 5 %H.
- Faces: 3-4.

### 4.3 Japanese PV defaults
- Display: custom logo; fallback a heavy rounded or black gothic, tracking 0 to +0.05 em, 12-20 %H for a title card or pop card.
- Catch copy: gothic bold, 7-8 %H (measured median), 2 lines of 9-11 chars, centre, white with soft shadow (outline about 1 %H only on pop styles); mincho light/regular with +0.3 to +0.5 em tracking for a solemn mood, 4-7 %H.
- Subtitle/caption: gothic bold or medium, 5-6.5 %H, 1-2 lines, line pitch 1.4-1.6 (measured 1.4-1.65), lower centre (y 80-92 %H) on a band (座布団) at 80 % width when over busy picture; no 。 except in narration.
- Sub-line under a title: gothic, 4-5.5 %H (0.25-0.45 of the main line), EN above JP.
- Date / CTA: gothic bold 5 %H (3.5-5.5), centre, white; units attached to digits with no spaces unless following Forza style.
- Colour: white or off-white (14 of 15), one accent per video (magenta, yellow, pink, green).
- Avoid: more than 4 typefaces; JP text below 3 %H except legal (measured legal 1-2 %H); vertical text inside the 5 % margin (the only 縦書き measured sat 28-72 %H, centre).
- Clichés and frequency: pink/colour brush band under captions 1 of 12 (6 uses); sepia EN quote with JP gloss 2 of 15; "続報は…" 3 of 15; full-width space separators 3 of 15; ！ in CTA 5 of 15.
