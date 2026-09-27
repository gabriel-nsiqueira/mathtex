//! Real LaTeX with amsmath and unicode-math from a local TeX Live tree, through the public API.

mod support;

use mathtex_engine::{MathMode, Options, TypesetError};
use mathtex_ir::LayoutNodeKind;
use support::{glyph_ids, latex_format, texlive, typeset, typesetter, LoadedOutlines};

const CAUCHY_SCHWARZ: &str = r"\left(\sum_{k=1}^n a_k b_k\right)^2\leq\left(\sum_{k=1}^n a_k^2\right)\left(\sum_{k=1}^n b_k^2\right)";

#[test]
fn unicode_math_renders_math_italic_glyphs_to_svg() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let result = typeset(&mut typesetter, "x+y", MathMode::Inline);
    let glyphs = glyph_ids(&result.fragment);
    assert!(
        glyphs.iter().any(|&glyph| glyph > 1000),
        "math italic glyphs sit past 1000 in latinmodern-math, got {glyphs:?}"
    );
    let svg = mathtex_svg::render(
        &result.fragment,
        &LoadedOutlines(typesetter.fonts()),
        &mathtex_svg::SvgOptions::default(),
    );
    assert!(svg.matches("<path").count() >= 2, "{svg}");
}

#[test]
fn cauchy_schwarz_lays_out_every_glyph() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default().with_cache_capacity(0));
    let first = typeset(&mut typesetter, CAUCHY_SCHWARZ, MathMode::Display);
    assert_eq!(glyph_ids(&first.fragment).len(), 33);
    // The TeX state is fresh for every fragment, so a second run lays out the same.
    let second = typeset(&mut typesetter, CAUCHY_SCHWARZ, MathMode::Display);
    assert_eq!(first, second);
}

#[test]
fn a_tex_error_reports_its_line_and_span_in_the_caller_input() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let tex = "x+1\n+\\nosuchmacro+y";
    let error = typesetter
        .typeset(tex, MathMode::Inline, &mathtex_engine::NoHostBoxes)
        .expect_err("an undefined control sequence fails");
    let TypesetError::Tex {
        message,
        line,
        span,
    } = error
    else {
        panic!("expected a TeX error, got {error:?}");
    };
    assert!(message.contains("Undefined control sequence"), "{message}");
    assert_eq!(line, Some(2));
    let span = span.expect("the offending token's span");
    assert_eq!(
        &tex[span.start as usize..span.end as usize],
        "\\nosuchmacro"
    );
}

#[test]
fn spans_index_the_caller_input() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let tex = r"\frac{a}{b}+c";
    let result = typeset(&mut typesetter, tex, MathMode::Inline);
    let slices = result
        .fragment
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)))
        .filter_map(|node| support::primary_slice(&result.fragment, tex, node.id))
        .collect::<Vec<_>>();
    for leaf in ["a", "b", "+", "c"] {
        assert!(slices.contains(&leaf), "{leaf:?} in {slices:?}");
    }
}

/// Glyph slices and rule slices of a typeset fragment, every visible node must carry a span.
fn visible_slices(fragment: &mathtex_ir::Fragment, tex: &str) -> (Vec<String>, Vec<String>) {
    let (mut glyphs, mut rules) = (Vec::new(), Vec::new());
    for node in &fragment.nodes {
        let visible = match node.kind {
            LayoutNodeKind::GlyphRun(_) => &mut glyphs,
            // Structural rules have a positive width and total height.
            LayoutNodeKind::Rule if node.width.0 > 0 && (node.height + node.depth).0 > 0 => {
                &mut rules
            }
            _ => continue,
        };
        let slice = support::primary_slice(fragment, tex, node.id)
            .unwrap_or_else(|| panic!("{tex:?}: visible node {:?} has no span", node.id));
        visible.push(slice.to_string());
    }
    (glyphs, rules)
}

/// Visible slices of `tex` typeset inline in the LaTeX format, `None` without TeX Live.
fn latex_slices(tex: &str) -> Option<(Vec<String>, Vec<String>)> {
    let (texmf, format) = (texlive()?, latex_format()?);
    let mut typesetter = typesetter(texmf, format, Options::default());
    let fragment = typeset(&mut typesetter, tex, MathMode::Inline).fragment;
    Some(visible_slices(&fragment, tex))
}

