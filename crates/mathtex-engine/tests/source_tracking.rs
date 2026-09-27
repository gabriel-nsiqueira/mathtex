//! Source tracking of typeset fragments: every visible mark maps back to the caller's input.

mod support;

use mathtex_engine::{HostBoxes, MathMode, NoHostBoxes, Options, FRAGMENT_SOURCE};
use mathtex_ir::{Fragment, LayoutNodeKind, Length, NodeId, SourceRange, SourceRole};
use support::{cm_fonts, cm_format, typesetter, TokenWideBoxes};

/// Typesets inline math over the Computer Modern fixture format, `None` when the fixtures are missing.
fn render_with(tex: &str, boxes: &dyn HostBoxes) -> Option<Fragment> {
    let (fonts, format) = (cm_fonts()?, cm_format()?);
    let mut typesetter = typesetter(fonts, format, Options::default());
    let result = typesetter
        .typeset(tex, MathMode::Inline, boxes)
        .unwrap_or_else(|error| panic!("{tex:?} typesets: {error}"));
    Some(result.fragment)
}

fn render(tex: &str) -> Option<Fragment> {
    render_with(tex, &NoHostBoxes)
}

fn is_visible(kind: &LayoutNodeKind) -> bool {
    matches!(kind, LayoutNodeKind::GlyphRun(_) | LayoutNodeKind::Rule)
}

/// The slice of `tex` a range covers, asserting it names the fragment and stays inside it.
fn slice<'a>(fragment: &Fragment, tex: &'a str, range: SourceRange) -> &'a str {
    let source = fragment
        .source_map
        .source(range.source)
        .expect("source id should resolve");
    assert_eq!(
        source.name, FRAGMENT_SOURCE,
        "span must point at the fragment"
    );
    let (s, e) = (range.span.start as usize, range.span.end as usize);
    assert!(
        s <= e && e <= tex.len(),
        "span [{s},{e}) out of bounds for len {}",
        tex.len()
    );
    &tex[s..e]
}

/// Kind and primary slice of every visible node, each must have a primary span.
fn visible_slices<'a>(fragment: &'a Fragment, tex: &'a str) -> Vec<(&'a LayoutNodeKind, &'a str)> {
    fragment
        .nodes
        .iter()
        .filter(|n| is_visible(&n.kind))
        .map(|n| {
            let range = fragment
                .primary_source_for_node(n.id)
                .unwrap_or_else(|| panic!("visible node {:?} has no primary source", n.id));
            (&n.kind, slice(fragment, tex, range))
        })
        .collect()
}

fn glyphs<'a>(visible: &[(&LayoutNodeKind, &'a str)]) -> Vec<&'a str> {
    visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| *s)
        .collect()
}

fn rules<'a>(visible: &[(&LayoutNodeKind, &'a str)]) -> Vec<&'a str> {
    visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::Rule))
        .map(|(_, s)| *s)
        .collect()
}

