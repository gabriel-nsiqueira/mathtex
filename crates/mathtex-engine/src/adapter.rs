use std::cell::RefCell;
use std::collections::BTreeMap;

use mathtex_font::{FontData, FontError, FontLoader, FontSpec, MathKernCorner};
use mathtex_ir::{GlyphId, Length};
use mathtex_portable_engine_generated as pe;

use crate::host_box::{HostBox, HostBoxRequest, HostBoxes, MathStyle};
use crate::resource::{ResourceKind, ResourceProvider, ResourceRequest};
use crate::shaper;
use crate::typeset::{Diagnostic, DiagnosticKind};

/// Native font table entries a format keeps: handle, `\font` spec and size in scaled points.
pub(crate) type NativeFontTable = Vec<(pe::PortableFontHandle, String, i32)>;

#[derive(Clone, Debug)]
struct LoadedFont {
    data: FontData,
    /// The `\font` spec and the size the instance was loaded at.
    spec: FontSpec,
}

impl LoadedFont {
    fn size(&self) -> Length {
        self.spec.size()
    }
}

/// Native fonts the engine loaded, by the handle the TeX state knows each by.
#[derive(Clone, Debug)]
pub(crate) struct FontTable {
    next_handle: pe::PortableFontHandle,
    loaded: BTreeMap<pe::PortableFontHandle, LoadedFont>,
}

impl Default for FontTable {
    fn default() -> Self {
        Self {
            next_handle: 1,
            loaded: BTreeMap::new(),
        }
    }
}

impl FontTable {
    /// Loads every font of a format's table at its original handle.
    pub(crate) fn restore(
        &mut self,
        loader: &impl FontLoader,
        table: &[(pe::PortableFontHandle, String, i32)],
    ) -> Result<(), (String, FontError)> {
        for (handle, spec, size) in table {
            let parsed = FontSpec::parse(spec, Length(*size));
            let data = loader
                .load(&parsed)
                .map_err(|error| (spec.clone(), error))?;
            self.loaded
                .insert(*handle, LoadedFont { data, spec: parsed });
            self.next_handle = self.next_handle.max(handle.saturating_add(1));
        }
        Ok(())
    }

    /// The table a format keeps to rebind these fonts.
    pub(crate) fn snapshot(&self) -> NativeFontTable {
        self.loaded
            .iter()
            .map(|(&handle, font)| (handle, font.spec.as_str().into(), font.size().0))
            .collect()
    }

    /// Drops the fonts a run loaded past the format's own, handles restart after the format's.
    pub(crate) fn keep_only(&mut self, table: &[(pe::PortableFontHandle, String, i32)]) {
        self.loaded
            .retain(|handle, _| table.iter().any(|(kept, _, _)| kept == handle));
        self.next_handle = self
            .loaded
            .keys()
            .next_back()
            .map_or(1, |last| last.saturating_add(1));
    }

    /// Borrows the table with a loader as the engine's native font platform for one run.
    pub(crate) fn platform<'a, L: FontLoader>(&'a mut self, loader: &'a L) -> NativeFonts<'a, L> {
        NativeFonts {
            loader,
            table: self,
        }
    }
}

/// XeTeX's native font platform over a [`FontTable`], loading through the host's [`FontLoader`].
pub(crate) struct NativeFonts<'a, L> {
    loader: &'a L,
    table: &'a mut FontTable,
}

impl<L> NativeFonts<'_, L> {
    fn font(&self, font: pe::PortableFontHandle) -> Option<&LoadedFont> {
        self.table.loaded.get(&font)
    }

    fn with_font<T: Default>(
        &self,
        font: pe::PortableFontHandle,
        f: impl FnOnce(&FontData, Length) -> Result<T, FontError>,
    ) -> T {
        self.font(font)
            .and_then(|loaded| f(&loaded.data, loaded.size()).ok())
            .unwrap_or_default()
    }
}