#[test]
fn text_and_math_alphabets_use_their_own_fonts() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let fragment = typeset(
        &mut typesetter,
        r"X\mathit{X}\symit{X}\text{X}",
        MathMode::Inline,
    )
    .fragment;
    let keys = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            LayoutNodeKind::GlyphRun(run) => run.font.key,
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        keys.len() >= 2,
        "latinmodern-math and Latin Modern Roman, got {keys:?}"
    );
    for key in keys {
        assert!(
            typesetter.fonts().font(key).is_some(),
            "the loader knows {key:?}"
        );
    }
}

#[test]
fn scripts_use_the_script_style_alternates() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let glyphs = glyph_ids(&typeset(&mut typesetter, "x^{x}", MathMode::Inline).fragment);
    // latinmodern-math's italic x is 1319, its ssty=1 script alternate 1427.
    assert!(
        glyphs.contains(&1319) && glyphs.contains(&1427),
        "{glyphs:?}"
    );
}

#[test]
fn glue_in_a_set_box_stretches() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let fragment = typeset(&mut typesetter, r"\hbox to 120pt{$a+b$}", MathMode::Inline).fragment;
    let last_glyph = fragment
        .flatten()
        .iter()
        .filter_map(|placed| match placed {
            mathtex_ir::Placed::Glyph { x, .. } => Some(x.to_pt()),
            _ => None,
        })
        .fold(0.0_f64, f64::max);
    assert!(
        last_glyph > 100.0,
        "b sits at {last_glyph}pt, about 115pt when the glue stretches"
    );
}

#[test]
fn a_format_loaded_from_static_and_owned_bytes_lays_out_the_same() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let fonts = || mathtex_engine::ResourceFontLoader::new(texmf);
    let owned = mathtex_engine::Format::from_owned(format.to_vec()).expect("owned bytes load");
    let fixed = mathtex_engine::Format::from_static(format).expect("static bytes load");
    let mut first =
        mathtex_engine::Typesetter::new(owned, fonts(), Options::default()).expect("fonts");
    let mut second =
        mathtex_engine::Typesetter::new(fixed, fonts(), Options::default()).expect("fonts");
    let a = typeset(&mut first, CAUCHY_SCHWARZ, MathMode::Display);
    let b = typeset(&mut second, CAUCHY_SCHWARZ, MathMode::Display);
    assert_eq!(a.fragment.flatten().len(), b.fragment.flatten().len());
    assert_eq!(a.fragment.surface, b.fragment.surface);
}

#[test]
fn radical_degrees_map_to_the_typed_characters() {
    let Some((glyphs, _)) = latex_slices(r"\sqrt[Z]{3}") else {
        return;
    };
    assert!(glyphs.contains(&"Z".to_string()), "{glyphs:?}");
    let (texmf, format) = (
        texlive().expect("present"),
        latex_format().expect("present"),
    );
    let mut typesetter = typesetter(texmf, format, Options::default());
    let tex = r"\sqrt[\phantom{x}]{x}";
    let fragment = typeset(&mut typesetter, tex, MathMode::Inline).fragment;
    let slot = r"\phantom{x}";
    let boxes = fragment
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, LayoutNodeKind::Box(_)))
        .filter_map(|node| support::primary_slice(&fragment, tex, node.id))
        .collect::<Vec<_>>();
    assert!(
        boxes.contains(&slot),
        "a box maps to the index slot, got {boxes:?}"
    );
    assert!(
        !boxes
            .iter()
            .any(|s| s.starts_with(slot) && s.len() > slot.len()),
        "a box overshoots the index slot into the radicand: {boxes:?}"
    );
}

