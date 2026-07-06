import { mathjax } from '@mathjax/src/js/mathjax.js';
import { TeX } from '@mathjax/src/js/input/tex.js';
import { SVG } from '@mathjax/src/js/output/svg.js';
import { liteAdaptor } from '@mathjax/src/js/adaptors/liteAdaptor.js';
import { RegisterHTMLHandler } from '@mathjax/src/js/handlers/html.js';
import '@mathjax/src/js/input/tex/base/BaseConfiguration.js';
import '@mathjax/src/js/input/tex/ams/AmsConfiguration.js';
import '@mathjax/src/js/input/tex/newcommand/NewcommandConfiguration.js';
import '@mathjax/src/js/input/tex/configmacros/ConfigMacrosConfiguration.js';
import fs from 'node:fs';
import path from 'node:path';

import { fileURLToPath as __f } from 'node:url';
const REPO = path.resolve(path.dirname(__f(import.meta.url)), '..', '..');
const assets = path.join(REPO, 'web', 'assets');

const wasm = await import(path.join(REPO, 'crates/mathtex-wasm/pkg-node/mathtex_wasm.js'));
const packaged = new Uint8Array(fs.readFileSync(path.join(assets, 'format.pkg')));
const FONTS = new Map();
for (const f of fs.readdirSync(assets))
  if (f.endsWith('.otf')) FONTS.set(f.toLowerCase().replace(/\.otf$/, ''), new Uint8Array(fs.readFileSync(path.join(assets, f))));
const norm = (n) => { n = String(n).trim(); const m = n.match(/^\[([^\]]+)\]/); if (m) n = m[1]; return n.split(/[:/]/)[0].toLowerCase().replace(/\.otf$/, ''); };
const resolve = (n) => FONTS.get(norm(n)) || null;
const mtx = (s) => wasm.render(packaged, resolve, s);

const adaptor = liteAdaptor();
RegisterHTMLHandler(adaptor);
const mjDoc = mathjax.document('', {
  InputJax: new TeX({ packages: ['base', 'ams', 'newcommand', 'configmacros'] }),
  OutputJax: new SVG({ fontCache: 'none' }),
});
const mj = (s) => adaptor.outerHTML(mjDoc.convert(s, { display: true }));

const CORPUS = [
  ['simple',    String.raw`x^2 + y^2 = z^2`],
  ['quadratic', String.raw`\frac{-b \pm \sqrt{b^2-4ac}}{2a}`],
  ['euler',     String.raw`e^{i\pi} + 1 = 0`],
  ['sum',       String.raw`\sum_{k=1}^{n} k = \frac{n(n+1)}{2}`],
  ['integral',  String.raw`\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}`],
  ['cauchy',    String.raw`\left(\sum_{k=1}^n a_k b_k\right)^2 \leq \left(\sum_{k=1}^n a_k^2\right)\left(\sum_{k=1}^n b_k^2\right)`],
];

const glyphs = (svg) => (svg.match(/<path/g) || []).length;
const uses = (svg) => (svg.match(/<use/g) || []).length;

console.log('== output sanity (both must draw real glyphs) ==');
console.log('expr'.padEnd(11), 'mtx<path>'.padEnd(11), 'mj<path>'.padEnd(10), 'mj<use>');
for (const [name, tex] of CORPUS) {
  const a = mtx(tex), b = mj(tex);
  console.log(name.padEnd(11), String(glyphs(a)).padEnd(11), String(glyphs(b)).padEnd(10), String(uses(b)));
}

function timeit(fn, iters) {
  const t = new Array(iters);
  for (let i = 0; i < iters; i++) { const a = process.hrtime.bigint(); fn(); t[i] = Number(process.hrtime.bigint() - a) / 1e6; }
  t.sort((x, y) => x - y);
  const sum = t.reduce((x, y) => x + y, 0);
  return { median: t[t.length >> 1], mean: sum / iters, p95: t[Math.floor(iters * 0.95)], min: t[0] };
}

const WARMUP = 10, ITERS = 30;
for (let i = 0; i < WARMUP; i++) for (const [, t] of CORPUS) { mtx(t); mj(t); }

console.log(`\n== warm per-render, ${ITERS} iters (ms: median / mean / p95) ==`);
console.log('expr'.padEnd(11), 'mathtex'.padEnd(28), 'mathjax'.padEnd(26), 'mtx/mj');
for (const [name, tex] of CORPUS) {
  const m = timeit(() => mtx(tex), ITERS), j = timeit(() => mj(tex), ITERS);
  console.log(
    name.padEnd(11),
    `${m.median.toFixed(2)} / ${m.mean.toFixed(2)} / ${m.p95.toFixed(2)}`.padEnd(28),
    `${j.median.toFixed(3)} / ${j.mean.toFixed(3)} / ${j.p95.toFixed(3)}`.padEnd(26),
    `${(m.median / j.median).toFixed(0)}x`,
  );
}

// A tiny expression isolates fixed call overhead, which approximates mathtex format reload cost.
const TRIVIAL = String.raw`1`;
const mTriv = timeit(() => mtx(TRIVIAL), ITERS), jTriv = timeit(() => mj(TRIVIAL), ITERS);
console.log(`\n== fixed per-call overhead probe ("1") ==`);
console.log(`mathtex "1": ${mTriv.median.toFixed(2)}ms median  (≈ format-reload tax paid every render)`);
console.log(`mathjax "1": ${jTriv.median.toFixed(3)}ms median`);

const mAll = timeit(() => { for (const [, t] of CORPUS) mtx(t); }, ITERS);
const jAll = timeit(() => { for (const [, t] of CORPUS) mj(t); }, ITERS);
console.log(`\n== full corpus (${CORPUS.length} exprs) median ==`);
console.log(`mathtex ${mAll.median.toFixed(2)}ms  (${(mAll.median / CORPUS.length).toFixed(2)}ms/expr)`);
console.log(`mathjax ${jAll.median.toFixed(2)}ms  (${(jAll.median / CORPUS.length).toFixed(3)}ms/expr)`);
console.log(`mathjax is ${(mAll.median / jAll.median).toFixed(0)}x faster on this corpus (warm, current APIs).`);
console.log(`\nNode ${process.version}; mathtex per-call re-instantiates the 8.4MB format (from_bytes + font rebind) every render().`);
