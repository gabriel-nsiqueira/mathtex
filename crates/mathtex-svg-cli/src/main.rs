//! CLI tool that lays out math through the XeTeX engine and writes SVG.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use mathtex_engine::font::{
    FontData, FontError, FontQuery, FontSystem, RustybuzzFontSystem, ShapeRequest, ShapedText,
};
use mathtex_engine::generated::generated_node_to_fragment;
use mathtex_engine::{
    portable_engine::EngineProfile, GeneratedFontSystemAdapter, GeneratedFormatCache,
    GeneratedResourceProvider, ProviderResourceRequest as ResourceRequest, Resource, ResourceError,
    ResourceFontSystem, ResourceKind, ResourceProvider, TexmfResources,
};
use mathtex_ir::{
    FontRef, Fragment, FragmentMetadata, GlyphId, GlyphOutline, LayoutNodeKind, NodeId,
};
use mathtex_render::GlyphOutlineSource;

/// OTF math font loaded through the XeTeX native `\font` path.
const FONT_FILE: &str = "latinmodern-math.otf";

/// Environment override for the TeXLive `texmf-dist` tree.
const TEXMF_ROOT_ENV: &str = "MATHTEX_TEXMF_ROOT";

const TEXMF_ROOT_CANDIDATES: &[&str] = &[
    "/usr/local/texlive/2026/texmf-dist",
    "/usr/local/texlive/2025/texmf-dist",
    "/Library/TeX/texmf-dist",
];

/// Resolves a XeTeX font spec to font file bytes from the texmf tree.
struct TexmfFontProvider {
    texmf: Arc<TexmfResources>,
}

impl TexmfFontProvider {
    /// Extracts the font filename from a XeTeX font spec.
    fn font_name(spec: &str) -> String {
        let s = spec.trim();
        if let Some(rest) = s.strip_prefix('[') {
            return rest.split(']').next().unwrap_or(rest).trim().to_string();
        }
        s.split([':', '/']).next().unwrap_or(s).trim().to_string()
    }
}

impl ResourceProvider for TexmfFontProvider {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let name = Self::font_name(&request.canonical_name());
        self.texmf.read(&name, ResourceKind::Font)
    }
}

/// First transcript line that signals a TeX error (`! ...`), if any.
fn tex_error(transcript: &str) -> Option<String> {
    transcript
        .lines()
        .find(|l| l.trim_start().starts_with("! "))
        .map(str::to_string)
}

/// Engine font system: rustybuzz over the texmf font tree.
struct NativeFontSystem {
    inner: RustybuzzFontSystem<ResourceFontSystem<TexmfFontProvider>>,
}

impl NativeFontSystem {
    fn new(texmf: Arc<TexmfResources>) -> Self {
        Self {
            inner: RustybuzzFontSystem::new(ResourceFontSystem::new(TexmfFontProvider { texmf })),
        }
    }
}

impl FontSystem for NativeFontSystem {
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        self.inner.load_font(query)
    }

    fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        self.inner.shape_text(request)
    }
}

/// Glyph outline source that maps all glyph runs to one configured math font.
struct SingleFontOutlines {
    font: FontData,
}

