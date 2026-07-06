//! Font loading, shaping, metrics, and OpenType math access for mathtex.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use core::cell::RefCell;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use mathtex_ir::{
    ByteSpan, Direction, FontId, GlyphId, GlyphOutline, Length, OutlineCommand, Point,
    PositionedGlyph,
};

// Re exported so applications share one parser version and faces interoperate with FontData.
pub use rustybuzz;
pub use ttf_parser;

/// Font loading and shaping boundary used by engine profiles.
pub trait FontSystem {
    /// Load font data that matches the query.
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError>;

    /// Shape text into positioned glyphs.
    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError>;
}

/// Font loading half of a font system.
pub trait FontLoader {
    /// Load font data that matches the query.
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError>;
}

/// Text shaping half of a font system, decoupled from any concrete shaper crate.
pub trait TextShaper {
    /// Shape text into positioned glyphs.
    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError>;
}

impl<T> FontSystem for &T
where
    T: FontSystem,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        (*self).load_font(query)
    }

    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        (*self).shape_text(request)
    }
}

impl<T> FontLoader for &T
where
    T: FontLoader,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        (*self).load_font(query)
    }
}

impl<T> TextShaper for &T
where
    T: TextShaper,
{
    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        (*self).shape_text(request)
    }
}

/// Font system assembled from independent loading and shaping services.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedFontSystem<L, S> {
    loader: L,
    shaper: S,
}

impl<L, S> ComposedFontSystem<L, S> {
    /// Create a font system from a loader and shaper.
    #[must_use]
    pub fn new(loader: L, shaper: S) -> Self {
        Self { loader, shaper }
    }

    /// Borrow the font loader.
    #[must_use]
    pub fn loader(&self) -> &L {
        &self.loader
    }

    /// Borrow the text shaper.
    #[must_use]
    pub fn shaper(&self) -> &S {
        &self.shaper
    }

    /// Split this font system into its loader and shaper.
    #[must_use]
    pub fn into_parts(self) -> (L, S) {
        (self.loader, self.shaper)
    }
}

impl<L, S> FontSystem for ComposedFontSystem<L, S>
where
    L: FontLoader,
    S: TextShaper,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        self.loader.load_font(query)
    }

    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        self.shaper.shape_text(request)
    }
}

/// Font system backed by `ttf-parser` and `rustybuzz`, with font bytes cached by [`FontId`].
#[derive(Debug)]
pub struct RustybuzzFontSystem<L> {
    loader: L,
    loaded_fonts: RefCell<BTreeMap<u32, CachedFont>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CachedFont {
    data: FontData,
    size: Length,
}

impl<L> RustybuzzFontSystem<L> {
    /// Create a Rustybuzz font system from a loader.
    #[must_use]
    pub fn new(loader: L) -> Self {
        Self {
            loader,
            loaded_fonts: RefCell::new(BTreeMap::new()),
        }
    }

    /// Borrow the wrapped loader.
    #[must_use]
    pub fn loader(&self) -> &L {
        &self.loader
    }

    /// Return the wrapped loader.
    #[must_use]
    pub fn into_loader(self) -> L {
        self.loader
    }
}

impl<L> FontSystem for RustybuzzFontSystem<L>
where
    L: FontLoader,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        let font = self.loader.load_font(query)?;
        validate_font(&font)?;
        self.loaded_fonts.borrow_mut().insert(
            font.id.0,
            CachedFont {
                data: font.clone(),
                size: query.size,
            },
        );
        Ok(font)
    }

    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        let cached = self
            .loaded_fonts
            .borrow()
            .get(&request.font.0)
            .cloned()
            .ok_or_else(|| FontError::NotFound {
                family: format!("font id {}", request.font.0),
            })?;
        shape_with_rustybuzz(&cached, request)
    }
}

impl<L> FontLoader for RustybuzzFontSystem<L>
where
    L: FontLoader,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        FontSystem::load_font(self, query)
    }
}

impl<L> TextShaper for RustybuzzFontSystem<L>
where
    L: FontLoader,
{
    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        FontSystem::shape_text(self, request)
    }
}

/// Font lookup request.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontQuery {
    /// Family name as seen by the font resolver.
    pub family: String,
    /// Size in scaled points.
    pub size: Length,
    /// Whether math font tables are required.
    pub math: bool,
}

/// Corner of an OpenType `MathKernInfo` record (mirrors HarfBuzz `hb_ot_math_kern_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathKernCorner {
    /// Superscript on the right.
    TopRight,
    /// Superscript on the left.
    TopLeft,
    /// Subscript on the right.
    BottomRight,
    /// Subscript on the left.
    BottomLeft,
}

/// One part of an OpenType math glyph assembly, with all measurements in scaled points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathAssemblyPart {
    /// Glyph id for this assembly part.
    pub glyph: u32,
    /// Start connector overlap at the leading edge, in scaled points.
    pub start_connector: i32,
    /// End connector overlap at the trailing edge, in scaled points.
    pub end_connector: i32,
    /// Full advance along the assembly axis, in scaled points.
    pub full_advance: i32,
    /// Whether this part is an extender (repeatable to reach the target size).
    pub extender: bool,
}

/// Face the application parsed and owns, the library borrows it and never parses or copies.
pub trait SharedFace: Send + Sync {
    /// Returns the rustybuzz face the application parsed.
    fn rustybuzz_face(&self) -> &rustybuzz::Face<'_>;

    /// Returns the ttf view of the face, defaults to the ttf face inside the rustybuzz face.
    fn ttf_face(&self) -> &ttf_parser::Face<'_> {
        self.rustybuzz_face()
    }
}

/// Where the parsed faces come from: library cached bytes, or a face the application owns.
enum FaceSource {
    Bytes {
        bytes: Arc<[u8]>,
        // Parse caches are shared across clones, so a face is parsed once per font, not once per clone.
        ttf: Arc<once_cell::race::OnceBox<ParsedFace>>,
        rustybuzz: Arc<once_cell::race::OnceBox<ParsedRustybuzzFace>>,
    },
    Shared(Arc<dyn SharedFace>),
}

impl Clone for FaceSource {
    fn clone(&self) -> Self {
        match self {
            Self::Bytes {
                bytes,
                ttf,
                rustybuzz,
            } => Self::Bytes {
                bytes: Arc::clone(bytes),
                ttf: Arc::clone(ttf),
                rustybuzz: Arc::clone(rustybuzz),
            },
            Self::Shared(face) => Self::Shared(Arc::clone(face)),
        }
    }
}

/// Loaded font identity plus its face source.
pub struct FontData {
    /// Stable font identity.
    pub id: FontId,
    /// Canonical font family or face name.
    pub canonical_name: String,
    source: FaceSource,
}

impl Clone for FontData {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            canonical_name: self.canonical_name.clone(),
            source: self.source.clone(),
        }
    }
}

