//! XeTeX font spec parsing, the host font loader trait, and OpenType metrics and math access.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

pub use mathtex_ir::FontKey;
use mathtex_ir::{GlyphId, GlyphOutline, Length, OutlineCommand};

// SharedFace and the face accessors name these crates' types, so hosts must use these exact versions.
pub use rustybuzz;
pub use ttf_parser;

/// Loads font faces for the engine, the only font trait a host implements.
pub trait FontLoader {
    /// Returns the face a spec names, with a key the host chooses to identify it in the IR.
    fn load(&self, spec: &FontSpec) -> Result<FontData, FontError>;
}

impl<T> FontLoader for &T
where
    T: FontLoader + ?Sized,
{
    fn load(&self, spec: &FontSpec) -> Result<FontData, FontError> {
        (**self).load(spec)
    }
}

/// A parsed XeTeX font spec, the engine shapes with its name, script, language and features only.
#[derive(Clone, Debug, PartialEq)]
pub struct FontSpec {
    spec: String,
    name: String,
    file: bool,
    face_index: u32,
    variants: Vec<String>,
    size: Length,
    script: Option<[u8; 4]>,
    language: Option<String>,
    features: Vec<ShapeFeature>,
    options: Vec<(String, String)>,
    vertical: bool,
    unknown: Vec<String>,
}

impl FontSpec {
    /// Parses a spec as XeTeX's `splitFontName` and `loadOTfont` read it, `size` is the loaded size.
    #[must_use]
    pub fn parse(spec: &str, size: Length) -> Self {
        let (name, file, face_index, variant, features) = split_font_name(spec);
        let mut parsed = Self {
            spec: spec.to_string(),
            name,
            file,
            face_index,
            variants: variant
                .split('/')
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect(),
            size,
            script: None,
            language: None,
            features: Vec::new(),
            options: Vec::new(),
            vertical: false,
            unknown: Vec::new(),
        };
        for option in features.split([':', ';', ',']) {
            parsed.read_option(option.trim_start_matches([' ', '\t']));
        }
        parsed
    }

    fn read_option(&mut self, option: &str) {
        if option.is_empty() {
            return;
        }
        if let Some((key, value)) = option.split_once('=') {
            match key {
                "script" => return self.script = Some(ot_tag(value)),
                "language" => return self.language = Some(value.to_string()),
                "mapping" | "extend" | "slant" | "embolden" | "letterspace" | "color"
                | "shaper" => return self.options.push((key.to_string(), value.to_string())),
                _ => {}
            }
        }
        if let Some(rest) = option.strip_prefix('+') {
            // XeTeX's read_tag_with_param bumps a nonnegative `+tag=n` so alternate 0 selects the first one.
            let (tag, param) = rest
                .split_once('=')
                .map_or((rest, 0i64), |(tag, value)| (tag, leading_int(value)));
            let value = if param >= 0 { param + 1 } else { param };
            self.features.push(ShapeFeature {
                tag: ot_tag(tag),
                value: value as u32,
            });
        } else if let Some(tag) = option.strip_prefix('-') {
            self.features.push(ShapeFeature {
                tag: ot_tag(tag),
                value: 0,
            });
        } else if let Some((tag, value)) = option.split_once('=') {
            match value.trim().parse::<u32>() {
                Ok(value) => self.features.push(ShapeFeature {
                    tag: ot_tag(tag),
                    value,
                }),
                Err(_) => self.unknown.push(option.to_string()),
            }
        } else if option.trim_end() == "vertical" {
            self.vertical = true;
        } else {
            self.unknown.push(option.to_string());
        }
    }

    /// The spec exactly as TeX gave it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.spec
    }

    /// File name inside `[...]` without the face index, or the bare font name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the spec names a file with `[...]`, otherwise it is a XeTeX name lookup.
    #[must_use]
    pub fn is_file(&self) -> bool {
        self.file
    }

    /// Face index of a `[file:N]` spec, 0 otherwise.
    #[must_use]
    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Style and renderer suffixes after `/`, such as `B`, `I`, `OT` or `AAT`, which the engine ignores.
    #[must_use]
    pub fn variants(&self) -> &[String] {
        &self.variants
    }

    /// Size the font is loaded at.
    #[must_use]
    pub fn size(&self) -> Length {
        self.size
    }

    /// OpenType script tag from `script=`.
    #[must_use]
    pub fn script(&self) -> Option<[u8; 4]> {
        self.script
    }

    /// Language from `language=`.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// OpenType features from `+tag`, `-tag` and `tag=value`, in spec order.
    #[must_use]
    pub fn features(&self) -> &[ShapeFeature] {
        &self.features
    }

    /// Value of an option the engine ignores, such as `mapping`, `color`, `embolden` or `letterspace`.
    #[must_use]
    pub fn option(&self, key: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    /// Whether the spec asks for `vertical` layout, which the engine ignores.
    #[must_use]
    pub fn vertical(&self) -> bool {
        self.vertical
    }

    /// Options XeTeX would warn about as unknown.
    #[must_use]
    pub fn unknown_options(&self) -> &[String] {
        &self.unknown
    }

    /// The name, then with `.otf` and `.ttf` when extensionless, so bare TU default names resolve as files.
    #[must_use]
    pub fn file_candidates(&self) -> Vec<String> {
        let mut candidates = vec![self.name.clone()];
        let base = self.name.rsplit(['/', '\\']).next().unwrap_or(&self.name);
        if !base.contains('.') {
            candidates.push(format!("{}.otf", self.name));
            candidates.push(format!("{}.ttf", self.name));
        }
        candidates
    }
}

