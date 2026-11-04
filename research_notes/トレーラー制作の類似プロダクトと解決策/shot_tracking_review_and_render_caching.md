# ショット管理・レビューツールと再レンダ/キャッシュ方式（shotpilot 向け調査）

調査日: 2026-11-05。予算の都合で呼び出し約20回。公式ドキュメントの直接取得に失敗した箇所（Autodesk features 403、Kitsu 一部 404）が多く、確認できなかった項目は各 Gaps に明記した。価格は変動するため、サードパーティ掲載値は参考扱い。

## 1. 製品ごとの仕組み（ショット・バージョン・承認・注釈・価格・対象）

### Takeaway
Kitsu と Flow Production Tracking（旧 ShotGrid）が「ショット単位のステータス＋バージョン＋プレイリスト審査」の典型で、注釈はフレーム/タイムコードに紐づく。どれも「再レンダ」は管理せず、レンダ結果を人が登録/レビューする側の道具。価格は数十人規模の制作会社向けで、個人開発者には過剰。

### Cited Findings
- 名称: ShotGrid は現在 Flow Production Tracking（旧 Shotgun/ShotGrid）。Epic の API 索引も「Flow Production Tracking (formerly Shotgun/ShotGrid)」と表記 — [Epic API: Shotgrid plugin](https://dev.epicgames.com/documentation/unreal-engine/API/PluginIndex/Shotgrid)
- Flow Production Tracking の価格: 公式価格は取得できず。サードパーティ（Software Advice/GetApp、2026年3月更新）が約 $45/ユーザー/月と掲載。リセラー（Motion Media）は「Call for Pricing」 — [Software Advice](https://www.softwareadvice.com/project-management/shotgrid-profile/)、[GetApp](https://www.getapp.com/project-management-planning-software/a/shotgrid)、[Motion Media](https://www.motionmedia.com/autodesk-flow-production-tracking-single-user-3-year-subscription/)
- Autodesk は Pipeline Toolkit・Python API・REST API でのカスタム連携を案内 — [Autodesk features (UK)](https://www.autodesk.com/uk/products/flow-production-tracking/features)（検索結果の要約経由。ページ本体は403で未確認）
- Kitsu（CGWire、オープンソース）: クラウド価格は年払いで Agency €20 / Production €30 / Studio €35 /ユーザー/月（Agency は1〜20人・100GB、Production は1〜100人・3TB で「TV series, films, video games」向け）。Enterprise は見積もりでオンプレ可。2週間トライアル — [CGWire pricing](https://www.cg-wire.com/pricing)
- Kitsu は2025年6月に席単位課金へ変更と報告 — [CGWire blog June 2025](https://blog.cg-wire.com/build-in-public-june-2025-update)（検索要約経由）。2019年の旧価格（€99/月・10人まで等）は古い — [CG Channel 2019](https://www.cgchannel.com/2019/02/check-out-open-source-production-tracking-tool-kitsu)
- Kitsu のプレイリスト審査: プレイリスト内でコメントと注釈を付けられ、クライアントは「Retake」ステータスに変更して修正を依頼できる。プレイリストの主目的は shot/asset のレビュー — [Kitsu client guide](https://kitsu.cg-wire.com/getting-started-client)、[Kitsu playlist](https://kitsu.cg-wire.com/playlist/)（いずれも検索要約経由。直接取得は404）
- Kitsu 2026年6月更新でオニオンスキンと注釈シェイプに対応、公開プレイリスト共有（アカウント不要のリンク）あり — [CGWire blog June 2026](https://blog.cg-wire.com/build-in-public-june-2026-update)（検索要約経由）
- ftrack: 公式価格ページ上 Studio $25/ユーザー/月（年払い）、Review $15/ユーザー/月の単体。G2 は Review $10（年）/$15（月）、Studio $25（年）/$30（月）。Review Pro アドオン $150/月/ワークスペース（同期レビュー10人、4K、透かし）。Studio と Review は同一ワークスペースで併用不可（第三者情報）— [ftrack pricing](https://www.ftrack.com/en/pricing)、[G2](https://www.g2.com/products/ftrack/pricing)、[Shade](https://shade.inc/blog/ftrack-for-post-production)
- ftrack Studio の構成: 計画・スケジュール、制作トラッキング、レビューと承認、レポート、API — [Shade](https://shade.inc/blog/ftrack-for-post-production)
- SyncSketch: フレーム単位で固定される注釈が中核。動画/プレイブラストも3Dアニメも同様 — [SyncSketch support: Reviewing 3D Models and Animations](https://support.syncsketch.com/hc/en-us/articles/32393818685588-Reviewing-3D-Models-and-Animations)。無料の Light プランあり — [Light plan](https://support.syncsketch.com/hc/en-us/articles/32393780263572-What-is-the-Light-Free-Plan)。価格は情報源間で不一致（$6.80〜$12/月〜、Capterra は $8〜）— [Spotsaas](https://www.spotsaas.com/product/syncsketch/pricing)、[SoftwareSuggest](https://www.softwaresuggest.com/syncsketch/pricing)、[Capterra](https://www.capterra.com/p/202567/SyncSketch/)。Unity が提供元 — [Unity SyncSketch](https://unity.com/en/products/syncsketch)
- Frame.io: V3 の旧プランは Free（2人・2プロジェクト）でもコメント/注釈とバージョン管理あり。Team 以上で社内限定コメント。V4 の価格は公式ページ参照で、取得できず — [Frame.io legacy plans](http://help.frame.io/en/articles/2682293-getting-started-what-are-the-pricing-plans-you-offer-legacy)。V4 は Pro 2TB / Team 3TB＋メンバーごと2TB（検索要約）— [Frame.io help](https://help.frame.io/en/articles/2682293-getting-started-what-are-the-pricing-plans-you-offer)。2024年の記事は Pro $15/月と記すが古い可能性 — [Videomaker](https://www.videomaker.com/news/nab-2024-frame-io-v4-is-the-best-collaboration-and-review-platform/)
- Autodesk Flow Capture: PIX と Moxion を統合した製品。営業経由のみ、価格は個別見積もり、30日トライアル、アカウントあたり最大6TB。HDR/Dolby Vision 再生、透かし/DRM、現場からのライブ配信 — [Autodesk Flow Capture](https://www.autodesk.com/products/flow-capture/overview)、[Shade](https://shade.inc/blog/utodesk-flow-capture-for-post-production)。ゲームエンジン連携は確認できず（Unreal 言及は別製品 Flow Studio のみ）
- Prism Pipeline: 2.0（2023年末）でコア等は LGPL、Unreal Engine プラグインは有料アドオン — [CG Channel: Prism 2.0](https://www.cgchannel.com/2023/11/prism-2-0)、[Prism plans](https://prism-pipeline.com/plans/)。価格数値は未確認

### Inferences
- 対象は「複数部門・数十人」。ステータスはショット/タスク単位、注釈はフレーム/タイムコード単位が共通。shotpilot の「1人＋AI」には、席課金・ストレージ課金・承認ワークフローが過剰。
- Kitsu のプレイリスト＋Retake ステータスは、shotpilot の「ショットを選んで1カットだけ直す」ループに最も近い語彙。

### Gaps
- cineSync、Moxion 単体の価格と機能は確認できず（Moxion は Flow Capture に統合）。
- Flow Production Tracking のステータス定義・バージョン・注釈の公式ドキュメント（help.autodesk.com は取得失敗）。
- Frame.io V4 の注釈種別（描画/範囲）、バージョンスタック、承認ステータスの一次情報。
- SyncSketch と Unreal の連携は確認できず。

## 2. 変更だけを再処理する仕組み（レンダファーム/PDG/DDC/USD）

### Takeaway
明示的に「変更分だけ」を判定するのは Houdini PDG（出力ファイルの存在・追跡状態による dirty 判定）と Unreal DDC（キャッシュ階層、再生成可能）。ファームマネージャ（Deadline）は再投入の単位は持つが、入力の変更検出は確認できなかった。shotpilot の fingerprint はこれらより厳密（入力の内容ハッシュ）で、強みになる。

### Cited Findings
- PDG/TOPs: ワークアイテムを dirty（古い、または古い可能性）として印を付け、前回の cook から変わった分だけをやり直す — [SideFX: Cooking](https://www.sidefx.com/docs/houdini/tops/cooking.html)（検索要約経由）
- PDG のキャッシュ状態は Clean / CleanPending / DirtyAlways / DirtyEmpty / DirtyMissing（期待出力が無い）/ DirtyNew（PDG が未追跡）/ DirtyStale（上流が cook 中に新ファイルを書いた）/ Undefined。ハッシュやパラメータ比較への言及はページに無い — [SideFX: workItemCacheState](https://www.sidefx.com/docs/houdini/tops/pdg/workItemCacheState.html)
- PDG の Cache Mode が Automatic なら出力ファイルがディスクにあるとキャッシュから cook、Write Files なら常に再 cook。TOPs は入出力ファイルの中身を確認せず、中間ファイルを手で差し替えても下流は dirty にならない（制限）— [SideFX: Cooking](https://www.sidefx.com/docs/houdini/tops/cooking.html)、[SideFX forum](https://www.sidefx.com/forum/post/316144)（検索要約経由）
- Unreal DDC: 速い順にキャッシュを引き、無ければ生成して非同期で各層へ書く。キャッシュは使い捨てで .uasset から再生成可能。共有 DDC（ネットワークドライブ）でチーム内の生成結果を共有。キーの形式と無効化規則はページに記載なし — [Epic: Derived Data Cache](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-derived-data-cache-in-unreal-engine)
- AWS Deadline Cloud: ジョブの再投入（resubmit）で一部フレームだけ試し、確認後に全範囲を流せる。ステップ依存があり、再投入時に依存のあるタスクは PENDING、無いものは READY。入力ファイルの変更検出や未変更分の再利用の記載は見つからず — [Deadline Cloud: view a job](https://docs.aws.amazon.com/deadline-cloud/latest/userguide/view-a-job.html)、[AWS announcement](https://aws.amazon.com/about-aws/whats-new/2024/10/aws-deadline-cloud-resubmitting-jobs)（検索要約経由）

### Inferences
- PDG は「出力ファイルが在るか」中心で、入力内容のハッシュではない。定義の内容ハッシュで take を管理する shotpilot のほうが、ファイルの手動差し替えを検出できる点で堅い。
- Deadline 系は「どのフレーム/タスクを流すか」を人や上位ツールが決める層。shotpilot の fingerprint は、その上位判断を自動化したものと位置づけられる。

### Gaps
- Tractor、Thinkbox Deadline（旧版）、Nuke のキャッシュ、USD レイヤ/アセットのバージョニングは調査できず。
- Unreal DDC のキー生成（ハッシュ対象）の仕様。

## 3. 上流が変わったときの「承認」の扱い

### Takeaway
今回の調査では、各ツールが「上流変更で承認を自動で外す」と明記した一次情報は見つからなかった。一般にバージョンは新しい提出物として積まれ、承認はそのバージョンに付くと考えられるが、これは推測。

### Cited Findings
- Kitsu: 審査はプレイリスト上でステータス（例: Retake）を人が変更する運用で、自動失効の記述は確認できず — [Kitsu client guide](https://kitsu.cg-wire.com/getting-started-client)（検索要約経由）
- Frame.io V3 の資料に承認ステータスの記述は無い — [Frame.io legacy plans](http://help.frame.io/en/articles/2682293-getting-started-what-are-the-pricing-plans-you-offer-legacy)

### Inferences
- 上記ツールの承認は人手のステータス操作で、内容の変化と機械的に結びついていない可能性が高い（未検証）。shotpilot の「look fingerprint に紐づけ、画が変わると承認が外れる」は差別化点になりうる。

### Gaps
- Flow Production Tracking / ftrack / Frame.io V4 で、新バージョン提出時に承認がどうなるかの公式記述。

## 4. Unreal / ゲームエンジンとの接続

### Takeaway
Flow Production Tracking には Epic 同梱の Experimental/Beta プラグインがあるが、Sequencer ショットとの同期の具体は確認できなかった。Prism は有料アドオンで対応。ゲームエンジン向けのレビューと再レンダを一体にしたものは見つからなかった。

### Cited Findings
- Epic の Flow Production Tracking プラグイン: Editor カテゴリ、Engine\Plugins\Experimental\Shotgrid、Beta 表記で「出荷には注意」。依存は Python Editor Script Plugin と Editor Scripting Utilities。機能説明はページに無い — [Epic API: Shotgrid plugin](https://dev.epicgames.com/documentation/unreal-engine/API/PluginIndex/Shotgrid)
- Autodesk は Maya・3ds Max・Unreal Engine などの制作ツールから Flow Production Tracking の管理/レビュー機能に届くと説明 — [Autodesk features](https://www.autodesk.com/uk/products/flow-production-tracking/features)（検索要約経由）
- Prism 2.0 は Unreal Engine 対応を追加（有料アドオン）。コミュニティ製の別プラグインは初期段階との声 — [CG Channel](https://www.cgchannel.com/2023/11/prism-2-0)、[Prism forum](https://prism-pipeline.com/forum/topic/prisim-unreal-engine/?part=3)

### Inferences
- いずれも「Python スクリプトでエンジンを操作する」点は shotpilot の `sp_entry.py` と同じ方式。shotpilot が差別化できるのは、レビューツール側が持たない「変わったショットだけ撮り直す」部分。

### Gaps
- Sequencer ショット ↔ トラッカーのショット同期の具体（Epic の virtual production ドキュメントや事例）。
- Autodesk Flow Capture / cineSync / SyncSketch / Frame.io の Unreal 連携の有無。

## 5. shotpilot が借りられる設計パターンと、過剰なもの

### Takeaway
借りる価値が高いのは、ショット単位ステータス、プレイリスト（選んだショットを連続再生して審査）、フレーム固定の注釈、バージョン積み。過剰なのは、席課金/権限管理/透かし/DRM/ライブ配信/スケジュール管理。

### Cited Findings
- （パターン根拠）ショット単位ステータスとプレイリスト審査＋Retake: Kitsu — [Kitsu playlist](https://kitsu.cg-wire.com/playlist/)
- （パターン根拠）フレーム固定の注釈: SyncSketch — [SyncSketch support](https://support.syncsketch.com/hc/en-us/articles/32393818685588-Reviewing-3D-Models-and-Animations)
- （過剰の根拠）透かし/DRM/ライブ配信は Flow Capture、同期レビュー/4K/透かしは ftrack Review Pro — [Autodesk Flow Capture](https://www.autodesk.com/products/flow-capture/overview)、[Shade](https://shade.inc/blog/ftrack-for-post-production)

### Inferences
- 借りる: (1) ショットごとの状態（fresh/stale/out-of-sync に加え、承認済み/要修正）。(2) take の版スタック（fingerprint 管理で既に近い。UI でスタック表示と戻しを出す）。(3) フレーム＋範囲を持つ注釈を Claude への指示にそのまま渡す（D-17）。(4) プレイリスト＝ stale のショットだけ、または選択ショットを順に再生する確認モード。(5) 公開リンクでアカウント不要のレビュー共有は、デモの共有用として検討余地あり。
- 過剰: 席/権限、スケジュール、DRM/透かし、同期ライブレビュー、複数部門のタスク割り当て、クラウドストレージ。
- 競合しない点: 既存ツールは「撮り直し」を管理しないので、fingerprint ベースの部分再レンダと「画が変わると承認が外れる」仕様は、調べた範囲では見当たらなかった（ただし第3節の通り一次情報は不足）。

### Gaps
- 上記の「見当たらなかった」は網羅調査ではなく、主要製品の公式ドキュメント取得に失敗した部分を含む。提出資料で「他にない」と書く前に、Flow Production Tracking と Frame.io V4 の承認/バージョン仕様を直接確認すること。
