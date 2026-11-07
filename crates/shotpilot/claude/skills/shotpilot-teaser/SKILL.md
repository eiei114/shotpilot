---
name: shotpilot-teaser
description: このゲームのティザー映像（Shotpilot、{{SHOTS}}）をゲームの変更に追いつかせる。ゲームのアセットや shots.yaml を変えたあと、どのカットが古くなったかを調べ、古いカットだけを撮り直して Cut をつなぎ直すときに使う。
allowed-tools: Bash(shotpilot *), PowerShell(shotpilot *)
---

# ティザーをゲームに追いつかせる（Shotpilot）

このゲームには、Shotpilot で撮ったティザーがある。定義は `{{SHOTS}}` の1ファイルで、撮った画（take）は `render/` に、何を撮ったかは `.shotpilot/state.json` にある。Shotpilot は、画が変わったカットだけを撮り直す。

コマンドはどれも、ゲームのフォルダから `shotpilot --file {{SHOTS}} <コマンド>` の形で打つ。

## 1. どのカットが古いかを見る

```
shotpilot --file {{SHOTS}} status
```

品質（previz / still / preview / final）ごとに、各カットの状態が出る。

- `fresh`: いまの take がそのまま使える。
- `STALE`: そのカットの定義（カメラ・光・秒数など、画を変える項目）が変わった。
- `GAME`: そのカットに写っていたゲームの素材（.uasset など）の中身が変わった。写っていなかった素材の変更では古くならない。

セッションの始めと、ファイルを変えたあとには、フックが同じ事実を知らせる。

## 2. 古いカットだけを撮り直す

```
shotpilot --file {{SHOTS}} render previz      # エンジンを使わない確認（数秒）
shotpilot --file {{SHOTS}} render preview     # UE を起動して撮る（数分）
```

`render` は古いカットだけを撮り、ほかのカットは前の take を使い回す。出力の `rendered` と `reused` で、何を撮り直したかが分かる。

- `render preview` と `render final` は UE を起動し、数分かかる。実行する前にユーザーに確かめる。
- ライト・配置・コードの変更は、ゲームの変化として検知しない。それで画が変わったなら、ユーザーと相談して `render <品質> --force <カット>` で撮り直す。

## 3. つなぎ直す

```
shotpilot --file {{SHOTS}} cut
```

撮った take をつなぎ、音と仕上げ（文字・つなぎ・カメラ効果）を載せる。撮り直しは起きない。

## 4. カットを直したいとき

画を変える項目は `set` で直す（そのカットだけ古くなる）。

```
shotpilot --file {{SHOTS}} set s03 camera.lens.focal_length=50
```

言葉で直すなら `revise` を使う。Shotpilot の中の Claude が組み直し、変えてよい範囲はコードで守られる。ここからは確認の入力ができないので、ユーザーが直すことを頼んだときに `--yes` を付けて1回で打つ（`--yes` なしは差分を出すだけで、打ち直すと Claude が組み直すので結果が変わる）。直したあとは `render previz` で、そのカットの画を確かめる。

```
shotpilot --file {{SHOTS}} revise --shot s03 --aspect camera --yes "もう少し引いて、門と城壁を入れる"
```

並び・目的・メモ・音・文字（字幕や台本）の変更では、撮り直しは起きない。

## 5. ゲームの更新に合わせた動画

アップデートの告知動画は、このティザーのカットを引き継いで隣に作る。ゲームが変わったカットだけが撮り直しになる。

```
shotpilot --file {{SHOTS}} promo update v1.1
```

## してはいけないこと

- `render/` の中を手で書き換えたり消したりしない（take は上書きしない決まり。定義を戻せば前の take が使われる）。
- `.shotpilot/state.json` を手で書き換えない。
