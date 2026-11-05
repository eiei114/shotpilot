# Motion of text, titles and transitions: ACTION and STRATEGY/SIMULATION teasers (measured)

Scope: motion and timing only. No wording or story is copied. Frames were never copied into the repo (scratch strips live outside it).

## 1. Method

- Videos: `research_data/teasers/videos/<id>.mp4`, 11 teasers (6 action incl. 2 Japanese, 5 strategy/simulation incl. 2 Japanese).
- Pass 1: overview strip per teaser at 1 fps (10 columns, 200 px) to locate every designed text moment (in-game HUD ignored).
- Pass 2: dense strips only around text moments: 4-5 fps for long holds, 8-12 fps for fades/type-on, 24 fps for slams, glitches and flashes (so one frame = 0.042 s at 24 fps, 0.083 s at 12 fps, 0.125 s at 8 fps). Timestamps are read off the stamped frames, so durations carry about +/- one frame of the strip's fps.
- Pass 3: `luma.sh` (12-24 fps) for fade ramps (YMAX/YAVG), black gaps and hard cuts (diff spikes), plus a scripted scan of all videos at 24 fps for brightness spikes (candidate flashes), then visual confirmation.
- "unclear" = not resolvable at the fps I looked at. Where I only saw the 1 fps overview, the row says "overview only".
- Size is share of frame height of the text block (rough).

## 2. Per teaser

Abbreviations: in = entrance, hold, out = exit. Times in seconds in the source video.

### 2.1 ACTION

#### yt-luWunttkCgY, Dear Passengers (Official 'Another Friendslop Game' Teaser), 36 s
Source: https://www.youtube.com/watch?v=luWunttkCgY. Cartoony physics comedy.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 33.33 | title logo, chunky custom 3D-bevel display type, yellow/white with dark outline, upper centre, about 25% height, two lines | hard cut, 0 frames (it is absent at 33.25 and fully there at 33.33 at 12 fps) | about 3.5 s to the end of the video, static in screen space while the picture keeps moving (camera swings, objects fly through) | none within the file (video ends, no fade) |

Distinctive: no designed text at all before the title (in-game name labels only). 33 s of pure picture, then a logo that hard cuts onto a still-moving shot with zero text animation. Cuts between shots are hard cuts throughout; no black, no flashes.

#### yt-0lEO0u1_ooY, Moylo (classic cartoony teaser), 32 s
Source: https://www.youtube.com/watch?v=0lEO0u1_ooY. Hand-drawn 2D, mostly white-paper backgrounds.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 29.08-29.9 | title logo ("Moylo", heavy rounded red display, black outline) plus "Wishlist On Steam" (small bold sans, with Steam glyph), centre-left, logo about 14% height | hard cut from a white sketch frame to a yellow background at 29.08; a drawn cartoon smoke puff fills the logo area (29.08-29.25), the logo shows through the puff (29.25-29.5), ring-shaped smoke particles fly outward and vanish by about 29.9: smoke/particle reveal, about 0.8 s, ease-out | about 2.6 s, static (to the end) | none within the file |

Transitions measured: shot to title is a hard cut with the puff covering it; the preceding shot ends with 1 black frame (28.33) then a white frame (28.42): black-to-white strobe of about 1-2 frames. White-paper to dark-grid scene cuts occur at 10.67, 14.33, 23.38, 28.38 (YAVG jumps of about +160), but they are cuts, not flashes.

Distinctive: the only designed text is the end title and it uses a diegetic, in-art effect (smoke puff) rather than a text effect.

#### yt-h8AcT7N-OoU, Defilade (Official Announcement Trailer), 75 s
Source: https://www.youtube.com/watch?v=h8AcT7N-OoU. Top-down tactical war game.

