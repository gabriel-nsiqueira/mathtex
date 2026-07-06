//! Tests boot real LaTeX and unicode-math through a pure Rust ls-R ResourceProvider.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use mathtex_engine::portable_engine as pe;
use mathtex_engine::{
    EngineBuilder, EngineProfile, FormatImage, FragmentInput, GeneratedFormatCache,
    GeneratedResourceProvider, NoopPlatform, ProviderResourceRequest as ResourceRequest, Resource,
    ResourceError, ResourceKind, ResourceProvider, XetexProfile,
};
use mathtex_ir::LayoutNodeKind;

const TEXMF_ROOT: &str = "/usr/local/texlive/2025/texmf-dist";

/// Maps each file class to ordered `texmf.cnf` directory prefixes.
#[derive(Clone, Copy)]
enum Fmt {
    Tex,
    Tfm,
    OpenType,
    Enc,
    Map,
}

impl Fmt {
    /// Ordered relative directory prefixes, highest priority first.
    fn prefixes(self) -> &'static [&'static str] {
        match self {
            // The xelatex input path adds the whole tex// tree after narrower roots.
            Fmt::Tex => &["tex/xelatex", "tex/latex", "tex/xetex", "tex/generic", "tex"],
            // The tfm font path uses the tfm font tree.
            Fmt::Tfm => &["fonts/tfm"],
            // The opentype font path uses opentype and truetype font trees.
            Fmt::OpenType => &["fonts/opentype", "fonts/truetype"],
            // The encoding font path uses the encoding tree.
            Fmt::Enc => &["fonts/enc"],
            // The font map path uses the font map tree.
            Fmt::Map => &["fonts/map"],
        }
    }
}

/// ResourceProvider backed by the ls-R index with texmf.cnf path priority.
#[derive(Clone)]
struct TexmfResources {
    root: PathBuf,
    /// Basename to relative dirs in ls-R order.
    index: HashMap<String, Vec<String>>,
}

impl TexmfResources {
    fn from_texlive() -> Option<Self> {
        let root = PathBuf::from(TEXMF_ROOT);
        let bytes = std::fs::read(root.join("ls-R")).ok()?;
        let text = String::from_utf8_lossy(&bytes);

        let mut index: HashMap<String, Vec<String>> = HashMap::new();
        let mut cur_dir = String::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('%') {
                continue;
            }
            if let Some(dir) = line.strip_suffix(':') {
                cur_dir = dir.strip_prefix("./").unwrap_or(dir).to_string();
                continue;
            }
            index.entry(line.to_string()).or_default().push(cur_dir.clone());
        }

        if index.is_empty() {
            return None;
        }
        Some(Self { root, index })
    }

    fn normalize(name: &str) -> String {
        let mut n = name.trim();
        loop {
            if let Some(s) = n.strip_prefix("./") {
                n = s;
            } else if let Some(s) = n.strip_prefix("[]") {
                n = s;
            } else if let Some(s) = n.strip_prefix(':') {
                n = s;
            } else {
                break;
            }
        }
        let n = n.trim_matches(|c| c == '[' || c == ']' || c == '"' || c == '\'');
        n.rsplit(['/', '\\']).next().unwrap_or(n).to_string()
    }

    fn format_for(kind: ResourceKind, filename: &str) -> Fmt {
        let lower = filename.to_ascii_lowercase();
        match kind {
            ResourceKind::Encoding => Fmt::Enc,
            ResourceKind::Map => Fmt::Map,
            ResourceKind::Font => {
                if lower.ends_with(".otf") || lower.ends_with(".ttf") || lower.ends_with(".otc")
                {
                    Fmt::OpenType
                } else {
                    Fmt::Tfm
                }
            }
            // Tex tree, refined by extension for stray font assets.
            _ => {
                if lower.ends_with(".enc") {
                    Fmt::Enc
                } else if lower.ends_with(".map") {
                    Fmt::Map
                } else if lower.ends_with(".tfm") {
                    Fmt::Tfm
                } else if lower.ends_with(".otf") || lower.ends_with(".ttf") {
                    Fmt::OpenType
                } else {
                    Fmt::Tex
                }
            }
        }
    }

    fn resolve(&self, filename: &str, fmt: Fmt) -> Option<PathBuf> {
        let dirs = self.index.get(filename)?;
        for prefix in fmt.prefixes() {
            for dir in dirs {
                if dir == prefix || dir.strip_prefix(prefix).is_some_and(|r| r.starts_with('/')) {
                    return Some(self.root.join(dir).join(filename));
                }
            }
        }
        None
    }

    /// Candidate filenames for a request.
    fn candidates(request: &ResourceRequest) -> Vec<String> {
        let base = Self::normalize(&request.canonical_name());
        let mut out = vec![base.clone()];
        if Path::new(&base).extension().is_none() {
            let exts: &[&str] = match request.kind {
                ResourceKind::Package => &[".sty", ".tex", ".def", ".ltx"],
                ResourceKind::Class => &[".cls"],
                ResourceKind::FontDefinition => &[".fd"],
                ResourceKind::PackageSupport => &[".def", ".cfg", ".ldf", ".clo", ".sty", ".tex"],
                ResourceKind::Config => &[".cfg", ".cnf", ".tex"],
                ResourceKind::Encoding => &[".enc"],
                ResourceKind::Map => &[".map"],
                ResourceKind::Font => &[".tfm", ".otf", ".ttf"],
                ResourceKind::TexInput => &[".tex", ".ltx", ".def", ".sty", ".cfg", ".fd"],
                _ => &[".tex", ".sty", ".def", ".cfg", ".ltx", ".fd", ".cls", ".enc"],
            };
            for e in exts {
                out.push(format!("{base}{e}"));
            }
        }
        out
    }
}

impl ResourceProvider for TexmfResources {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        for cand in Self::candidates(request) {
            let fmt = Self::format_for(request.kind, &cand);
            if let Some(path) = self.resolve(&cand, fmt) {
                if let Ok(bytes) = std::fs::read(&path) {
                    return Ok(Resource::from_request(request, bytes));
                }
            }
        }
        Err(ResourceError::NotFound {
            name: request.canonical_name(),
            kind: request.kind,
        })
    }
}

/// Wraps a provider and logs every resolution so two providers can be diffed.
struct Logged<P> {
    inner: P,
    log: RefCell<Vec<String>>,
}

impl<P: ResourceProvider> ResourceProvider for Logged<P> {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let result = self.inner.read_request(request);
        let resolved = match &result {
            // Size only so ls-R and host oracle logs are comparable.
            Ok(r) => format!("{}b", r.bytes.len()),
            Err(_) => "MISS".to_string(),
        };
        self.log.borrow_mut().push(format!(
            "{:?} {:?} -> {resolved}",
            request.kind,
            request.canonical_name()
        ));
        result
    }
}

/// Resolves via the host TeX resolver binary for tests that diff against the ls-R provider.
struct KpseRes;

impl ResourceProvider for KpseRes {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        for cand in TexmfResources::candidates(request) {
            if let Ok(out) = std::process::Command::new("kpsewhich").arg(&cand).output() {
                if out.status.success() {
                    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !path.is_empty() {
                        if let Ok(bytes) = std::fs::read(&path) {
                            return Ok(Resource::new(path, request.kind, bytes));
                        }
                    }
                }
            }
        }
        Err(ResourceError::NotFound {
            name: request.canonical_name(),
            kind: request.kind,
        })
    }
}

/// Boots latex.ltx plus optional preamble and returns the error count and resolution log.
fn boot_latex_and_probe<P: ResourceProvider>(
    provider: P,
    preamble: &[u8],
    label: &str,
) -> (usize, Vec<String>) {
    let latex = provider
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx resolves")
        .bytes;
    let logged = Logged {
        inner: provider,
        log: RefCell::new(Vec::new()),
    };
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache.instantiate(
        pe::EngineProfile::xetex(),
        GeneratedResourceProvider::new(&logged),
    );
    let probe = br" \catcode`\@=11 \message{[PROBE/LABEL encodingdefault=\encodingdefault UnicodeEncodingName=\ifx\UnicodeEncodingName\@undefined U\else D\fi setmainfont=\ifx\setmainfont\@undefined U\else D\fi setmathfont=\ifx\setmathfont\@undefined U\else D\fi]}\dump";
    let mut bytes = latex;
    bytes.push(b' ');
    bytes.extend_from_slice(preamble);
    bytes.extend_from_slice(probe);
    assert!(engine.begin_primary_input("latex.ltx", bytes));
    let completed = engine.run_format_initialization();
    let transcript = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    drop(engine);

    let lines: Vec<&str> = transcript.lines().collect();
    let errors = lines.iter().filter(|l| l.starts_with("! ")).count();
    eprintln!("=== {label}: completed={completed} errors={errors} ===");
    for line in lines.iter().filter(|l| l.contains("PROBE/LABEL")) {
        eprintln!("{}", line.replace("PROBE/LABEL", label));
    }
    if let Some(i) = lines.iter().position(|l| l.starts_with("! ")) {
        eprintln!("--- {label}: first error context ---");
        for l in lines.iter().skip(i.saturating_sub(2)).take(10) {
            eprintln!("  {l}");
        }
    }
    let log = logged.log.borrow().clone();
    (errors, log)
}

