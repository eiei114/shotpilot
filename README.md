# Shotpilot

UE5 のゲームからティザー映像（30〜60秒）を撮るための、ショット設計・撮影ツール。Built with Claude: Japan 2026 の提出作品（開発者1人）。

**ティザーのカットを Claude が組み、人間が1カットだけ直すと、その1カットだけが撮り直る。ゲームが変わったら、変わったものが写っていたカットだけが撮り直る。**

## 何を解くか

インディー開発者は、ゲームは作れてもトレーラーで詰まる。調査（`reports/`）で一番強かった不満は、ゲームの見た目や機能が変わるたびにトレーラーを全部撮り直すことだった。

- 「I also had to reshoot the whole thing because I decided to change my enemy textures」（Reddit、2026-08）
- 「watching the same 30 seconds about 100 times」（Reddit、2026-09）
- 撮り直しに耐えかねて「プレイを自動で再生して録画し、ゲームが更新されたら再実行する」仕組みを自分で作った開発者が、別々のスレッドに何人もいた。

似た製品は、どれも「どのカットが古くなったか」を判定しない（`reports/トレーラー制作の類似プロダクトと解決策.md`）。Shotpilot はカットごとに、画を決める定義の指紋（fingerprint）と、撮ったときに画面に写っていたゲームの素材の SHA-256 を持つ。どちらかが変わったカットだけを撮り直し、ほかのカットは前の take を使い回す。

調査の限界: 一次の証言は、Reddit の API で読めた 2024-12〜2026-11 の投稿と、日本語の数記事に限られる。制作時間の自己申告（約4時間〜3か月）は単発の声で、分布ではない。「変わったカットだけ撮り直したい」という明示的な要望は見つかっていない。Shotpilot の核は、近くの不満から立てた仮説として扱っている。

## 30 秒で試す（UE も API キーも要らない）

Rust（1.80 以上）があれば動く。`engine: mock` が UE の代役になる。

```bash
cargo install --path crates/shotpilot
mkdir demo && cd demo
shotpilot init                                  # サンプルの shots.yaml（6 カット、mock エンジン）
shotpilot render previz                         # 6 カットをエンジンなしで描く（1 秒かからない）
shotpilot set 3 camera.lens.focal_length=85     # 3 番目だけ寄る
shotpilot render previz                         # → previz: 1 rendered, 5 reused
shotpilot order --move s06_title --to 1         # 並べ替える
shotpilot render previz                         # → previz: 0 rendered, 6 reused（並びは画を変えない）
shotpilot status                                # 各カットの状態（fresh / STALE / GAME）
```

`render` の `rendered` と `reused` が、何を撮り直したかを示す。誤った指定にはメッセージだけが出る（直し方を含む）。アプリは `cargo build --release -p shotpilot-app` で、`target/release/shotpilot-app.exe` が開く。

## UE なしで本物の画を撮る（Three.js のサンプルゲーム、D-72）

mock の previz は灰色の立体なので、光や人まで見たいときは `engine: three` を使う（previz もゲームの光と人形で描かれる）。同梱のサンプルは、晴れた午後の丘の上の村 `KettleHill`（コージー）。白い風車（羽根が回る）、赤とクリームの日よけの屋台、提灯の下がる茶屋、子どもたちが周りを走る大きな樫の木、ひまわりの並ぶ畑の5か所に、色の違う屋根の家・井戸・道と、お茶を注ぐ・手を振る・水をやる・掃く・歩く村人を置いた。場所ごとに色と形が違うので、カメラを動かしたときに何が写ったかがひと目で分かる。Windows に入っている Edge（無ければ Chrome）を画面に出さずに起動して撮る。UE も API キーも要らない。

```bash
mkdir village && cd village
shotpilot init --engine three                   # KettleHill のティザー（10 カット）と、村のゲーム（game/）
shotpilot render still                          # 10 カット × 3 コマを Edge で描く（数秒）
shotpilot set s04_market camera.lens.focal_length=50
shotpilot render still                          # → still: 1 rendered, 9 reused
# ゲームのほうを直す: 水やりをする農夫の服を変える
#   game/objects/farmer.json の "dress" を "cloth_red" に変える
shotpilot status                                # 農夫が写っていた3カットだけが GAME になる
shotpilot render still                          # → still: 3 rendered, 7 reused
shotpilot render preview && shotpilot cut       # 全コマを撮り、文字と音を載せた Cut にする（ffmpeg）
```

