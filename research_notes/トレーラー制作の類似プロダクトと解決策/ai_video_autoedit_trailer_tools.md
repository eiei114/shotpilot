# AI動画生成・自動編集・ハイライト切り抜き・ゲームトレーラー生成プロダクトと、開発者のトレーラー課題への対応度

調査日: 2026-11-05。ツール呼び出し上限（約15回）のため、網羅は浅い。価格・仕様の多くは二次情報（比較ブログ、競合のマーケ記事）で、公式ページ未確認。確認できなかった点は各節の Gaps に明記した。

## 1. 生成動画（Runway / Veo / Sora / Kling など）をゲームトレーラーに使う場合と、炎上事例

### Takeaway
生成動画は「実際のゲームではない映像」を作るため、トレーラーの中核には向かない。ARK: Aquatica（2025-03）が代表的な炎上例で、2026 にも類似例が続く。Steam の開示規則は 2026-01 に改訂されたが、トレーラーの扱いは一次情報で明記を確認できなかった。

### Cited Findings
- ARK: Aquatica の GDC 2025 リビール・トレーラーは、ほぼ全編が生成AIと批判され、Snail Games が YouTube から削除した（直リンクでは残る）。Wildcard は関与を否定した — [Search summary: PC Gamer / 80.lv / TweakTown](https://www.pcgamer.com/games/survival-crafting/ark-devs-distance-themselves-from-ai-generated-trailer-we-did-not-know-that-they-were-doing-it)
- 80.lv（2025-03-24 付）: リードデザイナーの Matt Kohl が「マーケ部門が、我々のアセットの静止画を使って AI アニメのトレーラーを作った」「開発チームは知らなかった」と説明。トレーラーは完成版DLCを表していない。低評価率97%、批判コメントは約6,000件。使用AIツール名は記事に無い — [80.lv](https://80.lv/articles/ark-survival-evolved-s-ai-animated-trailer-was-created-by-the-marketing-department-said-dev)
- 同記事: Snail Games は Age of Wushu のトレーラー2本も、AI多用として中国のゲーマーに批判された — [80.lv](https://80.lv/articles/ark-survival-evolved-s-ai-animated-trailer-was-created-by-the-marketing-department-said-dev)
- 2026 年の類似例（二次・低品質ソース混在）: Superboo Studios「Fallen」トレーラー、Postal 新作（パブリッシャーが取りやめ）、SNK Fatal Fury のトレーラーが AI 疑惑で批判された。Vapor World は AI カットシーンで Steam 上で炎上 — [Search result summary](https://everythingedinburgh.com/games/news/vapor-world-steam-ai-cutscene-backlash/)。個別事実は一次確認していない。
- 同種の記事の論点は「ツールそのものではなく、体験を損なう目に見える手抜きが反発される」 — [everythingedinburgh](https://everythingedinburgh.com/games/news/vapor-world-steam-ai-cutscene-backlash/)
- 逆に、AIを使っていないのに疑われる事例もある（Shrine's Legacy、開発者の妻の声を AI 音声と疑われた件） — [Outlook Respawn](https://respawn.outlookindia.com/gaming/gaming-news/shrines-legacy-devs-deny-ai-slop-chatgpt-rumors), [Dexerto](https://www.dexerto.com/gaming/indie-dev-accused-of-using-ai-voice-acting-in-steam-game-reveals-it-was-actually-his-wife-3410085/)
- Runway: Aleph 2.0 は video_to_video で既存映像を編集・変換する。API は 28 クレジット/秒、最小56クレジット、1クレジット=0.01ドル。無料プランは Aleph 不可（125クレジット単発）。出典は集約サイト — [creatify.ai](https://creatify.ai/blog/runway-pricing-(2026)-plans-credits-and-what-you'll-actually-pay)、[Picsart API](https://picsart.com/api-platform/models/runway-aleph2/pricing)。Aleph の別価格（50クレジット等）の記載もあり不一致。
- Veo 3/3.1 は1生成あたり最大8秒が最も一貫した報告。Sora 2 と Kling は秒数が出典で食い違う。Sora は透かし除去が上位プランのみ、メタデータは残るとの報告。Kling 無料は透かし付き。すべて比較ブログで公式規約は未確認 — [aipedia 比較](https://aipedia-en.pages.dev/compare/veo3-vs-sora2-vs-kling-2026)、[invideo 比較](https://invideo.io/blog/kling-3-vs-veo-3-1/)
- 「Sora 2 Is Dead」と題する記事がある（サービス終了を示唆）。信頼性不明で未検証 — [tech-insider](https://tech-insider.org/au/?p=2708)
- 商用権はプランや規約に依存。企業向け補償（indemnity）は Vertex 上の Veo 3.1 で言及があるが、Kling/Sora では見つからなかった — [invideo 比較](https://invideo.io/blog/kling-3-vs-veo-3-1/)
- Steam 公式（Steamworks Content Survey）: AI は「Pre-Generated（開発中に生成）」と「Live-Generated（実行中に生成）」に分かれ、出荷物に対する審査は非AIと同じ基準。ページはマーケ素材やトレーラーでの開示には直接触れず、Distribution Agreement で「ゲームはマーケ素材と整合すること」を約束させる点のみ言及（取得結果の要約） — [Steamworks](https://partner.steamgames.com/doc/gettingstarted/contentsurvey#ai)
- VGC（2026-01-17）: 改訂後の規則は、ゲーム内・ストアページ・マーケ素材で使った生成AIの開示が必要。コード補助など効率化ツールは対象外。記事は「トレーラー」を明示していない — [VGC](https://www.videogameschronicle.com/news/valve-has-significantly-rewritten-steams-rules-for-how-developers-much-disclose-ai-use)
- 他の解説は、トレーラー映像・カプセル画像・ストア文を pre-generated の例に挙げる（検索要約、二次） — [PC Gamer](https://www.pcgamer.com/software/ai/steam-updates-ai-disclosure-form-to-specify-that-its-focused-on-ai-generated-content-that-is-consumed-by-players-not-efficiency-tools-used-behind-the-scenes/)、[GosuGamers](https://www.gosugamers.net/entertainment/news/77861-steam-revises-ai-policy-while-keeping-generative-content-disclosure-intact)
- 開示数: 2025 年上半期に約8,000本が開示、2024 年通年は約1,000本という分析がある一方、別の出典は「7%」としており分母が異なる可能性（要注意） — [PC Gamer 系検索結果](https://www.pcgamer.com/software/ai/steam-updates-ai-disclosure-form-to-specify-that-its-focused-on-ai-generated-content-that-is-consumed-by-players-not-efficiency-tools-used-behind-the-scenes/)

### Inferences
- 炎上の構造は「生成映像が実際の製品と違う」＋「開発チームが知らない」。実ゲームから撮った映像なら両方を避けられる（shotpilot の「ゲームから撮る」方針の根拠になる）。
- 生成動画は1生成8秒前後で、同じショットを再現する手段が無い。ショット単位の再現性・差し替えには向かない（秒数は二次情報）。
- Steam の開示は「プレイヤーが目にするもの」が焦点。トレーラーが明示対象かは一次で未確認なので、利用者向けには Steamworks の最新文言の確認を促すのが安全。

### Gaps
- Pika、Luma Ray、Hailuo の個別情報は未取得。Runway Gen-4 の公式利用規約・商用条件、Veo/Sora/Kling の公式規約も未確認。
- Steamworks の AI 開示ページで、トレーラーが明示されているかは未確認（取得結果は言及なし。VGC の記事も言及なし。二次ソースは含むと説明）。
- Sora の提供終了が事実かは未検証。

## 2. AI編集・クリッピング（Descript / Opus Clip / CapCut / Premiere / DaVinci / Kapwing / VEED / Wisecut）

### Takeaway
これらは「既存の映像・音声」を編集する道具で、エンジンを知らない。ビート同期、9:16 リフレーム、字幕、検索は得意だが、ゲーム映像の山場判定は弱い。取れていない映像を作ることも、ゲーム変更後に撮り直すこともできない。

### Cited Findings
- DaVinci Resolve 20（2025年5月最終版）: AI Detect Music Beats が音楽のビートにマーカーを置き、カット点やスナップに使える。IntelliScript は文字起こしエンジンで台本から タイムラインを組む — [ProVideo Coalition](https://www.provideocoalition.com/davinci-resolve-20-released/)、[Engadget](https://www.engadget.com/apps/davinci-resolve-20s-latest-ai-feature-can-create-an-entire-timeline-based-on-a-script-120009351.html)。新しい版が出ている可能性あり。
- Premiere Pro 25.2（2025-04）: Generative Extend が一般提供（4K・縦対応、Firefly Video Model、Content Credentials 付与）。Media Intelligence は映像を端末上のモデルで索引し、自然言語で被写体・カメラ角度・台詞を検索できる。Firefly クレジット消費は解像度等で変わる — [Adobe Newsroom](https://news.adobe.com/news/2025/04/new-ai-innovation-in-industry)、[Adobe Help](https://helpx.adobe.com/au/premiere-pro/using/whats-new/2025-2.html)、[NewscastStudio](https://www.newscaststudio.com/2025/04/03/adobe-brings-generative-extend-media-intelligence-to-premiere-pro)
- Opus Clip: 第三者レビューでは Free / Starter 約$15 / Pro 約$29 / Business。Virality Score（0-100 または 0-99 と出典で不一致）、ReframeAnything（縦クロップと被写体追跡）。高速な視覚アクション（ゲーム・スポーツ）では山場検出が弱く、手動補正が要るとの1出典のみ。ゲーム映像での9:16検証は見つからず — [eesel](https://eesel.ai/blog/opusclip-pricing)、[makerstack](https://makerstack.co/reviews/opus-clip-review/)
- 「AIゲームトレーラー」向けと称する汎用ツール: Invideo（ゲーム映像アップロードまたは説明から台本・ナレーション・カット・音楽込みで生成。チャット編集 Magic Box で素材差し替え）、Dreamina（Seedance ベース、プレイ映像・コンセプトアート・コンテ・音楽・プロンプトを受ける）、HeyGen、Alconost。Runway は実映像に強いと評される。Seeles は構成・ショットリスト・フックの計画のみで映像は編集しない — [検索要約（Invideo/Dreamina/Seeles）](https://dreamina.capcut.com/seedance/game-trailer-maker)、[Seeles](https://www.seeles.ai/features/tools/ai-game-trailer-generator)
- FluxNote のガイド: Runway はシーン検出で高アクション区間を提案、VEED Pro 約$25/月（アップロード25分制限）、InVideo Plus 約$20/月（HD書き出し月60本）、FluxNote $10/月、ElevenLabs v3 でナレーション、Clipchamp は無料TTS。OBS で撮影。著者は「核は本物のゲーム映像、AI生成は導入や転換に限る」と助言。BGM の「ロイヤリティフリー」が商用ゲームに使えない場合がある警告もある。ベンダー自身の販促記事 — [FluxNote](https://fluxnote.io/guides/ai-game-trailer-generator)
- 本調査では Descript、CapCut AI、Kapwing、Wisecut の個別情報は取得できなかった。

### Inferences
- これらの価値は「編集時間の短縮」「音とカットの同期」「SNS用の縦切り・字幕」。入力は常に既存映像か静止画。ゲームの内部状態（カメラ・配置・イベント）を知らず、変更後の再撮影やショット単位の再現はできない。
- 「トレーラー向け」と称するツールの多くはテンプレート＋ストック素材＋ナレーションの広告生成で、hook→title のようなゲーム固有構成を実測値から組む根拠は見つかっていない（出典はベンダー記事のみ）。

### Gaps
- Descript、CapCut、Kapwing、Wisecut、Wisecut の機能・価格・ゲーム用途の確認。
- 各ツールの構成・ペース決定ロジックの一次資料。
- 「Product Hunt / itch.io / Steam フォーラム」上のトレーラー専用スタートアップの調査は未実施（検索1回の結果に出たのは上記の汎用ツールと Seeles、フリーランス出品のみ）。

## 3. ゲーミング ハイライト検出ツール（Medal / Outplayed / Eklipse / Allstar / Powder / SteelSeries Moments / Steam Game Recording）

### Takeaway
対象は「プレイヤー視点の見せ場（キル・勝利）」のクリップ化で、開発者のトレーラー制作用ではない。検出は主にゲーム内イベントのテレメトリ。開発者がトレーラーに使っている証拠は見つからなかった。

### Cited Findings
- Steam Game Recording: 2024年6月ベータ開始。バックグラウンドで自動録画、クリップを MP4 に書き出せる。Steam 自体に AI ハイライト機能は見当たらない — [Game Developer](https://gamedeveloper.com/pc/steam-debuts-new-game-recording-tool-in-beta-form)、[PC Gamer](https://www.pcgamer.com/software/look-out-shadowplay-steam-now-has-a-built-in-game-recording-feature-that-will-automatically-record-gameplay-videos-you-can-clip-share-and-export)
- Eklipse: クラウドAIで、Twitch/Kick/YouTube の VOD URL、または Steam 録画フォルダ連携から自動クリップ。セッション終了後に処理する。無料枠は透かし付き、有料は約$19〜20/月（出典不一致）。出典は主に Eklipse 自身の比較ページ — [Eklipse 比較](https://eklipse.gg/compare/allstar-alternatives/)、[Eklipse AI Highlights](https://eklipse.gg/use-case/eklipse-game-highlights)
- Medal: 価格は月$24.99という競合の記載。自動検出の有無は出典で食い違う（キル・実績・レベルアップを検出する／検出しない）。「100+ ゲームのイベントクリップ」は未検証 — [Medal 比較ページ](https://medal.tv/learn/best-clipping-software)、[Eklipse 記事](https://blog.eklipse.gg/tools/medal-tv-review.html)
- Outplayed（Overwolf）: ゲーム内イベントをリアルタイム検出。価格は出典間で不一致（$2.99/月、無料、$75/月）で信頼できない — [Eklipse blog](https://blog.eklipse.gg/streaming-tips/7-best-outplayed-like-tools-to-try-in-2023-tsai.html)
- Powder: 40以上のタイトルで自動検出（1ガイド）。SteelSeries Moments: キル・勝利を AI 検出、自動イベントは対応タイトルに限る — [Eklipse Powder 比較](https://eklipse.gg/compare/powder-alternatives/)、[Medal 比較ページ](https://medal.tv/learn/best-clipping-software)
- 一般的な説明として「Powder や Medal はゲーム内イベント（キル・エース・勝利）をテレメトリから検出」。内部仕様の一次資料なし — 同上
- Streamlabs は ML で「ゲーム内イベントに基づく」ハイライトを検出 — [Streamlabs](https://streamlabs.com/highlighter)

### Inferences
- 検出はキル／勝利などPvP中心の既知タイトル向けで、未発売の自作ゲームや、ホラー・コージーなど「見せ場がキルではない」ジャンルには当てはまりにくい。
- 出力は偶然撮れたプレイ映像で、カメラ位置・時間帯・画質などの再現性が無い。

### Gaps
- Allstar の検出方式と価格（信頼できる情報なし）。NVIDIA Highlights は検索結果に出ず未確認。
- 開発者がこれらをトレーラーに使うかについて、直接の証拠は未発見。
- 価格は競合ブログ依存で、公式ページ未確認。

## 4. ゲームトレーラー専用サービスと、開発者の課題に対する総括

### Takeaway
解決している課題は「編集時間」「SNS用の縦切り・字幕」「音とのビート同期」「ナレーション作成」「構成案」。解決していない課題は「実ゲームの撮影」「ショットの再現性」「ゲーム変更後の撮り直し」「部分修正で他を触らない運用」。

### Cited Findings
- 調査で見つかった「AIゲームトレーラー」製品は、(a) 映像アップロード型の汎用広告生成（Invideo、HeyGen、Alconost）、(b) 画像／プロンプト→生成映像型（Dreamina/Seedance）、(c) 計画のみ（Seeles）、(d) フリーランス出品。エンジンに接続し、ゲームから撮影するものは見つからなかった — [Dreamina](https://dreamina.capcut.com/seedance/game-trailer-maker)、[Seeles](https://www.seeles.ai/features/tools/ai-game-trailer-generator)、[Fiverr 出品](https://fiverr.com/jakeoncid419/create-a-cinematic-ai-game-trailer-or-teaser-video)
- 修正の扱い: Invideo は生成後にチャットで素材を差し替えられる（検索要約）。ただし変更が他の部分に波及しないことの保証は確認できなかった — [Invideo](https://info.invideo.io/is-there-an-ai-that-can-create-a-cinematic-ad-for-a-new-video)
- ARK の件が示すとおり、静止画から作った生成トレーラーは「完成版を表していない」と開発側自身が否定する事態になった — [80.lv](https://80.lv/articles/ark-survival-evolved-s-ai-animated-trailer-was-created-by-the-marketing-department-said-dev)
- Steam 配布契約では、ゲームがマーケ素材と整合することが約束事になっている — [Steamworks](https://partner.steamgames.com/doc/gettingstarted/contentsurvey#ai)

### Inferences
- 解決する課題: 編集時間（Resolve/Premiere/Invideo）、ビート同期（Resolve の Detect Music Beats）、9:16 リフレームと字幕（Opus Clip 等）、ナレーション（ElevenLabs 等）、素材検索（Premiere Media Intelligence）。
- 解決しない課題: (1) 実ゲームの撮影（エンジン非認識）、(2) 再現可能なショット（生成は非決定、プレイ録画は再現不能）、(3) ゲーム変更後の撮り直し（ビルドが変わると映像は古くなる）、(4) 1カットだけ直して他を再レンダしない運用（生成型は再生成で別物になる）。
- shotpilot の差別化候補は、エンジン上でカメラ・ショットを定義し、変更されたショットだけ再撮影する点。今回の調査範囲では同種の製品は見つからなかったが、検索は浅く「存在しない」とは断定できない。

### Gaps
- Product Hunt、itch.io、Steam フォーラムでのトレーラー専用スタートアップ（2023〜2026）の網羅的調査。
- Unreal / Unity のシネマティック自動化（Sequencer 自動化、Cinemachine 等）を使った類似サービスは未調査（他の担当範囲の可能性）。
- 各製品のユーザー評価（Reddit / HN）は取得できていない。
