//! Compares fragments typeset with OpenType math fonts against live `xetex` `\showbox` dumps.

mod showbox;
mod support;

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use mathtex_engine::{MathMode, NoHostBoxes, Options};
use mathtex_ir::BoxKind;
use showbox::{compare, extract_showbox, normalize_fragment, parse_showbox, pt_to_sp, Norm};
use support::{bake, binary, texlive, typesetter};

const LM_MATH: &str = "latinmodern-math.otf";
const STIX_MATH: &str = "STIXTwoMath-Regular.otf";

fn math_setup(font: &str, extra: &str) -> String {
    format!(
        concat!(
            r"\catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
            "\\font\\mf=\"[{font}]:script=math\" at 10pt ",
            r"\textfont0=\mf \scriptfont0=\mf \scriptscriptfont0=\mf ",
            r"\textfont1=\mf \scriptfont1=\mf \scriptscriptfont1=\mf ",
            r"\textfont2=\mf \scriptfont2=\mf \scriptscriptfont2=\mf ",
            r"\textfont3=\mf \scriptfont3=\mf \scriptscriptfont3=\mf ",
            "{extra} ",
        ),
        font = font,
        extra = extra,
    )
}

// `"0028` is a TeX hexadecimal constant, U+0028.
const DELIM_SETUP: &str = concat!(
    "\\Udelcode`\\(=\"0 \"0028 \\Udelcode`\\)=\"0 \"0029 ",
    r"\def\frac#1#2{{#1\over#2}}",
    " \\def\\sqrt{\\Uradical\"0 \"221A }",
);

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Runs `xetex -ini` with `args` on `setup` and `\setbox0=\hbox{$math$}`, returning the dump.
fn run_xetex_showbox(xetex: &str, setup: &str, math: &str, args: &[&str]) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!(
        "mtx-xetex-native-conf-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let program = format!(
        concat!(
            r"\catcode`\{{=1 \catcode`\}}=2 ",
            r"\tracingonline=1 \showboxdepth=2147483647 \showboxbreadth=2147483647 ",
            "{setup} ",
            r"\setbox0=\hbox{{${math}$}}\showbox0 \end",
            "\n"
        ),
        setup = setup,
        math = math,
    );
    std::fs::write(dir.join("frag.tex"), program).expect("write tex source");
    let _ = Command::new(xetex)
        .current_dir(&dir)
        .env("TEXMFOUTPUT", &dir)
        .arg("-ini")
        .args(args)
        .args(["-interaction=batchmode", "frag.tex"])
        .output()
        .expect("run xetex");
    let log = std::fs::read_to_string(dir.join("frag.log")).expect("read xetex log");
    let _ = std::fs::remove_dir_all(&dir);
    extract_showbox(&log)
}

/// Typesets `math` against a format baked from `setup`, normalized for comparison.
fn engine_layout(setup: &str, math: &str) -> Result<Norm, String> {
    static FORMATS: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());
    let texmf = texlive().ok_or("no TeX Live tree")?;
    let format = FORMATS
        .lock()
        .expect("no test panicked while baking")
        .entry(setup.to_string())
        .or_insert_with(|| {
            bake(
                texmf,
                None,
                &format!(r"\catcode`\{{=1 \catcode`\}}=2 {setup}"),
            )
        })
        .clone();
    let mut typesetter = typesetter(texmf, &format, Options::default());
    let fragment = typesetter
        .typeset(math, MathMode::Inline, &NoHostBoxes)
        .map_err(|error| error.to_string())?
        .fragment;
    normalize_fragment(&fragment)
}

struct Case {
    name: &'static str,
    font: &'static str,
    extra: &'static str,
    math: &'static str,
}

