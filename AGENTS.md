# Shotpilot — Agent Instructions

UE5 プロジェクトからティザー映像（30〜60秒）を撮るための、ショット設計・撮影レーン。
ショットを `shots.yaml` に定義し、変わったショットだけを再レンダする。

一行で言うと「ティザーのショットを AI が組み、人間が1カットだけ直すと、その1カットだけ撮り直る」。

提出先は Built with Claude: Japan 2026。開発者は1人。

## 最優先の要件: 部分修正ループ（D-13）

次の4点は製品の芯にあたる。崩す変更はしない。迷ったらこれを守る側に倒す。

1. **ショット単位で再レンダする**: 7ショットのうち1つを直したら、撮り直すのはその1つだけ。
2. **定義と出力を分ける**: `shots.yaml` が唯一の正本。`render/` とエンジン側のアセットはすべてそこから導出する。
3. **状態を持つ**: 何を push したか、どの take が最新か、何を直したかを `.shotpilot/state.json` に残す。Sequencer で人間が直した差分は `pull` で `shots.yaml` に戻す（三方比較）。
4. **プレビューを安くする**: `previz`（エンジン起動なし）→ `still` → `preview` → `final` の順に、安い経路から確認できるようにする。

## コマンド

```bash
cargo test                        # 全テスト（UE も API キーも不要。mock エンジンで回る）
cargo test --test loop only_changed
cargo clippy --all-targets        # 警告ゼロを保つ
cargo build --release             # target/release/shotpilot.exe（CLI）
cargo build --release -p shotpilot-app   # target/release/shotpilot-app.exe（デスクトップアプリ）
shotpilot init --engine mock      # サンプル shots.yaml を作る
shotpilot init --engine three     # Three.js のサンプルゲーム（病棟、game/）と Lanternless のティザーを作る。Edge か Chrome で撮る（UE なし、D-72）
shotpilot status                  # fresh / stale / out-of-sync の一覧
shotpilot render previz           # 古くなったショットだけをエンジンなしで描く
shotpilot set s03 camera.lens.focal_length=50
shotpilot revise --aspect camera "もっと大きく動かして"  # カメラだけを全カット組み直す（台本・目的・秒数はコードで守る。--shot で絞る）
shotpilot check                   # ショット検査（P0 は承認を止める）
shotpilot approve s03             # 構図を承認（画が変わると外れる）
shotpilot approve --script        # 台本を承認（文字・並び・各カットの目的と秒数が変わると外れる。カメラでは外れない）
shotpilot brief genre=ホラー       # brief: を見る・書く
shotpilot finish logo=art/logo.png # finish: を見る・書く（書体・タイトルの大きさ・ロゴ・動き。design <id> でルックの書体と動きが入る）
shotpilot moodboard               # ブリーフからムードボードを4枚生成（Codex か OpenAI キー）
shotpilot storyboard [s03]        # previz の線画を下絵に、リアル寄りの絵コンテを生成（確認用。Cut には入らない）
shotpilot discover "規則で見せるホラー"  # 企画（D-45）: 足りないときだけ質問し、方向・ルック・スキル・企画を brief: に書く
shotpilot design [id] [--fork]    # ルック（デザインシステム）の一覧・指定。--fork でプロジェクトに DESIGN.md を写す
shotpilot skills                  # ティザーの型の手順書（スキル）の一覧
shotpilot concept [--match]       # 構成画（D-46）: 企画のビートごとの画を生成する / --match でレベルに似た場所を探す（無ければ別の候補を撮る）
shotpilot produce                 # 初回: 制約を全部渡して一気に1本出す（企画→ロケハン→ステージ→ムードボード→構成と仕上げ→previz→Cut）
shotpilot produce --review        # 同じだが、台本とカットの previz で止める（アプリの既定）。確かめて直したら produce --resume で音→撮影→Cut
shotpilot scout                   # 工程0: 各アンカーを4方向から撮る（scout/）。アンカーが無ければ、同じ UE の起動でレベルから取り込み、アセットも棚卸しする
shotpilot assets --scan           # 工程0: mesh を寸法付きで棚卸し → prop add <name> <asset>
shotpilot cut                     # 既存 take を連結する（レンダはしない。BGM・SE があれば載せる）
shotpilot sound                   # BGM と SE をミックスし、ローンチ映像7本と比べて採点する
shotpilot sound --design          # 音の設計をやり直す（ルックの音の性格に沿い、ほかのティザーで使用中の音は避ける。足りない音は検索語で出す）
shotpilot promo                   # 発信計画（D-47）: 公開後の動画ごとのチェックシートと、定期発信の予定
shotpilot promo update v1.1       # このカットを引き継いだアップデートトレーラーを隣に作る（ゲームが変わったカットだけ撮り直し）
shotpilot promo post s03 [--clip] # take をそのまま静止画か1カットの動画で promo/posts/ に書き出す
shotpilot reference ref.mp4       # 参照のティザーを測り（180 秒まで）、つなぎの長さを finish.motion に写す（D-71。画・文字・音は写さない）
shotpilot reference --compare     # 最新の Cut を同じ物差しで測り、参照と同じ割合の時刻のコマを並べる（人が見る用）
shotpilot-app [path/to/shots.yaml]   # アプリ。引数がなければホーム（最近のプロジェクト・新規作成）を開く
node tools/i18n_extract.js        # 英語モードの辞書に無い画面の文言を一覧する（D-51）
shotpilot eval --models claude-haiku-5-5,claude-opus-5-5 --runs 3   # 評価（D-68）: eval/cases の6題材で plan を書かせ、実測の型との近さをコードで採点する
shotpilot eval --replay eval/runs/<日時>   # 保存した応答を今のコードで採点し直す（モデルは呼ばない）
shotpilot claude install          # ゲームのフォルダに Claude Code の Skill とフック（古くなったカットを知らせる）を入れる（D-66）
shotpilot hook session-start      # Claude Code が呼ぶフック（post-tool も）。何も書き換えず、常に終了コード 0
```

`cut` の動画書き出しは ffmpeg を使う（無ければ storyboard と spec だけを出す）。`plan` と `revise` は、ログイン済みの Claude Code（`claude` コマンド）か API キー（アプリの設定、または `ANTHROPIC_API_KEY`）で Claude を呼ぶ（モデルは `SHOTPILOT_MODEL` で上書きできる。effort は `low`〜`max` をアプリの接続設定か `SHOTPILOT_EFFORT` で変えられ、既定は `medium`。Claude Code 経由でも API 経由でも同じ値が効く）。OpenAI 互換のサーバー（vLLM・Ollama・llama.cpp・OpenRouter など）は `SHOTPILOT_PROVIDER=openai` と `SHOTPILOT_OPENAI_BASE_URL`（キーは `SHOTPILOT_OPENAI_KEY`）で使える。`SHOTPILOT_CANDIDATES=3` で plan を3案書かせ、コードの採点で1つを選ぶ（D-68）。測り方は `eval/README.md`。

