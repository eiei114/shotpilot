"""Runs INSIDE Unreal Editor. Do not import from the shotpilot package here.

Launched by shotpilot with the env var SHOTPILOT_JOB pointing at a job JSON file:

  build / pull / scan : UnrealEditor-Cmd <uproject> -run=pythonscript -script=<this file>
  render              : UnrealEditor-Cmd <uproject> -ExecutePythonScript=<this file>
                        (full editor; MRQ renders through the PIE executor, then quits)

The result is written next to the job as <job>.result.json. The API surface used here
is the Sequencer scripting + MRQ Python API shipped with UE 5.x; calls that changed
between engine versions go through small fallbacks (_call_first).
"""

import json
import os
import time
import traceback

import unreal

JOB_PATH = os.environ.get("SHOTPILOT_JOB", "")
EAL = unreal.EditorAssetLibrary
_KEEP = {}  # keep UObjects alive across async MRQ callbacks


def log(msg):
    unreal.log("[shotpilot] " + str(msg))


def warn(msg):
    unreal.log_warning("[shotpilot] " + str(msg))


def _call_first(obj, names, *args):
    for n in names:
        fn = getattr(obj, n, None)
        if fn is not None:
            return fn(*args)
    raise AttributeError("none of %s on %s" % (names, obj))


def result_path():
    return JOB_PATH[:-5] + ".result.json" if JOB_PATH.endswith(".json") else JOB_PATH + ".result.json"


def write_result(res):
    res["finished_at"] = time.strftime("%Y-%m-%dT%H:%M:%S")
    with open(result_path(), "w", encoding="utf-8") as f:
        json.dump(res, f, indent=2, ensure_ascii=False)
    log("result -> " + result_path())


def split_asset(path):
    folder, name = path.rsplit("/", 1)
    return folder, name


def object_path(path):
    """/Game/A/B -> /Game/A/B.B"""
    return path if "." in path.rsplit("/", 1)[1] else path + "." + path.rsplit("/", 1)[1]


# ------------------------------------------------------------------ world / actors


def editor_world():
    try:
        return unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    except Exception:
        return unreal.EditorLevelLibrary.get_editor_world()


def ensure_map(map_path):
    if not map_path:
        return
    world = editor_world()
    if world is not None and world.get_path_name().split(".")[0] == map_path:
        return
    log("loading map " + map_path)
    try:
        unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level(map_path)
    except Exception:
        unreal.EditorLoadingAndSavingUtils.load_map(map_path)


def all_actors():
    try:
        return unreal.get_editor_subsystem(unreal.EditorActorSubsystem).get_all_level_actors()
    except Exception:
        return unreal.EditorLevelLibrary.get_all_level_actors()


def find_actor(label):
    for a in all_actors():
        if a.get_actor_label() == label or a.get_name() == label:
            return a
    return None


# ------------------------------------------------------------------ sequences


def add_root_track(seq, cls):
    return _call_first(seq, ["add_track", "add_master_track"], cls)


def root_tracks(seq):
    return _call_first(seq, ["get_tracks", "get_master_tracks"])


def binding_id(seq, binding):
    for fn in ("get_binding_id", "get_portable_binding_id"):
        f = getattr(seq, fn, None)
        if f is None:
            continue
        try:
            return f(binding)
        except TypeError:
            return f(seq, binding)
    bid = unreal.MovieSceneObjectBindingID()
    bid.set_editor_property("Guid", binding.get_id())
    return bid


TRANSFORM_NAMES = ["Location.X", "Location.Y", "Location.Z", "Rotation.X", "Rotation.Y", "Rotation.Z",
                   "Scale.X", "Scale.Y", "Scale.Z"]


def transform_channels(section):
    chans = section.get_all_channels()
    by_name = {}
    for i, c in enumerate(chans):
        name = str(c.get_name())
        # UE 5.x names channels like "Location.X_0"
        key = next((n for n in TRANSFORM_NAMES if name == n or name.startswith(n + "_")), None)
        if key is None:
            key = TRANSFORM_NAMES[i] if i < len(TRANSFORM_NAMES) else name
        by_name[key] = c
    return by_name


