"""Effects: Niagara systems built from the engine's own emitter templates through Epic's
NiagaraToolsets, and the materials they draw with.

The library (every make_* takes a content folder first and returns the saved NiagaraSystem; place it
with `lv.fx(label, at, system)`). The same call always gives the same frames (fixed tick, seeds, warm-up).

  make_fire(folder, size="torch"|"brazier"|"campfire")  flame, hot core, embers. Pair: a point light
        (brazier ~60 cd, radius 600; torch ~25 cd) with brazier_light_function(path, phase).
  make_smoke(folder, scale)        slow soot plume, lit per pixel. Pair: the fire's orange light, or the sun.
  make_steam(folder, scale)        kettle/cup wisps. Pair: a warm point light near the spout.
  make_sparks(folder, interval)    a burst every `interval` s (>= 2 s keeps the flash rule: no more than
        3 bright flashes a second). Pair: a short warm point light if wanted.
  make_leaves(folder, kind="leaf"|"petal")  falling leaves / blossom. Pair: the sun (lit per pixel).
  make_fireflies(folder)           pulsing motes (glow, self-lit). Pair: none; works in the dark.
  make_drizzle(folder)             rain streaks, only visible where a lamp lights them. Pair: a lamp ~90 cd
        above the column.
  make_drips(folder)               a drip that splashes, glints under a lamp. Pair: a lamp ~90 cd near it.
  make_mist(folder)                low ground haze. Pair: the sun or a lamp (it is lit per pixel).
  make_dust(path, material)        hanging dust in a box (the older one; see make_dust_material).

Gotchas: an HLSL body must not use reserved words as variable names (`line`, `point`, `triangle`...): the
material then fails to compile and the render is black or crashes. NullRHI builds cannot tell you.
"""

import json

import unreal

from .core import EAL, setp, make_material

# --------------------------------------------------------------------------- dust (Niagara)
# Hanging dust is the engine's own HangingParticulates emitter template, put into a system and
# tuned through Epic's NiagaraToolsets (UE 5.8, experimental): the same tool calls an AI assistant
# makes over MCP, reached from Python through ToolsetRegistry.ExecuteTool. The project has to enable
# the ToolsetRegistry and NiagaraToolsets plugins; without them the level is built without dust.

NIAGARA_TOOLS = "NiagaraToolsets.NiagaraToolset_System"
HANGING = "/Niagara/DefaultAssets/Templates/Emitters/HangingParticulates.HangingParticulates"


def niagara_tool(tool, **args):
    r = unreal.ToolsetRegistry.execute_tool(NIAGARA_TOOLS, tool, json.dumps(args))
    err = r.get_editor_property("error")
    if err:
        raise RuntimeError("%s: %s" % (tool, err))
    return r.get_value_as_json_string()


def make_dust_material(path):
    """A soft round sprite lit per pixel, shadows included, so dust shows in a beam of moonlight or
    under a lamp and stays dark elsewhere."""
    mel = unreal.MaterialEditingLibrary
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    folder, name = path.rsplit("/", 1)
    mat = unreal.AssetToolsHelpers.get_asset_tools().create_asset(name, folder, unreal.Material,
                                                                  unreal.MaterialFactoryNew())
    setp(mat, blend_mode=unreal.BlendMode.BLEND_TRANSLUCENT, two_sided=True,
         translucency_lighting_mode=unreal.TranslucencyLightingMode.TLM_SURFACE_PER_PIXEL_LIGHTING,
         used_with_niagara_sprites=True)
    uv = mel.create_material_expression(mat, unreal.MaterialExpressionTextureCoordinate, -900, 0)
    pc = mel.create_material_expression(mat, unreal.MaterialExpressionParticleColor, -900, 200)
    soft = mel.create_material_expression(mat, unreal.MaterialExpressionCustom, -400, 100)
    soft.set_editor_property("code", "float2 d = UV * 2.0 - 1.0; return saturate(1.0 - dot(d, d)) * A;")
    soft.set_editor_property("output_type", unreal.CustomMaterialOutputType.CMOT_FLOAT1)
    inputs = []
    for n in ("UV", "A"):
        ci = unreal.CustomInput()
        ci.set_editor_property("input_name", n)
        inputs.append(ci)
    soft.set_editor_property("inputs", inputs)
    mel.connect_material_expressions(uv, "", soft, "UV")
    mel.connect_material_expressions(pc, "A", soft, "A")
    mel.connect_material_property(pc, "", unreal.MaterialProperty.MP_BASE_COLOR)
    mel.connect_material_property(soft, "", unreal.MaterialProperty.MP_OPACITY)
    mel.recompile_material(mat)
    EAL.save_asset(path, only_if_is_dirty=False)
    return mat


def make_glow_material(path, strength=12.0):
    """A soft round sprite that glows in its particle colour (embers, sparks), added to the frame."""
    mel = unreal.MaterialEditingLibrary
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    folder, name = path.rsplit("/", 1)
    mat = unreal.AssetToolsHelpers.get_asset_tools().create_asset(name, folder, unreal.Material,
                                                                  unreal.MaterialFactoryNew())
    setp(mat, blend_mode=unreal.BlendMode.BLEND_ADDITIVE, shading_model=unreal.MaterialShadingModel.MSM_UNLIT,
         two_sided=True, used_with_niagara_sprites=True)
    uv = mel.create_material_expression(mat, unreal.MaterialExpressionTextureCoordinate, -900, 0)
    pc = mel.create_material_expression(mat, unreal.MaterialExpressionParticleColor, -900, 200)
    glow = mel.create_material_expression(mat, unreal.MaterialExpressionCustom, -400, 100)
    glow.set_editor_property("code", "float2 d = UV * 2.0 - 1.0; float r = saturate(1.0 - dot(d, d)); "
                                     "return PC.rgb * (r * r * PA * %.1f);" % strength)
    glow.set_editor_property("output_type", unreal.CustomMaterialOutputType.CMOT_FLOAT3)
    inputs = []
    for n in ("UV", "PC", "PA"):
        ci = unreal.CustomInput()
        ci.set_editor_property("input_name", n)
        inputs.append(ci)
    glow.set_editor_property("inputs", inputs)
    mel.connect_material_expressions(uv, "", glow, "UV")
    mel.connect_material_expressions(pc, "", glow, "PC")  # the colour output is rgb only
    mel.connect_material_expressions(pc, "A", glow, "PA")
    mel.connect_material_property(glow, "", unreal.MaterialProperty.MP_EMISSIVE_COLOR)
    mel.recompile_material(mat)
    EAL.save_asset(path, only_if_is_dirty=False)
    return mat


