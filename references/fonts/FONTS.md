# Fonts — 書体の選び方と候補

> 出典: open-design（nexu-io/open-design、Apache-2.0）の書体の文書（定番すぎる書体の一覧、選び方の手順）を、日本語のティザー向けに言い換えた。候補は Google Fonts の書体（OFL）に限り、太さ・人気順位・収録文字は 2026-11-05 に Google Fonts のメタデータと実ファイルから測った（`catalog.json`）。組み方の規則は `references/craft/TEASER-TYPE.md`。

ルックが書体を決め、`TEASER-TYPE.md` が組み方を決める。このフォルダは、その間で「どの書体を選べるか」を持つ。`catalog.json` には日本語 68 書体（Google Fonts にある日本語の全ファミリー）、欧文の候補 75 書体、定番すぎる書体 34 を入れた。各書体は、分類（明朝・ゴシック・丸ゴシック・筆・手書き・ポップ・表示用・ドット・等幅）、合う雰囲気（`registers`）、向く役割（`roles`）、漢字の収録（`coverage`）、作者の説明（`about`）を持つ。

## 1. 選び方

雰囲気の名前を先に決める。「ホラーだから明朝」と分類から入らず、このゲームの文字が何の声で話すかを一語で言う（告げる、ささやく、記録する、はしゃぐ、命じる）。次に、その声を持つ実物を思い浮かべる（古い活版の新聞、手書きの日記、駅の案内板、少年誌の見出し、計器の表示、絵本の背表紙）。実物が決まると、書体の種類が絞れる。

最初に浮かんだ書体は、たいていそのジャンルの定番なので、一度外して別の候補と並べる。分類の思い込みも疑う。物語のゲームに明朝が要るとは限らず、技術的なゲームにゴシックが要るとも限らない。測ったティザーでも、ホラーのタイトルの 2 割は血の滴る書体ではなく細い縦長かローマン大文字だった。

1 本で使うのは、ロゴを含めて 3 書体まで。表現の書体 1 つ（タイトル・キャッチ）と情報の書体 1 つ（字幕・日付・数え上げ）を基本にし、欧文が要るなら情報の書体と線の性格が近いものを足す。

## 2. 定番すぎる書体

次の書体は、そのジャンルらしく見えても「このゲームらしく」は見えない。最初のグループは、言語モデルが同じ書体に収束して出力が似通う（AI っぽく見える）ので使わない。全 34 書体と置き換え先は `catalog.json` の `overused`。

| 種類 | 書体 | 置き換え先の例 |
|---|---|---|
| 言語モデルの既定 | Inter, Roboto, Open Sans, Noto Sans, Lato, Poppins, Nunito, Playfair Display, Cormorant Garamond, EB Garamond, Bodoni Moda, Cinzel, Prata, Syne | Hanken Grotesk, Geist, Gloock, Newsreader, Libre Bodoni, Marcellus |
| ファンタジーの定番 | Cinzel, Cinzel Decorative（Trajan 風の金の大文字。Trajan は「映画ポスターの Arial」と呼ばれる。実測の RPG でも 10 本中 4 本） | Marcellus, Forum, Texturina, Pirata One |
| ホラーの定番 | Creepster, Nosifer, Eater, Butcherman（血の滴る書体。実測で 10 本中 3 本）、Special Elite, Courier Prime（かすれたタイプライター） | Gloock, Fraunces, Instrument Serif, IM Fell English, Cutive Mono |
| SF の定番 | Orbitron, Audiowide, Michroma, Rajdhani | Chakra Petch, Oxanium, Tomorrow, Saira |
| アクションの定番 | Bebas Neue, Oswald, Anton | Big Shoulders, League Gothic, Sofia Sans Extra Condensed, Barlow Condensed |
| レトロの定番 | Press Start 2P, VT323 | Pixelify Sans, Silkscreen, Jersey 10, Tiny5 |
| コージーの定番 | Fredoka, Baloo 2, Comfortaa, Quicksand | Gabarito, Young Serif, Bagel Fat One, Grandstander |

日本語では、Noto Sans JP と Noto Serif JP がどの環境にもある既定の書体にあたる。足りない字の受け皿には使うが、ルックの顔（タイトル・キャッチ）には使わない。

置き換え先にも流行りが混じる（Instrument Serif、Fraunces、Space Grotesk、Bricolage Grotesque は今よく使われている）。同じ置き換え先が続くと次の定番になるので、雰囲気ごとの候補（3 節）から毎回選び直す。

## 3. 雰囲気ごとの候補

