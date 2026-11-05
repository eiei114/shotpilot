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
shotpilot status                  # fresh / stale / out-of-sync の一覧
shotpilot render previz           # 古くなったショットだけをエンジンなしで描く
shotpilot set s03 camera.lens.focal_length=50
shotpilot check                   # ショット検査（P0 は承認を止める）
shotpilot approve s03             # 構図を承認（画が変わると外れる）
shotpilot approve --script        # 台本を承認（文字・並び・各カットの目的と秒数が変わると外れる。カメラでは外れない）
shotpilot brief genre=ホラー       # brief: を見る・書く
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
shotpilot promo                   # 発信計画（D-47）: 公開後の動画ごとのチェックシートと、定期発信の予定
shotpilot promo update v1.1       # このカットを引き継いだアップデートトレーラーを隣に作る（ゲームが変わったカットだけ撮り直し）
shotpilot promo post s03 [--clip] # take をそのまま静止画か1カットの動画で promo/posts/ に書き出す
shotpilot-app [path/to/shots.yaml]   # アプリ。引数がなければホーム（最近のプロジェクト・新規作成）を開く
node tools/i18n_extract.js        # 英語モードの辞書に無い画面の文言を一覧する（D-51）
```

`cut` の動画書き出しは ffmpeg を使う（無ければ storyboard と spec だけを出す）。`plan` と `revise` は、ログイン済みの Claude Code（`claude` コマンド）か API キー（アプリの設定、または `ANTHROPIC_API_KEY`）で Claude を呼ぶ（モデルは `SHOTPILOT_MODEL` で上書きできる。effort は `low`〜`max` をアプリの接続設定か `SHOTPILOT_EFFORT` で変えられ、既定は `medium`。Claude Code 経由でも API 経由でも同じ値が効く）。

## 構成

```
crates/shotpilot/            コア（ライブラリ + CLI）
  src/model.rs        shots.yaml のスキーマ・検証・resolve（Definition → ResolvedShot）
  src/fingerprint.rs  再レンダ判定。品質プリセット（previz/still/preview/final）
  src/state.rs        .shotpilot/state.json と render/<shot>/<quality>/<fp12>/ の take 管理
  src/content.rs      take が写していたゲームの中身（エンジンが報告したパッケージのファイルと SHA-256）。変わったカットだけを古いとする
  src/workspace.rs    上の3つとエンジンを束ねる。全操作はここを通る
  src/engines.rs      mock / ue5 バックエンド。共通の job/result JSON プロトコル
  src/sync.rs         Sequencer → shots.yaml の逆同期（base / engine / ours の三方比較）
  src/scout.rs        工程0「ロケハン」: アセットの棚卸し、props への追加、アンカーのロケハン写真
  src/moodboard.rs    ブリーフのムードボード（参照画像の保存、OpenAI での生成、plan に渡す画像ブロック）
  src/storyboard.rs   絵コンテ（D-33）。previz の線画を下絵に、ロケハン写真とムードボードを参照して画像生成で描く確認用の1枚
  src/produce.rs      初回の一気通し（D-28）。足りない下ごしらえを埋め、plan を1回、最安の品質で撮って仕上げ込みで Cut
  src/craft.rs        plan に書く前に渡す仕様（ブリーフ、骨格、規則、画角の距離表）とコードによる整形
  src/designsys.rs    ルック（D-45）。references/design-systems/<id>/ の DESIGN.md・tokens・manifest を埋め込み、plan に全文を渡し、tokens で仕上げを決める
  src/concept.rs      構成画（D-46）。企画のビートごとの画（concept/）と、ロケハンで見つけた場所の記録。plan に画と場所の写真を渡す
  src/skills.rs       スキル（D-45）。references/skills/<id>/SKILL.md を埋め込み、企画で選ばれたものの本文を plan に渡す
  src/refs.rs         実測リファレンス（references/craft/ と genres/<id>/、流行りの型 20 本を埋め込み）。plan に渡す文と、骨格の hook・title の長さと密度の形
  src/shotcheck.rs    ショット検査。トレーラーの規則でショットリストを lint する（P0 出典あり・承認を止める / P1 / P2 推定）
  src/previz.rs       エンジンを使わないワイヤーフレーム描画（PNG 書き出し）
  src/cut.rs          take をショット順に連結してティザーにする（ミックスがあれば音を載せ、仕上げがあれば焼き込む）
  src/finish.rs       仕上げ: 帯・色調・台本のカードと重ね文字・タイトル・つなぎ（黒/白）を ffmpeg のフィルタにする
  src/script.rs       台本（D-31）: ショットの間に入るカードと、ショットに重なる文字。Cut の並び（layout）を一か所で計算する
  src/sounddesign.rs  音の設計: Claude がライブラリから区間ごとの曲とカード・ショットの音を選び、取り込んで書き込む
  src/sound.rs        BGM と SE（`audio:` と各ショットの `sfx:`）の検証・配置・ミックス。cut/sound/ にキャッシュする
  src/soundcheck.rs   ミックスの検証。earshot の critique のうち、モデルを使わない部分の移植（MIT）
  src/library.rs      音源ライブラリ（プロジェクト外、D-26）の検索と、プロジェクトの audio/lib/ への取り込み
  src/ai.rs           Claude によるショットリスト作成（plan）と部分修正（revise）。Messages API を直接呼ぶ
  src/connect.rs      Claude への接続。既定はログイン済みの Claude Code（`claude -p`）、ほかに API キー
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

