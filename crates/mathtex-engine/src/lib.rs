//! Portable TeX engine library for formats, sessions, IR emission, and host platform traits.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

/// IR emitter that translates TeX shipout calls to layout nodes.
pub mod emit;
/// Format image serialization, deserialization, and snapshot types.
pub mod format;
/// Code generated from the WEB source: engine tables, dispatch, and node types.
pub mod generated;
/// Host platform trait and bundled implementations for diagnostics, limits, and clocks.
pub mod platform;
/// Registry mapping primitive names to dispatch opcodes.
pub mod primitive;
/// Engine profile definitions: catcodes, primitives, semantics, and patches.
pub mod profile;
/// Resource provider trait and bundled implementations for fonts, packages, and inputs.
pub mod resource;
/// Font adapter that loads bytes through a resource provider.
pub mod resource_font;
/// Engine session construction, execution, and fragment rendering.
pub mod session;
#[cfg(feature = "std")]
/// Native filesystem resource provider backed by a TeX live tree and `ls-R` index.
pub mod texmf;

pub use emit::{EmitNode, IrEmitter};
pub use format::{
    FormatImage, FormatInitError, FormatInitializer, FormatPackage, FormatResource,
    FormatResourceKind, FormatSnapshot, FormatState, MacroDefinition, RegisterSnapshot,
    SessionState,
};
pub use generated::{
    generated_node_to_fragment, EmptyGeneratedFontPlatform, EmptyGeneratedPlatform, GeneratedClock,
    GeneratedFontHandle, GeneratedFontMetrics, GeneratedFontPlatform, GeneratedFontSystemAdapter,
    GeneratedFormatCache, GeneratedLayoutCapture, GeneratedLinebreakRequest, GeneratedNodeHandle,
    GeneratedNodeKind, GeneratedNodeSnapshot, GeneratedNodeSourceSpan, GeneratedPlatform,
    GeneratedPlatformAdapter, GeneratedResourceProvider, GeneratedResourceRequestRecord,
    GeneratedSourceSpan,
};
pub use mathtex_font as font;
pub use mathtex_portable_engine_generated as portable_engine;
pub use platform::{
    CollectingDiagnosticSink, ConfigurablePlatform, Diagnostic, DiagnosticSeverity, DiagnosticSink,
    HostClock, HostLimits, LimitError, LinebreakRequest, NoopDiagnosticSink, NoopPlatform,
    Platform,
};
pub use primitive::{PrimitiveEntry, PrimitiveRegistry, PrimitiveRegistryError};
pub use profile::{
    CatcodeDefaults, EngineKind, EnginePatch, EnginePatches, EngineProfile, EngineSemantics,
    EtexProfile, ExtensionPolicy, FontSemantics, MathcodeDefaults, PrimitiveKind, PrimitiveOpcode,
    PrimitiveSpec, ProfileId, RegisterDefaults, TexCore, TexProfile, XetexProfile,
};
#[cfg(feature = "std")]
pub use resource::FileSystemResourceProvider;
#[cfg(feature = "std")]
pub use texmf::TexmfResources;
pub use resource::{
    InMemoryResourceProvider, OverlayResourceProvider, ResolverResourceProvider, Resource,
    ResourceBundle, ResourceError, ResourceKind, ResourceProvider,
    ResourceRequest as ProviderResourceRequest, ResourceRequestSource,
};
pub use resource_font::ResourceFontSystem;
pub use session::{
    BuildError, Engine, EngineBuilder, EngineError, EngineSession, FormatPreloadError,
    FragmentInput,
};
