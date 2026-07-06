//! Compares engine layout IR against live `xetex` `\showbox` dumps.

use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use mathtex_engine::font::{
    FontData, FontError, FontQuery, FontSystem, RustybuzzFontSystem, ShapeRequest, ShapedText,
};
use mathtex_engine::generated::generated_node_to_fragment;
use mathtex_engine::{
    portable_engine::EngineProfile, GeneratedFontSystemAdapter, GeneratedFormatCache,
    GeneratedResourceProvider, InMemoryResourceProvider, ResourceFontSystem, ResourceKind,
};
use mathtex_ir::{BoxKind, Fragment, FragmentMetadata, LayoutNodeKind, Length, NodeId};

/// Native glyph metrics reproduce XeTeX scaled point arithmetic exactly.
const SP_TOLERANCE: i32 = 0;

const XETEX_CANDIDATES: &[&str] = &["/Library/TeX/texbin/xetex", "xetex"];

/// OTF filename (as xetex resolves it) and on disk path candidates for the engine side.
#[derive(Clone, Copy)]
struct MathFont {
    /// OTF filename, e.g. `latinmodern-math.otf`.
    file: &'static str,
    /// Absolute path candidates, the case is skipped if none exist.
    paths: &'static [&'static str],
}

const LM_MATH: MathFont = MathFont {
    file: "latinmodern-math.otf",
    paths: &["/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/lm-math/latinmodern-math.otf"],
};

const STIX_MATH: MathFont = MathFont {
    file: "STIXTwoMath-Regular.otf",
    paths: &[
        "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/stix2-otf/STIXTwoMath-Regular.otf",
    ],
};

fn math_setup(font: &MathFont, extra: &str) -> String {
    format!(
        concat!(
            r"\catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
            "\\font\\mf=\"[{file}]:script=math\" at 10pt ",
            r"\textfont0=\mf \scriptfont0=\mf \scriptscriptfont0=\mf ",
            r"\textfont1=\mf \scriptfont1=\mf \scriptscriptfont1=\mf ",
            r"\textfont2=\mf \scriptfont2=\mf \scriptscriptfont2=\mf ",
            r"\textfont3=\mf \scriptfont3=\mf \scriptscriptfont3=\mf ",
            "{extra} ",
        ),
        file = font.file,
        extra = extra,
    )
}

// `"` is a TeX hex-digit prefix (`"0028` = U+0028), not a Rust string delimiter.
const DELIM_SETUP: &str = concat!(
    "\\Udelcode`\\(=\"0 \"0028 \\Udelcode`\\)=\"0 \"0029 ",
    r"\def\frac#1#2{{#1\over#2}}",
    " \\def\\sqrt{\\Uradical\"0 \"221A }",
);

#[derive(Clone, Debug, PartialEq)]
enum Norm {
    Box {
        kind: BoxKind,
        width: i32,
        height: i32,
        depth: i32,
        children: Vec<Norm>,
    },
    Kern {
        amount: i32,
    },
    Glue {
        amount: i32,
    },
    /// Native glyph with engine-resolved id and advance width.
    Glyph {
        glyph: u32,
        width: i32,
    },
    /// Rule node (e.g. fraction or radical bar). Running dimensions printed as `*` are not compared.
    Rule {
        width: i32,
        height: i32,
        depth: i32,
    },
    /// Marker node without geometry (`\mathon`, `\mathoff`, `\penalty`): filtered out before diffing.
    Skip,
}

impl Norm {
    fn label(&self) -> &'static str {
        match self {
            Norm::Box { .. } => "box",
            Norm::Kern { .. } => "kern",
            Norm::Glue { .. } => "glue",
            Norm::Glyph { .. } => "glyph",
            Norm::Rule { .. } => "rule",
            Norm::Skip => "skip",
        }
    }
}

/// Sentinel for TeX running/default dimensions (`*` in `\showbox`), value is -(2^30).
const RUNNING_DIM: i32 = -1_073_741_824;

