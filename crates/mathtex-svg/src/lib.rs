//! SVG rendering backend for mathtex layout fragments.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use mathtex_ir::{
    Color, Fragment, GlyphId, GlyphOutline, GlyphRun, LayoutNode, LayoutNodeKind, Length, NodeId,
    OutlineCommand, Point, PositionedGlyph, SourceRange, Style,
};
use mathtex_render::{GlyphOutlineSource, RenderBackend};

/// Unit-struct render backend that produces SVG strings from layout fragments.
#[derive(Clone, Copy, Debug, Default)]
pub struct SvgRenderer;

impl RenderBackend for SvgRenderer {
    type Output = String;
    type Error = SvgError;

    fn render_fragment(&self, fragment: &Fragment) -> Result<Self::Output, Self::Error> {
        render(fragment)
    }
}

/// Renders glyphs as empty positioned `<g>` markers carrying source and metadata attributes.
pub fn render(fragment: &Fragment) -> Result<String, SvgError> {
    render_inner(fragment, None)
}

/// Renders glyph outlines as filled `<path>` elements, falling back to empty `<g>` markers.
pub fn render_with_outlines(
    fragment: &Fragment,
    outlines: &dyn GlyphOutlineSource,
) -> Result<String, SvgError> {
    render_inner(fragment, Some(outlines))
}

/// Interns glyph outline paths into `<defs>` so repeated glyphs share one definition.
#[derive(Default)]
struct GlyphDefs {
    /// Accumulated `<path id="gN" d="…"/>` entries, written without the `<defs>` wrapper.
    buf: String,
    /// Maps each outline `d` string to its assigned def id.
    ids: BTreeMap<String, u32>,
}

