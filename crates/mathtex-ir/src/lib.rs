//! Renderer neutral layout IR for one typeset TeX fragment, with source spans.
#![forbid(unsafe_code)]

use std::fmt;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

/// One typeset fragment: a tree of layout nodes rooted at `root`, stored densely by id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fragment {
    /// The root node, placed at (0, 0) with its reference point on the surface baseline.
    pub root: NodeId,
    /// Every node of the tree, `nodes[i].id == NodeId(i)`.
    pub nodes: Vec<LayoutNode>,
    /// Source files and enclosing construct spans.
    pub source_map: SourceMap,
    /// How the fragment was produced.
    pub metadata: FragmentMetadata,
    /// Extent of the root box, `surface.baseline == root.height`.
    pub surface: Surface,
}

impl Fragment {
    /// Builds a fragment, deriving the surface from the root and checking every invariant.
    pub fn new(
        root: NodeId,
        nodes: Vec<LayoutNode>,
        source_map: SourceMap,
        metadata: FragmentMetadata,
    ) -> Result<Self, FragmentError> {
        let surface = nodes
            .get(root.index())
            .map_or_else(Surface::default, Surface::of_root);
        let fragment = Self {
            root,
            nodes,
            source_map,
            metadata,
            surface,
        };
        fragment.validate()?;
        Ok(fragment)
    }