/// Splits a spec into name, file flag, face index, variant and feature text, as XeTeX's `splitFontName`.
fn split_font_name(spec: &str) -> (String, bool, u32, &str, &str) {
    if let Some(inner) = spec.strip_prefix('[') {
        let (path, after) = inner.split_once(']').unwrap_or((inner, ""));
        let (path, face_index) = match path.rsplit_once(':') {
            Some((path, index)) if index.bytes().all(|byte| byte.is_ascii_digit()) => {
                (path, index.parse().unwrap_or(0))
            }
            _ => (path, 0),
        };
        let (variant, features) = after.split_once(':').unwrap_or((after, ""));
        return (path.to_string(), true, face_index, variant, features);
    }
    let (head, features) = spec.split_once(':').unwrap_or((spec, ""));
    let (name, variant) = head.split_once('/').unwrap_or((head, ""));
    (name.to_string(), false, 0, variant, features)
}

/// Pads or truncates a tag to four bytes with spaces, as HarfBuzz's `hb_tag_from_string`.
fn ot_tag(tag: &str) -> [u8; 4] {
    let bytes = tag.trim().as_bytes();
    std::array::from_fn(|index| bytes.get(index).copied().unwrap_or(b' '))
}

/// Reads an optionally negative leading decimal, as XeTeX's feature parameter loop.
fn leading_int(text: &str) -> i64 {
    let (negative, digits) = text
        .strip_prefix('-')
        .map_or((false, text), |rest| (true, rest));
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i64, |value, digit| {
            value
                .saturating_mul(10)
                .saturating_add(i64::from(digit - b'0'))
        });
    if negative {
        -value
    } else {
        value
    }
}

/// An OpenType feature setting, such as `ssty=1` for math script size variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeFeature {
    /// Four byte OpenType feature tag.
    pub tag: [u8; 4],
    /// Feature value, 0 disables and 1 selects the first alternate.
    pub value: u32,
}

/// Loads fonts from an in process map keyed by file name.
#[derive(Clone, Debug, Default)]
pub struct InMemoryFontLoader {
    fonts: BTreeMap<String, FontData>,
    next_key: u64,
}

impl InMemoryFontLoader {
    /// Creates an empty loader.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a font by file name and returns the loader.
    #[must_use]
    pub fn with_font(mut self, file: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        self.insert(file, bytes);
        self
    }

    /// Adds or replaces a font by file name and returns its key, keys are never reused.
    pub fn insert(&mut self, file: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> FontKey {
        self.next_key += 1;
        let key = FontKey(self.next_key);
        self.fonts.insert(file.into(), FontData::new(key, bytes));
        key
    }

    /// Adds or replaces a font the host built, keeping the host's key.
    pub fn insert_font_data(&mut self, file: impl Into<String>, font: FontData) {
        self.fonts.insert(file.into(), font);
    }
}

impl FontLoader for InMemoryFontLoader {
    fn load(&self, spec: &FontSpec) -> Result<FontData, FontError> {
        spec.file_candidates()
            .iter()
            .find_map(|name| self.fonts.get(name))
            .cloned()
            .ok_or_else(|| FontError::NotFound {
                name: spec.name().to_string(),
            })
    }
}

/// Corner of an OpenType `MathKernInfo` record, as HarfBuzz's `hb_ot_math_kern_t`.
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

/// A larger OpenType math glyph variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathVariant {
    /// The variant glyph.
    pub glyph: GlyphId,
    /// Advance along the stretch axis in scaled points.
    pub advance: i32,
}

/// One part of an OpenType math glyph assembly, measurements in scaled points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MathAssemblyPart {
    /// The part's glyph.
    pub glyph: GlyphId,
    /// Connector overlap at the leading edge.
    pub start_connector: i32,
    /// Connector overlap at the trailing edge.
    pub end_connector: i32,
    /// Advance along the assembly axis.
    pub full_advance: i32,
    /// Whether the part repeats to reach the target size.
    pub extender: bool,
}

/// A face the host parsed and owns, the library borrows it and never parses or copies it.
pub trait SharedFace: Send + Sync {
    /// Returns the rustybuzz face the host parsed.
    fn rustybuzz_face(&self) -> &rustybuzz::Face<'_>;

    /// Returns the ttf view of the face, by default the one inside the rustybuzz face.
    fn ttf_face(&self) -> &ttf_parser::Face<'_> {
        self.rustybuzz_face()
    }
}

/// Where a font's parsed faces come from.
#[derive(Clone)]
enum FaceSource {
    Bytes {
        bytes: Arc<[u8]>,
        // Clones share these caches, so a face is parsed once per font rather than once per clone.
        ttf: Arc<OnceLock<ParsedFace>>,
        rustybuzz: Arc<OnceLock<ParsedRustybuzzFace>>,
    },
    Shared(Arc<dyn SharedFace>),
}

/// A loaded font face and the key the host chose for it.
#[derive(Clone)]
pub struct FontData {
    /// Host chosen identity, carried into the IR's font references.
    pub key: FontKey,
    source: FaceSource,
}

impl PartialEq for FontData {
    fn eq(&self, other: &Self) -> bool {
        let same_face = match (&self.source, &other.source) {
            (FaceSource::Bytes { bytes: a, .. }, FaceSource::Bytes { bytes: b, .. }) => a == b,
            (FaceSource::Shared(a), FaceSource::Shared(b)) => Arc::ptr_eq(a, b),
            _ => false,
        };
        self.key == other.key && same_face
    }
}

impl Eq for FontData {}

