use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::profile::ProfileId;
use crate::resource::ResourceProvider;

/// Cached initialized macro/package state for an engine profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatImage {
    /// Format identifier string.
    pub id: String,
    /// Engine profile this format targets.
    pub profile: ProfileId,
    /// Packages incorporated during format initialization.
    pub packages: Vec<FormatPackage>,
    /// Opaque initialized state snapshot.
    pub state: FormatState,
}

impl FormatImage {
    /// Create an empty format image for early bootstrapping and tests.
    #[must_use]
    pub fn empty(id: impl Into<String>, profile: ProfileId) -> Self {
        Self {
            id: id.into(),
            profile,
            packages: Vec::new(),
            state: FormatState::Empty,
        }
    }

    /// Create a format image wrapping an already-built structured snapshot.
    #[must_use]
    pub fn with_snapshot(
        id: impl Into<String>,
        profile: ProfileId,
        snapshot: FormatSnapshot,
    ) -> Self {
        Self {
            id: id.into(),
            profile,
            packages: snapshot.packages.clone(),
            state: FormatState::Structured(snapshot),
        }
    }

    /// Create a [`FormatInitializer`] builder for this format id and profile.
    #[must_use]
    pub fn initializer(id: impl Into<String>, profile: ProfileId) -> FormatInitializer {
        FormatInitializer::new(id, profile)
    }

    /// Start building the plain TeX format, the provider must supply `plain.tex`.
    #[must_use]
    pub fn plain(profile: ProfileId) -> FormatInitializer {
        Self::initializer("plain", profile).preload_tex_input("plain.tex")
    }

    /// Start building the LaTeX format, the provider must supply `latex.ltx`.
    #[must_use]
    pub fn latex(profile: ProfileId) -> FormatInitializer {
        Self::initializer("latex", profile).preload_tex_input("latex.ltx")
    }

    /// Clone the format's initialized state into a fresh mutable `SessionState`.
    #[must_use]
    pub fn instantiate_session_state(&self) -> SessionState {
        match &self.state {
            FormatState::Empty => SessionState::default(),
            FormatState::Snapshot(bytes) => SessionState {
                opaque_engine_state: bytes.clone(),
                ..SessionState::default()
            },
            FormatState::Structured(snapshot) => SessionState {
                macros: snapshot.macros.clone(),
                packages: snapshot.packages.clone(),
                resources: snapshot.resources.clone(),
                opaque_engine_state: snapshot.opaque_engine_state.clone(),
                registers: snapshot.registers.clone(),
            },
        }
    }
}

/// Builder for a cached [`FormatImage`] initialized from package resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatInitializer {
    id: String,
    profile: ProfileId,
    tex_inputs: Vec<String>,
    packages: Vec<String>,
    macros: Vec<MacroDefinition>,
    registers: RegisterSnapshot,
    opaque_engine_state: Vec<u8>,
}

impl FormatInitializer {
    /// Create a new empty format initializer for the given id and profile.
    #[must_use]
    pub fn new(id: impl Into<String>, profile: ProfileId) -> Self {
        Self {
            id: id.into(),
            profile,
            tex_inputs: Vec::new(),
            packages: Vec::new(),
            macros: Vec::new(),
            registers: RegisterSnapshot::default(),
            opaque_engine_state: Vec::new(),
        }
    }

    /// Queue a TeX input file to load during format initialization.
    #[must_use]
    pub fn preload_tex_input(mut self, input: impl Into<String>) -> Self {
        self.tex_inputs.push(input.into());
        self
    }

    /// Queue a package style file to load during format initialization.
    #[must_use]
    pub fn preload_package(mut self, package: impl Into<String>) -> Self {
        self.packages.push(package.into());
        self
    }

    /// Add a macro definition to embed in the format.
    #[must_use]
    pub fn macro_definition(
        mut self,
        name: impl Into<String>,
        replacement: impl Into<Vec<u8>>,
    ) -> Self {
        self.macros.push(MacroDefinition {
            name: name.into(),
            replacement: replacement.into(),
        });
        self
    }

