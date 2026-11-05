# Motion intent vocabulary for shotpilot: web research

Scope: how to represent motion intent (eerie / punchy / gentle) as a small vocabulary and `tokens.json` fields, and how to render it with ffmpeg 8.1 (libass + drawtext) with no new dependencies. Camera/real-teaser measurements are other agents' work.

## Provenance legend

- [S] Sourced. URL cited inline. Where I saw only a search snippet (not the page body) it is marked "snippet only".
- [I] Inference by me (design judgement, not from a source).
- [T] Tested locally on ffmpeg 8.1.2 (gyan full build) in this session. Test files are in the session scratchpad, not in the repo.

## 0. Current state (what cannot be expressed today)

[S, repo] `references/design-systems/neon-action/tokens.json` carries `finish.{grade,letterbox,typeface}`, `palette`, `type`, `cards.{card_hold,title_hold,black,max_ja,max_en}`, `transitions.{default,accent}` (cut/flash/dip) and `camera`. There is no field for how text enters, holds or leaves, nor for tempo or overshoot. DESIGN.md prose ("short, loud and kinetic") is read by Claude but has no code path. `finish.rs` fades every line in about 0.4 s and out about 0.45 s, and the title always flickers in (task statement).

[S, repo] `TEASER-BASELINE.md` already holds the hard limits any motion must respect: at least 1.5 s hold per text, 4 JA chars / 3 EN words per second plus 0.5 s, 5% safe margin, 4.5:1 contrast, no sustained shake/rotation/rapid zoom on text. These are inherited from open-design's hyperframes profile.

## 1. Motion in design systems

**Takeaway.** Every mature system reduces "personality" to the same four or five numbers: duration tier, easing family (with separate enter/exit curves), overshoot (spring damping), travel distance, and stagger. Personality adjectives never appear in the token files themselves; they sit one layer above as a small number of named modes (Carbon: productive / expressive; M3: standard / emphasized; Fluent: 4 duration tiers x 2 easing families). That layering is the thing to copy: few named modes in the brief, numbers in tokens, and code that resolves one to the other.

**Cited findings**

- [S] IBM Carbon defines two modes. Productive is "efficient and responsive while staying subtle"; expressive is "more vibrant and highly visible", to be reserved for "occasional, important moments". Duration tokens: fast-01 70 ms, fast-02 110, moderate-01 150, moderate-02 240, slow-01 400, slow-02 700. Each mode has its own standard/entrance/exit curve, e.g. productive entrance `cubic-bezier(0, 0, 0.38, 0.9)`, expressive entrance `(0, 0, 0.3, 1)`, productive exit `(0.2, 0, 1, 0.9)`, expressive exit `(0.4, 0.14, 1, 1)`. https://carbondesignsystem.com/elements/motion/overview/
- [S] Material Design 3 (Android docs). Three easing families: standard (utility), emphasized (M3-styled), linear. Each has on-screen / enter (decelerate) / exit (accelerate) variants: emphasized-decelerate `(0.05, 0.7, 0.1, 1)`, emphasized-accelerate `(0.3, 0, 0.8, 0.15)`, standard `(0.2, 0, 0, 1)`. Durations short1..4 = 50..200 ms, medium1..4 = 250..400, long1..4 = 450..600, extra-long1..4 = 700..1000. "Duration should increase as an animation's area or traversal increases." Springs split into spatial (damping 0.9, may overshoot) and effects (damping 1, no overshoot) at fast/default/slow speeds. https://raw.githubusercontent.com/material-components/material-components-android/master/docs/theming/Motion.md
- [S] Atlassian bundles duration + easing + property into semantic tokens named by context, motion type and tempo (`motion.popup.enter`); exits are 50-100 ms shorter than entrances with a "practical" curve. Snippet only: https://atlassian.design/foundations/motion/applying-motion
- [S] W3C Design Tokens Community Group format (2025.10 draft) defines `duration` ({value, unit ms|s}), `cubicBezier` ([x1,y1,x2,y2], y may exceed 0..1 to allow overshoot) and a composite `transition` {duration, delay, timingFunction}. Unresolved: transition does not say which property is animated (issue 103). It is a community report, not a W3C standard. https://www.designtokens.org/tr/drafts/format/
- [S] Fluent 2 states three principles (purposeful, natural, consistent) and names enter/exit, elevation, top-level, container transform; I could not retrieve its token values (snippet only). https://fluent2.microsoft.design/motion
- [S] Apple HIG motion page body was not retrievable (title only). [I] From general knowledge, Apple expresses personality through spring presets (SwiftUI `.smooth` no bounce, `.snappy` slight, `.bouncy` noticeable): a three-step overshoot knob. Treat as unverified. https://developer.apple.com/design/human-interface-guidelines/motion
- [S] Adjective-to-parameter practice (low-quality sources, figures conflict, snippet only): define 3-5 brand motion words, map each to duration/easing/overshoot; luxury 500-800 ms slow, tech/precise 200-400 ms, playful with ease-out-back overshoot 10-20%; "one signature curve for 80% of animations and 3 durations (quick/standard/slow)"; no overshoot for errors or heavy objects. Another source warns that stopping at adjective workshops is the common failure and recommends 3-4 personality dimensions mapped to properties. https://uiux-atlas.pages.dev/lessons/motion/motion-design-language-and-brand-motion-identity ; https://playbooks.com/skills/dylantarre/animation-principles/brand-marketing
- [I] Not found: Shopify Polaris motion tokens (no query result retrieved; do not cite).
- [I] UI durations (70-400 ms) are tuned for interaction latency. Film text animation sits in the same range for accents (open-design: 120-220 ms) but runs longer for arrivals (350-650 ms), so reuse the structure, not the numbers.