    /// Checks dense ids, a single tree under the root, the root placement, the surface and source ids.
    pub fn validate(&self) -> Result<(), FragmentError> {
        for (index, node) in self.nodes.iter().enumerate() {
            if node.id.index() != index {
                return Err(FragmentError::IdMismatch { index, id: node.id });
            }
        }
        if self.nodes.is_empty() {
            return Ok(());
        }
        let root = self
            .nodes
            .get(self.root.index())
            .ok_or(FragmentError::RootOutOfRange(self.root))?;
        if root.origin != Point::ORIGIN {
            return Err(FragmentError::RootNotAtOrigin);
        }
        if self.surface != Surface::of_root(root) {
            return Err(FragmentError::SurfaceMismatch);
        }
        let mut has_parent = vec![false; self.nodes.len()];
        for node in &self.nodes {
            for &child in node.children() {
                let slot =
                    has_parent
                        .get_mut(child.index())
                        .ok_or(FragmentError::ChildOutOfRange {
                            parent: node.id,
                            child,
                        })?;
                if *slot || child == self.root {
                    return Err(FragmentError::MultipleParents(child));
                }
                *slot = true;
            }
        }
        let mut reached = vec![false; self.nodes.len()];
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if std::mem::replace(&mut reached[id.index()], true) {
                return Err(FragmentError::MultipleParents(id));
            }
            stack.extend_from_slice(self.nodes[id.index()].children());
        }
        if let Some(index) = reached.iter().position(|reached| !reached) {
            return Err(FragmentError::Unreachable(self.nodes[index].id));
        }
        for (index, file) in self.source_map.sources.iter().enumerate() {
            if file.id.index() != index {
                return Err(FragmentError::SourceIdMismatch { index, id: file.id });
            }
        }
        let known = |range: &SourceRange| self.source_map.source(range.source).is_some();
        for node in &self.nodes {
            if let Some(range) = &node.primary_source {
                if !known(range) {
                    return Err(FragmentError::UnknownSource {
                        node: node.id,
                        source: range.source,
                    });
                }
            }
        }
        for entry in &self.source_map.entries {
            if entry.node.index() >= self.nodes.len() {
                return Err(FragmentError::Unreachable(entry.node));
            }
            if !known(&entry.range) {
                return Err(FragmentError::UnknownSource {
                    node: entry.node,
                    source: entry.range.source,
                });
            }
        }
        Ok(())
    }

    /// Returns the node with the given id in constant time.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&LayoutNode> {
        self.nodes.get(id.index()).filter(|node| node.id == id)
    }

    /// Returns the root node, `None` for an empty fragment.
    #[must_use]
    pub fn root_node(&self) -> Option<&LayoutNode> {
        self.node(self.root)
    }

    /// Returns the children of a node, empty for leaves and unknown ids.
    #[must_use]
    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.node(id).map_or(&[], LayoutNode::children)
    }

    /// Iterates the enclosing construct entries of a node, innermost first.
    pub fn source_entries_for_node(&self, node: NodeId) -> impl Iterator<Item = &SourceMapEntry> {
        self.source_map.entries_for_node(node)
    }

    /// Iterates the node's primary origin followed by its enclosing construct origins.
    pub fn source_origins_for_node(&self, node: NodeId) -> impl Iterator<Item = SourceOrigin<'_>> {
        let primary = self
            .primary_source_for_node(node)
            .and_then(|range| self.origin(node, range, SourceRole::Primary));
        let enclosing = self.source_entries_for_node(node).filter_map(move |entry| {
            self.origin(node, entry.range, SourceRole::EnclosingConstruct)
        });
        primary.into_iter().chain(enclosing)
    }

    /// Returns the primary source range of a node.
    #[must_use]
    pub fn primary_source_for_node(&self, node: NodeId) -> Option<SourceRange> {
        self.node(node)?.primary_source
    }

    /// Resolves a glyph's cluster span to a range in the node's primary source file.
    #[must_use]
    pub fn glyph_source_range(&self, node: NodeId, glyph_index: usize) -> Option<SourceRange> {
        let node = self.node(node)?;
        let LayoutNodeKind::GlyphRun(run) = &node.kind else {
            return None;
        };
        let span = run.glyphs.get(glyph_index)?.cluster?;
        Some(SourceRange {
            source: node.primary_source?.source,
            span,
        })
    }

    /// Resolves a glyph's cluster span to a full source origin.
    #[must_use]
    pub fn glyph_source_origin(
        &self,
        node: NodeId,
        glyph_index: usize,
    ) -> Option<SourceOrigin<'_>> {
        let range = self.glyph_source_range(node, glyph_index)?;
        self.origin(node, range, SourceRole::Primary)
    }

    /// Absolute glyphs and rules in tree order, y down, origin at the surface top left.
    #[must_use]
    pub fn flatten(&self) -> Vec<Placed<'_>> {
        let mut out = Vec::new();
        if self.root_node().is_none() {
            return out;
        }
        let mut visited = vec![false; self.nodes.len()];
        let mut stack = vec![(self.root, Point::new(Length::ZERO, self.surface.baseline))];
        while let Some((id, parent)) = stack.pop() {
            let Some(node) = self.node(id) else {
                continue;
            };
            if std::mem::replace(&mut visited[id.index()], true) {
                continue;
            }
            let at = parent + node.origin;
            match &node.kind {
                LayoutNodeKind::Box(layout_box) => {
                    stack.extend(layout_box.children.iter().rev().map(|&child| (child, at)));
                }
                LayoutNodeKind::GlyphRun(run) => {
                    for (index, glyph) in run.glyphs.iter().enumerate() {
                        let source = self.glyph_source_range(id, index).or(node.primary_source);
                        out.push(Placed::Glyph {
                            node: id,
                            font: &run.font,
                            glyph_id: glyph.glyph_id,
                            x: at.x + glyph.offset.x,
                            y: at.y + glyph.offset.y,
                            source,
                        });
                    }
                }
                LayoutNodeKind::Rule => {
                    let height = node.height + node.depth;
                    // TeX draws a rule only when both its width and total height are positive.
                    if node.width > Length::ZERO && height > Length::ZERO {
                        out.push(Placed::Rule {
                            node: id,
                            x: at.x,
                            y: at.y - node.height,
                            width: node.width,
                            height,
                            source: node.primary_source,
                        });
                    }
                }
                LayoutNodeKind::Glue(_) | LayoutNodeKind::Kern(_) => {}
            }
        }
        out
    }

    fn origin(
        &self,
        node: NodeId,
        range: SourceRange,
        role: SourceRole,
    ) -> Option<SourceOrigin<'_>> {
        Some(SourceOrigin {
            node,
            source: self.source_map.source(range.source)?,
            span: range.span,
            role,
        })
    }
}

