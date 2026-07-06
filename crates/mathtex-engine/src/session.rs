use alloc::format;
use alloc::string::{String, ToString};

use mathtex_font::{
    FontData, FontError, FontQuery, FontSystem, NoFontSystem, ShapeRequest, ShapedText,
};
use mathtex_ir::{
    Fragment, FragmentKind, FragmentMetadata,
};

use crate::emit::IrEmitter;
use crate::format::{FormatImage, FormatResourceKind, SessionState};
use crate::generated::{
    generated_node_to_fragment, GeneratedFontSystemAdapter, GeneratedFormatCache,
    GeneratedPlatformAdapter, GeneratedResourceProvider,
};
use crate::platform::{LimitError, Platform};
use crate::primitive::{PrimitiveEntry, PrimitiveRegistry, PrimitiveRegistryError};
use crate::profile::{EngineKind, EngineProfile, EngineSemantics};
use crate::resource::{Resource, ResourceError, ResourceKind, ResourceProvider, ResourceRequest};

/// Engine initialized from a format image with associated resource, platform, and font boundaries.
#[derive(Clone, Debug)]
pub struct Engine<P, R, H, F = NoFontSystem> {
    profile: P,
    semantics: EngineSemantics,
    format: FormatImage,
    generated_format: GeneratedFormatCache,
    resources: R,
    platform: H,
    fonts: F,
    primitives: PrimitiveRegistry,
}

impl<P, R, H> Engine<P, R, H>
where
    P: EngineProfile,
    R: ResourceProvider,
    H: Platform,
{
    /// Constructs an engine without a font system.
    pub fn new(
        profile: P,
        format: FormatImage,
        resources: R,
        platform: H,
    ) -> Result<Self, BuildError> {
        let semantics = EngineSemantics::from_profile(&profile);
        let primitives = PrimitiveRegistry::from_specs(semantics.primitives())
            .map_err(BuildError::InvalidPrimitives)?;
        let generated_format = generated_format_for(&semantics, &format, &resources)
            .map_err(BuildError::FormatPreloadFailed)?;
        Ok(Self {
            profile,
            semantics,
            generated_format,
            format,
            resources,
            platform,
            fonts: NoFontSystem,
            primitives,
        })
    }
}

impl<P, R, H, F> Engine<P, R, H, F>
where
    P: EngineProfile,
    R: ResourceProvider,
    H: Platform,
    F: FontSystem,
{
    /// Constructs an engine with an explicit font system.
    pub fn new_with_fonts(
        profile: P,
        format: FormatImage,
        resources: R,
        platform: H,
        fonts: F,
    ) -> Result<Self, BuildError> {
        let semantics = EngineSemantics::from_profile(&profile);
        let primitives = PrimitiveRegistry::from_specs(semantics.primitives())
            .map_err(BuildError::InvalidPrimitives)?;
        let generated_format = generated_format_for(&semantics, &format, &resources)
            .map_err(BuildError::FormatPreloadFailed)?;
        Ok(Self {
            profile,
            semantics,
            generated_format,
            format,
            resources,
            platform,
            fonts,
            primitives,
        })
    }

    /// Returns a staged builder for this engine profile.
    pub fn builder(profile: P) -> EngineBuilder<P, (), (), NoFontSystem> {
        EngineBuilder::new(profile)
    }

    /// Create a session, the session holds a clone of the mutable format state.
    #[must_use]
    pub fn new_session(&self) -> EngineSession<'_, P, R, H, F> {
        EngineSession {
            engine: self,
            state: self.format.instantiate_session_state(),
        }
    }

    /// Returns the engine profile.
    #[must_use]
    pub fn profile(&self) -> &P {
        &self.profile
    }

    /// Computed semantics derived from the engine profile.
    #[must_use]
    pub fn semantics(&self) -> &EngineSemantics {
        &self.semantics
    }

    /// Returns the loaded format image.
    #[must_use]
    pub fn format(&self) -> &FormatImage {
        &self.format
    }

    /// Preloaded generated engine format image.
    #[must_use]
    pub fn generated_format(&self) -> &GeneratedFormatCache {
        &self.generated_format
    }

    /// Returns the resource provider.
    #[must_use]
    pub fn resources(&self) -> &R {
        &self.resources
    }

    /// Returns the platform adapter.
    #[must_use]
    pub fn platform(&self) -> &H {
        &self.platform
    }

    /// Returns the font system.
    #[must_use]
    pub fn fonts(&self) -> &F {
        &self.fonts
    }

    /// Primitive dispatch table for this engine profile.
    #[must_use]
    pub fn primitives(&self) -> &PrimitiveRegistry {
        &self.primitives
    }
}

/// Staged builder for constructing an engine with typed resource, platform, and font parameters.
#[derive(Clone, Debug)]
#[must_use]
pub struct EngineBuilder<P, R, H, F = NoFontSystem> {
    profile: P,
    format: Option<FormatImage>,
    resources: Option<R>,
    platform: Option<H>,
    fonts: F,
}

impl<P> EngineBuilder<P, (), (), NoFontSystem> {
    /// Creates a new builder for an engine with the given profile.
    pub fn new(profile: P) -> Self {
        Self {
            profile,
            format: None,
            resources: None,
            platform: None,
            fonts: NoFontSystem,
        }
    }
}

impl<P, R, H, F> EngineBuilder<P, R, H, F> {
    /// Sets the format image for the engine being built.
    pub fn format(mut self, format: FormatImage) -> Self {
        self.format = Some(format);
        self
    }

    /// Sets the resource provider for the engine being built.
    pub fn resources<NextR>(self, resources: NextR) -> EngineBuilder<P, NextR, H, F> {
        EngineBuilder {
            profile: self.profile,
            format: self.format,
            resources: Some(resources),
            platform: self.platform,
            fonts: self.fonts,
        }
    }

    /// Sets the platform adapter for the engine being built.
    pub fn platform<NextH>(self, platform: NextH) -> EngineBuilder<P, R, NextH, F> {
        EngineBuilder {
            profile: self.profile,
            format: self.format,
            resources: self.resources,
            platform: Some(platform),
            fonts: self.fonts,
        }
    }

    /// Sets the font system for the engine being built.
    pub fn fonts<NextF>(self, fonts: NextF) -> EngineBuilder<P, R, H, NextF> {
        EngineBuilder {
            profile: self.profile,
            format: self.format,
            resources: self.resources,
            platform: self.platform,
            fonts,
        }
    }
}

impl<P, R, H, F> EngineBuilder<P, R, H, F>
where
    P: EngineProfile,
    R: ResourceProvider,
    H: Platform,
    F: FontSystem,
{
    /// Validates the builder state and constructs an engine.
    pub fn build(self) -> Result<Engine<P, R, H, F>, BuildError> {
        let format = self.format.ok_or(BuildError::MissingFormat)?;
        if format.profile != self.profile.id() {
            return Err(BuildError::ProfileFormatMismatch {
                profile: self.profile.id().0,
                format_profile: format.profile.0,
            });
        }

        let resources = self.resources.expect("resources are required by type");
        let platform = self.platform.expect("platform is required by type");

        Engine::new_with_fonts(self.profile, format, resources, platform, self.fonts)
    }
}

/// Per fragment execution state cloned from the cached format at session creation.
#[derive(Clone, Debug)]
pub struct EngineSession<'a, P, R, H, F = NoFontSystem> {
    engine: &'a Engine<P, R, H, F>,
    state: SessionState,
}

