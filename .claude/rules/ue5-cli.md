---
paths:
  - "crates/shotpilot/ue/**"
  - "crates/shotpilot/src/engines.rs"
---

# UE5 連携の規約

## 起動経路（実測済み・D-09）

- Editor の GUI は開かない。`UnrealEditor-Cmd` から CLI で動かす。owner が sharehouse-horror で本番運用している経路なので、リスクとして扱わない。
- `build` / `pull` / `scan` は `-run=pythonscript -script=<sp_entry.py>` で動かす（commandlet で軽い）。
- `render` は `-ExecutePythonScript=<sp_entry.py>` で動かす。フルエディタを起動して MRQ を PIE executor で回し、終わったら終了する。
- 既定では、隔離した `-UserDir` で起動する（`project.isolate_userdir`）。owner の普段のエディタ設定を汚さない。
- Remote Control HTTP は使わない（Beta 扱いで、Sequencer 編集の汎用 API ではないため）。

## `ue/sp_entry.py`

- Unreal 同梱の Python で動く。標準ライブラリと `unreal` だけを使う。プロジェクトで唯一の Python ファイル。
- `engines.rs` が `include_str!` でバイナリに埋め込み、起動のたびに `.shotpilot/sp_entry.py` に書き出して渡す。直したら再ビルドする。
- 入力は環境変数 `SHOTPILOT_JOB` が指す job JSON。結果は `<job>.result.json` に書く。プロトコルを変えたら、`engines.rs` の `MockEngine` も同じように直す。mock と ue5 の挙動がずれると、テストが嘘をつく。
- Sequencer / MRQ の Python API はエンジンのバージョンで名前が違う。直接呼ばず、`_call_first(obj, [新名, 旧名])` のフォールバックを通す。
- MRQ の非同期コールバックの間に UObject が GC されないよう、`_KEEP` に参照を置いておく。
- 例外は握りつぶさない。result JSON にエラーとトレースバックを入れて返す。
- 出力のピクセルが変わる修正をしたら、`fingerprint::RENDERER_VERSION` を上げる。

## 実機で確かめたこと（UE 5.8、2026-11-03）

- build / pull / render still は実機で通った。build は 2 ショットで約 14 秒、render still は 2 ショット×3 フレームで約 26 秒。
- `-ExecutePythonScript` で起動すると、スクリプトが戻った時点でエディタが `QUIT_EDITOR` を出す（`EditorPythonExecuter.cpp`）。MRQ は非同期なので、`unreal.EditorPythonScripting.set_keep_python_script_alive(True)` でプロセスを保持し、終わったら False に戻す。
- spawnable のテンプレートでは `get_cine_camera_component()` が None を返す。`get_editor_property("camera_component")` なら取れる。
- Transform のチャネル名には `Location.X_0` のように `_0` が付く。名前は前方一致で照合する。
- `add_spawnable_from_class` と `add_marked_frame` は 5.8 で非推奨の警告が出るが、動作はする。
- Git Bash から `/Game/...` を引数で渡すと、Windows パスに書き換えられる（`MSYS_NO_PATHCONV=1` で防げる）。`model.rs` で検出して止めている。
- MRQ の PIE は、Game Override の設定が無いと `MoviePipelineGameMode`（空の GameMode）で回る。レベルを実行時に組むゲーム（七号室）はそのままだと真っ黒になる。`project.game_mode` を書くと `MoviePipelineGameOverrideSetting` で差し替える。5.8 では `game_mode_override` は非推奨で効かない（警告だけ出て空の GameMode のまま）。`soft_game_mode_override` に `SoftClassPath` を入れる。
- Rust 版でも実機で通った（2026-11-03、SPTest / Template_Default、2 ショット）。Python 版で push した状態と撮った take はそのまま有効だった（status が synced / fresh、build は何もしない）。1 ショットだけ直して render preview すると、そのショットだけ build・撮影された（約 25 秒）。pull は約 15 秒、render final（1920x1080、2 ショット × 30 フレーム）は約 37 秒、cut final は約 2 秒。
- 七号室（RoomSeven の作業コピー `Production/RoomSeven-shotpilot`、本体には書かない）も撮れた。`map: /Engine/Maps/Entry`、`game_mode: /Script/RoomSeven.R7CourtyardMode`、`ue_args: [-R7StageOnly]` で、PIE の中で C++ が中庭を建ててから MRQ が撮る。build 7 ショットは約 45 秒、render still 7 ショット×3 フレームは約 75 秒。コピー側で MovieRenderPipeline / SequencerScripting / LevelSequenceEditor を有効にしたが、C++ の再ビルドは要らなかった。

## 検証

- `sp_entry.py` はユニットテストできない。プロトコル側の挙動は、mock エンジンのテストで担保する。
- 実機の UE 起動は重いので、実行する前に人間に確認する。
- ショット単位の再レンダ（MRQ のジョブをショットごとに分ける構成）を崩さない。マスターシーケンス1本をまとめてレンダする形に戻さない。