/// A drawable item from [`Fragment::flatten`], lengths absolute from the surface top left, y down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Placed<'a> {
    /// One glyph, `(x, y)` is its origin on the baseline.
    Glyph {
        /// The glyph run node the glyph belongs to.
        node: NodeId,
        /// Font of the glyph run.
        font: &'a FontRef,
        /// Glyph index, or the character code for TFM fonts.
        glyph_id: GlyphId,
        /// Horizontal position of the glyph origin.
        x: Length,
        /// Baseline position of the glyph origin.
        y: Length,
        /// The glyph's cluster range, or the run's primary source.
        source: Option<SourceRange>,
    },
    /// One filled rectangle, `(x, y)` is its top left corner.
    Rule {
        /// The rule node.
        node: NodeId,
        /// Left edge.
        x: Length,
        /// Top edge.
        y: Length,
        /// Horizontal extent.
        width: Length,
        /// Vertical extent, the rule's height plus depth.
        height: Length,
        /// The rule's primary source.
        source: Option<SourceRange>,
    },
}

/// Why a fragment breaks the IR invariants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FragmentError {
    /// A node is stored at an index other than its id.
    IdMismatch {
        /// Position in `nodes`.
        index: usize,
        /// Id stored there.
        id: NodeId,
    },
    /// The root id indexes no node.
    RootOutOfRange(NodeId),
    /// The root origin is not (0, 0).
    RootNotAtOrigin,
    /// The surface does not match the root's width, height and depth.
    SurfaceMismatch,
    /// A box lists a child id that indexes no node.
    ChildOutOfRange {
        /// The box listing the child.
        parent: NodeId,
        /// The missing child id.
        child: NodeId,
    },
    /// A node is listed as a child more than once, or the root is listed as a child.
    MultipleParents(NodeId),
    /// A node is not reachable from the root.
    Unreachable(NodeId),
    /// A source file is stored at an index other than its id.
    SourceIdMismatch {
        /// Position in `sources`.
        index: usize,
        /// Id stored there.
        id: SourceId,
    },
    /// A node or entry refers to an unregistered source file.
    UnknownSource {
        /// The node carrying the range.
        node: NodeId,
        /// The unknown source id.
        source: SourceId,
    },
}

impl fmt::Display for FragmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdMismatch { index, id } => {
                write!(f, "node at index {index} has id {}", id.0)
            }
            Self::RootOutOfRange(id) => write!(f, "root id {} indexes no node", id.0),
            Self::RootNotAtOrigin => f.write_str("root origin is not (0, 0)"),
            Self::SurfaceMismatch => f.write_str("surface does not match the root box"),
            Self::ChildOutOfRange { parent, child } => {
                write!(f, "node {} lists missing child {}", parent.0, child.0)
            }
            Self::MultipleParents(id) => write!(f, "node {} has more than one parent", id.0),
            Self::Unreachable(id) => write!(f, "node {} is not reachable from the root", id.0),
            Self::SourceIdMismatch { index, id } => {
                write!(f, "source at index {index} has id {}", id.0)
            }
            Self::UnknownSource { node, source } => {
                write!(f, "node {} refers to unknown source {}", node.0, source.0)
            }
        }
    }
}

impl std::error::Error for FragmentError {}

/// A resolved source relationship of a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceOrigin<'a> {
    /// The node.
    pub node: NodeId,
    /// The source file holding the span.
    pub source: &'a SourceFile,
    /// Byte span within that file.
    pub span: ByteSpan,
    /// Whether the span is the node's own source or an enclosing construct.
    pub role: SourceRole,
}

/// How a fragment was produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FragmentMetadata {
    /// Format identifier, such as `plain`, `latex` or a host chosen name.
    pub format_id: String,
    /// Mode the fragment was typeset in.
    pub fragment_kind: FragmentKind,
}

