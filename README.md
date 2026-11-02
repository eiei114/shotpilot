# Shotpilot

UE5 プロジェクトからティザー映像（30〜60秒）を撮るための、ショット設計・撮影ツール。

ショットを `shots.yaml` に書くと、Sequencer のシーケンスを組み、Movie Render Queue で撮影し、素材を名前付きで保持する。1カットだけ直したら、撮り直すのはその1カットだけ。

```
shots.yaml ──build──▶ UE: /Game/Shotpilot/Shots/SEQ_<shot>   (ショットごとの Level Sequence)
    ▲                        │
    └────────pull────────────┘   Sequencer で人間が直した差分を戻す
    │
    └──render──▶ render/<shot>/<quality>/<fingerprint>/frame.NNNN.png   (上書きしない take)
                        │
                        └──cut──▶ cut/<quality>/ cut.mp4 / index.html / spec.md / manifest.json
```

## 部分修正ループ

| 要件 | 実装 |
|---|---|
| ショット単位で再レンダ | ショットごとに Level Sequence と MRQ ジョブを分ける。fingerprint が変わったショットだけを撮る |
| 定義と出力の分離 | 正本は `shots.yaml` だけ。シーケンスアセットも `render/` も、すべてそこから導出する |
| 状態の保持 | `.shotpilot/state.json` に、push した内容・最新の take・履歴を残す。take は fingerprint ごとのフォルダに置き、上書きしない。定義を元に戻すと、古い take をそのまま再利用する |
| 安いプレビュー | `previz`（エンジン不要のワイヤーフレーム。6ショットで数秒）→ `still`（1ショット3フレーム）→ `preview`（半解像度）→ `final` |

並び順、`purpose`、`notes`、`beat` は fingerprint に入れていない。だから並べ替えやメモの修正では何も再レンダされない。`cut` は既存の take をつなぐだけなので、すぐ終わる。

## インストール

Rust（1.80 以上）でビルドする。Python は要らない（UE の中で動く `sp_entry.py` だけは Unreal 同梱の Python が実行する）。

```bash
cargo install --path crates/shotpilot          # CLI: shotpilot
cargo build --release -p shotpilot-app         # アプリ: target/release/shotpilot-app.exe
```

`cut` で動画を書き出すには ffmpeg が必要（無ければ `index.html` / `spec.md` / `manifest.json` だけを出す）。

## アプリ

`shotpilot-app` を起動すると、ウィンドウが1枚開く（Tauri。ブラウザは使わない）。引数に `shots.yaml` かそのフォルダを渡すか、起動後にフォルダを選ぶ。`shots.yaml` が無いフォルダを選ぶと、サンプル（mock エンジン、6ショット）を作るか聞かれる。

- カードをクリックしてショットを直し、「Render stale」を押すと、直したカードだけが RENDERED になり、残りは REUSED のまま残る。
- 「Cut」で既存の take をつなぐ。storyboard と cut.mp4 は OS の既定アプリで開く。
- アプリと CLI は同じ `shots.yaml` を読み書きする。どちらから直しても数秒で反映される。
- 中ではコアのライブラリが 127.0.0.1 の空きポートでだけ待ち受け、ウィンドウがそこを表示する。別プロセスは立てない。

## まず試す（UE なし）

```bash
mkdir demo && cd demo
shotpilot init                       # サンプルの shots.yaml（engine: mock）
shotpilot render previz              # 6ショットをエンジンなしで描く
shotpilot cut                        # cut/best/index.html と cut.mp4
shotpilot set 3 camera.lens.focal_length=85    # 3番目だけ寄る
shotpilot render previz              # → s03 だけ rendered、残り5つは reused
shotpilot order --move s06_title --to 1        # 並べ替えは再レンダなし
shotpilot status
```

同じことはアプリ（`shotpilot-app`）からもできる。

`engine: mock` は UE の代役。シーケンスは JSON、レンダは色付きの previz で済ませる。ループ全体を UE なしで動かすためのもの。

## UE5 で使う

```bash
shotpilot init --engine ue5 --uproject C:/path/MyGame.uproject --map /Game/Maps/Courtyard
```

Git Bash では、`/Game/...` という引数が Windows パスに書き換えられる。`MSYS_NO_PATHCONV=1` を付けて実行するか、`shots.yaml` を直接書き換える。