def xform_values(xf):
    loc, rot = xf["location"], xf["rotation"]  # rotation = [pitch, yaw, roll]
    return {"Location.X": loc[0], "Location.Y": loc[1], "Location.Z": loc[2],
            "Rotation.X": rot[2], "Rotation.Y": rot[0], "Rotation.Z": rot[1]}


def key(channel, frame, value, interp):
    channel.add_key(unreal.FrameNumber(int(frame)), float(value), 0.0, unreal.MovieSceneTimeUnit.DISPLAY_RATE, interp)


def set_default(channel, value):
    try:
        channel.set_default(float(value))
    except Exception:
        pass


def camera_component(actor):
    """Spawnable templates return None from get_cine_camera_component(); the property works."""
    comp = None
    try:
        comp = actor.get_cine_camera_component()
    except Exception:
        pass
    return comp or actor.get_editor_property("camera_component")


def camera_binding(seq):
    for b in seq.get_bindings():
        if str(b.get_display_name()) == "SP_Camera":
            return b
    for b in _call_first(seq, ["get_spawnables"]):
        t = b.get_object_template()
        if t and isinstance(t, unreal.CineCameraActor):
            return b
    return None


def readback(seq):
    """Engine -> payload subset (frames, fps, start, end, lens). Used for pull and divergence checks."""
    rate = seq.get_display_rate()
    fps = rate.numerator / float(rate.denominator or 1)
    start_f, end_f = seq.get_playback_start(), seq.get_playback_end()
    out = {"frames": int(end_f - start_f), "fps": fps}
    cam = camera_binding(seq)
    if cam is None:
        out["error"] = "no SP_Camera binding"
        return out
    tracks = cam.find_tracks_by_type(unreal.MovieScene3DTransformTrack)
    if tracks and tracks[0].get_sections():
        ch = transform_channels(tracks[0].get_sections()[0])
        first, last = {}, {}
        for n in TRANSFORM_NAMES[:6]:
            c = ch.get(n)
            if c is None:
                continue
            keys = c.get_keys()
            if keys:
                ks = sorted(keys, key=lambda k: k.get_time().frame_number.value)
                first[n], last[n] = ks[0].get_value(), ks[-1].get_value()
            else:
                d = c.get_default() if hasattr(c, "get_default") else 0.0
                first[n] = last[n] = d
        def xf(v):
            return {"location": [v.get("Location.X", 0), v.get("Location.Y", 0), v.get("Location.Z", 0)],
                    "rotation": [v.get("Rotation.Y", 0), v.get("Rotation.Z", 0), v.get("Rotation.X", 0)]}
        out["start"], out["end"] = xf(first), xf(last)
    t = cam.get_object_template()
    if t is not None:
        comp = camera_component(t)
        lens = {"focal_length": comp.get_editor_property("current_focal_length"),
                "aperture": comp.get_editor_property("current_aperture")}
        fs = comp.get_editor_property("focus_settings")
        try:
            lens["focus_distance"] = fs.get_editor_property("manual_focus_distance")
        except Exception:
            pass
        out["lens"] = lens
    return out