impl<P, R, H, F> EngineSession<'_, P, R, H, F>
where
    P: EngineProfile,
    R: ResourceProvider,
    H: Platform,
    F: FontSystem,
{
    /// Typesets an inline math string and returns an IR fragment.
    pub fn layout_fragment(&mut self, input: &str) -> Result<Fragment, EngineError> {
        self.layout_fragment_input(FragmentInput::math_inline(input))
    }

    /// Typesets the given fragment input and returns an IR fragment.
    pub fn layout_fragment_input(&mut self, input: FragmentInput) -> Result<Fragment, EngineError> {
        self.check_input_limits(input.body.as_str())?;
        let generated_source = GeneratedFragmentSource::new(&input);
        let metadata = FragmentMetadata {
            engine_profile: self.engine.semantics.profile.0.to_string(),
            format_id: self.engine.format.id.clone(),
            fragment_kind: input.kind,
        };

        let mut generated = self.generated_engine();
        if !generated.begin_primary_input(
            input.source_name.as_str(),
            generated_source.tex.clone().into_bytes(),
        ) {
            return Err(EngineError::Layout {
                message: "generated TeX engine rejected primary input".to_string(),
            });
        }
        generated.begin_fragment_capture();
        let ran_main_control = generated.run_main_control();
        generated.end_fragment_capture();
        if !ran_main_control {
            // A surfaced TeX error carries a captured message, a bare fatal abort does not.
            let message = match generated.last_error_message() {
                Some(error) => format!("generated TeX engine reported an error: {error}"),
                None => generated_abort_error(generated.last_abort_status()),
            };
            return Err(EngineError::Layout { message });
        }

        self.check_resource_request_limits(generated.resource_request_count())?;
        if let Some(message) = generated_transcript_error(generated.transcript_bytes()) {
            return Err(EngineError::Layout { message });
        }

        let root = generated
            .captured_fragment_root()
            .ok_or_else(|| EngineError::Layout {
                message: "generated TeX engine did not capture a fragment root".to_string(),
            })?;
        let fragment =
            generated_node_to_fragment(&generated, root, metadata).ok_or_else(|| {
                EngineError::Layout {
                    message: "generated TeX engine could not convert captured fragment root"
                        .to_string(),
                }
            })?;
        self.check_layout_node_limits(fragment.nodes.len())?;
        Ok(fragment)
    }

    /// Instantiate the generated engine seeded from cached format state.
    #[must_use]
    pub fn generated_engine(&self) -> mathtex_portable_engine_generated::PortableTexEngine<'_> {
        self.engine
            .generated_format
            .instantiate(
                generated_profile_for(&self.engine.semantics),
                GeneratedResourceProvider::new(&self.engine.resources),
            )
            .with_platform(GeneratedPlatformAdapter::new(&self.engine.platform))
            .with_font_platform(GeneratedFontSystemAdapter::new(&self.engine.fonts))
    }

    /// Reads a TeX input file by name from the resource provider.
    pub fn load_input(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read(name, ResourceKind::TexInput)
    }

    /// Reads a TeX package by name from the resource provider.
    pub fn load_package(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read(name, ResourceKind::Package)
    }

    /// Reads a LaTeX document class by name from the resource provider.
    pub fn load_class(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_class(name)
    }

    /// Reads a font definition file by name from the resource provider.
    pub fn load_font_definition(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_font_definition(name)
    }

    /// Reads a package support file by name from the resource provider.
    pub fn load_package_support(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_package_support(name)
    }

    /// Reads a font file by name from the resource provider.
    pub fn load_font_resource(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_font(name)
    }

    /// Reads a font encoding file by name from the resource provider.
    pub fn load_encoding(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_encoding(name)
    }

    /// Reads a font map file by name from the resource provider.
    pub fn load_map(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_map(name)
    }

    /// Reads a TeX configuration resource by name from the resource provider.
    pub fn load_config(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_config(name)
    }

    /// Reads a package asset from the resource provider.
    pub fn load_asset(&self, package: &str, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_asset(package, name)
    }

    /// Reads a format image resource by name from the resource provider.
    pub fn load_format_image(&self, name: &str) -> Result<Resource, ResourceError> {
        self.engine.resources.read_format_image(name)
    }

    /// Loads a font matching the given query from the engine's font system.
    pub fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        self.engine.fonts.load_font(query)
    }

    /// Shapes a text run via the engine's font system.
    pub fn shape_text(&self, request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        self.engine.fonts.shape_text(request)
    }

    /// Returns the primitive dispatch entry for a control sequence name.
    #[must_use]
    pub fn primitive(&self, name: &str) -> Option<&PrimitiveEntry> {
        self.engine.primitives.get(name)
    }

    /// Computed semantics for this session.
    #[must_use]
    pub fn semantics(&self) -> &EngineSemantics {
        &self.engine.semantics
    }

    /// IR emitter seeded with session metadata, defaults to inline math fragment kind.
    #[must_use]
    pub fn ir_emitter(&self) -> IrEmitter {
        IrEmitter::new(FragmentMetadata {
            engine_profile: self.engine.semantics.profile.0.to_string(),
            format_id: self.engine.format.id.clone(),
            fragment_kind: FragmentKind::MathInline,
        })
    }

    /// IR emitter seeded with session metadata and the given fragment kind.
    #[must_use]
    pub fn ir_emitter_for(&self, kind: FragmentKind) -> IrEmitter {
        IrEmitter::new(FragmentMetadata {
            engine_profile: self.engine.semantics.profile.0.to_string(),
            format_id: self.engine.format.id.clone(),
            fragment_kind: kind,
        })
    }

    /// Session local state cloned from the cached format at session creation.
    #[must_use]
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Mutable access to the session's local state.
    #[must_use]
    pub fn state_mut(&mut self) -> &mut SessionState {
        &mut self.state
    }

    fn check_input_limits(&self, input: &str) -> Result<(), EngineError> {
        let actual = input.len();
        let limit = self.engine.platform.limits().max_input_bytes;
        if actual > limit {
            return Err(EngineError::Limit(LimitError::InputTooLarge {
                actual,
                limit,
            }));
        }
        Ok(())
    }

    fn check_resource_request_limits(&self, actual: usize) -> Result<(), EngineError> {
        let limit = self.engine.platform.limits().max_resource_requests;
        if actual > limit {
            return Err(EngineError::Limit(LimitError::TooManyResourceRequests {
                actual,
                limit,
            }));
        }
        Ok(())
    }

    fn check_layout_node_limits(&self, actual: usize) -> Result<(), EngineError> {
        let limit = self.engine.platform.limits().max_layout_nodes;
        if actual > limit {
            return Err(EngineError::Limit(LimitError::TooManyLayoutNodes {
                actual,
                limit,
            }));
        }
        Ok(())
    }
}

fn generated_profile_for(
    semantics: &EngineSemantics,
) -> mathtex_portable_engine_generated::EngineProfile {
    mathtex_portable_engine_generated::EngineProfile {
        id: semantics.profile.0,
        kind: match semantics.kind {
            EngineKind::Tex => mathtex_portable_engine_generated::EngineProfileKind::Tex,
            EngineKind::Etex => mathtex_portable_engine_generated::EngineProfileKind::Etex,
            EngineKind::Xetex => mathtex_portable_engine_generated::EngineProfileKind::Xetex,
        },
        etex: semantics.extensions.etex,
        xetex: semantics.extensions.xetex,
        unicode_scalars: semantics.catcodes.unicode_scalars,
        unicode_math: semantics.mathcodes.unicode_math,
        native_fonts: semantics.fonts.host_native_fonts,
    }
}

