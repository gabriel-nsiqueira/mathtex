use core::fmt;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use mathtex_font::{FontData, FontQuery, FontSystem, MathKernCorner, ShapeFeature, ShapeRequest};
use mathtex_ir::{
    BoxKind, BoxMetrics, ByteSpan, Direction, FontId, FontRef, Fragment, FragmentMetadata, Glue,
    GlyphId, GlyphRun, Kern, LayoutBox, LayoutNode, LayoutNodeKind, Length, NodeId, Point,
    PositionedGlyph, Rect, Rule, Size, SourceRange, SourceRole, Style, Surface,
};

use crate::platform::{LinebreakRequest, Platform};
use crate::resource::{ResourceKind, ResourceProvider, ResourceRequest as EngineResourceRequest};

/// Reexport of the portable engine node handle type.
pub type GeneratedNodeHandle = mathtex_portable_engine_generated::PortableNodeHandle;

/// Enclosing box dimensions used to resolve `null_flag` rule dimensions during shipout.
#[derive(Clone, Copy)]
struct ParentBox {
    vertical: bool,
    width: i32,
    height: i32,
    depth: i32,
}

/// Reexport of the portable engine node kind discriminant.
pub type GeneratedNodeKind = mathtex_portable_engine_generated::PortableNodeKind;

/// Reexport of the portable engine node snapshot type.
pub type GeneratedNodeSnapshot = mathtex_portable_engine_generated::PortableNodeSnapshot;

/// Reexport of the portable engine source span type.
pub type GeneratedSourceSpan = mathtex_portable_engine_generated::PortableSourceSpan;

/// Reexport of the portable engine node source span type.
pub type GeneratedNodeSourceSpan = mathtex_portable_engine_generated::PortableNodeSourceSpan;

/// Reexport of the portable engine resource request record type.
pub type GeneratedResourceRequestRecord =
    mathtex_portable_engine_generated::PortableResourceRequestRecord;

/// Reexport of the portable engine native font handle type.
pub type GeneratedFontHandle = mathtex_portable_engine_generated::PortableFontHandle;

/// Reexport of the portable engine font metrics type.
pub type GeneratedFontMetrics = mathtex_portable_engine_generated::PortableFontMetrics;

/// Reexport of the portable engine shaped glyph descriptor.
pub type GeneratedNativeGlyph = mathtex_portable_engine_generated::PortableNativeGlyph;

/// Reexport of the portable engine shaped text metrics type.
pub type GeneratedNativeTextMetrics = mathtex_portable_engine_generated::PortableNativeTextMetrics;

/// Reexport of the portable engine glyph metrics type.
pub type GeneratedNativeGlyphMetrics =
    mathtex_portable_engine_generated::PortableNativeGlyphMetrics;

/// Reexport of the portable engine math variant type.
pub type GeneratedMathVariant = mathtex_portable_engine_generated::PortableMathVariant;

/// Reexport of the portable engine math assembly part type.
pub type GeneratedMathAssemblyPart =
    mathtex_portable_engine_generated::PortableMathAssemblyPart;

/// Reexport of the portable engine math kern corner enum.
pub type GeneratedMathKernCorner = mathtex_portable_engine_generated::PortableMathKernCorner;

/// Trait alias for the portable engine's native font platform interface.
pub trait GeneratedFontPlatform: mathtex_portable_engine_generated::FontPlatform {}

// Temporary loop detector scaffold, currently disabled.
macro_rules! trace_loop {
    ($tag:literal) => {{
        let _ = $tag;
    }};
}

impl<T> GeneratedFontPlatform for T where T: mathtex_portable_engine_generated::FontPlatform {}

/// Empty font platform for use when native font loading is disabled.
pub type EmptyGeneratedFontPlatform = mathtex_portable_engine_generated::EmptyFontPlatform;

/// Trait alias for the portable engine's platform interface.
pub trait GeneratedPlatform: mathtex_portable_engine_generated::PortablePlatform {}

impl<T> GeneratedPlatform for T where T: mathtex_portable_engine_generated::PortablePlatform {}

/// Deterministic placeholder generated engine platform.
pub type EmptyGeneratedPlatform = mathtex_portable_engine_generated::EmptyPlatform;

/// Reexport of the portable engine clock type.
pub type GeneratedClock = mathtex_portable_engine_generated::PortableClock;

/// Reexport of the portable engine linebreak request type.
pub type GeneratedLinebreakRequest<'a> =
    mathtex_portable_engine_generated::PortableLinebreakRequest<'a>;

/// Adapts a [`Platform`] to the portable engine's `PortablePlatform` interface.
#[derive(Clone, Copy, Debug)]
pub struct GeneratedPlatformAdapter<P> {
    platform: P,
}

impl<P> GeneratedPlatformAdapter<P> {
    /// Wraps `platform` in the adapter.
    #[must_use]
    pub fn new(platform: P) -> Self {
        Self { platform }
    }

    /// Returns a reference to the wrapped platform.
    #[must_use]
    pub fn platform(&self) -> &P {
        &self.platform
    }

    /// Unwraps and returns the inner platform.
    #[must_use]
    pub fn into_platform(self) -> P {
        self.platform
    }
}

impl<P> mathtex_portable_engine_generated::PortablePlatform for GeneratedPlatformAdapter<P>
where
    P: Platform,
{
    fn clock(&mut self) -> GeneratedClock {
        let clock = self.platform.clock();
        GeneratedClock {
            seconds: clock.seconds,
            micros: clock.micros,
        }
    }

    fn linebreak_start(&mut self, request: GeneratedLinebreakRequest<'_>) {
        self.platform.linebreak_start(LinebreakRequest {
            font: request.font,
            locale: request.locale,
            text: request.text,
        });
    }

    fn linebreak_next(&mut self) -> Option<i32> {
        self.platform.linebreak_next()
    }
}

/// Adapter that resolves XeTeX native font requests through a [`FontSystem`].
#[derive(Clone, Debug)]
pub struct GeneratedFontSystemAdapter<F> {
    fonts: F,
    next_handle: GeneratedFontHandle,
    loaded_fonts: BTreeMap<GeneratedFontHandle, LoadedGeneratedFont>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LoadedGeneratedFont {
    data: FontData,
    size: Length,
    /// XeTeX `\font` spec the instance was loaded from (`[file]:features`).
    spec: String,
    /// Forced OpenType script tag from `script=` in the spec for feature lookup.
    script: Option<[u8; 4]>,
    /// OpenType features from the spec (e.g. `ssty=1` selects script size glyph variants).
    features: Vec<ShapeFeature>,
}

impl<F> GeneratedFontSystemAdapter<F> {
    /// Constructs the adapter wrapping `fonts`.
    #[must_use]
    pub fn new(fonts: F) -> Self {
        Self {
            fonts,
            next_handle: 1,
            loaded_fonts: BTreeMap::new(),
        }
    }

    /// Returns a reference to the wrapped font system.
    #[must_use]
    pub fn font_system(&self) -> &F {
        &self.fonts
    }

    /// Unwraps and returns the inner font system.
    #[must_use]
    pub fn into_font_system(self) -> F {
        self.fonts
    }

