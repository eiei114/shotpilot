# Previz / storyboard / shot-list tools (traditional and AI) and what they mean for shotpilot

調査日: 2026-11-05。検索は約17回で打ち切り。公式ページを直接取得できたのは Boords の export ページ、Storyboarder.ai のヘルプ、VO3 AI だけ。残りは検索スニペットと二次情報で、価格は矛盾が多い。各所に確度を書いた。

## Q1. Traditional / 3D previz tools: what do they take in, put out, and keep as camera/lens data? Can they hand it to UE?

### Takeaway
従来ツールの出力は「人間が読む成果物」（PDF、アニマティック、Premiere/FCP の XML）が中心で、カメラ・レンズを UE 向けの構造化データとして出す経路は、今回の調査では確認できなかった。UE 側のカメラ書き出しは Epic 側の機能（Sequencer から FBX）であって、previz ツール側の機能ではない。

### Cited Findings
- Storyboarder（Wonder Unit）は無料のオープンソース。Premiere / Final Cut / Avid / PDF / GIF に書き出せ、タイムラインは XML と PNG 群で渡す。記事の大半は 2017〜2020 年で、現行版は未確認。ライセンスは「MIT に追加条件あり」とする報道があるが、追加条件は未確認 — [Wonder Unit](https://wonderunit.com/software), [Digital Production 2017](https://digitalproduction.com/2017/10/24/kostenfreies-storyboard-programm/), [Boords 比較ページ（競合の宣伝）](https://boords.com/compare/boords-vs-wonder-unit)
- Storyboarder 2.0（2020-06）は Shot Generator を作り直した。内容は、ストックの 3D キャラの IK の整理、カメラ視点を探す新ウィンドウ、マルチユーザー VR。Shot Generator ではスクロールで焦点距離を変えられ、固定の焦点距離にスナップできる（後者は Scribd の転載で、旧版の記述の可能性あり） — [CG Channel](https://www.cgchannel.com/2020/06/download-free-storyboarding-tool-storyboarder-2-0/), [Scribd](https://www.scribd.com/document/659987572/Story-Boarder), [公式 wiki](https://github.com/wonderunit/storyboarder/wiki/Shot-Generator-Tutorial)
- Shot Designer（Hollywood Camera Work）は、俯瞰の配置図の上にカメラとキャラを置き、ショットリストと結ぶ。無料版は保存・書き出し・同期がなく、Pro は国ごとに価格が違う（UK £19.99、ES €22.99）。レンズについては、アプリ紹介に「レンズに忠実なカメラ角度」を取り込む記述があるだけ。焦点距離・フィルムサイズをカメラに付ける機能は、第三者レビューが「未言及」の欄に挙げており、確認できていない。App Store の評価は約12件で平均 3.42 — [App Store 紹介](https://apps.apple.com/ee/app/id556342711), [ProVideo Coalition](https://www.provideocoalition.com/shot-designer-the-scene-planning-tool-for-ios-android-mac-and-windows/), [appgoblin](https://appgoblin.info/apps/556342711), [filmtvsound](https://filmtvsound.com/index.php/production-planning/317-shot-designer-software)
- FrameForge Previz Studio は 3D の previz。「ほぼあらゆるカメラ・レンズの光学特性を再現する」とされ、Pro 版は物理カメラも扱える（Core は浮遊カメラのみ）。書き出しは PDF、HTML、カメラと俳優の動きをつなぐ動画。VFX 向けのカメラデータ形式の書き出しは見つからなかった。価格は 2011〜2015 年の情報だけで（Core $379〜、Pro $569〜$759、Premium $799 など）、出典間で食い違う。B&H の Pro DL は「販売終了」表示。現行の販売状況は不明 — [Post Magazine 2011](https://www.postmagazine.com/Publications/Post-Magazine/2011/January-1-2011/Review-FrameForge-Previz-Studio-3.aspx), [Videoguys](https://videoguys.com/blogs/news-and-sales/cyber-monday-special-30-off-frameforge-previz-studio-3-5), [B&H](https://www.bhphotovideo.com/c/product/1164152-REG/frameforge_pv3pro_p_previz_studio_3_5_pro.html), [FrameForge site license](https://www.frameforge.com/pages/site-license-covid19)
- ShotPro は iOS/Mac の 3D previz。「Unreal や Blender を学ばずに 3D の絵コンテを作る」という位置づけで、カメラ操作と照明がある。価格は買い切り $30（AlternativeTo、古い可能性）、App Store 米国 $39.99、オランダ €44.99。ショットリスト生成や AI は確認できない — [AlternativeTo](https://alternativeto.net/software/shotpro/about), [App Store US](https://apps.apple.com/us/app/shotpro/id982749248), [utildesk](https://utildesk-motia.pages.dev/en/tools/shotpro/)
- Cine Tracer は UE 製の撮影シミュレータ（作者 Matt Workman）。実在のカメラとライトを操作して画を組む。元は Cinema 4D プラグイン Cine Designer。初代は Steam $89.99（2021 年の記事）。Cine Tracer 2 は UE5 製で Early Access、初代の購入者には無料で提供される。Cine Tracer から Sequencer へカメラを出す手順は、公式記述が見つからなかった — [80.lv 2021](https://80.lv/articles/cine-tracer-v0-7-5-released), [Steam（検索結果経由）](https://store.steampowered.com/app/1774380?l=polish), [80.lv Inertia wheels](https://80.lv/articles/controlling-unreal-engine-5-camera-with-inertia-wheels/), [Epic spotlight](https://www.unrealengine.com/spotlights/cine-tracer-delivers-easy-to-use-realistic-cinematography-simulation?lang=en-US)
- UE のカメラ書き出し: ポゼッサブルなカメラは File > Export Selected、スポーナブルなカメラは Sequencer のカメラトラックを右クリックして出す。設定とアニメーションは保持される（2024 年の個人ガイド）。FBX 取り込みでは、4.15 時代の報告で焦点距離は入るがセンサーサイズが入らなかった。UE5 では取り込み時のスケールがマニュアルフォーカス距離に効かない、という報告がある。USD によるカメラ書き出しは確認できなかった — [versluis 2024](https://versluis.com/2024/08/how-to-export-cameras-from-unreal-engine), [Epic forum 2017](https://forums.unrealengine.com/t/sequencer-fbx-camera-import-aperture-settings/370485), [Epic forum（スケール）](https://forums.unrealengine.com/t/fbx-camera-import-into-sequencer-scale-does-not-apply-to-scene-dependent-attributes-e-g-focus-distance/2554548)
- StudioBinder のショットリストは、被写体・説明・カメラ・ショットサイズ・ショットタイプ・動き・機材・レンズ・特殊機材の欄を持つ。ショットデザイナーにはフレームレート、アングル、音、ロケ地もある。絵コンテのコマにはレンズ・動き・アングルのタグが付き、取り込んだ脚本からコマを作れる。価格は第三者の一覧で Indie $29 / Professional $42〜49 / Studio $85〜99 とされるが古い可能性があり、公式で未確認 — [StudioBinder shot list template](https://www.studiobinder.com/blog/shot-list-template/), [StudioBinder Shot Designer](https://www.studiobinder.com/shot-designer/), [SpotSaaS](https://spotsaas.com/product/studiobinder)
- Boords の書き出し（公式）: PDF（全プランで可、Free は透かし入り）、PPTX、MP4 アニマティック、画像 ZIP、After Effects（.zip と .jsx）、Premiere Pro / DaVinci Resolve（.zip と .xml、マーカーにメモ）、Word、XLSX（1コマ1行のショットリスト）はすべて有料プランのみ。アニマティックは1コマの既定が2秒、最大500コマ・13分で、トランジションと字幕は入らない。書き出しは「その時点のコピー」で、変更したら出し直す — [Boords export](https://boords.com/export)

### Inferences
- 従来の previz は、カメラ・レンズを「見た目を決める手段」として内部で持つが、UE に渡せる形式（JSON など）で書き出す例は今回は見つからなかった。shotpilot が `shots.yaml` にカメラとレンズを持ち、それを UE 側のスクリプトで Sequencer に組む作りは、この空白に当たる。ただし「他にない」とは言えない（調査していない製品がある）。
- Boords の書き出しは「出したら終わり」の一方向で、編集ソフトで直した結果は戻ってこない。shotpilot の `pull`（三方比較）はこの逆向きにあたる。

### Gaps
- Set-a-Light / Cine Designer の現状、Previs Pro、Cascadeur、PlotAI は検索しておらず、情報なし。
- Cine Tracer 2 の書き出し機能の公式記述と、現行価格は未確認。
- Shot Designer が焦点距離をカメラ属性として持つかは未確認。
- UE 5.x 公式ドキュメントでの Cine Camera の USD 書き出しは未確認。

## Q2. AI storyboard / script-to-shot tools: how do they turn text into shots, how consistent are they, what do users complain about?

### Takeaway
どのツールも「脚本 → 場面とショットに自動分割 → 1ショットずつ静止画 → 動画」と進み、一貫性は「キャラ・場所を再利用する要素」として持つ。苦情の裏づけは弱く、ベンダー自身の返信や競合の記事が主で、独立した検証は少ない。

### Cited Findings
- LTX Studio は脚本をシーンとショットに分け、キャラ・物・場所を再利用できる「Elements」として取り出し、ショットごとの演出とは別に持つ。調整はプロジェクト・ボード・フレームの各階層で行える。書き出しはアニメーション付き絵コンテ、MP4、ピッチデッキ — [diyai.io 2026](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/), [LTX blog](https://ltx.studio/blog/ltx-storyboard-generator-update)（後者は取得に失敗し、検索スニペットのみ確認）
- LTX の評価: Dupple と AIpedia は一貫性を強みと評す。競合の Luma Labs の記事は、学習コストが節約を上回るかを疑う。TechRaisal は Trustpilot 2.0/5 で、請求の分かりにくさ、サポート、音声の同期の苦情を挙げるが、一貫性は挙げていない。SourceForge ではベンダーが、一貫性への指摘に「改善中」と返信している（元の苦情の本文は見つからなかった） — [Dupple](https://dupple.com/reviews/ltx-studio), [Luma Labs](https://lumalabs.ai/news/ltx-studio-review), [TechRaisal](https://www.techraisal.com/blog/ltx-studio-the-ai-studio-transforming-script-to-screen_1752660411), [SourceForge](https://sourceforge.net/software/product/LTX-Studio/?page=2)
- Katalist は脚本を自動でショットに分け、承認したショットから AI 動画へ進む。キャラとシーンを持続させ、分割後にフレーミングとショットタイプを調整できる。価格は出典で食い違う（$19 / $39 / $59 / $99 の各説。ZIP と PPT の書き出しは $99 の Unlimited のみ、という競合の記述あり）。複数キャラの場面で崩れるという指摘は、競合 Atlabs の記事 — [diyai.io](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/), [Atlabs](https://www.atlabs.ai/blog/5-best-ai-storyboard-tools-for-marketing-agencies-in-2026), [kripeshadwani レビュー](https://kripeshadwani.com/katalist-ai-review/)
- Boords の AI は、プロンプトと画像、再利用できるキャスト・参照画像からコマを作り、1コマ単位でプロンプトか画像で作り直す。AI 機能はベータとする情報もある。有料プランの価格は確認できなかった — [diyai.io](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/), [theaireport](https://www.theaireport.ai/tooldatabase/boords)
- Storyboarder.ai（Wonder Unit の Storyboarder とは別製品）は、ショット説明と、キャラ・場所・物の参照から生成し、変種やコマ単位の再生成がある。3D Camera Angle 機能は、1つのコマについて 3D 空間のカメラをドラッグして動かし、同じ画を別アングルで生成する。ヘルプには焦点距離やレンズ設定を保持するとの記述がなく、アングルがどう符号化されるかも書かれていない — [Storyboarder.ai help](https://help.storyboarder.ai/en/articles/14691476-the-new-3d-camera-angle-feature-in-storyboarder-ai), [diyai.io](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/)
- Filmustage は脚本の AI ブレイクダウン（13種のタグ）と撮影スケジュール、AI の絵コンテとショットリストを持つ。無料の Explorer プランあり、有料は $39/月からとする一覧もある（食い違いあり）。ブレイクダウンの精度「最大86%」は SourceForge 掲載のベンダー主張で、独立した検証ではない。G2 は5件で 4.7/5 — [fitgap](https://us.fitgap.com/products/037096/filmustage), [SourceForge](https://sourceforge.net/software/product/Filmustage), [Filmustage vs StudioBinder](https://filmustage.com/filmustage-as-an-alternative-to-studiobinder/), [Filmustage use cases](https://filmustage.com/use-cases/)
- 「AI ショットリスト」系として、Studiovity の AI Shot List（脚本からショットリストを作り、AI 絵コンテにつなぐ。価格は問い合わせ）と ShotKraft（Pro $19.95/月、全ショットに AI 画像）がある — [Capterra](https://www.capterra.com/p/10038394/AI-Shot-List-Software/), [G2 ShotKraft](https://www.g2.com/de/products/shotkraft/pricing)
- 一般論として、監督が最初のボードをそのまま承認することは稀で、改稿への忠実さが試金石になる — [diyai.io](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/)

### Inferences
- これらの AI ツールが作る絵は生成画像で、実際の画面とは別物。shotpilot の絵コンテ（D-33）が「確認用」で止まる設計と、ティザーの画はすべてゲームから撮る原則は、この点で性質が違う。
- カメラの符号化が自然言語のプロンプトか、画像生成モデルへの 3D 位置指定にとどまるなら、レンズ値（焦点距離）を数値で持つ shotpilot の `camera.lens.focal_length` は精度で勝る。ただし、各製品の内部表現は未確認なので、断定はしない。

### Gaps
- Krock.io、Saga、Morphic、Plot は検索せず、情報なし。
- r/Filmmakers、r/gamedev、Product Hunt のコメントによる利用者の生の苦情は取得できなかった。G2 と Capterra の本文も読めていない。
- LTX Studio 公式の価格・カメラ/レンズ操作の詳細は、取得エラー（Header overflow）で読めていない。

## Q3. Tools for GAME trailers that structure beats, or trailer templates

### Takeaway
ゲームのティザー専用に「ビートを構造として持つ」製品は見つからなかった。あるのは、構成の助言（記事）、After Effects / Premiere のテンプレート（見た目と文字の演出）、テキストから8秒の生成クリップを作る AI ツールで、いずれも実機のゲーム映像を撮り直す仕組みではない。

### Cited Findings
- 構成の助言: フックは冒頭（ロゴは終端のエンドカードへ）。4部構成の一例は Hook（0〜5秒）、Core Loop（5〜25秒）、Escalation（25〜45秒）、Title / リリース情報（最後の5〜10秒）。別の型は「Tell, Show, Repeat」。ナレーションは安っぽく見えやすく、文字で補う。尺は90秒以内、60秒版なら45〜75秒の幅という記述がある。出典はマーケティング系のブログとツールベンダーで、秒数は出典によって違う — [Game Wisdom](https://game-wisdom.com/?p=33294), [Game World Observer](https://gameworldobserver.com/2022/09/22/common-game-trailer-mistkaes-indie-developers-steam), [Steam Page Analyzer](https://www.steampageanalyzer.com/blog/steam-trailer-best-practices), [autoae](https://autoae.online/blog/how-to-make-indie-game-launch-trailer-without-after-effects), [GMTK の字幕](https://amara.org/subtitles/UnJN0fMK8Och/en/1/download/How%20to%20Make%20an%20Indie%20Game%20Trailer%20%20Game%20Makers%20Toolkit.en.txt)
- テンプレート: AEJuice Epic Trailers（AE / Premiere、10秒・22アニメーション、$49 / $99 の表記）、Elgato の Game On トレーラー用イントロ、Motion Array の Games Epic Trailer（文字6枠・メディア3枠）、Videohive の Action Trailer // Games など。いずれもタイトルやテキストの演出が中心で、ショットの設計ではない — [Toolfarm AEJuice](https://www.toolfarm.com/?p=61575), [Elgato](https://marketplace.elgato.com/product/game-on-trailer-intro-templates-39cdba96-0a61-4bd6-ab57-800dbd6711a8), [Motion Array](https://motionarray.com/after-effects-templates/games-epic-trailer-176036), [Videohive](https://videohive.net/item/action-trailer-games/19308629)
- VO3 AI の Game Trailer Generator は、1ビートの文章（ボス登場、世界のフライスルーなど）から8秒の 16:9 クリップを作る（1クレジット/本）。全体の尺は複数ビートを生成して編集ソフトでつなぐ前提で、30〜60秒の並び（フライスルー → 盛り上がり → クライマックス → タイトル）を勧める。実機映像は使わない — [VO3 AI](https://www.vo3ai.com/tools/ai-game-trailer-generator)

### Inferences
- 「hook → establish → escalate → title」という型は、公開されている助言とほぼ重なる。差別化は型そのものではなく、型を実機から撮り直せる定義（shots.yaml）に落とす点にある。
- VO3 のような生成クリップは、実際のゲームと違う画を見せるリスクがある（shotpilot が避けている点）。

### Gaps
- ビートを構造化データとして持つゲーム向け製品は見つからなかったが、網羅的な検索ではない。
- 提出日が近いので、競合として確定的に「存在しない」と書くのは避ける。

## Q4. Iteration: can one shot change without redoing the rest?

### Takeaway
AI 絵コンテ系は「コマ単位の再生成」までは持つが、変更の影響範囲の管理（どこが古くなったか）は確認できなかった。Boords の書き出しは一回限りのコピー。

### Cited Findings
- コマ単位の再生成: LTX（フレーム階層で調整）、Boords（1コマをプロンプトか画像で）、Storyboarder.ai（変種・コマ単位の再生成）、Adobe Firefly Boards（パネル単位の反復）— [diyai.io](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/)
- Boords は書き出しが一回限りのコピーで、変更したら出し直す。アニマティックのトランジションと字幕は編集ソフト側で足す — [Boords export](https://boords.com/export)
- 同じ記事は、Storyboarder.ai について「書き出し品質とバージョン復元を試すべき」と注意している — [diyai.io](https://diyai.io/ai-tools/video-generation/ai-storyboard-generator/)

### Inferences
- 調べた範囲では、「定義の指紋から、どのショットが古いかを判定して、そのショットだけ撮る」という仕組みを持つ製品は確認できなかった。ここが D-13 の部分修正ループの独自性になりうる。ただし、各製品の内部の仕組みを検証したわけではない。
- shotpilot が学べる点: Boords の Premiere/Resolve XML、AE の .jsx 書き出しのように、Cut の出力を編集ソフトに渡す経路は需要があるらしい（`cut` の発展候補）。Storyboarder の XML と PNG の組もこの型。

### Gaps
- 各 AI ツールが「1コマ直したら前後の連続性がどうなるか」の実使用報告は見つからなかった。
- UE 側の Movie Render Queue でショット単位に再レンダする既存ワークフローは、今回の範囲外で未調査。

## 補足: 情報の信頼性
- diyai.io、Atlabs、Boords 比較ページ、Filmustage 比較ページは、記事の書き手が製品を売る立場にあり、競合や自社の宣伝を含む。
- 価格はほぼすべて二次情報で矛盾がある。提出物に書くなら公式ページで再確認すること。
- Boords の検索結果に「2027 年のランキング」や「Screenshot captured October 14, 2026」のような不自然な記述を含む低品質ページがあり、採用していない。
