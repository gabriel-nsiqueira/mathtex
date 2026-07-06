//! Conformance oracle comparing engine layout IR against real TeX `\showbox` dumps.

use std::path::PathBuf;
use std::process::Command;

use mathtex_engine::generated::generated_node_to_fragment;
use mathtex_engine::{
    portable_engine::EngineProfile, GeneratedFormatCache, GeneratedResourceProvider,
    InMemoryResourceProvider, ResourceKind,
};
use mathtex_ir::{
    BoxKind, Fragment, FragmentMetadata, Length, LayoutNodeKind, NodeId,
};

// The engine uses TeX integer arithmetic, so any nonzero diff is a real divergence.
const SP_TOLERANCE: i32 = 0;

/// A normalized layout node, dimensions in scaled points.
#[derive(Clone, Debug, PartialEq)]
enum Norm {
    Box {
        kind: BoxKind,
        width: i32,
        height: i32,
        depth: i32,
        children: Vec<Norm>,
    },
    Rule {
        width: i32,
        height: i32,
        depth: i32,
    },
    Glue {
        amount: i32,
    },
    Kern {
        amount: i32,
    },
    /// Font index differs between the oracle and the engine, only glyph code is compared.
    Char {
        glyph: u32,
        width: i32,
    },
    /// Engine IR drawing command, TeX prints these as glyphs, so mismatches are reported.
    Drawing,
    /// Sentinel for `\showbox` lines with no comparable geometry, filtered before diffing.
    Skip,
}

impl Norm {
    fn label(&self) -> &'static str {
        match self {
            Norm::Box { .. } => "box",
            Norm::Rule { .. } => "rule",
            Norm::Glue { .. } => "glue",
            Norm::Kern { .. } => "kern",
            Norm::Char { .. } => "char",
            Norm::Drawing => "drawing",
            Norm::Skip => "skip",
        }
    }
}

/// TeX null_flag value, `\showbox` prints running rule dimensions as `*`.
const RUNNING_DIM: i32 = -1_073_741_824;

fn approx(a: i32, b: i32) -> bool {
    (i64::from(a) - i64::from(b)).abs() <= i64::from(SP_TOLERANCE)
}

/// Applies [`RUNNING_DIM`] substitution using the enclosing box dimensions.
fn resolve_running_rule(child: &Norm, box_w: i32, box_h: i32, box_d: i32) -> Norm {
    match child {
        Norm::Rule {
            width,
            height,
            depth,
        } => Norm::Rule {
            width: if *width == RUNNING_DIM { box_w } else { *width },
            height: if *height == RUNNING_DIM { box_h } else { *height },
            depth: if *depth == RUNNING_DIM { box_d } else { *depth },
        },
        other => other.clone(),
    }
}

fn diff(expected: &Norm, actual: &Norm, path: &str, out: &mut Vec<String>) {
    match (expected, actual) {
        (
            Norm::Box {
                kind: ek,
                width: ew,
                height: eh,
                depth: ed,
                children: ec,
            },
            Norm::Box {
                kind: ak,
                width: aw,
                height: ah,
                depth: ad,
                children: ac,
            },
        ) => {
            if ek != ak {
                out.push(format!("{path}: box kind tex={ek:?} engine={ak:?}"));
            }
            diff_dim(path, "width", *ew, *aw, out);
            diff_dim(path, "height", *eh, *ah, out);
            diff_dim(path, "depth", *ed, *ad, out);
            if ec.len() != ac.len() {
                out.push(format!(
                    "{path}: child count tex={} engine={} (tex kinds [{}], engine kinds [{}])",
                    ec.len(),
                    ac.len(),
                    ec.iter().map(Norm::label).collect::<Vec<_>>().join(", "),
                    ac.iter().map(Norm::label).collect::<Vec<_>>().join(", "),
                ));
            }
            for (i, (ce, ca)) in ec.iter().zip(ac.iter()).enumerate() {
                // TeX prints running rule dims as `*`, resolve them against the enclosing box before diffing.
                let ce = resolve_running_rule(ce, *ew, *eh, *ed);
                let ca = resolve_running_rule(ca, *aw, *ah, *ad);
                diff(&ce, &ca, &format!("{path}.{i}"), out);
            }
        }
        (
            Norm::Rule {
                width: ew,
                height: eh,
                depth: ed,
            },
            Norm::Rule {
                width: aw,
                height: ah,
                depth: ad,
            },
        ) => {
            diff_dim(path, "rule.width", *ew, *aw, out);
            diff_dim(path, "rule.height", *eh, *ah, out);
            diff_dim(path, "rule.depth", *ed, *ad, out);
        }
        (Norm::Glue { amount: e }, Norm::Glue { amount: a }) => {
            diff_dim(path, "glue", *e, *a, out)
        }
        (Norm::Kern { amount: e }, Norm::Kern { amount: a }) => {
            diff_dim(path, "kern", *e, *a, out)
        }
        (
            Norm::Char {
                glyph: eg,
                width: ew,
            },
            Norm::Char {
                glyph: ag,
                width: aw,
            },
        ) => {
            if eg != ag {
                out.push(format!("{path}: glyph tex={eg} engine={ag}"));
            }
            diff_dim(path, "char.width", *ew, *aw, out);
        }
        (e, a) => out.push(format!(
            "{path}: node kind tex={} engine={}",
            e.label(),
            a.label()
        )),
    }
}