- **fingerprint にはピクセルを変えるものだけを入れる。** 並び順、`purpose`、メモ、音（`audio`、`sfx`）、`brief:`、仕上げ（`finish:`、`caption`、`transition`）は入れない。仕上げは take のピクセルではなく Cut の出力を変える。並べ替えやメモ、音の修正で再レンダが起きてはいけない。
- エンジン側スクリプトが、同じ入力に対して違う出力を出すように変わったら `fingerprint::RENDERER_VERSION` を上げる。previz の描画を変えたら `PREVIZ_VERSION` を上げる（エンジンの take は無効にしない）。
- fingerprint は Python 版と同じ正規化 JSON（キー整列・区切り `,` `:`）の SHA-256。`fingerprints_match_the_python_version` を落とさない（落ちると UE で撮った take が全部撮り直しになる）。
- **take は fingerprint で管理し、上書きしない。** 定義を元に戻したら、古い take を再利用する。`render/` 以下を手で編集しない。
- **ゲームの中身の変化は fingerprint に入れず、take の `deps` で見る（D-37）。** エンジンはレンダ中に写っていたもののパッケージを報告し、take はそのファイルの大きさ・時刻・SHA-256 を持つ。中身が変わったカットだけが古くなり、撮り直した take は `<fp12>~N/` に置く（前の take は消さない。中身を戻せば前の take が使われる）。fingerprint は定義だけのまま（Python 版との互換と、撮った take を守るため）。迷ったら「写っている」側に倒す（撮り直しすぎる方が、見落とすより安全）。
- `shots.yaml` を機械的に書き換えるときは、必ず `yamlio::Doc` を通す（変えるショットの行だけを差し替え、再パースして意図どおりか確かめてから書く）。人間が書いたコメントとキー順を消さない。
- Claude（`ai.rs`）が返した編集は、`shots.yaml` に書く前に resolve して検証する。編集してよいショットの範囲は、プロンプトではなくコードで強制する。音だけの指示（`Pointer::sound`）では `sfx` と `audio` だけを取り込む。カメラだけ・光だけの指示（`Pointer::fields`、`ai::ASPECTS`）では、そのキーだけを取り込む。文字・つなぎだけの指示（`finish`）では、ショットの `caption`・`transition`・`exposure` と、`Pointer::texts` に挙げた台本の行の文字・種類・タイミング（`ai::LINE_TEXT_FIELDS`）だけを取り込み、行の置き場所（`after` / `on`）は変えさせない。`finish:` は全体への指示（`Pointer::finish`）でだけ、色調・帯・書体（`ai::FINISH_FIELDS`）に限って変えさせる（タイトルとエンドカードは人間が書く）。ショットに絞った指示では BGM を変えさせない。
- 定義の編集は `Workspace::write_definition` の1か所を通し、変えた部分の前後を履歴（`undo`）に残す。元に戻すは、その部分が後から変わっていたら上書きせずに止める。戻した画は fingerprint が同じなので、古い take をそのまま使う（撮り直さない）。
- 台本の承認は `state.json` に script fingerprint（台本の文字と位置、各カットの id・beat・purpose・duration と並び）で残す（D-38）。カメラ・光・音では外れない。
- 構図の承認は `state.json` に look fingerprint（品質に依らない画の指紋）で残す。画が変わる編集で外れ、編集的な変更では外れない。P0 の指摘があるショットは承認させない。
- 絵コンテ（`storyboard/`）は確認用で、take ではない。`render/` にも state の take にも入れず、Cut には使わない。look fingerprint を名前にして、画が変わる編集で古くなり、編集的な変更では古くならない。生成するのは人間が頼んだときだけ（produce やレンダでは自動で描かない）。
- 構成画（`concept/`、D-46）はムードボードと同じく参照に限る。take にも Cut にも fingerprint にも入れない。生成するのは人間が頼んだとき（ボタンか `shotpilot concept`）だけで、企画や produce では自動で描かない。構成画と撮った写真の見比べは「本物の写真の中から選ぶ」ことで、Claude が自分の出力を見直すこと（D-22）ではない。選べる写真の id はコードで確かめる。
- 発信計画（`promo.json`、D-47）は編集的で、fingerprint にも shots.yaml にも入れない。派生プロジェクトの `promo.json` は元の計画への `link` だけを持つ（1つのゲームに計画は1つ）。派生で shots.yaml を書くのは新しいプロジェクトの分だけで、元のプロジェクトの定義と take は触らない。take を引き継ぐときは take フォルダをそのまま写し、写した側の take.json の `deps` だけを元のゲームの場所へ向け直す（同じ定義は同じ fingerprint なので撮り直さず、ゲームが変わったカットだけが古くなる）。
- 定期発信の投稿は take をそのまま書き出す（静止画は中央のコマ、動画はそのカットの segment）。切り出し・縦への組み直し・文字入れはしない（D-02）。
- ロケハンのカメラはショットではない。shots.yaml にも take の状態にも入れない（UE では content_root/Scout/ にシーケンスを作るだけ）。
- ムードボードの画像は雰囲気の参照に限る。生成画像をティザーの画として使わない（実際のゲームと違うものを見せない）。`brief.images` は編集的で、撮り直しを起こさない。
- 台本（`script:`）と、そこに付く音（`audio.music`、行の `sfx`）は編集的で、fingerprint に入れない。位置は必ず `script::layout` / `sound::spans` から出す（カードを足したり並べ替えたりしても音がずれないように）。
- `appear`（prop の出現）は、使ったショットの engine payload にだけ入れる。空のときにキーを足さない（足すと全ショットの fingerprint が変わる）。
- アンカーの大きさ（`{at, size, center}`）は previz の fingerprint にだけ入れる。エンジンの take を無効にしない。
- plan / revise のプロンプトに、自己検証・チェックリスト・自己採点の語を書かない（D-22、`craft::SELF_REVIEW_WORDS` のテストが守る）。品質は生成前の仕様（`craft.rs`）で上げる。
- 仕様は、ショットを書かせる全経路に同じものを届ける（D-48）。revise は `craft::edit_spec` を通し、画が変わりうる指示（自由な指示・カメラ・光）には plan と同じ部品（ブリーフ・方向・ルック・スキル・規則・基準値・ステージ・距離表と、そのカットのロケハン写真）、文字の指示にはルックと基準値、音の指示にはブリーフと方向を渡す。どの経路に何が届くかは `every_path_that_writes_shots_gets_the_frozen_spec` が固定している。経路を足したら、この表にも足す。
- 文字の読み時間は `script::reading_secs` の1か所に置く（D-49）。plan と文字の修正で書く行はコードで読める長さに延ばし、人間が書いた行は書き換えずに `text.hold` で示す。
- 撮影後の確認は、produce の初回のエンジンでの撮影の直後に1回だけ（D-50。D-22 の唯一の例外で、open-design の動画の例外に倣う）。聞くのは「被写体が写っているか、何が遮っているか」という事実だけで、点数や好みは聞かない。写っていないカットだけカメラを1回直して撮り直し、そのカットだけを見直す。2周目はしない。結果は state.json に見た take と一緒に残し、そのカットを撮り直すと消える。previz と普段の render では走らせない。
- 企画（`ai::discover`、D-45）は open-design の OD Next の計画ターンを写す。質問は欠けがあるときだけ（5問まで、推奨の既定値つき）、そうでなければ方向を1文で言い切った企画を書き、承認を待たずに制作へ進む。人間の確認は previz の段（D-38）。ルックとスキルの id はコードで実在を確かめ、知らない id は捨てる。発売日・機種などの事実は作らせない。
- ルック（`references/design-systems/`）とスキル（`references/skills/`）も plan に全文が渡るので、D-22 の語を書かない（`cargo test --test planning` が守る）。ルックは汎用に限り、実在のゲームやスタジオの名前・ロゴ・固有の色を写さない。ルックの tokens（色調・帯・書体）は Claude の提案より優先してコードで書く。
- 流行りの型（`refs::STYLES`、D-39）は実在ティザーの構成とリズムだけを真似させる。話の筋・キャラクター・文言は真似させない。分析文（TEASER.md の 1〜7 節）も plan に渡るので、D-22 の語を書かない（`cargo test --test styles` が守る）。
- `references/craft/` と `references/genres/` は実測から生成し、バイナリに埋め込む（D-29）。表は `aggregate_refs.py` が書き、auto マーカーの外の本文だけを手で書く。本文も plan に渡るので、D-22 の語を書かない。
- ショット検査の重さは根拠で決める。出典のある規則だけを P0 にし、推定の閾値は P2 に留める（実測で校正するまで）。
- `build` は、Sequencer 側で人間が加えた変更を黙って上書きしない（`--force` のときだけ上書きする）。
- 画面の文言は日本語で書き、英語は `web/i18n-en.json` の辞書で出す（D-51）。描画コードに英語の分岐を書かない。文言を足したり変えたりしたら `node tools/i18n_extract.js` で辞書に無いものを出し、英訳を足す（`{0}` などの穴は日本語と同じ組にする。`the_pages_switch_to_english_with_a_dictionary` が守る）。ユーザーや Claude が書いた文は訳さない（辞書に無い文はそのまま残り、穴に入る自由な文は `i18n-free.json` が示す）。DOM の文字を読んで処理を分けるコードを書かない（英語モードでは中身が変わる）。
- ユーザー向けのエラーは `Error::user`（`bail!`）で返す。CLI はメッセージだけを出す。`unwrap` で落とさない。
- 座標系は UE 準拠（cm、Z が上、X が前、Y が右、左手系、回転は度）。`geometry.rs` は依存なしのまま保つ。