## 構成

```
crates/shotpilot/            コア（ライブラリ + CLI）
  src/model.rs        shots.yaml のスキーマ・検証・resolve（Definition → ResolvedShot）
  src/fingerprint.rs  再レンダ判定。品質プリセット（previz/still/preview/final）
  src/state.rs        .shotpilot/state.json と render/<shot>/<quality>/<fp12>/ の take 管理
  src/content.rs      take が写していたゲームの中身（エンジンが報告したパッケージのファイルと SHA-256）。変わったカットだけを古いとする
  src/workspace.rs    上の3つとエンジンを束ねる。全操作はここを通る
  src/engines.rs      mock / ue5 / three バックエンド。共通の job/result JSON プロトコル。エディタの無いエンジンのシーケンスの台帳（SeqStore）
  src/three.rs        Three.js のエンジン（D-72）。game/ の JSON の読み込み、写っていた物の判定（D-56）とロケハンの壁よけ（D-55）、loopback のサーバーと headless の Edge / Chrome
  src/threegame.rs    three のサンプルゲーム。Lanternless の病棟（間取りとアンカーは UE 版と同じ座標、中身は three 独自）を、物ごとに1ファイルで書き出す
  three/              描画のページ（render.html。手続きで焼く質感、AO・被写界深度・ブルーム・高さの霧・グレイン）と同梱の Three.js 0.170（MIT。バイナリに埋め込む）、サンプルの shots.yaml（ward.yaml）
  src/sync.rs         Sequencer → shots.yaml の逆同期（base / engine / ours の三方比較）
  src/scout.rs        工程0「ロケハン」: アセットの棚卸し、props への追加、アンカーのロケハン写真
  src/moodboard.rs    ブリーフのムードボード（参照画像の保存、OpenAI での生成、plan に渡す画像ブロック）
  src/storyboard.rs   絵コンテ（D-33）。previz の線画を下絵に、ロケハン写真とムードボードを参照して画像生成で描く確認用の1枚
  src/suggest.rs      チップの予測（D-62）。積む UI に出す次のチップを、ブリーフ・カット・ロケハンの記述から Claude に低い effort で1回だけ出させる。何も書かない
  src/produce.rs      初回の一気通し（D-28）。足りない下ごしらえを埋め、plan を1回、最安の品質で撮って仕上げ込みで Cut
  src/craft.rs        plan に書く前に渡す仕様（ブリーフ、骨格、規則、画角の距離表）とコードによる整形
  src/designsys.rs    ルック（D-45）。references/design-systems/<id>/ の DESIGN.md・tokens・manifest を埋め込み、plan に全文を渡し、tokens で仕上げを決める
  src/concept.rs      構成画（D-46）。企画のビートごとの画（concept/）と、ロケハンで見つけた場所の記録。plan に画と場所の写真を渡す
  src/skills.rs       スキル（D-45）。references/skills/<id>/SKILL.md を埋め込み、企画で選ばれたものの本文を plan に渡す
  src/refs.rs         実測リファレンス（references/craft/ と genres/<id>/、流行りの型 24 本（6 ジャンル × 4）を埋め込み）。plan に渡す文と、骨格のビート配分・カット数・最初のカットと title の長さ
  src/reference.rs    参照のティザー（D-71）。動画をコーパスと同じ物差し（明るさとカット）で測り、つなぎの長さを finish.motion に、置き場所を plan に渡す。Cut と並べて比べる
  src/shotcheck.rs    ショット検査。トレーラーの規則でショットリストを lint する（P0 出典あり・承認を止める / P1 / P2 推定）
  src/previz.rs       エンジンを使わないワイヤーフレーム描画（PNG 書き出し）
  src/cut.rs          take をショット順に連結してティザーにする（ミックスがあれば音を載せ、仕上げがあれば焼き込む）
  src/finish.rs       仕上げ: 帯・色調・台本のカードと重ね文字・タイトル・つなぎ（黒/白）を ffmpeg のフィルタにする
  src/fonts.rs        書体（D-52）。references/fonts/catalog.json の目録、Google Fonts からの取得とキャッシュ、書体が持つ字の判定
  src/fx.rs           カメラ効果（D-57）。色ずれ・グリッチ・明滅・砂嵐・VHS・手持ちの揺れ・赤フラッシュの閉じた語彙と、Cut で焼く ffmpeg のフィルタ
  src/motion.rs       動き（D-53）。references/motion/profiles.json のジャンルごとの既定、finish.motion の解決、文字の入り・保ち・抜けの式とつなぎの長さ
  src/handover.rs     つなぎ（D-70）。references/motion/transitions.json のレシピ（open-design のつなぎの参照に倣う）を ffmpeg のフィルタにする。plan のつなぎを cut のほか 2 種に整える
  src/script.rs       台本（D-31）: ショットの間に入るカードと、ショットに重なる文字。Cut の並び（layout）を一か所で計算する
  src/sounddesign.rs  音の設計: Claude がライブラリから区間ごとの曲とカード・ショットの音を選び、取り込んで書き込む
  src/sound.rs        BGM・SE・環境音（`audio:`、各ショットの `sfx:`、場所ごとの `audio.ambience`）の検証・配置・ミックス。cut/sound/ にキャッシュする
  src/soundcheck.rs   ミックスの検証。earshot の critique のうち、モデルを使わない部分の移植（MIT）
  src/library.rs      音源ライブラリ（プロジェクト外、D-26）の検索と、プロジェクトの audio/lib/ への取り込み
  src/ai.rs           Claude によるショットリスト作成（plan）と部分修正（revise）。Messages API を直接呼ぶ
  src/connect.rs      Claude への接続。既定はログイン済みの Claude Code（`claude -p`）、ほかに API キーと OpenAI 互換のサーバー
  src/eval.rs         評価（D-68）。plan の出力を実測の型（refs::beats と tokens）と比べてコードで採点し、題材 × モデル × 回数で回して応答を残す
  src/claudecode.rs   Claude Code との連携（D-66）。ゲームを直している Claude Code に、古くなったカットをフックで知らせる。Skill とフックの取り付け
  claude/skills/      ゲームのフォルダに入れる Skill の雛形（バイナリに埋め込む）
  src/geometry.rs     UE 座標系のカメラ計算
  src/yamlio.rs       コメントを保持する YAML 編集（行単位の差し替え + 再パースで検証）
  src/templates.rs    shots.yaml の雛形
  src/main.rs         clap の CLI
  src/projects.rs     ホーム画面の裏側（D-30）。最近のプロジェクトの一覧、プロジェクトのカード、新規作成（mock / UE5）
  src/promo.rs        発信計画（D-47）。公開後の動画（発表・デモ・ローンチ・アップデート・セール）のチェックシート、隣への派生プロジェクト、定期発信の書き出し
  src/server.rs       アプリの裏側。Workspace の上に載る薄い HTTP + SSE 層（loopback 限定）
  web/index.html      アプリの studio 画面（ビルドなし、依存なし。バイナリに埋め込む）
  web/home.html       アプリのホーム画面（D-30。最近のプロジェクト、プロンプトからの新規作成、設定）
  web/i18n.js         英語モード（D-51）。PC の言語が日本語でなければ、画面の日本語を i18n-en.json の辞書で英語に置き換える
  ue/sp_entry.py      Unreal の中で動くスクリプト。唯一の Python（別ルールあり。バイナリに埋め込む）
  tests/              mock エンジンでループ全体を検証する
crates/shotpilot-app/        Tauri のデスクトップアプリ。コアを組み込み、ウィンドウ1枚で開く
```