    /// Set the initial register snapshot to embed in the format.
    #[must_use]
    pub fn registers(mut self, registers: RegisterSnapshot) -> Self {
        self.registers = registers;
        self
    }

    /// Append opaque engine seed bytes to embed in the format.
    #[must_use]
    pub fn opaque_engine_state(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.opaque_engine_state.extend(bytes.into());
        self
    }

    /// Build the cached format by resolving preloaded resources once.
    pub fn build<R>(self, resources: &R) -> Result<FormatImage, FormatInitError>
    where
        R: ResourceProvider,
    {
        let mut packages = Vec::with_capacity(self.packages.len());
        let mut format_resources = Vec::with_capacity(self.tex_inputs.len() + self.packages.len());
        let mut opaque_engine_state = self.opaque_engine_state;

        for input in self.tex_inputs {
            let resource =
                resources
                    .read_tex_input(&input)
                    .map_err(|error| FormatInitError::TexInput {
                        name: input.clone(),
                        message: format!("{error:?}"),
                    })?;
            opaque_engine_state.extend(&resource.bytes);
            format_resources.push(FormatResource {
                name: resource.canonical_name,
                kind: FormatResourceKind::TexInput,
                bytes: resource.bytes,
            });
        }

        for package in self.packages {
            let resource_name = package_resource_name(&package);
            let resource = resources.read_package(&resource_name).map_err(|error| {
                FormatInitError::Package {
                    name: resource_name.clone(),
                    message: format!("{error:?}"),
                }
            })?;
            let package = FormatPackage {
                name: package,
                version: None,
            };
            opaque_engine_state.extend(&resource.bytes);
            format_resources.push(FormatResource {
                name: resource.canonical_name,
                kind: FormatResourceKind::Package,
                bytes: resource.bytes,
            });
            packages.push(package);
        }

        let snapshot = FormatSnapshot {
            macros: self.macros,
            packages,
            resources: format_resources,
            opaque_engine_state,
            registers: self.registers,
        };

        Ok(FormatImage::with_snapshot(self.id, self.profile, snapshot))
    }
}

/// Package that contributed to a cached format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatPackage {
    /// Package name.
    pub name: String,
    /// Package version, if known.
    pub version: Option<String>,
}

/// Structured initialized format state that can be cloned into sessions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormatSnapshot {
    /// Macro definitions or macro state records preloaded into the format.
    pub macros: Vec<MacroDefinition>,
    /// Packages incorporated during format initialization.
    pub packages: Vec<FormatPackage>,
    /// Resources captured during format initialization.
    pub resources: Vec<FormatResource>,
    /// Opaque bytes for generated engine state during bootstrap.
    pub opaque_engine_state: Vec<u8>,
    /// Register values captured at format initialization time.
    pub registers: RegisterSnapshot,
}

/// Resource captured while initializing a cached format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatResource {
    /// Canonical resource name.
    pub name: String,
    /// Role of this resource in the format.
    pub kind: FormatResourceKind,
    /// Resource bytes captured at format initialization time.
    pub bytes: Vec<u8>,
}

/// Resource role captured in a cached format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatResourceKind {
    /// A TeX input file loaded during format initialization.
    TexInput,
    /// A package style file loaded during format initialization.
    Package,
}

/// Macro definition captured in a cached format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroDefinition {
    /// Control sequence name without leading backslash.
    pub name: String,
    /// Replacement text or opaque translated representation.
    pub replacement: Vec<u8>,
}

/// Register values captured in a cached format.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegisterSnapshot {
    /// Snapshot of TeX count registers.
    pub counts: Vec<i32>,
    /// Snapshot of TeX dimension registers in scaled points.
    pub dimensions: Vec<i32>,
    /// Snapshot of TeX token list registers as opaque bytes.
    pub token_lists: Vec<Vec<u8>>,
}

