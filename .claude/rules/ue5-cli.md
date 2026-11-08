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
- `sp_entry.py` が要るプラグイン（`engines::NEEDED_PLUGINS`: PythonScriptPlugin、EditorScriptingUtilities、SequencerScripting、LevelSequenceEditor、MovieRenderPipeline）のうち、.uproject が有効にしていないものは、起動引数 `-EnablePlugins=` でその起動だけ有効にする。ゲームの .uproject は書き換えない（2026-11-07、七号室の本体で MRQ が無く `MoviePipelineQueueSubsystem` が無いと落ちた）。UE 5.8 の `PluginManager.cpp` は、.uproject に書いてあるプラグイン（`"Enabled": false` も含む）は引数で上書きしない。そのときは render の前に `sp_entry.py` が「Movie Render Queue を有効にする」手順だけを返す（トレースバックは出さない）。
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
- レベルの見取り図（D-86、`outline` ジョブ。survey にも付く）: 見えるメッシュを、メッシュ自身の境界とコンポーネントの世界の変換から向き付きの箱にし（回した壁は薄い壁のまま。ピッチかロールが 10° を超える部品は世界の軸の箱にする）、何か（建物・扉・物・灯りの器具・キャラクター）を付けて `scout/level.json` に書く。灯りとプレイヤーの出現位置は点、階は床の板の高さ。空・霧・ボリューム・ランドスケープなどは描かない。コマンドレット（`-NullRHI`）で動き、七号室の `Lvl_Horror` で 101 個の形・22 点・2 階（0 と 500 cm）を約 2 分で読んだ（ほとんどは起動の時間）。配置図は選んだカットのカメラの階を出し、1F・2F で切り替えられる。
- previz のための見取り図の追加（D-93、2026-11-08）: 同じ `outline` が、形ごとのメッシュの三角形（`meshes` にメッシュごとに1回。LOD 0 から順に 3000 三角形以下の最初の LOD、全体で 30 万三角形まで。`ProceduralMeshLibrary.get_section_from_static_mesh` で読む）と、その形の置き方（`xf`: コンポーネントの世界の位置・回転・拡大）、灯りの点の詳細（`light`: 種類・向き・色（色温度を掛ける）・カンデラ（ルーメンと単位なしは換算、単位なしは 625 で 1 cd）・届く距離・円錐・影）、レベルの `env`（指数の高さの霧、スカイライト、太陽、範囲の無いポストプロセスボリュームの露出）を返す。読めないメッシュはログに出して箱のまま、灯りや env の読み損ねも警告だけで止めない。太陽は配置図の灯りの点に入れず `env.sun` に置く。ログの `level plan: N shapes (M meshes, K triangles), P points (L lamps) ... env [...]` で、メッシュと灯りが読めたかが分かる。実機で通った（2026-11-08、七号室の `Lvl_Horror`、コマンドレットで約1分）: 101 の形のうち 97 がメッシュ付き（7種、1300 三角形）、灯り18（すべてスポット。単位なしで 0.3〜1.4 cd）、`env` は霧・スカイライト（強さ 3）・太陽（10 lux）・露出（自動、補正 -2）。警告なし。灯りの真下には器具の円盤（`/Engine/BasicShapes/Cylinder` を潰したもの）が2枚あり、影の地図に入れると光だまりが全部消えるので、previz は灯りから 40 cm 以内の面を影に入れない。
- **World Partition のマップ（アクターが `Content/__ExternalActors__/<マップ>` にある）は、最初のコマに何も写らない**（2026-11-08、七号室の `Lvl_Horror`）。MRQ のウォームアップはプレイヤーの視点のまま進み、レベルはショットのカメラの周りに来てから読み込まれるので、0 コマ目は空と霧だけの一様な画になった（still の 14・29 コマ目は、カメラカットのそれより前の部分がウォームアップになって写っていた）。ロケハン写真は 0 コマ目だけなので、全部が真っ白に見えた。そこで `engines::streaming_lead` がそのマップを見分けたときだけ、カメラとカメラカットを 0 コマより 24 コマ前から置き（最初の姿勢のまま）、品質に `warmup_camera_cut: true` を入れて、そのカメラでウォームアップする。build と render の指紋はそのマップのプロジェクトでだけ変わり、ほかのレベル（4本のサンプルを含む）の take は動かない（`a_streamed_level_warms_up_on_the_shot_camera_and_other_levels_keep_their_fingerprints`）。エンジンのウォームアップのコマ数を増やすだけ（48）では直らなかった。暗いレベルを疑ってロケハンのカメラに作業灯を付けても変わらなかった（暗さが原因ではなかった）。
- 七号室の `-R7StageOnly` では、正面玄関は施錠されたまま、七号室の扉は物語で出現するまで壁になっている。撮影の構図はこの状態を前提に決める。
- 出現（`appear`）も七号室で通った（2026-11-03、テスト用の `teaser-appear-test/`、`content_root: /Game/ShotpilotTest` で本番と分けた）。static mesh（`SM_CY_Observer`）と skeletal mesh（`SK_CY_Businessman`、基準ポーズのまま）を spawnable で置き、Spawn トラックのキーで指定フレームから出る。build 約 51 秒、render still 1 ショット約 69 秒。still の 0 フレーム目には出現前の prop が映らず、59 フレーム目には映った。警告は既知の `add_spawnable_from_class` 非推奨だけ。

