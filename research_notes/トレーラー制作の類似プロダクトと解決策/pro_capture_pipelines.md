# プロのトレーラー制作・キャプチャ工程と解決策(How the pros solve capture / reproducibility / recapture / review)

調査範囲メモ: WebSearch/WebFetch 約15回。一次ソースを直接読めたものと、検索要約のみのものを明記する。Kert Gartner、Ant Workshop、Mammoth、Antimatter、Golden Ninja、Squeaky Wheel の制作工程は、信頼できる一次ソースを見つけられていない(Gaps参照)。日本語の制作会社の工程解説も見つからなかった。

## 1. 何を見せるか決める(台本、ショットリスト、ビート)

### Takeaway
プロの流れは「ビルドを触る → 台本を複数案書く → クライアントが1案を選ぶ → キャプチャディレクターが、撮れるものに合わせて台本を直し、台本の物語に沿ったショットリストを作る」。ショットリストは台本の後に来て、台本自体が「撮れること」で修正される。

### Cited Findings
- キャプチャディレクター(Jason White)は、クライアント(多くはパブリッシャー)から作業中ビルドを受け取る。マーケティングのブリーフは薄く、新バイオームや新機能の箇条書き程度のことが多い。ライターがビルドとクライアントの要望から台本を複数案書き、Jason が「ゲームプレイとして正確か、撮れるか」の観点で調整し、クライアントが選ぶ。選ばれた後に、台本の物語を語るショットリストを作る。 — [Aftermath: video game trailer capture director](https://aftermath.site/video-game-trailer-capture-director-maker/)
- 同じ人物は、制作者(プロデューサー、ライター、エディター)がプレイする時間を持てないため、"What Does It Feel Like to Play" 文書を書いて渡す。プレイして面白い・見栄えがする・仕組みとして興味深い点をメモする。キックオフでチーム全員がビート、ビジュアルモチーフ、ジョークを出す。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- 最初に理想形の「V1」を作る。V1 はクライアントが最終的に望むものより大胆なことが多く、その後に双方が受け入れる妥協点へ進む。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- EA の Game Capture チームは、キャプチャ前に制作・クリエイティブ・編集を集めた「creative alignment meeting」を持つ。使うマップ領域、武器、装備、乗り物を決め、ロケ地を下見して、開発パートナーと動きを組む(ブロッキング)。1回目のパスはほぼ必ずブロッキングパスで、リード(責任者)が各メンバーに「ここで位置につき、プレイヤーXを撃て」のような目標を与える。 — [CGW: EA's Game Capture Artists (Battlefield 2042 Hazard Zone, 2021)](https://cgw.com/Press-Center/In-Focus/2021/EAs-Game-Capture-Artists-Bring-Battlefield-2042-.aspx)
- Blizzard(BlizzCon 2026 "TrailerCraft")の WoW レイドボス例は、簡単なスケッチ → 背景 → カメラの動き、の順。「空っぽに見える」というフィードバックで敵や環境物を足した。 — [Inven Global, 2026-09-12](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- 開発側のコスト削減としての別アプローチ: GDC 2024 Independent Games Summit に「Make the Trailer Before the Game: A Marketing First Way to Prototype」というセッションがある(内容は未確認、題名のみ)。 — [GDC schedule](https://schedule.gdconf.com/session/independent-games-summit-make-the-trailer-before-the-game-a-marketing-first-way-to-prototype/900472)

### Inferences
- プロの順序は「台本(ブリーフ)→撮れるものとの突き合わせ→ショットリスト」で、shotpilot の「台本が先」(D-31)と同じ向き。ただしプロの突き合わせは人間(キャプチャディレクター)がビルドを遊び込んで行っており、ここが自動化の余地として大きい。
- V1 を大胆に作ってから妥協する流れは、「まず安く全体を出し、ショット単位で直す」と相性がよい。

### Gaps
- アニマティック(撮影前の動く絵コンテ)をゲームトレーラーで標準的に使うという一次ソースは見つからなかった。
- ビートシートの具体的なフォーマットは見つからなかった。

## 2. キャプチャ(キャプチャビルド、デバッグカメラ、チートメニュー、時間帯、NPC、フレーム固定)

### Takeaway
プロは「キャプチャ用ビルドを専用に作る」より、既存のデバッグ機能(Unreal のデバッグカメラ、開発者用 ImGui ツール、チート、フリーカム)を探して使い倒す。足りないときはゲーム側に小さなツールを足してもらう。それが乏しいと「時間と問題解決を大きく取られる」。

### Cited Findings
- Jason White: ツールは Unreal の標準デバッグカメラから、カメラ・被写界深度・キャラクター生成・ライティングを操作する開発者製 ImGui ツールまで幅がある。この制約の中で撮ることが「takes up a lot of time and problem solving」。撮影は1人でも、2人目がキャラを操作して1人がシネマティックカメラを操るか、複数の Capture Tech がシーン内でキャラを演じる形でも行う。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- Derek Lieu のデバッグツール推奨: HUD オン/オフ、オブジェクトのハイライト非表示、カーソル非表示、音楽オフ、レベルスキップ・無敵・インベントリ管理(「ベアミニマム」)。 — [Derek Lieu: The Best Debug Tools for Game Trailer Capture (2018)](https://www.derek-lieu.com/blog/2018/5/1/the-best-debug-tools-for-game-trailer-capture)
- 同記事: 時間帯を変えて固定できること("freeze it when you find a look that you like")、NPC を操作できること("an option to control them will help for shot composition")、フリーカム、フリーカム併用の時間停止、1/2・1/4・1/8 のスロー(編集側で正確な割合で戻せるため)。 — [Derek Lieu (2018)](https://www.derek-lieu.com/blog/2018/5/1/the-best-debug-tools-for-game-trailer-capture)
- GDC 2018「Capturing Great Footage for Game Trailers」(Derek Lieu)は、キャプチャを「演者・監督・カメラオペレーターの役」と位置づけ、デバッグツールとカメラ操作を扱う。ページには要約しかなく詳細は未確認。 — [GDC Vault](https://www.gdcvault.com/play/1025079)
- Blizzard の内部デバッグツール: fly/ghost モード、無敵、無限リソース、キャラ・アイテムの自由配置。Hearthstone ではマナとカードを調整して盤面を再現し、WoW ではキャラ・クリーチャーを配置する。ライティングを直接変えてキャラを背景から分離し、マップエディタで足りない物体(Heroes of the Storm の格闘リングとヘリコプター)を作る。複雑な戦闘は最適化していないビルドでスロー撮影して、編集で戻す。 — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Blizzard の "Audition Entity": 1人のアーティストが各ヒーローの動きを順に記録して1シーンに重ねる。記事は2018年の Overwatch 6対6シーンで13人・8時間が必要だった例と対比している。 — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Blizzard は深度パスとキャラ・背景レイヤーを抽出し、グラフィックスチームが最終ショットに仕上げる。 — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Saints Row: The Third(GDC 2012、Alexander Mejia, Volition): 既存のデバッグカメラ「Slew mode」を改造した Machinima_Mode を作った。実装はプログラマー約2日。外注先に少ない訓練でライブアクション風の映像を撮らせる狙いで、外注からのキャプチャ依頼が90%減ったと報告している。2012年の古い事例だが、「ゲーム側のカメラツールを外注に渡す」ことを直接扱う数少ない一次ソース。 — [GDC Vault, 2012](https://gdcvault.com/play/1015354/Saints-Row-the-Third-Real)
- EA の Game Capture は、1日の始めに開発パートナーと連絡して「最新ビルド」で作業することを確認する。ドーリーズ(1ショットの複数テイク)を見て選び、Frame.io へ上げる。 — [CGW, 2021](https://cgw.com/Press-Center/In-Focus/2021/EAs-Game-Capture-Artists-Bring-Battlefield-2042-.aspx)
- フライトシミュレーターのフォーラム投稿では、チームがプレイヤーにも提供されているリプレイツールをトレーラーに使うと述べている(公式側の発言とされるが、検索要約のみで本文は未取得)。 — [MSFS forum](https://forums.flightsimulator.com/t/any-chance-of-a-behind-the-scenes-video-for-the-trailer-making-process/526447)

### Inferences
- 「ショットごとに、時間帯・NPC・カメラ・ゲーム状態を決めて撮る」は人手で毎回やっており、その設定は台本ではなくキャプチャ担当の頭と作業メモにある。shots.yaml のようなマシン可読な形で持つ運用は、見つけた範囲では無かった。
- 「フレーム固定のオフライン描画」は、今回のソースでは Blizzard の Sequencer(正確な再生)と深度パスが最も近いが、オフライン描画を明言した記述は見つからなかった。

### Gaps
- Bungie / Naughty Dog / Insomniac / Ubisoft / Rockstar / Riot のキャプチャツールを扱った GDC 講演は見つからなかった(Saints Row 2012 と Lieu 2018 のみ)。
- 「トレーラーモード」の名前で出荷されている機能の一次ソースは見つからなかった。
- 日本語の PV 制作会社のキャプチャ工程の記述は見つからなかった。

## 3. 再現性(リプレイ、決定論的キャプチャ、セーブ状態)

### Takeaway
再現性の手段は主に「ゲーム状態のリセット/巻き戻し/リプレイ」と「Sequencer のような記録再生」で、ショット定義から機械的に再生成する仕組みは見つからなかった。

### Cited Findings
- Derek Lieu: 「any function which can reset the game's state to what it was a few seconds or minutes ago is a HUGE time saver」。例として Tacoma の巻き戻しと Spelunky 2 のレベル再開を挙げ、リセットがあるとリテイクが簡単になる。 — [Derek Lieu (2018)](https://www.derek-lieu.com/blog/2018/5/1/the-best-debug-tools-for-game-trailer-capture)
- Blizzard の Sequencer は「同じシーンを任意の時点でそのまま再生できる」と説明されている(記事の言い換え)。 — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Derek Lieu 関連の二次記事に、リプレイは CPU 負荷がプレイより軽く FPS が上がることがあるという記述がある(二次情報で、本人の記述かは未確認)。 — [Jikguard summary of the GDC talk](https://www.jikguard.com/news/video-capturing-great-footage-for-game-trailers)
- EA はドーリーズ(複数テイク)を撮って選ぶ運用で、決定論的な再生ではなく「撮り直して選ぶ」ことで再現性の問題を回避している。 — [CGW, 2021](https://cgw.com/Press-Center/In-Focus/2021/EAs-Game-Capture-Artists-Bring-Battlefield-2042-.aspx)

### Inferences
- プロの標準は「リプレイ・リセットがあればラッキー、無ければ複数テイクで選ぶ」。決定論的キャプチャは開発側の協力が前提で、外部の制作会社には期待できない。
- そのため shotpilot の「定義から撮る」は、プロの運用に対する差別化になりうる。ただしそれは UE の Sequencer を自分で組める UE プロジェクトに限った話。

### Gaps
- レースゲーム・スポーツ・格闘ゲームのリプレイをトレーラーに使う工程の一次ソースは、今回は見つからなかった。

## 4. ビルドが変わった後の再キャプチャ

### Takeaway
再キャプチャは日常的に起こり、人手で管理されている。ビルド番号でフッテージを管理する、またはショットリストを新ビルドへ自動で再実行する仕組みを書いた一次ソースは見つからなかった。

### Cited Findings
- Jason White: V1 とピクチャロックの間に、ノートに応えて新しい映像を撮りに戻る、またはビルドの変更・更新後に既存の映像を撮り直す。スケジュールが途中で削られたり変わったりして難しくなる。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- Jason White: 撮影日の終わり、または必要なときに、Games Coordinator へ映像を渡し、編集パイプラインへ取り込む。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- EA: 毎朝、開発パートナーと最新ビルドを確認する。EA の Game Capture Lead の求人は、開発パートナーと協力して「ビルドの準備状況」を、キャプチャチーム・プロデューサー・クリエイティブリードに共有することを職責に含める(検索要約で確認、求人本文は未取得)。 — [CGW, 2021](https://cgw.com/Press-Center/In-Focus/2021/EAs-Game-Capture-Artists-Bring-Battlefield-2042-.aspx); [EA job listing](https://ea.gr8people.com/jobs/158197/game-capture-lead)
- Blizzard は未完成のビルド(グレーボックス、仮アニメーション、未完成のエフェクトやテクスチャ)に当たることがあり、コミュニティは誤った武器などのミスを見つけるため、ゲームとの一致に気を使う。 — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Digic Studios によると、サードパーティのトレーラー制作はゲーム制作の中盤ごろから始められ、質の高いトレーラーは数か月かかる。 — [GamesBeat: Digic](https://gamesbeat.com/production-studio-digic-sheds-light-on-how-game-trailers-are-created/)

### Inferences
- 変更されたショットだけを撮り直す仕組みは、プロの現場では公開されていない。再キャプチャは「ノートが来たら担当者が該当ショットを思い出して撮り直す」という属人運用に見える(推定)。
- 「最新ビルドの確認」が毎朝の人手作業になっていること自体が、ビルドを入力とした再実行の需要を示す。

### Gaps
- ビルド番号によるフッテージのログ管理、「新ビルドでショットリストを再実行」する自動化は、公開情報では見つからなかった。社内の非公開運用の可能性が高く、「無い」とまでは言えない。

## 5. レビューと承認(カット・フレームごとのノート、ノートの一本化)

### Takeaway
業界の標準はタイムコード付きのレビューツール(Frame.io 等)と、クライアント側の窓口1人によるノートの一本化。EA はキャプチャのドーリーズ段階から Frame.io で承認を取る。

### Cited Findings
- EA は選んだドーリーズを Frame.io に上げ、パートナーがレビューし、リードがキャプチャのノートを書き込む。 — [CGW, 2021](https://cgw.com/Press-Center/In-Focus/2021/EAs-Game-Capture-Artists-Bring-Battlefield-2042-.aspx)
- 制作会社のブログでは、クライアント側に「Champion」を1人置き、社内のフィードバックを集めて衝突を解決し、1回分のノートとして編集者に送ることを勧め、ばらばらに届くノートを避けるよう警告している。同じ会社は標準契約に2回の統合リビジョンを含め、Frame.io で運用する(1社の慣行で、業界標準ではない)。 — [検索結果の要約元(Frame.io系ブログ)](https://fast.io/resources/video-feedback-tool/)
- タイムコード付きフィードバックは、見る人と編集者の間の翻訳のずれをなくすと主張されている(ツール提供元のブログで、割り引いて読む)。 — [Fast.io](https://fast.io/resources/video-feedback-tool/)
- Blizzard の例では、レビューのフィードバック(「空っぽに見える」)が追加の要素の投入につながった。 — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- Jason White は編集者と作業中ずっと連携して、ノートを出し、追加で必要な映像を決める。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)

### Inferences
- ノートは「動画のタイムコード」に付き、それをキャプチャ担当がショットに読み替える。ショット単位の ID を定義側に持てば、この読み替えが消える。

### Gaps
- ゲーム業界の発行元が承認を何ラウンドで行うかの一次ソースは見つからなかった。

## 6. 料金と、クライアントが用意するもの

### Takeaway
公開されている価格は幅が広く、キャプチャが標準で含まれるかは明記されないことが多い。ほぼ全員が「ビルドを早く」求める。大手の価格情報はトレーラー制作費ではなく放映枠のものが混ざるので注意。

### Cited Findings
- Trailer Farm は、クリエイティブ・台本・ゲームキャプチャ・編集・音響・ローカライズを手掛ける代理店と紹介され、インディー向けの Trailer Farm Indie は「AAA の価格なしで」という固定の透明な価格をうたうが、金額は公開されていない。クライアントの関与度は選べる。2014年ごろの情報で古い。 — [MCV/Develop: Services spotlight, The Trailer Farm](https://mcvuk.com/business-news/publishing/services-spotlight-the-trailer-farm/); [PocketGamer.biz: Trailer Farm Indie](http://www.pocketgamer.biz/industry/trailer-farm-indie/)
- Alconost のブログは、低予算 $1,000-2,000、中 $2,000-3,000、高 $3,000+ とし、長さよりシーン数が価格に効くとする。 — [Alconost](https://blog.alconost.com/en/game-trailer-video-cost)
- Pixune は 1分のトレーラーを 2D アニメで $5,000+、3D で $20,000+ と見積もり、台本が $200-500、自分でゲームプレイを録画するとコストを抑えられるとする(アニメ寄りの見積もり)。 — [Pixune](https://pixune.com/blog/how-much-does-a-video-game-trailer-cost/)
- 小規模サービスの一例: ティーザー $150-250、コアのゲームプレイ $350-500、いずれもクライアント提供の映像を使う。 — [検索要約より、元ページの特定は未確認](https://pixune.com/blog/how-much-does-a-video-game-trailer-cost/)
- Fiverr の出品では、Basic は「ゲームへの完全なアクセス」が必要で、上位ティアは独自にキャプチャを含み得る。ソースファイルはプレミアムに含まれ、下位ではオプション。 — [Fiverr listing](https://fiverr.com/chaoskernel/create-cinematic-game-trailers-and-teasers-15-to-90-sec)
- Summer Game Fest のトレーラー枠は、1分 $250,000、90秒 $350,000、2分半で最大 $550,000(Esquire の報道を Game Developer が引用、2024年6月)。これは放映枠の費用で、制作費ではない。 — [Game Developer](https://gamedeveloper.com/business/report-summer-games-fest-trailers-can-cost-devs-up-to-550k)
- クライアントが用意するもの: ビルド(早いほどよい)、ブリーフ、承認。Room8 のオンボーディングはゴール、観客、プラットフォーム、美術の方向、予算を聞く。 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/); [Room8](https://room8studio.com/news/video-game-trailer-production-a-glance-behind-the-scenes/)

### Inferences
- 低価格帯は「クライアントが映像を撮って渡す」前提で成り立つ。キャプチャが重い・遅い・人手依存の部分であり、そこを自動化すれば低価格帯の限界を押し広げられる。

### Gaps
- Ant Workshop、Mammoth、Antimatter、Golden Ninja、Squeaky Wheel の料金は見つからなかった(社名の実在確認もしていない)。
- キャプチャを含む価格ティアを明記した制作会社は見つからなかった。

## 高コスト・低速な部分(まとめ)

- 1シーンの人手: Overwatch の6対6シーンに13人・8時間(2018年、Blizzard 側の比較。Audition Entity 以前) — [Inven Global, 2026](https://www.invenglobal.com/articles/25875/everything-blizzard-says-about-creating-gameplay-trailers)
- デバッグツールが乏しいゲームでの制約下の撮影は多大な時間を食う — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- 高品質なトレーラーは数か月 — [GamesBeat: Digic](https://gamesbeat.com/production-studio-digic-sheds-light-on-how-game-trailers-are-created/)
- ビルド変更後の撮り直しと、途中のスケジュール変更 — [Aftermath](https://aftermath.site/video-game-trailer-capture-director-maker/)
- 外注へのキャプチャ支援でスタッフ時間を取られる(Saints Row 2012) — [GDC Vault](https://gdcvault.com/play/1015354/Saints-Row-the-Third-Real)

## 出典の信頼度メモ

- 直接読んだ(本文取得): Aftermath、GDC Vault(Saints Row、Lieu のセッションページ)、Derek Lieu のデバッグツール記事、CGW(EA)、Inven(Blizzard、日付が2026年9月で新しい。内容は取得ツールの要約なので原文確認を推奨)、Room8、Game Developer(SGF)。
- 検索要約のみ(本文未確認): MCV/Develop の Trailer Farm、GamesBeat の Digic と EA、MSFS フォーラム、Fiverr、Pixune の一部、Frame.io 系ブログ、EA 求人。
- Derek Lieu の記事は URL が日付違い(2018/3/21 は 404、2018/5/1 で取得)。