impl<L: FontLoader> pe::FontPlatform for NativeFonts<'_, L> {
    fn resolve_font_handle(&mut self, name: &[i32], size: i32) -> Option<pe::PortableFontHandle> {
        let spec = FontSpec::parse(&unicode_scalars_to_string(name), Length(size));
        let data = self.loader.load(&spec).ok()?;
        // A file that does not parse as OpenType is not a native font, XeTeX then tries a TFM.
        data.with_ttf_face(|_| ()).ok()?;
        let handle = self.table.next_handle;
        self.table.next_handle = handle.checked_add(1)?;
        self.table.loaded.insert(handle, LoadedFont { data, spec });
        Some(handle)
    }

    fn release_font_handle(&mut self, font: pe::PortableFontHandle, _type_flag: i32) {
        self.table.loaded.remove(&font);
    }

    fn font_table(&self) -> Vec<(pe::PortableFontHandle, String, i32)> {
        self.table.snapshot()
    }

    fn restore_font_table(&mut self, table: &[(pe::PortableFontHandle, String, i32)]) -> bool {
        self.table.restore(self.loader, table).is_ok()
    }

    fn font_metrics(&mut self, font: pe::PortableFontHandle) -> pe::PortableFontMetrics {
        self.with_font(font, |data, size| {
            let metrics = data.metrics(size)?;
            // XeTeX's getAscentAndDescent gives the descent negative, the engine negates it into a depth.
            Ok(pe::PortableFontMetrics {
                ascent: metrics.ascent,
                descent: -metrics.descent,
                xheight: metrics.xheight,
                capheight: metrics.capheight,
                slant: metrics.slant,
            })
        })
    }

    fn opentype_font_metrics(&mut self, font: pe::PortableFontHandle) -> pe::PortableFontMetrics {
        self.font_metrics(font)
    }

    fn is_opentype_math_font(&mut self, font: pe::PortableFontHandle) -> bool {
        self.with_font(font, |data, _| data.has_opentype_math())
    }

    fn using_opentype(&mut self, font: pe::PortableFontHandle) -> bool {
        // Every font this platform loads is shaped as an OpenType font.
        self.table.loaded.contains_key(&font)
    }

    fn math_symbol_parameter(&mut self, font: pe::PortableFontHandle, parameter: i32) -> i32 {
        self.with_font(font, |data, size| {
            data.math_symbol_parameter(parameter, size)
        })
    }

    fn math_extension_parameter(&mut self, font: pe::PortableFontHandle, parameter: i32) -> i32 {
        self.with_font(font, |data, size| {
            data.math_extension_parameter(parameter, size)
        })
    }

    fn opentype_math_constant(&mut self, font: pe::PortableFontHandle, constant: i32) -> i32 {
        self.with_font(font, |data, size| {
            data.opentype_math_constant(constant, size)
        })
    }

    fn opentype_math_accent_position(&mut self, font: pe::PortableFontHandle, glyph: i32) -> i32 {
        let Some(glyph) = glyph_id(glyph) else {
            return 0;
        };
        self.with_font(font, |data, size| {
            data.opentype_math_accent_position(glyph, size)
        })
    }

    fn math_glyph_italic_correction(&mut self, font: pe::PortableFontHandle, glyph: i32) -> i32 {
        let Some(glyph) = glyph_id(glyph) else {
            return 0;
        };
        self.with_font(font, |data, size| data.math_italic_correction(glyph, size))
    }

    fn math_glyph_variant(
        &mut self,
        font: pe::PortableFontHandle,
        glyph: i32,
        index: u16,
        horizontal: bool,
    ) -> Option<pe::PortableMathVariant> {
        let glyph = glyph_id(glyph)?;
        let loaded = self.font(font)?;
        let variant = loaded
            .data
            .math_variant(glyph, index, horizontal, loaded.size())
            .ok()??;
        Some(pe::PortableMathVariant {
            glyph: glyph_i32(variant.glyph),
            advance: variant.advance,
        })
    }

    fn math_glyph_assembly(
        &mut self,
        font: pe::PortableFontHandle,
        glyph: i32,
        horizontal: bool,
    ) -> Vec<pe::PortableMathAssemblyPart> {
        let Some(glyph) = glyph_id(glyph) else {
            return Vec::new();
        };
        let parts = self.with_font(font, |data, size| {
            data.math_assembly(glyph, horizontal, size)
        });
        parts
            .into_iter()
            .map(|part| pe::PortableMathAssemblyPart {
                glyph: glyph_i32(part.glyph),
                start_connector: part.start_connector,
                end_connector: part.end_connector,
                full_advance: part.full_advance,
                extender: part.extender,
            })
            .collect()
    }

    fn math_min_connector_overlap(&mut self, font: pe::PortableFontHandle) -> i32 {
        self.with_font(font, |data, size| data.math_min_connector_overlap(size))
    }

    fn math_kern_at(
        &mut self,
        font: pe::PortableFontHandle,
        glyph: i32,
        corner: pe::PortableMathKernCorner,
        correction_height: i32,
    ) -> i32 {
        let Some(glyph) = glyph_id(glyph) else {
            return 0;
        };
        let corner = match corner {
            pe::PortableMathKernCorner::TopRight => MathKernCorner::TopRight,
            pe::PortableMathKernCorner::TopLeft => MathKernCorner::TopLeft,
            pe::PortableMathKernCorner::BottomRight => MathKernCorner::BottomRight,
            pe::PortableMathKernCorner::BottomLeft => MathKernCorner::BottomLeft,
        };
        // The engine sums kerns in design units as XeTeX's get_ot_math_kern does.
        self.with_font(font, |data, _| {
            data.math_kern_units(glyph, corner, correction_height)
        })
    }

    fn math_points_to_units(&mut self, font: pe::PortableFontHandle, points: f32) -> f32 {
        self.with_font(font, |data, size| data.points_to_units(points, size))
    }

    fn math_units_to_scaled(&mut self, font: pe::PortableFontHandle, units: i32) -> i32 {
        self.with_font(font, |data, size| data.units_to_scaled(units, size))
    }

    fn math_point_size(&mut self, font: pe::PortableFontHandle) -> f32 {
        self.font(font)
            .map_or(0.0, |font| (f64::from(font.size().0) / 65536.0) as f32)
    }

    fn map_char_to_glyph(&mut self, font: pe::PortableFontHandle, codepoint: i32) -> i32 {
        let Some(codepoint) = u32::try_from(codepoint).ok().and_then(char::from_u32) else {
            return 0;
        };
        self.with_font(font, |data, _| data.glyph_index(codepoint))
            .map_or(0, glyph_i32)
    }

    fn map_glyph_to_index(&mut self, font: pe::PortableFontHandle, name: &str) -> i32 {
        self.with_font(font, |data, _| data.glyph_index_by_name(name))
            .map_or(0, glyph_i32)
    }

    fn ot_font_get(
        &mut self,
        font: pe::PortableFontHandle,
        what: i32,
        param1: i32,
        param2: i32,
        param3: i32,
    ) -> i32 {
        let Some(loaded) = self.font(font) else {
            return 0;
        };
        let data = &loaded.data;
        // `what` is the XeTeX_ext OpenType layout selector, params are script and language tags or indices.
        let result = match what {
            1 => data.ot_glyph_count(),                  // XeTeX_count_glyphs
            16 => data.ot_script_count(),                // XeTeX_OT_count_scripts
            17 => data.ot_language_count(param1 as u32), // XeTeX_OT_count_languages
            18 => data.ot_feature_count(param1 as u32, param2 as u32), // XeTeX_OT_count_features
            19 => data.ot_script_tag(param1 as u32),     // XeTeX_OT_script_code
            20 => data.ot_language_tag(param1 as u32, param2 as u32), // XeTeX_OT_language_code
            21 => data.ot_feature_tag(param1 as u32, param2 as u32, param3 as u32), // XeTeX_OT_feature_code
            _ => return 0,
        };
        result.map_or(0, |value| value.min(i32::MAX as u32) as i32)
    }

    fn font_spec(&self, font: pe::PortableFontHandle) -> Option<String> {
        self.font(font).map(|loaded| loaded.spec.as_str().into())
    }

    fn font_key(&self, font: pe::PortableFontHandle) -> Option<u64> {
        self.font(font).map(|loaded| loaded.data.key.0)
    }

    fn shape_native_text(
        &mut self,
        font: pe::PortableFontHandle,
        text: &[u16],
        _use_glyph_metrics: bool,
    ) -> pe::PortableNativeTextMetrics {
        let Some(loaded) = self.font(font) else {
            return pe::PortableNativeTextMetrics::default();
        };
        let metrics = loaded.data.metrics(loaded.size()).unwrap_or_default();
        let text = utf16_to_string(text);
        let Ok((glyphs, width)) = shaper::shape(&loaded.data, &loaded.spec, loaded.size(), &text)
        else {
            return pe::PortableNativeTextMetrics {
                height: metrics.ascent,
                depth: metrics.descent,
                ..pe::PortableNativeTextMetrics::default()
            };
        };
        pe::PortableNativeTextMetrics {
            width,
            height: metrics.ascent,
            depth: metrics.descent,
            glyphs: glyphs
                .into_iter()
                .map(|glyph| pe::PortableNativeGlyph {
                    glyph_id: glyph.glyph,
                    x: glyph.x,
                    y: glyph.y,
                    advance: glyph.advance,
                    cluster_start: glyph.cluster_start,
                    cluster_end: glyph.cluster_end,
                    // Source spans are filled later from tracked character identifiers.
                    src_start: 0,
                    src_end: 0,
                })
                .collect(),
        }
    }

    fn measure_native_glyph(
        &mut self,
        font: pe::PortableFontHandle,
        glyph: u16,
        _use_glyph_metrics: bool,
    ) -> pe::PortableNativeGlyphMetrics {
        let metrics = self.with_font(font, |data, size| {
            data.glyph_metrics(GlyphId(u32::from(glyph)), size)
                .map(Some)
        });
        metrics.map_or_else(pe::PortableNativeGlyphMetrics::default, |metrics| {
            pe::PortableNativeGlyphMetrics {
                width: metrics.width,
                height: metrics.height,
                depth: metrics.depth,
            }
        })
    }

    fn glyph_bounds(
        &mut self,
        font: pe::PortableFontHandle,
        glyph: u16,
    ) -> pe::PortableGlyphBounds {
        let bounds = self.with_font(font, |data, size| {
            data.glyph_bounds_points(GlyphId(u32::from(glyph)), size)
        });
        pe::PortableGlyphBounds {
            advance: bounds.advance,
            x_min: bounds.x_min,
            y_min: bounds.y_min,
            x_max: bounds.x_max,
            y_max: bounds.y_max,
        }
    }

    fn char_code_range(&mut self, font: pe::PortableFontHandle) -> Option<(i32, i32)> {
        let (first, last) = self.with_font(font, |data, _| data.char_code_range())?;
        Some((
            i32::try_from(first).unwrap_or(i32::MAX),
            i32::try_from(last).unwrap_or(i32::MAX),
        ))
    }
}

