# Shotpilot — Agent Instructions

UE5 プロジェクトからティザー映像（30〜60秒）を撮るための、ショット設計・撮影レーン。
ショットを `shots.yaml` に定義し、変わったショットだけを再レンダする。

一行で言うと「ティザーのショットを AI が組み、人間が1カットだけ直すと、その1カットだけ撮り直る」。

提出先は Built with Claude: Japan 2026 で、締切は **2026-11-08 20:00 JST**。開発者は1人。

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
shotpilot brief genre=ホラー       # brief: を見る・書く
shotpilot moodboard               # ブリーフからムードボードを4枚生成（Codex か OpenAI キー）
shotpilot storyboard [s03]        # previz の線画を下絵に、リアル寄りの絵コンテを生成（確認用。Cut には入らない）
shotpilot produce                 # 初回: 制約を全部渡して一気に1本出す（ロケハン→ステージ→ムードボード→構成と仕上げ→previz→Cut）
shotpilot scout                   # 工程0: 各アンカーを4方向から撮る（scout/）
shotpilot assets --scan           # 工程0: mesh を寸法付きで棚卸し → prop add <name> <asset>
shotpilot cut                     # 既存 take を連結する（レンダはしない。BGM・SE があれば載せる）
shotpilot sound                   # BGM と SE をミックスし、ローンチ映像7本と比べて採点する
shotpilot-app [path/to/shots.yaml]   # アプリ。引数がなければホーム（最近のプロジェクト・新規作成）を開く
```

`cut` の動画書き出しは ffmpeg を使う（無ければ storyboard と spec だけを出す）。`plan` と `revise` は、ログイン済みの Claude Code（`claude` コマンド）か API キー（アプリの設定、または `ANTHROPIC_API_KEY`）で Claude を呼ぶ（モデルは `SHOTPILOT_MODEL` で上書きできる。effort は `low`〜`max` をアプリの接続設定か `SHOTPILOT_EFFORT` で変えられ、既定は `medium`。Claude Code 経由でも API 経由でも同じ値が効く）。

## 構成

```
crates/shotpilot/            コア（ライブラリ + CLI）
  src/model.rs        shots.yaml のスキーマ・検証・resolve（Definition → ResolvedShot）
  src/fingerprint.rs  再レンダ判定。品質プリセット（previz/still/preview/final）
  src/state.rs        .shotpilot/state.json と render/<shot>/<quality>/<fp12>/ の take 管理
  src/workspace.rs    上の3つとエンジンを束ねる。全操作はここを通る
  src/engines.rs      mock / ue5 バックエンド。共通の job/result JSON プロトコル
  src/sync.rs         Sequencer → shots.yaml の逆同期（base / engine / ours の三方比較）
  src/scout.rs        工程0「ロケハン」: アセットの棚卸し、props への追加、アンカーのロケハン写真
  src/moodboard.rs    ブリーフのムードボード（参照画像の保存、OpenAI での生成、plan に渡す画像ブロック）
  src/storyboard.rs   絵コンテ（D-33）。previz の線画を下絵に、ロケハン写真とムードボードを参照して画像生成で描く確認用の1枚
  src/produce.rs      初回の一気通し（D-28）。足りない下ごしらえを埋め、plan を1回、最安の品質で撮って仕上げ込みで Cut
  src/craft.rs        plan に書く前に渡す仕様（ブリーフ、骨格、規則、画角の距離表）とコードによる整形
  src/refs.rs         実測リファレンス（references/craft/ と genres/<id>/ を埋め込み）。plan に渡す文と、骨格の hook・title の長さ
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
  src/server.rs       アプリの裏側。Workspace の上に載る薄い HTTP + SSE 層（loopback 限定）
  web/index.html      アプリの studio 画面（ビルドなし、依存なし。バイナリに埋め込む）
  web/home.html       アプリのホーム画面（D-30。最近のプロジェクト、プロンプトからの新規作成、設定）
  ue/sp_entry.py      Unreal の中で動くスクリプト。唯一の Python（別ルールあり。バイナリに埋め込む）
  tests/              mock エンジンでループ全体を検証する
