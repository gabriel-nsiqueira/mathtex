//! Thin entry point, all patcher logic is in the `mathtex-web2c-import` library.

use std::process::ExitCode;

fn main() -> ExitCode {
    match mathtex_web2c_import::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("patch_engine: {error}");
            ExitCode::FAILURE
        }
    }
}
