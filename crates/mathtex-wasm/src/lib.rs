//! wasm-bindgen entry points for packaged format building and SVG rendering.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mathtex_engine::font::{
    FontData, FontError, FontQuery, FontSystem, RustybuzzFontSystem, ShapeRequest, ShapedText,
};
use mathtex_engine::portable_engine as pe;
use mathtex_engine::{
    GeneratedFontSystemAdapter, GeneratedFormatCache, GeneratedResourceProvider,
    ProviderResourceRequest, Resource, ResourceError, ResourceFontSystem, ResourceKind,
    ResourceProvider,
};
use wasm_bindgen::prelude::*;

/// LaTeX + amsmath + unicode-math preamble ending with `\dump`, used to build the packaged format.
const PREAMBLE: &[u8] = br"\nonstopmode\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump";

/// Extracts a bare font filename from a XeTeX `\font` spec.
fn font_filename(spec: &str) -> String {
    let spec = spec.trim();
    if let Some(rest) = spec.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest).to_string()
    } else {
        spec.split([':', '/']).next().unwrap_or(spec).to_string()
    }
}

/// ResourceProvider backed by a resolver closure shared across engine services.
#[derive(Clone)]
struct SharedProvider(Rc<dyn Fn(&ProviderResourceRequest) -> Result<Resource, ResourceError>>);

impl ResourceProvider for SharedProvider {
    fn read_request(&self, request: &ProviderResourceRequest) -> Result<Resource, ResourceError> {
        (self.0)(request)
    }
}

/// Wraps SharedProvider and strips the full XeTeX font spec to a bare filename for the resolver.
struct FontProvider(SharedProvider);

impl ResourceProvider for FontProvider {
    fn read_request(&self, request: &ProviderResourceRequest) -> Result<Resource, ResourceError> {
        let name = font_filename(&request.canonical_name());
        self.0.read(&name, ResourceKind::Font)
    }
}

/// Font system: rustybuzz shaping over fonts loaded through a [`FontProvider`].
struct WasmFonts(RustybuzzFontSystem<ResourceFontSystem<FontProvider>>);

impl WasmFonts {
    fn new(provider: SharedProvider) -> Self {
        Self(RustybuzzFontSystem::new(ResourceFontSystem::new(FontProvider(
            provider,
        ))))
    }
}

impl FontSystem for WasmFonts {
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        self.0.load_font(query)
    }
    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        self.0.shape_text(request)
    }
}

/// Glyph outline source for SVG, font bytes come from the shared resolver and are cached per file.
struct WasmOutlines {
    provider: SharedProvider,
    cache: RefCell<HashMap<String, Option<FontData>>>,
}

impl mathtex_render::GlyphOutlineSource for WasmOutlines {
    fn glyph_run_outlines(
        &self,
        font: &mathtex_ir::FontRef,
        glyphs: &[mathtex_ir::GlyphId],
    ) -> Vec<Option<mathtex_ir::GlyphOutline>> {
        let file = font_filename(&font.name);
        let mut cache = self.cache.borrow_mut();
        let entry = cache.entry(file.clone()).or_insert_with(|| {
            self.provider
                .read(&file, ResourceKind::Font)
                .ok()
                .map(|res| FontData::new(mathtex_ir::FontId(0), file.clone(), res.bytes))
        });
        match entry.as_ref() {
            Some(font_data) => font_data
                .glyph_outlines(glyphs)
                .unwrap_or_else(|_| vec![None; glyphs.len()]),
            None => vec![None; glyphs.len()],
        }
    }
}

/// Wraps a JS `(name, kind) -> Uint8Array | null` resolver function as a `SharedProvider`.
fn js_provider(resolve: js_sys::Function) -> SharedProvider {
    SharedProvider(Rc::new(move |request: &ProviderResourceRequest| {
        let name = request.canonical_name();
        let kind = request.kind;
        let kind_name = format!("{kind:?}");
        let result = resolve
            .call2(
                &JsValue::NULL,
                &JsValue::from_str(&name),
                &JsValue::from_str(&kind_name),
            )
            .map_err(|_| ResourceError::NotFound {
                name: name.clone(),
                kind,
            })?;
        if result.is_null() || result.is_undefined() {
            return Err(ResourceError::NotFound { name, kind });
        }
        let bytes = js_sys::Uint8Array::from(result).to_vec();
        Ok(Resource {
            canonical_name: name,
            kind,
            bytes,
        })
    }))
}

/// Boots LaTeX + amsmath + unicode-math and returns a packaged `.fmt` buffer.
#[wasm_bindgen]
pub fn build_format(resolve: js_sys::Function) -> Result<Vec<u8>, JsValue> {
    let provider = js_provider(resolve);

    let latex = provider
        .read("latex.ltx", ResourceKind::TexInput)
        .map_err(|e| JsValue::from_str(&format!("cannot read latex.ltx: {e:?}")))?
        .bytes;

    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(provider.clone()),
        )
        .with_font_platform(GeneratedFontSystemAdapter::new(WasmFonts::new(
            provider.clone(),
        )));

    if !engine.begin_primary_input("latex.ltx", latex) {
        return Err(JsValue::from_str("failed to begin latex.ltx"));
    }
    engine.run_format_initialization();
    if !engine.begin_primary_input("preamble.tex", PREAMBLE.to_vec()) {
        return Err(JsValue::from_str("failed to begin preamble"));
    }
    engine.run_format_initialization();

    // Pack the hyphenation trie before snapshotting to free the builder scratch (~24 MB).
    engine.finalize_trie();
    let format_bytes = GeneratedFormatCache::from_engine(&engine).to_bytes();
    let font_table = engine.native_font_table();
    Ok(mathtex_engine::generated::pack_packaged_format(
        &format_bytes,
        &font_table,
    ))
}