About 14 feature callouts over gameplay (overview: about 0.1, 2.5, 8, 10, 13, 15, 17, 20, 27, 32.9, 38, 41, 43, 46 s) plus a CTA and two title cards. All share one motion language.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0.25-1.75 "3 HINDS DOWNED!" | feature callout, lower left, bold wide uppercase grotesque, white, about 3% height, with a small red tick/cursor at the left, a tiny second line beneath | type-on per character, 0.25-1.0 s = 15 characters in 0.75 s = about 20 chars/s (about 0.05 s per character), with a red block cursor following; ease: linear | complete at 1.0, then 0.65 s static | hard cut with the picture at 1.75 (cut to a white snow scene) |
| 32.85-35.4+ "GRAD ROCKETS & VARIOUS KINDS OF ARTILLERY" (38 characters, wraps to 3 lines) | callout, lower left, same style | type-on, 32.85 to 35.0 = 2.15 s = about 18 chars/s, cursor follows | about 1.0 s after completion | cut with the shot (36.0 shows a new scene) |
| 46.5-47.9 "HIND DOWN" | diegetic-looking callout: label with thin leader line to a helicopter, tiny caption beneath | type-on over about 0.5 s (HIND, HIND DO, HIND DOWN at 8 fps), the leader line is drawn with it (UI-like build) | about 1.2 s, then fades quickly as the explosion lands (47.7) | text fades under a white-orange blast; cut to white-sand frame 48.25 |
| 48.25-52.0 title "DEFILADE" | title, centre, wide-tracked geometric sans caps, white on near-black, about 7% height, tiny corner brackets in the frame corners | hard cut to black at 48.375 (one frame of near-white at 48.25), then type-on 48.5-49.0, 9 letters in about 0.45 s = about 0.05 s/letter, with a grey leading letter and red cursor | letters complete at 49.0; a thin red rule line draws under it 49.4-49.6; "WISHLIST ON" types on 50.1-50.4; Steam glyph appears 50.4; platform glyph row by 51; about 3 s total, static | cut to picture at 52.0 |
| 68.0-69.4 "WISHLIST NOW ON STEAM" | CTA overlay on gameplay, lower right, same wide caps, small (about 3%), tiny label above, a vertical tick at the right | present from about 63.5 (overview), no clear in (unclear) | about 5 s on the shot | cut to black at 69.4 |
| 69.5-72.0 title again | same title card | cut to black 69.375-69.5 (about 0.1 s), type-on 69.6-69.85, 0.25 s (faster than the first time), tagline and Steam glyph type on 70.3-71.0 | about 2.4 s | hard cut to a white UI-screenshot frame at 72.0 |

Distinctive: one consistent "terminal / tactical readout" language: type-on with block cursor at about 18-20 chars/s, hard cuts for everything else, no fades. The title is a type-on after a cut to black, not a logo.

#### yt-olOTdoW6XsQ, Tomb Raider: Catalyst (Teaser Trailer | PS5), 90 s
Source: https://www.youtube.com/watch?v=olOTdoW6XsQ. Cinematic, dark, platform-holder wrapper.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 1-6 | rating box, lower right, about 9% height | overview only, unclear | about 5 s | cut |
| 76.75-82.4 | title logo (gold serif-sans caps with ornament underline, "CATALYST" small tracked), centre, about 30% of the picture height | fade-in 76.75-77.5 (about 0.75 s) over the picture; ease-out feel (most of the opacity arrives in the first 0.4 s, 76.9-77.25) | 77.5-82.4 = about 4.9 s; logo static, picture nearly static with a very slow drift (slow push unclear) | hard cut to black at 82.42-82.5 (single frame diff 46) |
| 83.0-88.9 "2027" plus legal line | date, centre, small white sans (about 3%) | fade up from black 83.0-84.0 (about 1.0 s, linear-looking YMAX ramp 84 to 222 in 12 frames at 12 fps) | about 5 s | cut to platform-holder blue card at 89.0 |

Transitions: the body is dark hard cuts, with 5 explicit black beats (17.5-18.0, 27.0-28.1, 40.25-40.5, 55.0-55.5, 63.1-64.8), each entered and left with a hard cut (diff 14-52, no fade). Black gaps 0.5-1.7 s. No white flashes.

Distinctive: the title is a slow fade over the last picture and then black, then date; everything else is hard cut plus black beats.