    /// Returns the font data for a loaded handle, or `None` if not found.
    #[must_use]
    pub fn loaded_font(&self, handle: GeneratedFontHandle) -> Option<&FontData> {
        self.loaded_fonts.get(&handle).map(|font| &font.data)
    }
}

impl<F> mathtex_portable_engine_generated::FontPlatform for GeneratedFontSystemAdapter<F>
where
    F: FontSystem,
{
    fn resolve_font_handle(&mut self, name: &[i32], size: i32) -> Option<GeneratedFontHandle> {
        trace_loop!("resolve_font_handle");
        let spec = unicode_scalars_to_string(name);
        let (script, features) = parse_spec_shaping(&spec);
        let font = self
            .fonts
            .load_font(&FontQuery {
                family: spec.clone(),
                size: Length::from_scaled_points(size),
                math: true,
            })
            .ok()?;
        let next_handle = self.next_handle.checked_add(1)?;
        let handle = self.next_handle;
        self.next_handle = next_handle;
        self.loaded_fonts.insert(
            handle,
            LoadedGeneratedFont {
                data: font,
                size: Length::from_scaled_points(size),
                spec,
                script,
                features,
            },
        );
        Some(handle)
    }

    fn release_font_handle(&mut self, font: GeneratedFontHandle, _type_flag: i32) {
        self.loaded_fonts.remove(&font);
    }

    fn font_table(&self) -> Vec<(GeneratedFontHandle, String, i32)> {
        self.loaded_fonts
            .iter()
            .map(|(&handle, font)| (handle, font.spec.clone(), font.size.0))
            .collect()
    }

    fn restore_font_table(&mut self, table: &[(GeneratedFontHandle, String, i32)]) -> bool {
        let mut ok = true;
        for (handle, spec, size) in table {
            let size = Length::from_scaled_points(*size);
            match self.fonts.load_font(&FontQuery {
                family: spec.clone(),
                size,
                math: true,
            }) {
                Ok(data) => {
                    let (script, features) = parse_spec_shaping(spec);
                    self.loaded_fonts.insert(
                        *handle,
                        LoadedGeneratedFont {
                            data,
                            size,
                            spec: spec.clone(),
                            script,
                            features,
                        },
                    );
                    self.next_handle = self.next_handle.max(handle.saturating_add(1));
                }
                Err(_) => ok = false,
            }
        }
        ok
    }

    fn font_metrics(&mut self, font: GeneratedFontHandle) -> GeneratedFontMetrics {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.metrics(font.size).ok())
            .map(generated_font_metrics)
            .unwrap_or_default()
    }

    fn opentype_font_metrics(&mut self, font: GeneratedFontHandle) -> GeneratedFontMetrics {
        self.font_metrics(font)
    }

    fn is_opentype_math_font(&mut self, font: GeneratedFontHandle) -> bool {
        trace_loop!("is_opentype_math_font");
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.has_opentype_math().ok())
            .unwrap_or(false)
    }

    fn using_opentype(&mut self, font: GeneratedFontHandle) -> bool {
        // Fonts loaded through this adapter are shaped by rustybuzz as OpenType fonts.
        self.loaded_fonts.contains_key(&font)
    }

    fn math_symbol_parameter(&mut self, font: GeneratedFontHandle, parameter: i32) -> i32 {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.math_symbol_parameter(parameter, font.size).ok())
            .unwrap_or(0)
    }

    fn math_extension_parameter(&mut self, font: GeneratedFontHandle, parameter: i32) -> i32 {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| {
                font.data
                    .math_extension_parameter(parameter, font.size)
                    .ok()
            })
            .unwrap_or(0)
    }

    fn opentype_math_constant(&mut self, font: GeneratedFontHandle, constant: i32) -> i32 {
        trace_loop!("opentype_math_constant");
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.opentype_math_constant(constant, font.size).ok())
            .unwrap_or(0)
    }

    fn opentype_math_accent_position(&mut self, font: GeneratedFontHandle, glyph: i32) -> i32 {
        let Ok(glyph) = u32::try_from(glyph) else {
            return 0;
        };
        self.loaded_fonts
            .get(&font)
            .and_then(|font| {
                font.data
                    .opentype_math_accent_position(GlyphId(glyph), font.size)
                    .ok()
            })
            .unwrap_or(0)
    }

    fn math_glyph_italic_correction(&mut self, font: GeneratedFontHandle, glyph: i32) -> i32 {
        trace_loop!("math_glyph_italic_correction");
        let Ok(glyph) = u32::try_from(glyph) else {
            return 0;
        };
        self.loaded_fonts
            .get(&font)
            .and_then(|font| {
                font.data
                    .math_italic_correction(GlyphId(glyph), font.size)
                    .ok()
            })
            .unwrap_or(0)
    }

    fn math_glyph_variant(
        &mut self,
        font: GeneratedFontHandle,
        glyph: i32,
        index: u16,
        horizontal: bool,
    ) -> Option<GeneratedMathVariant> {
        trace_loop!("math_glyph_variant");
        let glyph = u32::try_from(glyph).ok()?;
        let loaded = self.loaded_fonts.get(&font)?;
        let (variant_glyph, advance) = loaded
            .data
            .math_variant(GlyphId(glyph), index, horizontal, loaded.size)
            .ok()??;
        Some(GeneratedMathVariant {
            glyph: variant_glyph.min(i32::MAX as u32) as i32,
            advance,
        })
    }

    fn math_glyph_assembly(
        &mut self,
        font: GeneratedFontHandle,
        glyph: i32,
        horizontal: bool,
    ) -> Vec<GeneratedMathAssemblyPart> {
        trace_loop!("math_glyph_assembly");
        let Ok(glyph) = u32::try_from(glyph) else {
            return Vec::new();
        };
        let Some(loaded) = self.loaded_fonts.get(&font) else {
            return Vec::new();
        };
        let Ok(parts) = loaded
            .data
            .math_assembly(GlyphId(glyph), horizontal, loaded.size)
        else {
            return Vec::new();
        };
        parts
            .into_iter()
            .map(|part| GeneratedMathAssemblyPart {
                glyph: part.glyph.min(i32::MAX as u32) as i32,
                start_connector: part.start_connector,
                end_connector: part.end_connector,
                full_advance: part.full_advance,
                extender: part.extender,
            })
            .collect()
    }

    fn math_min_connector_overlap(&mut self, font: GeneratedFontHandle) -> i32 {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.math_min_connector_overlap(font.size).ok())
            .unwrap_or(0)
    }

    fn math_kern_at(
        &mut self,
        font: GeneratedFontHandle,
        glyph: i32,
        corner: GeneratedMathKernCorner,
        correction_height: i32,
    ) -> i32 {
        trace_loop!("math_kern_at");
        let Ok(glyph) = u32::try_from(glyph) else {
            return 0;
        };
        let corner = match corner {
            GeneratedMathKernCorner::TopRight => MathKernCorner::TopRight,
            GeneratedMathKernCorner::TopLeft => MathKernCorner::TopLeft,
            GeneratedMathKernCorner::BottomRight => MathKernCorner::BottomRight,
            GeneratedMathKernCorner::BottomLeft => MathKernCorner::BottomLeft,
        };
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.math_kern_at(GlyphId(glyph), corner, correction_height).ok())
            .unwrap_or(0)
    }

    fn math_points_to_units(&mut self, font: GeneratedFontHandle, points: f32) -> f32 {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.points_to_units(points, font.size).ok())
            .unwrap_or(0.0)
    }

    fn math_units_to_scaled(&mut self, font: GeneratedFontHandle, units: i32) -> i32 {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.units_to_scaled(units, font.size).ok())
            .unwrap_or(0)
    }

    fn math_point_size(&mut self, font: GeneratedFontHandle) -> f32 {
        self.loaded_fonts
            .get(&font)
            .map(|font| (f64::from(font.size.0) / 65536.0) as f32)
            .unwrap_or(0.0)
    }

    fn map_char_to_glyph(&mut self, font: GeneratedFontHandle, codepoint: i32) -> i32 {
        trace_loop!("map_char_to_glyph");
        let Some(codepoint) = u32::try_from(codepoint).ok().and_then(char::from_u32) else {
            return 0;
        };
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.glyph_index(codepoint).ok().flatten())
            .map(|glyph| glyph.0.min(i32::MAX as u32) as i32)
            .unwrap_or(0)
    }

    fn map_glyph_to_index(&mut self, font: GeneratedFontHandle, name: &str) -> i32 {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| font.data.glyph_index_by_name(name).ok().flatten())
            .map(|glyph| glyph.0.min(i32::MAX as u32) as i32)
            .unwrap_or(0)
    }

    fn ot_font_get(
        &mut self,
        font: GeneratedFontHandle,
        what: i32,
        param1: i32,
        param2: i32,
        param3: i32,
    ) -> i32 {
        let Some(loaded) = self.loaded_fonts.get(&font) else {
            return 0;
        };
        let data = &loaded.data;
        // `what` is the XeTeX_ext OpenType layout selector, params are script and language tags or indices.
        let result = match what {
            1 => data.ot_glyph_count(),                                 // XeTeX_count_glyphs
            16 => data.ot_script_count(),                               // XeTeX_OT_count_scripts
            17 => data.ot_language_count(param1 as u32),                // XeTeX_OT_count_languages
            18 => data.ot_feature_count(param1 as u32, param2 as u32),  // XeTeX_OT_count_features
            19 => data.ot_script_tag(param1 as u32),                    // XeTeX_OT_script_code
            20 => data.ot_language_tag(param1 as u32, param2 as u32),   // XeTeX_OT_language_code
            21 => data.ot_feature_tag(param1 as u32, param2 as u32, param3 as u32), // XeTeX_OT_feature_code
            _ => return 0,
        };
        result.map_or(0, |value| value.min(i32::MAX as u32) as i32)
    }

    fn font_spec(&self, font: GeneratedFontHandle) -> Option<String> {
        self.loaded_fonts.get(&font).map(|loaded| loaded.spec.clone())
    }

    fn shape_native_text(
        &mut self,
        font: GeneratedFontHandle,
        text: &[u16],
        _use_glyph_metrics: bool,
    ) -> GeneratedNativeTextMetrics {
        let Some(loaded) = self.loaded_fonts.get(&font) else {
            return GeneratedNativeTextMetrics::default();
        };
        let text = utf16_to_string(text);
        let metrics = loaded.data.metrics(loaded.size).ok();
        let Ok(shaped) = self.fonts.shape_text(&ShapeRequest {
            font: loaded.data.id,
            text: text.as_str(),
            direction: Direction::LeftToRight,
            source: Some(ByteSpan {
                start: 0,
                end: text.len().min(u32::MAX as usize) as u32,
            }),
            script: loaded.script,
            features: loaded.features.clone(),
        }) else {
            return GeneratedNativeTextMetrics {
                height: metrics.map_or(0, |metrics| metrics.ascent),
                depth: metrics.map_or(0, |metrics| -metrics.descent),
                ..GeneratedNativeTextMetrics::default()
            };
        };

        let mut cursor = 0i32;
        let glyphs = shaped
            .glyphs
            .iter()
            .map(|glyph| {
                let native = GeneratedNativeGlyph {
                    glyph_id: glyph_id_u16(glyph.glyph_id),
                    x: cursor.saturating_add(glyph.offset.x.0),
                    y: glyph.offset.y.0,
                    advance: glyph.advance.x.0,
                    cluster_start: glyph.cluster.map_or(0, |span| span.start),
                    cluster_end: glyph.cluster.map_or(0, |span| span.end),
                    // Source spans are filled later from tracked character identifiers.
                    src_start: 0,
                    src_end: 0,
                };
                cursor = cursor.saturating_add(glyph.advance.x.0);
                native
            })
            .collect();

        GeneratedNativeTextMetrics {
            width: cursor,
            height: metrics.map_or(0, |metrics| metrics.ascent),
            depth: metrics.map_or(0, |metrics| -metrics.descent),
            glyphs,
        }
    }

    fn measure_native_glyph(
        &mut self,
        font: GeneratedFontHandle,
        glyph: u16,
        _use_glyph_metrics: bool,
    ) -> GeneratedNativeGlyphMetrics {
        self.loaded_fonts
            .get(&font)
            .and_then(|font| {
                font.data
                    .glyph_metrics(GlyphId(u32::from(glyph)), font.size)
                    .ok()
            })
            .map(|metrics| GeneratedNativeGlyphMetrics {
                width: metrics.width,
                height: metrics.height,
                depth: metrics.depth,
            })
            .unwrap_or_default()
    }
}