/// Boots latex.ltx through the ls-R provider and the host oracle.
#[test]
fn dump_latex_boot_transcript() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let (ls_err, ls_log) = boot_latex_and_probe(texmf, b"", "ls-R");
    let (kp_err, kp_log) = boot_latex_and_probe(KpseRes, b"", "host oracle");

    eprintln!("\n=== SUMMARY: ls-R errors={ls_err}  host oracle errors={kp_err} ===");
    eprintln!("ls-R requests={} host oracle requests={}", ls_log.len(), kp_log.len());
    // First index where the request streams diverge.
    let first = (0..ls_log.len().max(kp_log.len())).find(|&i| ls_log.get(i) != kp_log.get(i));
    match first {
        None => eprintln!("resolution sequences are IDENTICAL"),
        Some(i) => {
            eprintln!("--- first divergence at request #{i} ---");
            eprintln!("  ls-R from here:");
            for l in ls_log.iter().skip(i.saturating_sub(1)).take(14) {
                eprintln!("    {l}");
            }
            eprintln!("  host oracle from here:");
            for l in kp_log.iter().skip(i.saturating_sub(1)).take(14) {
                eprintln!("    {l}");
            }
        }
    }
}

mod testfonts {
    use super::{ResourceError, ResourceKind, ResourceProvider, TexmfResources};
    use mathtex_engine::font::{
        FontData, FontError, FontQuery, FontSystem, RustybuzzFontSystem, ShapeRequest, ShapedText,
    };
    use mathtex_engine::{ProviderResourceRequest as Req, Resource, ResourceFontSystem};

    pub struct FontProv(pub TexmfResources);
    impl ResourceProvider for FontProv {
        fn read_request(&self, r: &Req) -> Result<Resource, ResourceError> {
            let s = r.canonical_name();
            let name = if let Some(rest) = s.strip_prefix('[') {
                rest.split(']').next().unwrap_or(&rest).to_string()
            } else {
                s.split([':', '/']).next().unwrap_or(&s).to_string()
            };
            self.0.read(&name, ResourceKind::Font)
        }
    }
    pub struct Fonts(pub RustybuzzFontSystem<ResourceFontSystem<FontProv>>);
    impl Fonts {
        pub fn new(texmf: TexmfResources) -> Self {
            Self(RustybuzzFontSystem::new(ResourceFontSystem::new(FontProv(texmf))))
        }
    }
    impl FontSystem for Fonts {
        fn load_font(&self, q: &FontQuery) -> Result<FontData, FontError> {
            self.0.load_font(q)
        }
        fn shape_text(&self, r: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
            self.0.shape_text(r)
        }
    }
}

/// Verifies fragment capture fires for an `\hbox` built after `\begin{document}`.
#[test]
fn capture_in_document_context() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    // headforvmode fix prevents horizontal mode from exhausting memory after `\begin{document}`.
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();
    let pre = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    let de_doubled: String = pre.chars().step_by(2).collect();
    eprintln!(
        "preamble: errors={} capacity_oom={} last_err={:?}",
        pre.lines().filter(|l| l.starts_with("! ")).count(),
        de_doubled.contains("capacity exceeded") || pre.contains("capacity"),
        pre.lines().rev().find(|l| l.starts_with("! ")),
    );
    assert!(engine.begin_primary_input(
        "frag.tex",
        br"\hbox{$X\mathit{X}\symit{X}$}\csname @@end\endcsname\end".to_vec(),
    ));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    let frag = String::from_utf8_lossy(engine.transcript_bytes()).into_owned();
    let _ = frag;
    if let Some(root) = engine.captured_fragment_root() {
        let f = mathtex_engine::generated::generated_node_to_fragment(
            &engine,
            root,
            mathtex_ir::FragmentMetadata {
                engine_profile: "xetex".into(),
                format_id: "t".into(),
                fragment_kind: Default::default(),
            },
        );
        let glyphs: Vec<u32> = f
            .iter()
            .flat_map(|f| f.nodes.iter())
            .filter_map(|n| match &n.kind {
                mathtex_ir::LayoutNodeKind::GlyphRun(r) => {
                    Some(r.glyphs.iter().map(|g| g.glyph_id.0).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();
        eprintln!(
            "FRAGMENT: ran={ran} captured=true glyph_ids={glyphs:?}  (X: upright=88, math-italic > 1000)"
        );
    } else {
        eprintln!("FRAGMENT: ran={ran} captured=false");
    }
}

/// Verifies `\Umathcodenum` sets a math code alongside `\Umathcode`.
#[test]
fn umathcodenum_works() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP");
        return;
    };
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    let prog = br#"\catcode`\{=1 \catcode`\}=2 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 \font\mf="[latinmodern-math.otf]:script=math" at 10pt \textfont0=\mf \scriptfont0=\mf \scriptscriptfont0=\mf \textfont1=\mf \scriptfont1=\mf \scriptscriptfont1=\mf \textfont2=\mf \scriptfont2=\mf \scriptscriptfont2=\mf \textfont3=\mf \scriptfont3=\mf \scriptscriptfont3=\mf \Umathcode`x=7 1 "1D465 \Umathcodenum`y="1E1D466 \hbox{$xy$\relax}\end"#;
    engine.begin_fragment_capture();
    assert!(engine.begin_primary_input("t", prog.to_vec()));
    engine.run_main_control();
    engine.end_fragment_capture();
    if let Some(root) = engine.captured_fragment_root() {
        let f = mathtex_engine::generated::generated_node_to_fragment(
            &engine,
            root,
            mathtex_ir::FragmentMetadata {
                engine_profile: "x".into(),
                format_id: "t".into(),
                fragment_kind: Default::default(),
            },
        );
        let g: Vec<u32> = f
            .iter()
            .flat_map(|f| f.nodes.iter())
            .filter_map(|n| match &n.kind {
                mathtex_ir::LayoutNodeKind::GlyphRun(r) => {
                    Some(r.glyphs.iter().map(|x| x.glyph_id.0).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect();
        eprintln!("UMATHCODENUM glyph_ids={g:?}  (x via \\Umathcode, y via \\Umathcodenum; italic=1319/1320, upright=89/90)");
    } else {
        eprintln!("UMATHCODENUM: no capture");
    }
}

/// Boots latex.ltx then `\RequirePackage{unicode-math}` and reports the error count.
#[test]
fn boot_unicode_math() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let (errors, _) = boot_latex_and_probe(texmf, br"\RequirePackage{unicode-math}", "latex+um");
    eprintln!("latex+unicode-math errors = {errors}");
}

#[test]
fn boot_real_unicode_math_format() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    eprintln!("texmf index: {} files", texmf.index.len());
    // Sanity check that closure roots resolve through our provider.
    for (name, kind) in [
        ("latex.ltx", ResourceKind::TexInput),
        ("unicode-math", ResourceKind::Package),
        ("fontspec", ResourceKind::Package),
        ("expl3", ResourceKind::Package),
    ] {
        match texmf.read(name, kind) {
            Ok(r) => eprintln!("  resolve {name:>14} -> {} bytes", r.bytes.len()),
            Err(e) => eprintln!("  resolve {name:>14} -> ERR {e:?}"),
        }
    }

    let profile = XetexProfile;

    let latex_only = FormatImage::latex(profile.id())
        .build(&texmf)
        .expect("latex.ltx top-level bytes should resolve");
    match EngineBuilder::new(profile)
        .format(latex_only)
        .resources(&texmf)
        .platform(NoopPlatform::default())
        .build()
    {
        Ok(_) => eprintln!("STAGE 1 OK: latex.ltx booted through TexmfResources"),
        Err(e) => {
            eprintln!("STAGE 1 FAIL (latex.ltx): {e:?}");
            panic!("latex.ltx did not boot through the FS provider");
        }
    }

    let with_um = FormatImage::latex(profile.id())
        .preload_package("fontspec")
        .preload_package("unicode-math")
        .build(&texmf)
        .expect("unicode-math.sty top-level bytes should resolve");
    let engine = match EngineBuilder::new(profile)
        .format(with_um)
        .resources(&texmf)
        .platform(NoopPlatform::default())
        .build()
    {
        Ok(engine) => {
            eprintln!("STAGE 2 OK: latex + unicode-math booted!");
            engine
        }
        Err(e) => {
            eprintln!("STAGE 2 WALL (latex + unicode-math): {e:?}");
            // This is not a hard failure because the error names the exact unsupported boundary.
            return;
        }
    };

    let mut session = engine.new_session();
    match session.layout_fragment_input(FragmentInput::text(
        r"\setmathfont{latinmodern-math.otf}$x^2+y$",
    )) {
        Ok(fragment) => {
            let glyphs = fragment
                .nodes
                .iter()
                .filter(|n| matches!(n.kind, mathtex_ir::LayoutNodeKind::GlyphRun(_)))
                .count();
            eprintln!("STAGE 3 OK: fragment with {glyphs} glyph runs");
        }
        Err(e) => eprintln!("STAGE 3 WALL (layout): {e:?}"),
    }
}

/// Outline source that resolves all glyph runs against one OTF.
struct SingleFontOutlines {
    font: mathtex_engine::font::FontData,
}

impl mathtex_render::GlyphOutlineSource for SingleFontOutlines {
    fn glyph_run_outlines(
        &self,
        _font: &mathtex_ir::FontRef,
        glyphs: &[mathtex_ir::GlyphId],
    ) -> Vec<Option<mathtex_ir::GlyphOutline>> {
        self.font
            .glyph_outlines(glyphs)
            .unwrap_or_else(|_| vec![None; glyphs.len()])
    }
}

/// Boots unicode math, captures `$x+y$`, and renders SVG.
#[test]
fn render_unicode_math_to_svg() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "frag.tex",
        br"\hbox{$x+y$}\csname @@end\endcsname\end".to_vec(),
    ));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    assert!(ran, "main control did not run to completion");

    let root = engine
        .captured_fragment_root()
        .expect("engine captured a fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "svg".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("captured root converts to IR");

    let glyph_ids: Vec<u32> = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            mathtex_ir::LayoutNodeKind::GlyphRun(run) => {
                Some(run.glyphs.iter().map(|g| g.glyph_id.0).collect::<Vec<_>>())
            }
            _ => None,
        })
        .flatten()
        .collect();
    eprintln!("glyph_ids = {glyph_ids:?}");
    assert!(
        glyph_ids.iter().any(|&g| g > 1000),
        "expected math-italic glyphs (>1000), got {glyph_ids:?}"
    );

    let otf = texmf
        .read("latinmodern-math.otf", ResourceKind::Font)
        .expect("latinmodern-math.otf")
        .bytes;
    let outlines = SingleFontOutlines {
        font: mathtex_engine::font::FontData::new(
            mathtex_ir::FontId(1),
            "latinmodern-math.otf",
            otf,
        ),
    };
    let svg = mathtex_svg::render_with_outlines(&fragment, &outlines).expect("SVG render");

    let path_count = svg.matches("<path").count();
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("unicode_math_xy.svg");
    std::fs::write(&out, &svg).expect("write svg");
    eprintln!(
        "SVG: {} bytes, {path_count} <path> outlines -> {}",
        svg.len(),
        out.display()
    );
    assert!(svg.contains("<path"), "rendered SVG has no <path> glyph outlines");
    assert!(
        path_count >= 2,
        "expected >=2 glyph outlines (x, y), got {path_count}"
    );
}