## テスト

- ループの挙動を変えたら、**どのショットが再レンダ・再ビルドされ、どれがされなかったか**をテストで確かめる。`tests/common/mod.rs` の `launches(&ws)` で、mock エンジンの起動記録を読める。
- `only_changed_shot_rerenders` と `order_purpose_notes_never_invalidate` はデモの主張そのものにあたる。落としたまま先に進まない。
- テストから実際の UE や Claude API は呼ばない。UE は mock エンジンで、Claude は差し替えで代用する。

## スコープ

- やる: UE5 のティザー映像1本。ティザーの型（`hook → establish → escalate → title`）を同梱する。1ビートに3バリアントを出すのは余力があればやる。
- やる: デスクトップアプリ（`shotpilot-app`、Tauri）。ブラウザは使わない。中身はコアを組み込んだ loopback サーバーと、ビルド不要の HTML 1枚。
- やる: BGM と SE（D-19）。`shots.yaml` に書き、cut のときにミックスして載せる。映像は撮り直さない。検証は earshot 式のサウンドチェック。
- やる: 仕上げ（D-25）。帯・タイトル・エンドカード・字幕・カットのつなぎ（黒/白をはさむ）を shots.yaml に書き、Cut で焼き込む。撮り直しは起きない。
- やる: 公開後の発信計画（D-47）。ティザーのあとの動画（発表・ゲームプレイ・デモ／フェス・ローンチ・アップデート・セール）をチェックシートで並べ、それぞれを隣の別プロジェクトとして起こす。アップデート連動と定期発信の候補は、ゲームの変化の検知（D-37）から出す。1プロジェクトが作る映像は1本のまま。
- やらない: Unity / Godot / Roblox 対応、エンジン抽象化レイヤ、TikTok / Shorts 向け短尺（定期発信も横長の take のまま）、複数尺への展開、Electron、Web 公開する UI、GUI での録画後の自由な映像編集（トリム、エフェクト、クロスフェード）。
- mock エンジンはテストとデモ用で、別エンジン対応の足場ではない。抽象化を先回りして作らない。
- 録画後の映像編集は作らない（差別化にならない）。差別化できるのは録画の前だけ。音（D-19）と仕上げ（D-25）は例外で、定義の一部として扱い Cut で適用する。

## スタイル

- Rust（edition 2021）。Python は `ue/sp_entry.py` だけに留める（Unreal 同梱の Python で動かす必要があるため）。
- コード中のコメントと doc コメントは英語。ユーザー向けの CLI 出力は英語でもよい。サンプルの `shots.yaml` の中身は日本語でよい。
- 依存クレートを増やさない（現状: serde_json, yaml-rust2, sha2, miniz_oxide, chrono, ureq, clap。アプリは tauri, rfd）。
- コミットは小さく分ける。push は人間の確認を取ってから行う。