fn generated_font_metrics(metrics: mathtex_font::FontMetrics) -> GeneratedFontMetrics {
    GeneratedFontMetrics {
        ascent: metrics.ascent,
        descent: metrics.descent,
        xheight: metrics.xheight,
        capheight: metrics.capheight,
        slant: metrics.slant,
    }
}

/// Parses script tag and features from a XeTeX `\font` spec.
fn parse_spec_shaping(spec: &str) -> (Option<[u8; 4]>, Vec<ShapeFeature>) {
    let Some((_, feature_str)) = spec.split_once(':') else {
        return (None, Vec::new());
    };
    let mut script = None;
    let mut features = Vec::new();
    for part in feature_str.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(value) = part.strip_prefix("script=") {
            script = Some(ot_tag_bytes(value));
        } else if part.strip_prefix("language=").is_some() {
            // Language selection is not yet threaded into shaping.
        } else if let Some(rest) = part.strip_prefix('+') {
            // XeTeX increments nonnegative `+feat` params so `+ssty=0` selects the first alternate.
            let (tag, param) = rest.split_once('=').map_or((rest, 0i64), |(tag, value)| {
                (tag, value.trim().parse::<i64>().unwrap_or(0))
            });
            let value = if param >= 0 { param + 1 } else { param };
            features.push(ShapeFeature {
                tag: ot_tag_bytes(tag),
                value: value as u32,
            });
        } else if let Some(rest) = part.strip_prefix('-') {
            features.push(ShapeFeature {
                tag: ot_tag_bytes(rest),
                value: 0,
            });
        } else if let Some((tag, value)) = part.split_once('=') {
            features.push(ShapeFeature {
                tag: ot_tag_bytes(tag),
                value: value.trim().parse().unwrap_or(1),
            });
        }
    }
    (script, features)
}

/// Pad/truncate an OpenType tag string to 4 space padded bytes (HarfBuzz rules).
fn ot_tag_bytes(tag: &str) -> [u8; 4] {
    let bytes = tag.trim().as_bytes();
    [
        bytes.first().copied().unwrap_or(b' '),
        bytes.get(1).copied().unwrap_or(b' '),
        bytes.get(2).copied().unwrap_or(b' '),
        bytes.get(3).copied().unwrap_or(b' '),
    ]
}

fn unicode_scalars_to_string(name: &[i32]) -> String {
    name.iter()
        .map(|codepoint| {
            u32::try_from(*codepoint)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER)
        })
        .collect()
}

