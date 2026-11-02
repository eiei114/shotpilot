---
paths:
  - "src/shotpilot/ue/**"
  - "src/shotpilot/engines.py"
---

# UE5 連携の規約

## 起動経路（実測済み・D-09）

- Editor の GUI は開かない。`UnrealEditor-Cmd` から CLI で動かす。owner が sharehouse-horror で本番運用している経路なので、リスクとして扱わない。
- `build` / `pull` / `scan` は `-run=pythonscript -script=<sp_entry.py>` で動かす（commandlet で軽い）。
- `render` は `-ExecutePythonScript=<sp_entry.py>` で動かす。フルエディタを起動して MRQ を PIE executor で回し、終わったら終了する。
- 既定では、隔離した `-UserDir` で起動する（`project.isolate_userdir`）。owner の普段のエディタ設定を汚さない。
- Remote Control HTTP は使わない（Beta 扱いで、Sequencer 編集の汎用 API ではないため）。

## `ue/sp_entry.py`

- Unreal 同梱の Python で動く。**`shotpilot` パッケージを import しない**。標準ライブラリと `unreal` だけを使う。
- 入力は環境変数 `SHOTPILOT_JOB` が指す job JSON。結果は `<job>.result.json` に書く。プロトコルを変えたら、`engines.py` の `MockEngine` も同じように直す。mock と ue5 の挙動がずれると、テストが嘘をつく。
- Sequencer / MRQ の Python API はエンジンのバージョンで名前が違う。直接呼ばず、`_call_first(obj, [新名, 旧名])` のフォールバックを通す。
- MRQ の非同期コールバックの間に UObject が GC されないよう、`_KEEP` に参照を置いておく。
- 例外は握りつぶさない。result JSON にエラーとトレースバックを入れて返す。
- 出力のピクセルが変わる修正をしたら、`fingerprint.RENDERER_VERSION` を上げる。

## 検証

- `sp_entry.py` はユニットテストできない。プロトコル側の挙動は、mock エンジンのテストで担保する。
- 実機の UE 起動は重いので、実行する前に人間に確認する。
- ショット単位の再レンダ（MRQ のジョブをショットごとに分ける構成）を崩さない。マスターシーケンス1本をまとめてレンダする形に戻さない。
