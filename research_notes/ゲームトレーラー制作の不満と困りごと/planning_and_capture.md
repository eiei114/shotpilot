# Indie/small-team developers' complaints about planning and capturing game trailer footage (English-language)

Method note: about 15 tool calls. Reddit could not be fetched directly (search only returned third-party mirror snapshots and no usable first-hand quotes), and several pages returned 403/429 (ModDB, Medium Black Shell, Devolver blog). First-hand evidence below is therefore thin: a handful of devlogs and forum posts. Most of what was found is advice. Frequency can only be judged roughly, and "how strongly felt" is mostly inferred from tone.

## What is hardest about deciding WHAT to show and how to structure a 30-90 s teaser

### Takeaway
Direct first-hand complaints were rare. The documented difficulties are: (a) the game has little visible action (visual novel, early build), (b) real gameplay is too chaotic to read, (c) footage fails to convey the game's experience. Advice sources agree that the hook should be gameplay in the first seconds and that trailers stay under about 90 s. No reliable "I don't know what my hook is" quote was found.

### Cited Findings
- First-hand (itch.io forum, about 4 years old, undated): a visual novel dev says "The trouble with trailers for visual novels is that there isn't a lot of action to show", that the trailer "is boring without the music I wrote for it", and that with only 45 minutes of game done they "had to fabricate a few of the later scenes I was planning." Mild, resigned tone. — [itch.io post](https://itch.io/post/5657081)
- First-hand (Jetboard Joust-type devlog, "Trailer Trash pt 2"): "'real' gameplay footage proved too confusing", so the dev built artificial levels with one enemy at a time to make the trailer readable. Sifting footage was a "time-consuming process". — [itch.io devlog](https://itch.io/devlog/107312/trailer-trash-pt-2.amp)
- Derek Lieu (professional trailer editor, first-hand about his own work, 2023): planning is "the most difficult thing to solve, because every game has its own unique needs"; trailer "starts with finding the hook of your game"; footage can fail to represent "the experience of the game"; opening shots that are not gameplay "need to at least tell part of the story." — [Derek Lieu blog](https://derek-lieu-yrd2.squarespace.com/blog/2023/3/11/i-made-the-worst-firewatch-trailer-ever)
- Advice, summarising Lieu: gameplay should start in the first seconds, slow intros and logos lose viewers, cinematics/story are "nice to have." Lieu says his own recent trailers (Noita, Tactical Breach Wizards) open with "a few seconds of gameplay up front." — [Game World Observer](https://gameworldobserver.com/2022/09/22/common-game-trailer-mistkaes-indie-developers-steam)
- r/DestroyMyGame feedback (secondhand, via an aggregated search summary, so only the search tool's paraphrase, not verified at source): commenters criticise trailers that show too little ("you show one attack, a dash and throwing rocks") or have a slow opening. Mirror thread links found: [DestroyMyGame](https://nyc1.lr.ggtyler.dev/r/DestroyMyGame/comments/1gea3vh/time_to_destroy_my_trailer), [DestroyMyGame 2](https://reddit.birdcat.cafe/r/DestroyMyGame/comments/1ihyze6/trying_to_improve_my_solo_dev_game_tailer_is_this). I did not open them.
- Advice (presskit.gg, undated): first five seconds should answer "What is this, and why should I care?"; "Go over 90 seconds and you start bleeding viewers"; "Mute your trailer. Watch it silent. If it's boring, your visuals need work." — [presskit.gg](https://presskit.gg/field-guides/game-trailers-screenshots-guide)

### Inferences
- Complaints cluster around "footage does not show the game well" (low visible action, chaotic play), which supports a design where scenes are staged and composed deliberately rather than harvested from play.
- Lieu's statement that planning is hard because each game differs suggests a fixed template helps only as a starting point.

### Gaps
- No first-hand Reddit quotes (r/gamedev, r/IndieDev) were verifiable; Reddit access failed. "Too early to show" concern: nothing found.
- Strength of feeling cannot be measured from this sample.

## What goes wrong when capturing footage

### Takeaway
Documented problems: re-capturing after build changes, scripted moments needing many takes, positioning the player so cuts match, discovering bugs while scrutinising footage, and engine recording tools (Take Recorder, MRQ, Unity Recorder) not fitting live gameplay. Frame rate and HUD hiding show up as tool-level issues more than personal complaints.

### Cited Findings
- First-hand (devlog): trailer work "makes you capture and scrutinize a shedload of gameplay footage" which revealed new bugs; "multiple takes of this to get it right" for a scripted boss sequence, with bloopers (rocket launcher fired the wrong way); some edits had to be re-shot so the player was positioned correctly for the cut. — [itch.io devlog](https://itch.io/devlog/107312/trailer-trash-pt-2.amp)
- First-hand (professional capture director Jason White, Aftermath interview): he will "recapture existing footage due to build changes/updates"; each game's debug tools differ and "figuring out what is possible within those parameters takes up a lot of time and problem solving"; "the delay in providing builds to capture teams... is what really sets us back." Also on repeated takes: capturing a playthrough "for the umpteenth time." — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker.md)
- First-hand (Unreal forum, July 2025): a dev wants to film TPS gameplay from different Cine Camera angles in Level Sequencer but runtime-spawned projectiles, VFX, SFX and AI mean "Take Recorder doesn't capture these"; building a cutscene by hand is "too slow" because weapons must be attached and VFX/SFX spawned manually. Replies only suggested making a cutscene with events or a spectator system; no real solution given. — [Unreal forum](https://forums.unrealengine.com/t/recording-gameplay-footage-at-different-camera-angles/2598403)
- Unreal forum: Movie Render Queue (MRQ) cannot start during Play-In-Editor; one poster says that choosing MRQ forces them "to leave gameplay" (French-language thread, via search summary). — [UE forum, French](https://forums.unrealengine.com/t/comment-rendre-une-video-en-utliisant-movie-render-queue-tout-en-enregistrant-aussi-le-gameplay/1420232); another thread reports a World Partition level "simply isnt loaded" during MRQ renders — [UE forum](https://forums.unrealengine.com/t/how-do-i-render-record-a-video-in-world-partition-level-using-sequencer-it-is-all-unloaded/2701606) (both from search snippets; I did not open them).
- Unreal forum bug report: the debug "Hide HUD" option does not hide the "Free Camera Mode" text. — [UE forum](https://forums.unrealengine.com/t/debug-hide-hud-does-not-hide-free-camera-mode-text/2730590) (snippet only)
- Unity issue tracker (bugs, not complaints; mostly 2018-era): Movie Recorder with WebM drops FPS to 14 (Windows)/8 (macOS) vs 80+ for MP4; Recorder frames incorrectly recorded with Cinemachine at 25/50 fps timelines. — [Unity tracker](https://issuetracker.unity3d.com/issues/recorder-frame-rate-drops-drastically-when-using-movie-recorder-to-record-as-webm), [Unity tracker 2](https://issuetracker.unity3d.com/issues/frames-are-incorrectly-recorded-when-timelines-are-in-25-or-50-fps-and-using-cinemachine)
- Godot docs: Movie Maker mode is for non-real-time recording and is "not designed for capturing real-time footage"; AVI output limited to 4 GB; docs list trailers as a use case. — [Godot blog](https://godotengine.org/article/movie-maker-mode-arrives-in-godot-4)
- Roblox DevForum (July 2019, old): devs ask how others fly a camera with "no mouse cursor, no guis"; one advises "you can get smoother, better-looking camera movement by connecting a gamepad"; free cam needs edit access. — [Roblox DevForum](https://devforum.roblox.com/t/filming-a-game-trailer/310305)
- Advice (summaries from search; pages not opened): record 3-5x more footage than needed; 60 fps capture even for 30 fps output; use a fast drive; a 75 s trailer needs 10-20 min of raw footage; dev teams add free-cam and HUD-hide hotkeys (Paradox, A Game of Dwarves); a team found trailer footage "just wasn't good enough" months in (ModDB Sikanda devlog, 403 when fetched). — [GameAnalytics-type guides via search](https://www.moddb.com/news/sikanda-devlog-1-making-a-game-trailer), [Abratabia](https://www.abratabia.com/game-marketing/game-trailers.php)

### Inferences
- The strongest recurring friction: gameplay moments are hard to reproduce on demand and hard to re-film from a new angle without re-running the whole play (Unreal forum post, Aftermath, devlog).
- Engine recording tools tend to be built for either cinematics (MRQ/Movie Maker) or live capture (OBS), with a gap between them.

### Gaps
- Nothing first-hand found on OBS/ShadowPlay settings, lighting/time of day, or aspect ratio complaints beyond advice lists.
- NPC/enemy behaviour control: only the one-enemy-at-a-time staging example.

## Camera work pains and cinematic tools (Sequencer/MRQ, Cinemachine/Recorder, Godot)

### Takeaway
Evidence is mostly bug reports and tool limitations rather than expressed frustration. No direct learning-curve or render-time complaint was verified.

### Cited Findings
- See the Unreal forum Take Recorder and MRQ items above; the only first-hand camera-rig complaint is that doing it by hand is "too slow." — [Unreal forum](https://forums.unrealengine.com/t/recording-gameplay-footage-at-different-camera-angles/2598403)
- MRQ with game mode data needed warm-up frames: "first 3 frames need to be skipped to avoid sync mismatch." — [UE forum](https://forums.unrealengine.com/t/sync-of-movie-render-queue-game-mode/2577443) (snippet)
- Unity Discussions: a poster reports random frame drops and about 11 ms per frame in Cinemachine 2.9.7; responders disagreed on cause. — [Unity Discussions](https://discussions.unity.com/t/cinemachine-frame-drop/930736) (snippet)
- Roblox: smooth consistent camera movement valued; gamepad gives smoother results than keyboard (2019). See link above.

### Inferences
- Controller-driven free camera is the common low-cost workaround; scripted camera paths need setup effort that devs question.

### Gaps
- No verified quotes about Sequencer/Cinemachine learning curves or MRQ render times.

## Re-capturing / re-rendering after small changes (iteration cost)

### Takeaway
Re-capture after changes is confirmed by a professional capture director and by one indie dev, but no quantified iteration-cost complaint was found.

### Cited Findings
- Indie devlog, 6 Nov 2025 (Particulitix): "after redoing the enemies graphics, I had to redo the trailer." Neutral tone, one sentence. — [itch.io devlog](https://itch.io/devlog/1105267/gameplay-trailer.amp)
- Professional: "recapture existing footage due to build changes/updates." — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker.md)
- Derek Lieu (2023): he "didn't have the time to recapture the footage" to improve his deliberately bad trailer, i.e. recapture is time that is skipped when short. — [Derek Lieu blog](https://derek-lieu-yrd2.squarespace.com/blog/2023/3/11/i-made-the-worst-firewatch-trailer-ever)
- Devlog snippets that mention trailer redone after art overhaul (Devotionalia, Rocket Shipment "last one was really outdated") were seen only in search results. — [Devotionalia](https://gcbaccaris.itch.io/devotionalia/devlog/161052/update-art-overhaul-trailer)

### Inferences
- Art changes make existing footage outdated, forcing a full redo. That is a direct case for shot-level re-render, but the evidence is 2-3 anecdotes.

### Gaps
- No quote about changing a single shot and re-capturing only it, and no cost numbers.

## Gameplay trailer vs cinematic teaser, and honesty (bullshots)

### Takeaway
Advice favours real gameplay first; honesty is discussed mainly through Valve's 2016 screenshot policy. No indie dev complaint on this was found.

### Cited Findings
- Professional view: "a vertical slice is not a lie" (Jason White), representative of what the game is working toward. — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker.md)
- Steam (2016) banned concept art and pre-rendered images in screenshots, requiring actual in-game screenshots; the policy followed No Man's Sky anger. It concerned screenshots, not trailers. — [TweakTown](https://www.tweaktown.com/news/54782/bullshots-coming-end-steam/index.html)
- Advice: label in-development footage with a "Work in Progress" disclaimer (guide via search). Bugnet lists footage not reflecting the actual game as a common mistake. — [Bugnet](https://bugnet.io/blog/common-game-trailer-mistakes)
- Derek Lieu: cinematics/story are "nice to have" compared with gameplay. See Game World Observer link above.

### Inferences
- The tension is "staged but real build" versus "faked"; the cited sources treat staged footage from the real build as acceptable.

### Gaps
- No first-hand dev discussion on bullshots in trailers found; Reddit unavailable.