## 写っていたものの報告（deps、D-37）

- render の result で、各 `renders[key]` に `deps: [{package, file}]` を返す（file は絶対パス）。失敗は `deps_error` にトレースバックを入れる。撮影そのものは止めない。mock エンジンも同じ形で返す（`content/<アンカー名 | prop 名>.*`）。
- 集め方: MRQ の `on_individual_job_started_delegate` でいま撮っているショットを知り、`register_slate_post_tick_callback` で 5 tick ごとに PIE ワールド（`UnrealEditorSubsystem.get_game_world()`）のアクターを見る。画面に写っているアクターから、Blueprint クラス、メッシュ、マテリアル（動的インスタンスは親）、Niagara などのパッケージを取り、アセットレジストリの hard 依存を 3 段まで足す（上限 800）。`/Engine/`、`/Script/`、PIE の複製は除く。
- 「写っている」は安い順に3つで決める。①`was_recently_rendered(0.5)`、②バウンズがカメラの視野（PlayerCameraManager の位置・向き・FOV と出力の縦横比、10% 広げる）に入る、③カメラからバウンズの中の点（3×3×3）へ visibility チャンネルの線を引き、ほかのものに当たらずに届く点がある。標本はシーケンスのカメラ（`CameraActor`）がビューターゲットのときだけ取る。カメラ越しの標本が1つも無かったショットは、①だけで描画されたもの全部を返し、`deps_error` にそう書く。
- ①だけでは足りない。`was_recently_rendered` は影の描画でも真になるので、ディレクショナルライトの影がレベル全体を覆うと、全アクターが全ショットに入る（Lanternless では10ショットとも同じ 32 パッケージになった）。
- パッケージからファイルへは、`/Game/` をプロジェクトの Content、各プラグインはその `.uplugin` の名前のマウントとして対応づけ、`.uasset` か `.umap` があるものだけを返す。
- 見ないもの: エディタのレベルでの配置（World Partition の `__ExternalActors__` は PIE から戻せない）、ゲームのコード、画面の外から影だけを落とすアクター。判定できないとき（線引きの例外など）は写っている側に数える。
- 実機で通った（2026-11-05、Lanternless、still 10 ショット）。ショットごとに 7〜26 パッケージになり、人影と受付の女性（Quinn）が写る5ショットだけが `MI_Quinn_01` を持ち、倒れた患者（Manny）は写る2ショットだけが持った。`MI_Quinn_01` の色を変えると、その5ショットだけが GAME になり、`render still` はその5本だけを `<fp12>~2/` に撮った（約 47 秒）。マテリアルを元のファイルに戻すと、10 ショットとも前の take に戻った（UE は起動しない）。視野の余白の分だけ多めに入る（空の椅子のショットに、画面上端の外にいた廊下の奥の人影が入った）。ログに `deps <key>: N packages on screen over M camera samples (K drawn)` が出る（still で1ショット6〜7標本）。
- 品質プリセットの `render_warmup_frames`（描画するウォームアップの枚数）と `warmup_camera_cut`（ショットのカメラでウォームアップする）は、書いたときだけ MRQ に渡す。既定は変えていない（画が変わらないので RENDERER_VERSION は上げていない）。Lumen の影が出そろわないときに試す。