/// Prefix wrapping each fragment before source spans are rebased to caller input.
const FRAGMENT_PREFIX: &str = r"\hbox{$\displaystyle ";
// The leading newline prevents a user % comment from consuming the closing $ or \end on the next line.
const FRAGMENT_SUFFIX: &str = "\n$}\\csname @@end\\endcsname\\end";

/// Rebases IR source spans from the wrapped fragment frame to caller input.
fn rebase_source_spans_to_input(
    fragment: &mut mathtex_ir::Fragment,
    prefix: u32,
    input_len: u32,
) {
    let hi = prefix.saturating_add(input_len);
    let shift = |span: &mut mathtex_ir::ByteSpan| -> bool {
        if span.start >= prefix && span.end <= hi && span.start <= span.end {
            span.start -= prefix;
            span.end -= prefix;
            true
        } else {
            false
        }
    };
    for node in &mut fragment.nodes {
        let keep = node
            .primary_source
            .as_mut()
            .map_or(true, |range| shift(&mut range.span));
        if !keep {
            node.primary_source = None;
        }
        if let mathtex_ir::LayoutNodeKind::GlyphRun(run) = &mut node.kind {
            for glyph in &mut run.glyphs {
                let keep = glyph.cluster.as_mut().map_or(true, |span| shift(span));
                if !keep {
                    glyph.cluster = None;
                }
            }
        }
    }
    fragment
        .source_map
        .entries
        .retain_mut(|entry| shift(&mut entry.range.span));
}

/// Holds a deserialized format image for repeated fragment renders.
#[wasm_bindgen]
pub struct Session {
    cache: GeneratedFormatCache,
    font_table: Vec<(pe::PortableFontHandle, String, i32)>,
    provider: SharedProvider,
}

#[wasm_bindgen]
impl Session {
    /// Unpacks and deserializes the packaged format.
    #[wasm_bindgen(constructor)]
    pub fn new(packaged: &[u8], resolve: js_sys::Function) -> Result<Session, JsValue> {
        let (format_bytes, font_table) = mathtex_engine::generated::unpack_packaged_format(packaged)
            .ok_or_else(|| JsValue::from_str("not a packaged format for this build target"))?;
        let cache = GeneratedFormatCache::from_bytes(&format_bytes)
            .ok_or_else(|| JsValue::from_str("format image is not for this build target"))?;
        Ok(Session {
            cache,
            font_table,
            provider: js_provider(resolve),
        })
    }

    /// Renders one math fragment to SVG in `\displaystyle` against the cached format.
    pub fn render(&self, latex: &str) -> Result<String, JsValue> {
        let mut engine = self
            .cache
            .instantiate(
                pe::EngineProfile::xetex(),
                GeneratedResourceProvider::new(self.provider.clone()),
            )
            .with_font_platform(GeneratedFontSystemAdapter::new(WasmFonts::new(
                self.provider.clone(),
            )));
        if !engine.restore_native_font_table(&self.font_table) {
            return Err(JsValue::from_str("failed to rebind native fonts"));
        }

        let fragment_src = format!("{FRAGMENT_PREFIX}{latex}{FRAGMENT_SUFFIX}");
        if !engine.begin_primary_input("frag.tex", fragment_src.into_bytes()) {
            return Err(JsValue::from_str("failed to begin fragment"));
        }
        // Sandbox mode: $ breakouts, job control commands, and runaway loops are rejected as errors.
        engine.set_sandbox(true);
        // Enable source tracking after boot to avoid tracking boot allocations.
        engine.set_source_tracking(true);
        engine.begin_fragment_capture();
        let ran = engine.run_main_control();
        engine.end_fragment_capture();
        if !ran {
            // Surfaced errors carry a captured message, bare fatal aborts fall back to a generic message.
            let message = engine
                .last_error_message()
                .map(|m| format!("TeX error: {m}"))
                .unwrap_or_else(|| "engine did not run to completion".to_string());
            return Err(JsValue::from_str(&message));
        }

        let root = engine
            .captured_fragment_root()
            .ok_or_else(|| JsValue::from_str("no captured fragment"))?;
        let mut fragment = mathtex_engine::generated::generated_node_to_fragment(
            &engine,
            root,
            mathtex_ir::FragmentMetadata {
                engine_profile: "xetex".into(),
                format_id: "svg".into(),
                fragment_kind: Default::default(),
            },
        )
        .ok_or_else(|| JsValue::from_str("IR lowering failed"))?;

        // Source offsets index the wrapped fragment string, rebase to the caller input before returning.
        rebase_source_spans_to_input(&mut fragment, FRAGMENT_PREFIX.len() as u32, latex.len() as u32);

        let outlines = WasmOutlines {
            provider: self.provider.clone(),
            cache: RefCell::new(HashMap::new()),
        };
        mathtex_svg::render_with_outlines(&fragment, &outlines)
            .map_err(|e| JsValue::from_str(&format!("svg render failed: {e:?}")))
    }
}

/// Single-shot convenience for unpacking a format and rendering one fragment.
#[wasm_bindgen]
pub fn render(packaged: &[u8], resolve: js_sys::Function, latex: &str) -> Result<String, JsValue> {
    Session::new(packaged, resolve)?.render(latex)
}
