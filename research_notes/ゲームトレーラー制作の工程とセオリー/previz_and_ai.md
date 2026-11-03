# UE における previz / バーチャルプロダクションの実務と、トレーラー・previz ワークフローでの AI 支援の現状

調査日: 2026-11-03。検索は約15回に収めた。一次資料を開けたのは Epic の command line rendering ページだけで、残りは検索スニペットに基づく。確度が低いものはその旨を書いた。

## Q1. previz パイプライン（段階・担当・成果物・レビュー・最終への残存・ゲームトレーラーへの転用）

### Takeaway
previz は「pitchviz → previz → techviz → (撮影) → postviz」に分かれ、前段ほど承認と資金獲得に使われ、後段ほど撮影・合成の実務に使われる。ゲームトレーラーは映画の previz と違い、素材がエンジン内にあるため previz のカメラ・ショット定義がそのまま最終キャプチャになりうる。この点が shotpilot の立ち位置と重なる。

### Cited Findings
- Previsualization Society は 2009-09-30 に発表された非営利団体で、最初のチャーター・スポンサーは Autodesk。定義集やハンドブックを公開する予定だったが、本人たちの定義ページは取得できていない — [Post Magazine](https://www.postmagazine.com/Press-Center/Daily-News/2009/NEW-ORGANIZATION-DEDICATED-TO-PREVIS-LAUNCHES.aspx)
- pitchviz は制作の外側で、資金調達（広告のピッチ、長編のトレーラー）に previs を使う段階。postviz は撮影後に previs を実写に合成して最終シーンを確認する段階（二次資料の仏語レポート） — [3DVF ParisFX 2010](https://3dvf.com/redaction/dossier-744-compte-rendu-de-parisfx-2010-html/2/)
- techviz は、カメラ位置と動き、レンズ選択、セット位置、演者のブロッキングを静止画と動画で分解した資料。Third Floor の事例では previs のカメラデータを Spydercam の装置設定に渡している。Last Jedi ではコクピットのショットの検討に使われた — [Yann's blog (UAL)](https://yannsblog.myblog.arts.ac.uk/?p=284)、[Studio Daily 2012](https://studiodaily.com/2012/05/previs-techvis-and-postvis-on-the-avengers)、[CGW 2018](https://www.cgw.com/Press-Center/In-Focus/2018/Previs-and-Postvis-for-The-Last-Jedi.aspx)
- Third Floor は Age of Ultron で美術・VFX・スタント・セカンドユニット・特殊効果・編集の各部門と連携した。Avengers では previs で作った大量のモーキャプが postvis に再利用された — [AWN/VFXWorld](https://d1xfgk3mh635yx.cloudfront.net/vfxworld/third-floor-helps-visualize-avengers-age-ultron)、[Studio Daily](https://studiodaily.com/2012/05/previs-techvis-and-postvis-on-the-avengers)
- Endgame では Third Floor が previs 7,300 ショットを作り、うち 7,000 超が最終的に postvis になった。postvis の過程で新規ショットの previs も作られた — [VFX Voice](https://vfxvoice.com/?p=5167)（検索要約経由。数字は要原文確認）
- 構図は 3D で仮組みし、監督が承認するまで磨く。承認後に実写とVFXの配分を決める — [VFXWorld "prevalence previs"](https://d1xfgk3mh635yx.cloudfront.net/vfxworld/prevalence-previs)
- 「previs が最終にどれだけ残るか」の定量値は見つからなかった。元 previs アーティストの「アクションシーンでは最終が previs に驚くほど近い」という個人談や、承認済みのシーンが編集で落ちた例があるだけ — 同上、[Xsens press release](https://www.xsens.com/resources/press-releases/previs-returns-creativity-core-filmmaking)
- ゲームトレーラー側: 外注スタジオの工程は「ストーリーボード/アニマティック → レイアウトとカメラブロッキング → アニメーション → VFX → ライティング/レンダ → 編集 → 納品」。リアルタイム部分は UE や Unity で行う（ベンダーの自社紹介） — [Thunder Cloud Studio](https://thundercloud-studio.com/3d-cgi-animation-trailer-service/)
- Epic の Fortnite トレーラー白書では、アニマティックと最終編集に Premiere、モーキャプに MotionBuilder と Vicon、進行管理に Shotgun を使っている — [fxguide](https://www.fxguide.com/?p=198297)
- Team17 のキャプチャ・アーティスト求人は、Sequencer でシーンとシーケンスを組んで撮り、編集者に渡す工程を職務に挙げている。別のトレーラー制作会社の求人では free-cam、HUD オフ、時間制御が標準のキャプチャ手段として挙がる — [Team17 求人](https://apply.workable.com/team-17-digital/jobs/view/EB00A03F73.md)、[Gameplay Video Artist 求人](https://www.marketingmonk.so/jobboard/jobs/gameplay-video-artist-at-playsense-6535rm)
- Platige は映画から rip-o-matic を作り、カメラ機材・レンズ・編集スタイルを制作前に合意する（紹介記事） — [Cameras Hum, Engines Compile](https://www.startmotionmedia.com/what-tools-do-gaming-industry-professionals-use)
- 最終キャプチャは OBS ではなくキャプチャカードかエンジン書き出しを使う、という実務助言がある（マーケ会社のガイドで、権威は弱い） — 同上の検索結果群 [Morphic](https://morphic.com/resources/guides/gaming/indie-game-studios-ai-trailers)

### Inferences
- ゲームではカメラ・アクター・ライティングが最終と同じアセットなので、previz と final の差は主に品質プリセットになる。映画のように previz を捨てて作り直す工程が要らない。shotpilot の `previz → still → preview → final` はこの性質を前提にした設計と読める。
- 映画の techviz に当たるものは、shotpilot では `shots.yaml` のカメラ・レンズ定義そのもの。成果物がそのまま実行可能な点は映画より強い。
- 映画の previz はステークホルダーの承認用であり、トレーラーの pitchviz は publisher や社内への提示に当たる。

### Gaps
- Previsualization Society の公式用語集と stuntviz の定義は取れなかった。
- Halon と Proof の事例、previz の成果物の標準リスト、previz が最終に残る割合の実測値は見つからなかった。
- 実際のゲームトレーラー制作会社（Blur、Platige など）の一次資料によるショット単位の工程記述は薄い。

## Q2. Unreal Engine 固有の事情（Sequencer、MRQ/MRG、コマンドライン、VCam）

### Takeaway
Epic の標準は「Master Sequence にショット・トラックを置き、ショットごとに別の Sequence アセットを持つ」構造で、take は複製したシーケンスで表す。MRQ はコマンドラインで動くが、特定ショットだけを選ぶ公式引数は無く、Python エグゼキュータで自作する。分散の最小単位はショットとされている。

### Cited Findings
- Master Sequence は Shot Track とショットを含む Level Sequence で、通常の Level Sequence と本質的な違いは無い。ショットは各自が独立した Sequence アセットで、タイムライン上ではセクションとして並ぶ — [Epic 4.27 Shot Track](https://docs.unrealengine.com/4.27/AnimatingObjects/Sequencer/Overview/TracksShot)、[Epic: sequences, shots and takes](https://dev.epicgames.com/documentation/ja-jp/unreal-engine/sequences-shots-and-takes-in-unreal-engine)
- take は Level Sequence の複製で、いじっても元は変わらない。コミュニティの記事では、ショットをリネームすると take が全て失われるとされる（記事は 2023 年、挙動は版による） — [Versluis 2023](https://www.versluis.com/2023/09/working-with-takes-and-shots-in-unreal-engine/)
- Hierarchical Bias の既定値は、最上位の Master が 0、サブシーケンスが 100 で、同じアクターに対してはショット側のソースが優先される — [Epic 4.27 Subscenes](https://docs.unrealengine.com/4.27/AnimatingObjects/Sequencer/Overview/Tracks/TracksSub)（旧版ドキュメント。5.x の挙動は要確認）
- MRQ のコマンドラインには3方式がある。単一の Level Sequence + 設定、キュー全体、Python エグゼキュータ。`-LevelSequence` は `-MoviePipelineConfig`（Primary Config プリセット）とセットで使い、その sequence に含まれる全ショットを描く。単一ジョブ・単一マップのみ対応 — [Epic: command line rendering](https://dev.epicgames.com/documentation/en-us/unreal-engine/using-command-line-rendering-with-move-render-queue-in-unreal-engine)
- 同ページに特定ショットを選ぶ引数の記載は無い（実際に開いて確認）。Python エグゼキュータ（`MoviePipelinePythonHostExecutor` 継承、`init_unreal.py`、サンプルは `MoviePipelineExampleRuntimeExecutor.py`）が引数を読み、ジョブを構成できると書かれている — 同上
- Epic は、分散可能な最小単位は1つのカメラカット（Level Sequence Shot）で、リアルタイム機能は前フレームの情報に依存するため複数マシンで共有できないと述べる — 同上
- Movie Render Graph（MRQ の後継）では、Temporal Samples は Sampling Method ノードで Global に、Spatial Samples は Renderer ノードで描画レイヤごとに設定する。Warm Up は専用ノードに分離され、ウォームアップ出力は破棄できる — [Epic: transitioning to MRG](https://dev.epicgames.com/documentation/unreal-engine/transitioning-to-the-movie-render-graph-from-movie-render-queue-in-unreal-engine?lang=en-US)、[FMovieGraphTimeStepData](https://dev.epicgames.com/documentation/en-us/unreal-engine/API/Plugins/MovieRenderPipelineCore/FMovieGraphTimeStepData)
- VCam: VirtualCamera と Live Link プラグインを有効にして VCam アクターを置く。iPhone の ARKit が位置・回転を Live Link で送り、Take Recorder で記録して Sequencer で整える。複数カメラがあるときは接続先を選ぶ。VCam は Pixel Streaming 2 に未対応 — [Epic: VCam with Live Link](https://dev.epicgames.com/documentation/unreal-engine/controlling-a-virtual-camera-actor-using-live-link-in-unreal-engine)、[Unreal VCam (App Store)](https://apps.apple.com/us/app/-/id1547309663)

### Inferences
- shotpilot が「ショットを1つずつ MRQ に渡す」実装を持つなら、公式機能ではなく Python エグゼキュータ相当の自作に当たる。ショットごとに1ジョブを組むか、1 sequence に1ショットだけ入れる方式のどちらかになる。
- 「ショットが分散の最小単位」という Epic の記述は、ショット単位の fingerprint が最適な粒度であることを裏付ける。
- take を複製シーケンスで持つ Epic 流に対し、shotpilot は fingerprint 付きのレンダ出力で take を持つ。アセット管理が重い Epic 流のリネーム問題を避けている。

### Gaps
- draft と final の MRQ/MRG プリセット推奨値（AA サンプル数、ウォームアップ枚数）、典型的なレンダ時間とコストは、一次資料で見つからなかった。
- 「Epic 推奨のシネマティック・ワークフロー」の一括解説（Unreal Fest や GDC の講演）は取得できていない。
- Take Recorder の仕様の詳細、MRQ の Python からのショット単位のオーバーライド実装例は未確認。

## Q3. レビューと修正の実務（ショット単位のレビュー、バージョン、変更ショットだけの再レンダ）

### Takeaway
VFX/アニメ制作ではショット単位のバージョンとフレーム単位のノートが標準だが、「変更したショットだけ再レンダ」はファームや DCC 側の依存追跡の話で、レビュー製品の機能ではない。今回の検索では依存追跡の一次資料は取れなかった。

### Cited Findings
- ShotGrid は 2024年3月に Flow Production Tracking へ改称。SyncSketch は連携で ShotGrid のプレイリストをレビューとして取り込み、ノートを Push で返せる。返すとき、コメントスレッドはフレーム単位のノートになり、スケッチはフレーム精度の注釈として付く。ノートはプレイリスト・ショット・バージョンにリンクされる — [SyncSketch: Syncing Playlists and Notes with ShotGrid](https://support.syncsketch.com/hc/en-us/articles/32394066503316-Syncing-Playlists-and-Notes-with-ShotGrid)
- 取り込み順はプレイリストの順序で、無ければバージョン作成日 — 同上
- ftrack は Backlight 社のクラウド制作管理で、cineSync が同期レビューを担う（競合比較ページの記述で、機能資料ではない） — [Morphic ftrack alternatives](https://morphic.com/resources/alternatives/ftrack-alternatives)
- Third Floor の previs 編集者の仕事術は専用記事がある（未精読） — [Pro Video Coalition](https://www.provideocoalition.com/art-of-the-cut-with-the-previs-editors-of-the-third-floor/)

### Inferences
- 業界のレビューは「バージョン（メディア）+ ショット + フレーム + 注釈」の4点組でノートを持つ。shotpilot の「選択フレーム/領域に自然言語ノート」は同じデータ形で、ノートの宛先を人間でなく Claude にした形に当たる。
- 業界のノートは人間が読んで手で直すので、ノートからの自動修正は無い。ここが新規性になりうる。
- 内容ハッシュによる増分再レンダ（fingerprint）はビルドシステム（Make/Bazel 型）の発想で、映像制作のレビュー製品には見つからなかった。ただし「見つからなかった」であり、存在しない証明ではない。

### Gaps
- ftrack、frame.io の一次資料、Deadline など farm の依存追跡と再レンダ判定、Houdini PDG などのキャッシュ戦略は調べられていない。
- 「変更ショットだけ再レンダ」を VFX 現場がどう運用しているかの証言は無い。

## Q4. AI の現状（2023-2026）: カメラ配置、previz/絵コンテ、トレーラー編集、UE 向けエージェント、評価法

### Takeaway
LLM によるカメラ計画は2024年以降に論文が急増し、2026 年の論文が多い（大半はまだ研究デモ）。UE 向け MCP サーバーは OSS が複数あるが、Sequencer の深い制御を実証した資料は薄い。AI 編集は beat-sync と第一稿の自動組みが実用域で、演出判断は未実証。

### Cited Findings
研究（出荷物ではない）
- ChatCam（2024-09、NeurIPS 2024）: LLM エージェントが自然言語から放射場レンダ用のカメラ軌道を作る。テキスト条件付き軌道生成器 CineGPT とアンカー配置ツールを使う — [arXiv 2409.17331](https://arxiv.org/html/2409.17331v1)
- FilmAgent（2025-01）: 3D 空間での仮想映画制作を、計画・脚本・撮影の複数エージェントで行う。脚本には Critique-Correct-Verify、撮影には Debate-Judge を適用 — [FilmAgent PDF](https://filmagent.github.io/static/FilmAgent_arxiv_version_0830_v1.pdf)、[arXiv 2501.12909](https://arxiv.org/pdf/2501.12909v1)
- 2026年の論文（ID は検索結果の表記どおり。未精読）: Auteur は微調整した MLLM が演者相対のキーフレーム DSL を書き、決定論的に補間する — [arXiv 2606.01900](https://arxiv.org/pdf/2606.01900)。CinemaTraj は3Dシーングラフを持つ LLM エージェントが原子的カメラ軌道を合成する — [arXiv 2607.26910](https://arxiv.org/pdf/2607.26910)。CamPilot はプロ映画 14K 本から GRPO で撮影計画（アングル、ショットサイズ、動き）を学習する — [arXiv 2609.10943](https://arxiv.org/pdf/2609.10943)。FilmWorld は小説から映画への粗→細のショット計画 — [arXiv 2607.19038](https://arxiv.org/pdf/2607.19038)
- 共通の設計: LLM は構造化 DSL/パラメータだけを出し、補間や実行は決定論的コードに任せる（Auteur）。shotpilot の「Claude が編集した YAML をコードで検証して書く」と同じ向き。

製品・OSS
- UE 向け MCP: ChiR24/Unreal_mcp は Remote Control API 経由で、機能表に Sequencer、Movie Render Queue、Take Recorder を含む（対応版は README と Glama で 5.0–5.7 / 5.0–5.8 と食い違い） — [Glama: ChiR24](https://glama.ai/mcp/servers/@ChiR24/Unreal_mcp/tree/7f8ff476b87432da03a5565bbd864b2eacc77456)。flopperam/unreal-engine-mcp は UE 5.5+、Python MCP と C++ プラグインを TCP で接続し約793 stars — [SourcePulse](https://www.sourcepulse.org/projects/22206900)。Fab の商用リスト（UE 5.7）は304ツール/43カテゴリを謳う — [Fab](https://www.fab.com/listings/aa699a85-04b1-4746-a29c-962fc3a78f55?lang=en)
- Autodesk Flow Studio（旧 Wonder Studio、2025-03 に改称）はクラウドの AI 3D ツールで、映像から編集可能な CG シーンを作り、モーキャプ・カメラトラック・クリーンプレートを Maya/Blender/Unreal/USD に渡せる。価格は Free/Lite $10/Standard $45（2025-08 時点の報道） — [Digital Production 2025-03](https://digitalproduction.com/2025/03/18/wonder-studio-rebrands-now-flowing-with-autodesk/)、[Digital Production 2025-08](https://digitalproduction.com/2025/08/13/flow-studio-prices-drop-tiers-multiply-features-shift)、[Autodesk](https://www.autodesk.com/products/flow-studio)
- AI 編集: beat-sync は ML でテンポと構造を検出する機能として最も成熟している。CapCut の自動組みは「使える第一稿」という評価。映像を先に編集し後から BGM を載せる製品が多く、リズムがずれやすい（ベンダー記事、独立検証は乏しい） — [Opus Pro](https://www.opus.pro/blog/best-ai-beat-sync)、[Python Libraries](https://pythonlibraries.substack.com/p/ai-powered-video-editor-with-automatic)
- ゲームトレーラー向け生成 AI は「雰囲気・物語ショットは生成、実ゲームプレイは エンジン内キャプチャ」という棲み分けが語られる（ベンダー記述） — [Morphic](https://morphic.com/resources/guides/gaming/indie-game-studios-ai-trailers)

評価法
- Stable Cinemetrics（SCINE）: カメラのレンズ・被写界深度・露出・アングル・動き・意図を分類体系にし、各ノードに質問を生成する。10 以上の T2V モデル、2万本の人手評価で、アングルと軌道が最弱。微調整 VLM が 72.36% のペア選好精度で、ゼロショットを約20点上回る（AI 要約ページ経由、一次論文未確認） — [Moonlight review](https://www.themoonlight.io/review/stable-cinemetrics-structured-taxonomy-and-evaluation-for-professional-video-generation)
- EvalVerse は映画制作の工程に沿った評価で、専門家判断を VLM に蒸留する。数値は自己申告で未検証 — [benchmarklist.com](https://benchmarklist.com/models/hailuo-2.3-cinematography-composition/)

### Inferences
- 研究の主流は「生成映像」または「放射場上のカメラ」で、UE の Sequencer アセットを生成して差分で再レンダする系は見つからなかった。ただし網羅検索ではない。
- UE の MCP は「エディタ操作の遠隔化」で、状態管理・再レンダ判定・三方向マージを持つものは今回の資料に無い。shotpilot の新規性は AI の生成能力ではなく、周囲のループ構造にある。
- 評価は VLM 審判か人間のペア選好が主流。shotpilot が構図の自動採点を足すなら、これに倣えるが、現状 `shots.yaml` の構造検証だけで済ませているのは妥当。

### Gaps
- Cuebric、Runway/Sora 系の previz 利用、AI 絵コンテ製品（Storyboarder 系など）は今回取れなかった。
- CineMPC など古典的な自動撮影研究、トレーラー専用の AI 編集製品の実機評価、実務者の限界に関する独立した証言は不足。
- 2026 年論文は検索要約のみで、手法の詳細と再現性は未確認。

## Q5. 人間を残すパターン（監督が譲らない判断と、自動化が受け入れられる範囲）

### Takeaway
この点の実務者証言は今回ほぼ取れなかった。確認できたのは「構図は監督が承認するまで磨く」という一点で、それ以外は推論になる。

### Cited Findings
- 3D で仮組みした構図は、監督が承認するまで全員が見て磨く。承認後に実写とVFXの配分を決める — [VFXWorld "prevalence previs"](https://d1xfgk3mh635yx.cloudfront.net/vfxworld/prevalence-previs)
- 自動組みの編集は「第一稿」止まりという評価が複数ある — [Python Libraries](https://pythonlibraries.substack.com/p/ai-powered-video-editor-with-automatic)
- 検索要約では、自動カット提案やクリップ選択は受け入れられ、演出判断は未実証とされる（独立した監督の証言は無し）— [Dyerbolical](https://dyerbolical.com/mastering-trailer-editing-harnessing-ai-for-precision-cut-suggestions/)（ブログで権威は弱い）

### Inferences
- 構図の承認と picture lock を人間に残し、カバレッジ（複数案）、バリアント、粗編集を機械に任せる分担が、上の弱い証拠とも整合する。shotpilot の「AI が組み、人間が1カット直す」と一致する。
- 人間の修正を `pull` で正本に戻す三方比較は、「機械案を人間が上書きしても機械が黙って戻さない」という信頼の条件を満たす。映画の previz にはこの双方向の同期が無い（手作業で再現する）と推測するが、一次資料での確認は無い。

### Gaps
- 監督・DP・編集者による、AI に任せたくない判断についての一次証言（インタビュー、Unreal Fest/GDC 講演）は取得できていない。
- 余力で扱う「1ビート3バリアント」に近い、業界のバリアント承認フローの資料は見つからなかった。

## 対応表（shotpilot と業界実務）

- 対応が取れている: previz → final の品質段階（業界の previz/postviz と、ドラフト/最終の MRQ プリセット）、ショット単位が最小の分散単位であること（Epic の記述）、ショット・フレーム・ノートの組（SyncSketch 型）、LLM は構造化出力のみを出し決定論的コードが実行する設計（Auteur 型）、監督承認の構図ゲート。
- 業界に見つからなかった（新規性の候補）: fingerprint による変更ショットだけの再レンダ、Sequencer での人手編集を正本へ戻す三方比較、フレーム領域のノートから Claude が正本を修正し検証する閉ループ。いずれも「今回の検索では見つからなかった」という意味で、不在の証明ではない。
- shotpilot に無いもの: 複数人のレビュー・承認ステータス管理（ShotGrid 系）、ショット単位の注釈の履歴、ライブ操作（VCam/Live Link）、techviz の機材出力、postviz（撮影素材との合成）、分散レンダのファーム連携、構図の自動評価（VLM 審判）。
