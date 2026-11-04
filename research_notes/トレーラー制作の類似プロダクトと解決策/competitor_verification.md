# shotpilot の近接競合の実機能検証（2026-11-05 時点）

検証日: 2026-11-05。凡例: [一次] = 公式・原典、[二次] = 第三者記事・検索スニペットのみ。WebFetch は本文を要約モデル経由で取得しているため、引用は要約を含む。

## 1. Epic 公式の Unreal MCP と AI Assistant（UE 5.7 / 5.8）

### Takeaway
UE 5.8（2026-06-17 告知）に実験的な MCP プラグインが入り、5.8 のリリースノートに「SequencerTools toolset（AI による Sequence の作成・編集）」の1行がある。Movie Render Queue の起動、Take Recorder、ショット単位の再レンダ、承認の仕組みを公式 MCP が持つ根拠は見つからなかった。5.7 の AI Assistant は会話型で、エディタを操作しないと第三者は書いている（一次未確認）。

### Cited Findings
- 5.8 は 2026-06-17 に告知。"The new MCP plugin connects LLMs directly to Unreal Engine" — [Epic forum: Unreal Engine 5.8 Released](https://forums.unrealengine.com/t/unreal-engine-5-8-released/2729274) [一次]
- リリースノート Editor 節 "MCP Server (Experimental)": "This plugin enables agentic AI systems to connect to the Unreal Editor." — [UE 5.8 Release Notes](https://dev.epicgames.com/documentation/unreal-engine/unreal-engine-5-8-release-notes?lang=en-US) [一次]
- 同リリースノート Sequencer 節: "Add SequencerTools toolset for AI-driven sequence creation and editing." 別項に "Added an assistant toolset for animation features." — 同上 [一次]
- 同リリースノート: "Clicking on the Render Movie button will now respect the Movie Render Queue job type (i.e. Movie Graph)."、Movie Render Graph は Production Ready。これは Sequencer UI 側の変更で MCP のツールではない — 同上 [一次]
- 公式ドキュメント「Unreal MCP in Unreal Editor」: MCP サーバーをエディタプロセス内に埋め込み、ローカル HTTP（既定 `http://127.0.0.1:8000/mcp`）で接続。"Learn to use this Experimental feature, but use caution when shipping with it."、API とデータ形式は予告なく変わりうる。ツール自体は持たず AllToolsets 等のプラグインで有効化する。名前が挙がるのは SceneTools, ActorTools, MaterialInstanceTools, ObjectTools, GASToolsets。このページに Sequencer / Movie Render / camera の記載はない — [Unreal MCP in Unreal Editor](https://dev.epicgames.com/documentation/unreal-engine/unreal-mcp-in-unreal-editor) [一次、取得は要約経由]
- 5.8 のフォーラムスレッドで、ユーザーが「MCP のドキュメントは不完全で、toolset registry を手動で有効にしないと何も動かない」と書いている（Epic の発言ではない）— 同 Epic forum スレッド [一次の場のユーザー投稿]
- 5.8.1 のノートに MCP のクラッシュ修正があり、「Assistant がスクリプト実行中に行った変更は1つのトランザクションにまとまらなくなった」という互換性メモがある — 同 forum スレッド [一次]
- 第三者の整理では、公式 toolset は GAS, Niagara, PCG, UMG, StateTree, Blueprint, 素材、エディタ制御（console/PIE/viewport）、自動テスト、Slate 検査など。Sequencer は「Animation, Control Rig, Sequencer」の1領域にまとまる。カメラ・レンダの toolset の記載はない — [awesomeskills: ue-official-mcp](https://www.awesomeskills.dev/fr/skill/tc-imba-ue-official-mcp) [二次]
- Epic 自身が Claude Code 用に公式スキルのプラグインを公開（1,890 stars、作成 2026-06-08、最終 push 2026-10-27）。中身は未確認 — [GitHub API: EpicGames/unreal-engine-skills-for-claude-code-plugin](https://api.github.com/repos/EpicGames/unreal-engine-skills-for-claude-code-plugin) [一次、説明文は null]
- Puget Systems の記事（2026-07-09、Kelly Shipman）"Unreal Engine MCP Hands-On: Testing AI Inside the Editor"。ティーザー: "Kelly put Unreal Engine 5.8's new MCP integration to the test, letting Claude drive the editor directly." 本文は取得できず、結果（得意・失敗点）は未確認 — [Puget Systems blog tag: MCP](https://www.pugetsystems.com/blog/tag/mcp/) [一次、ティーザーのみ]
- 5.7 の AI Assistant は 2025-11 リリース。ドキュメント Q&A、C++ コード生成、手順案内をエディタ内で行う — [Digital Production, 2025-11-12](https://digitalproduction.com/2025/11/12/unreal-engine-5-7-foliage-pcg-and-in-editor-ai/) [二次]
- 同 Assistant は "primarily a conversational assistant"、"No editor automation" とされる。ただし MCP を売るベンダー（StraySpark）の比較記事（2026-03-24）で、利害がある — [StraySpark](https://www.strayspark.studio/blog/ue57-ai-assistant-vs-mcp-comparison) [二次、利害あり]

### Inferences
- 公式 MCP は「エディタ操作を汎用に開く層」で、ショットの定義・差分判定・承認の層は持たないと推測できる。ただし SequencerTools の中身（カメラ作成や MRQ 起動の有無）は未確認なので、断定は避ける。
- Epic が Sequencer の AI 編集を公式 toolset に入れたことで、「Sequencer を AI が触る」こと自体は差別化にならない。差別化は shots.yaml の正本、fingerprint による部分再レンダ、承認の紐づけに置く必要がある。

### Gaps
- SequencerTools toolset が公開するツール名と引数（`describe_toolset` の実出力）。ドキュメントに記載がなく、5.8 エディタで実機確認するしかない。
- Movie Render Queue 起動、Take Recorder が公式 MCP に含まれるか（確認できず）。
- UE 6 と MCP に関する Epic の公式発言（今回の取得範囲では見つからず）。リリースノートは 357k 文字のうち約 200k 文字までしか読んでいない。
- Puget の記事本文。5.7 の AI Assistant の公式記述（5.7 リリースノートの一次確認はしていない）。
- unrealengine.com の 5.8 ニュース投稿は 403 で取得できなかった。

## 2. Aura（Ramen）

### Takeaway
Aura は Blueprint・アニメーション・検証エージェント中心の汎用アシスタントで、1.0（2026-09-09）の公式発表に Sequencer・シネマティクス・トレーラー制作の記載はない。Coplay 買収は確認済み、Flopperam は GitHub リポジトリ上の記載で確認できた。

### Cited Findings
- Aura は 2026-01-02 に Unreal 向けにローンチ、2026-02 に 12.0 beta。Telos 2.0（Blueprint）、アニメーション・リギングのツール、自律ループの Dragon Agent — [GamesBeat, 2026-03-16](https://gamesbeat.com/ramen-acquires-coplay-to-build-multi-agent-ai-assistants-for-game-engines/) [二次、報道]
- Ramen が Coplay（Unity 向け AI ツール）を買収。GDC の 2026-03-16 に発表、条件は非公開。Coplay のオープンソース Unity MCP（約 7k stars）を含む。"first single tool for both Unity and Unreal" は会社側の主張 — 同 GamesBeat、[Business Wire 2026-03-16](https://www.businesswire.com/news/home/20260316440483/en/) [二次/プレスリリース]
- Aura 1.0 は 2026-09-09 ローンチ。再設計 UI、プロジェクトメモリ、マルチプロジェクト、Verification Agent、Custom Agents / Dynamic Workflows、Mac 対応、Fab 出品 — [GamesBeat: Aura 1.0](https://gamesbeat.com/ramen-launches-aura-1-0-to-push-agentic-ai-in-game-development/) [二次、報道]
- 価格: Indie $10/月、Pro $40/月、Ultimate $200/月、Fab 版は $150 一括（MCP 永久利用と Aura 1年分を含む）— 同上
- この記事に Sequencer・シネマティクスの言及はない — 同上（記事内の不在確認）
- Flopperam の `unreal-engine-mcp` リポジトリの説明文: "This MCP is now owned by Aura!"。1,116 stars、最終 push 2026-06-26、archive されていない — [GitHub API: flopperam/unreal-engine-mcp](https://api.github.com/repos/flopperam/unreal-engine-mcp) [一次]
- Flopperam のホスト版サーバーはカメラカット付き Level Sequence をシネマティクス機能に挙げているという検索スニペットがある — [flopperam/unreal-engine-mcp](https://github.com/flopperam/unreal-engine-mcp) [二次、スニペットのみ、未取得]

### Inferences
- Aura は Flopperam MCP（Level Sequence とカメラカットのツールを持つ可能性）を取り込んでいるので、「AI が Sequencer にカメラとカットを置く」能力は持ちうる。ただしトレーラー用のワークフロー（台本、ショット単位の再レンダ、承認）の根拠は見つからなかった。

### Gaps
- Aura の公式サイト・docs・changelog（tryaura.dev）は取得していない。2026 年の機能履歴、Sequencer 対応の有無は未確認。
- Flopperam 買収の日付・条件の報道は見つからなかった。ai-market-watch の「2026-06 に買収」という記載は、日付の整合性に疑義があり採用しない。リポジトリ説明文だけが根拠。

## 3. Cutscene Agent（arXiv 2604.25318）

### Takeaway
快手（Kuaishou）の研究。UE に埋め込んだ MCP サーバー経由で、ネイティブの Level Sequence を自動生成する。人間の承認や変更ショットのみの再レンダは本文の範囲では述べられていない。コード公開は確認できなかった。

### Cited Findings
- タイトル "Cutscene Agent: An LLM Agent Framework for Automated 3D Cutscene Generation"、2026-04-28 投稿（v1）、著者 Lanshan He ほか。Cutscene Toolkit（MCP）、director agent が animation / cinematography / sound design の subagent を統括、視覚推論のフィードバックループ、ベンチマーク CutsceneBench — [arXiv abs](https://arxiv.org/abs/2604.25318) [一次]
- 出力は "editable engine-native cinematic assets"。UE のネイティブ Level Sequence で、Sequencer で編集可能 — [arXiv HTML](https://arxiv.org/html/2604.25318) [一次、要約経由]
- ツールキットは "deliberately engine-agnostic" とされるが、実装は UE のみ — 同上
- カスタム subagent は "requiring no human intervention"。評価は LLM judge（Gemini 3.1 Pro）。人間の確認は失敗分析の "manual inspection" にとどまる — 同上
- Limitations: 対話中心のカットシーンが対象。アクション演出・群衆・動的環境は未対応。品質はアセットライブラリに制約され、TTS と顔アニメの外部サービスで遅延が出る。衝突検出は生成後に適用。創作タスクに正解データがない — 同上
- Future directions: 編集可能な出力形式で AI 生成から人間の仕上げへ受け渡す、とだけ書かれ、承認ステップは設計に入っていない — 同上
- プロジェクトページ `https://kuaishou-gamemind.github.io/cutscene_agent/` はリダイレクトのみで、GitHub リンクも "coming soon" の表示もなし — [project page](https://kuaishou-gamemind.github.io/cutscene_agent/) [一次]

### Inferences
- 対象はゲーム内カットシーン（対話劇）で、ティーザー映像の撮影・書き出しではない。再レンダの差分判定は論文内に出てこないので、shotpilot と直接競合しない可能性が高い。

### Gaps
- コード公開の有無（本文・プロジェクトページに記載なし。GitHub 検索はしていない）。
- HTML の約 93k 文字以降（付録）と PDF は未読（PDF はサイズ超過で取得できず）。付録に再レンダ・承認に関する記述がある可能性は排除できない。

## 4. Fab「Instant Movie Maker」とトレーラー自動生成プラグイン

### Takeaway
Instant Movie Maker（IMM）は、トレーラーマーカーから Level Sequence を生成し MRQ で書き出す Fab プラグイン。シード付きで再現可能だが、変更検出と部分再レンダの記載は見つからなかった。価格・対応バージョンは未確認。

### Cited Findings
- 説明: "rapidly generates, organizes, and renders level sequences"。シード付きショットのバリエーション、アクター追跡、衝突レポート、画像シーケンス出力（PNG/JPG/EXR）、任意の FFmpeg エンコード（既定オフ）、MRQ の自動設定。Movie Render Pipeline プラグインを自動で有効化 — [Fab listing](https://www.fab.com/listings/5eac9e24-50cb-429f-a222-011196be666a) [二次、Fab ページ本体は 403 で取得できず、検索スニペットのみ]
- 検索スニペット上、エディタ内利用が前提で、パッケージ前の無効化が推奨される。変更検出のスキップ機能の記載はなし — 同上、検索結果の要約 [二次、スニペットのみ]

### Inferences
- 「シードで同じショットを再生成できる」ことと「変更したショットだけを撮り直す」ことは別。IMM は前者に近く、後者の根拠は見つからなかった。

### Gaps
- 価格、対応エンジンバージョン、レビュー、更新日（Fab は 403）。
- ほかのトレーラー自動生成の Fab プラグインは、検索結果に出てこなかった。網羅的な調査はしていない。

## 5. オープンソースの Unreal MCP サーバー（Level Sequence・カメラ・MRQ）

### Takeaway
Sequence 作成、カメラ、MRQ レンダまで触れる OSS は複数ある。最大は ChiR24/Unreal_mcp（約 1.0k stars、2026-10-30 に push）。いずれも「ツールの箱」で、ショット定義の正本、差分判定、承認は持たない。

### Cited Findings
- ChiR24/Unreal_mcp（EngineWire）: 1,007 stars、作成 2025-09-07、pushed 2026-10-30、MIT、TypeScript + C++ の Automation Bridge — [GitHub API](https://api.github.com/repos/ChiR24/Unreal_mcp) [一次]
- README の内部ツール表に "Level Sequences, Movie Render Queue, media, Take Recorder, replays" とある。`manage_sequence` の中でカメラが扱われるかは README 要約から確認できず — [GitHub README](https://github.com/ChiR24/Unreal_mcp) [一次、要約経由]
- 公式サイトの説明: "Builds Level Sequences with cameras, cuts, keyed animation and particle triggers, and renders them with Movie Render Queue." cinematics/audio グループは 40 capability（Take Recorder を含む）。デモ "Slow Time" は 12 ショット、1,007 フレーム。UE 5.0〜5.8、Fab の Personal は無料 — [enginewire.dev](https://enginewire.dev/) [一次、製品サイト]
- Flopperam/unreal-engine-mcp: 1,116 stars、Aura に所有が移った（セクション2参照）
- GenOrca/unreal-mcp: 147 stars、pushed 2026-07-07、Apache-2.0。level_sequence ドメイン（camera cut トラックとキーフレーム）と viewport 画像取得のビジョンツール — [GitHub API](https://api.github.com/repos/GenOrca/unreal-mcp) [一次] と検索スニペット [二次]
- NAJEMWEHBE/unreal-ai-connection: 9 stars、pushed 2026-07-18、MIT、UE 5.7、143 ツール。Level Sequence のカメラカットと MRQ レンダを持つとスニペットにある — [GitHub API](https://api.github.com/repos/NAJEMWEHBE/unreal-ai-connection) [一次] と検索スニペット [二次]
- 3 stars の小規模プロジェクト: PoBruno/mcp-unreal-agent（sequencer を含む、2026-06-08）、CarlxsMG/UEAgent（2026-11-03）、77uinnoute/UnrealMCP（sequencer コマンド、UE 5.5〜5.8、2026-10-30）— [GitHub search API](https://api.github.com/search/repositories?q=unreal+mcp+sequencer&sort=stars&per_page=10) [一次]
- ほかに IvanMurzak/Unreal-MCP、remiphilippe/mcp-unreal（UE 5.7）、per-simmons/unreal-agent-harness などが検索に出たが、Sequencer 対応は未確認 — 検索結果 [二次、スニペットのみ]

### Inferences
- 「AI が Sequence を組み、MRQ を叩く」層はすでに OSS で埋まっている。shotpilot が MCP を使わず shots.yaml と専用エンジンスクリプトで駆動する設計は、この層とは別の価値（再現性・差分・承認）で説明するのが妥当。

### Gaps
- ChiR24 の `manage_sequence` の action 一覧（docs への直リンクが 404）。カメラ作成と MRQ 起動がどの action かは未確認。
- 各リポジトリで fingerprint 的な差分再レンダを持つかの個別確認はしていない（README 要約の範囲では記載なし）。

## 6. 「実プロジェクトからのトレーラー生成 + AI 計画」「変更ショットのみ再レンダ」「ショット単位の承認」

### Takeaway
2025〜2026 年の検索範囲では、これら3点（AI による計画、変更ショットのみ再レンダ、画に紐づく承認）を同時に満たす製品・OSS は見つからなかった。「見つからなかった」ことは存在しない証明ではない。

### Cited Findings
- トレーラー専用プロジェクトの検索（「game trailer AI Unreal Claude MCP Sequencer」等）では、汎用 MCP サーバーのみが出て、ゲームトレーラー専用のものは出なかった — 検索結果の要約（2026-11-05 実施）[二次]
- 「変更ショットのみ再レンダ」で OSS を探す検索も該当なし。IMM と Sequencer の Shot Track/Takes の説明だけが出た — 同上 [二次]
- GitHub の検索クエリ `unreal trailer shots sequencer ai` は 0 件 — [GitHub search API](https://api.github.com/search/repositories?q=unreal+trailer+shots+sequencer+ai&sort=updated&per_page=10) [一次]

### Inferences
- 近接競合は「(a) 公式 MCP + SequencerTools、(b) Aura / EngineWire などの汎用 MCP、(c) IMM のようなシード付き自動生成、(d) Cutscene Agent のような研究」の4系統。どれも差分判定と承認の記載は確認できなかった。

### Gaps
- Product Hunt、X/Twitter、Epic forum の個別検索はしていない（予算内で未実施）。
- 非 Unreal（Unity 等）やクラウド型の動画生成サービスは対象外。
- 未確認の事項は、公式 MCP の SequencerTools の中身、Aura の changelog、IMM の価格と対応バージョン、Puget 記事の本文。提出前に UE 5.8 実機で `list_toolsets` / `describe_toolset` を叩けば最も確実に埋まる。