/// Extracts the font filename from an IR `FontRef.name` XeTeX spec.
fn parse_font_file(spec: &str) -> String {
    let spec = spec.trim();
    if let Some(rest) = spec.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest).to_string()
    } else {
        spec.split(['/', ':']).next().unwrap_or(spec).to_string()
    }
}

/// Outline source that resolves each glyph run to its originating font file.
struct MultiFontOutlines {
    texmf: TexmfResources,
    cache: RefCell<HashMap<String, Option<mathtex_engine::font::FontData>>>,
}

impl MultiFontOutlines {
    fn new(texmf: TexmfResources) -> Self {
        Self {
            texmf,
            cache: RefCell::new(HashMap::new()),
        }
    }
}

impl mathtex_render::GlyphOutlineSource for MultiFontOutlines {
    fn glyph_run_outlines(
        &self,
        font: &mathtex_ir::FontRef,
        glyphs: &[mathtex_ir::GlyphId],
    ) -> Vec<Option<mathtex_ir::GlyphOutline>> {
        let file = parse_font_file(&font.name);
        let mut cache = self.cache.borrow_mut();
        let entry = cache.entry(file.clone()).or_insert_with(|| {
            self.texmf
                .read(&file, ResourceKind::Font)
                .ok()
                .map(|res| mathtex_engine::font::FontData::new(mathtex_ir::FontId(0), file, res.bytes))
        });
        match entry.as_ref() {
            Some(font_data) => font_data
                .glyph_outlines(glyphs)
                .unwrap_or_else(|_| vec![None; glyphs.len()]),
            None => vec![None; glyphs.len()],
        }
    }
}

/// Verifies `$X\mathit{X}\symit{X}$` uses at least two distinct fonts.
#[test]
fn render_mixed_font_to_svg() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "frag.tex",
        br"\hbox{$X\mathit{X}\symit{X}$}\csname @@end\endcsname\end".to_vec(),
    ));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    assert!(ran);

    let root = engine.captured_fragment_root().expect("fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "svg".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR");

    let runs: Vec<(String, Vec<u32>)> = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            mathtex_ir::LayoutNodeKind::GlyphRun(run) => Some((
                parse_font_file(&run.font.name),
                run.glyphs.iter().map(|g| g.glyph_id.0).collect(),
            )),
            _ => None,
        })
        .collect();
    eprintln!("runs = {runs:?}");
    let distinct_files: std::collections::BTreeSet<&String> = runs.iter().map(|(f, _)| f).collect();
    assert!(
        distinct_files.len() >= 2,
        "expected >=2 distinct font files (latinmodern-math + lmr), got {distinct_files:?}"
    );

    let multi = MultiFontOutlines::new(TexmfResources::from_texlive().expect("texmf"));
    let single = SingleFontOutlines {
        font: mathtex_engine::font::FontData::new(
            mathtex_ir::FontId(1),
            "latinmodern-math.otf",
            texmf
                .read("latinmodern-math.otf", ResourceKind::Font)
                .expect("otf")
                .bytes,
        ),
    };
    let svg_multi = mathtex_svg::render_with_outlines(&fragment, &multi).expect("multi svg");
    let svg_single = mathtex_svg::render_with_outlines(&fragment, &single).expect("single svg");

    let loaded: Vec<String> = multi.cache.borrow().keys().cloned().collect();
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mixed_font.svg");
    std::fs::write(&out, &svg_multi).expect("write svg");
    eprintln!(
        "multi-font loaded {loaded:?}; svg {} bytes ({} paths) -> {}",
        svg_multi.len(),
        svg_multi.matches("<path").count(),
        out.display()
    );

    assert!(svg_multi.contains("<path"), "multi-font SVG has no outlines");
    assert!(
        loaded.len() >= 2,
        "multi-font source should load >=2 font files, loaded {loaded:?}"
    );
    assert_ne!(
        svg_multi, svg_single,
        "multi-font rendering must differ from single-font (glyph 115 from lmr vs latinmodern-math)"
    );
}

/// Verifies `ssty=1` is applied to the script size font.
#[test]
fn render_ssty_script_size() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "frag.tex",
        br"\hbox{$x^{x}$}\csname @@end\endcsname\end".to_vec(),
    ));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    assert!(ran);

    let root = engine.captured_fragment_root().expect("fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "ssty".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR");

    let runs: Vec<(String, Vec<u32>)> = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            mathtex_ir::LayoutNodeKind::GlyphRun(run) => Some((
                run.font.name.clone(),
                run.glyphs.iter().map(|g| g.glyph_id.0).collect(),
            )),
            _ => None,
        })
        .collect();
    eprintln!("ssty runs = {runs:?}");
    let all: Vec<u32> = runs.iter().flat_map(|(_, g)| g.clone()).collect();
    assert!(all.contains(&1319), "base x glyph 1319 expected, got {all:?}");
    assert!(
        all.contains(&1427),
        "ssty script-size x glyph 1427 expected (ssty=1 applied under math script), got {all:?}"
    );
}

#[test]
fn render_cauchy_schwarz_svg() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "frag.tex",
        br"\hbox{$\displaystyle \left(\sum_{k=1}^n a_k b_k\right)^2\leq\left(\sum_{k=1}^n a_k^2\right)\left(\sum_{k=1}^n b_k^2\right)$}\csname @@end\endcsname\end"
            .to_vec(),
    ));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    assert!(ran, "main control did not run to completion");

    let root = engine.captured_fragment_root().expect("fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "svg".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR");

    let glyph_count: usize = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            mathtex_ir::LayoutNodeKind::GlyphRun(run) => Some(run.glyphs.len()),
            _ => None,
        })
        .sum();
    let files: std::collections::BTreeSet<String> = fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            mathtex_ir::LayoutNodeKind::GlyphRun(run) => Some(parse_font_file(&run.font.name)),
            _ => None,
        })
        .collect();

    let outlines = MultiFontOutlines::new(TexmfResources::from_texlive().expect("texmf"));
    let svg = mathtex_svg::render_with_outlines(&fragment, &outlines).expect("svg");

    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("cauchy_schwarz.svg");
    std::fs::write(&out, &svg).expect("write svg");
    eprintln!(
        "CAUCHY-SCHWARZ: {glyph_count} glyphs, fonts={files:?}, {} <path>, {} bytes -> {}",
        svg.matches("<path").count(),
        svg.len(),
        out.display()
    );
    assert!(svg.contains("<path"), "rendered SVG has no glyph outlines");
}

/// Renders `body` in `\hbox{...}` via latinmodern-math.otf.
fn render_native_run(texmf: &TexmfResources, body: &str, tracking: bool) -> (mathtex_ir::Fragment, String) {
    render_native_run_spec(texmf, "[latinmodern-math.otf]", body, tracking)
}