## 守ること（不変条件）

- **fingerprint にはピクセルを変えるものだけを入れる。** 並び順、`purpose`、メモ、音（`audio`、`sfx`、ショットの `ambience`）、`brief:`、仕上げ（`finish:`、`caption`、`transition`）は入れない。仕上げは take のピクセルではなく Cut の出力を変える。並べ替えやメモ、音の修正で再レンダが起きてはいけない。 守るテスト: `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`、`order_purpose_notes_never_invalidate`、`editing_sound_never_rerenders`、`the_finish_is_definition_and_never_rerenders`。
- エンジン側スクリプトが、同じ入力に対して違う出力を出すように変わったら `fingerprint::RENDERER_VERSION` を上げる。previz の描画を変えたら `PREVIZ_VERSION` を上げる（エンジンの take は無効にしない）。エンジンへの入力を一部のプロジェクトでだけ変えるとき（World Partition のマップのカメラの前置き、`engines::streaming_lead`）は、そのプロジェクトの build と render の指紋にだけ入れ、ほかのプロジェクトの指紋は変えない。 守るテスト: `previz_drawing_is_pinned_to_its_version`、`fingerprints_match_the_python_version`、`anchors_take_a_size_and_previz_alone_follows_it`、`a_streamed_level_warms_up_on_the_shot_camera_and_other_levels_keep_their_fingerprints`。 人間が守る: sp_entry.py の出力を変えたときに RENDERER_VERSION を上げること（UE の画は mock では再現できない）。
- fingerprint は Python 版と同じ正規化 JSON（キー整列・区切り `,` `:`）の SHA-256。`fingerprints_match_the_python_version` を落とさない（落ちると UE で撮った take が全部撮り直しになる）。 守るテスト: `fingerprints_match_the_python_version`。
- **take は fingerprint で管理し、上書きしない。** 定義を元に戻したら、古い take を再利用する。`render/` 以下を手で編集しない。 守るテスト: `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`、`revert_reuses_old_take`、`a_game_change_rerenders_only_the_shots_that_show_it`、`a_nudge_rerenders_only_its_shot_and_undo_reuses_the_old_take`。 人間が守る: render/ 以下を手で編集しないこと。
- **ゲームの中身の変化は fingerprint に入れず、take の `deps` で見る（D-37）。** エンジンはレンダ中に写っていたもののパッケージを報告し、take はそのファイルの大きさ・時刻・SHA-256 を持つ。中身が変わったカットだけが古くなり、撮り直した take は `<fp12>~N/` に置く（前の take は消さない。中身を戻せば前の take が使われる）。fingerprint は定義だけのまま（Python 版との互換と、撮った take を守るため）。迷ったら「写っている」側に倒す（撮り直しすぎる方が、見落とすより安全）。 守るテスト: `a_game_change_rerenders_only_the_shots_that_show_it`、`previz_never_goes_stale_from_the_game_and_the_cut_falls_back_to_it`、`a_forced_take_replaces_and_a_deleted_file_counts_as_a_change`。 人間が守る: UE の中で、画面に写っているものだけを数える判定（D-56、sp_entry.py）。
- `shots.yaml` を機械的に書き換えるときは、必ず `yamlio::Doc` を通す（変えるショットの行だけを差し替え、再パースして意図どおりか確かめてから書く）。人間が書いたコメントとキー順を消さない。 守るテスト: `a_rejected_machine_write_leaves_the_file_untouched`、`every_machine_write_path_keeps_hand_written_comments`、`comments_survive_machine_edits`、`replace_keeps_other_comments`。
- Claude（`ai.rs`）が返した編集は、`shots.yaml` に書く前に resolve して検証する。編集してよいショットの範囲は、プロンプトではなくコードで強制する。音だけの指示（`Pointer::sound`）では `sfx`・ショットの `ambience`（`none` か実在するアンカー）と `audio` だけを取り込む。カメラだけ・光だけの指示（`Pointer::fields`、`ai::ASPECTS`）では、そのキーだけを取り込む。文字・つなぎだけの指示（`finish`）では、ショットの `caption`・`transition`・`exposure` と、`Pointer::texts` に挙げた台本の行の文字・種類・タイミング（`ai::LINE_TEXT_FIELDS`）だけを取り込み、行の置き場所（`after` / `on`）は変えさせない。`finish:` は全体への指示（`Pointer::finish`）でだけ、色調・帯・書体（`ai::FINISH_FIELDS`）に限って変えさせる（タイトルとエンドカードは人間が書く）。ショットに絞った指示では BGM を変えさせない。 守るテスト: `a_sound_note_takes_only_the_sfx`、`a_sound_note_may_give_a_place_its_air_and_silence_a_shot`、`a_note_on_the_camera_changes_only_the_camera`、`a_finishing_note_stays_inside_its_scope`、`the_finishing_never_rerenders_and_undo_takes_it_back`。
- 定義の編集は `Workspace::write_definition` の1か所を通し、変えた部分の前後を履歴（`undo`）に残す。元に戻すは、その部分が後から変わっていたら上書きせずに止める。戻した画は fingerprint が同じなので、古い take をそのまま使う（撮り直さない）。 守るテスト: `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`、`undo_goes_back_one_edit_at_a_time_and_stops_at_a_plan`、`undo_never_overwrites_a_later_hand_edit`、`a_nudge_rerenders_only_its_shot_and_undo_reuses_the_old_take`。
- 台本の承認は `state.json` に script fingerprint（台本の文字と位置、各カットの id・beat・purpose・duration と並び）で残す（D-38）。カメラ・光・音では外れない。 守るテスト: `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`、`script_approval_follows_the_script_and_never_the_camera_light_or_sound`、`the_first_pass_stops_at_the_review_and_resumes_after_it`。
- 構図の承認は `state.json` に look fingerprint（品質に依らない画の指紋）で残す。画が変わる編集で外れ、編集的な変更では外れない。P0 の指摘があるショットは承認させない。 守るテスト: `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`、`composition_approval_survives_every_editorial_edit_and_follows_the_picture`、`approval_follows_the_picture_not_the_notes`、`opening_on_the_title_is_p0_and_blocks_approval`。
- 絵コンテ（`storyboard/`）は確認用で、take ではない。`render/` にも state の take にも入れず、Cut には使わない。look fingerprint を名前にして、画が変わる編集で古くなり、編集的な変更では古くならない。生成するのは人間が頼んだときだけ（produce やレンダでは自動で描かない）。 守るテスト: `generated_pictures_never_become_takes_or_part_of_the_cut`、`storyboard_frames_are_a_preview_outside_the_takes`、`storyboard_over_http`。
- 構成画（`concept/`、D-46）はムードボードと同じく参照に限る。take にも Cut にも fingerprint にも入れない。生成するのは人間が頼んだとき（ボタンか `shotpilot concept`）だけで、企画や produce では自動で描かない。構成画と撮った写真の見比べは「本物の写真の中から選ぶ」ことで、Claude が自分の出力を見直すこと（D-22）ではない。選べる写真の id はコードで確かめる。 守るテスト: `generated_pictures_never_become_takes_or_part_of_the_cut`、`the_plan_describes_concept_frames_and_pictures_come_on_request`、`the_scout_finds_a_place_for_each_frame_and_tries_others_when_none_fits`、`frames_with_no_place_stay_mood_only`。
- 発信計画（`promo.json`、D-47）は編集的で、fingerprint にも shots.yaml にも入れない。派生プロジェクトの `promo.json` は元の計画への `link` だけを持つ（1つのゲームに計画は1つ）。派生で shots.yaml を書くのは新しいプロジェクトの分だけで、元のプロジェクトの定義と take は触らない。take を引き継ぐときは take フォルダをそのまま写し、写した側の take.json の `deps` だけを元のゲームの場所へ向け直す（同じ定義は同じ fingerprint なので撮り直さず、ゲームが変わったカットだけが古くなる）。 守るテスト: `deriving_a_milestone_video_never_touches_the_origin`、`deriving_an_update_trailer_never_touches_the_origin_and_only_repoints_deps`、`an_update_trailer_reuses_every_cut_the_game_did_not_change`、`a_milestone_video_takes_the_groundwork_but_no_shots`。
- 定期発信の投稿は take をそのまま書き出す（静止画は中央のコマ、動画はそのカットの segment）。切り出し・縦への組み直し・文字入れはしない（D-02）。 守るテスト: `a_still_post_is_the_takes_own_frame_and_writes_nothing_else`、`a_post_is_a_take_as_it_is_and_the_next_one_is_due_after_the_cadence`。
- ロケハンのカメラはショットではない。shots.yaml にも take の状態にも入れない（UE では content_root/Scout/ にシーケンスを作るだけ）。 守るテスト: `scouting_photographs_anchors_without_touching_shots`、`a_scout_camera_the_engine_pulled_in_front_of_a_wall_is_recorded_where_it_stood`、`scouting_an_empty_project_brings_the_level_in_one_launch`。 人間が守る: UE の中で content_root/Scout/ にだけシーケンスを作ること。
- ムードボードの画像は雰囲気の参照に限る。生成画像をティザーの画として使わない（実際のゲームと違うものを見せない）。`brief.images` は編集的で、撮り直しを起こさない。 守るテスト: `generated_pictures_never_become_takes_or_part_of_the_cut`、`the_mood_board_is_editorial_and_reaches_the_planner`、`plan_sees_the_mood_board_as_images`。
- 台本（`script:`）と、そこに付く音（`audio.music`、行の `sfx`）は編集的で、fingerprint に入れない。位置は必ず `script::layout` / `sound::spans` から出す（カードを足したり並べ替えたりしても音がずれないように）。 守るテスト: `the_script_puts_cards_between_shots_and_keeps_sound_on_its_shot`、`music_cues_and_card_hits_follow_the_script`、`script_lines_are_typed_in_place_and_never_rerender`、`editing_sound_never_rerenders`。
- **環境音は場所に付ける（D-76、D-83）。** `audio.ambience.<アンカー>` に場所ごとに1つ書き、コード（`sound::heard`・`sound::plan`）が、その場所を撮るカット（カメラの `anchor` か `look_at`）の下に敷き、場所が変わる境目で切り替える。位置は `script::layout` から出す（並べ替えても音は場所に付いてくる）。カードをはさんで場所が変わるときは次の場所の音を `sound::AMBIENCE_LEAD`（0.3 秒）先に入れ、同じ場所の中のカードでは続ける。場所の無いカットは前の場所の音を続け、ショットの `ambience: none` はそのカットを無音にし、アンカーの名前はその場所の音を借りる。短い音はループして区間を埋め、最後のカットで止まる（タイトルとエンドカードは曲に任せる）。環境音は `audio.music` に入れない（音の設計の決まりごと）。どれも編集的で fingerprint に入れず、撮り直しは起きない。 守るテスト: `the_air_of_a_place_follows_the_shots_that_film_it`、`a_shot_of_no_place_keeps_the_air_before_it_and_reordering_moves_the_air`、`editing_the_air_never_rerenders`、`a_short_air_loops_to_fill_its_stretch`、`sound_design_picks_from_the_library_and_lands_on_the_pieces`、`any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`。
- `appear`（prop の出現）は、使ったショットの engine payload にだけ入れる。空のときにキーを足さない（足すと全ショットの fingerprint が変わる）。 守るテスト: `a_prop_popping_in_rerenders_only_its_shot`、`fingerprints_match_the_python_version`。
- アンカーの大きさ（`{at, size, center}`）は previz の fingerprint にだけ入れる。エンジンの take を無効にしない。 守るテスト: `anchors_take_a_size_and_previz_alone_follows_it`。
- plan / revise のプロンプトに、自己検証・チェックリスト・自己採点の語を書かない（D-22、`craft::SELF_REVIEW_WORDS` のテストが守る）。品質は生成前の仕様（`craft.rs`）で上げる。 守るテスト: `every_path_that_writes_shots_gets_the_frozen_spec`、`prompts_never_ask_claude_to_inspect_its_own_output`、`references_never_ask_claude_to_inspect_its_own_output`、`the_prompt_never_asks_the_model_to_look_back_at_itself`。
- **Claude に話す場所は左の欄の1つにする（D-62、D-82 で変更）。** 左の欄ではいつでも書いて送れる。チップは打たずに済ませる近道で、カット表の行・ムードボード・ホームの雰囲気ではチップを積んで送り、積んだチップをつないだ文がそのまま指示になる。予測のチップはプロでだけ出す（シンプルでは予測を呼ばない）。チップは定番（コード）と Claude の予測（`suggest.rs`、`/api/suggest`）で、予測は読むだけで何も書かず、撮り直しも起きない。積んだ指示は手で書いた指示と同じ経路（revise・ムードボード・ピッチ）に渡り、範囲はそちらのコードで強制する。予測の呼び出しは effort `low` のまま（`WithEffort` が上書きしない）で、ロケハンの記述（`stage:`、そのカットのアンカーの分）を根拠に渡し、自己検証の語を書かない（テストが守る）。候補に無いことは「✎ 書き足す」で書けるが、例外として扱う。人間が書く事実（タイトル、エンドカード、投稿文）と数値の調整は文字入力のまま。 守るテスト: `chips_are_suggested_over_http_and_nothing_is_written`、`effort_is_saved_and_applied_to_every_request`、`the_prompt_never_asks_the_model_to_look_back_at_itself`、`chips_are_tidied_and_never_repeat_the_stack`。 人間が守る: 画面で、左の欄のほかに Claude への自由入力の欄を出さないこと。シンプルでは、右の画面に常に出ている入力欄を置かないこと（打つ場面はその場で開く）。
- **任せる場所は左下の1か所にする（D-84）。** Claude やエンジンに仕事を頼むボタン（▶ 続ける、ロケハン・ステージ・構成画・ムードボード・棚卸し・撮り直し・Cut）は、左の欄の入力の上（ドック）にだけ置く。右のキャンバスは人の手（見る・選ぶ・動かす・OK）だけにする。▶ は道の 📍 まで一気通しを進め、📍 での停止（produce の `stop`）は、その部分が今回走ったときだけ止まり、押し直すと済んだ部分を飛ばして次へ進む。📍 と場所の OK の印は閲覧者のもの（ブラウザの保存領域）で、shots.yaml にも state.json にも入れない。 守るテスト: `a_pass_stops_where_the_human_pinned_and_goes_on_past_what_is_done`、`the_first_pass_stops_at_the_review_and_resumes_after_it`。 人間が守る: 右のキャンバスに Claude やエンジンを走らせるボタンを置かないこと。キャンバスの各段は1画面に収め、縦にスクロールしないと見えない情報を置かないこと（長い一覧だけは枠の中でスクロールしてよい）。
- **構成の段には、カット表と切り替えられるキャンバスを置く（D-63）。** カットとカードを流れの順に並べ、上にロケハンの場所、下に雰囲気のノードを置いて線でつなぐ。つなぐのは「積む」と同じで、雰囲気はそのカットの「このカット」、場所は「カメラだけ」のスタックにチップとして積まれ、送るまで何も変わらない。1つの雰囲気をつないだ複数カットへまとめて送るときは `/api/revise` の `shots`（カットの一覧）で送り、範囲はコードで強制する（つないでいないカットは変わらない）。盤面の配置・雰囲気のノード・線・表示位置は閲覧者のもの（ブラウザの保存領域）で、shots.yaml にも state.json にも入れない。 守るテスト: `a_mood_wired_to_some_cuts_changes_only_those_cuts`、`chat_scope_is_enforced_over_http`。 人間が守る: 盤面でつなぐ・積む操作が、送るまで何も変えないこと（画面の JS）。
- 仕様は、ショットを書かせる全経路に同じものを届ける（D-48）。revise は `craft::edit_spec` を通し、画が変わりうる指示（自由な指示・カメラ・光）には plan と同じ部品（ブリーフ・方向・ルック・スキル・規則・基準値・文字の書き方・ステージ・距離表と、そのカットのロケハン写真）、文字の指示にはルックと基準値と文字の書き方とタイトル用の書体の一覧とステージ（カメラ効果とタイトルの書体は場所から決まるため）、音の指示と一気に作るときの音の設計（`sounddesign.rs`）にはブリーフと方向とルックの音の性格を渡す。文字の書き方は `references/craft/TEASER-TYPE.md` の 10 節だけを `refs::type_text` で、動きの書き方は `references/motion/MOTION.md` の 6 節と、つなぎの書き方は `references/motion/TRANSITIONS.md` の 8 節だけを `refs::motion_text` で、参照のティザーの測った回数と置き場所は `reference::planner_text` で渡し、書体・大きさ・動きの秒数はコードが決める（D-52、D-53）。どの経路に何が届くかは `every_path_that_writes_shots_gets_the_frozen_spec` が固定している。経路を足したら、この表にも足す。Claude を呼ぶ関数は `tests/ai.rs` の `CLAUDE_CALLS` に全部載せ、表の行の名前か、仕様を渡さない理由を書く（`every_call_to_claude_is_in_the_spec_table_or_says_why_not` がソースから拾って照らし合わせる）。 守るテスト: `every_path_that_writes_shots_gets_the_frozen_spec`、`every_call_to_claude_is_in_the_spec_table_or_says_why_not`。
- **音は作品ごとに変える（D-64）。** 音の設計（`sounddesign.rs`）の決まりごとは汎用のものだけにし、ジャンルの語を書かない（ジャンルとルックの音は DESIGN.md 7 節とブリーフから渡す。`the_designers_own_rules_belong_to_no_genre` が守る）。ライブラリは取り込んだプロジェクトを `sounds/usage.json` に覚え、ほかのゲームのティザーが今も使う音に印を付けて一覧の後ろに回す（自分と、発信計画で同じ計画につながる同じゲームの動画は数えない）。Claude に渡す一覧は `Library::prompt_list_for` の1か所で作り、切るときはパックを順に回して、切った件数を注記に出す。ほかのティザーと重なった音は注記とアプリの音源ライブラリで人間に見せる。 守るテスト: `a_sound_note_marks_library_sounds_another_teaser_already_uses`、`the_designers_own_rules_belong_to_no_genre`、`the_next_teaser_is_told_which_sounds_another_game_took`、`the_library_remembers_which_teasers_use_a_sound_and_lists_the_worn_ones_last`。
- 文字の読み時間は `script::reading_secs` の1か所に置く（D-49）。plan と文字の修正で書く行はコードで読める長さに延ばし、人間が書いた行は書き換えずに `text.hold` で示す。 守るテスト: `revised_card_words_are_held_for_their_reading_time`、`reading_time_is_computed_in_one_place`、`text_stays_long_enough_to_read`。
- 撮影後の確認は、produce の初回のエンジンでの撮影の直後に1回だけ（D-50。D-22 の唯一の例外で、open-design の動画の例外に倣う）。聞くのは「被写体が写っているか、何が遮っているか」という事実だけで、点数や好みは聞かない。写っていないカットだけカメラを1回直して撮り直し、そのカットだけを見直す。2周目はしない。結果は state.json に見た take と一緒に残し、そのカットを撮り直すと消える。previz と普段の render では走らせない。 守るテスト: `after_the_first_shoot_only_a_shot_whose_subject_is_hidden_is_fixed_once`、`previz_and_clear_frames_need_no_repair`。
- 企画（`ai::discover`、D-45）は open-design の OD Next の計画ターンを写す。質問は欠けがあるときだけ（5問まで、推奨の既定値つき）、そうでなければ方向を1文で言い切った企画を書き、承認を待たずに制作へ進む。人間の確認は previz の段（D-38）。ルックとスキルの id はコードで実在を確かめ、知らない id は捨てる。発売日・機種などの事実は作らせない。 守るテスト: `the_planning_turn_is_told_never_to_invent_facts`、`a_question_comes_back_as_a_form_and_writes_nothing`、`without_questions_it_always_plans`、`the_plan_keeps_only_known_ids_and_lands_in_the_brief`。
- ルック（`references/design-systems/`）とスキル（`references/skills/`）も plan に全文が渡るので、D-22 の語を書かない（`cargo test --test planning` が守る）。ルックは汎用に限り、実在のゲームやスタジオの名前・ロゴ・固有の色を写さない。ルックの tokens（色調・帯・書体）は Claude の提案より優先してコードで書く。 守るテスト: `the_catalogues_load_and_read_cleanly`、`the_look_sets_the_finish`、`the_plan_moves_as_the_look_says_unless_the_brief_asks_otherwise`。 人間が守る: ルックとスキルに実在のゲーム・スタジオの名前・ロゴ・固有の色を写さないこと。
- 流行りの型（`refs::STYLES`、D-39）は実在ティザーの構成とリズムだけを真似させる。話の筋・キャラクター・文言は真似させない。分析文（TEASER.md の 1〜7 節）も plan に渡るので、D-22 の語を書かない（`cargo test --test styles` が守る）。 守るテスト: `a_style_is_followed_for_structure_never_for_story`、`four_recent_styles_per_genre_each_analysed`、`picking_a_style_is_editorial_and_reaches_the_plan`、`a_style_sets_the_pace_of_the_skeleton`。 人間が守る: 分析文に実在ティザーの台詞やキャラクター名を書かないこと。
- **骨格は実測の型から作る（D-68）。** ビートの持ち時間とビートごとのカット数は、型の分析（TEASER.md 2 節の秒数）と実測のカット時刻から取る（`refs::beats`。型が無ければジャンルの4本の平均、ジャンルも無ければ24本の平均）。最初のカットは型の最初のカット、最後は黒味と title カードの時間を残す（`craft::card_reserve`）。規則は崩さない: hook は `hook.long` の内、1カットは 1〜8 秒、`craft::MAX_SHOTS` まで。型が escalate を速く切るなら、丸めで逆転させない。推定の規則のうち `hook.long`（ティザーは 10 秒、分析した型の hook の上位 4 分の 1）と `tempo.escalate`（選んだ型が escalate を遅く切るなら指摘しない）は、分析した型で校正した。型の分析を足したら 2 節の各ビートに秒数を書く（`every_style_names_where_its_beats_fall` が落とす）。 守るテスト: `a_style_sets_the_pace_of_the_skeleton`、`every_style_names_where_its_beats_fall`、`skeleton_follows_the_craft`、`the_inferred_tempo_rules_are_calibrated_on_the_analysed_styles`。
- **plan の品質はコードで測り、コードで選ぶ（D-68）。** 評価（`eval.rs`）は plan の出力を実測の型と比べるコードの採点だけで、モデルに採点や見直しをさせない。`SHOTPILOT_CANDIDATES` の複数案は同じ仕様で別々に書かせ、コードの点数で1つを残す（モデルに他の案も点数も見せない）。採点し直せるように、残した案の応答をそのまま保存する。 守るテスト: `several_plans_are_written_and_code_keeps_the_best_one`、`the_score_compares_the_beats_and_the_last_piece_as_the_references_were_measured`、`a_noschema_model_gets_the_schema_as_text_and_the_same_model_otherwise`、`run_writes_results_replies_and_a_summary_and_never_touches_the_cases`。
- **弱いモデルの返答はコードで直し、直したことを人間に見せる（D-68）。** JSON の取り出し、語彙のゆれの正規化、文字の空の台本の行を外すことはコードで行い、`Planned::adjustments` に残す。契約に合わない返答（知らないアンカー、id の重複、壊れた JSON）だけを、事実を書いて2回まで頼み直す（自己検証の語は書かない）。`SHOTPILOT_REPAIR=off` で直さない動きに戻る。 守るテスト: `an_unknown_anchor_is_asked_again_once_with_the_facts`、`with_repair_off_every_bad_reply_fails_on_the_first_call`、`a_script_line_with_no_words_is_left_out_and_the_plan_still_writes`、`vocabulary_wobble_is_normalised_and_noted`。
- `references/craft/` と `references/genres/` は実測から生成し、バイナリに埋め込む（D-29）。表は `aggregate_refs.py` が書き、auto マーカーの外の本文だけを手で書く。本文も plan に渡るので、D-22 の語を書かない。 守るテスト: `references_never_ask_claude_to_inspect_its_own_output`、`brief_genre_picks_a_measured_reference`、`four_recent_styles_per_genre_each_analysed`。 人間が守る: 表は aggregate_refs.py が書き、auto マーカーの外だけを手で書くこと。
- ショット検査の重さは根拠で決める。出典のある規則だけを P0 にし（承認を止めるのは P0 だけ）、推定の閾値は P2 に留める（実測で校正するまで）。例外は最初から P1 の推定の規則 3 つ（`runtime.target`・`hook.long`・`camera.inside`）で、推定の規則を P1 に足すときはテストの一覧にも足す（D-65）。 守るテスト: `every_rule_has_a_known_basis_and_a_unique_id`、`inferred_rules_are_p2_except_the_three_named_at_p1`、`only_sourced_rules_are_p0`、`opening_on_the_title_is_p0_and_blocks_approval`。
- `build` は、Sequencer 側で人間が加えた変更を黙って上書きしない（`--force` のときだけ上書きする）。 守るテスト: `build_overwrites_editor_edits_only_with_force`、`build_refuses_to_clobber_editor_edits`、`render_blocked_when_diverged`。
- 画面の文言は日本語で書き、英語は `web/i18n-en.json` の辞書で出す（D-51）。描画コードに英語の分岐を書かない。文言を足したり変えたりしたら `node tools/i18n_extract.js` で辞書に無いものを出し、英訳を足す（`{0}` などの穴は日本語と同じ組にする。`the_pages_switch_to_english_with_a_dictionary` が守る）。ユーザーや Claude が書いた文は訳さない（辞書に無い文はそのまま残り、穴に入る自由な文は `i18n-free.json` が示す）。DOM の文字を読んで処理を分けるコードを書かない（英語モードでは中身が変わる）。 守るテスト: `every_ui_string_has_an_english_entry`、`the_pages_switch_to_english_with_a_dictionary`。
- **書体は目録（`references/fonts/catalog.json`）にあるものだけを取ってくる（D-52）。** どれも OFL（Kosugi と Kosugi Maru は Apache 2.0）で、`<config>/shotpilot/fonts`（`$SHOTPILOT_FONTS`）に置いて焼き込む。Windows などのシステム書体は読むだけで、写したり同梱したりしない。取得は CLI とアプリが `fonts::allow_fetch` で許したときだけで、ライブラリとテストはネットに出ない。書体に無い字を含む行は、その行だけシステム書体で描き、Cut の注記に出す。ルックの書体は目録のうち定番すぎるもの（`overused`）を使わず、カードと字幕の書体は漢字を持つものにする（`every_look_names_catalogue_faces_that_are_not_overused` が守る）。書体・タイトルの大きさ・ロゴ（`finish.fonts` / `title_size` / `logo`）は編集的で、撮り直しは起きない。タイトルだけの書体（`finish.fonts.title`、D-59）は、Claude が plan と全体への仕上げの指示で目録の一覧（`fonts::title_text`、ルックの雰囲気と崩れ・かすれ）から選び、コードが目録・定番すぎないこと・タイトルの字を持つことを確かめる（`fonts::title_spec`）。タイトルの書体に無い字は表現用の書体で組む。 守るテスト: `a_family_outside_the_catalogue_is_refused`、`an_uncached_face_is_unavailable_without_allow_fetch`、`every_look_names_catalogue_faces_that_are_not_overused`、`the_planner_may_give_the_title_a_worn_face_from_the_catalogue`。
- **動きは語彙から選ばせ、秒数はコードが決める（D-53）。** 文字の入り（`motion::ENTRANCES`）・保ち方・抜けとつなぎ（`finish::TRANSITIONS`）は閉じた語彙で、ルックの `motion`（`references/motion/profiles.json` のジャンルの既定と上書き）が `finish.motion` に入る。Claude が書けるのは、台本の行の `motion`（入り方だけ）と、plan と全体への指示での `energy`・`variant`・`avoid` だけ。どのつなぎもカットの尺を変えない（クロスフェードは入れない）。入りは行の表示時間の 35%、抜けは 25% までに収める。1 秒に 3 回を超えるフラッシュ（flash・whiteout のつなぎ、flicker の入り）はカット検査の P0（`flash.rate`、WCAG 2.3.1）。`finish.motion` が無いときは `base` で、動きを足す前と同じ式になる。どれも編集的で、撮り直しは起きない。 守るテスト: `the_transition_vocabulary_has_no_crossfade`、`every_profile_speaks_the_closed_vocabulary`、`no_entrance_or_exit_outruns_its_share_of_the_line`、`too_many_flashes_in_a_second_stop_approval`。
- **カメラ効果は語彙から選ばせ、秒数と強さはコードが決める（D-57）。** 語彙は `fx::FX`（chroma・glitch・flicker・static・vhs・shake・redflash）の7つで、各カットの `fx:` に名前か `{type, at}` で書き、Cut で文字の下に焼き込む。fingerprint に入れず、撮り直しは起きない。どの効果もカットの尺を変えない。光る効果（flicker は2回、static と redflash は1回）は `flash.rate` に数える。`finish.motion.avoid` は効果も外す（自分の名前、flash で static と redflash、glitch で static）。外した効果は Cut の注記に出す。Claude が書けるのは plan、文字・つなぎの指示（`ai::ASPECTS` の finish）、範囲を絞らない自由な指示で、カメラだけ・光だけ・音だけの指示では書けない（D-65）。何に使うかの手引き（`fx::guide`）は `refs::motion_text` に付けて全経路に渡す。 守るテスト: `a_free_note_on_a_cut_may_set_its_camera_effects`、`only_plan_and_the_finish_note_may_set_camera_effects`、`camera_effects_are_the_seven_named_ones`、`camera_effects_are_burned_in_at_the_cut_and_never_rerender`。
- **つなぎはレシピから描き、入れ替えは覆った瞬間のカットの境目で行う（D-70）。** 描き方は `references/motion/transitions.json`（出ていくカットの終わりと入るカットの始めに分けた、覆いの色・長さ・緩急・露出）にだけ置き、`handover.rs` が ffmpeg のフィルタにする。`finish.rs` に描き方を直書きしない。2 つの画を重ねるつなぎ（クロスフェード、プッシュなど）は足さない。長さは `profiles.json` の `dip`・`flash`・`whiteout`・`glitch` と勢いで決まる。`finish.motion` が無いときは、レシピ以前と同じ `fade` の式で描く（`the_base_profile_draws_the_hand_overs_it_drew_before` が守る）。plan が書いたつなぎは、コードで `cut` のほか 2 種（`dip` とルックの差し色を先に）に整え、人間や修正で 3 種以上になったらカット検査の P2（`transition.mix`）で示す。つなぎの語を足すときは、`finish::TRANSITIONS` と `transitions.json` と `TRANSITIONS.md` を一緒に直す（`every_recipe_is_a_hand_over_with_known_parts` が守る）。どれも編集的で、撮り直しは起きない。 守るテスト: `the_base_profile_draws_the_hand_overs_it_drew_before`、`every_recipe_is_a_hand_over_with_known_parts`、`a_plan_hands_over_with_cut_and_two_others`、`more_than_two_kinds_of_hand_over_are_pointed_out`。
- **参照のティザーから写すのはタイミングだけ（D-71）。** 測る物差しはコーパスと同じにする（カットは D-29 の場面転換の山、明るさは D-53 の 12fps・幅 320px としきい値）。しきい値を変えると、コーパスの数値とも、前に測った参照とも比べられなくなる。写すのはつなぎの長さ（`finish.motion` の `dip`・`flash`・`whiteout`・`accent`・`card`。秒は測ったままで、勢いで伸び縮みさせない）と、回数と置き場所の割合（plan と画・文字の修正への節）だけ。参照の画・文字・音は Cut にも plan にも入れない。`reference/<id>/` の縮小版とコマは比べる用で、人が見るだけにする。比べた結果を Claude に読ませない（D-22）。測った長さはルックより強く、plan とルックの適用でも上に重ねる（open-design の「測ったもの > 人の言葉 > 好み」）。避けた動き（`avoid`）は人の言葉なので残す。保ちが 1.2 秒を超える白はつなぎではなく白い画として数えない。参照を切り替えると、前の参照が書いたままの値は取り除く（人が直した値は残す）。参照の縮小版とコマは新しいプロジェクトの `.gitignore` で除外する（測った値の `facts.json` は残す）。どれも編集的で、撮り直しは起きない。 守るテスト: `a_video_is_measured_and_a_cut_compared_with_it`、`a_measured_reference_lends_its_lengths_and_nothing_re_renders`、`a_plan_keeps_what_the_reference_measured`、`switching_references_leaves_nothing_of_the_old_one`、`a_reference_teaser_is_borrowed_from_the_design_panel`。
- **Claude Code のフック（D-66）は知らせるだけにする。** 撮影せず、shots.yaml も state.json も take も書き換えない（覚えておくのは `.shotpilot/claude_hook.json` だけ）。何も古くなければ何も出さず、同じ変化は1回だけ言う。どんな失敗でも終了コード 0 で黙る（ゲーム開発のセッションを止めない）。取り付けは、既存の `.claude/settings.json` のキー・順序・ほかのフックを保ち、読めない JSON は書き換えずに止める。 守るテスト: `the_hook_speaks_claude_codes_json_and_never_fails_the_session`、`nothing_out_of_date_means_no_words`、`a_game_change_names_only_the_cuts_that_showed_it`、`an_edit_to_shots_yaml_says_what_it_means_for_rerendering`、`install_adds_the_skill_and_hooks_and_keeps_the_rest`、`install_refuses_to_overwrite_settings_it_cannot_read`。
- **Three.js の体験版（D-72）は、UE と同じループを同じ判定で回す。** ゲームは `project.game`（既定 `game/`）の JSON で、物ごとに1ファイル（`objects/<名前>.json`）にする。写っていた物（視野と、ほかの固い物に遮られないか。D-56）とロケハンの壁よけ（D-55、UE の `clear_view` と同じ規則）は Rust で決め、ブラウザは描くだけにする。take の `deps` は `scene.json` と写っていた物のファイルで、物を1つ直すとそれが写っていたカットだけが古くなり、戻せば前の take を使う。描画のページ（`three/render.html`）はショットの時刻 `t` だけで動かし、時計や `Math.random` を使わない。描き方を変えたら `three::RENDERER_VERSION` を上げる（three の take の指紋にだけ入り、UE と mock の take は動かない）。テストからブラウザを起動しない（`three::use_stand_in_browser`）。 守るテスト: `the_three_sample_films_without_ue_and_redraws_only_the_shot_that_changed`、`editing_one_thing_in_the_three_game_redraws_only_the_shots_that_showed_it`、`a_wall_hides_what_is_behind_it_from_the_three_takes`、`a_scout_camera_behind_a_three_wall_stands_in_front_of_it`、`the_three_game_offers_its_anchors_places_and_props`、`three_takes_carry_their_own_renderer_version`、`a_three_project_without_its_game_says_so_and_previz_still_works`、`the_home_makes_a_three_project_with_the_ward_to_film`。 人間が守る: render.html の描き方を変えたときに `three::RENDERER_VERSION` を上げること（画素はテストで見ない）。
- ユーザー向けのエラーは `Error::user`（`bail!`）で返す。CLI はメッセージだけを出す。`unwrap` で落とさない。 守るテスト: `user_errors_are_400_with_message`、`unknown_kinds_and_amounts_and_the_ends_are_user_errors`。 人間が守る: CLI でメッセージだけを出し、本体に unwrap を増やさないこと。
- 座標系は UE 準拠（cm、Z が上、X が前、Y が右、左手系、回転は度）。`geometry.rs` は依存なしのまま保つ。 守るテスト: `yaw_and_pitch_follow_the_ue_directions`、`geometry_uses_nothing_but_std_and_serde_json`、`basis_is_ue_convention`、`project_center_and_sides`。