fn generated_format_for<R>(
    semantics: &EngineSemantics,
    format: &FormatImage,
    resources: &R,
) -> Result<GeneratedFormatCache, FormatPreloadError>
where
    R: ResourceProvider,
{
    let profile = generated_profile_for(semantics);
    let base = GeneratedFormatCache::initialized(profile);
    if !format_has_generated_preload_resources(format) {
        return Ok(base);
    }

    let provider = CachedGeneratedFormatResources { format, resources };
    let mut engine = base.instantiate(profile, GeneratedResourceProvider::new(provider));

    for resource in generated_format_preload_resources(format) {
        let mut bytes = Vec::with_capacity(resource.bytes.len() + GENERATED_FORMAT_FINALIZER.len());
        bytes.extend_from_slice(resource.bytes.as_slice());
        bytes.extend_from_slice(GENERATED_FORMAT_FINALIZER);
        if !engine.begin_primary_input(resource.name.as_str(), bytes) {
            return Err(FormatPreloadError::InputRejected {
                name: resource.name.clone(),
            });
        }
        let completed = engine.run_format_initialization();
        if let Some(request) = generated_failed_resource_request(&engine) {
            return Err(FormatPreloadError::Resource {
                name: resource.name.clone(),
                resource: request.name.clone(),
            });
        }
        if let Some(message) = generated_transcript_error(engine.transcript_bytes()) {
            return Err(FormatPreloadError::Transcript {
                name: resource.name.clone(),
                message,
            });
        }
        if !completed {
            if let Some(message) = generated_transcript_abort_error(engine.transcript_bytes()) {
                return Err(FormatPreloadError::Transcript {
                    name: resource.name.clone(),
                    message,
                });
            }
            return Err(FormatPreloadError::Aborted {
                name: resource.name.clone(),
                status: engine.last_abort_status(),
            });
        }
    }
    // All packages, hence all \patterns, are loaded. Pack the trie to drop ~24 MB of builder scratch.
    engine.finalize_trie();
    Ok(GeneratedFormatCache::from_engine_owned(engine))
}

const GENERATED_FORMAT_FINALIZER: &[u8] = br"\dump";

fn generated_failed_resource_request<'a>(
    engine: &'a mathtex_portable_engine_generated::PortableTexEngine<'_>,
) -> Option<&'a mathtex_portable_engine_generated::PortableResourceRequestRecord> {
    engine.resource_request_records().iter().find(|request| {
        request.byte_len.is_none() && !is_optional_generated_resource_probe(request)
    })
}

fn is_optional_generated_resource_probe(
    request: &mathtex_portable_engine_generated::PortableResourceRequestRecord,
) -> bool {
    let basename = generated_resource_probe_basename(request.name.as_str());
    basename == "texsys.aux" || basename == "babel-texput.cfg"
}

fn generated_resource_probe_basename(mut name: &str) -> &str {
    loop {
        if let Some(stripped) = name.strip_prefix("./") {
            name = stripped;
            continue;
        }
        if let Some(stripped) = name.strip_prefix("[]") {
            name = stripped;
            continue;
        }
        if let Some(stripped) = name.strip_prefix(':') {
            name = stripped;
            continue;
        }
        break name;
    }
}

fn generated_format_preload_resources(
    format: &FormatImage,
) -> impl Iterator<Item = &crate::format::FormatResource> {
    match &format.state {
        crate::format::FormatState::Structured(snapshot) => Some(snapshot.resources.iter()),
        _ => None,
    }
    .into_iter()
    .flatten()
    .filter(|resource| {
        matches!(
            resource.kind,
            FormatResourceKind::TexInput | FormatResourceKind::Package
        )
    })
}

fn format_has_generated_preload_resources(format: &FormatImage) -> bool {
    let crate::format::FormatState::Structured(snapshot) = &format.state else {
        return false;
    };
    snapshot.resources.iter().any(|resource| {
        matches!(
            resource.kind,
            FormatResourceKind::TexInput | FormatResourceKind::Package
        )
    })
}

#[derive(Clone, Copy, Debug)]
struct CachedGeneratedFormatResources<'a, R> {
    format: &'a FormatImage,
    resources: &'a R,
}

impl<R> ResourceProvider for CachedGeneratedFormatResources<'_, R>
where
    R: ResourceProvider,
{
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        if matches!(request.kind, ResourceKind::TexInput | ResourceKind::Package) {
            if let Some(resource) = cached_format_resource(self.format, request) {
                return Ok(resource);
            }
        }
        self.resources.read_request(request)
    }
}

fn cached_format_resource(format: &FormatImage, request: &ResourceRequest) -> Option<Resource> {
    let crate::format::FormatState::Structured(snapshot) = &format.state else {
        return None;
    };
    snapshot
        .resources
        .iter()
        .find(|resource| {
            format_resource_kind_to_resource_kind(resource.kind) == Some(request.kind)
                && resource.name == request.canonical_name()
        })
        .map(|resource| Resource {
            canonical_name: resource.name.clone(),
            kind: request.kind,
            bytes: resource.bytes.clone(),
        })
}

fn format_resource_kind_to_resource_kind(kind: FormatResourceKind) -> Option<ResourceKind> {
    match kind {
        FormatResourceKind::TexInput => Some(ResourceKind::TexInput),
        FormatResourceKind::Package => Some(ResourceKind::Package),
    }
}

struct GeneratedFragmentSource {
    tex: String,
}

impl GeneratedFragmentSource {
    fn new(input: &FragmentInput) -> Self {
        let mut tex = String::with_capacity(input.body.len() + 96);
        match input.kind {
            FragmentKind::Text => {
                tex.push_str(r"\hbox{");
            }
            FragmentKind::MathInline => {
                tex.push_str(r"\hbox{$");
            }
            FragmentKind::MathDisplay => {
                tex.push_str(r"\hbox{$\displaystyle ");
            }
            _ => {
                tex.push_str(r"\hbox{");
            }
        }

        tex.push_str(input.body.as_str());

        // Invoke primitive end under LaTeX, this is a harmless `\relax` under plain TeX.
        match input.kind {
            FragmentKind::Text => tex.push_str(r"\relax}\csname @@end\endcsname\end"),
            FragmentKind::MathInline | FragmentKind::MathDisplay => {
                tex.push_str(r"\relax$}\csname @@end\endcsname\end")
            }
            _ => tex.push_str(r"\relax}\csname @@end\endcsname\end"),
        }

        Self { tex }
    }
}

fn generated_transcript_error(transcript: &[u8]) -> Option<String> {
    let transcript = core::str::from_utf8(transcript).ok()?;
    for (error_start, _) in transcript.match_indices('!').rev() {
        let error_tail = &transcript[error_start..];
        let error = error_tail
            .lines()
            .next()
            .unwrap_or("generated TeX engine reported an error")
            .trim();
        if !error.starts_with("! ") {
            continue;
        }
        let has_line_context = error_tail
            .lines()
            .skip(1)
            .take(4)
            .any(|line| line.trim_start().starts_with("l."));
        let has_compact_line_context = error_tail
            .get(..error_tail.len().min(512))
            .is_some_and(|tail| tail.contains("l."));
        if has_line_context || has_compact_line_context || error.contains("Emergency stop") {
            return Some(format!("generated TeX engine reported an error: {error}"));
        }
    }
    None
}

fn generated_transcript_abort_error(transcript: &[u8]) -> Option<String> {
    let transcript = core::str::from_utf8(transcript).ok()?;
    transcript
        .match_indices('!')
        .rev()
        .find_map(|(error_start, _)| {
            let error = transcript[error_start..].lines().next()?.trim();
            error
                .starts_with("! ")
                .then(|| format!("generated TeX engine aborted after error: {error}"))
        })
}

fn generated_abort_error(status: Option<i32>) -> String {
    match status {
        Some(status) => format!("generated TeX engine aborted with status {status}"),
        None => "generated TeX engine aborted".to_string(),
    }
}

/// Typed fragment input for layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentInput {
    /// Source name used in source map entries.
    pub source_name: String,
    /// TeX source body to be typeset.
    pub body: String,
    /// Fragment kind controlling how the body is wrapped and parsed.
    pub kind: FragmentKind,
}

impl FragmentInput {
    /// Inline math input with source name "input".
    #[must_use]
    pub fn math_inline(body: impl Into<String>) -> Self {
        Self::new("input", body, FragmentKind::MathInline)
    }

    /// Display math input with source name "input".
    #[must_use]
    pub fn math_display(body: impl Into<String>) -> Self {
        Self::new("input", body, FragmentKind::MathDisplay)
    }

    /// Text input with source name "input".
    #[must_use]
    pub fn text(body: impl Into<String>) -> Self {
        Self::new("input", body, FragmentKind::Text)
    }

