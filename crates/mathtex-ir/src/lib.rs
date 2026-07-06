//! Renderer neutral intermediate representation for a laid out TeX/LaTeX fragment.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// A complete renderer neutral layout result for one TeX/LaTeX fragment.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fragment {
    /// Fragment coordinate surface dimensions.
    pub surface: Surface,
    /// All layout nodes in this fragment.
    pub nodes: Vec<LayoutNode>,
    /// Source map relating nodes back to input positions.
    pub source_map: SourceMap,
    /// Metadata about the engine and format that produced this fragment.
    pub metadata: FragmentMetadata,
}

impl Fragment {
    /// Returns the layout node with the given identifier, if present.
    #[must_use]
    pub fn node(&self, node: NodeId) -> Option<&LayoutNode> {
        self.nodes.iter().find(|candidate| candidate.id == node)
    }

    /// Iterates all source map entries for the given node.
    pub fn source_entries_for_node(&self, node: NodeId) -> impl Iterator<Item = &SourceMapEntry> {
        self.source_map.entries_for_node(node)
    }

    /// Iterates resolved source origins for the given node.
    pub fn source_origins_for_node(&self, node: NodeId) -> impl Iterator<Item = SourceOrigin<'_>> {
        self.source_entries_for_node(node)
            .filter_map(|entry| self.source_origin_for_entry(entry))
    }

    /// Resolves a source map entry to its origin metadata.
    #[must_use]
    pub fn source_origin_for_entry(&self, entry: &SourceMapEntry) -> Option<SourceOrigin<'_>> {
        let source = self.source_map.source(entry.range.source)?;
        Some(SourceOrigin {
            node: entry.node,
            source,
            span: entry.range.span,
            role: entry.role,
        })
    }

    /// Returns the primary source range for the given node, if any.
    #[must_use]
    pub fn primary_source_for_node(&self, node: NodeId) -> Option<SourceRange> {
        self.source_map.primary_range_for_node(node)
    }

    /// Resolves the glyph cluster byte span to a SourceRange using the node's primary source.
    #[must_use]
    pub fn glyph_source_range(&self, node: NodeId, glyph_index: usize) -> Option<SourceRange> {
        let node = self.node(node)?;
        let LayoutNodeKind::GlyphRun(run) = &node.kind else {
            return None;
        };
        let span = run.glyphs.get(glyph_index)?.cluster?;
        let source = node
            .primary_source
            .or_else(|| self.primary_source_for_node(node.id))?
            .source;

        Some(SourceRange { source, span })
    }

    /// Resolves a glyph's source cluster to a full SourceOrigin.
    #[must_use]
    pub fn glyph_source_origin(
        &self,
        node: NodeId,
        glyph_index: usize,
    ) -> Option<SourceOrigin<'_>> {
        let range = self.glyph_source_range(node, glyph_index)?;
        let source = self.source_map.source(range.source)?;
        Some(SourceOrigin {
            node,
            source,
            span: range.span,
            role: SourceRole::Primary,
        })
    }
}

/// A resolved source map relationship with source metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceOrigin<'a> {
    /// The layout node this origin relates to.
    pub node: NodeId,
    /// The source file containing the originating text.
    pub source: &'a SourceFile,
    /// Byte span within that source file.
    pub span: ByteSpan,
    /// How this source range contributed to the node.
    pub role: SourceRole,
}

/// Metadata describing how a fragment was produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FragmentMetadata {
    /// Engine profile identifier, such as `tex`, `etex`, or `xetex`.
    pub engine_profile: String,
    /// Format image identifier, such as `plain`, `latex`, or a custom format.
    pub format_id: String,
    /// Whether this fragment is inline math, display math, or text.
    pub fragment_kind: FragmentKind,
}

/// Classifies the mode in which a fragment was typeset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FragmentKind {
    /// Inline math, typeset in text style.
    #[default]
    MathInline,
    /// Display math, typeset centered on its own line.
    MathDisplay,
    /// Paragraph text mode.
    Text,
}

