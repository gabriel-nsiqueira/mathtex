//! Thin entry point, all patcher logic is in the `mathtex-web2c-import` library.

fn main() -> Result<(), mathtex_web2c_import::PatchError> {
    mathtex_web2c_import::run()
}