fn utf16_to_string(text: &[u16]) -> String {
    char::decode_utf16(text.iter().copied())
        .map(|codepoint| codepoint.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

fn glyph_id_u16(glyph: GlyphId) -> u16 {
    u16::try_from(glyph.0).unwrap_or(0)
}

/// Layout capture from stripped generated engine output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeneratedLayoutCapture {
    /// Number of times a page was built and sent to output.
    pub page_builds: usize,
    /// Number of completed shipout calls.
    pub shipouts: usize,
    /// Number of `\special` outputs recorded.
    pub special_outputs: usize,
    /// Number of picture file loads.
    pub picture_loads: usize,
    /// Number of source special whatsits emitted.
    pub source_specials: usize,
    /// Number of `\write` whatsit diagnostics recorded.
    pub write_whatsit_diagnostics: usize,
    /// Number of pdf extension calls recorded.
    pub pdf_extensions: usize,
    /// Total bytes written to the engine transcript.
    pub transcript_bytes: usize,
    /// Number of times the page top was pruned.
    pub page_top_prunes: usize,
    /// Node handle of the most recent shipped out box, if any.
    pub last_shipout_box: Option<GeneratedNodeHandle>,
    /// Node handle of the captured fragment root, if any.
    pub captured_fragment_root: Option<GeneratedNodeHandle>,
    /// Final abort status code from the engine, if it aborted.
    pub last_abort_status: Option<i32>,
}

impl GeneratedLayoutCapture {
    /// Reads capture statistics directly from a running engine.
    #[must_use]
    pub fn from_engine(engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>) -> Self {
        Self {
            page_builds: engine.stripped_page_build_count(),
            shipouts: engine.stripped_shipout_count(),
            special_outputs: engine.stripped_special_output_count(),
            picture_loads: engine.stripped_picture_load_count(),
            source_specials: engine.stripped_source_special_count(),
            write_whatsit_diagnostics: engine.stripped_write_whatsit_diagnostic_count(),
            pdf_extensions: engine.stripped_pdf_extension_count(),
            transcript_bytes: engine.transcript_bytes().len(),
            page_top_prunes: engine.stripped_page_top_prune_count(),
            last_shipout_box: engine.last_stripped_shipout_box(),
            captured_fragment_root: engine.captured_fragment_root(),
            last_abort_status: engine.last_abort_status(),
        }
    }

    /// Snapshots the last shipped out box, or `None` if none was shipped.
    #[must_use]
    pub fn last_shipout_snapshot(
        &self,
        engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>,
    ) -> Option<GeneratedNodeSnapshot> {
        engine.snapshot_node(self.last_shipout_box?)
    }

    /// Converts the captured output to an IR `Fragment`.
    #[must_use]
    pub fn to_fragment(
        &self,
        engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>,
        metadata: FragmentMetadata,
    ) -> Option<Fragment> {
        let root = self.captured_fragment_root.or(self.last_shipout_box)?;
        let mut builder = GeneratedIrBuilder::new(engine, metadata);
        let root_node = builder.emit_node(root, Point::default(), None)?;
        builder.finish_with_root(root_node)
    }

    /// Source provenance parameters are unused because this path does not thread source spans yet.
    #[must_use]
    pub fn to_fragment_with_source(
        &self,
        engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>,
        metadata: FragmentMetadata,
        _source_name: impl Into<String>,
        _source_span: ByteSpan,
    ) -> Option<Fragment> {
        let root = self.captured_fragment_root.or(self.last_shipout_box)?;
        let mut builder = GeneratedIrBuilder::new(engine, metadata);
        let root_node = builder.emit_node(root, Point::default(), None)?;
        builder.finish_with_root(root_node)
    }

    /// Converts captured output to an IR `Fragment` using source spans recorded in the engine.
    #[must_use]
    pub fn to_fragment_with_recorded_source(
        &self,
        engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>,
        metadata: FragmentMetadata,
    ) -> Option<Fragment> {
        let root = self.captured_fragment_root.or(self.last_shipout_box)?;
        let mut builder = GeneratedIrBuilder::new(engine, metadata);
        let root_node = builder.emit_node(root, Point::default(), None)?;
        builder.finish_with_root(root_node)
    }
}

/// Converts a single engine node and its children to an IR `Fragment`.
#[must_use]
pub fn generated_node_to_fragment(
    engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>,
    root: GeneratedNodeHandle,
    metadata: FragmentMetadata,
) -> Option<Fragment> {
    let mut builder = GeneratedIrBuilder::new(engine, metadata);
    let root_node = builder.emit_node(root, Point::default(), None)?;
    builder.finish_with_root(root_node)
}

struct GeneratedIrBuilder<'a, 'resources> {
    engine: &'a mathtex_portable_engine_generated::PortableTexEngine<'resources>,
    fragment: Fragment,
    next_node: u32,
    visits_remaining: usize,
}

impl<'a, 'resources> GeneratedIrBuilder<'a, 'resources> {
    fn new(
        engine: &'a mathtex_portable_engine_generated::PortableTexEngine<'resources>,
        metadata: FragmentMetadata,
    ) -> Self {
        Self {
            engine,
            fragment: Fragment {
                metadata,
                ..Fragment::default()
            },
            next_node: 0,
            visits_remaining: 16_384,
        }
    }

    fn finish_with_root(mut self, root: NodeId) -> Option<Fragment> {
        let root_node = self.fragment.node(root)?;
        if let LayoutNodeKind::Box(layout_box) = &root_node.kind {
            self.fragment.surface = Surface {
                width: layout_box.metrics.width,
                height: layout_box.metrics.total_height(),
                baseline: layout_box.metrics.height,
            };
        }
        Some(self.fragment)
    }

    fn emit_node(
        &mut self,
        handle: GeneratedNodeHandle,
        origin: Point,
        parent: Option<ParentBox>,
    ) -> Option<NodeId> {
        self.consume_visit()?;
        let snapshot = self.engine.snapshot_node(handle)?;
        match snapshot.kind {
            GeneratedNodeKind::HorizontalBox
            | GeneratedNodeKind::VerticalBox
            | GeneratedNodeKind::UnsetBox => self.emit_box(snapshot, origin),
            GeneratedNodeKind::Rule => {
                // Running rule dimensions inherit from the enclosing box.
                const NULL_FLAG: i32 = -1_073_741_824; // -2^30
                let mut width = snapshot.width;
                let mut height = snapshot.height;
                let mut depth = snapshot.depth;
                if let Some(p) = parent {
                    if p.vertical {
                        if width == NULL_FLAG {
                            width = p.width;
                        }
                    } else {
                        if height == NULL_FLAG {
                            height = p.height;
                        }
                        if depth == NULL_FLAG {
                            depth = p.depth;
                        }
                    }
                }
                Some(self.emit(
                    &snapshot,
                    origin,
                    LayoutNodeKind::Rule(Rule {
                        size: Size {
                            width: Length::from_scaled_points(width),
                            height: Length::from_scaled_points(height + depth),
                        },
                        color: Default::default(),
                    }),
                ))
            }
            GeneratedNodeKind::Glue => Some(self.emit(
                &snapshot,
                origin,
                LayoutNodeKind::Glue(Glue {
                    amount: Length::from_scaled_points(snapshot.width),
                }),
            )),
            GeneratedNodeKind::Kern => Some(self.emit(
                &snapshot,
                origin,
                LayoutNodeKind::Kern(Kern {
                    amount: Length::from_scaled_points(snapshot.width),
                }),
            )),
            GeneratedNodeKind::Character => {
                let cluster: Option<ByteSpan> = None;
                Some(self.emit(
                    &snapshot,
                    origin,
                    LayoutNodeKind::GlyphRun(GlyphRun {
                        font: FontRef {
                            id: FontId(snapshot.font as u32),
                            name: self
                                .engine
                                .native_font_spec(snapshot.font)
                                .or_else(|| self.engine.font_name(snapshot.font))
                                .unwrap_or_else(|| format!("generated-font-{}", snapshot.font)),
                            size: Length::from_scaled_points(self.engine.font_at_size(snapshot.font)),
                            features: Vec::new(),
                        },
                        direction: Direction::LeftToRight,
                        script: None,
                        language: None,
                        glyphs: Vec::from([PositionedGlyph {
                            glyph_id: GlyphId(snapshot.character as u32),
                            offset: Point::default(),
                            advance: Point {
                                x: Length::from_scaled_points(snapshot.width),
                                y: Length::ZERO,
                            },
                            cluster,
                        }]),
                    }),
                ))
            }
            GeneratedNodeKind::NativeWord | GeneratedNodeKind::NativeGlyph => {
                Some(self.emit_native_glyph_run(snapshot, origin))
            }
            GeneratedNodeKind::OutputWhatsit => None,
            _ => None,
        }
    }

    fn emit_native_glyph_run(&mut self, snapshot: GeneratedNodeSnapshot, origin: Point) -> NodeId {
        let cluster: Option<ByteSpan> = None;
        let glyphs = if snapshot.native_glyphs.is_empty() {
            Vec::from([PositionedGlyph {
                glyph_id: GlyphId(snapshot.character as u32),
                offset: Point::default(),
                advance: Point {
                    x: Length::from_scaled_points(snapshot.width),
                    y: Length::ZERO,
                },
                cluster,
            }])
        } else {
            snapshot
                .native_glyphs
                .iter()
                .map(|glyph| PositionedGlyph {
                    glyph_id: GlyphId(u32::from(glyph.glyph_id)),
                    offset: Point {
                        x: Length::from_scaled_points(glyph.x),
                        y: Length::from_scaled_points(glyph.y),
                    },
                    advance: Point {
                        x: Length::from_scaled_points(glyph.advance),
                        y: Length::ZERO,
                    },
                    // Glyph source spans are set by `src_resolve_native_glyphs`, 0/0 means unmapped.
                    cluster: if glyph.src_end > glyph.src_start {
                        Some(ByteSpan {
                            start: glyph.src_start,
                            end: glyph.src_end,
                        })
                    } else {
                        None
                    },
                })
                .collect()
        };

        self.emit(
            &snapshot,
            origin,
            LayoutNodeKind::GlyphRun(GlyphRun {
                font: FontRef {
                    id: FontId(snapshot.font as u32),
                    name: self
                        .engine
                        .native_font_spec(snapshot.font)
                        .or_else(|| self.engine.font_name(snapshot.font))
                        .unwrap_or_else(|| format!("generated-font-{}", snapshot.font)),
                    size: Length::from_scaled_points(self.engine.font_at_size(snapshot.font)),
                    features: Vec::new(),
                },
                direction: Direction::LeftToRight,
                script: None,
                language: None,
                glyphs,
            }),
        )
    }

    fn emit_box(&mut self, snapshot: GeneratedNodeSnapshot, origin: Point) -> Option<NodeId> {
        let children = self.emit_children(
            snapshot.list,
            snapshot.kind,
            snapshot.width,
            snapshot.height,
            snapshot.depth,
            snapshot.glue_set,
            snapshot.glue_sign,
            snapshot.glue_order,
        );
        let kind = match snapshot.kind {
            GeneratedNodeKind::VerticalBox => BoxKind::Vertical,
            GeneratedNodeKind::HorizontalBox => BoxKind::Horizontal,
            GeneratedNodeKind::UnsetBox => BoxKind::Math,
            _ => BoxKind::Math,
        };
        Some(self.emit_with_primary_source(
            &snapshot,
            origin,
            LayoutNodeKind::Box(LayoutBox {
                kind,
                metrics: BoxMetrics {
                    width: Length::from_scaled_points(snapshot.width),
                    height: Length::from_scaled_points(snapshot.height),
                    depth: Length::from_scaled_points(snapshot.depth),
                    shift: Length::from_scaled_points(snapshot.shift),
                },
                children,
            }),
            None,
        ))
    }

    fn emit_children(
        &mut self,
        first: Option<GeneratedNodeHandle>,
        parent_kind: GeneratedNodeKind,
        parent_width: i32,
        parent_height: i32,
        parent_depth: i32,
        glue_set: f64,
        glue_sign: i32,
        glue_order: i32,
    ) -> Vec<NodeId> {
        // Replicates TeX `hlist_out` and `vlist_out` cursor advancement with shifts folded into `origin`.
        let mut children = Vec::new();
        let vertical = matches!(parent_kind, GeneratedNodeKind::VerticalBox);
        let mut cursor = first;
        let mut cur_h = 0i32;
        let mut cur_v = if vertical { -parent_height } else { 0 };
        while let Some(handle) = cursor {
            let Some(snapshot) = self.engine.snapshot_node(handle) else {
                break;
            };
            let true_box = matches!(
                snapshot.kind,
                GeneratedNodeKind::HorizontalBox
                    | GeneratedNodeKind::VerticalBox
                    | GeneratedNodeKind::UnsetBox
            );
            let has_extent = true_box
                || matches!(
                    snapshot.kind,
                    GeneratedNodeKind::Rule
                        | GeneratedNodeKind::Character
                        | GeneratedNodeKind::NativeWord
                        | GeneratedNodeKind::NativeGlyph
                        | GeneratedNodeKind::Ligature
                );
            let is_spacing = matches!(
                snapshot.kind,
                GeneratedNodeKind::Kern | GeneratedNodeKind::Glue
            );
            // Glue advances by its set width after `glue_set`, other nodes advance by their own width.
            let advance = if matches!(snapshot.kind, GeneratedNodeKind::Glue) {
                let natural = snapshot.width;
                if glue_sign == 1 && snapshot.glue_stretch_order == glue_order {
                    natural + (glue_set * f64::from(snapshot.glue_stretch)).round() as i32
                } else if glue_sign == 2 && snapshot.glue_shrink_order == glue_order {
                    natural - (glue_set * f64::from(snapshot.glue_shrink)).round() as i32
                } else {
                    natural
                }
            } else {
                snapshot.width
            };
            let origin = if vertical {
                if is_spacing {
                    let origin = Point {
                        x: Length::ZERO,
                        y: Length::from_scaled_points(cur_v),
                    };
                    cur_v += advance;
                    origin
                } else if has_extent {
                    cur_v += snapshot.height;
                    let origin = Point {
                        x: Length::from_scaled_points(if true_box { snapshot.shift } else { 0 }),
                        y: Length::from_scaled_points(cur_v),
                    };
                    cur_v += snapshot.depth;
                    origin
                } else {
                    // Penalty / mark / whatsit: recorded but contributes no extent.
                    Point {
                        x: Length::ZERO,
                        y: Length::from_scaled_points(cur_v),
                    }
                }
            } else {
                let origin = Point {
                    x: Length::from_scaled_points(cur_h),
                    y: Length::from_scaled_points(if true_box { snapshot.shift } else { 0 }),
                };
                cur_h += advance;
                origin
            };
            let parent = ParentBox {
                vertical,
                width: parent_width,
                height: parent_height,
                depth: parent_depth,
            };
            if let Some(child) = self.emit_node(handle, origin, Some(parent)) {
                children.push(child);
            }
            cursor = snapshot.link;
        }
        children
    }

    fn emit(
        &mut self,
        snapshot: &GeneratedNodeSnapshot,
        origin: Point,
        kind: LayoutNodeKind,
    ) -> NodeId {
        self.emit_with_primary_source(snapshot, origin, kind, None)
    }

    fn emit_with_primary_source(
        &mut self,
        snapshot: &GeneratedNodeSnapshot,
        origin: Point,
        kind: LayoutNodeKind,
        _preferred_primary_source: Option<SourceRange>,
    ) -> NodeId {
        let id = NodeId(self.next_node);
        self.next_node += 1;
        // Entries are recorded as Primary so node source lookup works uniformly.
        let primary_source = snapshot.source.as_ref().map(|span| {
            let source = self.fragment.source_map.intern_source(span.name.clone());
            SourceRange {
                source,
                span: ByteSpan {
                    start: span.start,
                    end: span.end,
                },
            }
        });
        if let Some(range) = primary_source {
            self.fragment
                .source_map
                .add_entry(id, range, SourceRole::Primary);
            // Emit enclosing construct spans after the leaf so consumers can choose wider matches.
            for span in self.engine.node_enclosing_spans(snapshot.handle) {
                let source = self.fragment.source_map.intern_source(span.name.clone());
                let enclosing = SourceRange {
                    source,
                    span: ByteSpan {
                        start: span.start,
                        end: span.end,
                    },
                };
                self.fragment
                    .source_map
                    .add_entry(id, enclosing, SourceRole::EnclosingConstruct);
            }
        }
        self.fragment.nodes.push(LayoutNode {
            id,
            origin,
            bounds: Rect {
                origin: Point::default(),
                size: Size {
                    width: Length::from_scaled_points(snapshot.width),
                    height: Length::from_scaled_points(snapshot.height + snapshot.depth),
                },
            },
            primary_source,
            style: Style::default(),
            kind,
        });
        id
    }

    fn consume_visit(&mut self) -> Option<()> {
        if self.visits_remaining == 0 {
            return None;
        }
        self.visits_remaining -= 1;
        Some(())
    }
}

/// A cached snapshot of a generated engine format image for repeated instantiation.
#[derive(Clone)]
pub struct GeneratedFormatCache {
    image: mathtex_portable_engine_generated::PortableFormatImage,
}

impl GeneratedFormatCache {
    /// Returns an empty format cache with no initialized state.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            image: mathtex_portable_engine_generated::PortableFormatImage::empty(),
        }
    }

    /// Initializes a generated engine once and snapshots it as a reusable format cache.
    #[must_use]
    pub fn initialized(profile: mathtex_portable_engine_generated::EngineProfile) -> Self {
        let mut engine = mathtex_portable_engine_generated::PortableTexEngine::from_format(
            profile,
            &mathtex_portable_engine_generated::PortableFormatImage::empty(),
            mathtex_portable_engine_generated::EmptyResourceProvider,
        );
        assert!(
            engine.initialize_format_state(),
            "generated engine base initialization failed"
        );
        Self::from_engine_owned(engine)
    }

    /// Snapshots the engine's current format state into a reusable cache.
    #[must_use]
    pub fn from_engine(engine: &mathtex_portable_engine_generated::PortableTexEngine<'_>) -> Self {
        Self {
            image: engine.snapshot_format(),
        }
    }

    /// Consumes an engine into a format cache by moving its state in place, avoiding a full clone.
    #[must_use]
    pub fn from_engine_owned(
        engine: mathtex_portable_engine_generated::PortableTexEngine<'_>,
    ) -> Self {
        Self {
            image: engine.into_format(),
        }
    }

    /// Returns a reference to the raw format image.
    #[must_use]
    pub fn image(&self) -> &mathtex_portable_engine_generated::PortableFormatImage {
        &self.image
    }

    /// Serializes the format image to a buffer valid for the same build target only.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.image.to_bytes()
    }

    /// Deserializes a format cache from a buffer produced by [`Self::to_bytes`].
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        Some(Self {
            image: mathtex_portable_engine_generated::PortableFormatImage::from_bytes(bytes)?,
        })
    }

    /// Creates a new engine instance from this cached format image.
    #[must_use]
    pub fn instantiate<'resources, R>(
        &self,
        profile: mathtex_portable_engine_generated::EngineProfile,
        resources: R,
    ) -> mathtex_portable_engine_generated::PortableTexEngine<'resources>
    where
        R: mathtex_portable_engine_generated::ResourceProvider + 'resources,
    {
        mathtex_portable_engine_generated::PortableTexEngine::from_format(
            profile,
            &self.image,
            resources,
        )
    }
}