impl PartialEq for FontData {
    fn eq(&self, other: &Self) -> bool {
        let sources_equal = match (&self.source, &other.source) {
            (FaceSource::Bytes { bytes: a, .. }, FaceSource::Bytes { bytes: b, .. }) => a == b,
            (FaceSource::Shared(a), FaceSource::Shared(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        self.id == other.id && self.canonical_name == other.canonical_name && sources_equal
    }
}

impl Eq for FontData {}

impl core::fmt::Debug for FontData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FontData")
            .field("id", &self.id)
            .field("canonical_name", &self.canonical_name)
            .finish_non_exhaustive()
    }
}

impl FontData {
    /// Create font data from an id, canonical name, and raw bytes, the library parses lazily, once.
    #[must_use]
    pub fn new(id: FontId, canonical_name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            id,
            canonical_name: canonical_name.into(),
            source: FaceSource::Bytes {
                bytes: bytes.into(),
                ttf: Arc::new(once_cell::race::OnceBox::new()),
                rustybuzz: Arc::new(once_cell::race::OnceBox::new()),
            },
        }
    }

    /// Create font data from a face the application already parsed, the library never parses it.
    #[must_use]
    pub fn from_shared_face(
        id: FontId,
        canonical_name: impl Into<String>,
        face: Arc<dyn SharedFace>,
    ) -> Self {
        Self {
            id,
            canonical_name: canonical_name.into(),
            source: FaceSource::Shared(face),
        }
    }

    /// Returns the raw bytes when the library owns them, None for application shared faces.
    #[must_use]
    pub fn bytes(&self) -> Option<&Arc<[u8]>> {
        match &self.source {
            FaceSource::Bytes { bytes, .. } => Some(bytes),
            FaceSource::Shared(_) => None,
        }
    }

    /// Runs `f` with the ttf face, parsed at most once, shared across clones.
    pub fn with_ttf_face<R>(
        &self,
        f: impl FnOnce(&ttf_parser::Face<'_>) -> R,
    ) -> Result<R, FontError> {
        Ok(f(parse_face(self)?))
    }

    /// Runs `f` with the rustybuzz face, parsed at most once, shared across clones and with shaping.
    pub fn with_rustybuzz_face<R>(
        &self,
        f: impl FnOnce(&rustybuzz::Face<'_>) -> R,
    ) -> Result<R, FontError> {
        Ok(f(parse_rustybuzz_face(self)?))
    }

    /// Parse overall font metrics at the requested size.
    pub fn metrics(&self, size: Length) -> Result<FontMetrics, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        Ok(FontMetrics {
            ascent: scale_font_units(i32::from(face.ascender()), size, units_per_em),
            descent: scale_font_units(i32::from(face.descender()), size, units_per_em),
            xheight: scale_font_units(i32::from(face.x_height().unwrap_or(0)), size, units_per_em),
            capheight: scale_font_units(
                i32::from(face.capital_height().unwrap_or(0)),
                size,
                units_per_em,
            ),
            slant: (face.italic_angle() * 65_536.0) as i32,
        })
    }

    /// Return metrics for a glyph at the requested size.
    pub fn glyph_metrics(
        &self,
        glyph: GlyphId,
        size: Length,
    ) -> Result<FontGlyphMetrics, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let Ok(glyph_id) = u16::try_from(glyph.0) else {
            return Ok(FontGlyphMetrics::default());
        };
        let glyph_id = ttf_parser::GlyphId(glyph_id);
        let width = face
            .glyph_hor_advance(glyph_id)
            .map(|advance| scale_font_units(i32::from(advance), size, units_per_em))
            .unwrap_or(0);
        let (height, depth) = face
            .glyph_bounding_box(glyph_id)
            .map(|bbox| {
                (
                    scale_font_units(i32::from(bbox.y_max).max(0), size, units_per_em),
                    scale_font_units((-i32::from(bbox.y_min)).max(0), size, units_per_em),
                )
            })
            .unwrap_or((0, 0));
        Ok(FontGlyphMetrics {
            width,
            height,
            depth,
        })
    }

    /// Return outlines in font design units with y pointing up, one entry per input glyph.
    pub fn glyph_outlines(
        &self,
        glyphs: &[GlyphId],
    ) -> Result<Vec<Option<GlyphOutline>>, FontError> {
        let face = parse_face(self)?;
        let units_per_em = face.units_per_em();
        let mut out = Vec::with_capacity(glyphs.len());
        for glyph in glyphs.iter().copied() {
            let Ok(glyph_id) = u16::try_from(glyph.0) else {
                out.push(None);
                continue;
            };
            let mut collector = OutlineCollector {
                commands: Vec::new(),
            };
            // `outline_glyph` returns `None` for blank or bitmap only glyphs, treat as absent.
            if face
                .outline_glyph(ttf_parser::GlyphId(glyph_id), &mut collector)
                .is_none()
            {
                out.push(None);
            } else {
                out.push(Some(GlyphOutline {
                    units_per_em,
                    commands: collector.commands,
                }));
            }
        }
        Ok(out)
    }

    /// Look up a glyph id by Unicode codepoint.
    pub fn glyph_index(&self, codepoint: char) -> Result<Option<GlyphId>, FontError> {
        let face = parse_face(self)?;
        Ok(face
            .glyph_index(codepoint)
            .map(|glyph| GlyphId(u32::from(glyph.0))))
    }

    /// Look up a glyph id by glyph name.
    pub fn glyph_index_by_name(&self, name: &str) -> Result<Option<GlyphId>, FontError> {
        let face = parse_face(self)?;
        Ok(face
            .glyph_index_by_name(name)
            .map(|glyph| GlyphId(u32::from(glyph.0))))
    }

    /// Report whether the font has an OpenType math table.
    pub fn has_opentype_math(&self) -> Result<bool, FontError> {
        Ok(parse_face(self)?.tables().math.is_some())
    }

    /// Number of glyphs in the font (`\XeTeXcountglyphs`).
    pub fn ot_glyph_count(&self) -> Result<u32, FontError> {
        Ok(u32::from(parse_face(self)?.number_of_glyphs()))
    }

    /// Number of OpenType layout scripts from the larger layout script list.
    pub fn ot_script_count(&self) -> Result<u32, FontError> {
        let face = parse_face(self)?;
        Ok(larger_script_list(face).map_or(0, |scripts| u32::from(scripts.len())))
    }

    /// OpenType script tag at `index`, packed big endian to match XeTeX `hb_tag_t` encoding.
    pub fn ot_script_tag(&self, index: u32) -> Result<u32, FontError> {
        let Ok(index) = u16::try_from(index) else {
            return Ok(0);
        };
        let face = parse_face(self)?;
        Ok(larger_script_list(face)
            .and_then(|scripts| scripts.get(index))
            .map_or(0, |script| script.tag.0))
    }

    /// Number of languages under `script_tag`, summed across OpenType layout tables.
    pub fn ot_language_count(&self, script_tag: u32) -> Result<u32, FontError> {
        let face = parse_face(self)?;
        let script_tag = ttf_parser::Tag(script_tag);
        let mut count = 0u32;
        for table in [face.tables().gsub, face.tables().gpos].into_iter().flatten() {
            if let Some(script) = table
                .scripts
                .index(script_tag)
                .and_then(|index| table.scripts.get(index))
            {
                count += u32::from(script.languages.len());
            }
        }
        Ok(count)
    }