def make_dust(path, material, rate=40.0, size=(1.0, 2.2), color=(0.85, 0.85, 0.8, 0.9), wind=(4.0, 2.0, 1.0),
              box=(400.0, 400.0, 260.0), life=(6.0, 10.0)):
    """A Niagara system of dust hanging in a 4 x 4 x 2.6 m box around its actor, already settled at
    0 s (warmed up), the same every run (deterministic)."""
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    folder, name = path.rsplit("/", 1)
    system = unreal.AssetToolsHelpers.get_asset_tools().create_asset(name, folder, unreal.NiagaraSystem,
                                                                     unreal.NiagaraSystemFactoryNew())
    ref = {"refPath": "%s.%s" % (path, name)}
    niagara_tool("AddEmitter", system=ref, templateEmitter={"refPath": HANGING}, emitterName="Dust")
    em = {"system": ref, "emitterName": "Dust", "scriptName": "", "moduleName": "", "rendererIndex": -1,
          "inputNameStack": []}

    def put(script, module, input_name, struct, value):
        niagara_tool("SetStackInputData", stackInputRef=dict(em, scriptName=script, moduleName=module, inputNameStack=[input_name]),
                     inputData={"struct": {"refPath": struct}, "value": value})

    f, v3, lc = "/Script/Niagara.NiagaraFloat", "/Script/CoreUObject.Vector3f", "/Script/CoreUObject.LinearColor"
    put("EmitterUpdateScript", "SpawnRate", "SpawnRate", f, {"value": rate})
    put("ParticleSpawnScript", "InitializeParticle", "Lifetime Min", f, {"value": life[0]})
    put("ParticleSpawnScript", "InitializeParticle", "Lifetime Max", f, {"value": life[1]})
    put("ParticleSpawnScript", "InitializeParticle", "Uniform Sprite Size Min", f, {"value": size[0]})
    put("ParticleSpawnScript", "InitializeParticle", "Uniform Sprite Size Max", f, {"value": size[1]})
    put("ParticleSpawnScript", "InitializeParticle", "Color", lc,
        {"r": color[0], "g": color[1], "b": color[2], "a": color[3]})
    put("ParticleSpawnScript", "ShapeLocation", "Box Size", v3, {"x": box[0], "y": box[1], "z": box[2]})
    put("ParticleUpdateScript", "WindForce", "Wind Speed", v3, {"x": wind[0], "y": wind[1], "z": wind[2]})
    niagara_tool("SetRendererData", renderer=dict(em, rendererIndex=0),
                 rendererData={"propertyValues": json.dumps({"Material": {"refPath": material.get_path_name()},
                                                             "bCastShadows": False})})
    niagara_tool("SetEmitterData", emitter=em,
                 emitterData={"propertyValues": json.dumps({"bDeterminism": True, "RandomSeed": 7})})
    system = EAL.load_asset(path)
    setp(system, warmup_time=8.0, warmup_tick_delta=1.0 / 30.0)
    EAL.save_asset(path, only_if_is_dirty=False)
    return system


# --------------------------------------------------------------------------- particle materials
# One HLSL body per material, in a Custom node that sees UV (0..1 on the sprite), PC (particle colour,
# float3) and PA (its alpha), AGE (0..1 over the particle's life), RND (0..1, fixed per particle), T (game time, s) and
# WP (pixel world position, cm) and returns float4(rgb, a). No textures: shapes are noise in code.

NOISE_LIB = """#define HASH(p) frac(sin(dot((p), float2(127.1, 311.7))) * 43758.5453)
#define VN(p) lerp(lerp(HASH(floor(p)), HASH(floor(p) + float2(1, 0)), smoothstep(0, 1, frac(p).x)), lerp(HASH(floor(p) + float2(0, 1)), HASH(floor(p) + float2(1, 1)), smoothstep(0, 1, frac(p).x)), smoothstep(0, 1, frac(p).y))
#define FBM(p) (0.5 * VN(p) + 0.25 * VN((p) * 2.03 + 7.1) + 0.125 * VN((p) * 4.1 + 3.3))
"""


def make_particle_material(path, code, mode="add", depth_fade=0.0, ambient=0.0):
    """A sprite material from an HLSL body (see above). mode: "add" (glows, added to the frame),
    "lit" (translucent, lit per pixel by the level's lights, rgb is the surface colour) or
    "blend" (translucent, unlit). `depth_fade` (cm) softens where a sprite meets geometry; `ambient`
    (lit only) adds that fraction of the colour as light of its own, so it never goes pitch black."""
    mel = unreal.MaterialEditingLibrary
    if EAL.does_asset_exist(path):
        EAL.delete_asset(path)
    folder, name = path.rsplit("/", 1)
    mat = unreal.AssetToolsHelpers.get_asset_tools().create_asset(name, folder, unreal.Material,
                                                                  unreal.MaterialFactoryNew())
    props = dict(two_sided=True, used_with_niagara_sprites=True)
    if mode == "add":
        props.update(blend_mode=unreal.BlendMode.BLEND_ADDITIVE, shading_model=unreal.MaterialShadingModel.MSM_UNLIT)
    elif mode == "blend":
        props.update(blend_mode=unreal.BlendMode.BLEND_TRANSLUCENT, shading_model=unreal.MaterialShadingModel.MSM_UNLIT)
    else:
        props.update(blend_mode=unreal.BlendMode.BLEND_TRANSLUCENT,
                     translucency_lighting_mode=unreal.TranslucencyLightingMode.TLM_SURFACE_PER_PIXEL_LIGHTING)
    setp(mat, **props)
    mk = lambda cls, x, y: mel.create_material_expression(mat, cls, x, y)
    uv = mk(unreal.MaterialExpressionTextureCoordinate, -1100, -200)
    pc = mk(unreal.MaterialExpressionParticleColor, -1100, -80)
    age = mk(unreal.MaterialExpressionParticleRelativeTime, -1100, 40)
    rnd = mk(unreal.MaterialExpressionParticleRandom, -1100, 160)
    t = mk(unreal.MaterialExpressionTime, -1100, 280)
    wp = mk(unreal.MaterialExpressionWorldPosition, -1100, 400)
    node = mk(unreal.MaterialExpressionCustom, -700, 0)
    node.set_editor_property("code", NOISE_LIB + code)
    node.set_editor_property("output_type", unreal.CustomMaterialOutputType.CMOT_FLOAT4)
    inputs = []
    for n in ("UV", "PC", "PA", "AGE", "RND", "T", "WP"):
        ci = unreal.CustomInput()
        ci.set_editor_property("input_name", n)
        inputs.append(ci)
    node.set_editor_property("inputs", inputs)
    for n, src, out in (("UV", uv, ""), ("PC", pc, ""), ("PA", pc, "A"), ("AGE", age, ""), ("RND", rnd, ""),
                        ("T", t, ""), ("WP", wp, "")):
        mel.connect_material_expressions(src, out, node, n)
    rgb = mk(unreal.MaterialExpressionComponentMask, -400, -100)
    setp(rgb, r=True, g=True, b=True, a=False)
    alpha = mk(unreal.MaterialExpressionComponentMask, -400, 100)
    setp(alpha, r=False, g=False, b=False, a=True)
    mel.connect_material_expressions(node, "", rgb, "")
    mel.connect_material_expressions(node, "", alpha, "")
    opacity = alpha
    if depth_fade > 0.0:
        fade = mk(unreal.MaterialExpressionDepthFade, -150, 100)
        setp(fade, fade_distance_default=depth_fade)
        mel.connect_material_expressions(alpha, "", fade, "Opacity")
        opacity = fade
    if mode == "add":  # additive: only the colour counts, so the body multiplies its own alpha in
        mel.connect_material_property(rgb, "", unreal.MaterialProperty.MP_EMISSIVE_COLOR)
    elif mode == "blend":
        mel.connect_material_property(rgb, "", unreal.MaterialProperty.MP_EMISSIVE_COLOR)
        mel.connect_material_property(opacity, "", unreal.MaterialProperty.MP_OPACITY)
    else:
        mel.connect_material_property(rgb, "", unreal.MaterialProperty.MP_BASE_COLOR)
        mel.connect_material_property(opacity, "", unreal.MaterialProperty.MP_OPACITY)
        if ambient > 0.0:
            k = mk(unreal.MaterialExpressionConstant, -400, 300)
            setp(k, r=float(ambient))
            mul = mk(unreal.MaterialExpressionMultiply, -150, 250)
            mel.connect_material_expressions(rgb, "", mul, "A")
            mel.connect_material_expressions(k, "", mul, "B")
            mel.connect_material_property(mul, "", unreal.MaterialProperty.MP_EMISSIVE_COLOR)
    mel.recompile_material(mat)
    EAL.save_asset(path, only_if_is_dirty=False)
    return mat