impl Default for GeneratedFormatCache {
    fn default() -> Self {
        Self::empty()
    }
}

/// Magic header for packaged format files: `MTXPKG` followed by version and compression bytes.
const PACKAGED_FORMAT_MAGIC: &[u8; 6] = b"MTXPKG";
const PACKAGED_FORMAT_VERSION: u8 = 2;
const PACKAGED_COMPRESSION_DEFLATE: u8 = 1;
/// DEFLATE level for packaging, only the compressor pays this cost, not the reader.
const PACKAGED_DEFLATE_LEVEL: u8 = 7;

fn encode_packaged_payload(
    format_bytes: &[u8],
    font_table: &[(mathtex_portable_engine_generated::PortableFontHandle, String, i32)],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(format_bytes.len() + 64);
    out.extend_from_slice(&(format_bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(format_bytes);
    out.extend_from_slice(&(font_table.len() as u32).to_le_bytes());
    for (handle, spec, size) in font_table {
        out.extend_from_slice(&(*handle as u64).to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        let spec = spec.as_bytes();
        out.extend_from_slice(&(spec.len() as u32).to_le_bytes());
        out.extend_from_slice(spec);
    }
    out
}

/// Packs format image bytes and a font table into a DEFLATE compressed buffer.
#[must_use]
pub fn pack_packaged_format(
    format_bytes: &[u8],
    font_table: &[(mathtex_portable_engine_generated::PortableFontHandle, String, i32)],
) -> Vec<u8> {
    let payload = encode_packaged_payload(format_bytes, font_table);
    let compressed = miniz_oxide::deflate::compress_to_vec(&payload, PACKAGED_DEFLATE_LEVEL);
    let mut out = Vec::with_capacity(compressed.len() + 16);
    out.extend_from_slice(PACKAGED_FORMAT_MAGIC);
    out.push(PACKAGED_FORMAT_VERSION);
    out.push(PACKAGED_COMPRESSION_DEFLATE);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&compressed);
    out
}

/// Decompresses a buffer from [`pack_packaged_format`] into format bytes and the font table.
#[must_use]
pub fn unpack_packaged_format(
    bytes: &[u8],
) -> Option<(
    Vec<u8>,
    Vec<(mathtex_portable_engine_generated::PortableFontHandle, String, i32)>,
)> {
    fn take<'a>(b: &'a [u8], cursor: &mut usize, n: usize) -> Option<&'a [u8]> {
        let slice = b.get(*cursor..*cursor + n)?;
        *cursor += n;
        Some(slice)
    }
    let mut cursor = 0usize;
    if take(bytes, &mut cursor, 6)? != PACKAGED_FORMAT_MAGIC {
        return None;
    }
    if *bytes.get(cursor)? != PACKAGED_FORMAT_VERSION {
        return None;
    }
    cursor += 1;
    let compression = *bytes.get(cursor)?;
    cursor += 1;
    let payload_len = u64::from_le_bytes(take(bytes, &mut cursor, 8)?.try_into().ok()?) as usize;
    let compressed = bytes.get(cursor..)?;
    let payload = match compression {
        PACKAGED_COMPRESSION_DEFLATE => {
            miniz_oxide::inflate::decompress_to_vec(compressed).ok()?
        }
        0 => compressed.to_vec(),
        _ => return None,
    };
    if payload.len() != payload_len {
        return None;
    }

    let mut cursor = 0usize;
    let format_len = u64::from_le_bytes(take(&payload, &mut cursor, 8)?.try_into().ok()?) as usize;
    let format_bytes = take(&payload, &mut cursor, format_len)?.to_vec();
    let font_count = u32::from_le_bytes(take(&payload, &mut cursor, 4)?.try_into().ok()?) as usize;
    let mut font_table = Vec::with_capacity(font_count);
    for _ in 0..font_count {
        let handle = u64::from_le_bytes(take(&payload, &mut cursor, 8)?.try_into().ok()?) as usize;
        let size = i32::from_le_bytes(take(&payload, &mut cursor, 4)?.try_into().ok()?);
        let spec_len = u32::from_le_bytes(take(&payload, &mut cursor, 4)?.try_into().ok()?) as usize;
        let spec = String::from_utf8(take(&payload, &mut cursor, spec_len)?.to_vec()).ok()?;
        font_table.push((handle, spec, size));
    }
    Some((format_bytes, font_table))
}

