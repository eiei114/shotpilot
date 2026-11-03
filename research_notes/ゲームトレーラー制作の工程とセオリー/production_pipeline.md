# Professional game trailer production pipeline (phases, artifacts, review, capture, timelines, solo-dev advice)

Coverage note: many primary pages were blocked (403/404) or returned only summaries (Derek Lieu's own guide pages, PC Gamer, GDC Vault, Fortnite playbook). Statements below are limited to what fetched pages actually said. No source was found for Ayzenberg, Mob Scene, Buddha Jones, Trailer Park/AV Squad, Sony/Nintendo/Xbox showcase specs, or Summer Game Fest specs. Vendor-marketing pages are flagged as such.

## 1. Phases and the artifact produced at each step

### Takeaway
The pro pipeline is brief -> concept/script (several options, client picks one) -> shot list + build/tool discovery -> capture -> edit with a back-and-forth to capture -> audio -> finishing/platform specs. Cinematic (CG or engine-cinematic) work adds storyboard, animatic and key art before production. No source gave a single canonical phase list with sign-off names; the closest are Derek Lieu's guide outline and the capture-director (Aftermath) account.

### Cited Findings
- Derek Lieu's "How to Make a Trailer" guide is organised as Pre-production (production checklist, list of assets to request from the developer, SFX library), Concept (is there enough game for a trailer, core/elevator pitch, audience, marketing funnel, trailer type chosen by marketing timing), Capture (inventory game content, assess capture quality, break down visuals and dialogue), Editing (outline, story structure, music-driven structure, cold open, tutorial-style, transitions, pacing, sound design), Polish (eye trace, accents, tension/release), and Finishing (common-flaw check, captions, naming, console platform spec check). This is a translated table of contents only; individual steps are not detailed — [gcores translation of Derek Lieu guide](https://www.gcores.com/articles/151970)
- Lieu's blog timeline posts describe: loose ideas -> "Garbage Cut" of rough visuals -> placing mandatory content and working around it -> editing "in waves" -> finishing and export (titles only, no detail fetched) — [Derek Lieu blog index (2022-2023 posts)](https://derek-lieu-yrd2.squarespace.com/blog/tag/trailer+editing)
- Capture-director workflow at a trailer/capture vendor (Jason, supervising Capture Artists): (1) extensive playthrough of the client's WIP build, plus a "What Does It Feel Like to Play" document for non-players; (2) writers produce several scripts from the client's often thin brief, he adjusts for gameplay accuracy and capture feasibility, client picks one; (3) shot list plus internal kickoff; (4) tool discovery (what the debug tools allow); (5) capture sessions; (6) footage ingested by a Games Coordinator and sent to editorial, with the capture lead giving notes and identifying missing coverage; (7) ambitious V1, then iterate to a client-accepted version — [Aftermath: video game trailer capture director](https://aftermath.site/video-game-trailer-capture-director-maker/)
- Cinematic/CG trailer pre-production artifacts named by a vendor: script, storyboard, basic visualization, concept art, color script, animatic; these form a rough timed prototype that fixes narrative flow before expensive production. Vendor claims it shows cost and needed assets, and lets the studio fit a fixed budget (vendor marketing) — [Room 8 Group](https://room8group.com/news/how-trailer-pre-production-augments-your-game-development-and-marketing-efforts/)
- Digic Studios (vendor) describes: brief/vision -> creative refinement through "numerous discussions" -> early asset creation (sometimes first versions of in-game assets) -> cinematic production with storyboards, animatics, key art. Published 2024-10-01, updated 2025-06-18 — [GamesBeat/Digic](https://gamesbeat.com/production-studio-digic-sheds-light-on-how-game-trailers-are-created/)
- Final delivery artifacts for Steam: .mov/.wmv/.mp4, H.264 + AAC preferred, up to 1920x1080, 30 or 60 fps, 5,000+ kbps, 16:9, 44.1/48 kHz audio; trailer category chosen per trailer (General/Cinematic, Teaser, Gameplay, Interview/Dev Diary); custom thumbnail must be a 1920x1080 frame from the video; the game cannot be released while a trailer is still converting — [Steamworks trailer docs](https://partner.steamgames.com/doc/store/trailer)
- Lieu's guide lists a finishing check for console platform spec compliance (details not retrieved) — [gcores translation](https://www.gcores.com/articles/151970)

### Inferences
- The "source of truth" shifts by stage: client brief/script (words) -> shot list (a capture plan) -> raw capture folders -> edit timeline (NLE project, versioned) -> final masters. For shotpilot, shots.yaml most resembles the shot list plus script, which in the pro flow is a living document revised after V1 notes; the timeline in an NLE is the true source for pacing/music sync, which shotpilot does not own.
- Script options (several scripts, client picks one) and animatic are the cheap-decision steps; shotpilot's previz/still/preview ladder maps onto the animatic role. Choosing among script options and judging "what feels fun" are human judgment; shot-list drafting, coverage checking and debug-camera setup are automatable.

### Gaps
- No primary source for a full AAA agency phase list (Ayzenberg, Mob Scene, Buddha Jones, Trailer Park/AV Squad); the search returned nothing on them.
- Derek Lieu's own guide, pre-production checklist and "assets to request from the developer" were not retrievable (only titles).
- Sony/Nintendo/Xbox/Summer Game Fest delivery specs: not found. Only older, undated Day of the Devs/Future of Play slot specs surfaced via search snippet (1080p, 30-60 s short slot; up to 90 s full trailer, [heyglitch](https://heyglitch.com/futureofplay-direct-returns-this-june-for-e3-2022-submissions-now-open/)); these may be outdated and were not verified by fetch.

## 2. Contents of a trailer brief; first questions editors ask the developer

### Takeaway
Briefs from clients are often thin; the editor/capture lead has to extract core, audience, marketing stage and scope. Lieu's list of up-front questions is the best-sourced one.

### Cited Findings
- Questions/prerequisites in Lieu's Concept stage: is there enough game content for a trailer; what is the core and how does it differ from inspirations and comparable games; who is the audience; where in the marketing timeline is the game (determines trailer type); is an editor being hired and with what scope; narrative- or gameplay-driven; is the concept hard to grasp (so it must be stated literally); are there external quotes/reviews to use; console target needing platform compliance — [gcores translation](https://www.gcores.com/articles/151970)
- Capture vendors say client briefs are often thin and that the client usually provides the WIP build, usually via the publisher, sometimes with the developer — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- A case where the handoff was smooth (2016, Game Developer blog by editor M. Joshua Cauller): client supplied a firm deadline (release date), a clear description of the expansion and its main selling point, downloadable captured footage, key message phrases, full soundtrack and logo art, and a copy of the game; the trailer was then finished in two days — [Game Developer, 2016-06-28](https://gamedeveloper.com/business/how-do-you-hand-off-your-game-for-smooth-trailer-production-)
- Lieu suggests asking developers for custom builds (e.g. different camera angles, switching day to night instantly), per a search-result summary of his interviews (secondary) — [Game Dev Unchained search summary / 80.lv](https://80.lv/articles/game-dev-unchained-making-game-trailers/?amp=1)

### Inferences
- A shotpilot-style brief schema could be: goal/marketing stage, audience, core loop + comparables, platform/format targets (Steam category), length, must-show features, available assets (music, logo, quotes), deadline, build/map availability. Much of this can be asked by an AI intake; "is there enough game" is a human call.

### Gaps
- No complete template of a AAA agency brief was found.
- Lieu's Aug 2023 post "Tips For Working With a Trailer Editor" exists (title listed) but its text was not retrievable.

## 3. How in-game capture is planned and executed; who builds capture builds

### Takeaway
Capture is its own craft (actor, director, camera operator). Teams use the engine's debug camera or custom debug UIs, spawn/god modes, slow motion, sequencer-style replayable scenes, and depth passes; builds come from the developer/publisher, usually unfinished.

### Cited Findings
- Capture is "being an actor, director, and camera operator" (GDC session description, Lieu, "Capturing Great Footage for Game Trailers") — [GDC Vault listing via search](https://gdcvault.com/play/1025079/contactUs) (snippet only, not fetched)
- Tooling ranges from Unreal's built-in debug camera to a custom ImGui built by devs controlling camera, depth of field, character spawning, lighting; vendor staff roles: Capture Techs (play characters), Capture Artists, Games Coordinator (ingest) — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- Blizzard Video (about 50 artists): internal tools such as fly/ghost mode, god mode, infinite resources, spawning characters/items; slow-motion capture in unoptimized builds then sped up in edit; dedicated lighting; map editors to build things not in game; "Audition Entity" lets one artist record each hero in sequence and layer into one scene; Sequencer-like tool replays scenes exactly; depth passes for DoF/effects. Overwatch 2018 6v6 scene took 13 artists and 8 hours, and any timing miss meant re-filming — [Inven Global on Blizzard talk](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers) (article mentions "BlizzCon 2026"; date looks odd, treat as unverified)
- EA's capture lead passes 4K raw capture to editorial for the online edit and final exports; capture artists change/remove UI and camera settings — [LBB on EA game capture team](https://lbbonline.com/news/how-electronic-arts-game-capture-team-strives-for-visceral-authenticity) (via search snippet only; fetch blocked)
- Authenticity: Blizzard notes that showing gear that differed from the shipped game was noticed by fans — [Inven Global](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Lieu says capture scope depends more on game length/size than trailer length (blog post summary) — [Derek Lieu blog index](https://derek-lieu-yrd2.squarespace.com/blog/tag/trailer+editing)
- Steam's own guidance: first trailer mainly gameplay from the player's POV, HUD visible where possible, must communicate when muted — [Steamworks](https://partner.steamgames.com/doc/store/trailer)
- A fork of this in the Fortnite/UEFN marketing playbook exists ([Fortnite playbook](https://www.fortnite.com/developer/developer-marketing-playbook/trailer)) but was blocked, not read.

### Inferences
- Unreal's Sequencer/Movie Render Queue-style deterministic replay is the same idea Blizzard uses to avoid re-filming (replayable scenes). This supports shotpilot's choice of scripted, reproducible shots over live play capture: a shot defined in YAML re-renders identically, removing the "missed timing = redo the whole scene" cost.
- Human-played gameplay beats (player skill, enemy AI timing) are what shotpilot can't script; scripted camera shots over staged scenes are what it can.

### Gaps
- No source on who builds capture builds beyond "client/publisher provides"; no data on cost of building capture-specific builds.
- Photo mode vs free cam trade-offs: no dedicated source.

## 4. Review cycles, locks, and causes of rework

### Takeaway
Reviews are gated on client approval, V1 -> notes -> revisions; the expensive rework is new capture caused by notes, build changes, or schedule/scope changes. Specific round counts and the "picture lock / music lock" order were not found in citable form.

### Cited Findings
- Rework causes from a capture director: client notes after V1 requiring new captures; build changes/updates requiring recapture of existing footage; mid-project schedule/scope changes; build limitations; trailers from vertical slices with scripted elements unlike the final game. Late builds and shortened schedules are the main setbacks; ideal is more time with the build before capture — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- Vendor claims (marketing, low reliability): approvals and unclear briefs are the usual cause of delays; look-dev needs 6-8 weeks of iteration before the first hero shot in CG; 48-72 hour feedback windows recommended; each gate needs client approval before the next — [search result summary of Pixune/Nord/others](https://pixune.com/?p=97859)
- One studio says there should be a constant back and forth between the edit and the game, up to the very end — [search summary of Thunder Cloud](https://thundercloud-studio.com/article/how-a-game-trailer-is-made/) (page blocked; snippet only)
- Version control practice: Lieu names sequences by WIP version and always keeps old versions because a client may reference an old one later — [Derek Lieu blog via search summary](https://derek-lieu-yrd2.squarespace.com/blog/2024/5/5/professional-video-editing-tidbits)
- Blizzard example of review-driven rework: feedback that a scene looked empty added enemies and environment — [Inven Global](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Lieu's translated guide does not mention feedback rounds at all — [gcores](https://www.gcores.com/articles/151970)

### Inferences
- Rework cost clusters at the capture boundary: notes that need new footage are expensive, notes that only re-cut are cheap. This is exactly where shot-level re-render (only the changed shot) pays off, and where a cheap earlier gate (previz/animatic approval before capture) reduces V1 note volume.
- Build churn invalidating all footage matches shotpilot's fingerprint idea: tie takes to build/asset version so only affected shots go stale.

### Gaps
- Typical number of rounds and what is locked when (picture lock, music lock, final mix): not found in any retrieved source.
- Film trailer analogues (Jonny Elwyn's "Inside Professional Trailer Editing") appeared in search results but were not read.

## 5. Timelines and team sizes: indie teaser vs AAA reveal

### Takeaway
Indie editors work in days to weeks with a small supplied-footage handoff; cinematic CG trailers by vendors run months. Evidence is thin and mostly vendor-sourced.

### Cited Findings
- Smooth handoff example: trailer finished in two days when footage, music, logo, messaging, and a firm deadline were supplied (2016, single editor) — [Game Developer](https://gamedeveloper.com/business/how-do-you-hand-off-your-game-for-smooth-trailer-production-)
- Some AAA studios spend a month only capturing; editing can easily take multiple months (interview summary, secondary) — [80.lv/Game Dev Unchained summary](https://80.lv/articles/game-dev-unchained-making-game-trailers/?amp=1)
- Blizzard Video: about 50 artists for trailers, streams, social and in-game videos; project directors, capture artists, graphics team for post — [Inven Global](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Digic: 3-5 minute cinematic trailer takes at least 4-5 months; 40-50 minutes of cutscenes up to a year; third-party trailer production can begin around mid game development — [GamesBeat/Digic, 2024](https://gamesbeat.com/production-studio-digic-sheds-light-on-how-game-trailers-are-created/)
- Vendor guides (low reliability): cinematic trailer 8-16 weeks, full CG 12-24+ weeks; contact studio 3-4 months before submission deadline — [search summary incl. Pixune](https://pixune.com/?p=97859)
- Lieu GDC 2024 talk "Make the Trailer Before the Game: A Marketing First Way to Prototype" advocates making a trailer early as a prototype/self-evaluation tool — [GDC schedule (search summary)](https://schedule.gdconf.com/session/independent-games-summit-make-the-trailer-before-the-game-a-marketing-first-way-to-prototype/900472) (page blocked)

### Inferences
- For the 7-day / 30-60 s shotpilot teaser, the closest analogue is the indie "supplied assets, days" end; the capture/lookdev heavy AAA timeline is what the tool aims to compress.

### Gaps
- No timeline or team-size data for AAA reveal trailers by Ayzenberg/Mob Scene/etc., and no Reddit/forum data collected.

## 6. What pros say indie/solo devs do wrong; recommended solo workflow

### Takeaway
Consistent advice: show gameplay immediately, no slow intro/logos, show the core loop with HUD, avoid copying slow AAA cinematic trailers, and ship a simple trailer early rather than over-polish.

### Cited Findings
- Lieu mistakes relayed by Simon Carless (GameDiscoverCo, per WN Hub 2022-09-22): slow intros, delayed gameplay, story-first openings, copying AAA trailers; from another Lieu talk: unfamiliar logos, repetitive content, poor music choices — [WN Hub](https://wnhub.io/news/analytics/item-237)
- Lieu: avoid logos at the start; attention wanes after about five seconds; "do audio first" (secondary summary) — [search summary of Lieu interviews](https://80.lv/articles/game-dev-unchained-making-game-trailers/?amp=1)
- "Simplest trailer" workflow (Game Developer): publish the Steam page early; make a minimum viable trailer; capture raw gameplay and trim into core-loop clips; each shot about 3-5 s; show one full loop, variety, avoid repeating a shot more than ~3 times; HUD on; length roughly 90-180 s; do not over-polish title cards/transitions/VO; MVP trailer is a mistake once you already have an audience — [Game Developer: the simplest trailer for your Steam page](https://www.gamedeveloper.com/business/the-simplest-trailer-to-make-for-your-steam-page)
- Chris Zukowski (secondary summaries of his talk): launching without a trailer and having no gameplay in the trailer are mistakes; puzzle-game trailer that did not explain the mechanics had to be redone; if budget is limited, spend on capsule art before the trailer — [search results summarising Zukowski](https://castbox.fm/episode/How-to-NOT-screw-up-your-indie-game-launch.-30-mistakes-in-30-minutes.-|-Chris-Zukowski-id2135356-id720114603)
- Valve: first two valid trailers appear before screenshots; trailer must work muted — [Steamworks](https://partner.steamgames.com/doc/store/trailer)
- Capture-side lesson: Blizzard's "looks like the game but better" standard; showing non-shipping content gets noticed — [Inven Global](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)

### Inferences
- Important tension for shotpilot: a 30-60 s cinematic **teaser** (hook -> establish -> escalate -> title) is the AAA-style reveal that Lieu/Zukowski warn indies against for a Steam page's first trailer, which they say should be gameplay-forward with HUD and no logo-first opening. Steam does have a separate "Teaser" category, so teasers are legitimate, but the evidence favours pairing with a gameplay trailer; the title/logo end card should come last, not first.
- Automatable: shot-list proposal, coverage vs core-loop checks, Steam spec export (codec, 1080p, fps, audio rate), caption/thumbnail frame selection. Human judgment: hook choice, what "feels fun", music taste, final cut feel, truthfulness vs actual game.

### Gaps
- Primary Zukowski and Lieu texts were not read directly (summaries only); no solo-dev-specific case studies with timelines.