# --------------------------------------------------------------------------- Niagara builder
# A small layer over NiagaraToolsets so an effect reads as data: a system, its emitters (copied
# from the engine's own emitter templates) and the values put into their module inputs.

INFO_TOOLS = "NiagaraToolsets.NiagaraToolset_Info"
TEMPLATES = "/Niagara/DefaultAssets/Templates/Emitters/%s.%s"
NF = "/Script/Niagara.NiagaraFloat"
NI = "/Script/Niagara.NiagaraInt32"
NB = "/Script/Niagara.NiagaraBool"
V2, V3 = "/Script/CoreUObject.Vector2f", "/Script/CoreUObject.Vector3f"
LC = "/Script/CoreUObject.LinearColor"
NIAGARA_EDITOR = "/Script/NiagaraEditor.NiagaraExt_StackInputData_"
_ENUMS = {}


class Int(int):
    """An integer module input (a bare number is a float)."""


class Enum(str):
    """An enum input given by its display name in the editor ("Cylinder"); the engine's own
    enumerator name is looked up from the input's current value."""


class Hlsl(str):
    """An input computed by an HLSL expression (it can read Particles.*, Emitter.*, Engine.*)."""


class Curve(list):
    """A float curve over normalized age: [(time, value), ...], linear between the points."""


class Link(str):
    """An input linked to a Niagara attribute ("Particles.NormalizedAge")."""


def _decode(text):
    """A tool's result: JSON, sometimes JSON inside a JSON string; its payload is under returnValue."""
    v = text
    for _ in range(3):
        if isinstance(v, str):
            try:
                v = json.loads(v)
            except ValueError:
                return v
    return v["returnValue"] if isinstance(v, dict) and "returnValue" in v else v


def niagara_tool_json(tool, **args):
    return _decode(niagara_tool(tool, **args))


def info_tool(tool, **args):
    r = unreal.ToolsetRegistry.execute_tool(INFO_TOOLS, tool, json.dumps(args))
    err = r.get_editor_property("error")
    if err:
        raise RuntimeError("%s: %s" % (tool, err))
    return _decode(r.get_value_as_json_string())


def _curve_json(points):
    keys = [{"interpMode": "RCIM_Linear", "tangentMode": "RCTM_Auto", "tangentWeightMode": "RCTWM_WeightedNone",
             "time": float(t), "value": float(v), "arriveTangent": 0.0, "arriveTangentWeight": 0.0,
             "leaveTangent": 0.0, "leaveTangentWeight": 0.0} for (t, v) in points]
    return json.dumps({"Curve": {"keys": keys, "defaultValue": 3.4028234663852886e+38,
                                 "preInfinityExtrap": "RCCE_Constant", "postInfinityExtrap": "RCCE_Constant"},
                       "bUseLUT": True, "bExposeCurve": False})


