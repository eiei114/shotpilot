# ゲームエンジンのシネマティック／キャプチャ機能とプラグインは、トレーラー制作の問題をどう解いているか

調査日: 2026-11-05。検索ベースの調査で、公式ドキュメントの一部（Sequencer 概要、MRG 詳細、MRQ 公式）は本文を取得できなかった。取得できなかった点は各 Gaps に書いた。フォーラム投稿は逸話であり、公式の仕様ではない。

## Q1. UE5 の標準機能（Sequencer / MRQ / MRG / Take Recorder / Replay / Level Snapshots）は何を解き、どこで詰まるか

### Takeaway
Sequencer は「ショット単位の構造」（Shot Track、サブシーケンス、Take）を持つが、これは人間が手で切り替える版管理にとどまる。どのショットが変わったかを検出して、そのショットだけを再レンダする機能は、今回見つけた範囲の標準機能にはない。レンダ側は「有効／無効」を手動かスクリプトで切り替える作りで、Take Recorder と MRQ にはそれぞれ既知の取りこぼし・アーティファクト報告がある。

### Cited Findings
- Sequencer は Shot Track と Master Sequence でショット単位に整理でき、「Each Shot corresponds to its own Sequence asset.」。Take は「experiment with different shot contents without modifying the original shot」するためのもので、右クリックの New Take で作り、Takes リストで切り替える。 — [Epic 4.27 Shot Track](https://docs.unrealengine.com/4.27/AnimatingObjects/Sequencer/Overview/TracksShot)、[Epic: Sequences, Shots, and Takes](https://dev.epicgames.com/documentation/unreal-engine/sequences-shots-and-takes-in-unreal-engine)（検索結果の要約から。4.26/4.27 の古い記述を含む）
- Take は「just another version of a subsequence (or shot)」というコミュニティの説明。命名は Project Settings の Level Sequences で変える。初回の New Take が Take 2 と表示され、Take 1 が一覧に出ない癖がある（4.26 時点の記述）。 — [UE フォーラム: Sequence takes](https://forums.unrealengine.com/t/sequence-takes/436206)、[Versluis 2023](https://www.versluis.com/2023/09/working-with-takes-and-shots-in-unreal-engine/)
- MRQ でショットを除外する方法は、公式の UI やコマンドラインではなくフォーラムの Python 回避策として語られている。`job.shot_info` の各要素の `enabled` フラグを書き換える。ショットの順序と inner_name / outer_name が期待どおりである必要がある。 — [UE フォーラム: exclude shots via python](https://forums.unrealengine.com/t/how-to-exclude-certain-shots-from-a-mrq-rendering-via-python/2563979)
- 公式 API に「ジョブ」単位の SetIsEnabled はある（キューを読み込んだ後にジョブを無効にする用途）。ショット単位ではない。 — [Epic API: SetIsEnabled](https://dev.epicgames.com/documentation/unreal-engine/BlueprintAPI/MovieRenderPipeline/SetIsEnabled)
- Movie Render Graph のドキュメント目次には「Per-shot settings / Shot ranges / Supported features / Warm-up / Remote rendering / Command-line usage / Caveats」がある（本文は取得できず見出しのみ）。 — [Epic: Movie Render Graph](https://dev.epicgames.com/documentation/en-us/unreal-engine/movie-render-graph-in-unreal-engine)
- UE 5.5 の API には、各ショットの評価前にショット固有のグラフ設定を用意する仕組みがある。 — [Epic API: UMovieGraphDefaultRenderer（5.5）](https://dev.epicgames.com/documentation/unreal-engine/API/Plugins/MovieRenderPipelineCore/Graph/UMovieGraphDefaultRenderer/SetupRenderingPi-?application_version=5.5)
- MRG の不具合報告: 5.6/5.7 でカスタムフレーム範囲を単一フレームのサブシーケンスに使うと、ショットをまたいでサンプリングされて二重フレームが出る。回避策はショットを直接レンダするか、シャッター方式を変えること。UE 5.7.1 では複数カメラ時に最初のカメラしか出力されない報告もあり、返信で MRG への切り替えが勧められている。 — [UE フォーラム: MRG double frames 5.6/5.7](https://forums.unrealengine.com/t/5-6-x-5-7-x-movie-render-graph-double-frames-in-outputs-custom-frame-range/2709369)、[UE フォーラム: renders only first camera](https://forums.unrealengine.com/t/movie-render-queue-renders-only-first-camera/2693089)
- Deadline 連携: 2019 年頃のフォーラムでは「組み込みの Deadline 連携は MRQ 非対応」「ショットを別タスクに分けて進捗を追うのが難しい」とされ、カスタムの Deadline プラグインで MRQ をコマンドライン起動する回避策が出ている（古い情報）。現行の Deadline が MRG をショット単位で扱えるかは未確認。 — [UE フォーラム: UE5 Deadline Integration](https://forums.unrealengine.com/t/ue5-deadline-integration/724808)、[Thinkbox フォーラム](https://forums.thinkboxsoftware.com/t/unreal-engine-5-integration/29591)
- ウォームアップ／Lumen／TSR のアーティファクト（いずれもフォーラムの逸話）:
  - Render Warm Up Count（64/128 を試す）と Engine Warm Up Count（100/200）を上げ、Use Camera Cut for Warm Up と Render Warm Up Frames を有効にする提案。 — [UE フォーラム: AA と Lumen の不安定さ](https://forums.unrealengine.com/t/using-anti-aliasing-in-movie-render-queue-causing-lumen-artifacts-instability/688123)
  - World Partition の大きなレベルで、影が正しく出るまで 150〜300 フレームかかった。回避策は frame 0 の前に 300 フレーム余分にレンダして捨てること。ウォームアップフレームでは期待どおり捨てられなかったという。 — [UE フォーラム: Lumen の影の読み込み](https://forums.unrealengine.com/t/shadows-take-a-long-time-to-load-when-rendering-in-movie-render-queue-using-lumen/2631862)
  - 5.5 で TSR + Lumen のときランダムに壊れたフレームが出る報告。Lumen の前フレームの「残像」報告もある。 — [UE フォーラム: 5.5 MRQ Render Glitch](https://forums.unrealengine.com/t/ue-5-5-mrq-render-glitch-random-corrupted-frames-tsr-lumen/2695475)、[UE フォーラム: Lumen echos](https://forums.unrealengine.com/t/lumen-is-giving-echos-or-blur-from-previous-frames-when-rendering-from-movie-render-queue/1354486)
  - 黒フレームの原因が、サブレベルが Always Loaded でない、または自動露出が有効だった、という例。 — [UE フォーラム: black frames](https://forums.unrealengine.com/t/why-are-my-renders-are-coming-out-black-sometimes-seeing-frames-sometimes-not/2473566)
- Take Recorder が記録できるもの: レベル内のアクター、カメラカット、Level Sequence、Live Link、マイク、近くでスポーンしたアクター、プレイヤーのアクター、レベル可視性。 — [Epic: Take Recorder](https://dev.epicgames.com/documentation/en-us/unreal-engine/take-recorder-in-unreal-engine)
- Take Recorder の制限（公式）: キー付けできないプロパティは記録されない。Nearby Spawned Actors はゲームセッション中にスポーンしたものだけ。Spawn Emitter 由来のパーティクルは World ソース使用時のみ。レベル可視性は開始時の状態だけで、途中の変化は追わない。保存しないと記録は失われる。Auto Serialize は「experimental」。Niagara には言及がない。 — [Epic: Take Recorder](https://dev.epicgames.com/documentation/en-us/unreal-engine/take-recorder-in-unreal-engine)
- Take Recorder と Niagara（フォーラム、5.3 前後）: スポーンしたコンポーネントが一つおきにしか記録されない。ゲームプレイ中にスポーンした Niagara は Sequence に出ない（回避策は専用の Niagara アクターとしてスポーンする）。UE-208896 は一意名の生成が重複して一部の Niagara が記録されない原因とされる。UE-211063 は Blueprint ノードによる Niagara のアクティベートが記録されず、FX System Toggle トラックを手で足す必要がある。後続バージョンで直ったかは未確認。 — [UE フォーラム: 動的コンポーネント](https://forums.unrealengine.com/t/take-recorder-not-recording-all-dynamically-spawned-actor-components/1996294)、[UE フォーラム: Niagara](https://forums.unrealengine.com/t/take-recorder-not-recording-niagara/605801)、[UE-208896](https://issues.unrealengine.com/issue/UE-208896)、[UE-211063](https://issues.unrealengine.com/issue/UE-211063)
- Replay System は DemoNetDriver がレプリケーションのデータを Replay Streamer に渡して保存する。ローカルファイル Streamer が既定（4.20 以降）で `Saved/Demos/*.replay`。再生は pause・速度変更・時間ジャンプができ、`demorec` / `demoplay` / `demoscrub` などのコンソールコマンドがある。Epic は「producing gameplay trailers or in-game cutscenes」に役立つと明記している。 — [Epic: Using the Replay System](https://dev.epicgames.com/documentation/unreal-engine/using-the-replay-system-in-unreal-engine)、[Epic: DemoNetDriver and Streamers](https://dev.epicgames.com/documentation/unreal-engine/demonetdriver-and-streamers-in-unreal-engine)
- Level Snapshots はレベルの状態を保存し、「Restore Level Snapshot」でアクターを保存時の状態に戻せる（削除されたアクターの復元、一部のアクター／プロパティだけの復元、フィルタ、取り消しが可能）。Landscape、USD アクター、コンソール変数は追跡しない。 — [Epic: Level Snapshots](https://dev.epicgames.com/documentation/unreal-engine/level-snapshots-in-unreal-engine)

### Inferences
- 「変わったショットだけ再レンダ」は、Epic の標準では人間が手で `enabled` を切るか Python で書く領域で、内容ハッシュによる陳腐化検出は見つからなかった（不在の証明ではない）。
- Take は版を並べて切り替えるだけで、「定義が同じなら以前の成果物を再利用する」という内容アドレス的な管理ではない。
- Replay は「ゲームプレイを別カメラで撮り直す」ための標準の道具だが、ゲームのコードやアセットが変わると古いリプレイは再生できなくなる可能性が高い（公式の明言は未確認。推測）。
- Level Snapshots は状態の復元であり、カメラ／ショット定義の管理とは別の層。

### Gaps
- Sequencer 概要ページと MRG の各小節（Per-shot settings、Warm-up、Remote rendering、Caveats）の本文を取得できなかった。公式の記述での「ショット範囲」「リモートレンダ」の仕様は未確認。
- Replay が UE 5.5〜5.7 でバージョン互換をどう扱うか、リプレイからの Sequencer 再撮影の公式手順は見つからなかった。
- MRQ の Python 自動化（MoviePipelinePIEExecutor 等）の公式ページは取得していない。
- Take Recorder の Niagara 問題が 5.5〜5.7 で解消したかは未確認。

## Q2. UE のマーケットプレイス（Fab）とサードパーティのトレーラー向けプラグイン

### Takeaway
見つかった有償／無償の道具は、カメラ操作・レンズ・ライティングの補助か、既存の MRQ のフレーム選択の補助にとどまる。再レンダ判定や版管理を担うものは見つからなかった。価格とレビューは取得できていない。

### Cited Findings
- Trailer Camera Toolkit（Altar Creative）: プレイ可能なビルドの中で動くリアルタイムのカメラシステムで、「no Sequencer, no editor required」。対象は「marketing teams, cinematic capture artists, and trailer teams」。有効化はコンソールコマンド `ToggleMarketingCamera`。クライアント側で完結する隔離モデルで、1クライアントにつき PlayerController を1つに限る設定だと動かない可能性がある。録画・書き出しの仕組みは説明されていない。 — [UE フォーラム投稿](https://forums.unrealengine.com/t/altar-creative-trailer-camera-toolkit/2690110)、[導入ページ](https://sites.google.com/altarcreative.com/marketingcamera/installation-integration)、[Fab リスティング](https://www.fab.com/listings/b387f8d7-7367-466f-b58a-714c2adb4d09)（価格は取得できず）
- MRQ Frame Picker（Alfredo Mendez）: 開いている Level Sequence の、あるアクターのトラックのキーフレームを読み、そのフレームだけの MRQ キューを作る。保存済みの MRQ プリセットをジョブにコピーしてフレーム範囲だけを設定し、「it never touches your render settings」。Editor Utility Widget として動く。必要なプラグインは Movie Render Pipeline、Python Editor Script、Sequencer Scripting、Editor Scripting Utilities。価格・対応バージョンは記載なし。 — [UE フォーラム投稿](https://forums.unrealengine.com/t/alfredo-mendez-movie-render-queue-frame-picker-keyframe-render-tool/2741460)、[Fab リスティング](https://www.fab.com/listings/63f4e108-b443-4d7e-9b0e-821915d4ca1d)
- Cinematographer（Lite / PRO）: 標準の Cine Camera の上に載るカメラシステム。Lite は 13 のカメラボディと 42 のレンズに限られ、カスタムボディは PRO のみ。実機カメラに合わせたい人向け。 — [Fab: Cinematographer](https://www.fab.com/listings/b1eb371c-6635-44e5-bba4-e3f870a5d4e4)（ベンダー自身の記述）
- Cinematic Toolkit（Form Studios）: 焦点距離プリセット（24/35/50/85/135mm）、3点照明ツール、Level Sequence へのカメラ／ライトの即時追加。 — [Fab: Cinematic Toolkit](https://www.fab.com/listings/14683ce0-0e8f-4c3b-905b-8dce7ce9de0f)
- 同名に近い別製品: StraySpark の「Camera Toolkit」は複数カメラとタイムライン書き出しを扱う別物。 — [StraySpark docs](https://www.strayspark.studio/docs/camera-toolkit/bake-and-export)

### Inferences
- この領域のプラグインは「カメラを置く・動かす・レンズを選ぶ」が中心で、「どのショットが古いか」を扱うものは今回の検索では出なかった。
- Trailer Camera Toolkit はゲーム内で自由に撮る方向（録画は別手段）で、再現性（同じ入力で同じ映像）は売り文句になっていない。

### Gaps
- 上記すべての価格、ライセンス、評価、UE 対応バージョン。Fab の個別ページ本文は取得できなかった。
- 「ショット管理プラグイン」や「ゲームプレイレコーダー」の専用製品は、検索では特定できなかった。

## Q3. Unity（Timeline / Cinemachine 3 / Recorder / Sequences）

### Takeaway
Unity は Timeline + Cinemachine + Recorder の組み合わせで撮るが、ショット管理用の Sequences パッケージは Unity 6.1 でサポート終了予定とされている。Sequences に撮影単位の版管理や再レンダ判定は書かれていない。

### Cited Findings
- Sequences: Timeline 上に「editorial structures」を作り、ショットの文脈でアセットを管理する。Sequences ウィンドウ、Sequence Assets（Variants を含む）、Assembly、Timeline、Hierarchy の各ウィンドウで構成される。Sequences ウィンドウから Unity Recorder を使って録画・書き出しができる。 — [Unity: Sequences 2.1 features](https://docs.unity3d.com/Packages/com.unity.sequences@2.1/manual/features.html)、[Unity 6 Cinematic Studio](https://docs.unity3d.com/6/Documentation/Manual/CinematicStudioFeature.html)
- 「Support for the Sequences package is scheduled to end as of Unity 6.1.」同ページに版管理と Take の記述はない。 — [Unity: Sequences 2.1 features](https://docs.unity3d.com/Packages/com.unity.sequences@2.1/manual/features.html)
- Cinematic Studio フィーチャーセット自体もサポート終了予定と書かれている（Unity 6.0 マニュアル）。 — [Unity 6.0 Cinematic Studio](https://docs.unity3d.com/6/Documentation/Manual/CinematicStudioFeature.html)
- Recorder: Play モードから画像シーケンス、動画、アニメーションクリップ、音声、AOV（任意の出力変数）を保存する。Timeline の Recorder Track のクリップで範囲を制御でき、全体を書き出す必要はない。 — [Unity 6.0 Cinematic Studio](https://docs.unity3d.com/6/Documentation/Manual/CinematicStudioFeature.html)
- Cinemachine 3.0: Brain が優先度のスタックからアクティブなカメラを選び、ブレンドしてカメラに適用する。カットは 0 時間のブレンド。Timeline の Cinemachine Track に置いたクリップが重なればブレンド、端が接すればカットになり、Timeline が Brain の判断を上書きする。 — [Cinemachine 3.0 Brain](https://docs.unity3d.com/Packages/com.unity.cinemachine@3.0/manual/CinemachineBrain.html)、[Cinemachine 3.0 Timeline](https://docs.unity3d.com/Packages/com.unity.cinemachine@3.0/manual/CinemachineTimeline.html)
- Unity Live Capture の Take System: Take Recorder Track にショットをクリップとして並べ、複数の Take で一つのショットを作る。このパッケージも Unity 6.1 でサポート終了予定とされている。 — [Unity Live Capture: Shots in Timeline](https://docs.unity3d.com/Packages/com.unity.live-capture@4.0/manual/take-system-shots-in-timeline.html)

### Inferences
- Unity では公式のショット管理層が縮小方向にあり、Timeline + Cinemachine + Recorder を自前で束ねる形になっている。再レンダの最小化は標準にない。

### Gaps
- Cinemachine 3.1 の自動フレーミング機能の詳細、Recorder の複数レコーダー同時動作、Recorder のバッチ（コマンドライン）実行の公式記述は取得していない。
- Unity 6.1 以降に Sequences の後継があるかは未確認。

## Q4. Godot（Movie Maker mode と周辺アドオン）

### Takeaway
Godot の Movie Maker は固定 FPS のオフラインレンダで、フレーム刻みの時間は一定になるが、ショット管理や差分再レンダはない。入力リプレイのアドオンは存在するが小規模で、シネマティック用途に合う成熟した製品は見つからなかった。

### Cited Findings
- Movie Maker はオフラインレンダで、`--fixed-fps` が `_process()` の delta を実レンダ時間に関係なく一定にする。例: `godot --path <project> --write-movie output.avi --fixed-fps 30`。出力解像度はビューポートに従い、ウィンドウは表示解像度に制限される。 — [Godot docs: MovieWriter](https://docs.godotengine.org/en/latest/classes/class_moviewriter.html)
- 組み込みの出力形式は OGV、AVI（MJPEG）、PNG 連番 + WAV の3種。AVI は 4GB まで。音のミックスレートは FPS で割り切れる値にしないとずれる。正常終了しないと書き込みが完了せず、Ctrl+C では `_write_end()` が呼ばれない。 — [Godot docs: MovieWriter](https://docs.godotengine.org/en/latest/classes/class_moviewriter.html)
- 入力リプレイ: drsounds/godot-replayer-system は Godot 3.5 向けの作業中のイベント記録／再生で、動画は出さない。「SReplay」は Godot 4 向けの決定的な入力の記録と再生で、デバッグ寄り（ブログ記事経由）。GIF Replay Recorder は直近数秒の映像を記録する。 — [godot-replayer-system](https://github.com/drsounds/godot-replayer-system)、[A Godot Replay System（2025-09-27）](https://dresswithpockets.github.io/2025/09/27/godot-replay.html)、[Asset Library: GIF Replay Recorder](https://godotengine.org/asset-library/asset/edit/22094)

### Inferences
- 固定 FPS は「時間の刻みの再現性」だけを保証し、物理や乱数やゲーム側の非決定性は別の問題として残る。

### Gaps
- Godot 公式のシネマティック用のシーケンサー機能（AnimationPlayer 以外）と、現行のシネマティック系アドオンの有無は確認できなかった。
- 「決定的な出力」をどこまで保証するかの公式の言及は見つからなかった（上の docs は delta の一定化までしか述べていない）。

## Q5. リプレイ／決定的再生・フォトモード・OS／ドライバ側の録画（Steam、Ansel、Highlights）

### Takeaway
ゲーム側の入力リプレイ（またはエンジンのリプレイ）を持つタイトルだけが「同じ場面を別カメラで撮り直す」ことができる。Steam の Game Recording は後追いのクリップ抽出に向き、撮り直しの再現性は提供しない。Ansel / Highlights については裏付けが取れなかった。

### Cited Findings
- Steam Game Recording は 2024 年 6 月に発表され、2024 年 11 月に全ユーザーへ公開された。バックグラウンド録画は容量上限を超えると古い映像から上書きされる。タイムラインにはスクリーンショットや実績のマーカーが付き、対応ゲームは開発者定義のマーカーを送れる。Steam Deck と、オーバーレイが動く非 Steam ゲームでも使える。 — [PC Gamer](https://pcgamer.com/software/platforms/steam-game-recording-is-now-available-for-everyone-and-its-packed-with-neat-features)、[Benzinga](https://in.benzinga.com/general/gaming/24/11/41784656/steams-built-in-game-recording-now-available-to-all-users-what-you-need-to-know)
- UE の Replay System は上記 Q1 のとおり、トレーラー用途を公式に挙げている。 — [Epic: Using the Replay System](https://dev.epicgames.com/documentation/unreal-engine/using-the-replay-system-in-unreal-engine)

### Inferences
- Steam の録画は「起きたことを取り出す」道具で、定義を直して撮り直す道具ではない。ゲームが更新されたら素材は撮り直しになる。

### Gaps
- NVIDIA Ansel / Highlights の現状（終了・非推奨かどうか）は検索で何も出ず、未確認。
- ゴーストシステム、巻き戻し、フォトモードの具体例（ゲームごとの実装）は調べていない。
- GDC 講演の調査はしていない。

## Q6. 開発者の痛み (a)〜(e) との対応、およびどの道具も解いていないこと

### Takeaway
各ツールは「撮る」「カメラを置く」「ショットを並べる」を支えるが、ゲームの変更後に「どのショットが古くなったか」を検出して、そのショットだけを撮り直す層はどれにも見つからなかった。再現性は、エンジンのリプレイや固定FPSなど、部分的な手段に分散している。

### Cited Findings
- (a) 変更後の全撮り直し: MRQ の部分実行は手動の enabled 切り替えか Python（[UE フォーラム](https://forums.unrealengine.com/t/how-to-exclude-certain-shots-from-a-mrq-rendering-via-python/2563979)）、キーフレーム単位の選択は MRQ Frame Picker（[UE フォーラム](https://forums.unrealengine.com/t/alfredo-mendez-movie-render-queue-frame-picker-keyframe-render-tool/2741460)）。どちらも「何が変わったか」は人間が決める。
- (b) 再現できない take: Replay System（[Epic](https://dev.epicgames.com/documentation/unreal-engine/using-the-replay-system-in-unreal-engine)）、Godot の固定 FPS（[Godot](https://docs.godotengine.org/en/latest/classes/class_moviewriter.html)）、Godot の入力リプレイ・アドオン（[blog](https://dresswithpockets.github.io/2025/09/27/godot-replay.html)）が部分的に応える。Sequencer の Take は版の切り替えで、同じ入力から同じ映像を保証する仕組みではない（[Epic](https://dev.epicgames.com/documentation/unreal-engine/sequences-shots-and-takes-in-unreal-engine)）。
- (c) Take Recorder の取りこぼし: [Epic の制限](https://dev.epicgames.com/documentation/en-us/unreal-engine/take-recorder-in-unreal-engine)と [UE-208896](https://issues.unrealengine.com/issue/UE-208896)、[UE-211063](https://issues.unrealengine.com/issue/UE-211063)。マテリアルの変化の取りこぼしを直接述べた一次資料は見つからなかった（キー付けできないプロパティは記録されない、という公式記述のみ）。
- (d) MRQ のウォームアップ・Lumen・TSR・バッチ失敗: Q1 のフォーラム投稿群を参照。
- (e) 学習コスト: Sequencer の Shot/Take の癖（Take 2 から始まる等）は [Versluis](https://www.versluis.com/2023/09/working-with-takes-and-shots-in-unreal-engine/) が回避策つきで書いている。定量的な「学習曲線」の調査は見つからなかった。

### Inferences
- 今回の調査範囲で、内容に基づく陳腐化検出、定義と出力の分離、状態の永続化、安い順のプレビュー階段を一つにまとめた製品は見つからなかった（不在の証明ではない）。
- どの道具も、成果物の同一性を「ショット定義のハッシュ」で管理しない。Take は人間が名前で管理する。
- Epic と Unity の公式の動きとして、Unity 側のショット管理（Sequences、Live Capture）がサポート終了予定なのは、シェアを取りにくい領域だと読める。ただし理由は公式に書かれていない（推測）。

### Gaps
- 「この領域に既存製品が全くない」とは断言できない。Fab 全体の網羅検索、UE 5.6/5.7 のリリースノート、Epic の MRG 公式ページ本文は未確認。
- 評価（レビュー）や価格の一次情報。
