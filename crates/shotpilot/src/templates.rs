//! shots.yaml starter templates (written as text so the comments survive).

pub const DEMO: &str = r#"# Shotpilot shot definition. This file is the source of truth:
# everything in render/ and in the engine's Shotpilot folder is derived from it.
version: 1

project:
  name: {name}
  engine: {engine}            # mock (no UE needed) | ue5
  uproject: {uproject}
  map: {map}
  content_root: /Game/Shotpilot   # where shot sequences are written in the UE project
  sun_actor: DirectionalLight      # actor label driven by environment.time_of_day (optional)
  isolate_userdir: true            # run UE with its own -UserDir so your editor settings stay untouched

teaser:
  title: 夜の中庭
  pitch: シェアハウスの中庭で、住人が一人ずつ消えていく一人称ホラー
  target_duration: 30
  fps: 30
  resolution: [1920, 1080]

# Named points in the level. `shotpilot scan` fills these from actors tagged
# `shotpilot` (or labelled SP_*). Units: cm, Z up.
anchors:
  gate: [0, -1500, 100]
  well: [0, 0, 80]
  window: [800, 600, 450]
  door: [-700, 500, 120]
  lantern: [300, -300, 180]

defaults:
  lens: {focal_length: 35, aperture: 2.8}
  environment: {time_of_day: dusk}

# Order here is the order of the cut. Reordering never re-renders anything.
# beat: hook -> establish -> escalate -> title (the teaser shape; editorial only)
shots:
  - id: s01_gate
    beat: hook
    purpose: 門の外から中庭を覗く。静かな導入
    duration: 5
    camera:
      anchor: gate
      offset: [-300, -250, 70]
      look_at: well
      move: {type: dolly_in, distance: 220}
      lens: {focal_length: 24}

  - id: s02_well
    beat: establish
    purpose: 井戸のクローズアップ。何かがいる気配
    duration: 3.5
    camera:
      anchor: well
      offset: [-180, 60, 60]
      lens: {focal_length: 85, aperture: 1.8}

  - id: s03_window
    beat: escalate
    purpose: 2階の窓を見上げる。人影が一瞬だけ映る
    duration: 4
    camera:
      anchor: window
      offset: [-700, -500, -280]
      move: {type: crane_up, distance: 80}
      lens: {focal_length: 50}
    environment: {time_of_day: night}
    markers:
      - {at: 2.2, label: silhouette}

  - id: s04_lantern
    beat: escalate
    purpose: 提灯が揺れる。風はない
    duration: 4
    camera:
      anchor: lantern
      offset: [-250, -150, -20]
      move: {type: orbit_left, degrees: 25}

  - id: s05_door
    beat: escalate
    purpose: 扉がゆっくり開く
    duration: 5
    camera:
      anchor: door
      offset: [-350, -250, 50]
      move: {type: dolly_in, distance: 120}
    markers:
      - {at: 1.5, label: door opens}

  - id: s06_title
    beat: title
    purpose: 中庭の俯瞰。タイトル
    duration: 6
    camera:
      anchor: well
      offset: [-1400, -1400, 1100]
      move: {type: crane_up, distance: 300}
      lens: {focal_length: 18}
    environment: {time_of_day: night}
    notes: タイトルは編集で載せる。上1/3を空けておく
"#;

pub const EMPTY: &str = r#"version: 1

project:
  name: {name}
  engine: {engine}
  uproject: {uproject}
  map: {map}
  content_root: /Game/Shotpilot
  sun_actor: null
  isolate_userdir: true

teaser:
  title: ""
  pitch: ""
  target_duration: 45
  fps: 30
  resolution: [1920, 1080]

anchors: {}

defaults:
  lens: {focal_length: 35, aperture: 2.8}

shots: []
"#;

pub const GITIGNORE: &str = "render/\ncut/\n.shotpilot/jobs/\n.shotpilot/logs/\n.shotpilot/ue_userdir/\n.shotpilot/mock_engine/\n.shotpilot/sp_entry.py\n";

/// Fill a template. `uproject` and `map` are YAML scalars (use "null" for none).
pub fn render(tpl: &str, name: &str, engine: &str, uproject: &str, map: &str) -> String {
    tpl.replace("{name}", &crate::yamlio::emit_str(name))
        .replace("{engine}", engine)
        .replace("{uproject}", uproject)
        .replace("{map}", map)
}
