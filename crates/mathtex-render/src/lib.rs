//! Render backend traits for converting layout IR to a concrete output format.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use mathtex_ir::{FontRef, Fragment, GlyphId, GlyphOutline};

/// Glyph contour source, font face parsing is amortized once per glyph run.
pub trait GlyphOutlineSource {
    /// Returns one outline per glyph in glyphs, None for missing glyphs or unavailable fonts.
    fn glyph_run_outlines(&self, font: &FontRef, glyphs: &[GlyphId]) -> Vec<Option<GlyphOutline>>;
}

/// Converts layout IR to a concrete output format.
pub trait RenderBackend {
    /// Concrete backend output, such as an SVG string or display list.
    type Output;
    /// Error type returned when rendering fails.
    type Error;

    /// Renders a layout fragment and returns the backend-specific output.
    fn render_fragment(&self, fragment: &Fragment) -> Result<Self::Output, Self::Error>;
}
