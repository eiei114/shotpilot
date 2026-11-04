# Reddit one-hand complaints from game developers about making their own trailers/teasers

Access route actually used: reddit.com, old.reddit.com and api.reddit.com all blocked (HTML login/redirect page, no JSON). PullPush returned HTTP 521. **Arctic Shift (arctic-shift.photon-reddit.com) worked** for non-text queries: I paged through `/api/posts/search?subreddit=X&sort=desc` (date-ordered, no text filter because title/body filtering timed out), filtered titles/selftext locally for trailer/teaser/footage, then read each thread via `/api/comments/search?link_id=<id>`. Every quote below is full post/comment text retrieved this way (not search snippets) unless marked. WebSearch only returned third-party mirrors and was used just to confirm the approach (not quoted).

Coverage: r/gamedev, r/IndieDev, r/indiegames, r/IndieGaming, r/unrealengine, r/UnrealEngine5, r/Unity3D, r/godot, r/gamemarketing scanned (DestroyMyGame scanned but not read in detail). The scan went newest-first; effective coverage of threads I could read is **2024-12 to 2026-11**. Nothing from 2019-2023 was retrieved (scan did not reach it in time) -- this is a gap, not evidence of absence. Note: dates are as stored (today is 2026-11-05).

Permalink form: post `https://www.reddit.com/comments/<postid>/`; comment `https://www.reddit.com/comments/<postid>/_/<commentid>/` (these short forms redirect on reddit.com; I could not open them myself, they are built from the IDs returned by the API).

Tags: [C]=complaint, [Q]=question showing confusion, [A]=advice/workaround. Stage tags in brackets.

## What do devs most often complain about, in their own words, and which stage dominates?

### Takeaway
Across ~20 threads, complaints concentrate on (1) deciding what to show/what the point of the trailer is (the "communication" problem, said by several to be harder than recording or editing), (2) the sheer time and friction of capture + edit loops, and (3) capture-quality problems (frame pacing, Unity Recorder/Take Recorder, resolution). Tools-for-NLE learning (DaVinci vs Premiere) is the most frequent beginner question, but it is rarely the heaviest complaint.

### Cited Findings

