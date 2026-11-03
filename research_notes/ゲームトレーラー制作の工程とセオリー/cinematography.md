# Virtual cinematography rules for an automated UE camera planner (shot size, lens, movement, composition, continuity, engine specifics)

Provenance legend: [S] = from a fetched or searched source (cited inline); [C] = computed by me from pinhole-camera geometry (math is verifiable; inputs are stated assumptions); [I] = my engineering inference or common craft convention with no primary source found (low-medium confidence, treat as tunable defaults). Search coverage was thin: StudioBinder, Mascelli, Katz, Blain Brown and GDC pages were not retrievable (403 or not returned), so book-level claims are NOT directly verified.

## 1. Shot sizes, distances, focal lengths, and computing framing

### Takeaway
Shot size is defined by where the frame cuts the body, not by the lens; distance and focal length jointly produce it, so the planner should solve distance from (shot size, focal length, filmback) with `d = f * H_visible / h_sensor`. Sources agree on the body-cut definitions; the cm distances below are computed, not sourced.

### Cited Findings
- Shot-size ladder by body cut: EWS = subject tiny or not visible, vast setting; WS = head-to-toe with significant environment; FS = head-to-toe, background de-emphasised; MFS roughly knees up; Cowboy = mid-thigh up; MS = waist up; MCU = chest/shoulders up; CU = face or key detail fills frame; ECU = small part (eye, mouth, hand). Effects: EWS scale/isolation/sense of place, often opens a scene; CU intimacy; ECU tension/emphasis — [Filmdaft](https://filmdaft.com/the-most-frequently-asked-questions-about-camera-shots-and-camera-angles/)
- The same shot size can be made with different focal lengths: a CU on 20mm exaggerates the nose/centre of the face, on 200mm it looks flatter and the background seems closer — [Filmdaft](https://filmdaft.com/the-most-frequently-asked-questions-about-camera-shots-and-camera-angles/)
- One guide places close-up work at longer lenses (85-135mm); a forum answer argues focal length is irrelevant to the definition of shot size (framing only). Treat lens numbers as convention — [MasterClass (search snippet)](https://www.masterclass.com/articles/shot-sizes-explained); conflicting view [Shooting People Q&A](https://shootingpeople.org/qna/q/camera-measurements)
- UE CineCamera filmback: SensorWidth/SensorHeight are in mm (min clamp 0.001), SensorAspectRatio is read-only and derived; horizontal FOV derives from focal length + sensor width; `get_horizontal_field_of_view()` exists — [Epic FCameraFilmbackSettings](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/CinematicCamera/FCameraFilmbackSettings?lang=en-US), [UE 4.27 CineCameraActors](https://docs.unrealengine.com/4.27/AnimatingObjects/Sequencer/Cameras/CineCameraActors)
- "Super 35" is not one size: a forum cites 24.889 x 18.66 mm; another user's docs list 22 x 11.88 mm; an ARRI Alexa S35 body is listed at 28.17 x 18.13 mm (low-quality source). A user matched live footage only after entering the sensor size manually rather than using the preset — [UE forum](https://forums.unrealengine.com/t/cinema-camera-focal-length-incorrect/367059), [UE answers](https://answers.unrealengine.com/questions/469322/view.html)

### Inferences
Assumptions: subject 170 cm, S35 filmback 24.89 mm wide, output 16:9 so effective sensor height h_s = 24.89/1.778 = 14.0 mm; full frame 36 mm wide.
- Core formulas [C]: `h_s = sensor_w / aspect`; `HFOV = 2*atan(sensor_w/(2f))`; `VFOV = 2*atan(h_s/(2f))`; `H_visible(d) = d*h_s/f`; solve `d = f*H_visible/h_s`. Distance is camera-to-subject plane (UE: cm from camera location). With a named anchor, forward offset = d.
- HFOV on S35 24.89 mm [C]: 14mm 83.2 deg, 18mm 69.2, 24mm 54.8, 35mm 39.1, 50mm 27.9, 85mm 16.7, 135mm 10.5. Full frame 36 mm [C]: 24mm 73.7, 35mm 54.4, 50mm 39.6, 85mm 23.9. The planner must read sensor_w from the actual filmback, never assume it.
- Target visible height H_visible for a 170 cm person (includes headroom ~5-10%) [I]: ECU ~12 cm, CU ~35-40 cm, MCU ~55-60 cm, MS ~90-100 cm, cowboy ~125 cm, MFS ~140 cm, FS ~190-200 cm, WS ~350-500 cm, EWS >=1500 cm (subject <10% of frame height). As `H_visible = k * subject_height`: CU 0.23, MCU 0.34, MS 0.55, cowboy 0.73, FS 1.15, WS 2.4, EWS >=9.
- Resulting distances [C] (S35 24.89, 16:9, d = f*H/14.0): CU (H=40): 35mm 100 cm, 50mm 143 cm, 85mm 243 cm. MS (H=95): 35mm 238, 50mm 339, 85mm 577 cm. FS (H=195): 24mm 334, 35mm 488, 50mm 696 cm. WS (H=400): 18mm 514, 24mm 686 cm. EWS (H=1600): 24mm ~27 m, 18mm ~20.6 m. So "CU of a 170 cm character at 50 mm = about 140 cm (range 110-180)", at 85 mm about 240 cm. At 2.39:1 (h_s = 10.4 mm) every distance is x1.35 for the same H.
- Check in code: compute framed height of the anchor's bounding height at the planned distance; classify to nearest k; flag a mismatch with the declared size by >25%.
- Face distortion [I]: below ~1 m with <=24mm distortion is visible; for character CUs prefer >=35mm and >=100 cm unless distortion is intended. Clamp d >= ~50 cm (CineCamera min focus distance is a per-lens setting, unverified here).

### Gaps
- No primary source gives a canonical k table or distances (Mascelli/Katz/Brown not retrievable). Epic's default filmback preset values are not listed on the fetched page. How UE handles filmback aspect versus output aspect (constrain aspect / letterbox) is unverified.

## 2. Lens choice conventions and emotional effect (horror, action, epic)

### Takeaway
Wide lenses exaggerate depth, distort the periphery and set subjects within (or dwarfed by) the environment; long lenses compress depth, isolate and flatten. Horror mixes both: wide in tight spaces for oppression, long lens plus negative space for watched-from-afar dread. Supporting sources are DP interviews and blogs, not texts.

### Cited Findings
- Horror DPs: *The Invitation* used a 29mm for a tight CU pushed in on a face, wider lenses even in the heroine's CUs ("I wanted to be able to see around her") so the environment dwarfs her, 1.85:1 to keep set verticality, and lots of jib over a long dinner table. *Men* (Rob Hardy) used Panavision H Series 35mm and 55mm as go-tos with sharp foreground / soft background layering. *Crimes of the Future* used controlled deliberate moves, no handheld, no POV — [ASC "Scare Tactics"](https://theasc.com/article/scare-tactics)
- Wide lens distorts peripheral space (rooms feel vast and oppressive); long lens compresses depth and flattens spatial relationships — [Indie Cinema blog (secondary)](https://blog.indiecinema.co/?p=892229), [Lynda long-lens course](https://www.lynda.com/Premiere-Pro-tutorials/Understanding-long-lenses/105371/122958-4.html?w=0)
- Negative space (empty area around a subject) evokes isolation or dread; Flanagan-style shots hold on empty space before the payoff — [Indie Cinema blog (secondary)](https://blog.indiecinema.co/?p=892229)
- Zoom flattens versus dolly changing perspective; dolly zoom (vertigo) = sudden realisation/unease — [Filmdaft](https://filmdaft.com/the-most-frequently-asked-questions-about-camera-shots-and-camera-angles/)

### Inferences
- Bands for S35 24.89 mm [I]: ultra-wide 12-18 mm (tight interiors, vehicles; keep subject off frame edges); wide 20-28 mm (environment + character, establishing, action travel); normal 35-50 mm (default hero/dialogue, minimal distortion); short tele 65-85 mm (portrait CUs, mild compression); tele 100-200 mm (voyeuristic/surveillance, corridor compression, distant approaching threat). Epic establishing: 14-24 mm low angle or 24-35 mm crane.
- Horror presets [I]: tight room 16-24 mm at 1.5-3 m from subject at eye height (150-170 cm) or low (<60 cm); voyeur 85-135 mm from >=8 m with a foreground occluder; threat reveal = WS 24-35 mm with subject <15% of frame area and large negative space.
- Check: CU with f<24 warns (distortion); EWS with f>85 warns unless a "compression" intent is set. Lens should differ across a cut by >=20 mm or one band (section 5).

### Gaps
- No quantitative lens-to-emotion mapping in a primary text; band edges are convention. Game-trailer-specific lens practice (GDC) not retrieved.

## 3. Camera movement vocabulary, speeds, durations, easing, cancelling moves

### Takeaway
Every move should be motivated (by subject motion or a reveal); dolly-in = intimacy, pull-out = reveal. Sources support motivation and meaning but give no numeric speeds, so the speeds below are tunable defaults.

### Cited Findings
- Dolly: slow push-in draws viewer into a character's emotional state; pull-out emphasises discovery/reveal. Pan: vista, follow, or suspense by revealing/withholding. Tilt up = grand/strong, tilt down = diminishing. Crane/jib = grand openings and closings. Handheld = urgent/documentary. Whip pan = fast pan with blur, urgency/confusion, usable as a transition. Steadicam/gimbal = long flowing shots — [Filmdaft](https://filmdaft.com/the-most-frequently-asked-questions-about-camera-shots-and-camera-angles/)
- All shots, static or moving, should be motivated, especially dolly shots into/out of a set; subject motion is ample motivation to truck in; trucking out is the more "unnatural" move — [Grammar of the Shot (O'Reilly)](https://www.oreilly.com/library/view/grammar-of-the/9780240521213/Chapter06u.html)
- Practitioner preference for easing in/out so a dolly slows at the ends and has force mid-move; camera moves are usually meant to be invisible — [Bricks in Motion forum (weak)](https://bricksinmotion.com/forums/post/375926/)
- Restrained, deliberate, non-handheld moves suit a "classical" look — [ASC](https://theasc.com/article/scare-tactics)

### Inferences
- Rules [I]: one move per shot; direction motivated by the anchor; ease-in-out (smoothstep/cubic) with ~0.3-0.5 s near-zero velocity at each end unless a whip.
- Amounts [I]: dolly_in/out travel 15-40% of start distance for a subtle push (300 cm -> 45-120 cm), up to 60% dramatic; peak speed <=1 m/s intimate, <=3 m/s epic; truck 50-300 cm; crane 100-400 cm (1000+ cm for epic EWS); orbit 15-45 deg over 3-6 s (<=10 deg/s), <=90 deg total.
- Duration [I]: trailer shots 1.5-4 s; a move needs >=1 s; if the move changes framing by <8% of frame size, make the shot static.
- Cancelling across a cut [I]: avoid dolly_in -> dolly_in at the same size (stutter); avoid dolly_out ending on A then dolly_in starting at the same size on B (yo-yo); consecutive trucks keep screen direction; orbit_left then orbit_right on the same subject cancels. Check: adjacent moves with opposite screen-axis direction require a size or angle step.
- Move the camera rather than change focal length within a shot (zoom flattens) unless a vertigo effect is explicitly requested.

### Gaps
- No sourced numeric speeds/durations. Blain Brown's easing text not retrieved. Rack focus semantics unsourced.

## 4. Composition

### Takeaway
Composition is checkable by projecting anchor bounds to screen space, but I found few citable specifics beyond horror negative space and foreground/background focus layering; the numeric thresholds are conventions.

### Cited Findings
- Negative space creates isolation/dread — [Indie Cinema blog](https://blog.indiecinema.co/?p=892229)
- Depth layering: sharp foreground subject with soft background figure draws attention to the reaction (*Men*) — [ASC](https://theasc.com/article/scare-tactics)
- Aspect ratio chosen to serve set verticality (1.85:1 over 2.39:1) — [ASC](https://theasc.com/article/scare-tactics)
- Classical framing excludes distracting functional elements — [ASC](https://theasc.com/article/scare-tactics)

### Inferences (all [I])
- Thirds: put eye/key point on x = 0.333 or 0.667, eye y ~0.333 from top, tolerance +/-0.06; centred symmetric framing only when flagged (horror, one-point perspective).
- Headroom: CU/MCU 3-8% of frame height; MS 8-12%. Lead room: >=1.3x more space in front of a facing/moving subject than behind (15-25% of frame width).
- Negative space for dread: subject bbox <=15-20% of frame, offset to a third.
- Horizon: roll 0 deg default; dutch 10-20 deg only for disorientation (1-4 deg reads as error); do not combine dutch with dolly + orbit.
- Tangents: background edges should not kiss the frame edge or the subject silhouette; flag separation <1-2% of frame.
- Layering: >=2 depth planes for WS/establishing; foreground occluder <=25-30% of frame.
- Geometry: sphere sweep along the whole move path with radius >= near-clip + margin (>=20-30 cm); reject cameras inside collision geometry; ray from camera to anchor head height must hit the anchor first unless an occluder is deliberate. Default UE near clip ~10 cm (unverified for CineCamera).

### Gaps
- No primary numerics for thirds/headroom/dutch thresholds; Mascelli/Katz inaccessible.

## 5. Continuity and sequencing

### Takeaway
Successive shots of the same subject should differ by >=30 deg of camera bearing, or by a shot-size step / >=20 mm lens change, to read as deliberate; the 180 deg rule preserves screen direction but is a breakable guideline.

### Cited Findings
- 30 deg rule: when cutting between shots of the same subject, move the camera >=30 deg around it; less risks a jump cut. Variant "20mm/30 degree": a focal change under ~20 mm at the same angle looks similar. The axial cut (same angle) is an exception that relies on the 20mm rule. Rationale: the cut needs a "substantially different view" for motivation. Murch: full figure -> slightly tighter ankles-up is too subtle. Post-continuity montage uses jump cuts deliberately — [Wikipedia 30-degree rule](https://en.wikipedia.org/wiki/30-degree_rule)
- 180 deg rule: the axis of action is the line between two subjects; camera stays on one half-circle so screen positions are preserved; crossing swaps screen sides. Deliberate breaks: Godard, Kubrick's *The Shining*, Ozu's 360-degree space. Legitimate crossings: move the actors, a slow in-shot track across the line, a neutral shot along the axis, or a cutaway — [Filmdaft 180 deg rule](https://filmdaft.com/understanding-the-180-degree-rule-in-media/)
- Under-30-deg changes look like jump cuts; deliberate use gives urgency/unrest — [RouteNote](https://licensing.routenote.com/blog/what-is-the-30-degree-rule/)

### Inferences
- Adjacent shots A->B on the same anchor [I]: pass if bearing difference around the anchor >=30 deg OR |size step| >=1 rung on CU-MCU-MS-MWS-WS-EWS OR (|delta f| >=20 mm AND >=15% change in H_visible). Fail = jump cut. For ~2 s trailer shots prefer >=2 rungs or >=45 deg.
- Progression [I]: wide -> medium -> close builds a scene; in escalation allow shrinking sizes; no more than 2 consecutive shots of the same size; no 3 consecutive shots in the same lens band; alternate moving and static.
- 180 deg [I]: axis from two anchors (or anchor + facing); the sign of cross(axis, camera offset) in XY must stay constant through a sequence; flag flips unless the shot is neutral (camera within ~10 deg of the axis) or the camera crosses in-shot.
- Screen direction [I]: a subject moving screen-left in A moves screen-left in B (compute from projected velocity sign).
- Contrast [I]: consecutive shots should differ in >=2 of {size rung, bearing >=30 deg, lens band, move vs static, high/low angle}. Opening hook and title cards may break this.

### Gaps
- Mascelli's "five C's" and Katz not retrieved; trailer cutting rhythm unsourced.

## 6. Virtual camera specifics: collision, near clip, DOF, shutter, fps, aspect

### Takeaway
Use the real CineCamera physical parameters (filmback, focal length, aperture, focus distance) and a film-style 180 deg shutter; the UE API facts are sourced, while fps/aspect/DOF defaults are conventions.

### Cited Findings
- CineCameraComponent exposes filmback (mm), focal length and a horizontal-FOV getter — [Epic Python API 5.3](https://docs.unrealengine.com/5.3/en-US/PythonAPI/class/CineCameraComponent.html), [UE 4.27 docs](https://docs.unrealengine.com/4.27/AnimatingObjects/Sequencer/Cameras/CineCameraActors)
- 180 deg shutter: shutter ~ 1/(2*fps) = 1/48 s at 24 fps; to match 24 fps blur at 60 fps use a 144 deg shutter; 60 fps with a fast shutter looks crisp and less cinematic — [DIY Photography](https://www.diyphotography.net/?p=309010), [Fora Soft](https://www.forasoft.com/learn/video-encoding/articles/frame-rate-24-60-120-fps-judder-motion-blur)
- In games blur is post-process; one developer renders 60 fps with 3 blur samples; claims that 24 fps is automatically cinematic are disputed (forum sources, weak) — [Polycount](https://polycount.com/discussion/comment/2174834), [GOG forum](https://www.gog.com/forum/general/should_you_play_video_games_in_24_fps_to_maintain_the_cinematic_effect/post37)
- Horror films chose 1.85:1 over 2.39:1 to preserve verticality — [ASC](https://theasc.com/article/scare-tactics)

### Inferences
- Defaults [I]: 24 (or 30) fps with a 180 deg shutter equivalent (UE motion blur amount ~0.5 is the usual mapping, unverified); 16:9 for platform delivery; optional 2.39:1 letterbox by crop with the same horizontal FOV, so recompute h_s = w/2.39 (distances x1.35).
- DOF [I]: f/1.4-f/2.8 for CU/MCU, f/4-f/8 for WS/EWS; focus distance = camera-to-anchor distance, set explicitly. With circle of confusion ~0.025 mm for S35, a 50mm f/2.8 CU at ~143 cm gives a total DOF of only a few cm to ~15 cm [C-approx]; flag when eye-to-ear depth exceeds DOF unintentionally.
- Collision/near clip [I]: sweep the full move path; keep >=20-30 cm from geometry; anchor must be visible; dolly_in must not end closer than ~40 cm to the anchor or inside the minimum d for its shot size.
- Whip pan <=0.3-0.5 s with strong blur; no shot under 1 s.

### Gaps
- UE defaults (near clip, min focus distance, motion-blur-to-shutter mapping in Movie Render Queue) are unverified; the Epic 4.27 CineCameraActors page returned 403 on fetch (search snippet only). GDC virtual cinematography/in-game cinematic talks were not retrieved.
