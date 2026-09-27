//! Auto-patched portable engine source from Web2C/C2Rust bootstrap output.

// Clippy lints the translated functions trigger, other lints stay on for the hand written runtime.
#![allow(
    clippy::assign_op_pattern,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::if_same_then_else,
    clippy::manual_range_patterns,
    clippy::needless_return,
    clippy::precedence,
    clippy::single_match,
    clippy::toplevel_ref_arg,
    clippy::unit_arg,
    clippy::unnecessary_cast,
    clippy::wildcard_in_or_patterns
)]

pub mod runtime;
// XeTeX engine functions, split into per letter partial `impl` blocks under `functions/`.
pub(crate) mod functions;

pub use runtime::memory_word_bytes;
pub use runtime::{
    EmptyFontPlatform, EmptyPlatform, EmptyResourceProvider, FontPlatform, FormatImageError,
    GlyphAssembly, PortableClock, PortableFontHandle, PortableFontMetrics, PortableFormatImage,
    PortableError, PortableErrorKind, PortableGlyphBounds, PortableHostBox, PortableHostBoxGlyph,
    PortableHostBoxRequest,
    PortableHostBoxRule, PortableHostBoxRun, PortableHostBoxStyle, PortableLinebreakRequest,
    PortableMathAssemblyPart, PortableMathKernCorner, PortableMathVariant, PortableNativeGlyph,
    PortableNativeGlyphMetrics, PortableNativeTextMetrics, PortableNoadKind, PortableNodeHandle,
    PortableNodeKind, PortableNodeSnapshot, PortablePlatform, PortableResourceRequestRecord,
    PortableSourceSpan, PortableTexEngine, ResourceKind, ResourceProvider, ResourceRequest,
    FILE_SIZE_PROBE_MODE, SANDBOX_OP_BUDGET,
};