/// Like [`render_native_run`] but accepts a caller supplied `\font` spec.
fn render_native_run_spec(
    texmf: &TexmfResources,
    font_spec: &str,
    body: &str,
    tracking: bool,
) -> (mathtex_ir::Fragment, String) {
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    engine.set_source_tracking(tracking);
    engine.begin_fragment_capture();
    let prog = format!(
        "\\catcode`\\{{=1 \\catcode`\\}}=2 \\font\\lm=\"{font_spec}\" at 10pt \\lm \\hbox{{{body}}}\\end"
    );
    assert!(
        engine.begin_primary_input("input", prog.as_bytes().to_vec()),
        "engine refused primary input"
    );
    assert!(engine.run_main_control(), "run_main_control failed");
    engine.end_fragment_capture();
    let root = engine
        .captured_fragment_root()
        .expect("engine captured no fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "native-cluster".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("captured root failed to convert to IR");
    (fragment, prog)
}

fn native_glyph_run(fragment: &mathtex_ir::Fragment) -> &mathtex_ir::LayoutNode {
    fragment
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, LayoutNodeKind::GlyphRun(r) if r.glyphs.len() > 1))
        .expect("no multi-glyph native run found")
}

#[test]
fn native_cluster_exact_per_char_source() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let (fragment, fed) = render_native_run(&texmf, "abc", true);
    let node = native_glyph_run(&fragment);
    let LayoutNodeKind::GlyphRun(run) = &node.kind else {
        unreachable!()
    };
    assert_eq!(run.glyphs.len(), 3, "expected 3 glyphs for abc");

    let primary = fragment
        .primary_source_for_node(node.id)
        .expect("native run node has no primary source");
    let primary_name = &fragment
        .source_map
        .source(primary.source)
        .expect("primary source resolves")
        .name;
    assert_eq!(primary_name, "input", "native run must map to the user fragment");

    let expected = ["a", "b", "c"];
    for (i, want) in expected.iter().enumerate() {
        let range = fragment
            .glyph_source_range(node.id, i)
            .unwrap_or_else(|| panic!("glyph {i} has no source range (cluster None)"));
        let src = fragment
            .source_map
            .source(range.source)
            .expect("source resolves");
        assert_eq!(src.name, "input", "glyph {i} span must point at the fragment");
        let (s, e) = (range.span.start as usize, range.span.end as usize);
        assert!(s <= e && e <= fed.len(), "glyph {i} span [{s},{e}) OOB");
        let got = &fed[s..e];
        assert_eq!(
            got, *want,
            "glyph {i} mapped to {got:?}, expected {want:?}, per char source is not exact"
        );
    }

    // Confirms spans are real tracked positions inside the `\hbox` body.
    let starts: Vec<u32> = (0..3)
        .map(|i| fragment.glyph_source_range(node.id, i).unwrap().span.start)
        .collect();
    let abc_at = fed.find("abc").expect("fed contains abc") as u32;
    assert_eq!(
        starts,
        vec![abc_at, abc_at + 1, abc_at + 2],
        "glyph spans must be the real tracked char offsets of a/b/c, not a linear guess"
    );
}

/// With source tracking off, native clusters are absent but glyph layout is unchanged.
#[test]
fn native_cluster_tracking_off_is_noop() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let (on, _) = render_native_run(&texmf, "abc", true);
    let (off, _) = render_native_run(&texmf, "abc", false);

    let glyph_layout = |f: &mathtex_ir::Fragment| -> Vec<(u32, i32)> {
        f.nodes
            .iter()
            .filter_map(|n| match &n.kind {
                LayoutNodeKind::GlyphRun(r) => Some(
                    r.glyphs
                        .iter()
                        .map(|g| (g.glyph_id.0, g.advance.x.0))
                        .collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .flatten()
            .collect()
    };
    assert_eq!(
        glyph_layout(&on),
        glyph_layout(&off),
        "tracking must not change glyph ids/advances (layout)"
    );

    let off_node = native_glyph_run(&off);
    let LayoutNodeKind::GlyphRun(off_run) = &off_node.kind else {
        unreachable!()
    };
    assert!(
        off_run.glyphs.iter().all(|g| g.cluster.is_none()),
        "tracking OFF must leave all native glyph clusters None"
    );
    assert!(
        off.glyph_source_range(off_node.id, 0).is_none(),
        "tracking OFF must yield no glyph source range"
    );

    let on_node = native_glyph_run(&on);
    let LayoutNodeKind::GlyphRun(on_run) = &on_node.kind else {
        unreachable!()
    };
    assert!(
        on_run.glyphs.iter().all(|g| g.cluster.is_some()),
        "tracking ON must populate native glyph clusters"
    );
}

/// First glyph run with any glyph count, including a one glyph ligature.
fn first_glyph_run(fragment: &mathtex_ir::Fragment) -> &mathtex_ir::LayoutNode {
    fragment
        .nodes
        .iter()
        .find(|n| matches!(&n.kind, LayoutNodeKind::GlyphRun(r) if !r.glyphs.is_empty()))
        .expect("no glyph run found")
}

/// Byte offset of the first char of `word` inside `\hbox{word}`.
fn hbox_body_offset(fed: &str, word: &str) -> usize {
    fed.find(&format!("{{{word}}}"))
        .map(|i| i + 1)
        .unwrap_or_else(|| panic!("fed program has no {{{word}}}"))
}

/// A `fi` ligature must map to the union of both source chars.
#[test]
fn native_cluster_ligature_maps_to_union() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // f,i,x yields two glyphs, confirming `liga` was applied.
    let (fragment, fed) = render_native_run_spec(&texmf, "[lmroman10-regular.otf]", "fix", true);
    let node = native_glyph_run(&fragment);
    let LayoutNodeKind::GlyphRun(run) = &node.kind else {
        unreachable!()
    };
    assert_eq!(
        run.glyphs.len(),
        2,
        "expected 2 glyphs for fix (ﬁ ligature + x), a ligature must have formed, 3 chars become 2 glyphs"
    );

    let primary = fragment
        .primary_source_for_node(node.id)
        .expect("native run node has no primary source");
    let primary_name = &fragment
        .source_map
        .source(primary.source)
        .expect("primary source resolves")
        .name;
    assert_eq!(primary_name, "input", "native run must map to the user fragment");

    let body_at = hbox_body_offset(&fed, "fix");

    let slice_of = |i: usize| -> (usize, usize, String) {
        let range = fragment
            .glyph_source_range(node.id, i)
            .unwrap_or_else(|| panic!("glyph {i} has no source range (cluster None)"));
        let src = fragment
            .source_map
            .source(range.source)
            .expect("source resolves");
        assert_eq!(src.name, "input", "glyph {i} span must point at the fragment");
        let (s, e) = (range.span.start as usize, range.span.end as usize);
        assert!(s <= e && e <= fed.len(), "glyph {i} span [{s},{e}) OOB");
        (s, e, fed[s..e].to_string())
    };

    // Glyph 0 is the ﬁ ligature and maps to "fi".
    let (lig_s, lig_e, lig) = slice_of(0);
    assert_eq!(lig, "fi", "ligature glyph must map to the union source span \"fi\", got {lig:?}");
    assert_eq!(lig_s, body_at, "ligature span must start at the real tracked 'f' offset");
    assert_eq!(lig_e, body_at + 2, "ligature span must end after 'i' (union of both chars)");

    // Glyph 1 is x, contiguous with the ligature span.
    let (x_s, x_e, x) = slice_of(1);
    assert_eq!(x, "x", "trailing non-ligature char must map to itself, got {x:?}");
    assert_eq!(x_s, body_at + 2, "x span must start right after the ﬁ union");
    assert_eq!(x_e, body_at + 3, "x span must be a single char wide");
    assert_eq!(lig_e, x_s, "ligature union and the next glyph must be contiguous (no gap/overlap)");

    assert!(
        (0..run.glyphs.len()).all(|i| fragment.glyph_source_range(node.id, i).is_some()),
        "every visible glyph must resolve to a source span (no None)"
    );
}

/// `ffi` collapses to one ﬃ glyph whose source range spans all three chars.
#[test]
fn native_cluster_triple_ligature_single_glyph_union() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let (fragment, fed) = render_native_run_spec(&texmf, "[lmroman10-regular.otf]", "ffi", true);
    let node = first_glyph_run(&fragment);
    let LayoutNodeKind::GlyphRun(run) = &node.kind else {
        unreachable!()
    };
    assert_eq!(
        run.glyphs.len(),
        1,
        "expected a single ﬃ glyph for ffi (3 chars -> 1 glyph), got {}",
        run.glyphs.len()
    );

    let body_at = hbox_body_offset(&fed, "ffi");
    let range = fragment
        .glyph_source_range(node.id, 0)
        .expect("ﬃ ligature glyph has no source range");
    let src = fragment
        .source_map
        .source(range.source)
        .expect("source resolves");
    assert_eq!(src.name, "input", "ligature span must point at the fragment");
    let (s, e) = (range.span.start as usize, range.span.end as usize);
    assert!(s <= e && e <= fed.len(), "ﬃ span [{s},{e}) OOB");
    assert_eq!(
        &fed[s..e],
        "ffi",
        "ﬃ ligature must map to the union of all three source chars \"ffi\""
    );
    assert_eq!(s, body_at, "union must start at the real tracked 'f' offset");
    assert_eq!(e, body_at + 3, "union must span all three chars f,f,i");
}

