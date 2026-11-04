# UE Python: PIE visibility, asset dependencies, warm-up for per-shot MRQ jobs (UE 5.5-5.8)

Method note: Epic Python API pages were fetched for 5.7 (the only version page that loaded reliably). Items marked UNVERIFIED come from background knowledge or could not be confirmed in this session. Not tested on a real editor.

## 1. Getting the PIE world and its actors from Python

### Takeaway
`unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_game_world()` exists in 5.7 and is the non-deprecated route; `EditorLevelLibrary.get_game_world()` is deprecated; `EditorLevelLibrary.get_pie_worlds(include_dedicated_server)` also exists. Editor-side actor lookups (`EditorActorSubsystem.get_all_level_actors`) deliberately exclude PIE actors, so enumerate from the game world explicitly. Whether `GameplayStatics.get_all_actors_of_class(world, Actor)` works was not confirmed here.

### Cited Findings
- `UnrealEditorSubsystem` 5.7 methods: `get_editor_world() -> World`, `get_game_world() -> World` (no description), `get_level_viewport_camera_info`, `set_level_viewport_camera_info`. — [UnrealEditorSubsystem 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/UnrealEditorSubsystem?application_version=5.7)
- `EditorLevelLibrary.get_game_world()` is marked deprecated ("Use the function in Unreal Editor Subsystem"); `get_pie_worlds(include_dedicated_server: bool) -> Array[World]` is listed and not deprecated; `editor_end_play()` not deprecated. — [EditorLevelLibrary 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/EditorLevelLibrary?application_version=5.7)
- Python docs state the editor has many Worlds (edited level, each PIE instance, tools with viewports), and `EditorActorSubsystem.get_all_level_actors` excludes actors in PIE/preview worlds. — [EditorActorSubsystem](https://api.unrealengine.com/INT/PythonAPI/class/EditorActorSubsystem.html) (search-result summary, page not re-read)
- Python Actor class has `get_actor_bounds(only_colliding_components, include_from_child_actors=False) -> (origin, box_extent)`, `get_components_by_class(component_class)`, `get_actor_label()`, and an `actor_guid` property (no `get_actor_guid` method). — [Actor 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/Actor?application_version=5.7)

### Inferences
- Robust pattern (not tested): `ues = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem); world = ues.get_game_world()`; if None, fall back to `unreal.EditorLevelLibrary.get_pie_worlds(False)[0]`. Then `unreal.GameplayStatics.get_all_actors_of_class(world, unreal.Actor)` (UNVERIFIED in 5.5-5.8; `GameplayStatics` is Blueprint-exposed so likely available) or `world.get_level(...)`-style iteration (UNVERIFIED).
- The world is valid only while PIE is running, so queries must happen inside an MRQ callback or tick, not before `render` is called.

### Gaps
- No source confirmed `get_all_actors_of_class` against the PIE world in 5.5-5.8, or differences between 5.5 and 5.8 for `get_game_world`.

## 2. Hooks during MRQ rendering and knowing the current shot

### Takeaway
Per-job and per-shot "finished" delegates exist on `MoviePipelinePIEExecutor` and `MoviePipeline`, but the shot one only fires if `flush_disk_writes_per_shot` is enabled in the output setting. Those fire after the shot, so they are too late to sample the PIE world unless the world is still alive (UNVERIFIED). No frame-level or current-shot accessor was found in the Python docs; a Slate tick callback plus a pipeline reference is the likely route (UNVERIFIED).

### Cited Findings
- `MoviePipelinePIEExecutor` 5.7 delegates: `on_executor_errored_delegate`, `on_executor_finished_delegate`, `on_individual_job_started_delegate` (called right before a job initializes a UMoviePipeline), `on_individual_job_finished_delegate`, `on_individual_job_work_finished_delegate`, `on_individual_shot_work_finished_delegate` (params list files written for that shot; requires a flag in the output settings). Methods: `is_rendering_offscreen`, `set_is_rendering_offscreen`, `set_allow_using_unsaved_levels`, `set_initialization_time`. `get_active_movie_pipeline` is not on this page. — [MoviePipelinePIEExecutor 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/MoviePipelinePIEExecutor?application_version=5.7)
- C++ docs: the shot-finished event is only called if the pipeline is set up correctly, usually `bFlushDiskWritesPerShot = true` in `UMoviePipelineOutputSetting` for each job. — [OnIndividualShotWorkFinished (C++)](https://dev.epicgames.com/documentation/unreal-engine/API/Plugins/MovieRenderPipelineEditor/UMoviePipelinePIEExecutor/OnIndividualShot-_2)
- `unreal.MoviePipeline` (5.7) exposes `get_current_job()`, `get_pipeline_primary_config()`, `on_movie_pipeline_shot_work_finished_delegate` (fires per shot after flush; only if FlushDiskWritesPerShot), `on_movie_pipeline_work_finished_delegate`, `initialize(job)`. No `get_pipeline_state`, active-shot, or frame accessor on the page. — [MoviePipeline 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/MoviePipeline?application_version=5.7)

### Inferences
- Because each shotpilot shot is its own job, "which shot is rendering" is simply the job: bind `on_individual_job_started_delegate` and remember `job.job_name` / sequence path, then use `unreal.register_slate_post_tick_callback(fn)` (UNVERIFIED in docs here; known editor Python API) to sample the PIE world a few ticks after start and unregister afterward.
- With one job per shot, sampling once after the warm-up (the middle of the shot) is enough; per-frame sampling is not needed for a static asset list. Moving cameras might want start/mid/end samples.

### Gaps
- Could not verify `register_slate_post_tick_callback` / `pre_tick` signatures, `MoviePipelinePythonHostExecutor`, `subsystem.get_active_executor()`, or how to reach the live `MoviePipeline` from the executor in Python. Pipeline state and current-frame accessors were not found.

## 3. Deciding visibility

### Takeaway
`Actor.was_recently_rendered(tolerance=0.2)` is exposed in Python 5.7, but it also returns true for shadow and other non-screen passes and is unreliable for Nanite meshes in non-editor builds. In PIE-in-editor the Nanite caveat is reported only for cooked builds. A manual camera-frustum test against `get_actor_bounds` is the robust fallback, and you can combine both.

### Cited Findings
- `Actor.was_recently_rendered(tolerance=0.2) -> bool`: true if the actor was rendered within `tolerance` seconds of game time. `get_last_render_time` is not on the Actor page. — [Actor 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/Actor?application_version=5.7)
- `WasRecentlyRendered` returns true if the mesh was rendered in a shadow or any other pass; the on-screen variant `GetLastRenderTimeOnScreen` is not exposed to Blueprint. — [UE forum thread](https://forums.unrealengine.com/t/uprimitivecomponent-getlastrendertime-wasrecentlyrendered-for-nanite-static-meshes-not-working/2672626); [WasRecentlyRendered (C++)](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Runtime/Engine/UPrimitiveComponent/WasRecentlyRendered); [AnswerHub](https://answers.unrealengine.com/questions/570908/view.html)
- In non-editor builds `GetLastRenderTime`/`WasRecentlyRendered` fail for Nanite static meshes because of the always-visible path (`FSceneProxyBase::SupportsAlwaysVisible`); the poster fell back to a manual frustum test (no occlusion). Version not stated. — [UE forum thread](https://forums.unrealengine.com/t/uprimitivecomponent-getlastrendertime-wasrecentlyrendered-for-nanite-static-meshes-not-working/2672626)

### Inferences
- Over-inclusion is acceptable here (a false positive costs one extra re-render; a false negative loses correctness), so shadow-pass inclusion is tolerable. A frustum test (camera location/rotation, `CineCameraComponent` filmback/FOV, aspect) against sphere/box bounds, widened a bit, is deterministic and does not depend on render-thread timing; it ignores occlusion, which again only over-includes.
- A shot whose camera is off-screen but whose shadow falls on screen is a real dependency; `was_recently_rendered` would catch it, frustum would not. Using the union is safest. Suggested.

### Gaps
- `PrimitiveComponent.was_recently_rendered` / `get_last_render_time` were not confirmed in Python (the PrimitiveComponent page was truncated). Whether `was_recently_rendered` behaves under MRQ's offscreen PIE (and with Nanite in editor PIE) was not found.

## 4. From a visible actor to the assets that shape its pixels

### Takeaway
`AssetRegistry.get_dependencies(package_name, AssetRegistryDependencyOptions)` returns on-disk package names; set only `include_hard_package_references=True`. Component-level asset collection (mesh, materials, Niagara, etc.) has to be done by walking components; no source was found enumerating exact Python property names for each component type.

### Cited Findings
- `AssetRegistry.get_dependencies(package_name: Name, dependency_options: AssetRegistryDependencyOptions) -> Array[Name] | None`; `get_referencers(...)` has the same shape; both report on-disk references only. `get_asset_by_object_path` is deprecated in favor of `k2_get_asset_by_object_path(SoftObjectPath, include_only_on_disk_assets=False, skip_ar_filtered_assets=True)`. No DependencyQuery variant appears in the Python docs. — [AssetRegistry Python API](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/AssetRegistry)
- `AssetRegistryDependencyOptions` has `include_hard_package_references`, `include_soft_package_references` (and others); 5.1 docs list all defaults False while 4.27 lists soft/hard True, so always set explicitly. — [AssetRegistryDependencyOptions 5.1](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/AssetRegistryDependencyOptions?application_version=5.1)
- Working example: AYON's Unreal plugin calls `get_dependencies` with only `include_hard_package_references=True` and filters to `/Game/` paths. — [AYON validate_no_dependencies](https://docs.ayon.dev/ayon-unreal/latest/autoapi/client/ayon_unreal/plugins/publish/validate_no_dependencies.html)

### Inferences (UNVERIFIED snippet, to be tested)
```python
ar = unreal.AssetRegistryHelpers.get_asset_registry()
opt = unreal.AssetRegistryDependencyOptions(
    include_soft_package_references=False, include_hard_package_references=True,
    include_searchable_names=False, include_soft_management_references=False,
    include_hard_management_references=False)
def deps(pkg, seen, cap=500, depth=4):
    stack=[(unreal.Name(pkg),0)]
    while stack and len(seen)<cap:
        p,d=stack.pop(); s=str(p)
        if s in seen or s.startswith(("/Engine/","/Script/")): continue
        seen.add(s)
        if d<depth:
            for x in (ar.get_dependencies(p,opt) or []): stack.append((x,d+1))
```
- Per actor: `actor.get_components_by_class(unreal.StaticMeshComponent)` then `comp.static_mesh` and `comp.get_materials()`; `unreal.SkeletalMeshComponent`, `unreal.NiagaraComponent` (`get_asset()`), decals, `unreal.InstancedStaticMeshComponent` all derive from MeshComponent/PrimitiveComponent; use `get_outermost().get_name()` or `get_path_name()` on each asset to get the package name (property/method names not verified per class). The Blueprint class package comes from `actor.get_class().get_outermost().get_name()` (UNVERIFIED); runtime-spawned actors from a C++ class would have a `/Script/...` class, which is skipped.
- Dynamic material instances created at runtime have transient packages (`/Engine/Transient`), so walk to the parent (`get_parent()`/`parent`) to reach a real asset (UNVERIFIED).

### Gaps
- Per-component Python property names, landscape and text-render specifics, and exact `AssetRegistryDependencyOptions` constructor kwargs in 5.5-5.8 were not verified.

## 5. Package name to file on disk, and what changes when a texture changes

### Takeaway
Package-to-file mapping can be done in Rust from `/Game/...` using the Content folder; Python-side helpers were not verified. Virtual Assets move texture bulk data out of the `.uasset` into `Saved/VirtualizedPayloads` only when the feature is enabled; if it is off, a texture's pixels live in the package file(s).

### Cited Findings
- With virtualization on, the `.uasset` shrinks and the bulk payload is stored under the project's `Saved/VirtualizedPayloads`; `DumpPackagePayloadInfo` lists local and virtualized payloads; payloads can be "rehydrated" into the package file. — [Debugging Virtual Assets](https://dev.epicgames.com/documentation/en-us/unreal-engine/debugging-virtual-assets-in-unreal-engine); [Virtual Assets in 5.1](https://www.unrealengine.com/en-US/tech-blog/virtual-assets-in-unreal-engine-5-1-smaller-faster-data-syncs)

### Inferences
- If a project has virtualization off (default, UNVERIFIED), the `.uasset` (plus `.umap` for levels) changes when texture pixels change, so hashing it works. If on, hash the `.uasset` and also look for `.upayload`/hash entries (the package's payload ids are inside the `.uasset` header, so a pixel change should change the `.uasset` bytes anyway, UNVERIFIED).
- Python helpers to try: `unreal.SystemLibrary.get_project_content_directory()` / `unreal.Paths.project_content_dir()`; `/Game/X/Y` maps to `<Content>/X/Y.uasset`; plugin mounts `/PluginName/...` map to `<Plugin>/Content/...` (the Rust side can resolve it from `.uproject`/plugin dirs). `.uexp`/`.ubulk` splitting applies to cooked data; uncooked editor packages are a single file (UNVERIFIED in this session).
- Hashing the file also catches non-visual edits (e.g. metadata/thumbnail changes); the Rust side should accept over-invalidation.

### Gaps
- No source confirmed `.uexp`/`.ubulk` behavior for uncooked packages, `unreal.PackageName` helper availability, or the default of virtualization.

## 6. World Partition / One File Per Actor

### Takeaway
No Python recipe was found for mapping a PIE actor to its editor actor or its `__ExternalActors__` package. Treat as impractical to rely on.

### Cited Findings
- C++ only: `FWorldPartitionActorDesc::GetActorPackage` / `IWorldPartitionActorDescInstance::GetActorPackage` (5.5) return the actor's package; no Python equivalent appeared in search. — [GetActorPackage 5.5](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/WorldPartition/IWorldPartitionActorDescInstance-/GetActorPackage?application_version=5.5)

### Inferences
- The simplest sound approach: hash the `.umap` (and, if used, the `__ExternalActors__/<map>/` directory) from Rust as a whole-level dependency for every shot, trading precision for safety. For levels built at runtime by a GameMode (the target case), the level file hardly holds the actors, so this adds little noise; the GameMode's C++/BP assets are the real dependency.
- PIE actors are duplicated from editor actors, and the `PIE` name prefix on world/package names (`UEDPIE_0_`) can be stripped to map to the editor package (UNVERIFIED).

### Gaps
- Python access to ActorDesc, `is_packaged_externally`, or `get_external_package` is not listed on the Actor page in 5.7; no workaround verified.

## 7. MRQ warm-up and multi-shot reliability

### Takeaway
Use per-shot jobs with `engine_warm_up_count` for ticking systems and `render_warm_up_count` plus `render_warm_up_frames=True` for temporal/GPU effects, and a camera-cut track extended before the playback range with `use_camera_cut_for_warm_up`. Several version-specific bugs exist (warm-up at wrong camera, 5.7 first-shot-only, 5.6/5.7 double frames in Movie Render Graph).

### Cited Findings
- `MoviePipelineAntiAliasingSetting` 5.7 properties: `engine_warm_up_count` (frames at shot start the engine ticks without rendering or GPU submission; cheaper), `render_warm_up_count` (initial frames rendered then discarded to build temporal history; more expensive), `render_warm_up_frames` (bool; submit warm-up frames to GPU, off by default, needed for GPU particles), `use_camera_cut_for_warm_up` (derive warm-up length from excess camera-cut track outside the playback range; sequence evaluated every frame so skeletal meshes leave bind pose), plus `temporal_sample_count`, `spatial_sample_count`, `override_anti_aliasing`, `anti_aliasing_method`. Names confirmed exact. — [MoviePipelineAntiAliasingSetting 5.7](https://dev.epicgames.com/documentation/en-us/unreal-engine/python-api/class/MoviePipelineAntiAliasingSetting?application_version=5.7)
- Movie Render Graph: warm-up frames run at the Player Start rather than the shot camera, first frame dark; Epic could not reproduce in a clean 5.5 template; the poster's workaround was enabling "emulate motion blur" on the Warm Up node; status open. — [Forum thread](https://forums.unrealengine.com/t/warm-up-frames-in-movie-render-graph-rendering-at-player-location/2647810)
- UE 5.7: MRQ stops writing frames after the first shot when changing Sequencer FPS; Epic said fixed in 5.8, a user reports not fixed in 5.8; workarounds: match Sequencer tick interval to the sequence frame rate, adjust `temporal_sample_count` (one repro failed at 9, worked at 8/10/12/16 with 25 fps), or use Movie Render Graph. Trigger unconfirmed. — [Forum thread](https://forums.unrealengine.com/t/movie-render-queue-stops-writing-frames-after-first-shot-when-changing-sequencer-fps-ue-5-7/2677837)
- 5.6.x/5.7.x Movie Render Graph: custom frame range can output double (blended) frames, mostly with 1-frame ranges; Epic-affiliated reply says it samples across shots; workarounds: render the shot instead of a custom range, or change shutter method off frame center. — [Forum thread](https://forums.unrealengine.com/t/5-6-x-5-7-x-movie-render-graph-double-frames-in-outputs-custom-frame-range/2709369)
- Related: a 5.5 MRQ corrupted-frames thread (TSR + Lumen) suggests enabling both "Use Camera Cut for Warm Up" and "Render Warm Up Frames"; plausible lead, unconfirmed. — [Forum thread](https://forums.unrealengine.com/t/ue-5-5-mrq-render-glitch-random-corrupted-frames-tsr-lumen/2695475)

### Inferences
- Suggested starting point for per-shot jobs (not verified by testing): `use_camera_cut_for_warm_up=True` with the camera cut extended N frames before the playback start, `render_warm_up_frames=True`, `engine_warm_up_count` ~ 32+, `render_warm_up_count` ~ 16-32 for Lumen/TSR (the exact numbers are a guess; no source gave numbers), `temporal_sample_count=1` with motion blur off to sidestep the 5.7 first-shot bug. Because each shot is its own job, the multi-shot bug is largely avoided.
- When the level is built at runtime by a GameMode, warm-up must also cover the spawn time; a camera at the player start during warm-up may see an empty level, so check the first frames.

### Gaps
- Official guidance on warm-up counts for Lumen was not found; whether the "player start" warm-up problem affects the classic MRQ (not Graph) with a runtime-spawned level is unconfirmed. 5.8 behavior of these bugs is covered only by forum reports.
