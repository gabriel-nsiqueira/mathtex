//! SVG backend for mathtex layout fragments, drawing glyph outlines shared through `<defs>`.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt::Write;

use mathtex_ir::{
    FontKey, Fragment, GlyphId, GlyphOutline, Length, NodeId, OutlineCommand, Placed, SourceRange,
};

/// Supplies glyph outlines by the host's font key, the SVG backend asks once per font.
pub trait GlyphOutlineSource {
    /// Returns one outline per glyph, `None` for blank glyphs and faces the source cannot read.
    fn glyph_outlines(&self, font: FontKey, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>>;
}

/// How [`render`] writes the SVG.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SvgOptions {
    /// Prefix of the glyph def ids, distinct prefixes let several SVGs share one page.
    pub id_prefix: String,
    /// Fill with `currentColor` instead of black, so the SVG takes the surrounding text color.
    pub current_color: bool,
    /// Write `width` and `height` in pt so the SVG has an intrinsic size.
    pub intrinsic_size: bool,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            id_prefix: String::from("g"),
            current_color: false,
            intrinsic_size: true,
        }
    }
}

impl SvgOptions {
    /// Sets the glyph def id prefix.
    #[must_use]
    pub fn with_id_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.id_prefix = prefix.into();
        self
    }

    /// Chooses between `currentColor` and black fills.
    #[must_use]
    pub fn with_current_color(mut self, on: bool) -> Self {
        self.current_color = on;
        self
    }

    /// Chooses whether to write the intrinsic width and height.
    #[must_use]
    pub fn with_intrinsic_size(mut self, on: bool) -> Self {
        self.intrinsic_size = on;
        self
    }
}

/// Renders a fragment's glyphs and rules, glyphs of TFM fonts and glyphs without outlines are skipped.
#[must_use]
pub fn render(
    fragment: &Fragment,
    outlines: &dyn GlyphOutlineSource,
    options: &SvgOptions,
) -> String {
    let placed = fragment.flatten();
    let outlines = fetch_outlines(&placed, outlines);
    let width = pt(fragment.surface.width);
    let height = pt(fragment.surface.height);
    let mut svg = String::new();
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}""#
    );
    if options.intrinsic_size {
        let _ = write!(svg, r#" width="{width}pt" height="{height}pt""#);
    }
    let fill = if options.current_color {
        "currentColor"
    } else {
        "black"
    };
    let _ = write!(
        svg,
        r#" fill="{fill}" data-format="{}">"#,
        escape_attr(&fragment.metadata.format_id)
    );

    let mut defs = String::new();
    let mut def_ids: BTreeMap<(FontKey, GlyphId), usize> = BTreeMap::new();
    let mut body = String::new();
    for item in &placed {
        match *item {
            Placed::Glyph {
                node,
                font,
                glyph_id,
                x,
                y,
                source,
            } => {
                let Some(key) = font.key else {
                    continue;
                };
                let Some(outline) = outlines.get(&(key, glyph_id)) else {
                    continue;
                };
                let next = def_ids.len();
                let def = *def_ids.entry((key, glyph_id)).or_insert_with(|| {
                    let _ = write!(
                        defs,
                        r#"<path id="{}{next}" d="{}"/>"#,
                        escape_attr(&options.id_prefix),
                        path_data(outline)
                    );
                    next
                });
                let scale = pt(font.size) / f32::from(outline.units_per_em);
                let _ = write!(
                    body,
                    r##"<use{} data-glyph-id="{}" href="#{}{def}" transform="translate({} {}) scale({scale} {})"/>"##,
                    item_attrs(node, source),
                    glyph_id.0,
                    escape_attr(&options.id_prefix),
                    pt(x),
                    pt(y),
                    -scale,
                );
            }
            Placed::Rule {
                node,
                x,
                y,
                width,
                height,
                source,
            } => {
                let _ = write!(
                    body,
                    r#"<rect{} x="{}" y="{}" width="{}" height="{}"/>"#,
                    item_attrs(node, source),
                    pt(x),
                    pt(y),
                    pt(width),
                    pt(height),
                );
            }
        }
    }
    if !defs.is_empty() {
        let _ = write!(svg, "<defs>{defs}</defs>");
    }
    svg.push_str(&body);
    svg.push_str("</svg>");
    svg
}