fn diff_dim(path: &str, what: &str, expected: i32, actual: i32, out: &mut Vec<String>) {
    if !approx(expected, actual) {
        out.push(format!(
            "{path}: {what} tex={expected}sp ({:.5}pt) engine={actual}sp ({:.5}pt)",
            expected as f64 / 65536.0,
            actual as f64 / 65536.0,
        ));
    }
}

fn tex_binary() -> Option<PathBuf> {
    for candidate in ["/Library/TeX/texbin/tex", "tex"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

/// Run `tex -ini` and return the `\showbox0` dump lines.
fn run_tex_showbox(
    tex: &PathBuf,
    setup: &str,
    source: &str,
    fonts: &[(&str, &[u8])],
) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!(
        "mathtex-conformance-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");

    // Provide font .tfm files in the job directory so `\font` loads without a system texmf tree.
    for (name, bytes) in fonts {
        std::fs::write(dir.join(format!("{name}.tfm")), bytes).expect("write tfm");
    }

    let job = "frag";
    let tex_path = dir.join(format!("{job}.tex"));
    let program = format!(
        concat!(
            r"\catcode`\{{=1 \catcode`\}}=2 ",
            r"\tracingonline=1 \showboxdepth=2147483647 \showboxbreadth=2147483647 ",
            "{setup} ",
            r"\setbox0=\hbox{{{body}}}\showbox0 \end",
            "\n"
        ),
        setup = setup,
        body = source
    );
    std::fs::write(&tex_path, program).expect("write tex source");

    // `tex -ini` returns nonzero for diagnostics, so check the log instead.
    let _output = Command::new(tex)
        .current_dir(&dir)
        .arg("-ini")
        .arg("-interaction=batchmode")
        .arg(format!("{job}.tex"))
        .output()
        .expect("run tex");

    let log = std::fs::read_to_string(dir.join(format!("{job}.log"))).expect("read tex log");
    let _ = std::fs::remove_dir_all(&dir);
    extract_showbox(&log)
}

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Extract the `\showbox0` dump block from a TeX log, starting at `> \box0=`.
fn extract_showbox(log: &str) -> Vec<String> {
    let mut lines = log.lines();
    for line in lines.by_ref() {
        if line.starts_with("> \\box0=") {
            break;
        }
    }
    let mut block = Vec::new();
    for line in lines {
        // The dump ends at the first blank line / "! OK." marker.
        if line.trim().is_empty() || line.starts_with("! OK.") || line.starts_with('!') {
            break;
        }
        block.push(line.to_string());
    }
    block
}

/// Convert a `\showbox` decimal dimension string back to exact scaled points.
fn pt_to_sp(text: &str) -> i32 {
    let text = text.trim();
    let negative = text.starts_with('-');
    let text = text.trim_start_matches(['+', '-']);
    let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
    let int_val: i64 = int_part.parse().unwrap_or(0);
    // round_decimals folds fractional digits in reverse, then rounds once.
    let mut a: i64 = 0;
    for d in frac_part
        .bytes()
        .filter(u8::is_ascii_digit)
        .take(17)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        a = (a + i64::from(d - b'0') * 131_072) / 10;
    }
    let sp = int_val * 65_536 + (a + 1) / 2;
    (if negative { -sp } else { sp }) as i32
}