#[test]
fn radicals_fractions_and_accents_map_marks_to_whole_constructs() {
    for (tex, construct, leaves) in [
        (r"\sqrt[3]{x}", r"\sqrt[3]{x}", &["x", "3"][..]),
        (r"\sqrt{y}", r"\sqrt{y}", &["y"][..]),
        (r"\frac{a}{b}", r"\frac{a}{b}", &["a", "b"][..]),
        (r"\overline{x}", r"\overline{x}", &["x"][..]),
        (r"\hat{x}", r"\hat{x}", &["x"][..]),
        (r"\widehat{xy}", r"\widehat{xy}", &["x", "y"][..]),
        (r"\dot q", r"\dot q", &["q"][..]),
        (
            r"\left(x+y\right)",
            r"\left(x+y\right)",
            &["x", "+", "y"][..],
        ),
        (r"{n \choose k}", r"{n \choose k}", &["n", "k"][..]),
    ] {
        let Some((glyphs, rules)) = latex_slices(tex) else {
            return;
        };
        assert!(
            glyphs.iter().chain(&rules).any(|s| s == construct),
            "{tex}: a mark maps to {construct:?}, glyphs {glyphs:?} rules {rules:?}"
        );
        for leaf in leaves {
            assert!(
                glyphs.iter().any(|g| g == leaf),
                "{tex}: leaf {leaf:?} in {glyphs:?}"
            );
        }
        for mark in glyphs.iter().chain(&rules) {
            assert!(
                !mark.starts_with('{') || mark == construct,
                "{tex}: {mark:?} floored to a brace"
            );
        }
    }
}

#[test]
fn nested_radicals_and_fractions_keep_their_own_extents() {
    let tex = r"x=\sqrt[3]{-\frac{q}{2}+\sqrt{\frac{q^2}{4}+\frac{p^3}{27}}}";
    let Some((glyphs, rules)) = latex_slices(tex) else {
        return;
    };
    let all = glyphs.iter().chain(&rules).collect::<Vec<_>>();
    let inner = r"\sqrt{\frac{q^2}{4}+\frac{p^3}{27}}";
    let outer = r"\sqrt[3]{-\frac{q}{2}+\sqrt{\frac{q^2}{4}+\frac{p^3}{27}}}";
    assert!(all.iter().any(|s| *s == inner), "{all:?}");
    assert!(all.iter().any(|s| *s == outer), "{all:?}");
    assert!(rules.iter().any(|r| r == r"\frac{q}{2}"), "{rules:?}");
    for mark in &all {
        if mark.starts_with(r"\sqrt") {
            assert!(
                mark.ends_with('}'),
                "radical mark {mark:?} is a whole construct"
            );
        }
    }
    assert_eq!(
        glyphs.iter().filter(|g| *g == outer).count(),
        1,
        "one surd maps to the outer radical"
    );
}

#[test]
fn the_sandbox_allows_nested_math_in_text_and_rejects_breakouts() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default().with_cache_capacity(0));
    for tex in [
        r"a+\hbox{$x$}+b",
        r"\hbox{$x$}+\hbox{$y$}",
        r"\hbox{$x$ $y$}",
        r"\hbox{$\hbox{$x$}$}",
    ] {
        if let Err(error) = typesetter.typeset(tex, MathMode::Display, &mathtex_engine::NoHostBoxes)
        {
            panic!("{tex}: {error}");
        }
    }
    for tex in [r"a$b$", r"$x$+$y$", r"a$ \relax $b", r"\input{article.cls}"] {
        match typesetter.typeset(tex, MathMode::Display, &mathtex_engine::NoHostBoxes) {
            Err(TypesetError::Sandbox { .. }) => {}
            other => panic!("{tex}: expected a sandbox rejection, got {other:?}"),
        }
    }
}

#[test]
fn array_rows_do_not_leak_the_next_row_span() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let tex = r"\begin{array}{cc} &  \\ x & \end{array}";
    let fragment = typeset(&mut typesetter, tex, MathMode::Inline).fragment;
    let empty_boxes = fragment
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, LayoutNodeKind::Box(_)) && node.children().is_empty())
        .filter_map(|node| support::primary_slice(&fragment, tex, node.id))
        .collect::<Vec<_>>();
    assert!(
        !empty_boxes.contains(&"x"),
        "an empty first row cell took x's span: {empty_boxes:?}"
    );
    let tex = r"\begin{array}{cc} &  \\ \phantom{x} & \end{array}";
    let fragment = typeset(&mut typesetter, tex, MathMode::Inline).fragment;
    let slices = fragment
        .nodes
        .iter()
        .filter_map(|node| support::primary_slice(&fragment, tex, node.id))
        .collect::<Vec<_>>();
    assert!(
        !slices.contains(&r"\phantom"),
        "a span lost its argument: {slices:?}"
    );
}

