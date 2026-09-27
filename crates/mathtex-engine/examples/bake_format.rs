//! Bakes a LaTeX format with amsmath and unicode-math from a local TeX Live tree.

use std::path::PathBuf;
use std::process::ExitCode;

use mathtex_engine::{FormatBuilder, ResourceFontLoader, TexmfResources};

/// Everything a fragment may use must be loaded here, the sandbox refuses file access while typesetting.
const PREAMBLE: &str = r"\nonstopmode\documentclass{article}\usepackage{amsmath}\usepackage{unicode-math}\setmathfont{latinmodern-math.otf}\begin{document}";

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(output), None) = (args.next().map(PathBuf::from), args.next()) else {
        eprintln!("usage: bake_format <output.pkg>");
        return ExitCode::FAILURE;
    };
    let texmf = match TexmfResources::discover() {
        Ok(texmf) => texmf,
        Err(error) => {
            eprintln!("bake_format: {error}");
            return ExitCode::FAILURE;
        }
    };
    let fonts = ResourceFontLoader::new(&texmf);
    let format = match FormatBuilder::new(PREAMBLE)
        .kernel("latex.ltx")
        .build(&texmf, &fonts)
    {
        Ok(format) => format,
        Err(error) => {
            eprintln!("bake_format: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = std::fs::write(&output, &format) {
        eprintln!("bake_format: cannot write {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    eprintln!(
        "bake_format: wrote {} bytes to {}",
        format.len(),
        output.display()
    );
    ExitCode::SUCCESS
}