impl fmt::Debug for FontData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontData")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl FontData {
    /// Wraps raw font bytes, parsed lazily and at most once.
    #[must_use]
    pub fn new(key: FontKey, bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            key,
            source: FaceSource::Bytes {
                bytes: bytes.into(),
                ttf: Arc::default(),
                rustybuzz: Arc::default(),
            },
        }
    }

    /// Wraps a face the host already parsed, the library never parses it.
    #[must_use]
    pub fn from_shared_face(key: FontKey, face: Arc<dyn SharedFace>) -> Self {
        Self {
            key,
            source: FaceSource::Shared(face),
        }
    }

    /// Returns the raw bytes the library owns, `None` for host shared faces.
    #[must_use]
    pub fn bytes(&self) -> Option<&Arc<[u8]>> {
        match &self.source {
            FaceSource::Bytes { bytes, .. } => Some(bytes),
            FaceSource::Shared(_) => None,
        }
    }

    /// Runs `f` with the ttf face, parsed at most once and shared across clones.
    pub fn with_ttf_face<R>(
        &self,
        f: impl FnOnce(&ttf_parser::Face<'_>) -> R,
    ) -> Result<R, FontError> {
        Ok(f(self.ttf_face()?))
    }

    /// Runs `f` with the rustybuzz face, parsed at most once and shared across clones.
    pub fn with_rustybuzz_face<R>(
        &self,
        f: impl FnOnce(&rustybuzz::Face<'_>) -> R,
    ) -> Result<R, FontError> {
        Ok(f(self.rustybuzz_face()?))
    }

    /// Overall metrics at `size`, as XeTeX reads them for a native font.
    pub fn metrics(&self, size: Length) -> Result<FontMetrics, FontError> {
        let face = self.ttf_face()?;
        let upem = units_per_em(face);
        // XeTeX's getSlant is D2Fix(tan(-italicAngle)), italicAngle is the post table angle in degrees.
        let angle = f64::from(face.italic_angle());
        Ok(FontMetrics {
            ascent: scale_font_units(i32::from(face.ascender()), size, upem),
            descent: -scale_font_units(i32::from(face.descender()), size, upem),
            xheight: scale_font_units(i32::from(face.x_height().unwrap_or(0)), size, upem),
            capheight: scale_font_units(i32::from(face.capital_height().unwrap_or(0)), size, upem),
            slant: d2fix((-angle).to_radians().tan()),
        })
    }

    /// Width, height and depth of a glyph at `size`, from its advance and bounding box.
    pub fn glyph_metrics(
        &self,
        glyph: GlyphId,
        size: Length,
    ) -> Result<FontGlyphMetrics, FontError> {
        let face = self.ttf_face()?;
        let upem = units_per_em(face);
        let Some(glyph) = ttf_glyph(glyph) else {
            return Ok(FontGlyphMetrics::default());
        };
        let width = face.glyph_hor_advance(glyph).map_or(0, |advance| {
            scale_font_units(i32::from(advance), size, upem)
        });
        let (height, depth) = face.glyph_bounding_box(glyph).map_or((0, 0), |bbox| {
            (
                scale_font_units(i32::from(bbox.y_max).max(0), size, upem),
                scale_font_units((-i32::from(bbox.y_min)).max(0), size, upem),
            )
        });
        Ok(FontGlyphMetrics {
            width,
            height,
            depth,
        })
    }

    /// Advance and control box of a glyph in points at `size`, as XeTeX's `getGlyphBounds` reads them.
    pub fn glyph_bounds_points(
        &self,
        glyph: GlyphId,
        size: Length,
    ) -> Result<GlyphBoundsPoints, FontError> {
        let face = self.ttf_face()?;
        // XeTeX's unitsToPoints multiplies by the f32 point size before dividing by units per em.
        let upem = f32::from(face.units_per_em().max(1));
        let point_size = point_size(size);
        let points = |units: f32| (units * point_size) / upem;
        let Some(glyph) = ttf_glyph(glyph) else {
            return Ok(GlyphBoundsPoints::default());
        };
        let advance = face
            .glyph_hor_advance(glyph)
            .map_or(0.0, |advance| points(f32::from(advance)));
        Ok(face.glyph_bounding_box(glyph).map_or(
            GlyphBoundsPoints {
                advance,
                ..GlyphBoundsPoints::default()
            },
            |bbox| GlyphBoundsPoints {
                advance,
                x_min: points(f32::from(bbox.x_min)),
                y_min: points(f32::from(bbox.y_min)),
                x_max: points(f32::from(bbox.x_max)),
                y_max: points(f32::from(bbox.y_max)),
            },
        ))
    }

    /// Smallest and largest code points the Unicode cmaps map to a glyph, as FreeType's first and last char.
    pub fn char_code_range(&self) -> Result<Option<(u32, u32)>, FontError> {
        let face = self.ttf_face()?;
        let Some(cmap) = face.tables().cmap else {
            return Ok(None);
        };
        let mut range: Option<(u32, u32)> = None;
        for subtable in cmap.subtables {
            if !subtable.is_unicode() {
                continue;
            }
            subtable.codepoints(|codepoint| {
                if subtable
                    .glyph_index(codepoint)
                    .is_some_and(|glyph| glyph.0 != 0)
                {
                    range = Some(range.map_or((codepoint, codepoint), |(low, high)| {
                        (low.min(codepoint), high.max(codepoint))
                    }));
                }
            });
        }
        Ok(range)
    }

    /// Outlines in font design units with y up, one entry per glyph, `None` for blank glyphs.
    pub fn glyph_outlines(
        &self,
        glyphs: &[GlyphId],
    ) -> Result<Vec<Option<GlyphOutline>>, FontError> {
        let face = self.ttf_face()?;
        let units_per_em = face.units_per_em();
        Ok(glyphs
            .iter()
            .map(|&glyph| {
                let glyph = ttf_glyph(glyph)?;
                let mut collector = OutlineCollector {
                    commands: Vec::new(),
                };
                face.outline_glyph(glyph, &mut collector)?;
                Some(GlyphOutline {
                    units_per_em,
                    commands: collector.commands,
                })
            })
            .collect())
    }

    /// Looks a glyph up by Unicode code point.
    pub fn glyph_index(&self, codepoint: char) -> Result<Option<GlyphId>, FontError> {
        Ok(self
            .ttf_face()?
            .glyph_index(codepoint)
            .map(|glyph| GlyphId(u32::from(glyph.0))))
    }

    /// Looks a glyph up by glyph name.
    pub fn glyph_index_by_name(&self, name: &str) -> Result<Option<GlyphId>, FontError> {
        Ok(self
            .ttf_face()?
            .glyph_index_by_name(name)
            .map(|glyph| GlyphId(u32::from(glyph.0))))
    }

    /// Whether the font has an OpenType math table.
    pub fn has_opentype_math(&self) -> Result<bool, FontError> {
        Ok(self.ttf_face()?.tables().math.is_some())
    }

    /// Number of glyphs in the font, `\XeTeXcountglyphs`.
    pub fn ot_glyph_count(&self) -> Result<u32, FontError> {
        Ok(u32::from(self.ttf_face()?.number_of_glyphs()))
    }

    /// Number of scripts in the larger of the GSUB and GPOS script lists.
    pub fn ot_script_count(&self) -> Result<u32, FontError> {
        Ok(larger_script_list(self.ttf_face()?).map_or(0, |scripts| u32::from(scripts.len())))
    }

    /// Script tag at `index`, packed big endian as XeTeX's `hb_tag_t`.
    pub fn ot_script_tag(&self, index: u32) -> Result<u32, FontError> {
        let Ok(index) = u16::try_from(index) else {
            return Ok(0);
        };
        Ok(larger_script_list(self.ttf_face()?)
            .and_then(|scripts| scripts.get(index))
            .map_or(0, |script| script.tag.0))
    }

    /// Number of languages under `script_tag`, summed across GSUB and GPOS.
    pub fn ot_language_count(&self, script_tag: u32) -> Result<u32, FontError> {
        let face = self.ttf_face()?;
        let script_tag = ttf_parser::Tag(script_tag);
        let mut count = 0u32;
        for table in layout_tables(face) {
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

    /// Language tag at `index` under `script_tag`, `\XeTeXOTlanguagetag`.
    pub fn ot_language_tag(&self, script_tag: u32, index: u32) -> Result<u32, FontError> {
        let Ok(index) = u16::try_from(index) else {
            return Ok(0);
        };
        let face = self.ttf_face()?;
        let script_tag = ttf_parser::Tag(script_tag);
        for table in layout_tables(face) {
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

    /// Number of features under `script_tag` and `language_tag`, summed across GSUB and GPOS.
    pub fn ot_feature_count(&self, script_tag: u32, language_tag: u32) -> Result<u32, FontError> {
        let face = self.ttf_face()?;
        let script_tag = ttf_parser::Tag(script_tag);
        let mut count = 0u32;
        for table in layout_tables(face) {
            if let Some(langsys) = language_system(table, script_tag, language_tag) {
                count += u32::from(langsys.feature_indices.len());
            }
        }
        Ok(count)
    }

    /// Feature tag at `index` under `script_tag` and `language_tag`, `\XeTeXOTfeaturetag`.
    pub fn ot_feature_tag(
        &self,
        script_tag: u32,
        language_tag: u32,
        index: u32,
    ) -> Result<u32, FontError> {
        let Ok(index) = u16::try_from(index) else {
            return Ok(0);
        };
        let face = self.ttf_face()?;
        let script_tag = ttf_parser::Tag(script_tag);
        for table in layout_tables(face) {
            if let Some(langsys) = language_system(table, script_tag, language_tag) {
                if let Some(feature_index) = langsys.feature_indices.get(index) {
                    return Ok(table
                        .features
                        .get(feature_index)
                        .map_or(0, |feature| feature.tag.0));
                }
            }
        }
        Ok(0)
    }

    /// OpenType math constant by XeTeX and HarfBuzz index, in scaled points or percent.
    pub fn opentype_math_constant(&self, constant: i32, size: Length) -> Result<i32, FontError> {
        let face = self.ttf_face()?;
        let Some(constants) = face.tables().math.and_then(|table| table.constants) else {
            return Ok(0);
        };
        let Some(value) = math_constant_value(constants, constant) else {
            return Ok(0);
        };
        if is_math_constant_percentage(constant) {
            Ok(value)
        } else {
            Ok(scale_font_units(value, size, units_per_em(face)))
        }
    }

    /// OpenType math italic correction of a glyph, in scaled points.
    pub fn math_italic_correction(&self, glyph: GlyphId, size: Length) -> Result<i32, FontError> {
        let face = self.ttf_face()?;
        let value = ttf_glyph(glyph).and_then(|glyph| {
            face.tables()
                .math?
                .glyph_info?
                .italic_corrections?
                .get(glyph)
        });
        Ok(value.map_or(0, |value| {
            scale_font_units(i32::from(value.value), size, units_per_em(face))
        }))
    }

    /// One `MathKernInfo` corner at a correction height in scaled points, in scaled points.
    pub fn math_kern_at(
        &self,
        glyph: GlyphId,
        corner: MathKernCorner,
        correction_height: i32,
        size: Length,
    ) -> Result<i32, FontError> {
        let height = self.points_to_units((correction_height as f32) / 65536.0, size)? as i32;
        let kern = self.math_kern_units(glyph, corner, height)?;
        self.units_to_scaled(kern, size)
    }

    /// One `MathKernInfo` corner in font design units, XeTeX sums kerns in these units before scaling.
    pub fn math_kern_units(
        &self,
        glyph: GlyphId,
        corner: MathKernCorner,
        correction_height: i32,
    ) -> Result<i32, FontError> {
        let face = self.ttf_face()?;
        let Some(kern_info) = ttf_glyph(glyph)
            .and_then(|glyph| face.tables().math?.glyph_info?.kern_infos?.get(glyph))
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
        let mut index = 0u16;
        while index < kern.count() {
            match kern.height(index) {
                Some(height) if correction_height < i32::from(height.value) => break,
                _ => index += 1,
            }
        }
        Ok(kern.kern(index).map_or(0, |value| i32::from(value.value)))
    }

    /// The larger math glyph variant at `index`, `None` past the last one.
    pub fn math_variant(
        &self,
        glyph: GlyphId,
        index: u16,
        horizontal: bool,
        size: Length,
    ) -> Result<Option<MathVariant>, FontError> {
        let face = self.ttf_face()?;
        let variant = ttf_glyph(glyph).and_then(|glyph| {
            math_constructions(face, horizontal)?
                .get(glyph)?
                .variants
                .get(index)
        });
        Ok(variant.map(|variant| MathVariant {
            glyph: GlyphId(u32::from(variant.variant_glyph.0)),
            advance: scale_font_units(
                i32::from(variant.advance_measurement),
                size,
                units_per_em(face),
            ),
        }))
    }

    /// Math glyph assembly parts, measurements in scaled points, empty when the glyph has none.
    pub fn math_assembly(
        &self,
        glyph: GlyphId,
        horizontal: bool,
        size: Length,
    ) -> Result<Vec<MathAssemblyPart>, FontError> {
        let face = self.ttf_face()?;
        let upem = units_per_em(face);
        let Some(assembly) = ttf_glyph(glyph)
            .and_then(|glyph| math_constructions(face, horizontal)?.get(glyph)?.assembly)
        else {
            return Ok(Vec::new());
        };
        let scale = |units: u16| scale_font_units(i32::from(units), size, upem);
        Ok(assembly
            .parts
            .into_iter()
            .map(|part| MathAssemblyPart {
                glyph: GlyphId(u32::from(part.glyph_id.0)),
                start_connector: scale(part.start_connector_length),
                end_connector: scale(part.end_connector_length),
                full_advance: scale(part.full_advance),
                extender: part.part_flags.extender(),
            })
            .collect())
    }

    /// Minimum connector overlap between assembly parts, in scaled points.
    pub fn math_min_connector_overlap(&self, size: Length) -> Result<i32, FontError> {
        let face = self.ttf_face()?;
        let overlap = face
            .tables()
            .math
            .and_then(|table| table.variants)
            .map_or(0, |variants| i32::from(variants.min_connector_overlap));
        Ok(scale_font_units(overlap, size, units_per_em(face)))
    }

    /// Converts points to design units in f32, as XeTeX's `pointsToUnits`.
    pub fn points_to_units(&self, points: f32, size: Length) -> Result<f32, FontError> {
        let upem = units_per_em(self.ttf_face()?);
        let point_size = point_size(size);
        if point_size == 0.0 {
            return Ok(0.0);
        }
        Ok((points * upem as f32) / point_size)
    }

    /// Converts design units to scaled points, as XeTeX's `D2Fix(unitsToPoints(units))`.
    pub fn units_to_scaled(&self, units: i32, size: Length) -> Result<i32, FontError> {
        Ok(scale_font_units(
            units,
            size,
            units_per_em(self.ttf_face()?),
        ))
    }

    /// Top accent attachment of a glyph in scaled points, `\XeTeXmathaccent`.
    pub fn opentype_math_accent_position(
        &self,
        glyph: GlyphId,
        size: Length,
    ) -> Result<i32, FontError> {
        let face = self.ttf_face()?;
        let value = ttf_glyph(glyph).and_then(|glyph| {
            face.tables()
                .math?
                .glyph_info?
                .top_accent_attachments?
                .get(glyph)
        });
        Ok(value.map_or(0, |value| {
            scale_font_units(i32::from(value.value), size, units_per_em(face))
        }))
    }

    /// Symbol font parameter mapped to OpenType math constants, as XeTeX's `get_native_mathsy_param`.
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

    /// Extension font parameter mapped to OpenType math constants, as XeTeX's `get_native_mathex_param`.
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

    fn invalid(&self, message: String) -> FontError {
        FontError::Invalid {
            name: format!("font key {}", self.key.0),
            message,
        }
    }

    fn ttf_face(&self) -> Result<&ttf_parser::Face<'_>, FontError> {
        match &self.source {
            FaceSource::Shared(face) => Ok(face.ttf_face()),
            FaceSource::Bytes { bytes, ttf, .. } => {
                if ttf.get().is_none() {
                    let parsed = ParsedFace::try_new(Arc::clone(bytes), |bytes| {
                        ttf_parser::Face::parse(bytes, 0)
                            .map_err(|error| self.invalid(format!("invalid font data: {error}")))
                    })?;
                    // A racing initializer may win, either way the cell is populated afterwards.
                    let _ = ttf.set(parsed);
                }
                Ok(ttf
                    .get()
                    .expect("ttf cache populated above")
                    .borrow_dependent())
            }
        }
    }

    fn rustybuzz_face(&self) -> Result<&rustybuzz::Face<'_>, FontError> {
        match &self.source {
            FaceSource::Shared(face) => Ok(face.rustybuzz_face()),
            FaceSource::Bytes {
                bytes, rustybuzz, ..
            } => {
                if rustybuzz.get().is_none() {
                    let parsed = ParsedRustybuzzFace::try_new(Arc::clone(bytes), |bytes| {
                        rustybuzz::Face::from_slice(bytes, 0)
                            .ok_or_else(|| self.invalid("invalid font data".to_string()))
                    })?;
                    let _ = rustybuzz.set(parsed);
                }
                Ok(rustybuzz
                    .get()
                    .expect("rustybuzz cache populated above")
                    .borrow_dependent())
            }
        }
    }
}

/// Overall font metrics in scaled points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontMetrics {
    /// Distance from the baseline up to the ascender line.
    pub ascent: i32,
    /// Distance from the baseline down to the descender line, positive like a TeX depth.
    pub descent: i32,
    /// Height of a lowercase x.
    pub xheight: i32,
    /// Height of a capital letter.
    pub capheight: i32,
    /// Slant as a 16.16 fixed point ratio, `\fontdimen1`.
    pub slant: i32,
}

/// Metrics of one glyph in scaled points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontGlyphMetrics {
    /// Horizontal advance.
    pub width: i32,
    /// Height above the baseline.
    pub height: i32,
    /// Depth below the baseline.
    pub depth: i32,
}

/// A glyph's advance and control box in points, y up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphBoundsPoints {
    /// Horizontal advance.
    pub advance: f32,
    /// Left edge of the control box.
    pub x_min: f32,
    /// Bottom edge of the control box.
    pub y_min: f32,
    /// Right edge of the control box.
    pub x_max: f32,
    /// Top edge of the control box.
    pub y_max: f32,
}