/// Mutable session state cloned from [`FormatImage`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionState {
    /// Macro definitions active in this session.
    pub macros: Vec<MacroDefinition>,
    /// Packages loaded into this session.
    pub packages: Vec<FormatPackage>,
    /// Resources captured for this session.
    pub resources: Vec<FormatResource>,
    /// Opaque engine state bytes for this session.
    pub opaque_engine_state: Vec<u8>,
    /// Register values active in this session.
    pub registers: RegisterSnapshot,
}

impl SessionState {
    /// Add or replace a macro in the session state.
    pub fn define_macro(&mut self, name: impl Into<String>, replacement: impl Into<Vec<u8>>) {
        let name = name.into();
        let replacement = replacement.into();
        if let Some(existing) = self
            .macros
            .iter_mut()
            .find(|macro_def| macro_def.name == name)
        {
            existing.replacement = replacement;
            return;
        }

        self.macros.push(MacroDefinition { name, replacement });
    }

    /// Look up a macro definition by control sequence name.
    #[must_use]
    pub fn macro_definition(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.iter().find(|macro_def| macro_def.name == name)
    }

    /// Look up a loaded package by name.
    #[must_use]
    pub fn package(&self, name: &str) -> Option<&FormatPackage> {
        self.packages.iter().find(|package| package.name == name)
    }

    /// Look up a captured resource by canonical name.
    #[must_use]
    pub fn resource(&self, name: &str) -> Option<&FormatResource> {
        self.resources.iter().find(|resource| resource.name == name)
    }
}

/// Format initialization failure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatInitError {
    /// A requested TeX input could not be loaded.
    TexInput {
        /// Name of the TeX input file.
        name: String,
        /// Description of the load failure.
        message: String,
    },
    /// A requested package could not be loaded.
    Package {
        /// Name of the package resource file.
        name: String,
        /// Description of the load failure.
        message: String,
    },
}

fn package_resource_name(name: &str) -> String {
    if name.ends_with(".sty") {
        name.to_string()
    } else {
        format!("{name}.sty")
    }
}

