//! Conformance oracle against real TeX `\showbox` dumps, plus regressions over the Computer Modern fixtures.

mod showbox;
mod support;

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use mathtex_engine::{MathMode, NoHostBoxes, Options, TypesetError};
use mathtex_ir::{BoxKind, Fragment, LayoutNodeKind, Length};
use showbox::{compare, extract_showbox, normalize_fragment, parse_showbox, Norm};
use support::{
    bake, binary, cm_fonts, primary_slice, texlive, typesetter, TokenWideBoxes, CM_MATH_SETUP,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Runs `tex -ini` on `setup` and `\setbox0=\hbox{body}`, returning the `\showbox0` dump lines.
fn run_tex_showbox(tex: &str, setup: &str, body: &str) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!(
        "mathtex-conformance-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let program = format!(
        concat!(
            r"\catcode`\{{=1 \catcode`\}}=2 \catcode`\$=3 ",
            r"\tracingonline=1 \showboxdepth=2147483647 \showboxbreadth=2147483647 ",
            "{setup} ",
            r"\setbox0=\hbox{{{body}}}\showbox0 \end",
            "\n"
        ),
        setup = setup,
        body = body
    );
    std::fs::write(dir.join("frag.tex"), program).expect("write tex source");
    // `tex -ini` exits nonzero after `\showbox`, so the log is what counts.
    let _ = Command::new(tex)
        .current_dir(&dir)
        .args(["-ini", "-interaction=batchmode", "frag.tex"])
        .output()
        .expect("run tex");
    let log = std::fs::read_to_string(dir.join("frag.log")).expect("read tex log");
    let _ = std::fs::remove_dir_all(&dir);
    extract_showbox(&log)
}

/// Bakes `setup` into an INITEX format with math catcodes, fonts from TeX Live.
fn engine_format(setup: &str) -> Option<Vec<u8>> {
    let texmf = texlive()?;
    let preamble = format!(r"\catcode`\{{=1 \catcode`\}}=2 \catcode`\$=3 {setup}");
    Some(bake(texmf, None, &preamble))
}

/// Typesets `math` against `format`, as TeX's `\hbox{$math$}`.
fn engine_fragment(format: &[u8], math: &str) -> Result<Fragment, TypesetError> {
    let texmf = texlive().expect("a format was baked");
    let mut typesetter = typesetter(texmf, format, Options::default());
    typesetter
        .typeset(math, MathMode::Inline, &NoHostBoxes)
        .map(|typeset| typeset.fragment)
}

/// Math setup every INITEX case needs, math mode wants symbol fonts, `"270370` is plain TeX's surd.
macro_rules! math_setup {
    () => {
        concat!(
            r"\catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
            r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 ",
            r"\textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm ",
            r"\textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni ",
            r"\textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy ",
            r"\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex ",
            r"\def\text#1{\hbox{#1}}\def\frac#1#2{{#1\over#2}}",
            "\\def\\sqrt{\\radical\"270370 } ",
        )
    };
}

const MATH_SETUP: &str = math_setup!();

struct Case {
    name: &'static str,
    /// TeX run before the box on both sides, the engine bakes it into its format.
    setup: &'static str,
    /// Math list of the compared `\hbox{$...$}`.
    math: &'static str,
}

