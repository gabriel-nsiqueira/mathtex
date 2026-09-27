# mathtex

`mathtex` runs the XeTeX engine itself, translated to Rust, and hands you the finished layout as an intermediate representation (IR).

- **TeX's own layout.** All math comes straight from XeTeX's own algorithms.
- **Runs anywhere Rust runs.** Native and `wasm32-unknown-unknown`, with no TeX install.
- **Made for editors.** Every glyph maps back to the input that produced it.
- **Safe with user input.** Each formula runs sandboxed from a fresh copy of the format, with a work budget, no file access and structured errors.
- **Fast.** A few milliseconds per formula in a release build.

## Quick start

Bake LaTeX code into formats, then typeset as many formulas as it is needed. The following code displays a basic use of the library:

```rust
use mathtex_engine::{
    Format, FormatBuilder, MathMode, NoHostBoxes, Options, ResourceFontLoader, TexmfResources,
    Typesetter,
};

let texmf = TexmfResources::discover()?;
let fonts = ResourceFontLoader::new(&texmf);
let bytes = FormatBuilder::new(
    r"\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}",
)
.kernel("latex.ltx")
.build(&texmf, &fonts)?;

let mut typesetter = Typesetter::new(Format::from_owned(bytes)?, fonts, Options::default())?;
let typeset = typesetter.typeset(r"\frac{a}{b}+\sqrt{x}", MathMode::Display, &NoHostBoxes)?;
let svg = mathtex_svg::render(&typeset.fragment, &outlines, &mathtex_svg::SvgOptions::default());
```

Ship the format with your app and parse them with `Format::from_static`.

> Note: A format belongs to the engine build that baked it, and any other build will be refused to load.

## Development

```sh
tools/fetch-test-fixtures.sh
cargo test --workspace
cargo run -p mathtex-svg-cli -- 'x^2+y' out.svg
```

Tests read Computer Modern TFMs and sample fonts from a small sparse clone of the pinned `texlive-source` commit, which the fetch script creates in the gitignored `cache/` directory. The LaTeX, unicode-math and oracle tests also need an installed TeX Live, found through `MATHTEX_TEXMF_ROOT` or `kpsewhich`. Missing sources skip with a notice, and `MATHTEX_REQUIRE_FIXTURES=1` or `MATHTEX_REQUIRE_TEXLIVE=1` make them fail instead.

### Web demo

There is a web demo in `web/`. It can be built with `wasm-pack build crates/mathtex-wasm --target web`. To dump its format, run `node tools/build-format/dump.mjs`. It can be end-to-end tested with `node test/e2e.mjs`.

### Regenerating the engine

- `generated/portable-engine` is produced entirely by `tools/web2c-import` from the XeTeX sources.
- `tools/bootstrap-texlive-web2c-c2rust.sh` rebuilds those inputs from a `texlive-source` clone.
- `tools/patch-translated-engine.sh` regenerates the engine after a change to the patcher or `runtime/prelude.rs.in`.
- `tools/check-regen.sh` fails when the committed engine differs from a fresh regeneration.

## License

The hand written crates are `MIT OR Apache-2.0`. The generated engine derives from the TeX, eTeX and XeTeX sources, whose license texts are collected in `generated/portable-engine/THIRD-PARTY-LICENSES.json`.
