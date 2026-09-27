use mathtex_font::rustybuzz;
use mathtex_font::ttf_parser::Tag;
use mathtex_font::{FontData, FontError, FontSpec};
use mathtex_ir::Length;

/// A shaped glyph as XeTeX stores it in a native word node, positions in scaled points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShapedGlyph {
    pub(crate) glyph: u16,
    /// Offset from the word origin, y down.
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) advance: i32,
    /// UTF-8 byte span of `text` the glyph's cluster covers.
    pub(crate) cluster_start: u32,
    pub(crate) cluster_end: u32,
}

/// Shapes `text` with the spec's script, language and features at `size`, returning glyphs and width.
pub(crate) fn shape(
    font: &FontData,
    spec: &FontSpec,
    size: Length,
    text: &str,
) -> Result<(Vec<ShapedGlyph>, i32), FontError> {
    let (raw, total) = font.with_rustybuzz_face(|face| {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.set_direction(rustybuzz::Direction::LeftToRight);
        buffer.guess_segment_properties();
        if let Some(script) = spec.script() {
            // from_iso15924_tag maps `math`, so rustybuzz picks the script that holds the ssty lookups.
            if let Some(script) = rustybuzz::Script::from_iso15924_tag(Tag::from_bytes(&script)) {
                buffer.set_script(script);
            }
        }
        if let Some(language) = spec.language().and_then(|tag| tag.parse().ok()) {
            buffer.set_language(language);
        }
        let features: Vec<rustybuzz::Feature> = spec
            .features()
            .iter()
            .map(|feature| {
                rustybuzz::Feature::new(Tag::from_bytes(&feature.tag), feature.value, ..)
            })
            .collect();
        let shaped = rustybuzz::shape(face, &features, buffer);
        // XeTeX's getGlyphPositions sums advances in design units and scales each position once.
        let mut pen = (0i32, 0i32);
        let mut raw = Vec::with_capacity(shaped.len());
        for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            raw.push((
                info.glyph_id,
                info.cluster,
                pen.0.saturating_add(position.x_offset),
                pen.1.saturating_add(position.y_offset),
                position.x_advance,
            ));
            pen.0 = pen.0.saturating_add(position.x_advance);
            pen.1 = pen.1.saturating_add(position.y_advance);
        }
        (raw, pen.0)
    })?;
    let scale = |units: i32| font.units_to_scaled(units, size);
    let mut glyphs = Vec::with_capacity(raw.len());
    for &(glyph, cluster, x, y, advance) in &raw {
        let (cluster_start, cluster_end) = cluster_span(&raw, cluster, text.len());
        glyphs.push(ShapedGlyph {
            glyph: u16::try_from(glyph).unwrap_or(0),
            x: scale(x)?,
            // HarfBuzz offsets grow upward, XeTeX negates them into its y down coordinates.
            y: scale(y.saturating_neg())?,
            advance: scale(advance)?,
            cluster_start,
            cluster_end,
        });
    }
    Ok((glyphs, scale(total)?))
}

/// A cluster covers every byte up to the next larger cluster start, both ends clamped to the text.
fn cluster_span(raw: &[(u32, u32, i32, i32, i32)], cluster: u32, len: usize) -> (u32, u32) {
    let len = u32::try_from(len).unwrap_or(u32::MAX);
    let start = cluster.min(len);
    let end = raw
        .iter()
        .map(|&(_, other, ..)| other.min(len))
        .filter(|&other| other > start)
        .min()
        .unwrap_or(len);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_cover_every_byte_up_to_the_next_cluster() {
        // A ligature `fi` at 0, an `x` at 2 and a two byte character at 3, with one glyph past the text.
        let raw = [
            (1, 0, 0, 0, 0),
            (2, 2, 0, 0, 0),
            (3, 3, 0, 0, 0),
            (4, 9, 0, 0, 0),
        ];
        assert_eq!(cluster_span(&raw, 0, 5), (0, 2));
        assert_eq!(cluster_span(&raw, 2, 5), (2, 3));
        assert_eq!(cluster_span(&raw, 3, 5), (3, 5));
        assert_eq!(cluster_span(&raw, 9, 5), (5, 5));
    }
}