fn corpus() -> Vec<Case> {
    let case = |name, setup, math| Case { name, setup, math };
    let rm = concat!(math_setup!(), r"\tenrm");
    vec![
        case(
            "hbox-vrule",
            MATH_SETUP,
            r"\hbox{\vrule width 1pt height 2pt depth 0pt}",
        ),
        case("hbox-glyph-A", rm, r"\hbox{A}"),
        case("hbox-glyph-AB", rm, r"\hbox{AB}"),
        case("hbox-kern", MATH_SETUP, r"\hbox{\kern 3pt}"),
        case("text-A-b", MATH_SETUP, r"\hbox{\tenrm A b}"),
        case("math-x", MATH_SETUP, "x"),
        case("math-x-sup-2", MATH_SETUP, "x^2"),
        case("math-x-sub-i", MATH_SETUP, "x_i"),
        case("math-a-plus-b", MATH_SETUP, "a+b"),
        case("math-frac-a-b", MATH_SETUP, r"\frac{a}{b}"),
        case("math-sqrt-x", MATH_SETUP, r"\sqrt{x}"),
        case("text-hi", MATH_SETUP, r"\text{\tenrm hi}"),
        // Boxes appended in internal vertical mode stay in their box.
        case("vbox-hbox", rm, r"\vbox{\hbox{W}}"),
        case("vtop-hbox", rm, r"\vtop{\hbox{W}\hbox{W}}"),
        case("fbox-shape", rm, r"\vrule\vbox{\hrule\hbox{W}\hrule}\vrule"),
        case(
            "math-vcenter-hbox",
            MATH_SETUP,
            r"\vcenter{\hbox{\tenrm W}}",
        ),
        // Knuth-Plass line breaking exercises paragraph glue and discretionaries.
        case(
            "par-linebreak",
            concat!(math_setup!(), r"\tenrm \hsize=100pt \parindent=0pt"),
            r"\vbox{\noindent Quisque ut dolor sit amet velit aliquam tincidunt nec eu nisl porttitor mauris.\par}",
        ),
    ]
}

#[test]
fn conformance_corpus_matches_tex() {
    let (Some(tex), Some(_)) = (binary("tex"), texlive()) else {
        return;
    };
    let mut report = String::new();
    let mut mismatched = Vec::new();
    let mut formats = std::collections::BTreeMap::new();
    for case in corpus() {
        let block = run_tex_showbox(&tex, case.setup, &format!("${}$", case.math));
        assert!(!block.is_empty(), "{}: empty \\showbox dump", case.name);
        let format = formats
            .entry(case.setup)
            .or_insert_with(|| engine_format(case.setup).expect("TeX Live is present"));
        let actual = engine_fragment(format, case.math)
            .map_err(|error| error.to_string())
            .and_then(|fragment| normalize_fragment(&fragment));
        let (expected, diffs) = compare(&block, &actual, case.name);
        report.push_str(&format!(
            "\n===== {} =====\n  tex:    {expected:?}\n  engine: {actual:?}\n",
            case.name
        ));
        if !diffs.is_empty() {
            mismatched.push(case.name);
            for diff in &diffs {
                report.push_str(&format!("  DIFF {diff}\n"));
            }
        }
    }
    assert!(
        mismatched.is_empty(),
        "engine differs from tex on {mismatched:?}\n{report}"
    );
}

#[test]
fn golden_showbox_parses_rules_kerns_and_glyphs() {
    let Some(tex) = binary("tex") else {
        return;
    };
    let children = |setup: &str, body: &str| {
        let Norm::Box { children, .. } = parse_showbox(&run_tex_showbox(&tex, setup, body)) else {
            panic!("expected a root box for {body}");
        };
        children
    };
    let rule = children("", r"\vrule width 1pt height 2pt depth 0pt");
    assert_eq!(rule.len(), 1);
    assert!(matches!(
        rule[0].node,
        Norm::Rule {
            width: 65_536,
            height: 131_072,
            depth: 0
        }
    ));
    let kern = children("", r"\kern 3pt");
    assert_eq!(kern[0].node, Norm::Kern { amount: 196_608 });
    let glyph = children(r"\font\tenrm=cmr10 \tenrm", "A");
    assert!(matches!(glyph[0].node, Norm::Glyph { glyph, .. } if glyph == u32::from(b'A')));
    // The parser drops the math markers around `$x$`, leaving the one character.
    let tree = parse_showbox(&run_tex_showbox(&tex, MATH_SETUP, "$x$"));
    let Norm::Box {
        kind,
        width,
        height,
        children,
        ..
    } = tree
    else {
        panic!("expected a root box");
    };
    assert_eq!(kind, BoxKind::Horizontal);
    // tex prints \hbox(4.30554+0.0)x5.71527.
    assert_eq!((height, width), (282_168, 374_556));
    assert_eq!(children.len(), 1);
    assert!(matches!(children[0].node, Norm::Glyph { glyph, .. } if glyph == u32::from(b'x')));
}