class Emitter:
    """One emitter of a Niagara system, with `put` for the values of its module inputs."""

    def __init__(self, system, name):
        self.system, self.name = system, name

    def at(self, script="", module="", renderer=-1, inputs=()):
        return {"system": self.system.ref, "emitterName": self.name, "scriptName": script, "moduleName": module,
                "rendererIndex": renderer, "inputNameStack": list(inputs)}

    def _enum(self, script, module, inputs, display):
        cur = niagara_tool_json("GetStackInputData", stackInputRef=self.at(script, module, inputs=inputs))
        enum_path = cur["value"]["enum"]["refPath"]
        names = _ENUMS.get(enum_path)
        if names is None:
            vals = _decode(info_tool("UEnum_Info", enum={"refPath": enum_path}))["Values"]
            names = _ENUMS[enum_path] = {v["DisplayName"]: v["Name"] for v in vals}
        if display not in names:
            raise RuntimeError("%s has no value %r (has %s)" % (enum_path, display, sorted(names)))
        prefix = enum_path.rsplit(".", 1)[-1]
        return {"struct": {"refPath": NIAGARA_EDITOR + "Enum"},
                "value": {"enum": {"refPath": enum_path}, "enumName": "%s::%s" % (prefix, names[display]),
                          "displayName": display}}

    def _value(self, script, module, inputs, v):
        if isinstance(v, Enum):
            return self._enum(script, module, inputs, str(v))
        if isinstance(v, Hlsl):
            return {"struct": {"refPath": NIAGARA_EDITOR + "HlslExpression"}, "value": {"hlslExpression": str(v)}}
        if isinstance(v, Link):
            return {"struct": {"refPath": NIAGARA_EDITOR + "Linked"},
                    "value": {"linkedVariable": {"name": str(v), "type": {"classStructOrEnum": {"refPath": NF}}}}}
        if isinstance(v, Curve):
            return {"struct": {"refPath": NIAGARA_EDITOR + "DataInterface"},
                    "value": {"propertyValues": _curve_json(v)}}
        if isinstance(v, bool):
            return {"struct": {"refPath": NB}, "value": {"value": -1 if v else 0}}
        if isinstance(v, Int):
            return {"struct": {"refPath": NI}, "value": {"value": int(v)}}
        if isinstance(v, (int, float)):
            return {"struct": {"refPath": NF}, "value": {"value": float(v)}}
        if isinstance(v, (tuple, list)):
            keys = {2: ("x", "y"), 3: ("x", "y", "z"), 4: ("r", "g", "b", "a")}[len(v)]
            struct = {2: V2, 3: V3, 4: LC}[len(v)]
            return {"struct": {"refPath": struct}, "value": dict(zip(keys, [float(x) for x in v]))}
        raise TypeError("cannot put %r" % (v,))

    def put(self, script, module, name, value, sub=()):
        inputs = [name] + list(sub)
        niagara_tool("SetStackInputData", stackInputRef=self.at(script, module, inputs=inputs),
                     inputData=self._value(script, module, inputs, value))

    def emit(self, module, **kw):
        """Emitter update inputs (EmitterState, SpawnRate, SpawnBurst_Instantaneous)."""
        for k, v in kw.items():
            self.put("EmitterUpdateScript", module, k.replace("_", " "), v)

    def spawn(self, module, **kw):
        """Particle spawn inputs (InitializeParticle, ShapeLocation, AddVelocity); `_` stands for a space."""
        for k, v in kw.items():
            self.put("ParticleSpawnScript", module, k.replace("_", " "), v)

    def update(self, module, **kw):
        for k, v in kw.items():
            self.put("ParticleUpdateScript", module, k.replace("_", " "), v)

    def add_module(self, script, asset, after=""):
        """Add a module to a script's stack, after the module named `after` (at the end if empty)."""
        niagara_tool("AddModule", moduleLocationRef=self.at(script, after), moduleAsset={"refPath": asset})

    def disable(self, module, script="ParticleUpdateScript"):
        niagara_tool("SetModuleEnabled", moduleRef=self.at(script, module), bEnabled=False)

    def renderer(self, material, index=0, **props):
        props = dict(props, Material={"refPath": material.get_path_name()})
        niagara_tool("SetRendererData", renderer=self.at(renderer=index),
                     rendererData={"propertyValues": json.dumps(props)})

    def data(self, **props):
        niagara_tool("SetEmitterData", emitter=self.at(), emitterData={"propertyValues": json.dumps(props)})

    def inputs(self):
        return niagara_tool_json("GetEmitterInputValues", emitterRef=self.at())


class System:
    """A new Niagara system asset (an existing one at the path is replaced)."""

    def __init__(self, path):
        self.path = path
        folder, self.name = path.rsplit("/", 1)
        if EAL.does_asset_exist(path):
            EAL.delete_asset(path)
        self.asset = unreal.AssetToolsHelpers.get_asset_tools().create_asset(
            self.name, folder, unreal.NiagaraSystem, unreal.NiagaraSystemFactoryNew())
        self.ref = {"refPath": "%s.%s" % (path, self.name)}

    def add(self, name, template, seed):
        niagara_tool("AddEmitter", system=self.ref,
                     templateEmitter={"refPath": TEMPLATES % (template, template)}, emitterName=name)
        e = Emitter(self, name)
        e.data(bDeterminism=True, RandomSeed=int(seed))
        return e

    def save(self, warmup=0.0):
        system = EAL.load_asset(self.path)
        if warmup:
            setp(system, warmup_time=float(warmup), warmup_tick_delta=1.0 / 30.0)
        EAL.save_asset(self.path, only_if_is_dirty=False)
        return system


# --------------------------------------------------------------------------- effects library
# Each make_* builds (or rebuilds) one Niagara system under `folder` and returns it. The actor that
# plays it sits at the base of the effect (fire, smoke, steam, sparks, mist) or at the centre of
# the volume (leaves, fireflies, drizzle: see each function). Everything is deterministic: fixed
# seeds, a fixed tick, and a warm-up long enough that the first frame of a shot already shows a
# settled effect (shotpilot starts each shot's play at 0 s). Materials come along under `folder`.
#
# Lights are not part of a system: pair fire with a point light (`brazier_light_function` breathes
# one), sparks with a short-lived warm light if you want one, drizzle and mist with the lamp that
# lights them (they are lit per pixel and are invisible in the dark otherwise).

_UPDATE_BEFORE_SOLVER = "ScaleColor"  # new force modules go after this one (the solver is last)


def _emitter(s, name, template, seed, rate=None, life=(1.0, 2.0), size=(1.0, 2.0), color=(1.0, 1.0, 1.0, 1.0),
             rotation=(0.0, 360.0)):
    """An emitter from `template` with its particles' basics set: spawn rate, life, size, colour and
    the sprite's start angle. The template's own alpha fade is switched off (materials fade by age)."""
    e = s.add(name, template, seed)
    if rate is not None:
        e.emit("SpawnRate", SpawnRate=float(rate))
    e.spawn("InitializeParticle", Lifetime_Min=float(life[0]), Lifetime_Max=float(life[1]),
            Uniform_Sprite_Size_Min=float(size[0]), Uniform_Sprite_Size_Max=float(size[1]), Color=color,
            Sprite_Rotation_Angle_Min=float(rotation[0]), Sprite_Rotation_Angle_Max=float(rotation[1]))
    e.disable("ScaleColor")
    return e


def _shape(e, kind, **kw):
    """Where particles are born: "Cylinder" (radius, height), "Box / Plane" (box_size), "Sphere" (radius)."""
    e.spawn("ShapeLocation", Shape_Primitive=Enum(kind))
    e.spawn("ShapeLocation", **kw)


def _cone(e, speed, angle=10.0, axis=(0.0, 0.0, 1.0)):
    e.spawn("AddVelocity", Velocity_Mode=Enum("In Cone"), Velocity_Speed=float(speed), Cone_Axis=axis,
            Cone_Angle=float(angle))


def _forces(e, gravity=(0.0, 0.0, 0.0), drag=0.0, curl=None):
    """Gravity (cm/s^2, + is up), linear drag, and a curl noise force `curl` = (strength, frequency)."""
    e.update("GravityForce", Gravity=tuple(float(g) for g in gravity))
    if drag:
        e.update("Drag", Use_Linear_Drag=True, Drag=float(drag))
    else:
        e.update("Drag", Use_Linear_Drag=False)
    if curl:
        e.add_module("ParticleUpdateScript", "/Niagara/Modules/Update/Forces/CurlNoiseForce.CurlNoiseForce",
                     after="Drag")
        e.update("CurlNoiseForce", Noise_Strength=float(curl[0]), Noise_Frequency=float(curl[1]))