impl fmt::Debug for GeneratedFormatCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GeneratedFormatCache")
            .finish_non_exhaustive()
    }
}

/// Adapter that routes generated engine resource requests through the engine [`ResourceProvider`].
#[derive(Clone, Debug)]
pub struct GeneratedResourceProvider<R> {
    inner: R,
}

impl<R> GeneratedResourceProvider<R> {
    /// Wraps `inner` in the resource provider adapter.
    #[must_use]
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Returns a reference to the inner resource provider.
    #[must_use]
    pub fn inner(&self) -> &R {
        &self.inner
    }

    /// Unwraps and returns the inner resource provider.
    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R> mathtex_portable_engine_generated::ResourceProvider for GeneratedResourceProvider<R>
where
    R: ResourceProvider,
{
    fn read(
        &mut self,
        generated_request: mathtex_portable_engine_generated::ResourceRequest<'_>,
    ) -> Option<Vec<u8>> {
        let name = normalized_generated_resource_name(generated_request.name);
        let kind = generated_kind_to_engine_kind(generated_request.kind)?;
        let mut request = if kind == ResourceKind::Asset {
            generated_request.package.map_or_else(
                || EngineResourceRequest::new(name, kind),
                |package| EngineResourceRequest::asset(package, name),
            )
        } else {
            EngineResourceRequest::new(name, kind)
        };
        if let Some(source) = request_source_from_generated(generated_request.source.as_ref()) {
            request = request.with_source(source.name, source.span);
        }
        let resource = read_generated_resource_request(&self.inner, &request).or_else(|| {
            if kind != ResourceKind::Asset {
                return None;
            }
            let mut fallback = EngineResourceRequest::new(name, ResourceKind::TexInput);
            if let Some(source) = request_source_from_generated(generated_request.source.as_ref()) {
                fallback = fallback.with_source(source.name, source.span);
            }
            read_generated_resource_request(&self.inner, &fallback)
        })?;
        Some(resource.bytes)
    }
}

fn read_generated_resource_request<R>(
    provider: &R,
    request: &EngineResourceRequest,
) -> Option<crate::resource::Resource>
where
    R: ResourceProvider,
{
    if let Ok(resource) = provider.read_request(request) {
        return Some(resource);
    }

    for name in generated_resource_candidate_names(request.name.as_str(), request.kind) {
        let mut candidate = request.clone();
        candidate.name = name;
        if let Ok(resource) = provider.read_request(&candidate) {
            return Some(resource);
        }
    }

    None
}

fn generated_resource_candidate_names(name: &str, kind: ResourceKind) -> Vec<String> {
    if resource_name_has_extension(name) {
        return Vec::new();
    }

    let suffixes: &[&str] = match kind {
        ResourceKind::TexInput => &[".tex", ".ltx"],
        ResourceKind::Package => &[".sty"],
        ResourceKind::Class => &[".cls"],
        ResourceKind::FontDefinition => &[".fd"],
        ResourceKind::Font => &[".tfm", ".otf", ".ttf"],
        ResourceKind::Encoding => &[".enc"],
        ResourceKind::Map => &[".map"],
        ResourceKind::PackageSupport
        | ResourceKind::Config
        | ResourceKind::FormatImage
        | ResourceKind::Asset => &[],
    };

    suffixes
        .iter()
        .map(|suffix| {
            let mut candidate = String::with_capacity(name.len() + suffix.len());
            candidate.push_str(name);
            candidate.push_str(suffix);
            candidate
        })
        .collect()
}

fn resource_name_has_extension(name: &str) -> bool {
    let last_separator = name.rfind(['/', '\\']).map_or(0, |index| index + 1);
    name[last_separator..].contains('.')
}

fn normalized_generated_resource_name(mut name: &str) -> &str {
    while let Some(stripped) = name.strip_prefix("./") {
        name = stripped;
    }
    name
}

fn request_source_from_generated(
    source: Option<&mathtex_portable_engine_generated::PortableSourceSpan>,
) -> Option<crate::resource::ResourceRequestSource> {
    let source = source?;
    Some(crate::resource::ResourceRequestSource {
        name: source.name.clone(),
        span: ByteSpan {
            start: source.start,
            end: source.end,
        },
    })
}

fn generated_kind_to_engine_kind(
    kind: mathtex_portable_engine_generated::ResourceKind,
) -> Option<ResourceKind> {
    match kind {
        mathtex_portable_engine_generated::ResourceKind::TexInput => Some(ResourceKind::TexInput),
        mathtex_portable_engine_generated::ResourceKind::Package => Some(ResourceKind::Package),
        mathtex_portable_engine_generated::ResourceKind::Class => Some(ResourceKind::Class),
        mathtex_portable_engine_generated::ResourceKind::FontDefinition => {
            Some(ResourceKind::FontDefinition)
        }
        mathtex_portable_engine_generated::ResourceKind::PackageSupport => {
            Some(ResourceKind::PackageSupport)
        }
        mathtex_portable_engine_generated::ResourceKind::Font => Some(ResourceKind::Font),
        mathtex_portable_engine_generated::ResourceKind::Encoding => Some(ResourceKind::Encoding),
        mathtex_portable_engine_generated::ResourceKind::Map => Some(ResourceKind::Map),
        mathtex_portable_engine_generated::ResourceKind::Config => Some(ResourceKind::Config),
        mathtex_portable_engine_generated::ResourceKind::FormatImage => {
            Some(ResourceKind::FormatImage)
        }
        mathtex_portable_engine_generated::ResourceKind::Asset => Some(ResourceKind::Asset),
        mathtex_portable_engine_generated::ResourceKind::Other(_) => Some(ResourceKind::TexInput),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::InMemoryResourceProvider;

    #[test]
    fn generated_provider_adapter_reads_package_bytes() {
        let mut resources = InMemoryResourceProvider::new().with_resource(
            "amsmath.sty",
            ResourceKind::Package,
            br"\ProvidesPackage{amsmath}",
        );
        resources.insert_request(
            EngineResourceRequest::asset("mhchem", "arrows.dat"),
            b"asset",
        );
        let mut generated = GeneratedResourceProvider::new(resources);

        let bytes = mathtex_portable_engine_generated::ResourceProvider::read(
            &mut generated,
            mathtex_portable_engine_generated::ResourceRequest {
                name: "amsmath.sty",
                kind: mathtex_portable_engine_generated::ResourceKind::Package,
                package: None,
                format: 26,
                mode: "rb",
                source: None,
            },
        )
        .expect("generated provider should read package");

        assert_eq!(bytes, br"\ProvidesPackage{amsmath}");

        let bytes = mathtex_portable_engine_generated::ResourceProvider::read(
            &mut generated,
            mathtex_portable_engine_generated::ResourceRequest {
                name: "arrows.dat",
                kind: mathtex_portable_engine_generated::ResourceKind::Asset,
                package: Some("mhchem"),
                format: 26,
                mode: "rb",
                source: None,
            },
        )
        .expect("generated provider should read package asset");

        assert_eq!(bytes, b"asset");
    }

    #[test]
    fn generated_engine_can_be_constructed_from_cached_format_state() {
        let resources = GeneratedResourceProvider::new(InMemoryResourceProvider::new());
        let format = GeneratedFormatCache::empty();
        let engine = format.instantiate(
            mathtex_portable_engine_generated::EngineProfile::xetex(),
            resources,
        );

        assert_eq!(
            engine.profile(),
            mathtex_portable_engine_generated::EngineProfile::xetex()
        );
        assert_eq!(engine.resource_request_count(), 0);
        assert_eq!(
            GeneratedLayoutCapture::from_engine(&engine),
            GeneratedLayoutCapture::default()
        );
        assert_eq!(
            GeneratedLayoutCapture::from_engine(&engine).last_shipout_snapshot(&engine),
            None
        );
        assert_eq!(
            GeneratedLayoutCapture::from_engine(&engine).to_fragment(
                &engine,
                FragmentMetadata {
                    engine_profile: "xetex".into(),
                    format_id: "empty".into(),
                    fragment_kind: Default::default(),
                },
            ),
            None
        );
        assert_eq!(
            GeneratedLayoutCapture::from_engine(&engine).to_fragment_with_source(
                &engine,
                FragmentMetadata {
                    engine_profile: "xetex".into(),
                    format_id: "empty".into(),
                    fragment_kind: Default::default(),
                },
                "input.tex",
                ByteSpan { start: 0, end: 3 },
            ),
            None
        );
    }

    #[test]
    fn generated_format_cache_can_snapshot_initialized_engine_state() {
        let cached = GeneratedFormatCache::initialized(
            mathtex_portable_engine_generated::EngineProfile::xetex(),
        );
        let second = cached.instantiate(
            mathtex_portable_engine_generated::EngineProfile::xetex(),
            GeneratedResourceProvider::new(InMemoryResourceProvider::new()),
        );

        assert_eq!(
            second.profile(),
            mathtex_portable_engine_generated::EngineProfile::xetex()
        );
        assert_eq!(second.resource_request_count(), 0);
        assert_eq!(
            GeneratedLayoutCapture::from_engine(&second),
            GeneratedLayoutCapture::default()
        );
    }

    #[test]
    fn generated_engine_runs_main_control_for_primary_end_input() {
        let format = GeneratedFormatCache::initialized(
            mathtex_portable_engine_generated::EngineProfile::tex(),
        );
        let mut engine = format.instantiate(
            mathtex_portable_engine_generated::EngineProfile::tex(),
            GeneratedResourceProvider::new(InMemoryResourceProvider::new()),
        );

        assert!(engine.begin_primary_input("input.tex", br"\end".to_vec()));
        engine.run_format_initialization();
        assert_eq!(engine.last_abort_status(), None);
    }

    #[test]
    fn generated_engine_loads_input_through_resource_provider_during_main_control() {
        let resources = InMemoryResourceProvider::new().with_resource(
            "child.tex",
            ResourceKind::TexInput,
            br"\relax",
        );
        let format = GeneratedFormatCache::initialized(
            mathtex_portable_engine_generated::EngineProfile::tex(),
        );
        let mut engine = format.instantiate(
            mathtex_portable_engine_generated::EngineProfile::tex(),
            GeneratedResourceProvider::new(resources),
        );

        assert!(engine.begin_primary_input("input.tex", br"\input child.tex \end".to_vec()));
        engine.run_main_control();

        assert_eq!(engine.last_abort_status(), None);
        assert_eq!(engine.resource_request_count(), 1);
        let request = &engine.resource_request_records()[0];
        assert_eq!(request.name, "child.tex");
        assert_eq!(
            request.kind,
            mathtex_portable_engine_generated::ResourceKind::TexInput
        );
        assert_eq!(request.byte_len, Some(6));
        // Source span tracking was removed, resource requests carry no source.
        assert_eq!(request.source, None);
    }

    #[test]
    fn generated_engine_captures_fragment_root_to_ir_without_shipout() {
        let format = GeneratedFormatCache::initialized(
            mathtex_portable_engine_generated::EngineProfile::tex(),
        );
        let mut engine = format.instantiate(
            mathtex_portable_engine_generated::EngineProfile::tex(),
            GeneratedResourceProvider::new(InMemoryResourceProvider::new()),
        );

        engine.begin_fragment_capture();
        assert!(engine.begin_primary_input(
            "input.tex",
            br"\catcode`{=1 \catcode`}=2 \hbox{\vrule width 1pt height 2pt depth 0pt}\end".to_vec()
        ));
        engine.run_main_control();
        engine.end_fragment_capture();

        let capture = GeneratedLayoutCapture::from_engine(&engine);
        assert_eq!(capture.last_abort_status, None);
        assert!(!String::from_utf8_lossy(engine.transcript_bytes()).contains('!'));
        assert_eq!(capture.shipouts, 0);
        let register = engine
            .captured_fragment_root()
            .expect("fragment root should be captured");
        let snapshot = engine
            .snapshot_node(register)
            .expect("captured root should snapshot");
        assert_eq!(
            snapshot.kind,
            mathtex_portable_engine_generated::PortableNodeKind::HorizontalBox
        );

        let fragment = generated_node_to_fragment(
            &engine,
            register,
            FragmentMetadata {
                engine_profile: "tex".into(),
                format_id: "generated".into(),
                fragment_kind: Default::default(),
            },
        )
        .expect("captured root should convert to IR");

        let root = fragment
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                LayoutNodeKind::Box(layout_box) => Some(layout_box),
                _ => None,
            })
            .expect("fragment should contain a root box");
        assert_eq!(root.kind, BoxKind::Horizontal);
        assert_eq!(root.children.len(), 1);
        assert!(fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::Rule(_))));
    }

    #[test]
    fn generated_engine_loads_tfm_and_emits_glyph_ir_without_shipout() {
        let resources = InMemoryResourceProvider::new().with_resource(
            "cmr10",
            ResourceKind::Font,
            include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm").to_vec(),
        );
        let format = GeneratedFormatCache::initialized(
            mathtex_portable_engine_generated::EngineProfile::tex(),
        );
        let mut engine = format.instantiate(
            mathtex_portable_engine_generated::EngineProfile::tex(),
            GeneratedResourceProvider::new(resources),
        );

        engine.begin_fragment_capture();
        assert!(engine.begin_primary_input(
            "input.tex",
            br"\catcode`{=1 \catcode`}=2 \font\tenrm=cmr10 \hbox{\tenrm A}\end".to_vec()
        ));
        engine.run_main_control();
        engine.end_fragment_capture();

        let capture = GeneratedLayoutCapture::from_engine(&engine);
        assert_eq!(capture.last_abort_status, None);
        assert!(!String::from_utf8_lossy(engine.transcript_bytes()).contains('!'));
        assert_eq!(capture.shipouts, 0);
        assert_eq!(engine.resource_request_count(), 1);
        let request = &engine.resource_request_records()[0];
        assert_eq!(request.name, "cmr10");
        assert_eq!(
            request.kind,
            mathtex_portable_engine_generated::ResourceKind::Font
        );
        assert!(request.byte_len.is_some());

        let fragment = generated_node_to_fragment(
            &engine,
            engine
                .captured_fragment_root()
                .expect("fragment root should be captured"),
            FragmentMetadata {
                engine_profile: "tex".into(),
                format_id: "generated".into(),
                fragment_kind: Default::default(),
            },
        )
        .expect("font-backed captured root should convert to IR");

        let glyph_run = fragment
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                LayoutNodeKind::GlyphRun(run) => Some(run),
                _ => None,
            })
            .expect("font-backed hbox should emit a glyph run");
        assert_eq!(glyph_run.glyphs.len(), 1);
        assert_eq!(glyph_run.glyphs[0].glyph_id, GlyphId(u32::from(b'A')));
        assert!(glyph_run.glyphs[0].advance.x.0 > 0);
    }
}