画は、質感をノイズから焼いた PBR の材質（芝・瓦・漆喰・木・布）を HDR で描き、太陽の影・AO・レンズの絞りどおりの被写界深度・ブルーム・遠くの霞を重ねる（still は10カットで約5秒、preview の約1200コマで約30秒）。ゲームは `game/` の JSON で、物ごとに1ファイル（`objects/<名前>.json`）になっている。物を直すと、それが写っていたカットだけが古くなる（写っていたかは、視野と、ほかの壁に遮られていないかで決める。村を見渡す全景のカットには、遠くの物も写っている）。`shotpilot scout` で各アンカーを4方向から撮れる（壁の向こうに出たカメラは壁の手前へ寄せる）。アプリでは、ホームの「Three.js のサンプルゲームで試す」から開ける。プロンプトから作るときは、エンジンを Three.js にして「ゲーム」の欄で撮るゲームのフォルダ（scene.json のあるフォルダ）を選べる。ティザーのフォルダはそのゲームのフォルダの中にでき（UE の .uproject と同じ）、選ばなければ同梱の村が入る。選ぶためのゲームのフォルダとして、リポジトリに `samples/KettleHill-three/`（同梱の村と同じ中身。中に作ったティザーは `.gitignore` でリポジトリから外れる）を置いている。描くブラウザは `SHOTPILOT_BROWSER` で指定できる。デモ動画は UE で撮っている。three は試すための体験版で、自分の Three.js のゲームを撮る機能ではない。

## ゲームを Claude Code で直しているなら（D-66）

ゲーム本体を Claude Code で直していると、ティザーのどのカットが古くなったかは誰も教えてくれない。Shotpilot は、それを Claude Code の中で知らせる。

```bash
cd MyGame                                           # .uproject のあるフォルダ
shotpilot --file Teaser/shots.yaml claude install   # Skill とフックを入れる（既存の設定とフックは残す）
```

- **フック**（SessionStart と、Edit・Write・Bash・PowerShell のあとの PostToolUse）が、セッションの始めとファイルを変えたあとに、古くなったカットだけを事実の文で Claude に渡す。何も古くなければ何も言わない。同じ変化は1回だけ言う。
  ```
  Shotpilot（ティザー Teaser/shots.yaml、10 カット）: 撮り直しが要るカットがある。
  - ゲームの素材が変わった（M_AwningRed）。それが画面に写っていたカットだけが古い:
    - preview: s01_windmill, s04_market, s06_oak_light, s10_hill_title
  撮り直すのはこのカットだけで、ほかのカットは前の take を使い回す: `shotpilot --file Teaser/shots.yaml render preview`（UE を起動する。数分かかる）。
  手順は Skill「shotpilot-teaser」にある。ライト・配置・コードの変更は検知の対象外。
  ```
  （サンプルのゲーム KettleHill で、市場の日よけの色を変えたあとのセッションの始め。Claude は Skill の手順で status を見て、UE を起動する前に確かめ、任せると `preview: 4 rendered, 6 reused` で撮り直して Cut をつなぎ直した）
  shots.yaml を直したときは、画が変わったカットだけを挙げる。目的や並び、音だけの変更なら「撮り直しは無い」と言う。
- **Skill**（`.claude/skills/shotpilot-teaser/SKILL.md`）には、`status` で古いカットを見る → 古いカットだけ `render` → `cut` でつなぎ直す、の手順と、してはいけないこと（`render/` や `state.json` を手で触らない）を書いてある。「ティザーを追いつかせて」と頼めば Claude はこの手順で進め、UE を起動する前にユーザーに確かめる。
- フックは知らせるだけで、撮影もファイルの書き換えもしない。どんな失敗でも終了コード 0 で黙る（ゲーム開発のセッションを止めない）。