/// Font loading or parsing failure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontError {
    /// The loader has no font for the spec.
    NotFound {
        /// The name the spec asked for.
        name: String,
    },
    /// The font exists but is unusable.
    Invalid {
        /// The font's name or key.
        name: String,
        /// Why the font is unusable.
        message: String,
    },
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "font not found: {name}"),
            Self::Invalid { name, message } => write!(f, "font {name} is unusable: {message}"),
        }
    }
}

impl std::error::Error for FontError {}

/// Names the self_cell dependent's lifetime.
type TtfFace<'a> = ttf_parser::Face<'a>;

self_cell::self_cell!(
    /// Owns font bytes together with the ttf face parsed from them.
    struct ParsedFace {
        owner: Arc<[u8]>,
        #[covariant]
        dependent: TtfFace,
    }
);

/// Names the self_cell dependent's lifetime.
type RbFace<'a> = rustybuzz::Face<'a>;

self_cell::self_cell!(
    /// Owns font bytes together with the rustybuzz face parsed from them.
    struct ParsedRustybuzzFace {
        owner: Arc<[u8]>,
        #[covariant]
        dependent: RbFace,
    }
);

fn units_per_em(face: &ttf_parser::Face<'_>) -> i32 {
    i32::from(face.units_per_em()).max(1)
}