#### yt-ggvPJeCHbdM, Midnight Yammy (Japanese, 8-bit action puzzle teaser), 34 s
Source: https://www.youtube.com/watch?v=ggvPJeCHbdM. Pixel-art game, neon-and-black cards.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0-3.4 | card on black: two lines of Japanese (heavy rounded gothic, warm yellow with orange glow, about 6% height, centred) and a small white English line (about 2%) | per-line glyph scramble/glitch build: columns of mirrored/garbled fragments appear and resolve from 0.0 to about 0.75 s (about 0.75 s); the English line pops in on 0.67-0.75 | about 2.6 s static | glitch: vertical-slice dropouts and the neon logo's strokes interleave with the card from 3.45 to about 4.1 (about 0.65 s), colour shifts from yellow to magenta |
| 3.45-7.0 | title logo (neon outline pixel display, magenta/pink, two lines with an L-shaped underline), centre, about 30% height | glitch-transform from the card, 3.45-4.1 (about 0.65 s) | to 7.0 (about 3 s); logo colour alternates magenta and pink every 0.17-0.25 s (about 4-5 Hz hue flicker, low luminance change); no scale | cut to gameplay at 7.0 |
| 28.7-31.5 | title logo again over gameplay | dissolve: logo fades up 28.7-29.1 (about 0.4 s) while the picture fades to black beneath (28.75-29.12) | about 2.4 s, same neon hue alternation | cross-dissolve with horizontal smear into the end card, 31.5-31.75 (about 0.25 s) |
| 31.75-34 | end card on black: JP two lines (yellow, glow, about 5%) plus a small English line | dissolve in as above | about 2.3 s to the end, static | none |

Distinctive: the text is "glitch/scramble" language matching the 8-bit theme, not fades; the same neon-yellow card style is reused for CTA. Transitions between gameplay stages are hard cuts, 3 black frames at the first cut.

#### yt-zH6ymTkaOso, SHIKA-Q (Japanese official teaser, 40 s)
Source: https://www.youtube.com/watch?v=zH6ymTkaOso. Fast competitive puzzle game, blue/red energy look.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0-2 | small studio mark on black | overview only | about 2 s | cut |
| 2.1-5.0 | tagline: outlined diamond glyph, then "I'M SPECIAL." (condensed bold caps, white, about 3%) | glyph fades in 2.08-2.17, tagline fades in 2.58-2.67 (about 0.1-0.2 s); background figures fade up 2.7-3.5 (about 0.8 s) | about 2.3 s | cut to black (5.0-5.125, 4 frames at 24 fps) |
| 5.17-8.13 | title logo (custom spiky sticker logo, magenta with cyan outline and sparks), centre, about 40% height | slam: scale about 3x at 5.167, about 1.3x at 5.208, settled at 5.25 = 3 frames = 0.125 s, from black, ease-out snap, no visible overshoot | about 2.9 s, static with sparkle glints | hard cut into the next slam at 8.17 |
| 8.17-11.7 | "PUZZLE GAME" (italic condensed display caps, cream, about 22%) plus a yellow JP subtitle line above (about 6%) | slam with horizontal motion blur: huge at 8.167, 8.208, 8.25, 8.29, settled by 8.33 = about 0.17 s; subtitle appears with the settle | about 3.4 s static | hard cut to gameplay at 11.75 |
| 13.08-16.17 callout 1 (JP, yellow bold italic, dark outline/gradient, about 9%, upper centre) | callout | slam with horizontal streaks: 13.083 huge and smeared, 13.125 intermediate, 13.17 settled = about 0.1 s | about 3.0 s | hard cut with the picture at 16.17 |
| 16.5-about 20.2 callout 2 (2 lines, about 12%) | callout | slam: 16.5 and 16.54 huge/smeared, settled at 16.58-16.62 = about 0.125 s | about 3.4 s | cut at about 20.2 (white burst) |
| about 24, 27, 33 | callouts 3-5 ("rank match", "high score", "30+ songs"), same style | overview only; same slam assumed (inferred) | about 3 s each | cut |
| 36-40 | end card: logo (smaller, about 22%), release season text and platform logos | cut (overview only) | about 4 s | end of video |

Brightness bursts in the fastest passage (16-21 s, designed picture effects): cyan-white burst 16.2-16.45, white burst 20.25-20.5, plus the slam frames; about 2 per 5 s, 0.1 s-0.25 s long each (roughly 0.4 per second).

