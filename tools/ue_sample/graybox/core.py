"""Shared pieces of the sample levels (D-54): logging, paint, the Level spawner, the ambient
sequence that moves the mannequins. Each game module builds on these."""

import math

import unreal

LP = "/Game/LevelPrototyping"
MANNY = "/Game/Characters/Mannequins/Meshes/SKM_Manny_Simple"
QUINN = "/Game/Characters/Mannequins/Meshes/SKM_Quinn_Simple"
ANIMS = "/Game/Characters/Mannequins/Anims"

EAL = unreal.EditorAssetLibrary
# engine basic shapes: 100 cm, pivot at the centre
CUBE = "/Engine/BasicShapes/Cube"
SPHERE = "/Engine/BasicShapes/Sphere"
CONE = "/Engine/BasicShapes/Cone"
CYLINDER = "/Engine/BasicShapes/Cylinder"


def log(msg):
    unreal.log("[graybox] " + str(msg))


def warn(msg):
    unreal.log_warning("[graybox] " + str(msg))


class Missing(Exception):
    pass


def load(path):
    if not EAL.does_asset_exist(path):
        raise Missing(path)
    return EAL.load_asset(path)


def setp(obj, **props):
    """Set editor properties, warning (not failing) on names this engine version lacks."""
    for key, value in props.items():
        try:
            obj.set_editor_property(key, value)
        except Exception as e:  # cosmetic settings only: the level still builds
            warn("%s.%s not set (%s)" % (type(obj).__name__, key, e))


def movable(component):
    try:
        component.set_mobility(unreal.ComponentMobility.MOVABLE)
    except Exception as e:
        warn("mobility not set (%s)" % e)


# --------------------------------------------------------------------------- materials
# Plain colours, no textures: the shapes stay graybox, only the paint and the light change. Each
# material is a few lines of HLSL in a Custom node fed the world position (WP) and the time (T), so
# a wall can be two-tone by height and a lamp can flicker with no texture or curve asset.

def _custom(mat, code, out, x, y, sources):
    mel = unreal.MaterialEditingLibrary
    node = mel.create_material_expression(mat, unreal.MaterialExpressionCustom, x, y)
    node.set_editor_property("code", code)
    node.set_editor_property("output_type", out)
    inputs = []
    for name in ("WP", "T"):
        ci = unreal.CustomInput()
        ci.set_editor_property("input_name", name)
        inputs.append(ci)
    node.set_editor_property("inputs", inputs)
    mel.connect_material_expressions(sources["WP"], "", node, "WP")
    mel.connect_material_expressions(sources["T"], "", node, "T")
    return node


def make_material(path, base="return float3(0.18, 0.18, 0.18);", roughness="return 0.8;", emissive=None,
                  light_function=False):
    """A material from HLSL bodies (each sees WP, the world position in cm, and T, the time in s).
    A light function material only uses `emissive`, as the light's brightness."""
    mel = unreal.MaterialEditingLibrary
    if EAL.does_asset_exist(path):  # left over from a level deleted to be built again
        EAL.delete_asset(path)
    folder, name = path.rsplit("/", 1)
    mat = unreal.AssetToolsHelpers.get_asset_tools().create_asset(name, folder, unreal.Material,
                                                                  unreal.MaterialFactoryNew())
    sources = {"WP": mel.create_material_expression(mat, unreal.MaterialExpressionWorldPosition, -900, 0),
               "T": mel.create_material_expression(mat, unreal.MaterialExpressionTime, -900, 200)}
    f1, f3 = unreal.CustomMaterialOutputType.CMOT_FLOAT1, unreal.CustomMaterialOutputType.CMOT_FLOAT3
    if light_function:
        mat.set_editor_property("material_domain", unreal.MaterialDomain.MD_LIGHT_FUNCTION)
        mel.connect_material_property(_custom(mat, emissive, f1, -400, 0, sources), "",
                                      unreal.MaterialProperty.MP_EMISSIVE_COLOR)
    else:
        mel.connect_material_property(_custom(mat, base, f3, -400, -200, sources), "",
                                      unreal.MaterialProperty.MP_BASE_COLOR)
        mel.connect_material_property(_custom(mat, roughness, f1, -400, 0, sources), "",
                                      unreal.MaterialProperty.MP_ROUGHNESS)
        if emissive:
            mel.connect_material_property(_custom(mat, emissive, f3, -400, 200, sources), "",
                                          unreal.MaterialProperty.MP_EMISSIVE_COLOR)
    mel.recompile_material(mat)
    EAL.save_asset(path, only_if_is_dirty=False)
    return mat



