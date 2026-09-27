//! Typesets LaTeX math with a XeTeX engine: bake a [`Format`] once, then [`Typesetter::typeset`] fragments.
#![forbid(unsafe_code)]

mod adapter;
mod format;
mod host_box;
mod lower;
mod resource;
mod resource_font;
mod shaper;
mod texmf;
mod typeset;

pub use format::{BuildError, Format, FormatBuilder, FormatError};
pub use host_box::{
    HostBox, HostBoxGlyph, HostBoxRequest, HostBoxRule, HostBoxRun, HostBoxes, MathStyle,
    NoHostBoxes,
};
pub use mathtex_font as font;
pub use resource::{
    FileSystemResourceProvider, InMemoryResourceProvider, Resource, ResourceError, ResourceKind,
    ResourceProvider, ResourceRequest,
};
pub use resource_font::ResourceFontLoader;
pub use texmf::{TexmfError, TexmfResources, TEXMF_ROOT_ENV};
pub use typeset::{
    Diagnostic, DiagnosticKind, MathMode, Options, Typeset, TypesetError, Typesetter,
    FRAGMENT_SOURCE,
};