impl GlyphDefs {
    /// Returns the def id for outline `d`, emitting a `<path id>` on first encounter.
    fn intern(&mut self, d: &str) -> Result<u32, SvgError> {
        if let Some(&id) = self.ids.get(d) {
            return Ok(id);
        }
        let id = self.ids.len() as u32;
        write!(self.buf, r#"<path id="g{id}" d="{d}"/>"#).map_err(|_| SvgError::Serialize)?;
        self.ids.insert(String::from(d), id);
        Ok(id)
    }
}

fn render_inner(
    fragment: &Fragment,
    outlines: Option<&dyn GlyphOutlineSource>,
) -> Result<String, SvgError> {
    let mut svg = String::new();
    write!(
        svg,
        // `<use>` shadow trees inherit this fill, colored rules override it.
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" fill="currentColor" data-engine-profile="{}" data-format="{}">"#,
        length_to_svg(fragment.surface.width),
        length_to_svg(fragment.surface.height),
        escape_attr(&fragment.metadata.engine_profile),
        escape_attr(&fragment.metadata.format_id)
    )
    .map_err(|_| SvgError::Serialize)?;

    // Seed y with `surface.baseline` so baseline glyphs land at SVG y = baseline.
    let base = Point {
        x: Length::ZERO,
        y: fragment.surface.baseline,
    };
    let mut body = String::new();
    let mut defs = GlyphDefs::default();
    if let Some(root) = select_root(fragment) {
        render_node(&mut body, &mut defs, fragment, root, base, outlines)?;
    } else {
        // No root Box found: draw every node at the baseline seeded offset.
        for node in &fragment.nodes {
            render_node(&mut body, &mut defs, fragment, node.id, base, outlines)?;
        }
    }

    if !defs.buf.is_empty() {
        svg.push_str("<defs>");
        svg.push_str(&defs.buf);
        svg.push_str("</defs>");
    }
    svg.push_str(&body);
    svg.push_str("</svg>");
    Ok(svg)
}

/// Finds the root `Box`: the one whose id appears in no other node's children list.
fn select_root(fragment: &Fragment) -> Option<NodeId> {
    let mut is_child: Vec<NodeId> = Vec::new();
    for node in &fragment.nodes {
        match &node.kind {
            LayoutNodeKind::Box(b) => is_child.extend(b.children.iter().copied()),
            LayoutNodeKind::List(l) => is_child.extend(l.children.iter().copied()),
            LayoutNodeKind::Group { children } => is_child.extend(children.iter().copied()),
            _ => {}
        }
    }
    fragment.nodes.iter().find_map(|node| match &node.kind {
        LayoutNodeKind::Box(_) if !is_child.contains(&node.id) => Some(node.id),
        _ => None,
    })
}

/// Draws a node and its descendants at absolute positions accumulated as `parent + node.origin`.
fn render_node(
    svg: &mut String,
    defs: &mut GlyphDefs,
    fragment: &Fragment,
    id: NodeId,
    parent: Point,
    outlines: Option<&dyn GlyphOutlineSource>,
) -> Result<(), SvgError> {
    let Some(node) = fragment.node(id) else {
        return Ok(());
    };
    let abs = Point {
        x: Length(parent.x.0.saturating_add(node.origin.x.0)),
        y: Length(parent.y.0.saturating_add(node.origin.y.0)),
    };
    let source_attrs = source_attrs(node, fragment)?;
    let style_attrs = style_attrs(node.style)?;
    match &node.kind {
        LayoutNodeKind::Rule(rule) => {
            // SVG y grows downward, so the rule's top left is (abs.x, abs.y - height).
            let fill = if rule.color.r == 0
                && rule.color.g == 0
                && rule.color.b == 0
                && rule.color.a == 255
            {
                "currentColor".to_string()
            } else {
                format!(
                    "rgba({},{},{},{})",
                    rule.color.r,
                    rule.color.g,
                    rule.color.b,
                    f32::from(rule.color.a) / 255.0
                )
            };
            write!(
                svg,
                r#"<rect data-node-id="{}"{}{} x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                node.id.0,
                source_attrs,
                style_attrs,
                length_to_svg(abs.x),
                length_to_svg(Length(abs.y.0.saturating_sub(rule.size.height.0))),
                length_to_svg(rule.size.width),
                length_to_svg(rule.size.height),
                fill,
            )
            .map_err(|_| SvgError::Serialize)?;
        }
        LayoutNodeKind::GlyphRun(run) => {
            render_glyph_run(
                svg,
                defs,
                fragment,
                node,
                abs,
                run,
                &source_attrs,
                &style_attrs,
                outlines,
            )?;
        }
        LayoutNodeKind::Box(b) => {
            write!(
                svg,
                r#"<g data-node-id="{}"{}{}>"#,
                node.id.0, source_attrs, style_attrs
            )
            .map_err(|_| SvgError::Serialize)?;
            // `shift_amount` is already folded into child origins by the IR emitter.
            for &child in &b.children {
                render_node(svg, defs, fragment, child, abs, outlines)?;
            }
            svg.push_str("</g>");
        }
        LayoutNodeKind::List(l) => {
            write!(
                svg,
                r#"<g data-node-id="{}"{}{}>"#,
                node.id.0, source_attrs, style_attrs
            )
            .map_err(|_| SvgError::Serialize)?;
            for &child in &l.children {
                render_node(svg, defs, fragment, child, abs, outlines)?;
            }
            svg.push_str("</g>");
        }
        LayoutNodeKind::Group { children } => {
            write!(
                svg,
                r#"<g data-node-id="{}"{}{}>"#,
                node.id.0, source_attrs, style_attrs
            )
            .map_err(|_| SvgError::Serialize)?;
            for &child in children {
                render_node(svg, defs, fragment, child, abs, outlines)?;
            }
            svg.push_str("</g>");
        }
        LayoutNodeKind::Glue(_) | LayoutNodeKind::Kern(_) | LayoutNodeKind::Drawing(_) => {
            write!(
                svg,
                r#"<g data-node-id="{}"{}{} transform="translate({} {})"/>"#,
                node.id.0,
                source_attrs,
                style_attrs,
                length_to_svg(abs.x),
                length_to_svg(abs.y)
            )
            .map_err(|_| SvgError::Serialize)?;
        }
        _ => {
            write!(
                svg,
                r#"<g data-node-id="{}"{}{}/>"#,
                node.id.0, source_attrs, style_attrs
            )
            .map_err(|_| SvgError::Serialize)?;
        }
    }
    Ok(())
}

/// Error type returned by SVG rendering operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SvgError {
    /// Failure to write formatted output to the SVG string buffer.
    Serialize,
}

fn length_to_svg(length: Length) -> f32 {
    length.0 as f32 / 65_536.0
}

fn source_attrs(node: &LayoutNode, fragment: &Fragment) -> Result<String, SvgError> {
    let Some(range) = node
        .primary_source
        .or_else(|| fragment.primary_source_for_node(node.id))
    else {
        return Ok(String::new());
    };

    source_range_attrs(range)
}