既定の Noto は除いた。日本語の「かなだけ」の書体は漢字を 1 字も持たないので、漢字の無い一語（「ようこそ」「デモ」など）にだけ使う。

| 雰囲気 | 日本語（漢字あり） | 日本語（かなだけ。漢字の無い一語に） | 欧文 |
|---|---|---|---|
| 不穏・ホラー（`dread`） | Shippori Mincho, Zen Old Mincho, Shippori Mincho B1, Yuji Mai, Zen Kurenaido, Zen Antique, Shippori Antique, Kaisei Tokumin, Shippori Antique B1 | Shizuru | Instrument Serif, Fraunces, Newsreader, Cutive Mono, Gloock, IM Fell English, Libre Caslon Display, Grenze Gotisch, Texturina |
| 壮大・ファンタジー（`epic`） | Shippori Mincho, Zen Old Mincho, Yuji Mai, Yuji Syuku, Kaisei Tokumin | Aoboshi One | IM Fell English, Marcellus, Forum, Eczar, Pirata One, Grenze Gotisch, Almendra, Texturina |
| 和風（`wa`） | Shippori Mincho, Zen Old Mincho, Shippori Mincho B1, Potta One, Yuji Mai, Yuji Syuku, Zen Antique, Zen Antique Soft, Kaisei Tokumin, Yuji Boku | Darumadrop One、変体仮名の Yuji Hentaigana Akari / Akebono（飾りに限る） | （和風に合わせるなら epic か noir の欧文） |
| SF・技術（`tech`） | Zen Kaku Gothic New, M PLUS 1, Murecho, IBM Plex Sans JP, DotGothic16, M PLUS 1 Code | — | Chakra Petch, Oxanium, Tomorrow, Saira, Unbounded, Geist, Geist Mono, Martian Mono, IBM Plex Mono, IBM Plex Sans Condensed, Quantico, Share Tech Mono, Doto, Handjet |
| やさしい・コージー（`gentle`） | M PLUS Rounded 1c, Zen Maru Gothic, Kosugi Maru, Kiwi Maru, Kaisei Decol, Kaisei Opti, Kaisei HarunoUmi, Hina Mincho, New Tegomin, Klee One, Zen Kurenaido, Yomogi, Yusei Magic, Hachi Maru Pop, Mochiy Pop One, Stick, Zen Antique Soft, Yuji Boku | Cherry Bomb One, Kapakana, Tsukimi Rounded, Slackside One | Young Serif, Fraunces, Literata, Gabarito, Grandstander, Gluten, Bagel Fat One, Shantell Sans |
| ポップ・明るい（`pop`） | Zen Maru Gothic, M PLUS Rounded 1c, LINE Seed JP, Dela Gothic One, Kaisei Decol, Kiwi Maru, Hachi Maru Pop, Mochiy Pop One, Yusei Magic, RocknRoll One, Reggae One, Rampart One, Train One, Stick | Monomaniac One, Cherry Bomb One, Darumadrop One, Shizuru, Rock 3D, Palette Mosaic, Chokokutai | Gabarito, Grandstander, Gluten, Bagel Fat One |
| 勢い・アクション（`kinetic`） | Dela Gothic One, M PLUS 1, Reggae One, RocknRoll One, Rampart One, Train One, Potta One, Shippori Antique | Monomaniac One, Rock 3D | Big Shoulders, League Gothic, Sofia Sans Extra Condensed, Barlow Condensed, Alumni Sans, Smooch Sans, Archivo, Anybody, Saira, Unbounded, Big Shoulders Stencil |
| ミステリー・ノワール（`noir`） | Zen Kaku Gothic Antique, BIZ UDPMincho, BIZ UDMincho（明朝の表現の書体は dread の欄から） | Aoboshi One, Kapakana | Libre Caslon Text, Libre Caslon Display, Libre Bodoni, Gloock, Spectral, Rufina, Tenor Sans, Cutive Mono, IBM Plex Mono |
| レトロ・ドット（`retro`） | DotGothic16, Kosugi, Kosugi Maru, Hachi Maru Pop, Hina Mincho, New Tegomin, Train One, Zen Antique, Zen Antique Soft, Shippori Antique, Shippori Antique B1 | Palette Mosaic | Pixelify Sans, Silkscreen, Jersey 10, Tiny5, Handjet, Doto, Sixtyfour |
| 戦略・司令室（`command`） | Zen Kaku Gothic New, M PLUS 1, M PLUS 2, M PLUS U, Murecho, IBM Plex Sans JP, BIZ UDGothic, BIZ UDPGothic, Kosugi, Dela Gothic One, M PLUS 1 Code | — | Barlow Condensed, IBM Plex Sans Condensed, Chivo, Chivo Mono, Familjen Grotesk, Quantico, Share Tech Mono, Zilla Slab, Archivo, Big Shoulders, Big Shoulders Stencil, Geist Mono, Martian Mono |
| 静か・アート（`minimal`） | Zen Kaku Gothic New, Zen Kaku Gothic Antique, M PLUS 2, M PLUS U, LINE Seed JP, Murecho, BIZ UDGothic, BIZ UDPGothic, Kaisei Opti, Sawarabi Mincho, Sawarabi Gothic | Aoboshi One | Instrument Serif, Newsreader, Hanken Grotesk, Schibsted Grotesk, Familjen Grotesk, Geist, Jost, Tenor Sans |
| 語り・日記（`narrative`） | Shippori Mincho, Shippori Mincho B1, Zen Old Mincho, Zen Kaku Gothic Antique, Kaisei HarunoUmi, Hina Mincho, New Tegomin, Klee One, Zen Kurenaido, Yomogi, Kiwi Maru, BIZ UDPMincho, BIZ UDMincho, Sawarabi Mincho | Slackside One | Newsreader, Literata, Spectral, Shantell Sans |
| 崩れ・かすれ（`worn`） | Yuji Mai, Potta One | Shizuru, Darumadrop One | IM Fell English, Fredericka the Great, Protest Revolution, Rock Salt, Rubik Glitch, Rubik Burned, Rubik Distressed, Rubik Dirt, Rubik Scribble, Cabin Sketch, Road Rage, Frijole, Caesar Dressing, Metal Mania, Mystery Quest |