## 2. Kinetic typography taxonomies

**Takeaway.** Every consumer tool splits text motion into three independent slots: In, Loop (hold-time ambient), Out. The catalog inside each slot is a few dozen presets that collapse to roughly eight primitive effects (fade, slide/rise, scale-pop, track/spacing, blur-focus, wipe/mask, type-on, flip/rotate) differing mainly in speed, easing and overshoot. For a teaser, a small In list, a very small Out list and Hold = "still or one ambient" is enough.

**Cited findings**

- [S] CapCut organises text animation into In, Out and Loop tabs, combinable on one layer, each with a speed slider. In examples: typewriter, fade, flip-up, scale-up. Loop: wave, bounce, shake. Snippets only; marketing/tutorial pages. https://www.capcut.com/resource/text-animation-video-maker
- [S] After Effects preset packs on the market split into Animate In (about 100), Animate Out (about 80) and In-and-Out (about 20), single-line vs multi-line. Adobe's own page says presets or the Animate menu (per-character animators). The "In/Out" naming is vendor convention, not an official AE category list. Snippet only. https://videohive.net/item/bounce-dynamic-smooth-text-animation-builder-and-presets/19691145 ; https://www.adobe.com/products/aftereffects/text-animation
- [S] Canva's Enter/Exit/Loop and Apple Motion text behaviours: no result found. Unverified.
- [S] Twelve principles applied to type (secondary sources, snippet only): squash/stretch = X/Y scale differences on impact with area kept roughly constant; anticipation = a few frames of reverse motion before a move; overshoot = elastic ease; staging = one action moves at a time. Follow-through / overlap: no type-specific guidance found. https://www.adobe.com/in/creativecloud/roc/blog/video/animation-principles
- [S] open-design's hyperframes captions reference picks highlight/exit by "energy level" (high: karaoke + accent glow + 15% scale pop, exit scatter/drop; medium: subtle karaoke, fade+slide; low: warm slow shift 3% scale, exit collapse) and says emphasis words "always break the pattern" with stronger scale/overshoot. design-templates/hyperframes/references/dynamic-techniques.md in nexu-io/open-design (read via gh API).
- [I] Mapping the principles to the five things we can render: anticipation = tiny pre-move (skip, too short for a teaser); overshoot = a `\t` chain 100 -> 104 -> 100; squash/stretch = unequal `\fscx`/`\fscy`; staging = one line at a time (already a baseline rule); arcs/follow-through = skip.

