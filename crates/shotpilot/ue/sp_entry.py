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
import math
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


def _trace_fraction(start, end, near=60.0):
    """How far along start->end (0..1) the first wall on the visibility channel is, or None.
    Skips what the trace starts inside (the anchor itself) and small things right next to the
    start (a chair's backrest), so the camera stops at walls, not at the anchor's own clutter.
    Needs the full editor: a commandlet has no static mesh collision to trace against."""
    q = unreal.TraceTypeQuery
    channel = getattr(q, "ECC_VISIBILITY", None) or q.TRACE_TYPE_QUERY1
    total = (end - start).length()
    ignore = []
    for _ in range(6):
        hit = unreal.SystemLibrary.line_trace_single(editor_world(), start, end, channel, False, ignore,
                                                     unreal.DrawDebugTrace.NONE, True)
        if isinstance(hit, tuple):  # (ok, hit) on versions that return the bool too
            hit = hit[1] if hit and hit[0] else None
        if hit is None:
            return None
        h = hit.to_dict()  # 5.8: the fields themselves are protected from Python
        t, actor = float(h.get("time", 1.0)), h.get("hit_actor")
        small = False
        if actor is not None and not h.get("initial_overlap") and t * total < near:
            _, ext = actor.get_actor_bounds(False)
            small = 2 * max(ext.x, ext.y, ext.z) < 200.0
        if actor is not None and (h.get("initial_overlap") or small):
            ignore.append(actor)
            continue
        return t if 0.0 <= t < 1.0 else None
    return None


def clear_view(payload, target, margin=30.0, min_lens=12.0):
    """Scout cameras only (they are not shots): when something solid stands between the anchor and
    the camera, move the camera along the same line to just in front of it, and widen the lens by
    the same ratio so the anchor keeps its size in frame. Returns (payload, what changed or None)."""
    loc = payload["start"]["location"]
    s, e = unreal.Vector(*target), unreal.Vector(*loc)
    t = _trace_fraction(s, e)
    if t is None:
        return payload, None
    total = (e - s).length()
    if total < 1e-3:
        return payload, None
    free = t * total  # open line from the anchor to the wall
    ratio = max(free - margin, min(free * 0.5, 40.0)) / total
    new = [round(target[i] + (loc[i] - target[i]) * ratio, 1) for i in range(3)]
    payload = json.loads(json.dumps(payload))
    for k in ("start", "end"):
        payload[k]["location"] = new
    lens = payload.setdefault("lens", {})
    focal = round(max(min_lens, float(lens.get("focal_length", 35.0)) * max(ratio, 1e-3)), 1)
    lens["focal_length"] = focal
    log("scout camera pulled in front of a wall: %s -> %s, %smm" % (loc, new, focal))
    return payload, {"location": new, "focal_length": focal}


def build_shot(item, project, force):
    path = item["asset"]
    payload = item["payload"]
    moved, moved_error = None, None
    if item.get("clear_view"):
        try:
            payload, moved = clear_view(payload, item["clear_view"]["target"])
        except Exception:  # a scout photo from the planned spot beats no photo: report, keep going
            moved_error = traceback.format_exc()
            warn("scout camera not checked for walls: " + moved_error.strip().splitlines()[-1])
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
    # a streamed level: the camera and its cut start this many frames early, holding the first
    # pose, so MRQ can warm up on the shot's camera while the level streams in around it
    lead = int(item.get("lead", 0))
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
    sec.set_range(-lead, frames)
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
    cs.set_range(-lead, frames)
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
    out = {"status": "built", "engine": readback(seq)}
    if moved:
        out["clear_view"] = moved
    if moved_error:
        out["clear_view_error"] = moved_error
    return out


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


# Actors that are never places by themselves: cameras, volumes, sky, the level's own machinery.
_SKIP_CLASS = ("Camera", "Volume", "Sky", "Fog", "PostProcess", "Landscape", "Brush", "WorldSettings", "Info",
               "Reflection", "Decal", "Note", "Atmosphere", "Cloud", "Navigation", "LevelSequence", "Audio", "Sound",
               "Trigger", "Blocking", "HLOD", "DirectionalLight", "SkyLight")