崩れ・かすれ（`worn`）は 2026-11-06 に足した。Google Fonts 公式のタグ（google/fonts の `tags/all/families.csv`）の `/Theme/Distressed` と `/Seasonal/Halloween` から、定番すぎる書体（Creepster などの血の滴る書体、Special Elite）を除いて選んだ。どれもタイトル（と一部はカード）向けで、ルックの雰囲気に関わらずタイトルの候補になる（5 節）。Rubik の崩れ版は土台が丸いゴシックなので、静かなホラーには軽く見える。

## 4. ルックごとの組み合わせの例

ルック（`references/design-systems/<id>/`）ごとに、表現の書体・情報の書体・欧文を 1 組ずつ挙げる。これは例で、3 節の同じ雰囲気の欄から入れ替えてよい。

| ルック | 表現（タイトル・キャッチ） | 情報（字幕・日付・数え上げ） | 欧文 |
|---|---|---|---|
| quiet-dread | Shippori Mincho（Regular〜Medium、字間を空ける） | Zen Kaku Gothic Antique | Instrument Serif / Newsreader |
| vhs-found-footage | DotGothic16 | Kosugi（等幅） | Tiny5 / Cutive Mono |
| cozy-pastel | Zen Maru Gothic（Bold〜Black） | M PLUS Rounded 1c | Gabarito / Young Serif |
| storybook | Kaisei HarunoUmi か Zen Old Mincho | Zen Maru Gothic | Young Serif / Literata |
| grimdark | Kaisei Tokumin か Shippori Mincho B1 | Zen Kaku Gothic Antique | Grenze Gotisch / Texturina |
| epic-fantasy | Shippori Mincho（ExtraBold）、和風なら Yuji Syuku | Zen Kaku Gothic Antique | Marcellus / Forum |
| neon-action | Dela Gothic One か Reggae One | M PLUS 1（Medium〜Black） | Big Shoulders / Sofia Sans Extra Condensed |
| retro-pixel | DotGothic16 | DotGothic16 | Pixelify Sans / Jersey 10 |
| arthouse-minimal | Shippori Mincho（Regular） | Zen Kaku Gothic New（Light〜Regular） | Instrument Serif / Hanken Grotesk |
| noir-mystery | Zen Old Mincho | BIZ UDPMincho（帯の上の字幕） | Libre Caslon Text / IBM Plex Mono |
| cold-scifi | M PLUS 1（Thin と Black の対比） | IBM Plex Sans JP | Chakra Petch / Geist Mono |
| command-room | Zen Kaku Gothic New（Bold） | BIZ UDGothic（等幅で桁が揃う） | Barlow Condensed / Share Tech Mono |

## 5. 収録文字

`catalog.json` の `coverage` は、各書体の実ファイルが JIS 第1水準（2,965 字）と第2水準（3,390 字）の漢字をどれだけ持つかを数えた値。68 書体のうち、第2水準まで全部あるのは 45、第1水準まで（第2水準は一部）が 9、第1水準も一部だけが 1、漢字を 1 字も持たないのが 13 だった。

