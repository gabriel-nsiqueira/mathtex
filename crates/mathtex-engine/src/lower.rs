use std::fmt;

use mathtex_ir::{
    Axis, BoxKind, ByteSpan, FontKey, FontRef, Fragment, FragmentError, FragmentMetadata, Glue,
    GlyphId, GlyphRun, Kern, LayoutBox, LayoutNode, LayoutNodeKind, Length, NodeId, Point,
    PositionedGlyph, SourceMap, SourceRange,
};
use mathtex_portable_engine_generated as pe;

use pe::{
    PortableNodeHandle as NodeHandle, PortableNodeKind as NodeKind,
    PortableNodeSnapshot as NodeSnapshot,
};

/// Why an engine node tree could not be lowered to IR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum LowerError {
    /// The tree needs more IR nodes than the cap allows, leaders can repeat a box many times.
    NodeLimit {
        /// The cap that was hit.
        limit: usize,
    },
    /// The root handle is not a node the lowering can read.
    UnreadableRoot,
    /// The lowering produced a fragment that breaks the IR invariants.
    Invalid(FragmentError),
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeLimit { limit } => write!(f, "layout needs more than {limit} IR nodes"),
            Self::UnreadableRoot => f.write_str("the captured root is not a readable node"),
            Self::Invalid(error) => write!(f, "lowering produced an invalid fragment: {error}"),
        }
    }
}

/// Lowers an engine node and everything below it to an IR fragment of at most `max_nodes` nodes.
pub(crate) fn lower(
    engine: &pe::PortableTexEngine<'_>,
    root: NodeHandle,
    metadata: FragmentMetadata,
    max_nodes: usize,
) -> Result<Fragment, LowerError> {
    let mut builder = IrBuilder {
        engine,
        nodes: Vec::new(),
        source_map: SourceMap::default(),
        max_nodes,
    };
    let snapshot = engine
        .snapshot_node(root)
        .ok_or(LowerError::UnreadableRoot)?;
    let root = builder
        .emit_node(snapshot, Point::ORIGIN, None)?
        .ok_or(LowerError::UnreadableRoot)?;
    Fragment::new(root, builder.nodes, builder.source_map, metadata).map_err(LowerError::Invalid)
}

/// TeX's `null_flag`, the value of a running rule dimension.
const RUNNING: i32 = -0x4000_0000;
/// Glue subtypes of `\leaders`, `\cleaders` and `\xleaders`.
const A_LEADERS: i32 = 100;
const C_LEADERS: i32 = 101;
/// TeX's `billion`, the bound `vet_glue` clamps glue products to.
const BILLION: f64 = 1_000_000_000.0;

/// List position and glue rounding state of `hlist_out` and `vlist_out` for one box.
struct ListCursor {
    vertical: bool,
    /// Current position along the list axis, `cur_h` or `cur_v`.
    at: i32,
    /// Start of the list along its axis, `left_edge` or `top_edge`.
    edge: i32,
    cur_g: i32,
    cur_glue: f64,
}

impl ListCursor {
    /// The set size of a glue node, with `hlist_out`'s cumulative rounding of stretch and shrink.
    fn glue_amount(&mut self, parent: &NodeSnapshot, glue: &NodeSnapshot) -> i32 {
        let before = glue.width.saturating_sub(self.cur_g);
        let applies = match parent.glue_sign {
            1 if glue.glue_stretch_order == parent.glue_order => Some(f64::from(glue.glue_stretch)),
            2 if glue.glue_shrink_order == parent.glue_order => Some(-f64::from(glue.glue_shrink)),
            _ => None,
        };
        if let Some(delta) = applies {
            self.cur_glue += delta;
            let glue_temp = (parent.glue_set * self.cur_glue).clamp(-BILLION, BILLION);
            self.cur_g = glue_temp.round() as i32;
        }
        before.saturating_add(self.cur_g)
    }

    fn point(&self, cross: i32, along: i32) -> Point {
        if self.vertical {
            Point::new(Length(cross), Length(along))
        } else {
            Point::new(Length(along), Length(cross))
        }
    }
}

struct IrBuilder<'a, 'resources> {
    engine: &'a pe::PortableTexEngine<'resources>,
    nodes: Vec<LayoutNode>,
    source_map: SourceMap,
    max_nodes: usize,
}