fn source_range_attrs(range: SourceRange) -> Result<String, SvgError> {
    let mut attrs = String::new();
    write!(
        attrs,
        r#" data-source-id="{}" data-source-start="{}" data-source-end="{}""#,
        range.source.0, range.span.start, range.span.end
    )
    .map_err(|_| SvgError::Serialize)?;
    Ok(attrs)
}

fn glyph_source_attrs(
    fragment: &Fragment,
    node: &LayoutNode,
    glyph_index: usize,
) -> Result<String, SvgError> {
    fragment
        .glyph_source_range(node.id, glyph_index)
        .map_or_else(|| Ok(String::new()), source_range_attrs)
}

// `origin` is the absolute pen position of the run.
#[allow(clippy::too_many_arguments)]
fn render_glyph_run(
    svg: &mut String,
    defs: &mut GlyphDefs,
    fragment: &Fragment,
    node: &LayoutNode,
    origin: Point,
    run: &GlyphRun,
    source_attrs: &str,
    style_attrs: &str,
    outlines: Option<&dyn GlyphOutlineSource>,
) -> Result<(), SvgError> {
    write!(
        svg,
        r#"<g data-node-id="{}"{}{} data-font-id="{}" data-font="{}" data-font-size="{}" data-glyph-count="{}">"#,
        node.id.0,
        source_attrs,
        style_attrs,
        run.font.id.0,
        escape_attr(&run.font.name),
        length_to_svg(run.font.size),
        run.glyphs.len()
    )
    .map_err(|_| SvgError::Serialize)?;

    // Fetch outlines for all glyphs in the run in one call to avoid per glyph overhead.
    let run_outlines = outlines.map(|source| {
        let ids: Vec<GlyphId> = run.glyphs.iter().map(|glyph| glyph.glyph_id).collect();
        source.glyph_run_outlines(&run.font, &ids)
    });

    for (index, glyph) in run.glyphs.iter().copied().enumerate() {
        let outline = run_outlines
            .as_ref()
            .and_then(|list| list.get(index))
            .and_then(Option::as_ref);
        render_glyph(
            svg,
            defs,
            fragment,
            node,
            origin,
            index,
            glyph,
            run.font.size,
            outline,
        )?;
    }

    svg.push_str("</g>");
    Ok(())
}

// `origin` is the absolute pen position of the run.
#[allow(clippy::too_many_arguments)]
fn render_glyph(
    svg: &mut String,
    defs: &mut GlyphDefs,
    fragment: &Fragment,
    node: &LayoutNode,
    origin: Point,
    glyph_index: usize,
    glyph: PositionedGlyph,
    font_size: Length,
    outline: Option<&GlyphOutline>,
) -> Result<(), SvgError> {
    let source_attrs = glyph_source_attrs(fragment, node, glyph_index)?;
    // `origin` is the absolute pen position (parent + node.origin + baseline), add per glyph offset on top.
    let x = Length(origin.x.0.saturating_add(glyph.offset.x.0));
    let y = Length(origin.y.0.saturating_add(glyph.offset.y.0));

    // Render via `<use href="#gN">`, the transform places and flips the glyph.
    if let Some(outline) = outline.filter(|o| !o.commands.is_empty() && o.units_per_em > 0) {
        let scale = length_to_svg(font_size) / f32::from(outline.units_per_em);
        let def_id = defs.intern(&outline_path_data(outline))?;
        write!(
            svg,
            r##"<use data-glyph-index="{}" data-glyph-id="{}"{} data-advance-x="{}" data-advance-y="{}" href="#g{}" transform="translate({} {}) scale({} {})"/>"##,
            glyph_index,
            glyph.glyph_id.0,
            source_attrs,
            length_to_svg(glyph.advance.x),
            length_to_svg(glyph.advance.y),
            def_id,
            length_to_svg(x),
            length_to_svg(y),
            scale,
            -scale,
        )
        .map_err(|_| SvgError::Serialize)?;
        return Ok(());
    }

    write!(
        svg,
        r#"<g data-glyph-index="{}" data-glyph-id="{}"{} data-advance-x="{}" data-advance-y="{}" transform="translate({} {})"/>"#,
        glyph_index,
        glyph.glyph_id.0,
        source_attrs,
        length_to_svg(glyph.advance.x),
        length_to_svg(glyph.advance.y),
        length_to_svg(x),
        length_to_svg(y)
    )
    .map_err(|_| SvgError::Serialize)
}