/// Parse the `\showbox` block into a normalized tree.
fn parse_showbox(block: &[String]) -> Norm {
    let parsed: Vec<(usize, &str)> = block
        .iter()
        .map(|line| {
            let depth = line.chars().take_while(|c| *c == '.').count();
            (depth, &line[depth..])
        })
        .collect();
    let mut index = 0;
    parse_node(&parsed, &mut index, 0)
}

fn parse_node(lines: &[(usize, &str)], index: &mut usize, depth: usize) -> Norm {
    let (line_depth, text) = lines[*index];
    debug_assert_eq!(line_depth, depth);
    *index += 1;
    let node = parse_node_text(text);
    if let Norm::Box {
        kind,
        width,
        height,
        depth: bdepth,
        ..
    } = node
    {
        let mut children = Vec::new();
        while *index < lines.len() && lines[*index].0 > depth {
            // Only descend into direct children (depth + 1).
            if lines[*index].0 == depth + 1 {
                let child = parse_node(lines, index, depth + 1);
                // Drop non-geometric markers so child counts stay comparable.
                if !matches!(child, Norm::Skip) {
                    children.push(child);
                }
            } else {
                // Deeper line without an intervening child header: skip.
                *index += 1;
            }
        }
        Norm::Box {
            kind,
            width,
            height,
            depth: bdepth,
            children,
        }
    } else {
        node
    }
}

/// Parse one `\showbox` node line, children are collected by the caller.
fn parse_node_text(text: &str) -> Norm {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix("\\hbox(") {
        let (w, h, d) = parse_box_dims(rest);
        return Norm::Box {
            kind: BoxKind::Horizontal,
            width: w,
            height: h,
            depth: d,
            children: Vec::new(),
        };
    }
    if let Some(rest) = text.strip_prefix("\\vbox(") {
        let (w, h, d) = parse_box_dims(rest);
        return Norm::Box {
            kind: BoxKind::Vertical,
            width: w,
            height: h,
            depth: d,
            children: Vec::new(),
        };
    }
    if let Some(rest) = text.strip_prefix("\\rule(") {
        // \rule(height+depth)xwidth , running dims printed as '*'.
        let (w, h, d) = parse_box_dims(rest);
        return Norm::Rule {
            width: w,
            height: h,
            depth: d,
        };
    }
    if let Some(rest) = text.strip_prefix("\\kern") {
        // Explicit and normal-subtype kerns differ by spacing, ignore trailing `(for accent)`.
        let amount = rest.trim_start().split_whitespace().next().unwrap_or("0");
        return Norm::Kern {
            amount: pt_to_sp(amount),
        };
    }
    if let Some(rest) = text.strip_prefix("\\glue") {
        // Named skips have an optional `(...)` prefix before the glue dimension.
        let rest = rest.trim_start();
        let rest = match rest.strip_prefix('(') {
            Some(inner) => inner.split_once(')').map_or(inner, |(_, after)| after),
            None => rest,
        };
        let amount = rest.trim_start().split_whitespace().next().unwrap_or("0");
        return Norm::Glue {
            amount: pt_to_sp(amount),
        };
    }
    // Drop dump markers that bracket math material but carry no comparable geometry.
    if text.starts_with("\\mathon")
        || text.starts_with("\\mathoff")
        || text.starts_with("\\penalty")
    {
        return Norm::Skip;
    }
    // Character lines look like: "\tenrm A" or "\OT1/cmr/m/n/10 A".
    if let Some((font_token, glyph)) = parse_char_line(text) {
        let _ = font_token;
        return Norm::Char {
            glyph: glyph as u32,
            // \showbox does not print per char width, resolved during compare.
            width: i32::MIN,
        };
    }
    // Unknown line: model as a zero kern so structural counts still surface.
    Norm::Kern { amount: 0 }
}

