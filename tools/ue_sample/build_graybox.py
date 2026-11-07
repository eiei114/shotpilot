"""Build the graybox level of a sample game (D-54).

The four sample games are ordinary Unreal projects made in the editor from the Third Person
(Blueprint) template: no gameplay, you can only walk around. This script lays out one game's level
from the template's prototype meshes and mannequins, once. Afterwards the project does not depend on
it: the level is an ordinary asset you can keep editing by hand.

Run it inside the Unreal Editor of that project:
    File > Execute Python Script...   (the project name picks the game)
or in the Output Log, Python mode:
    py "C:/path/to/build_graybox.py" Lanternless
or as a commandlet (no window, about a minute with the editor start):
    UnrealEditor-Cmd.exe <Game>.uproject -run=pythonscript -script="C:/path/to/build_graybox.py <Game>" -unattended -nopause -NullRHI

Pass --audit (with an existing level) to only open it and log a structural audit (graybox/audit.py:
z-fighting, floating parts, bodies inside solids, duplicates); --audit-out=<file> also writes the
report. Every build runs the same audit at its end. Nothing is saved by an audit.

An existing level is never overwritten. Pass --rebuild to build it again: the game's whole content
folder (/Game/<Game>/: the map, its paint, effects, animations and sequences) is first copied to
<Project>/Saved/GrayboxBackups/<time>/ and then deleted. Hand edits made in the editor are lost from
the level (they stay in the backup).
Anchors for shotpilot are actors labelled SP_<name> (the same rule as `shotpilot scan`).

The level code lives next to this file in graybox/: core (the spawner and paint), fx (Niagara),
anim (character animation), and one module per game.
"""

import datetime
import importlib
import os
import shutil
import sys

import unreal

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import graybox  # noqa: E402
from graybox import core, fx, anim, audit  # noqa: E402
from graybox import lanternless, kettlehill, ashenpillars, gridhold  # noqa: E402

# the editor keeps one Python interpreter for its whole session: reload so an edited module is used
for _m in (core, fx, anim, audit, lanternless, kettlehill, ashenpillars, gridhold):
    importlib.reload(_m)

log, warn, load, EAL, Level, Missing = core.log, core.warn, core.load, core.EAL, core.Level, core.Missing

# game name -> (level path, builder)
GAMES = {
    "Lanternless": (lanternless.LANTERNLESS_MAP, lanternless.build_lanternless),
    "KettleHill": (kettlehill.KETTLEHILL_MAP, kettlehill.build_kettlehill),
    "AshenPillars": (ashenpillars.ASHEN_MAP, ashenpillars.build_ashenpillars),
    "Gridhold": (gridhold.GRIDHOLD_MAP, gridhold.build_gridhold),
}


def new_level(path):
    les = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    try:
        ok = les.new_level(path, False)  # a plain level: World Partition hides unloaded actors from scan
    except TypeError:
        ok = les.new_level(path)
    if not ok:
        raise RuntimeError("could not create " + path)
    # a commandlet does not switch the editor world to the new level: open it explicitly
    les.load_level(path)
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    if world is None or world.get_path_name().split(".")[0] != path:
        raise RuntimeError("the editor world is %s, not %s" % (world.get_path_name() if world else None, path))
    return les


def point_project_at(map_path):
    """Make the level the project's startup and default map (DefaultEngine.ini)."""
    obj = map_path + "." + map_path.rsplit("/", 1)[1]
    try:
        cdo = unreal.get_default_object(unreal.GameMapsSettings)
        cdo.set_editor_property("editor_startup_map", unreal.SoftObjectPath(obj))
        cdo.set_editor_property("game_default_map", unreal.SoftObjectPath(obj))
    except Exception as e:  # the ini below is what the next launch reads
        warn("map settings not updated in memory (%s)" % e)
    ini = os.path.join(unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_config_dir()),
                       "DefaultEngine.ini")
    section = "[/Script/EngineSettings.GameMapsSettings]"
    keys = {"EditorStartupMap": obj, "GameDefaultMap": obj}
    with open(ini, encoding="utf-8-sig", newline="") as f:
        text = f.read()
    nl = "\r\n" if "\r\n" in text else "\n"
    lines = text.split(nl)
    if section not in [l.strip() for l in lines]:
        lines += ["", section]
    start = [l.strip() for l in lines].index(section)
    end = start + 1
    while end < len(lines) and not lines[end].strip().startswith("["):
        end += 1
    seen = set()
    for i in range(start + 1, end):
        k = lines[i].split("=", 1)[0].strip()
        if k in keys:
            lines[i] = "%s=%s" % (k, keys[k])
            seen.add(k)
    lines[start + 1:start + 1] = ["%s=%s" % (k, v) for k, v in keys.items() if k not in seen]
    with open(ini, "w", encoding="utf-8", newline="") as f:
        f.write(nl.join(lines))
    log("startup and default map -> " + obj)


def clear_game_content(name):
    """Back up /Game/<name>/ on disk, then delete it in the editor (the --rebuild path)."""
    folder = "/Game/" + name
    content = unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_content_dir())
    src = os.path.join(content, name)
    if os.path.isdir(src):
        stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
        saved = unreal.Paths.convert_relative_path_to_full(unreal.Paths.project_saved_dir())
        dst = os.path.join(saved, "GrayboxBackups", stamp, name)
        shutil.copytree(src, dst)
        log("backed up %s to %s" % (folder, dst))
    if not EAL.does_directory_exist(folder):
        return
    # the level being replaced must not be the open world: step onto a blank one first
    try:
        unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).new_level("/Temp/Untitled_Graybox", False)
    except Exception:
        pass
    if not EAL.delete_directory(folder):
        raise RuntimeError("could not delete %s (is it open in an editor?)" % folder)
    log("deleted %s" % folder)


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    rebuild = "--rebuild" in argv
    audit_only = "--audit" in argv
    audit_out = next((a.split("=", 1)[1] for a in argv if a.startswith("--audit-out=")), None)
    name = args[0] if args else unreal.SystemLibrary.get_game_name()
    if name not in GAMES:
        unreal.log_error("[graybox] no level for '%s'. Known games: %s. Pass one as an argument "
                         "(py build_graybox.py <Game>) or name the project after it."
                         % (name, ", ".join(GAMES)))
        return
    map_path, build = GAMES[name]
    if audit_only:  # open the existing level and report; nothing is rebuilt or saved
        if not EAL.does_asset_exist(map_path):
            unreal.log_error("[graybox] %s does not exist: nothing to audit" % map_path)
            return
        unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level(map_path)
        audit.run(audit_out)
        return
    if EAL.does_asset_exist(map_path) and not rebuild:
        log("%s already exists: not touching it (pass --rebuild to build it again)" % map_path)
        return
    try:
        load(core.LP + "/Meshes/SM_Cube")
        load(core.MANNY)
    except Missing as e:
        unreal.log_error("[graybox] %s is missing. Make the project from the Third Person (Blueprint) "
                         "template, which brings the prototype meshes and the mannequins." % e)
        return
    if rebuild:
        clear_game_content(name)
    les = new_level(map_path)
    build(Level(name))
    les.save_current_level()
    point_project_at(map_path)
    audit.run(audit_out)  # findings go to the log; the build never fails on them
    log("built %s. Press Play to walk it; anchors are the SP_* actors." % map_path)


main(sys.argv)