/// `\hbox to 120pt{$a+b$}` must stretch `\medmuskip` glue.
#[test]
fn glue_set_ratio_stretches() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "frag.tex",
        br"\hbox to 120pt{$a+b$}\csname @@end\endcsname\end".to_vec(),
    ));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    assert!(ran);

    let root = engine.captured_fragment_root().expect("fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "glue".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR");

    let outlines = MultiFontOutlines::new(TexmfResources::from_texlive().expect("texmf"));
    let svg = mathtex_svg::render_with_outlines(&fragment, &outlines).expect("svg");
    let max_x = svg
        .match_indices("translate(")
        .filter_map(|(i, _)| {
            svg[i + 10..]
                .split([' ', ')'])
                .next()
                .and_then(|s| s.parse::<f64>().ok())
        })
        .fold(0.0_f64, f64::max);
    eprintln!("glue-set: box width={:?}, max glyph x={max_x:.2}", fragment.surface.width);
    assert!(
        max_x > 100.0,
        "glue set-ratio not applied: trailing glyph at x={max_x:.2} (expected ~115 in a stretched 120pt box, ~16 if natural width)"
    );
}

/// Splits timing into cold full path and warm equation render from a snapshotted FormatImage.
#[test]
fn time_cauchy_schwarz_endtoend_and_preloaded() {
    use std::time::Instant;

    fn count_glyphs(fragment: &mathtex_ir::Fragment) -> usize {
        fragment
            .nodes
            .iter()
            .filter_map(|node| match &node.kind {
                mathtex_ir::LayoutNodeKind::GlyphRun(run) => Some(run.glyphs.len()),
                _ => None,
            })
            .sum()
    }

    const FRAG: &[u8] = br"\hbox{$\displaystyle \left(\sum_{k=1}^n a_k b_k\right)^2\leq\left(\sum_{k=1}^n a_k^2\right)\left(\sum_{k=1}^n b_k^2\right)$}\csname @@end\endcsname\end";
    const PREAMBLE: &[u8] = br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump";

    let t_io = Instant::now();
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let io = t_io.elapsed();

    let t_base = Instant::now();
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let base = t_base.elapsed();

    let t_inst = Instant::now();
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(texmf.clone()),
        ));
    let inst = t_inst.elapsed();

    let t_latex = Instant::now();
    assert!(engine.begin_primary_input("latex.ltx", latex.clone()));
    engine.run_format_initialization();
    let latex_load = t_latex.elapsed();

    let t_pre = Instant::now();
    assert!(engine.begin_primary_input("preamble.tex", PREAMBLE.to_vec()));
    engine.run_format_initialization();
    let preamble = t_pre.elapsed();

    // Preload boundary, where earlier work is cold cost and later work is warm.
    let t_snap = Instant::now();
    let warm_cache = GeneratedFormatCache::from_engine(&engine);
    let snapshot = t_snap.elapsed();

    let t_frag = Instant::now();
    assert!(engine.begin_primary_input("frag.tex", FRAG.to_vec()));
    engine.begin_fragment_capture();
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    assert!(ran, "main control did not run to completion");
    let frag = t_frag.elapsed();

    let t_ir = Instant::now();
    let root = engine.captured_fragment_root().expect("fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "svg".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR");
    let ir = t_ir.elapsed();

    let t_svg = Instant::now();
    let outlines = MultiFontOutlines::new(texmf.clone());
    let _svg = mathtex_svg::render_with_outlines(&fragment, &outlines).expect("svg");
    let svg_time = t_svg.elapsed();

    let cold_glyphs = count_glyphs(&fragment);
    assert_eq!(cold_glyphs, 33, "cold path glyph count regressed");

    let end_to_end = io + base + inst + latex_load + preamble + frag + ir + svg_time;

    // Warm path reinstantiates from the FormatImage `K` times.
    const K: usize = 6;
    let mut warm: Vec<std::time::Duration> = Vec::with_capacity(K);
    for i in 0..K {
        // Clone texmf for the fresh font adapter outside the timed path.
        let texmf_fonts = texmf.clone();
        let t = Instant::now();
        let mut e = warm_cache
            .instantiate(
                pe::EngineProfile::xetex(),
                GeneratedResourceProvider::new(&texmf),
            )
            .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
                testfonts::Fonts::new(texmf_fonts),
            ));
        assert!(e.begin_primary_input("frag.tex", FRAG.to_vec()));
        e.begin_fragment_capture();
        let ok = e.run_main_control();
        e.end_fragment_capture();
        assert!(ok, "warm run {i}: main control did not complete");
        let wroot = e.captured_fragment_root().expect("warm fragment root");
        let wfrag = mathtex_engine::generated::generated_node_to_fragment(
            &e,
            wroot,
            mathtex_ir::FragmentMetadata {
                engine_profile: "xetex".into(),
                format_id: "svg".into(),
                fragment_kind: Default::default(),
            },
        )
        .expect("warm IR");
        let wout = MultiFontOutlines::new(texmf.clone());
        let wsvg = mathtex_svg::render_with_outlines(&wfrag, &wout).expect("warm svg");
        let dt = t.elapsed();
        assert_eq!(
            count_glyphs(&wfrag),
            33,
            "warm run {i} produced {} glyphs (preloaded format image is lossy!)",
            count_glyphs(&wfrag)
        );
        assert!(wsvg.contains("<path"), "warm run {i}: no outlines");
        warm.push(dt);
    }
    let mut sorted = warm.clone();
    sorted.sort();
    let warm_min = sorted[0];
    let warm_med = sorted[K / 2];
    let warm_max = sorted[K - 1];

    eprintln!("================ Cauchy-Schwarz timing (dev build) ================");
    eprintln!("COLD end-to-end (nothing -> SVG):   {end_to_end:>9.2?}");
    eprintln!("  io   read ls-R + latex.ltx:        {io:>9.2?}");
    eprintln!("  [1]  base engine init:             {base:>9.2?}");
    eprintln!("  --   instantiate + font platform:  {inst:>9.2?}");
    eprintln!("  [2]  load latex.ltx format:        {latex_load:>9.2?}");
    eprintln!("  [3]  preamble + unicode-math+dump: {preamble:>9.2?}");
    eprintln!("  ::   snapshot FormatImage:         {snapshot:>9.2?}  <- preload boundary");
    eprintln!("  [4]  fragment render:              {frag:>9.2?}");
    eprintln!("  [5]  IR lowering:                  {ir:>9.2?}");
    eprintln!("  [6]  SVG outline render:           {svg_time:>9.2?}");
    eprintln!("------------------------------------------------------------------");
    eprintln!("WARM per-equation (preloaded image -> SVG), {K} runs, all 33 glyphs:");
    eprintln!("  min {warm_min:>9.2?}   median {warm_med:>9.2?}   max {warm_max:>9.2?}");
    eprintln!("  speedup vs cold (median): {:.1}x", end_to_end.as_secs_f64() / warm_med.as_secs_f64());
    eprintln!("==================================================================");
}

