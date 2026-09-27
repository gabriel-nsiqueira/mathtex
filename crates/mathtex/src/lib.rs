//! Facade crate for the mathtex engine, IR, fonts and SVG backend.
#![forbid(unsafe_code)]

pub use mathtex_engine as engine;
pub use mathtex_font as font;
pub use mathtex_ir as ir;
pub use mathtex_svg as svg;