## テスト

- ループの挙動を変えたら、**どのショットが再レンダ・再ビルドされ、どれがされなかったか**をテストで確かめる。`tests/common/mod.rs` の `launches(&ws)` で、mock エンジンの起動記録を読める。
- `only_changed_shot_rerenders` と `order_purpose_notes_never_invalidate` はデモの主張そのものにあたる。落としたまま先に進まない。
- テストから実際の UE や Claude API は呼ばない。UE は mock エンジンで、Claude は差し替えで代用する。
- 不変条件の各項目は、末尾に「守るテスト:」でそれを守るテストの名前を書き、コードで確かめられない部分は「人間が守る:」で書き分ける（D-65）。項目を足したり変えたりしたら、テストを足して名前を書く。名前が無い項目や実在しないテストの名前があると `every_invariant_names_the_tests_that_keep_it` が落ちる。
- D-13 のループは `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed`（tests/model.rs）が、編集・元に戻す・撮影・ゲームの変化・承認・開き直しのランダムな列で確かめる。期待値はテストの側で「何が画を変えるか」だけから出す（実装の指紋の計算は使わない）。落ちたら種と手順が出るので、`SHOTPILOT_MODEL_SEED=<n> cargo test --test model` で再現する。画を変える項目や編集的な項目を足したら、このテストの操作にも足す。

