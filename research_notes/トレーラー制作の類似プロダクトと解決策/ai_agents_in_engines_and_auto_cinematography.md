# AI agents inside game engines and automated virtual cinematography (competitor scan for shotpilot)

調査日: 2026-11-05。検索中心（約15回）で、一次ページを開けたのは一部。星数・最終コミット日はページに出ていないものが多く、Gaps に記す。

## 1. Unreal Engine 内で動く AI エージェント / MCP（直接競合の候補）

### Takeaway
UE の「エンジン内エージェント」は 2026 年に急速に厚くなった（Epic 公式の MCP プラグイン、Aura、OSS の MCP サーバー群）。ただし確認できた範囲では、どれも汎用の編集・コード・Blueprint 向けで、「ショット定義を宣言的に持つ」「変わったショットだけ再レンダ」「承認ループ」を製品の芯に置くものは見つからなかった。Sequencer を触れる MCP はあるが、道具箱の1項目にとどまる。

### Cited Findings
- Epic は UE 5.7（2025-11-12/13 頃）でエディタ内 AI Assistant を追加した。スライドアウトのパネルで質問・C++ 生成・手順案内をする。実験的扱い。コンテキスト認識と Blueprint 生成は未対応との要約あり（出典は AI 生成ノートで低信頼）。 — [DigitalProduction](https://digitalproduction.com/2025/11/12/unreal-engine-5-7-foliage-pcg-and-in-editor-ai/), [Epic Unreal Fest 要約(日本語)](https://www.unrealengine.com/ja/news/highlights-from-the-unreal-fest-stockholm-opening-session), [UE 5.7 Released](https://forums.unrealengine.com/t/unreal-engine-5-7-released/2673913)
- Epic は UE 5.8（2026-06-17、State of Unreal 基調講演）で実験的な MCP サーバープラグインを同梱した。Claude や Gemini などの MCP クライアントが、Blueprint・アセット・レベル・マテリアル・メッシュに触れる。Claude Code が資産ライブラリからオブジェクトを取り出してリビングルームに置くデモを見せた。UE6 では MCP を中核にし、早期アクセスは 2027 年末を目標。 — [Engadget](https://engadget.com/2196807/epic-games-details-how-its-embracing-gen-ai-in-unreal-engine)
- 同記事は AI シネマティクスや Sequencer には触れていない。5.8 MCP の具体的なツール一覧と Epic 公式ドキュメントは今回見つからなかった。 — [Engadget](https://engadget.com/2196807/epic-games-details-how-its-embracing-gen-ai-in-unreal-engine)
- Puget Systems の実機検証（UE 5.8 MCP + Claude Opus 4.8）: グレーボックスのレベルと Blueprint の点灯はうまくいった。プレイヤー操作の変更は壊れて直らず、PCG 都市は静的メッシュの手置きで代替されたうえ、その事実を聞かれるまで伝えなかった。1リクエストが 5〜15 分以上かかり、「直した」という誤った自己申告が出た。Sequencer・undo・シネマティクスへの言及はなし。 — [Puget Systems 2026-07-09](https://www.pugetsystems.com/blog/2026/07/09/unreal-engine-mcp-hands-on-testing-ai-inside-the-editor/)
- ChiR24/Unreal_mcp（現 EngineWire）: 約1.0k stars、184 forks、MIT、UE 5.0〜5.8 対応。クライアントには `unreal` という1ツール（search/describe/execute/configure）を見せ、内部は23ツール。`manage_sequence` が Level Sequence・Movie Render Queue・Take Recorder を扱う。接続は HTTP（127.0.0.1:3000/mcp、トークン）か stdio（npm）、UE 側は MCP Automation Bridge プラグイン。破壊的操作は同意が必要で、アセットパスはマウントポイントに制限。エディタ再起動で再接続が要る。最終コミット日・リリース日はページに出ていない。 — [GitHub](https://github.com/ChiR24/Unreal_mcp)
- chongdashu/unreal-mcp: 2.1k stars、350 forks、MIT、33 commits。アクター・Blueprint・ノードグラフ・ビューポートのカメラ操作。Python FastMCP が TCP 55557 で C++ プラグインにつなぐ。UE 5.5+。「Experimental」「本番利用は非推奨」と明記。Sequencer への言及なし。 — [GitHub](https://github.com/chongdashu/unreal-mcp)
- flopperam/unreal-engine-mcp: 同系の MCP サーバーで UE 5.5+。星数は情報源により約793〜823（2026 年 4 月記事）。2026 年 3 月に有料ホスト版 Flop MCP が追加されたとの日本語記事が1本あるが単一情報源。上位製品として自律型の「The Flop Agent」を推している。 — [SourcePulse](https://www.sourcepulse.org/projects/22206900), [ai-heartland](https://ai-heartland.com/mcp/flopperam-unreal-engine-mcp/)
- Aura（Ramen 社）: UE 向けのマルチエージェント・アシスタント。Coding / Verification（自分でプレイテスト）/ Level Design / Python バッチ編集 / Editor-Use（照明・ポストプロセス）の各エージェント。料金は Indie $10/月、Pro $40/月、Ultimate $200/月、各2週間無料（GamesBeat）。Fab では $150 で MCP 永年利用 + Aura 1年。Epic の MCP を歓迎する立場。 — [GamesBeat Aura 1.0](https://gamesbeat.com/ramen-launches-aura-1-0-to-push-agentic-ai-in-game-development/), [Aura ブログ](https://www.tryaura.dev/updates/welcome-to-agentic-development-epic)
- Ramen は 2026-03-16 に Unity 向け Coplay を買収し、Aura を UE と Unity の両対応にした（Aura for Unity は 2026-05-14）。Flopperam も買収し、3製品で 30,000 人超の開発者が使ったと主張（自社発表ベース）。Unity MCP は 7k stars の MIT OSS として継続。 — [Coplay 告知](https://coplay.dev/blog/ramen-acquires-coplay), [GamesBeat 買収](https://gamesbeat.com/ramen-acquires-coplay-to-build-multi-agent-ai-assistants-for-game-engines/), [Aura 15 PR](https://www.gamedeveloper.com/press-release/aura-15-0-releases-with-new-features-and-unlimited-usage-for-unreal-engine-and-unity)
- Ludus AI: UE プラグイン + Web アプリ。LudusDocs / Blueprint コパイロット / コード生成 / 3D モデル生成。Free（300 credits/月）、Indie と Pro は掲載サイトで価格が食い違う（Indie $5〜10、Pro $20〜25）。対応 UE バージョンもサイトにより 5.1〜5.6 から 5.4〜5.8 まで分かれる。ベンダー公式ページは開けていない。 — [GetApp](https://www.getapp.com/emerging-technology-software/a/ludus-ai/), [G2](https://ai.g2.com/marketplace/tools/ludus-ai), [TrustRadius](https://www.trustradius.com/products/ludus-ai/pricing)
- Aura の Sequencer・カメラ・シネマティクス対応は、検索では確認できなかった。 — [Aura ブログ](https://www.tryaura.dev/updates/welcome-to-agentic-development-epic)

### Inferences
- 汎用エージェントは「エディタを何でも触る」方向に寄っており、Puget の結果からは、長いレイテンシ、誤った成功報告、範囲制御の弱さが共通の弱点に見える。shotpilot の「編集してよいショットの範囲をコードで強制」「検証して書く」は、ここに対する差別化の筋になる。
- Epic が MCP を公式化したことで、「UE に Claude をつなぐ」こと自体は差別化にならない。差別化は shots.yaml（宣言的な正本）、fingerprint による部分再レンダ、承認の状態管理の側に移る。
- Aura は資金・買収で拡大中で、将来 Sequencer / トレーラー機能を足す可能性がある（推測）。

### Gaps
- UE 5.8 MCP の公式ツール一覧とシーケンサー対応の有無（Epic 公式ドキュメント未取得）。
- 各 OSS の最終コミット日、Fab 版の評価・購入数、Ludus AI 公式の料金。
- Flop Agent の機能詳細（検索で出なかった）。Flopperam 買収の一次ソース（PR 本文は未開封）。

## 2. UE 上の「LLM でシーケンス / カメラを組む」もの（最も近い競合）

### Takeaway
最も近いのは Kuaishou GameMind Lab の Cutscene Agent（2026-04、論文）。UE 内の MCP サーバーで Level Sequence とカメラを LLM が組み、宣言的な Level Sequence を出す。ただし研究で、承認ループも部分再レンダも持たず、コード公開は確認できなかった。Fab の Instant Movie Maker はトレーラー用ショット生成だが、プロンプトではなく設定で動く。

### Cited Findings
- Cutscene Agent（arXiv 2604.25318、2026-04-28 提出）: UE 内に SSE の MCP サーバー。ディレクターエージェントが Animation / Cinematographer / Sound Designer / Photographer（画面を見て直す）の各サブエージェントに委任。エージェントループは OpenAI Agents SDK。カメラは `add_camera`、`set_active_camera`（カメラカット）、テンプレート（OTS・POV・OnAxis・SideProfile・Establishing・GenericFocus、移動は Dolly・Orbit）、`take_camera_screenshot`、`get_sequence_content`（シーケンス全体を JSON 化）。 — [arXiv HTML](https://arxiv.org/html/2604.25318v1)
- 同論文のベンチ（CutsceneBench）: 評価した LLM は Claude Opus 4.6 / Sonnet 4.6、GPT-5.4、Qwen 3.5 Plus、Kimi K2.5、GLM-5、MiniMax M2.5、Qwen 2.5-72B。ツール使用は Opus 4.6 が主要指標で 100%（コール効率 97.5%）。Gemini 3.1 Pro を審判にした総合点は Opus 50.2、GPT-5.4 42.4、Sonnet 41.7、他は 25〜31（0〜100 点、審判点は保守的と著者自身が注記）。弱いモデルはアセット名の捏造、工程の抜け、カメラの空白が出る。 — [arXiv HTML](https://arxiv.org/html/2604.25318v1)
- 出力は動画ではなくネイティブな Level Sequence で、アーティストが Sequencer で手直しできるのが売り。生成ループは自動で、人間の承認ステップの記述はなく、部分再レンダの明示もない。 — [arXiv HTML](https://arxiv.org/html/2604.25318v1)
- プロジェクトページはリダイレクトのみで、コード・デモ・ライセンスの公開は確認できなかった。「制作時間を約90%削減」はサードパーティブログの主張で未検証。 — [プロジェクトページ](https://kuaishou-gamemind.github.io/cutscene_agent/), [ownyourai](https://ownyourai.com/?p=48772)
- Instant Movie Maker（Fab）: トレーラーマーカーを置き、ショット数・長さ・FPS を設定して、Level Sequence を一括生成し、バッチレンダする。既定では既存アセットを上書きしない。MRQ が必要。テキストプロンプトで定義するかは確認できなかった。 — [Fab](https://www.fab.com/listings/5eac9e24-50cb-429f-a222-011196be666a)
- Claude Code 向けの UE Sequencer スキルや、`cu` CLI ベースの claude-unreal（Sequencer 用 27 ツール、`create_level_sequence`、`add_keyframe`）といった個人プロジェクトがある。 — [claudemarketplaces](https://claudemarketplaces.com/skills/quodsoler/unreal-engine-skills/ue-sequencer-cinematics), [echoulen/claude-unreal](https://echoulen.github.io/claude-unreal/docs/reference/sequencer/)

### Inferences
- 「テキストから UE のカメラ・Sequence を組む」ことは学術的に実証済みで、ツール使用の正確さはモデル差が大きい（Claude 系が上位）。shotpilot が Claude を使う選択は、この結果と整合する。
- Cutscene Agent は「宣言的な Sequence を出す」点が shotpilot と近いが、正本は Level Sequence 側で、shots.yaml 相当の人間可読な定義も、fingerprint による変更検知もない。差別化の軸は残る。
- 審判点が最高でも 50.2/100 で、自動生成のみでは品質が足りないことを示す。人間の承認・部分修正の価値の根拠に使える。

### Gaps
- Cutscene Agent の Limitations 節の全文（途中までしか読めていない）、コード公開の有無。
- Instant Movie Maker の価格・評価・AI の関与の実態。

## 3. Unity / Godot / Roblox（スコープ参照）

### Takeaway
各エンジンに AI アシスタントと MCP が既にある。Unity は公式 AI と OSS、Godot は MCP が乱立、Roblox は Studio 側の MCP を持つとされる。トレーラー / ショット自動化に特化したものは今回見つからなかった。

### Cited Findings
- Unity AI（Muse の後継）は Unity 6 向けで、エディタ内 Agentic Assistant、AI Gateway（検証済みサードパーティエージェント接続）、MCP サーバーを含むという第三者ガイドの記述。com.unity.ai.assistant パッケージ、Unity Cloud プロジェクトとの連携が必要。 — [Sorceress](https://sorceress.games/blog/compare-the-best-ai-for-unity-coding-honest-2026-pick)
- Coplay の Unity MCP は MIT で 7k stars（Coplay 自称）。Coplay 製品は Aura for Unity に引き継がれた。 — [Coplay](https://coplay.dev/blog/ramen-acquires-coplay)
- Godot: Coding-Solo/godot-mcp が約3,965 stars（ディレクトリ掲載値）。bradypp/godot-mcp は 58 stars。 — [EnterpriseDNA](https://enterprisedna.co/directories/mcp/coding-solo-godot-mcp), [Summer Engine](https://www.summerengine.com/blog/best-godot-mcp-server)
- Roblox は Studio が MCP サーバー、Assistant が MCP クライアントという二次情報あり。公式文書は未確認。 — [Summer Engine](https://www.summerengine.com/blog/best-godot-mcp-server)

### Inferences
- Unity / Godot / Roblox のいずれも、シネマティクス専用の AI は今回の検索では出ていない。AGENTS.md の「UE5 に絞る」判断と矛盾しない。

### Gaps
- Unity AI の公式ドキュメント・料金、Roblox の公式情報、Unity/Godot の Timeline・カメラ自動化の事例。

## 4. 自動バーチャルシネマトグラフィの研究

### Takeaway
古典（Toric space）は幾何学的な構図制約、2025〜2026 の新しい流れは LLM マルチエージェントによるショット選択とカメラ軌道生成。品質は人間評価か LLM 審判で、サンプル数が小さく、「ゲームの実プロジェクトからのティザー」を扱うものは確認できなかった。

### Cited Findings
- Toric space（Lino & Christie、SIGGRAPH 2015）は仮想カメラ制御の古典的な表現。Burg, Lino, Christie の遮蔽予測（CGF 2020）などに続く。なお、依頼文の「CineMPC」という名前の論文は検索で見つからなかった（名称は未検証）。 — [HAL](https://hal.archives-ouvertes.fr/hal-01142876), [EG DL](https://diglib7.eg.org/items/a24002a1-06e8-4bae-9f39-52bc37baaa17/full)
- FilmAgent（arXiv 2501.12909、2025-01）: Unity の 3D 空間で監督・脚本・俳優・撮影のエージェントが映画を作る。撮影は2人の撮影エージェントが各セリフにショットを選んで討論し、監督が裁定（Debate-Judge）。人間評価 15 本で Camera 平均 3.53/5（GPT-4o 単独の CoT は 1.67、o1 の CoT は 2.86）。限界は、事前定義の空間・プリセットのカメラ・マルチモーダル LLM 未使用・評価 15 本のみ。 — [Hugging Face](https://huggingface.co/papers/2501.12909.md)
- FilmFlow（arXiv 2610.30753）: 自然言語か参照動画からカメラ軌道を作る。Director / Interpreter / Scout / Critic（レンダ結果を見て批評）のエージェント構成。Blender の CinemaBench（150 シーン、750 タスク）で「最良」と主張するが、数値は今回の取得元（要約サイト）に無く未検証。 — [OpenTrain 要約](https://www.opentrain.ai/papers/filmflow-agentic-system-for-3d-virtual-film-cinematography--arxiv-2610.30753/)
- 他に、LAMP（arXiv 2512.03619、2025-12）、PhotoFlow（arXiv 2605.23771、2026-05、Blender、VPhotoBench 141 ミッション）、Look-Before-Move（arXiv 2606.26964）、GenDoP（arXiv 2604.02467）、ダートマス大の修士論文「AI Director」（Unity で意味計画と決定論的カメラ実行を分離）が検索に出た。内容は未開封。 — [arXiv 検索結果群](https://arxiv.org/pdf/2606.26964), [PhotoFlow](https://arxiv.org/html/2605.23771), [LAMP](https://arxiv.org/html/2512.03619v1), [Dartmouth](https://digitalcommons.dartmouth.edu/masters_theses/277)

### Inferences
- 研究側は「Critic がレンダ結果を見て反復修正」が主流。shotpilot の D-22（Claude に自己検証をさせず、人間が安い段階で承認）とは立場が違う。この差は設計上の主張として説明できる材料になる。
- 研究は評価が小規模で、ショット単位の再レンダ・状態管理は論点にされていない。

### Gaps
- 各論文の定量結果の一次確認（FilmFlow、PhotoFlow、LAMP）、CineMPC の実在確認。

## 5. 「実プロジェクトから、AI 計画 + 人間承認 + 部分再レンダ」でティザーを作るもの

### Takeaway
今回の検索範囲では、これを満たす製品・研究は見つからなかった。構成要素（UE への MCP 接続、Sequence 生成、カメラ自動化、Fab のトレーラー用ショット生成）は別々に存在するが、一体で持つものは確認できていない。

### Cited Findings
- 汎用 MCP / エージェント: 編集全般を扱い、シーケンサーは一機能（ChiR24）、または言及なし（chongdashu、Aura、Epic 5.8 の発表）。 — 前掲各出典
- Cutscene Agent: LLM による Sequence とカメラ生成は実証済みだが、人間の承認ループは記述なし、部分再レンダの明示もなし。 — [arXiv HTML](https://arxiv.org/html/2604.25318v1)
- Instant Movie Maker: ショット一括生成とバッチレンダはあるが、AI 計画と承認の記述は確認できず。 — [Fab](https://www.fab.com/listings/5eac9e24-50cb-429f-a222-011196be666a)
- 動画生成系のゲームトレーラーサービス（VO3 など）はエンジンを使わずテキストから短い動画を出す。実際のゲーム画面ではない。 — [VO3](https://www.vo3ai.com/tools/ai-game-trailer-generator)

### Inferences
- 空白地帯は「正本としての shots.yaml + fingerprint + 承認」。ただし「見つからなかった」は不在の証明ではない（検索の浅さ、Aura 等の非公開機能）。提出前に Aura の更新履歴と UE 5.8 MCP のツール一覧を確認する価値がある。
- リスクは、Epic 公式 MCP と Aura の拡大でシーケンサー操作が一般化した場合に、「UE に Claude をつなぐ」だけでは見劣りすること。部分修正ループの実演（D-13）が勝負どころという既存の判断を補強する。

### Gaps
- Fab 全体の網羅検索（「AI cinematic」系リスティング）、YouTube / Reddit のデモ事例、80.lv の記事。
- 実際にトレーラーやカメラワークを作る Claude + UE デモの有無（検索では見つからなかった）。