fn ttf_glyph(glyph: GlyphId) -> Option<ttf_parser::GlyphId> {
    u16::try_from(glyph.0).ok().map(ttf_parser::GlyphId)
}

/// XeTeX keeps the point size as f32 in `XeTeXFontInst`.
fn point_size(size: Length) -> f32 {
    (f64::from(size.0) / 65536.0) as f32
}

/// XeTeX's D2Fix, a C cast truncates toward zero after adding one half.
fn d2fix(value: f64) -> i32 {
    (value * 65536.0 + 0.5)
        .trunc()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// Scales design units to scaled points as XeTeX's `D2Fix(unitsToPoints(value))`.
fn scale_font_units(value: i32, size: Length, units_per_em: i32) -> i32 {
    let points = (value as f32 * point_size(size)) / (units_per_em.max(1) as f32);
    d2fix(f64::from(points))
}

fn layout_tables<'a>(
    face: &ttf_parser::Face<'a>,
) -> impl Iterator<Item = ttf_parser::opentype_layout::LayoutTable<'a>> {
    [face.tables().gsub, face.tables().gpos]
        .into_iter()
        .flatten()
}

/// The larger of the GSUB and GPOS script lists, as XeTeX's `getLargerScriptListTable`.
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

/// Resolves a language system, `language_tag == 0` selects the script default.
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

