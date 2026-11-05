# Japanese typography for burned-in video text: web research

Scope: the WEB side of the typography references for shotpilot's finishing pass (cards on black, captions on a dark band, running count, title, end card; Japanese with an optional small English sub-line, rendered by ffmpeg). Researched 2026-11-05.

## Provenance legend

- [S] Sourced. I read the page (fetched text or a raw file) and the URL is cited inline.
- [S snippet] Only a search-result snippet or a search-engine summary was seen. Treat as a lead, not as confirmed.
- [I] Inference or convention. My own conclusion, or a common practice without a primary source.

Coverage caveat. Search quality for Japanese design-practice sources was poor. I found no usable Morisawa, Fontworks, MdN or デザインのひきだし article on genre-specific typeface choice, and no broadcaster's telop guideline with numbers. Section 3 is therefore thin and mostly [I]. Do not present it as sourced.

---

## 1. Authoritative Japanese text-layout rules (JLREQ)

Source: W3C, Requirements for Japanese Text Layout, https://www.w3.org/TR/jlreq/ . The page is about 540k characters; I read the parts covering sections 1-3.3 in four chunks (the last chunk, from offset 400000, was not read). Section 3.2 (Latin and numeral spacing, baseline, size ratio) was only partly visible.

### Takeaway

- Default is solid setting (ベタ組み): no added space between characters. Tracking is not a lever in Japanese; the exceptions (fixed-interval, even spacing, tight/詰め組み) are for short headings, captions, display text. So for display text, tracking is allowed, but positive only, and uniformly applied.
- Full-stops, commas, and brackets are half-em advance characters, and the layout engine adds half-em space to make them look full-width. In video, where lines are short and punctuation is mostly dropped (see section 2), the practical rule is: avoid 。、 at line ends, use brackets sparingly, and never start a line with a closing bracket, 。、, ー, small kana, or ・.
- Latin next to kanji/kana wants about a quarter-em gap; solid setting looks too tight. This is the single most relevant rule for the English sub-line and for numerals inside Japanese.
- Ruby is half the base size; emphasis dots are half the base size.

### Cited findings

- [S] Solid setting default: "Kanji, hiragana and katakana are normally set solid" (section 2.1.3); even spacing and tsumegumi appear mainly in short headings, running heads, captions, table labels and some display text. https://www.w3.org/TR/jlreq/
- [S] Punctuation (3.1): commas, full stops, opening and closing brackets, middle dots have a half-em advance. Added space: comma half em after; full stop half em after (mid-line, and in principle at line end); opening bracket half em before; closing bracket half em after; middle dot quarter em before and after. This added space may be removed in line adjustment (except after full stops). https://www.w3.org/TR/jlreq/
- [S] Question and exclamation marks are full-width; at sentence end no space before and one em after; if a closing bracket follows, no space after the mark and half em after the bracket. https://www.w3.org/TR/jlreq/
- [S] Hyphen is a quarter em; en dash and double hyphen are half-width; wave dash is full-width (3.1.6). https://www.w3.org/TR/jlreq/
- [S] Line-start prohibitions (禁則, 3.1.7): closing brackets, hyphens, dividing marks, middle dots, full stops, commas, iteration marks, prolonged sound mark, small kana. Line-end prohibition (3.1.8): opening brackets. https://www.w3.org/TR/jlreq/
- [S] Unbreakable sequences (3.1.10): consecutive em dashes, ellipses, two-dot leaders; European numerals do not break between digits; prefixed (yen) and postfixed (%) symbols stay with the numeral. https://www.w3.org/TR/jlreq/
- [S] Western text (3.2): Western word space is one third em. Kana or kanji adjacent to Latin letters or numerals get quarter em between them; the text notes solid setting there "looks too tight". Half-width numerals are recommended in horizontal text. https://www.w3.org/TR/jlreq/
- [S] Vertical text (2.3.2): single alphanumerics or capitalized abbreviations set upright in one cell (full-width monospaced glyphs); English words and sentences are rotated 90 degrees clockwise, with extra space before and after. Horizontal text always keeps Latin upright. https://www.w3.org/TR/jlreq/
- [S] Tate-chu-yoko (縦中横, 3.2.5): usually applied to two-digit numbers; characters set left-to-right solid, then centered in the vertical line. https://www.w3.org/TR/jlreq/
- [S] Ruby (3.3): horizontal text puts ruby above the base, vertical to the right. Ruby size is half the base ("the half size of the base characters"). Headings of 12 pt or larger may use ruby smaller than half. Base text under about 7 pt should avoid ruby. Overhang over hiragana/katakana up to one ruby em. Mono-ruby and group-ruby (group ruby cannot be broken across lines). https://www.w3.org/TR/jlreq/
- [S] Emphasis dots (3.3.9): half the base size; bullet for horizontal text above the character, sesame dot for vertical on the right. https://www.w3.org/TR/jlreq/
- [S] Line gap (2.4): typically half em to one em; half em acceptable for short lines. Recommended maximum line length about 40 characters horizontal. This is body-text guidance; for display lines the video value of 1.3-1.4 (line-height) is consistent with a gap of 0.3-0.4 em [I].
- [S] Superscripts and subscripts about 60% of base size. https://www.w3.org/TR/jlreq/
- [I] The 和欧混植 size rule (Latin slightly smaller than kana frame, baseline alignment) is in section 3.2 and is not covered by what I read. Practitioner convention is to scale Latin to roughly 90-95% of the kana em and keep a shared baseline. Mark as unsourced.