fn corpus() -> Vec<Case> {
    let case = |name, font, extra, math| Case {
        name,
        font,
        extra,
        math,
    };
    vec![
        case("native-x", LM_MATH, "", "x"),
        case("native-x-sup-2", LM_MATH, "", "x^2"),
        case("native-x-sub-i", LM_MATH, "", "x_i"),
        case("native-x-sub-i-sup-2", LM_MATH, "", "x_i^2"),
        case("native-a-plus-b", LM_MATH, "", "a+b"),
        // STIX MathKernInfo cut-ins produce visible kerns, latinmodern-math has none.
        case("kern-stix-F-sup-2", STIX_MATH, "", "F^2"),
        case("kern-stix-V-sub-x", STIX_MATH, "", "V_x"),
        case("kern-stix-T-sub-x", STIX_MATH, "", "T_x"),
        case("kern-stix-V-sub-x-sup-2", STIX_MATH, "", "V_x^2"),
        // A growable delimiter picks a larger single glyph variant rather than an assembly.
        case(
            "variant-left-frac-right",
            LM_MATH,
            DELIM_SETUP,
            r"\left(\frac{a}{b}\right)",
        ),
        case(
            "variant-sqrt-frac",
            LM_MATH,
            DELIM_SETUP,
            r"\sqrt{\frac{a}{b}}",
        ),
        case(
            "frac-plus-sqrt",
            LM_MATH,
            DELIM_SETUP,
            r"\frac{a}{b}+\sqrt{x^2}",
        ),
        // The nest outgrows the precomposed variants, forcing stacked delimiter parts.
        case(
            "assembly-left-tall-right",
            LM_MATH,
            DELIM_SETUP,
            r"\left(\frac{\frac{\frac{a}{b}}{\frac{a}{b}}}{\frac{\frac{a}{b}}{\frac{a}{b}}}\right)",
        ),
    ]
}

