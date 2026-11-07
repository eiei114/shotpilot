---
name: run-app
description: shotpilot のデスクトップアプリ（shotpilot-app）と CLI をビルドして、アプリのウィンドウを開く。「ビルドして起動」「アプリを開いて」「動かして見たい」「最新で立ち上げ直して」と言われたときに使う。プロジェクトのパスを渡すとそのプロジェクトで開く。
allowed-tools: PowerShell(& *run.ps1*), Bash(powershell *run.ps1*), Bash(pwsh *run.ps1*)
---

# アプリをビルドして起動する

手順は `run.ps1` に決め打ちにしてある。手で cargo を打たず、これを1回呼ぶ。

```powershell
& "$env:CLAUDE_PROJECT_DIR\.claude\skills\run-app\run.ps1"                 # release でビルドし、ホームを開く
& "$env:CLAUDE_PROJECT_DIR\.claude\skills\run-app\run.ps1" <shots.yaml か、そのフォルダ>   # そのプロジェクトで開く
& "$env:CLAUDE_PROJECT_DIR\.claude\skills\run-app\run.ps1" -Dev            # debug ビルド。コンソールにログが出る
& "$env:CLAUDE_PROJECT_DIR\.claude\skills\run-app\run.ps1" -NoBuild        # ビルドせず、前のビルドを開く
```

Bash の道具から呼ぶときは `pwsh -NoProfile -File .claude/skills/run-app/run.ps1 [引数]`。ビルドは数分かかることがあるので、timeout は 600000 にする。

## スクリプトがすること

1. 開いている `shotpilot-app.exe` を閉じる。Windows は動いている exe を差し替えられないので、閉じないとビルドが失敗する。shots.yaml は編集のたびに書かれているので、閉じても定義は失われない（盤面の配置はブラウザの保存領域にあり、これも残る）。
2. `cargo build -p shotpilot -p shotpilot-app`（既定は `--release`）。CLI も同じコアで一緒にビルドする。失敗したら `Compiling` の行を除いたエラーの末尾だけを出して止まる。
3. ユーザーのホームフォルダを作業フォルダにしてアプリを起動する（今いるフォルダの shots.yaml を勝手に開かないため）。3 秒後にまだ動いているかを確かめ、すぐ落ちていたら止まる。

## 使い分け

- 引数の無い起動はホーム（最近のプロジェクト・新規作成）。サンプルのゲーム（D-54）を開くなら `C:\Users\Keisu\Projects\ShotpilotSamples\<名前>\teaser` を渡す（Lanternless・KettleHill・AshenPillars・Gridhold）。
- アプリが起動直後に落ちる、画面が出ないときは `-Dev` で開き直す。release は `windows_subsystem = "windows"` でコンソールを持たないため、エラーが見えない。
- ビルドが失敗したら、出たエラーを直してからもう一度このスクリプトを呼ぶ。警告の数が出たら `cargo clippy --all-targets` で見る（AGENTS.md は警告ゼロを保つ）。

## 報告

ユーザーには、ビルドした種類（release / debug）、開いた先（ホームかプロジェクト）、前のウィンドウを閉じたかどうかを1〜2文で伝える。ビルドが失敗したときは、失敗したことと最初のエラーを原文のまま示す。
