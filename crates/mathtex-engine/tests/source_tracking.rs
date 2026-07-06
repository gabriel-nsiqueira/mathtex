//! Source tracking tests for provenance-backed visible mark spans.

use mathtex_engine::generated::generated_node_to_fragment;
use mathtex_engine::{
    portable_engine::EngineProfile, GeneratedFormatCache, GeneratedResourceProvider,
    InMemoryResourceProvider, ResourceKind,
};
use mathtex_ir::{Fragment, FragmentMetadata, LayoutNodeKind, NodeId, SourceRange, SourceRole};

const CMR10: &[u8] = include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm");
const CMMI10: &[u8] =
    include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmmi10.tfm");
const CMSY10: &[u8] =
    include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmsy10.tfm");
const CMEX10: &[u8] =
    include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmex10.tfm");

/// Plain TeX preamble prepended to every test program.
const SETUP: &str = concat!(
    r"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 ",
    r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 ",
    r"\textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm ",
    r"\textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni ",
    r"\textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy ",
    r"\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex ",
    r"\def\frac#1#2{{#1\over#2}}",
);

fn fonts() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("cmr10", CMR10),
        ("cmmi10", CMMI10),
        ("cmsy10", CMSY10),
        ("cmex10", CMEX10),
    ]
}

/// Render a complete TeX program, optionally disabling source tracking.
fn render_program(program: &str, tracking: bool) -> Fragment {
    let mut resources = InMemoryResourceProvider::new();
    for (name, bytes) in fonts() {
        resources = resources.with_resource(name, ResourceKind::Font, bytes.to_vec());
    }
    let format = GeneratedFormatCache::initialized(EngineProfile::tex());
    let mut engine = format.instantiate(EngineProfile::tex(), GeneratedResourceProvider::new(resources));

    engine.set_source_tracking(tracking);
    engine.begin_fragment_capture();
    assert!(
        engine.begin_primary_input("input", program.as_bytes().to_vec()),
        "engine refused primary input"
    );
    assert!(engine.run_main_control(), "run_main_control failed");
    engine.end_fragment_capture();

    let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    assert!(
        !transcript.contains('!'),
        "engine reported a TeX error:\n{transcript}"
    );

    let root = engine
        .captured_fragment_root()
        .expect("engine captured no fragment root");
    generated_node_to_fragment(
        &engine,
        root,
        FragmentMetadata {
            engine_profile: "tex".into(),
            format_id: "source-tracking".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("captured root failed to convert to IR")
}

fn math_program(body: &str) -> String {
    format!("{SETUP}\\hbox{{${body}$}}\\end")
}

fn is_visible(kind: &LayoutNodeKind) -> bool {
    matches!(
        kind,
        LayoutNodeKind::GlyphRun(_) | LayoutNodeKind::Rule(_) | LayoutNodeKind::Drawing(_)
    )
}

/// Slice fed by a source range, asserts the source name is "input".
fn slice<'a>(fragment: &Fragment, fed: &'a str, range: SourceRange) -> &'a str {
    let source = fragment
        .source_map
        .source(range.source)
        .expect("source id should resolve");
    assert_eq!(source.name, "input", "span must point at the user fragment");
    let (s, e) = (range.span.start as usize, range.span.end as usize);
    assert!(
        s <= e && e <= fed.len(),
        "span [{s},{e}) out of bounds for fed len {}",
        fed.len()
    );
    &fed[s..e]
}

fn visible_slices<'a>(fragment: &'a Fragment, fed: &'a str) -> Vec<(&'a LayoutNodeKind, &'a str)> {
    fragment
        .nodes
        .iter()
        .filter(|n| is_visible(&n.kind))
        .map(|n| {
            let range = fragment
                .primary_source_for_node(n.id)
                .unwrap_or_else(|| panic!("visible node {:?} has no primary source", n.id));
            (&n.kind, slice(fragment, fed, range))
        })
        .collect()
}

#[test]
fn substring_fidelity_single_math_letter() {
    let fed = math_program("x");
    let fragment = render_program(&fed, true);

    let visible = visible_slices(&fragment, &fed);
    assert!(!visible.is_empty(), "no visible nodes for $x$");
    let glyphs: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| *s)
        .collect();
    assert_eq!(glyphs, vec!["x"], "the x glyph must slice to \"x\"");
}

fn entries_for<'a>(
    fragment: &'a Fragment,
    fed: &'a str,
    node: NodeId,
) -> Vec<(SourceRole, &'a str)> {
    fragment
        .source_entries_for_node(node)
        .map(|entry| (entry.role, slice(fragment, fed, entry.range)))
        .collect()
}

