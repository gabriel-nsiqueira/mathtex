import init, { WebEditor } from './pkg/mathtex_editor_js.js';

const out = document.getElementById('out');
const stage = document.getElementById('stage');
const capture = document.getElementById('capture');
const latexEl = document.getElementById('latex');
const status = document.getElementById('status');

const FONT_FILES = [
  'latinmodern-math.otf',
  'lmroman10-regular.otf', 'lmroman10-bold.otf', 'lmroman10-italic.otf', 'lmroman10-bolditalic.otf',
  'lmromanslant10-regular.otf', 'lmmono10-regular.otf',
  'lmroman5-regular.otf', 'lmroman6-regular.otf', 'lmroman7-regular.otf',
  'lmroman8-regular.otf', 'lmroman9-regular.otf', 'lmroman12-regular.otf',
];

function normalizeFontName(name) {
  let n = String(name).trim();
  const m = n.match(/^\[([^\]]+)\]/);
  if (m) n = m[1];
  n = n.split(/[:/]/)[0];
  return n.toLowerCase().replace(/\.otf$/, '');
}
function makeResolve(map) {
  return (name) => map.get(normalizeFontName(name)) || null;
}

let editor = null;

// IR viewBox units are points, 5px per point gives a comfortable display size.
const PT_TO_PX = 5;

function draw() {
  if (!editor) return;
  try {
    out.innerHTML = editor.render();
    const svg = out.querySelector('svg');
    if (svg) {
      const vb = (svg.getAttribute('viewBox') || '0 0 0 0').split(/\s+/).map(Number);
      svg.setAttribute('width', (vb[2] * PT_TO_PX).toFixed(2));
      svg.setAttribute('height', (vb[3] * PT_TO_PX).toFixed(2));
    }
    latexEl.textContent = editor.latex();
  } catch (e) {
    latexEl.textContent = String(e && e.message ? e.message : e);
  }
}

// Command keys and plain ascii use keydown while composed input uses input.
capture.addEventListener('keydown', (e) => {
  if (!editor) return;
  const k = e.key;
  if (k === 'Shift' || k === 'Control' || k === 'Alt' || k === 'Meta') return;
  if (e.isComposing || k === 'Dead' || k === 'Process' || k === 'Unidentified') return;
  if ((e.ctrlKey || e.metaKey) && !['a', 'A'].includes(k)) return; // keep browser shortcuts
  editor.key(k, e.shiftKey, e.ctrlKey, e.altKey, e.metaKey);
  e.preventDefault(); // stops the character from landing in the textarea
  draw();
});

capture.addEventListener('input', () => {
  if (!editor) return;
  const text = capture.value;
  capture.value = '';
  if (text) {
    editor.text(text);
    draw();
  }
});

stage.addEventListener('mousedown', (e) => {
  e.preventDefault();
  capture.focus();
});
capture.addEventListener('focus', () => stage.classList.add('focused'));
capture.addEventListener('blur', () => stage.classList.remove('focused'));

// Each script token is a single character or a named key (ArrowRight etc.) sent to the keymap.
const NAMED = new Set(['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown', 'Backspace', 'Tab', 'Enter']);
function replay(script) {
  if (!editor) return;
  editor = new WebEditor(window.__packaged, window.__resolve);
  for (const step of script) {
    const key = NAMED.has(step) ? step : (step === 'Right' ? 'ArrowRight'
      : step === 'Left' ? 'ArrowLeft' : step === 'Up' ? 'ArrowUp'
      : step === 'Down' ? 'ArrowDown' : step);
    editor.key(key, false, false, false, false);
  }
  draw();
  capture.focus();
}

async function main() {
  status.textContent = 'loading wasm engine…';
  await init();
  status.textContent = 'loading packaged format + fonts…';
  const V = '?v=1';
  const [pkgRes, ...fontResponses] = await Promise.all([
    fetch('./assets/format.pkg' + V),
    ...FONT_FILES.map((f) => fetch('./assets/' + f + V)),
  ]);
  const packaged = new Uint8Array(await pkgRes.arrayBuffer());
  const map = new Map();
  for (let i = 0; i < FONT_FILES.length; i++) {
    map.set(FONT_FILES[i].toLowerCase().replace(/\.otf$/, ''), new Uint8Array(await fontResponses[i].arrayBuffer()));
  }
  const resolve = makeResolve(map);
  window.__packaged = packaged;
  window.__resolve = resolve;

  editor = new WebEditor(packaged, resolve);
  status.textContent = 'ready, click the box and type math';
  capture.focus();

  for (const b of document.querySelectorAll('.examples button')) {
    b.addEventListener('click', () => replay(JSON.parse(b.dataset.script)));
  }

  draw();
}

main().catch((e) => {
  status.textContent = 'failed to initialize';
  latexEl.textContent = String(e && e.message ? e.message : e);
});
