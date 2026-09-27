//! TeX Live and texlive-source fixture access, shared formats and small hosts for the integration tests.
#![allow(
    dead_code,
    reason = "each test binary uses a different subset of these helpers"
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::process::Command;
use std::sync::OnceLock;

use mathtex_engine::font::FontLoader;
use mathtex_engine::{
    Format, FormatBuilder, HostBox, HostBoxRequest, HostBoxes, InMemoryResourceProvider, MathMode,
    Options, ResourceFontLoader, ResourceKind, ResourceProvider, ResourceRequest, TexmfResources,
    Typeset, Typesetter, TEXMF_ROOT_ENV,
};
use mathtex_ir::{FontKey, Fragment, GlyphId, GlyphOutline, LayoutNodeKind, Length, NodeId};

/// Setting this to 1 turns every skip for a missing TeX Live tree or binary into a failure.
const REQUIRE_ENV: &str = "MATHTEX_REQUIRE_TEXLIVE";

fn required() -> bool {
    std::env::var(REQUIRE_ENV).as_deref() == Ok("1")
}

/// Skips the calling test with a notice, or fails it under `MATHTEX_REQUIRE_TEXLIVE=1`.
fn skip(reason: &str) {
    assert!(!required(), "{REQUIRE_ENV}=1 but {reason}");
    eprintln!("skipping: {reason}");
}

/// The TeX Live tree, parsed once per test binary.
pub fn texlive() -> Option<&'static TexmfResources> {
    static TEXMF: OnceLock<Option<TexmfResources>> = OnceLock::new();
    let texmf = TEXMF.get_or_init(|| match TexmfResources::discover() {
        Ok(texmf) => Some(texmf),
        // A configured root that does not load is a broken setup, not a machine without TeX Live.
        Err(error) if std::env::var_os(TEXMF_ROOT_ENV).is_some() => {
            panic!("{TEXMF_ROOT_ENV} is set but unusable: {error}")
        }
        Err(error) => {
            eprintln!("no TeX Live tree: {error}");
            None
        }
    });
    if texmf.is_none() {
        skip("no TeX Live tree was found");
    }
    texmf.as_ref()
}

/// Fonts loaded from the TeX Live tree.
pub type TexLiveFonts = ResourceFontLoader<&'static TexmfResources>;

/// The path of a TeX binary that runs, such as `tex` or `xetex`.
pub fn binary(name: &str) -> Option<String> {
    let found = [format!("/Library/TeX/texbin/{name}"), name.to_string()]
        .into_iter()
        .find(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        });
    if found.is_none() {
        skip(&format!("no {name} binary was found"));
    }
    found
}

/// Fonts loaded from the Computer Modern fixtures.
pub type CmFonts = ResourceFontLoader<&'static InMemoryResourceProvider>;

/// The Computer Modern math TFMs from the texlive-source fixtures, read once per test binary.
pub fn cm_fonts() -> Option<&'static InMemoryResourceProvider> {
    static FONTS: OnceLock<InMemoryResourceProvider> = OnceLock::new();
    mathtex_test_fixtures::root()?;
    Some(FONTS.get_or_init(|| {
        let mut fonts = InMemoryResourceProvider::new();
        for (name, file) in [
            ("cmr10.tfm", mathtex_test_fixtures::CMR10),
            ("cmmi10.tfm", mathtex_test_fixtures::CMMI10),
            ("cmsy10.tfm", mathtex_test_fixtures::CMSY10),
            ("cmex10.tfm", mathtex_test_fixtures::CMEX10),
        ] {
            let bytes = mathtex_test_fixtures::read(file).expect("the checkout was found");
            fonts.insert(ResourceRequest::new(name, ResourceKind::Font), bytes);
        }
        fonts
    }))
}

/// The [`CM_MATH_SETUP`] format over the Computer Modern fixtures, baked once per test binary.
pub fn cm_format() -> Option<&'static [u8]> {
    static FORMAT: OnceLock<Vec<u8>> = OnceLock::new();
    let fonts = cm_fonts()?;
    Some(FORMAT.get_or_init(|| bake(fonts, None, CM_MATH_SETUP)))
}