## 3. How AI / agentic video tools specify motion

**Takeaway.** Nobody maps free adjectives to parameters inside a renderer. Two patterns exist. (1) Prompt-level craft rules: open-design tells the model what each adjective-like quality means (speed = weight, easing = emotion, energy -> transition family) and leaves the rendering to GSAP. (2) Parametric camera controls (Kling, Runway) with numeric axes and a speed/intensity scalar, no adjectives. The workable bridge for shotpilot is a third thing: a short enum the model chooses from, with code resolving enums to numbers. The open-design energy -> transition table is the closest existing precedent and has exactly three levels.

**Cited findings: open-design (nexu-io/open-design, read through `gh api`, Apache-2.0)**

- [S] `plugins/_official/scenarios/od-next-strategy/assets/task-profiles/motion-design.md` (generated from `skills/motion-design/SKILL.md`). Contents:
  - Brief resolution: ask only if the answer changes the story; otherwise state the assumption. A compact beat map `time -> viewer takeaway -> focal object -> transformation -> copy/audio -> exit`, and "a recurring object, shape, line or typographic motif".
  - Timing table: small accent / punctuation 120-220 ms ("decisive snap, then settle"); main element arrival 350-650 ms; meaningful shape transformation 600-1200 ms; large reveal / camera move 900-1800 ms. "Film heuristics, not UI response-time limits."
  - Easing rules: ease-out for arrival, ease-in for motivated exit, smooth in/out for camera; constant speed only for mechanical motion; "playful material can overshoot; precise instruments should not bounce by default"; fast actions get preparation and recovery.
  - Three layers: primary / secondary (weight, causality) / ambient (only if needed; quiet scenes may be still). "Alternate action and rest ... endless maximum intensity makes everything feel equal."
  - Kinetic typography: "let words act out meaning through weight, tracking, direction and transformation. Keep a stable reading window; do not rotate, blur or scatter every line. ... Reveal timing is not reading time."
  - Transitions chosen "because of the relationship between beats", not variety. Defect-to-decision table (slideshow, no weight, unreadable, generic, monotonous...). No numeric aesthetic score.
- [S] `.../task-profiles/hyperframes.md` (v3.1.0). Freeze motion timing, transitions, safe areas in a "Design Spec". Config value **Motion intensity: restrained / standard / intense**: restrained leads with cuts and fades, standard may use translation, scaling and graphic transitions, intense may use continuous camera moves and multi-layer choreography, "but still never unmotivated shake, rotation, or rapid zoom". Text readability: titles at least 1.5 s, about 1 s per 4 Chinese characters plus headroom, 5% padding horizontal, kinetic type never sustained shake/rotation/rapid zoom. "Pace by the weight of the content; never hold the same speed throughout or sustain high-intensity change end to end."
- [S] `design-templates/hyperframes/references/motion-principles.md`: "Easing is emotion, not technique. The transition is the verb. The easing is the adverb." Examples: expo.out = confident, sine.inOut = dreamy, elastic.out = playful. Speed communicates weight: fast 0.15-0.3 s energy/urgency; medium 0.3-0.5 s professional; slow 0.5-0.8 s gravity/luxury; very slow 0.8-2.0 s cinematic/emotional. Variation rules (no more than 2 tweens with the same ease per scene; slowest scene 3x slowest; do not start at t=0, offset 0.1-0.3 s). Build / breathe / resolve split 0-30 / 30-70 / 70-100%. Asymmetry: entrance 0.4 s, exit 0.25 s. Total stagger under 500 ms. Hard cut = "wake up", crossfade = "this continues", slow dissolve = "drift with me".
- [S] `design-templates/hyperframes/references/transitions.md`: **Energy -> primary transition**: Calm (wellness, brand story, luxury) = blur crossfade / focus pull, accent light leak / iris, 0.5-0.8 s, sine.inOut; Medium = push slide / staggered blocks, 0.3-0.5 s, power2-3; High (promos, sports, launch) = zoom through / overexposure, accent staggered blocks / gravity drop, 0.15-0.3 s, power4 / expo. "Pick ONE primary (60-70% of scene changes) + 1-2 accents." Also a **mood -> transition** table: tense/edgy = glitch, VHS, chromatic aberration, ripple; dramatic = zoom through, gravity drop, overexposure, color dip to black; premium = focus pull, dip to black; playful = elastic push, iris. Also a rule: "exit animations are BANNED except on the final scene; the transition is the exit". This rule is the opposite of what a hard-cut teaser needs (see 5(b)).
- [S] `design-templates/hyperframes/references/dynamic-techniques.md`: see section 2 energy table (five levels from Low to High).
- [S] Open-design also lists `skills/emilkowalski-motion`, `motion-frames` and a `motion-kinetic-motion-designer` example (file names only, not read).