fn pt_to_sp(text: &str) -> i32 {
    let text = text.trim();
    let negative = text.starts_with('-');
    let text = text.trim_start_matches(['+', '-']);
    let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
    let int_val: i64 = int_part.parse().unwrap_or(0);
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

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn run_xetex_showbox(xetex: &str, setup: &str, body: &str) -> Vec<String> {
    let dir = std::env::temp_dir().join(format!(
        "mtx-xetex-native-conf-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let job = "frag";
    let tex_path = dir.join(format!("{job}.tex"));
    let program = format!(
        concat!(
            r"\catcode`\{{=1 \catcode`\}}=2 ",
            r"\tracingonline=1 \showboxdepth=2147483647 \showboxbreadth=2147483647 ",
            "{setup} ",
            r"\setbox0=\hbox{{{{{body}}}}}\showbox0 \end",
            "\n"
        ),
        setup = setup,
        body = body,
    );
    std::fs::write(&tex_path, program).expect("write tex source");

    let _output = Command::new(xetex)
        .current_dir(&dir)
        .env("TEXMFOUTPUT", &dir)
        .arg("-ini")
        .arg("-interaction=batchmode")
        .arg(format!("{job}.tex"))
        .output()
        .expect("run xetex");

    let log = std::fs::read_to_string(dir.join(format!("{job}.log"))).expect("read xetex log");
    let block = extract_showbox(&log);
    let _ = std::fs::remove_dir_all(&dir);
    block
}

fn extract_showbox(log: &str) -> Vec<String> {
    let mut lines = log.lines();
    for line in lines.by_ref() {
        if line.starts_with("> \\box0=") {
            break;
        }
    }
    let mut block = Vec::new();
    for line in lines {
        if line.trim().is_empty() || line.starts_with("! OK.") || line.starts_with('!') {
            break;
        }
        block.push(line.to_string());
    }
    block
}

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
    let (_line_depth, text) = lines[*index];
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
            if lines[*index].0 == depth + 1 {
                let child = parse_node(lines, index, depth + 1);
                if !matches!(child, Norm::Skip) {
                    children.push(child);
                }
            } else {
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
        // Format: `\rule(height+depth)xwidth`, running dims print as `*`.
        let (w, h, d) = parse_box_dims(rest);
        return Norm::Rule {
            width: w,
            height: h,
            depth: d,
        };
    }
    if let Some(rest) = text.strip_prefix("\\kern") {
        // Normal kerns print `\kern0.16` (no space), explicit kerns print `\kern 3.0`.
        let amount = rest.split_whitespace().next().unwrap_or("0");
        return Norm::Kern {
            amount: pt_to_sp(amount),
        };
    }
    if let Some(rest) = text.strip_prefix("\\glue") {
        // Plain `\glue 3.0 plus 1.0` or named `\glue(\thinmuskip) 1.7 ...`.
        let rest = rest.trim_start();
        let rest = match rest.strip_prefix('(') {
            Some(inner) => inner.split_once(')').map_or(inner, |(_, after)| after),
            None => rest,
        };
        let amount = rest.split_whitespace().next().unwrap_or("0");
        return Norm::Glue {
            amount: pt_to_sp(amount),
        };
    }
    if text.starts_with("\\mathon")
        || text.starts_with("\\mathoff")
        || text.starts_with("\\penalty")
    {
        return Norm::Skip;
    }
    // Native glyph lines look like `\mf glyph#89`, XeTeX does not print the advance width.
    if let Some(glyph) = parse_native_glyph_line(text) {
        return Norm::Glyph {
            glyph,
            width: i32::MIN,
        };
    }
    // Unknown lines are modeled as a zero kern so structural child counts surface in diffs.
    Norm::Kern { amount: 0 }
}

fn parse_native_glyph_line(text: &str) -> Option<u32> {
    let text = text.strip_prefix('\\')?;
    let (_font, rest) = text.split_once(' ')?;
    let num = rest.strip_prefix("glyph#")?;
    let end = num.find(|c: char| !c.is_ascii_digit()).unwrap_or(num.len());
    num[..end].parse::<u32>().ok()
}

fn parse_running(text: &str) -> i32 {
    let text = text.trim();
    if text == "*" {
        RUNNING_DIM
    } else {
        pt_to_sp(text)
    }
}

fn parse_box_dims(rest: &str) -> (i32, i32, i32) {
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
            let w_text = &after[p + 1..];
            let end = w_text
                .find(|c: char| c == ',' || c.is_whitespace())
                .unwrap_or(w_text.len());
            parse_running(&w_text[..end])
        })
        .unwrap_or(0);
    (width, height, depth)
}

struct NativeFontSystem {
    inner: RustybuzzFontSystem<ResourceFontSystem<InMemoryResourceProvider>>,
    /// OTF filename the font is registered under in the resource provider.
    canonical: String,
    queries: Rc<RefCell<Vec<String>>>,
}