class Level:
    """Spawns actors into the current level, sized in cm whatever the mesh's own pivot is."""

    def __init__(self, game):
        self.game = game
        self.actors = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
        self.cube = load(LP + "/Meshes/SM_Cube")
        self.meshes = {}
        self.cylinder = load(LP + "/Meshes/SM_Cylinder")
        self.mats = {
            "floor": load(LP + "/Materials/MI_PrototypeGrid_Gray"),
            "wall": load(LP + "/Materials/MI_PrototypeGrid_Gray_02"),
            "prop": load(LP + "/Materials/MI_PrototypeGrid_TopDark"),
            "accent": load(LP + "/Materials/MI_DefaultColorway"),
        }
        self.walls = 0

    def _spawn(self, cls, at, rot=None):
        a = self.actors.spawn_actor_from_class(cls, at, rot or unreal.Rotator())
        if a is None:
            raise RuntimeError("could not spawn a %s into the editor world" % cls.__name__)
        return a

    def _done(self, actor, label, folder, tags=()):
        if actor is None:
            raise RuntimeError("could not spawn %s into the editor world" % label)
        actor.set_actor_label(label)
        try:
            actor.set_folder_path(self.game + "/" + folder)
        except Exception:
            pass
        if tags:
            actor.set_editor_property("tags", [unreal.Name(t) for t in tags])
        return actor

    def box(self, label, center, size, mat="wall", yaw=0.0, folder="Geometry", mesh=None):
        """A mesh scaled to `size` (x, y, z before the yaw) with its bounds centred on `center`."""
        mesh = mesh or self.cube
        b = mesh.get_bounds()
        o, e = b.origin, b.box_extent
        sx, sy, sz = size[0] / (2 * e.x), size[1] / (2 * e.y), size[2] / (2 * e.z)
        ox, oy, oz = o.x * sx, o.y * sy, o.z * sz
        c, s = math.cos(math.radians(yaw)), math.sin(math.radians(yaw))
        loc = unreal.Vector(center[0] - (ox * c - oy * s), center[1] - (ox * s + oy * c), center[2] - oz)
        # spawn by class, not from the asset: the asset path needs an editor viewport (fails in a commandlet)
        a = self._done(self._spawn(unreal.StaticMeshActor, loc,
                                                          unreal.Rotator(roll=0.0, pitch=0.0, yaw=yaw)), label, folder)
        a.set_actor_scale3d(unreal.Vector(sx, sy, sz))
        comp = a.get_component_by_class(unreal.StaticMeshComponent)
        comp.set_static_mesh(mesh)
        for i in range(max(1, comp.get_num_materials())):
            comp.set_material(i, self.mats[mat])
        return a

    def shape(self, label, center, size, mesh=CUBE, mat="prop", rot=(0.0, 0.0, 0.0), folder="Props"):
        """An engine basic shape (pivot at its centre) scaled to `size` (cm, before turning) and
        turned by `rot` = (pitch, yaw, roll) about its centre."""
        m = self.meshes.get(mesh) or self.meshes.setdefault(mesh, load(mesh))
        e = m.get_bounds().box_extent
        r = unreal.Rotator(roll=rot[2], pitch=rot[0], yaw=rot[1])
        a = self._done(self._spawn(unreal.StaticMeshActor, unreal.Vector(*center), r), label, folder)
        a.set_actor_scale3d(unreal.Vector(size[0] / (2 * e.x), size[1] / (2 * e.y), size[2] / (2 * e.z)))
        comp = a.get_component_by_class(unreal.StaticMeshComponent)
        comp.set_static_mesh(m)
        for i in range(max(1, comp.get_num_materials())):
            comp.set_material(i, self.mats[mat])
        return a

    def group(self, label, at, parts, folder="Props"):
        """An empty actor at `at` holding `parts` (actors), so a sequence can turn them as one."""
        # a mesh actor without a mesh: it brings a movable root to hang the parts on
        hub = self._done(self._spawn(unreal.StaticMeshActor, unreal.Vector(*at)), label, folder)
        movable(hub.get_component_by_class(unreal.StaticMeshComponent))
        for p in parts:
            movable(p.get_component_by_class(unreal.StaticMeshComponent))
            p.attach_to_actor(hub, "", unreal.AttachmentRule.KEEP_WORLD, unreal.AttachmentRule.KEEP_WORLD,
                              unreal.AttachmentRule.KEEP_WORLD, False)
        return hub

    def hold(self, holder, label, size, mat="steel", socket="hand_r", loc=(0.0, 0.0, 50.0), rot=(0.0, 0.0, 0.0)):
        """A prop (a box of `size`) in a mannequin's hand: snapped to the hand bone, then moved by
        `loc` and turned by `rot` (pitch, yaw, roll) in the hand's own frame."""
        a = self.shape(label, (0.0, 0.0, 0.0), size, mesh=CUBE, mat=mat, folder="Characters")
        comp = a.get_component_by_class(unreal.StaticMeshComponent)
        movable(comp)
        skel = holder.get_component_by_class(unreal.SkeletalMeshComponent)
        snap, world = unreal.AttachmentRule.SNAP_TO_TARGET, unreal.AttachmentRule.KEEP_WORLD
        a.attach_to_component(skel, socket, snap, snap, world, False)
        comp.set_editor_property("relative_location", unreal.Vector(*loc))
        comp.set_editor_property("relative_rotation", unreal.Rotator(roll=rot[2], pitch=rot[0], yaw=rot[1]))
        return a

    def hold_ground(self, label, at, size, yaw=0.0, mat="steel"):
        """A prop lying on the ground (a dropped sword)."""
        return self.shape(label, at, size, mesh=CUBE, mat=mat, rot=(0.0, yaw, 0.0), folder="Characters")

    def post(self, label, at, diameter, height, mat="prop", folder="Props"):
        """An upright cylinder standing on z = at[2]."""
        return self.box(label, (at[0], at[1], at[2] + height / 2), (diameter, diameter, height),
                        mat=mat, folder=folder, mesh=self.cylinder)

    def wall(self, a, b, height=300.0, thick=20.0, holes=(), mat="wall", ends=(0.0, 0.0)):
        """A straight wall on the floor from point a to b, with openings given as
        (start, end, bottom, top) in cm measured along the wall from a. `ends` = (start, end) is added
        to the length at each end (opening positions stay measured from a): +thick/2 at an outer corner
        lets this wall own the corner, -thick/2 on the other wall stops it at the owner's inner face."""
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        ux, uy = dx / length, dy / length
        yaw = math.degrees(math.atan2(dy, dx))
        pieces, u = [], -ends[0]
        for (u0, u1, bottom, top) in sorted(holes):
            if u0 > u:
                pieces.append((u, u0, 0.0, height))
            if bottom > 0:
                pieces.append((u0, u1, 0.0, bottom))
            if top < height:
                pieces.append((u0, u1, top, height))
            u = u1
        if u < length + ends[1]:
            pieces.append((u, length + ends[1], 0.0, height))
        for (u0, u1, z0, z1) in pieces:
            self.walls += 1
            mid = (u0 + u1) / 2
            self.box("Wall_%02d" % self.walls, (a[0] + ux * mid, a[1] + uy * mid, (z0 + z1) / 2),
                     (u1 - u0, thick, z1 - z0), mat=mat, yaw=yaw)

    def bars(self, a, b, u0, u1, bottom, top, gap=18.0):
        """Vertical bars across a window opening of wall a->b (keeps the player inside)."""
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        ux, uy = dx / length, dy / length
        n = int((u1 - u0) // gap)
        for i in range(1, n + 1):
            u = u0 + (u1 - u0) * i / (n + 1)
            self.post("Window_Bar", (a[0] + ux * u, a[1] + uy * u, bottom), 4.0, top - bottom, folder="Geometry")

    def mannequin(self, at, rot, mesh, anim, playing=True, rate=1.0):
        """A SkeletalMeshActor playing one animation in a loop (or frozen on its first frame).
        `anim` is under the template's Anims folder, or a full /Game path."""
        skm, seq = load(mesh), load(anim if anim.startswith("/") else ANIMS + "/" + anim)
        a = self._spawn(unreal.SkeletalMeshActor, unreal.Vector(*at), rot)
        comp = a.get_component_by_class(unreal.SkeletalMeshComponent)
        if hasattr(comp, "set_skeletal_mesh_asset"):
            comp.set_skeletal_mesh_asset(skm)
        else:
            comp.set_skeletal_mesh(skm)
        setp(comp, animation_mode=unreal.AnimationMode.ANIMATION_SINGLE_NODE)
        try:
            data = unreal.SingleAnimationPlayData(anim_to_play=seq, saved_looping=True, saved_playing=playing,
                                                  saved_position=0.0, saved_play_rate=rate)
            comp.set_editor_property("animation_data", data)
        except Exception as e:
            warn("animation_data not set (%s)" % e)
        try:
            comp.override_animation_data(seq, True, playing, 0.0, rate)
        except Exception as e:
            warn("override_animation_data failed (%s)" % e)
        return a

    def figure(self, label, at, facing, mesh=MANNY, anim="Unarmed/MM_Idle", rate=1.0):
        """A mannequin standing at `at`, facing the `facing` yaw, looping an in-place animation."""
        # the mannequin mesh faces +Y in its asset, so turn it a quarter less
        a = self.mannequin(at, unreal.Rotator(roll=0.0, pitch=0.0, yaw=facing - 90.0), mesh, anim, rate=rate)
        return self._done(a, label, "Characters")

    def lying(self, label, center, along, pose, surface=0.0, mesh=MANNY, face_up=True):
        """A body on a surface at height `surface`, centred on `center` (x, y), its length along the
        `along` yaw, frozen in `pose` (a pose_anim made with floor -1 for face up, +1 for face down)."""
        # roll -90 turns the mannequin's front (+Y) up, +90 down
        rot = unreal.Rotator(roll=-90.0 if face_up else 90.0, pitch=0.0, yaw=along - 90.0)
        a = self.mannequin((center[0], center[1], surface), rot, mesh, pose, playing=False)
        try:  # centre the body by its bounds, whichever end the mesh pivot is at
            o, e = a.get_actor_bounds(False)
            if e.x > 1 and e.z > 1:
                loc = a.get_actor_location()
                a.set_actor_location(unreal.Vector(loc.x + center[0] - o.x, loc.y + center[1] - o.y,
                                                   loc.z + surface - (o.z - e.z)), False, False)
        except Exception as e:
            warn("%s not centred (%s)" % (label, e))
        return self._done(a, label, "Characters")

    def _light(self, c, candela, color, radius, scatter, light_function):
        movable(c)
        setp(c, intensity_units=unreal.LightUnits.CANDELAS)
        c.set_intensity(candela)
        c.set_light_color(unreal.LinearColor(color[0], color[1], color[2], 1.0))
        c.set_attenuation_radius(radius)
        if scatter is not None:  # how strongly it lights the volumetric fog (the visible beam)
            setp(c, volumetric_scattering_intensity=scatter)
        if light_function is not None:
            setp(c, light_function_material=light_function)

    def point(self, label, at, candela, color, radius=900.0, source=4.0, scatter=None, light_function=None):
        a = self._spawn(unreal.PointLight, unreal.Vector(*at), unreal.Rotator())
        c = a.get_component_by_class(unreal.PointLightComponent)
        self._light(c, candela, color, radius, scatter, light_function)
        setp(c, source_radius=source)
        return self._done(a, label, "Lights")

    def spot(self, label, at, toward, candela, color, cone=20.0, radius=1500.0, scatter=None):
        """A spot light at `at` aimed at the point `toward`."""
        d = [toward[i] - at[i] for i in range(3)]
        yaw = math.degrees(math.atan2(d[1], d[0]))
        pitch = math.degrees(math.atan2(d[2], math.hypot(d[0], d[1])))
        a = self._spawn(unreal.SpotLight, unreal.Vector(*at), unreal.Rotator(roll=0.0, pitch=pitch, yaw=yaw))
        c = a.get_component_by_class(unreal.SpotLightComponent)
        self._light(c, candela, color, radius, scatter, None)
        c.set_inner_cone_angle(cone * 0.6)
        c.set_outer_cone_angle(cone)
        return self._done(a, label, "Lights")

    def sun(self, label, pitch, yaw, lux, color):
        a = self._spawn(unreal.DirectionalLight, unreal.Vector(0, 0, 1000),
                                               unreal.Rotator(roll=0.0, pitch=pitch, yaw=yaw))
        c = a.get_component_by_class(unreal.DirectionalLightComponent)
        movable(c)
        c.set_intensity(lux)
        c.set_light_color(unreal.LinearColor(color[0], color[1], color[2], 1.0))
        setp(c, atmosphere_sun_light=True)
        return self._done(a, label, "Lights")

    def sky(self, intensity=1.0):
        self._done(self._spawn(unreal.SkyAtmosphere, unreal.Vector(0, 0, 0)),
                   "SkyAtmosphere", "Lights")
        a = self._spawn(unreal.SkyLight, unreal.Vector(0, 0, 1200))
        c = a.get_component_by_class(unreal.SkyLightComponent)
        movable(c)
        setp(c, real_time_capture=True)
        c.set_intensity(intensity)
        return self._done(a, "SkyLight", "Lights")

    def fog(self, density, color, volumetric=False, forward=None, extinction=None):
        """Height fog. `forward` (0..0.9) makes the volumetric fog scatter toward the camera, so a
        beam reads brighter looking into the light; `extinction` thickens it."""
        a = self._spawn(unreal.ExponentialHeightFog, unreal.Vector(0, 0, 0))
        c = a.get_component_by_class(unreal.ExponentialHeightFogComponent)
        setp(c, fog_density=density, fog_height_falloff=0.2)
        if forward is not None:
            setp(c, volumetric_fog_scattering_distribution=forward)
        if extinction is not None:
            setp(c, volumetric_fog_extinction_scale=extinction)
        try:
            c.set_fog_inscattering_color(unreal.LinearColor(color[0], color[1], color[2], 1.0))
        except Exception as e:
            warn("fog colour not set (%s)" % e)
        try:
            c.set_volumetric_fog(volumetric)
        except Exception:
            setp(c, enable_volumetric_fog=volumetric)
        return self._done(a, "HeightFog", "Lights")

    def exposure(self, ev100):
        """Hold the exposure at one EV100 (the template measures exposure in EV100), so the same
        shot definition always renders the same brightness."""
        a = self._spawn(unreal.PostProcessVolume, unreal.Vector(0, 0, 0))
        setp(a, unbound=True)
        s = a.get_editor_property("settings")
        setp(s, override_auto_exposure_method=True, auto_exposure_method=unreal.AutoExposureMethod.AEM_HISTOGRAM,
             override_auto_exposure_min_brightness=True, auto_exposure_min_brightness=ev100,
             override_auto_exposure_max_brightness=True, auto_exposure_max_brightness=ev100,
             override_auto_exposure_bias=True, auto_exposure_bias=0.0)
        a.set_editor_property("settings", s)
        return self._done(a, "FixedExposure", "Lights")

    def fx(self, label, at, system):
        """A Niagara system placed in the level (it plays on its own when the game starts)."""
        a = self._spawn(unreal.NiagaraActor, unreal.Vector(*at))
        a.get_component_by_class(unreal.NiagaraComponent).set_asset(system)
        return self._done(a, label, "FX")

    def player_start(self, at, yaw):
        a = self._spawn(unreal.PlayerStart, unreal.Vector(at[0], at[1], at[2] + 100.0),
                                               unreal.Rotator(roll=0.0, pitch=0.0, yaw=yaw))
        return self._done(a, "PlayerStart", "Gameplay")


TRANSFORM_NAMES = ["Location.X", "Location.Y", "Location.Z", "Rotation.X", "Rotation.Y", "Rotation.Z",
                   "Scale.X", "Scale.Y", "Scale.Z"]


def transform_channels(section):
    """A transform section's channels by name (UE 5.x names them like "Location.X_0")."""
    out = {}
    for i, c in enumerate(section.get_all_channels()):
        name = str(c.get_name())
        key = next((n for n in TRANSFORM_NAMES if name == n or name.startswith(n + "_")), None)
        out[key or (TRANSFORM_NAMES[i] if i < len(TRANSFORM_NAMES) else name)] = c
    return out



def set_play_rate(params, rate):
    """Skeletal animation section play rate: a float before 5.5, a time-warp struct since."""
    try:
        params.set_editor_property("play_rate", float(rate))
        return
    except Exception:
        pass
    try:
        r = params.get_editor_property("play_rate")
        r.set_fixed_play_rate(float(rate))
        params.set_editor_property("play_rate", r)
    except Exception as e:
        warn("play rate not set (%s)" % e)


class Ambient:
    """The level's life: one level sequence that starts with play and loops, moving the mannequins
    that walk. shotpilot renders every shot in a fresh play session, so every shot sees the same
    moment of it (the clock starts at 0 with each shot)."""

    def __init__(self, lv, path, seconds, fps=30):
        self.lv, self.fps, self.frames = lv, fps, int(round(seconds * fps))
        folder, name = path.rsplit("/", 1)
        if EAL.does_asset_exist(path):  # left over from a level deleted to be built again
            EAL.delete_asset(path)
        self.seq = unreal.AssetToolsHelpers.get_asset_tools().create_asset(
            name, folder, unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
        self.seq.set_display_rate(unreal.FrameRate(fps, 1))
        self.seq.set_playback_start(0)
        self.seq.set_playback_end(self.frames)

    def _f(self, t):
        return int(round(t * self.fps))

    def walker(self, label, path, clips, mesh=MANNY):
        """A mannequin moving through `path`, a list of (seconds, x, y, facing yaw), with `clips`, a list
        of (from, to, animation, play rate) in seconds. Keep the facing values continuous (270, not -90,
        after 180) so a turn goes the short way."""
        t0, x0, y0, f0 = path[0]
        a = self.lv.mannequin((x0, y0, 0.0), unreal.Rotator(roll=0.0, pitch=0.0, yaw=f0 - 90.0), mesh, clips[0][2])
        self.lv._done(a, label, "Characters")
        b = self.seq.add_possessable(a)
        sec = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
        sec.set_range(0, self.frames)
        ch = transform_channels(sec)
        lin = unreal.MovieSceneKeyInterpolation.LINEAR
        for (t, x, y, facing) in path:
            for name, value in (("Location.X", x), ("Location.Y", y), ("Location.Z", 0.0), ("Rotation.X", 0.0),
                                ("Rotation.Y", 0.0), ("Rotation.Z", facing - 90.0)):
                ch[name].add_key(unreal.FrameNumber(self._f(t)), float(value), 0.0,
                                 unreal.MovieSceneTimeUnit.DISPLAY_RATE, lin)
        for name in ("Scale.X", "Scale.Y", "Scale.Z"):
            try:
                ch[name].set_default(1.0)
            except Exception:
                pass
        track = b.add_track(unreal.MovieSceneSkeletalAnimationTrack)
        for (c0, c1, anim, rate) in clips:
            s = track.add_section()
            s.set_range(self._f(c0), self._f(c1))
            p = s.get_editor_property("params")
            p.set_editor_property("animation", load(ANIMS + "/" + anim))
            set_play_rate(p, rate)
            s.set_editor_property("params", p)
        return a

    def spin(self, actor, turns=1.0, axis="Rotation.X"):
        """Turn `actor` (a group) about one axis, `turns` full turns per loop, at a steady speed."""
        b = self.seq.add_possessable(actor)
        sec = b.add_track(unreal.MovieScene3DTransformTrack).add_section()
        sec.set_range(0, self.frames)
        ch = transform_channels(sec)
        lin = unreal.MovieSceneKeyInterpolation.LINEAR
        loc, rot = actor.get_actor_location(), actor.get_actor_rotation()
        start = {"Location.X": loc.x, "Location.Y": loc.y, "Location.Z": loc.z,
                 "Rotation.X": rot.roll, "Rotation.Y": rot.pitch, "Rotation.Z": rot.yaw}
        for name, v in start.items():
            ch[name].add_key(unreal.FrameNumber(0), float(v), 0.0, unreal.MovieSceneTimeUnit.DISPLAY_RATE, lin)
        ch[axis].add_key(unreal.FrameNumber(self.frames), float(start[axis] + 360.0 * turns), 0.0,
                         unreal.MovieSceneTimeUnit.DISPLAY_RATE, lin)
        for name in ("Scale.X", "Scale.Y", "Scale.Z"):
            try:
                ch[name].set_default(1.0)
            except Exception:
                pass
        return actor

    def place(self):
        """The actor that plays the sequence by itself from the start of play, forever."""
        a = self.lv._spawn(unreal.LevelSequenceActor, unreal.Vector(0, 0, 0))
        try:
            a.set_sequence(self.seq)
        except Exception:
            setp(a, level_sequence_asset=self.seq)
        ps = a.get_editor_property("playback_settings")
        setp(ps, loop_count=unreal.MovieSceneSequenceLoopCount(value=-1))
        set_on = []  # the auto-play switch has lived on the settings and on the actor across versions
        for name, target in (("settings", ps), ("actor", a)):
            try:
                target.set_editor_property("auto_play", True)
                set_on.append(name)
            except Exception:
                pass
        a.set_editor_property("playback_settings", ps)
        if not set_on:
            warn("auto play not set: the walkers will stand still")
        else:
            log("ambient sequence auto-plays (set on %s)" % ", ".join(set_on))
        self.lv._done(a, "Ambient_Life", "Cinematics")
        EAL.save_loaded_asset(self.seq)
        return a


# ---------------------------------------------------------------------------------------------
# Shared by the other three games: paint from a palette, mannequins walking a closed loop.
# ---------------------------------------------------------------------------------------------

def paint(lv, folder, palette):
    """Make each material of `palette` (name -> keyword arguments of make_material) under
    `folder` and register it on the level by name."""
    for name, kw in palette.items():
        lv.mats[name] = make_material(folder + "M_" + "".join(w.capitalize() for w in name.split("_")), **kw)


NOISE = "float n = 0.5 + 0.25 * sin(WP.x * 0.0041 + 0.7) + 0.25 * sin(WP.y * 0.0037 + 1.9);\n"
GRAIN = "float g = frac(sin(dot(floor(WP.xy / 23.0), float2(12.9898, 78.233))) * 43758.5453);\n"


def loop_path(points, period, phase=0.0):
    """A closed walk as (seconds, x, y, facing) keys over `period`, starting `phase` seconds into
    the loop: `points` is [(seconds, x, y)] with the last point the first one again. Facings follow
    the direction of travel and stay continuous."""
    legs = []
    for (t0, x0, y0), (t1, x1, y1) in zip(points, points[1:]):
        legs.append((t0, t1, x0, y0, x1, y1, math.degrees(math.atan2(y1 - y0, x1 - x0))))

    def at(t):
        t %= period
        for (t0, t1, x0, y0, x1, y1, f) in legs:
            if t0 <= t <= t1:
                u = (t - t0) / max(t1 - t0, 1e-6)
                return x0 + (x1 - x0) * u, y0 + (y1 - y0) * u, f
        return legs[0][2], legs[0][3], legs[0][6]

    times = sorted({0.0, period} | {((t - phase) % period) for (t, _, _) in points})
    keys, last = [], None
    for t in times:
        x, y, f = at(t + phase if t < period else phase + period - 1e-4)
        if last is not None:  # unwrap so a turn goes the short way
            while f - last > 180.0:
                f -= 360.0
            while f - last < -180.0:
                f += 360.0
        keys.append((t, x, y, f))
        last = f
    return keys