def _grow(e, curve):
    """Sprite scale over the particle's life: [(age 0..1, factor), ...]."""
    e.add_module("ParticleUpdateScript", "/Niagara/Modules/Update/Size/ScaleSpriteSize.ScaleSpriteSize")
    e.update("ScaleSpriteSize", Scale_Sprite_Size_Mode=Enum("Uniform Curve"), Uniform_Curve_Sprite_Scale=Curve(curve))


def _draw(e, material, sort=False, **props):
    if sort:
        props.setdefault("sortMode", "ViewDistance")
    e.renderer(material, bCastShadows=False, **props)


def _finish(s, warmup, fixed_tick=1.0 / 60.0):
    """Fixed simulation step (so a shot does not depend on the frame rate) and the warm-up."""
    niagara_tool("SetSystemData", system=s.ref, systemData={"propertyValues": json.dumps(
        {"bFixedTickDelta": True, "fixedTickDeltaTime": fixed_tick})})
    return s.save(warmup)


def _lit_ambient(path, code, ambient, depth_fade):
    return make_particle_material(path, code, mode="lit", depth_fade=depth_fade, ambient=ambient)


# ------------------------------------------------------------------ fire
FLAME = """float2 p = UV * 2.0 - 1.0;
float h = saturate(0.5 - 0.5 * p.y);
float sd = RND * 31.7;
float tt = T * (1.5 + RND * 0.9);
float n1 = FBM(float2(p.x * 1.7 + sd, h * 2.4 - tt));
float n2 = FBM(float2(p.x * 1.1 + sd * 0.7 + 4.0, h * 1.6 - tt * 0.7 + 9.0));
float x = p.x + (n2 - 0.5) * 1.1 * h;
float w = 0.66 * pow(1.0 - h, 0.7) * (0.6 + 0.8 * n1);
float d = abs(x) / max(w, 0.02);
float body = saturate(1.0 - d);
body = body * body * (3.0 - 2.0 * body);
float tip = saturate((1.0 - h) * 1.7 + (n1 - 0.45) * 1.5);
float dens = body * smoothstep(0.0, 0.14, h) * smoothstep(0.08, 0.5, tip);
float life = smoothstep(0.0, 0.14, AGE) * (1.0 - smoothstep(0.45, 1.0, AGE));
float heat = saturate(dens * 0.85 + (1.0 - h) * 0.3 - AGE * 0.6);
float3 col = lerp(float3(0.45, 0.03, 0.01), float3(1.0, 0.3, 0.03), smoothstep(0.0, 0.4, heat));
col = lerp(col, float3(1.0, 0.62, 0.18), smoothstep(0.55, 1.0, heat));
return float4(col * dens * life * %(power).2f * PC.rgb * PA, 1.0);"""

GLOW_BLOB = """float2 p = UV * 2.0 - 1.0;
float r = saturate(1.0 - dot(p, p));
float env = sin(AGE * 3.14159);
return float4(PC.rgb * (r * r * r * env * env * PA * %(power).2f), 1.0);"""

EMBER = """float2 p = UV * 2.0 - 1.0;
float r = saturate(1.0 - dot(p, p));
float fade = smoothstep(0.0, 0.08, AGE) * (1.0 - smoothstep(0.5, 1.0, AGE));
float shimmer = 0.8 + 0.2 * sin(T * 6.0 + RND * 50.0);
float3 col = lerp(float3(1.0, 0.55, 0.12), float3(0.8, 0.1, 0.02), AGE);
return float4(col * (r * r * r) * fade * shimmer * PC.rgb * PA * %(power).2f, 1.0);"""

FLAME_GAIN = 0.235  # the sprites add up: 1.7 x 0.235 = 0.4 is a clear orange flame with a yellow core
FIRE_SIZES = {"torch": 0.45, "brazier": 1.0, "campfire": 1.45}


def make_fire(folder, name="NS_Fire", size="brazier", embers=True, seed=11, power=1.7):
    """A flame with a hot core, a faint glow hugging the flame and rising embers. `size` is "torch" (flame
    about 30 cm tall), "brazier" (70 cm) or "campfire" (100 cm), or a number scaling the brazier.
    Origin at the base of the flame. `power` is the flame's brightness (default 1.7 reads as an orange
    flame with a yellow core at EV 1.0-1.3; 1.0 is a dim fire, above 2.5 it burns out to white). Pair
    with a point light (brazier about 60 cd, torch 25 cd, campfire 80 cd, orange 1.0, 0.52, 0.22,
    radius 500-700) driven by `brazier_light_function`; give the light scatter=0 (or little) unless the
    level has volumetric fog, or it shows as a round glow of its own."""
    k = FIRE_SIZES.get(size, size) if isinstance(size, str) else float(size)
    h = 70.0 * k
    m = folder + "/Materials/"
    s = System(folder + "/" + name)
    flame = _emitter(s, "Flame", "Fountain", seed, rate=30.0, life=(0.7, 1.0), size=(0.95 * h, 1.3 * h),
                     rotation=(-12.0, 12.0))
    _shape(flame, "Cylinder", Cylinder_Radius=0.27 * h, Cylinder_Height=2.0)
    flame.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, 0.5 * h))
    _cone(flame, 0.22 * h, angle=6.0)
    _forces(flame, curl=(0.9 * h, 0.012))
    _grow(flame, [(0.0, 0.55), (0.25, 1.0), (1.0, 0.6)])
    _draw(flame, make_particle_material(m + "M_Flame", FLAME % {"power": power * FLAME_GAIN}, "add"))

    core = _emitter(s, "Core", "Fountain", seed + 1, rate=18.0, life=(0.5, 0.8), size=(0.5 * h, 0.7 * h),
                    color=(1.0, 0.9, 0.55, 1.0), rotation=(-6.0, 6.0))
    _shape(core, "Cylinder", Cylinder_Radius=0.1 * h, Cylinder_Height=2.0)
    core.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, 0.3 * h))
    _cone(core, 0.15 * h, angle=4.0)
    _forces(core, curl=(0.4 * h, 0.015))
    _grow(core, [(0.0, 0.6), (0.3, 1.0), (1.0, 0.5)])
    _draw(core, make_particle_material(m + "M_FlameCore", FLAME % {"power": power * 1.1 * FLAME_GAIN}, "add"))

    glow = _emitter(s, "Glow", "Fountain", seed + 2, rate=8.0, life=(2.0, 2.0), size=(1.3 * h, 1.6 * h),
                    color=(1.0, 0.34, 0.06, 1.0))
    _shape(glow, "Sphere", Sphere_Radius=0.05 * h)
    glow.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, 0.5 * h))
    _cone(glow, 1.0, angle=1.0)
    _forces(glow)
    _draw(glow, make_particle_material(m + "M_FlameGlow", GLOW_BLOB % {"power": 0.035}, "add"))

    if embers:
        em = _emitter(s, "Embers", "Fountain", seed + 3, rate=5.0 * k + 2.0, life=(1.8, 3.2),
                      size=(max(0.9, 1.0 * k), max(1.8, 2.0 * k)), color=(1.0, 1.0, 1.0, 1.0))
        _shape(em, "Cylinder", Cylinder_Radius=0.25 * h, Cylinder_Height=2.0)
        em.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, 0.6 * h))
        _cone(em, 0.8 * h, angle=28.0)
        _forces(em, gravity=(0.0, 0.0, -14.0), drag=0.35, curl=(2.2 * h, 0.014))
        _draw(em, make_particle_material(m + "M_FlameEmber", EMBER % {"power": 7.0}, "add"))
    return _finish(s, warmup=4.0)


