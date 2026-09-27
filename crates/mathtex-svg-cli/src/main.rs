//! Typesets one math expression through the XeTeX engine and writes it as SVG.

use std::process::ExitCode;

use mathtex_engine::{
    Format, FormatBuilder, MathMode, NoHostBoxes, Options, ResourceFontLoader, TexmfResources,
    Typesetter,
};
use mathtex_ir::{FontKey, GlyphId, GlyphOutline};
use mathtex_svg::{GlyphOutlineSource, SvgOptions};

/// A small INITEX format with latinmodern-math in every family, used when no format file is given.
const PREAMBLE: &str = concat!(
    r"\catcode`\{=1 \catcode`\}=2 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \catcode`\#=6 ",
    "\\font\\mf=\"[latinmodern-math.otf]:script=math\" at 10pt ",
    r"\textfont0=\mf \scriptfont0=\mf \scriptscriptfont0=\mf ",
    r"\textfont1=\mf \scriptfont1=\mf \scriptscriptfont1=\mf ",
    r"\textfont2=\mf \scriptfont2=\mf \scriptscriptfont2=\mf ",
    r"\textfont3=\mf \scriptfont3=\mf \scriptscriptfont3=\mf ",
    "\\Udelcode`\\(=\"0 \"0028 \\Udelcode`\\)=\"0 \"0029 ",
    r"\def\frac#1#2{{#1\over#2}}",
    "\\def\\sqrt{\\Uradical\"0 \"221A }",
);

/// Draws glyphs from the faces the typesetter loaded, found by their keys.
struct LoadedOutlines<'a>(&'a ResourceFontLoader<&'a TexmfResources>);

impl GlyphOutlineSource for LoadedOutlines<'_> {
    fn glyph_outlines(&self, font: FontKey, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>> {
        self.0
            .font(font)
            .and_then(|font| font.glyph_outlines(glyphs).ok())
            .unwrap_or_else(|| vec![None; glyphs.len()])
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("mathtex-svg-cli: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let expression = args.next().unwrap_or_else(|| "x^2+y".into());
    let output = args.next().unwrap_or_else(|| "eq.svg".into());
    let format_file = args.next();
    // The typesetter supplies the math shifts, so dollars the caller passed are dropped.
    let expression = expression.trim().trim_matches('$');

    let texmf = TexmfResources::discover().map_err(|error| error.to_string())?;
    let fonts = ResourceFontLoader::new(&texmf);
    let format = match &format_file {
        Some(path) => {
            std::fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))?
        }
        None => FormatBuilder::new(PREAMBLE)
            .build(&texmf, &fonts)
            .map_err(|error| error.to_string())?,
    };
    let format = Format::from_owned(format).map_err(|error| error.to_string())?;

    let started = std::time::Instant::now();
    let mut typesetter =
        Typesetter::new(format, fonts, Options::default()).map_err(|error| error.to_string())?;
    let typeset = typesetter
        .typeset(expression, MathMode::Display, &NoHostBoxes)
        .map_err(|error| error.to_string())?;
    for warning in &typeset.warnings {
        eprintln!("warning: {}", warning.message);
    }
    let svg = mathtex_svg::render(
        &typeset.fragment,
        &LoadedOutlines(typesetter.fonts()),
        &SvgOptions::default(),
    );
    let elapsed = started.elapsed();
    std::fs::write(&output, &svg).map_err(|error| format!("cannot write {output}: {error}"))?;
    eprintln!(
        "{expression} -> {output}: {} nodes, {} bytes in {elapsed:?}",
        typeset.fragment.nodes.len(),
        svg.len()
    );
    Ok(())
}