## ロケハンの下ごしらえ（survey、D-44）

- `survey` は1回の commandlet で、アンカー（`scan` と同じくタグ `shotpilot` か `SP_` 接頭辞のアクター）と、頼まれたときはアセットの棚卸しを返す。タグ付きのアクターが無ければ、目印の候補 `candidates`（最大 60）を返す。選ぶのは UE ではなく Rust 側（D-45）。
- `candidates: true` を付けると、タグ付きのアクターがあっても候補を返す（タグ付きのものは除く）。構成画に合う場所が見つからないときに、別の候補を探すために使う（D-46）。
- 候補は「場所」（2026-11-08 に変更）。場所を作るもの（扉・キャラクター・エフェクト・灯り・物・プレイヤーの出現位置）と、床の板から読んだ部屋と廊下を集め、3.5 m 以内のものを1つの場所にまとめて、いちばん語るもの（扉・キャラクター > エフェクト・物 > 灯り > 廊下 > 部屋・小物 > 出現位置）の名前を付ける（灯りは `light`、2つ目からは `_2`）。壁・床・天井・柱や、基本形状（Cube・Cylinder・Ramp など）だけの StaticMeshActor は建物の部品として場所にしない（床探しにだけ使う）。きっかけは七号室の `Lvl_Horror`: 試作用の箱だけで組まれたレベルで、旧方式は 44 個の候補が形の5種類に潰れ、天井の板・柱・スロープ・壁の角・ドア枠を「場所」として選び、壁よけがその部品自身を素通りして写真が壁のアップになった。灯りは器具（メッシュや火花のエフェクト付きの Blueprint でも）ではなく、その下の床を目の高さ（床 + 120 cm）から見る。床の高さは当たり判定なしで、下にある板の上面から取る（コマンドレットには当たり判定が無い）。床の板（厚さが短辺の 20% 以下で 150 cm 以下）は、上に別の板（上の階か屋根）があれば床、無くて下に板があれば屋根。各候補は `{name, label, cls, kind, kinds, score, at, size, center}`（`label` はまとめたものの名前を ` + ` でつなぐ）。場所を作るものが何も無いレベルでは、旧方式（基本形状そのもの）に戻る。
- 選び方（`Workspace::scout`）: Claude に繋がっていれば、企画（`brief.plan` の Scout targets）に合う場所を最大 6 か所 Claude が選ぶ（`ai::pick_locations`、候補にない名前は捨てる）。繋がっていないか失敗したら、大きさで重みを付けて 3 m 以上離れたものを遠い順に選ぶ（`scout::spread`）。
- エディタのレベルを読むだけなので、実行時にレベルを組むゲーム（七号室の `/Engine/Maps/Entry`）では何も見つからない。そのときは「タグを付けるか anchors を書く」と返す。World Partition のマップは、読み込まれていないセルのアクターを拾えない。
- mock は `level.json`（タグ付き）と `landmarks.json`（自動の候補）で同じ形を返す。
- **実機では未確認（2026-11-05）。** 最初に実機で走らせるときは、ログの `scan: N landmark candidates (K kinds), returning M` と、選ばれた場所が妥当かを確かめる。

## サンプルのゲーム（D-54）