/// Mode a fragment was typeset in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FragmentKind {
    /// Inline math.
    #[default]
    MathInline,
    /// Display math.
    MathDisplay,
    /// Horizontal mode text.
    Text,
}

/// Extent of a fragment, the root box's width, total height and height above the baseline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Surface {
    /// Root width.
    pub width: Length,
    /// Root height plus depth.
    pub height: Length,
    /// Distance from the top edge down to the baseline, the root height.
    pub baseline: Length,
}

impl Surface {
    /// The surface a root node spans.
    #[must_use]
    pub fn of_root(root: &LayoutNode) -> Self {
        Self {
            width: root.width,
            height: root.height + root.depth,
            baseline: root.height,
        }
    }
}

/// A node of the layout tree.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutNode {
    /// Id, equal to the node's index in [`Fragment::nodes`].
    pub id: NodeId,
    /// Reference point relative to the parent's reference point, the box's left baseline point, y down.
    pub origin: Point,
    /// Horizontal extent to the right of the reference point.
    pub width: Length,
    /// Extent above the reference point.
    pub height: Length,
    /// Extent below the reference point.
    pub depth: Length,
    /// The source span this node was typeset from.
    pub primary_source: Option<SourceRange>,
    /// What the node is.
    pub kind: LayoutNodeKind,
}

impl LayoutNode {
    /// Returns the children of a box, empty for every other node.
    #[must_use]
    pub fn children(&self) -> &[NodeId] {
        match &self.kind {
            LayoutNodeKind::Box(layout_box) => &layout_box.children,
            _ => &[],
        }
    }
}

/// What a layout node is.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum LayoutNodeKind {
    /// A box with positioned children.
    Box(LayoutBox),
    /// Glyphs of one font.
    GlyphRun(GlyphRun),
    /// A filled rectangle spanning the node's width, height and depth.
    Rule,
    /// Glue after glue setting.
    Glue(Glue),
    /// A kern.
    Kern(Kern),
}

/// A TeX box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutBox {
    /// Horizontal or vertical list.
    pub kind: BoxKind,
    /// Children in list order.
    pub children: Vec<NodeId>,
}

/// Whether a box holds a horizontal or a vertical list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoxKind {
    /// An hbox.
    Horizontal,
    /// A vbox.
    Vertical,
}

/// Direction a glue or kern moves the list position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// Along a horizontal list.
    #[default]
    Horizontal,
    /// Along a vertical list.
    Vertical,
}

/// Glyphs of one font.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphRun {
    /// Font of every glyph in the run.
    pub font: FontRef,
    /// Glyphs in visual order.
    pub glyphs: Vec<PositionedGlyph>,
}

/// A glyph placed relative to its run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PositionedGlyph {
    /// Glyph index in an OpenType font, or the character code for TFM fonts.
    pub glyph_id: GlyphId,
    /// Offset of the glyph origin from the run's reference point, y down.
    pub offset: Point,
    /// Source bytes the glyph was shaped from, in the run's primary source file.
    pub cluster: Option<ByteSpan>,
}

/// A font as the IR names it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontRef {
    /// The key the host's font loader returned, `None` for TFM fonts.
    pub key: Option<FontKey>,
    /// The XeTeX font spec of a native font, the TFM name, or empty for host box runs.
    pub spec: String,
    /// Size the font was loaded at.
    pub size: Length,
}

/// Host chosen identity of a loaded font face.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontKey(pub u64);

/// Glue after glue setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Glue {
    /// Set size along the axis, natural size plus the stretch or shrink the parent box applied.
    pub amount: Length,
    /// List direction the glue lies in.
    pub axis: Axis,
}

/// A kern.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Kern {
    /// Size along the axis.
    pub amount: Length,
    /// List direction the kern lies in.
    pub axis: Axis,
}

/// Source files and the enclosing construct spans of nodes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceMap {
    /// Registered source files, `sources[i].id == SourceId(i)`.
    pub sources: Vec<SourceFile>,
    /// Enclosing construct spans, innermost first per node.
    pub entries: Vec<SourceMapEntry>,
}