/// Asks the source once per font for the distinct glyphs placed in that font.
fn fetch_outlines(
    placed: &[Placed<'_>],
    source: &dyn GlyphOutlineSource,
) -> BTreeMap<(FontKey, GlyphId), GlyphOutline> {
    let mut wanted: BTreeMap<FontKey, Vec<GlyphId>> = BTreeMap::new();
    for item in placed {
        if let Placed::Glyph { font, glyph_id, .. } = item {
            if let Some(key) = font.key {
                wanted.entry(key).or_default().push(*glyph_id);
            }
        }
    }
    let mut found = BTreeMap::new();
    for (key, mut glyphs) in wanted {
        glyphs.sort_unstable();
        glyphs.dedup();
        let outlines = source.glyph_outlines(key, &glyphs);
        for (glyph, outline) in glyphs.into_iter().zip(outlines) {
            if let Some(outline) = outline.filter(|o| !o.commands.is_empty() && o.units_per_em > 0)
            {
                found.insert((key, glyph), outline);
            }
        }
    }
    found
}

fn item_attrs(node: NodeId, source: Option<SourceRange>) -> String {
    let mut attrs = format!(r#" data-node-id="{}""#, node.0);
    if let Some(range) = source {
        let _ = write!(
            attrs,
            r#" data-source-id="{}" data-source-start="{}" data-source-end="{}""#,
            range.source.0, range.span.start, range.span.end
        );
    }
    attrs
}

fn pt(length: Length) -> f32 {
    length.to_pt() as f32
}

fn path_data(outline: &GlyphOutline) -> String {
    let mut data = String::new();
    for (index, command) in outline.commands.iter().enumerate() {
        if index > 0 {
            data.push(' ');
        }
        let _ = match *command {
            OutlineCommand::MoveTo { x, y } => write!(data, "M{x} {y}"),
            OutlineCommand::LineTo { x, y } => write!(data, "L{x} {y}"),
            OutlineCommand::QuadTo { cx, cy, x, y } => write!(data, "Q{cx} {cy} {x} {y}"),
            OutlineCommand::CurveTo {
                c1x,
                c1y,
                c2x,
                c2y,
                x,
                y,
            } => write!(data, "C{c1x} {c1y} {c2x} {c2y} {x} {y}"),
            OutlineCommand::Close => write!(data, "Z"),
        };
    }
    data
}

fn escape_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathtex_ir::{
        BoxKind, ByteSpan, FontRef, FragmentMetadata, GlyphRun, LayoutBox, LayoutNode,
        LayoutNodeKind, Point, PositionedGlyph, SourceMap,
    };
    use std::cell::RefCell;

    const PT: i32 = 65_536;

    /// Every glyph is the same triangle, the calls record which glyphs each font was asked for.
    #[derive(Default)]
    struct TriangleOutlines {
        calls: RefCell<Vec<(FontKey, Vec<GlyphId>)>>,
    }

    impl GlyphOutlineSource for TriangleOutlines {
        fn glyph_outlines(&self, font: FontKey, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>> {
            self.calls.borrow_mut().push((font, glyphs.to_vec()));
            glyphs
                .iter()
                .map(|glyph| {
                    (glyph.0 != 0).then(|| GlyphOutline {
                        units_per_em: 1000,
                        commands: vec![
                            OutlineCommand::MoveTo { x: 0.0, y: 0.0 },
                            OutlineCommand::LineTo {
                                x: 500.0,
                                y: 1000.0,
                            },
                            OutlineCommand::LineTo { x: 1000.0, y: 0.0 },
                            OutlineCommand::Close,
                        ],
                    })
                })
                .collect()
        }
    }

    fn node(
        id: u32,
        origin: (i32, i32),
        size: (i32, i32, i32),
        kind: LayoutNodeKind,
    ) -> LayoutNode {
        LayoutNode {
            id: mathtex_ir::NodeId(id),
            origin: Point::new(Length(origin.0 * PT), Length(origin.1 * PT)),
            width: Length(size.0 * PT),
            height: Length(size.1 * PT),
            depth: Length(size.2 * PT),
            primary_source: None,
            kind,
        }
    }

    fn run(key: Option<u64>, glyphs: &[u32]) -> LayoutNodeKind {
        LayoutNodeKind::GlyphRun(GlyphRun {
            font: FontRef {
                key: key.map(FontKey),
                spec: "[test.otf]".into(),
                size: Length(10 * PT),
            },
            glyphs: glyphs
                .iter()
                .enumerate()
                .map(|(index, &glyph)| PositionedGlyph {
                    glyph_id: GlyphId(glyph),
                    offset: Point::new(Length(index as i32 * PT), Length::ZERO),
                    cluster: None,
                })
                .collect(),
        })
    }

    fn fragment() -> Fragment {
        let mut source_map = SourceMap::default();
        let input = source_map.add_source("input");
        let mut nodes = vec![
            node(
                0,
                (0, 0),
                (20, 8, 2),
                LayoutNodeKind::Box(LayoutBox {
                    kind: BoxKind::Horizontal,
                    children: vec![
                        mathtex_ir::NodeId(1),
                        mathtex_ir::NodeId(2),
                        mathtex_ir::NodeId(3),
                    ],
                }),
            ),
            node(1, (1, 0), (3, 7, 0), run(Some(7), &[42, 42, 0])),
            node(2, (5, -1), (4, 1, 1), LayoutNodeKind::Rule),
            node(3, (12, 0), (5, 7, 0), run(None, &[65])),
        ];
        nodes[2].primary_source = Some(SourceRange {
            source: input,
            span: ByteSpan { start: 2, end: 9 },
        });
        let metadata = FragmentMetadata {
            format_id: "latex".into(),
            ..FragmentMetadata::default()
        };
        Fragment::new(mathtex_ir::NodeId(0), nodes, source_map, metadata).expect("valid fragment")
    }

    #[test]
    fn glyphs_share_one_def_and_sit_on_the_baseline() {
        let source = TriangleOutlines::default();
        let svg = render(&fragment(), &source, &SvgOptions::default());

        // The run is fetched once, deduplicated, and the TFM run is never asked for.
        assert_eq!(
            *source.calls.borrow(),
            [(FontKey(7), vec![GlyphId(0), GlyphId(42)])]
        );
        assert_eq!(svg.matches("<path ").count(), 1);
        assert!(svg.contains(r#"<defs><path id="g0" d="M0 0 L500 1000 L1000 0 Z"/></defs>"#));
        // The baseline sits 8pt down, the run starts 1pt right and its second glyph 1pt further.
        assert!(svg.contains(r##"href="#g0" transform="translate(1 8) scale(0.01 -0.01)""##));
        assert!(svg.contains(r##"href="#g0" transform="translate(2 8) scale(0.01 -0.01)""##));
        assert_eq!(
            svg.matches("<use").count(),
            2,
            "the blank glyph draws nothing"
        );
    }

    #[test]
    fn rules_are_placed_by_their_top_edge_with_sources() {
        let svg = render(
            &fragment(),
            &TriangleOutlines::default(),
            &SvgOptions::default(),
        );
        // The rule's baseline is 7pt down, its top 1pt above that, its extent height plus depth.
        assert!(svg.contains(
            r#"<rect data-node-id="2" data-source-id="0" data-source-start="2" data-source-end="9" x="5" y="6" width="4" height="2"/>"#
        ));
    }

    #[test]
    fn options_choose_fill_prefix_and_intrinsic_size() {
        let fragment = fragment();
        let plain = render(
            &fragment,
            &TriangleOutlines::default(),
            &SvgOptions::default(),
        );
        assert!(plain.starts_with(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10" width="20pt" height="10pt" fill="black" data-format="latex">"#
        ));

        let options = SvgOptions::default()
            .with_id_prefix("eq1-")
            .with_current_color(true)
            .with_intrinsic_size(false);
        let svg = render(&fragment, &TriangleOutlines::default(), &options);
        assert!(svg.contains(r#"viewBox="0 0 20 10" fill="currentColor""#));
        assert!(svg.contains(r#"<path id="eq1-0""#));
        assert!(svg.contains(r##"href="#eq1-0""##));
    }
}