# the engine's and the prototyping packs' plain shapes: what a blockout builds walls and floors from
_SHAPES = ("cube", "cylinder", "cone", "sphere", "plane", "ramp", "quartercylinder", "wedge", "pipe", "torus",
           "stair", "wall", "floor", "ceiling", "pillar", "column", "beam", "trim")
# what marks a place, first: a door or a character is where something happens; a lamp says where to look
_PRIORITY = {"character": 5.0, "door": 5.0, "effect": 4.0, "item": 4.0, "light": 3.0, "corridor": 2.5,
             "prop": 2.0, "room": 2.0, "start": 1.5, "shape": 0.5}
_DOORISH = ("door", "gate", "shutter", "hatch", "elevator", "lift")


def _box(a):
    """(min, max) of an actor's bounds, or None."""
    try:
        o, e = a.get_actor_bounds(False)
    except Exception:
        return None
    if e.x <= 0.5 and e.y <= 0.5 and e.z <= 0.5:
        return None
    return ([o.x - e.x, o.y - e.y, o.z - e.z], [o.x + e.x, o.y + e.y, o.z + e.z])


def _mesh_name(a):
    try:
        comp = a.get_component_by_class(unreal.StaticMeshComponent)
        sm = comp.get_editor_property("static_mesh") if comp else None
        return sm.get_name().lower() if sm else ""
    except Exception:
        return ""


def _has(a, cls_name):
    cls = getattr(unreal, cls_name, None)
    if cls is None:
        return False
    try:
        return bool(a.get_components_by_class(cls))
    except Exception:
        return False


def _kind(a, cls, label, box):
    """What an actor is to a location scout: a place-maker (door, character, light, ...), a
    structural piece of the building ("structure"), or nothing (None)."""
    lo = (label + " " + cls).lower()
    if box is not None and (cls == "SkeletalMeshActor" or _has(a, "SkeletalMeshComponent")):
        return "character"
    if cls in ("PointLight", "SpotLight", "RectLight") or _has(a, "LocalLightComponent"):
        try:
            comp = a.get_component_by_class(unreal.LocalLightComponent)
            if comp and (comp.get_editor_property("intensity") <= 0 or not comp.is_visible()):
                return None
        except Exception:
            pass
        return "light"
    if cls == "PlayerStart":
        return "start"
    if any(k in cls for k in _SKIP_CLASS):
        return None
    if cls in ("NiagaraActor", "Emitter") or _has(a, "NiagaraComponent") or _has(a, "ParticleSystemComponent"):
        return "effect"
    if box is None:
        return None
    if not _has(a, "MeshComponent"):
        return None
    size = [box[1][i] - box[0][i] for i in range(3)]
    big, thin = max(size), min(size)
    if any(k in lo for k in _DOORISH):
        return "door"
    # walls, floors, ceilings and plain blocks frame the places; they are not places
    shape = _mesh_name(a)
    plain = any(s in shape.replace("sm_", "").replace("shape_", "") for s in _SHAPES)
    if big > 2500 or (big >= 250 and thin < 0.15 * big) or (plain and cls == "StaticMeshActor"):
        return "structure"
    if big < 40 or size[2] < 15:
        return None
    return "item" if cls.endswith("_C") else "prop"


def _floor_under(p, boxes):
    """The top of the highest horizontal piece under a point (no collision in a commandlet: bounds only)."""
    best = None
    for lo, hi in boxes:
        if hi[0] - lo[0] < 100 or hi[1] - lo[1] < 100:
            continue
        if lo[0] - 10 <= p[0] <= hi[0] + 10 and lo[1] - 10 <= p[1] <= hi[1] + 10 and hi[2] <= p[2] + 20:
            if best is None or hi[2] > best:
                best = hi[2]
    return best