impl NativeFontSystem {
    fn new(file: &str, font_bytes: Vec<u8>, queries: Rc<RefCell<Vec<String>>>) -> Self {
        let mut resources = InMemoryResourceProvider::new();
        // Register under the bare OTF filename, bracket queries are normalized in `load_font`.
        resources = resources.with_resource(file, ResourceKind::Font, font_bytes);
        Self {
            inner: RustybuzzFontSystem::new(ResourceFontSystem::new(resources)),
            canonical: file.to_string(),
            queries,
        }
    }
}

impl FontSystem for NativeFontSystem {
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        self.queries.borrow_mut().push(query.family.clone());
        if let Ok(font) = self.inner.load_font(query) {
            return Ok(font);
        }
        // Normalize bracket queries naming the font stem to the canonical filename.
        let stem = self
            .canonical
            .strip_suffix(".otf")
            .unwrap_or(&self.canonical)
            .to_ascii_lowercase();
        if query.family.to_ascii_lowercase().contains(&stem) {
            let mut q = query.clone();
            q.family = self.canonical.clone();
            return self.inner.load_font(&q);
        }
        self.inner.load_font(query)
    }

    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        self.inner.shape_text(request)
    }
}

fn len(value: Length) -> i32 {
    value.0
}

fn engine_layout_native(
    font: &MathFont,
    font_path: &str,
    setup: &str,
    body: &str,
) -> Result<Norm, String> {
    let font_bytes = std::fs::read(font_path).map_err(|e| format!("read otf: {e}"))?;
    let queries: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let font_system = NativeFontSystem::new(font.file, font_bytes, Rc::clone(&queries));

    let format = GeneratedFormatCache::initialized(EngineProfile::xetex());
    let resources = GeneratedResourceProvider::new(InMemoryResourceProvider::new());
    let mut engine = format
        .instantiate(EngineProfile::xetex(), resources)
        .with_font_platform(GeneratedFontSystemAdapter::new(font_system));

    let program = format!(r"\catcode`{{=1 \catcode`}}=2 {setup} \hbox{{{{{body}}}}}\end");
    engine.begin_fragment_capture();
    if !engine.begin_primary_input("input.tex", program.into_bytes()) {
        return Err("engine refused primary input".to_string());
    }
    engine.run_main_control();
    engine.end_fragment_capture();

    let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    let queries = queries.borrow().clone();
    if transcript.contains('!') {
        return Err(format!(
            "engine reported a TeX error:\n{transcript}\n--- font queries: {queries:?}"
        ));
    }

    let Some(root) = engine.captured_fragment_root() else {
        return Err(format!(
            "engine captured no fragment root (font queries: {queries:?})"
        ));
    };
    let fragment = generated_node_to_fragment(
        &engine,
        root,
        FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "conformance-native".into(),
            fragment_kind: Default::default(),
        },
    )
    .ok_or_else(|| "captured root failed to convert to IR".to_string())?;

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
        LayoutNodeKind::Kern(k) => Norm::Kern {
            amount: len(k.amount),
        },
        LayoutNodeKind::Glue(g) => Norm::Glue {
            amount: len(g.amount),
        },
        LayoutNodeKind::Rule(r) => Norm::Rule {
            width: len(r.size.width),
            // IR rules store height+depth combined in size.height, corpus rules have depth 0.
            height: len(r.size.height),
            depth: 0,
        },
        LayoutNodeKind::GlyphRun(run) => {
            let glyph = run.glyphs.first();
            Norm::Glyph {
                glyph: glyph.map(|g| g.glyph_id.0).unwrap_or(0),
                width: glyph.map(|g| len(g.advance.x)).unwrap_or(0),
            }
        }
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
        _ => Norm::Skip,
    }
}

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

fn approx(a: i32, b: i32) -> bool {
    (i64::from(a) - i64::from(b)).abs() <= i64::from(SP_TOLERANCE)
}