## ハッカソン期間中に作ったもの

開発期間は 2026-11-02 12:30 〜 11-08 20:00。既存のプロジェクトのコードと設計は使っていない（D-05）。git の記録は次のとおり（決定の記録は `.claude/rules/product-decisions.md` の D-01〜D-99）。

| 日 | したこと |
|---|---|
| 11/03 00:27 | 最初のコミット。Python の試作（fingerprint・状態・previz・UE の撮影スクリプト、約 2,000 行） |
| 11/03 02:14 | コアを Rust に移し（D-16）、02:24 に Python 版を消した（UE の中で動く `sp_entry.py` だけが残る）。UE 5.8 で build → still → preview → pull → final → cut を実機で通した |
| 11/03 | Tauri のアプリ、音（BGM と SE、D-19）、工程バー、ムードボード、ロケハン、一気に作る（D-28） |
| 11/04 | ホーム画面（D-30）、台本が先の流れ（D-31）、実測したティザー 701 本からの型（D-29）、絵コンテ（D-33） |
| 11/05 | ゲームの変化の検知（D-37）、カット表での確認（D-36）、previz で止める（D-38）、企画（D-45）、構成画（D-46）、発信計画（D-47）、書体と動き（D-52、D-53） |
| 11/05〜06 | サンプルのゲーム4本（D-54、D-61）、カメラ効果（D-57）、チップで積む指示（D-62）、ノードのキャンバス（D-63）、音を作品ごとに変える（D-64）、不変条件をテストに結ぶ（D-65）、Claude Code との連携（D-66） |
| 11/06〜07 | サンプルの場所の作り込み（D-69）。構造の点検、GeometryScripting の形、Niagara の効果集、ジャンルごとの動き、4本のティザーの撮り直し |
| 11/07 | Three.js のサンプルゲーム（D-72）。UE を持たない人が、同じ病棟のティザーを Edge か Chrome で撮れる。画も作り込んだ（質感・AO・被写界深度・霧、D-73） |
| 11/07 夜 | 工程バーを「道」に替え、人が確かめる「あなたの番」の画面（D-80）。上から見た配置図でカメラを手で動かす（D-81）。シンプルとプロ（D-82）。場所ごとの環境音（D-83） |
| 11/08 未明 | 任せる場所を左下の1か所に（D-84）。既存のレベルを書き換えずに読むロケハン（D-85）と、配置図への見取り図（D-86）。カット検査の指摘をまとめて直す（D-91）。仮撮りを陰影のあるグレーボックスに（D-92） |
| 11/08 | 仮撮りをゲームの灯りで照らす（D-93）。作業中も頼むことを列に積む（D-94）。指摘を右端の1列に集める（D-95）。仮撮りの印を画から外す（D-96）。3D の見方（D-97）。最初に進め方を選ぶ（D-98）。左の欄の入力の箱（D-99） |
| 11/08 夕方 | Three.js のサンプルを、暗い病棟から晴れた午後の丘の上の村 KettleHill に替えた（D-101）。何が写ったかがひと目で分かるように |

## Claude の使い方

### 製品の中で

Claude の呼び出しは役割ごとに分け、Claude が書いてよい範囲はプロンプトではなくコードで強制する。接続は、ログイン済みの Claude Code を headless で呼ぶのが既定（`claude -p`。API キーも使える）。

| 役割 | 何を渡し、何を返させるか | コードが守ること |
|---|---|---|
| 企画（`discover`、D-45） | 最初のプロンプト。欠けがあるときだけ質問フォーム（5問まで） | ルックとスキルの id は実在を確かめる。発売日などの事実は作らせない |
| 台本と構成（`plan`） | ブリーフ・骨格・規則・距離表・ロケハン写真・ムードボード | ビート順・尺・読み時間はコードで整える。自己検証の語を書かない（D-22） |
| 修正（`revise`） | 指したカットと、plan と同じ凍結した仕様（D-48） | 変えてよいカットとキーをコードで絞る（カメラだけ・光だけ・音だけ・文字だけ） |
| 音の設計 | ライブラリの一覧、ブリーフとルックの音の性格 | ほかのティザーが使う音に印を付ける（D-64） |
| 場所選び（D-45、D-46） | ロケハンの写真と構成画 | 選べる写真の id はコードで確かめる |
| 撮影後の確認（D-50） | 初回の撮影の中央フレーム | 聞くのは「被写体が写っているか」だけ。直すのは1回 |
| チップの予測（D-62） | ロケハンで読んだ場所の記述 | effort は `low` に固定。何も書かない |