    /// Language tag at `index` under `script_tag` (`\XeTeXOTlanguagetag`).
    pub fn ot_language_tag(&self, script_tag: u32, index: u32) -> Result<u32, FontError> {
        let Ok(index) = u16::try_from(index) else {
            return Ok(0);
        };
        let face = parse_face(self)?;
        let script_tag = ttf_parser::Tag(script_tag);
        for table in [face.tables().gsub, face.tables().gpos].into_iter().flatten() {
            if let Some(script) = table
                .scripts
                .index(script_tag)
                .and_then(|script_index| table.scripts.get(script_index))
            {
                if index < script.languages.len() {
                    return Ok(script.languages.get(index).map_or(0, |lang| lang.tag.0));
                }
            }
        }
        Ok(0)
    }

    /// Number of features under `script_tag` and `language_tag`, summed across OpenType layout tables.
    pub fn ot_feature_count(&self, script_tag: u32, language_tag: u32) -> Result<u32, FontError> {
        let face = parse_face(self)?;
        let script_tag = ttf_parser::Tag(script_tag);
        let mut count = 0u32;
        for table in [face.tables().gsub, face.tables().gpos].into_iter().flatten() {
            if let Some(langsys) = language_system(table, script_tag, language_tag) {
                count += u32::from(langsys.feature_indices.len());
            }
        }
        Ok(count)
    }

    /// Feature tag at `index` under `script_tag`/`language_tag` (`\XeTeXOTfeaturetag`).
    pub fn ot_feature_tag(
        &self,
        script_tag: u32,
        language_tag: u32,
        index: u32,
    ) -> Result<u32, FontError> {
        let Ok(index) = u16::try_from(index) else {
            return Ok(0);
        };
        let face = parse_face(self)?;
        let script_tag = ttf_parser::Tag(script_tag);
        for table in [face.tables().gsub, face.tables().gpos].into_iter().flatten() {
            if let Some(langsys) = language_system(table, script_tag, language_tag) {
                if index < langsys.feature_indices.len() {
                    if let Some(feature_index) = langsys.feature_indices.get(index) {
                        return Ok(table
                            .features
                            .get(feature_index)
                            .map_or(0, |feature| feature.tag.0));
                    }
                }
            }
        }
        Ok(0)
    }

    /// OpenType math constant by XeTeX and HarfBuzz constant index.
    pub fn opentype_math_constant(&self, constant: i32, size: Length) -> Result<i32, FontError> {
        let face = parse_face(self)?;
        let Some(constants) = face.tables().math.and_then(|table| table.constants) else {
            return Ok(0);
        };
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let Some(value) = math_constant_value(constants, constant) else {
            return Ok(0);
        };
        if is_math_constant_percentage(constant) {
            Ok(value)
        } else {
            Ok(scale_font_units(value, size, units_per_em))
        }
    }

    /// OpenType math italic correction for a glyph, scaled to scaled points.
    pub fn math_italic_correction(&self, glyph: GlyphId, size: Length) -> Result<i32, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let Ok(glyph_id) = u16::try_from(glyph.0) else {
            return Ok(0);
        };
        let Some(corrections) = face
            .tables()
            .math
            .and_then(|table| table.glyph_info)
            .and_then(|info| info.italic_corrections)
        else {
            return Ok(0);
        };
        let Some(value) = corrections.get(ttf_parser::GlyphId(glyph_id)) else {
            return Ok(0);
        };
        Ok(scale_font_units(i32::from(value.value), size, units_per_em))
    }

    /// Evaluate one OpenType math `MathKernInfo` corner at a correction height in font design units.
    pub fn math_kern_at(
        &self,
        glyph: GlyphId,
        corner: MathKernCorner,
        correction_height: i32,
    ) -> Result<i32, FontError> {
        let face = parse_face(self)?;
        let Ok(glyph_id) = u16::try_from(glyph.0) else {
            return Ok(0);
        };
        let Some(kern_info) = face
            .tables()
            .math
            .and_then(|table| table.glyph_info)
            .and_then(|info| info.kern_infos)
            .and_then(|infos| infos.get(ttf_parser::GlyphId(glyph_id)))
        else {
            return Ok(0);
        };
        let kern = match corner {
            MathKernCorner::TopRight => kern_info.top_right,
            MathKernCorner::TopLeft => kern_info.top_left,
            MathKernCorner::BottomRight => kern_info.bottom_right,
            MathKernCorner::BottomLeft => kern_info.bottom_left,
        };
        let Some(kern) = kern else {
            return Ok(0);
        };
        let count = kern.count();
        let mut i = 0u16;
        while i < count {
            match kern.height(i) {
                Some(h) if correction_height < i32::from(h.value) => break,
                _ => i += 1,
            }
        }
        Ok(kern.kern(i).map(|v| i32::from(v.value)).unwrap_or(0))
    }

    /// Return the larger OpenType math glyph variant at `index`, with its advance scaled to points.
    pub fn math_variant(
        &self,
        glyph: GlyphId,
        index: u16,
        horizontal: bool,
        size: Length,
    ) -> Result<Option<(u32, i32)>, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let Ok(glyph_id) = u16::try_from(glyph.0) else {
            return Ok(None);
        };
        let Some(variants) = face.tables().math.and_then(|table| table.variants) else {
            return Ok(None);
        };
        let constructions = if horizontal {
            variants.horizontal_constructions
        } else {
            variants.vertical_constructions
        };
        let Some(construction) = constructions.get(ttf_parser::GlyphId(glyph_id)) else {
            return Ok(None);
        };
        let Some(variant) = construction.variants.get(index) else {
            return Ok(None);
        };
        let advance = scale_font_units(
            i32::from(variant.advance_measurement),
            size,
            units_per_em,
        );
        Ok(Some((u32::from(variant.variant_glyph.0), advance)))
    }

    /// OpenType math glyph assembly parts, each metric scaled to scaled points.
    pub fn math_assembly(
        &self,
        glyph: GlyphId,
        horizontal: bool,
        size: Length,
    ) -> Result<Vec<MathAssemblyPart>, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let Ok(glyph_id) = u16::try_from(glyph.0) else {
            return Ok(Vec::new());
        };
        let Some(variants) = face.tables().math.and_then(|table| table.variants) else {
            return Ok(Vec::new());
        };
        let constructions = if horizontal {
            variants.horizontal_constructions
        } else {
            variants.vertical_constructions
        };
        let Some(assembly) = constructions
            .get(ttf_parser::GlyphId(glyph_id))
            .and_then(|construction| construction.assembly)
        else {
            return Ok(Vec::new());
        };
        let parts = assembly
            .parts
            .into_iter()
            .map(|part| MathAssemblyPart {
                glyph: u32::from(part.glyph_id.0),
                start_connector: scale_font_units(
                    i32::from(part.start_connector_length),
                    size,
                    units_per_em,
                ),
                end_connector: scale_font_units(
                    i32::from(part.end_connector_length),
                    size,
                    units_per_em,
                ),
                full_advance: scale_font_units(
                    i32::from(part.full_advance),
                    size,
                    units_per_em,
                ),
                extender: part.part_flags.extender(),
            })
            .collect();
        Ok(parts)
    }

    /// OpenType math minimum connector overlap between assembly parts, in scaled points.
    pub fn math_min_connector_overlap(&self, size: Length) -> Result<i32, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let overlap = face
            .tables()
            .math
            .and_then(|table| table.variants)
            .map(|variants| i32::from(variants.min_connector_overlap))
            .unwrap_or(0);
        Ok(scale_font_units(overlap, size, units_per_em))
    }

    /// Convert points to font design units using XeTeX `pointsToUnits` in `f32`.
    pub fn points_to_units(&self, points: f32, size: Length) -> Result<f32, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let point_size = (f64::from(size.0) / 65536.0) as f32;
        if point_size == 0.0 {
            return Ok(0.0);
        }
        Ok((points * units_per_em as f32) / point_size)
    }

    /// Convert font design units to scaled points using XeTeX `D2Fix(unitsToPoints(...))`.
    pub fn units_to_scaled(&self, units: i32, size: Length) -> Result<i32, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        Ok(scale_font_units(units, size, units_per_em))
    }

    /// Top accent attachment position for `glyph`, scaled to scaled points (`\XeTeXmathaccent`).
    pub fn opentype_math_accent_position(
        &self,
        glyph: GlyphId,
        size: Length,
    ) -> Result<i32, FontError> {
        let face = parse_face(self)?;
        let units_per_em = i32::from(face.units_per_em()).max(1);
        let Ok(glyph_id) = u16::try_from(glyph.0) else {
            return Ok(0);
        };
        let Some(attachments) = face
            .tables()
            .math
            .and_then(|table| table.glyph_info)
            .and_then(|info| info.top_accent_attachments)
        else {
            return Ok(0);
        };
        let Some(value) = attachments.get(ttf_parser::GlyphId(glyph_id)) else {
            return Ok(0);
        };
        Ok(scale_font_units(i32::from(value.value), size, units_per_em))
    }

    /// Return the symbol font parameter at `parameter` index, mapping to OpenType math constants.
    pub fn math_symbol_parameter(&self, parameter: i32, size: Length) -> Result<i32, FontError> {
        match parameter {
            5 => self.opentype_math_constant(6, size),
            6 => Ok(size.0),
            8 => self.opentype_math_constant(33, size),
            9 => self.opentype_math_constant(32, size),
            10 => self.opentype_math_constant(22, size),
            11 => self.opentype_math_constant(35, size),
            12 => self.opentype_math_constant(34, size),
            13 | 14 => self.opentype_math_constant(11, size),
            15 => self.opentype_math_constant(12, size),
            16 | 17 => self.opentype_math_constant(8, size),
            18 => self.opentype_math_constant(14, size),
            19 => self.opentype_math_constant(10, size),
            20 => self.opentype_math_constant(2, size),
            21 => {
                let delim1 = self.math_symbol_parameter(20, size)?;
                Ok(((i64::from(size.0) * 3) / 2)
                    .min(i64::from(delim1))
                    .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
            }
            22 => self.opentype_math_constant(5, size),
            _ => Ok(0),
        }
    }

    /// Return the extension font parameter at `parameter` index, mapping to OpenType math constants.
    pub fn math_extension_parameter(&self, parameter: i32, size: Length) -> Result<i32, FontError> {
        match parameter {
            5 => self.opentype_math_constant(6, size),
            6 => Ok(size.0),
            8 => self.opentype_math_constant(38, size),
            9 => self.opentype_math_constant(18, size),
            10 => self.opentype_math_constant(20, size),
            11 => self.opentype_math_constant(19, size),
            12 => self.opentype_math_constant(21, size),
            13 => self.opentype_math_constant(26, size),
            _ => Ok(0),
        }
    }
}

