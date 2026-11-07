@AGENTS.md

## Claude Code 向けの補足

- 規約の正本は `AGENTS.md`。ここに同じことを二重に書かない。規約を変えるときは `AGENTS.md` を直す。
- 自動メモリは切っている（`.claude/settings.json`）。残したい判断や知見は `.claude/rules/product-decisions.md` に追記してコミットする。
- UE 側の規約は `.claude/rules/ue5-cli.md` にある。`crates/shotpilot/ue/` と `engines.rs` を触るときだけ読み込まれる。
- テストやビルドのログは長くなりやすい。`cargo test 2>&1 | grep -E "test result|FAILED|panicked"` や `| tail` で絞ってから読む。
- サブエージェントへの委譲は、明示的に頼まれたときと、並列化しないと終わらない調査のときだけ使う。
- フック（`.claude/settings.json` → `tools/hooks/dev_guard.js`）が2つある。Edit・Write のあと、画面の文言が英語の辞書に無ければその場で知らせる（D-51）。D-13 のループを支えるファイル（fingerprint・state・content・workspace・model・yamlio・engines・script、ループのテスト、AGENTS.md）を変えたターンは、終わる前にループのテストを走らせ、落ちていればターンを終えずに戻す（1回だけ）。
- コミットの前に、変更を不変条件に照らして見てもらうときは `invariant-reviewer`（読み取り専用、sonnet）を使う。

# Compact instructions

圧縮するときは、次のものを残す。
- 変更したファイルと変更の要点
- 最後のテスト結果（失敗していればテスト名とエラー）
- 決まったことと未解決の課題

ツール出力の全文や、読んだだけで使わなかったファイルの中身は捨ててよい。
