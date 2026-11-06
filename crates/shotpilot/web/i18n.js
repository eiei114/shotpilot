/* Shotpilot's English mode. The pages are written in Japanese; on a PC whose language is not
 * Japanese, this replaces the Japanese text on screen with English from the dictionary
 * (`SP_EN`, web/i18n-en.json, put in front of this file by the server). It translates text
 * nodes and a few attributes as the pages draw them, so the drawing code stays as it is.
 *
 * Keys are the Japanese as the source writes it; `{0}`, `{1}`, ... stand for values filled in
 * at run time (numbers, ids). A text that is not in the dictionary stays as it is, so the
 * user's own words and Claude's answers are never touched. */
(() => {
  // `?lang=en` (or ja) overrides the PC's language for this page's origin, for checking the other one
  let forced = null;
  try {
    const q = new URLSearchParams(location.search).get("lang");
    if (q === "ja" || q === "en") localStorage.setItem("sp_lang", q);
    if (q === "auto") localStorage.removeItem("sp_lang");
    forced = localStorage.getItem("sp_lang");
  } catch { /* storage may be unavailable */ }
  const pref = forced || (navigator.languages && navigator.languages[0]) || navigator.language || "ja";
  const lang = /^ja\b/i.test(pref) ? "ja" : "en";
  window.SP_LANG = lang;
  document.documentElement.lang = lang;
  window.tr = (s) => s;
  if (lang === "ja" || typeof SP_EN !== "object") return;

  const KANA = /[぀-ヿ一-鿿]/;
  const exact = new Map();
  const patterns = [];
  const esc = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  for (const [k, v] of Object.entries(SP_EN)) {
    if (!v) continue;
    if (!/\{\d+\}/.test(k)) { exact.set(k, v); continue; }
    // a hole at the very start or end is often an icon element, not text: "{0}保存先" also reads "保存先"
    const edge = /^\{(\d+)\}(.*)$|^(.*?)\{(\d+)\}$/.exec(k);
    if (edge) {
      const rest = (edge[2] ?? edge[3]).trim(), hole = `{${edge[1] ?? edge[4]}}`;
      if (rest && !/\{\d+\}/.test(rest) && !exact.has(rest)) exact.set(rest, v.replace(hole, "").trim());
    }
    const order = [];
    const parts = k.split(/(\{\d+\})/);
    for (const p of parts) { const m = /^\{(\d+)\}$/.exec(p); if (m) order.push(+m[1]); }
    const build = (hole) => new RegExp(`^${parts.map((p) => (/^\{\d+\}$/.test(p) ? hole : esc(p))).join("")}$`, "s");
    const loose = new Set((typeof SP_FREE === "object" && SP_FREE[k]) || []);
    // a pattern that takes someone's own words needs a real word around them: a lone particle
    // ("{0} を{1}") would cut through any Japanese sentence
    if (loose.size && !/[゠-ヿ一-鿿]/.test(k.replace(/\{\d+\}/g, ""))) continue;
    // the shortest holes first, then the longest ("{0}（{1}）" around a phrase that has brackets of its own)
    patterns.push({ res: [build("(.*?)"), build("(.*)")], v, order, loose, fixed: k.replace(/\{\d+\}/g, "").length });
  }
  patterns.sort((a, b) => b.fixed - a.fixed); // the most specific pattern first

  function one(s, depth = 0) {
    const hit = exact.get(s);
    if (hit !== undefined) return hit;
    if (depth > 6) return null;
    for (const p of patterns) {
      for (const re of p.res) {
        const m = re.exec(s);
        if (!m) continue;
        // a hole for a value (a number, an id, a known label) must not swallow Japanese we cannot
        // translate: that would be someone's own words matched by accident ("証拠のカット")
        let ok = true;
        const vals = {};
        p.order.forEach((n, j) => {
          const g = m[1 + j];
          if (!KANA.test(g)) { vals[n] = g; return; }
          const t = tr(g, depth + 1);
          if (t === g && !p.loose.has(n)) ok = false;
          vals[n] = t;
        });
        if (ok) return p.v.replace(/\{(\d+)\}/g, (_, i) => vals[+i] ?? "");
      }
    }
    return null;
  }

  /** A string in English when the dictionary knows it; otherwise as it is. */
  function tr(s, depth = 0) {
    if (typeof s !== "string" || !KANA.test(s)) return s;
    // a tooltip of several lines: each line on its own
    if (/\n/.test(s.trim()) && depth < 6) return s.split("\n").map((l) => tr(l, depth + 1)).join("\n");
    const lead = /^\s*/.exec(s)[0], trail = /\s*$/.exec(s)[0];
    const core = s.trim().replace(/\s+/g, " ");
    let r = one(core, depth);
    if (r == null) {
      // an icon, a number or punctuation around a known phrase ("✓ 撮影", "（撮り直しなし）"):
      // take off the front only, the back only, then both (brackets may belong to the phrase)
      const m = /^([^\p{L}]*)(.*?)([^\p{L}]*)$/u.exec(core);
      if (m) {
        for (const [pre, mid, post] of [[m[1], m[2] + m[3], ""], ["", m[1] + m[2], m[3]], [m[1], m[2], m[3]]]) {
          if (!mid || mid === core) continue;
          const x = one(mid, depth);
          if (x != null) { r = pre + x + post; break; }
        }
      }
    }
    if (r == null && depth < 6) {
      // a chip made of parts ("ティザー · 30秒", "previz · 撮った 6")
      for (const sep of [" · ", " / ", " → ", "、"]) {
        if (!core.includes(sep)) continue;
        // only when every Japanese part is known: a sentence of someone's own stays whole
        const src = core.split(sep);
        const parts = src.map((p) => tr(p, depth + 1));
        if (parts.every((p) => !KANA.test(p))) { r = parts.join(sep === "、" ? ", " : sep); break; }
      }
    }
    return r == null ? s : lead + r + trail;
  }
  window.tr = tr;

  const ATTRS = ["title", "placeholder", "aria-label", "alt", "data-ex", "data-row-ex"];
  // what the user writes is never touched: the text in a box (its placeholder is still ours),
  // anything editable, and anything marked data-noi18n
  const SKIP = new Set(["SCRIPT", "STYLE", "TEXTAREA"]);
  const theirs = (el) => !el || el.isContentEditable || !!el.closest?.("[data-noi18n]");
  const mine = (el) => theirs(el) || SKIP.has(el.tagName);

  const WIDE = /[（）]/;
  function text(n) {
    if (mine(n.parentElement)) return;
    // what is left of a translated line: "（s01, s02）" next to English reads "(s01, s02)"
    if (!KANA.test(n.data)) {
      if (WIDE.test(n.data)) n.data = n.data.replace(/（/g, " (").replace(/）/g, ")").replace(/^ \(/, "(");
      return;
    }
    const t = tr(n.data);
    if (t !== n.data) n.data = t;
  }
  function attr(el, a) {
    if (theirs(el)) return;
    const v = el.getAttribute(a);
    if (!v || !KANA.test(v)) return;
    const t = tr(v);
    if (t !== v) el.setAttribute(a, t);
  }
  function walk(root) {
    if (root.nodeType === 3) return text(root);
    if (root.nodeType !== 1 || theirs(root)) return;
    for (const a of ATTRS) attr(root, a);
    const w = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT);
    for (let n = w.nextNode(); n; n = w.nextNode()) {
      if (n.nodeType === 3) text(n);
      else for (const a of ATTRS) attr(n, a);
    }
  }

  new MutationObserver((list) => {
    for (const m of list) {
      if (m.type === "characterData") text(m.target);
      else if (m.type === "attributes") attr(m.target, m.attributeName);
      else for (const n of m.addedNodes) walk(n);
    }
  }).observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });

  // dialogs the pages open with a message
  for (const name of ["alert", "confirm", "prompt"]) {
    const orig = window[name].bind(window);
    window[name] = (msg, ...rest) => orig(tr(String(msg ?? "")), ...rest);
  }
  // CSS-drawn labels
  document.documentElement.style.setProperty("--label-rendering", '"Filming"');
  document.documentElement.style.setProperty("--label-revising", '"Revising…"');
  document.documentElement.style.setProperty("--label-filming", '"Filming…"');
  document.addEventListener("DOMContentLoaded", () => { walk(document.body); document.title = tr(document.title); });
})();
