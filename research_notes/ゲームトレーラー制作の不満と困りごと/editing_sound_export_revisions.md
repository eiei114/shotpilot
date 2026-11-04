# ゲームトレーラー制作の後半工程（編集・音・文字・書き出し・修正ループ）の不満

調査上の注意: Reddit 本体は WebFetch が 403 / 接続拒否で読めなかった（r/DestroyMyGame の個別スレッドは未確認）。Reddit 由来の引用は検索結果の要約経由で、原文での照合ができていない。一次の愚痴（first-hand）は少なく、助言記事（advice）が大半。以下で「一次」「助言」を区別した。

## 編集の悩み（NLE、尺、テンポ、1本にかかる時間）

### Takeaway
1本に数週間かかるという報告が複数あり、時間の大半は素材の選別と切り詰めに消える。一次の不満は少数で、頻度は低〜中。強度は「ゲーム開発の時間を食う」という種類。

### Cited Findings
- 一次（Reddit コメント、検索要約経由・原文未確認）: 「It took me weeks full time to do mine, and I already knew how to use video editing software. So it's probably at least 2k if you get a pro.」 — [検索結果の要約](https://gamedeveloper.com/business/a-quick-guide-to-making-indie-game-trailers)（出典スレッド URL は特定できず）
- 一次（devlog、要約経由）: Godstone の開発者は、トレーラーに「way over a month」かかり、3時間超の録画から良い場面を絞り込む作業が重かったと書いている。 — [Godstone Dev Log 2](https://itch.io/devlog/293321/godstone-dev-log-2-trailer-release.amp)
- 一次（Game Developer 寄稿、年代は古い）: 100ドル予算の例で、制作の大半は2週間で、5回のドラフトを上げて批評・調整・再収録・再編集した。 — [how we made a game trailer for $100](https://www.gamedeveloper.com/business/how-we-made-a-game-trailer-for-100-)。同記事の著者は初稿のナレーションを「It sounded so clumsy to me」と述べ、非ネイティブで直せなかったと書く（WebFetch で確認）。
- 一次（Game Developer、小規模チーム）: 旧来の手順は「time-consuming, especially during planning」で、宣伝作業が「cut into the time we spend for the game itself」。台本を書いては書き直し「without much success」だった。 — [early access update trailer](https://gamedeveloper.com/business/how-our-small-indie-team-created-a-trailer-for-our-biggest-early-access-update-yet-)
- 助言: 所要「three weeks」を見込めという記事あり。急ぐと出来に出ると述べる。 — [worst trailer mistakes](https://www.gamedeveloper.com/business/worst-trailer-mistakes)（検索要約のみ）
- 助言（Derek Lieu）: AAA や映画の編集様式を真似るのが「one of the biggest mistakes a lot of indie games do」。長めで機能的なショットに引くべきと述べる。 — [Derek Lieu のブログ](https://derek-lieu-yrd2.squarespace.com/blog/2023/4/23/more-trailer-editing-is-not-better-trailer-editing)（検索要約経由）

### Inferences
- 時間のかかる部分は「編集技術」より「素材の取捨選択と構成」。NLE 学習の不満そのものは今回ほぼ見つからなかった。

### Gaps
- Premiere / DaVinci / CapCut の学習コストや「映像の人ではない」ことを訴える一次の引用は取得できなかった（Reddit 非取得）。

## 音楽・SE・音量・著作権

### Takeaway
BGM は「汎用で変化がない」ことが批評の定番。著作権クレームの一次の不満は強いが、2013〜14年と古い。音量・ローカライズ・ボイスの不満は見つからなかった。

### Cited Findings
- 助言（Lieu）: ゲームプレイ用の曲は始まりも終わりも同じに聞こえ、「flattens out the trailer's pace and makes it feel like it's going nowhere」。音を先に決めて編集すべきとも述べる。 — [Derek Lieu のブログ](https://derek-lieu-yrd2.squarespace.com/blog/2023/4/23/more-trailer-editing-is-not-better-trailer-editing)、[PC Gamer](https://www.pcgamer.com/the-art-of-making-effective-game-trailers)（検索要約経由）
- Lieu 指摘の頻出問題に「bad music choices」。 — [Game World Observer, 2022-09](https://gameworldobserver.com/2022/09/22/common-game-trailer-mistkaes-indie-developers-steam)
- 一次（Reddit、検索要約経由・スレッド特定不可）: DestroyMyGame の批評に「definitely boost some of the sound effects in the trailer. Sounds like you just recorded them from the game and thats it.」とあった。 — [r/DestroyMyGame 検索結果](https://redlib.belloworld.it/r/DestroyMyGame/comments/1dgq4ym/destroy_my_trailer_so_i_can_make_it_better)（本文は403で未確認）
- 一次（2013年、古い）: Terry Cavanagh が自作ゲームのトレーラーに INDmusic から著作権クレームを受け、異議申し立てが約1か月後に却下された。Mike Bithell も同様。 — [PCGamesN](https://www.pcgamesn.com/indie/youtube-has-rejected-terry-cavanaghs-copyright-appeal-his-own-game)、[PocketGamer.biz](https://www.pocketgamer.biz/indie-devs-hit-out-at-youtube-over-false-copyright-claims/)
- 小規模チーム（一次）: 動的な曲が多く「epicness」の1曲がなく、小さな曲を連結して代用した。 — [early access trailer 記事](https://gamedeveloper.com/business/how-our-small-indie-team-created-a-trailer-for-our-biggest-early-access-update-yet-)
- 一次: 追加トラックを Melodyloops.com で20ドルで購入。 — [$100 trailer](https://www.gamedeveloper.com/business/how-we-made-a-game-trailer-for-100-)

### Inferences
- Content ID の古い事例は、音源のライセンス証跡を残す必要の裏付けになるが、現在の頻度は不明。

### Gaps
- ラウドネス（LUFS）、ビートへのカット合わせ、ボイスオーバーの一次の愚痴は見つからなかった。2020年以降の著作権クレームの一次報告も未取得。

## 文字・タイトル・エンドカード・字幕・ローカライズ

### Takeaway
タイトルカードは「汎用的な機能の羅列」と「ゲーム名が出ない」が批評の核。ローカライズ版トレーラーの愚痴は未発見。

### Cited Findings
- 助言（Lieu）: 「Don't use title cards at all if you don't have something interesting to say with them」。カードがキーアートと重なると動画プレーヤーの進行バーに隠れる。 — [Derek Lieu のブログ](https://derek-lieu-yrd2.squarespace.com/blog/2023/3/11/i-made-the-worst-firewatch-trailer-ever)（検索要約経由）
- 助言（Lieu）: ナレーションやカードで言及したのに映さないのは典型的な失敗。情報の詰め込み（字幕、UI、特徴、キャラ名）も失敗。 — 同上（要約経由）
- 一次（Reddit、要約経由）: 「the name of the game isn't in the trailer!」。冒頭と終わりで名前を出し行動喚起を置くよう勧めるコメント。 — [r/DestroyMyGame](https://redlib.belloworld.it/r/DestroyMyGame/comments/1fp448h/destroy_my_games_first_trailer)（要約のみ）
- Steam は、視聴者が音なしで見ている可能性を明記している。 — [Steamworks](https://partner.steamgames.com/doc/store/trailer)

### Inferences
- 音なし視聴が前提なので、字幕・カードはピクセルに焼く必要がある。

### Gaps
- 言語別トレーラー（Steam は言語別・国別の設定に対応）を作る負担の一次の愚痴は見つからなかった。

## 書き出し・プラットフォーム仕様

### Takeaway
Steam の仕様は公式で明確。失敗の主因は解像度と音声サンプルレートで、再投稿は変換キューの最後尾に回る。祭典（Next Fest / Wholesome Direct / Day of the Devs）の仕様は取得できなかった。

### Cited Findings
- 公式: 最大 1920x1080、30/29.97 または 60/59.94 fps、5,000+ Kbps、.mov / .wmv / .mp4、H.264 + AAC 推奨、16:9 推奨（4:3 可）、音声は 44kHz か 48kHz。 — [Steamworks](https://partner.steamgames.com/doc/store/trailer)
- 公式: 「The most common reason for a trailer to fail is the resolution of the video.」音声サンプルレートも失敗原因。変換は FIFO キューで数時間かかり、「Deletion / re-submission will move the trailer to the back of the queue.」。変換中は発売できない。 — 同上
- 公式: ポスター画像は動画内のフレームで 1920x1080 の jpg/png。マイクロトレーラー（6秒ループ）は店ページの最初の動画から自動生成され、カスタム不可。「may have less than 10 seconds to make an impression」（Discovery Queue）。 — 同上
- 報道: Valve は新プレーヤーに合わせて約40万本のトレーラーを再エンコードした。 — [TweakTown](https://www.tweaktown.com/news/106750/steam-has-400-000-trailers-and-valve-had-to-re-encode-all-of-them-for-new-video-player-update/index.html)（本文は403で未確認、見出しのみ）
- 助言: 縦型 10〜30 秒版を別に編集せよという案内。ショート動画は「1つの具体的な瞬間」を見せるべきで、ミニトレーラーではないとする。 — [itch.io blog](https://itch.io/blog/1030474/creating-a-trailer-that-sells-your-game-a-complete-guide-for-indie-and-aaa-developers.amp)、[geekchamp](https://geekchamp.com/promote-an-indie-game-across-tiktok-and-youtube-shorts/)（要約経由）

### Inferences
- 解像度・サンプルレートの検証を書き出し前に自動化すれば、Steam の再投稿キュー落ちを避けられる。

### Gaps
- Steam の自動再生がミュートか、Next Fest・Wholesome Direct・Day of the Devs の提出仕様は、公式ページを取得できず未確認。

## 修正ループ（フィードバック、アート変更、更新ごとの作り直し）

### Takeaway
「再編集・再収録」は一次の記述があるが数は少ない。アート変更でトレーラーを作り直した例が確認できた。バージョン数の定量データはない。

### Cited Findings
- 一次（itch.io devlog、2025-11-06）: 「So, after redoing the enemies graphics, I had to redo the trailer.」 — [The Workshop Geek devlog](https://itch.io/devlog/1105267/gameplay-trailer.amp)（工数の記述なし）
- 一次: 5回のドラフトを上げ、批評を受けて調整・再収録・再編集した。 — [$100 trailer](https://www.gamedeveloper.com/business/how-we-made-a-game-trailer-for-100-)（要約経由、年代は古い）
- 一次: 初稿ナレーションを第三者（Reddit のファン）に見せて書き直し案をもらい、「much better」と評価。 — 同上（WebFetch で確認）
- 助言（Reddit 投稿の要約）: ヒーロー映像を軸に、機能が変わったらブロック単位で差し替えるべきという案。 — [r/gamedev スナップショット](https://reddit.sentinel-team.org/posts/1u6i39v/snapshots/2026-06-16T00%3A31%3A58.357124Z)（本文未確認、要約のみ）
- 助言: 映像がゲームの実際と合わない点を主要な失敗に挙げる。 — [Bugnet](https://bugnet.io/blog/common-game-trailer-mistakes)（要約経由）

### Inferences
- 「画が変わるとトレーラーが古くなる」苦痛は実在するが、一次の頻度は今回の検索では1件のみ。

### Gaps
- パブリッシャーや Next Fest 由来の再編集強制、更新・シーズンごとの作り直し本数、「全部撮り直し」という直接の引用は見つからなかった。

## フィードバック文化（r/DestroyMyGame の定番批評）

### Takeaway
最頻の批評は、冒頭が遅い、ロゴ・暗転が先、ジャンルが不明、中盤が反復、ゲーム名がない。複数のソースで一致するのは「ゲームプレイを最初の数秒から」。

### Cited Findings
- 助言: 「Slow intros don't work」「Gameplay should start from the first seconds」（記事の文言）。 — [Game World Observer, 2022-09](https://gameworldobserver.com/2022/09/22/common-game-trailer-mistkaes-indie-developers-steam)
- Lieu が挙げる頻出問題は「unknown logos」、反復、悪い選曲。 — 同上
- 一次（Reddit、要約経由）: 「Theres also a slow fade-in black screen at the beginning... Why?」 — [r/DestroyMyGame](https://redlib.belloworld.it/r/DestroyMyGame/comments/1dgq4ym/destroy_my_trailer_so_i_can_make_it_better)（本文未確認）
- 一次（Reddit、要約経由）: 「it isn't totally clear what you're trying to get across in the trailer, and it goes on a bit too long. A lot of the content from ~0:20 to 1:00 is very similar.」 — r/DestroyMyGame 検索結果（個別スレッドは特定不可）
- 一次（Reddit、要約経由）: 冒頭の話者パートを切って「straight to the action」にせよ、という助言。 — 同上

### Inferences
- 批評の大半は構成（冒頭・反復・名前の出し方）に関するもので、後半工程の技術的な不満ではない。

### Gaps
- Reddit 本文を直接読めず、頻度の数え上げはできていない。上記の引用は検索要約経由なので、報告書で使う前に原文の照合が望ましい。