/// Overall font metrics in TeX scaled point units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontMetrics {
    /// Distance from baseline to top of typical ascenders.
    pub ascent: i32,
    /// Distance from baseline to bottom of typical descenders (negative).
    pub descent: i32,
    /// Height of a lowercase x.
    pub xheight: i32,
    /// Height of a capital letter.
    pub capheight: i32,
    /// Italic slant as a fixed point angle.
    pub slant: i32,
}

/// Metrics for a single glyph in TeX scaled point units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontGlyphMetrics {
    /// Horizontal advance width.
    pub width: i32,
    /// Height above baseline.
    pub height: i32,
    /// Depth below baseline.
    pub depth: i32,
}

/// OpenType feature request (tag + value), e.g. `ssty=1` for math script size substitution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeFeature {
    /// Four byte OpenType feature tag, e.g. `*b"ssty"`.
    pub tag: [u8; 4],
    /// One based alternate index for `ssty`, `0` disables the feature.
    pub value: u32,
}

/// Text shaping request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeRequest<'a> {
    /// Font to use for shaping, identified by its `FontId`.
    pub font: FontId,
    /// Input text to shape.
    pub text: &'a str,
    /// Text direction for the buffer.
    pub direction: Direction,
    /// Source span for cluster mapping.
    pub source: Option<ByteSpan>,
    /// OpenType script tag override, e.g. `*b"math"`, for features under the `math` script.
    pub script: Option<[u8; 4]>,
    /// OpenType features to apply during shaping, e.g. `ssty=1`.
    pub features: Vec<ShapeFeature>,
}

/// Shaped text result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShapedText {
    /// Glyphs in visual order.
    pub glyphs: Vec<PositionedGlyph>,
}

impl ShapedText {
    /// Construct a `ShapedText` from a glyph vec.
    #[must_use]
    pub fn new(glyphs: impl Into<Vec<PositionedGlyph>>) -> Self {
        Self {
            glyphs: glyphs.into(),
        }
    }
}

/// Font system failure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontError {
    /// Font was not found by the loader.
    NotFound {
        /// Family name that was requested.
        family: String,
    },
    /// Font exists but cannot satisfy the request.
    Invalid {
        /// Family name.
        family: String,
        /// Reason the font is unusable.
        message: String,
    },
    /// Text shaping is not supported by this font system.
    ShapingUnsupported {
        /// Reason shaping is unsupported.
        message: String,
    },
}

/// Alias so the self_cell dependent type names its lifetime explicitly.
type TtfFace<'a> = ttf_parser::Face<'a>;

self_cell::self_cell!(
    /// Owns `Arc<[u8]>` together with the `ttf_parser::Face` parsed from it.
    struct ParsedFace {
        owner: Arc<[u8]>,
        #[covariant]
        dependent: TtfFace,
    }
);

/// Alias so the self_cell dependent type names its lifetime explicitly.
type RbFace<'a> = rustybuzz::Face<'a>;

self_cell::self_cell!(
    /// Owns `Arc<[u8]>` together with the `rustybuzz::Face` parsed from it.
    struct ParsedRustybuzzFace {
        owner: Arc<[u8]>,
        #[covariant]
        dependent: RbFace,
    }
);