def _rooms(slabs, boxes):
    """Floors from the horizontal slabs of a blockout: a slab with a ceiling over it (the storey
    above or a roof) is a floor; the top slab is the roof. A long narrow floor is a corridor."""
    out = []
    for i, (lo, hi) in enumerate(slabs):
        sx, sy = hi[0] - lo[0], hi[1] - lo[1]
        if min(sx, sy) < 200:
            continue
        def overlaps(b):
            return min(hi[0], b[1][0]) - max(lo[0], b[0][0]) > 0.5 * sx and min(hi[1], b[1][1]) - max(lo[1], b[0][1]) > 0.5 * sy
        above = [b for j, b in enumerate(slabs) if j != i and hi[2] + 150 <= b[0][2] <= hi[2] + 900 and overlaps(b)]
        below = [b for j, b in enumerate(slabs) if j != i and lo[2] - 900 <= b[1][2] <= lo[2] - 150 and overlaps(b)]
        if not above and below:
            continue  # a roof
        top = hi[2]
        out.append({"kind": "corridor" if max(sx, sy) >= 3 * min(sx, sy) else "room",
                    "label": "floor", "cls": "StaticMeshActor",
                    "p": [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, top], "floor": top,
                    "size": [min(sx, 2000.0), min(sy, 2000.0), 250.0]})
    return out