Distinctive: heavy text; every card/callout slams in within 0.1-0.17 s and leaves with a hard cut with the shot. Callouts recur every 3.4 s (13.1, 16.5, about 22.9, 27, 33).

### 2.2 STRATEGY / SIMULATION

#### yt-fGgsNvxaA04, Miniopolis (Announcement Trailer), 89 s
Source: https://www.youtube.com/watch?v=fGgsNvxaA04. City builder.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 76.7-89 | title: "miniopolis", large 3D-looking, lowercase, grey letters sitting in the scene (diegetic; the lettering is clipped by the frame sides), about 30% height | the shot before dims to black 76.2-76.6 (about 0.4 s fade to black), then a fade from black with rack focus (blurred at 76.8, sharp by 77.6) about 0.9 s: blur-to-focus plus fade | about 11 s on screen; letters static; camera movement is a slow drift (unclear) | none in the file |
| 78.6-89 | CTA "WISHLIST NOW", bottom centre, bold white sans, about 4% height, Steam mark bottom left, "Demo Coming Soon" bottom right (about 1.5%) | fade in 78.6-79.6 (about 1.0 s, linear) | about 9 s, then fade out near the end (88.0-89.0, about 1 s) | end |

Transitions: dissolve-like soft transitions between scenes (day/night swaps at 74.4-75.2 alternating fast), a dip through black 76.2-76.7 (about 0.5 s) before the title. No text elsewhere (the rest is in-game build UI).

Distinctive: text is rare (title plus CTA only). The title uses rack focus, a camera-style effect, not a graphic effect.

#### yt-UpH6HB-Km_0, Neighborhoods (Reveal Trailer), 59 s
Source: https://www.youtube.com/watch?v=UpH6HB-Km_0. Cosy city builder.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0.25-2.8 | title logo (custom hand-lettered logo with white sticker outline, green/red), centre, about 18% height | fade in 0.25-1.0 (about 0.75 s, near-linear with a slight ease-in) over the opening shot; no scale | about 1.4 s static (1.0-2.4) | fade out 2.45-2.8 (about 0.4 s) while the picture cross-dissolves into the next shot (2.7-3.2, double exposure) |
| 56.7-60 end card: logo (about 18%), "Wishlist Now!" (hand-drawn sans, white with outline, about 5%), URL (about 3%), publisher mark (about 4%) | end card | shot fades to black 56.0-56.4 (0.4 s), black 56.46-56.69 (0.23 s), then hard cut to the card at 56.7: all elements appear at once over a new shot, no text animation | about 3.3 s, static | end |

Distinctive: the opening logo fades in and out like a slide (1.4 s), and the end card has no animation at all.