漢字の無い 13 書体は、Monomaniac One, Cherry Bomb One, Aoboshi One, Darumadrop One, Kapakana, Shizuru, Tsukimi Rounded, Slackside One, Rock 3D, Palette Mosaic, Chokokutai, Yuji Hentaigana Akari, Yuji Hentaigana Akebono。変体仮名の 2 書体はカタカナも無い。Murecho は常用漢字までで、第1水準の 73% しか無い。Train One は第1水準だけ、Sawarabi Mincho は第2水準を 1 割しか持たない。M PLUS の各書体は第2水準の 6〜7 割。

収録の無い字は、別の書体で描かれるか、□（豆腐）になる。カードや字幕に使う書体は、漢字が `full` か `level1` のものから選び、それ以外の書体はタイトルの一語に限る。

## 6. 大きさと太さの下限

| 分類 | 下限 |
|---|---|
| 筆・手書き・ポップ・表示用（袋文字、立体） | 画面高さ 5% 以上。小さい行には同じ雰囲気の明朝かゴシックを使う |
| 明朝を画の上に置く | 画面高さ 5% 以上、太さは Medium 以上。細い横画は動画の圧縮で消える |
| ドット書体 | 元の格子の整数倍の大きさ（DotGothic16 なら 16 の倍数の px）。半端な倍率では点がにじむ |
| UD 書体（BIZ UD、M PLUS U） | 字幕・数え上げ・日付の小さな行の第一候補 |
| 欧文の大文字だけの行 | +0.1 em 以上の字間（`TEASER-TYPE.md` 4 節） |

## 7. ライセンス

Google Fonts の日本語 68 書体は、Kosugi と Kosugi Maru（Apache 2.0）を除いて SIL Open Font License 1.1。OFL の書体は、動画のタイトルや字幕に使ってよく、作った映像の権利は作った人に残る（OFL FAQ 1.1）。禁止されているのは、書体だけを売ることと、改変した書体に元の予約名を使うこと。書体を shotpilot に同梱して配るときは、OFL.txt を一緒に置く。

Windows に入っている書体（游ゴシック、游明朝、メイリオ、BIZ UD）は、文字を画像として描いて動画に焼き込み、書体のファイルを動画に入れない限り、動画の字幕や文字に使ってよい（Microsoft のフォント再配布 FAQ）。ただし書体ファイルを配布物やゲームに含めることはできない。shotpilot は利用者の PC にある書体を読むだけにする。

モリサワ、フォントワークス（LETS）などの有償の書体は、利用者自身の契約の範囲で使う。動画での利用に追加料金はないとする記事はあるが、一次の規約は確認できていない。Google Fonts 以外の無料書体（源ノ角ゴシック・源ノ明朝、LINE Seed JP、源暎こぶり明朝、装甲明朝、x12y16pxMaruMonica、PixelMplus、美咲フォント、ラノベPOP）は、調べた範囲ではいずれも商用で使えるが、各書体の最新の規約を配布元で読んでから使う（`research_notes/文字デザインと書体/web_japanese_typography.md`）。

## 8. 目録の更新

`catalog.json` は `research_data/tools/build_fonts.py`（git 管理外）が書く。測った値（太さ・人気順位・追加日・収録文字・作者の説明）は `research_data/fonts/` の元データから、分類・雰囲気・役割・注記はスクリプトの中の表から取る。分類や雰囲気を直すときはスクリプトの表を直して作り直す。

## 5. タイトルだけの書体（`finish.fonts.title`）

ルックは `display`（タイトル・カード）・`body`（情報）・`latin`（英語の添え書き）を決める。タイトルだけを別の書体にしたいときは `title` を書く（D-59）。ロゴ画像（`finish.logo`）があればそれが優先し、無いときに書体で組む。

- 選ぶのは Claude。plan と全体への仕上げの指示で、目録のうちタイトル向き（役割 `title`）で定番すぎない書体の一覧から、企画・最初のプロンプト・ロケハンで見えた場所に合うものを `title_face` に書く。一覧はルックの雰囲気の書体と、崩れ・かすれ（`worn`）の書体を並べる。合うものが無ければ空にして、ルックの `display` のまま。
- コードが確かめる。目録にあること、定番すぎないこと、タイトルの字を持つこと（欧文の書体は欧文のタイトルだけ、かなだけの書体は漢字の無いタイトルだけ）。太さは 700 に近いもの。
- 焼き込みでは、タイトルの書体に無い字を含むタイトルは、表現用の書体（`display`）で組み、注記に出す（システムの書体には落とさない）。
- 編集的で、撮り直しは起きない。

