#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

//! Facade crate for the mathtex engine, IR, and render backend traits.

pub use mathtex_engine as engine;
pub use mathtex_font as font;
pub use mathtex_ir as ir;
pub use mathtex_render as render;
pub use mathtex_svg as svg;