#### yt--uDHQlmGstM, Winning Post 10 2026 (Japanese, horse-racing sim), 82 s
Source: https://www.youtube.com/watch?v=-uDHQlmGstM. The most text-heavy of the strategy set.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0-2.5 | rating box | overview only | 2.5 s | cut to black |
| 2.5-5.8 | date "1993.12.26", centre, thin grey-white sans, about 4% | linear fade in 2.5-3.9 (about 1.4 s) on black; the archival picture fades up behind 4.0-4.8 (about 0.8 s) | about 1.8 s over the picture | cut/fade to black 5.8 |
| 6-8 | publisher logo on black | overview only | 2 s | cut |
| 13.0-15.0, 20-22, 25-27, 30-32 "it is X" sentences (4 in a row, about 5 s apart) | card on white: black heavy italic Japanese gothic, centred, about 9% | blur-to-focus: the shot whips to white at 12.58 (yellow smear), text is a heavy horizontal motion blur at 12.67-12.75, sharp at 12.83 = about 0.2 s, snap; a one-frame blur pulse again at about 13.4-13.5 | about 2.1 s static | vignette closes on the card, text blurs, whip into the next shot, 15.0-15.17 (about 0.17 s) |
| 17.25-19.7 race-name tag (top left, small, about 3%) and horse-name overlay (lower right, heavy italic white with shadow, about 8%) | overlay on picture | tag appears with the shot after a white flash 17.04-17.17; name: blurred and drifting at 18.33-18.42, sharp at 18.5 (about 0.17 s) | about 1.2 s, static | ends at the whip into the next white card (19.7-19.9) |
| 22-25, 27-30 | same overlays | overview only, same pattern assumed (inferred) | about 2 s | cut |
| 46.3-49.0 "その蹄跡を" card (2 lines with ruby, bold italic, about 14%) | card | same blur-to-focus 46.33-46.5 (about 0.17 s) on white; at 47.25-47.4 the background flips white to black while the text blurs and re-focuses (about 0.15 s) | white 0.75 s, black 1.5 s | 48.9-49.0 vignette, 49.08 blown to white, white 49.12-49.25 (about 0.15 s flash, 3-4 frames at 24 fps) into the title |
| 49.3-51.6 title | gold 3D logo, large | the 3D logo flies through the camera (fast scale-down from huge, 49.375-50.0, about 0.6 s), shine/light streak, settles 50.0-50.65; then a huge "2026" slams in from scale about 3x at 50.7 and shrinks until 51.6 (about 0.9 s); hold about 0.6 s | motion: a slow scale-down and light sweep through the hold | whip-blur to white 51.6-51.8 (about 0.15 s) |
| 51.9-54 clean title card on white | logo assembling with partial wipes 51.875-52.4 (about 0.5 s) | hold about 1.1 s | fade to black 53.5-54.0 (about 0.5 s, dip), then up from black into the green end card over about 0.3 s |
| 54-59 end card (logo, date line, platform row, small) | end card | fade up from black about 0.3 s | about 5 s | cut |
| 59-77 | info slides (bonus lists, small dense text) | hard cuts between slides (overview) | 5-6 s each | cut |
| 77-81 | publisher logo on white | overview only | about 4 s | end |

Distinctive: the most Japanese-style teaser: white-background text cards with blurred-to-sharp italic type and speed-line whips between cards and footage; hard flashes to white; many static slides after the title.

#### yt-QW_7oiMLM6o, Frostpunk 2: The Surge (Date Reveal), 60 s
Source: https://www.youtube.com/watch?v=QW_7oiMLM6o. Grim, cinematic.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0.5-3.8+ "11 BIT STUDIOS presents" | very small tracked caps (about 1.5%), centre | linear fade in 0.5-1.0 (about 0.5 s) on black; the opening picture fades up under it 2.1-3.3 (about 1.2 s) | text visible to about 3.8, then unclear | unclear (fades with the shot) |
| 37.7-39.0 "NEW EXPANSION" | card on dark smoke: condensed caps, grey-blue metallic, about 4%, centre | hard cut from the flag shot to the smoke card at 37.75 (text already at about 40% opacity), slow brightening 37.8-38.4 (about 0.6 s) | 1.2 s total; very faint motion in the smoke only | hard cut to the next picture at 39.1 |
| 43.0 "COMING", 46.0 "DECEMBER 14" | same smoke-card style | overview only | about 1 s each | cut |
| 48.55 shot to black | cut to black | black 48.55-49.75 (about 1.2 s) | | |
| 49.75-55 title "FROSTPUNK 2 / THE SURGE" | centre-left, about 12% height, light grey caps, subtitle small | fade in 49.75-50.2 (about 0.5 s) as the picture fades up from black 49.8-50.3 | about 4.8 s, static; a lightning flicker in the picture only | hard cut to the edition card at 55.0 |
| 50.7-50.95 date + platform glyphs | small (about 2%) | fade in about 0.3 s | to 55 | cut |
| 55-60 edition card (bundle art, logos) | cut, static | 5 s | end |

Distinctive: text arrives on smoke cards at a slow ease, the title fades from black after a long black beat.

#### yt-iCPdNuagTl8, Capsule Mates (Japanese, monster-raising board game), 75 s
Source: https://www.youtube.com/watch?v=iCPdNuagTl8. Dark fantasy, bronze type.

