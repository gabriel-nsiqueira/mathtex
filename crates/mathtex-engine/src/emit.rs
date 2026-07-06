use alloc::vec::Vec;

use mathtex_ir::{
    Fragment, FragmentMetadata, Glue, GlyphRun, Kern, LayoutBox, LayoutList, LayoutNode,
    LayoutNodeKind, ListKind, NodeId, Point, Rect, Rule, Style, Surface,
};

/// IR emitter for layout code, replacing TeX shipout.
#[derive(Clone, Debug)]
pub struct IrEmitter {
    fragment: Fragment,
    next_node: u32,
}

impl IrEmitter {
    /// Creates a new emitter for a fragment with the given metadata.
    #[must_use]
    pub fn new(metadata: FragmentMetadata) -> Self {
        Self {
            fragment: Fragment {
                metadata,
                ..Fragment::default()
            },
            next_node: 0,
        }
    }

    /// Sets the drawing surface dimensions for the current fragment.
    pub fn set_surface(&mut self, surface: Surface) {
        self.fragment.surface = surface;
    }

    /// Emits a box node and returns its identifier.
    pub fn emit_box(&mut self, node: EmitNode, layout_box: LayoutBox) -> NodeId {
        self.emit(node, LayoutNodeKind::Box(layout_box))
    }

    /// Emits a list node wrapping the given children and returns its identifier.
    pub fn emit_list(&mut self, node: EmitNode, kind: ListKind, children: Vec<NodeId>) -> NodeId {
        self.emit(node, LayoutNodeKind::List(LayoutList { kind, children }))
    }

    /// Emits a glyph run node and returns its identifier.
    pub fn emit_glyph_run(&mut self, node: EmitNode, glyph_run: GlyphRun) -> NodeId {
        self.emit(node, LayoutNodeKind::GlyphRun(glyph_run))
    }

    /// Emits a rule node and returns its identifier.
    pub fn emit_rule(&mut self, node: EmitNode, rule: Rule) -> NodeId {
        self.emit(node, LayoutNodeKind::Rule(rule))
    }

    /// Emits a glue node and returns its identifier.
    pub fn emit_glue(&mut self, node: EmitNode, glue: Glue) -> NodeId {
        self.emit(node, LayoutNodeKind::Glue(glue))
    }

    /// Emits a kern node and returns its identifier.
    pub fn emit_kern(&mut self, node: EmitNode, kern: Kern) -> NodeId {
        self.emit(node, LayoutNodeKind::Kern(kern))
    }

    /// Consumes the emitter and returns the completed fragment.
    #[must_use]
    pub fn finish(self) -> Fragment {
        self.fragment
    }

    fn emit(&mut self, node: EmitNode, kind: LayoutNodeKind) -> NodeId {
        let id = NodeId(self.next_node);
        self.next_node += 1;

        self.fragment.nodes.push(LayoutNode {
            id,
            origin: node.origin,
            bounds: node.bounds,
            primary_source: None,
            style: node.style,
            kind,
        });

        id
    }
}

/// Geometry and style carried by a single layout node during emission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EmitNode {
    /// Position of the node origin in fragment coordinates.
    pub origin: Point,
    /// Bounding box of the node in fragment coordinates.
    pub bounds: Rect,
    /// TeX style level applied to nodes within this region.
    pub style: Style,
}

impl EmitNode {
    /// Returns a copy of this node with the style set to `style`.
    #[must_use]
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

