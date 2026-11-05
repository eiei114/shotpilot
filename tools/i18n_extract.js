// The English mode (crates/shotpilot/web/i18n.js): list the Japanese UI strings that the
// dictionary (web/i18n-en.json) does not know yet, and refresh web/i18n-free.json (which holes of a
// key hold the user's or Claude's own words, so they are left as written).
// Keys keep their interpolations as {0}, {1}, ...; the runtime turns them into patterns.
// usage: node tools/i18n_extract.js [repo]   (prints the missing keys as JSON)
const fs = require("fs");
const path = require("path");
const repo = process.argv[2] || path.join(__dirname, "..");
const KANA = /[\u3040-\u30ff\u4e00-\u9fff]/;
const OPEN = "\u0001", CLOSE = "\u0004";
const HOLE = new RegExp(`${OPEN}(\\d*)${CLOSE}`, "g");
const keys = new Map(); // key -> first source
const exprs = []; // expression source of each hole
const free = {}; // key -> hole numbers that hold free text
const FREE = /esc\(|\.(text|ja|en|note|direction|purpose|label|name|title|plan|why|what|detail|summary|reason|message|msg|core|pitch|subject|look|description|headline|example|error)\b|\b(note|text|label|name|title|msg|why|what|detail|reason)\b/;

function addSegment(text, src) {
  for (let seg of text.split(/<[^>]*>/)) {
    seg = seg.replace(/\s+/g, " ").trim();
    if (!KANA.test(seg)) continue;
    let i = 0;
    const kinds = [];
    const key = seg.replace(HOLE, (_, e) => { kinds.push(e === "" ? "" : exprs[+e]); return `{${i++}}`; });
    if (!keys.has(key)) keys.set(key, src);
    const f = kinds.map((e, n) => (FREE.test(e) ? n : -1)).filter((n) => n >= 0);
    if (f.length) free[key] = [...new Set([...(free[key] || []), ...f])];
  }
}

const ATTR = /\b(title|placeholder|aria-label|alt|data-ex|data-row-ex)="([^"]*)"/g;
function addAttrs(text, src) {
  let m;
  ATTR.lastIndex = 0;
  while ((m = ATTR.exec(text))) addSegment(m[2].replace(/&#10;/g, " "), src);
}

/** String literals of JS (or Rust), with template expressions replaced by holes. */
function literals(code, rust) {
  const out = [];
  let i = 0;
  const prevSig = () => { let j = i - 1; while (j >= 0 && /\s/.test(code[j])) j--; return j < 0 ? "" : code[j]; };
  function skipNoise() {
    const c = code[i];
    if (c === "/" && code[i + 1] === "/") { while (i < code.length && code[i] !== "\n") i++; return true; }
    if (c === "/" && code[i + 1] === "*") { const e = code.indexOf("*/", i + 2); i = e < 0 ? code.length : e + 2; return true; }
    if (rust || c !== "/") return false;
    const p = prevSig();
    const word = /\b(return|typeof|case|in|of)\s*$/.test(code.slice(Math.max(0, i - 10), i));
    if (!(p === "" || "(,=:[!&|?{};+-*%<>~^".includes(p) || word)) return false;
    i++;
    let cls = false;
    while (i < code.length && code[i] !== "\n") {
      const d = code[i];
      if (d === "\\") { i += 2; continue; }
      if (d === "[") cls = true;
      else if (d === "]") cls = false;
      else if (d === "/" && !cls) { i++; break; }
      i++;
    }
    while (/[a-z]/.test(code[i] || "")) i++;
    return true;
  }
  function readTemplate() {
    let buf = "";
    while (i < code.length) {
      const c = code[i];
      if (c === "\\") { buf += code[i + 1] === "n" ? "\n" : code[i + 1]; i += 2; continue; }
      if (c === "`") { i++; return buf; }
      if (c === "$" && code[i + 1] === "{") {
        i += 2;
        const at = i;
        readExpr();
        exprs.push(code.slice(at, i - 1));
        buf += OPEN + (exprs.length - 1) + CLOSE;
        continue;
      }
      buf += c; i++;
    }
    return buf;
  }
  function readQuoted(q) {
    let buf = "";
    while (i < code.length) {
      const c = code[i];
      if (c === "\\") { buf += code[i + 1] === "n" ? "\n" : code[i + 1]; i += 2; continue; }
      if (c === q) { i++; return buf; }
      if (c === "\n" && !rust) return buf;
      buf += c; i++;
    }
    return buf;
  }
  function readExpr() {
    let depth = 1;
    while (i < code.length && depth > 0) {
      if (skipNoise()) continue;
      const c = code[i];
      if (c === "`") { i++; out.push(readTemplate()); continue; }
      if (c === '"' || c === "'") { i++; out.push(readQuoted(c)); continue; }
      if (c === "{") depth++;
      if (c === "}") depth--;
      i++;
    }
  }
  while (i < code.length) {
    if (skipNoise()) continue;
    const c = code[i];
    if (!rust && c === "`") { i++; out.push(readTemplate()); continue; }
    if (c === '"' || (!rust && c === "'")) { i++; out.push(readQuoted(c)); continue; }
    i++;
  }
  return out;
}

function page(file) {
  const html = fs.readFileSync(file, "utf8");
  const src = path.basename(file);
  const scripts = [];
  const shell = html
    .replace(/<style[\s\S]*?<\/style>/g, "")
    .replace(/<script[^>]*>([\s\S]*?)<\/script>/g, (_, s) => { scripts.push(s); return ""; })
    .replace(/<!--[\s\S]*?-->/g, "");
  addAttrs(shell, src);
  addSegment(shell.replace(ATTR, ""), src);
  for (const s of scripts) {
    for (const lit of literals(s, false)) {
      if (!KANA.test(lit)) continue;
      addAttrs(lit, src);
      addSegment(lit.replace(/\b(title|placeholder|aria-label|alt|data-[a-z-]+)="[^"]*"/g, ""), src);
    }
  }
}

function rust(file) {
  const code = fs.readFileSync(file, "utf8").replace(/'(\\.|[^'\\])'/g, "' '").replace(/'[a-z_]+\b(?!')/g, "");
  const src = path.basename(file);
  for (let lit of literals(code, true)) {
    if (!KANA.test(lit)) continue;
    lit = lit.replace(/\{\{/g, "\u0002").replace(/\}\}/g, "\u0003").replace(/\{[^{}]*\}/g, OPEN + CLOSE).replace(/\u0002/g, "{").replace(/\u0003/g, "}");
    for (const line of lit.split("\n")) addSegment(line, src);
  }
}

const web = path.join(repo, "crates/shotpilot/web");
page(path.join(web, "home.html"));
page(path.join(web, "index.html"));
for (const f of fs.readdirSync(path.join(repo, "crates/shotpilot/src"))) {
  if (!f.endsWith(".rs") || ["finish.rs", "templates.rs"].includes(f)) continue;
  rust(path.join(repo, "crates/shotpilot/src", f));
}
const dictFile = path.join(web, "i18n-en.json");
const dict = JSON.parse(fs.readFileSync(dictFile, "utf8"));
const missing = [...keys.keys()].filter((k) => !(k in dict));
console.log(JSON.stringify(Object.fromEntries(missing.map((k) => [k, ""])), null, 1));
// keep hand-written entries for keys the sources do not show directly
const freeFile = path.join(web, "i18n-free.json");
const old = fs.existsSync(freeFile) ? JSON.parse(fs.readFileSync(freeFile, "utf8")) : {};
const merged = Object.fromEntries(Object.entries({ ...old, ...free }).filter(([k]) => k in dict).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
fs.writeFileSync(freeFile, JSON.stringify(merged, null, 1) + "\n");
console.error(`${keys.size} keys in the sources, ${missing.length} not in i18n-en.json; add them (Japanese -> English) and rerun`);