    /// Constructs a fragment input with the given source name, body, and fragment kind.
    #[must_use]
    pub fn new(
        source_name: impl Into<String>,
        body: impl Into<String>,
        kind: FragmentKind,
    ) -> Self {
        Self {
            source_name: source_name.into(),
            body: body.into(),
            kind,
        }
    }
}

/// Error returned when constructing an engine.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// No format image was supplied to the builder.
    MissingFormat,
    /// The primitive registry rejected the profile's primitive specifications.
    InvalidPrimitives(PrimitiveRegistryError),
    /// Format resource preloading failed during engine build.
    FormatPreloadFailed(FormatPreloadError),
    /// The format image's profile does not match the requested engine profile.
    ProfileFormatMismatch {
        /// Requested profile.
        profile: &'static str,
        /// Profile recorded in the format image.
        format_profile: &'static str,
    },
}

/// Error encountered while preloading format resources into the generated engine.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatPreloadError {
    /// The generated engine rejected the given input file.
    InputRejected {
        /// Name of the resource that was rejected.
        name: String,
    },
    /// The generated engine aborted before completing initialization.
    Aborted {
        /// Name of the resource being processed when the abort occurred.
        name: String,
        /// Exit status from the aborted engine, if available.
        status: Option<i32>,
    },
    /// A required resource could not be loaded during preload.
    Resource {
        /// Name of the format resource being processed.
        name: String,
        /// Name of the missing resource that was requested.
        resource: String,
    },
    /// The engine transcript contained a TeX error during preload.
    Transcript {
        /// Name of the format resource being processed when the error occurred.
        name: String,
        /// Error message extracted from the engine transcript.
        message: String,
    },
}