def _candidates(limit):
    """Places to scout when nothing is tagged (D-44, D-45, D-84). A place is where something
    happens or can be seen: a door, a character, an effect, a lamp, a prop, the player start, a
    room or corridor read from the floors. Walls, floors, ceilings and plain blocks are the
    building, never a place (a blockout is made of them). Things within 3.5 m make one place,
    named after its most telling member. shotpilot picks among them (Claude, by the plan; else
    the most spread-out ones)."""
    things, boxes, slabs, shapes = [], [], [], []
    for a in all_actors():
        cls = a.get_class().get_name()
        try:
            if a.is_hidden_ed() or a.is_editor_only():
                continue
        except Exception:
            pass
        box = _box(a)
        if box is not None and _has(a, "MeshComponent"):
            boxes.append(box)
        label = a.get_actor_label()
        kind = _kind(a, cls, label, box)
        if kind is None:
            continue
        if kind == "structure":
            size = [box[1][i] - box[0][i] for i in range(3)]
            if size[2] <= min(150.0, max(60.0, 0.12 * min(size[0], size[1]))) and size[0] >= 200 and size[1] >= 200:
                slabs.append(box)
            shapes.append((a, cls, label, box))
            continue
        loc = a.get_actor_location()
        p = [loc.x, loc.y, loc.z]
        if box is not None and kind not in ("light", "start"):
            p = [(box[0][0] + box[1][0]) / 2, (box[0][1] + box[1][1]) / 2, box[0][2]]
        things.append({"kind": kind, "label": label, "cls": cls, "p": p, "box": box})
    for t in things:
        f = _floor_under(t["p"], boxes) if t["kind"] in ("light", "start", "effect") else t["p"][2]
        t["floor"] = t["p"][2] if f is None else f
    things += _rooms(slabs, boxes)
    if not things:  # nothing but blocks: the old way, the plain shapes themselves
        for a, cls, label, box in shapes:
            things.append({"kind": "shape", "label": label, "cls": cls, "box": box, "floor": box[0][2],
                           "p": [(box[0][0] + box[1][0]) / 2, (box[0][1] + box[1][1]) / 2, box[0][2]]})
    # one place per neighbourhood, seeded by the most telling thing
    things.sort(key=lambda t: -_PRIORITY[t["kind"]])
    places = []
    for t in things:
        for pl in places:
            s = pl[0]
            if math.hypot(t["p"][0] - s["p"][0], t["p"][1] - s["p"][1]) <= 350 and abs(t["floor"] - s["floor"]) < 200:
                pl.append(t)
                break
        else:
            places.append([t])
    out, used = [], {}
    for pl in places:
        s = pl[0]
        base = {"light": "light", "start": "start", "room": "room", "corridor": "corridor"}.get(s["kind"]) or _anchor_name(s["label"]) or s["kind"]
        used[base] = used.get(base, 0) + 1
        name = base if used[base] == 1 else "%s_%d" % (base, used[base])
        solid = [t["box"] for t in pl if t.get("box") is not None and t["kind"] not in ("light", "start")]
        floor = s["floor"]
        if s["kind"] in ("room", "corridor"):
            cx, cy = s["p"][0], s["p"][1]
            size = s["size"]
            center = [cx, cy, floor + 120]
        elif solid:
            lo = [min(b[0][i] for b in solid) for i in range(3)]
            hi = [max(b[1][i] for b in solid) for i in range(3)]
            cx, cy = (lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2
            size = [max(hi[0] - lo[0], 60.0), max(hi[1] - lo[1], 60.0), max(hi[2] - lo[2], 60.0)]
            center = [cx, cy, max((lo[2] + hi[2]) / 2, floor + 60)]
        else:  # a lamp or the start alone: the floor around it, looked at from eye height
            cx, cy = s["p"][0], s["p"][1]
            size = [300.0, 300.0, 240.0]
            center = [cx, cy, floor + 120]
        r = lambda v: [round(x, 1) for x in v]
        kinds = sorted({t["kind"] for t in pl}, key=lambda k: -_PRIORITY[k])
        labels = [t["label"] for t in pl[:4]]
        score = 100.0 * _PRIORITY[s["kind"]] + 20.0 * min(len(pl) - 1, 5)
        out.append({"name": name, "label": " + ".join(labels), "cls": s["cls"], "kind": s["kind"], "kinds": kinds,
                    "score": round(score, 1), "at": r([cx, cy, floor]), "size": r(size), "center": r(center)})
    out.sort(key=lambda c: -c["score"])
    counts = {}
    for t in things:
        counts[t["kind"]] = counts.get(t["kind"], 0) + 1
    log("scan: %d places from %s (%d structural pieces left out), returning %d" % (
        len(out), ", ".join("%s %d" % kv for kv in sorted(counts.items())), len(shapes), min(len(out), limit)))
    return out[:limit]


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
    out = {"anchors": anchors}
    if (job.get("auto") and not anchors) or job.get("candidates"):
        out["candidates"] = [c for c in _candidates(int(job.get("limit", 60))) if c["name"] not in anchors]
    return out


def run_survey(job):
    """Stage 0 in one launch: the tagged anchors (else landmark candidates to pick from) and, when
    asked, the mesh inventory."""
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
# While MRQ renders a shot in PIE, a tick callback samples the actors on screen and collects
# the content packages behind them: the actor's Blueprint class, meshes, materials, Niagara
# systems, then their hard dependencies (materials -> textures). When the job finishes, the
# packages that are files under the project are reported as the shot's `deps`; shotpilot
# hashes them, so a later change to one of them (a texture, a material) re-renders only the
# shots that showed it.
# "On screen" is three tests, cheapest first: the renderer drew the actor lately
# (was_recently_rendered, which is also true for shadow passes, so a directional light's
# shadow alone marks the whole level), its bounds reach into the camera's view (with a
# margin), and a visibility trace from the camera reaches some point of its bounds without
# hitting something else first. Samples are taken only while the sequence's camera is the
# view target. When no sample saw that camera, the shot reports everything the renderer
# drew (re-rendering too much beats missing a change) and says so in `deps_error`.
# Not covered: where actors are placed in an editor level (World Partition keeps them in
# __ExternalActors__ packages we cannot map back from PIE), game code, and actors that only
# cast a shadow into the frame from outside it.

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


VIEW_MARGIN = 1.1   # widen the view a little: edges, motion blur and bokeh still show things
GRID = (-0.9, 0.0, 0.9)  # where in an actor's bounds the traces aim (27 points)


def _view(world, aspect):
    """The camera the PIE world looks through, as (origin, forward, right, up, tan_h, tan_v), or
    None while it is not a camera actor (warm-up frames before the sequence's camera cut)."""
    pcm = unreal.GameplayStatics.get_player_camera_manager(world, 0)
    if pcm is None:
        return None
    target = None
    for get in (lambda: pcm.get_view_target(),
                lambda: unreal.GameplayStatics.get_player_controller(world, 0).get_view_target()):
        try:
            target = get()
            break
        except Exception:
            continue
    if target is not None and not isinstance(target, unreal.CameraActor):
        return None
    rot = pcm.get_camera_rotation()
    tan_h = math.tan(math.radians(float(pcm.get_fov_angle())) / 2.0) * VIEW_MARGIN
    return (pcm.get_camera_location(), rot.get_forward_vector(), rot.get_right_vector(), rot.get_up_vector(),
            tan_h, tan_h / max(aspect, 0.1))


def _in_view(view, p):
    """(depth, in view?) of a world point."""
    o, f, r, u, th, tv = view
    d = p - o
    x, y, z = d.dot(f), d.dot(r), d.dot(u)
    return x, (x > 1.0 and abs(y) <= x * th and abs(z) <= x * tv)


def _box_culled(view, origin, ext):
    """True when the box lies wholly outside one side of the view (a conservative test)."""
    o, f, r, u, th, tv = view
    corners = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                d = unreal.Vector(origin.x + sx * ext.x, origin.y + sy * ext.y, origin.z + sz * ext.z) - o
                corners.append((d.dot(f), d.dot(r), d.dot(u)))
    planes = (lambda x, y, z: x < 1.0, lambda x, y, z: y > x * th, lambda x, y, z: -y > x * th,
              lambda x, y, z: z > x * tv, lambda x, y, z: -z > x * tv)
    return any(all(out(*c) for c in corners) for out in planes)


def _reaches(world, start, end, actor):
    """Whether a visibility trace from the camera gets to `end` without hitting something
    other than `actor` first. What the trace starts inside (a camera in a trigger) is skipped."""
    q = unreal.TraceTypeQuery
    channel = getattr(q, "ECC_VISIBILITY", None) or q.TRACE_TYPE_QUERY1
    ignore = []
    for _ in range(4):
        hit = unreal.SystemLibrary.line_trace_single(world, start, end, channel, False, ignore,
                                                     unreal.DrawDebugTrace.NONE, True)
        if isinstance(hit, tuple):  # (ok, hit) on versions that return the bool too
            hit = hit[1] if hit and hit[0] else None
        if hit is None:
            return True
        h = hit.to_dict()
        other = h.get("hit_actor")
        if other is None or other.get_path_name() == actor.get_path_name():
            return True
        if h.get("initial_overlap"):
            ignore.append(other)
            continue
        return False
    return True


def _on_screen(world, view, actor):
    origin, ext = actor.get_actor_bounds(False)
    o = view[0]
    if all(abs(c) <= e + 10.0 for c, e in ((o.x - origin.x, ext.x), (o.y - origin.y, ext.y), (o.z - origin.z, ext.z))):
        return True  # the camera is inside it
    if _box_culled(view, origin, ext):
        return False
    aims = []
    for fx in GRID:
        for fy in GRID:
            for fz in GRID:
                p = unreal.Vector(origin.x + fx * ext.x, origin.y + fy * ext.y, origin.z + fz * ext.z)
                depth, inside = _in_view(view, p)
                if inside:
                    aims.append((depth, p))
    if not aims:
        return True  # it spans the view without a sample point inside: count it
    for _, p in sorted(aims, key=lambda a: a[0]):  # nearest first: the likeliest to be seen
        if _reaches(world, o, p, actor):
            return True
    return False


class ShotContent(object):
    """Samples what is on screen while each MRQ job runs (see the comment above)."""

    EVERY = 5  # ticks between samples

    def __init__(self):
        self.key = None
        self.ticks = 0
        self.packages = {}   # render key -> set of packages seen on screen
        self.drawn = {}      # render key -> set of packages the renderer drew (shadows too)
        self.views = {}      # render key -> samples taken through the sequence's camera
        self.aspect = {}     # render key -> output width / height
        self.errors = {}     # render key -> first traceback
        self.by_actor = {}   # actor path -> packages (an actor's assets do not change mid-shot)
        self.shown = set()   # actor paths already on screen in this job
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
        self.key, self.ticks, self.by_actor, self.shown = key, 0, {}, set()
        self.packages.setdefault(key, set())
        self.drawn.setdefault(key, set())
        self.views.setdefault(key, 0)

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
            key = self.key
            try:
                view = _view(world, self.aspect.get(key, 16.0 / 9.0))
            except Exception:
                view = None
                self.errors.setdefault(key, traceback.format_exc())
            if view is not None:
                self.views[key] += 1
            for a in unreal.GameplayStatics.get_all_actors_of_class(world, unreal.Actor):
                if not a.was_recently_rendered(0.5):
                    continue
                path = a.get_path_name()
                if path not in self.by_actor:
                    self.by_actor[path] = _actor_packages(a)
                self.drawn[key].update(self.by_actor[path])
                if view is None or path in self.shown:
                    continue
                try:
                    shown = _on_screen(world, view, a)
                except Exception:
                    shown = True  # cannot tell: count it rather than miss it
                    self.errors.setdefault(key, traceback.format_exc())
                if shown:
                    self.shown.add(path)
                    self.packages[key].update(self.by_actor[path])
        except Exception:
            self.errors.setdefault(self.key, traceback.format_exc())

    def report(self, key):
        """([{package, file}], note or error) for one render key, or (None, error)."""
        note = self.errors.get(key)
        if self.views.get(key):
            packages = self.packages.get(key, set())
        else:
            packages = self.drawn.get(key, set())
            if packages:
                note = note or ("the sequence's camera was never the view target while sampling; "
                                "reporting everything the renderer drew (shadow passes included)")
        log("deps %s: %d packages on screen over %d camera samples (%d drawn)"
            % (key, len(self.packages.get(key, ())), self.views.get(key, 0), len(self.drawn.get(key, ()))))
        if note and not packages:
            return None, note
        out = []
        try:
            for pkg in sorted(_expand(packages)):
                f = _package_file(pkg)
                if f:
                    out.append({"package": pkg, "file": f})
        except Exception:
            return None, traceback.format_exc()
        return out, note


def keep_alive(flag):
    try:
        unreal.EditorPythonScripting.set_keep_python_script_alive(bool(flag))
    except Exception:
        warn("EditorPythonScripting.set_keep_python_script_alive unavailable: " + traceback.format_exc())


def finish_render_process():
    keep_alive(False)  # lets the -ExecutePythonScript runner close the editor
    unreal.SystemLibrary.quit_editor()


class SetupError(Exception):
    """The editor lacks something the job needs; the message says what to do (no traceback)."""


def run_render(job):
    # Movie Render Queue comes from a plugin; without it nothing can be filmed, so say so
    # before building anything
    if getattr(unreal, "MoviePipelineQueueSubsystem", None) is None:
        raise SetupError(
            "Movie Render Queue is not available in this editor. shotpilot asked for it with "
            "-EnablePlugins=MovieRenderPipeline; if this still shows, enable the Movie Render Queue "
            "plugin in the project (Edit > Plugins > Movie Render Queue, or add "
            '{"Name": "MovieRenderPipeline", "Enabled": true} to the .uproject Plugins) and try again.')
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
    for item in job["render"]:
        res = item.get("resolution") or [16, 9]
        content.aspect[item["key"]] = float(res[0]) / max(float(res[1]), 1.0)
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

    # UE checks the arity of delegate callables, so take exactly the one job argument.
    def on_job_start(job):
        try:
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
    except SetupError as e:
        write_result({"fatal": str(e)})
        if kind == "render":
            finish_render_process()
    except Exception:
        write_result({"fatal": traceback.format_exc()})
        if kind == "render":
            finish_render_process()


main()
