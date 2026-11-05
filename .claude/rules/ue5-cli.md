---
paths:
  - "crates/shotpilot/ue/**"
  - "crates/shotpilot/src/engines.rs"
---

# UE5 連携の規約

## 起動経路（実測済み・D-09）

- Editor の GUI は開かない。`UnrealEditor-Cmd` から CLI で動かす。owner が sharehouse-horror で本番運用している経路なので、リスクとして扱わない。
- `build` / `pull` / `scan` / `assets` / `survey` は `-run=pythonscript -script=<sp_entry.py>` で動かす（commandlet で軽い）。
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
- 七号室の render final（7 ショット、1920x1080、930 フレーム）は 1 回の起動で約 5 分。1 回目は 4 本目の途中で PIE が終わり（ゲーム側のログなし。PIE ウィンドウへの Esc か、ウィンドウを閉じた可能性が高い）、残り 4 本が失敗した。撮り終えた 3 本は take として残り、再実行では失敗した 4 本だけを撮った。MRQ のジョブが途中で止まったときは、その理由を result の `error` に書く。
- 七号室の `-R7StageOnly` では、正面玄関は施錠されたまま、七号室の扉は物語で出現するまで壁になっている。撮影の構図はこの状態を前提に決める。
- 出現（`appear`）も七号室で通った（2026-11-03、テスト用の `teaser-appear-test/`、`content_root: /Game/ShotpilotTest` で本番と分けた）。static mesh（`SM_CY_Observer`）と skeletal mesh（`SK_CY_Businessman`、基準ポーズのまま）を spawnable で置き、Spawn トラックのキーで指定フレームから出る。build 約 51 秒、render still 1 ショット約 69 秒。still の 0 フレーム目には出現前の prop が映らず、59 フレーム目には映った。警告は既知の `add_spawnable_from_class` 非推奨だけ。

## 写っていたものの報告（deps、D-37）

- render の result で、各 `renders[key]` に `deps: [{package, file}]` を返す（file は絶対パス）。失敗は `deps_error` にトレースバックを入れる。撮影そのものは止めない。mock エンジンも同じ形で返す（`content/<アンカー名 | prop 名>.*`）。
- 集め方: MRQ の `on_individual_job_started_delegate` でいま撮っているショットを知り、`register_slate_post_tick_callback` で 10 tick ごとに PIE ワールド（`UnrealEditorSubsystem.get_game_world()`）のアクターを見る。`was_recently_rendered(0.5)` のアクターから、Blueprint クラス、メッシュ、マテリアル（動的インスタンスは親）、Niagara などのパッケージを取り、アセットレジストリの hard 依存を 3 段まで足す（上限 800）。`/Engine/`、`/Script/`、PIE の複製は除く。
- パッケージからファイルへは、`/Game/` をプロジェクトの Content、各プラグインはその `.uplugin` の名前のマウントとして対応づけ、`.uasset` か `.umap` があるものだけを返す。
- 見ないもの: エディタのレベルでの配置（World Partition の `__ExternalActors__` は PIE から戻せない）、ゲームのコード。シャドウだけに写ったアクターも含まれる（撮り直しすぎる側に倒している）。
- **実機では未確認（2026-11-05）。** API 名は 5.7 の Python リファレンスで確かめ、ロジックは偽の `unreal` モジュールで通した。最初に実機で撮るときは、result の `deps` と `deps_error` を確かめる。
- 品質プリセットの `render_warmup_frames`（描画するウォームアップの枚数）と `warmup_camera_cut`（ショットのカメラでウォームアップする）は、書いたときだけ MRQ に渡す。既定は変えていない（画が変わらないので RENDERER_VERSION は上げていない）。Lumen の影が出そろわないときに試す。

## ロケハンの下ごしらえ（survey、D-42）

- `survey` は1回の commandlet で、アンカー（`scan` と同じくタグ `shotpilot` か `SP_` 接頭辞のアクター）と、頼まれたときはアセットの棚卸しを返す。タグ付きのアクターが無ければ、目印の候補 `candidates`（最大 60）を返す。選ぶのは UE ではなく Rust 側（D-43）。
- 候補の条件: メッシュを持ち、ライト・カメラ・ボリューム・空などではないアクターのうち、最大辺 40〜2500 cm・高さ 15 cm 以上のもの。ラベルから名前を作り（`SM_Old_Well3` → `old_well`）、同じ名前は大きいもの1つに絞る。各候補は `{name, label, cls, score, at, size, center}`。
- 選び方（`Workspace::scout`）: Claude に繋がっていれば、企画（`brief.plan` の Scout targets）に合う場所を最大 6 か所 Claude が選ぶ（`ai::pick_locations`、候補にない名前は捨てる）。繋がっていないか失敗したら、大きさで重みを付けて 3 m 以上離れたものを遠い順に選ぶ（`scout::spread`）。
- エディタのレベルを読むだけなので、実行時にレベルを組むゲーム（七号室の `/Engine/Maps/Entry`）では何も見つからない。そのときは「タグを付けるか anchors を書く」と返す。World Partition のマップは、読み込まれていないセルのアクターを拾えない。
- mock は `level.json`（タグ付き）と `landmarks.json`（自動の候補）で同じ形を返す。
- **実機では未確認（2026-11-05）。** 最初に実機で走らせるときは、ログの `scan: N landmark candidates (K kinds), returning M` と、選ばれた場所が妥当かを確かめる。

## 検証

- `sp_entry.py` はユニットテストできない。プロトコル側の挙動は、mock エンジンのテストで担保する。
- 実機の UE 起動は重いので、実行する前に人間に確認する。
- ショット単位の再レンダ（MRQ のジョブをショットごとに分ける構成）を崩さない。マスターシーケンス1本をまとめてレンダする形に戻さない。