**Cited findings: other tools**

- [S] HeyGen HyperFrames: open-source (Apache-2.0), HTML + `data-*` timing + GSAP timeline, deterministic frame-identical render, built for agents, supports caption sync, beat-synced animation, shader transitions. Motion is authored as code, not as vocabulary. Snippet only. https://dev.to/_46ea277e677b888e0cd13/heygen-hyperframes-an-open-source-video-framework-built-for-ai-agents-not-humans-mj8
- [S] Kling camera control (ComfyUI node docs): six numeric parameters (horizontal_movement, vertical_movement, pan, tilt, roll, zoom; at least one non-zero), presets, and slow/medium/fast speed in the Motion Control guide; Runway Gen-3 Alpha Turbo "lets users select direction and intensity" (snippet only, axis list not confirmed). Pattern: direction enum + one intensity scalar. https://docs.comfy.org/built-in-nodes/partner-node/video/kwai_vgi/kling-camera-controls.md ; https://alternativeto.net/news/2024/11/runway-presents-advanced-camera-control-for-gen-3-alpha-turbo
- [S] Remotion, Lottie/Rive, Jitter, Cavalry: not researched in depth (no sources retrieved). [I] They are authoring tools with keyframe/spring parameters; Remotion's `spring({damping, stiffness, mass})` is the same shape as M3 springs. Unverified here.
- [S] Motion briefs (small-studio templates, snippet only): ask look/feel/pace with example URLs/GIFs, how the audience should feel afterwards, likes and dislikes, brand constraints, placement, audio. A sound-design brief guide says to replace "make it exciting" with concrete descriptors of tone, pace and density ("warm, restrained, modern"). https://milanote.com/guide/motion-graphics-brief ; https://hound-studio.com/how-to/guide-on-conducting-surveys-for-animation-clients-ask-smarter-animate-better

## 4. Rendering feasibility with ffmpeg 8.1 (no new dependencies)

**Takeaway.** libass covers every text-entrance type we need in one `Dialogue` line each, with per-line `\t` animation, so `finish.rs` could emit an `.ass` file and use the `ass` filter instead of long drawtext chains. drawtext alone can do fade, slide, size pop and (via expression) overshoot, but not per-character tracking, blur, rotation about an origin, or masks. For transitions, only dip, flash, glitch frames and a zoom punch on the incoming shot keep the cut length; xfade-style crossfades are out.

**ASS override tags** [T: all tags below were rendered with `ass=t.ass` on a 1280x720 canvas and checked on extracted frames; `\kf` verified to render without error; the exact look of `\fry` flip was seen as a tilt only]

