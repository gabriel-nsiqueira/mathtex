# mathtex

mathtex is a TeX layout engine written in Rust. It typesets LaTeX math into a renderer neutral layout IR. It runs natively and on `wasm32-unknown-unknown`, so a browser page can typeset math locally with no TeX install and no server. A packaged format file stands in for the TeX distribution at render time.

The engine core is translated from the TeX, eTeX and XeTeX sources through Web2C and C2Rust, then reworked by an AST patcher that routes host access behind traits and adds source span tracking. The translated core is unsafe Rust. Hand authored crates around it own sessions, formats, resources, fonts and rendering.

## Layout

```
crates/mathtex-engine      profiles, format building and serialization, resource
                           and font systems, fragment sessions, IR lowering
crates/mathtex-ir          layout IR, including source spans
crates/mathtex-render      backend traits
crates/mathtex-font        font loading and shaping over rustybuzz and ttf-parser
crates/mathtex-svg         SVG backend with deduplicated glyph outlines
crates/mathtex-wasm        wasm bindings, build_format and a cached Session renderer
crates/mathtex-svg-cli     native binary, one expression to SVG via a local TeXLive tree
crates/mathtex             facade crate
generated/portable-engine  machine generated engine core, never edited by hand
tools/web2c-import         the patcher that produces the generated crate
tools/build-format         dumps the packaged format used by the web page and tests
tools/bench                speed comparison against MathJax in Node
web                        browser demo, served from Cloudflare Pages
test/e2e.mjs               end to end render test in Node
```

## Build and test

```sh
cargo test --workspace
cargo run -p mathtex-svg-cli -- 'x^2+y' out.svg
```

The CLI and some engine tests read a local TeXLive tree and fixtures under `vendor`. Set `MATHTEX_TEXMF_ROOT` to a `texmf-dist` directory when the default locations do not match your install.

## Web demo

```sh
wasm-pack build crates/mathtex-wasm --target web    --out-dir pkg-web  --release
wasm-pack build crates/mathtex-wasm --target nodejs --out-dir pkg-node --release
cp crates/mathtex-wasm/pkg-web/mathtex_wasm.js crates/mathtex-wasm/pkg-web/mathtex_wasm_bg.wasm web/pkg/
node tools/build-format/dump.mjs
node test/e2e.mjs
cd web && python3 -m http.server 8000
```

The format must be dumped by a wasm32 build, which is why `dump.mjs` runs the Node wasm module. Deploys go out with `npx wrangler pages deploy web --project-name mathtex-demo --branch main`. The editor page under `web` loads a wasm module built from the separate mathtex editor repository.

## Regenerating the engine

`generated/portable-engine` is produced entirely by tooling. After changing the patcher, run `tools/patch-translated-engine.sh`. A full bootstrap from upstream sources runs through `tools/bootstrap-texlive-web2c-c2rust.sh` and needs a clone of `texlive-source` under `vendor`, the `tie` and `tangle` tools, `c2rust` and nightly Rust.

## License

`MIT OR Apache-2.0` for the hand authored crates. The generated engine core derives from the TeX family sources. Their license texts are collected into `generated/portable-engine/THIRD-PARTY-LICENSES.json` by `tools/generate_third_party_licenses.py`.