- `Lanternless` / `KettleHill` / `AshenPillars` / `Gridhold` は `C:\Users\Keisu\Projects\ShotpilotSamples\` にある普通の UE プロジェクト（Third Person の Blueprint テンプレート）。七号室は使わない。
- マップは、そのプロジェクトのエディタで `tools/ue_sample/build_graybox.py` を走らせて組む（ファイル → Python スクリプトを実行。プロジェクト名でゲームを選ぶ）。この道具は製品の外にあり、`sp_entry.py` の規約（埋め込み、job JSON）は当てはまらない。既存のマップは上書きしない。作り直すときは `--rebuild`（`/Game/<ゲーム>/` を `Saved/GrayboxBackups/<時刻>/` に写してから消して組む。手で直した分はバックアップにだけ残る）。`--audit` は組まずに点検だけを行う（保存しない）。
- 組んだマップは World Partition を使わない普通のレベルで、自動露出は固定（PostProcessVolume で EV100 を1つに留める）。マネキンは SkeletalMeshActor で、その場の動きは単体アニメのループ、歩くものは自動再生・無限ループの LevelSequenceActor（`/Game/<ゲーム>/Cinematics/`）が possessable の transform とアニメのトラックで動かす。遺体は、自前の1姿勢のアニメ（`/Game/<ゲーム>/Anims/A_*`）で止めて寝かせたもの。テンプレートの Death アニメ6本は、最後のコマでも腰が 83〜89 cm の高さにあり、倒れきらない（ラグドールに渡す作り）。姿勢は MM_Idle の複製に、AnimPose で骨を関節ごとに回した結果を AnimDataController の `set_bone_track_keys` で全コマに書いて作る。寝かせる向きは roll -90 で仰向け、+90 でうつ伏せ。
- 5.8 では、スケルタルアニメのセクションの `play_rate` が数値ではなく `MovieSceneTimeWarpVariant` で、`set_fixed_play_rate` で書く。LevelSequenceActor の自動再生は `playback_settings.auto_play`。`AnimPoseExtensions.set_bone_pose` は引数4つまで（子の骨はローカルを保ってついてくる）。`HitResult` の中身は `to_dict()` でしか読めず、`GameplayStatics.break_hit_result` は無い。
- `Lanternless` は、エディタの新規作成と同じ手順をシェルでなぞって作った（2026-11-05。UE には新規作成の CLI が無い）。`TP_ThirdPersonBP` の Config と Content を写し（`TemplateDefs.ini` の FilesToIgnore に従う）、共有パック（`TemplateResources/High/{LevelPrototyping,Characters,Input}/Content`）を `Content/<パック名>/` に入れ、ini の `TP_ThirdPersonBP` を置き換え、新しい ProjectID と `EngineAssociation: 5.8` とプラグイン（PythonScriptPlugin、EditorScriptingUtilities、SequencerScripting、MovieRenderPipeline、LevelSequenceEditor）を書いた。
- 実機で通った（2026-11-05）: `build_graybox.py` はコマンドレット（`-run=pythonscript -NullRHI`）で約 1 秒、`[graybox]` の警告なし。`shotpilot scan` で `SP_` の 5 アンカーが取れた。
- コマンドレットでは `new_level` のあとエディタのワールドが新しいレベルに切り替わらない（`load_level` で開き直す）。`spawn_actor_from_object` はアクターを置けない（`spawn_actor_from_class` で置いてからメッシュを差す）。LevelPrototyping の `SM_Cube` はピボットが角にある（bounds から位置を出しているので影響なし）。
- 実機で通った（2026-11-05）: MRQ の PIE で、単体アニメのループも、自動再生のシーケンスで歩くマネキンも動く。露出（EV100 = 1.0）は灯りの当たる所で見やすい（灯りの無い部屋は月光 2 lux と弱い灯りを足した）。
- **`still` ではレベルの中の時間がほぼ進まない。** still は鍵のコマだけを飛ばして撮るので、ショットの時刻が 0→3→6 秒と進んでも、PIE のワールドは撮ったコマの数（十数 tick）しか進まない。レベルの中で動くもの（自動再生のシーケンス、アニメ、パーティクル）は still では止まって見え、`preview` と `final`（全コマ）で初めてショットの時刻と揃う。
- ロケハンの撮影位置は、`scout::cameras` がアンカーから4方向に一定の距離を引いて決めるので、部屋や廊下が狭いと壁の向こうに出る（Lanternless では 20 枚の約半分が真っ黒だった）。そこで、ロケハンの build 項目にだけ `clear_view: {target}` を付け、`sp_entry.py` がアンカーの中心から撮影位置へ線を引いて（visibility チャンネル）、壁に当たればその 30 cm 手前へ寄せ、寄せた比でレンズを広角にする（12 mm まで）。アンカー自身（線の始まりで重なっているもの）と、60 cm 以内の小物（最大辺 2 m 未満）は無視して引き直す。寄せた位置は build の結果の `clear_view` で返り、`scout/index.json` の `location` と `focal_length` に残る。失敗しても build は止めず、`clear_view_error` を返して元の位置で撮る。普通のショットには付けない（定義どおりに撮る）。
- 当たり判定はフルエディタでしか効かない。コマンドレット（`-run=pythonscript -NullRHI`）では、箱メッシュの当たり判定が無く、線は地面にも壁にも当たらない（マネキンの物理アセットにだけ当たる）。ロケハンは render と同じフルエディタの起動で build するので効く。Lanternless では 20 台中 14 台が寄せられ、写真が写るようになった。
- 光と塗り（D-58）。マテリアルは `make_material` が Custom ノードに HLSL を数行書いて作る（入力はワールド座標 `WP` と時刻 `T`、テクスチャなし）。コマンドレットで作って保存でき、コンパイルは撮影のフルエディタで済む。明滅は、ライトファンクションのマテリアル（`MD_LIGHT_FUNCTION`）と器具の発光に同じ式（`FLICKER`）を使って揃える。式はゲームの時刻で動くので、各ショット（PIE は 0 秒から）の同じ時刻に同じ明滅が起き、still では時刻が進まないので明滅しない。ボリューメトリックフォグの光の筋は、フォグの密度 0.05・前方散乱 0.7・ライトの `volumetric_scattering_intensity`（窓のスポットは 8）で見えるようになった（0.02 では見えなかった）。
- Niagara（D-58）。UE 5.8 同梱の実験的プラグイン `ToolsetRegistry` と `NiagaraToolsets` を uproject で有効にすると、`unreal.ToolsetRegistry.execute_tool("NiagaraToolsets.NiagaraToolset_System", <道具>, <JSON>)` で、AI 向けの公式の道具（46 個。`CreateNiagaraSystem`、`AddEmitter`、`SetStackInputData`、`SetRendererData`、`SetEmitterData`、`GetEmitterInputValues` など）を Python から呼べる。コマンドレット（`-NullRHI`）でも動き、システムのコンパイルまで済む（`GetSystemCompileState` が `UpToDate`）。道具の一覧と引数は `get_all_toolset_json_schemas()` で取れる。引数名はヘッダの引数名の先頭を小文字にしたもの（`SetRendererData` は `renderer`、`SetEmitterData` は `emitter`、`SetStackInputData` は `stackInputRef`）。アセットは `{"refPath": "/Game/X.X"}`、スタックの場所は `{system, emitterName, scriptName, moduleName, rendererIndex, inputNameStack}`。スクリプト名は `EmitterUpdateScript`、`ParticleSpawnScript`、`ParticleUpdateScript`。空のシステムは `NiagaraSystemFactoryNew` で作れ、`HangingParticulates`（`/Niagara/DefaultAssets/Templates/Emitters/`）を足せばほこりになる。先回しは `warmup_time` をシステムに直接書く。
- ほこりの粒は、ライティングボリュームで照らす半透明（`TLM_VOLUMETRIC_NON_DIRECTIONAL`）では暗すぎて見えなかった。画素ごとに照らす `TLM_SURFACE_PER_PIXEL_LIGHTING` にすると、灯りや月光の中でだけ見える。マテリアルには `used_with_niagara_sprites` を立てる。
- レベルの光や配置を変えても D-37 の deps は変わらない（アセットの中身ではないため）。作り直したら `shotpilot render <品質> --force` で撮り直す。
- 残り3本（2026-11-06、D-61）。プロジェクトは、テンプレート `Templates/TP_ThirdPersonBP` の Config と Content を写し、`TemplateDefs.ini` の無視するファイル（TemplateDefs.ini、config.ini）を消し、共有パック3つを `Content/<パック名>/` に入れ、ini の `TP_ThirdPersonBP` を名前に置き換え、`DefaultGame.ini` の ProjectID を新しくし、uproject を Lanternless と同じプラグインで書いて作った。
- 組み立ての道具: エンジンの基本形状（`/Engine/BasicShapes/` の Cube・Sphere・Cone・Cylinder、100 cm で中心が原点）を向き付きで置く `shape`、部品をまとめて回す `group`（メッシュの無い StaticMeshActor を親にして `attach_to_actor`）とシーケンスの `spin`、閉じた経路を位相をずらして歩かせる `loop_path`、手に持たせる `hold`（`hand_r` ソケットに `attach_to_component` で付ける）、光る粒の素材 `make_glow_material`（加算・アンリット）。
- 分かったこと: 板を傾けて切妻屋根にするとき、y の小さい側の板は roll を負にする（逆にすると谷形になる）。右手の `hand_r` は X 軸が腕の付け根の方を向くので、剣は X に沿わせて −X 側へずらすと脚に沿って下を向く。屋外の露出は、太陽 8〜10 lux で EV100 0.9〜1.8 が目安（3.0 では真昼でも夕方のように暗い）。アンカーにした物が他の物の塊の中にある（階段の途中の段、積んだ木箱の1つ）と、ロケハンの写真がその塊の中から撮られて黒くなる。厚さの無いアンカー（地面の板）だと、写真が地面すれすれになる。`scan` は消えたアンカーを shots.yaml から消さない。
- エディタが同じプロジェクトを開いている間に shotpilot が UE を起動しても、build（シーケンスの削除と作り直し）と render still は通った（2026-11-05、Lanternless の s07、約 55 秒）。開いているエディタのログに警告は出なかった。**まだ確かめていないこと:** そのショットのシーケンスをエディタの Sequencer で開いているとき、とくに未保存の編集があるとき（shotpilot の build はディスクの内容しか読まないので、あとでエディタで保存すると build の結果を上書きする恐れがある）。
- 作り込み（2026-11-07、D-69）。組み立ては `tools/ue_sample/graybox/` のパッケージに分けた: `core`（Level の置き方・塗り・Ambient）、`shapes`（GeometryScripting の形）、`fx`（Niagara の効果）、`anim`（動きの作り方）と `animcheck`（GPU なしで描く棒人間の確認図）、`audit`（構造の点検）、ジャンルごとの動き `moves_horror` / `moves_cozy` / `moves_action` / `moves_strategy`、ゲームごとの組み立て4つ。エディタは Python のインタプリタを1つ使い続けるので、入口が毎回全モジュールを reload する。
- 点検（`audit.py`）は組むたびに最後に走る（ビルドは止めない）。z-fighting（同じ向きで同一平面に重なる面、1 cm² 以上。別の固体に埋もれた重なりは数えない）、浮き（何にも触れない部品、地面とつながらない塊、下に隙間のある屋根、端が何にも載らない梁）、体（足の下の面、固体の中の体、手から離れた持ち物、シーケンスで歩く経路）、重複を出す。当たり判定を使わず境界の計算だけなので、コマンドレットで動く。箱・円柱・円錐しか形を知らず、shapes の形は境界箱で見るので、アーチの開口を通る人や丸い塔に付けた扉が「中にある」と出る（誤検出）。開いた隙間（妻の三角）や角の欠けは見えないので、目で見る。
- 形（`shapes.py`）は GeometryScripting で閉じた立体を作る（妻の閉じた家、厚みと軒のある屋根、円錐の屋根、柱頭と溝彫りの円柱、樽・井戸・急須・壺の回転体、アーチ、階段、斜路、岩、樹冠、幹）。GeometryScripting は ModelingToolsEditorMode と一緒に入るので、プラグインを足さなくてよい。アセットは `SM_<種類>_<引数のハッシュ>` で、同じ引数なら使い回す（作り方を変えたら `SHAPES_VERSION` を上げる）。当たり判定は complex-as-simple で、フルエディタの visibility の線が壁・妻・屋根・階段の各段に当たり、アーチの開口と井戸の穴は抜けることを確かめた（ロケハンの寄せ、D-55 が効く）。`lathe(segments=4, rotate=45)` で四角い塔と四角錐の屋根が1つの閉じた立体になる。
- 壁の角（`Level.wall` の `ends=(始め, 終わり)`、長さに足す cm）。外角を両方の壁で厚みの半分ずつ伸ばすと、外の面が同一平面で重なって z-fighting になる。角を持つ壁を厚みの半分伸ばし、もう片方をその内面で止める（半分縮める）。T 字で厚い壁から始まる壁も、厚みの半分先から始める。
- 動き（`anim.make_move`）。公式の Control Rig の道（AnimationAssistantToolset、`ControlRigSequencerLibrary.find_or_create_control_rig_track`）は、コマンドレットで Slate のアサート（`CurrentApplication.IsValid()`）に当たってプロセスごと落ちる。そこで、数個のキーの姿勢を FK と2関節の IK で解き（足は床に残り、腰を落とすと膝が曲がる）、`AnimDataController.set_bone_track_keys` で MM_Idle の複製に書く。長さは `set_number_of_frames`、ループでは終わりの時刻にキーを置かない。テンプレートの歩行アニメはルートモーション（300 cm/s）を持つので、重ねる動きは root のトラックを定数で書き直して `enable_root_motion` を外したその場歩きにし、シーケンスでの移動を 300 cm/s × 再生速度にすると足が滑らない。手に持たせる物は、仕上がったアニメのその時刻の手の骨の姿勢（`AnimPoseExtensions.get_bone_pose`）から取り付けを計算する。`core.Ambient.walker` は `/Game` のアニメを読めず、高さを 0 に固定しているので、Lanternless・AshenPillars・Gridhold は自分のモジュールで歩かせ方を持っている。
- Niagara の効果集（`fx.py`）: 火（松明・篝火・焚き火、火の粉つき）、煙、湯気、火花（一定の間隔で弾ける。光の規則の内）、落ち葉・花びら、蛍、霧雨、しずく、低い霧。エンジンの Emitter テンプレート（Fountain、OmnidirectionalBurst、HangingParticulates）と HLSL で描くスプライトだけで作り、種を固定して先回しする（同じショットを2回撮るとシミュレーションは同じ）。火には点光源（篝火 60 cd、松明 25、焚き火 80、橙）を `brazier_light_function` で息づかせ、フォグの無いレベルでは光源の `scatter` を 0 にする（でないと光源そのものが丸いにじみになる）。昼の露出（EV100 1.8）では火の光を小さく（鍛冶 220〜350 cd。2500 で中庭が白く飛んだ）。霧雨は上に灯り（約 90 cd）が無いと見えない。NiagaraFluids の Gas テンプレートは組めて MRQ で落ちずに撮れるが、オフラインでは何も描かれず、採らなかった（ゲームの uproject で有効にしない）。
- **HLSL で `line` を変数名にしない（予約語）。** マテリアルのコンパイルが落ち、そのマテリアルを使うレベルの撮影が真っ黒になるか、レンダラが落ちる。NullRHI のビルドでは分からない。撮影の `.shotpilot/logs/*render.log` を `error:` で探す。`point`、`triangle`、`half`、`texture`、`sample`、`buffer`、`vector`、`matrix` も避ける。
- 新しいレベルの最初の撮影はシェーダーのコンパイルで 4〜6 分かかる。2回目からは効果を全部入れても1ショット 5〜12 秒（preview、960x540）。GPU は1枚なので、フルエディタの撮影は同時に1つずつにする。
- 確認用の shotpilot のプロジェクト（`render still` で構図を見るためのもの）は `content_root` を `/Game/_Probe/` の下にする。本番のティザーと同じ `/Game/Shotpilot` に書くと、ティザーのシーケンス（`SEQ_Teaser` と `Shots/`）が書き換わり、エディタで直されたように見える（そのときは `build --force` で shots.yaml から作り直す）。
- 既知の小さな問題: `--rebuild` は毎回 `NewLevel ... /Temp/Untitled_Graybox` のエラーを1行出し、コマンドレットの終了コードが 1 になる（レベルは組まれる）。コマンドレットは成功しても終了時に落ちることがあるので、成否は終了コードでなくログの `[graybox]` の行で見る。`--rebuild` のたびに `Saved/GrayboxBackups/` に写しが増える。

## 検証

- `sp_entry.py` はユニットテストできない。プロトコル側の挙動は、mock エンジンのテストで担保する。
- 実機の UE 起動は重いので、実行する前に人間に確認する。
- ショット単位の再レンダ（MRQ のジョブをショットごとに分ける構成）を崩さない。マスターシーケンス1本をまとめてレンダする形に戻さない。