impl IrBuilder<'_, '_> {
    /// Lowers one node placed at `origin`, `None` for nodes that leave no trace in the IR.
    fn emit_node(
        &mut self,
        snapshot: NodeSnapshot,
        origin: Point,
        parent: Option<&NodeSnapshot>,
    ) -> Result<Option<NodeId>, LowerError> {
        let size = (snapshot.width, snapshot.height, snapshot.depth);
        let id = match snapshot.kind {
            NodeKind::HorizontalBox | NodeKind::VerticalBox | NodeKind::UnsetBox => {
                self.emit_box(snapshot, origin)?
            }
            NodeKind::Rule => {
                let size = running_rule(&snapshot, parent);
                self.push(&snapshot, origin, size, LayoutNodeKind::Rule)?
            }
            NodeKind::Character | NodeKind::Ligature => {
                let primary = self.character_source(&snapshot);
                let run = GlyphRun {
                    font: self.font_ref(snapshot.font, None),
                    glyphs: Vec::from([PositionedGlyph {
                        glyph_id: GlyphId(u32::try_from(snapshot.character).unwrap_or(0)),
                        offset: Point::ORIGIN,
                        cluster: primary.as_ref().map(|span| ByteSpan {
                            start: span.start,
                            end: span.end,
                        }),
                    }]),
                };
                self.push_with_source(
                    primary,
                    &snapshot,
                    origin,
                    size,
                    LayoutNodeKind::GlyphRun(run),
                )?
            }
            NodeKind::NativeWord | NodeKind::NativeGlyph => {
                self.emit_native_glyph_run(&snapshot, origin)?
            }
            NodeKind::HostBoxRef => match self.emit_host_box(&snapshot, origin)? {
                Some(id) => id,
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        Ok(Some(id))
    }

    fn emit_box(&mut self, snapshot: NodeSnapshot, origin: Point) -> Result<NodeId, LowerError> {
        let vertical = snapshot.kind == NodeKind::VerticalBox;
        let children = self.emit_list(&snapshot, vertical)?;
        let kind = LayoutNodeKind::Box(LayoutBox {
            kind: if vertical {
                BoxKind::Vertical
            } else {
                BoxKind::Horizontal
            },
            children,
        });
        let size = (snapshot.width, snapshot.height, snapshot.depth);
        self.push(&snapshot, origin, size, kind)
    }

    /// Places a box's list as TeX's `hlist_out` and `vlist_out` move the reference point.
    fn emit_list(
        &mut self,
        parent: &NodeSnapshot,
        vertical: bool,
    ) -> Result<Vec<NodeId>, LowerError> {
        let start = if vertical { -parent.height } else { 0 };
        let mut cursor = ListCursor {
            vertical,
            at: start,
            edge: start,
            cur_g: 0,
            cur_glue: 0.0,
        };
        let mut children = Vec::new();
        let mut next = parent.list;
        while let Some(handle) = next {
            let Some(snapshot) = self.engine.snapshot_node(handle) else {
                break;
            };
            next = snapshot.link;
            let child = match snapshot.kind {
                NodeKind::HorizontalBox | NodeKind::VerticalBox | NodeKind::UnsetBox => {
                    let origin = if vertical {
                        cursor.at += snapshot.height;
                        let origin = cursor.point(snapshot.shift, cursor.at);
                        cursor.at += snapshot.depth;
                        origin
                    } else {
                        let origin = cursor.point(snapshot.shift, cursor.at);
                        cursor.at += snapshot.width;
                        origin
                    };
                    Some(self.emit_box(snapshot, origin)?)
                }
                NodeKind::Rule => {
                    let (width, height, depth) = running_rule(&snapshot, Some(parent));
                    let origin = if vertical {
                        let origin = cursor.point(0, cursor.at + height);
                        cursor.at += height + depth;
                        origin
                    } else {
                        let origin = cursor.point(0, cursor.at);
                        cursor.at += width;
                        origin
                    };
                    Some(self.push(
                        &snapshot,
                        origin,
                        (width, height, depth),
                        LayoutNodeKind::Rule,
                    )?)
                }
                NodeKind::Glue => {
                    let amount = cursor.glue_amount(parent, &snapshot);
                    let child = self.emit_glue(&snapshot, parent, &cursor, amount)?;
                    cursor.at += amount;
                    Some(child)
                }
                NodeKind::Kern => {
                    let child = self.push_spacing(&snapshot, &cursor, snapshot.width, false)?;
                    cursor.at += snapshot.width;
                    Some(child)
                }
                NodeKind::Math if !vertical => {
                    // A math node moves the position by `\mathsurround`, lowered as a kern when nonzero.
                    let child = if snapshot.width == 0 {
                        None
                    } else {
                        Some(self.push_spacing(&snapshot, &cursor, snapshot.width, false)?)
                    };
                    cursor.at += snapshot.width;
                    child
                }
                NodeKind::Character
                | NodeKind::Ligature
                | NodeKind::NativeWord
                | NodeKind::NativeGlyph
                | NodeKind::HostBoxRef
                    if !vertical =>
                {
                    let origin = cursor.point(0, cursor.at);
                    cursor.at += snapshot.width;
                    self.emit_node(snapshot, origin, Some(parent))?
                }
                // XeTeX's vlist_out stacks glyph nodes, such as vertical delimiter assembly parts.
                NodeKind::NativeGlyph => {
                    cursor.at += snapshot.height;
                    let origin = cursor.point(0, cursor.at);
                    cursor.at += snapshot.depth;
                    self.emit_node(snapshot, origin, Some(parent))?
                }
                _ => None,
            };
            children.extend(child);
        }
        Ok(children)
    }

    /// Lowers glue, with rule leaders as a rule and box leaders as a box of the repeated leader boxes.
    fn emit_glue(
        &mut self,
        glue: &NodeSnapshot,
        parent: &NodeSnapshot,
        cursor: &ListCursor,
        amount: i32,
    ) -> Result<NodeId, LowerError> {
        let leader = glue
            .leader
            .filter(|_| glue.subtype >= A_LEADERS)
            .and_then(|handle| self.engine.snapshot_node(handle));
        let Some(leader) = leader else {
            return self.push_spacing(glue, cursor, amount, true);
        };
        if leader.kind == NodeKind::Rule {
            let (origin, size) = if cursor.vertical {
                let width = if leader.width == RUNNING {
                    parent.width
                } else {
                    leader.width
                };
                (cursor.point(0, cursor.at + amount), (width, amount, 0))
            } else {
                let running =
                    |value: i32, fallback: i32| if value == RUNNING { fallback } else { value };
                let size = (
                    amount,
                    running(leader.height, parent.height),
                    running(leader.depth, parent.depth),
                );
                (cursor.point(0, cursor.at), size)
            };
            return self.push(glue, origin, size, LayoutNodeKind::Rule);
        }
        let step = if cursor.vertical {
            leader.height + leader.depth
        } else {
            leader.width
        };
        if step <= 0 || amount <= 0 {
            return self.push_spacing(glue, cursor, amount, true);
        }
        // TeX pads the glue by 10sp against floating point rounding, then places whole boxes inside.
        let span = amount + 10;
        let end = cursor.at + span;
        let mut lx = 0;
        let mut at = cursor.at;
        if glue.subtype == A_LEADERS {
            at = cursor.edge + step * ((at - cursor.edge) / step);
            if at < cursor.at {
                at += step;
            }
        } else {
            let (lq, lr) = (span / step, span % step);
            if glue.subtype == C_LEADERS {
                at += lr / 2;
            } else {
                lx = lr / (lq + 1);
                at += (lr - (lq - 1) * lx) / 2;
            }
        }
        let mut copies = Vec::new();
        while at + step <= end {
            let origin = if cursor.vertical {
                cursor.point(leader.shift, at - cursor.at + leader.height)
            } else {
                cursor.point(leader.shift, at - cursor.at)
            };
            copies.push(self.emit_box(leader.clone(), origin)?);
            at += step + lx;
        }
        let (kind, size) = if cursor.vertical {
            (BoxKind::Vertical, (leader.width, 0, amount))
        } else {
            (BoxKind::Horizontal, (amount, leader.height, leader.depth))
        };
        let container = LayoutNodeKind::Box(LayoutBox {
            kind,
            children: copies,
        });
        self.push(glue, cursor.point(0, cursor.at), size, container)
    }

    /// Pushes a glue or kern at the cursor, a vertical one extends below its origin as a depth.
    fn push_spacing(
        &mut self,
        snapshot: &NodeSnapshot,
        cursor: &ListCursor,
        amount: i32,
        glue: bool,
    ) -> Result<NodeId, LowerError> {
        let (axis, size) = if cursor.vertical {
            (Axis::Vertical, (0, 0, amount))
        } else {
            (Axis::Horizontal, (amount, 0, 0))
        };
        let amount = Length(amount);
        let kind = if glue {
            LayoutNodeKind::Glue(Glue { amount, axis })
        } else {
            LayoutNodeKind::Kern(Kern { amount, axis })
        };
        self.push(snapshot, cursor.point(0, cursor.at), size, kind)
    }

    /// Expands a host box marker into a box holding the host supplied glyph runs and rules.
    fn emit_host_box(
        &mut self,
        snapshot: &NodeSnapshot,
        origin: Point,
    ) -> Result<Option<NodeId>, LowerError> {
        let Some(record) = usize::try_from(snapshot.character)
            .ok()
            .and_then(|index| self.engine.host_box_record(index))
        else {
            return Ok(None);
        };
        let size = (record.width, record.height, record.depth);
        let mut children = Vec::new();
        for run in &record.runs {
            let glyphs = run
                .glyphs
                .iter()
                .map(|glyph| PositionedGlyph {
                    glyph_id: GlyphId(glyph.glyph),
                    offset: Point::new(Length(glyph.x), Length(glyph.y)),
                    cluster: None,
                })
                .collect();
            let kind = LayoutNodeKind::GlyphRun(GlyphRun {
                font: FontRef {
                    key: Some(FontKey(run.font_key)),
                    spec: String::new(),
                    size: Length(run.font_size),
                },
                glyphs,
            });
            // Host box children carry no source, the enclosing box covers them.
            children.push(self.push_with_source(None, snapshot, Point::ORIGIN, size, kind)?);
        }
        for rule in &record.rules {
            // A host rule's (x, y) is its bottom left corner, which is its baseline point with no depth.
            let origin = Point::new(Length(rule.x), Length(rule.y));
            let size = (rule.width, rule.height, 0);
            children.push(self.push_with_source(
                None,
                snapshot,
                origin,
                size,
                LayoutNodeKind::Rule,
            )?);
        }
        let kind = LayoutNodeKind::Box(LayoutBox {
            kind: BoxKind::Horizontal,
            children,
        });
        self.push(snapshot, origin, size, kind).map(Some)
    }

    fn emit_native_glyph_run(
        &mut self,
        snapshot: &NodeSnapshot,
        origin: Point,
    ) -> Result<NodeId, LowerError> {
        let glyphs = if snapshot.native_glyphs.is_empty() {
            Vec::from([PositionedGlyph {
                glyph_id: GlyphId(u32::try_from(snapshot.character).unwrap_or(0)),
                offset: Point::ORIGIN,
                cluster: None,
            }])
        } else {
            snapshot
                .native_glyphs
                .iter()
                .map(|glyph| PositionedGlyph {
                    glyph_id: GlyphId(u32::from(glyph.glyph_id)),
                    offset: Point::new(Length(glyph.x), Length(glyph.y)),
                    // `src_resolve_native_glyphs` fills these spans, 0..0 means unmapped.
                    cluster: (glyph.src_end > glyph.src_start).then_some(ByteSpan {
                        start: glyph.src_start,
                        end: glyph.src_end,
                    }),
                })
                .collect()
        };
        let key = self.engine.native_font_key(snapshot.font).map(FontKey);
        let run = GlyphRun {
            font: self.font_ref(snapshot.font, key),
            glyphs,
        };
        let size = (snapshot.width, snapshot.height, snapshot.depth);
        self.push(snapshot, origin, size, LayoutNodeKind::GlyphRun(run))
    }

    fn font_ref(&self, font: i32, key: Option<FontKey>) -> FontRef {
        FontRef {
            key,
            spec: self
                .engine
                .native_font_spec(font)
                .or_else(|| self.engine.font_name(font))
                .unwrap_or_default(),
            size: Length(self.engine.font_at_size(font)),
        }
    }

    /// A character's own span, or for a ligature the span of the characters it replaced.
    fn character_source(&self, snapshot: &NodeSnapshot) -> Option<pe::PortableSourceSpan> {
        if snapshot.kind != NodeKind::Ligature {
            return snapshot.source.clone();
        }
        // The ligature carries its first character's span, an original may carry an earlier stray span.
        let mut span = snapshot.source.clone();
        let mut next = snapshot.list;
        while let Some(original) = next.and_then(|handle| self.engine.snapshot_node(handle)) {
            next = original.link;
            let Some(part) = original.source else {
                continue;
            };
            match &mut span {
                None => span = Some(part),
                Some(span) if span.name == part.name && part.start >= span.start => {
                    span.end = span.end.max(part.end);
                }
                Some(_) => {}
            }
        }
        span
    }

    fn push(
        &mut self,
        snapshot: &NodeSnapshot,
        origin: Point,
        size: (i32, i32, i32),
        kind: LayoutNodeKind,
    ) -> Result<NodeId, LowerError> {
        let primary = snapshot.source.clone();
        self.push_with_source(primary, snapshot, origin, size, kind)
    }

    /// Appends a node, its id is its index, enclosing spans follow the primary span in the source map.
    fn push_with_source(
        &mut self,
        primary: Option<pe::PortableSourceSpan>,
        snapshot: &NodeSnapshot,
        origin: Point,
        (width, height, depth): (i32, i32, i32),
        kind: LayoutNodeKind,
    ) -> Result<NodeId, LowerError> {
        if self.nodes.len() >= self.max_nodes {
            return Err(LowerError::NodeLimit {
                limit: self.max_nodes,
            });
        }
        let id = NodeId(
            u32::try_from(self.nodes.len()).map_err(|_| LowerError::NodeLimit {
                limit: self.max_nodes,
            })?,
        );
        let primary_source = primary.map(|span| SourceRange {
            source: self.source_map.intern_source(span.name),
            span: ByteSpan {
                start: span.start,
                end: span.end,
            },
        });
        if primary_source.is_some() {
            for span in self.engine.node_enclosing_spans(snapshot.handle) {
                let range = SourceRange {
                    source: self.source_map.intern_source(span.name),
                    span: ByteSpan {
                        start: span.start,
                        end: span.end,
                    },
                };
                self.source_map.add_entry(id, range);
            }
        }
        self.nodes.push(LayoutNode {
            id,
            origin,
            width: Length(width),
            height: Length(height),
            depth: Length(depth),
            primary_source,
            kind,
        });
        Ok(id)
    }
}

/// A rule's size with running dimensions taken from the enclosing box, as `hlist_out` and `vlist_out` do.
fn running_rule(rule: &NodeSnapshot, parent: Option<&NodeSnapshot>) -> (i32, i32, i32) {
    let Some(parent) = parent else {
        return (rule.width, rule.height, rule.depth);
    };
    let running = |value: i32, fallback: i32| if value == RUNNING { fallback } else { value };
    if parent.kind == NodeKind::VerticalBox {
        (running(rule.width, parent.width), rule.height, rule.depth)
    } else {
        (
            rule.width,
            running(rule.height, parent.height),
            running(rule.depth, parent.depth),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::ProviderFiles;
    use crate::resource::{InMemoryResourceProvider, ResourceKind};

    const MAX_NODES: usize = 1 << 20;

    /// An INITEX state with nothing loaded.
    fn initex() -> pe::PortableFormatImage {
        let empty = pe::PortableFormatImage::empty();
        let mut engine = pe::PortableTexEngine::from_format(&empty, pe::EmptyResourceProvider);
        assert!(engine.initialize_format_state());
        engine.into_format()
    }

    /// Lowers the captured root of `body`, run after brace catcodes in a fresh INITEX engine.
    fn lower_with(
        resources: InMemoryResourceProvider,
        body: &str,
        max_nodes: usize,
    ) -> Result<Fragment, LowerError> {
        let image = initex();
        let mut engine = pe::PortableTexEngine::from_format(&image, ProviderFiles(&resources));
        engine.begin_fragment_capture();
        let program = format!("\\catcode`{{=1 \\catcode`}}=2 {body}\\end");
        assert!(engine.begin_primary_input("input.tex", program.into_bytes()));
        assert!(engine.run_main_control());
        engine.end_fragment_capture();
        let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
        assert!(!transcript.contains('!'), "{transcript}");
        let root = engine.captured_fragment_root().expect("captured root");
        super::lower(&engine, root, FragmentMetadata::default(), max_nodes)
    }

    fn lower(body: &str) -> Fragment {
        lower_with(InMemoryResourceProvider::new(), body, MAX_NODES).expect("lowered fragment")
    }

    fn root_children(fragment: &Fragment) -> Vec<&LayoutNode> {
        fragment
            .children(fragment.root)
            .iter()
            .map(|&id| fragment.node(id).expect("child"))
            .collect()
    }

    fn origin_sp(node: &LayoutNode) -> (i32, i32) {
        (node.origin.x.0, node.origin.y.0)
    }

    const PT: i32 = 65_536;

    #[test]
    fn engine_inputs_resolve_through_the_resource_provider() {
        let resources = InMemoryResourceProvider::new().with_resource(
            "child.tex",
            ResourceKind::TexInput,
            br"\relax",
        );
        let image = initex();
        let mut engine = pe::PortableTexEngine::from_format(&image, ProviderFiles(&resources));

        assert!(engine.begin_primary_input("input.tex", br"\input child.tex \end".to_vec()));
        engine.run_main_control();

        assert_eq!(engine.last_abort_status(), None);
        assert_eq!(engine.resource_request_count(), 1);
        let request = &engine.resource_request_records()[0];
        assert_eq!(request.name, "child.tex");
        assert_eq!(request.kind, pe::ResourceKind::TexInput);
        assert_eq!(request.byte_len, Some(6));
        assert_eq!(request.source, None);
    }

    #[test]
    fn captured_root_lowers_to_a_rooted_fragment_with_rule_extents() {
        let fragment = lower(r"\hbox{\vrule width 1pt height 2pt depth 3pt}");
        fragment.validate().expect("valid fragment");
        let root = fragment.root_node().expect("root");
        assert_eq!(root.origin, Point::ORIGIN);
        assert!(matches!(&root.kind, LayoutNodeKind::Box(b) if b.kind == BoxKind::Horizontal));
        assert_eq!(fragment.surface.width, Length(PT));
        assert_eq!(fragment.surface.baseline, Length(2 * PT));
        assert_eq!(fragment.surface.height, Length(5 * PT));
        let rule = root_children(&fragment)[0];
        assert_eq!(rule.kind, LayoutNodeKind::Rule);
        assert_eq!((rule.height, rule.depth), (Length(2 * PT), Length(3 * PT)));
    }

    #[test]
    fn glue_is_set_with_cumulative_rounding_as_hlist_out() {
        // Positions and glue amounts checked against tex and dvitype, per glue rounding would drift by 1sp.
        let fragment = lower(concat!(
            r"\hbox to 10.5pt{\vrule width 1pt height 1pt\hskip 0pt plus 1pt\vrule width 1pt height 1pt",
            r"\hskip 0pt plus 1pt\vrule width 1pt height 1pt\hskip 0pt plus 1pt\vrule width 1pt height 1pt}",
        ));
        let children = root_children(&fragment);
        let rules = children
            .iter()
            .filter(|node| node.kind == LayoutNodeKind::Rule)
            .map(|node| origin_sp(node))
            .collect::<Vec<_>>();
        assert_eq!(rules, [(0, 0), (207_531, 0), (415_061, 0), (622_592, 0)]);
        let glue = children
            .iter()
            .filter_map(|node| match node.kind {
                LayoutNodeKind::Glue(glue) => Some(glue.amount.0),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(glue, [141_995, 141_994, 141_995]);
    }

    #[test]
    fn rule_leaders_become_a_rule_sized_to_the_glue() {
        let fragment = lower(r"\hbox to 20pt{\leaders\hrule\hfil}");
        let children = root_children(&fragment);
        assert_eq!(children.len(), 1);
        let rule = children[0];
        assert_eq!(rule.kind, LayoutNodeKind::Rule);
        assert_eq!(rule.width, Length(20 * PT));
        assert_eq!((rule.height, rule.depth), (Length(26_214), Length::ZERO));

        let fragment = lower(r"\vbox to 20pt{\hrule width 3pt\leaders\hrule\vfil}");
        let leader = root_children(&fragment)[1];
        assert_eq!(leader.kind, LayoutNodeKind::Rule);
        // The running width comes from the vbox, the rule fills the 19.6pt the glue spans.
        assert_eq!(leader.width, Length(3 * PT));
        assert_eq!(leader.height, Length(20 * PT - 26_214));
        assert_eq!(origin_sp(leader), (0, 0));
    }

    /// Absolute x of each leader box copy in an `\hbox to 25pt` after a 1pt skip.
    fn leader_copies(leaders: &str) -> Vec<i32> {
        let fragment = lower(&format!(
            r"\hbox to 25pt{{\hskip 1pt{leaders}\hbox to 7pt{{\vrule width 1pt height 2pt\hfil}}\hfil}}"
        ));
        let container = root_children(&fragment)[1];
        assert_eq!(container.width, Length(24 * PT));
        fragment
            .children(container.id)
            .iter()
            .map(|&id| container.origin.x.0 + fragment.node(id).expect("copy").origin.x.0)
            .collect()
    }

    #[test]
    fn box_leaders_repeat_as_aligned_centered_and_expanded_leaders() {
        // Expected positions are the rule positions tex writes to the DVI file for the same boxes.
        assert_eq!(leader_copies(r"\leaders"), [7 * PT, 14 * PT]);
        assert_eq!(leader_copies(r"\cleaders"), [163_845, 622_597, 1_081_349]);
        assert_eq!(leader_copies(r"\xleaders"), [114_691, 622_597, 1_130_503]);
    }

    #[test]
    fn vertical_box_leaders_place_copies_down_the_list() {
        let fragment =
            lower(r"\vbox to 20pt{\vskip 2pt\cleaders\vbox to 5pt{\hrule width 3pt\vfil}\vfil}");
        let container = root_children(&fragment)[1];
        assert_eq!(origin_sp(container), (0, -18 * PT));
        assert_eq!(container.depth, Length(18 * PT));
        let baselines = fragment
            .children(container.id)
            .iter()
            .map(|&id| container.origin.y.0 + fragment.node(id).expect("copy").origin.y.0)
            .collect::<Vec<_>>();
        // tex puts the copies' baselines 753659sp, 425979sp and 98299sp above the vbox baseline.
        assert_eq!(baselines, [-753_659, -425_979, -98_299]);
    }

    #[test]
    fn lowering_past_the_node_cap_is_an_error() {
        let body = r"\hbox to 100pt{\xleaders\hbox to 1pt{\vrule width 1pt height 1pt}\hfil}";
        assert_eq!(
            lower_with(InMemoryResourceProvider::new(), body, 50),
            Err(LowerError::NodeLimit { limit: 50 })
        );
        let fragment =
            lower_with(InMemoryResourceProvider::new(), body, 1000).expect("within the cap");
        assert_eq!(fragment.flatten().len(), 100);
    }

    #[test]
    fn tfm_characters_and_ligatures_lower_with_their_metrics() {
        let Some(cmr10) = mathtex_test_fixtures::read(mathtex_test_fixtures::CMR10) else {
            return;
        };
        let resources =
            InMemoryResourceProvider::new().with_resource("cmr10", ResourceKind::Font, cmr10);
        let fragment = lower_with(resources, r"\font\tenrm=cmr10 \hbox{\tenrm Afi}", MAX_NODES)
            .expect("lowered fragment");
        let glyphs = root_children(&fragment)
            .into_iter()
            .filter_map(|node| match &node.kind {
                LayoutNodeKind::GlyphRun(run) => Some((node, run)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(glyphs.len(), 2, "A and the fi ligature");
        let (a, run) = glyphs[0];
        assert_eq!(run.glyphs[0].glyph_id, GlyphId(u32::from(b'A')));
        assert_eq!(run.font.key, None);
        assert_eq!(run.font.spec, "cmr10");
        // cmr10 gives A a height of 6.83331pt and no depth.
        assert_eq!((a.height, a.depth), (Length(447_828), Length::ZERO));
        let (fi, run) = glyphs[1];
        // cmr10 puts the fi ligature at character 12.
        assert_eq!(run.glyphs[0].glyph_id, GlyphId(12));
        assert_eq!(fi.origin.x, a.width);
    }
}