def build_shot(item, project, force):
    path = item["asset"]
    payload = item["payload"]
    if EAL.does_asset_exist(path):
        if item.get("base") is not None and not force:
            seq = unreal.load_asset(path)
            rb = readback(seq)
            if not same(rb, item["base"]):
                warn("%s was edited in the editor since the last push; not overwriting" % item["id"])
                return {"status": "diverged", "engine": rb}
        EAL.delete_asset(path)
    folder, name = split_asset(path)
    seq = unreal.AssetToolsHelpers.get_asset_tools().create_asset(
        name, folder, unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
    fps, frames = int(payload["fps"]), int(payload["frames"])
    seq.set_display_rate(unreal.FrameRate(fps, 1))
    seq.set_playback_start(0)
    seq.set_playback_end(frames)

    cam = seq.add_spawnable_from_class(unreal.CineCameraActor)
    cam.set_display_name("SP_Camera")
    template = cam.get_object_template()
    comp = camera_component(template)
    lens = payload["lens"]
    sw = float(lens.get("sensor_width", 36.0))
    fb = comp.get_editor_property("filmback")
    fb.set_editor_property("sensor_width", sw)
    fb.set_editor_property("sensor_height", sw * 9.0 / 16.0)
    comp.set_editor_property("filmback", fb)
    comp.set_editor_property("current_focal_length", float(lens["focal_length"]))
    comp.set_editor_property("current_aperture", float(lens.get("aperture", 2.8)))
    if lens.get("focus_distance"):
        fs = comp.get_editor_property("focus_settings")
        fs.set_editor_property("focus_method", unreal.CameraFocusMethod.MANUAL)
        fs.set_editor_property("manual_focus_distance", float(lens["focus_distance"]))
        comp.set_editor_property("focus_settings", fs)

    interp = (unreal.MovieSceneKeyInterpolation.LINEAR if payload.get("curve") == "linear"
              else unreal.MovieSceneKeyInterpolation.AUTO)
    tr = cam.add_track(unreal.MovieScene3DTransformTrack)
    sec = tr.add_section()
    sec.set_range(0, frames)
    ch = transform_channels(sec)
    a, b = xform_values(payload["start"]), xform_values(payload["end"])
    for n, v in a.items():
        key(ch[n], 0, v, interp)
        if frames > 1 and abs(b[n] - v) > 1e-4:
            key(ch[n], frames - 1, b[n], interp)
    for n in ("Scale.X", "Scale.Y", "Scale.Z"):
        if n in ch:
            set_default(ch[n], 1.0)

    cc = add_root_track(seq, unreal.MovieSceneCameraCutTrack)
    cs = cc.add_section()
    cs.set_range(0, frames)
    cs.set_camera_binding_id(binding_id(seq, cam))

    env = payload.get("environment") or {}
    sun_label = project.get("sun_actor")
    if sun_label and env.get("sun_elevation") is not None:
        sun = find_actor(sun_label)
        if sun is None:
            warn("sun_actor '%s' not found in level; time_of_day ignored" % sun_label)
        else:
            sb = seq.add_possessable(sun)
            st = sb.add_track(unreal.MovieScene3DTransformTrack).add_section()
            st.set_range(0, frames)
            sch = transform_channels(st)
            loc, scl = sun.get_actor_location(), sun.get_actor_scale3d()
            vals = {"Location.X": loc.x, "Location.Y": loc.y, "Location.Z": loc.z,
                    "Rotation.X": 0.0, "Rotation.Y": -float(env["sun_elevation"]),
                    "Rotation.Z": float(env.get("sun_yaw", 45.0)),
                    "Scale.X": scl.x, "Scale.Y": scl.y, "Scale.Z": scl.z}
            for n, v in vals.items():
                if n in sch:
                    set_default(sch[n], v)

    for m in payload.get("markers") or []:
        try:
            tick = seq.get_tick_resolution()
            t = int(m["frame"] * (tick.numerator / float(tick.denominator)) / fps)
            mf = unreal.MovieSceneMarkedFrame()
            mf.set_editor_property("frame_number", unreal.FrameNumber(t))
            mf.set_editor_property("label", m["label"])
            seq.add_marked_frame(mf)
        except Exception as e:  # markers are nice-to-have
            warn("marker skipped: %s" % e)

    EAL.save_loaded_asset(seq)
    return {"status": "built", "engine": readback(seq)}


def same(rb, base, tol_loc=0.5, tol_rot=0.05, tol_lens=0.01):
    if rb.get("error") or rb.get("frames") != base.get("frames"):
        return False
    for k in ("start", "end"):
        if k not in rb:
            return False
        for i in range(3):
            if abs(rb[k]["location"][i] - base[k]["location"][i]) > tol_loc:
                return False
            d = (rb[k]["rotation"][i] - base[k]["rotation"][i] + 180.0) % 360.0 - 180.0
            if abs(d) > tol_rot:
                return False
    for k in ("focal_length", "aperture"):
        if abs(float(rb.get("lens", {}).get(k, 0)) - float(base["lens"].get(k, 0))) > tol_lens:
            return False
    return True


def build_master(master, fps):
    path = master["asset"]
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    folder, name = split_asset(path)
    seq = unreal.AssetToolsHelpers.get_asset_tools().create_asset(
        name, folder, unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
    seq.set_display_rate(unreal.FrameRate(int(fps), 1))
    track = add_root_track(seq, unreal.MovieSceneCinematicShotTrack)
    t = 0
    for s in master["shots"]:
        sub = unreal.load_asset(s["asset"])
        if sub is None:
            warn("master: missing " + s["asset"])
            continue
        sec = track.add_section()
        sec.set_sequence(sub)
        sec.set_range(t, t + int(s["frames"]))
        sec.set_shot_display_name(s["id"])
        t += int(s["frames"])
    seq.set_playback_start(0)
    seq.set_playback_end(t)
    EAL.save_loaded_asset(seq)
    return {"asset": path, "frames": t}


def run_build(job):
    project = job["project"]
    ensure_map(project.get("map"))
    out = {"shots": {}}
    for item in job.get("build", []):
        try:
            out["shots"][item["id"]] = build_shot(item, project, job.get("force", False))
        except Exception:
            out["shots"][item["id"]] = {"status": "error", "error": traceback.format_exc()}
    if job.get("master"):
        try:
            out["master"] = build_master(job["master"], job["master"]["fps"])
        except Exception:
            out["master"] = {"error": traceback.format_exc()}
    return out


def run_pull(job):
    out = {"shots": {}}
    for item in job["pull"]:
        if not EAL.does_asset_exist(item["asset"]):
            out["shots"][item["id"]] = None
            continue
        out["shots"][item["id"]] = readback(unreal.load_asset(item["asset"]))
    return out


def run_scan(job):
    ensure_map(job["project"].get("map"))
    tag, prefix = job.get("tag", "shotpilot"), job.get("prefix", "SP_")
    anchors = {}
    for a in all_actors():
        label = a.get_actor_label()
        tagged = any(str(t) == tag for t in a.tags)
        if tagged or label.startswith(prefix):
            loc = a.get_actor_location()
            name = label[len(prefix):] if label.startswith(prefix) else label
            anchors[name.lower().replace(" ", "_")] = [round(loc.x, 1), round(loc.y, 1), round(loc.z, 1)]
    return {"anchors": anchors}


# ------------------------------------------------------------------ render (async, full editor)


def _output_class(fmt):
    return {"jpg": unreal.MoviePipelineImageSequenceOutput_JPG,
            "exr": unreal.MoviePipelineImageSequenceOutput_EXR}.get(fmt, unreal.MoviePipelineImageSequenceOutput_PNG)


def _ranges(frames):
    """[0,1,2,7,8] -> [(0,3),(7,9)] half-open"""
    out, start, prev = [], None, None
    for f in sorted(frames):
        if start is None:
            start = prev = f
        elif f == prev + 1:
            prev = f
        else:
            out.append((start, prev + 1))
            start = prev = f
    if start is not None:
        out.append((start, prev + 1))
    return out


def configure_job(job, item, map_path):
    q = item["quality"]
    job.set_editor_property("job_name", "%s_%s" % (item["id"], item["quality_name"]))
    job.sequence = unreal.SoftObjectPath(object_path(item["asset"]))
    job.map = unreal.SoftObjectPath(object_path(map_path))
    cfg = job.get_configuration()
    out = cfg.find_or_add_setting_by_class(unreal.MoviePipelineOutputSetting)
    out.output_resolution = unreal.IntPoint(int(item["resolution"][0]), int(item["resolution"][1]))
    out.output_directory = unreal.DirectoryPath(item["out_dir"].replace("\\", "/"))
    out.file_name_format = "frame.{frame_number}"
    out.zero_pad_frame_numbers = 4
    out.override_existing_output = True
    out.flush_disk_writes_per_shot = True
    s, e = item["range"]
    out.use_custom_playback_range = True
    out.custom_start_frame = int(s)
    out.custom_end_frame = int(e)
    cfg.find_or_add_setting_by_class(unreal.MoviePipelineDeferredPassBase)
    cfg.find_or_add_setting_by_class(_output_class(q.get("format", "png")))
    aa = cfg.find_or_add_setting_by_class(unreal.MoviePipelineAntiAliasingSetting)
    aa.spatial_sample_count = int(q.get("spatial_samples", 1))
    aa.temporal_sample_count = int(q.get("temporal_samples", 1))
    aa.engine_warm_up_count = int(q.get("warmup_frames", 0))
    if aa.temporal_sample_count > 1:
        aa.override_anti_aliasing = True
        aa.anti_aliasing_method = unreal.AntiAliasingMethod.AAM_NONE
    sp = q.get("screen_percentage")
    if sp:
        cv = cfg.find_or_add_setting_by_class(unreal.MoviePipelineConsoleVariableSetting)
        try:
            cv.add_or_update_console_variable("r.ScreenPercentage", float(sp))
        except Exception:
            cv.console_variables["r.ScreenPercentage"] = float(sp)
    cfg.initialize_transient_settings()


def keep_alive(flag):
    try:
        unreal.EditorPythonScripting.set_keep_python_script_alive(bool(flag))
    except Exception:
        warn("EditorPythonScripting.set_keep_python_script_alive unavailable: " + traceback.format_exc())


def finish_render_process():
    keep_alive(False)  # lets the -ExecutePythonScript runner close the editor
    unreal.SystemLibrary.quit_editor()


def run_render(job):
    result = {"build": None, "renders": {}}
    if job.get("build") or job.get("master"):
        result["build"] = run_build(job)
    blocked = {sid for sid, r in (result["build"] or {}).get("shots", {}).items() if r.get("status") != "built"}

    subsystem = unreal.get_editor_subsystem(unreal.MoviePipelineQueueSubsystem)
    queue = subsystem.get_queue()
    queue.delete_all_jobs()
    started = time.time()
    index = {}
    for item in job["render"]:
        if item["id"] in blocked:
            result["renders"][item["key"]] = {"status": "error", "error": "build failed or diverged"}
            continue
        for rng in _ranges(item["frames"]):
            j = queue.allocate_new_job(unreal.MoviePipelineExecutorJob)
            it = dict(item)
            it["range"] = rng
            configure_job(j, it, job["project"]["map"])
            index[j.get_editor_property("job_name")] = item["key"]
        result["renders"][item["key"]] = {"status": "pending"}

    if not queue.get_jobs():
        write_result(result)
        finish_render_process()
        return

    def on_job(params):
        try:
            name = params.job.get_editor_property("job_name")
            k = index.get(name)
            if k and result["renders"][k]["status"] != "error":
                result["renders"][k]["status"] = "ok" if params.success else "error"
        except Exception:
            warn(traceback.format_exc())

    def on_done(executor, success):
        for k, r in result["renders"].items():
            if r["status"] == "pending":
                r["status"] = "ok" if success else "error"
        result["seconds"] = round(time.time() - started, 1)
        write_result(result)
        finish_render_process()

    # -ExecutePythonScript quits the editor as soon as this script returns unless told
    # to keep it alive; MRQ renders asynchronously after we return.
    keep_alive(True)
    executor = unreal.MoviePipelinePIEExecutor(subsystem)
    executor.on_individual_job_work_finished_delegate.add_callable_unique(on_job)
    executor.on_executor_finished_delegate.add_callable_unique(on_done)
    _KEEP["executor"], _KEEP["on_job"], _KEEP["on_done"] = executor, on_job, on_done
    subsystem.render_queue_with_executor_instance(executor)


def main():
    if not JOB_PATH or not os.path.exists(JOB_PATH):
        unreal.log_error("[shotpilot] SHOTPILOT_JOB not set or missing: %r" % JOB_PATH)
        return
    with open(JOB_PATH, encoding="utf-8") as f:
        job = json.load(f)
    kind = job["kind"]
    log("job %s (%s)" % (JOB_PATH, kind))
    try:
        if kind == "render":
            run_render(job)  # writes its own result asynchronously
            return
        res = {"build": run_build, "pull": run_pull, "scan": run_scan}[kind](job)
        write_result(res)
    except Exception:
        write_result({"fatal": traceback.format_exc()})
        if kind == "render":
            finish_render_process()


main()