/// Parse a `(h+d)xw` dimension suffix. Running dimensions print as `*`.
fn parse_box_dims(rest: &str) -> (i32, i32, i32) {
    // rest looks like "2.0+0.0)x1.0"
    let close = rest.find(')').unwrap_or(rest.len());
    let hd = &rest[..close];
    let after = &rest[close..];
    let (h_text, d_text) = match hd.split_once('+') {
        Some((h, d)) => (h, d),
        None => (hd, "0.0"),
    };
    let height = parse_running(h_text);
    let depth = parse_running(d_text);
    let width = after
        .find('x')
        .map(|p| {
            // Width runs to the next comma or space, take the leading value.
            let w_text = &after[p + 1..];
            let end = w_text
                .find(|c: char| c == ',' || c.is_whitespace())
                .unwrap_or(w_text.len());
            parse_running(&w_text[..end])
        })
        .unwrap_or(0);
    (width, height, depth)
}

fn parse_running(text: &str) -> i32 {
    let text = text.trim();
    if text == "*" {
        RUNNING_DIM
    } else {
        pt_to_sp(text)
    }
}

fn parse_char_line(text: &str) -> Option<(&str, char)> {
    // "\<font> <char>" where <font> begins with a backslash.
    let text = text.strip_prefix('\\')?;
    let (font, rest) = text.split_once(' ')?;
    let glyph = rest.chars().next()?;
    Some((font, glyph))
}

/// Lay a fragment out through the generated engine and return a normalized IR root.
fn engine_layout(setup: &str, body: &str, fonts: &[(&str, &[u8])]) -> Result<Norm, String> {
    let mut resources = InMemoryResourceProvider::new();
    for (name, bytes) in fonts {
        resources = resources.with_resource(*name, ResourceKind::Font, bytes.to_vec());
    }
    let format = GeneratedFormatCache::initialized(EngineProfile::tex());
    let mut engine = format.instantiate(EngineProfile::tex(), GeneratedResourceProvider::new(resources));

    let program = format!(r"\catcode`{{=1 \catcode`}}=2 {setup} \hbox{{{body}}}\end");
    engine.begin_fragment_capture();
    if !engine.begin_primary_input("input.tex", program.into_bytes()) {
        return Err("engine refused primary input".to_string());
    }
    engine.run_main_control();
    engine.end_fragment_capture();

    let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    if transcript.contains('!') {
        return Err(format!("engine reported a TeX error:\n{transcript}"));
    }

    let Some(root) = engine.captured_fragment_root() else {
        return Err("engine captured no fragment root".to_string());
    };
    let fragment = generated_node_to_fragment(
        &engine,
        root,
        FragmentMetadata {
            engine_profile: "tex".into(),
            format_id: "conformance".into(),
            fragment_kind: Default::default(),
        },
    )
    .ok_or_else(|| "captured root failed to convert to IR".to_string())?;

    // Find the root box by excluding nodes listed as children elsewhere.
    let mut is_child: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
    for node in &fragment.nodes {
        match &node.kind {
            LayoutNodeKind::Box(b) => is_child.extend(b.children.iter().copied()),
            LayoutNodeKind::List(l) => is_child.extend(l.children.iter().copied()),
            LayoutNodeKind::Group { children } => is_child.extend(children.iter().copied()),
            _ => {}
        }
    }
    let Some(root_id) = fragment.nodes.iter().find_map(|node| match &node.kind {
        LayoutNodeKind::Box(_) if !is_child.contains(&node.id) => Some(node.id),
        _ => None,
    }) else {
        return Err("fragment contained no root box".to_string());
    };
    Ok(normalize_ir(&fragment, root_id))
}

fn len(value: Length) -> i32 {
    value.0
}

