# Web research: motion of text, titles and transitions in game/film trailers

Scope: what practitioners and published guidance say about text/title/transition motion by genre, plus the hard limits (photosensitivity, readability). Collected 2026-11-05.

## Provenance legend

- [S] from a search result or fetched page, cited inline with URL. Note: WebSearch returned summaries, not full pages. Only the W3C WCAG page was actually fetched and read. Everything else marked [S] is a search-result summary of the page (snippet-level), and says so.
- [I] inference or common craft convention with no source found.
- Claim strength: **hard** = standard/regulation text; **soft** = one blog or student deck; **lore** = repeated online without a primary source.

Overall finding: the web has almost no quantified guidance on trailer text motion (fade lengths, easing, flash-frame counts by genre). The searches for genre-specific motion (cozy bounce, HUD scan lines, fantasy light sweeps) returned template-store listings only. The only hard numbers are photosensitivity and subtitle/reading limits. The genre table below is therefore mostly [I], and must be calibrated against the frame-by-frame measurements from the other agents.

---

## 1. Game trailer craft: titles, cards, supers, transitions

### Takeaway
Practitioner advice is about *whether and where* to use text, not how it moves. Derek Lieu's line is: use title cards sparingly, only when they say something interesting; reveal the title/logo after the hook; keep the hook inside ~10-15 s (attention wanes after ~5 s). Motion specifics must come from measurement and genre craft [I].

### Cited findings
- Lieu: don't use title cards if you have nothing interesting to say with them; the title/logo reveal comes after the opening hook; his rules for a cinematic trailer include no voiceover or title cards. [S, snippet-level, secondhand summaries] https://derek-lieu-yrd2.squarespace.com/blog/2023/3/11/i-made-the-worst-firewatch-trailer-ever , https://derek-lieu-yrd2.squarespace.com/blog/2023/1/7/what-game-trailers-can-learn-from-film-history . Strength: soft. The search tool said it found no single authoritative Lieu page on title-card pacing.
- Hook within first 10-15 s; logos at the very start are discouraged since attention wanes after ~5 s. [S, snippet; GMTK transcript and summaries] https://wnhub.io/news/analytics/item-237 . Soft.
- GDC-derived advice: "try not to describe things with text; let the game do the talking." [S, snippet via a Reddit post on the GDC talk] https://pol1.lr.ggtyler.dev/r/hobbygamedev/comments/192er9q/attempt_at_a_trailer . Lore.
- Film-trailer convention (student analysis): titles start ~10 s in, ~5 s between consecutive cards, thriller titles are bold, black/white/grey and "tend to fade in/out". [S, student slide decks, not industry] https://www.slideshare.net/slideshow/trailer-titlecredit-convention-research/39350377 . Lore (anecdotal), though consistent with [I] expectations.
- No source found for: Ant Farm / Buddha Jones / Trailer Park Group motion specs, Kert Gartner text motion, School of Motion/Art of the Title specifics on trailer supers. Searches surfaced only stock-template sales pages. Treat as not found.

## 2. Genre conventions for text motion and transitions

### Takeaway
No source gives numbers. Sourced fragments: horror leans on silence and cuts to black; a white flash is a standard "dramatic transition" and is implemented as a 1-2 frame full-white layer or a 3-10 frame dip. Everything else (cozy bounce, HUD builds, fantasy light sweeps) is common craft [I], supported only by the existence of template categories.