fn outline_path_data(outline: &GlyphOutline) -> String {
    let mut data = String::new();
    for (index, command) in outline.commands.iter().enumerate() {
        if index > 0 {
            data.push(' ');
        }
        match *command {
            OutlineCommand::MoveTo { x, y } => {
                let _ = write!(data, "M{x} {y}");
            }
            OutlineCommand::LineTo { x, y } => {
                let _ = write!(data, "L{x} {y}");
            }
            OutlineCommand::QuadTo { cx, cy, x, y } => {
                let _ = write!(data, "Q{cx} {cy} {x} {y}");
            }
            OutlineCommand::CurveTo {
                c1x,
                c1y,
                c2x,
                c2y,
                x,
                y,
            } => {
                let _ = write!(data, "C{c1x} {c1y} {c2x} {c2y} {x} {y}");
            }
            OutlineCommand::Close => data.push('Z'),
        }
    }
    data
}

fn style_attrs(style: Style) -> Result<String, SvgError> {
    let mut attrs = String::new();
    write!(
        attrs,
        r#" data-foreground="{}" data-opacity="{}""#,
        color_to_hex(style.foreground),
        f32::from(style.opacity.0) / 255.0
    )
    .map_err(|_| SvgError::Serialize)?;

    if let Some(background) = style.background {
        write!(attrs, r#" data-background="{}""#, color_to_hex(background))
            .map_err(|_| SvgError::Serialize)?;
    }

    Ok(attrs)
}

fn color_to_hex(color: Color) -> String {
    let mut value = String::new();
    write!(
        value,
        "#{:02x}{:02x}{:02x}{:02x}",
        color.r, color.g, color.b, color.a
    )
    .expect("writing to string cannot fail");
    value
}

fn escape_attr(input: &str) -> String {
    let mut out = String::new();
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
        Alpha, ByteSpan, Color, Direction, FontId, FontRef, FragmentKind, FragmentMetadata,
        GlyphId, GlyphRun, NodeId, Point, PositionedGlyph, Rect, Rule, Size, SourceId, SourceMap,
        SourceRange, SourceRole, Style, Surface,
    };

    #[test]
    fn svg_exposes_primary_source_range() {
        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input");
        source_map.add_entry(
            NodeId(3),
            SourceRange {
                source,
                span: ByteSpan { start: 2, end: 9 },
            },
            SourceRole::Primary,
        );

        let fragment = Fragment {
            surface: Surface {
                width: Length::from_scaled_points(65_536),
                height: Length::from_scaled_points(65_536),
                baseline: Length::ZERO,
            },
            nodes: alloc::vec![mathtex_ir::LayoutNode {
                id: NodeId(3),
                origin: Point::default(),
                bounds: Rect::default(),
                primary_source: None,
                style: Style::default(),
                kind: LayoutNodeKind::Rule(Rule {
                    size: Size {
                        width: Length::from_scaled_points(65_536),
                        height: Length::from_scaled_points(32_768),
                    },
                    color: Color::default(),
                }),
            }],
            source_map,
            metadata: FragmentMetadata {
                engine_profile: "tex".into(),
                format_id: "latex".into(),
                fragment_kind: FragmentKind::MathInline,
            },
        };

        let svg = render(&fragment).expect("svg should render");

        assert!(svg.contains(r#"data-node-id="3""#));
        assert!(svg.contains(r#"data-source-id="0""#));
        assert!(svg.contains(r#"data-source-start="2""#));
        assert!(svg.contains(r#"data-source-end="9""#));
    }

    #[test]
    fn svg_prefers_node_primary_source_over_source_map_lookup() {
        let fragment = Fragment {
            nodes: alloc::vec![mathtex_ir::LayoutNode {
                id: NodeId(1),
                origin: Point::default(),
                bounds: Rect::default(),
                primary_source: Some(SourceRange {
                    source: SourceId(4),
                    span: ByteSpan { start: 11, end: 12 },
                }),
                style: Style {
                    foreground: Color {
                        r: 12,
                        g: 34,
                        b: 56,
                        a: 255,
                    },
                    background: Some(Color {
                        r: 1,
                        g: 2,
                        b: 3,
                        a: 255,
                    }),
                    opacity: Alpha(128),
                },
                kind: LayoutNodeKind::Group {
                    children: alloc::vec![],
                },
            }],
            ..Fragment::default()
        };

        let svg = render(&fragment).expect("svg should render");

        assert!(svg.contains(r#"data-source-id="4""#));
        assert!(svg.contains(r#"data-source-start="11""#));
        assert!(svg.contains(r#"data-source-end="12""#));
        assert!(svg.contains(r##"data-foreground="#0c2238ff""##));
        assert!(svg.contains(r##"data-background="#010203ff""##));
        assert!(svg.contains(r#"data-opacity="0.5019608""#));
    }

    #[test]
    fn svg_emits_positioned_glyphs_with_cluster_sources() {
        let mut source_map = SourceMap::default();
        let source = source_map.add_source("input");
        let node = NodeId(9);
        source_map.add_entry(
            node,
            SourceRange {
                source,
                span: ByteSpan { start: 4, end: 8 },
            },
            SourceRole::Primary,
        );

        let fragment = Fragment {
            surface: Surface {
                width: Length::from_scaled_points(4 * 65_536),
                height: Length::from_scaled_points(2 * 65_536),
                baseline: Length::from_scaled_points(65_536),
            },
            nodes: alloc::vec![mathtex_ir::LayoutNode {
                id: node,
                origin: Point {
                    x: Length::from_scaled_points(65_536),
                    y: Length::from_scaled_points(2 * 65_536),
                },
                bounds: Rect::default(),
                primary_source: None,
                style: Style::default(),
                kind: LayoutNodeKind::GlyphRun(GlyphRun {
                    font: FontRef {
                        id: FontId(7),
                        name: "Test Font".into(),
                        size: Length::from_scaled_points(10 * 65_536),
                        features: alloc::vec![],
                    },
                    direction: Direction::LeftToRight,
                    script: None,
                    language: None,
                    glyphs: alloc::vec![
                        PositionedGlyph {
                            glyph_id: GlyphId(42),
                            offset: Point::default(),
                            advance: Point {
                                x: Length::from_scaled_points(65_536),
                                y: Length::ZERO,
                            },
                            cluster: Some(ByteSpan { start: 4, end: 6 }),
                        },
                        PositionedGlyph {
                            glyph_id: GlyphId(43),
                            offset: Point {
                                x: Length::from_scaled_points(65_536),
                                y: Length::ZERO,
                            },
                            advance: Point {
                                x: Length::from_scaled_points(2 * 65_536),
                                y: Length::ZERO,
                            },
                            cluster: Some(ByteSpan { start: 6, end: 8 }),
                        },
                    ],
                }),
            }],
            source_map,
            ..Fragment::default()
        };

        let svg = render(&fragment).expect("svg should render");

        assert!(svg.contains(r#"data-font-id="7""#));
        assert!(svg.contains(r#"data-font="Test Font""#));
        assert!(svg.contains(r#"data-glyph-index="0" data-glyph-id="42""#));
        assert!(svg.contains(r#"data-glyph-index="1" data-glyph-id="43""#));
        assert!(svg.contains(r#"data-source-start="4" data-source-end="6""#));
        assert!(svg.contains(r#"data-source-start="6" data-source-end="8""#));
        // No Box parent: base=(0,1), abs=(1,3), glyph 1 lands at (2,3).
        assert!(svg.contains(r#"transform="translate(1 3)"#));
        assert!(svg.contains(r#"transform="translate(2 3)"#));
    }

    struct TriangleOutlines;

    impl GlyphOutlineSource for TriangleOutlines {
        fn glyph_run_outlines(
            &self,
            _font: &FontRef,
            glyphs: &[GlyphId],
        ) -> alloc::vec::Vec<Option<GlyphOutline>> {
            glyphs
                .iter()
                .map(|_| {
                    Some(GlyphOutline {
                        units_per_em: 1000,
                        commands: alloc::vec![
                            OutlineCommand::MoveTo { x: 0.0, y: 0.0 },
                            OutlineCommand::LineTo { x: 500.0, y: 1000.0 },
                            OutlineCommand::LineTo { x: 1000.0, y: 0.0 },
                            OutlineCommand::Close,
                        ],
                    })
                })
                .collect()
        }
    }

    #[test]
    fn svg_draws_glyph_outlines_as_paths() {
        let fragment = Fragment {
            surface: Surface {
                width: Length::from_scaled_points(10 * 65_536),
                height: Length::from_scaled_points(10 * 65_536),
                baseline: Length::from_scaled_points(8 * 65_536),
            },
            nodes: alloc::vec![mathtex_ir::LayoutNode {
                id: NodeId(1),
                origin: Point {
                    x: Length::from_scaled_points(65_536),
                    y: Length::from_scaled_points(2 * 65_536),
                },
                bounds: Rect::default(),
                primary_source: None,
                style: Style::default(),
                kind: LayoutNodeKind::GlyphRun(GlyphRun {
                    font: FontRef {
                        id: FontId(1),
                        name: "Outline Font".into(),
                        size: Length::from_scaled_points(10 * 65_536),
                        features: alloc::vec![],
                    },
                    direction: Direction::LeftToRight,
                    script: None,
                    language: None,
                    glyphs: alloc::vec![PositionedGlyph {
                        glyph_id: GlyphId(42),
                        offset: Point::default(),
                        advance: Point {
                            x: Length::from_scaled_points(65_536),
                            y: Length::ZERO,
                        },
                        cluster: None,
                    }],
                }),
            }],
            ..Fragment::default()
        };

        let svg = render_with_outlines(&fragment, &TriangleOutlines).expect("svg should render");

        // Outline interned as `g0`, no Box parent gives base=(0,8) and abs=(1,10).
        assert!(svg.contains(r#"<defs><path id="g0" d="M0 0 L500 1000 L1000 0 Z"/></defs>"#));
        assert!(svg.contains(r#"<use data-glyph-index="0" data-glyph-id="42""#));
        assert!(svg.contains(r##"href="#g0" transform="translate(1 10) scale(0.01 -0.01)""##));
        // The outline data lives only in the <defs> entry, never inlined on the use.
        assert_eq!(svg.matches(r#"d="M0 0 L500 1000 L1000 0 Z""#).count(), 1);
        assert!(!svg.contains("<path data-glyph"));

        // Without an outline source, the same glyph stays an empty <g> marker.
        let plain = render(&fragment).expect("svg should render");
        assert!(!plain.contains("<path"));
        assert!(plain.contains(r#"<g data-glyph-index="0" data-glyph-id="42""#));
    }

    #[test]
    fn svg_accumulates_nested_origins() {
        use mathtex_ir::{BoxKind, BoxMetrics, LayoutBox};

        // Relative child (3,0) plus glyph run (5,0) yields absolute x=8, y=baseline.
        let root = mathtex_ir::LayoutNode {
            id: NodeId(1),
            origin: Point::default(),
            bounds: Rect::default(),
            primary_source: None,
            style: Style::default(),
            kind: LayoutNodeKind::Box(LayoutBox {
                kind: BoxKind::Horizontal,
                metrics: BoxMetrics::default(),
                children: alloc::vec![NodeId(2)],
            }),
        };
        let child = mathtex_ir::LayoutNode {
            id: NodeId(2),
            origin: Point {
                x: Length::from_scaled_points(3 * 65_536),
                y: Length::ZERO,
            },
            bounds: Rect::default(),
            primary_source: None,
            style: Style::default(),
            kind: LayoutNodeKind::Box(LayoutBox {
                kind: BoxKind::Horizontal,
                metrics: BoxMetrics::default(),
                children: alloc::vec![NodeId(3)],
            }),
        };
        let run = mathtex_ir::LayoutNode {
            id: NodeId(3),
            origin: Point {
                x: Length::from_scaled_points(5 * 65_536),
                y: Length::ZERO,
            },
            bounds: Rect::default(),
            primary_source: None,
            style: Style::default(),
            kind: LayoutNodeKind::GlyphRun(GlyphRun {
                font: FontRef {
                    id: FontId(1),
                    name: "Nested".into(),
                    size: Length::from_scaled_points(10 * 65_536),
                    features: alloc::vec![],
                },
                direction: Direction::LeftToRight,
                script: None,
                language: None,
                glyphs: alloc::vec![PositionedGlyph {
                    glyph_id: GlyphId(7),
                    offset: Point::default(),
                    advance: Point::default(),
                    cluster: None,
                }],
            }),
        };

        let fragment = Fragment {
            surface: Surface {
                width: Length::from_scaled_points(20 * 65_536),
                height: Length::from_scaled_points(10 * 65_536),
                baseline: Length::from_scaled_points(4 * 65_536),
            },
            // Children are emitted before the root, as in the engine depth first emission.
            nodes: alloc::vec![run, child, root],
            ..Fragment::default()
        };

        let svg = render(&fragment).expect("svg should render");

        // Absolute position: (3+5, 4) = (8, 4).
        assert!(
            svg.contains(r#"transform="translate(8 4)"#),
            "nested glyph should accumulate origins to (8 4); svg = {svg}"
        );
        assert!(!svg.contains(r#"transform="translate(0 0)"#));
        assert!(!svg.contains(r#"transform="translate(5 0)"#));
    }
}
