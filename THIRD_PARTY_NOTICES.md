# Third-party notices

Shotpilot itself is MIT-licensed (`LICENSE`). It ports, bundles or adapts the following works.

## Sound analysis and synthesis code — MIT

`crates/shotpilot/src/soundcheck.rs` is a Rust port of the model-free half of an MIT-licensed
sound analysis library (Copyright (c) 2026 Leon Lin), including its reference profile of launch
films, weights, tolerances and verdict bands.

`crates/shotpilot/src/forge/` adapts the synthesis building blocks of the same work (damped modes,
a time-varying state-variable filter, PolyBLEP oscillators, the Shepard glissando, a noise-tail
room), its sub-drop, reverse-cymbal and struck-body models with their preset parameters, and the
layer designs of its impact, riser and whoosh recipes, rebuilt in Rust.

```
MIT License

Copyright (c) 2026 Leon Lin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## three.js 0.170.0 — MIT

Bundled unmodified as `crates/shotpilot/three/three.module.min.js` and embedded in the binary.
Copyright © 2010-2024 three.js authors. The full license is in
`crates/shotpilot/three/LICENSE-three.txt`.

## open-design — Apache License 2.0

Copyright nexu-io/open-design contributors (<https://github.com/nexu-io/open-design>).
No code is copied. The following files restate, with changes for teasers, parts of
open-design's OD Next planning turn, Design baseline, typography references, `design-systems/`
and `skills/` layout, the `motion-design` and `hyperframes` task profiles, and the hyperframes
transition references. Each names its source at the top (`catalog.json` in `sources`, `ai.rs`
in the doc comment of `discover_system`):

- `references/craft/TEASER-BASELINE.md`
- `references/craft/TEASER-TYPE.md`
- `references/fonts/FONTS.md` and the `overused` list in `references/fonts/catalog.json`
- `references/motion/MOTION.md`
- `references/motion/TRANSITIONS.md`
- `references/motion/transitions.json`
- the planning turn's system prompt in `crates/shotpilot/src/ai.rs` (`discover_system`)

The license text is at <https://www.apache.org/licenses/LICENSE-2.0>.

## Not included

- Fonts are fetched from Google Fonts on demand (OFL; Kosugi, Kosugi Maru and Rock Salt are
  Apache 2.0) and cached outside the repository. None are bundled.
- The UE sample games use Epic's UE 5.8 Third Person template assets. The assets are not in this
  repository; only the build scripts under `tools/ue_sample/` are.
- Sound effects and music from the author's Splice library are not in this repository. The
  placeholder sounds written by `shotpilot init` are synthesized in code.
- `references/teasers/` holds measurements and written analysis of public teasers only. No video
  or frame of those works is included.