---

## 2. Japanese video text practice

### Takeaway

- Netflix's Japanese guide is the best primary source: it limits a line to 13 full-width characters (horizontal subtitles), two lines at most, reading speed 4 characters/second, and tells translators not to use 。 and 、 but to substitute half-width and full-width spaces. It does not state font size.
- Teaser cards are not subtitles (they are read as display text), so the 13-character limit is a good ceiling for caption lines and a floor-level discipline for cards.
- Size, outline and band values come from blog-level sources. The consistent finding is: white text, black outline or a semi-transparent dark band (座布団), size about 3.5-8% of frame height. These numbers vary by source; use them as ranges, not rules.

### Cited findings

- [S] Netflix Japanese Timed Text Style Guide, https://partnerhelp.netflixstudios.com/hc/en-us/articles/215767517-Japanese-Timed-Text-Style-Guide (read via fetch):
  - Font: proportional sans serif (example given: MS Gothic), white. No font size is stated (the size reference was removed in a 2020 change).
  - Max characters per line: 13 full-width for horizontal subtitles, 11 vertical; SDH 16 horizontal, 11 vertical. Full-width character, space, or punctuation counts 1; half-width counts 0.5.
  - Maximum two lines per event.
  - Position: horizontal subtitles centered, at the bottom; avoid the top unless needed. Vertical subtitles go on the right by default, top-aligned.
  - Punctuation: do not use 。 or 、. Replace 、 with a half-width space and 。 with a full-width space. ？ and ！ are full-width and, if a new sentence follows on the line, are followed by a full-width space. Official titles of works may keep punctuation. Do not mix half-width English with full-width symbols such as ! ? " on the same line.
  - Quotes: half-width double quotes without spaces for citations; 「」 for titles of albums, books, films, songs, programs.
  - Numbers: horizontal, single digits full-width, two or more digits half-width. Vertical, single digits full-width, two digits half-width with tate-chu-yoko, three or more full-width set vertically. No commas; up to four digits no comma, larger use 万 (e.g. １万2345). Dates as 2008年12月31日.
  - Latin: acronyms full-width without periods or spaces (ＢＢＣ); words or acronyms longer than five letters may be half-width. Replace English with Japanese where natural.
  - Ruby: first appearance only (kids' content: every time); never use kanji as ruby; cover the whole kanji group where it helps.
  - Reading speed: up to 4 characters/second (subtitles), 7 (SDH). Minimum duration 500 ms (12 frames at 24 fps).
- [S snippet] Size relative to frame height, from a search summary of blog posts (not primary): optimal text size 3.5-5% of frame height (38-54 px at 1080p); another source 6-8% (about 50 pt); BBC's guideline is cited secondhand as line height 7-8% of frame height; subtitle-tool font size 60-63 at 1920x1080. Sources are personal and agency blogs, e.g. https://note.com/inadigital/n/nfbbf2927e541 and https://dream8.net/2026/09/09/video-telop-font-color-guide/ (seen as titles only).
- [S snippet] Outline: white text with black outline is called the most versatile; thickness suggestions of 5-10% of text size, or 2-4 px for short-form video; a semi-transparent black base (座布団) when the background is busy. Same blog-level sources.
- [S snippet] Contrast: WCAG AA 4.5:1 cited as a readability reference (nemovideo blog, https://www.nemovideo.com/ja-JP/blog/text-overlay-design-guide-jp). Not specific to video.
- [S snippet] Safe areas for 1920x1080 from Xbox docs: action-safe 1792x1016 px, title-safe 1720x1000 px (game-console figures, not broadcast): https://devdocs.xbox.com/ja-jp/build/core-features/graphics/overviews/screen-areas . Traditional broadcast convention: action-safe 90%, title-safe 80% of frame (Filmora explainer, snippet). The EBU R95 document defines safe areas for 1080p (https://tech.ebu.ch/docs/r/r095.pdf); I saw only the title. Vertical-video platform margins quoted: TikTok avoid top 9% and bottom 20%, Reels top 14% and bottom 20-35% (snippet, irrelevant to horizontal teasers).
- [S snippet] Burned-in text is not affected by viewer caption settings, so legibility must be built into the render (pc-koubou / youtube-related snippets).
- [I] A UD font (e.g. BIZ UD) is a readability choice rather than a broadcast rule. I found no broadcaster guideline stating a UD requirement. The Google Fonts description of BIZ UD is cited in section 5.

---

## 3. Japanese game PV / teaser typography and logo practice

### Takeaway

Little sourced material was found. The sourced points are: (a) the general typeface-impression table from Fujifilm, (b) one game-creator comment that opening fonts must suit characters and background and also stay legible, and (c) a vendor claim that brush faces suit horror and wa-fantasy. Everything else in the register mapping is convention [I] and is labelled so.

### Cited findings

- [S snippet] Typeface impressions (Fujifilm design seminar, via search summary): mincho reads as flowing, soft, feminine and formal; gothic as strong, hard, masculine and modern; rounded gothic as gentle and cute; brush faces as Japanese-style, classical, traditional; design faces for playfulness and personality. Advice: thin mincho or gothic for large amounts of text; heavy gothic or decorative faces for titles and impact; strong decorative faces become hard to read when small. https://www.fujifilm.com/fb/ja/support/colorprint/howto/design/seminar-05
- [S snippet] A game creator (Junichi Kawamura) says opening fonts must match the characters and background mood, cannot be chosen by taste alone, and must account for player legibility (gamebiz, https://gamebiz.jp/news/226490 ; seen only through a search summary).
- [S snippet] Shouwa Shotai brush fonts including "泥姫" are described as suited to horror and Japanese fantasy (Gizmodo Japan, https://www.gizmodo.jp/article/showa_font/). Vendor-promotional.
- [S snippet] A Japanese research proceeding on horror game packages reports many title logos use Latin letters only, and that Latin letterforms in Japanese horror differ from overseas works: https://www.jstage.jst.go.jp/article/digrajproc/10/0/10_203/_article/-char/en
- [S snippet] Practitioner process: place candidate typefaces on a game screenshot and compare around a thousand faces before narrowing (summary of the Kawamura piece).
- [S snippet] English-language trade pieces (typetype.org gaming fonts, PC Gamer "How game developers pick their fonts") say display faces suit logos, heavy impact faces create excitement, script faces give intimacy. Not Japanese-specific.

Convention table (all [I], my synthesis from the above and from the Google Fonts descriptions in section 5; not sourced as rules):

| Genre / mood | Typical Japanese register | Notes |
|---|---|---|
| Horror | Mincho (thin to regular) with wide letterspacing on cards; brush (毛筆) for titles | Restraint and contrast of size; small text, much black |
| Fantasy / 和風 | Mincho, calligraphic (Yuji family), brush | Large title in brush, sub-line in mincho |
| Sci-fi | Gothic (geometric, UD-like), outlined or pixel for retro | English sub-line carries more weight |
| Pop / cozy | Rounded gothic, handwritten, kana-pop faces | Large size, generous line height |
| Action | Heavy gothic, slanted display (Reggae One style) | Short lines, big numerals |
| Retro / pixel | Dot gothic | Render at integer scale only |

- [I] Emphasis by size: in Japanese display text, emphasis is by size and weight contrast, not italics (synthetic italic distorts kana) and not underline. Use two sizes per card at most.
- [I] English under Japanese: set smaller (about 40-60% of the Japanese size), lighter weight, tracked slightly positive, in a Latin face that matches the Japanese register (serif under mincho, sans under gothic). Not sourced.

---

## 4. Fonts and licensing for burned-in video text

### Takeaway

- OFL fonts are safe for video output; the only prohibitions are selling the font by itself and misusing the Reserved Font Name on modified versions.
- Windows-bundled fonts are allowed for video captions if rendered to bitmap, which is what ffmpeg drawtext does, and the font file is not embedded in the video. Microsoft explicitly says they may not be shipped in a game or app, so shotpilot must not bundle them in the installer; looking them up from the user's own system is fine.
- Commercial services: Morisawa installed-plan terms reportedly carry no extra fee by use (video, books, broadcast) [snippet only]. Fontworks LETS has been reorganized under Monotype; the game-embedding option ended, with prices reported as a large increase. For a tool that burns text into user video, the safest bundled set is OFL fonts.

### Cited findings

- [S] SIL OFL FAQ, https://openfontlicense.org/ofl-faq/ : designers can use OFL fonts for print, logos, graphics, video titling and products ("Yes. You are very welcome to do so", FAQ 1.1). You remain the author and copyright holder of derived graphics (1.1.1). Embedding an OFL font in a document does not change the document's licence (1.13). The font is not sold by itself (1.5); derivatives cannot be sold by themselves (3.3); bundling with sold software is allowed (1.4); modified versions stay under OFL (3.5); Reserved Font Names must not be used to identify a modified font (5.3).
- [S] Microsoft, Font redistribution FAQ, https://learn.microsoft.com/en-us/typography/fonts/font-faq (page dated 2024-11-19):
  - "Can I use the fonts to produce captions and text included in videos?" Answer: viewed as the same as graphic files, providing the captions or text is rendered as a bitmap image, and the font files themselves are not embedded within the video file.
  - Graphic files made with the fonts can be shared and sold; the same holds for logos.
  - Fonts may not be redistributed, copied to other computers or servers, converted to other formats (including bitmap fonts), or modified. They may not be embedded in a game, app, or device; PNG/GIF/JPEG images of words and passages can be used in a game or app, but converting to a bitmap font is not allowed.
  - Extended rights (shipping the font with an app or game) come from the foundry or Monotype.
  - The FAQ covers the fonts supplied with Windows as system-wide resources. I did not verify per family that Yu Gothic, Yu Mincho, Meiryo, BIZ UD are in that set; [I] they ship with Windows 10/11 and fall under it, but the Microsoft fonts list was not read.
- [S snippet] Morisawa: installed plans (Morisawa Fonts standard plans, Select 8, 24) set no additional licence fee by use (video, books, broadcast) per a third-party article citing note.morisawa.co.jp (https://atsoho.com/blog/video-font-license-commercial-use-check) . The primary Morisawa terms were not retrieved (morisawa.co.jp/fonts/faq returned 404). Also from snippets: Morisawa App Tools ONE for app and game embedding priced at 72,000 yen per PC per year (ex-tax), date unknown (gamebiz, https://gamebiz.jp/news/171140); the 2013 game pack was 315,000 yen per year (https://kyodonewsprwire.jp/release/201306112465).
- [S snippet] Fontworks LETS: a 2018 CGWORLD article says no secondary-use fee arises for telops in TV programs and CMs or for game use (https://cgworld.jp/feature/201808-font.html). A 2021 notice says imaging fonts for apps/games was always permitted, with the embedding option adding font-file embedding (GameBusiness). In 2025 the game-embedding option's new sign-ups and renewals ended (reported 2025-11-30, https://www.gamespark.jp/article/2025/11/30/160034.html [S, read]): old price about 60,000 yen/year per license; Monotype overseas plan listed at USD 20,500/year for 5 licenses (over 3.2 million yen), 25,000-user cap; mojimo-game 5,280 yen/year for a subset, individuals only. The article does not address video or still-image use. A 2026-06-04 notice announces a new LETS/Monotype game/app embedding and distribution option (seen as a news headline only on fontworks.co.jp).
- [S snippet] TypeBank: only a 2013 "TB Game Pack 50" press item surfaced; nothing on current video terms. Not researched further.
- [S] Source Han Sans: LICENSE.txt in the repo header reads "licensed under the SIL Open Font License, Version 1.1", Reserved Font Name 'Source' (Adobe, 2014-2025). https://raw.githubusercontent.com/adobe-fonts/source-han-sans/release/LICENSE.txt . Source Han Serif: header read only to the copyright and Reserved Font Name line (2017-2022); the repo is the same family under the same terms [I]. https://raw.githubusercontent.com/adobe-fonts/source-han-serif/release/LICENSE.txt
- [S snippet] LINE Seed JP: SIL OFL 1.1 (GitHub https://github.com/line/seed ; LY Corporation release https://www.lycorp.co.jp/ja/news/release/020040/ says it is offered on Google Fonts).
- [S snippet] 源暎こぶり明朝: SIL OFL 1.1, commercial use allowed (Impress Watch, https://forest.watch.impress.co.jp/docs/review/1113819.html). 源暎エムゴ: licence not confirmed; author's pages read (https://okoneya.jp/font/) give no licence text. Do not list as cleared.
- [S snippet] 装甲明朝 (Flop Design): SIL OFL 1.1, free for personal and commercial use per a 2017 article (https://forest.watch.impress.co.jp/docs/review/1094085.html). Official page returned HTTP 500.
- [S] x12y16pxMaruMonica: author's own licence (not OFL). Commercial use permitted; bundling the .ttf in a game release permitted; credit optional; may not sell the font itself or claim authorship; author says terms may change but without introducing fees. Video not named but the medium is unrestricted. https://hicchicc.github.io/00ff/
- [S snippet] PixelMplus: licence inherited from M+ FONTS (free use, redistribution, modification without conditions), 2013 review: https://forest.watch.impress.co.jp/docs/review/613180.html . Check the licence file in the current download.
- [S] 美咲フォント / Little Limit: the font list page (https://littlelimit.net/font.htm) carries a general notice: "Unlimited permission is granted to use, copy, and distribute them, with or without modification", commercial and noncommercial. The Misaki page itself (https://littlelimit.net/misaki.htm) defers to that licence section. Good enough to list as free commercial use; the exact per-font terms page (misaki.htm licence) was not displayed.
- [S snippet] ラノベPOP: free for commercial and noncommercial use, built on M+ FONTS (Impress Watch reviews, https://forest.watch.impress.co.jp/docs/review/693009.html for the older release; v2 on BOOTH). Bundling/video terms not read.
- Not confirmed: けいふぉんと (the page I fetched was a different font's), 源暎エムゴ. Do not include them in a bundle.

### Licence table

| Font / source | Video burn-in allowed? | Conditions | URL |
|---|---|---|---|
| Any OFL font (Google Fonts, Source Han, LINE Seed JP, M PLUS, Noto, Zen, Shippori, Kaisei etc.) | Yes [S] | Do not sell the font by itself; modified fonts must change name if RFN is reserved; keep OFL.txt if redistributing the font | https://openfontlicense.org/ofl-faq/ |
| Windows-bundled: Yu Gothic, Yu Mincho, Meiryo, BIZ UD (as shipped in Windows) | Yes [S, with the family membership as inference] | Text must be rendered as bitmap, font not embedded in the video; cannot redistribute, convert, or ship in a game/app | https://learn.microsoft.com/en-us/typography/fonts/font-faq |
| Source Han Sans / Serif | Yes [S] | OFL 1.1, RFN 'Source' | https://raw.githubusercontent.com/adobe-fonts/source-han-sans/release/LICENSE.txt |
| LINE Seed JP | Yes [S snippet] | OFL 1.1 | https://github.com/line/seed |
| 源暎こぶり明朝 | Yes [S snippet] | OFL 1.1 | https://forest.watch.impress.co.jp/docs/review/1113819.html |
| 源暎エムゴ | Unconfirmed | Licence not found | https://okoneya.jp/font/ |
| 装甲明朝 | Yes [S snippet] | OFL 1.1 | https://forest.watch.impress.co.jp/docs/review/1094085.html |
| x12y16pxMaruMonica | Yes [S] (video not named, medium unrestricted) | No selling the font; no claiming authorship; credit optional | https://hicchicc.github.io/00ff/ |
| PixelMplus | Yes [S snippet] | M+ licence (free, no conditions) | https://forest.watch.impress.co.jp/docs/review/613180.html |
| 美咲フォント | Yes [S] general notice | "Unlimited permission to use, copy and distribute", as is | https://littlelimit.net/font.htm |
| ラノベPOP | Yes [S snippet] | Free commercial/noncommercial; M+ based | https://forest.watch.impress.co.jp/docs/review/693009.html |
| けいふぉんと | Unconfirmed | Not retrieved | none |
| Morisawa Fonts (subscription) | Reportedly yes, no extra fee by use [S snippet] | Installed plans; needs subscription; game/app embedding is a separate product; primary terms not seen | https://atsoho.com/blog/video-font-license-commercial-use-check |
| Fontworks LETS / Monotype | Telop and game imaging reportedly yes (2018) [S snippet]; current terms unconfirmed | Subscription; game-embedding option ended 2025 | https://www.gamespark.jp/article/2025/11/30/160034.html |
| TypeBank | Unknown | Not researched beyond 2013 press release | none |
| Adobe Fonts (includes Morisawa faces, BIZ UDPGothic) | Commercial use allowed per Adobe [S snippet] | Requires Creative Cloud sync; not redistributable | https://fonts.adobe.com/foundries/morisawa |

### shotpilot implications [I]

- ffmpeg drawtext/subtitles renders glyphs into frames, which matches the bitmap condition in Microsoft's FAQ. Keep it that way: never mux a font file into the mp4 or copy Windows fonts into the project or installer.
- Bundle (embed in the binary) only OFL fonts, and ship OFL.txt with them. This avoids dependence on the user's Windows install and works on other OSes.
- Licence conditions around Reserved Font Names matter only if shotpilot subsets or renames fonts. Do not subset in place; if subsetting for size, rename.

---

## 5. Google Fonts Japanese families

Sources: the `DESCRIPTION.en_us.html` files in the google/fonts repository (the text shown on each family's "About" page), e.g. https://raw.githubusercontent.com/google/fonts/main/ofl/shipporimincho/DESCRIPTION.en_us.html . Files retrieved for the families listed below; all [S] unless noted. I did not have the owner's list of 68, so rows cover the families I could fetch (about 45). Families not fetched: Zen Kaku Gothic Antique (read inside the Zen Kaku Gothic New text), Kosugi / Kosugi Maru (Apache, descriptions not displayed), M PLUS Rounded 1c (folder not found), Yuji Mai / Boku included. Zen Tokyo Zoo and Zen Loop are Latin-only and not usable for Japanese text. Register tags are my inference [I] from the stated character.

### Takeaway

- Mincho for horror, fantasy, and wa registers: Shippori Mincho (Tsukiji No. 5 heritage, designed to be beautiful at large size), Zen Old Mincho (weights up to Black, works at large size), Kaisei Tokumin (extra bold for titles), Noto Serif JP, BIZ UD Mincho (UD, thicker horizontals).
- Brush and calligraphic for 和風: Yuji family (calligrapher Yuji Kataoka), Potta One (brush-inspired).
- Gothic for sci-fi and UI-like text: Zen Kaku Gothic New / Antique, Murecho, BIZ UD Gothic, M PLUS 1 / 2, Noto Sans JP, LINE Seed JP [LINE Seed JP not fetched; OFL via snippet].
- Heavy display for action: Dela Gothic One, Reggae One, Rampart One, Train One, RocknRoll One.
- Pop and cozy: Mochiy Pop One, Hachi Maru Pop, Cherry Bomb One, Zen Maru Gothic, Kiwi Maru, Stick, Yusei Magic, Darumadrop One.
- Retro: DotGothic16.

### Table: family -> stated character -> suggested register / genre tags

Quotation marks mark text copied from the About text; other cells are paraphrase.

| Family | Stated character (About text) | Register / genre tags [I] |
|---|---|---|
| Shippori Mincho | "old-style Mincho" based on Tsukiji Type Foundry No. 5; Regular for novels, Extra Bold for headlines; "beautiful even when the characters are enlarged"; alternate characters included | mincho, literary, horror, fantasy, 和風, title and card |
| Shippori Antique (and B1) | follows manga dialogue standards; kana in an antique Ming-style based on Shippori Mincho with thick lines; kanji from Genseki Gothic; B1 has rounded corners and ink pooling | antique, retro-horror, 昭和, manga-like emphasis |
| Zen Old Mincho | "classic design" mincho, started at Regular; Bold and Black added; "works well in large sizes" | mincho, horror, fantasy, title, caption |
| Zen Antique / Soft | two kinds of antique kanji; hiragana and Latin slightly lighter, katakana and kanji slightly heavier "unique rhythm"; Soft has rounded corners | antique, 和風, horror, retro |
| Zen Kaku Gothic New / Antique | contemporary gothic; "naturally high legibility"; Antique is "classical yet simple and stylish", good for title, text, captions | gothic, sci-fi, UI, caption, modern |
| Zen Maru Gothic | rounded sans with deep corner rounds; "soft and natural"; thin weight easy to use | rounded, cozy, pop |
| Zen Kurenaido | handwriting-like, brush elements removed, "ballpoint" impression | handwritten, cozy, diary |
| Kaisei Tokumin | "Extra Bold Serif Kana" for strong titles, keeps legibility | mincho-heavy, title, horror, fantasy |
| Kaisei Opti | "modern style" typeface, "cheerful and breezy" | mincho-modern, cozy, light |
| Kaisei Decol | dots at stroke ends, "cute and fun", best for short sentences and titles | decorative mincho, cozy, short title |
| Kaisei Haru No Umi | wavy organic kana, katakana slightly smaller, old-style impression, better at text size | mincho-soft, cozy, narrative |
| Hina Mincho | "old-fashioned and cute" | mincho-cute, cozy, retro |
| New Tegomin | mincho drawn on a square grid; clean with the organic nature of handwriting | mincho-hand, cozy, narrative |
| Yuji Syuku | calligrapher Yuji Kataoka's handwriting; "tradition and dignity, but approachable" | calligraphic, 和風, fantasy, title |
| Yuji Mai | "free, unrestrained, emotional beauty" | calligraphic, 和風, drama |
| Yuji Boku | "new and joyful design", simple and warm | calligraphic-light, 和風, cozy |
| Potta One | inspired by brush lettering | brush, 和風, action, horror title |
| Dela Gothic One | "flat, very thick Gothic", for posters and packaging, stable and strong | heavy gothic, action, title, numeral count |
| Reggae One | popular in Japanese boys' magazines; sharpened ends, "dynamic pulse", rhythm, movement, energy | display, action, hype title |
| Rampart One | outline shadow font, 3-D blocks, impact and strength | display-outline, action, title |
| Train One | gothic with outer and inner line, vibrant, "strong first impression" for logos and titles; sold as "Railway" by FontWorks | display-outline, action, retro, title |
| RocknRoll One | pop style; strokes of varying intensity, rounded dots, lively | pop, action-light, cozy title |
| Mochiy Pop One | casual gothic based on writing by women in their 20s; display font for manga, magazines, movies, signs; kanji from Noto Sans JP | pop, cozy, title |
| Hachi Maru Pop | cute, popular among young Japanese girls in the 1970s-80s; informal handwriting, works at larger sizes | pop, cozy, retro 昭和 |
| Cherry Bomb One | "Cute with a kick"; round and modern; title logos, packaging, children's magazines | pop, cozy, title |
| Stick | straight lines, cute and playful, pastoral ambience | pop, cozy, light |
| Yusei Magic | permanent-marker handwriting, thick verticals and thin horizontals, highly visible | handwritten, pop, cozy, board-style |
| Yomogi | extra-thin handwriting, cute and readable, monospaced, also vertical | handwritten, cozy |
| Darumadrop One | flavorful handwriting inspired by Daruma Otoshi | handwritten, pop, 和風-playful |
| Slackside One | sophisticated handwritten Japanese ("slackside means loose end") | handwritten, light, cozy |
| Klee One | pencil/pen handwriting script, quiet and elegant, ideal for body text | script, cozy, narrative |
| Kiwi Maru | round font for "everyday and slang" expressions in the digital age; created for digital devices | rounded, cozy, casual |
| Tsukimi Rounded | kana-only, rounded sans based on Tsukiji Type Foundry specimen | kana-only; needs a kanji fallback |
| Palette Mosaic | built from primitive shapes by artists in the Shibuya Font project; "strong and solid" | display, pop, experimental |
| DotGothic16 | based on old 16x16 gothic bitmap fonts; recreates early game, phone and screen feel | pixel, retro game, sci-fi retro |
| Murecho | low-contrast, flat-terminal gothic for text settings in Japan; Joyo kanji; nine weights, variable | gothic, caption, UI |
| M PLUS 1 | "comfortable curves and straight lines", modern and generous; small text to big titles; nine weights | gothic, caption, sci-fi, title |
| M PLUS 2 | "somewhat classic letterforms", "classic modern" | gothic, caption |
| M PLUS 1p | proportional Latin with fixed-full-width kana; kana mix straight lines and hand-drawn curves; seven weights | gothic, caption (older design) |
| M PLUS 1 Code | monospaced half-width Latin with M PLUS 1 kana; programming | gothic-mono, count, tech |
| Noto Sans JP | unmodulated design for Japanese; multi-weight | gothic, neutral baseline fallback |
| Noto Serif JP | modulated (serif) design; multi-weight | mincho, neutral baseline fallback |
| Sawarabi Gothic | designed for high legibility at small sizes; limited kanji (4,469 in this version) | gothic, caption (limited glyphs, check coverage) |
| Sawarabi Mincho | "delicate and beautiful", for text and headlines; about 3,297 kanji in this version | mincho, light (limited glyphs, check coverage) |
| BIZ UD Gothic / UDP Gothic | Morisawa universal design font for education and business documents; kana slightly smaller than kanji; kanji hooks and serifs simplified; large counters | UD gothic, caption, count |
| BIZ UD Mincho / UDP Mincho | UD mincho with horizontal strokes thicker than ordinary mincho and enlarged dakuten | UD mincho, caption on band, narrative |
| IBM Plex Sans JP | corporate IBM typeface, "neutral yet friendly Grotesque" | gothic, sci-fi, tech (Latin-led) |
| LINE Seed JP | not fetched; OFL via LY release; corporate typeface [S snippet] | gothic, pop-light, UI |
| Zen Tokyo Zoo, Zen Loop | Latin-only Zen fonts (stripes / loop forms) | not for Japanese text |

### Notes on pairing and use in shotpilot [I]

- Mincho title plus gothic sub-line is not a rule; keep one family for Japanese per project and vary only weight and size. Pair a Latin from the same foundry (e.g. Zen Old Mincho with a serif Latin) for the English line.
- Check glyph coverage before selecting Sawarabi (partial kanji) and Tsukimi Rounded (kana only). For cards with rare kanji, fall back to Noto.
- Kana-pop fonts (Hachi Maru Pop, Cherry Bomb One, Mochiy Pop One) lose legibility at caption size; reserve for titles and cards.
- Brush faces (Yuji, Potta) at less than about 5% of frame height are hard to read on motion; reserve for titles.

---

## Appendix A. Rules for Japanese display text in video

Rule strength: S = sourced as stated; S* = blog-level or snippet number; I = convention. Numbers marked S* vary between sources, so treat them as ranges.

| Rule | Value | Strength | Source |
|---|---|---|---|
| Max characters per horizontal subtitle line | 13 full-width characters (subtitles); 16 (SDH) | S | Netflix JA guide |
| Max lines per caption | 2 | S | Netflix JA guide |
| Reading speed | at most 4 characters/s (subtitles); 7 (SDH) | S | Netflix JA guide |
| Min duration of a caption | 500 ms (12 frames at 24 fps) | S | Netflix JA guide |
| Caption position | bottom centre, horizontal; avoid top unless needed | S | Netflix JA guide |
| Caption colour and font | white, proportional sans (MS Gothic cited) | S | Netflix JA guide |
| Dropping 。 and 、 | 、 -> half-width space; 。 -> full-width space; keep for official titles | S | Netflix JA guide |
| Half-width vs full-width counting | half-width counts 0.5 | S | Netflix JA guide |
| Numbers, horizontal | single digit full-width; two or more digits half-width; no comma, use 万 beyond four digits | S | Netflix JA guide |
| Mixed Latin | do not mix half-width Latin with full-width ! ? " on a line; Latin acronyms full-width, over five letters may be half-width | S | Netflix JA guide |
| Solid setting default | 0 added tracking on body Japanese; tracking only for short headings and display | S | JLREQ 2.1.3 |
| Gap between Japanese and Latin | about 1/4 em | S | JLREQ 3.2 |
| Western word space | 1/3 em | S | JLREQ 3.2 |
| Punctuation advance | comma, full stop, brackets, middle dot occupy half em; added half-em space; middle dot quarter em on each side | S | JLREQ 3.1 |
| Line-start prohibited | closing brackets, 。、, ー, small kana, ・, hyphens, iteration marks, ？！ | S | JLREQ 3.1.7 |
| Line-end prohibited | opening brackets | S | JLREQ 3.1.8 |
| Do not break | ―― (consecutive em dashes), ……, digit runs, number-plus-unit symbols | S | JLREQ 3.1.10 |
| Ruby size | 1/2 of base | S | JLREQ 3.3 |
| Emphasis dot size | 1/2 of base | S | JLREQ 3.3.9 |
| Line gap, short lines | 0.5 em acceptable; typically 0.5-1 em | S (body text) | JLREQ 2.4 |
| Display line height | 1.3-1.4 on CJK display text | I (matches owner's brief) | none |
| No negative tracking on CJK | 0 or positive only | I (matches JLREQ solid setting; a negative value breaks the solid setting) | none |
| Text height as share of frame height | 3.5-5% (small), 6-8% (large); 38-54 px at 1080p | S* | blog posts via search snippet |
| Outline width | 5-10% of text size, or 2-4 px | S* | blog posts via search snippet |
| Band (座布団) | semi-transparent black under text on busy backgrounds | S* | blog posts via search snippet |
| Action-safe / title-safe | 90% / 80% of frame (broadcast convention); 1792x1016 / 1720x1000 at 1080p (Xbox) | S* | Filmora snippet; Xbox docs |
| Contrast | 4.5:1 minimum as a reference | S* | WCAG via blog |
| English sub-line size | about 40-60% of Japanese size | I | none |
| Emphasis | size and weight contrast, not synthetic italic | I | none |
| Vertical text numerals | two-digit numbers in tate-chu-yoko, English words rotated 90 degrees clockwise | S | JLREQ 2.3.2, 3.2.5; Netflix |

Open items for the owner: no broadcaster telop guideline was found (NHK or commercial networks); Morisawa Fonts primary terms for video/game were not reachable; JLREQ 3.2 baseline and size ratio for 和欧混植 was not read; 源暎エムゴ and けいふぉんと licences remain unconfirmed; the owner's list of 68 Google Fonts Japanese families was not available, so the table covers the families I could fetch.
