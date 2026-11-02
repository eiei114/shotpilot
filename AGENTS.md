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
pip install -e ".[dev,ai]"        # 開発用インストール
python -m pytest -q               # 全テスト（UE 不要。mock エンジンで回る）
python -m pytest -q tests/test_loop.py -k only_changed
shotpilot init --engine mock      # サンプル shots.yaml を作る
shotpilot status                  # fresh / stale / out-of-sync の一覧
shotpilot render previz           # 古くなったショットだけをエンジンなしで描く
shotpilot set s03 camera.lens.focal_length=50
shotpilot cut                     # 既存 take を連結する（レンダはしない）
```

`cut` と `order` は ffmpeg を使う。`plan` と `revise` は `ANTHROPIC_API_KEY` を使う（モデルは `SHOTPILOT_MODEL` で上書きできる）。

## 構成

```
src/shotpilot/
  model.py        shots.yaml のスキーマ・検証・resolve（Definition → ResolvedShot）
  fingerprint.py  再レンダ判定。品質プリセット（previz/still/preview/final）
  state.py        .shotpilot/state.json と render/<shot>/<quality>/<fp12>/ の take 管理
  workspace.py    上の3つとエンジンを束ねる。全操作はここを通る
  engines.py      mock / ue5 バックエンド。共通の job/result JSON プロトコル
  sync.py         Sequencer → shots.yaml の逆同期（base / engine / ours の三方比較）
  previz.py       エンジンを使わないワイヤーフレーム描画（依存なしの PNG 書き出し）
  cut.py          take をショット順に連結してティザーにする
  ai.py           Claude によるショットリスト作成（plan）と部分修正（revise）
  geometry.py     UE 座標系のカメラ計算
  yamlio.py       コメントを保持する YAML 入出力（ruamel round-trip）
  templates.py    shots.yaml の雛形
  cli.py          argparse の CLI
  ue/sp_entry.py  Unreal の中で動くスクリプト（別ルールあり）
tests/            mock エンジンでループ全体を検証する
```

## 守ること（不変条件）

- **fingerprint にはピクセルを変えるものだけを入れる。** 並び順、`purpose`、メモは入れない。並べ替えやメモの修正で再レンダが起きてはいけない。
- エンジン側スクリプトが、同じ入力に対して違う出力を出すように変わったら `fingerprint.RENDERER_VERSION` を上げる。
- **take は fingerprint で管理し、上書きしない。** 定義を元に戻したら、古い take を再利用する。`render/` 以下を手で編集しない。
- `shots.yaml` を機械的に書き換えるときは、必ず `yamlio` の round-trip を通す。人間が書いたコメントとキー順を消さない。
- Claude（`ai.py`）が返した編集は、`shots.yaml` に書く前に resolve して検証する。編集してよいショットの範囲は、プロンプトではなくコードで強制する。
- `build` は、Sequencer 側で人間が加えた変更を黙って上書きしない（`--force` のときだけ上書きする）。
- ユーザー向けのエラーは `ShotpilotError` を投げる。CLI はトレースバックなしでメッセージだけを出す。
- 座標系は UE 準拠（cm、Z が上、X が前、Y が右、左手系、回転は度）。`geometry.py` は依存なしのまま保つ。

## テスト

- ループの挙動を変えたら、**どのショットが再レンダ・再ビルドされ、どれがされなかったか**をテストで確かめる。`tests/conftest.py` の `launches(ws)` で、mock エンジンの起動記録を読める。
- `test_only_changed_shot_rerenders` と `test_order_purpose_notes_never_invalidate` はデモの主張そのものにあたる。落としたまま先に進まない。
- テストから実際の UE や Claude API は呼ばない。UE は mock エンジンで、Claude は差し替えで代用する。

## スコープ（7日間）

- やる: UE5 のティザー映像1本。ティザーの型（`hook → establish → escalate → title`）を同梱する。1ビートに3バリアントを出すのは余力があればやる。
- やらない: Unity / Godot / Roblox 対応、エンジン抽象化レイヤ、TikTok / Shorts 向け短尺、複数尺への展開、GUI 統合。
- mock エンジンはテストとデモ用で、別エンジン対応の足場ではない。抽象化を先回りして作らない。
- 録画後の編集機能は作らない（差別化にならない）。差別化できるのは録画の前だけ。

## スタイル

- Python 3.11 以上。`from __future__ import annotations` を付け、型ヒントを書く。
- コード中のコメントと docstring は英語。ユーザー向けの CLI 出力は英語でもよい。サンプルの `shots.yaml` の中身は日本語でよい。
- ランタイム依存を増やさない（現状は YAML ライブラリだけ。`anthropic` は `ai` extra に入れる）。
- コミットは小さく分ける。push は人間の確認を取ってから行う。