/// Math setup, muskips INITEX leaves at zero, and LaTeX's `\everymath` shape for the sandbox cases.
const HOST_SETUP: &str = concat!(
    r"\thinmuskip=3mu \medmuskip=4mu \thickmuskip=5mu ",
    r"\catcode`\@=11 \def\@gobble#1{}\def\@ignorefalse{\global\let\if@ignore\iffalse}",
    r"\everymath{\aftergroup\@ignorefalse}",
);

/// The Computer Modern fixture format with [`HOST_SETUP`], baked once per test binary.
fn host_format() -> Option<&'static [u8]> {
    static FORMAT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let fonts = cm_fonts()?;
    Some(FORMAT.get_or_init(|| bake(fonts, None, &format!("{CM_MATH_SETUP}{HOST_SETUP}"))))
}

#[test]
fn a_host_box_keeps_its_width_as_the_nucleus_of_any_atom() {
    let (Some(fonts), Some(format)) = (cm_fonts(), host_format()) else {
        return;
    };
    let mut typesetter = typesetter(fonts, format, Options::default());
    let boxes = TokenWideBoxes::default();
    let mut width = |math: &str| {
        typesetter
            .typeset(math, MathMode::Inline, &boxes)
            .unwrap_or_else(|error| panic!("{math}: {error}"))
            .fragment
            .surface
            .width
            .0
    };
    let thin = width(r"\mskip\thinmuskip");
    let med = width(r"\mskip\medmuskip");
    let thick = width(r"\mskip\thickmuskip");
    assert!(thin > 0 && med > 0 && thick > 0);
    for style in ["", r"\displaystyle "] {
        let ord = width(&format!(r"{style}a\hostbox{{1}}x"));
        for limits in ["", r"\limits", r"\nolimits"] {
            let op = width(&format!(r"{style}a\mathop{{\hostbox{{1}}}}{limits} x"));
            assert_eq!(op - ord, 2 * thin, "{style}{limits}");
        }
        assert_eq!(
            width(&format!(r"{style}a\mathbin{{\hostbox{{1}}}}x")) - ord,
            2 * med
        );
        assert_eq!(
            width(&format!(r"{style}a\mathrel{{\hostbox{{1}}}}x")) - ord,
            2 * thick
        );
    }
}

#[test]
fn a_sandboxed_fragment_cannot_leave_the_wrapper_math() {
    let (Some(fonts), Some(format)) = (cm_fonts(), host_format()) else {
        return;
    };
    let mut typesetter = typesetter(fonts, format, Options::default().with_cache_capacity(0));
    let mut run = |math: &str| typesetter.typeset(math, MathMode::Display, &NoHostBoxes);
    // The last two back the wrapper's `$` up while scanning a keyword or a number.
    for math in [
        r"x",
        r"x+\hbox{\tenrm a$y$b}",
        r"\hbox{$x$}+\hbox{$y$}",
        r"a+\hbox{$x$}+b",
        r"\hbox{$x$ $y$}",
        r"\hbox{$\hbox{$x$}$}",
        r"x\mskip 3mu",
        r"x\count1=5",
    ] {
        if let Err(error) = run(math) {
            panic!("{math}: {error}");
        }
    }
    for math in [
        r"x$ \tenrm text \@gobble",
        r"x$}\hbox{\tenrm text}{$",
        r"x$}",
        r"x$\relax",
        r"\def\a#1{x#1\tenrm text}\a",
        r"\def\a#1{}x$\a",
        r"\def\a{$}x\a\def\b#1{}\b",
        r"x\count1=5$\def\b#1{}\b",
    ] {
        match run(math) {
            Err(TypesetError::Sandbox { message, .. }) => {
                assert!(message.contains("math shift"), "{math}: {message}")
            }
            other => panic!("{math}: expected a sandbox rejection, got {other:?}"),
        }
    }
    // A stray closing brace is a TeX error inside the math group.
    assert!(matches!(run(r"x}"), Err(TypesetError::Tex { .. })));
    assert!(matches!(
        run(r"\openin1=missing"),
        Err(TypesetError::Sandbox { .. })
    ));
}