どの経路に何が届くかは `every_path_that_writes_shots_gets_the_frozen_spec` が固定し、Claude を呼ぶ関数が全部その表に載っているかは `every_call_to_claude_is_in_the_spec_table_or_says_why_not` がソースから拾って確かめる。

### 開発で（Claude Code）

- 規約の正本は `AGENTS.md`（`CLAUDE.md` はそれを読み込むだけ）。不変条件の各項目には、守るテストの名前と、コードで確かめられない部分（「人間が守る:」）を書く。名前の無い項目や実在しないテストがあると `every_invariant_names_the_tests_that_keep_it` が落ちる。
- 部分修正ループは `any_sequence_of_edits_rerenders_exactly_the_shots_whose_picture_changed` が、編集・元に戻す・撮影・ゲームの変化・承認のランダムな列（60 手 × 6 通りの種）で確かめる。期待値は「何が画を変えるか」だけから出す。実装をわざと5通りに壊して、どれでも落ちることを確かめた（D-65）。
- フック（`tools/hooks/dev_guard.js`）: 画面の文言を足して英語の辞書に無ければ、その場で Claude に知らせる。ループを支えるファイルを変えたターンは、終わる前にループのテストを走らせ、落ちていればターンを終えずに戻す。
- 自動メモリは切り、判断は `.claude/rules/product-decisions.md` に1行ずつ残す。UE 側の規約（`.claude/rules/ue5-cli.md`）は、UE を触るファイルを開いたときだけ読み込まれる。
- サブエージェントは既定で sonnet。変更を不変条件に照らして読む読み取り専用の `invariant-reviewer` を置いている。

## 失敗と直し

実機で見つけた失敗を、仕組みで再発させないようにした記録（詳しくは各 D-）。

| 症状 | 原因 | 直し | 守るテスト |
|---|---|---|---|
| ロケハン写真の約半分が真っ黒（Lanternless で 20 枚中約 10 枚） | 室内で、アンカーから一定の距離を引くと壁の中から撮っていた | UE が壁の手前へ寄せ、寄せた比でレンズを広げる（D-55） | `a_scout_camera_the_engine_pulled_in_front_of_a_wall_is_recorded_where_it_stood` |
| マネキンを1体直すと 10 カット全部が撮り直し | 影の描画でも「写っていた」と数えていた | 視野に入り、遮られずに見えるものだけを数える（D-56） | `a_game_change_rerenders_only_the_shots_that_show_it` |
| 人物の寄りや風車の羽根を狙ったカットが、足元や根元を写す | アンカーの点が基準点（足元・根元）だった | 狙う高さ `camera.aim` を足した。書かないカットは前と同じ画（D-60） | `aiming_at_the_upper_part_rerenders_that_shot_only_and_base_is_as_before` |
| 4本のティザーで同じ音が使い回され、ストラテジーにホラーの曲 | 音の設計だけ、企画とルックの音の性格が渡っていなかった | 同じ仕様を渡し、ほかのティザーが使う音に印を付ける。Claude を呼ぶ全経路を表と照らすテストを足した（D-64） | `the_next_teaser_is_told_which_sounds_another_game_took`、`every_call_to_claude_is_in_the_spec_table_or_says_why_not` |
| 不変条件 38 項目のうち、守るテストの名前があったのは 9 項目 | 文章で書いた決まりを、どのテストも見ていなかった | 全項目にテストの名前を書き、足りない 29 本を足した（D-65） | `every_invariant_names_the_tests_that_keep_it` |