Structure / what to show (stage: concept)
1. [Q][concept] r/gamedev, 2026-08-16, "How do I decide what to put in my trailer?" -- "I have four main loops. Gathering, brewing, village creation/management, tavern hours." ... "But now...I have to make a trailer." https://www.reddit.com/comments/1vpe878/
2. [C][concept] r/IndieDev, 2026-08-17, comment by the OP of "How can trailers be so hard to make?" -- "thinking what to make the trailer about and then getting it right (not deviate from the plan) is a total pain, and then even if you manage to achieve it, there is no guarantee it was a good idea that will sell/your audience will like it." https://www.reddit.com/comments/1vputb7/_/p46wav0/
3. [C][concept] r/IndieDev, 2026-08-17, a reply in that thread -- "The hard part about a trailer isn't the recording, the editing, or scoring. It's the communication. I seriously spent more time thinking up what _gameplay_ beats best encapsulate my player fantasy and storyboarding them out than I did implementing fake UI, shooting, editing, and scoring combined." https://www.reddit.com/comments/1vputb7/_/p43of1w/
4. [Q][concept] r/gamedev, 2026-03-04, "How do you all make trailers?" -- "How do you choose between tens if not hundreds of clips which to use and how to position them? How do you decide on the timing of the trailer overall? When and where to place text?" https://www.reddit.com/comments/1rk6o7f/
5. [C][concept] r/gamedev, 2026-10-05, "Genuinely struggling to create a good trailer" -- "I've watched all the advice videos. I've read everything there is to read. ... and I've made like 6 versions. No matter what, I just cannot make a good trailer." https://www.reddit.com/comments/1wy7bua/
6. [C][concept] r/gamedev, 2026-06-06 (comment on a tool-builder's survey post, quoted as dev opinion, not tool endorsement) -- "you cut 6 versions, A/B nothing properly cause your wishlist volume is too small to be statistically meaningful, and just kinda vibe-pick one." https://www.reddit.com/comments/1ty993x/_/oq1o5u4/
7. [Q][concept/pre-release] r/gamedev, 2026-08-10, "How do developers release game trailers long before their games are finished?" -- "we were supposed to have our trailer done last November. However, we're finding it challenging to get the footage we need for the trailer at our current stage of development." / "to capture footage of all these things, we have to actually develop them." https://www.reddit.com/comments/1vki1cl/
8. [Q][concept/pre-release] r/IndieDev, 2026-09-26, "Making a trailer when you don't have a game" -- "do they not run into problems with winding up needing to make some design changes that don't fit with the trailer?" https://www.reddit.com/comments/1wq2eof/

Time and friction (stage: whole process)
9. [C][time] r/IndieDev, 2026-08-16 -- "I commissioned a musician to make the music, spent a lot of time learning to use Davinci, and spent an insane amount of hours adding commands and setting up the scenes of the game to record. I heavily underestimated how hard making a trailer is." / "I felt like giving up so many times." https://www.reddit.com/comments/1vputb7/
10. [C][time/edit] r/IndieDev, 2026-09-21, "I'm finally working on the trailer for our next Steam game..." -- "I've made 3 game trailers before, and this is my 4th one. The only thing I've learned from making the previous 3 is: Making a game trailer takes WAY too much time." / "you end up spending hours trying different clips, timings, transitions, text, music, effects... and watching the same 30 seconds about 100 times" https://www.reddit.com/comments/1wm69jk/
11. [C][time] r/IndieGaming, 2026-09-04 -- "Day 1: 'Ah, Unreal Sequencer is so cool! I can do so much stuff!' Day 7: *writing my own eulogy...*" (title: "Making this trailer NEARLY KILLED ME!") https://www.reddit.com/comments/1w6eciq/
12. [C][time] r/godot, 2026-08-12 -- "Typically, I detest the process of making trailers for my games. It doesn't excite me creatively, it takes forever, I'm never really very happy with the results, and I dread doing it." https://www.reddit.com/comments/1vlygud/
13. [C][edit/skills] r/gamedev, 2026-07-22 -- "I'm honestly getting frustrated." / "the software feels incredibly overwhelming. Between the Edit, Color, Fairlight, Fusion, and Deliver pages, it feels like I'm learning multiple applications instead of just editing a trailer." (DaVinci Resolve) https://www.reddit.com/comments/1v3d7vq/
14. [C][edit] r/gamedev, 2026-06-12 -- "While I was making the trailer my free Adobe Premiere Pro trail [sic] and now that software is costly and overall shit" https://www.reddit.com/comments/1u3tlut/
15. [C][edit/skills] r/gamemarketing, 2026-09-11 -- "I'm not a creator and Ive tried doing it manually - I suck at editing in capcut." https://www.reddit.com/comments/1wcys4y/
16. [C][edit] r/IndieDev, 2026-08-17, reply in "How can trailers be so hard to make?" -- "When I am editing its a lot of scrubbing frame by frame to find the exact frame i need to cut on. And then multiply by the number of cuts" https://www.reddit.com/comments/1vputb7/_/p43lrxc/
17. [C][edit/skills] r/IndieDev, 2026-10-08, "Looking for Steam trailer editors!!" -- "I do have CapCut for other socials advertising but im just not creative enough to put together the trailer im envisioning for my game haha" https://www.reddit.com/comments/1x0j6lx/_/pel2t4m/

Capture (stage: footage)
18. [C][capture/frame pacing] r/unrealengine, 2026-01-25 -- "For the trailer, we chose a cinematic capture approach: 24fps, upscaled to 30fps ... In hindsight, this was a mistake for gameplay footage." / "Players instinctively equate frame pacing in trailers with real performance" https://www.reddit.com/comments/1qmjadi/
19. [C][capture/engine tool] r/UnrealEngine5, 2026-06-11, "How do you build your trailers?" -- "we want the trailer to have gameplay not just be cinematic but Take recorder is a POS honestly, not registering armour or weapon swaps or material changes" https://www.reddit.com/comments/1u2xj6p/
20. [C][capture/engine tool] r/UnrealEngine5, 2026-06-11 (reply) -- "I've been exporting in EXR image sequences and taking those absurdly huge files and bringing them into da Vinci ... all though it's very slow and tedious. I have no idea how I'm going to record actual gameplay because i absolutely loathe take recorder haha. I can always just screen record a build but it's a huge quality drop." https://www.reddit.com/comments/1u2xj6p/_/or0vy91/
21. [C][capture/Unity] r/Unity3D, 2026-06-07 -- "Unity Recorder doesn't seem to produce clean output, the footage looks pixelated and lacks sharpness, even at higher resolutions." https://www.reddit.com/comments/1tz82yd/
22. [C][capture/performance] r/Unity3D, 2026-06-15 -- "we're talking 100FPS drop. I've tweaked all the settings with Unity and closed my tabs and I think 19FPS is the best I get while recording with Unity Recorder. With OBS I'm still above 100 with minimal artifacting. Neither is good enough for a trailer though." https://www.reddit.com/comments/1u5j0f4/_/orlv77s/
23. [C][capture/resolution] r/gamedev, 2026-09-21 -- "When i record the game play in 4K and cut the trailer with the 4K footage and later export it to 1080p it does not look as crisp and sharp as the native 1080 version. Here i loose my pixel perfect pixels." https://www.reddit.com/comments/1wlkh1n/
24. [C][capture/multiple versions] r/gamedev, 2026-09-21 (same OP) -- "The problem with having multiple versions (one for youtube and one for the steam trailer) is that my game i random at a lot of places and i want to record gameplay. I would then have 2 slightly different versions which is not what i exactly want." https://www.reddit.com/comments/1wlkh1n/_/pazzkrh/
25. [C][capture/free camera, controller] r/gamedev, 2026-01-09 -- "Mouse movement is too jittery and fast to look good in a trailer. Nobody can move a mouse in a perfectly linearly interpolated motion." (reply by a dev who records trailer footage) https://www.reddit.com/comments/1q84vg4/_/nykplf3/ ; same thread: "It's virtually impossible and certainly impossible to reproduce." https://www.reddit.com/comments/1q84vg4/_/nykubqu/
26. [C][capture/staging, UI] r/gamedev, 2026-08-07 -- "Setting up for a good clip takes time, and you might mess up and forget to hide some UI or turn the music off. Investing in good tools for setting up your shots is worth it" https://www.reddit.com/comments/1vh629o/
27. [C][capture/cheats] r/gamedev, 2026-08-07 (reply) -- "I just made my first trailer over the weekend and the amount of extra cheats I had to build in as well as the 30+ bugs I found was nuts!" https://www.reddit.com/comments/1vh629o/_/p22jqob/
28. [C][capture/UE MRQ] r/unrealengine, 2024-12-28 -- "Every single time I render, the MRQ renders 3 frames correctly, then proceeds to completely change the aspect ratio and framing on the shot ... I basically have to stop the render every 3 frames and start it again from where it reached." (OP says they have worked on feature-length films and trailers.) https://www.reddit.com/comments/1ho2um0/
29. [C][capture/UE MRQ] r/unrealengine, 2026-01-14 -- "After recoring Rigid bodies, for some reason render queue starting new simulation and dont use prerecorded." / "Chaos Cache recordings for some reason was different speed after recording ... (it was pain)" https://www.reddit.com/comments/1qcne07/
30. [C][capture/mobile] r/gamedev, 2026-10-08 -- "I'm trying record gameplay with Snipping tool but since this is a mobile game, I really dont want to show the mouse moving etc." https://www.reddit.com/comments/1x0sxuf/

Cinematic-tool confusion (Unity/UE)
31. [Q][capture/engine tool] r/gamedev, 2026-08-17 -- "I'm using Unity, but I'm not sure how people actually record the gameplay and create those cinematic shots. ... Do you use Unity Timeline/Cinemachine or another tool?" https://www.reddit.com/comments/1vq6fwc/

Outsourcing / cost
32. [C][outsource] r/gamedev, 2026-01-02 -- "I been using using Fiverr to hire an video editor to make trailer for my steam game, and the result was not great." https://www.reddit.com/comments/1q1zrjt/
33. [C][outsource] r/IndieDev, 2026-10-08 -- "I'm tired of getting burnt on Fiverr and just having everyone on there using AI." https://www.reddit.com/comments/1x0j6lx/
34. [C][outsource] r/gamedev, 2026-04-07, a question from a tool-builder (quoting what he heard from devs; treat as secondhand) -- "Hired a studio -> thousands of dollars, weeks of revisions, ended up looking generic / Did it myself in After Effects -> ate 2 weeks I should've spent on the game" https://www.reddit.com/comments/1semcvd/
35. [C][outsource] r/gamedev, 2026-05-01 (reply) -- "I paid for an editor on fiverr and regretted it so much I decided to make my own trailer." https://www.reddit.com/comments/1t0pvtx/_/ojbriqv/
36. [C][outsource] r/gamedev, 2026-06-05 -- "I recently received a marketing video for my game from an external company we hired ... some AI-generated imagery had been mixed into the video. ... So I ended up re-editing the video myself. As you can probably tell, I'm a programmer, not a video editor" https://www.reddit.com/comments/1txf3x5/
37. [A][cost] r/gamedev, 2026-01-02 -- "You can get a friend to edit together some game footage you record for the cost of a six-pack. You can get someone with some experience to do it for a few hundred. A professionally edited trailer might cost you more like $10-15k. Hiring a studio to make a fully CGI and acted trailer could cost you closer to $100k for a minute." https://www.reddit.com/comments/1q1zrjt/_/nx99gp9/
38. [A][outsource] r/gamedev, 2026-08-16 -- "Make it yourself, and for a reason specific to your game ... nobody you commission will know which four seconds of your game actually show that. You would spend longer briefing them than editing it." https://www.reddit.com/comments/1vpt0n8/_/p41ochu/

### Inferences
- Stage dominance: "what to show" and "time/friction of the capture-edit loop" appear most (items 1-12); editing-software confusion is the most common question but the lightest complaint (13-17). Capture complaints cluster on three things: frame pacing/quality (18, 21-23), engine capture tools being unreliable (19-20, 28-29), and the manual set-up work (cheats, hiding UI, staging) (26-27).
- Several devs explicitly say the hardest part is not recording/editing but deciding the message (3, 2, 38), which a shot-list-first tool would address more than a video editor would.
- Sample is skewed to r/gamedev/IndieDev self-selected posters in 2026; Reddit frequency is not representative of all devs.

### Gaps
- No 2019-2023 threads retrieved. Pre-2025 UE/Unity forum pain (MRQ/Sequencer) is covered only by one 2024-12 thread.
- r/DestroyMyGame was fetched (16k posts) but I did not read threads; its commentary is about critique, not production pain.
- Comment bodies are only the top ~6-14 by score per thread; later comments not read.

## Are there quotes about re-capturing / re-editing after the game changes, or making many versions?

### Takeaway
Yes, directly and repeatedly. Devs say they redo trailers whenever visuals/features change, that outdated trailers hurt, and the more advanced ones script their capture so it can be re-run -- they describe this as the main time saver. Several state outright that manual recapture is the pain; some argue against automation.

### Cited Findings
- [A/pain][re-capture] r/gamedev, 2026-08-07: "I'm a Software Engineer and too busy coding the game to learn video editing, trying to capture just the right gameplay, having to consistently redo the videos as the game develops and such, so I just scripted code to play and change scenes and simply just screen recorded that. Now I just rerun it with game updates to get new up-to-date recordings. It's been really convenient in the long run as I have made 4 versions of my trailer now." https://www.reddit.com/comments/1vh629o/_/p22xbzh/
- [A][re-capture] r/gamedev, 2026-08-09: "I just coded mine out in scripted gameplay, presents the text, changes scenes, everything. Then I just full screen, press record, and that's it. As the game evolves, all I do is re-run and have an updated video." https://www.reddit.com/comments/1vj819h/_/p2jccm3/
- [A][re-render] r/gamedev, 2026-08-17: "I cut my first draft with ffmpeg from that shot list, plain gameplay capture, no camera tricks, and only then moved to an editor for polish. The scripted cut turned out to be the real win, when the game changes you re-render instead of redoing the whole edit." https://www.reddit.com/comments/1vq6fwc/_/p44fyw3/
- [A][re-render, studio practice] r/gamedev, 2026-02-24, "How I do self-recording in-engine game trailers": "Real gameplay is too hard to control and impossible to re-record. ... recreate those in a more sanitary environment ... orchestrated via engine cutscene tools ... Then the whole trailer can be rebaked at anytime as its edited or the background lighting is changed, new animations are delivered, you want to change the player/weapon skins" https://www.reddit.com/comments/1rcl89a/_/o72eqkp/
- [A][re-render] same thread, OP's defender: "One aspect is that this makes iteration much easier through game development, as most of the trailer will just update itself when you make changes to the game. You only need to hit the render button. Rather than updating the entire video edit afterwards." https://www.reddit.com/comments/1rcl89a/_/o744o4i/
- [C] same thread, OP-side: "Trailer editing was turning into a whole second project for me. In-engine and capture definitely makes iteration less brutal." https://www.reddit.com/comments/1rcl89a/_/o6z1r27/
- [counter-view] same thread: "I just see the nightmare of everything breaking when as the game progresses and you either delete all your stuff or spend a lot of time maintaining settings/code you don't need to." https://www.reddit.com/comments/1rcl89a/_/o746n4j/ ; and "Level layouts? Ok, say one part is the player walks from A to B. The map changes, the player now takes too long to do it and the video is broken..." https://www.reddit.com/comments/1rcl89a/_/o7492rl/
- [C][reshoot] r/gamedev, 2026-08-06: "I also had to reshoot the whole thing because I decided to change my enemy textures ... I also got burnt out on the process and decided that it was good enough at a certain point" https://www.reddit.com/comments/1vh629o/_/p22lhl2/
- [C][reshoot after look change] r/IndieGaming, 2026-09-13: "I'll have to redo the trailer now, as The Endless Harvest now has a completely different look!" https://www.reddit.com/comments/1wejt5c/
- [C][redo vs. twice] r/IndieDev, 2026-09-18: "my original plan was to redo the trailer/screenshots once I reach that instead of doing everything twice." https://www.reddit.com/comments/1wjjawb/
- [C][update] r/gamedev, 2026-04-04: "Trailer hitches / GC hiccups -> Applied object pooling, re-recorded the trailer" https://www.reddit.com/comments/1sc0e2v/
- [C][update, Q about outsourced] r/gamedev, 2026-05-01: "It's obviously still under development so if paying someone how would you deal with it becoming out of date? Do they give you video files you can update the clips in or do you just pay them again?" https://www.reddit.com/comments/1t0pvtx/
- [C][out of date] r/gamedev, 2026-09-17, Steam puzzle post-mortem: "i didn't update my trailer after completing more of the game, i didn't start the trailer with gameplay but started at the start of the story." (snippet of selftext as returned by the API; full post not read) https://www.reddit.com/comments/1wicyud/
- [C][out of date] r/gamedev, 2026-05-01 post-mortem (snippet only): "Mistakes: Inconsistent social media content, relying on a single outdated trailer for too long" https://www.reddit.com/comments/1t0bw5m/
- [A][Next Fest] r/gamedev, 2026-10-27 (snippet only): "Update the trailer with the best one you can have, I went in with an old one." https://www.reddit.com/comments/1xg7yic/
- [A][planned recut] r/IndieDev, 2026-09-04: "this is the first version of the trailer. I plan to keep recutting it as the game grows." https://www.reddit.com/comments/1w6qwa7/
- [C][many versions] r/gamedev, 2026-06-06 (see above #6): "you cut 6 versions"; r/gamedev 2026-10-05: "I've made like 6 versions."
- [A][godot] r/godot, 2026-11-02: "The trailer is all in-engine ... Movie Maker mode so every frame comes out perfect no matter how slow the recording is. A fixed seed passed as a command-line argument, so the same decisions replay the exact same run every time. Scripted takes: each shot starts from a prepared save with a specific build and plays out a list of actions" (comment truncated by me at that point) https://www.reddit.com/comments/1xlsjij/_/19s0t5cdb8clf/
- [C][social cuts] r/IndieDev, 2026-09-21 reply: "check the vertical crop early lol. we had a caption covering the hand doing the actual action." https://www.reddit.com/comments/1wm69jk/_/pb51ijt/ ; "[A] I re-edit the footage specifically for social media" (AMA, snippet) https://www.reddit.com/comments/1tkqsd0/

### Inferences
- The "script the capture so you can re-run it when the game changes" pattern appears independently in 5 separate threads (2026-02 to 2026-11), all from ordinary devs, not a vendor. It is exactly the use case the ask's key question describes. Counter-opinion (fragile when levels change) is also in the data.
- Evidence of redo after game/visual change is frequent and unprompted (Endless Harvest, enemy textures, performance fixes, outdated trailer as a named mistake).

### Gaps
- No quote specifically describing Next Fest multi-version workflow beyond "update the trailer" (snippet only).
- No quote about re-rendering only changed shots in Sequencer/MRQ specifically.

## Quotes wishing for a tool / asking to automate camera shots / re-render only changed shots?

### Takeaway
No poster was found literally asking for "re-render only changed shots." Closest: devs wishing for tooling to set up shots, devs who built their own scripted capture, and a cinematic-studio dev who "rebakes" the whole trailer. Two tool-builder posts (surveys) exist but are marketing/market-research, not first-hand wishes.

### Cited Findings
- [wish][tooling] r/gamedev, 2026-08-07: "You might need better tooling. Setting up for a good clip takes time ... Investing in good tools for setting up your shots is worth it" https://www.reddit.com/comments/1vh629o/
- [wish][automation] r/gamedev, 2026-08-07: "But yeah I'm a very 'SWE automate all the things' kind of guy" and "I even considered writing an event log to a file that could be used by a script to automatically cut 4 second clips from the video but I decided to go manual with both cutting and recording instead." https://www.reddit.com/comments/1vh629o/_/p22ztx6/
- [wish][engine] r/UnrealEngine5, 2026-06-11: "Do people just animate a cool level sequence or record with OBS or is their some secret take recorder setting I'm missing?" https://www.reddit.com/comments/1u2xj6p/
- [wish][engine] r/unrealengine, 2026-01-14: "I didnt find a way to blend animation of face and body in sequencer ... I believe Unreal should add such opportunity" https://www.reddit.com/comments/1qcne07/
- [workaround][smoothing] r/gamedev, 2026-04-13: "the part nobody talks about is how bad raw mouse input looks when you're the one controlling the camera ... ended up building a little tool called cinemauz that smooths your mouse input in real time" (commenter is the tool's author) https://www.reddit.com/comments/1semcvd/_/ofs4vvw/
- [skeptic] r/gamedev, 2026-06-06: "Is this the new Ai grift? I keep seeing the same kind of post across multiple totally different subreddits." (reaction to a "what wastes your time" survey) https://www.reddit.com/comments/1ty993x/_/oq1ued8/ ; r/gamedev 2026-04-07: "'please try out my solution for this problem I made up'" https://www.reddit.com/comments/1semcvd/_/oeqz3d8/
- [workaround][godot] r/godot, 2026-08-12: "Why shouldn't I animate full sequences for my trailers in Godot and just use OpenShot to do the final bit of splicing together and exporting?" (motive: video editing software is "unfamiliar to me and everything I try to do has so much friction") https://www.reddit.com/comments/1vlygud/

### Inferences
- Pain-driven DIY (scripted playthrough + screen record, fixed seed + Movie Maker mode) is the observed substitute for a product. Dev communities also react hostilely to unsolicited tool pitches, so first-hand wish statements are rare and mostly implicit.

### Gaps
- No Reddit quote found asking for Sequencer/MRQ batch or "only changed shots" re-render. Search was title-oriented; comment-body text search was unavailable on the API.

## How many hours/days/weeks do devs say a trailer took?

### Takeaway
Self-reported times range from ~4 hours (rough first trailer) through 5 days, "a week of solid work", 1.5 months, to "weeks full time" and "3 months". Hours-per-minute rules of thumb also appear. These are single self-reports, not statistics.

### Cited Findings
- 9 hours with AI video tools, r/gamedev, 2026-09-09: "It took NINE hours" (post title; commenters mocked the quality) https://www.reddit.com/comments/1wauye1/
- ~4 hours (first) vs ~1 week (second), r/IndieGaming, 2026-06-02: "That first trailer took about 4 hours? ... This new trailer took me about a week of solid work" https://www.reddit.com/comments/1tu27tg/
- 5 days, r/indiegames, 2026-06-20 (snippet of selftext): "it took me 5 days to put together" https://www.reddit.com/comments/1uastr2/
- "weeks full time", r/gamedev, 2026-01-02: "It took me weeks full time to do mine, and I already knew how to use video editing software. So it's probably at least 2k." https://www.reddit.com/comments/1q1zrjt/_/nx9fxj2/
- 1.5 months, r/gamedev, 2026-01-30: "I spent a month and a half creating my first cinematic teaser trailer ... reworking it several times from scratch" (title) https://www.reddit.com/comments/1qqbg9t/
- 3 months, r/gamedev, 2025-12-22 (snippet of selftext, Kickstarter trailer): "Trailer took 3 months to make" https://www.reddit.com/comments/1psbkw1/
- 2 weeks in After Effects (secondhand, from a tool-builder's recap): "Did it myself in After Effects -> ate 2 weeks I should've spent on the game" https://www.reddit.com/comments/1semcvd/
- "trailer was supposed to be done last November" (delay, not duration): https://www.reddit.com/comments/1vki1cl/
- Sprint: "We just crunched out my game's trailer for the upcoming Steam Next Fest" (title only; no duration given) https://www.reddit.com/comments/1x8mbwm/ ; cinematic studio claim "our very first UE-trailer in just 10 days" (title) https://www.reddit.com/comments/1iy5lsa/ (2025-02-26, r/unrealengine)

### Inferences
- Wide spread, with the strongest anecdotal signals for "days to weeks" for a solo dev. A rule-of-thumb "24 hours per finished minute" appeared in a mirror-site search summary, which I did not verify in a Reddit thread -- not cited.

### Gaps
- No distribution data; time-to-trailer for re-edits after changes not stated numerically except "4 versions" and reshoot anecdotes.
- Items marked "snippet" are selftext excerpts returned by the API where I did not read the full post.