# ------------------------------------------------------------------ smoke and steam
PUFF = """float2 p = UV * 2.0 - 1.0;
float n = FBM(p * %(freq).2f + RND * 23.0 + float2(0.0, -T * %(boil).3f));
float edge = saturate(1.0 - dot(p, p) * (0.9 + 0.6 * n));
float dens = pow(edge, 1.4) * smoothstep(0.12, 0.6, n * 0.8 + edge * 0.5);
float life = smoothstep(0.0, %(fade_in).2f, AGE) * pow(1.0 - smoothstep(%(fade_out).2f, 1.0, AGE), 1.3);
float3 col = PC.rgb * (0.75 + 0.5 * n) * lerp(0.8, 1.15, AGE);
return float4(col, saturate(dens * life * PA * %(opacity).2f));"""


def make_smoke(folder, name="NS_Smoke", scale=1.0, wind=(18.0, 6.0), color=(0.34, 0.32, 0.31), opacity=0.55,
               seed=21, ambient=0.3):
    """A slow, soft plume (a chimney, a campfire, smouldering ruins). Origin at the source; `scale` 1.0
    puffs about 40 cm wide that open up to 1.6 m and climb some 4 m before they fade. `wind` is the
    drift speed in cm/s (x, y), `color` the soot colour (lit per pixel by the level's lights, so it
    takes the sun or the fire's orange). `ambient` keeps the shadow side from going black."""
    k = float(scale)
    s = System(folder + "/" + name)
    e = _emitter(s, "Smoke", "Fountain", seed, rate=3.6, life=(6.0, 9.0), size=(34.0 * k, 46.0 * k),
                 color=color + (1.0,))
    _shape(e, "Sphere", Sphere_Radius=8.0 * k)
    _cone(e, 40.0 * k, angle=7.0)
    drag = 0.2
    _forces(e, gravity=(wind[0] * drag, wind[1] * drag, 12.0 * k), drag=drag, curl=(28.0 * k, 0.012))
    _grow(e, [(0.0, 0.5), (0.35, 1.5), (1.0, 3.6)])
    _draw(e, _lit_ambient(folder + "/Materials/M_Smoke", PUFF % dict(freq=1.6, boil=0.05, fade_in=0.1, fade_out=0.3,
                                                                    opacity=opacity), ambient, 40.0), sort=True)
    return _finish(s, warmup=10.0)


def make_steam(folder, name="NS_Steam", scale=1.0, opacity=0.34, seed=31, ambient=0.4):
    """A wisp of steam (a kettle spout, a tea cup): small white puffs that rise, wobble and vanish within
    about two seconds. Origin at the spout. `scale` 1.0 is a kettle (puffs open from 4 to 16 cm)."""
    k = float(scale)
    s = System(folder + "/" + name)
    e = _emitter(s, "Steam", "Fountain", seed, rate=15.0, life=(1.4, 2.2), size=(3.5 * k, 5.0 * k),
                 color=(0.96, 0.96, 0.95, 1.0))
    _shape(e, "Sphere", Sphere_Radius=0.8 * k)
    _cone(e, 20.0 * k, angle=12.0)
    _forces(e, gravity=(0.0, 0.0, 28.0 * k), drag=0.8, curl=(40.0 * k, 0.06 / max(k, 0.2)))
    _grow(e, [(0.0, 0.6), (1.0, 3.4)])
    _draw(e, _lit_ambient(folder + "/Materials/M_Steam", PUFF % dict(freq=2.2, boil=0.2, fade_in=0.15, fade_out=0.2,
                                                                    opacity=opacity), ambient, 0.0), sort=True)
    return _finish(s, warmup=3.0)


# ------------------------------------------------------------------ sparks
SPARK = """float2 p = UV * 2.0 - 1.0;
float ln = pow(saturate(1.0 - abs(p.x)), 3.0) * pow(saturate(1.0 - abs(p.y)), 0.7);
float fade = (1.0 - AGE * AGE);
float3 col = lerp(float3(1.0, 0.86, 0.5), float3(1.0, 0.3, 0.04), saturate(AGE * 1.4));
return float4(col * ln * fade * PC.rgb * PA * %(power).2f, 1.0);"""


def make_sparks(folder, name="NS_Sparks", interval=3.0, count=22, speed=(180.0, 520.0), angle=55.0, seed=41,
                power=9.0, length=18.0, width=2.4, lead=0.4):
    """A burst of sparks every `interval` seconds (a sword clash, a forge, a grinder): `count` streaks leave
    the origin inside a cone of `angle` degrees about straight up at `speed` cm/s, arc under gravity and
    die out within a second. Bursts stay at least 2 s apart by default, well inside the rule of no more
    than three bright flashes a second (each spark is a small streak, never a full-frame flash). The
    warm-up ends `lead` seconds (0.4) after a burst, so the first burst in play comes about
    `interval - lead` s into the shot (measured: up to 0.3 s earlier still) and then every `interval`."""
    s = System(folder + "/" + name)
    e = _emitter(s, "Sparks", "OmnidirectionalBurst", seed, life=(0.5, 1.1), size=(1.0, 1.0))
    e.emit("EmitterState", Loop_Behavior=Enum("Infinite"), Loop_Duration=float(interval))
    e.emit("SpawnBurst_Instantaneous", Spawn_Count=Int(count), Spawn_Time=0.0)
    e.spawn("InitializeParticle", Sprite_Size_Mode=Enum("Non-Uniform"), Sprite_Size=(float(width), float(length)))
    _shape(e, "Sphere", Sphere_Radius=3.0)
    e.spawn("AddVelocity", Velocity_Mode=Enum("In Cone"), Cone_Axis=(0.0, 0.0, 1.0), Cone_Angle=float(angle))
    e.put("ParticleSpawnScript", "AddVelocity", "Velocity Speed", float(sum(speed) / 2.0))
    _forces(e, gravity=(0.0, 0.0, -980.0), drag=0.5)
    _draw(e, make_particle_material(folder + "/Materials/M_Spark", SPARK % {"power": power}, "add"),
          alignment="VelocityAligned", facingMode="FaceCamera")
    return _finish(s, warmup=2.0 * interval + lead)