impl GlyphOutlineSource for SingleFontOutlines {
    fn glyph_run_outlines(&self, _font: &FontRef, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>> {
        self.font
            .glyph_outlines(glyphs)
            .unwrap_or_else(|_| vec![None; glyphs.len()])
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let raw_expr = args.next().unwrap_or_else(|| "x^2+y".to_string());
    let out_path = args.next().unwrap_or_else(|| "eq.svg".to_string());

    // The fragment wrapper supplies $...$, strip any dollars the caller passed.
    let body = raw_expr.trim().trim_matches('$').to_string();

    let Some(texmf_root) = find_texmf_root() else {
        eprintln!(
            "mathtex-svg-cli: no TeXLive ls-R index found; set {TEXMF_ROOT_ENV} to a texmf-dist directory"
        );
        return ExitCode::FAILURE;
    };
    let Some(texmf) = TexmfResources::from_root(&texmf_root) else {
        eprintln!(
            "mathtex-svg-cli: no TeXLive ls-R index under {} (run mktexlsr)",
            texmf_root.display()
        );
        return ExitCode::FAILURE;
    };
    let texmf = Arc::new(texmf);

    // Math font bytes for the outline source, `FontData` caches the face parse.
    let outline_font = match texmf.read(FONT_FILE, ResourceKind::Font) {
        Ok(r) => FontData::new(mathtex_ir::FontId(1), FONT_FILE, r.bytes),
        Err(e) => {
            eprintln!("mathtex-svg-cli: cannot resolve {FONT_FILE}: {e:?}");
            return ExitCode::FAILURE;
        }
    };

    let t = std::time::Instant::now();

    let base = GeneratedFormatCache::initialized(EngineProfile::xetex());
    let mut engine = base
        .instantiate(
            EngineProfile::xetex(),
            GeneratedResourceProvider::new(&*texmf),
        )
        .with_font_platform(GeneratedFontSystemAdapter::new(NativeFontSystem::new(
            texmf.clone(),
        )));

    engine.begin_fragment_capture();
    if !engine.begin_primary_input("frag.tex", native_math_document(&body).into_bytes()) {
        eprintln!("mathtex-svg-cli: engine refused fragment input");
        return ExitCode::FAILURE;
    }
    engine.run_main_control();
    engine.end_fragment_capture();

    let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    if let Some(err) = tex_error(&transcript) {
        eprintln!("mathtex-svg-cli: fragment layout reported a TeX error:\n{err}");
        let _ = std::fs::write("/tmp/mathtex-svg-cli.log", &transcript);
        eprintln!("mathtex-svg-cli: full transcript written to /tmp/mathtex-svg-cli.log");
        return ExitCode::FAILURE;
    }

    let Some(root) = engine.captured_fragment_root() else {
        eprintln!("mathtex-svg-cli: engine captured no fragment root");
        return ExitCode::FAILURE;
    };
    let Some(fragment) = generated_node_to_fragment(
        &engine,
        root,
        FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "mathtex-svg-cli".into(),
            fragment_kind: Default::default(),
        },
    ) else {
        eprintln!("mathtex-svg-cli: captured root failed to convert to IR");
        return ExitCode::FAILURE;
    };
    let root_id = select_root(&fragment);

    let outlines = SingleFontOutlines { font: outline_font };
    let svg = match render_fragment(&fragment, root_id, &outlines) {
        Ok(svg) => svg,
        Err(e) => {
            eprintln!("mathtex-svg-cli: SVG render failed: {e:?}");
            return ExitCode::FAILURE;
        }
    };

    let elapsed = t.elapsed();

    if let Err(e) = std::fs::write(&out_path, &svg) {
        eprintln!("mathtex-svg-cli: failed to write {out_path}: {e}");
        return ExitCode::FAILURE;
    }

    let has_path = svg.contains("<path");
    eprintln!("expression:    {body}");
    eprintln!("output:        {out_path}");
    eprintln!("texmf:         {}", texmf.root().display());
    eprintln!("layout+render: {elapsed:?}");
    eprintln!("fragment nodes: {}", fragment.nodes.len());
    eprintln!("svg bytes:     {}", svg.len());
    eprintln!("contains <path: {has_path}");

    if has_path {
        ExitCode::SUCCESS
    } else {
        eprintln!("mathtex-svg-cli: warning, rendered SVG has no <path> glyph outlines");
        ExitCode::FAILURE
    }
}

fn find_texmf_root() -> Option<PathBuf> {
    if let Ok(root) = std::env::var(TEXMF_ROOT_ENV) {
        let root = PathBuf::from(root);
        if root.join("ls-R").is_file() {
            return Some(root);
        }
    }

    TEXMF_ROOT_CANDIDATES
        .iter()
        .map(Path::new)
        .find(|root| root.join("ls-R").is_file())
        .map(Path::to_path_buf)
}

fn native_math_document(body: &str) -> String {
    format!(
        concat!(
            r"\catcode`\{{=1 \catcode`\}}=2 ",
            r"\catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
            "\\font\\mf=\"[{font}]:script=math\" at 10pt ",
            r"\textfont0=\mf \scriptfont0=\mf \scriptscriptfont0=\mf ",
            r"\textfont1=\mf \scriptfont1=\mf \scriptscriptfont1=\mf ",
            r"\textfont2=\mf \scriptfont2=\mf \scriptscriptfont2=\mf ",
            r"\textfont3=\mf \scriptfont3=\mf \scriptscriptfont3=\mf ",
            "\\Udelcode`\\(=\"0 \"0028 \\Udelcode`\\)=\"0 \"0029 ",
            r"\def\frac#1#2{{#1\over#2}} ",
            "\\def\\sqrt{{\\Uradical\"0 \"221A }} ",
            r"\hbox{{$",
            "{body}",
            r"\relax$}}\end",
        ),
        font = FONT_FILE,
        body = body,
    )
}

/// The captured root box is the one no other node references as a child.
fn select_root(fragment: &Fragment) -> Option<NodeId> {
    let mut is_child: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
    for node in &fragment.nodes {
        match &node.kind {
            LayoutNodeKind::Box(b) => is_child.extend(b.children.iter().copied()),
            LayoutNodeKind::List(l) => is_child.extend(l.children.iter().copied()),
            LayoutNodeKind::Group { children } => is_child.extend(children.iter().copied()),
            _ => {}
        }
    }
    fragment.nodes.iter().find_map(|node| match &node.kind {
        LayoutNodeKind::Box(_) if !is_child.contains(&node.id) => Some(node.id),
        _ => None,
    })
}

/// Renders the full fragment to SVG via `mathtex_svg::render_with_outlines`.
fn render_fragment(
    fragment: &Fragment,
    _root_id: Option<NodeId>,
    outlines: &dyn GlyphOutlineSource,
) -> Result<String, mathtex_svg::SvgError> {
    mathtex_svg::render_with_outlines(fragment, outlines)
}