/// Returns (role, slice) entries for the first glyph whose primary span matches glyph.
fn glyph_entries(body: &str, glyph: &str) -> Vec<(SourceRole, String)> {
    let fed = math_program(body);
    let fragment = render_program(&fed, true);
    for node in fragment.nodes.iter().filter(|n| is_visible(&n.kind)) {
        if !matches!(node.kind, LayoutNodeKind::GlyphRun(_)) {
            continue;
        }
        if let Some(range) = fragment.primary_source_for_node(node.id) {
            if slice(&fragment, &fed, range) == glyph {
                return entries_for(&fragment, &fed, node.id)
                    .into_iter()
                    .map(|(r, s)| (r, s.to_string()))
                    .collect();
            }
        }
    }
    panic!("no glyph slicing to {glyph:?} in {body:?}");
}

#[test]
fn mathbin_leaf_primary_exact() {
    let fed = math_program(r"\mathbin{+}");
    let fragment = render_program(&fed, true);
    let glyphs: Vec<&str> = visible_slices(&fragment, &fed)
        .into_iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| s)
        .collect();
    assert!(
        glyphs.contains(&"+"),
        "the + glyph of \\mathbin{{+}} must slice to exactly \"+\" (its own leaf), got {glyphs:?}"
    );
}

#[test]
fn mathbin_enclosing_construct_entry() {
    let entries = glyph_entries(r"\mathbin{+}", "+");
    assert_eq!(entries.first(), Some(&(SourceRole::Primary, "+".to_string())));
    assert!(
        entries
            .iter()
            .any(|(r, s)| *r == SourceRole::EnclosingConstruct && s == r"\mathbin{+}"),
        "expected EnclosingConstruct \\mathbin{{+}}; got {entries:?}"
    );
}

#[test]
fn fraction_argument_enclosing_construct_entry() {
    for (glyph, _which) in [("a", "num"), ("b", "den")] {
        let entries = glyph_entries(r"\frac{a}{b}", glyph);
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
    for body in [r"\mathbin{+}", r"\frac{a}{b}", r"x^2", r"\frac{x+y}{z}"] {
        let fed = math_program(body);
        let fragment = render_program(&fed, true);
        for node in fragment.nodes.iter().filter(|n| is_visible(&n.kind)) {
            let Some(primary) = fragment.primary_source_for_node(node.id) else {
                continue;
            };
            let (ps, pe) = (primary.span.start, primary.span.end);
            let mut prev: Option<(u32, u32)> = Some((ps, pe));
            for entry in fragment.source_entries_for_node(node.id) {
                if entry.role != SourceRole::EnclosingConstruct {
                    continue;
                }
                let (es, ee) = (entry.range.span.start, entry.range.span.end);
                assert!(
                    es <= ps && ee >= pe,
                    "{body:?}: enclosing [{es},{ee}) must contain primary [{ps},{pe})"
                );
                if let Some((vs, ve)) = prev {
                    assert!(
                        es <= vs && ee >= ve,
                        "{body:?}: enclosing [{es},{ee}) must contain inner [{vs},{ve})"
                    );
                }
                prev = Some((es, ee));
            }
        }
    }
}

#[test]
fn scan_loss_char_primitive() {
    let fed = math_program(r"\char98");
    let fragment = render_program(&fed, true);

    let visible = visible_slices(&fragment, &fed);
    let glyphs: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| *s)
        .collect();
    assert!(!glyphs.is_empty(), "no glyph for \\char98");
    for g in &glyphs {
        assert_ne!(*g, "98", "glyph mapped to the digits (scan loss)");
        assert!(
            g.starts_with("\\char"),
            "glyph span {g:?} should cover the \\char construct"
        );
    }
}

#[test]
fn containment_fraction() {
    let body = r"\frac{a}{b}";
    let fed = math_program(body);
    let fragment = render_program(&fed, true);

    let visible = visible_slices(&fragment, &fed);
    let glyphs: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| *s)
        .collect();
    let rules: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::Rule(_)))
        .map(|(_, s)| *s)
        .collect();

    assert!(glyphs.contains(&"a"), "numerator glyph should slice to a; got {glyphs:?}");
    assert!(glyphs.contains(&"b"), "denominator glyph should slice to b; got {glyphs:?}");
    // TeX generates the fraction bar as a rule node spanning the whole \frac invocation.
    assert!(
        rules.iter().any(|r| *r == body),
        "fraction rule should cover {body:?}; got rules {rules:?}"
    );
}

const SQRT: &str = "\\def\\sqrt[#1]#2{{#1}\\radical\"270370{#2}}";