impl SourceMap {
    /// Registers a source file and returns its id.
    pub fn add_source(&mut self, name: impl Into<String>) -> SourceId {
        let id = SourceId(u32::try_from(self.sources.len()).unwrap_or(u32::MAX));
        self.sources.push(SourceFile {
            id,
            name: name.into(),
        });
        id
    }

    /// Returns the id of a source file by name, registering it when new.
    pub fn intern_source(&mut self, name: impl Into<String>) -> SourceId {
        let name = name.into();
        if let Some(source) = self.sources.iter().find(|source| source.name == name) {
            return source.id;
        }
        self.add_source(name)
    }

    /// Records an enclosing construct span of a node.
    pub fn add_entry(&mut self, node: NodeId, range: SourceRange) {
        self.entries.push(SourceMapEntry { node, range });
    }

    /// Returns a registered source file.
    #[must_use]
    pub fn source(&self, id: SourceId) -> Option<&SourceFile> {
        self.sources
            .get(id.index())
            .filter(|source| source.id == id)
    }

    /// Iterates the enclosing construct entries of a node.
    pub fn entries_for_node(&self, node: NodeId) -> impl Iterator<Item = &SourceMapEntry> {
        self.entries.iter().filter(move |entry| entry.node == node)
    }
}

/// A source file, such as the host's input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// Id, equal to the file's index in [`SourceMap::sources`].
    pub id: SourceId,
    /// Name the engine read the file under.
    pub name: String,
}

/// An enclosing construct span of a node, such as a macro call around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceMapEntry {
    /// The node.
    pub node: NodeId,
    /// The enclosing span.
    pub range: SourceRange,
}

/// A span within a registered source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceRange {
    /// The file.
    pub source: SourceId,
    /// Byte span in that file.
    pub span: ByteSpan,
}

/// How a source span relates to a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceRole {
    /// The span the node was typeset from.
    Primary,
    /// A construct enclosing the node's own span.
    EnclosingConstruct,
}

/// A half open span of UTF-8 byte offsets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    /// First byte.
    pub start: u32,
    /// One past the last byte.
    pub end: u32,
}

/// A point in scaled points, y down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: Length,
    /// Vertical coordinate, growing downward.
    pub y: Length,
}

impl Point {
    /// The point (0, 0).
    pub const ORIGIN: Self = Self {
        x: Length::ZERO,
        y: Length::ZERO,
    };