/// Opaque initialized state snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatState {
    /// Empty state used for bootstrapping and tests.
    #[default]
    Empty,
    /// Opaque engine bytes, reserved for a future structured snapshot.
    Snapshot(Vec<u8>),
    /// Structured snapshot used to initialize each session state.
    Structured(FormatSnapshot),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ProfileId;
    use crate::{InMemoryResourceProvider, ResourceKind};

    #[test]
    fn format_image_instantiates_mutable_session_state() {
        let snapshot = FormatSnapshot {
            macros: alloc::vec![MacroDefinition {
                name: "text".into(),
                replacement: b"text macro".to_vec(),
            }],
            packages: alloc::vec![FormatPackage {
                name: "latex".into(),
                version: Some("2026".into()),
            }],
            resources: alloc::vec![FormatResource {
                name: "plain.tex".into(),
                kind: FormatResourceKind::TexInput,
                bytes: b"plain".to_vec(),
            }],
            opaque_engine_state: b"engine".to_vec(),
            registers: RegisterSnapshot {
                counts: alloc::vec![1, 2],
                dimensions: alloc::vec![3],
                token_lists: alloc::vec![b"tokens".to_vec()],
            },
        };
        let format = FormatImage::with_snapshot("latex", ProfileId("tex"), snapshot);

        let mut session = format.instantiate_session_state();
        session.define_macro("text", b"changed".to_vec());
        session.define_macro("frac", b"fraction".to_vec());

        assert_eq!(
            session
                .macro_definition("text")
                .expect("text macro")
                .replacement,
            b"changed".to_vec()
        );
        assert_eq!(
            session
                .macro_definition("frac")
                .expect("frac macro")
                .replacement,
            b"fraction".to_vec()
        );
        assert_eq!(
            session.resource("plain.tex").expect("plain resource").bytes,
            b"plain".to_vec()
        );
        assert_eq!(session.registers.counts, alloc::vec![1, 2]);

        match &format.state {
            FormatState::Structured(snapshot) => {
                assert_eq!(snapshot.macros[0].replacement, b"text macro".to_vec());
                assert_eq!(snapshot.resources[0].bytes, b"plain".to_vec());
                assert_eq!(snapshot.registers.counts, alloc::vec![1, 2]);
            }
            other => panic!("unexpected format state: {other:?}"),
        }
    }

    #[test]
    fn initializer_preloads_packages_through_resource_provider() {
        let resources = InMemoryResourceProvider::new()
            .with_resource("latex.ltx.sty", ResourceKind::Package, b"latex")
            .with_resource("amsmath.sty", ResourceKind::Package, b"ams");

        let format = FormatImage::initializer("latex+amsmath", ProfileId("tex"))
            .preload_package("latex.ltx.sty")
            .preload_package("amsmath")
            .macro_definition("text", b"text macro")
            .opaque_engine_state(b"seed".to_vec())
            .build(&resources)
            .expect("format should initialize");

        assert_eq!(format.packages.len(), 2);
        assert_eq!(format.packages[1].name, "amsmath");
        let session = format.instantiate_session_state();
        assert!(session.package("amsmath").is_some());
        assert!(session.resource("amsmath.sty").is_some());
        assert_eq!(
            session
                .macro_definition("text")
                .expect("text macro")
                .replacement,
            b"text macro".to_vec()
        );
        assert_eq!(session.opaque_engine_state, b"seedlatexams".to_vec());
    }

    #[test]
    fn initializer_reports_missing_preloaded_package() {
        let resources = InMemoryResourceProvider::new();

        let error = FormatImage::initializer("latex+missing", ProfileId("tex"))
            .preload_package("missing")
            .build(&resources)
            .expect_err("missing package should fail initialization");

        match error {
            FormatInitError::TexInput { .. } => panic!("unexpected TeX input error"),
            FormatInitError::Package { name, message } => {
                assert_eq!(name, "missing.sty");
                assert!(message.contains("NotFound"));
            }
        }
    }

    #[test]
    fn initializer_preloads_tex_input_resources() {
        let resources = InMemoryResourceProvider::new().with_resource(
            "plain.tex",
            ResourceKind::TexInput,
            br"\def\plainchar{A}".to_vec(),
        );

        let format = FormatImage::initializer("plain", ProfileId("tex"))
            .preload_tex_input("plain.tex")
            .build(&resources)
            .expect("format should preload TeX input bytes");

        let snapshot = match format.state {
            FormatState::Structured(snapshot) => snapshot,
            other => panic!("unexpected format state: {other:?}"),
        };
        assert_eq!(snapshot.resources.len(), 1);
        assert_eq!(snapshot.resources[0].name, "plain.tex");
        assert_eq!(snapshot.resources[0].kind, FormatResourceKind::TexInput);
        assert_eq!(snapshot.resources[0].bytes, br"\def\plainchar{A}".to_vec());
    }

    #[test]
    fn real_format_constructors_preload_engine_entrypoints() {
        let resources = InMemoryResourceProvider::new()
            .with_resource("plain.tex", ResourceKind::TexInput, br"\dump".to_vec())
            .with_resource("latex.ltx", ResourceKind::TexInput, br"\dump".to_vec());

        let plain = FormatImage::plain(ProfileId("tex"))
            .build(&resources)
            .expect("plain format entrypoint should resolve");
        let latex = FormatImage::latex(ProfileId("tex"))
            .build(&resources)
            .expect("latex format entrypoint should resolve");

        assert_eq!(plain.id, "plain");
        assert_eq!(latex.id, "latex");

        let plain_snapshot = match plain.state {
            FormatState::Structured(snapshot) => snapshot,
            other => panic!("unexpected plain format state: {other:?}"),
        };
        let latex_snapshot = match latex.state {
            FormatState::Structured(snapshot) => snapshot,
            other => panic!("unexpected latex format state: {other:?}"),
        };

        assert_eq!(plain_snapshot.resources[0].name, "plain.tex");
        assert_eq!(latex_snapshot.resources[0].name, "latex.ltx");
    }
}