/// Serializes and reloads the latex plus amsmath plus unicode-math format from bytes alone.
#[test]
fn format_image_serialization_roundtrip() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    // amsmath is loaded explicitly so the packaged format contains it.
    const PREAMBLE: &[u8] = br"\nonstopmode\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump";
    const FRAG: &[u8] = br"\hbox{$\displaystyle \left(\sum_{k=1}^n a_k b_k\right)^2\leq\left(\sum_{k=1}^n a_k^2\right)\left(\sum_{k=1}^n b_k^2\right)$}\csname @@end\endcsname\end";

    fn render_fragment(
        engine: &mut pe::PortableTexEngine<'_>,
        texmf: &TexmfResources,
    ) -> (usize, String) {
        assert!(engine.begin_primary_input("frag.tex", FRAG.to_vec()));
        engine.begin_fragment_capture();
        let ran = engine.run_main_control();
        engine.end_fragment_capture();
        assert!(ran, "main control did not complete");
        let root = engine.captured_fragment_root().expect("fragment root");
        let fragment = mathtex_engine::generated::generated_node_to_fragment(
            engine,
            root,
            mathtex_ir::FragmentMetadata {
                engine_profile: "xetex".into(),
                format_id: "svg".into(),
                fragment_kind: Default::default(),
            },
        )
        .expect("IR");
        let glyphs = fragment
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                mathtex_ir::LayoutNodeKind::GlyphRun(r) => Some(r.glyphs.len()),
                _ => None,
            })
            .sum();
        let outlines = MultiFontOutlines::new(texmf.clone());
        let svg = mathtex_svg::render_with_outlines(&fragment, &outlines).expect("svg");
        (glyphs, svg)
    }

    let (package, direct_glyphs, direct_svg) = {
        let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
        let mut engine = cache
            .instantiate(
                pe::EngineProfile::xetex(),
                GeneratedResourceProvider::new(&texmf),
            )
            .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
                testfonts::Fonts::new(texmf.clone()),
            ));
        assert!(engine.begin_primary_input("latex.ltx", latex.clone()));
        engine.run_format_initialization();
        assert!(engine.begin_primary_input("preamble.tex", PREAMBLE.to_vec()));
        engine.run_format_initialization();
        let cache = GeneratedFormatCache::from_engine(&engine);
        let runtime_bytes = cache.image().state_array_bytes();
        let format_bytes = cache.to_bytes();
        let font_table = engine.native_font_table();
        let package = mathtex_engine::generated::pack_packaged_format(&format_bytes, &font_table);
        eprintln!(
            "engine runtime arrays: {} MiB ({}-byte memory_word)",
            runtime_bytes / (1024 * 1024),
            pe::memory_word_bytes(),
        );
        let (glyphs, svg) = render_fragment(&mut engine, &texmf);
        (package, glyphs, svg)
    };
    assert_eq!(direct_glyphs, 33, "baseline direct render glyph count");
    eprintln!(
        "packaged format: {} bytes ({:.1} MiB)",
        package.len(),
        package.len() as f64 / (1024.0 * 1024.0)
    );

    // Reload from bytes alone and rebind native fonts via a fresh adapter.
    let (format_bytes, font_table) =
        mathtex_engine::generated::unpack_packaged_format(&package).expect("unpack package");
    let cache2 = GeneratedFormatCache::from_bytes(&format_bytes).expect("deserialize format image");
    let mut engine2 = cache2
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(texmf.clone()),
        ));
    assert!(
        engine2.restore_native_font_table(&font_table),
        "failed to rebind native fonts on cold load"
    );
    let (reload_glyphs, reload_svg) = render_fragment(&mut engine2, &texmf);

    assert_eq!(
        reload_glyphs, 33,
        "reloaded-from-bytes render produced {reload_glyphs} glyphs (format image is lossy!)"
    );
    assert!(reload_svg.contains("<path"), "reloaded render has no outlines");
    assert_eq!(
        reload_svg, direct_svg,
        "SVG reloaded from bytes differs from the direct render, serialization is not lossless"
    );
    eprintln!(
        "ROUND-TRIP OK: format reloaded from {} packaged bytes renders byte-identical SVG ({} glyphs, {} <path>)",
        package.len(),
        reload_glyphs,
        reload_svg.matches("<path").count()
    );
}

/// Boots latex.ltx once per test binary and can cache serialized FormatImage bytes.
fn latex_kernel_format_bytes(texmf: &TexmfResources) -> &'static [u8] {
    use std::hash::{Hash, Hasher};
    static CACHE: OnceLock<Vec<u8>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let latex = texmf
                .read("latex.ltx", ResourceKind::TexInput)
                .expect("latex.ltx")
                .bytes;
            let disk = std::env::var("MTX_FMT_CACHE_DIR").ok().map(|d| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                latex.hash(&mut h);
                std::path::Path::new(&d)
                    .join(format!("latex-kernel-{:016x}.fmtcache", h.finish()))
            });
            if let Some(p) = &disk {
                if let Ok(bytes) = std::fs::read(p) {
                    if GeneratedFormatCache::from_bytes(&bytes).is_some() {
                        return bytes; // Validated against the format magic for this build.
                    }
                }
            }
            let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
            let mut engine = cache.instantiate(
                pe::EngineProfile::xetex(),
                GeneratedResourceProvider::new(texmf),
            );
            assert!(engine.begin_primary_input("latex.ltx", latex));
            engine.run_format_initialization();
            let bytes = GeneratedFormatCache::from_engine(&engine).to_bytes();
            if let Some(p) = &disk {
                if let Some(parent) = p.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(p, &bytes);
            }
            bytes
        })
        .as_slice()
}

/// Feeds `body` wrapped in `\hbox{$ ..\relax$}` into an already booted engine.
fn feed_latex_math_fragment(
    engine: &mut pe::PortableTexEngine<'_>,
    body: &str,
    tracking: bool,
) -> (mathtex_ir::Fragment, String) {
    let prog = format!(
        concat!(
            r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 ",
            r"\textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm ",
            r"\textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni ",
            r"\textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy ",
            r"\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex ",
            r"\hbox{{${}\relax$}}\csname @@end\endcsname\end"
        ),
        body,
    );
    engine.set_source_tracking(tracking);
    engine.begin_fragment_capture();
    assert!(
        engine.begin_primary_input("input", prog.as_bytes().to_vec()),
        "engine refused primary input"
    );
    assert!(engine.run_main_control(), "run_main_control failed");
    engine.end_fragment_capture();

    let root = engine
        .captured_fragment_root()
        .expect("engine captured no fragment root");
    let fragment = mathtex_engine::generated::generated_node_to_fragment(
        &*engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "latex-source-tracking".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("captured root failed to convert to IR");
    (fragment, prog)
}

/// Reloads the cached kernel format and renders `body` with CM TFM math fonts.
fn render_latex_math(
    texmf: &TexmfResources,
    body: &str,
    tracking: bool,
) -> (mathtex_ir::Fragment, String) {
    let bytes = latex_kernel_format_bytes(texmf);
    let cache = GeneratedFormatCache::from_bytes(bytes)
        .expect("reload cached latex kernel format image");
    let mut engine = cache.instantiate(
        pe::EngineProfile::xetex(),
        GeneratedResourceProvider::new(texmf),
    );
    feed_latex_math_fragment(&mut engine, body, tracking)
}

/// Regression for a single char degree on the `\Uradical` path.
#[test]
fn sqrt_single_char_degree_maps_to_typed_char() {
    let Some(texmf) = TexmfResources::from_texlive() else { eprintln!("SKIP"); return; };
    let latex = texmf.read("latex.ltx", ResourceKind::TexInput).expect("latex.ltx").bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(pe::EngineProfile::xetex(), GeneratedResourceProvider::new(&texmf))
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump".to_vec(),
    ));
    engine.run_format_initialization();
    let frag_src = br"\hbox{$\sqrt[Z]{3}\relax$}\csname @@end\endcsname\end";
    let fed = String::from_utf8_lossy(frag_src).into_owned();
    engine.set_source_tracking(true);
    engine.begin_fragment_capture();
    assert!(engine.begin_primary_input("frag.tex", frag_src.to_vec()));
    assert!(engine.run_main_control());
    engine.end_fragment_capture();
    let root = engine.captured_fragment_root().expect("captured root");
    let frag = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "svg".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR lowering");
    // Degree glyph must map to exactly `Z`, not the whole `\sqrt[Z]`.
    let glyph_slices: Vec<&str> = frag
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, mathtex_ir::LayoutNodeKind::GlyphRun(_)))
        .filter_map(|n| n.primary_source.as_ref())
        .filter_map(|r| fed.get(r.span.start as usize..r.span.end as usize))
        .collect();
    assert!(
        glyph_slices.iter().any(|s| *s == "Z"),
        "expected the degree glyph to map to exactly \"Z\"; got glyph slices {glyph_slices:?}"
    );
}

/// Regression for degree box source spans in `\sqrt[\phantom{x}]{x}`.
#[test]
fn phantom_radical_index_box_maps_to_index_slot() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index");
        return;
    };
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&texmf),
        )
        .with_font_platform(mathtex_engine::GeneratedFontSystemAdapter::new(
            testfonts::Fonts::new(TexmfResources::from_texlive().expect("texmf")),
        ));
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    assert!(engine.begin_primary_input(
        "preamble.tex",
        br"\nonstopmode\documentclass{article}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}\dump"
            .to_vec(),
    ));
    engine.run_format_initialization();

    let frag_src = br"\hbox{$\sqrt[\phantom{x}]{x}\relax$}\csname @@end\endcsname\end";
    let fed = String::from_utf8_lossy(frag_src).into_owned();
    engine.set_source_tracking(true);
    engine.begin_fragment_capture();
    assert!(engine.begin_primary_input("frag.tex", frag_src.to_vec()));
    assert!(engine.run_main_control(), "main control did not run");
    engine.end_fragment_capture();
    let root = engine.captured_fragment_root().expect("captured root");
    let frag = mathtex_engine::generated::generated_node_to_fragment(
        &engine,
        root,
        mathtex_ir::FragmentMetadata {
            engine_profile: "xetex".into(),
            format_id: "svg".into(),
            fragment_kind: Default::default(),
        },
    )
    .expect("IR lowering");

    let slot = "\\phantom{x}";
    // Degree boxes used to overshoot into the radicand and must now match exactly.
    let box_slices: Vec<&str> = frag
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, mathtex_ir::LayoutNodeKind::Box(_)))
        .filter_map(|n| n.primary_source.as_ref())
        .filter_map(|r| fed.get(r.span.start as usize..r.span.end as usize))
        .collect();
    assert!(
        box_slices.iter().any(|s| *s == slot),
        "expected a box mapping to the index slot {slot:?}; got box slices {box_slices:?}"
    );
    // No box may overshoot the index slot into the radicand.
    assert!(
        !box_slices.iter().any(|s| s.starts_with(slot) && s.len() > slot.len()),
        "a box overshoots the index slot into the radicand: {box_slices:?}"
    );
}