#[test]
fn native_conformance_corpus_matches_xetex() {
    let (Some(xetex), Some(_)) = (binary("xetex"), texlive()) else {
        return;
    };
    let mut report = String::new();
    let mut mismatched = Vec::new();
    for case in corpus() {
        let setup = math_setup(case.font, case.extra);
        let block = run_xetex_showbox(&xetex, &setup, case.math, &[]);
        assert!(
            !block.is_empty(),
            "{}: empty xetex \\showbox dump",
            case.name
        );
        let actual = engine_layout(&setup, case.math);
        let (expected, diffs) = compare(&block, &actual, case.name);
        report.push_str(&format!(
            "\n===== {} ({}) =====\n  xetex:  {expected:?}\n  engine: {actual:?}\n",
            case.name, case.font
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
        "engine differs from xetex on {mismatched:?}\n{report}"
    );
}

/// The children of xetex's box for `math`.
fn xetex_children(xetex: &str, setup: &str, math: &str) -> Vec<showbox::Child> {
    let Norm::Box { children, .. } = parse_showbox(&run_xetex_showbox(xetex, setup, math, &[]))
    else {
        panic!("expected a root box for {math}");
    };
    children
}

#[test]
fn golden_xetex_dumps_parse_glyphs_kerns_and_variants() {
    let Some(xetex) = binary("xetex") else {
        return;
    };
    let tree = parse_showbox(&run_xetex_showbox(
        &xetex,
        &math_setup(LM_MATH, ""),
        "x",
        &[],
    ));
    let Norm::Box {
        kind,
        width,
        height,
        depth,
        children,
        ..
    } = tree
    else {
        panic!("expected a root box");
    };
    assert_eq!(kind, BoxKind::Horizontal);
    // xetex prints \hbox(4.31+0.0)x5.44, glyph 89 then an italic correction kern of 0.16pt.
    assert_eq!((width, height, depth), (356_516, 282_460, 0));
    assert_eq!(children.len(), 2);
    assert!(matches!(children[0].node, Norm::Glyph { glyph: 89, .. }));
    assert_eq!(children[1].node, Norm::Kern { amount: 10_486 });

    // STIX glyph 8 has a constant TopRight superscript cut-in of 0.44pt.
    let stix = xetex_children(&xetex, &math_setup(STIX_MATH, ""), "F^2");
    assert!(matches!(stix[0].node, Norm::Glyph { glyph: 8, .. }));
    assert_eq!(
        stix[1].node,
        Norm::Kern {
            amount: pt_to_sp("0.44")
        }
    );

    let setup = math_setup(LM_MATH, DELIM_SETUP);
    let dump = |math: &str| {
        format!(
            "{:?}",
            parse_showbox(&run_xetex_showbox(&xetex, &setup, math, &[]))
        )
    };
    // latinmodern-math's `(`, glyph 9, picks its fifth vertical variant.
    let variants = dump(r"\left(\frac{a}{b}\right)");
    for glyph in ["2433", "2434"] {
        assert!(
            variants.contains(&format!("Glyph {{ glyph: {glyph}")),
            "{glyph} in {variants}"
        );
    }
    let assembly = dump(
        r"\left(\frac{\frac{\frac{a}{b}}{\frac{a}{b}}}{\frac{\frac{a}{b}}{\frac{a}{b}}}\right)",
    );
    for part in ["2503", "2504", "2505", "2506", "2507", "2508"] {
        assert!(
            assembly.contains(&format!("Glyph {{ glyph: {part}")),
            "{part} in {assembly}"
        );
    }
}

/// Compares whole trees when `whole_tree`, else root sizes, `-etex` enables the metric primitives.
fn compare_with_xetex(
    xetex: &str,
    font: &str,
    extra: &str,
    math: &str,
    whole_tree: bool,
) -> Vec<String> {
    let setup = math_setup(font, extra);
    let block = run_xetex_showbox(xetex, &setup, math, &["-etex"]);
    let actual = engine_layout(&setup, math);
    if whole_tree {
        return compare(&block, &actual, math).1;
    }
    let actual = match actual {
        Ok(actual) => actual,
        Err(reason) => return vec![format!("{math}: engine produced no layout ({reason})")],
    };
    let dims = |norm: &Norm| match norm {
        Norm::Box {
            width,
            height,
            depth,
            ..
        } => Some((*width, *height, *depth)),
        _ => None,
    };
    match (dims(&parse_showbox(&block)), dims(&actual)) {
        (Some(expected), Some(actual)) if expected == actual => Vec::new(),
        (expected, actual) => vec![format!("{math}: xetex {expected:?} engine {actual:?}")],
    }
}

#[test]
fn native_char_metrics_match_xetex() {
    let (Some(xetex), Some(_)) = (binary("xetex"), texlive()) else {
        return;
    };
    let mut diffs = Vec::new();
    // Each metric becomes an explicit kern, so the trees compare scaled point for scaled point.
    let kerns = concat!(
        r"\hbox{\kern\fontcharwd\mf`x \kern\fontcharht\mf`x \kern\fontchardp\mf`g ",
        r"\kern\fontcharic\mf`f \kern\fontcharht\mf`A \kern\fontchardp\mf`( ",
        r"\mf \kern\XeTeXglyphbounds1 \XeTeXcharglyph`f \kern\XeTeXglyphbounds2 \XeTeXcharglyph`f ",
        r"\kern\XeTeXglyphbounds3 \XeTeXcharglyph`f \kern\XeTeXglyphbounds4 \XeTeXcharglyph`f ",
        r"\kern\XeTeXfirstfontchar\mf sp \kern\XeTeXlastfontchar\mf sp}",
    );
    diffs.extend(compare_with_xetex(&xetex, LM_MATH, "", kerns, true));
    // Italic corrections of a native word and a glyph, and text accents placed from bearings and heights.
    for text in [
        "\\hbox{\\mf \u{1d453}\\/}",
        r"\hbox{\mf\XeTeXglyph\XeTeXcharglyph`f\/}",
        "\\hbox{\\mf\\accent\"0302 x}",
        "\\hbox{\\mf\\accent\"005E A}",
    ] {
        diffs.extend(compare_with_xetex(&xetex, LM_MATH, "", text, false));
    }
    assert!(
        diffs.is_empty(),
        "native char metrics diverge from xetex:\n{}",
        diffs.join("\n")
    );
}

#[test]
fn stix_math_kerns_match_xetex() {
    let (Some(xetex), Some(_)) = (binary("xetex"), texlive()) else {
        return;
    };
    // Smaller script fonts scale script kerns by 0.7, which XeTeX adds to the base kern before truncating.
    let script_sizes = concat!(
        "\\font\\ms=\"[STIXTwoMath-Regular.otf]:script=math\" at 7pt ",
        "\\font\\mss=\"[STIXTwoMath-Regular.otf]:script=math\" at 5pt ",
        r"\scriptfont0=\ms \scriptfont1=\ms \scriptfont2=\ms \scriptfont3=\ms ",
        r"\scriptscriptfont0=\mss \scriptscriptfont1=\mss \scriptscriptfont2=\mss \scriptscriptfont3=\mss",
    );
    let mut diffs = Vec::new();
    // A^T and L^T are where truncating the scaled script kern alone lands one design unit off.
    for base in ["A", "L", "V", "f"] {
        for script in ["T", "2", "x"] {
            for shape in ["^", "_"] {
                let math = format!("{base}{shape}{{{script}}}");
                diffs.extend(compare_with_xetex(
                    &xetex,
                    STIX_MATH,
                    script_sizes,
                    &math,
                    true,
                ));
            }
        }
    }
    assert!(
        diffs.is_empty(),
        "math kerns diverge from xetex:\n{}",
        diffs.join("\n")
    );
}