/// INITEX with latinmodern-math as every math family and Latin Modern Roman as `\lmr`, baked once.
fn native_format() -> Option<&'static [u8]> {
    static FORMAT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let texmf = texlive()?;
    Some(FORMAT.get_or_init(|| {
        support::bake(
            texmf,
            None,
            concat!(
                r"\catcode`\{=1 \catcode`\}=2 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
                "\\font\\lm=\"[latinmodern-math.otf]:script=math\" at 10pt ",
                "\\font\\lmr=\"[lmroman10-regular.otf]\" at 10pt ",
                r"\textfont0=\lm \scriptfont0=\lm \scriptscriptfont0=\lm ",
                r"\textfont1=\lm \scriptfont1=\lm \scriptscriptfont1=\lm ",
                r"\textfont2=\lm \scriptfont2=\lm \scriptscriptfont2=\lm ",
                r"\textfont3=\lm \scriptfont3=\lm \scriptscriptfont3=\lm ",
            ),
        )
    }))
}

/// The slice of `tex` each glyph of the first multi glyph or ligature run covers.
fn cluster_slices(tex: &str) -> Option<Vec<String>> {
    let (texmf, format) = (texlive()?, native_format()?);
    let mut typesetter = typesetter(texmf, format, Options::default());
    let fragment = typeset(&mut typesetter, tex, MathMode::Inline).fragment;
    let node = fragment
        .nodes
        .iter()
        .find(|node| matches!(&node.kind, LayoutNodeKind::GlyphRun(run) if run.font.spec.contains("lmroman") || run.glyphs.len() > 1))
        .expect("a native text run");
    let LayoutNodeKind::GlyphRun(run) = &node.kind else {
        unreachable!()
    };
    Some(
        (0..run.glyphs.len())
            .map(|glyph| {
                let range = fragment
                    .glyph_source_range(node.id, glyph)
                    .unwrap_or_else(|| panic!("{tex:?}: glyph {glyph} has no span"));
                tex[range.span.start as usize..range.span.end as usize].to_string()
            })
            .collect(),
    )
}

#[test]
fn native_glyphs_map_to_their_characters_and_ligatures_to_the_union() {
    let Some(abc) = cluster_slices(r"\hbox{\lmr abc}") else {
        return;
    };
    assert_eq!(abc, ["a", "b", "c"]);
    assert_eq!(
        cluster_slices(r"\hbox{\lmr fix}").expect("present"),
        ["fi", "x"]
    );
    assert_eq!(
        cluster_slices(r"\hbox{\lmr ffi}").expect("present"),
        ["ffi"]
    );
}

#[test]
fn umathcode_and_umathcodenum_set_math_codes() {
    let (Some(texmf), Some(format)) = (texlive(), native_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let tex = "\\Umathcode`x=7 1 \"1D465 \\Umathcodenum`y=\"1E1D466 xy";
    let glyphs = glyph_ids(&typeset(&mut typesetter, tex, MathMode::Inline).fragment);
    // The math italic x and y of latinmodern-math, the upright letters would be 89 and 90.
    assert_eq!(glyphs, [1319, 1320]);
}

#[test]
fn the_baked_format_defines_hostbox() {
    let (Some(texmf), Some(format)) = (texlive(), latex_format()) else {
        return;
    };
    let mut typesetter = typesetter(texmf, format, Options::default());
    let boxes = support::TokenWideBoxes::default();
    let with_box = typesetter
        .typeset(r"a\hostbox{3}b", MathMode::Inline, &boxes)
        .expect("the host box typesets");
    let plain = typeset(&mut typesetter, "ab", MathMode::Inline);
    assert_eq!(
        with_box.fragment.surface.width.0,
        plain.fragment.surface.width.0 + 3 * 65_536
    );
    assert_eq!(boxes.requests.borrow().len(), 1);
}