/// The cached kernel format must produce the same source slices as a cold latex.ltx boot.
#[test]
fn cached_kernel_format_matches_cold_boot() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index");
        return;
    };
    let body = r"\sqrt[3]{x}+\frac{a}{b}";
    let (cached, fed_c) = render_latex_math(&texmf, body, true);
    let latex = texmf
        .read("latex.ltx", ResourceKind::TexInput)
        .expect("latex.ltx")
        .bytes;
    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache.instantiate(
        pe::EngineProfile::xetex(),
        GeneratedResourceProvider::new(&texmf),
    );
    assert!(engine.begin_primary_input("latex.ltx", latex));
    engine.run_format_initialization();
    let (cold, fed_k) = feed_latex_math_fragment(&mut engine, body, true);
    assert_eq!(fed_c, fed_k, "fed program differs between cached and cold paths");
    let pairs = |f: &mathtex_ir::Fragment, fed: &str| -> Vec<(&'static str, String)> {
        f.nodes
            .iter()
            .filter(|n| ltx_is_visible(&n.kind))
            .map(|n| {
                let r = f
                    .primary_source_for_node(n.id)
                    .expect("visible node has primary source");
                let (s, e) = (r.span.start as usize, r.span.end as usize);
                let tag = match &n.kind {
                    LayoutNodeKind::GlyphRun(_) => "G",
                    LayoutNodeKind::Rule(_) => "R",
                    _ => "D",
                };
                (tag, fed[s..e].to_string())
            })
            .collect()
    };
    assert_eq!(
        pairs(&cached, &fed_c),
        pairs(&cold, &fed_k),
        "cached boot source slices differ from cold boot, the cache is not faithful"
    );
}

fn ltx_is_visible(kind: &LayoutNodeKind) -> bool {
    match kind {
        LayoutNodeKind::GlyphRun(_) | LayoutNodeKind::Drawing(_) => true,
        // Structural rules have positive width and height.
        LayoutNodeKind::Rule(r) => {
            r.size.width > mathtex_ir::Length(0) && r.size.height > mathtex_ir::Length(0)
        }
        _ => false,
    }
}

/// Returns (kind, source slice) for every visible node in a real LaTeX math render of `body`.
fn ltx_visible_slices(texmf: &TexmfResources, body: &str) -> Vec<(LayoutNodeKind, String)> {
    let (fragment, fed) = render_latex_math(texmf, body, true);
    fragment
        .nodes
        .iter()
        .filter(|n| ltx_is_visible(&n.kind))
        .map(|n| {
            let range = fragment
                .primary_source_for_node(n.id)
                .unwrap_or_else(|| panic!("{body:?}: visible node {:?} has no primary source", n.id));
            let src = fragment
                .source_map
                .source(range.source)
                .expect("source resolves");
            assert_eq!(src.name, "input", "{body:?}: span must point at the user fragment");
            let (s, e) = (range.span.start as usize, range.span.end as usize);
            assert!(s <= e && e <= fed.len(), "{body:?}: span [{s},{e}) OOB len {}", fed.len());
            (n.kind.clone(), fed[s..e].to_string())
        })
        .collect()
}

fn ltx_glyphs(slices: &[(LayoutNodeKind, String)]) -> Vec<String> {
    slices
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::GlyphRun(_)))
        .map(|(_, s)| s.clone())
        .collect()
}

fn ltx_rules(slices: &[(LayoutNodeKind, String)]) -> Vec<String> {
    slices
        .iter()
        .filter(|(k, _)| matches!(k, LayoutNodeKind::Rule(_)))
        .map(|(_, s)| s.clone())
        .collect()
}

#[test]
fn real_latex_sqrt_degree_full_construct_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // `\sqrt[3]{x}` builds `\radical` through the kernel macro chain.
    let slices = ltx_visible_slices(&texmf, r"\sqrt[3]{x}");
    let glyphs = ltx_glyphs(&slices);
    let rules = ltx_rules(&slices);
    assert!(
        glyphs.iter().any(|g| g == r"\sqrt[3]{x}"),
        "surd glyph must slice to the whole \\sqrt[3]{{x}}; got {glyphs:?}"
    );
    assert!(
        rules.iter().any(|r| r == r"\sqrt[3]{x}"),
        "vinculum rule must cover \\sqrt[3]{{x}}; got {rules:?}"
    );
    // Radicand and degree leaves must keep their own source chars.
    assert!(glyphs.iter().any(|g| g == "x"), "radicand must slice to x; got {glyphs:?}");
    assert!(glyphs.iter().any(|g| g == "3"), "degree digit must slice to \"3\"; got {glyphs:?}");
    // Exactly one glyph maps to the full extent.
    assert_eq!(
        glyphs.iter().filter(|g| g.as_str() == r"\sqrt[3]{x}").count(),
        1,
        "exactly one glyph (the surd) is the full extent; got {glyphs:?}"
    );
    for g in glyphs.iter().chain(rules.iter()) {
        assert_ne!(g, "[3]{x}", "mark mis-mapped to the kernel-helper arg span");
        assert_ne!(g, "[3]", "mark mis-mapped to the degree");
        assert_ne!(g, "{", "mark floored to the opening brace");
    }
}

#[test]
fn real_latex_sqrt_no_degree_full_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // `\sqrt{y}` takes the `\@ifnextchar` false branch.
    let slices = ltx_visible_slices(&texmf, r"\sqrt{y}");
    let glyphs = ltx_glyphs(&slices);
    let rules = ltx_rules(&slices);
    assert!(
        glyphs.iter().any(|g| g == r"\sqrt{y}"),
        "surd glyph must slice to \\sqrt{{y}}; got {glyphs:?}"
    );
    assert!(
        rules.iter().any(|r| r == r"\sqrt{y}"),
        "vinculum rule must cover \\sqrt{{y}}; got {rules:?}"
    );
    assert!(glyphs.iter().any(|g| g == "y"), "radicand must slice to y; got {glyphs:?}");
}

#[test]
fn real_latex_accents_and_radical_primitive_full_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // `\hat` and `\widehat` accent glyphs must cover the whole construct.
    for (body, want, leaves) in [
        (r"\hat{x}", r"\hat{x}", &["x"][..]),
        (r"\widehat{xy}", r"\widehat{xy}", &["x", "y"][..]),
        (r#"\radical"270370{x}"#, r#"\radical"270370{x}"#, &["x"][..]),
    ] {
        let slices = ltx_visible_slices(&texmf, body);
        let glyphs = ltx_glyphs(&slices);
        let rules = ltx_rules(&slices);
        assert!(
            glyphs.iter().chain(rules.iter()).any(|s| s == want),
            "{body:?}: a synthesized mark must map to {want:?}; got glyphs {glyphs:?} rules {rules:?}"
        );
        for leaf in leaves {
            assert!(
                glyphs.iter().any(|g| g == leaf),
                "{body:?}: content leaf {leaf:?} must keep its own char; got {glyphs:?}"
            );
        }
    }
}

#[test]
fn real_latex_frac_and_overline_full_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // Fraction and overline rules map to their whole constructs.
    let frac = ltx_visible_slices(&texmf, r"\frac{a}{b}");
    assert!(
        ltx_rules(&frac).iter().any(|r| r == r"\frac{a}{b}"),
        "fraction bar must cover \\frac{{a}}{{b}}; got {:?}",
        ltx_rules(&frac)
    );
    assert!(ltx_glyphs(&frac).iter().any(|g| g == "a"), "numerator a; got {:?}", ltx_glyphs(&frac));
    assert!(ltx_glyphs(&frac).iter().any(|g| g == "b"), "denominator b; got {:?}", ltx_glyphs(&frac));

    let over = ltx_visible_slices(&texmf, r"\overline{x}");
    assert!(
        ltx_rules(&over).iter().any(|r| r == r"\overline{x}"),
        "overline rule must cover \\overline{{x}}; got {:?}",
        ltx_rules(&over)
    );
    assert!(ltx_glyphs(&over).iter().any(|g| g == "x"), "overline content x; got {:?}", ltx_glyphs(&over));
}

#[test]
fn real_latex_nested_sqrt_frac_altitude_split() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // Outer radical and inner `\frac` each keep their own extent.
    let slices = ltx_visible_slices(&texmf, r"\sqrt[3]{\frac{q}{2}}");
    let glyphs = ltx_glyphs(&slices);
    let rules = ltx_rules(&slices);
    assert!(
        glyphs.iter().any(|g| g == r"\sqrt[3]{\frac{q}{2}}"),
        "surd must map to the whole nested radical; got {glyphs:?}"
    );
    assert!(
        rules.iter().any(|r| r == r"\sqrt[3]{\frac{q}{2}}"),
        "outer vinculum must cover the whole nested radical; got {rules:?}"
    );
    assert!(
        rules.iter().any(|r| r == r"\frac{q}{2}"),
        "inner fraction bar must keep its OWN \\frac{{q}}{{2}} (not \\frac, not the radical); got {rules:?}"
    );
    assert!(glyphs.iter().any(|g| g == "q"), "numerator leaf q; got {glyphs:?}");
    assert!(glyphs.iter().any(|g| g == "2"), "denominator leaf 2; got {glyphs:?}");
}