# ------------------------------------------------------------------ leaves and petals
LEAF = """float2 p = UV * 2.0 - 1.0;
float ang = RND * 6.2832 + T * (0.5 + RND * 0.9);
float2 q = float2(cos(ang) * p.x - sin(ang) * p.y, sin(ang) * p.x + cos(ang) * p.y);
float flip = abs(cos(T * (0.9 + RND * 1.3) + RND * 12.0)) * 0.85 + 0.15;
q.x = q.x / flip;
float width = %(width).2f * (1.0 - q.y * q.y * %(taper).2f);
float body = smoothstep(0.0, 0.08, width - abs(q.x)) * step(abs(q.y), 0.95);
float vein = 1.0 - 0.35 * smoothstep(0.05, 0.0, abs(q.x));
float h = frac(RND * 7.31);
float3 a = float3(%(c0)s);
float3 b = float3(%(c1)s);
float3 c = float3(%(c2)s);
float3 col = h < 0.5 ? lerp(a, b, h * 2.0) : lerp(b, c, (h - 0.5) * 2.0);
float fade = smoothstep(0.0, 0.06, AGE) * (1.0 - smoothstep(0.9, 1.0, AGE));
return float4(col * vein, body * fade * PA);"""

LEAF_KINDS = {
    "leaf": dict(width=0.42, taper=0.9, size=(8.0, 13.0), c0="0.30, 0.42, 0.08", c1="0.72, 0.42, 0.07", c2="0.55, 0.14, 0.05"),
    "petal": dict(width=0.5, taper=0.7, size=(5.0, 8.0), c0="0.95, 0.62, 0.72", c1="0.98, 0.78, 0.82", c2="1.0, 0.9, 0.9"),
}


def make_leaves(folder, name="NS_Leaves", kind="leaf", area=(900.0, 900.0), height=350.0, count=48, seed=51,
                ambient=0.45):
    """Leaves ("leaf": greens, ambers, reds) or blossom petals ("petal": pinks) tumbling down. The volume is
    `area` wide and `height` tall, centred on the actor: they are born across the top and fall at about
    35 cm/s, swaying, spinning and turning edge-on. `count` is roughly how many are in the air at once.
    Lit per pixel, so they take the sun's colour."""
    spec = LEAF_KINDS[kind]
    life = (9.0, 12.0)
    s = System(folder + "/" + name)
    e = _emitter(s, "Leaves", "Fountain", seed, rate=count / 10.5, life=life, size=spec["size"], rotation=(0.0, 0.0))
    _shape(e, "Box / Plane", Box_Size=(area[0], area[1], 10.0))
    e.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, height / 2.0))
    e.spawn("AddVelocity", Velocity_Mode=Enum("Linear"), Velocity=(8.0, 3.0, -height / 11.0))
    _forces(e, gravity=(2.0, 0.8, -6.0), drag=0.3, curl=(34.0, 0.006))
    code = LEAF % dict(width=spec["width"], taper=spec["taper"], c0=spec["c0"], c1=spec["c1"], c2=spec["c2"])
    _draw(e, _lit_ambient(folder + "/Materials/M_" + kind.capitalize(), code, ambient, 0.0), sort=True)
    return _finish(s, warmup=life[1] + 1.0)


# ------------------------------------------------------------------ fireflies
FIREFLY = """float2 p = UV * 2.0 - 1.0;
float r = saturate(1.0 - dot(p, p));
float pulse = pow(saturate(0.5 + 0.5 * sin(T * (0.8 + RND * 0.9) + RND * 60.0)), 2.0);
float fade = smoothstep(0.0, 0.15, AGE) * (1.0 - smoothstep(0.85, 1.0, AGE));
float3 col = lerp(float3(0.65, 1.0, 0.22), float3(1.0, 0.9, 0.3), frac(RND * 3.7));
float glow = pow(r, 8.0) + 0.22 * r * r;
return float4(col * glow * pulse * fade * PC.rgb * PA * %(power).2f, 1.0);"""


def make_fireflies(folder, name="NS_Fireflies", area=(700.0, 700.0, 220.0), count=24, seed=61, power=7.0):
    """Fireflies (or floating light motes): small warm-green lights drifting in a box `area` (cm) around the
    actor, each pulsing slowly (a period of 4 to 8 s, never a flash). `count` is about how many exist."""
    life = (9.0, 14.0)
    s = System(folder + "/" + name)
    e = _emitter(s, "Fireflies", "Fountain", seed, rate=count / 11.5, life=life, size=(6.0, 9.0))
    _shape(e, "Box / Plane", Box_Size=tuple(area))
    e.spawn("AddVelocity", Velocity_Mode=Enum("Linear"), Velocity=(0.0, 0.0, 0.0))
    _forces(e, drag=0.4, curl=(16.0, 0.01))
    _draw(e, make_particle_material(folder + "/Materials/M_Firefly", FIREFLY % {"power": power}, "add"))
    return _finish(s, warmup=life[1] + 1.0)


# ------------------------------------------------------------------ rain
STREAK = """float2 p = UV * 2.0 - 1.0;
float ln = pow(saturate(1.0 - abs(p.x)), 2.0) * pow(saturate(1.0 - abs(p.y)), 0.6);
return float4(float3(0.85, 0.92, 1.0), ln * PA * %(opacity).2f);"""

DROP = """float2 p = UV * 2.0 - 1.0;
float r = saturate(1.0 - dot(p, p));
return float4(float3(0.85, 0.92, 1.0), pow(r, 0.7) * PA * %(opacity).2f);"""


DROP_GLINT = """float2 p = UV * 2.0 - 1.0;
float r = saturate(1.0 - dot(p, p));
return float4(float3(0.6, 0.78, 1.0) * (pow(r, 0.7) * PA * %(opacity).2f * 1.6), 1.0);"""


