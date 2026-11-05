"""Runs INSIDE Unreal Editor. Do not import from the shotpilot package here.

Launched by shotpilot with the env var SHOTPILOT_JOB pointing at a job JSON file:

  build / pull / scan / assets / survey :
                        UnrealEditor-Cmd <uproject> -run=pythonscript -script=<this file>
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

    for ap in payload.get("appear") or []:
        add_prop(seq, ap, frames)

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


def add_prop(seq, ap, frames):
    """A prop that pops in at ap["frame"]: a spawnable mesh actor with a constant transform and
    a spawn track that turns it on at that frame. No animation (skeletal meshes keep their
    reference pose)."""
    asset = unreal.load_asset(ap["asset"])
    if asset is None:
        raise RuntimeError("prop '%s': asset %s not found" % (ap["prop"], ap["asset"]))
    skeletal = ap.get("kind") == "skeletal"
    b = seq.add_spawnable_from_class(unreal.SkeletalMeshActor if skeletal else unreal.StaticMeshActor)
    b.set_display_name("SP_Prop_" + ap["prop"])
    t = b.get_object_template()
    if skeletal:
        comp = t.get_editor_property("skeletal_mesh_component")
        _call_first(comp, ["set_skeletal_mesh_asset", "set_skeletal_mesh"], asset)
    else:
        comp = t.get_editor_property("static_mesh_component")
        comp.set_static_mesh(asset)

    tr = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
    tr.set_range(0, frames)
    ch = transform_channels(tr)
    vals = xform_values({"location": ap["location"], "rotation": ap["rotation"]})
    vals.update({"Scale.X": 1.0, "Scale.Y": 1.0, "Scale.Z": 1.0})
    for n, v in vals.items():
        if n in ch:
            set_default(ch[n], v)

    spawn = next((x for x in b.get_tracks() if isinstance(x, unreal.MovieSceneSpawnTrack)), None)
    if spawn is None:
        spawn = b.add_track(unreal.MovieSceneSpawnTrack)
    secs = spawn.get_sections()
    sec = secs[0] if secs else spawn.add_section()
    sec.set_range(0, frames)
    chan = sec.get_all_channels()[0]
    start = int(ap.get("frame", 0))
    if start > 0:
        chan.set_default(False)
        chan.add_key(unreal.FrameNumber(0), False)
        chan.add_key(unreal.FrameNumber(start), True)
    else:
        chan.set_default(True)
        chan.add_key(unreal.FrameNumber(0), True)


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


def _actor_entry(a, require_size=False):
    """[x, y, z], or {at, size, center} when the actor has bounds (empty actors have none)."""
    loc = a.get_actor_location()
    at = [round(loc.x, 1), round(loc.y, 1), round(loc.z, 1)]
    try:
        origin, extent = a.get_actor_bounds(False)
        size = [round(2 * extent.x, 1), round(2 * extent.y, 1), round(2 * extent.z, 1)]
        if all(v > 1.0 for v in size):
            return {"at": at, "size": size, "center": [round(origin.x, 1), round(origin.y, 1), round(origin.z, 1)]}
    except Exception:  # bounds are optional: keep the point
        pass
    return None if require_size else at


def _anchor_name(label):
    """An anchor id from an actor label: SM_Old_Well3 -> old_well. Empty when nothing usable is left."""
    s = "".join(c if c.isascii() and (c.isalnum() or c in "_-") else "_" for c in label.lower())
    parts = [p for p in s.split("_") if p]
    if len(parts) > 1 and parts[0] in ("sm", "sk", "skm", "bp", "s", "a"):
        parts = parts[1:]
    while parts and parts[-1].isdigit():
        parts.pop()
    name = "_".join(parts).rstrip("0123456789").strip("_-")
    if name and not name[0].isalpha():
        name = "a_" + name
    return name[:40]


# Actors that are never landmarks: lights, cameras, volumes, sky, the level's own machinery.
_SKIP_CLASS = ("Light", "Camera", "Volume", "Sky", "Fog", "PostProcess", "PlayerStart", "Landscape", "Brush",
               "WorldSettings", "Info", "Reflection", "Decal", "Note", "Atmosphere", "Cloud", "Navigation",
               "LevelSequence", "Emitter", "Niagara", "Audio", "Sound", "Trigger", "Blocking", "HLOD")


def _auto_anchors(limit):
    """Landmarks picked from the level when nothing is tagged: medium-to-large visible meshes,
    one per kind (by label), spread out so the scout photos cover the level."""
    cands = []
    for a in all_actors():
        cls = a.get_class().get_name()
        if any(k in cls for k in _SKIP_CLASS):
            continue
        try:
            if a.is_hidden_ed() or a.is_editor_only():
                continue
        except Exception:
            pass
        try:
            if not a.get_components_by_class(unreal.MeshComponent):
                continue
        except Exception:
            continue
        e = _actor_entry(a, require_size=True)
        if not isinstance(e, dict):
            continue
        sx, sy, sz = e["size"]
        big, flat = max(sx, sy, sz), min(sx, sy, sz)
        # floors, ceilings and whole buildings frame nothing; tiny clutter is not a place
        if big < 40 or big > 2500 or sz < 15:
            continue
        name = _anchor_name(a.get_actor_label())
        if not name:
            continue
        score = min((sx * sy * sz) ** (1.0 / 3.0), 400.0) * (0.5 if flat < 25 else 1.0)
        cands.append((score, name, e))
    best = {}
    for c in cands:  # one per kind: six chairs are one landmark
        if c[1] not in best or c[0] > best[c[1]][0]:
            best[c[1]] = c
    pool = sorted(best.values(), key=lambda c: -c[0])
    picked = []

    def dist(e, f):
        a, b = e["center"], f["center"]
        return ((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2) ** 0.5

    while pool and len(picked) < limit:
        if not picked:
            c = pool[0]
        else:
            # far from what is picked, weighted by how much it is there; nothing closer than 3 m
            far = [(c[0] * min(dist(c[2], p[2]) for p in picked), c) for c in pool]
            far = [x for x in far if min(dist(x[1][2], p[2]) for p in picked) >= 300]
            if not far:
                break
            c = max(far, key=lambda x: x[0])[1]
        picked.append(c)
        pool.remove(c)
    log("scan: %d landmark candidates, picked %s" % (len(cands), [c[1] for c in picked]))
    return {c[1]: c[2] for c in picked}


def run_scan(job):
    ensure_map(job["project"].get("map"))
    tag, prefix = job.get("tag", "shotpilot"), job.get("prefix", "SP_")
    anchors = {}
    for a in all_actors():
        label = a.get_actor_label()
        tagged = any(str(t) == tag for t in a.tags)
        if tagged or label.startswith(prefix):
            name = label[len(prefix):] if label.startswith(prefix) else label
            anchors[name.lower().replace(" ", "_")] = _actor_entry(a)
    if anchors or not job.get("auto"):
        return {"anchors": anchors}
    return {"anchors": _auto_anchors(int(job.get("limit", 6))), "auto": True}


def run_survey(job):
    """Stage 0 in one launch: the anchors (tagged, else picked from the level) and, when asked,
    the mesh inventory."""
    out = run_scan(dict(job, auto=True))
    if job.get("assets"):
        out["assets"] = run_assets(job)["assets"]
    return out


def _bounds_size(asset):
    for fn in ("get_bounds", "get_imported_bounds"):
        f = getattr(asset, fn, None)
        if f is None:
            continue
        try:
            b = f()
            e = b.box_extent
            size = [round(2 * e.x, 1), round(2 * e.y, 1), round(2 * e.z, 1)]
            if all(v > 0.5 for v in size):
                return size
        except Exception:
            continue
    return None


def run_assets(job):
    """Static and skeletal meshes under /Game with their size, for the props catalogue."""
    reg = unreal.AssetRegistryHelpers.get_asset_registry()
    skip = (job["project"].get("content_root") or "/Game/Shotpilot").rstrip("/")
    limit = int(job.get("limit", 400))
    out = []
    for cls, kind in (("StaticMesh", "static"), ("SkeletalMesh", "skeletal")):
        try:
            flt = unreal.ARFilter(class_paths=[unreal.TopLevelAssetPath("/Script/Engine", cls)],
                                  package_paths=["/Game"], recursive_paths=True)
        except Exception:  # older engines
            flt = unreal.ARFilter(class_names=[cls], package_paths=["/Game"], recursive_paths=True)
        for ad in reg.get_assets(flt):
            path = str(ad.package_name)
            if path.startswith(skip + "/") or path.startswith("/Game/ShotpilotTest/"):
                continue
            entry = {"asset": path, "kind": kind, "size": None}
            if len(out) < limit:
                try:
                    entry["size"] = _bounds_size(unreal.load_asset(path))
                except Exception as e:
                    warn("bounds of %s: %s" % (path, e))
            out.append(entry)
    log("assets: %d meshes" % len(out))
    return {"assets": out}


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


def configure_job(job, item, map_path, game_mode=None):
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
    if game_mode:
        # MRQ swaps in its own empty MoviePipelineGameMode by default. Games that build
        # their level at runtime (project.game_mode) need their own mode to run in PIE.
        cls = unreal.load_class(None, game_mode)
        if cls is None:
            raise RuntimeError("project.game_mode not found: " + game_mode)
        go = cfg.find_or_add_setting_by_class(unreal.MoviePipelineGameOverrideSetting)
        # 5.8 reads SoftGameModeOverride; the hard-class property is deprecated and ignored there
        for prop, value in (("soft_game_mode_override", unreal.SoftClassPath(game_mode)),
                            ("soft_game_mode_override", cls), ("game_mode_override", cls)):
            try:
                go.set_editor_property(prop, value)
                log("game mode override %s = %s" % (prop, game_mode))
                break
            except Exception:
                continue
        else:
            raise RuntimeError("could not set the MRQ game mode override on this engine version")
    cfg.find_or_add_setting_by_class(_output_class(q.get("format", "png")))
    aa = cfg.find_or_add_setting_by_class(unreal.MoviePipelineAntiAliasingSetting)
    aa.spatial_sample_count = int(q.get("spatial_samples", 1))
    aa.temporal_sample_count = int(q.get("temporal_samples", 1))
    aa.engine_warm_up_count = int(q.get("warmup_frames", 0))
    # optional, only when the quality preset names them (Lumen and TSR settle over rendered
    # warm-up frames; warming up on the camera cut keeps the warm-up at the shot camera)
    for key, prop, cast in (("render_warmup_frames", "render_warm_up_count", int),
                            ("render_warmup_frames", "render_warm_up_frames", lambda v: int(v) > 0),
                            ("warmup_camera_cut", "use_camera_cut_for_warm_up", bool)):
        if q.get(key) is None:
            continue
        try:
            aa.set_editor_property(prop, cast(q[key]))
        except Exception:
            raise RuntimeError("quality.%s: this engine has no MoviePipelineAntiAliasingSetting.%s" % (key, prop))
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


# ------------------------------------------------------------------ what a shot showed (game content)
#
# While MRQ renders a shot in PIE, a tick callback samples the actors the renderer drew
# (Actor.was_recently_rendered) and collects the content packages behind them: the actor's
# Blueprint class, meshes, materials, Niagara systems, then their hard dependencies
# (materials -> textures). When the job finishes, the packages that are files under the
# project are reported as the shot's `deps`; shotpilot hashes them, so a later change to
# one of them (a texture, a material) re-renders only the shots that showed it.
# Not covered: where actors are placed in an editor level (World Partition keeps them in
# __ExternalActors__ packages we cannot map back from PIE), and game code.

SKIP_ROOTS = ("/Script/", "/Engine/", "/Temp/", "/Memory/", "/Transient", "/Niagara/")
DEPS_LIMIT = 800
_CONTENT_DIRS = {}


def _package_of(obj):
    """The content package an object lives in, or None (engine, transient, PIE copies)."""
    if obj is None:
        return None
    try:
        path = obj.get_path_name() if hasattr(obj, "get_path_name") else str(obj)
    except Exception:
        return None
    pkg = str(path).split(".")[0].split(":")[0]
    if not pkg.startswith("/") or pkg.startswith(SKIP_ROOTS) or "UEDPIE" in pkg:
        return None
    return pkg


def _actor_packages(actor):
    out = set()
    try:
        out.add(_package_of(actor.get_class()))
    except Exception:
        pass
    try:
        comps = actor.get_components_by_class(unreal.ActorComponent)
    except Exception:
        comps = []
    for c in comps:
        for prop in ("static_mesh", "skeletal_mesh_asset", "skeletal_mesh", "asset", "decal_material",
                     "material", "font", "landscape_material"):
            try:
                out.add(_package_of(c.get_editor_property(prop)))
            except Exception:
                continue
        try:
            mats = c.get_materials()
        except Exception:
            mats = []
        for m in mats or []:
            # a dynamic material instance lives in the level; its parent is the asset
            if isinstance(m, unreal.MaterialInstanceDynamic):
                try:
                    m = m.get_editor_property("parent")
                except Exception:
                    continue
            out.add(_package_of(m))
    out.discard(None)
    return out


def _expand(packages, depth=3):
    """Packages plus their hard package dependencies (mesh -> material -> texture)."""
    reg = unreal.AssetRegistryHelpers.get_asset_registry()
    opts = unreal.AssetRegistryDependencyOptions()
    for prop, on in (("include_hard_package_references", True), ("include_soft_package_references", False),
                     ("include_searchable_names", False), ("include_soft_management_references", False),
                     ("include_hard_management_references", False)):
        try:
            opts.set_editor_property(prop, on)
        except Exception:
            pass
    seen, frontier = set(packages), list(packages)
    for _ in range(depth):
        nxt = []
        for p in frontier:
            try:
                deps = reg.get_dependencies(p, opts) or []
            except Exception:
                deps = []
            for d in deps:
                d = _package_of(str(d))
                if d is None or d in seen:
                    continue
                seen.add(d)
                nxt.append(d)
                if len(seen) >= DEPS_LIMIT:
                    return seen
        frontier = nxt
    return seen


def _content_dirs():
    """Mount point -> content folder: /Game/ is the project's, each plugin mounts its name."""
    if _CONTENT_DIRS:
        return _CONTENT_DIRS
    full = unreal.Paths.convert_relative_path_to_full
    _CONTENT_DIRS["/Game/"] = full(unreal.Paths.project_content_dir())
    plugins = full(unreal.Paths.project_plugins_dir())
    for dirpath, dirnames, filenames in os.walk(plugins):
        if dirpath[len(plugins):].count(os.sep) > 3:
            dirnames[:] = []
            continue
        for fn in filenames:
            if fn.endswith(".uplugin"):
                _CONTENT_DIRS["/" + fn[:-len(".uplugin")] + "/"] = os.path.join(dirpath, "Content")
    return _CONTENT_DIRS


