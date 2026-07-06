// Builds web/assets/format.pkg and web/assets/latinmodern-math.otf using an ls-R lookup.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');

const TEXMF = process.env.MATHTEX_TEXMF_ROOT ?? '/usr/local/texlive/2025/texmf-dist';

// Search prefixes follow texmf.cnf priority order to match the host TeX resolver.
const PREFIXES = {
  tex: ['tex/xelatex', 'tex/latex', 'tex/xetex', 'tex/generic', 'tex'],
  tfm: ['fonts/tfm'],
  opentype: ['fonts/opentype', 'fonts/truetype'],
  enc: ['fonts/enc'],
  map: ['fonts/map'],
};

const SUFFIXES = {
  // Font stems such as lmroman10-regular need extensions so fontspec can locate text fonts.
  Font: ['.otf', '.ttf', '.otc', '.tfm'],
  Package: ['.sty', '.tex', '.def', '.ltx'],
  Class: ['.cls'],
  FontDefinition: ['.fd'],
  PackageSupport: ['.def', '.cfg', '.ldf', '.clo', '.sty', '.tex'],
  Config: ['.cfg', '.cnf', '.tex'],
  Encoding: ['.enc'],
  Map: ['.map'],
};

function parseLsR(text) {
  const index = new Map(); // basename to dirs in ls-R order
  let dir = '';
  for (let line of text.split('\n')) {
    line = line.replace(/\s+$/, '');
    if (!line || line.startsWith('%')) continue;
    if (line.endsWith(':')) {
      dir = line.slice(0, -1).replace(/^\.\//, '');
      continue;
    }
    if (!index.has(line)) index.set(line, []);
    index.get(line).push(dir);
  }
  return index;
}

function normalize(name) {
  let n = name.trim();
  for (;;) {
    if (n.startsWith('./')) n = n.slice(2);
    else if (n.startsWith('[]')) n = n.slice(2);
    else if (n.startsWith(':')) n = n.slice(1);
    else break;
  }
  n = n.replace(/^[[\]"']+|[[\]"']+$/g, '');
  const parts = n.split(/[/\\]/);
  return parts[parts.length - 1] || n;
}

// Selects the search format and refines it by extension, matching format_for.
function formatFor(kind, filename) {
  const lower = filename.toLowerCase();
  if (kind === 'Encoding') return 'enc';
  if (kind === 'Map') return 'map';
  if (kind === 'Font') {
    return lower.endsWith('.otf') || lower.endsWith('.ttf') || lower.endsWith('.otc')
      ? 'opentype'
      : 'tfm';
  }
  if (lower.endsWith('.enc')) return 'enc';
  if (lower.endsWith('.map')) return 'map';
  if (lower.endsWith('.tfm')) return 'tfm';
  if (lower.endsWith('.otf') || lower.endsWith('.ttf')) return 'opentype';
  return 'tex';
}

function resolveOne(index, filename, fmt) {
  const dirs = index.get(filename);
  if (!dirs) return null;
  for (const prefix of PREFIXES[fmt]) {
    for (const dir of dirs) {
      if (dir === prefix || (dir.startsWith(prefix) && dir[prefix.length] === '/')) {
        return path.join(TEXMF, dir, filename);
      }
    }
  }
  return null;
}

function makeResolver(index) {
  return (name, kind) => {
    const base = normalize(name);
    const candidates = [base];
    if (!path.extname(base)) {
      for (const ext of SUFFIXES[kind] || []) candidates.push(base + ext);
    }
    for (const cand of candidates) {
      const file = resolveOne(index, cand, formatFor(kind, cand));
      if (file) {
        try {
          return new Uint8Array(fs.readFileSync(file));
        } catch {
        }
      }
    }
    return null;
  };
}

function main() {
  if (!fs.existsSync(path.join(TEXMF, 'ls-R'))) {
    console.error(`No TeXLive ls-R at ${TEXMF}; cannot build the format.`);
    process.exit(2);
  }
  const index = parseLsR(fs.readFileSync(path.join(TEXMF, 'ls-R'), 'utf8'));
  const resolve = makeResolver(index);

  const wasm = require(path.join(repo, 'crates/mathtex-wasm/pkg-node/mathtex_wasm.js'));

  console.error('Booting LaTeX + amsmath + unicode-math + latinmodern-math ...');
  const t0 = Date.now();
  const packaged = wasm.build_format(resolve);
  console.error(`Packaged format: ${(packaged.length / 1024 / 1024).toFixed(1)} MiB in ${((Date.now() - t0) / 1000).toFixed(1)}s`);

  const assets = path.join(repo, 'web', 'assets');
  fs.mkdirSync(assets, { recursive: true });
  fs.writeFileSync(path.join(assets, 'format.pkg'), Buffer.from(packaged));

  const otf = resolveOne(index, 'latinmodern-math.otf', 'opentype');
  if (!otf) {
    console.error('latinmodern-math.otf not found');
    process.exit(2);
  }
  fs.copyFileSync(otf, path.join(assets, 'latinmodern-math.otf'));

  console.error(`Wrote ${path.join(assets, 'format.pkg')} and latinmodern-math.otf`);
}

main();