/// The top level coordinate surface for a rendered fragment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Surface {
    /// Horizontal extent of the fragment bounding box.
    pub width: Length,
    /// Vertical extent of the fragment bounding box.
    pub height: Length,
    /// Baseline offset from the top edge.
    pub baseline: Length,
}

/// A positioned renderable or grouping node.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutNode {
    /// Stable node identifier within the fragment.
    pub id: NodeId,
    /// Position of the node origin in fragment coordinates.
    pub origin: Point,
    /// Bounding rectangle in fragment coordinates.
    pub bounds: Rect,
    /// Primary source range when the node has one direct origin.
    pub primary_source: Option<SourceRange>,
    /// Visual style applied to this node.
    pub style: Style,
    /// Content payload of this node.
    pub kind: LayoutNodeKind,
}

impl LayoutNode {
    /// Returns the baseline y coordinate, boxes use origin.y + height, glyph runs use origin.y.
    #[must_use]
    pub fn baseline_y(&self) -> Option<Length> {
        match &self.kind {
            LayoutNodeKind::Box(layout_box) => Some(Length(
                self.origin.y.0 + layout_box.metrics.baseline_offset().0,
            )),
            LayoutNodeKind::GlyphRun(_) => Some(self.origin.y),
            LayoutNodeKind::List(_)
            | LayoutNodeKind::Group { .. }
            | LayoutNodeKind::Rule(_)
            | LayoutNodeKind::Glue(_)
            | LayoutNodeKind::Kern(_)
            | LayoutNodeKind::Drawing(_) => None,
        }
    }
}

/// Renderer neutral node payloads.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LayoutNodeKind {
    /// TeX style box with width, height, depth, and positioned children.
    Box(LayoutBox),
    /// TeX style list before or after packaging into a box.
    List(LayoutList),
    /// A grouping of child nodes with no layout box.
    Group {
        /// Child node identifiers in this group.
        children: Vec<NodeId>,
    },
    /// A sequence of shaped glyphs from one font.
    GlyphRun(GlyphRun),
    /// A solid rule, such as a fraction bar.
    Rule(Rule),
    /// Flexible spacing emitted after layout resolution.
    Glue(Glue),
    /// Fixed horizontal or vertical spacing between adjacent items.
    Kern(Kern),
    /// Drawing commands for extensible delimiters or geometry beyond glyphs and rules.
    Drawing(Drawing),
}

/// TeX style box data retained in renderer neutral layout IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutBox {
    /// Whether this is a horizontal, vertical, math, or text box.
    pub kind: BoxKind,
    /// TeX box metrics.
    pub metrics: BoxMetrics,
    /// Child nodes positioned relative to this box.
    pub children: Vec<NodeId>,
}

/// Distinguishes the axis or mode of a TeX box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoxKind {
    /// Horizontal box (hbox), items laid out left to right.
    Horizontal,
    /// Vertical box (vbox), items stacked top to bottom.
    Vertical,
    /// Math mode box.
    Math,
    /// Box produced by text layout inside math or normal text.
    Text,
}

/// TeX box dimensions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BoxMetrics {
    /// Horizontal extent.
    pub width: Length,
    /// Height above baseline.
    pub height: Length,
    /// Depth below baseline.
    pub depth: Length,
    /// Shift applied when this box is placed in a parent list.
    pub shift: Length,
}

impl BoxMetrics {
    /// Baseline offset from the top edge of the box.
    #[must_use]
    pub const fn baseline_offset(&self) -> Length {
        self.height
    }

    /// Height plus depth, giving the total vertical extent.
    #[must_use]
    pub const fn total_height(&self) -> Length {
        Length(self.height.0 + self.depth.0)
    }
}

/// TeX style list data retained before or after box packaging.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutList {
    /// Whether this is a horizontal, vertical, math, paragraph, or text list.
    pub kind: ListKind,
    /// Child node identifiers in list order.
    pub children: Vec<NodeId>,
}