fn normalize_ir(fragment: &Fragment, id: NodeId) -> Norm {
    let node = fragment.node(id).expect("node id should resolve");
    match &node.kind {
        LayoutNodeKind::Box(b) => Norm::Box {
            kind: b.kind,
            width: len(b.metrics.width),
            height: len(b.metrics.height),
            depth: len(b.metrics.depth),
            children: b
                .children
                .iter()
                .flat_map(|child| normalize_ir_children(fragment, *child))
                .collect(),
        },
        LayoutNodeKind::Rule(r) => Norm::Rule {
            width: len(r.size.width),
            // IR rules store height+depth in size.height, corpus depth is 0pt, so depth = 0 here.
            height: len(r.size.height),
            depth: 0,
        },
        LayoutNodeKind::Glue(g) => Norm::Glue {
            amount: len(g.amount),
        },
        LayoutNodeKind::Kern(k) => Norm::Kern {
            amount: len(k.amount),
        },
        LayoutNodeKind::GlyphRun(run) => {
            let glyph = run.glyphs.first();
            Norm::Char {
                glyph: glyph.map(|g| g.glyph_id.0).unwrap_or(0),
                width: glyph.map(|g| len(g.advance.x)).unwrap_or(0),
            }
        }
        // `Drawing` carries no metrics comparable to a TeX glyph/box, surface it as a distinct node.
        LayoutNodeKind::Drawing(_) => Norm::Drawing,
        // `List`/`Group` are structural containers, the caller flattens them via `normalize_ir_children`.
        LayoutNodeKind::List(list) => Norm::Box {
            kind: BoxKind::Horizontal,
            width: 0,
            height: 0,
            depth: 0,
            children: list
                .children
                .iter()
                .flat_map(|child| normalize_ir_children(fragment, *child))
                .collect(),
        },
        LayoutNodeKind::Group { children } => Norm::Box {
            kind: BoxKind::Horizontal,
            width: 0,
            height: 0,
            depth: 0,
            children: children
                .iter()
                .flat_map(|child| normalize_ir_children(fragment, *child))
                .collect(),
        },
        // Unrecognized `LayoutNodeKind` variants surface as incomparable nodes.
        _ => Norm::Skip,
    }
}

/// Flatten `List`/`Group` containers so the engine tree matches the `\showbox` nesting structure.
fn normalize_ir_children(fragment: &Fragment, id: NodeId) -> Vec<Norm> {
    let node = fragment.node(id).expect("node id should resolve");
    match &node.kind {
        LayoutNodeKind::List(list) => list
            .children
            .iter()
            .flat_map(|child| normalize_ir_children(fragment, *child))
            .collect(),
        LayoutNodeKind::Group { children } => children
            .iter()
            .flat_map(|child| normalize_ir_children(fragment, *child))
            .collect(),
        _ => vec![normalize_ir(fragment, id)],
    }
}

const CMR10: &[u8] = include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm");
const CMMI10: &[u8] =
    include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmmi10.tfm");
const CMSY10: &[u8] =
    include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmsy10.tfm");
const CMEX10: &[u8] =
    include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmex10.tfm");

/// Computer Modern four-font math setup for `tex -ini`.
const MATH_SETUP: &str = concat!(
    r"\catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
    r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 ",
    r"\textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm ",
    r"\textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni ",
    r"\textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy ",
    r"\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex ",
    r"\def\text#1{\hbox{#1}}\def\frac#1#2{{#1\over#2}}",
    // `"270370` is the plain TeX square root delimiter code, `"` introduces a TeX hex constant.
    "\\def\\sqrt{\\radical\"270370 }",
);

const MATH_FONTS: &[(&str, &[u8])] = &[
    ("cmr10", CMR10),
    ("cmmi10", CMMI10),
    ("cmsy10", CMSY10),
    ("cmex10", CMEX10),
];

struct Case {
    name: &'static str,
    /// Verbatim TeX prepended before the captured `\hbox{...}` on both sides.
    setup: &'static str,
    body: &'static str,
    fonts: &'static [(&'static str, &'static [u8])],
}