## 仕組み

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
| 安いプレビュー | `previz`（エンジン不要。読んだレベルや three のゲームを、ゲームの灯りと影・霧・露出・材質の色で描く。灯りを知らないレベルは灰色の立体。6ショットで数秒）→ `still`（1ショット3フレーム）→ `preview`（半解像度）→ `final` |

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
- 台本が先（D-31）: 文字がティザーの背骨になる。`script:` に、ショットの間に入るカード（冒頭の一人称、世界の決まり、黒の間、明滅するタイトル、最後のメッセージ）と、ショットに重なる字幕・数え上げを書き、各ショットはその1行を見せる証拠として選ぶ。音は区間ごとの曲（`audio.music`、from/until に台本やショットの id）と、行やショットに付く SE で組み、Cut で -16 LUFS にそろえる。色調（`finish.grade: horror`）と露出もここで焼く。
- 初回は「一気に作る」で、制約を全部渡して見られる1本まで出す。足りないロケハン写真・ステージの理解・ムードボードを埋め、Claude に構成と仕上げ（字幕・つなぎ・タイトル・エンドカード）を1回で組ませ、previz で撮って仕上げ込みで Cut する（CLI は `shotpilot produce`）。そのあとが検証の段で、1カットずつ確かめて承認し、気になるところだけ指して直す（直したカットだけが撮り直る）。
- キャンバスの上には、トレーラー制作の工程どおりに「0 ロケハン → 1 ブリーフ → 2 構成 → 3 構図 → 4 撮影 → 5 仕上げ」が並び、それぞれの状態（未記入、P0 の数、承認数、final の数、cut の有無）を出す。
  - **ロケハン**: プロジェクトの中に何があるかを集める。各アンカーを4方向から撮った写真（mock は previz、UE は still。`scout/` に置く）と、UE の Asset Registry から寸法付きで棚卸しした mesh の一覧（`.shotpilot/assets.json`）から、ショットに出せるもの（`props:`）を選ぶ。写真は「構成を組む」ときに各アンカー1枚ずつ Claude にも見せる。CLI は `shotpilot scout`、`shotpilot assets --scan`、`shotpilot prop add <name> <asset>`。
  - **ブリーフ**: 種類（ティザー / ゲームプレイトレーラー）、ジャンル、コア体験、対象、必ず見せるもの、参考トレーラーを書く。`shots.yaml` の `brief:` に入り、撮り直しは起きない。
  - **ムードボード**（ブリーフの中）: 色・光・空気感の参照画像を置く。手持ちのスクリーンショットやコンセプトアートをドロップするか、ブリーフから4枚（場所・ディテール・不穏・タイトル）を生成する。生成は既定でログイン済みの Codex CLI（`codex exec`、読み取り専用で起動し、`~/.codex/generated_images/` から画像を回収。キー不要。4枚で約3分）を使い、無ければ OpenAI の API キー（既定 `gpt-image-2`、`SHOTPILOT_IMAGE_MODEL` で変更）を使う。接続設定で切り替えられ、CLI は `shotpilot moodboard --direction "..."`。画像は `moodboard/` に置かれ、`brief.images` に並ぶ。「構成を組む」ときに Claude に画像として見せる（雰囲気の参照で、ショットの画はすべてゲームから撮る）。API キーで生成するときのキーは接続設定で入れるか `OPENAI_API_KEY`。
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
- 「仕上げ」の「デザイン」では、帯（2.39 など）、タイトル（title ビートのショットに重ねる）、エンドカード（最後に足す黒い画面）、ショットごとの字幕と、次のカットへのつなぎ（カット / 黒をはさむ / 白をはさむ）を決める。`shots.yaml` の `finish:` と各ショットの `caption` / `transition` に書かれ、Cut のときに ffmpeg で焼き込まれる。撮り直しは起きない。クロスフェードは尺が縮んで SE がずれるので入れていない。
- 「仕上げ」の「サウンド」では、選んだ BGM / SE の音量・位置・フェードを直し、ミックスを試聴し、サウンドチェックの結果を見る。サウンドチェックは、プロのローンチ映像7本と比べた採点・直す順の一覧・ラウドネスの流れを出す。指摘ごとの「Claude に頼む」で、その指摘を会話に渡せる。
- 「サウンド」を開いて会話すると「音だけ」の指示になる。Claude が返した編集から、コードが `sfx` と `audio` だけを取り込む（カメラは変わらない）。
- アプリと CLI は同じ `shots.yaml` を読み書きする。どちらから直しても数秒で反映される。
- 中ではコアのライブラリが 127.0.0.1 の空きポートでだけ待ち受け、ウィンドウがそこを表示する。別プロセスは立てない。