| time | kind | in | hold | out / what follows |
|---|---|---|---|---|
| 0-4.6 | title logo (bronze metallic serif/sans caps with JP katakana line, light glow behind), centre, about 25% | video starts on the logo (opening in unclear) | about 4.6 s | dissolve into the text over 4.6-5.3 (about 0.7 s) |
| 5.3-7.4 "walk and raise," | tagline line, heavy bronze Mincho-like type with dark outline, about 9%, centre | cross-dissolve 4.6-5.3 (about 0.7 s) | about 2.1 s, static | whole scene fades to black 7.4-7.7 (about 0.3 s) |
| about 8.0-10.4 "fight on the board." | same style | probably the same dissolve (not checked, inferred) | about 2.4 s | fades with the picture 10.3-10.7 (about 0.4 s), black 10.7-11.05 (0.35 s), picture fades up 11.05-11.5 (0.45 s) |
| 68.2-75 end logo | same logo | picture fades to black 67.2-67.8 (0.6 s), black 67.8-68.15, logo fades in 68.2-68.9 (about 0.7 s) over a dim background that also brightens | about 6 s to the end | end |

Transitions: dip through black about 0.35 s, with 0.3-0.6 s fades each side, every 5-10 s (black at 19, 28, 36, 43, 52, 60); in-picture bursts of white light (about 22, 39-40, 44) are artwork, not transitions.

Distinctive: slow, even-paced fade language, a slower fade than the Western teasers (0.3-0.7 s), and few text moments (2 taglines + logo twice).

## 3. Synthesis by genre

### 3.1 ACTION (6 teasers)
- Frequency of designed text: bimodal. Western: Dear Passengers 1 (title only), Moylo 1 (title/CTA), Tomb Raider 2 (title, date), Defilade about 14 callouts. Japanese: Midnight Yammy 4, SHIKA-Q 10.
- Entrance (all measured): cut 0 s (Dear Passengers); type-on 0.25-0.75 s for about 9-15 characters, about 18-20 chars/s (Defilade); slam 0.10-0.17 s (3-4 frames at 24 fps, from about 2-3x scale, with horizontal motion blur, no visible overshoot; SHIKA-Q); glitch/scramble 0.65-0.75 s (Midnight Yammy); smoke-puff reveal about 0.8 s (Moylo); fade 0.75 s (Tomb Raider); fly-through plus slam about 0.9 s (Winning Post, sports sim but same family).
- Title in: 0 s (cut) / 0.125 s (slam) / 0.45 s (type-on) / 0.65-0.8 s (glitch, puff, fade). Median about 0.5-0.65 s; range 0 to 0.9 s. The common point: no 0.3 s opacity fade. If a fade is used, it is 0.75 s over a picture.
- Hold: titles 2.4-4.9 s (median about 3.0 s); callouts 1.0-3.4 s (Defilade type-on callouts end 0.65-1.0 s after completion; SHIKA-Q slams stay 3.0-3.4 s). Motion during hold: none (static) in 5 of 6; neon hue flicker at 4-5 Hz (Midnight Yammy); small sparkle (SHIKA-Q). Picture behind a title may move (Dear Passengers).
- Exit: hard cut with the shot in 8 of 9 measured callouts/titles (Dear Passengers, Moylo, Defilade 3/3, SHIKA-Q 3/3); glitch swap 0.65 s; cut to black (Tomb Raider 0.04 s, Defilade 0.1 s). A fade-out is rare in this genre.
- Transitions: hard cuts, hard cuts to black with 0.1-1.7 s black beats (Tomb Raider, Defilade), one-frame near-white flash before a black title card (Defilade at 48.25), 3-4 frame white/black bursts around slams (SHIKA-Q 5.0-5.125 black then slam), puff cover (Moylo). White flashes in the fastest passages are about 0.4 per second at most (SHIKA-Q 16-21 s: 2 bursts of 0.1-0.25 s in 5 s; Defilade explosion passages 1-frame flashes at 9.1, 14.8, 15.96, 16.0, 48.2, about 0.3/s). I never saw 3 or more flashes in one second.
- Overlay placement: lower left for callouts (Defilade, 3% height, caps, wide tracking, white, on a soft gradient); lower right for CTA; centred 9-12% for Japanese callouts; centred 30-40% for titles.
- Camera on title shots: static in screen space; the picture may continue moving (Dear Passengers) or be nearly still (Tomb Raider, slow drift/push unclear).
- Japanese vs Western action: Japanese teasers carry 4-10 text elements in 35-40 s (a callout every about 3-3.4 s) with a heavy, glowing, outlined display type (about 6-12% height) and strong entrance effects (scramble, slam with blur). Western action uses 0-2 text elements, or a calm tactical readout (type-on). Japanese exit = cut; Western exit = cut.