/// Distinguishes the axis or mode of a TeX list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ListKind {
    /// Horizontal list (hlist), items laid out left to right.
    Horizontal,
    /// Vertical list (vlist), items stacked top to bottom.
    Vertical,
    /// Math mode list.
    Math,
    /// Paragraph list before line breaking.
    Paragraph,
    /// Text list, including text inside math.
    Text,
}

/// A shaped sequence of glyphs from one font.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphRun {
    /// Font identity and size for all glyphs in this run.
    pub font: FontRef,
    /// Text direction for rendering.
    pub direction: Direction,
    /// Optional script tag, such as `Latn` or `Arab`.
    pub script: Option<Tag>,
    /// Optional language tag following language tag conventions.
    pub language: Option<String>,
    /// Positioned glyphs in visual order.
    pub glyphs: Vec<PositionedGlyph>,
}

/// A glyph with its layout position and optional source cluster.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PositionedGlyph {
    /// Backend independent glyph identifier in the referenced font.
    pub glyph_id: GlyphId,
    /// Offset from the run origin.
    pub offset: Point,
    /// Advance to the next glyph.
    pub advance: Point,
    /// Source byte cluster for this glyph.
    pub cluster: Option<ByteSpan>,
}

/// Font identity and size for a glyph run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontRef {
    /// Stable font identifier assigned by the engine or resource layer.
    pub id: FontId,
    /// Human readable family or resource name.
    pub name: String,
    /// Rendered size in scaled points.
    pub size: Length,
    /// OpenType feature requests that affected shaping.
    pub features: Vec<FontFeature>,
}

/// An OpenType feature tag and value pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontFeature {
    /// Four byte tag in OpenType format.
    pub tag: Tag,
    /// Feature activation value, 1 enables, 0 disables.
    pub value: u32,
}

/// A solid rectangle in layout coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rule {
    /// Width and height of the rule.
    pub size: Size,
    /// Fill color of the rule.
    pub color: Color,
}

/// Flexible spacing after TeX layout has resolved it to a concrete width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Glue {
    /// Resolved glue amount.
    pub amount: Length,
}

/// A fixed spacing adjustment between adjacent layout items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Kern {
    /// The amount of space to insert.
    pub amount: Length,
}

/// Backend neutral drawing data using path commands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawing {
    /// Ordered path commands composing the drawing.
    pub commands: Vec<DrawCommand>,
    /// Stroke style, or None if the path is not stroked.
    pub stroke: Option<Stroke>,
    /// Fill color, or None if the path is not filled.
    pub fill: Option<Color>,
}

/// A path drawing command in local layout coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DrawCommand {
    /// Move the current point without drawing.
    MoveTo(Point),
    /// Draw a straight line to the given point.
    LineTo(Point),
    /// Draw a cubic Bezier curve to the given point.
    CubicTo {
        /// First control point.
        ctrl1: Point,
        /// Second control point.
        ctrl2: Point,
        /// End point of the curve.
        to: Point,
    },
    /// Close the current subpath.
    Close,
}

/// Stroke style for a drawing path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stroke {
    /// Stroke color.
    pub color: Color,
    /// Stroke width.
    pub width: Length,
}

/// Renderer neutral style state attached to layout nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// Foreground color for text and rules.
    pub foreground: Color,
    /// Background color when one is explicitly active.
    pub background: Option<Color>,
    /// Overall opacity applied to this node.
    pub opacity: Alpha,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            foreground: Color::default(),
            background: None,
            opacity: Alpha::OPAQUE,
        }
    }
}

/// Alpha channel represented independently from color payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alpha(pub u8);

impl Alpha {
    /// Fully transparent (alpha = 0).
    pub const TRANSPARENT: Self = Self(0);
    /// Fully opaque (alpha = 255).
    pub const OPAQUE: Self = Self(255);
}

impl Default for Alpha {
    fn default() -> Self {
        Self::OPAQUE
    }
}

/// Maps layout nodes to regions in source input files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    /// Registered source files.
    pub sources: Vec<SourceFile>,
    /// Node to source relationships. A node may have multiple entries.
    pub entries: Vec<SourceMapEntry>,
}