/// Returns the rustybuzz face: the application face when shared, one cached parse otherwise.
fn parse_rustybuzz_face(font: &FontData) -> Result<&rustybuzz::Face<'_>, FontError> {
    match &font.source {
        FaceSource::Shared(face) => Ok(face.rustybuzz_face()),
        FaceSource::Bytes {
            bytes, rustybuzz, ..
        } => {
            if rustybuzz.get().is_none() {
                let parsed = ParsedRustybuzzFace::try_new(Arc::clone(bytes), |bytes| {
                    rustybuzz::Face::from_slice(bytes, 0).ok_or_else(|| FontError::Invalid {
                        family: font.canonical_name.clone(),
                        message: "invalid font data".to_string(),
                    })
                })?;
                let _ = rustybuzz.set(alloc::boxed::Box::new(parsed));
            }
            Ok(rustybuzz
                .get()
                .expect("rustybuzz cache populated above")
                .borrow_dependent())
        }
    }
}

fn validate_font(font: &FontData) -> Result<(), FontError> {
    parse_face(font).map(|_| ())
}

/// Larger OpenType layout script list, mirroring XeTeX `getLargerScriptListTable`.
fn larger_script_list<'a>(
    face: &ttf_parser::Face<'a>,
) -> Option<ttf_parser::opentype_layout::ScriptList<'a>> {
    let gsub = face.tables().gsub.map(|table| table.scripts);
    let gpos = face.tables().gpos.map(|table| table.scripts);
    match (gsub, gpos) {
        (Some(sub), Some(pos)) => Some(if pos.len() > sub.len() { pos } else { sub }),
        (sub, pos) => sub.or(pos),
    }
}

/// Resolve a language system, `language_tag == 0` selects the script default.
fn language_system<'a>(
    table: ttf_parser::opentype_layout::LayoutTable<'a>,
    script_tag: ttf_parser::Tag,
    language_tag: u32,
) -> Option<ttf_parser::opentype_layout::LanguageSystem<'a>> {
    let script = table
        .scripts
        .index(script_tag)
        .and_then(|index| table.scripts.get(index))?;
    if language_tag == 0 {
        script.default_language
    } else {
        script
            .languages
            .index(ttf_parser::Tag(language_tag))
            .and_then(|index| script.languages.get(index))
            .or(script.default_language)
    }
}

/// Returns the ttf face: the application face when shared, one cached parse otherwise.
fn parse_face(font: &FontData) -> Result<&ttf_parser::Face<'_>, FontError> {
    let invalid = |error: ttf_parser::FaceParsingError| FontError::Invalid {
        family: font.canonical_name.clone(),
        message: format!("invalid font data: {error:?}"),
    };

    match &font.source {
        FaceSource::Shared(face) => Ok(face.ttf_face()),
        FaceSource::Bytes { bytes, ttf, .. } => {
            if ttf.get().is_none() {
                let parsed = ParsedFace::try_new(Arc::clone(bytes), |bytes| {
                    ttf_parser::Face::parse(bytes, 0).map_err(invalid)
                })?;
                // `set` only fails when another initializer raced, `get` is populated either way.
                let _ = ttf.set(alloc::boxed::Box::new(parsed));
            }
            Ok(ttf.get().expect("ttf cache populated above").borrow_dependent())
        }
    }
}

/// Collects `ttf_parser` contour callbacks into [`OutlineCommand`]s.
struct OutlineCollector {
    commands: Vec<OutlineCommand>,
}

impl ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(OutlineCommand::MoveTo { x, y });
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(OutlineCommand::LineTo { x, y });
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.commands.push(OutlineCommand::QuadTo { cx, cy, x, y });
    }

    fn curve_to(&mut self, c1x: f32, c1y: f32, c2x: f32, c2y: f32, x: f32, y: f32) {
        self.commands.push(OutlineCommand::CurveTo {
            c1x,
            c1y,
            c2x,
            c2y,
            x,
            y,
        });
    }

    fn close(&mut self) {
        self.commands.push(OutlineCommand::Close);
    }
}

fn math_constant_value(constants: ttf_parser::math::Constants<'_>, constant: i32) -> Option<i32> {
    let value = match constant {
        0 => i32::from(constants.script_percent_scale_down()),
        1 => i32::from(constants.script_script_percent_scale_down()),
        2 => i32::from(constants.delimited_sub_formula_min_height()),
        3 => i32::from(constants.display_operator_min_height()),
        4 => i32::from(constants.math_leading().value),
        5 => i32::from(constants.axis_height().value),
        6 => i32::from(constants.accent_base_height().value),
        7 => i32::from(constants.flattened_accent_base_height().value),
        8 => i32::from(constants.subscript_shift_down().value),
        9 => i32::from(constants.subscript_top_max().value),
        10 => i32::from(constants.subscript_baseline_drop_min().value),
        11 => i32::from(constants.superscript_shift_up().value),
        12 => i32::from(constants.superscript_shift_up_cramped().value),
        13 => i32::from(constants.superscript_bottom_min().value),
        14 => i32::from(constants.superscript_baseline_drop_max().value),
        15 => i32::from(constants.sub_superscript_gap_min().value),
        16 => i32::from(constants.superscript_bottom_max_with_subscript().value),
        17 => i32::from(constants.space_after_script().value),
        18 => i32::from(constants.upper_limit_gap_min().value),
        19 => i32::from(constants.upper_limit_baseline_rise_min().value),
        20 => i32::from(constants.lower_limit_gap_min().value),
        21 => i32::from(constants.lower_limit_baseline_drop_min().value),
        22 => i32::from(constants.stack_top_shift_up().value),
        23 => i32::from(constants.stack_top_display_style_shift_up().value),
        24 => i32::from(constants.stack_bottom_shift_down().value),
        25 => i32::from(constants.stack_bottom_display_style_shift_down().value),
        26 => i32::from(constants.stack_gap_min().value),
        27 => i32::from(constants.stack_display_style_gap_min().value),
        28 => i32::from(constants.stretch_stack_top_shift_up().value),
        29 => i32::from(constants.stretch_stack_bottom_shift_down().value),
        30 => i32::from(constants.stretch_stack_gap_above_min().value),
        31 => i32::from(constants.stretch_stack_gap_below_min().value),
        32 => i32::from(constants.fraction_numerator_shift_up().value),
        33 => i32::from(constants.fraction_numerator_display_style_shift_up().value),
        34 => i32::from(constants.fraction_denominator_shift_down().value),
        35 => i32::from(
            constants
                .fraction_denominator_display_style_shift_down()
                .value,
        ),
        36 => i32::from(constants.fraction_numerator_gap_min().value),
        37 => i32::from(constants.fraction_num_display_style_gap_min().value),
        38 => i32::from(constants.fraction_rule_thickness().value),
        39 => i32::from(constants.fraction_denominator_gap_min().value),
        40 => i32::from(constants.fraction_denom_display_style_gap_min().value),
        41 => i32::from(constants.skewed_fraction_horizontal_gap().value),
        42 => i32::from(constants.skewed_fraction_vertical_gap().value),
        43 => i32::from(constants.overbar_vertical_gap().value),
        44 => i32::from(constants.overbar_rule_thickness().value),
        45 => i32::from(constants.overbar_extra_ascender().value),
        46 => i32::from(constants.underbar_vertical_gap().value),
        47 => i32::from(constants.underbar_rule_thickness().value),
        48 => i32::from(constants.underbar_extra_descender().value),
        49 => i32::from(constants.radical_vertical_gap().value),
        50 => i32::from(constants.radical_display_style_vertical_gap().value),
        51 => i32::from(constants.radical_rule_thickness().value),
        52 => i32::from(constants.radical_extra_ascender().value),
        53 => i32::from(constants.radical_kern_before_degree().value),
        54 => i32::from(constants.radical_kern_after_degree().value),
        55 => i32::from(constants.radical_degree_bottom_raise_percent()),
        _ => return None,
    };
    Some(value)
}