fn glyph_id(glyph: i32) -> Option<GlyphId> {
    u32::try_from(glyph).ok().map(GlyphId)
}

fn glyph_i32(glyph: GlyphId) -> i32 {
    i32::try_from(glyph.0).unwrap_or(i32::MAX)
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

/// The resource kind of an engine file request, engine formats without a kind of their own read as TeX input.
fn resource_kind(kind: pe::ResourceKind) -> ResourceKind {
    match kind {
        pe::ResourceKind::TexInput | pe::ResourceKind::Other(_) => ResourceKind::TexInput,
        pe::ResourceKind::Package => ResourceKind::Package,
        pe::ResourceKind::Class => ResourceKind::Class,
        pe::ResourceKind::FontDefinition => ResourceKind::FontDefinition,
        pe::ResourceKind::PackageSupport => ResourceKind::PackageSupport,
        pe::ResourceKind::Font => ResourceKind::Font,
        pe::ResourceKind::Encoding => ResourceKind::Encoding,
        pe::ResourceKind::Map => ResourceKind::Map,
        pe::ResourceKind::Config => ResourceKind::Config,
        pe::ResourceKind::FormatImage => ResourceKind::FormatImage,
        pe::ResourceKind::Asset => ResourceKind::Asset,
    }
}

/// The name TeX asked for, then with each of the kind's suffixes when it has no extension.
fn candidate_names(name: &str, kind: ResourceKind) -> impl Iterator<Item = String> + '_ {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let suffixes = if base.contains('.') {
        &[][..]
    } else {
        kind.suffixes()
    };
    std::iter::once(name.to_string())
        .chain(suffixes.iter().map(move |suffix| format!("{name}{suffix}")))
}

