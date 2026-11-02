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
shotpilot cut                     # 既存 take を連結する（レンダはしない）
shotpilot-app [path/to/shots.yaml]   # アプリ。引数がなければフォルダを選ぶ
```

`cut` の動画書き出しは ffmpeg を使う（無ければ storyboard と spec だけを出す）。`plan` と `revise` は `ANTHROPIC_API_KEY` を使う（モデルは `SHOTPILOT_MODEL` で上書きできる）。

## 構成

```
crates/shotpilot/            コア（ライブラリ + CLI）
  src/model.rs        shots.yaml のスキーマ・検証・resolve（Definition → ResolvedShot）
  src/fingerprint.rs  再レンダ判定。品質プリセット（previz/still/preview/final）
  src/state.rs        .shotpilot/state.json と render/<shot>/<quality>/<fp12>/ の take 管理
  src/workspace.rs    上の3つとエンジンを束ねる。全操作はここを通る
  src/engines.rs      mock / ue5 バックエンド。共通の job/result JSON プロトコル
  src/sync.rs         Sequencer → shots.yaml の逆同期（base / engine / ours の三方比較）
  src/previz.rs       エンジンを使わないワイヤーフレーム描画（PNG 書き出し）
  src/cut.rs          take をショット順に連結してティザーにする
  src/ai.rs           Claude によるショットリスト作成（plan）と部分修正（revise）。Messages API を直接呼ぶ
  src/geometry.rs     UE 座標系のカメラ計算
  src/yamlio.rs       コメントを保持する YAML 編集（行単位の差し替え + 再パースで検証）
  src/templates.rs    shots.yaml の雛形
  src/main.rs         clap の CLI
  src/server.rs       アプリの裏側。Workspace の上に載る薄い HTTP + SSE 層（loopback 限定）
  web/index.html      アプリの画面（ビルドなし、依存なし。バイナリに埋め込む）
  ue/sp_entry.py      Unreal の中で動くスクリプト。唯一の Python（別ルールあり。バイナリに埋め込む）
  tests/              mock エンジンでループ全体を検証する
crates/shotpilot-app/        Tauri のデスクトップアプリ。コアを組み込み、ウィンドウ1枚で開く
```

## 守ること（不変条件）

- **fingerprint にはピクセルを変えるものだけを入れる。** 並び順、`purpose`、メモは入れない。並べ替えやメモの修正で再レンダが起きてはいけない。
- エンジン側スクリプトが、同じ入力に対して違う出力を出すように変わったら `fingerprint::RENDERER_VERSION` を上げる。previz の描画を変えたら `PREVIZ_VERSION` を上げる（エンジンの take は無効にしない）。
- fingerprint は Python 版と同じ正規化 JSON（キー整列・区切り `,` `:`）の SHA-256。`fingerprints_match_the_python_version` を落とさない（落ちると UE で撮った take が全部撮り直しになる）。
- **take は fingerprint で管理し、上書きしない。** 定義を元に戻したら、古い take を再利用する。`render/` 以下を手で編集しない。
- `shots.yaml` を機械的に書き換えるときは、必ず `yamlio::Doc` を通す（変えるショットの行だけを差し替え、再パースして意図どおりか確かめてから書く）。人間が書いたコメントとキー順を消さない。
- Claude（`ai.rs`）が返した編集は、`shots.yaml` に書く前に resolve して検証する。編集してよいショットの範囲は、プロンプトではなくコードで強制する。
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
- やらない: Unity / Godot / Roblox 対応、エンジン抽象化レイヤ、TikTok / Shorts 向け短尺、複数尺への展開、Electron、Web 公開する UI、GUI での録画後編集。
- mock エンジンはテストとデモ用で、別エンジン対応の足場ではない。抽象化を先回りして作らない。
- 録画後の編集機能は作らない（差別化にならない）。差別化できるのは録画の前だけ。

## スタイル

- Rust（edition 2021）。Python は `ue/sp_entry.py` だけに留める（Unreal 同梱の Python で動かす必要があるため）。
- コード中のコメントと doc コメントは英語。ユーザー向けの CLI 出力は英語でもよい。サンプルの `shots.yaml` の中身は日本語でよい。
- 依存クレートを増やさない（現状: serde_json, yaml-rust2, sha2, miniz_oxide, chrono, ureq, clap。アプリは tauri, rfd）。
- コミットは小さく分ける。push は人間の確認を取ってから行う。