### Cited findings
- Horror: a sudden cut to black with a total audio drop can hit harder than a loud bang; "it takes nerve to hold the screen black longer than feels comfortable." [S, snippet; loosely attributed blog] https://dyerbolical.com/shadows-in-the-cut-revolutionising-horror-promotion/ . Lore/opinion.
- Horror: *Paranormal Activity* trailer opens on black that flickers twice like TV interference; fade-to-black and straight cuts are the common horror transitions, with fade-to-black read as mysterious/tense. [S, student analyses via search snippet] https://www.slideshare.net/slideshow/paranormal-activity-2/5596104 , https://prezi.com/o_wylsnd1gvq/editing-styles-of-horror-films . Soft.
- Horror: title reveal after a whispered name with silence before it (*Jessabelle* trailer breakdown). [S, snippet] https://www.slideshare.net/slideshow/jessabelle-trailer2-39280563/39280563 . Soft.
- White flash transition: trailers "often use a white flash for quick dramatic transitions". Implementation advice from editors' forums: full-white layer for 1-2 frames fading 100% to 0; "a short 3-10 frame transition will give you the effect"; Final Cut dip-to-colour with ~80 softness holds white for ~2 frames at a short length. [S, snippets from forums] https://www.dvinfo.net/forum/final-cut-suite/123393-white-flash-transition.html , https://blog.alex4d.com/2009/02/03/final-cut-tip-the-flash-frame-transition-you-already-have/ , https://community.adobe.com/t5/premiere-pro/video-effects-for-a-horror-thriller-trailer/m-p/5798185 . Soft.
- Template catalogs list "Cartoon Bounce Text Title", "Game Over Glitch Title", flicker/glitch/noise horror titles with channel shifts. [S, listings] https://uppbeat.io/motion-graphics/asset/bloody-trailer-title-2190 , https://videobolt.net/author/ezome/modern-glitch-title-5 . This shows the idioms exist; it is not evidence of how pros use them.
- Not found: any source on action slams/shake/speed-ramp counts, cozy easing/overshoot values, fantasy sweeps, HUD builds, strategy UI builds, pixel/retro text. Everything in the table for those is [I].

## 3. Japanese PV / teaser conventions

### Takeaway
No explanatory article on ゲームPV/アニメPV 文字演出 was found; searches returned stock-telop packs and academic papers. The only usable sourced fragment is the typeface-to-mood mapping in telop packs: 明朝 for 和風ホラー / quotes / warnings, ゴシック for chapter heads and titles. Claims about 縦書き, 一文字ずつ, impact テロップ in PV are [I] (well-known conventions but unsourced here). Defer to the frame-by-frame measurements.

### Cited findings
- A DaVinci Resolve horror telop pack ships ゴシック / 明朝 / 手書き in colour variants; 細明朝 is pitched for Japanese-horror mood, narration, quotes and warnings, ゴシック for chapter headings and titles. [S, product page summary] https://booth.pm/ja/items/8179171 . Lore (product marketing).
- Font choice changes the impression of a text string (e.g. 行書体 for a breeze) in comics. [S, Meiji Univ. paper summary] https://www.ai-gakkai.or.jp/jsai2016/webprogram/2016/pdf/894.pdf . Weak support for "typeface carries tone".
- Variety-style telop motion packs exist (9 movement types) but are TV-variety, not PV. [S] https://booth.pm/en/items/8806915 . Not applicable to teasers.
- [I] Known conventions to test against measurement: 縦書き 明朝 for 和 tone or end-card catchcopy; per-character reveal for title/catchcopy with long hold (余韻); fast impact テロップ in action/anime PV; thick ゴシック with outline in 実況-style or variety. No source.

## 4. Photosensitivity limits (these are hard constraints)

### Takeaway
Treat as inviolable for any flicker or white flash: **at most 3 flashes in any 1 s**, and avoid full-frame (or >25% of screen) saturated-red flashes. A single 1-2 frame white flash is allowed (it is 1 flash, not 3/s). A title "flicker" must stay at or below 3 pairs of bright/dark changes per second, or stay small (<25% of a 10-degree field) or low contrast. Japanese broadcast guidance is stricter on the wider cases (scene changes, patterns). No source says game-trailer platforms require it; Steam's trailer page mentions nothing about it in the summary we saw [S snippet], so this is a safety/good-practice floor, not a platform gate (unverified).