/// Glyph slices of `tex`, `None` when TeX Live is missing.
fn glyph_slices(tex: &str) -> Option<Vec<String>> {
    let fragment = render(tex)?;
    Some(
        glyphs(&visible_slices(&fragment, tex))
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

#[test]
fn substring_fidelity_single_math_letter() {
    let Some(g) = glyph_slices("x") else {
        return;
    };
    assert_eq!(g, ["x"], "the x glyph must slice to \"x\"");
}

fn entries_for<'a>(
    fragment: &'a Fragment,
    tex: &'a str,
    node: NodeId,
) -> Vec<(SourceRole, &'a str)> {
    fragment
        .source_origins_for_node(node)
        .map(|origin| {
            let range = SourceRange {
                source: origin.source.id,
                span: origin.span,
            };
            (origin.role, slice(fragment, tex, range))
        })
        .collect()
}

/// (role, slice) entries of the first glyph whose primary span is `glyph`.
fn glyph_entries(tex: &str, glyph: &str) -> Option<Vec<(SourceRole, String)>> {
    let fragment = render(tex)?;
    for node in &fragment.nodes {
        if !matches!(node.kind, LayoutNodeKind::GlyphRun(_)) {
            continue;
        }
        if let Some(range) = fragment.primary_source_for_node(node.id) {
            if slice(&fragment, tex, range) == glyph {
                return Some(
                    entries_for(&fragment, tex, node.id)
                        .into_iter()
                        .map(|(r, s)| (r, s.to_string()))
                        .collect(),
                );
            }
        }
    }
    panic!("no glyph slicing to {glyph:?} in {tex:?}");
}

#[test]
fn mathbin_leaf_primary_exact() {
    let Some(g) = glyph_slices(r"\mathbin{+}") else {
        return;
    };
    assert!(
        g.contains(&"+".to_string()),
        "the + glyph must slice to \"+\", got {g:?}"
    );
}

#[test]
fn mathbin_enclosing_construct_entry() {
    let Some(entries) = glyph_entries(r"\mathbin{+}", "+") else {
        return;
    };
    assert_eq!(
        entries.first(),
        Some(&(SourceRole::Primary, "+".to_string()))
    );
    assert!(
        entries
            .iter()
            .any(|(r, s)| *r == SourceRole::EnclosingConstruct && s == r"\mathbin{+}"),
        "expected EnclosingConstruct \\mathbin{{+}}; got {entries:?}"
    );
}

#[test]
fn fraction_argument_enclosing_construct_entry() {
    for glyph in ["a", "b"] {
        let Some(entries) = glyph_entries(r"\frac{a}{b}", glyph) else {
            return;
        };
        assert_eq!(
            entries.first(),
            Some(&(SourceRole::Primary, glyph.to_string())),
            "primary must stay the leaf {glyph:?}; got {entries:?}"
        );
        assert!(
            entries
                .iter()
                .any(|(r, s)| *r == SourceRole::EnclosingConstruct && s == r"\frac{a}{b}"),
            "{glyph:?}: expected enclosing \\frac{{a}}{{b}}; got {entries:?}"
        );
    }
}

#[test]
fn enclosing_entries_contain_primary_and_nest_outward() {
    for tex in [r"\mathbin{+}", r"\frac{a}{b}", r"x^2", r"\frac{x+y}{z}"] {
        let Some(fragment) = render(tex) else {
            return;
        };
        for node in fragment.nodes.iter().filter(|n| is_visible(&n.kind)) {
            let Some(primary) = fragment.primary_source_for_node(node.id) else {
                continue;
            };
            let (ps, pe) = (primary.span.start, primary.span.end);
            let mut inner = (ps, pe);
            for entry in fragment.source_entries_for_node(node.id) {
                let (es, ee) = (entry.range.span.start, entry.range.span.end);
                assert!(
                    es <= inner.0 && ee >= inner.1,
                    "{tex:?}: enclosing [{es},{ee}) must contain inner [{},{})",
                    inner.0,
                    inner.1
                );
                inner = (es, ee);
            }
        }
    }
}

#[test]
fn scan_loss_char_primitive() {
    let Some(g) = glyph_slices(r"\char98") else {
        return;
    };
    assert!(!g.is_empty(), "no glyph for \\char98");
    for glyph in &g {
        assert!(
            glyph.starts_with("\\char"),
            "glyph span {glyph:?} should cover \\char98"
        );
    }
}

#[test]
fn containment_fraction() {
    let tex = r"\frac{a}{b}";
    let Some(fragment) = render(tex) else {
        return;
    };
    let visible = visible_slices(&fragment, tex);
    let (g, r) = (glyphs(&visible), rules(&visible));
    assert!(g.contains(&"a") && g.contains(&"b"), "leaves; got {g:?}");
    // TeX generates the fraction bar as a rule spanning the whole \frac invocation.
    assert!(
        r.contains(&tex),
        "fraction rule should cover {tex:?}; got {r:?}"
    );
}

const SQRT: &str = "\\def\\sqrt[#1]#2{{#1}\\radical\"270370{#2}}";

#[test]
fn radical_surd_and_vinculum_map_to_whole_construct() {
    let tex = format!("{SQRT}\\sqrt[3]{{x}}");
    let Some(fragment) = render(&tex) else {
        return;
    };
    let visible = visible_slices(&fragment, &tex);
    let (g, r) = (glyphs(&visible), rules(&visible));
    assert!(g.contains(&r"\sqrt[3]{x}"), "surd glyph; got {g:?}");
    assert!(!g.contains(&"[3]"), "a glyph mis-mapped to the degree [3]");
    assert!(r.contains(&r"\sqrt[3]{x}"), "vinculum; got {r:?}");
    assert!(
        g.contains(&"3") && g.contains(&"x"),
        "degree and radicand; got {g:?}"
    );
}

#[test]
fn nested_fraction_in_argument_keeps_own_extent() {
    // \frac replayed from a token list must recover the \frac{q}{2} extent, not the enclosing call.
    let tex = "\\def\\sqrtx#1{\\radical\"270370{#1}}\\sqrtx{\\frac{q}{2}}";
    let Some(fragment) = render(tex) else {
        return;
    };
    let r = rules(&visible_slices(&fragment, tex));
    assert!(r.contains(&r"\frac{q}{2}"), "nested \\frac bar; got {r:?}");
}

#[test]
fn cardano_radical_coverage_and_extent() {
    let tex = format!(
        "{SQRT}x=\\sqrt[3]{{-\\frac{{q}}{{2}}+\\sqrt[2]{{\\frac{{q^2}}{{4}}+\\frac{{p^3}}{{27}}}}}}"
    );
    let Some(fragment) = render(&tex) else {
        return;
    };
    let visible = visible_slices(&fragment, &tex);
    assert!(
        visible.iter().all(|(_, s)| !s.is_empty()),
        "empty span slice"
    );
    let radicals = visible
        .iter()
        .map(|(_, s)| *s)
        .filter(|s| s.contains("\\sqrt["))
        .collect::<Vec<_>>();
    assert!(
        radicals.len() >= 2,
        "both radicals' marks; got {radicals:?}"
    );
    for mark in &radicals {
        assert!(
            mark.starts_with("\\sqrt["),
            "radical mark {mark:?} must be a whole construct"
        );
    }
}

#[test]
fn macro_invocation_synthesized_bar() {
    let tex = r"\def\foo{\frac{1}{2}}\foo";
    let Some(fragment) = render(tex) else {
        return;
    };
    let r = rules(&visible_slices(&fragment, tex));
    assert!(
        r.contains(&r"\foo"),
        "the bar maps to the \\foo invocation; got {r:?}"
    );
}

#[test]
fn multi_line_offsets_index_the_whole_input() {
    let Some(g) = glyph_slices("a\n+b") else {
        return;
    };
    assert!(
        g.contains(&"a".to_string()) && g.contains(&"b".to_string()),
        "{g:?}"
    );
}

#[test]
fn coverage_no_visible_node_is_none() {
    for tex in [
        r"x",
        r"x+y",
        r"\char98",
        r"\frac{a}{b}",
        r"x^2",
        r"\def\foo{\frac{1}{2}}\foo",
    ] {
        let Some(fragment) = render(tex) else {
            return;
        };
        let visible = visible_slices(&fragment, tex);
        assert!(!visible.is_empty(), "no visible nodes for {tex:?}");
        assert!(
            visible.iter().all(|(_, s)| !s.is_empty()),
            "{tex:?}: empty slice"
        );
    }
}

// Regression: clean_box stamped a noad script field with the base span, so x^2 mapped 2 to "x".
#[test]
fn scripts_map_to_their_own_source() {
    for (tex, leaves) in [
        ("x^2", &["x", "2"][..]),
        ("x_2", &["x", "2"][..]),
        ("x^2_3", &["x", "2", "3"][..]),
        ("x^{y}", &["x", "y"][..]),
        ("x_{y}", &["x", "y"][..]),
        // Built by make_op and clean_box, limits must carry their own span, not the operator's.
        (r"\mathop{S}\limits_{a}^{b}", &["S", "a", "b"][..]),
    ] {
        let Some(g) = glyph_slices(tex) else {
            return;
        };
        for leaf in leaves {
            assert!(g.contains(&leaf.to_string()), "{tex}: {leaf:?} in {g:?}");
        }
    }
}

#[test]
fn leading_skipped_blank_excluded_from_span() {
    // After a control word, the space skip_blanks consumed must not widen the next token's span.
    let Some(g) = glyph_slices(r"\displaystyle x") else {
        return;
    };
    assert_eq!(g, ["x"]);
}

/// Primary slices of every box as wide as the 5pt host box.
fn host_box_slices(tex: &str) -> Option<Vec<String>> {
    let boxes = TokenWideBoxes::default();
    let fragment = render_with(tex, &boxes)?;
    let width = Length::from_scaled_points(5 * 65_536);
    Some(
        fragment
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, LayoutNodeKind::Box(_)) && node.width == width)
            .map(|node| {
                let range = fragment
                    .primary_source_for_node(node.id)
                    .unwrap_or_else(|| panic!("host box node {:?} has no primary source", node.id));
                slice(&fragment, tex, range).to_string()
            })
            .collect(),
    )
}

#[test]
fn host_box_atom_spans_its_own_call_site() {
    // The script case copies the placeholder into the field word, the box must keep the \hostbox bytes.
    for tex in [r"a\hostbox{5}b", r"x^{\hostbox{5}}"] {
        let Some(slices) = host_box_slices(tex) else {
            return;
        };
        assert!(!slices.is_empty(), "{tex}: host box atom must be present");
        for text in &slices {
            assert!(
                text.starts_with("\\hostbox"),
                "{tex}: host box maps to {text:?}"
            );
        }
    }
}