crates/shotpilot-app/        Tauri のデスクトップアプリ。コアを組み込み、ウィンドウ1枚で開く
```

## 守ること（不変条件）

- **fingerprint にはピクセルを変えるものだけを入れる。** 並び順、`purpose`、メモ、音（`audio`、`sfx`）、`brief:`、仕上げ（`finish:`、`caption`、`transition`）は入れない。仕上げは take のピクセルではなく Cut の出力を変える。並べ替えやメモ、音の修正で再レンダが起きてはいけない。
- エンジン側スクリプトが、同じ入力に対して違う出力を出すように変わったら `fingerprint::RENDERER_VERSION` を上げる。previz の描画を変えたら `PREVIZ_VERSION` を上げる（エンジンの take は無効にしない）。
- fingerprint は Python 版と同じ正規化 JSON（キー整列・区切り `,` `:`）の SHA-256。`fingerprints_match_the_python_version` を落とさない（落ちると UE で撮った take が全部撮り直しになる）。
- **take は fingerprint で管理し、上書きしない。** 定義を元に戻したら、古い take を再利用する。`render/` 以下を手で編集しない。
- `shots.yaml` を機械的に書き換えるときは、必ず `yamlio::Doc` を通す（変えるショットの行だけを差し替え、再パースして意図どおりか確かめてから書く）。人間が書いたコメントとキー順を消さない。
- Claude（`ai.rs`）が返した編集は、`shots.yaml` に書く前に resolve して検証する。編集してよいショットの範囲は、プロンプトではなくコードで強制する。音だけの指示（`Pointer::sound`）では `sfx` と `audio` だけを取り込む。ショットに絞った指示では BGM を変えさせない。
- 構図の承認は `state.json` に look fingerprint（品質に依らない画の指紋）で残す。画が変わる編集で外れ、編集的な変更では外れない。P0 の指摘があるショットは承認させない。
- 絵コンテ（`storyboard/`）は確認用で、take ではない。`render/` にも state の take にも入れず、Cut には使わない。look fingerprint を名前にして、画が変わる編集で古くなり、編集的な変更では古くならない。生成するのは人間が頼んだときだけ（produce やレンダでは自動で描かない）。
- ロケハンのカメラはショットではない。shots.yaml にも take の状態にも入れない（UE では content_root/Scout/ にシーケンスを作るだけ）。
- ムードボードの画像は雰囲気の参照に限る。生成画像をティザーの画として使わない（実際のゲームと違うものを見せない）。`brief.images` は編集的で、撮り直しを起こさない。
- 台本（`script:`）と、そこに付く音（`audio.music`、行の `sfx`）は編集的で、fingerprint に入れない。位置は必ず `script::layout` / `sound::spans` から出す（カードを足したり並べ替えたりしても音がずれないように）。
- `appear`（prop の出現）は、使ったショットの engine payload にだけ入れる。空のときにキーを足さない（足すと全ショットの fingerprint が変わる）。
- アンカーの大きさ（`{at, size, center}`）は previz の fingerprint にだけ入れる。エンジンの take を無効にしない。
- plan / revise のプロンプトに、自己検証・チェックリスト・自己採点の語を書かない（D-22、`craft::SELF_REVIEW_WORDS` のテストが守る）。品質は生成前の仕様（`craft.rs`）で上げる。
- `references/craft/` と `references/genres/` は実測から生成し、バイナリに埋め込む（D-29）。表は `aggregate_refs.py` が書き、auto マーカーの外の本文だけを手で書く。本文も plan に渡るので、D-22 の語を書かない。
- ショット検査の重さは根拠で決める。出典のある規則だけを P0 にし、推定の閾値は P2 に留める（実測で校正するまで）。
- `build` は、Sequencer 側で人間が加えた変更を黙って上書きしない（`--force` のときだけ上書きする）。
- ユーザー向けのエラーは `Error::user`（`bail!`）で返す。CLI はメッセージだけを出す。`unwrap` で落とさない。
- 座標系は UE 準拠（cm、Z が上、X が前、Y が右、左手系、回転は度）。`geometry.rs` は依存なしのまま保つ。

## テスト

- ループの挙動を変えたら、**どのショットが再レンダ・再ビルドされ、どれがされなかったか**をテストで確かめる。`tests/common/mod.rs` の `launches(&ws)` で、mock エンジンの起動記録を読める。
- `only_changed_shot_rerenders` と `order_purpose_notes_never_invalidate` はデモの主張そのものにあたる。落としたまま先に進まない。
- テストから実際の UE や Claude API は呼ばない。UE は mock エンジンで、Claude は差し替えで代用する。

## スコープ（7日間）

- やる: UE5 のティザー映像1本。ティザーの型（`hook → establish → escalate → title`）を同梱する。1ビートに3バリアントを出すのは余力があればやる。
- やる: デスクトップアプリ（`shotpilot-app`、Tauri）。ブラウザは使わない。中身はコアを組み込んだ loopback サーバーと、ビルド不要の HTML 1枚。
- やる: BGM と SE（D-19）。`shots.yaml` に書き、cut のときにミックスして載せる。映像は撮り直さない。検証は earshot 式のサウンドチェック。
- やる: 仕上げ（D-25）。帯・タイトル・エンドカード・字幕・カットのつなぎ（黒/白をはさむ）を shots.yaml に書き、Cut で焼き込む。撮り直しは起きない。
- やらない: Unity / Godot / Roblox 対応、エンジン抽象化レイヤ、TikTok / Shorts 向け短尺、複数尺への展開、Electron、Web 公開する UI、GUI での録画後の自由な映像編集（トリム、エフェクト、クロスフェード）。
- mock エンジンはテストとデモ用で、別エンジン対応の足場ではない。抽象化を先回りして作らない。
- 録画後の映像編集は作らない（差別化にならない）。差別化できるのは録画の前だけ。音（D-19）と仕上げ（D-25）は例外で、定義の一部として扱い Cut で適用する。

## スタイル

- Rust（edition 2021）。Python は `ue/sp_entry.py` だけに留める（Unreal 同梱の Python で動かす必要があるため）。
- コード中のコメントと doc コメントは英語。ユーザー向けの CLI 出力は英語でもよい。サンプルの `shots.yaml` の中身は日本語でよい。
- 依存クレートを増やさない（現状: serde_json, yaml-rust2, sha2, miniz_oxide, chrono, ureq, clap。アプリは tauri, rfd）。
- コミットは小さく分ける。push は人間の確認を取ってから行う。