### 3.2 STRATEGY / SIMULATION (5 teasers)
- Frequency: low and calm. Western: Miniopolis 2 (title, CTA), Neighborhoods 2 (title, end card), Frostpunk 2 about 6 (studio line, 3 smoke cards, title, date). Japanese: Winning Post about 12 + slides, Capsule Mates 4 (logo, 2 taglines, logo).
- Entrance (measured): fade 0.4-0.75 s (Neighborhoods logo 0.75 s, Frostpunk title 0.5 s, 11 bit 0.5 s, date 1.4 s on black, CTA 1.0 s); rack-focus-plus-fade 0.9 s (Miniopolis); cross-dissolve 0.7 s (Capsule Mates); slow ease-in brighten 0.6 s on a smoke card (Frostpunk); cut with no animation for end cards (Neighborhoods, Frostpunk edition card); Winning Post (JP): blur-to-focus 0.17-0.2 s on white cards and 0.5-0.9 s fly-through for the title.
- Title in: 0.5-0.9 s fade (median about 0.75 s) for Western, 0.7 s dissolve for Capsule Mates, 0.9 s fly-through for Winning Post. Always over a picture or from black.
- Hold: titles 1.4-12 s (Neighborhoods 1.4 s open, 3.3 s end; Frostpunk 4.8 s; Capsule Mates 4.6 s open and about 6 s end; Miniopolis 11 s); body cards 1.2-2.4 s (smoke cards 1.2 s, taglines 2.1-2.4 s, white cards 2.1 s). Motion during hold: none (static) everywhere; picture behind drifts slowly.
- Exit: fade 0.3-0.5 s, usually together with the picture (a fade-through-black: Capsule Mates 0.3-0.4 s fade, 0.35 s black, 0.45 s fade up; Neighborhoods 0.4 s logo fade into a 0.5 s dissolve; Miniopolis 0.4 s fade, 0.1-0.2 s black); end cards cut in (Neighborhoods 0.4 s fade, 0.23 s black, cut; Frostpunk cut).
- Transitions: dip through black 0.3-0.5 s (with 0.3-0.6 s fades), cross-dissolve 0.5 s, hard cut (end cards, smoke cards), black beats 1.2 s (Frostpunk). White flashes: none measured in the Western strategy teasers; Winning Post has 4 white events in 10 s (11.1, 12.6, 15.3, 17.0), each 0.1-0.15 s (about 0.4/s), plus one 0.15 s flash into the title.
- Overlay placement: CTA bottom centre (Miniopolis, about 4%); title centre or upper centre; end card text 3-5% below a logo; tiny legal text 1.5% at the bottom.
- Camera on title: static or very slow drift over a live shot (Miniopolis 3D letters in-scene; Neighborhoods drifts slightly; Frostpunk static).
- Japanese vs Western strategy: Winning Post shows how Japanese teasers differ: more cards, white-background cards with heavy italic type, speed-line whips and flash cuts, post-title information slides. Capsule Mates differs from Western ones only by tagline cards in a heavy bronze Mincho type and slightly longer holds (6-7 s end logo).

### 3.3 Contrast action vs strategy
- Action: text lands (cut, slam 0.1-0.17 s, type-on at 20 chars/s, glitch) and leaves with a cut. Strategy: text fades (0.4-0.75 s in, 0.3-0.5 s out) and often passes through black. Action holds are 1-3.4 s per callout, strategy cards 1.2-2.4 s but titles are held much longer (up to 11 s).
- Action transitions: hard cuts and flash/black beats. Strategy: dissolves and dips through black. Time between text moments: action callouts every 3.4 s (SHIKA-Q) or 2-3 s (Defilade); strategy cards 5-30 s apart.