## スコープ

- やる: UE5 のティザー映像1本。ティザーの型（`hook → establish → escalate → title`）を同梱する。1ビートに3バリアントを出すのは余力があればやる。
- やる: デスクトップアプリ（`shotpilot-app`、Tauri）。ブラウザは使わない。中身はコアを組み込んだ loopback サーバーと、ビルド不要の HTML 1枚。
- やる: BGM と SE（D-19）。`shots.yaml` に書き、cut のときにミックスして載せる。映像は撮り直さない。検証は earshot 式のサウンドチェック。
- やる: 仕上げ（D-25）。帯・タイトル・エンドカード・字幕・カットのつなぎ（黒/白をはさむ）を shots.yaml に書き、Cut で焼き込む。撮り直しは起きない。
- やる: Three.js のサンプルゲームで撮る体験版（D-72）。審査員や UE を持たない人が、同じループを手元で試すためのもの。自分の Three.js のゲームを撮る対応はまだしない。デモ動画は UE で撮る。
- やる: 公開後の発信計画（D-47）。ティザーのあとの動画（発表・ゲームプレイ・デモ／フェス・ローンチ・アップデート・セール）をチェックシートで並べ、それぞれを隣の別プロジェクトとして起こす。アップデート連動と定期発信の候補は、ゲームの変化の検知（D-37）から出す。1プロジェクトが作る映像は1本のまま。
- やらない: Unity / Godot / Roblox 対応、エンジン抽象化レイヤ、TikTok / Shorts 向け短尺（定期発信も横長の take のまま）、複数尺への展開、Electron、Web 公開する UI、GUI での録画後の自由な映像編集（トリム、エフェクト、クロスフェード）。
- mock エンジンはテストとデモ用で、別エンジン対応の足場ではない。抽象化を先回りして作らない。
- 録画後の映像編集は作らない（差別化にならない）。差別化できるのは録画の前だけ。音（D-19）と仕上げ（D-25）は例外で、定義の一部として扱い Cut で適用する。

