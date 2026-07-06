//! Auto-patched portable engine source from Web2C/C2Rust bootstrap output.

pub mod runtime;
// Shared TeX core, split into per-symbol-group partial `impl` blocks under
// `functions/`. The union of those bodies is the complete shared core.
pub(crate) mod functions;
// Generated per-profile import metadata (id, kind, capabilities, source chain,
// pool, allowed host boundaries) for the tex/etex/xetex import profiles.
pub mod profiles;
// Profile-specific (xetex-only) patched functions live in `dispatch`.
pub(crate) mod dispatch;

pub use profiles::{
    ImportCapabilities, ImportProfile, ImportProfileKind, ImportStateBounds, IMPORT_PROFILES,
};

pub use runtime::memory_word_bytes;
pub use runtime::{
    EmptyFontPlatform, EmptyPlatform, EmptyResourceProvider, EngineProfile, EngineProfileKind,
    FontPlatform, GlyphAssembly, PortableClock, PortableFontHandle, PortableFontMetrics,
    PortableFormatImage, PortableLinebreakRequest, PortableMathAssemblyPart, PortableMathKernCorner,
    PortableMathVariant, PortableNativeGlyph, PortableNativeGlyphMetrics, PortableNativeTextMetrics,
    PortableNodeHandle, PortableNodeKind, PortableNodeSnapshot, PortableNodeSourceSpan,
    PortablePlatform, PortableResourceRequestRecord, PortableSourceSpan, PortableTexEngine,
    ResourceKind, ResourceProvider, ResourceRequest,
};