## 4. Proposed motion profiles (plain values; only measured values, inferred ones are marked)

Values in seconds. "cut" = alpha 0 or 1 in one frame.

### ACTION (Western-leaning: tactical readout)
- text_in (callout / overlay): type-on, 0.05 s per character (about 20 chars/s), capped at 1.0 s total; if too long, speed up rather than lengthen. Block cursor optional (UI-like build).
- text_hold: until the shot ends, minimum 0.65 s after completion (measured 0.65-1.0 s).
- text_out: cut with the shot (0 s).
- card_in (card on black): cut, or glitch/scramble 0.7 s for retro/8-bit looks.
- title_in: type-on (0.05 s per letter) after a 0.1-0.15 s cut to black; or cut (0 s); alternative slam 0.125 s (inferred: for kinetic looks).
- title_hold_motion: none (static); optional 4 Hz hue flicker for neon looks (measured only on one teaser).
- title_hold: 2.4-3.5 s (median 3.0).
- title_out: cut (0 s). 
- transition_default: cut.
- transition_accent: flash 0.1-0.15 s (3-4 frames at 24 fps); dip to black 0.1-0.5 s; never more than 1 flash per 2 s (measured about 0.4/s peak).
- overlay placement: lower left, 3% height, wide-tracked caps, white; CTA lower right.

### ACTION (Japanese / kinetic, high text density)
- text_in: slam 0.125 s (3 frames at 24 fps from about 2.5x scale with horizontal motion blur, ease-out snap, no overshoot).
- text_hold: 3.0-3.4 s static; one callout every 3.4 s.
- text_out: cut with the shot (0 s).
- card_in: glitch scramble 0.75 s.
- title_in: slam 0.125 s after 0.125-0.17 s of black.
- title_hold_motion: none or sparkle/glint (inferred as optional).
- title_out: cut into the next slam.
- transition_default: cut.
- transition_accent: black 0.125-0.17 s before a title; white flash 0.1-0.25 s inside the picture (about 0.4/s peak).
- overlay placement: centre or upper centre, 9-12% height.

### STRATEGY / SIMULATION (calm, builder, cinematic)
- text_in: fade 0.5-0.75 s (linear to ease-in-out); for date/small text on black 1.0-1.4 s (linear).
- text_hold: cards 1.2-2.4 s; static (no motion).
- text_out: fade 0.3-0.5 s, together with the picture.
- card_in: fade 0.6 s or slow brighten 0.6 s; hard cut for the card itself when it is the end card.
- title_in: fade 0.75 s (Neighborhoods) / 0.5 s from black (Frostpunk) / 0.9 s blur-to-focus (Miniopolis, inferred as an option for looks with a camera feel).
- title_hold: 1.4 s (open, then dissolve) to 5 s; end card 3.3-6 s.
- title_hold_motion: none.
- title_out: fade 0.4 s with a 0.5 s dissolve; end card: cut in after 0.4 s fade to black plus 0.23 s black.
- transition_default: dip through black 0.3-0.5 s (0.3-0.6 s fades with 0.35 s black), or 0.5 s dissolve (not possible without changing length; use dip).
- transition_accent: black beat 1.2 s before the title; white flash 0.1-0.15 s only as a rare accent (Japanese sports sim only; not in the Western ones).
- overlay placement: CTA bottom centre 4%; legal 1.5%; end-card stack: logo, then CTA at 5%, then URL at 3%.

### Notes for implementation (inferred)
- Moves cheaply expressible with ffmpeg today: cut, alpha fade 0.3-1.0 s, dip through black, white flash 0.1-0.15 s, type-on (per-character alpha or drawtext reveal at 20 chars/s), slam (animated scale 2.5x to 1x over 3-4 frames), horizontal blur at the start of the slam (boxblur with a time-varying radius), hold static. Glitch scramble, smoke puffs, 3D fly-through and rack focus need assets or approximation (blur-to-focus is a blurring ramp 0.9 s).
- Photosensitivity: the measured peaks stay under about 0.4 flashes per second; a limit of 2 per second (well below 3 per second) is safely inside what the corpus does.
