//! Converts a packaged format into a flat static format image.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some((input, output)) = paths() else {
        eprintln!("usage: bake_static_format <packaged input> <baked output>");
        return ExitCode::FAILURE;
    };
    let packaged = match std::fs::read(&input) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", input.display());
            return ExitCode::FAILURE;
        }
    };
    let Some(baked) = mathtex_engine::generated::bake_packaged_format(&packaged) else {
        eprintln!("input is not a packaged format for this build target");
        return ExitCode::FAILURE;
    };
    if let Err(error) = std::fs::write(&output, baked) {
        eprintln!("cannot write {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn paths() -> Option<(PathBuf, PathBuf)> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(args.next()?);
    let output = PathBuf::from(args.next()?);
    if args.next().is_some() {
        return None;
    }
    Some((input, output))
}
