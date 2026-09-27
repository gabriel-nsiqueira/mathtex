//! wasm-bindgen entry points: bake a packaged format and render math fragments to SVG.

use mathtex_engine::{
    Format, FormatBuilder, MathMode, NoHostBoxes, Options, Resource, ResourceError,
    ResourceFontLoader, ResourceProvider, ResourceRequest, Typesetter,
};
use mathtex_ir::{FontKey, GlyphId, GlyphOutline};
use mathtex_svg::{GlyphOutlineSource, SvgOptions};
use wasm_bindgen::prelude::*;

/// LaTeX with amsmath and unicode-math, everything a fragment may use must load here.
const PREAMBLE: &str = r"\nonstopmode\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}";

/// Resources from a JS `(name, kind) -> Uint8Array | null` resolver, `kind` is a `ResourceKind` name.
struct JsResources(js_sys::Function);

impl ResourceProvider for JsResources {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let name = request.canonical_name();
        let kind = JsValue::from_str(&format!("{:?}", request.kind));
        let result = self
            .0
            .call2(&JsValue::NULL, &JsValue::from_str(&name), &kind)
            .map_err(|_| ResourceError::not_found(request))?;
        if result.is_null() || result.is_undefined() {
            return Err(ResourceError::not_found(request));
        }
        Ok(Resource::answering(
            request,
            js_sys::Uint8Array::from(result).to_vec(),
        ))
    }
}

type Fonts = ResourceFontLoader<JsResources>;

/// Draws glyphs from the faces the typesetter loaded, found by their keys.
struct LoadedOutlines<'a>(&'a Fonts);

impl GlyphOutlineSource for LoadedOutlines<'_> {
    fn glyph_outlines(&self, font: FontKey, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>> {
        self.0
            .font(font)
            .and_then(|font| font.glyph_outlines(glyphs).ok())
            .unwrap_or_else(|| vec![None; glyphs.len()])
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Bakes LaTeX, amsmath and unicode-math into a packaged format.
#[wasm_bindgen]
pub fn build_format(resolve: js_sys::Function) -> Result<Vec<u8>, JsValue> {
    let resources = JsResources(resolve);
    let fonts = ResourceFontLoader::new(&resources);
    FormatBuilder::new(PREAMBLE)
        .kernel("latex.ltx")
        .build(&resources, &fonts)
        .map_err(js_error)
}

/// A loaded format and its fonts, rendering fragments one after another.
#[wasm_bindgen]
pub struct Session {
    typesetter: Typesetter<Fonts>,
}

#[wasm_bindgen]
impl Session {
    /// Loads a packaged format, fonts come from `resolve`.
    #[wasm_bindgen(constructor)]
    pub fn new(packaged: &[u8], resolve: js_sys::Function) -> Result<Session, JsValue> {
        let format = Format::from_owned(packaged.to_vec()).map_err(js_error)?;
        let fonts = ResourceFontLoader::new(JsResources(resolve));
        let typesetter = Typesetter::new(format, fonts, Options::default()).map_err(js_error)?;
        Ok(Session { typesetter })
    }

    /// Renders one display style fragment to SVG filled with `currentColor`.
    pub fn render(&mut self, latex: &str) -> Result<String, JsValue> {
        let typeset = self
            .typesetter
            .typeset(latex, MathMode::Display, &NoHostBoxes)
            .map_err(js_error)?;
        Ok(mathtex_svg::render(
            &typeset.fragment,
            &LoadedOutlines(self.typesetter.fonts()),
            &SvgOptions::default().with_current_color(true),
        ))
    }
}

/// Loads a format and renders one fragment.
#[wasm_bindgen]
pub fn render(packaged: &[u8], resolve: js_sys::Function, latex: &str) -> Result<String, JsValue> {
    Session::new(packaged, resolve)?.render(latex)
}