    /// Builds a point.
    #[must_use]
    pub const fn new(x: Length, y: Length) -> Self {
        Self { x, y }
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

/// A TeX dimension in scaled points, 65536 per point, arithmetic saturates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Length(pub i32);

impl Length {
    /// Zero length.
    pub const ZERO: Self = Self(0);
    /// Scaled points per TeX point.
    pub const SP_PER_PT: i32 = 65_536;

    /// Builds a length from scaled points.
    #[must_use]
    pub const fn from_scaled_points(value: i32) -> Self {
        Self(value)
    }

    /// Builds a length from TeX points, rounded to the nearest scaled point.
    #[must_use]
    pub fn from_pt(points: f64) -> Self {
        Self((points * f64::from(Self::SP_PER_PT)).round() as i32)
    }

    /// Returns the length in TeX points.
    #[must_use]
    pub fn to_pt(self) -> f64 {
        f64::from(self.0) / f64::from(Self::SP_PER_PT)
    }
}

impl Add for Length {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl AddAssign for Length {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Length {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl SubAssign for Length {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for Length {
    type Output = Self;

    fn neg(self) -> Self {
        Self(self.0.saturating_neg())
    }
}

impl Mul<i32> for Length {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self {
        Self(self.0.saturating_mul(rhs))
    }
}

impl Mul<f64> for Length {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self((f64::from(self.0) * rhs).round() as i32)
    }
}

/// Node id, the node's index in [`Fragment::nodes`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Returns the id as a vector index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Source file id, the file's index in [`SourceMap::sources`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(pub u32);

impl SourceId {
    /// Returns the id as a vector index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Glyph index in an OpenType font, or the character code for TFM fonts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlyphId(pub u32);

/// Glyph outline command in font design units, y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutlineCommand {
    /// Starts a contour.
    MoveTo {
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// Straight segment.
    LineTo {
        /// Horizontal position.
        x: f32,
        /// Vertical position.
        y: f32,
    },
    /// Quadratic Bezier segment.
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
    /// Cubic Bezier segment.
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
    /// Closes the contour.
    Close,
}

/// Glyph outline in font design units, y up, empty for blank glyphs such as spaces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlyphOutline {
    /// Design units per em of the font.
    pub units_per_em: u16,
    /// Contour commands.
    pub commands: Vec<OutlineCommand>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const PT: i32 = Length::SP_PER_PT;

    fn node(
        id: u32,
        origin: (i32, i32),
        size: (i32, i32, i32),
        kind: LayoutNodeKind,
    ) -> LayoutNode {
        LayoutNode {
            id: NodeId(id),
            origin: Point::new(Length(origin.0 * PT), Length(origin.1 * PT)),
            width: Length(size.0 * PT),
            height: Length(size.1 * PT),
            depth: Length(size.2 * PT),
            primary_source: None,
            kind,
        }
    }

    fn hbox(children: &[u32]) -> LayoutNodeKind {
        LayoutNodeKind::Box(LayoutBox {
            kind: BoxKind::Horizontal,
            children: children.iter().copied().map(NodeId).collect(),
        })
    }

    fn run(glyphs: &[(u32, i32, i32)]) -> LayoutNodeKind {
        LayoutNodeKind::GlyphRun(GlyphRun {
            font: FontRef {
                key: Some(FontKey(3)),
                spec: "[test.otf]".into(),
                size: Length(10 * PT),
            },
            glyphs: glyphs
                .iter()
                .map(|&(glyph, x, y)| PositionedGlyph {
                    glyph_id: GlyphId(glyph),
                    offset: Point::new(Length(x * PT), Length(y * PT)),
                    cluster: None,
                })
                .collect(),
        })
    }

    fn sample() -> Fragment {
        let nodes = vec![
            node(0, (0, 0), (20, 8, 2), hbox(&[1, 3])),
            node(1, (3, -2), (5, 4, 0), hbox(&[2])),
            node(2, (1, 0), (4, 4, 0), run(&[(7, 0, 0), (8, 2, 1)])),
            node(3, (10, 0), (6, 3, 1), LayoutNodeKind::Rule),
        ];
        Fragment::new(
            NodeId(0),
            nodes,
            SourceMap::default(),
            FragmentMetadata::default(),
        )
        .expect("valid fragment")
    }

    #[test]
    fn surface_derives_from_the_root() {
        let fragment = sample();
        assert_eq!(fragment.surface.width, Length(20 * PT));
        assert_eq!(fragment.surface.height, Length(10 * PT));
        assert_eq!(fragment.surface.baseline, Length(8 * PT));
    }

    #[test]
    fn flatten_accumulates_parent_relative_origins_from_the_baseline() {
        let fragment = sample();
        let placed = fragment.flatten();
        let pt = |value: i32| Length(value * PT);
        assert_eq!(placed.len(), 3);
        // Run origin is (3 + 1, 8 - 2), the second glyph adds its offset of (2, 1).
        assert!(
            matches!(placed[0], Placed::Glyph { glyph_id: GlyphId(7), x, y, .. } if x == pt(4) && y == pt(6))
        );
        assert!(
            matches!(placed[1], Placed::Glyph { glyph_id: GlyphId(8), x, y, .. } if x == pt(6) && y == pt(7))
        );
        // The rule's top is its baseline minus its height, its extent is height plus depth.
        assert_eq!(
            placed[2],
            Placed::Rule {
                node: NodeId(3),
                x: pt(10),
                y: pt(5),
                width: pt(6),
                height: pt(4),
                source: None,
            }
        );
    }

    #[test]
    fn validate_rejects_broken_trees() {
        let mut fragment = sample();
        fragment.nodes.swap(1, 2);
        assert!(matches!(
            fragment.validate(),
            Err(FragmentError::IdMismatch { index: 1, .. })
        ));

        let mut fragment = sample();
        if let LayoutNodeKind::Box(root) = &mut fragment.nodes[0].kind {
            root.children.push(NodeId(2));
        }
        assert_eq!(
            fragment.validate(),
            Err(FragmentError::MultipleParents(NodeId(2)))
        );

        let mut fragment = sample();
        if let LayoutNodeKind::Box(root) = &mut fragment.nodes[0].kind {
            root.children.pop();
        }
        assert_eq!(
            fragment.validate(),
            Err(FragmentError::Unreachable(NodeId(3)))
        );

        let mut fragment = sample();
        fragment.nodes[0].origin.x = Length(1);
        assert_eq!(fragment.validate(), Err(FragmentError::RootNotAtOrigin));

        let mut fragment = sample();
        fragment.surface.baseline = Length::ZERO;
        assert_eq!(fragment.validate(), Err(FragmentError::SurfaceMismatch));
    }

    #[test]
    fn source_origins_list_the_primary_span_then_enclosing_spans() {
        let mut fragment = sample();
        let input = fragment.source_map.add_source("input");
        let package = fragment.source_map.add_source("amsmath.sty");
        assert_eq!(fragment.source_map.intern_source("input"), input);
        fragment.nodes[2].primary_source = Some(SourceRange {
            source: input,
            span: ByteSpan { start: 1, end: 5 },
        });
        fragment.source_map.add_entry(
            NodeId(2),
            SourceRange {
                source: package,
                span: ByteSpan { start: 10, end: 20 },
            },
        );
        if let LayoutNodeKind::GlyphRun(run) = &mut fragment.nodes[2].kind {
            run.glyphs[1].cluster = Some(ByteSpan { start: 2, end: 4 });
        }
        fragment.validate().expect("valid");

        let origins = fragment
            .source_origins_for_node(NodeId(2))
            .collect::<Vec<_>>();
        assert_eq!(origins.len(), 2);
        assert_eq!(origins[0].role, SourceRole::Primary);
        assert_eq!(origins[0].span, ByteSpan { start: 1, end: 5 });
        assert_eq!(origins[1].source.name, "amsmath.sty");
        assert_eq!(origins[1].role, SourceRole::EnclosingConstruct);

        assert_eq!(fragment.glyph_source_range(NodeId(2), 0), None);
        let cluster = SourceRange {
            source: input,
            span: ByteSpan { start: 2, end: 4 },
        };
        assert_eq!(fragment.glyph_source_range(NodeId(2), 1), Some(cluster));
        let placed = fragment.flatten();
        assert!(
            matches!(placed[0], Placed::Glyph { source: Some(range), .. } if range.span == ByteSpan { start: 1, end: 5 })
        );
        assert!(matches!(placed[1], Placed::Glyph { source: Some(range), .. } if range == cluster));

        fragment.nodes[3].primary_source = Some(SourceRange {
            source: SourceId(9),
            span: ByteSpan::default(),
        });
        assert!(matches!(
            fragment.validate(),
            Err(FragmentError::UnknownSource { .. })
        ));
    }

    #[test]
    fn length_arithmetic_saturates_and_converts_points() {
        assert_eq!(Length::from_pt(1.5), Length(98_304));
        assert_eq!(Length(98_304).to_pt(), 1.5);
        assert_eq!(Length(3) + Length(4) - Length(10), Length(-3));
        assert_eq!(-Length(5), Length(-5));
        assert_eq!(Length(5) * 3, Length(15));
        assert_eq!(Length(10) * 0.25, Length(3));
        assert_eq!(Length(i32::MAX) + Length(1), Length(i32::MAX));
        assert_eq!(-Length(i32::MIN), Length(i32::MAX));
    }
}
