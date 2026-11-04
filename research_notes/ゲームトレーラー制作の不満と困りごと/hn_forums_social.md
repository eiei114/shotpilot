# ゲームトレーラー制作の不満と困りごと: HN・フォーラム・SNS（Reddit 以外）

調査日 2026-11-05。結論を先に書くと、**目標の「一次の不満の逐語引用 15 件以上」は達成できなかった**。取得できて、かつ出典・日付付きで引用できたのは下記の約 10 件で、そのうち 2019 年以降の純粋な「不満」と言えるものはさらに少ない。この領域の一次の不満は Reddit・X・Bluesky・YouTube に集中していて、今回のアクセス経路では届かなかった。以下は取得できたものと、できなかったことの正確な記録。

アクセス結果:
- Bluesky 公開検索 API（public.api.bsky.app searchPosts）は curl で `403 Forbidden`。認証か別経路が必要。
- Mastodon の検索 API（v2/search）は試していない（通常は認証が必要）。タグのタイムライン（mastodon.gamedev.place の `#gamedev` `#indiedev` `#trailer` `#gametrailer`）は取得できたが、「トレーラーを作る苦労」を述べた投稿は見つからず、宣伝投稿ばかりだった。
- HN（Algolia）は comment/story を 12 クエリほど検索。「trailer」を含むコメントは大半がゲーム制作と無関係（GameTrailers サイト、年齢確認など）。制作の苦労を述べた HN コメントは見つからなかった。
- X/Twitter、YouTube コメントは WebSearch で一件も取得できず（検索結果にそもそも出ない）。
- Steam コミュニティと TIGSource は、検索に該当スレッドが出ず確認できなかった。Steamworks 開発者フォーラムは非公開。
- Unreal フォーラムの MRQ 系スレッド（タイトルのみ確認。本文は取得失敗または未取得）。

## 1. 開発者はトレーラー制作のどの工程で何に困っているか（何を見せるか・撮影・編集・音楽・時間）

### Takeaway
取得できた一次の声では、困りごとは「何を見せるかが決まらない」「録画素材を山ほど撮って切り出す手間」「無料編集ソフトの不具合」「自作が気に入らない」に集中していた。コスト（外注費）や Steam 仕様の不満は、今回の経路では一次の声として取れていない。

### Cited Findings
引用は逐語。日付が相対表記（「N years ago」）のものは、取得日 2026-11 から逆算した概数で、その旨を付す。