impl SourceMap {
    /// Registers a new source file by name and returns its identifier.
    pub fn add_source(&mut self, name: impl Into<String>) -> SourceId {
        let id = SourceId(self.sources.len() as u32);
        self.sources.push(SourceFile {
            id,
            name: name.into(),
        });
        id
    }

    /// Returns the existing source identifier for a name, or registers a new one.
    pub fn intern_source(&mut self, name: impl Into<String>) -> SourceId {
        let name = name.into();
        if let Some(source) = self.sources.iter().find(|source| source.name == name) {
            return source.id;
        }
        self.add_source(name)
    }

    /// Appends a source map entry linking a node to a source range.
    pub fn add_entry(&mut self, node: NodeId, range: SourceRange, role: SourceRole) {
        self.entries.push(SourceMapEntry { node, range, role });
    }

    /// Returns the source file for the given identifier.
    #[must_use]
    pub fn source(&self, id: SourceId) -> Option<&SourceFile> {
        self.sources.iter().find(|source| source.id == id)
    }

    /// Iterates all entries for the given node.
    pub fn entries_for_node(&self, node: NodeId) -> impl Iterator<Item = &SourceMapEntry> {
        self.entries.iter().filter(move |entry| entry.node == node)
    }

    /// Returns the primary source range for the given node, if present.
    #[must_use]
    pub fn primary_range_for_node(&self, node: NodeId) -> Option<SourceRange> {
        self.entries_for_node(node)
            .find(|entry| entry.role == SourceRole::Primary)
            .map(|entry| entry.range)
    }
}

/// A source resource, such as user input or a loaded package file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// Stable identifier for this source file.
    pub id: SourceId,
    /// File name or description of this source.
    pub name: String,
}

/// A single mapping from a layout node to a source range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMapEntry {
    /// The node this entry maps.
    pub node: NodeId,
    /// Source range within a registered source file.
    pub range: SourceRange,
    /// How this source range relates to the node.
    pub role: SourceRole,
}

/// A span within a registered source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRange {
    /// The registered source file containing this range.
    pub source: SourceId,
    /// Byte span in that resource.
    pub span: ByteSpan,
}

/// How a source range contributed to a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceRole {
    /// The primary user visible expression source.
    Primary,
    /// Enclosing construct or macro invocation range, entries are emitted innermost first.
    EnclosingConstruct,
    /// Macro expansion whose expansion produced this node.
    MacroExpansion,
    /// User source declaration that requested a resource, such as `\input` or `\usepackage`.
    ResourceRequest,
    /// Package file that defined a macro or environment used here.
    Package,
    /// TeX font definition source such as `.fd`.
    FontDefinition,
    /// Generic resource file referenced during typesetting.
    Resource,
}

/// A half open byte span.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ByteSpan {
    /// Inclusive start byte.
    pub start: u32,
    /// Exclusive end byte.
    pub end: u32,
}

/// A point in scaled layout units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: Length,
    /// Vertical coordinate.
    pub y: Length,
}

/// A two dimensional size in layout units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    /// Horizontal extent.
    pub width: Length,
    /// Vertical extent.
    pub height: Length,
}

/// A rectangle in layout coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// Top left origin.
    pub origin: Point,
    /// Width and height of the rectangle.
    pub size: Size,
}

/// TeX style scaled point value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Length(pub i32);

impl Length {
    /// Zero length.
    pub const ZERO: Self = Self(0);

    /// Constructs a length from a raw scaled point value.
    #[must_use]
    pub const fn from_scaled_points(value: i32) -> Self {
        Self(value)
    }
}

/// Stable node identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// Stable source identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

/// Stable font identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FontId(pub u32);

/// Backend independent glyph identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u32);