### Cited findings
- **WCAG 2.3.1 (Level A)**, fetched and read: general flash = pair of opposing relative-luminance changes each >= 10% of max (1.0), where the darker state is below 0.80. Red flash = any pair of opposing transitions involving a saturated red (R/(R+G+B) >= 0.8, difference > 0.2 in CIE 1976 u'v'). Pass if no more than 3 general and/or red flashes in any 1 s period, OR combined concurrent flash area <= 0.006 sr within any 10-degree field (~25% of that field). 341 x 256 CSS px = 10-degree estimate on a 1024 x 768 reference. [S, fetched] https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html . Hard.
- Same: current understanding docs cite ITU-R BT.1702 (20 cd/m2 change, darker image < 160 cd/m2) as the industry definition. [S, search snippet] https://www.w3.org/WAI/WCAG21/Understanding/three-flashes-or-below-threshold . Hard (snippet-level). Note: the ITU page fetch returned only a TOC, so BT.1702 numbers are via W3C only.
- **Ofcom / Harding**: Rule 2.13 of the Broadcasting Code requires precautions for PSE; guidance caps at 3 flashes/s (a 11 flashes/s news sequence was non-compliant); the Harding test is the automated PSE check, renamed "PSE test" in DPP specs; a pass does not guarantee compliance. The area and red thresholds sit in Annex 1 of Ofcom's guidance note, which I did not reach. [S, snippets] https://en.wikipedia.org/wiki/Harding_test , https://www.ofcom.org.uk/siteassets/resources/documents/about-ofcom/bulletins/broadcast-bulletins/archive/obb178/obb178.pdf . Hard for Ofcom's 3/s; area/red unverified.
- **Japan, NHK / 民放連 「アニメーション等の映像手法に関するガイドライン」** (made 1998-04-08 after the 1997 ポケモン incident; revised 2006 after ITU-R BT.1702, again 2020). Per search summary of the guideline text: 点滅は原則 1秒に3回を超えない; 「鮮やかな赤色」の点滅は特に慎重に; 点滅が同時に起こる面積が画面の1/4を超え、輝度変化10%以上 is the criterion for flashes to avoid; if over that, limit to 5 per second, luminance change <= 20%, and never more than 2 s continuous (given red rule is met); 強いコントラストの画面反転や輝度変化20%超の急激な場面転換も原則 1秒に3回まで; 縞・渦巻・同心円など規則的パターンが画面の大部分を占めるのを避ける. [S, snippet of guideline text] https://j-ba.or.jp/stance/jba103852.html , https://www.actv135.ne.jp/company/kijun_guideline.pdf , https://www.saiki.tv/basis/pdf/skill.pdf . Hard (regulatory text via summary; verify quotes in the official PDF before citing in UI).
- Implication for shotpilot [I]: dip-through-white = 1 flash. Two white flashes within 0.33 s would already hit the limit. Flicker title: cap at 3 bright/dark pairs per 1 s window, keep luminance step < 10% or area < 25% of frame, avoid red-dominant flicker, and never chain a flash with a flickering title in the same second.

## 5. Readability of on-screen text

### Takeaway
Numbers exist for subtitles, not trailer supers. Use them as lower bounds: hold >= 5/6 s (20 frames @24) for any text, Japanese <= ~4 chars/s, <= 13 full-width chars per line. Steam autoplay is silent and tiny, so large, high-contrast, few words. Size/safe-area numbers were not found online; use broadcast convention [I].

### Cited findings
- Netflix timed text: minimum 5/6 s (20 frames @24fps) per event, max 7 s; min 2-frame gap. [S, snippet] https://backlothelp.netflix.com/hc/en-us/articles/215758617 . Hard for subtitles; for supers it is a floor.
- Japanese subtitles: <= 4 chars/s, <= 13 full-width chars per line, max 2 lines (Netflix Japanese guide, via secondhand summary; the 4 cps practice is described as 85+ years old and not scientifically validated). [S, snippet] https://backlothelp.netflix.com/hc/en-us/articles/215767517 , https://www.jstage.jst.go.jp/article/let/54/0/54_167/_article/-char/en . Hard-ish for Netflix; soft otherwise.
- Broadcast-graphics rule of thumb: show a title long enough to be read twice: ~3 s for 10 letters, +1 s per 5 extra letters. [S, snippet, general titling guide] https://help.pinnaclesys.com/pinnacle/v24/en/help/pinnacle-studio/h2-giving-your-project-a-title.html . Soft; useful for card hold.
- Contrast >= 4.5:1; white with black outline or translucent band; bold gothic recommended for small screens; check on the smallest screen. [S, snippet of a JP production blog] https://0120.co.jp/blog/video-125/ . Soft.
- Steam: store makes a 600x380 poster and a 232x130 thumbnail; Valve says choose a representative frame and avoid text in it. Autoplay is silent (third-party), ~5 s to hold a viewer, high contrast reads better small. [S, snippet] https://partner.steamgames.com/doc/store/trailer , https://www.steampageanalyzer.com/blog/steam-trailer-best-practices . Official facts: sizes and "avoid text"; the rest is community consensus.
- Not found: text height as % of frame, title-safe/action-safe percentages (the search returned nothing). [I] The broadcast convention of ~90% title-safe / ~93% action-safe is widely known but unverified here. At 232 px thumbnail width, 5% text height (~6 px) is already unreadable, which supports "few, huge words" in the first seconds.

