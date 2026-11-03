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
                                    ▲
    audio: / sfx: ──mix──▶ cut/sound/mix.wav（BGM と SE。cut のときに映像へ載せる。再レンダはしない）
```

## 部分修正ループ

| 要件 | 実装 |
|---|---|
| ショット単位で再レンダ | ショットごとに Level Sequence と MRQ ジョブを分ける。fingerprint が変わったショットだけを撮る |
| 定義と出力の分離 | 正本は `shots.yaml` だけ。シーケンスアセットも `render/` も、すべてそこから導出する |
| 状態の保持 | `.shotpilot/state.json` に、push した内容・最新の take・履歴を残す。take は fingerprint ごとのフォルダに置き、上書きしない。定義を元に戻すと、古い take をそのまま再利用する |
| 安いプレビュー | `previz`（エンジン不要のワイヤーフレーム。6ショットで数秒）→ `still`（1ショット3フレーム）→ `preview`（半解像度）→ `final` |

並び順、`purpose`、`notes`、`beat`、音（`audio`、`sfx`）は fingerprint に入れていない。だから並べ替えやメモの修正、BGM や SE の直しでは何も再レンダされない。`cut` は既存の take をつなぐだけなので、すぐ終わる。

## インストール

Rust（1.80 以上）でビルドする。Python は要らない（UE の中で動く `sp_entry.py` だけは Unreal 同梱の Python が実行する）。

```bash
cargo install --path crates/shotpilot          # CLI: shotpilot
cargo build --release -p shotpilot-app         # アプリ: target/release/shotpilot-app.exe
```

`cut` で動画を書き出すには ffmpeg が必要（無ければ `index.html` / `spec.md` / `manifest.json` だけを出す）。

## アプリ

`shotpilot-app` を起動すると、ウィンドウが1枚開く（Tauri。ブラウザは使わない）。引数に `shots.yaml` かそのフォルダを渡すか、起動後にフォルダを選ぶ。`shots.yaml` が無いフォルダを選ぶと、サンプル（mock エンジン、6ショット）を作るか聞かれる。

- 画面は左が会話、右がキャンバス。キャンバスの下に、尺に比例したタイムラインがある。
- キャンバスの上には、トレーラー制作の工程どおりに「1 ブリーフ → 2 構成 → 3 構図 → 4 撮影 → 5 仕上げ」が並び、それぞれの状態（未記入、P0 の数、承認数、final の数、cut の有無）を出す。
  - **ブリーフ**: 種類（ティザー / ゲームプレイトレーラー）、ジャンル、コア体験、対象、必ず見せるもの、参考トレーラーを書く。`shots.yaml` の `brief:` に入り、撮り直しは起きない。
  - **構成**: ショットを hook / establish / escalate / title の列に並べ、ショット検査（`shotcheck.rs`）の指摘を一覧する。
  - **構図**: 安いプレビュー（previz / still）で画角を決め、「構図を承認」する。承認は画に紐づき、画が変わる修正をすると自動で外れる（戻すと戻る）。P0 の指摘があるショットは承認できない。
  - **撮影**: 承認済みのショットだけを preview / final で撮る。
  - **仕上げ**: Cut した映像と、サウンド（BGM / SE、サウンドチェック）。
- ショット検査の指摘は、タイムラインのクリップに P0 / P1 の印で出る。「Claude に直させる」で、その指摘を指示欄に入れられる（送るのは自分）。
- タイムラインでショットとその瞬間を選び、プレビューの上をドラッグして直したいところを囲み、左の欄に「どう直すか」を書く。Claude はそのフレームの画像と囲んだ範囲を見て、そのショットだけを直す。
- 数値を直接直したいときは「調整」に切り替える。
- Claude への接続は、入力欄の下のモデル名から選ぶ。既定ではログイン済みの Claude Code をそのまま使う（API キー不要）。API キーを使う場合は、そこで入力すると `%APPDATA%\shotpilot\settings.json` に保存される。
- 「撮り直す」を押すと、古くなったショットだけが RENDERED になり、残りは reused のまま残る。
- 「Cut」で既存の take をつなぎ、「仕上げ」の「映像」で cut.mp4 を再生する。BGM と SE があれば、音を載せた cut.mp4 になる。
- タイムラインの下に BGM と SE のレーンがある（波形つき）。SE はドラッグで動かせて、カットの境目に吸着する（Alt で吸着なし）。別のショットの上に落とすと、そのショットの SE になる。レーンをダブルクリックすると、その位置に SE を足す。各カットの印は、そこで音が立ち上がっているか（緑）、いないか（黄の点線）を示す。
- 「仕上げ」の「サウンド」では、選んだ BGM / SE の音量・位置・フェードを直し、ミックスを試聴し、サウンドチェックの結果を見る。サウンドチェックは [earshot](https://github.com/Leonxlnx/earshot) の `critique` の検証部分を移植したもので、プロのローンチ映像7本と比べた採点・直す順の一覧・ラウドネスの流れを出す。指摘ごとの「Claude に頼む」で、その指摘を会話に渡せる。
- 「サウンド」を開いて会話すると「音だけ」の指示になる。Claude が返した編集から、コードが `sfx` と `audio` だけを取り込む（カメラは変わらない）。
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
shotpilot sound                      # BGM と SE をミックスし、ローンチ映像7本と比べて採点する
shotpilot sound bgm.gain=-9          # 音を直しても再レンダなし
shotpilot status
```