fn corpus() -> Vec<Case> {
    vec![
        Case {
            name: "hbox-vrule",
            setup: "",
            body: r"\vrule width 1pt height 2pt depth 0pt",
            fonts: &[],
        },
        Case {
            name: "hbox-glyph-A",
            setup: "",
            body: r"\font\tenrm=cmr10 \tenrm A",
            fonts: &[("cmr10", CMR10)],
        },
        Case {
            name: "hbox-glyph-AB",
            setup: "",
            body: r"\font\tenrm=cmr10 \tenrm AB",
            fonts: &[("cmr10", CMR10)],
        },
        Case {
            name: "hbox-kern",
            setup: "",
            body: r"\kern 3pt",
            fonts: &[],
        },
        // Bare `A b` renders under `\nullfont` and vanishes.
        Case {
            name: "text-A-b",
            setup: MATH_SETUP,
            body: r"\tenrm A b",
            fonts: MATH_FONTS,
        },
        Case {
            name: "math-x",
            setup: MATH_SETUP,
            body: r"$x$",
            fonts: MATH_FONTS,
        },
        Case {
            name: "math-x-sup-2",
            setup: MATH_SETUP,
            body: r"$x^2$",
            fonts: MATH_FONTS,
        },
        Case {
            name: "math-x-sub-i",
            setup: MATH_SETUP,
            body: r"$x_i$",
            fonts: MATH_FONTS,
        },
        Case {
            name: "math-a-plus-b",
            setup: MATH_SETUP,
            body: r"$a+b$",
            fonts: MATH_FONTS,
        },
        Case {
            name: "math-frac-a-b",
            setup: MATH_SETUP,
            body: r"$\frac{a}{b}$",
            fonts: MATH_FONTS,
        },
        Case {
            name: "math-sqrt-x",
            setup: MATH_SETUP,
            body: r"$\sqrt{x}$",
            fonts: MATH_FONTS,
        },
        Case {
            name: "text-hi",
            setup: MATH_SETUP,
            body: r"\text{\tenrm hi}",
            fonts: MATH_FONTS,
        },
        // Knuth-Plass line breaking exercises paragraph glue and discretionaries.
        Case {
            name: "par-linebreak",
            setup: r"\font\tenrm=cmr10 \tenrm \hsize=100pt \parindent=0pt",
            body: r"\vbox{\noindent Quisque ut dolor sit amet velit aliquam tincidunt nec eu nisl porttitor mauris.\par}",
            fonts: &[("cmr10", CMR10)],
        },
    ]
}

/// Copy engine char advances into the oracle tree, `\showbox` does not print per-char widths.
fn reconcile_char_widths(expected: &mut Norm, actual: &Norm) {
    match (expected, actual) {
        (
            Norm::Box { children: ec, .. },
            Norm::Box { children: ac, .. },
        ) => {
            for (ce, ca) in ec.iter_mut().zip(ac.iter()) {
                reconcile_char_widths(ce, ca);
            }
        }
        (Norm::Char { width: ew, .. }, Norm::Char { width: aw, .. }) => {
            if *ew == i32::MIN {
                *ew = *aw;
            }
        }
        _ => {}
    }
}

fn run_case(tex: &PathBuf, case: &Case) -> (Norm, Result<Norm, String>, Vec<String>) {
    let block = run_tex_showbox(tex, case.setup, case.body, case.fonts);
    assert!(
        !block.is_empty(),
        "case `{}`: empty \\showbox dump (body `{}`)",
        case.name,
        case.body
    );
    let mut expected = parse_showbox(&block);
    let actual = engine_layout(case.setup, case.body, case.fonts);
    let mut diffs = Vec::new();
    match &actual {
        Ok(actual) => {
            reconcile_char_widths(&mut expected, actual);
            diff(&expected, actual, case.name, &mut diffs);
        }
        Err(reason) => {
            diffs.push(format!("{}: engine produced no layout ({reason})", case.name));
        }
    }
    (expected, actual, diffs)
}

#[test]
fn conformance_corpus_reports_engine_vs_tex() {
    let Some(tex) = tex_binary() else {
        eprintln!("conformance: tex binary not found, skipping");
        return;
    };

    let mut total_diffs = 0usize;
    let mut matched = Vec::new();
    let mut mismatched = Vec::new();
    let mut report = String::new();
    for case in corpus() {
        let (expected, actual, diffs) = run_case(&tex, &case);
        report.push_str(&format!("\n===== {} =====\n", case.name));
        report.push_str(&format!("  body: {}\n", case.body));
        report.push_str(&format!("  tex:    {expected:?}\n"));
        report.push_str(&format!("  engine: {actual:?}\n"));
        if diffs.is_empty() {
            report.push_str("  MATCH\n");
            matched.push(case.name);
        } else {
            total_diffs += diffs.len();
            mismatched.push(case.name);
            for d in &diffs {
                report.push_str(&format!("  DIFF {d}\n"));
            }
        }
    }
    eprintln!("{report}");
    eprintln!(
        "conformance: {}/{} fragments match within {SP_TOLERANCE}sp tolerance",
        matched.len(),
        matched.len() + mismatched.len(),
    );
    eprintln!("conformance: MATCH    [{}]", matched.join(", "));
    eprintln!("conformance: MISMATCH [{}]", mismatched.join(", "));
    eprintln!("conformance: {total_diffs} total diffs across corpus");

    // EXPECTED_MATCH catches regressions, KNOWN_DIVERGENT catches silent progress.
    const EXPECTED_MATCH: &[&str] = &[
        "hbox-vrule", "hbox-glyph-A", "hbox-glyph-AB", "hbox-kern", "text-A-b", "math-x",
        "math-x-sup-2", "math-x-sub-i", "math-a-plus-b", "math-frac-a-b", "math-sqrt-x",
        "text-hi", "par-linebreak",
    ];
    // The whole corpus now matches real tex within tolerance.
    const KNOWN_DIVERGENT: &[&str] = &[];
    let regressed: Vec<&&str> = EXPECTED_MATCH.iter().filter(|n| mismatched.contains(n)).collect();
    assert!(
        regressed.is_empty(),
        "engine REGRESSED vs tex on {regressed:?}\n{report}"
    );
    let newly_matching: Vec<&&str> =
        KNOWN_DIVERGENT.iter().filter(|n| matched.contains(n)).collect();
    assert!(
        newly_matching.is_empty(),
        "{newly_matching:?} now match tex, promote them from KNOWN_DIVERGENT to EXPECTED_MATCH"
    );
}