---

## Table: genre -> text motion (all entries [I] unless a source is given)

| Genre | Text entrance | Hold motion | Exit | Title reveal | Transitions | Numbers / source |
|---|---|---|---|---|---|---|
| Horror | Hard cut-in or slow fade; occasional 2-flicker open | Static, very slow drift or subtle jitter; hold in silence | Cut to black, or fade (slower than entrance) | Late; after silence; flicker/glitch-in or slow fade; whispered-name lead-in | Smash cut to black, fade to black, held black | Black hold "longer than comfortable" [S lore dyerbolical]; flicker twice then settle [S soft, student]; cap flicker at <=3/s [S hard WCAG] |
| Action | Slam/scale-down with 1-3 frame shake on beat | Static, snap to hits | Hard cut on next beat | Slam on a hit, white flash 1-2 frames | Hard cuts, white flash, speed ramps, flash frames | White flash as 1-2 frames, or 3-10 frame dip [S soft forums]; <=3 flashes/s [S hard]; no sourced slam/shake numbers |
| Cozy / life-sim | Soft rise + slight overshoot/bounce, ~0.3-0.5 s | Gentle float or none | Soft fade or sink | Handwritten/rounded; bounce-in; warm settle | Dissolve through warm tint; avoid black | Only existence of bounce templates [S listing]; numbers are [I] |
| Fantasy / RPG | Slow fade (0.8-1.5 s), light sweep | Static serif, faint particles | Slow fade | Serif, gold, light sweep, long hold | Slow dip through black or white, slow dissolves | [I] only; none found |
| Sci-fi | HUD build, scan-line wipe, brief glitch | Thin-line UI, slight flicker | Glitch-out or wipe | HUD assembly, glitch resolve | Glitch cut, hard cut, white flash | [I] only; glitch templates exist [S listing] |
| Strategy / sim | UI-like slide/pop, data-style count-up | Static panel | Slide out | Map/UI reveal; clean sans | Hard cuts, wipes | [I] only |
| Mystery | Type-on (per character), redaction wipe | Static | Cut or fade | Type-on resolve | Fade to black, hard cuts | [I] only |
| Retro / pixel | Stepped (low-fps) pop-in, blink | Blink | Stepped cut | Pixel-font pop-in | Hard cuts, palette flash | [I] only; keep blink under 3/s [S hard] |
| Film-trailer generic (white-on-black cards) | Fade in | Hold ~3 s for 10 letters, +1 s per 5 letters [S soft broadcast graphics] | Fade out | Bold sans/serif, black/white | Fade to black between cards | ~5 s between cards, first title ~10 s in [S soft, student]; fade length unsourced |
| Japanese PV (any genre) | Per-character reveal, impact telop, 縦書き for 和 | Long hold (余韻) for catchcopy | Fade or hard cut | 明朝/筆 for 和; thick ゴシック for action | Black dips, flash on hits | Typeface split [S lore booth.pm]; <= 4 cps, <= 13 chars/line [S Netflix JP]; rest [I] |

Recommended use: treat sections 4-5 as hard rules in code, treat the table as defaults to be overwritten by measured values from teaser analysis.