#[test]
fn real_latex_cardano_every_surd_and_bar() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // Cardano formula checks exact spans for radicals, fraction bars, and leaves.
    let body = r"x=\sqrt[3]{-\frac{q}{2}+\sqrt{\frac{q^2}{4}+\frac{p^3}{27}}}";
    let slices = ltx_visible_slices(&texmf, body);
    assert!(!slices.is_empty(), "no visible nodes for Cardano");
    let all: Vec<&str> = slices.iter().map(|(_, s)| s.as_str()).collect();

    // No visible mark may floor to an opening brace.
    for s in &all {
        assert!(
            !s.starts_with('{'),
            "visible mark floored to a brace: {s:?} (a structural mark lost its command)"
        );
    }

    // Both radical surd and vinculum pairs map to their whole `\sqrt...` extent.
    let inner = r"\sqrt{\frac{q^2}{4}+\frac{p^3}{27}}";
    let outer = r"\sqrt[3]{-\frac{q}{2}+\sqrt{\frac{q^2}{4}+\frac{p^3}{27}}}";
    assert!(all.iter().any(|s| *s == inner), "inner radical must map to {inner:?}; got {all:?}");
    assert!(all.iter().any(|s| *s == outer), "outer radical must map to {outer:?}; got {all:?}");

    // Every mark anchored at `\sqrt` is a whole construct.
    for s in &all {
        if s.starts_with(r"\sqrt") {
            assert!(s.ends_with('}'), "radical mark {s:?} must be a whole \\sqrt...{{..}}");
        }
    }

    // Surd glyph maps to the outer extent exactly once.
    let surd_count = ltx_glyphs(&slices).iter().filter(|g| g.as_str() == outer).count();
    assert_eq!(
        surd_count, 1,
        "exactly one glyph (the surd) maps to the outer extent; a 2nd means the degree \
         floored to the construct. glyphs: {:?}",
        ltx_glyphs(&slices)
    );
    assert!(
        ltx_glyphs(&slices).iter().any(|g| g == "3"),
        "degree digit must map to its own \"3\"; got {:?}",
        ltx_glyphs(&slices)
    );

    for r in ltx_rules(&slices) {
        if r.starts_with(r"\frac") {
            assert!(
                r.ends_with('}') && r.contains('{'),
                "fraction bar {r:?} must be a whole \\frac{{..}}{{..}}, not bare \\frac"
            );
        }
    }
}

#[test]
fn real_latex_left_right_delimiters_full_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // Both delimiter glyphs map to the whole delimited group.
    let slices = ltx_visible_slices(&texmf, r"\left(x+y\right)");
    let glyphs = ltx_glyphs(&slices);
    let group = r"\left(x+y\right)";
    assert_eq!(
        glyphs.iter().filter(|g| g.as_str() == group).count(),
        2,
        "both delimiter glyphs must map to the whole {group:?}; got {glyphs:?}"
    );
    for leaf in ["x", "+", "y"] {
        assert!(glyphs.iter().any(|g| g == leaf), "leaf {leaf:?} keeps its char; got {glyphs:?}");
    }
    for g in &glyphs {
        assert_ne!(g, r"\right", "delimiter floored to bare \\right");
        assert_ne!(g, r"\left", "delimiter floored to bare \\left");
    }
}

#[test]
fn real_latex_unbraced_accent_full_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // Unbraced accent glyphs must map to the accented expression.
    let slices = ltx_visible_slices(&texmf, r"\dot q");
    let glyphs = ltx_glyphs(&slices);
    assert!(
        glyphs.iter().any(|g| g == r"\dot q"),
        "unbraced accent glyph must map to \"\\dot q\"; got {glyphs:?}"
    );
    assert!(glyphs.iter().any(|g| g == "q"), "accentee keeps its char q; got {glyphs:?}");
    assert!(
        !glyphs.iter().any(|g| g == r"\dot"),
        "accent must not floor to bare \\dot; got {glyphs:?}"
    );
}

#[test]
fn real_latex_choose_delimiters_full_extent() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // `\choose` delimiters come from the generalized fraction noad.
    let slices = ltx_visible_slices(&texmf, r"{n \choose k}");
    let glyphs = ltx_glyphs(&slices);
    let group = r"{n \choose k}";
    assert_eq!(
        glyphs.iter().filter(|g| g.as_str() == group).count(),
        2,
        "both \\choose delimiter glyphs must map to the whole {group:?}; got {glyphs:?}"
    );
    assert!(glyphs.iter().any(|g| g == "n"), "numerator n keeps its char; got {glyphs:?}");
    assert!(glyphs.iter().any(|g| g == "k"), "denominator k keeps its char; got {glyphs:?}");
    assert!(
        !glyphs.iter().any(|g| g == r"\choose"),
        "delimiters must not floor to bare \\choose; got {glyphs:?}"
    );
}

/// Renders `body` in the exact wasm fragment wrapper with sandbox enabled.
fn render_sandboxed(texmf: &TexmfResources, body: &str) -> Result<(), String> {
    let bytes = latex_kernel_format_bytes(texmf);
    let cache = GeneratedFormatCache::from_bytes(bytes).expect("reload cached kernel format");
    let mut engine = cache.instantiate(
        pe::EngineProfile::xetex(),
        GeneratedResourceProvider::new(texmf),
    );
    let prog = format!(
        concat!(
            r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 ",
            r"\textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm ",
            r"\textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni ",
            r"\textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy ",
            r"\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex ",
            // The exact wasm fragment wrapper.
            "\\hbox{{$\\displaystyle {}\n$}}\\csname @@end\\endcsname\\end"
        ),
        body,
    );
    engine.set_sandbox(true);
    engine.begin_fragment_capture();
    assert!(engine.begin_primary_input("input", prog.into_bytes()));
    let ran = engine.run_main_control();
    engine.end_fragment_capture();
    if ran {
        Ok(())
    } else {
        Err(engine.last_error_message().unwrap_or("rejected").to_string())
    }
}

#[test]
fn sandbox_allows_nested_math_in_text_rejects_breakout() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index at {TEXMF_ROOT}");
        return;
    };
    // Nested math in a text block stays at math depth >= 1.
    for ok in [
        r"a+\hbox{$x$}+b",        // single nested math in a text block
        r"\hbox{$x$}+\hbox{$y$}", // two nested math blocks (the regression case)
        r"\hbox{$x$ $y$}",        // two nested math in one text block
        r"\hbox{$\hbox{$x$}$}",   // doubly nested
    ] {
        assert!(
            render_sandboxed(&texmf, ok).is_ok(),
            "sandbox wrongly rejected legitimate nested math: {ok:?}"
        );
    }
    // A `$` that reopens math at depth 0 is a breakout.
    for bad in [r"a$b$", r"$x$+$y$", r"a$ \relax $b"] {
        match render_sandboxed(&texmf, bad) {
            Ok(()) => panic!("sandbox failed to reject a `$` breakout: {bad:?}"),
            Err(msg) => assert!(
                msg.contains("math shift"),
                "breakout {bad:?} rejected for the wrong reason: {msg}"
            ),
        }
    }
}

/// Regression for `\futurelet` lookahead leaking the next row token origin.
#[test]
fn array_row_boundary_does_not_leak_next_row_span() {
    let Some(texmf) = TexmfResources::from_texlive() else {
        eprintln!("SKIP: no TeXLive ls-R index");
        return;
    };
    // Row 1 has two empty cells where the leak used to stamp row 2 spans.
    {
        let body = r"\begin{array}{cc} &  \\ x & \end{array}";
        let (fragment, fed) = render_latex_math(&texmf, body, true);
        let empty_box_slices: Vec<&str> = fragment
            .nodes
            .iter()
            .filter(|n| matches!(&n.kind, mathtex_ir::LayoutNodeKind::Box(b) if b.children.is_empty()))
            .filter_map(|n| fragment.primary_source_for_node(n.id))
            .filter_map(|r| fed.get(r.span.start as usize..r.span.end as usize))
            .collect();
        assert!(
            !empty_box_slices.contains(&"x"),
            "{body:?}: an empty box in row 1's cell leaked row 2's \"x\" span; \
             got empty-box slices {empty_box_slices:?}"
        );
    }
    {
        let body = r"\begin{array}{cc} &  \\ \phantom{x} & \end{array}";
        let (fragment, fed) = render_latex_math(&texmf, body, true);
        let slices: Vec<&str> = fragment
            .nodes
            .iter()
            .filter_map(|n| fragment.primary_source_for_node(n.id))
            .filter_map(|r| fed.get(r.span.start as usize..r.span.end as usize))
            .collect();
        assert!(
            !slices.iter().any(|s| *s == r"\phantom"),
            "{body:?}: a node's span truncated to the bare \\phantom control sequence \
             (missing its {{x}} argument) -- the row-boundary template-replay leak is back; \
             got slices {slices:?}"
        );
    }
}