/// Fills in glyph advance widths from the engine tree into the oracle tree, which lacks them.
fn reconcile_glyph_widths(expected: &mut Norm, actual: &Norm) {
    match (expected, actual) {
        (Norm::Box { children: ec, .. }, Norm::Box { children: ac, .. }) => {
            for (ce, ca) in ec.iter_mut().zip(ac.iter()) {
                reconcile_glyph_widths(ce, ca);
            }
        }
        (Norm::Glyph { width: ew, .. }, Norm::Glyph { width: aw, .. }) if *ew == i32::MIN => {
            *ew = *aw;
        }
        _ => {}
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
                out.push(format!("{path}: box kind {ek:?} != {ak:?}"));
            }
            if !approx(*ew, *aw) {
                out.push(format!("{path}: width {ew} != {aw} (Δ{})", aw - ew));
            }
            if !approx(*eh, *ah) {
                out.push(format!("{path}: height {eh} != {ah} (Δ{})", ah - eh));
            }
            if !approx(*ed, *ad) {
                out.push(format!("{path}: depth {ed} != {ad} (Δ{})", ad - ed));
            }
            if ec.len() != ac.len() {
                out.push(format!(
                    "{path}: child count {} != {} (expected [{}], actual [{}])",
                    ec.len(),
                    ac.len(),
                    ec.iter().map(Norm::label).collect::<Vec<_>>().join(","),
                    ac.iter().map(Norm::label).collect::<Vec<_>>().join(","),
                ));
            }
            for (i, (ce, ca)) in ec.iter().zip(ac.iter()).enumerate() {
                diff(ce, ca, &format!("{path}/{i}"), out);
            }
        }
        (Norm::Glyph { glyph: eg, width: ewd }, Norm::Glyph { glyph: ag, width: awd }) => {
            if eg != ag {
                out.push(format!("{path}: glyph id {eg} != {ag}"));
            }
            if !approx(*ewd, *awd) {
                out.push(format!("{path}: glyph width {ewd} != {awd} (Δ{})", awd - ewd));
            }
        }
        (Norm::Kern { amount: e }, Norm::Kern { amount: a }) => {
            if !approx(*e, *a) {
                out.push(format!("{path}: kern {e} != {a} (Δ{})", a - e));
            }
        }
        (Norm::Glue { amount: e }, Norm::Glue { amount: a }) => {
            if !approx(*e, *a) {
                out.push(format!("{path}: glue {e} != {a} (Δ{})", a - e));
            }
        }
        (
            Norm::Rule { width: ew, height: eh, depth: ed },
            Norm::Rule { width: aw, height: ah, depth: ad },
        ) => {
            // Running dimensions (`*`) are context dependent, skip comparison on either side.
            let cmp = |e: i32, a: i32| e == RUNNING_DIM || a == RUNNING_DIM || approx(e, a);
            if !cmp(*ew, *aw) {
                out.push(format!("{path}: rule width {ew} != {aw} (Δ{})", aw - ew));
            }
            if !cmp(*eh, *ah) {
                out.push(format!("{path}: rule height {eh} != {ah} (Δ{})", ah - eh));
            }
            if !cmp(*ed, *ad) {
                out.push(format!("{path}: rule depth {ed} != {ad} (Δ{})", ad - ed));
            }
        }
        (e, a) => {
            if e != a {
                out.push(format!("{path}: {} != {}", e.label(), a.label()));
            }
        }
    }
}

struct Case {
    name: &'static str,
    font: MathFont,
    /// Per case extra setup (delimiter codes, `\frac`), empty for plain glyph cases.
    extra: &'static str,
    body: &'static str,
}

fn corpus() -> Vec<Case> {
    vec![
        Case { name: "native-x", font: LM_MATH, extra: "", body: r"$x$" },
        Case { name: "native-x-sup-2", font: LM_MATH, extra: "", body: r"$x^2$" },
        Case { name: "native-x-sub-i", font: LM_MATH, extra: "", body: r"$x_i$" },
        Case { name: "native-x-sub-i-sup-2", font: LM_MATH, extra: "", body: r"$x_i^2$" },
        Case { name: "native-a-plus-b", font: LM_MATH, extra: "", body: r"$a+b$" },
        // STIX MathKernInfo cut-ins produce visible `\kern` nodes, latinmodern-math has none.
        Case { name: "kern-stix-F-sup-2", font: STIX_MATH, extra: "", body: r"$F^2$" },
        // `V` subscript: negative BottomRight cut-in, prints as `\kern-1.18999`.
        Case { name: "kern-stix-V-sub-x", font: STIX_MATH, extra: "", body: r"$V_x$" },
        // `T` subscript: negative BottomRight cut-in, prints as `\kern-0.89998`.
        Case { name: "kern-stix-T-sub-x", font: STIX_MATH, extra: "", body: r"$T_x$" },
        Case { name: "kern-stix-V-sub-x-sup-2", font: STIX_MATH, extra: "", body: r"$V_x^2$" },
        // Growable delimiter selects a larger single glyph variant rather than an assembly.
        Case {
            name: "variant-left-frac-right",
            font: LM_MATH,
            extra: DELIM_SETUP,
            body: r"$\left(\frac{a}{b}\right)$",
        },
        Case {
            name: "variant-sqrt-frac",
            font: LM_MATH,
            extra: DELIM_SETUP,
            body: r"$\sqrt{\frac{a}{b}}$",
        },
        // Fraction nest exceeds precomposed variants, forcing stacked delimiter parts.
        Case {
            name: "assembly-left-tall-right",
            font: LM_MATH,
            extra: DELIM_SETUP,
            body: r"$\left(\frac{\frac{\frac{a}{b}}{\frac{a}{b}}}{\frac{\frac{a}{b}}{\frac{a}{b}}}\right)$",
        },
    ]
}

