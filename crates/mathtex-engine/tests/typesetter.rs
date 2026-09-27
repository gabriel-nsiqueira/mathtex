//! Host boxes, the fragment cache and warnings of a Typesetter over a Computer Modern INITEX format.

mod support;

use std::cell::Cell;

use mathtex_engine::font::FontLoader;
use mathtex_engine::{
    DiagnosticKind, HostBox, HostBoxGlyph, HostBoxRequest, HostBoxRule, HostBoxRun, HostBoxes,
    MathMode, MathStyle, NoHostBoxes, Options, Typeset,
};
use mathtex_ir::{FontKey, GlyphId, LayoutNodeKind, Length, Point};
use support::{bake, cm_fonts, cm_format, texlive, typesetter, CmFonts, TokenWideBoxes};

fn pt(points: i32) -> Length {
    Length::from_scaled_points(points * 65_536)
}

/// A typesetter over the Computer Modern fixture format.
fn cm_typesetter(options: Options) -> Option<mathtex_engine::Typesetter<CmFonts>> {
    let (fonts, format) = (cm_fonts()?, cm_format()?);
    Some(typesetter(fonts, format, options))
}

fn run<L: FontLoader>(
    typesetter: &mut mathtex_engine::Typesetter<L>,
    tex: &str,
    mode: MathMode,
    boxes: &dyn HostBoxes,
) -> Typeset {
    typesetter
        .typeset(tex, mode, boxes)
        .unwrap_or_else(|error| panic!("{tex:?} typesets: {error}"))
}

/// Answers every request with one fixed box.
struct FixedBox(HostBox);

impl HostBoxes for FixedBox {
    fn host_box(&self, _request: &HostBoxRequest) -> Option<HostBox> {
        Some(self.0.clone())
    }
}

#[test]
fn a_missing_host_box_renders_nothing_and_warns() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let with_box = run(
        &mut typesetter,
        r"a\hostbox{5}b",
        MathMode::Inline,
        &TokenWideBoxes::default(),
    );
    let missing = run(
        &mut typesetter,
        r"a\hostbox{5}b",
        MathMode::Inline,
        &NoHostBoxes,
    );
    let plain = run(&mut typesetter, "ab", MathMode::Inline, &NoHostBoxes);
    assert_eq!(
        with_box.fragment.surface.width,
        plain.fragment.surface.width + pt(5)
    );
    // The empty box is still an Ord atom, it adds no width of its own.
    assert_eq!(missing.fragment.surface.width, plain.fragment.surface.width);
    assert_eq!(missing.warnings.len(), 1);
    assert_eq!(missing.warnings[0].kind, DiagnosticKind::HostBox);
    assert!(plain.warnings.is_empty(), "{:?}", plain.warnings);
}

#[test]
fn a_host_box_places_its_metrics_and_render_data_as_one_ord_atom() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let host_box = HostBox::new(pt(5), pt(12), pt(3))
        .with_run(HostBoxRun::new(
            FontKey(77),
            pt(10),
            vec![HostBoxGlyph::new(GlyphId(70_000), Point::ORIGIN)],
        ))
        .with_rule(HostBoxRule::new(
            Point::new(Length::ZERO, pt(-2)),
            pt(5),
            Length::from_scaled_points(26_214),
        ));
    let fragment = run(
        &mut typesetter,
        r"a\hostbox{7}b",
        MathMode::Inline,
        &FixedBox(host_box),
    )
    .fragment;
    let plain = run(&mut typesetter, "ab", MathMode::Inline, &NoHostBoxes).fragment;

    let host_runs = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            LayoutNodeKind::GlyphRun(run) if run.font.key == Some(FontKey(77)) => Some(run),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(host_runs.len(), 1);
    // Glyph ids past the 16 bit range survive, host fonts may be large.
    assert_eq!(host_runs[0].glyphs[0].glyph_id, GlyphId(70_000));
    assert_eq!(host_runs[0].font.size, pt(10));
    let host_rules = fragment
        .nodes
        .iter()
        .filter(|node| {
            node.kind == LayoutNodeKind::Rule
                && node.width == pt(5)
                && node.height == Length::from_scaled_points(26_214)
        })
        .count();
    assert_eq!(host_rules, 1);
    // a, the box and b are Ord atoms, so the box adds exactly its width.
    assert_eq!(fragment.surface.width, plain.surface.width + pt(5));
    assert_eq!(fragment.surface.baseline, pt(12));
    assert_eq!(fragment.surface.height, pt(15));
}

#[test]
fn a_host_box_keeps_its_width_through_a_fraction_rebox() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let host_box = HostBox::new(pt(5), pt(8), Length::ZERO);
    // The denominator is narrower than "ab", so make_fraction reboxes it.
    let fragment = run(
        &mut typesetter,
        r"{ab\over\hostbox{7}}",
        MathMode::Inline,
        &FixedBox(host_box),
    )
    .fragment;
    let boxes = fragment
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, LayoutNodeKind::Box(_)) && node.width == pt(5))
        .collect::<Vec<_>>();
    assert_eq!(
        boxes.len(),
        1,
        "the rebox dissolves the shell, one record box remains"
    );
    let parent = fragment
        .nodes
        .iter()
        .find(|node| node.children().contains(&boxes[0].id))
        .expect("the record box sits in the reboxed line");
    // Centering glue splits the leftover space, not the whole line width.
    let expected = (parent.width.0 - pt(5).0) / 2;
    let offset = boxes[0].origin.x.0;
    assert!(
        (offset - expected).abs() <= 2,
        "box at {offset}, expected {expected}"
    );
    assert!(offset > 0);
}