`init` は、サンプルが参照する仮の音（`audio/drone.wav`、`hit.wav`、`whoosh.wav`。合成した音）も書き出す。

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
- `project.game_mode`（例 `/Script/MyGame.MyGameMode`）を書くと、MRQ はその GameMode で PIE を回す。書かなければ MRQ 既定の空の GameMode。レベルを実行時に組むゲームでは必須（既定のままだと何もない空間を撮る）。

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
  well: {at: [0, 0, 80], size: [140, 140, 90], center: [0, 0, 45]}   # 大きさ付き（scan が UE の外形から入れる）
  marker: [500, 0, 0]         # 点だけでもよい（previz は人の大きさの箱で描く）
props:                        # ショットに出現させられるもの（アニメーションなし。キャラは基準ポーズのまま）
  stranger: {asset: /Game/Characters/Mannequins/Meshes/SKM_Manny, kind: skeletal, size: [60, 40, 180]}
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

### 音（BGM と SE）

```yaml
audio:
  gain: 0                                   # 全体（dB）
  bgm: {file: audio/drone.wav, gain: -6, fade_in: 1.5, fade_out: 2}   # start / offset も書ける
shots:
  - id: s03_window
    sfx:
      - {file: audio/hit.wav, at: 2.2, gain: -3, label: silhouette}   # at = ショットの頭からの秒
```

- 音のファイルは `shots.yaml` と同じ場所の `audio/` に置く（wav はそのまま読む。mp3 / ogg / flac / m4a は ffmpeg で読む）。
- SE はショットに属するので、並べ替えるとショットと一緒に動く。`at` をマイナスにすると、前のカットから鳴り始める（-3 秒まで。カットに向かうウーッシュなど）。
- ミックスは `cut/sound/mix.wav` に置き、配置と元ファイルが変わらない限り作り直さない。`cut` はそれを映像に載せる。音のファイルが見つからないときは、無音のまま cut して理由を出す。
- サウンドチェックで測らないもの: ML モデルが要る指標（realness、音楽・発話の判定）と、ビートの検出。採点表では「未計測」と表示する。

## Claude で組む・直す

```bash
export ANTHROPIC_API_KEY=...
shotpilot plan --pitch "シェアハウスで住人が一人ずつ消えるホラー" --duration 30 --write
shotpilot revise "3番目はもっと寄って、少し見上げる感じに" --shot s03_window
shotpilot revise "タイトルカットを最初に持ってきて"
```

- `plan`: pitch・ブリーフ・anchors から、hook → establish → escalate → title の型でショットリストを作る。Claude には書く前に、骨格（ショット数・ビート・尺・ショットサイズ）、トレーラーの規則、画角の距離表、各アンカーの大きさを渡し、1回で組ませる。自分の出力を見直させることはしない（D-22）。ビート順・尺の範囲・総尺はコードが整え、整えた点を表示する。`plan --spec` で、渡す内容だけを表示できる。
- `appear`: ショットの途中で prop をいきなり出す。`appear: [{prop: stranger, at: 1.5, anchor: door, offset: [150, 0, -120]}]` と書くと、1.5 秒の時点から扉の脇に立つ（向きは既定でカメラの方、`yaw` で指定もできる）。UE では Sequencer の spawnable と Spawn トラックで出し、previz では赤い箱で描く。fingerprint に入るのは使ったショットだけで、出現を足すとそのショットだけが撮り直しになる。Claude も plan / revise で使える。
- ショット検査は、被写体（`look_at` のアンカー、無ければ `anchor`）の箱を画面に投影し、画面の高さの何割を占めるかを測る（`check` とアプリで表示）。カメラが箱の中にある、小さすぎる、大きすぎて切れている、を指摘する。
- `revise`: 指示に関係するショットだけを書き換える（並べ替えと、SE・BGM も含む）。返ってきた編集は、適用する前に resolve して検証する。`--shot` を付けると、編集できる範囲をコードで制限する（このとき BGM は変えられない）。Claude が `audio/` に無いファイルを使おうとしたら、何も適用しない。
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
| `check [--json]` | ショット検査。P0（承認を止める）/ P1 / P2 と直し方を出す。P0 があれば終了コード 1 |
| `approve <shot…> [--undo]` | 構図を承認する（画が変わると外れる） |
| `brief [key=value…]` | `brief:` を見る・書く（`kind=gameplay genre=ホラー must_show='[井戸, 窓]'`） |
| `set <shot> key=value…` | 1ショットのフィールドを書き換える |
| `order …` / `order --move <shot> --to N` | 並べ替える |
| `plan` / `revise` | Claude で組む・直す |
| `cut [best\|quality]` | take を連結し、storyboard・spec・mp4 を出す（音があれば載せる） |
| `sound [key=value…] [--json]` | BGM と SE をミックスし、サウンドチェックを出す。`bgm.file=…` などで `audio:` を直す |
| `takes <shot>` / `log` / `gc` | take の一覧、履歴、古い take の削除 |

## 開発

```bash
cargo test                 # UE も API キーも要らない（mock エンジンと差し替えクライアントで回す）
cargo clippy --all-targets
```

fingerprint は Python 版（〜0.1）と同じ正規化 JSON を hash するので、Python 版で撮った UE の take はそのまま再利用される。previz だけは描画が変わったので撮り直しになる（数秒で終わる）。

規約は `AGENTS.md` にある。

サウンドチェック（`soundcheck.rs`）は earshot（MIT License, Copyright (c) 2026 Leon Lin）の `critique` / `profile` / `features` を Rust に移植したもの。基準のプロファイル、重み、許容幅、判定の区切りは earshot のものをそのまま使う。