/// Answers engine file requests from a [`ResourceProvider`], trying suffixed names for bare ones.
pub(crate) struct ProviderFiles<'a, R: ?Sized>(pub(crate) &'a R);

impl<R: ResourceProvider + ?Sized> pe::ResourceProvider for ProviderFiles<'_, R> {
    fn read(&mut self, request: pe::ResourceRequest<'_>) -> Option<Vec<u8>> {
        let mut name = request.name;
        while let Some(rest) = name.strip_prefix("./") {
            name = rest;
        }
        let kind = resource_kind(request.kind);
        let lookup = |kind: ResourceKind| {
            candidate_names(name, kind).find_map(|candidate| {
                let request = match (kind, request.package) {
                    (ResourceKind::Asset, Some(package)) => {
                        ResourceRequest::asset(package, candidate)
                    }
                    _ => ResourceRequest::new(candidate, kind),
                };
                self.0.read_request(&request).ok()
            })
        };
        let resource = lookup(kind).or_else(|| {
            // An asset a package did not register is often a plain TeX input beside it.
            (kind == ResourceKind::Asset).then(|| lookup(ResourceKind::TexInput))?
        })?;
        Some(resource.bytes)
    }
}

/// Answers the TFM requests of a typeset run through the host's [`FontLoader`], by file name with `.tfm`.
pub(crate) struct LoaderFiles<'a, L>(pub(crate) &'a L);