#[test]
fn host_box_requests_carry_the_style_and_size_of_their_site() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let boxes = TokenWideBoxes::default();
    for (tex, mode) in [
        (r"\hostbox{1}", MathMode::Inline),
        (r"\hostbox{2}", MathMode::Display),
        (r"x^{\hostbox{3}}", MathMode::Inline),
        (r"x^{y^{\hostbox{4}}}", MathMode::Inline),
        (r"\text{\tenrm\hostbox{5}}", MathMode::Inline),
    ] {
        run(&mut typesetter, tex, mode, &boxes);
    }
    let requests = boxes.requests.borrow();
    let style = |token: u32| {
        requests
            .iter()
            .find(|request| request.token == token)
            .unwrap_or_else(|| panic!("token {token} reached the host"))
    };
    assert_eq!(style(1).style, MathStyle::Text);
    assert_eq!(style(2).style, MathStyle::Text);
    assert_eq!(style(3).style, MathStyle::Script);
    assert_eq!(style(4).style, MathStyle::ScriptScript);
    assert_eq!(style(5).style, MathStyle::Text);
    // Inside \text the request carries the selected text font's size.
    assert_eq!(style(5).size, pt(10));
}

#[test]
fn a_host_box_with_a_negative_size_is_rejected_with_a_warning() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let negative = FixedBox(HostBox::new(pt(-1), pt(2), Length::ZERO));
    let result = run(
        &mut typesetter,
        r"a\hostbox{1}b",
        MathMode::Inline,
        &negative,
    );
    let plain = run(&mut typesetter, "ab", MathMode::Inline, &NoHostBoxes);
    assert_eq!(result.fragment.surface.width, plain.fragment.surface.width);
    assert_eq!(result.warnings.len(), 1);
    assert!(
        result.warnings[0].message.contains("negative"),
        "{:?}",
        result.warnings
    );
}

/// Counts requests, with a settable revision for token 1.
#[derive(Default)]
struct CountingBoxes {
    requests: Cell<usize>,
    revision: Cell<Option<u64>>,
}

impl HostBoxes for CountingBoxes {
    fn host_box(&self, request: &HostBoxRequest) -> Option<HostBox> {
        self.requests.set(self.requests.get() + 1);
        Some(HostBox::new(pt(request.token as i32), pt(1), Length::ZERO))
    }

    fn revision(&self, _token: u32) -> Option<u64> {
        self.revision.get()
    }
}

#[test]
fn fragments_are_cached_while_their_host_boxes_keep_a_revision() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let boxes = CountingBoxes::default();
    let tex = r"a\hostbox{1}";
    run(&mut typesetter, tex, MathMode::Inline, &boxes);
    run(&mut typesetter, tex, MathMode::Inline, &boxes);
    assert_eq!(boxes.requests.get(), 2, "no revision, no caching");

    boxes.revision.set(Some(1));
    let first = run(&mut typesetter, tex, MathMode::Inline, &boxes);
    let cached = run(&mut typesetter, tex, MathMode::Inline, &boxes);
    assert_eq!(boxes.requests.get(), 3, "the second run is a cache hit");
    assert_eq!(first, cached);
    run(&mut typesetter, tex, MathMode::Display, &boxes);
    assert_eq!(boxes.requests.get(), 4, "the mode is part of the key");

    boxes.revision.set(Some(2));
    run(&mut typesetter, tex, MathMode::Inline, &boxes);
    assert_eq!(boxes.requests.get(), 5, "a new revision typesets again");

    let Some(mut uncached) = cm_typesetter(Options::default().with_cache_capacity(0)) else {
        return;
    };
    run(&mut uncached, tex, MathMode::Inline, &boxes);
    run(&mut uncached, tex, MathMode::Inline, &boxes);
    assert_eq!(boxes.requests.get(), 7, "capacity 0 turns caching off");
}

#[test]
fn overfull_boxes_and_missing_characters_come_back_as_warnings() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let overfull = run(
        &mut typesetter,
        r"\hbox to 1pt{\tenrm xxxx}",
        MathMode::Inline,
        &NoHostBoxes,
    );
    assert_eq!(
        overfull.warnings.iter().map(|w| w.kind).collect::<Vec<_>>(),
        [DiagnosticKind::OverfullBox]
    );
    // cmr10 has no character 200.
    let missing = run(
        &mut typesetter,
        r"\hbox{\tenrm\char200}",
        MathMode::Inline,
        &NoHostBoxes,
    );
    assert_eq!(
        missing.warnings.iter().map(|w| w.kind).collect::<Vec<_>>(),
        [DiagnosticKind::MissingCharacter]
    );
}

#[test]
fn plain_tex_bakes_and_typesets() {
    let Some(texmf) = texlive() else {
        return;
    };
    let format = bake(texmf, None, r"\input plain");
    let mut typesetter = typesetter(texmf, &format, Options::default());
    let result = run(
        &mut typesetter,
        r"x^2+\sqrt{y}",
        MathMode::Display,
        &NoHostBoxes,
    );
    assert!(support::glyph_ids(&result.fragment).len() >= 4);
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn a_trailing_comment_cannot_swallow_the_wrapper() {
    let Some(mut typesetter) = cm_typesetter(Options::default()) else {
        return;
    };
    let commented = run(
        &mut typesetter,
        "x+y% a comment",
        MathMode::Inline,
        &NoHostBoxes,
    );
    let plain = run(&mut typesetter, "x+y", MathMode::Inline, &NoHostBoxes);
    assert_eq!(commented.fragment.surface, plain.fragment.surface);
}