def _package_file(pkg):
    for mount, folder in _content_dirs().items():
        if pkg.startswith(mount):
            base = os.path.join(folder, *pkg[len(mount):].split("/"))
            for ext in (".uasset", ".umap"):
                if os.path.isfile(base + ext):
                    return os.path.normpath(base + ext)
    return None


def _game_world():
    for get in (lambda: unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_game_world(),
                lambda: unreal.EditorLevelLibrary.get_game_world(),
                lambda: (unreal.EditorLevelLibrary.get_pie_worlds(False) or [None])[0]):
        try:
            w = get()
            if w is not None:
                return w
        except Exception:
            continue
    return None


class ShotContent(object):
    """Samples what the renderer draws while each MRQ job runs (see the comment above)."""

    EVERY = 10  # ticks between samples

    def __init__(self):
        self.key = None
        self.ticks = 0
        self.packages = {}   # render key -> set of packages seen on screen
        self.errors = {}     # render key -> first traceback
        self.by_actor = {}   # actor path -> packages (an actor's assets do not change mid-shot)
        self.handle = None

    def start(self):
        self.handle = unreal.register_slate_post_tick_callback(self.tick)

    def stop(self):
        if self.handle is not None:
            try:
                unreal.unregister_slate_post_tick_callback(self.handle)
            except Exception:
                warn(traceback.format_exc())
            self.handle = None

    def job_started(self, key):
        self.key, self.ticks, self.by_actor = key, 0, {}
        self.packages.setdefault(key, set())

    def tick(self, delta):
        if self.key is None:
            return
        self.ticks += 1
        if self.ticks % self.EVERY:
            return
        try:
            world = _game_world()
            if world is None:
                return
            seen = self.packages[self.key]
            for a in unreal.GameplayStatics.get_all_actors_of_class(world, unreal.Actor):
                if not a.was_recently_rendered(0.5):
                    continue
                path = a.get_path_name()
                if path not in self.by_actor:
                    self.by_actor[path] = _actor_packages(a)
                seen.update(self.by_actor[path])
        except Exception:
            self.errors.setdefault(self.key, traceback.format_exc())

    def report(self, key):
        """[{package, file}] for one render key, or (None, error)."""
        if key in self.errors and not self.packages.get(key):
            return None, self.errors[key]
        out = []
        try:
            for pkg in sorted(_expand(self.packages.get(key, set()))):
                f = _package_file(pkg)
                if f:
                    out.append({"package": pkg, "file": f})
        except Exception:
            return None, traceback.format_exc()
        return out, self.errors.get(key)


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
            configure_job(j, it, job["project"]["map"], job["project"].get("game_mode"))
            index[j.get_editor_property("job_name")] = item["key"]
        result["renders"][item["key"]] = {"status": "pending"}

    if not queue.get_jobs():
        write_result(result)
        finish_render_process()
        return

    content = ShotContent()
    content_error = None
    try:
        content.start()
    except Exception:
        content_error = traceback.format_exc()

    def put_deps(k):
        r = result["renders"].get(k)
        if r is None or r.get("status") != "ok" or "deps" in r:
            return
        if content_error:
            r["deps_error"] = content_error
            return
        deps, err = content.report(k)
        if deps is not None:
            r["deps"] = deps
        if err:
            r["deps_error"] = err

    def on_job_start(*args):
        try:
            job = args[0] if args else None
            content.job_started(index.get(job.get_editor_property("job_name")) if job is not None else None)
        except Exception:
            warn(traceback.format_exc())

    def on_job(params):
        try:
            name = params.job.get_editor_property("job_name")
            k = index.get(name)
            if k and result["renders"][k]["status"] != "error":
                result["renders"][k]["status"] = "ok" if params.success else "error"
                if not params.success:
                    result["renders"][k]["error"] = "MRQ reported this job as failed (see the UE log)"
            content.key = None
        except Exception:
            warn(traceback.format_exc())

    def on_done(executor, success):
        content.stop()
        for k, r in result["renders"].items():
            if r["status"] == "pending":
                r["status"] = "ok" if success else "error"
                if not success:
                    r["error"] = ("render stopped before this shot finished (PIE ended or MRQ aborted); "
                                  "rerun to render only the missing shots")
            put_deps(k)
        result["seconds"] = round(time.time() - started, 1)
        write_result(result)
        finish_render_process()

    # -ExecutePythonScript quits the editor as soon as this script returns unless told
    # to keep it alive; MRQ renders asynchronously after we return.
    keep_alive(True)
    executor = unreal.MoviePipelinePIEExecutor(subsystem)
    try:
        executor.on_individual_job_started_delegate.add_callable_unique(on_job_start)
    except Exception:
        content_error = content_error or traceback.format_exc()
    executor.on_individual_job_work_finished_delegate.add_callable_unique(on_job)
    executor.on_executor_finished_delegate.add_callable_unique(on_done)
    _KEEP["executor"], _KEEP["on_job"], _KEEP["on_done"] = executor, on_job, on_done
    _KEEP["content"], _KEEP["on_job_start"] = content, on_job_start
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
        res = {"build": run_build, "pull": run_pull, "scan": run_scan, "assets": run_assets,
               "survey": run_survey}[kind](job)
        write_result(res)
    except Exception:
        write_result({"fatal": traceback.format_exc()})
        if kind == "render":
            finish_render_process()


main()