#[test]
fn a_box_set_then_placed_is_part_of_the_fragment() {
    let (Some(fonts), Some(format)) = (cm_fonts(), host_format()) else {
        return;
    };
    let mut typesetter = typesetter(fonts, format, Options::default());
    let result = typesetter
        .typeset(
            r"\setbox0\hbox{\tenrm W}\box0",
            MathMode::Inline,
            &NoHostBoxes,
        )
        .expect("the fragment typesets");
    let fragment = result.fragment;
    let root = fragment.root_node().expect("root");
    let placed = fragment
        .children(fragment.root)
        .iter()
        .map(|&id| fragment.node(id).expect("child"))
        .find(|node| matches!(node.kind, LayoutNodeKind::Box(_)))
        .expect("the placed box is a child of the wrapper box");
    assert_eq!(placed.width, root.width);
    assert!(fragment.children(placed.id).iter().any(|&id| {
        matches!(&fragment.node(id).expect("glyph").kind, LayoutNodeKind::GlyphRun(run) if run.glyphs[0].glyph_id.0 == u32::from(b'W'))
    }));
}

#[test]
fn ligatures_keep_the_span_of_their_characters_and_math_surround_becomes_kerns() {
    let (Some(fonts), Some(format)) = (cm_fonts(), host_format()) else {
        return;
    };
    let mut typesetter = typesetter(fonts, format, Options::default());
    let tex = r"\hbox{\tenrm fi\mathsurround=2pt $x$}";
    let fragment = typesetter
        .typeset(tex, MathMode::Inline, &NoHostBoxes)
        .expect("the fragment typesets")
        .fragment;
    let ligature = fragment
        .nodes
        .iter()
        .find(|node| {
            matches!(&node.kind, LayoutNodeKind::GlyphRun(run) if run.glyphs[0].glyph_id.0 == 12)
        })
        .expect("cmr10 puts the fi ligature at character 12");
    assert_eq!(primary_slice(&fragment, tex, ligature.id), Some("fi"));
    let surrounds = fragment
        .nodes
        .iter()
        .filter_map(|node| match node.kind {
            LayoutNodeKind::Kern(kern) => Some(kern.amount),
            _ => None,
        })
        .filter(|&amount| amount == Length::from_scaled_points(2 * 65_536))
        .count();
    assert_eq!(surrounds, 2);
}

#[test]
fn the_budget_and_node_cap_end_runaway_fragments() {
    let (Some(fonts), Some(format)) = (cm_fonts(), host_format()) else {
        return;
    };
    let options = Options::default().with_op_budget(10_000).with_max_nodes(50);
    let mut typesetter = typesetter(fonts, format, options);
    assert_eq!(
        typesetter.typeset(r"\def\x{\x}\x", MathMode::Inline, &NoHostBoxes),
        Err(TypesetError::Budget)
    );
    let leaders = r"\hbox to 100pt{\xleaders\hbox to 1pt{\vrule width 1pt height 1pt}\hfil}";
    assert_eq!(
        typesetter.typeset(leaders, MathMode::Inline, &NoHostBoxes),
        Err(TypesetError::NodeLimit { limit: 50 })
    );
}