impl<L: FontLoader> pe::ResourceProvider for LoaderFiles<'_, L> {
    fn read(&mut self, request: pe::ResourceRequest<'_>) -> Option<Vec<u8>> {
        if resource_kind(request.kind) != ResourceKind::Font {
            return None;
        }
        let name = request.name.trim_start_matches("./");
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let file = if base.contains('.') {
            name.to_string()
        } else {
            format!("{name}.tfm")
        };
        let font = self.0.load(&FontSpec::parse(&file, Length::ZERO)).ok()?;
        font.bytes().map(|bytes| bytes.to_vec())
    }
}

/// What a typeset run asked the host for, and the warnings its answers raised.
#[derive(Debug, Default)]
pub(crate) struct HostBoxLog {
    /// Every token requested, in request order with repeats.
    pub(crate) tokens: Vec<u32>,
    pub(crate) warnings: Vec<Diagnostic>,
    /// A request carried a token no `u32` holds, so the run cannot be cached.
    pub(crate) invalid_token: bool,
}

/// The engine platform of a typeset run, answering `\hostbox` from the host's [`HostBoxes`].
pub(crate) struct HostBoxPlatform<'a> {
    pub(crate) boxes: &'a dyn HostBoxes,
    pub(crate) log: &'a RefCell<HostBoxLog>,
}

impl HostBoxPlatform<'_> {
    fn warn(&self, message: String) {
        self.log.borrow_mut().warnings.push(Diagnostic {
            kind: DiagnosticKind::HostBox,
            message,
        });
    }
}