fn math_constructions<'a>(
    face: &ttf_parser::Face<'a>,
    horizontal: bool,
) -> Option<ttf_parser::math::GlyphConstructions<'a>> {
    let variants = face.tables().math?.variants?;
    Some(if horizontal {
        variants.horizontal_constructions
    } else {
        variants.vertical_constructions
    })
}

/// Collects ttf-parser contour callbacks into [`OutlineCommand`]s.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const PT: i32 = 65_536;
    const LM_MATH: &str = "fonts/opentype/public/lm-math/latinmodern-math.otf";
    const LM_ITALIC: &str = "fonts/opentype/public/lm/lmroman10-italic.otf";
    const STIX_MATH: &str = "fonts/opentype/public/stix2-otf/STIXTwoMath-Regular.otf";

    /// Reads a font under `MATHTEX_TEXMF_ROOT`, `None` when the variable is unset.
    fn texlive_font(relative: &str) -> Option<FontData> {
        let Some(root) = std::env::var_os("MATHTEX_TEXMF_ROOT") else {
            assert!(
                std::env::var("MATHTEX_REQUIRE_TEXLIVE").as_deref() != Ok("1"),
                "MATHTEX_REQUIRE_TEXLIVE=1 but MATHTEX_TEXMF_ROOT is not set"
            );
            eprintln!("skipping: MATHTEX_TEXMF_ROOT is not set");
            return None;
        };
        let path = Path::new(&root).join(relative);
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "MATHTEX_TEXMF_ROOT is set but {} is unreadable: {error}",
                path.display()
            )
        });
        Some(FontData::new(FontKey(1), bytes))
    }

    /// Reads a font from the texlive-source fixtures, `None` after a skip notice.
    fn fixture_font(relative: &str) -> Option<FontData> {
        mathtex_test_fixtures::read(relative).map(|bytes| FontData::new(FontKey(2), bytes))
    }

    fn ten_pt() -> Length {
        Length(10 * PT)
    }

    #[test]
    fn spec_parses_files_features_and_suffixes() {
        let spec = FontSpec::parse(
            "[latinmodern-math.otf]:script=math;+ssty=0;-liga,language=DEU;mapping=tex-text;kern=2",
            ten_pt(),
        );
        assert!(spec.is_file());
        assert_eq!(spec.name(), "latinmodern-math.otf");
        assert_eq!(spec.script(), Some(*b"math"));
        assert_eq!(spec.language(), Some("DEU"));
        assert_eq!(spec.option("mapping"), Some("tex-text"));
        assert_eq!(
            spec.features(),
            [
                ShapeFeature {
                    tag: *b"ssty",
                    value: 1
                },
                ShapeFeature {
                    tag: *b"liga",
                    value: 0
                },
                ShapeFeature {
                    tag: *b"kern",
                    value: 2
                },
            ]
        );
        assert_eq!(spec.file_candidates(), ["latinmodern-math.otf"]);

        let spec = FontSpec::parse("Latin Modern Roman/B/OT:+smcp;vertical;bogus", ten_pt());
        assert!(!spec.is_file());
        assert_eq!(spec.name(), "Latin Modern Roman");
        assert_eq!(spec.variants(), ["B", "OT"]);
        assert_eq!(
            spec.features(),
            [ShapeFeature {
                tag: *b"smcp",
                value: 1
            }]
        );
        assert!(spec.vertical());
        assert_eq!(spec.unknown_options(), ["bogus"]);

        let spec = FontSpec::parse("[fonts/collection.ttc:2]/AAT:color=FF0000", ten_pt());
        assert_eq!(spec.name(), "fonts/collection.ttc");
        assert_eq!(spec.face_index(), 2);
        assert_eq!(spec.variants(), ["AAT"]);
        assert_eq!(spec.option("color"), Some("FF0000"));
        assert_eq!(spec.size(), ten_pt());

        let spec = FontSpec::parse("lmroman10-regular:+tlig=-1", ten_pt());
        assert_eq!(
            spec.file_candidates(),
            [
                "lmroman10-regular",
                "lmroman10-regular.otf",
                "lmroman10-regular.ttf"
            ]
        );
        assert_eq!(spec.features()[0].value, u32::MAX);
    }

    #[test]
    fn in_memory_loader_resolves_candidates_and_never_reuses_keys() {
        let mut fonts = InMemoryFontLoader::new();
        let first = fonts.insert("a.otf", b"one".to_vec());
        let second = fonts.insert("b.otf", b"two".to_vec());
        let replaced = fonts.insert("a.otf", b"three".to_vec());
        assert_ne!(first, second);
        assert_ne!(replaced, first);
        assert_ne!(replaced, second);

        let font = fonts.load(&FontSpec::parse("a", ten_pt())).expect("a.otf");
        assert_eq!(font.key, replaced);
        assert_eq!(&**font.bytes().expect("owned bytes"), b"three");
        assert_eq!(
            fonts.load(&FontSpec::parse("[missing.otf]", ten_pt())),
            Err(FontError::NotFound {
                name: "missing.otf".into()
            })
        );
    }

    #[test]
    fn scale_font_units_matches_xetex_d2fix_rounding() {
        // The latinmodern-math `2` rounds 436469.76sp up to 436470 as XeTeX does.
        assert_eq!(scale_font_units(666, ten_pt(), 1000), 436_470);
        assert_eq!(scale_font_units(431, ten_pt(), 1000), 282_460);
        assert_eq!(scale_font_units(528, ten_pt(), 1000), 346_030);
        assert_eq!(scale_font_units(16, ten_pt(), 1000), 10_486);
        // D2Fix truncates after adding one half, so minus 10485.26 becomes minus 10485.
        assert_eq!(scale_font_units(-16, ten_pt(), 1000), -10_485);
        assert_eq!(scale_font_units(0, ten_pt(), 1000), 0);
    }

    #[test]
    fn font_data_clones_share_both_parsed_faces() {
        let Some(font) = fixture_font(mathtex_test_fixtures::DEJAVU_SANS) else {
            return;
        };
        let clone = font.clone();
        let ttf = |font: &FontData| {
            font.with_ttf_face(|face| face as *const ttf_parser::Face<'_> as usize)
                .expect("ttf parse")
        };
        let rb = |font: &FontData| {
            font.with_rustybuzz_face(|face| face as *const rustybuzz::Face<'_> as usize)
                .expect("rustybuzz parse")
        };
        assert_eq!(ttf(&font), ttf(&clone));
        assert_eq!(rb(&font), rb(&clone));
        assert!(Arc::ptr_eq(
            font.bytes().expect("owned bytes"),
            clone.bytes().expect("owned bytes")
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
    fn host_owned_face_is_borrowed_and_never_parsed_by_the_library() {
        let Some(owned) = fixture_font(mathtex_test_fixtures::LM_MONO) else {
            return;
        };
        let bytes = Arc::clone(owned.bytes().expect("owned bytes"));
        let app_face = AppOwnedFace::try_new(bytes, |bytes| {
            rustybuzz::Face::from_slice(bytes, 0).ok_or("host parse failed")
        })
        .expect("host parses its own font");
        let app_face: Arc<dyn SharedFace> = Arc::new(AppShared(app_face));
        let app_ptr = app_face.rustybuzz_face() as *const rustybuzz::Face<'_> as usize;
        let font = FontData::from_shared_face(FontKey(3), Arc::clone(&app_face));

        assert!(font.bytes().is_none());
        let lib_ptr = font
            .with_rustybuzz_face(|face| face as *const rustybuzz::Face<'_> as usize)
            .expect("borrow shared face");
        assert_eq!(lib_ptr, app_ptr);
        assert!(font.metrics(ten_pt()).expect("metrics").ascent > 0);
    }

    #[test]
    fn font_data_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FontData>();
    }

    #[test]
    fn metrics_slant_is_the_tangent_of_the_italic_angle_and_descent_is_positive() {
        let (Some(italic), Some(math)) = (texlive_font(LM_ITALIC), texlive_font(LM_MATH)) else {
            return;
        };
        // xetex reports \fontdimen1 of lmroman10-italic.otf as 0.25pt.
        assert_eq!(italic.metrics(ten_pt()).expect("metrics").slant, PT / 4);
        let metrics = math.metrics(ten_pt()).expect("metrics");
        assert_eq!(metrics.slant, 0);
        assert!(metrics.descent > 0, "descent {}", metrics.descent);
    }

    #[test]
    fn glyph_outlines_extract_design_unit_contours_from_cff_and_truetype() {
        for (file, units_per_em) in [
            (mathtex_test_fixtures::LM_MONO, 1000),
            (mathtex_test_fixtures::DEJAVU_SANS, 2048),
        ] {
            let Some(font) = fixture_font(file) else {
                return;
            };
            let x = font.glyph_index('x').unwrap().unwrap();
            let space = font.glyph_index(' ').unwrap().unwrap();
            let outlines = font.glyph_outlines(&[x, space]).unwrap();
            let x_outline = outlines[0].as_ref().expect("x has an outline");
            assert_eq!(x_outline.units_per_em, units_per_em, "{file}");
            assert!(matches!(
                x_outline.commands[0],
                OutlineCommand::MoveTo { .. }
            ));
            assert!(outlines[1].is_none(), "space has no outline in {file}");
            // CFF draws curves as cubics and TrueType as quadratics.
            let o = font.glyph_index('o').unwrap().unwrap();
            let o_outline = font.glyph_outlines(&[o]).unwrap().remove(0).expect("o");
            let cubic =
                |command: &OutlineCommand| matches!(command, OutlineCommand::CurveTo { .. });
            let quadratic =
                |command: &OutlineCommand| matches!(command, OutlineCommand::QuadTo { .. });
            let cff = file == mathtex_test_fixtures::LM_MONO;
            assert_eq!(o_outline.commands.iter().any(cubic), cff, "{file}");
            assert_eq!(o_outline.commands.iter().any(quadratic), !cff, "{file}");
        }
    }

    #[test]
    fn math_variant_returns_larger_paren_glyphs_from_latinmodern() {
        let Some(font) = texlive_font(LM_MATH) else {
            return;
        };
        // Glyph 9 vertical variant 4 is glyph 2433 advancing 1175061sp at 10pt.
        let paren = font.glyph_index('(').unwrap().unwrap();
        assert_eq!(paren, GlyphId(9));
        assert_eq!(
            font.math_variant(paren, 4, false, ten_pt()).unwrap(),
            Some(MathVariant {
                glyph: GlyphId(2433),
                advance: 1_175_061
            })
        );
        assert_eq!(
            font.math_variant(paren, 0, false, ten_pt()).unwrap(),
            Some(MathVariant {
                glyph: GlyphId(9),
                advance: 653_394
            })
        );
        assert_eq!(font.math_variant(paren, 99, false, ten_pt()).unwrap(), None);
    }

    #[test]
    fn math_assembly_returns_paren_parts_from_latinmodern() {
        let Some(font) = texlive_font(LM_MATH) else {
            return;
        };
        let paren = font.glyph_index('(').unwrap().unwrap();
        let parts = font.math_assembly(paren, false, ten_pt()).unwrap();
        let part =
            |glyph, start_connector, end_connector, full_advance, extender| MathAssemblyPart {
                glyph: GlyphId(glyph),
                start_connector,
                end_connector,
                full_advance,
                extender,
            };
        // Bottom 2503, extender 2504, top 2505, connectors of 249 and 498 units, advances of 1495 and 498.
        assert_eq!(
            parts,
            [
                part(2503, 0, 163_185, 979_763, false),
                part(2504, 326_369, 326_369, 326_369, true),
                part(2505, 163_185, 0, 979_763, false),
            ]
        );
        // A minimum overlap of 20 units is 13107sp at 10pt.
        assert_eq!(font.math_min_connector_overlap(ten_pt()).unwrap(), 13_107);
    }

    #[test]
    fn math_kern_reads_stix_cut_ins_in_units_and_scaled_points() {
        let Some(font) = texlive_font(STIX_MATH) else {
            return;
        };
        // `F` has one TopRight kern of 44 units, so every height returns it.
        let f = font.glyph_index('F').unwrap().unwrap();
        assert_eq!(
            font.math_kern_units(f, MathKernCorner::TopRight, 0)
                .unwrap(),
            44
        );
        assert_eq!(
            font.math_kern_units(f, MathKernCorner::TopRight, 100_000)
                .unwrap(),
            44
        );
        // `V` has BottomRight heights 126 and 280 with kerns -193, -119 and 56.
        let v = font.glyph_index('V').unwrap().unwrap();
        let bottom_right = |height| {
            font.math_kern_units(v, MathKernCorner::BottomRight, height)
                .unwrap()
        };
        assert_eq!(bottom_right(0), -193);
        assert_eq!(bottom_right(200), -119);
        assert_eq!(bottom_right(300), 56);
        assert_eq!(
            font.math_kern_units(v, MathKernCorner::TopRight, 0)
                .unwrap(),
            0
        );
        // 200 units is 2pt at 10pt in a 1000 unit font, and -119 units scales like any other length.
        let scaled = font
            .math_kern_at(v, MathKernCorner::BottomRight, 2 * PT, ten_pt())
            .unwrap();
        assert_eq!(scaled, scale_font_units(-119, ten_pt(), 1000));
    }

    #[test]
    fn latinmodern_has_no_math_kern_info() {
        let Some(font) = texlive_font(LM_MATH) else {
            return;
        };
        let x = font.glyph_index('x').unwrap().unwrap();
        for corner in [
            MathKernCorner::TopRight,
            MathKernCorner::TopLeft,
            MathKernCorner::BottomRight,
            MathKernCorner::BottomLeft,
        ] {
            assert_eq!(font.math_kern_units(x, corner, 0).unwrap(), 0);
        }
    }
}