/// Error returned by layout operations.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineError {
    /// Platform limit exceeded during layout.
    Limit(LimitError),
    /// Input could not be parsed.
    Parse {
        /// Description of the parse error.
        message: String,
    },
    /// TeX engine reported a layout error.
    Layout {
        /// Description of the layout error.
        message: String,
    },
    /// A required resource could not be satisfied.
    Resource {
        /// Description of the resource error.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;
    use alloc::vec::Vec;

    use super::*;
    use crate::{
        EngineKind, EnginePatch, EngineProfile, ExtensionPolicy, FormatImage, FormatSnapshot,
        HostLimits, InMemoryResourceProvider, LimitError, MacroDefinition, NoopPlatform,
        PrimitiveKind, PrimitiveOpcode, PrimitiveRegistryError, PrimitiveSpec, ProfileId,
        ResourceFontSystem, TexProfile, XetexProfile,
    };
    use mathtex_font::InMemoryFontSystem;
    use mathtex_ir::{ByteSpan, Direction, FragmentKind, LayoutNodeKind, Length};

    const TEST_FORMAT_INPUT: &str = "mathtex-test-format.tex";
    const TEST_FORMAT_SETUP: &[u8] = br"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \def\usepackage#1{\input #1.sty\relax} \def\text#1{\hbox{#1}}";
    const TEST_MATH_FONT_SETUP: &[u8] = br"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \catcode`\$=3 \catcode`\^=7 \catcode`\_=8 \def\usepackage#1{\input #1.sty\relax} \def\text#1{\hbox{#1}} \font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 \textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm \textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni \textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy \textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex";

    fn test_format(
        id: &'static str,
        profile: ProfileId,
        setup: &'static [u8],
    ) -> (FormatImage, InMemoryResourceProvider) {
        let resources = InMemoryResourceProvider::new().with_resource(
            TEST_FORMAT_INPUT,
            ResourceKind::TexInput,
            setup,
        );
        let format = FormatImage::initializer(id, profile)
            .preload_tex_input(TEST_FORMAT_INPUT)
            .build(&resources)
            .expect("test format should preload setup");
        (format, resources)
    }

    fn basic_test_format(profile: ProfileId) -> (FormatImage, InMemoryResourceProvider) {
        test_format("latex", profile, TEST_FORMAT_SETUP)
    }

    fn math_test_format(profile: ProfileId) -> (FormatImage, InMemoryResourceProvider) {
        test_format("plain-math", profile, TEST_MATH_FONT_SETUP)
    }

    #[derive(Clone, Copy, Debug)]
    struct PlainFixtureResources;

    impl PlainFixtureResources {
        const TFMS: &'static [(&'static str, &'static [u8])] = &[
            (
                "cmbsy10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbsy10.tfm"),
            ),
            (
                "cmbx10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbx10.tfm"),
            ),
            (
                "cmbx5",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbx5.tfm"),
            ),
            (
                "cmbx6",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbx6.tfm"),
            ),
            (
                "cmbx7",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbx7.tfm"),
            ),
            (
                "cmbx8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbx8.tfm"),
            ),
            (
                "cmbx9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmbx9.tfm"),
            ),
            (
                "cmcsc10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmcsc10.tfm"),
            ),
            (
                "cmdunh10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmdunh10.tfm"),
            ),
            (
                "cmex10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmex10.tfm"),
            ),
            (
                "cmmi10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmi10.tfm"),
            ),
            (
                "cmmi5",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmi5.tfm"),
            ),
            (
                "cmmi6",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmi6.tfm"),
            ),
            (
                "cmmi7",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmi7.tfm"),
            ),
            (
                "cmmi8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmi8.tfm"),
            ),
            (
                "cmmi9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmi9.tfm"),
            ),
            (
                "cmmib10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmmib10.tfm"),
            ),
            (
                "cmr10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmr10.tfm"),
            ),
            (
                "cmr5",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmr5.tfm"),
            ),
            (
                "cmr6",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmr6.tfm"),
            ),
            (
                "cmr7",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmr7.tfm"),
            ),
            (
                "cmr8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmr8.tfm"),
            ),
            (
                "cmr9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmr9.tfm"),
            ),
            (
                "cmsl10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsl10.tfm"),
            ),
            (
                "cmsl8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsl8.tfm"),
            ),
            (
                "cmsl9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsl9.tfm"),
            ),
            (
                "cmsltt10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsltt10.tfm"),
            ),
            (
                "cmss10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmss10.tfm"),
            ),
            (
                "cmssbx10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmssbx10.tfm"),
            ),
            (
                "cmssi10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmssi10.tfm"),
            ),
            (
                "cmssq8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmssq8.tfm"),
            ),
            (
                "cmssqi8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmssqi8.tfm"),
            ),
            (
                "cmsy10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsy10.tfm"),
            ),
            (
                "cmsy5",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsy5.tfm"),
            ),
            (
                "cmsy6",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsy6.tfm"),
            ),
            (
                "cmsy7",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsy7.tfm"),
            ),
            (
                "cmsy8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsy8.tfm"),
            ),
            (
                "cmsy9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmsy9.tfm"),
            ),
            (
                "cmti10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmti10.tfm"),
            ),
            (
                "cmti7",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmti7.tfm"),
            ),
            (
                "cmti8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmti8.tfm"),
            ),
            (
                "cmti9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmti9.tfm"),
            ),
            (
                "cmtt10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmtt10.tfm"),
            ),
            (
                "cmtt8",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmtt8.tfm"),
            ),
            (
                "cmtt9",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmtt9.tfm"),
            ),
            (
                "cmu10",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/cmu10.tfm"),
            ),
            (
                "manfnt",
                include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/tfm/manfnt.tfm"),
            ),
        ];

        fn base_name(name: &str) -> &str {
            name.rsplit(['/', '\\']).next().unwrap_or(name)
        }

        fn font_name(name: &str) -> &str {
            let name = Self::base_name(name);
            name.strip_suffix(".tfm").unwrap_or(name)
        }
    }

    impl ResourceProvider for PlainFixtureResources {
        fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
            let name = request.canonical_name();
            let base_name = Self::base_name(name.as_str());
            let bytes = match request.kind {
                ResourceKind::TexInput if base_name == "plain.tex" => Some(
                    include_bytes!("../../../vendor/mathtex-fixtures/plain-tex/base/plain.tex")
                        .as_slice(),
                ),
                ResourceKind::TexInput if base_name == "hyphen" || base_name == "hyphen.tex" => {
                    Some(
                        include_bytes!(
                            "../../../vendor/mathtex-fixtures/plain-tex/hyphen/hyphen.tex"
                        )
                        .as_slice(),
                    )
                }
                ResourceKind::Font => {
                    let font_name = Self::font_name(base_name);
                    Self::TFMS
                        .iter()
                        .find_map(|(name, bytes)| (*name == font_name).then_some(*bytes))
                }
                _ => None,
            };

            bytes
                .map(|bytes| Resource::from_request(request, bytes.to_vec()))
                .ok_or_else(|| ResourceError::NotFound {
                    name,
                    kind: request.kind,
                })
        }
    }

    fn assert_real_plain_format_loads<R>(resources: R)
    where
        R: ResourceProvider + Copy,
    {
        let profile = TexProfile;
        let format = FormatImage::plain(profile.id())
            .build(&resources)
            .expect("real plain.tex should resolve");
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build with real plain format");
        let mut session = engine.new_session();

        let fragment = session
            .layout_fragment_input(FragmentInput::text(r"\tenrm A"))
            .expect("real plain format should define plain font macros");
        let has_glyph_node = fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)));

        assert_eq!(fragment.metadata.format_id, "plain");
        assert!(has_glyph_node);
    }

    #[cfg(feature = "std")]
    #[derive(Clone, Copy, Debug)]
    struct HostResolverTestResources;

    #[cfg(feature = "std")]
    impl HostResolverTestResources {
        fn is_available() -> bool {
            Self::resolve("plain.tex").is_some()
                && Self::resolve("hyphen.tex").is_some()
                && Self::resolve("cmr10.tfm").is_some()
        }

        fn is_latex_available() -> bool {
            Self::resolve("latex.ltx").is_some() && Self::resolve("cmr10.tfm").is_some()
        }

        fn resolve(name: &str) -> Option<Vec<u8>> {
            let output = std::process::Command::new("kpsewhich")
                .arg(name)
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let path = core::str::from_utf8(&output.stdout).ok()?.trim();
            if path.is_empty() {
                return None;
            }
            std::fs::read(path).ok()
        }

        fn candidate_names(request: &ResourceRequest) -> Vec<String> {
            let name = normalize_host_resolver_name(request.canonical_name());
            let mut candidates = Vec::with_capacity(4);
            candidates.push(name.clone());

            if std::path::Path::new(&name).extension().is_none() {
                match request.kind {
                    ResourceKind::Package => candidates.push(format!("{name}.sty")),
                    ResourceKind::Class => candidates.push(format!("{name}.cls")),
                    ResourceKind::FontDefinition => candidates.push(format!("{name}.fd")),
                    ResourceKind::Font => {
                        candidates.push(format!("{name}.tfm"));
                        candidates.push(format!("{name}.otf"));
                        candidates.push(format!("{name}.ttf"));
                    }
                    ResourceKind::Encoding => candidates.push(format!("{name}.enc")),
                    ResourceKind::Map => candidates.push(format!("{name}.map")),
                    _ => {}
                }
            }

            candidates
        }
    }

    #[cfg(feature = "std")]
    fn normalize_host_resolver_name(mut name: String) -> String {
        while let Some(stripped) = name.strip_prefix("./") {
            name = stripped.to_string();
        }
        name
    }

    #[cfg(feature = "std")]
    impl ResourceProvider for HostResolverTestResources {
        fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
            for candidate in Self::candidate_names(request) {
                if let Some(bytes) = Self::resolve(&candidate) {
                    return Ok(Resource::from_request(request, bytes));
                }
            }

            Err(ResourceError::NotFound {
                name: request.canonical_name(),
                kind: request.kind,
            })
        }
    }

    #[test]
    fn builder_rejects_format_for_different_profile() {
        let format = FormatImage::empty("latex", XetexProfile.id());

        let error = EngineBuilder::new(TexProfile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect_err("profile mismatch must be rejected");

        assert_eq!(
            error,
            BuildError::ProfileFormatMismatch {
                profile: "tex",
                format_profile: "xetex",
            }
        );
    }

    #[test]
    fn transcript_error_detection_requires_tex_error_shape() {
        assert_eq!(
            generated_transcript_error(b"LaTeX says hello! This is only a message."),
            None
        );
        assert_eq!(
            generated_transcript_error(b"before,! Emergency stop.\nl.12 \\bad\n"),
            Some("generated TeX engine reported an error: ! Emergency stop.".to_string())
        );
    }

    #[test]
    fn builder_reports_generated_format_preload_failure() {
        let profile = TexProfile;
        let resources = InMemoryResourceProvider::new().with_resource(
            TEST_FORMAT_INPUT,
            ResourceKind::TexInput,
            br"\input missing-format-dependency.tex".to_vec(),
        );
        let format = FormatImage::initializer("missing-dependency", profile.id())
            .preload_tex_input(TEST_FORMAT_INPUT)
            .build(&resources)
            .expect("format initializer should capture preloaded resource bytes");

        let error = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect_err("missing nested preload resource must fail engine build");

        match error {
            BuildError::FormatPreloadFailed(FormatPreloadError::Resource { name, resource }) => {
                assert_eq!(name, TEST_FORMAT_INPUT);
                assert_eq!(resource, "missing-format-dependency.tex");
            }
            other => panic!("unexpected preload error: {other:?}"),
        }
    }

    #[test]
    fn session_instantiates_generated_engine_from_cached_format() {
        let format = FormatImage::empty("latex", XetexProfile.id());
        let engine = EngineBuilder::new(XetexProfile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();
        let generated = session.generated_engine();

        assert_eq!(
            generated.profile(),
            mathtex_portable_engine_generated::EngineProfile::xetex()
        );
        assert!(!generated.profile().native_fonts);
        assert_eq!(generated.resource_request_count(), 0);
    }

    #[derive(Clone, Debug)]
    struct CustomSemanticProfile {
        fonts: crate::FontSemantics,
        extensions: ExtensionPolicy,
    }

    impl EngineProfile for CustomSemanticProfile {
        fn id(&self) -> ProfileId {
            ProfileId("custom-semantic")
        }

        fn kind(&self) -> EngineKind {
            EngineKind::Xetex
        }

        fn primitives(&self) -> &[PrimitiveSpec] {
            &[]
        }

        fn catcode_defaults(&self) -> crate::CatcodeDefaults {
            crate::CatcodeDefaults {
                unicode_scalars: self.extensions.xetex,
            }
        }

        fn mathcode_defaults(&self) -> crate::MathcodeDefaults {
            crate::MathcodeDefaults {
                unicode_math: false,
            }
        }

        fn register_defaults(&self) -> crate::RegisterDefaults {
            crate::RegisterDefaults::extended()
        }

        fn font_semantics(&self) -> crate::FontSemantics {
            self.fonts
        }

        fn extension_policy(&self) -> ExtensionPolicy {
            self.extensions
        }
    }

    #[test]
    fn generated_engine_uses_custom_profile_semantic_flags() {
        let profile = CustomSemanticProfile {
            fonts: crate::FontSemantics::tex(),
            extensions: ExtensionPolicy {
                etex: true,
                xetex: true,
            },
        };
        let format = FormatImage::empty("custom-semantic", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let generated = engine.new_session().generated_engine().profile();

        assert_eq!(generated.id, "custom-semantic");
        assert_eq!(
            generated.kind,
            mathtex_portable_engine_generated::EngineProfileKind::Xetex
        );
        assert!(generated.etex);
        assert!(generated.xetex);
        assert!(generated.unicode_scalars);
        assert!(!generated.unicode_math);
        assert!(!generated.native_fonts);
    }

    #[derive(Clone, Debug)]
    struct PrimitiveProfile {
        primitives: Vec<PrimitiveSpec>,
    }

    impl EngineProfile for PrimitiveProfile {
        fn id(&self) -> ProfileId {
            ProfileId("primitive-profile")
        }

        fn kind(&self) -> EngineKind {
            EngineKind::Etex
        }

        fn primitives(&self) -> &[PrimitiveSpec] {
            &self.primitives
        }

        fn catcode_defaults(&self) -> crate::CatcodeDefaults {
            crate::CatcodeDefaults::default()
        }

        fn mathcode_defaults(&self) -> crate::MathcodeDefaults {
            crate::MathcodeDefaults::default()
        }

        fn register_defaults(&self) -> crate::RegisterDefaults {
            crate::RegisterDefaults::default()
        }

        fn font_semantics(&self) -> crate::FontSemantics {
            crate::FontSemantics::default()
        }

        fn extension_policy(&self) -> crate::ExtensionPolicy {
            crate::ExtensionPolicy::default()
        }
    }

    #[test]
    fn engine_exposes_profile_primitive_dispatch_entries() {
        let profile = PrimitiveProfile {
            primitives: alloc::vec![PrimitiveSpec {
                name: Cow::Borrowed("hbox"),
                opcode: PrimitiveOpcode(44),
                kind: PrimitiveKind::Layout,
            }],
        };
        let format = FormatImage::empty("plain", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        let primitive = session.primitive("hbox").expect("hbox primitive");

        assert_eq!(primitive.opcode, PrimitiveOpcode(44));
        assert_eq!(primitive.kind, PrimitiveKind::Layout);
        assert!(session.primitive("missing").is_none());
    }

    #[test]
    fn engine_caches_runtime_semantics_for_shared_tex_core() {
        let profile = TexProfile;
        let format = FormatImage::empty("plain", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        assert_eq!(engine.semantics().profile, ProfileId("tex"));
        assert_eq!(session.semantics(), engine.semantics());
        assert!(!engine.semantics().has_patch(EnginePatch::Xetex));
        assert!(!engine.semantics().is_xetex());
        assert!(!engine.semantics().catcodes.unicode_scalars);
    }

    #[test]
    fn xetex_profile_is_runtime_patch_over_shared_core() {
        let profile = XetexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        assert_eq!(engine.semantics().profile, ProfileId("xetex"));
        assert_eq!(engine.semantics().kind, EngineKind::Xetex);
        assert!(engine.semantics().has_patch(EnginePatch::Etex));
        assert!(engine.semantics().has_patch(EnginePatch::Xetex));
        assert!(engine.semantics().is_xetex());
        assert!(engine.semantics().catcodes.unicode_scalars);
        assert!(engine.semantics().fonts.shaped_text);
        assert!(session.primitive("XeTeXrevision").is_some());
    }

    #[derive(Clone, Debug)]
    struct PolicyProfile {
        primitives: Vec<PrimitiveSpec>,
        extensions: ExtensionPolicy,
    }

    impl EngineProfile for PolicyProfile {
        fn id(&self) -> ProfileId {
            ProfileId("policy-profile")
        }

        fn kind(&self) -> EngineKind {
            EngineKind::Xetex
        }

        fn primitives(&self) -> &[PrimitiveSpec] {
            &self.primitives
        }

        fn catcode_defaults(&self) -> crate::CatcodeDefaults {
            crate::CatcodeDefaults {
                unicode_scalars: self.extensions.xetex,
            }
        }

        fn mathcode_defaults(&self) -> crate::MathcodeDefaults {
            crate::MathcodeDefaults {
                unicode_math: self.extensions.xetex,
            }
        }

        fn register_defaults(&self) -> crate::RegisterDefaults {
            crate::RegisterDefaults::extended()
        }

        fn font_semantics(&self) -> crate::FontSemantics {
            crate::FontSemantics {
                unicode_fonts: self.extensions.xetex,
                shaped_text: self.extensions.xetex,
                unicode_math_fonts: self.extensions.xetex,
                host_native_fonts: false,
            }
        }

        fn extension_policy(&self) -> crate::ExtensionPolicy {
            self.extensions
        }
    }

    #[test]
    fn custom_profile_extension_policy_becomes_runtime_patch_flags() {
        let profile = PolicyProfile {
            primitives: alloc::vec![PrimitiveSpec {
                name: Cow::Borrowed("customxetex"),
                opcode: PrimitiveOpcode(800),
                kind: PrimitiveKind::Extension,
            }],
            extensions: ExtensionPolicy {
                etex: false,
                xetex: true,
            },
        };
        let format = FormatImage::empty("custom", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");

        assert!(engine.semantics().has_patch(EnginePatch::Etex));
        assert!(engine.semantics().has_patch(EnginePatch::Xetex));
        assert!(engine.primitives().get("customxetex").is_some());
    }

    #[test]
    fn engine_rejects_duplicate_primitive_names() {
        let profile = PrimitiveProfile {
            primitives: alloc::vec![
                PrimitiveSpec {
                    name: Cow::Borrowed("input"),
                    opcode: PrimitiveOpcode(1),
                    kind: PrimitiveKind::Resource,
                },
                PrimitiveSpec {
                    name: Cow::Borrowed("input"),
                    opcode: PrimitiveOpcode(2),
                    kind: PrimitiveKind::Resource,
                },
            ],
        };
        let format = FormatImage::empty("plain", profile.id());

        let error = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect_err("duplicate primitive names should fail");

        assert_eq!(
            error,
            BuildError::InvalidPrimitives(PrimitiveRegistryError::DuplicateName {
                name: "input".to_string(),
            })
        );
    }

    #[test]
    fn session_loads_packages_through_configured_provider() {
        let profile = TexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let resources = InMemoryResourceProvider::new().with_resource(
            "amsmath.sty",
            ResourceKind::Package,
            br"\ProvidesPackage{amsmath}".to_vec(),
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");

        let session = engine.new_session();
        let package = session
            .load_package("amsmath.sty")
            .expect("package should be loaded through provider");

        assert_eq!(package.kind, ResourceKind::Package);
        assert_eq!(package.bytes, br"\ProvidesPackage{amsmath}".to_vec());
    }

    #[test]
    fn session_loads_typed_resources_through_provider() {
        let profile = TexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let mut resources = InMemoryResourceProvider::new()
            .with_resource("article.cls", ResourceKind::Class, b"class")
            .with_resource("ot1cmr.fd", ResourceKind::FontDefinition, b"fd")
            .with_resource("size10.clo", ResourceKind::PackageSupport, b"support")
            .with_resource("latinmodern-math.otf", ResourceKind::Font, b"font")
            .with_resource("t1.enc", ResourceKind::Encoding, b"encoding")
            .with_resource("pdftex.map", ResourceKind::Map, b"map")
            .with_resource("texmf.cnf", ResourceKind::Config, b"config")
            .with_resource("latex.fmt", ResourceKind::FormatImage, b"format");
        resources.insert_request(
            crate::ProviderResourceRequest::asset("mhchem", "arrows.dat"),
            b"asset",
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        let class = session.load_class("article.cls").expect("class");
        let font_definition = session
            .load_font_definition("ot1cmr.fd")
            .expect("font definition");
        let package_support = session
            .load_package_support("size10.clo")
            .expect("package support");
        let font = session
            .load_font_resource("latinmodern-math.otf")
            .expect("font");
        let encoding = session.load_encoding("t1.enc").expect("encoding");
        let map = session.load_map("pdftex.map").expect("map");
        let config = session.load_config("texmf.cnf").expect("config");
        let asset = session.load_asset("mhchem", "arrows.dat").expect("asset");
        let format_image = session
            .load_format_image("latex.fmt")
            .expect("format image");

        assert_eq!(class.kind, ResourceKind::Class);
        assert_eq!(class.bytes, b"class");
        assert_eq!(font_definition.kind, ResourceKind::FontDefinition);
        assert_eq!(font_definition.bytes, b"fd");
        assert_eq!(package_support.kind, ResourceKind::PackageSupport);
        assert_eq!(package_support.bytes, b"support");
        assert_eq!(font.kind, ResourceKind::Font);
        assert_eq!(font.bytes, b"font");
        assert_eq!(encoding.kind, ResourceKind::Encoding);
        assert_eq!(encoding.bytes, b"encoding");
        assert_eq!(map.kind, ResourceKind::Map);
        assert_eq!(map.bytes, b"map");
        assert_eq!(config.kind, ResourceKind::Config);
        assert_eq!(config.bytes, b"config");
        assert_eq!(asset.kind, ResourceKind::Asset);
        assert_eq!(asset.canonical_name, "mhchem/arrows.dat");
        assert_eq!(asset.bytes, b"asset");
        assert_eq!(format_image.kind, ResourceKind::FormatImage);
        assert_eq!(format_image.bytes, b"format");
    }

    #[test]
    fn repeated_sessions_reuse_cached_format_metadata() {
        let profile = TexProfile;
        let format = FormatImage::empty("latex+amsmath", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");

        let first = engine.new_session();
        let second = engine.new_session();

        let first_ir = first.ir_emitter().finish();
        let second_ir = second.ir_emitter().finish();

        assert_eq!(first_ir.metadata.format_id, "latex+amsmath");
        assert_eq!(second_ir.metadata.format_id, "latex+amsmath");
        assert_eq!(first_ir.metadata.engine_profile, "tex");
        assert_eq!(second_ir.metadata.engine_profile, "tex");
    }

    #[test]
    fn layout_fragment_input_uses_generated_engine_without_manual_core() {
        let profile = TexProfile;
        let (format, resources) = basic_test_format(profile.id());
        let resources = resources
            .with_resource(
                "cmr10",
                ResourceKind::Font,
                include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm")
                    .to_vec(),
            )
            .with_resource(
                "cmmi10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmmi10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmmi10.tfm",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmmi10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmsy10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmsy10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmsy10.tfm",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmsy10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmex10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmex10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmex10.tfm",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmex10.tfm"
                )
                .to_vec(),
            );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\font\tenrm=cmr10 \tenrm A";
        let fragment = session
            .layout_fragment_input(FragmentInput::new("equation.tex", body, FragmentKind::Text))
            .expect("generated engine should layout a boxed text fragment");
        let _glyph_node = fragment
            .nodes
            .iter()
            .find(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)))
            .expect("glyph run should be emitted");

        assert_eq!(fragment.metadata.fragment_kind, FragmentKind::Text);
    }

    #[test]
    fn layout_rejects_input_that_exceeds_platform_limit() {
        let profile = TexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let platform = NoopPlatform::with_limits(HostLimits {
            max_input_bytes: 4,
            ..HostLimits::default()
        });
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(platform)
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let error = session
            .layout_fragment("12345")
            .expect_err("input should exceed limit");

        assert_eq!(
            error,
            EngineError::Limit(LimitError::InputTooLarge {
                actual: 5,
                limit: 4,
            })
        );
    }

    #[test]
    fn layout_fragment_loads_tex_inputs_inside_generated_engine() {
        let profile = TexProfile;
        let (format, resources) = basic_test_format(profile.id());
        let resources = resources.with_resource(
            "snippet.tex",
            ResourceKind::TexInput,
            br"\vrule width 1pt height 2pt depth 0pt".to_vec(),
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\input snippet.tex";
        let fragment = session
            .layout_fragment_input(FragmentInput::text(body))
            .expect("generated engine should own input loading");
        assert!(fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::Rule(_))));
    }

    #[test]
    fn layout_fragment_loads_packages_inside_generated_engine() {
        let profile = TexProfile;
        let (format, resources) = basic_test_format(profile.id());
        let resources = resources
            .with_resource(
                "cmr10",
                ResourceKind::Font,
                include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm")
                    .to_vec(),
            )
            .with_resource(
                "testpkg.sty",
                ResourceKind::Package,
                br"\def\pkgchar{A}".to_vec(),
            );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\font\tenrm=cmr10 \tenrm A \usepackage{testpkg}\pkgchar";
        let fragment = session
            .layout_fragment_input(FragmentInput::text(body))
            .expect("generated engine should expand package loading through TeX");
        assert!(fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_))));
    }

    #[test]
    fn generated_format_cache_reuses_preloaded_package_state() {
        let profile = TexProfile;
        let format_resources = InMemoryResourceProvider::new()
            .with_resource(TEST_FORMAT_INPUT, ResourceKind::TexInput, TEST_FORMAT_SETUP)
            .with_resource(
                "cachedpkg.sty",
                ResourceKind::Package,
                br"\def\cachedpkgchar{A}".to_vec(),
            );
        let format = FormatImage::initializer("latex+cachedpkg", profile.id())
            .preload_tex_input(TEST_FORMAT_INPUT)
            .preload_package("cachedpkg")
            .build(&format_resources)
            .expect("format should preload package bytes");
        let resources = InMemoryResourceProvider::new().with_resource(
            "cmr10",
            ResourceKind::Font,
            include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm").to_vec(),
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\font\tenrm=cmr10 \tenrm \cachedpkgchar";
        let fragment = session
            .layout_fragment_input(FragmentInput::text(body))
            .expect("preloaded generated package macro should be cached");
        let has_glyph_node = fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)));

        assert_eq!(fragment.metadata.format_id, "latex+cachedpkg");
        assert!(has_glyph_node);
        assert!(!fragment
            .source_map
            .sources
            .iter()
            .any(|source| source.name == "cachedpkg.sty"));
    }

    #[test]
    fn generated_format_cache_reuses_preloaded_tex_input_state() {
        let profile = TexProfile;
        let format_resources = InMemoryResourceProvider::new()
            .with_resource(TEST_FORMAT_INPUT, ResourceKind::TexInput, TEST_FORMAT_SETUP)
            .with_resource(
                "plain.tex",
                ResourceKind::TexInput,
                br"\def\plainchar{A}".to_vec(),
            );
        let format = FormatImage::initializer("plain", profile.id())
            .preload_tex_input(TEST_FORMAT_INPUT)
            .preload_tex_input("plain.tex")
            .build(&format_resources)
            .expect("format should preload TeX input bytes");
        let resources = InMemoryResourceProvider::new().with_resource(
            "cmr10",
            ResourceKind::Font,
            include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm").to_vec(),
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\font\tenrm=cmr10 \tenrm \plainchar";
        let fragment = session
            .layout_fragment_input(FragmentInput::text(body))
            .expect("preloaded generated TeX input macro should be cached");
        let has_glyph_node = fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)));

        assert_eq!(fragment.metadata.format_id, "plain");
        assert!(has_glyph_node);
        assert!(!fragment
            .source_map
            .sources
            .iter()
            .any(|source| source.name == "plain.tex"));
    }

    #[test]
    fn math_fragment_uses_format_owned_math_catcodes_and_fonts() {
        let profile = TexProfile;
        let (format, resources) = math_test_format(profile.id());
        let resources = resources
            .with_resource(
                "cmr10",
                ResourceKind::Font,
                include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm")
                    .to_vec(),
            )
            .with_resource(
                "cmmi10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmmi10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmsy10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmsy10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmex10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmex10.tfm"
                )
                .to_vec(),
            );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let fragment = session
            .layout_fragment_input(FragmentInput::math_inline("x^2"))
            .expect("generated engine should layout superscript math from format catcodes");
        let glyph_count = fragment
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)))
            .count();

        assert_eq!(fragment.metadata.format_id, "plain-math");
        assert!(glyph_count >= 2);
    }

    #[cfg(feature = "std")]
    #[test]
    fn real_plain_format_loads_through_generated_engine_when_texlive_is_available() {
        if !HostResolverTestResources::is_available() {
            return;
        }

        assert_real_plain_format_loads(HostResolverTestResources);
    }

    #[test]
    fn real_plain_format_loads_from_vendored_fixture() {
        assert_real_plain_format_loads(PlainFixtureResources);
    }

    #[cfg(feature = "std")]
    #[test]
    fn real_latex_format_loads_through_generated_engine_when_texlive_is_available() {
        if !HostResolverTestResources::is_latex_available() {
            return;
        }

        let profile = XetexProfile;
        let format = FormatImage::latex(profile.id())
            .build(&HostResolverTestResources)
            .expect("real latex.ltx should resolve");
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(HostResolverTestResources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build with real latex format");
        let mut session = engine.new_session();

        let fragment = session
            .layout_fragment_input(FragmentInput::text(r"\font\tenrm=cmr10 \tenrm A"))
            .expect("real latex format should leave the engine usable for fragments");
        let has_glyph_node = fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)));

        assert_eq!(fragment.metadata.format_id, "latex");
        assert!(has_glyph_node);
    }

    #[test]
    fn math_fragment_supports_text_macro_inside_generated_engine() {
        let profile = TexProfile;
        let (format, resources) = math_test_format(profile.id());
        let resources = resources
            .with_resource(
                "cmr10",
                ResourceKind::Font,
                include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm")
                    .to_vec(),
            )
            .with_resource(
                "cmmi10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmmi10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmsy10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmsy10.tfm"
                )
                .to_vec(),
            )
            .with_resource(
                "cmex10",
                ResourceKind::Font,
                include_bytes!(
                    "../../../vendor/texlive-source/texk/web2c/tests/generated-tfm/cmex10.tfm"
                )
                .to_vec(),
            );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\font\tenrm=cmr10 \font\teni=cmmi10 \font\tensy=cmsy10 \font\tenex=cmex10 \textfont0=\tenrm \scriptfont0=\tenrm \scriptscriptfont0=\tenrm \textfont1=\teni \scriptfont1=\teni \scriptscriptfont1=\teni \textfont2=\tensy \scriptfont2=\tensy \scriptscriptfont2=\tensy \textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex \text{\tenrm A}";
        let fragment = session
            .layout_fragment_input(FragmentInput::math_inline(body))
            .expect("generated engine should allow text boxes inside math");
        let _glyph_node = fragment
            .nodes
            .iter()
            .find(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_)))
            .expect("text macro should emit glyph IR");

        assert_eq!(fragment.metadata.fragment_kind, FragmentKind::MathInline);
    }

    #[test]
    fn generated_package_loading_resolves_package_assets() {
        let profile = TexProfile;
        let (format, resources) = basic_test_format(profile.id());
        let mut resources = resources
            .with_resource(
                "cmr10",
                ResourceKind::Font,
                include_bytes!("../../../vendor/texlive-source/texk/web2c/tests/cmr10.tfm")
                    .to_vec(),
            )
            .with_resource(
                "assetpkg.sty",
                ResourceKind::Package,
                br"\input data.dat\relax".to_vec(),
            );
        resources.insert_request(
            crate::ProviderResourceRequest::asset("assetpkg", "data.dat"),
            br"\def\assetpkgchar{A}".to_vec(),
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(resources)
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let body = r"\font\tenrm=cmr10 \usepackage{assetpkg}\tenrm \assetpkgchar";
        let fragment = session
            .layout_fragment_input(FragmentInput::text(body))
            .expect("generated engine should load package-owned TeX assets");

        assert!(fragment
            .nodes
            .iter()
            .any(|node| matches!(node.kind, LayoutNodeKind::GlyphRun(_))));
    }

    #[test]
    fn generated_layout_does_not_expand_legacy_host_macros() {
        let profile = TexProfile;
        let format = FormatImage::with_snapshot(
            "latex+macros",
            profile.id(),
            FormatSnapshot {
                macros: alloc::vec![MacroDefinition {
                    name: "speedtext".into(),
                    replacement: br"\text{speed}".to_vec(),
                }],
                ..FormatSnapshot::default()
            },
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let mut session = engine.new_session();

        let error = session
            .layout_fragment(r"a+\speedtext")
            .expect_err("generated engine should not use legacy host macro tables");

        match error {
            EngineError::Layout { message } => {
                assert!(message.contains("generated TeX engine reported an error"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn sessions_load_and_shape_text_through_font_system() {
        let profile = XetexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let fonts = InMemoryFontSystem::new()
            .with_font("Latin Modern Math", b"font")
            .with_fallback_shaping();
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .fonts(fonts)
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        let font = session
            .load_font(&FontQuery {
                family: "Latin Modern Math".to_string(),
                size: Length::from_scaled_points(655_360),
                math: true,
            })
            .expect("font should load");
        let shaped = session
            .shape_text(&ShapeRequest {
                font: font.id,
                text: "xy",
                direction: Direction::LeftToRight,
                source: Some(ByteSpan { start: 10, end: 12 }),
                script: None,
                features: Vec::new(),
            })
            .expect("text should shape through font system");

        assert_eq!(font.canonical_name, "Latin Modern Math");
        assert_eq!(shaped.glyphs.len(), 2);
        assert_eq!(
            shaped.glyphs[0].cluster,
            Some(ByteSpan { start: 10, end: 11 })
        );
        assert_eq!(
            shaped.glyphs[1].cluster,
            Some(ByteSpan { start: 11, end: 12 })
        );
    }

    #[test]
    fn sessions_can_load_fonts_from_resource_provider_adapter() {
        let profile = XetexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let font_resources = InMemoryResourceProvider::new().with_resource(
            "Latin Modern Math.otf",
            ResourceKind::Font,
            b"font-bytes",
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .fonts(ResourceFontSystem::new(font_resources))
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        let font = session
            .load_font(&FontQuery {
                family: "Latin Modern Math.otf".to_string(),
                size: Length::from_scaled_points(655_360),
                math: true,
            })
            .expect("font should load from ResourceProvider");

        assert_eq!(font.canonical_name, "Latin Modern Math.otf");
        assert_eq!(&**font.bytes().expect("library owned bytes"), b"font-bytes");
        assert!(session
            .shape_text(&ShapeRequest {
                font: font.id,
                text: "x",
                direction: Direction::LeftToRight,
                source: None,
                script: None,
                features: Vec::new(),
            })
            .is_err());
    }

    #[test]
    fn sessions_clone_cached_format_state_without_sharing_mutations() {
        let profile = TexProfile;
        let format = FormatImage::with_snapshot(
            "latex+text",
            profile.id(),
            FormatSnapshot {
                macros: alloc::vec![MacroDefinition {
                    name: "text".into(),
                    replacement: b"initial".to_vec(),
                }],
                ..FormatSnapshot::default()
            },
        );
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");

        let mut first = engine.new_session();
        let second = engine.new_session();

        first
            .state_mut()
            .define_macro("text", b"session-local".to_vec());

        assert_eq!(
            first
                .state()
                .macro_definition("text")
                .expect("first session macro")
                .replacement,
            b"session-local".to_vec()
        );
        assert_eq!(
            second
                .state()
                .macro_definition("text")
                .expect("second session macro")
                .replacement,
            b"initial".to_vec()
        );

        match &engine.format().state {
            crate::FormatState::Structured(snapshot) => {
                assert_eq!(snapshot.macros[0].replacement, b"initial".to_vec());
            }
            other => panic!("unexpected format state: {other:?}"),
        }
    }

    #[test]
    fn session_creates_metadata_seeded_ir_emitter() {
        let profile = TexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        let fragment = session.ir_emitter().finish();

        assert_eq!(fragment.metadata.engine_profile, "tex");
        assert_eq!(fragment.metadata.format_id, "latex");
        assert_eq!(fragment.metadata.fragment_kind, FragmentKind::MathInline);
    }

    #[test]
    fn session_creates_mode_specific_ir_emitter() {
        let profile = TexProfile;
        let format = FormatImage::empty("latex", profile.id());
        let engine = EngineBuilder::new(profile)
            .format(format)
            .resources(InMemoryResourceProvider::new())
            .platform(NoopPlatform::default())
            .build()
            .expect("engine should build");
        let session = engine.new_session();

        let fragment = session.ir_emitter_for(FragmentKind::Text).finish();

        assert_eq!(fragment.metadata.fragment_kind, FragmentKind::Text);
    }
}