impl pe::PortablePlatform for HostBoxPlatform<'_> {
    fn host_box(&mut self, request: pe::PortableHostBoxRequest) -> Option<pe::PortableHostBox> {
        let Ok(token) = u32::try_from(request.token) else {
            self.log.borrow_mut().invalid_token = true;
            self.warn(format!(
                "\\hostbox{{{}}} names no token, nothing is drawn",
                request.token
            ));
            return None;
        };
        self.log.borrow_mut().tokens.push(token);
        let style = match request.style {
            pe::PortableHostBoxStyle::Text => MathStyle::Text,
            pe::PortableHostBoxStyle::Script => MathStyle::Script,
            pe::PortableHostBoxStyle::ScriptScript => MathStyle::ScriptScript,
        };
        let request = HostBoxRequest::new(token, style, Length(request.font_size));
        let Some(host_box) = self.boxes.host_box(&request) else {
            self.warn(format!(
                "the host has no box for \\hostbox{{{token}}}, nothing is drawn"
            ));
            return None;
        };
        if let Some(problem) = negative_size(&host_box) {
            self.warn(format!(
                "the box for \\hostbox{{{token}}} has a negative {problem}, nothing is drawn"
            ));
            return None;
        }
        Some(portable_host_box(host_box))
    }
}

/// The first negative extent of a host box or one of its rules.
fn negative_size(host_box: &HostBox) -> Option<&'static str> {
    let zero = Length::ZERO;
    if host_box.width < zero || host_box.height < zero || host_box.depth < zero {
        return Some("size");
    }
    host_box
        .rules
        .iter()
        .any(|rule| rule.width < zero || rule.height < zero)
        .then_some("rule")
}

fn portable_host_box(host_box: HostBox) -> pe::PortableHostBox {
    pe::PortableHostBox {
        width: host_box.width.0,
        height: host_box.height.0,
        depth: host_box.depth.0,
        runs: host_box
            .runs
            .into_iter()
            .map(|run| pe::PortableHostBoxRun {
                font_key: run.font.0,
                font_size: run.size.0,
                glyphs: run
                    .glyphs
                    .into_iter()
                    .map(|glyph| pe::PortableHostBoxGlyph {
                        glyph: glyph.glyph.0,
                        x: glyph.origin.x.0,
                        y: glyph.origin.y.0,
                    })
                    .collect(),
            })
            .collect(),
        rules: host_box
            .rules
            .into_iter()
            .map(|rule| pe::PortableHostBoxRule {
                x: rule.origin.x.0,
                y: rule.origin.y.0,
                width: rule.width.0,
                height: rule.height.0,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryResourceProvider;

    fn request<'a>(
        name: &'a str,
        kind: pe::ResourceKind,
        package: Option<&'a str>,
    ) -> pe::ResourceRequest<'a> {
        pe::ResourceRequest {
            name,
            kind,
            package,
            format: 26,
            mode: "rb",
            source: None,
        }
    }

    #[test]
    fn provider_files_try_the_kind_suffixes_and_package_assets() {
        let mut resources = InMemoryResourceProvider::new()
            .with_resource("amsmath.sty", ResourceKind::Package, b"pkg")
            .with_resource("data.dat", ResourceKind::TexInput, b"plain");
        resources.insert(ResourceRequest::asset("mhchem", "arrows.dat"), b"asset");
        let mut files = ProviderFiles(&resources);

        let read = |files: &mut ProviderFiles<'_, _>, name, kind, package| {
            pe::ResourceProvider::read(files, request(name, kind, package))
        };
        assert_eq!(
            read(&mut files, "./amsmath", pe::ResourceKind::Package, None),
            Some(b"pkg".to_vec())
        );
        assert_eq!(
            read(
                &mut files,
                "arrows.dat",
                pe::ResourceKind::Asset,
                Some("mhchem")
            ),
            Some(b"asset".to_vec())
        );
        assert_eq!(
            read(
                &mut files,
                "data.dat",
                pe::ResourceKind::Asset,
                Some("other")
            ),
            Some(b"plain".to_vec())
        );
        assert_eq!(
            read(&mut files, "amsmath.cls", pe::ResourceKind::Package, None),
            None
        );
    }
}