fn is_math_constant_percentage(constant: i32) -> bool {
    matches!(constant, 0 | 1 | 55)
}

fn shape_with_rustybuzz(
    cached: &CachedFont,
    request: &ShapeRequest<'_>,
) -> Result<ShapedText, FontError> {
    let face = parse_rustybuzz_face(&cached.data)?;
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(request.text);
    buffer.set_direction(match request.direction {
        Direction::LeftToRight => rustybuzz::Direction::LeftToRight,
        Direction::RightToLeft => rustybuzz::Direction::RightToLeft,
        Direction::TopToBottom => rustybuzz::Direction::TopToBottom,
        _ => rustybuzz::Direction::LeftToRight,
    });
    buffer.guess_segment_properties();
    if let Some(script_tag) = request.script {
        // `from_iso15924_tag` maps `math` so rustybuzz selects the script with `ssty` lookups.
        if let Some(script) =
            rustybuzz::Script::from_iso15924_tag(ttf_parser::Tag::from_bytes(&script_tag))
        {
            buffer.set_script(script);
        }
    }

    let features: Vec<rustybuzz::Feature> = request
        .features
        .iter()
        .map(|feature| {
            rustybuzz::Feature::new(ttf_parser::Tag::from_bytes(&feature.tag), feature.value, ..)
        })
        .collect();
    let shaped = rustybuzz::shape(face, &features, buffer);
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();
    let units_per_em = face.units_per_em().max(1);
    let glyphs = infos
        .iter()
        .zip(positions.iter())
        .map(|(info, position)| PositionedGlyph {
            glyph_id: GlyphId(info.glyph_id),
            advance: Point {
                x: scaled_font_units(position.x_advance, cached.size, units_per_em),
                y: scaled_font_units(position.y_advance, cached.size, units_per_em),
            },
            offset: Point {
                x: scaled_font_units(position.x_offset, cached.size, units_per_em),
                y: scaled_font_units(position.y_offset, cached.size, units_per_em),
            },
            cluster: request
                .source
                .map(|source| cluster_source_span(request.text, source, info.cluster)),
        })
        .collect();

    Ok(ShapedText { glyphs })
}

fn scaled_font_units(value: i32, size: Length, units_per_em: i32) -> Length {
    Length::from_scaled_points(scale_font_units(value, size, units_per_em))
}

    /// Scale font design units to scaled points, reproducing XeTeX arithmetic.
fn scale_font_units(value: i32, size: Length, units_per_em: i32) -> i32 {
    // m_pointSize is f32 in XeTeXFontInst, unitsToPoints runs in f32 before D2Fix promotes to f64.
    let point_size = (f64::from(size.0) / 65536.0) as f32;
    let points = (value as f32 * point_size) / (units_per_em.max(1) as f32);
    // D2Fix uses truncation because C integer casts truncate toward zero.
    let fixed = (f64::from(points) * 65536.0 + 0.5).trunc();
    fixed.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

fn cluster_source_span(text: &str, source: ByteSpan, cluster: u32) -> ByteSpan {
    let start = usize::try_from(cluster)
        .ok()
        .map(|cluster| cluster.min(text.len()))
        .unwrap_or(text.len());
    let mut end = text.len();
    for (index, _) in text.char_indices() {
        if index > start {
            end = index;
            break;
        }
    }
    ByteSpan {
        start: source.start.saturating_add(start as u32),
        end: source.start.saturating_add(end as u32).min(source.end),
    }
}

/// Deterministic empty font system.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFontSystem;

impl FontSystem for NoFontSystem {
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        Err(FontError::NotFound {
            family: query.family.clone(),
        })
    }

    fn shape_text(&self, _request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        Err(FontError::ShapingUnsupported {
            message: "no font shaper configured".to_string(),
        })
    }
}

/// Font system backed by an in process font map, for embedded and browser environments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InMemoryFontSystem {
    fonts: BTreeMap<String, FontData>,
    shape_without_native_engine: bool,
}

impl InMemoryFontSystem {
    /// Create an empty in memory font system.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable deterministic fallback shaping for tests and bootstrap, without a native shaping engine.
    #[must_use]
    pub fn with_fallback_shaping(mut self) -> Self {
        self.shape_without_native_engine = true;
        self
    }

    /// Add a font by family name and return `self`.
    #[must_use]
    pub fn with_font(mut self, family: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.insert(family, bytes);
        self
    }

    /// Insert or replace a font.
    pub fn insert(&mut self, family: impl Into<String>, bytes: impl Into<Vec<u8>>) {
        let family = family.into();
        let id = FontId(self.fonts.len() as u32);
        let bytes: Vec<u8> = bytes.into();
        self.fonts
            .insert(family.clone(), FontData::new(id, family, bytes));
    }

    /// Add a font the application already constructed, sharing its parsed faces, and return `self`.
    #[must_use]
    pub fn with_font_data(mut self, font: FontData) -> Self {
        self.insert_font_data(font);
        self
    }

    /// Insert or replace an application owned font keyed by its canonical name, parses stay shared.
    pub fn insert_font_data(&mut self, font: FontData) {
        self.fonts.insert(font.canonical_name.clone(), font);
    }
}

impl FontSystem for InMemoryFontSystem {
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        self.fonts
            .get(&query.family)
            .cloned()
            .ok_or_else(|| FontError::NotFound {
                family: query.family.clone(),
            })
    }

    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        if !self.shape_without_native_engine {
            return Err(FontError::ShapingUnsupported {
                message: "fallback shaping is disabled".to_string(),
            });
        }

        let glyphs = request
            .text
            .char_indices()
            .enumerate()
            .map(|(index, (byte_offset, ch))| {
                let start = request
                    .source
                    .map_or(byte_offset as u32, |span| span.start + byte_offset as u32);
                PositionedGlyph {
                    glyph_id: GlyphId(ch as u32),
                    offset: Point {
                        x: Length::from_scaled_points((index as i32) * 65_536),
                        y: Length::ZERO,
                    },
                    advance: Point {
                        x: Length::from_scaled_points(65_536),
                        y: Length::ZERO,
                    },
                    cluster: Some(ByteSpan {
                        start,
                        end: start + ch.len_utf8() as u32,
                    }),
                }
            })
            .collect();

        Ok(ShapedText { glyphs })
    }
}

impl FontLoader for InMemoryFontSystem {
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        FontSystem::load_font(self, query)
    }
}

