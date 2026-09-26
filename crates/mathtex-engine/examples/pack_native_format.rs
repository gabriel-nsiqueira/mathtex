//! Builds the native packaged LaTeX format used by Notatus.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use mathtex_engine::font::{
    FontData, FontError, FontQuery, FontSystem, RustybuzzFontSystem, ShapeRequest, ShapedText,
};
use mathtex_engine::portable_engine as pe;
use mathtex_engine::{
    GeneratedFontSystemAdapter, GeneratedFormatCache, GeneratedResourceProvider,
    ProviderResourceRequest as ResourceRequest, Resource, ResourceError, ResourceFontSystem,
    ResourceKind, ResourceProvider, TexmfResources,
};

const TEXMF_ROOT_ENV: &str = "MATHTEX_TEXMF_ROOT";
const TEXMF_ROOT_CANDIDATES: &[&str] = &[
    "/usr/local/texlive/2026/texmf-dist",
    "/usr/local/texlive/2025/texmf-dist",
    "/Library/TeX/texmf-dist",
];
const PREAMBLE: &[u8] = br"\nonstopmode\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\def\hostbox#1{\Uhostbox #1\relax}\begin{document}\dump";

struct TexmfFontProvider {
    texmf: Arc<TexmfResources>,
}

impl TexmfFontProvider {
    fn font_name(spec: &str) -> String {
        let spec = spec.trim();
        if let Some(rest) = spec.strip_prefix('[') {
            return rest.split(']').next().unwrap_or(rest).trim().to_string();
        }
        spec.split([':', '/'])
            .next()
            .unwrap_or(spec)
            .trim()
            .to_string()
    }
}

impl ResourceProvider for TexmfFontProvider {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let name = Self::font_name(&request.canonical_name());
        self.texmf.read(&name, ResourceKind::Font)
    }
}

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

fn main() -> ExitCode {
    let Some(output) = output_path() else {
        eprintln!("usage: pack_native_format <output path>");
        return ExitCode::FAILURE;
    };
    let Some(texmf_root) = find_texmf_root() else {
        eprintln!("cannot find a texmf-dist ls-R index");
        return ExitCode::FAILURE;
    };
    let Some(texmf) = TexmfResources::from_root(texmf_root) else {
        eprintln!("cannot load the texmf-dist ls-R index");
        return ExitCode::FAILURE;
    };
    let texmf = Arc::new(texmf);

    let latex = match texmf.read("latex.ltx", ResourceKind::TexInput) {
        Ok(resource) => resource.bytes,
        Err(error) => {
            eprintln!("cannot read latex.ltx: {error:?}");
            return ExitCode::FAILURE;
        }
    };

    let cache = GeneratedFormatCache::initialized(pe::EngineProfile::xetex());
    let mut engine = cache
        .instantiate(
            pe::EngineProfile::xetex(),
            GeneratedResourceProvider::new(&*texmf),
        )
        .with_font_platform(GeneratedFontSystemAdapter::new(NativeFontSystem::new(
            texmf.clone(),
        )));

    if !engine.begin_primary_input("latex.ltx", latex) {
        eprintln!("failed to begin latex.ltx");
        return ExitCode::FAILURE;
    }
    engine.run_format_initialization();
    if !engine.begin_primary_input("preamble.tex", PREAMBLE.to_vec()) {
        eprintln!("failed to begin preamble");
        return ExitCode::FAILURE;
    }
    engine.run_format_initialization();

    engine.finalize_trie();
    let format_bytes = GeneratedFormatCache::from_engine(&engine).to_bytes();
    let font_table = engine.native_font_table();
    let packaged = mathtex_engine::generated::pack_packaged_format(&format_bytes, &font_table);
    if let Err(error) = std::fs::write(&output, packaged) {
        eprintln!("cannot write {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn output_path() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next()?);
    if args.next().is_some() {
        return None;
    }
    Some(output)
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