/// Glyph outline command in font design units, with y increasing upward.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutlineCommand {
    /// Move the pen to a new position.
    MoveTo {
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// Draw a straight line to the given position.
    LineTo {
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// Draw a quadratic Bezier curve.
    QuadTo {
        /// Control point horizontal position.
        cx: f32,
        /// Control point vertical position.
        cy: f32,
        /// End point horizontal position.
        x: f32,
        /// End point vertical position.
        y: f32,
    },
    /// Draw a cubic Bezier curve.
    CurveTo {
        /// First control point horizontal position.
        c1x: f32,
        /// First control point vertical position.
        c1y: f32,
        /// Second control point horizontal position.
        c2x: f32,
        /// Second control point vertical position.
        c2y: f32,
        /// End point horizontal position.
        x: f32,
        /// End point vertical position.
        y: f32,
    },
    /// Close the current subpath.
    Close,
}

/// Glyph outline in font design units, with empty commands for invisible glyphs such as spaces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlyphOutline {
    /// The font units per em, outline coordinates are in these units.
    pub units_per_em: u16,
    /// Ordered outline path commands for this glyph.
    pub commands: Vec<OutlineCommand>,
}

/// Four byte tag in OpenType format.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Tag(pub [u8; 4]);

/// Text direction for glyph runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Direction {
    /// Left to right, the default for Latin scripts.
    #[default]
    LeftToRight,
    /// Right to left, used for Arabic and Hebrew.
    RightToLeft,
    /// Top to bottom, used for some vertical layouts.
    TopToBottom,
}