impl TextShaper for InMemoryFontSystem {
    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        FontSystem::shape_text(self, request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEJAVU: &[u8] =
        include_bytes!("../../../vendor/texlive-source/libs/gd/libgd-src/tests/freetype/DejaVuSans.ttf");

    #[test]
    fn font_data_clones_share_both_parsed_faces() {
        let font = FontData::new(FontId(7), "DejaVu Sans", DEJAVU.to_vec());
        let clone = font.clone();

        let ttf_ptr = font
            .with_ttf_face(|face| face as *const ttf_parser::Face<'_> as usize)
            .expect("ttf parse");
        let clone_ttf_ptr = clone
            .with_ttf_face(|face| face as *const ttf_parser::Face<'_> as usize)
            .expect("ttf parse via clone");
        assert_eq!(ttf_ptr, clone_ttf_ptr, "clone must reuse the same ttf parse");

        let rb_ptr = font
            .with_rustybuzz_face(|face| face as *const rustybuzz::Face<'_> as usize)
            .expect("rustybuzz parse");
        let clone_rb_ptr = clone
            .with_rustybuzz_face(|face| face as *const rustybuzz::Face<'_> as usize)
            .expect("rustybuzz parse via clone");
        assert_eq!(rb_ptr, clone_rb_ptr, "clone must reuse the same rustybuzz parse");

        assert!(Arc::ptr_eq(
            font.bytes().expect("library owned bytes"),
            clone.bytes().expect("library owned bytes"),
        ));
    }

    self_cell::self_cell!(
        struct AppOwnedFace {
            owner: Arc<[u8]>,
            #[covariant]
            dependent: RbFace,
        }
    );

    struct AppShared(AppOwnedFace);

    impl SharedFace for AppShared {
        fn rustybuzz_face(&self) -> &rustybuzz::Face<'_> {
            self.0.borrow_dependent()
        }
    }

    #[test]
    fn application_owned_face_is_borrowed_and_never_parsed_by_the_library() {
        let bytes: Arc<[u8]> = DEJAVU.to_vec().into();
        let app_face = AppShared(
            AppOwnedFace::try_new(Arc::clone(&bytes), |bytes| {
                rustybuzz::Face::from_slice(bytes, 0).ok_or("app parse failed")
            })
            .expect("application parses its own font"),
        );
        let app_face: Arc<dyn SharedFace> = Arc::new(app_face);
        let app_ptr = app_face.rustybuzz_face() as *const rustybuzz::Face<'_> as usize;

        let font = FontData::from_shared_face(FontId(3), "DejaVu Sans", Arc::clone(&app_face));

        assert!(font.bytes().is_none(), "the library holds no bytes");
        let lib_ptr = font
            .with_rustybuzz_face(|face| face as *const rustybuzz::Face<'_> as usize)
            .expect("borrow shared face");
        assert_eq!(lib_ptr, app_ptr, "the library must borrow the application face");

        let metrics = font.metrics(Length::from_scaled_points(655_360)).expect("metrics");
        assert!(metrics.ascent > 0);
        let glyph = font.glyph_index('A').expect("glyph lookup");
        assert!(glyph.is_some());
    }

    #[test]
    fn font_data_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FontData>();
    }

    #[test]
    fn fallback_shaping_preserves_source_clusters() {
        let fonts = InMemoryFontSystem::new().with_fallback_shaping();
        let shaped = FontSystem::shape_text(
            &fonts,
            &ShapeRequest {
                font: FontId(0),
                text: "ab",
                direction: Direction::LeftToRight,
                source: Some(ByteSpan { start: 4, end: 6 }),
                script: None,
                features: Vec::new(),
            },
        )
        .expect("fallback shaping should work");

        assert_eq!(shaped.glyphs.len(), 2);
        assert_eq!(shaped.glyphs[0].glyph_id, GlyphId('a' as u32));
        assert_eq!(
            shaped.glyphs[0].cluster,
            Some(ByteSpan { start: 4, end: 5 })
        );
        assert_eq!(
            shaped.glyphs[1].cluster,
            Some(ByteSpan { start: 5, end: 6 })
        );
    }

    #[test]
    fn no_font_system_makes_missing_font_explicit() {
        let error = NoFontSystem
            .load_font(&FontQuery {
                family: "missing".to_string(),
                size: Length::ZERO,
                math: false,
            })
            .expect_err("font should be missing");

        assert_eq!(
            error,
            FontError::NotFound {
                family: "missing".to_string(),
            }
        );
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestLoader;

    impl FontLoader for TestLoader {
        fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
            Ok(FontData::new(
                FontId(9),
                query.family.clone(),
                b"parsed-by-rust-loader".to_vec(),
            ))
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestShaper;

    impl TextShaper for TestShaper {
        fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
            Ok(ShapedText::new(alloc::vec![PositionedGlyph {
                glyph_id: GlyphId(100),
                offset: Point::default(),
                advance: Point {
                    x: Length::from_scaled_points(request.text.len() as i32),
                    y: Length::ZERO,
                },
                cluster: request.source,
            }]))
        }
    }

    #[test]
    fn composed_font_system_splits_loading_from_shaping() {
        let fonts = ComposedFontSystem::new(TestLoader, TestShaper);

        let font = fonts
            .load_font(&FontQuery {
                family: "Latin Modern Math".to_string(),
                size: Length::from_scaled_points(12 * 65_536),
                math: true,
            })
            .expect("loader should resolve font");
        let shaped = fonts
            .shape_text(&ShapeRequest {
                font: font.id,
                text: "xy",
                direction: Direction::LeftToRight,
                source: Some(ByteSpan { start: 2, end: 4 }),
                script: None,
                features: Vec::new(),
            })
            .expect("shaper should shape text");

        assert_eq!(font.id, FontId(9));
        assert_eq!(
            &**font.bytes().expect("library owned bytes"),
            b"parsed-by-rust-loader"
        );
        assert_eq!(shaped.glyphs[0].glyph_id, GlyphId(100));
        assert_eq!(
            shaped.glyphs[0].cluster,
            Some(ByteSpan { start: 2, end: 4 })
        );
    }

    #[test]
    fn scale_font_units_matches_xetex_d2fix_rounding() {
        // The '2' glyph in latinmodern-math rounds 436469.76sp to 436470 like XeTeX.
        let ten_pt = Length::from_scaled_points(10 * 65_536);
        assert_eq!(
            scale_font_units(666, ten_pt, 1000),
            436_470,
            "D2Fix must round 436469.76sp up to 436470 (xetex parity)"
        );
        // 'x' glyph height of 431 units gives 282460.16sp, rounded to 282460.
        assert_eq!(scale_font_units(431, ten_pt, 1000), 282_460);
        // 'x' advance of 528 units gives 346030.08sp, rounded to 346030.
        assert_eq!(scale_font_units(528, ten_pt, 1000), 346_030);
        // Glyph 89 italic correction of 16 units gives 10485.76sp, rounded to 10486.
        assert_eq!(scale_font_units(16, ten_pt, 1000), 10_486);
        // Negative quantities use C cast truncation, so minus 10485.26 becomes minus 10485.
        assert_eq!(scale_font_units(-16, ten_pt, 1000), -10_485);
        assert_eq!(scale_font_units(0, ten_pt, 1000), 0);
    }

    #[test]
    fn rustybuzz_font_system_shapes_loaded_ttf_and_reads_metrics() {
        let fonts = RustybuzzFontSystem::new(InMemoryFontSystem::new().with_font(
            "DejaVu Sans",
            include_bytes!("../../../vendor/texlive-source/libs/gd/libgd-src/tests/freetype/DejaVuSans.ttf")
                .as_slice(),
        ));
        let font = FontSystem::load_font(
            &fonts,
            &FontQuery {
                family: "DejaVu Sans".to_string(),
                size: Length::from_scaled_points(10 * 65_536),
                math: false,
            },
        )
        .expect("real TTF should parse");

        let metrics = font
            .metrics(Length::from_scaled_points(10 * 65_536))
            .expect("metrics should parse");
        let shaped = FontSystem::shape_text(
            &fonts,
            &ShapeRequest {
                font: font.id,
                text: "AV",
                direction: Direction::LeftToRight,
                source: Some(ByteSpan { start: 7, end: 9 }),
                script: None,
                features: Vec::new(),
            },
        )
        .expect("rustybuzz should shape cached font");

        assert!(metrics.ascent > 0);
        assert!(shaped.glyphs.len() >= 2);
        assert!(shaped.glyphs[0].advance.x > Length::ZERO);
        assert_eq!(
            shaped.glyphs[0].cluster,
            Some(ByteSpan { start: 7, end: 8 })
        );
    }

    const LM_MATH: &str = "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/lm-math/latinmodern-math.otf";
    const STIX_MATH: &str =
        "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/stix2-otf/STIXTwoMath-Regular.otf";

    fn load_system_font(path: &str) -> Option<FontData> {
        let bytes = std::fs::read(path).ok()?;
        Some(FontData::new(FontId(1), path, bytes))
    }

    fn ten_pt() -> Length {
        Length::from_scaled_points(10 * 65_536)
    }

    #[test]
    fn glyph_outlines_extract_design_unit_contours_from_latinmodern() {
        let Some(font) = load_system_font(LM_MATH) else {
            eprintln!("SKIP: {LM_MATH} not found");
            return;
        };
        let x = font.glyph_index('x').unwrap().unwrap();
        // The run is parsed once, 'x' has an outline and the space glyph does not.
        let space = font.glyph_index(' ').unwrap().unwrap();
        let outlines = font.glyph_outlines(&[x, space]).unwrap();
        assert_eq!(outlines.len(), 2);

        let x_outline = outlines[0].as_ref().expect("'x' has an outline");
        assert_eq!(x_outline.units_per_em, 1000, "lm-math upem");
        assert!(
            !x_outline.commands.is_empty(),
            "'x' outline has contour commands"
        );
        assert!(
            matches!(x_outline.commands[0], OutlineCommand::MoveTo { .. }),
            "a contour starts with MoveTo"
        );

        assert!(outlines[1].is_none(), "space has no outline");
    }

    #[test]
    fn math_variant_returns_larger_paren_glyphs_from_latinmodern() {
        let Some(font) = load_system_font(LM_MATH) else {
            eprintln!("SKIP: {LM_MATH} not found");
            return;
        };
        // In latinmodern-math, glyph 9 vertical variant 4 is glyph 2433 with advance 1175061 sp.
        let paren = font.glyph_index('(').unwrap().unwrap();
        assert_eq!(paren.0, 9);
        let variant = font
            .math_variant(paren, 4, false, ten_pt())
            .unwrap()
            .expect("'(' has a 5th vertical variant");
        assert_eq!(variant.0, 2433, "variant glyph id");
        assert_eq!(variant.1, 1_175_061, "variant advance sp");
        // variant[0] is the base glyph itself with advance 653394 sp.
        let base = font.math_variant(paren, 0, false, ten_pt()).unwrap().unwrap();
        assert_eq!(base, (9, 653_394));
        // Out of range index yields None.
        assert!(font.math_variant(paren, 99, false, ten_pt()).unwrap().is_none());
    }

    #[test]
    fn math_assembly_returns_paren_parts_from_latinmodern() {
        let Some(font) = load_system_font(LM_MATH) else {
            eprintln!("SKIP: {LM_MATH} not found");
            return;
        };
        let paren = font.glyph_index('(').unwrap().unwrap();
        let parts = font.math_assembly(paren, false, ten_pt()).unwrap();
        // The glyph 9 vertical assembly uses bottom 2503, extender 2504, and top 2505.
        assert_eq!(parts.len(), 3, "'(' assembly has 3 parts");
        assert_eq!(
            parts[0],
            MathAssemblyPart {
                glyph: 2503,
                start_connector: 0,
                end_connector: 163_185,  // 249 units
                full_advance: 979_763,   // 1495 units
                extender: false,
            }
        );
        assert_eq!(
            parts[1],
            MathAssemblyPart {
                glyph: 2504,
                start_connector: 326_369, // 498 units
                end_connector: 326_369,
                full_advance: 326_369,
                extender: true,
            }
        );
        assert_eq!(
            parts[2],
            MathAssemblyPart {
                glyph: 2505,
                start_connector: 163_185,
                end_connector: 0,
                full_advance: 979_763,
                extender: false,
            }
        );
        // Minimum connector overlap of 20 units gives 13107 sp at 10pt.
        assert_eq!(font.math_min_connector_overlap(ten_pt()).unwrap(), 13_107);
    }

    #[test]
    fn math_kern_reads_stix_cut_ins() {
        let Some(font) = load_system_font(STIX_MATH) else {
            eprintln!("SKIP: {STIX_MATH} not found");
            return;
        };
        // 'F' glyph 8 has one TopRight superscript kern value, so any correction height returns 44.
        let f = font.glyph_index('F').unwrap().unwrap();
        assert_eq!(f.0, 8);
        assert_eq!(
            font.math_kern_at(f, MathKernCorner::TopRight, 0).unwrap(),
            44
        );
        assert_eq!(
            font.math_kern_at(f, MathKernCorner::TopRight, 100_000).unwrap(),
            44
        );
        // 'V' glyph 24 has BottomRight heights [126, 280] and kerns [-193, -119, 56].
        let v = font.glyph_index('V').unwrap().unwrap();
        assert_eq!(v.0, 24);
        assert_eq!(
            font.math_kern_at(v, MathKernCorner::BottomRight, 0).unwrap(),
            -193,
            "below first height -> kern[0]"
        );
        assert_eq!(
            font.math_kern_at(v, MathKernCorner::BottomRight, 200).unwrap(),
            -119,
            "between heights -> kern[1]"
        );
        assert_eq!(
            font.math_kern_at(v, MathKernCorner::BottomRight, 300).unwrap(),
            56,
            "above last height -> kern[last]"
        );
        // A glyph or corner with no kern record contributes zero.
        assert_eq!(
            font.math_kern_at(v, MathKernCorner::TopRight, 0).unwrap(),
            0
        );
    }

    #[test]
    fn latinmodern_has_no_math_kern_info() {
        // latinmodern-math has no MathKernInfo, so all math kern values are zero.
        let Some(font) = load_system_font(LM_MATH) else {
            eprintln!("SKIP: {LM_MATH} not found");
            return;
        };
        let x = font.glyph_index('x').unwrap().unwrap();
        for corner in [
            MathKernCorner::TopRight,
            MathKernCorner::TopLeft,
            MathKernCorner::BottomRight,
            MathKernCorner::BottomLeft,
        ] {
            assert_eq!(font.math_kern_at(x, corner, 0).unwrap(), 0);
        }
    }
}