| Motion type | Minimal line (text part after `Dialogue: 0,start,end,Style,,0,0,0,,`) |
|---|---|
| Fade in/out | `{\an5\pos(640,360)\fad(400,450)}TEXT` |
| Slide in (move + settle) | `{\an5\move(240,360,640,360,0,300)}TEXT` (move 0-300 ms; add `\t(0,300,\alpha&H00&)` with start `\alpha&HFF&` for fade-slide) |
| Scale punch with overshoot | `{\an5\pos(640,500)\fscx300\fscy300\alpha&HFF&\t(0,120,\fscx100\fscy100\alpha&H00&)\t(120,300,\fscx104\fscy104)\t(300,420,\fscx100\fscy100)}TEXT` |
| Tracking in (letter spacing closes) | `{\an5\pos(640,360)\fsp30\blur8\alpha&HFF&\t(0,600,\fsp0\blur0\alpha&H00&)}TEXT` (a focus-pull feel; the `fsp` shift moves the line's width, so keep `\an5`) |
| Rotate / tilt settle | `{\an5\pos(300,200)\frz-8\fry90\t(0,300,\fry0\frz0)}TEXT` (`\frx`, `\fry` are 3D; keep small) |
| Rotate about a point | `{\an5\pos(900,420)\org(900,520)\frz40\t(0,400,\frz0)}TEXT` |
| Wipe (mask reveal) | `{\an5\pos(640,620)\clip(0,0,0,720)\t(0,500,\clip(0,0,1280,720))}TEXT` |
| Type-on / karaoke | `{\pos(640,300)\kf40}T{\kf40}Y{\kf40}P{\kf40}E` with SecondaryColour = pre-highlight colour (each `\kf` is centiseconds; the sweep shows the pre-colour until reached; for a hard per-character reveal use `\k` plus a transparent secondary colour) |
| Tracking / blur loop while held | `\t` can only run once per range; a loop is built by chaining several `\t` ranges (tiny scale breathing 100 -> 102 -> 100) |
| Exit | `\fad(in,out)` for fade; `\t(end-250,end,\alpha&HFF&\fscx90\fscy90)` for shrink-out; `\t(end-200,end,\clip(...))` for wipe-out |

Constraints, all [S/I]: `\t` accepts an optional accel exponent as its first argument (`\t(t1,t2,accel,tags)`), giving ease-in (`accel>1`) or ease-out (`accel<1`) as a power curve only, not a cubic-bezier; real cubic-bezier needs a chain of short `\t` segments. [I] Per-character delay (stagger) is done by emitting one `Dialogue` per character with its own `\pos`, which needs a monospace-ish layout calculation; skip it and use word-level lines. [I] Font fallback is handled by `fontsdir=` on the `ass` filter; `finish.rs` already resolves a typeface, so the same file path can be reused. [T] `ass` ran correctly with the default Arial; Japanese glyph coverage depends on the chosen font and was not tested.

**drawtext alone** [T: expression `fontsize='90+40*max(0,1-t/0.15)':y='300+60*pow(max(0,1-t/0.4),3)':alpha='min(1,t/0.2)'` rendered; a Windows path in `fontfile=` broke the filtergraph until a local relative font file was used, i.e. keep the existing path escaping in `finish.rs`]

- alpha ramp (fade in/out), x/y expressions (slide, ease-out via `pow`, sine bob for a Hold loop), fontsize expression (scale pop, overshoot via `1+0.08*sin(...)*exp(-...)`), `enable='between(t,a,b)'` (hard cut-in, flicker via `mod(t,0.08)<0.04`), `shadowx`/`borderw` for a glitch double.
- Cannot do: blur, rotation about a point beyond `text_w` tricks, letter-spacing, masks/wipes (workaround: a `drawbox` with matching background colour, fragile), per-character timing.
- [I] A wipe via `\clip` and tracking via `\fsp` are the two effects that most expand the vocabulary and both need ASS.

**Transitions and cut length**

- [T] `xfade` overlaps its inputs: two 2 s clips with `duration=0.5:offset=1.5` produced 3.5 s, not 4 s. That is why crossfades are excluded; it also breaks the take-length contract used by sound and script layout.
- Length-preserving transitions [I, `z.mp4` check [T] for punch]:
  - cut: nothing.
  - dip: fade-out last N frames and fade-in first N frames of the next shot through black (existing 0.25 s). Variants: dip-to-white, dip duration tied to tempo (0.15-0.5 s).
  - flash: `fade=t=in:d=0.12:color=white` on the incoming shot (existing). Variants: strobe (two flashes), overexposure ramp (`eq=brightness` expression decaying over 0.15-0.3 s).
  - zoom punch on the incoming shot: `scale=w='W*(1+0.15*max(0,1-t/0.2))':h=-2:eval=frame,crop=W:H` rendered; length unchanged (1.00 s). Gives "impact" without overlap.
  - glitch frames: replace 2-4 frames at the boundary with `rgbashift`/`hue`/`noise` or a horizontally displaced copy (`crop+pad` offsets); length unchanged because frames are replaced, not added. Not rendered here.
  - match / continue: not possible without overlap; a cut is the answer.
  - whip: a 3-5 frame horizontal motion blur (`tmix` or `boxblur` on the last frames of the outgoing shot and first of the incoming) plus the cut. Not rendered here.

## 5. Proposal

Design rule [I]: the model picks from enums and one or two adjectives; the look's tokens set defaults; code resolves to numbers and clamps to the baseline's reading-time limits. Never let the model write numbers for duration or easing.

### (a) Minimal vocabulary

Text entrance (In), pick 1 per line, 8 values:
`cut` (appears), `fade`, `rise` (small upward slide + fade), `punch` (scale 300% -> 100% with 4% overshoot), `track` (letter spacing closes + blur), `wipe` (left-to-right mask), `type` (character or word reveal), `drop` (heavier slide from above with overshoot, for impact).

Hold, 3 values: `still`, `breathe` (scale 100 -> 102 over the hold), `pulse` (+3% on the beat, only for high energy).

Exit (Out), 3 values: `cut` (disappears, default in a hard-cut teaser), `fade`, `shrink` (fast scale-down + fade). Rule from open-design adapted to us: exits are shorter than entrances (about 0.5x).

Transitions, 6 values, all length-preserving: `cut`, `dip` (black), `dip_white`, `flash`, `punch` (zoom punch on the incoming shot), `glitch` (3 replaced frames). `whip` is optional later.

Scalar knobs (three; every other number derives from them):

- `energy` 0-3 (still / low / mid / high) or named `calm | measured | driving | frantic`. Sets durations, hold-after-landing, transition length, accent frequency. Directly follows open-design's calm / medium / high.
- `expressiveness` `productive | expressive`, from Carbon. Controls overshoot and distortion: productive = no overshoot, no blur, no rotation; expressive = overshoot, track blur, tilt allowed.
- `weight` `light | normal | heavy`. Distance and scale amount (heavy = larger travel/scale, longer settle, "drop"/"punch" favoured). Optional; fold into `energy` if the vocabulary must shrink further.

Derivation table [I, anchored on open-design ranges]:

| energy | in duration | out duration | transition length (dip/flash) | accent cap |
|---|---|---|---|---|
| calm | 0.5-0.8 s | 0.35-0.5 s | dip 0.4-0.6 s | no flash |
| measured | 0.3-0.5 s | 0.2-0.3 s | dip 0.25 s | flash only at title |
| driving | 0.15-0.3 s | 0.1-0.2 s | flash 0.12 s | 1 in 4 cuts |
| frantic | 0.08-0.15 s | 0 (cut) | flash 0.08 s / punch | 1 in 3 cuts |

Hold after landing always satisfies `script::reading_secs` (baseline: at least 1.5 s); the in/out durations are inside the line's total length, never added to it. Time budget per line: in + hold + out = line duration (open-design: "time budget includes entrances, reading holds and exits").

### (b) Sketch of a `motion` block in `tokens.json`

```json
"motion": {
  "energy": "driving",
  "expressiveness": "expressive",
  "weight": "heavy",
  "text": {
    "card":    { "in": "punch", "hold": "still",   "out": "cut" },
    "caption": { "in": "rise",  "hold": "still",   "out": "fade" },
    "count":   { "in": "cut",   "hold": "pulse",   "out": "cut" },
    "title":   { "in": "punch", "hold": "breathe", "out": "fade" }
  },
  "transitions": { "default": "cut", "accent": "flash", "drop": "dip" },
  "easing": { "enter": "out_strong", "exit": "in_soft", "overshoot": 0.04 },
  "stagger": 0.0
}
```

Notes [I]: `transitions` already exists in tokens; the new block keeps it (migration: keep the old key as the source of truth, `motion.transitions` absent). `easing` names are a closed list (`linear`, `out_soft`, `out_strong`, `in_soft`, `inout`) that the renderer maps to `\t` accel exponents (soft = 1.5/0.67, strong = 3/0.33; note accel is a power curve, not bezier); the numeric `overshoot` is derived from `expressiveness` and only overridable by the look author, never by Claude. `text` roles match the roles finish.rs already distinguishes (card, caption, count, title). The block is editorial and must stay out of the fingerprint (same invariant as `finish:`).

Per-line and per-cut overrides in `shots.yaml` (editable by Claude under the existing range rules): `text.motion: punch` (entrance only) and `transition: punch|glitch|...`. Keep this to entrance and transition, because Claude picking a full in/hold/out per line would exceed what it can reliably select.

### (c) Mapping user adjectives (brief) onto it

The brief gets one optional field (e.g. `brief.motion: [adjective, ...]`, at most two) that Claude reads in the planning turn. Claude copies it into a look override only by picking enum values; the table below is what the planning prompt shows, as a translation list (not a checklist, per D-22):

| Adjective(s) | energy | expressiveness | text in / out | transitions | Notes |
|---|---|---|---|---|---|
| gentle, calm, warm | calm | productive | fade, rise / fade | dip, cut | nothing sharper than fade; hold `breathe` |
| eerie, uneasy | calm (long holds) | expressive but no overshoot | `track`, `fade` / `fade` | `dip`, occasional `glitch` | blur and slow tracking; silence and still after motion; no flash |
| tense, dread | measured | productive | `cut`, `fade` / `cut` | `cut`, `dip` at the drop | stillness, abrupt cut-in of text to read as a hit |
| punchy, impact, loud | driving | expressive | `punch`, `drop` / `cut` | `cut` on the beat, `flash`, `punch` | accent cap 1 in 4 |
| frantic, chaotic | frantic | expressive | `cut`, `punch` / `cut` | `glitch`, `punch`, `flash` | still must respect 1.5 s reading and no sustained shake |
| elegant, premium | calm | productive | `fade`, `wipe` / `fade` | `dip` | long in, hold `still` |
| playful, cheerful | measured | expressive | `rise`, `punch` / `shrink` | `cut`, `flash` | overshoot on |
| precise, technical | measured | productive | `wipe`, `type` / `cut` | `cut` | no overshoot, constant-speed wipe |
| epic, cinematic | calm to measured | productive | `fade`, `track` / `fade` | `dip`, `dip_white` | slow, large in the title |

Resolution order [I]: explicit user adjective > look default > genre default. If two adjectives conflict (gentle + punchy), the first is `energy`, the second adjusts `expressiveness` only. Contradictions Claude cannot resolve become one question in the existing discover turn (as D-45 does for other gaps), not a guess. Motion-brief practice (section 3) supports asking for references and dislikes; a "not this" adjective (e.g. "not flashy") would map to disallowing `flash`/`glitch`, which is cheap to implement as a deny-list.

### Open questions and risks [I]

- Calibration: the energy table uses open-design's film ranges, not measurements of the 701 reference teasers. The other agents' measured text-in/out durations and cut-transition frequencies should replace the "derivation" numbers before code is written.
- `\t` accel is a power curve; if real cubic-bezier curves matter (Carbon/M3 parity), generate 4-6 `\t` segments from a bezier in code.
- CJK font coverage for ASS is untested here; the title flicker (currently a drawtext effect) would map to `enable`-style flicker or a chain of `\alpha` `\t` steps.
- Moving to an ASS file changes `finish.rs` output (a hash for the Cut, not a take); it does not touch fingerprints.