/// Color with red, green, blue, and alpha channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha channel.
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_map_tracks_multiple_sources_for_node() {
        let mut source_map = SourceMap::default();
        let input = source_map.add_source("input");
        let package = source_map.add_source("amsmath.sty");
        let input_again = source_map.intern_source("input");

        source_map.add_entry(
            NodeId(7),
            SourceRange {
                source: input,
                span: ByteSpan { start: 1, end: 5 },
            },
            SourceRole::Primary,
        );
        source_map.add_entry(
            NodeId(7),
            SourceRange {
                source: package,
                span: ByteSpan { start: 10, end: 20 },
            },
            SourceRole::Package,
        );
        source_map.add_entry(
            NodeId(7),
            SourceRange {
                source: input,
                span: ByteSpan { start: 0, end: 16 },
            },
            SourceRole::ResourceRequest,
        );

        let entries = source_map.entries_for_node(NodeId(7)).count();

        assert_eq!(input_again, input);
        assert_eq!(source_map.sources.len(), 2);
        assert_eq!(entries, 3);
        assert_eq!(
            source_map.source(input).expect("input source").name,
            "input"
        );
        assert_eq!(
            source_map.primary_range_for_node(NodeId(7)),
            Some(SourceRange {
                source: input,
                span: ByteSpan { start: 1, end: 5 },
            })
        );

        let fragment = Fragment {
            source_map,
            ..Fragment::default()
        };
        let origins = fragment
            .source_origins_for_node(NodeId(7))
            .collect::<Vec<_>>();
        assert_eq!(origins.len(), 3);
        assert_eq!(origins[0].source.name, "input");
        assert_eq!(origins[0].span, ByteSpan { start: 1, end: 5 });
        assert_eq!(origins[1].source.name, "amsmath.sty");
        assert_eq!(origins[1].role, SourceRole::Package);
        assert_eq!(origins[2].role, SourceRole::ResourceRequest);
    }

    #[test]
    fn fragment_resolves_glyph_clusters_to_original_source_ranges() {
        let mut source_map = SourceMap::default();
        let input = source_map.add_source("input");
        let node = NodeId(4);
        source_map.add_entry(
            node,
            SourceRange {
                source: input,
                span: ByteSpan { start: 0, end: 6 },
            },
            SourceRole::Primary,
        );
        let fragment = Fragment {
            source_map,
            nodes: alloc::vec![LayoutNode {
                id: node,
                origin: Point::default(),
                bounds: Rect::default(),
                primary_source: None,
                style: Style::default(),
                kind: LayoutNodeKind::GlyphRun(GlyphRun {
                    font: FontRef {
                        id: FontId(1),
                        name: "test".into(),
                        size: Length::ZERO,
                        features: alloc::vec![],
                    },
                    direction: Direction::LeftToRight,
                    script: None,
                    language: None,
                    glyphs: alloc::vec![PositionedGlyph {
                        glyph_id: GlyphId(9),
                        offset: Point::default(),
                        advance: Point::default(),
                        cluster: Some(ByteSpan { start: 2, end: 4 }),
                    }],
                }),
            }],
            ..Fragment::default()
        };

        assert_eq!(fragment.node(node).expect("node").id, node);
        assert_eq!(
            fragment.glyph_source_range(node, 0),
            Some(SourceRange {
                source: input,
                span: ByteSpan { start: 2, end: 4 },
            })
        );
        let origin = fragment
            .glyph_source_origin(node, 0)
            .expect("glyph source origin");
        assert_eq!(origin.source.name, "input");
        assert_eq!(origin.span, ByteSpan { start: 2, end: 4 });
        assert_eq!(origin.role, SourceRole::Primary);
        assert_eq!(fragment.glyph_source_range(node, 1), None);
    }

    #[test]
    fn ir_represents_tex_boxes_and_lists_without_renderer_terms() {
        let list = LayoutNode {
            id: NodeId(1),
            origin: Point::default(),
            bounds: Rect::default(),
            primary_source: None,
            style: Style::default(),
            kind: LayoutNodeKind::List(LayoutList {
                kind: ListKind::Math,
                children: alloc::vec![NodeId(2)],
            }),
        };
        let boxed = LayoutNode {
            id: NodeId(2),
            origin: Point {
                x: Length::ZERO,
                y: Length::from_scaled_points(10_000),
            },
            bounds: Rect::default(),
            primary_source: None,
            style: Style::default(),
            kind: LayoutNodeKind::Box(LayoutBox {
                kind: BoxKind::Horizontal,
                metrics: BoxMetrics {
                    width: Length::from_scaled_points(65_536),
                    height: Length::from_scaled_points(32_768),
                    depth: Length::from_scaled_points(16_384),
                    shift: Length::ZERO,
                },
                children: alloc::vec![],
            }),
        };
        let glyphs = LayoutNode {
            id: NodeId(3),
            origin: Point {
                x: Length::ZERO,
                y: Length::from_scaled_points(50_000),
            },
            bounds: Rect::default(),
            primary_source: None,
            style: Style::default(),
            kind: LayoutNodeKind::GlyphRun(GlyphRun {
                font: FontRef {
                    id: FontId(1),
                    name: "test".into(),
                    size: Length::ZERO,
                    features: alloc::vec![],
                },
                direction: Direction::LeftToRight,
                script: None,
                language: None,
                glyphs: alloc::vec![],
            }),
        };

        let fragment = Fragment {
            nodes: alloc::vec![list, boxed, glyphs],
            ..Fragment::default()
        };

        assert_eq!(fragment.nodes[0].baseline_y(), None);
        assert_eq!(
            fragment.nodes[1].baseline_y(),
            Some(Length::from_scaled_points(42_768))
        );
        assert_eq!(
            fragment.nodes[2].baseline_y(),
            Some(Length::from_scaled_points(50_000))
        );

        match &fragment.nodes[0].kind {
            LayoutNodeKind::List(layout_list) => {
                assert_eq!(layout_list.kind, ListKind::Math);
                assert_eq!(layout_list.children, alloc::vec![NodeId(2)]);
            }
            other => panic!("unexpected node: {other:?}"),
        }

        match &fragment.nodes[1].kind {
            LayoutNodeKind::Box(layout_box) => {
                assert_eq!(layout_box.kind, BoxKind::Horizontal);
                assert_eq!(layout_box.metrics.depth, Length::from_scaled_points(16_384));
                assert_eq!(
                    layout_box.metrics.baseline_offset(),
                    Length::from_scaled_points(32_768)
                );
                assert_eq!(
                    layout_box.metrics.total_height(),
                    Length::from_scaled_points(49_152)
                );
            }
            other => panic!("unexpected node: {other:?}"),
        }
    }
}