#[test]
fn radical_surd_and_vinculum_map_to_whole_construct() {
    let body = format!("{SQRT}\\sqrt[3]{{x}}");
    let fed = math_program(&body);
    let fragment = render_program(&fed, true);
    let visible = visible_slices(&fragment, &fed);

    let glyphs: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| *s)
        .collect();
    let rules: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::Rule(_)))
        .map(|(_, s)| *s)
        .collect();

    assert!(
        glyphs.contains(&r"\sqrt[3]{x}"),
        "surd glyph must slice to the whole \\sqrt[3]{{x}}; got glyphs {glyphs:?}"
    );
    for g in &glyphs {
        assert_ne!(*g, "[3]", "a glyph mis-mapped to the degree [3]");
    }
    // The vinculum is a rule node, it must span the whole radical construct.
    assert!(
        rules.iter().any(|r| *r == r"\sqrt[3]{x}"),
        "vinculum rule must cover \\sqrt[3]{{x}}; got rules {rules:?}"
    );
    assert!(glyphs.contains(&"3"), "degree must slice to 3; got {glyphs:?}");
    assert!(glyphs.contains(&"x"), "radicand must slice to x; got {glyphs:?}");
}

#[test]
fn nested_fraction_in_argument_keeps_own_extent() {
    // \frac replayed from a token list must recover the \frac{q}{2} extent, not the enclosing call site.
    let body = "\\def\\sqrtx#1{\\radical\"270370{#1}}\\sqrtx{\\frac{q}{2}}";
    let fed = math_program(body);
    let fragment = render_program(&fed, true);
    let rules: Vec<&str> = visible_slices(&fragment, &fed)
        .into_iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::Rule(_)))
        .map(|(_, s)| s)
        .collect();
    assert!(
        rules.iter().any(|r| *r == r"\frac{q}{2}"),
        "nested \\frac bar must slice to \\frac{{q}}{{2}}; got rules {rules:?}"
    );
}

#[test]
fn cardano_radical_coverage_and_extent() {
    let body = format!(
        "{SQRT}x=\\sqrt[3]{{-\\frac{{q}}{{2}}+\\sqrt[2]{{\\frac{{q^2}}{{4}}+\\frac{{p^3}}{{27}}}}}}"
    );
    let fed = math_program(&body);
    let fragment = render_program(&fed, true);

    let visible: Vec<_> = fragment
        .nodes
        .iter()
        .filter(|n| is_visible(&n.kind))
        .collect();
    assert!(!visible.is_empty(), "no visible nodes for Cardano");
    for node in &visible {
        let range = fragment
            .primary_source_for_node(node.id)
            .unwrap_or_else(|| panic!("Cardano: visible node {:?} has None source", node.id));
        let s = slice(&fragment, &fed, range);
        assert!(!s.is_empty(), "Cardano: empty span slice");
    }
    let radical_marks: Vec<&str> = visible_slices(&fragment, &fed)
        .into_iter()
        .map(|(_, s)| s)
        .filter(|s| s.contains("\\sqrt["))
        .collect();
    assert!(
        radical_marks.len() >= 2,
        "expected both radicals' marks to map to \\sqrt[..] extents; got {radical_marks:?}"
    );
    for m in &radical_marks {
        assert!(
            m.starts_with("\\sqrt["),
            "radical mark {m:?} must be a whole \\sqrt[..] construct, not a degree"
        );
    }
}

#[test]
fn macro_invocation_synthesized_bar() {
    let body = r"\def\foo{\frac{1}{2}}\foo";
    let fed = math_program(body);
    let fragment = render_program(&fed, true);

    let visible = visible_slices(&fragment, &fed);
    let rules: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::Rule(_)))
        .map(|(_, s)| *s)
        .collect();
    assert!(
        rules.iter().any(|r| *r == r"\foo"),
        "synthesized fraction bar should map to the \\foo invocation; got {rules:?}"
    );
}

#[test]
fn multi_line_absolute_offsets() {
    let body = "a\n+b";
    let fed = math_program(body);
    assert!(fed.contains('\n'), "fed must be multi-line");
    let fragment = render_program(&fed, true);

    let visible = visible_slices(&fragment, &fed);
    let glyphs: Vec<&str> = visible
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| *s)
        .collect();
    // Spans are absolute byte offsets into the fed buffer, not offsets relative to a line.
    assert!(glyphs.contains(&"a"), "line-1 glyph a missing; got {glyphs:?}");
    assert!(glyphs.contains(&"b"), "line-2 glyph b missing; got {glyphs:?}");
}