/// Bakes `preamble` into a format, reading inputs and fonts from `resources`.
pub fn bake<R: ResourceProvider>(resources: &R, kernel: Option<&str>, preamble: &str) -> Vec<u8> {
    let fonts = ResourceFontLoader::new(resources);
    let builder = FormatBuilder::new(preamble);
    let builder = match kernel {
        Some(kernel) => builder.kernel(kernel),
        None => builder,
    };
    builder
        .build(resources, &fonts)
        .unwrap_or_else(|error| panic!("the format bakes: {error}"))
}

/// The LaTeX, amsmath and unicode-math format hosts ship, baked once per test binary.
pub fn latex_format() -> Option<&'static [u8]> {
    static FORMAT: OnceLock<Vec<u8>> = OnceLock::new();
    let texmf = texlive()?;
    let bytes = FORMAT.get_or_init(|| {
        bake(
            texmf,
            Some("latex.ltx"),
            r"\nonstopmode\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}",
        )
    });
    Some(bytes)
}

/// Plain INITEX catcodes and Computer Modern math fonts from TFM files.
pub const CM_MATH_SETUP: &str = concat!(
    r"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 ",
    r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 ",
    r"\textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm ",
    r"\textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni ",
    r"\textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy ",
    r"\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex ",
    r"\def\text#1{\hbox{#1}}\def\frac#1#2{{#1\over#2}}",
);

/// A typesetter over bytes a test baked, fonts from `resources`.
pub fn typesetter<R: ResourceProvider>(
    resources: R,
    format: &[u8],
    options: Options,
) -> Typesetter<ResourceFontLoader<R>> {
    let format = Format::from_owned(format.to_vec()).expect("a freshly baked format loads");
    Typesetter::new(format, ResourceFontLoader::new(resources), options)
        .expect("the format's fonts load")
}

/// Typesets with no host boxes, panicking on an error.
pub fn typeset<L: FontLoader>(
    typesetter: &mut Typesetter<L>,
    tex: &str,
    mode: MathMode,
) -> Typeset {
    typesetter
        .typeset(tex, mode, &mathtex_engine::NoHostBoxes)
        .unwrap_or_else(|error| panic!("{tex:?} typesets: {error}"))
}

/// Hands every token `t` back as a box `t` points wide, 2pt high and 1pt deep, and records the requests.
#[derive(Default)]
pub struct TokenWideBoxes {
    pub requests: RefCell<Vec<HostBoxRequest>>,
    /// Token revisions, a token without one makes its fragments uncachable.
    pub revisions: BTreeMap<u32, u64>,
}

impl HostBoxes for TokenWideBoxes {
    fn host_box(&self, request: &HostBoxRequest) -> Option<HostBox> {
        self.requests.borrow_mut().push(*request);
        let pt = |points: i32| Length::from_scaled_points(points * 65_536);
        Some(HostBox::new(pt(request.token as i32), pt(2), pt(1)))
    }

    fn revision(&self, token: u32) -> Option<u64> {
        self.revisions.get(&token).copied()
    }
}

/// Draws the glyphs of fonts the typesetter loaded, found by their keys.
pub struct LoadedOutlines<'a>(pub &'a TexLiveFonts);

impl mathtex_svg::GlyphOutlineSource for LoadedOutlines<'_> {
    fn glyph_outlines(&self, font: FontKey, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>> {
        self.0
            .font(font)
            .and_then(|font| font.glyph_outlines(glyphs).ok())
            .unwrap_or_else(|| vec![None; glyphs.len()])
    }
}

/// Every glyph id in node order.
pub fn glyph_ids(fragment: &Fragment) -> Vec<u32> {
    fragment
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            LayoutNodeKind::GlyphRun(run) => Some(run.glyphs.iter().map(|glyph| glyph.glyph_id.0)),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The slice of `tex` a node's primary span covers.
pub fn primary_slice<'a>(fragment: &Fragment, tex: &'a str, node: NodeId) -> Option<&'a str> {
    let range = fragment.primary_source_for_node(node)?;
    let source = fragment.source_map.source(range.source)?;
    assert_eq!(source.name, mathtex_engine::FRAGMENT_SOURCE);
    tex.get(range.span.start as usize..range.span.end as usize)
}