## もう少し試す（UE なし）

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

### サンプルのゲーム

手元に UE5 のゲームが無いときに試すための、グレーボックスのゲームを自分で作れる。ゲームの仕組みは無く、テンプレートのキャラクターで歩けるだけ。4本ある（D-54、D-61）。ホラーの `Lanternless`（夜の閉じた病棟）、コージーの `KettleHill`（丘の上の茶屋の村。アンカーなしで、場所選びを Claude に任せる）、アクション・アドベンチャーの `AshenPillars`（灰の降る廃神殿）、ストラテジーの `Gridhold`（平原の砦）。どれも UE 5.8 の Third Person テンプレートから作り、Epic のアセットはリポジトリに入れない。以下は Lanternless の手順（ほかの3本も同じスクリプトで組む）。

1. UE 5.8 のエディタで、新規プロジェクトを「ゲーム → 三人称 → Blueprint」、スターターコンテンツ無し、名前 `Lanternless` で作る。
2. 上の4つのプラグインと、効果を組むための実験的なプラグイン `ToolsetRegistry` と `NiagaraToolsets`（Epic 同梱）を有効にして、エディタを再起動する。後の2つが無いと、効果（火・煙・霧など）の無いレベルになる。
3. 「ファイル → Python スクリプトを実行」で `tools/ue_sample/build_graybox.py` を選ぶ。`/Game/Lanternless/Maps/Lvl_Ward` ができ、起動マップになる。再生すると歩ける。エディタを開かずに組むなら、コマンドレットで同じスクリプトを走らせる（エディタの起動を含めて1〜2分）。

   ```bash
   UnrealEditor-Cmd.exe C:/path/Lanternless/Lanternless.uproject -run=pythonscript -script=C:/path/shotpilot/tools/ue_sample/build_graybox.py -unattended -nopause -NullRHI
   ```

4. アプリのホームで UE5 を選んで `.uproject` とマップを指定する（または `init --engine ue5 --uproject ... --map /Game/Lanternless/Maps/Lvl_Ward`）。アンカーは `SP_` の付いたアクター（受付、ストレッチャー、空の椅子、倒れた患者、廊下の奥の人影）。

マネキンは、ジャンルごとの動き（`tools/ue_sample/graybox/moves_<ジャンル>.py`、9〜12 個）で動く。Lanternless なら、廊下の奥の人影がよろめきながら近づき、壁に向かって揺れる患者、床を這う患者、戸口から覗く患者、クリップボードに書く看護師がいる（歩くものはレベルの自動再生シーケンスが動かす）。動きは全コマを撮る `preview` と `final` で写る。`still` は飛び飛びの3コマなので、レベルの中の時間がほとんど進まない。