- 【編集ツール／complaint】「Shotcut is good to start but you should move on to something more professionnal as soon as you have the chance IMO. The many problems I had are: Blur lagging so much I had to export the entire video to see how it looks like / Font border gets ugly for more than 5 px / Audio desync present in the video preview of the software but not in the export / Impossible to move groups of clips, so you need to export parts of the video to be able to move it as one block / Various crashes」— Green Frisbee Games、itch.io フォーラム（投稿は「7 years ago」表記なので約 2019 年）— [How can I create a good looking trailer with limited resources?](https://itch.io/t/212517/how-can-i-create-a-good-looking-trailer-with-limited-resources)
- 【何を見せるか・撮影／advice】「Don't just record gameplay! It doesn't work. Instead, look at trailers for successful games (Indie and AAA) and copy what they do!」— WarpZone、同スレッド（「8 years ago」表記、約 2018 年。2019 年より前の可能性あり）。同スレッド metalsnail は「your trailer could be mostly gameplay footage, but probably in clips of no more than three seconds」「I use OBS to capture gameplay, it's fiddly, but has the good features and plays nice with my hardware.」と助言（約 2018〜2019 年）— 同 URL
- 【何を見せるか・撮影／confusion】「I want to get more views and downloads on my game. This is why I want a good trailer… How will I be able to do this with limited resources? (I don't own any movie production software like Vegas or Premiere)」— Jort Driegen、同スレッド、2018-04-02（2019 年より前。参考）— 同 URL
- 【ゲームが変わった後の作り直し／complaint（状況の説明）】「the game have massive rework. and now is completely different we need new steam page, and for sure a new trailers.」— Alex、itch.io「[EDITING] [FREE] Professional Video Editor making game trailers」スレッド、2024-06 頃（スレッド作成 2024-06-17、投稿は「2 years ago」）— [itch.io](https://itch.io/t/3848261/editing-free-professional-video-editor-making-game-trailers-)
- 【自作の出来／complaint】「I just finished my classic arcade-inspired game and created my own trailer, but I don't like it and it isn't intriguing enough.」— BloodyFish、同スレッド、2024-06 頃 — 同 URL
- 【外注・編集の需要／事実】無料でトレーラーを編集すると申し出たスレッドに複数の開発者が殺到し、編集者が求める素材は「gameplay footage (without music, only with sfx) / logo / font / music / GDD, or just info about the game」と列挙。編集者自身が「I'm doing the work for free」と書き、支払い能力のない開発者が「I don't have any money to give, I'll be happy to promote you on my socials」と返している — MizuMiaw、BloodyFish ほか、同スレッド、2024-06 頃 — 同 URL
- 【撮影の手間／事実】「After choosing the software, i did over 3 hours of recording where i clipped all these recordings into the best scenes.」— dementedplant（Godstone devlog 2）、itch.io、約 5 年前（約 2021 年。「5 yr」表記）— [Godstone dev log 2: Trailer Release!](https://dementedplant.itch.io/godstone/devlog/293321/godstone-dev-log-2-trailer-release)
- 【何を見せるか／complaint】「it might be a bit difficult to make one if you don't have an action-based game with lot of flashy onscreen movement.」「The gameplay of our game is fairly hard to understand without playing it yourself」— Placeholder Gameworks（Death and Taxes devlog 19）、itch.io。日付は取得したページから特定できず — [DevLog 19: Making a Trailer for a Non-Action Game](https://placeholdergameworks.itch.io/death-and-taxes/devlog/119570/devlog-19-making-a-trailer-for-a-non-action-game)
- 【時間／事実】「Invested hours: Gathered materials, recorded, and edited game trailer: 2.」— lekib（Project C devlog）、itch.io、2025-05-20 — [Project C game trailer](https://itch.io/devlog/948894/project-c-game-trailer.amp)
- 【ゲームが変わった後の作り直し／示唆】「New #gameTrailer for my tactical #towerDefence game Fomorian Company, matching the current UI and art style」— aceade@mastodon.ie、2026-10-15（新トレーラーの理由が「現在の UI とアートスタイルに合わせる」）— [Mastodon](https://mastodon.ie/@aceade/117445006901540861)
- 【時間・計画／complaint（2019 年より前、参考）】「This process was time-consuming, especially during planning. And most of the time the trailers we created didn't really grab the audience's attention as much as we would have hoped.」「We kept weighting what to show in the trailer without having the time to deliberate.」— Pawee Pakamekanon（Project Nimbus）、Game Developer、2016-04-06。過去の手順は「Captured footages using in-game playthough. We used a camera-switching script to help capturing the scene from different angles」— [Game Developer](https://gamedeveloper.com/business/how-our-small-indie-team-created-a-trailer-for-our-biggest-early-access-update-yet-)
- 【コスト（2019 年より前、参考）】「you don't have $2000 to hire a video production team to make a trailer for you… You have $100, because you are indie, and you are poor.」— Aksel Junkkila、Game Developer、2016-02-10 — [How we made a game trailer for $100](https://www.gamedeveloper.com/business/how-we-made-a-game-trailer-for-100-)
- 【プラットフォーム／complaint】HN 投稿「YouTube removes indie dev's game trailer for piracy」— Farbklex の説明文「The video is made in the style of 2000s instruction videos for downloading games for free. But it only shows how to get to the Steam page of the dev's game to download the demo. YouTube still removed the video…」2024-02-24 — [HN item 39491194](https://news.ycombinator.com/item?id=39491194)

スニペットのみ（ページ本文を確認していない。引用として使う場合は要確認）:
- 検索結果の要約によると、「itch.io の devlog 作者が、作業途中の生のビルド映像からトレーラーを作り、"hate" と書いた」とされる（[itch.io devlog](https://itch.io/devlog/1063397/poc-first-roughttrailer.amp) の要約。ページ本文は取得失敗のため逐語なし）。
- 検索結果の要約によると、「2026 年のソロ開発者の投稿が、マーケティングは仕事の半分で嫌いになる、と述べる」とされる（出典 URL が特定できず未採用）。

### Inferences
- 一次の声には「録画素材を大量に撮って切り出す」「自作が気に入らず外注・無料編集者に頼る」という二つの型があり、後者は編集者に渡す素材一覧（映像・ロゴ・フォント・音楽・GDD）まで書かれていて、依頼の手間そのものが摩擦になっている。
- ゲームが変わるとトレーラーと Steam ページを丸ごと作り直す、という作り直しの動機は、Alex と aceade の二例で裏付けられる。ただし二例とも「作り直しが苦痛だ」とまでは言っておらず、事実の記述にとどまる。

### Gaps
- 一次の不満の逐語引用が 15 件に届いていない。特に X・Bluesky・YouTube コメント・TIGSource・Steam は取得不能だった。Reddit 担当の調査で補うのが妥当。
- 音楽の権利や選曲、Steam の仕様・アップロードの苦労について、一次の声を取得できなかった。

## 2. 作り直し・複数バージョン管理・ツールへの願望

### Takeaway
「ショット単位で撮り直したい」「ショットを自動で組んでほしい」という明示的な願望の逐語引用は、取得できなかった。かわりに、Unreal 用のトレーラー向けカメラ道具が商品として出ていることが、需要の間接証拠として取れた（いずれも販売者の宣伝文で、開発者の不満ではない）。

### Cited Findings
- Epic フォーラムの製品紹介: 「The Trailer Camera Toolkit is a powerful real-time camera system for Unreal Engine that works directly inside your playable builds, no Sequencer, no editor required. Designed for marketing teams, cinematic capture artists, and trailer teams」— Icorice（販売者）、2026-01-06 — [Altar Creative - Trailer Camera Toolkit](https://forums.unrealengine.com/t/altar-creative-trailer-camera-toolkit/2690110)
- スニペットのみ: StraySpark のブログは「ほとんどのインディートレーラーは純粋なシネマティクスではなく、ゲームプレイ映像と制御されたカメラショットの混合で、その場合 Sequencer は摩擦になる」と主張。ただし著者は Cinematic Spline Tool の販売者で、宣伝として扱う — [StraySpark](https://www.strayspark.studio/blog/cinematic-game-trailer-without-sequencer-indie-guide)
- スニペットのみ（本文未取得）: Unreal フォーラムに MRQ/Sequencer の出力と見た目が合わない系のスレッドが多数ある。タイトル例: 「I seriously don't understand how MovieRenderQueue works」「Movie Render Queue output does not match Sequencer/Viewport」「Movie Render Queue Uses wrong camera when rendering」「Level Sequence Fails to Render out Cameras Past the First」— [forums.unrealengine.com 検索結果](https://forums.unrealengine.com/t/movie-render-queue-output-does-not-match-sequencer-viewport/1714994)。これらはバグ報告で、トレーラー制作専用の不満ではない。
- itch.io の助言（約 2018〜2019 年）として「Make two or three trailers and choose the best one.」— woodsmoke、同上スレッド — [itch.io](https://itch.io/t/212517/how-can-i-create-a-good-looking-trailer-with-limited-resources)

### Inferences
- 「Sequencer を使わずに撮れる道具」が複数商品化されていることは、Sequencer/MRQ の設定コスト自体が障壁だという見方を支えるが、開発者本人の言葉では確認できていない。

### Gaps
- 「1 ショットだけ撮り直したい」「バージョンを複数持つのが面倒」といった願望の逐語引用は見つからなかった。

## 3. AI 製・自動生成トレーラーへの反応

### Takeaway
AI 製トレーラーへの反応は強い否定が中心で、開発者側は弁明・謝罪に回っている。ただし今回取れたのはニュース記事のスニペットで、開発者本人や HN・SNS の逐語ではない。

### Cited Findings
スニペットのみ（記事本文は未取得。要約を転記）:
- ARK: Aquatica DLC（GDC 2025）のトレーラーが AI 生成として批判され、元の開発者は「The marketing department used still images of some of our assets to create an AI-animated trailer」と距離を置いた。YouTube から取り下げられた — [PC Gamer](https://pcgamer.com/games/survival-crafting/ark-devs-distance-themselves-from-ai-generated-trailer-we-did-not-know-that-they-were-doing-it)
- Fallen（Superboo Studios）のインディートレーラーが AI 画像の混入を指摘され、制作者は「tone pitch」の AI "placeholders" だったと説明 — [Robots Over Dinosaurs](https://robotsoverdinosaurs.net/?p=207761)
- Humankind 2 は、AI 疑惑を否定して別映像で証明した — [Kotaku](https://kotaku.com/game-accused-of-using-ai-to-generate-its-reveal-trailer-proves-it-was-made-by-people-2000731291)
- HN には 2024-10-30 付の投稿「We tried to make a strategy game trailer with AI. Oh boy (prompts included)」（gamescodedogs）があるが、コメントは取得できず内容は不明 — [HN item 41998587](https://news.ycombinator.com/item?id=41998587)

### Inferences
- AI 生成の画を「ゲームの画」として見せると炎上する、という流れは、ゲーム内映像だけで構成する Shotpilot の方針（ムードボードは参照に限る）と整合する。ただしこれは開発者の逐語ではなく、ニュース要約からの推測。

### Gaps
- インディー開発者本人が AI トレーラー生成ツールを歓迎・拒否する逐語の声は取れなかった。