- `UnrealEditor-Cmd` は、`.uproject` の `EngineAssociation` から探す。見つからなければ `project.ue_cmd` か環境変数 `SHOTPILOT_UE_CMD` で指定する。
- プロジェクトでは `Python Editor Script Plugin`、`Editor Scripting Utilities`、`Sequencer Scripting`、`Movie Render Queue` を有効にしておく。
- `build` / `pull` / `scan` は commandlet（`-run=pythonscript`、`-NullRHI`）で動く。`render` はエディタを起動し、MRQ の PIE executor で撮り終えたら終了する。
- 既定では、隔離した `-UserDir`（`.shotpilot/ue_userdir`）で起動する。普段のエディタ設定には触れない。
- `project.ue_args` に書いた引数は、UE の起動のたびに付ける（例: ゲームが実行時にレベルを組むプロジェクトで、そのモードを選ぶフラグ）。撮れる画が変わるので、エンジン品質の fingerprint に入る（空なら入らない）。

```bash
shotpilot scan                 # タグ shotpilot が付いたアクター、またはラベルが SP_* のアクターを anchors に取り込む
shotpilot render still         # 構図と光を、1ショット3フレームで確認する
shotpilot render preview
shotpilot pull                 # Sequencer で直したカメラを shots.yaml に戻す
shotpilot render final
shotpilot cut final
```

### Sequencer で直した場合

`build` は、Sequencer 側で人間が直したシーケンスを上書きしない。`pull` は次の3つを比べる。

- base: 最後に push した内容
- engine: アセットの現状
- ours: `shots.yaml` の現状

| 状況 | 結果 |
|---|---|
| エンジン側だけが変わった | `shots.yaml` に書き戻す（カメラは location / rotation の明示形式になり、元の意図はコメントに残る） |
| 両方が変わった | conflict。`pull --prefer engine` か `--prefer yaml` で選ぶ |

## shots.yaml

```yaml
anchors:                      # レベル内の名前付きの点（cm、Z が上）
  well: [0, 0, 80]
shots:
  - id: s02_well
    beat: establish           # hook → establish → escalate → title
    purpose: 井戸のクローズアップ。何かがいる気配
    duration: 3.5
    camera:
      anchor: well            # anchor + offset で置き、anchor を見る
      offset: [-180, 60, 60]
      move: {type: dolly_in, distance: 120}   # static / dolly / truck / crane / orbit
      lens: {focal_length: 85, aperture: 1.8}
    environment: {time_of_day: night}         # project.sun_actor の回転をシーケンスで駆動する
    markers:
      - {at: 1.5, label: door opens}
```

カメラは `location` + `rotation` / `look_at` でも書ける。`end:` を書けば、終点を明示できる。

## Claude で組む・直す

```bash
export ANTHROPIC_API_KEY=...
shotpilot plan --pitch "シェアハウスで住人が一人ずつ消えるホラー" --duration 30 --write
shotpilot revise "3番目はもっと寄って、少し見上げる感じに" --shot s03_window
shotpilot revise "タイトルカットを最初に持ってきて"
```

- `plan`: pitch と anchors から、hook → establish → escalate → title の型でショットリストを作る。
- `revise`: 指示に関係するショットだけを書き換える（並べ替えも含む）。返ってきた編集は、適用する前に resolve して検証する。`--shot` を付けると、編集できる範囲をコードで制限する。
- モデルの既定は `claude-opus-5-5`。`SHOTPILOT_MODEL` で上書きできる。Messages API を直接呼ぶ（ストリーミング、構造化出力、拒否されたときはサーバー側で既定のモデルに切り替える `fallbacks: "default"`）。

## コマンド一覧

| コマンド | 内容 |
|---|---|
| `init` | `shots.yaml` の雛形を作る |
| `validate` / `status` | 検証。各ショットの build と take の鮮度、ティザーの構成を確認する |
| `build [shot…]` | 変わったショットのシーケンスとマスターシーケンスを更新する |
| `render [quality] [shot…]` | 古くなったショットだけを撮る（`--force` で強制） |
| `pull [--prefer engine\|yaml]` | Sequencer の編集を `shots.yaml` に戻す |
| `scan` | レベルのアクターから anchors を取り込む |
| `set <shot> key=value…` | 1ショットのフィールドを書き換える |
| `order …` / `order --move <shot> --to N` | 並べ替える |
| `plan` / `revise` | Claude で組む・直す |
| `cut [best\|quality]` | take を連結し、storyboard・spec・mp4 を出す |
| `takes <shot>` / `log` / `gc` | take の一覧、履歴、古い take の削除 |

## 開発

```bash
cargo test                 # UE も API キーも要らない（mock エンジンと差し替えクライアントで回す）
cargo clippy --all-targets
```

fingerprint は Python 版（〜0.1）と同じ正規化 JSON を hash するので、Python 版で撮った UE の take はそのまま再利用される。previz だけは描画が変わったので撮り直しになる（数秒で終わる）。

規約は `AGENTS.md` にある。