fn first_existing(candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|p| PathBuf::from(p).exists())
        .map(|p| (*p).to_string())
}

fn xetex_binary() -> Option<String> {
    for candidate in XETEX_CANDIDATES {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some((*candidate).to_string());
        }
    }
    None
}

fn run_case(xetex: &str, case: &Case) -> (Norm, Result<Norm, String>, Vec<String>) {
    let Some(font_path) = first_existing(case.font.paths) else {
        return (
            Norm::Skip,
            Err(format!("font {} not found on disk", case.font.file)),
            vec![format!("{}: SKIP (font {} missing)", case.name, case.font.file)],
        );
    };
    let setup = math_setup(&case.font, case.extra);
    let block = run_xetex_showbox(xetex, &setup, case.body);
    assert!(
        !block.is_empty(),
        "case `{}`: empty xetex \\showbox dump (body `{}`)",
        case.name,
        case.body
    );
    let mut expected = parse_showbox(&block);
    let actual = engine_layout_native(&case.font, &font_path, &setup, case.body);
    let mut diffs = Vec::new();
    match &actual {
        Ok(actual) => {
            reconcile_glyph_widths(&mut expected, actual);
            diff(&expected, actual, case.name, &mut diffs);
        }
        Err(reason) => {
            diffs.push(format!("{}: engine produced no layout ({reason})", case.name));
        }
    }
    (expected, actual, diffs)
}

#[test]
fn native_conformance_corpus_reports_engine_vs_xetex() {
    let Some(xetex) = xetex_binary() else {
        eprintln!("native conformance: xetex binary not found, skipping");
        return;
    };

    let mut total_diffs = 0usize;
    let mut matched = Vec::new();
    let mut mismatched = Vec::new();
    let mut skipped = Vec::new();
    let mut report = String::new();
    for case in corpus() {
        if first_existing(case.font.paths).is_none() {
            eprintln!("native conformance: SKIP {} (font {} missing)", case.name, case.font.file);
            skipped.push(case.name);
            continue;
        }
        let (expected, actual, diffs) = run_case(&xetex, &case);
        report.push_str(&format!("\n===== {} ({}) =====\n", case.name, case.font.file));
        report.push_str(&format!("  body:   {}\n", case.body));
        report.push_str(&format!("  xetex:  {expected:?}\n"));
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
        "native conformance: {}/{} fragments match xetex within {SP_TOLERANCE}sp tolerance",
        matched.len(),
        matched.len() + mismatched.len(),
    );
    eprintln!("native conformance: MATCH    [{}]", matched.join(", "));
    eprintln!("native conformance: MISMATCH [{}]", mismatched.join(", "));
    eprintln!("native conformance: SKIPPED  [{}]", skipped.join(", "));
    eprintln!("native conformance: {total_diffs} total diffs across corpus");

    // EXPECTED_MATCH failures mean regression, KNOWN_DIVERGENT matches need promotion.
    const EXPECTED_MATCH: &[&str] = &[
        "native-x",
        "native-x-sup-2",
        "native-x-sub-i",
        "native-x-sub-i-sup-2",
        "native-a-plus-b",
        "kern-stix-F-sup-2",
        "kern-stix-V-sub-x",
        "kern-stix-T-sub-x",
        "kern-stix-V-sub-x-sup-2",
        "variant-left-frac-right",
        "variant-sqrt-frac",
        "assembly-left-tall-right",
    ];
    const KNOWN_DIVERGENT: &[&str] = &[];

    let regressed: Vec<&&str> = EXPECTED_MATCH
        .iter()
        .filter(|n| mismatched.contains(n))
        .collect();
    assert!(
        regressed.is_empty(),
        "engine REGRESSED vs xetex on {regressed:?}\n{report}"
    );
    let newly_matching: Vec<&&str> = KNOWN_DIVERGENT
        .iter()
        .filter(|n| matched.contains(n))
        .collect();
    assert!(
        newly_matching.is_empty(),
        "{newly_matching:?} now match xetex, promote them from KNOWN_DIVERGENT to EXPECTED_MATCH"
    );
}