def make_drizzle(folder, name="NS_Drizzle", area=(320.0, 320.0), height=320.0, rate=240.0, speed=560.0,
                 opacity=0.42, seed=71):
    """A light drizzle: thin streaks falling through a column `area` wide and `height` tall, centred on the
    actor. They are lit per pixel and carry no light of their own, so they show only where a lamp
    reaches them (put the lamp above the column; about 90 cd with the fixed exposure). `rate` is
    streaks per second (about 240 for 3 m x 3 m; scale it with the area)."""
    life = height / speed
    s = System(folder + "/" + name)
    e = _emitter(s, "Rain", "Fountain", seed, rate=rate, life=(life, life), size=(1.0, 1.0))
    _shape(e, "Box / Plane", Box_Size=(area[0], area[1], 10.0))
    e.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, height / 2.0),
            Sprite_Size_Mode=Enum("Non-Uniform"), Sprite_Size=(0.55, 24.0))
    e.spawn("AddVelocity", Velocity_Mode=Enum("Linear"), Velocity=(40.0, 0.0, -speed))
    _forces(e)
    _draw(e, make_particle_material(folder + "/Materials/M_Streak", STREAK % {"opacity": opacity}, "lit"),
          alignment="VelocityAligned")
    return _finish(s, warmup=3.0)


def _drop_material(path, opacity, glint, ambient):
    if glint:
        return make_particle_material(path, DROP_GLINT % {"opacity": opacity}, "add")
    return make_particle_material(path, DROP % {"opacity": opacity}, "lit", ambient=ambient)


def make_drips(folder, name="NS_Drips", intervals=(1.7, 2.9), drop=200.0, opacity=0.9, seed=81, glint=True,
               ambient=0.6):
    """Water dripping from a point (a ceiling, a pipe) onto the floor `drop` cm below the actor: one drop
    for each interval in `intervals` (seconds), each falling under gravity and bursting into a few
    droplets on the floor at the very moment it lands. Lit per pixel, so a lamp near the drip makes them
    glint (about 90 cd with the fixed exposure). By default (`glint=True`) the drops are added light, so
    they show with no lamp near (a cold blue-white glint); `glint=False` lights them per pixel instead."""
    fall = (2.0 * drop / 980.0) ** 0.5
    s = System(folder + "/" + name)
    for i, iv in enumerate(intervals):
        d = _emitter(s, "Drop%d" % i, "OmnidirectionalBurst", seed + i, life=(fall, fall), size=(1.0, 1.0))
        d.emit("EmitterState", Loop_Behavior=Enum("Infinite"), Loop_Duration=float(iv))
        d.emit("SpawnBurst_Instantaneous", Spawn_Count=Int(1), Spawn_Time=0.0)
        d.spawn("InitializeParticle", Sprite_Size_Mode=Enum("Non-Uniform"), Sprite_Size=(1.6, 8.0))
        _shape(d, "Sphere", Sphere_Radius=0.2)
        _cone(d, 40.0, angle=1.0, axis=(0.0, 0.0, -1.0))  # a little push down: aligned sprites need a velocity
        _forces(d, gravity=(0.0, 0.0, -980.0))
        _draw(d, _drop_material(folder + "/Materials/M_Drop", opacity, glint, ambient), alignment="VelocityAligned")
        sp = _emitter(s, "Splash%d" % i, "OmnidirectionalBurst", seed + 10 + i, life=(0.3, 0.5), size=(2.4, 4.0))
        sp.emit("EmitterState", Loop_Behavior=Enum("Infinite"), Loop_Duration=float(iv))
        sp.emit("SpawnBurst_Instantaneous", Spawn_Count=Int(10), Spawn_Time=fall)
        _shape(sp, "Sphere", Sphere_Radius=1.0)
        sp.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, -drop + 1.0))
        sp.spawn("AddVelocity", Velocity_Mode=Enum("In Cone"), Cone_Axis=(0.0, 0.0, 1.0), Cone_Angle=50.0)
        sp.put("ParticleSpawnScript", "AddVelocity", "Velocity Speed", 130.0)
        _forces(sp, gravity=(0.0, 0.0, -980.0), drag=0.3)
        _draw(sp, _drop_material(folder + "/Materials/M_Splash", opacity, glint, ambient))
    return _finish(s, warmup=max(intervals) * 2.0 + 0.3)


# ------------------------------------------------------------------ ground mist
MIST = """float2 p = UV * 2.0 - 1.0;
float n = FBM(p * 1.3 + RND * 17.0 + float2(T * 0.01, 0.0));
float edge = saturate(1.0 - dot(p, p));
float dens = edge * edge * smoothstep(0.25, 0.85, n + edge * 0.35);
float life = sin(AGE * 3.14159);
return float4(PC.rgb * (0.8 + 0.4 * n), saturate(dens * life * PA * %(opacity).2f));"""


def make_mist(folder, name="NS_Mist", area=(800.0, 800.0), thickness=70.0, opacity=0.16, color=(0.74, 0.78, 0.86),
              count=26, seed=91, ambient=0.55):
    """Low ground mist: big soft wisps (1.6 to 3 m across) drifting slowly over `area`, `thickness` cm deep,
    centred on the actor at floor level. `count` is about how many wisps exist; `opacity` is each one's
    density (0.16 is a haze you read as a layer; 0.3 is a bank). Lit per pixel and softened where it
    meets the floor, so it takes the sun or a lamp."""
    life = (14.0, 18.0)
    s = System(folder + "/" + name)
    e = _emitter(s, "Mist", "Fountain", seed, rate=count / 16.0, life=life, size=(160.0, 300.0), color=color + (1.0,))
    _shape(e, "Box / Plane", Box_Size=(area[0], area[1], thickness))
    e.spawn("InitializeParticle", UsePositionOffset=True, Position_Offset=(0.0, 0.0, thickness * 0.5 + 10.0))
    e.spawn("AddVelocity", Velocity_Mode=Enum("Linear"), Velocity=(7.0, 3.0, 0.0))
    _forces(e, drag=0.5, curl=(8.0, 0.004))
    _grow(e, [(0.0, 0.8), (1.0, 1.25)])
    _draw(e, _lit_ambient(folder + "/Materials/M_Mist", MIST % {"opacity": opacity * 3.0}, ambient, 60.0), sort=True)
    return _finish(s, warmup=life[1] + 1.0)


FIRE = ("float f = 0.75 + 0.15 * sin(T * 11.0 + WP.x * 0.07) * sin(T * 4.3 + WP.y * 0.05) + 0.1 * sin(T * 23.0 + WP.z * 0.2);\n"
        "return float3(1.0, 0.36, 0.06) * 7.0 * f;")


def brazier_light_function(path, phase):
    """A fire's breathing light: gentle and continuous (never a flash)."""
    return make_material(path, emissive="return 0.82 + 0.1 * sin(T * 9.0 + %.2f) + 0.08 * sin(T * 21.0 + %.2f);"
                         % (phase, 2.0 * phase), light_function=True)