#[test]
fn coverage_no_visible_node_is_none() {
    for body in [
        r"x",
        r"x+y",
        r"\char98",
        r"\frac{a}{b}",
        r"x^2",
        r"\def\foo{\frac{1}{2}}\foo",
    ] {
        let fed = math_program(body);
        let fragment = render_program(&fed, true);
        let visible: Vec<_> = fragment
            .nodes
            .iter()
            .filter(|n| is_visible(&n.kind))
            .collect();
        assert!(!visible.is_empty(), "no visible nodes for {body:?}");
        for node in visible {
            let range = fragment
                .primary_source_for_node(node.id)
                .unwrap_or_else(|| panic!("{body:?}: visible node {:?} has None source", node.id));
            let s = slice(&fragment, &fed, range);
            assert!(!s.is_empty(), "{body:?}: empty span slice");
            assert!(
                fed.contains(s),
                "{body:?}: span slice {s:?} not a substring of fed"
            );
        }
    }
}

#[test]
fn tracking_does_not_change_layout() {
    let fed = math_program(r"\frac{a}{b}");

    let kinds = |f: &Fragment| -> Vec<std::mem::Discriminant<LayoutNodeKind>> {
        f.nodes
            .iter()
            .map(|n| std::mem::discriminant(&n.kind))
            .collect()
    };
    let bounds = |f: &Fragment| -> Vec<(i32, i32)> {
        f.nodes
            .iter()
            .map(|n| (n.bounds.size.width.0, n.bounds.size.height.0))
            .collect()
    };

    let on = render_program(&fed, true);
    let off = render_program(&fed, false);

    assert_eq!(kinds(&on), kinds(&off), "node kinds changed with tracking");
    assert_eq!(bounds(&on), bounds(&off), "node bounds changed with tracking");

    assert!(
        off.nodes.iter().all(|n| n.primary_source.is_none()),
        "tracking OFF must leave primary_source None"
    );
    assert!(
        off.source_map.entries.is_empty(),
        "tracking OFF must not populate the source map"
    );
    assert!(
        on.nodes.iter().any(|n| n.primary_source.is_some()),
        "tracking ON must populate spans"
    );
}

// Regression: clean_box stamped a noad script field with the base span (x^2 caused 2 to map to "x").
fn glyph_slices(fed: &str, body: &str) -> Vec<String> {
    let fragment = render_program(fed, true);
    visible_slices(&fragment, fed)
        .into_iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|_| !body.is_empty())
        .collect()
}

#[test]
fn superscript_glyph_maps_to_own_source() {
    let fed = math_program("x^2");
    let g = glyph_slices(&fed, "x^2");
    assert!(g.contains(&"x".to_string()), "base x must map to \"x\"; got {g:?}");
    assert!(
        g.contains(&"2".to_string()),
        "superscript 2 must map to \"2\" (its own source), not the base; got {g:?}"
    );
}

#[test]
fn subscript_single_glyph_maps_to_own_source() {
    let fed = math_program("x_2");
    let g = glyph_slices(&fed, "x_2");
    assert!(g.contains(&"x".to_string()), "base x; got {g:?}");
    assert!(g.contains(&"2".to_string()), "subscript 2 must map to \"2\"; got {g:?}");
}

#[test]
fn both_scripts_map_to_own_source() {
    let fed = math_program("x^2_3");
    let g = glyph_slices(&fed, "x^2_3");
    for c in ["x", "2", "3"] {
        assert!(
            g.contains(&c.to_string()),
            "glyph {c:?} must map to its own source; got {g:?}"
        );
    }
}

#[test]
fn braced_scripts_map_to_own_source() {
    for (body, sup) in [("x^{y}", "y"), ("x_{y}", "y")] {
        let fed = math_program(body);
        let g = glyph_slices(&fed, body);
        assert!(g.contains(&"x".to_string()), "{body}: base x; got {g:?}");
        assert!(
            g.contains(&sup.to_string()),
            "{body}: braced single-char script must map to {sup:?}, not the base; got {g:?}"
        );
    }
}

#[test]
fn big_operator_limits_map_to_own_source() {
    // Built via make_op + clean_box, limit glyphs must carry their own span, not the operator span.
    let fed = math_program(r"\mathop{S}\limits_{a}^{b}");
    let g = glyph_slices(&fed, "limits");
    for c in ["S", "a", "b"] {
        assert!(
            g.contains(&c.to_string()),
            "limit/base glyph {c:?} must map to its own source; got {g:?}"
        );
    }
}

#[test]
fn leading_skipped_blank_excluded_from_span() {
    // After a control word, consumed skip_blanks space must not widen the token span.
    let fed = math_program(r"\displaystyle x");
    let fragment = render_program(&fed, true);
    let glyphs: Vec<String> = visible_slices(&fragment, &fed)
        .into_iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| s.to_string())
        .collect();
    assert_eq!(
        glyphs,
        vec!["x".to_string()],
        "x must map to exactly \"x\" with no absorbed leading blank; got {glyphs:?}"
    );
}