## スタイル

- Rust（edition 2021）。Python は `ue/sp_entry.py` だけに留める（Unreal 同梱の Python で動かす必要があるため）。例外は `tools/ue_sample/`（入口の `build_graybox.py` と、組み立ての `graybox/` パッケージ）で、サンプルのゲーム（D-54）のマップをエディタの中で組む道具。製品には入らず、バイナリにも埋め込まない。
- コード中のコメントと doc コメントは英語。ユーザー向けの CLI 出力は英語でもよい。サンプルの `shots.yaml` の中身は日本語でよい。
- 依存クレートを増やさない（現状: serde_json, yaml-rust2, sha2, miniz_oxide, chrono, ureq, clap。アプリは tauri, rfd）。three の描画は同梱の Three.js（`three/three.module.min.js`）と、PC にある Edge か Chrome で行う。
- 画面のスクロールバーと範囲スライダーは、各ページ冒頭の共通 CSS（`::-webkit-scrollbar` と `::-webkit-slider-*`、色は `--scroll` / `--scroll-hover`、スライダーのつまみの色は `--range`）で描く。OS 既定のバー（矢印・白いトラック）を出さない。`scrollbar-color` と `scrollbar-width` は使わない（Chromium ではその要素だけ OS のバーに戻る）。ページを足したら同じ CSS を入れる（`every_page_draws_its_own_scrollbars` が守る）。
- コミットは小さく分ける。push は人間の確認を取ってから行う。
