import init, { Session } from "./pkg/mathtex_wasm.js";

const out = document.getElementById("out");
const err = document.getElementById("err");
const input = document.getElementById("latex");
const status = document.getElementById("status");
const backdrop = document.getElementById("backdrop");

// SVG elements carry data-source-start/data-source-end as UTF-8 byte offsets into the input.
const enc = new TextEncoder();
const dec = new TextDecoder();
const escapeHtml = (s) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

let activeRange = null; // {start, end} byte offsets into input.value, or null
let hoveredEl = null;

function renderBackdrop() {
    const text = input.value;
    if (!activeRange) {
        backdrop.textContent = text;
        return;
    }
    // Slice by byte offset since positions are UTF-8 byte indices.
    const bytes = enc.encode(text);
    const s = Math.max(0, Math.min(activeRange.start, bytes.length));
    const e = Math.max(s, Math.min(activeRange.end, bytes.length));
    backdrop.innerHTML =
        escapeHtml(dec.decode(bytes.slice(0, s))) +
        "<mark>" +
        escapeHtml(dec.decode(bytes.slice(s, e))) +
        "</mark>" +
        escapeHtml(dec.decode(bytes.slice(e)));
}

function setHover(el) {
    if (el === hoveredEl) return;
    if (hoveredEl) hoveredEl.classList.remove("src-hover");
    hoveredEl = el;
    if (el) {
        el.classList.add("src-hover");
        const start = Number(el.getAttribute("data-source-start"));
        const end = Number(el.getAttribute("data-source-end"));
        activeRange =
            Number.isFinite(start) && Number.isFinite(end)
                ? { start, end }
                : null;
    } else {
        activeRange = null;
    }
    renderBackdrop();
}

// Event delegation on the container survives each innerHTML replacement.
out.addEventListener("mouseover", (e) => {
    const el =
        e.target && e.target.closest
            ? e.target.closest("[data-source-start]")
            : null;
    setHover(el);
});
out.addEventListener("mouseleave", () => setHover(null));
input.addEventListener("scroll", () => {
    backdrop.scrollTop = input.scrollTop;
    backdrop.scrollLeft = input.scrollLeft;
});

// Latin Modern Roman faces provide the unicode math defaults for \text and \mathrm.
const FONT_FILES = [
    "latinmodern-math.otf",
    "lmroman10-regular.otf",
    "lmroman10-bold.otf",
    "lmroman10-italic.otf",
    "lmroman10-bolditalic.otf",
    "lmromanslant10-regular.otf",
    "lmmono10-regular.otf",
    "lmroman5-regular.otf",
    "lmroman6-regular.otf",
    "lmroman7-regular.otf",
    "lmroman8-regular.otf",
    "lmroman9-regular.otf",
    "lmroman12-regular.otf",
];

let session = null;

// Engine font names are reduced to the bare lowercase stem the font map is keyed on.
function normalizeFontName(name) {
    let n = String(name).trim();
    const m = n.match(/^\[([^\]]+)\]/);
    if (m) n = m[1];
    n = n.split(/[:/]/)[0];
    return n.toLowerCase().replace(/\.otf$/, "");
}

function makeResolve(map) {
    return (name) => map.get(normalizeFontName(name)) || null;
}

function draw() {
    if (!session) return;
    err.textContent = "";
    try {
        out.innerHTML = session.render(input.value);
    } catch (e) {
        out.innerHTML = "";
        err.textContent = String(e && e.message ? e.message : e);
    }
    hoveredEl = null;
    setHover(null);
}

async function main() {
    status.textContent = "loading wasm engine…";
    await init();
    status.textContent = "loading packaged format + fonts…";
    // URL version query forces a refetch after each asset change.
    const V = "?v=6";
    const [pkgRes, ...fontResponses] = await Promise.all([
        fetch("./assets/format.pkg" + V),
        ...FONT_FILES.map((f) => fetch("./assets/" + f + V)),
    ]);
    const packaged = new Uint8Array(await pkgRes.arrayBuffer());

    const map = new Map();
    let fontBytesTotal = 0;
    for (let i = 0; i < FONT_FILES.length; i++) {
        const bytes = new Uint8Array(await fontResponses[i].arrayBuffer());
        fontBytesTotal += bytes.length;
        map.set(FONT_FILES[i].toLowerCase().replace(/\.otf$/, ""), bytes);
    }
    const resolve = makeResolve(map);

    status.textContent = "instantiating format…";
    const t0 = performance.now();
    session = new Session(packaged, resolve);
    const setupMs = performance.now() - t0;

    status.textContent =
        `ready, format ${(packaged.length / 1024 / 1024).toFixed(1)} MiB + ${FONT_FILES.length} fonts ` +
        `${(fontBytesTotal / 1024 / 1024).toFixed(1)} MiB, loaded in ${setupMs.toFixed(0)} ms; ` +
        `each render reuses the cached format, all in wasm`;
    input.addEventListener("input", draw);
    draw();
}

main().catch((e) => {
    status.textContent = "failed to initialize";
    err.textContent = String(e && e.message ? e.message : e);
});
