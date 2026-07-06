// Uses format.pkg and bundled fonts, with no TeXLive filesystem boot.

import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const assets = path.join(repo, 'web', 'assets');

const wasm = require(path.join(repo, 'crates/mathtex-wasm/pkg-node/mathtex_wasm.js'));

const packaged = new Uint8Array(fs.readFileSync(path.join(assets, 'format.pkg')));

// \text and \mathrm require Latin Modern Roman, beyond the math font, or glyphs render blank.
function loadFonts(dir) {
  const map = new Map();
  for (const f of fs.readdirSync(dir)) {
    if (!f.toLowerCase().endsWith('.otf')) continue;
    const bytes = new Uint8Array(fs.readFileSync(path.join(dir, f)));
    map.set(f.toLowerCase().replace(/\.otf$/, ''), bytes);
  }
  return map;
}
function normalizeFontName(name) {
  let n = String(name).trim();
  const m = n.match(/^\[([^\]]+)\]/); // "[lmroman10-regular]:mapping=..." gives the stem
  if (m) n = m[1];
  n = n.split(/[:/]/)[0];
  return n.toLowerCase().replace(/\.otf$/, '');
}
const FONTS = loadFonts(assets);
function makeResolve({ log = false } = {}) {
  return (name) => {
    const bytes = FONTS.get(normalizeFontName(name)) || null;
    if (!bytes && log) console.log(`    [resolver miss] ${name}`);
    return bytes;
  };
}
const resolve = makeResolve();

const CAUCHY =
  String.raw`\left(\sum_{k=1}^n a_k b_k\right)^2\leq\left(\sum_{k=1}^n a_k^2\right)\left(\sum_{k=1}^n b_k^2\right)`;

let failures = 0;
function check(name, fn) {
  try {
    fn();
    console.log(`  ok   ${name}`);
  } catch (err) {
    failures++;
    console.log(`  FAIL ${name}: ${err.message}`);
  }
}

console.log('mathtex wasm E2E');

const session = new wasm.Session(packaged, resolve);
const svg = session.render(CAUCHY);
fs.writeFileSync(path.join(assets, 'cauchy_schwarz.wasm.svg'), svg);

// SVG output uses <use href="#gN"> for glyph instances and <path> for unique outlines in <defs>.
const glyphCount = (svg.match(/<use/g) || []).length;
const defCount = (svg.match(/<path/g) || []).length;
console.log(`  rendered ${svg.length} bytes, ${glyphCount} glyph <use> reusing ${defCount} <defs> outline(s)`);

check('produces an <svg> root', () => assert.ok(svg.startsWith('<svg'), 'missing <svg>'));
check('Cauchy Schwarz draws 33 glyph outlines', () =>
  assert.strictEqual(glyphCount, 33, `expected 33 glyph <use>, got ${glyphCount}`));
check('glyph outlines are deduped into <defs> (fewer defs than instances)', () =>
  assert.ok(defCount > 0 && defCount < glyphCount,
    `expected unique outlines (${defCount}) < instances (${glyphCount})`));
check('amsmath is in the format (renders a second equation)', () => {
  const frac = session.render(String.raw`\frac{a}{b}+\sqrt{x}`);
  assert.ok((frac.match(/<use/g) || []).length > 0, 'second equation drew no glyphs');
});
check('\\text{…} draws native Latin Modern Roman text glyphs', () => {
  // These glyphs were blank before bundling the text fonts.
  const t = session.render(String.raw`x+\text{for all}+y`);
  const n = (t.match(/<use/g) || []).length;
  assert.ok(n >= 9, `expected >=9 glyph <use> (x,y + 6 text letters), got ${n}`);
});
check('\\mathrm{…} draws text-font glyphs', () => {
  const m = session.render(String.raw`\mathrm{abc}`);
  assert.strictEqual((m.match(/<use/g) || []).length, 3, 'expected 3 \\mathrm glyphs');
});
check('a reused Session renders many fragments identically', () => {
  assert.strictEqual(session.render(CAUCHY), svg, 'Session render is not deterministic across calls');
});
check('one-shot render() free function still works', () => {
  const one = wasm.render(packaged, resolve, String.raw`a+b`);
  assert.ok((one.match(/<use/g) || []).length >= 3, 'one-shot render drew too few glyphs');
});
check('Session rejects a foreign packaged buffer', () => {
  const ret = (() => {
    try {
      new wasm.Session(new Uint8Array([1, 2, 3]), resolve);
      return 'no-throw';
    } catch {
      return 'threw';
    }
  })();
  assert.strictEqual(ret, 'threw', 'expected an error for a bad package');
});

if (failures) {
  console.log(`\n${failures} failure(s)`);
  process.exit(1);
}
console.log('\nall E2E checks passed');