マップは普通のアセットなので、組んだあとはエディタで手を入れてよい。スクリプトは既存のマップを上書きしない。作り直すときは引数に `--rebuild` を付ける（`/Game/<ゲーム>/` を `<プロジェクト>/Saved/GrayboxBackups/<時刻>/` に写してから組み直すので、手で直した分はそこに残る）。`--audit` は組まずに構造の点検（z-fighting、浮いた部品、壁にめり込んだ体）だけを出す。組み直したら、ティザーは `shotpilot render <品質> --force` で撮り直す（レベルの配置の変化は、ゲームの変化の検知に入らない）。

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
- モデルの既定は `claude-opus-5-5`。`SHOTPILOT_MODEL` で上書きできる。effort（どこまで考えるか）は `low` / `medium` / `high` / `xhigh` / `max` から選べ、既定は `medium`。アプリでは入力欄の下のモデル名から開く接続設定で変え、CLI では `SHOTPILOT_EFFORT=high` のように環境変数で変える（環境変数が優先）。Claude Code 経由（`--effort`）でも API 経由（`output_config.effort`）でも同じ値が効く。Messages API を直接呼ぶ（ストリーミング、構造化出力、拒否されたときはサーバー側で既定のモデルに切り替える `fallbacks: "default"`）。
- 弱いモデルでも型を再現できるように、構成の骨格（何カット・各何秒・どのビートか）は選んだ流行りの型の実測から作り、返答の崩れ（JSON の前後の文、語彙のゆれ、文字の空の行）はコードで直す（D-68）。OpenAI 互換のサーバー（vLLM・Ollama・llama.cpp・OpenRouter など）は `SHOTPILOT_PROVIDER=openai` と `SHOTPILOT_OPENAI_BASE_URL` で使える。`SHOTPILOT_CANDIDATES=3` にすると、plan を3案書かせてコードの採点で1つを残す（呼び出しは3倍）。モデルごとの再現の度合いは `shotpilot eval` で測る（`eval/README.md`）。

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
| `eval [--models a,b] [--runs N] [--candidates N]` / `eval --replay <dir>` | plan を題材ごとに書かせ、実測の型との近さをコードで採点する（D-68、`eval/README.md`） |
| `claude install [--dir <game>] [--force] [--print]` | ゲームのフォルダに Claude Code の Skill とフックを入れる（D-66） |
| `hook session-start\|post-tool` | Claude Code が呼ぶフック。古くなったカットを知らせるだけで、常に終了コード 0 |

## 開発

```bash
cargo test                 # UE も API キーも要らない（mock エンジンと差し替えクライアントで回す）
cargo clippy --all-targets # 警告ゼロを保つ
node tools/i18n_extract.js # 画面の文言のうち、英語の辞書に無いものを出す
```

fingerprint は Python 版（〜0.1）と同じ正規化 JSON を hash するので、Python 版で撮った UE の take はそのまま再利用される。previz だけは描画が変わったので撮り直しになる（数秒で終わる）。

規約は `AGENTS.md` に、判断の記録は `.claude/rules/product-decisions.md` にある。

## 出典とクレジット

- ルックとスキルは汎用で、実在のゲームやスタジオの名前・ロゴ・色を写さない。
- 書体は Google Fonts のもの（OFL。Kosugi・Kosugi Maru・Rock Salt は Apache 2.0）。使うときに取ってきてキャッシュし、リポジトリにもバイナリにも入れない。
- サンプルのゲームは Epic の UE 5.8 Third Person テンプレートのアセット（マネキン、試作用の箱）で作る。アセットはリポジトリに入れず、組み立てのスクリプト（`tools/ue_sample/`）だけを置く。効果・形・動きもエンジンと Epic 同梱のプラグイン（Niagara、NiagaraToolsets、GeometryScripting）でコードから作り、外部の素材は使わない。
- 効果音と曲は、作者が Splice で購入したサンプルを手元のライブラリ（リポジトリの外）から使う。リポジトリには入れず、再配布しない。`shotpilot init` の仮の音（`drone.wav` など）はコードで合成したもの。
- Three.js（MIT License, Copyright © 2010-2024 three.js authors）の 0.170.0 を `crates/shotpilot/three/` に同梱し、バイナリに埋め込む（ライセンスは `three/LICENSE-three.txt`）。サンプルの村（`threegame.rs`）は箱・円柱・球・マネキンをコードで組んだもので、質感（芝・瓦・漆喰・木・布）も描画のページがノイズから焼く。外部の素材は使わない。仮の音（オルゴール・風と小鳥・チャイム）もコードで合成している。
- ムードボードと構成画は OpenAI の画像生成（参照のみ）。ティザーの画はすべてゲームから撮る。
- 実測したティザー（`references/teasers/`）は、長さ・カット・明るさなどの測定値と分析だけを置く。動画とフレーム画像は入れていない。

Shotpilot 自体のライセンスは MIT（`LICENSE`）。移植・同梱・翻案した第三者のもののライセンス表記は `THIRD_PARTY_NOTICES.md` にまとめた。