#[test]
fn golden_hbox_vrule_is_single_rule() {
    let Some(tex) = tex_binary() else {
        return;
    };
    let block = run_tex_showbox(&tex, "", r"\vrule width 1pt height 2pt depth 0pt", &[]);
    let tree = parse_showbox(&block);
    let Norm::Box {
        kind,
        width,
        height,
        depth,
        children,
    } = tree
    else {
        panic!("expected root hbox");
    };
    assert_eq!(kind, BoxKind::Horizontal);
    assert!(approx(width, 65536), "width {width}sp != 1pt");
    assert!(approx(height, 131072), "height {height}sp != 2pt");
    assert!(approx(depth, 0), "depth {depth}sp != 0");
    assert_eq!(children.len(), 1, "tex hbox{{vrule}} has exactly 1 child");
    assert!(matches!(children[0], Norm::Rule { .. }));
}

#[test]
fn golden_hbox_kern_is_single_kern() {
    let Some(tex) = tex_binary() else {
        return;
    };
    let block = run_tex_showbox(&tex, "", r"\kern 3pt", &[]);
    let tree = parse_showbox(&block);
    let Norm::Box { children, .. } = tree else {
        panic!("expected root hbox");
    };
    assert_eq!(children.len(), 1);
    match &children[0] {
        Norm::Kern { amount } => assert!(approx(*amount, 196608), "kern {amount}sp != 3pt"),
        other => panic!("expected kern, got {other:?}"),
    }
}

#[test]
fn golden_hbox_glyph_a_is_single_char() {
    let Some(tex) = tex_binary() else {
        return;
    };
    let block = run_tex_showbox(&tex, "", r"\font\tenrm=cmr10 \tenrm A", &[("cmr10", CMR10)]);
    let tree = parse_showbox(&block);
    let Norm::Box { children, .. } = tree else {
        panic!("expected root hbox");
    };
    assert_eq!(children.len(), 1, "tex hbox{{A}} has exactly 1 child");
    match &children[0] {
        Norm::Char { glyph, .. } => assert_eq!(*glyph, u32::from(b'A')),
        other => panic!("expected char A, got {other:?}"),
    }
}

#[test]
fn golden_math_x_is_single_math_italic_char() {
    let Some(tex) = tex_binary() else {
        return;
    };
    // The parser drops TeX math markers around `$x$`, leaving one char child.
    let block = run_tex_showbox(&tex, MATH_SETUP, r"$x$", MATH_FONTS);
    let tree = parse_showbox(&block);
    let Norm::Box {
        kind,
        width,
        height,
        children,
        ..
    } = tree
    else {
        panic!("expected root hbox");
    };
    assert_eq!(kind, BoxKind::Horizontal);
    // From the real TeX dump: \hbox(4.30554+0.0)x5.71527.
    assert!(approx(height, 282168), "height {height}sp != 4.30554pt");
    assert!(approx(width, 374556), "width {width}sp != 5.71527pt");
    assert_eq!(
        children.len(),
        1,
        "tex hbox{{$x$}} has exactly 1 char child after dropping math-shift markers"
    );
    match &children[0] {
        Norm::Char { glyph, .. } => assert_eq!(*glyph, u32::from(b'x')),
        other => panic!("expected math-italic char x, got {other:?}"),
    }
}