#[test]
fn golden_native_x_is_glyph_plus_italic_kern() {
    let Some(xetex) = xetex_binary() else {
        return;
    };
    if first_existing(LM_MATH.paths).is_none() {
        return;
    }
    let block = run_xetex_showbox(&xetex, &math_setup(&LM_MATH, ""), r"$x$");
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
    // XeTeX output: `\hbox(4.31+0.0)x5.44`, glyph #89 followed by italic-correction `\kern0.16`.
    assert_eq!(width, 356516, "width != 5.44pt");
    assert_eq!(height, 282460, "height != 4.31pt");
    assert_eq!(depth, 0);
    assert_eq!(children.len(), 2, "x = glyph + italic kern");
    assert!(matches!(children[0], Norm::Glyph { glyph: 89, .. }));
    match &children[1] {
        Norm::Kern { amount } => assert_eq!(*amount, 10486, "italic kern != 0.16pt"),
        other => panic!("expected italic kern, got {other:?}"),
    }
}

#[test]
fn golden_stix_f_superscript_has_math_kern_cut_in() {
    let Some(xetex) = xetex_binary() else {
        return;
    };
    if first_existing(STIX_MATH.paths).is_none() {
        return;
    }
    // STIX glyph 8 has a constant TopRight superscript cut-in of `\kern0.44`.
    let block = run_xetex_showbox(&xetex, &math_setup(&STIX_MATH, ""), r"$F^2$");
    let Norm::Box { children, .. } = parse_showbox(&block) else {
        panic!("expected root hbox");
    };
    assert!(matches!(children[0], Norm::Glyph { glyph: 8, .. }), "base 'F'");
    match &children[1] {
        Norm::Kern { amount } => assert_eq!(*amount, pt_to_sp("0.44"), "math-kern cut-in != 0.44pt"),
        other => panic!("expected math-kern, got {other:?}"),
    }
}

#[test]
fn golden_left_frac_right_uses_paren_variant_glyphs() {
    let Some(xetex) = xetex_binary() else {
        return;
    };
    if first_existing(LM_MATH.paths).is_none() {
        return;
    }
    // latinmodern-math glyph 9 `(` selects its 5th vertical variant.
    let setup = math_setup(&LM_MATH, DELIM_SETUP);
    let block = run_xetex_showbox(&xetex, &setup, r"$\left(\frac{a}{b}\right)$");
    let dump = format!("{:?}", parse_showbox(&block));
    assert!(dump.contains("Glyph { glyph: 2433"), "left paren variant 2433 absent:\n{dump}");
    assert!(dump.contains("Glyph { glyph: 2434"), "right paren variant 2434 absent:\n{dump}");
}

#[test]
fn golden_tall_left_right_builds_paren_assembly() {
    let Some(xetex) = xetex_binary() else {
        return;
    };
    if first_existing(LM_MATH.paths).is_none() {
        return;
    }
    // Deep fraction nest forces delimiters past all precomposed variants into assemblies.
    let setup = math_setup(&LM_MATH, DELIM_SETUP);
    let block = run_xetex_showbox(
        &xetex,
        &setup,
        r"$\left(\frac{\frac{\frac{a}{b}}{\frac{a}{b}}}{\frac{\frac{a}{b}}{\frac{a}{b}}}\right)$",
    );
    let dump = format!("{:?}", parse_showbox(&block));
    for part in ["2503", "2504", "2505", "2506", "2507", "2508"] {
        assert!(
            dump.contains(&format!("Glyph {{ glyph: {part}")),
            "assembly part glyph {part} absent:\n{dump}"
        );
    }
}
