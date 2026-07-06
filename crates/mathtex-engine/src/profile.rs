use alloc::borrow::Cow;
use alloc::vec::Vec;

/// Opaque identifier string for an engine profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProfileId(pub &'static str);

/// Identifies which TeX engine variant is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineKind {
    /// Plain TeX baseline engine.
    Tex,
    /// eTeX extended engine.
    Etex,
    /// XeTeX Unicode and font extensions.
    Xetex,
}

/// Trait implemented by each engine variant to declare its capabilities and defaults.
pub trait EngineProfile {
    /// Returns the unique identifier for this profile.
    fn id(&self) -> ProfileId;

    /// Returns which TeX engine variant this profile represents.
    fn kind(&self) -> EngineKind;

    /// Returns the list of primitives available for this profile.
    fn primitives(&self) -> &[PrimitiveSpec];

    /// Returns default category code settings for this profile.
    fn catcode_defaults(&self) -> CatcodeDefaults;

    /// Returns default math code settings for this profile.
    fn mathcode_defaults(&self) -> MathcodeDefaults;

    /// Returns default register counts for this profile.
    fn register_defaults(&self) -> RegisterDefaults;

    /// Returns font capability settings for this profile.
    fn font_semantics(&self) -> FontSemantics;

    /// Returns which extension families are active for this profile.
    fn extension_policy(&self) -> ExtensionPolicy;
}

/// Owned copy of profile settings so shared TeX code can check the active profile at runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineSemantics {
    /// Profile identifier for this semantics snapshot.
    pub profile: ProfileId,
    /// Engine variant reported by the profile.
    pub kind: EngineKind,
    /// Primitives declared by the profile.
    pub primitives: Vec<PrimitiveSpec>,
    /// Category code defaults for this profile.
    pub catcodes: CatcodeDefaults,
    /// Math code defaults for this profile.
    pub mathcodes: MathcodeDefaults,
    /// Register count defaults for this profile.
    pub registers: RegisterDefaults,
    /// Font capability settings for this profile.
    pub fonts: FontSemantics,
    /// Extension families active for this profile.
    pub extensions: ExtensionPolicy,
    /// Computed patch flags derived from the extension policy.
    pub patches: EnginePatches,
}

impl EngineSemantics {
    /// Constructs an `EngineSemantics` snapshot from any `EngineProfile` implementor.
    #[must_use]
    pub fn from_profile<P>(profile: &P) -> Self
    where
        P: EngineProfile,
    {
        let extensions = profile.extension_policy();
        Self {
            profile: profile.id(),
            kind: profile.kind(),
            primitives: profile.primitives().to_vec(),
            catcodes: profile.catcode_defaults(),
            mathcodes: profile.mathcode_defaults(),
            registers: profile.register_defaults(),
            fonts: profile.font_semantics(),
            extensions,
            patches: EnginePatches::from_extension_policy(extensions),
        }
    }

    /// Returns the slice of primitives registered for this profile.
    #[must_use]
    pub fn primitives(&self) -> &[PrimitiveSpec] {
        &self.primitives
    }

    /// Finds a `PrimitiveSpec` by control sequence name, or `None` if not registered.
    #[must_use]
    pub fn primitive(&self, name: &str) -> Option<&PrimitiveSpec> {
        self.primitives
            .iter()
            .find(|primitive| primitive.name == name)
    }

    /// Returns `true` if a primitive with the given name is registered for this profile.
    #[must_use]
    pub fn has_primitive(&self, name: &str) -> bool {
        self.primitive(name).is_some()
    }

    /// Returns `true` if the given engine patch is active.
    #[must_use]
    pub const fn has_patch(&self, patch: EnginePatch) -> bool {
        self.patches.contains(patch)
    }

    /// Returns `true` if the XeTeX patch is active.
    #[must_use]
    pub const fn is_xetex(&self) -> bool {
        self.has_patch(EnginePatch::Xetex)
    }
}

/// Translation target for Web2C/C2Rust code with profile differences applied as patches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexCore {
    profile: ProfileId,
    kind: EngineKind,
    primitives: &'static [PrimitiveSpec],
    catcodes: CatcodeDefaults,
    mathcodes: MathcodeDefaults,
    registers: RegisterDefaults,
    fonts: FontSemantics,
    extensions: ExtensionPolicy,
    patches: EnginePatches,
}

impl TexCore {
    /// Constructs a plain TeX `TexCore` with no patches applied.
    #[must_use]
    pub const fn tex(profile: ProfileId) -> Self {
        Self {
            profile,
            kind: EngineKind::Tex,
            primitives: TEX_CORE_PRIMITIVES,
            catcodes: CatcodeDefaults {
                unicode_scalars: false,
            },
            mathcodes: MathcodeDefaults {
                unicode_math: false,
            },
            registers: RegisterDefaults::tex(),
            fonts: FontSemantics::tex(),
            extensions: ExtensionPolicy::tex(),
            patches: EnginePatches::empty(),
        }
    }

    /// Returns a new `TexCore` with the given patch applied, updating all dependent fields.
    #[must_use]
    pub const fn with_patch(mut self, patch: EnginePatch) -> Self {
        self.patches = self.patches.with(patch);
        match patch {
            EnginePatch::Etex => {
                self.kind = EngineKind::Etex;
                self.primitives = ETEX_PROFILE_PRIMITIVES;
                self.registers = RegisterDefaults::extended();
                self.extensions = ExtensionPolicy {
                    etex: true,
                    ..self.extensions
                };
            }
            EnginePatch::Xetex => {
                self.kind = EngineKind::Xetex;
                self.primitives = XETEX_PROFILE_PRIMITIVES;
                self.catcodes = CatcodeDefaults {
                    unicode_scalars: true,
                };
                self.mathcodes = MathcodeDefaults { unicode_math: true };
                self.registers = RegisterDefaults::extended();
                self.fonts = FontSemantics {
                    unicode_fonts: true,
                    shaped_text: true,
                    unicode_math_fonts: true,
                    host_native_fonts: false,
                };
                self.extensions = ExtensionPolicy {
                    etex: true,
                    xetex: true,
                };
            }
        }
        self
    }

    /// Returns the profile identifier.
    #[must_use]
    pub const fn profile(&self) -> ProfileId {
        self.profile
    }

    /// Returns the engine kind.
    #[must_use]
    pub const fn kind(&self) -> EngineKind {
        self.kind
    }

    /// Returns the primitive list for this core.
    #[must_use]
    pub const fn primitives(&self) -> &'static [PrimitiveSpec] {
        self.primitives
    }

    /// Returns the category code defaults.
    #[must_use]
    pub const fn catcode_defaults(&self) -> CatcodeDefaults {
        self.catcodes
    }

    /// Returns the math code defaults.
    #[must_use]
    pub const fn mathcode_defaults(&self) -> MathcodeDefaults {
        self.mathcodes
    }

    /// Returns the register count defaults.
    #[must_use]
    pub const fn register_defaults(&self) -> RegisterDefaults {
        self.registers
    }

    /// Returns the font semantics.
    #[must_use]
    pub const fn font_semantics(&self) -> FontSemantics {
        self.fonts
    }

    /// Returns the extension policy.
    #[must_use]
    pub const fn extension_policy(&self) -> ExtensionPolicy {
        self.extensions
    }

    /// Returns the active patch flags.
    #[must_use]
    pub const fn patches(&self) -> EnginePatches {
        self.patches
    }

    /// Returns `true` if the given patch is active on this core.
    #[must_use]
    pub const fn has_patch(&self, patch: EnginePatch) -> bool {
        self.patches.contains(patch)
    }
}

/// An individual capability patch that can be layered onto a `TexCore`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnginePatch {
    /// eTeX register and primitive extensions.
    Etex,
    /// XeTeX Unicode and font extensions, which also imply eTeX behavior.
    Xetex,
}

/// Bit-set recording which engine patches are active on a core.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnginePatches {
    etex: bool,
    xetex: bool,
}

impl EnginePatches {
    /// Returns an empty patch set with no patches active.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            etex: false,
            xetex: false,
        }
    }

    /// Returns a new `EnginePatches` with the given patch added.
    #[must_use]
    pub const fn with(mut self, patch: EnginePatch) -> Self {
        match patch {
            EnginePatch::Etex => {
                self.etex = true;
            }
            EnginePatch::Xetex => {
                self.etex = true;
                self.xetex = true;
            }
        }
        self
    }

    /// Derives an `EnginePatches` value from an `ExtensionPolicy`.
    #[must_use]
    pub const fn from_extension_policy(policy: ExtensionPolicy) -> Self {
        Self {
            etex: policy.etex || policy.xetex,
            xetex: policy.xetex,
        }
    }

    /// Returns `true` if the given patch is present in this set.
    #[must_use]
    pub const fn contains(&self, patch: EnginePatch) -> bool {
        match patch {
            EnginePatch::Etex => self.etex,
            EnginePatch::Xetex => self.xetex,
        }
    }
}

/// Primitive descriptor, behavior is in the engine core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveSpec {
    /// Control sequence name without the leading backslash.
    pub name: Cow<'static, str>,
    /// Opcode used for dispatch in the translated engine.
    pub opcode: PrimitiveOpcode,
    /// Coarse category of this primitive's runtime behavior.
    pub kind: PrimitiveKind,
}

/// Integer opcode used to dispatch a primitive in the translated engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimitiveOpcode(pub u32);

/// Coarse category of a primitive's runtime behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrimitiveKind {
    /// Primitive that expands during tokenization.
    Expandable,
    /// Primitive that performs a variable or register assignment.
    Assignment,
    /// Math mode primitive.
    Math,
    /// Box and list construction.
    Layout,
    /// Resource loading such as \\input.
    Resource,
    /// XeTeX extension primitive.
    Extension,
    #[default]
    /// Catch-all for primitives that do not fit any other category.
    Other,
}

/// Default category code configuration for a profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatcodeDefaults {
    /// Whether scalar Unicode code points above U+00FF are valid character tokens.
    pub unicode_scalars: bool,
}

/// Default math code configuration for a profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MathcodeDefaults {
    /// Whether Unicode math code points are used instead of classic 15-bit codes.
    pub unicode_math: bool,
}

/// Default register counts for count, dimension, skip, and token registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegisterDefaults {
    /// Number of count registers available.
    pub count_registers: u16,
    /// Number of dimension registers available.
    pub dimension_registers: u16,
    /// Number of skip registers available.
    pub skip_registers: u16,
    /// Number of token list registers available.
    pub token_registers: u16,
}

impl Default for RegisterDefaults {
    fn default() -> Self {
        Self::tex()
    }
}

impl RegisterDefaults {
    /// 256 registers each, matching classic TeX limits.
    #[must_use]
    pub const fn tex() -> Self {
        Self {
            count_registers: 256,
            dimension_registers: 256,
            skip_registers: 256,
            token_registers: 256,
        }
    }

    /// 32768 registers each, matching eTeX and XeTeX limits.
    #[must_use]
    pub const fn extended() -> Self {
        Self {
            count_registers: 32_768,
            dimension_registers: 32_768,
            skip_registers: 32_768,
            token_registers: 32_768,
        }
    }
}

/// Font loading and shaping capabilities for a profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontSemantics {
    /// Whether Unicode-indexed fonts are supported.
    pub unicode_fonts: bool,
    /// Whether shaped text output via a text shaper is enabled.
    pub shaped_text: bool,
    /// Whether Unicode math fonts are supported.
    pub unicode_math_fonts: bool,
    /// Engine may call host platform font assembly hooks.
    pub host_native_fonts: bool,
}

impl FontSemantics {
    /// Returns plain TeX font semantics with all capabilities disabled.
    #[must_use]
    pub const fn tex() -> Self {
        Self {
            unicode_fonts: false,
            shaped_text: false,
            unicode_math_fonts: false,
            host_native_fonts: false,
        }
    }
}

/// Which TeX extension families the engine activates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtensionPolicy {
    /// Whether eTeX extensions are active.
    pub etex: bool,
    /// Whether XeTeX extensions are active.
    pub xetex: bool,
}

impl ExtensionPolicy {
    /// Plain TeX: no extension families.
    #[must_use]
    pub const fn tex() -> Self {
        Self {
            etex: false,
            xetex: false,
        }
    }
}

const TEX_CORE_PRIMITIVES: &[PrimitiveSpec] = &[
    PrimitiveSpec {
        name: Cow::Borrowed("relax"),
        opcode: PrimitiveOpcode(0),
        kind: PrimitiveKind::Expandable,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("input"),
        opcode: PrimitiveOpcode(1),
        kind: PrimitiveKind::Resource,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("hbox"),
        opcode: PrimitiveOpcode(2),
        kind: PrimitiveKind::Layout,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("vbox"),
        opcode: PrimitiveOpcode(3),
        kind: PrimitiveKind::Layout,
    },
];

const ETEX_PROFILE_PRIMITIVES: &[PrimitiveSpec] = &[
    PrimitiveSpec {
        name: Cow::Borrowed("relax"),
        opcode: PrimitiveOpcode(0),
        kind: PrimitiveKind::Expandable,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("input"),
        opcode: PrimitiveOpcode(1),
        kind: PrimitiveKind::Resource,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("hbox"),
        opcode: PrimitiveOpcode(2),
        kind: PrimitiveKind::Layout,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("vbox"),
        opcode: PrimitiveOpcode(3),
        kind: PrimitiveKind::Layout,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("expanded"),
        opcode: PrimitiveOpcode(100),
        kind: PrimitiveKind::Expandable,
    },
];

const XETEX_PROFILE_PRIMITIVES: &[PrimitiveSpec] = &[
    PrimitiveSpec {
        name: Cow::Borrowed("relax"),
        opcode: PrimitiveOpcode(0),
        kind: PrimitiveKind::Expandable,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("input"),
        opcode: PrimitiveOpcode(1),
        kind: PrimitiveKind::Resource,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("hbox"),
        opcode: PrimitiveOpcode(2),
        kind: PrimitiveKind::Layout,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("vbox"),
        opcode: PrimitiveOpcode(3),
        kind: PrimitiveKind::Layout,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("expanded"),
        opcode: PrimitiveOpcode(100),
        kind: PrimitiveKind::Expandable,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("XeTeXrevision"),
        opcode: PrimitiveOpcode(200),
        kind: PrimitiveKind::Extension,
    },
    PrimitiveSpec {
        name: Cow::Borrowed("font"),
        opcode: PrimitiveOpcode(201),
        kind: PrimitiveKind::Assignment,
    },
];

/// Plain TeX compatible baseline profile.
#[derive(Clone, Copy, Debug, Default)]
pub struct TexProfile;

impl EngineProfile for TexProfile {
    fn id(&self) -> ProfileId {
        self.core().profile()
    }

    fn kind(&self) -> EngineKind {
        self.core().kind()
    }

    fn primitives(&self) -> &[PrimitiveSpec] {
        self.core().primitives()
    }

    fn catcode_defaults(&self) -> CatcodeDefaults {
        self.core().catcode_defaults()
    }

    fn mathcode_defaults(&self) -> MathcodeDefaults {
        self.core().mathcode_defaults()
    }

    fn register_defaults(&self) -> RegisterDefaults {
        self.core().register_defaults()
    }

    fn font_semantics(&self) -> FontSemantics {
        self.core().font_semantics()
    }

    fn extension_policy(&self) -> ExtensionPolicy {
        self.core().extension_policy()
    }
}

impl TexProfile {
    /// Returns the `TexCore` for this profile.
    #[must_use]
    pub const fn core(&self) -> TexCore {
        TexCore::tex(ProfileId("tex"))
    }
}

/// eTeX profile used by modern LaTeX formats.
#[derive(Clone, Copy, Debug, Default)]
pub struct EtexProfile;

impl EngineProfile for EtexProfile {
    fn id(&self) -> ProfileId {
        self.core().profile()
    }

    fn kind(&self) -> EngineKind {
        self.core().kind()
    }

    fn primitives(&self) -> &[PrimitiveSpec] {
        self.core().primitives()
    }

    fn catcode_defaults(&self) -> CatcodeDefaults {
        self.core().catcode_defaults()
    }

    fn mathcode_defaults(&self) -> MathcodeDefaults {
        self.core().mathcode_defaults()
    }

    fn register_defaults(&self) -> RegisterDefaults {
        self.core().register_defaults()
    }

    fn font_semantics(&self) -> FontSemantics {
        self.core().font_semantics()
    }

    fn extension_policy(&self) -> ExtensionPolicy {
        self.core().extension_policy()
    }
}

impl EtexProfile {
    /// Returns the `TexCore` for this profile with the eTeX patch applied.
    #[must_use]
    pub const fn core(&self) -> TexCore {
        TexCore::tex(ProfileId("etex")).with_patch(EnginePatch::Etex)
    }
}

/// XeTeX profile placeholder.
#[derive(Clone, Copy, Debug, Default)]
pub struct XetexProfile;

impl EngineProfile for XetexProfile {
    fn id(&self) -> ProfileId {
        self.core().profile()
    }

    fn kind(&self) -> EngineKind {
        self.core().kind()
    }

    fn primitives(&self) -> &[PrimitiveSpec] {
        self.core().primitives()
    }

    fn catcode_defaults(&self) -> CatcodeDefaults {
        self.core().catcode_defaults()
    }

    fn mathcode_defaults(&self) -> MathcodeDefaults {
        self.core().mathcode_defaults()
    }

    fn register_defaults(&self) -> RegisterDefaults {
        self.core().register_defaults()
    }

    fn font_semantics(&self) -> FontSemantics {
        self.core().font_semantics()
    }

    fn extension_policy(&self) -> ExtensionPolicy {
        self.core().extension_policy()
    }
}

impl XetexProfile {
    /// Returns the `TexCore` for this profile with eTeX and XeTeX patches applied.
    #[must_use]
    pub const fn core(&self) -> TexCore {
        TexCore::tex(ProfileId("xetex"))
            .with_patch(EnginePatch::Etex)
            .with_patch(EnginePatch::Xetex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xetex_profile_declares_unicode_font_and_extension_semantics() {
        let profile = XetexProfile;

        assert_eq!(profile.kind(), EngineKind::Xetex);
        assert!(profile.font_semantics().unicode_fonts);
        assert!(profile.font_semantics().shaped_text);
        assert!(profile.font_semantics().unicode_math_fonts);
        assert!(!profile.font_semantics().host_native_fonts);
        assert!(profile.extension_policy().etex);
        assert!(profile.extension_policy().xetex);
        assert!(profile.register_defaults().count_registers > 256);
    }

    #[test]
    fn runtime_semantics_resolve_profile_gated_primitives() {
        let tex = EngineSemantics::from_profile(&TexProfile);
        let xetex = EngineSemantics::from_profile(&XetexProfile);

        assert!(tex.has_primitive("input"));
        assert!(xetex.has_primitive("input"));
        assert!(!tex.has_primitive("XeTeXrevision"));
        assert!(xetex.has_primitive("XeTeXrevision"));
        assert_eq!(
            xetex
                .primitive("XeTeXrevision")
                .expect("xetex primitive")
                .kind,
            PrimitiveKind::Extension
        );
    }

    #[test]
    fn tex_and_xetex_profiles_share_the_same_core_before_patches() {
        let tex = TexProfile.core();
        let xetex = XetexProfile.core();

        assert_eq!(tex.profile(), ProfileId("tex"));
        assert_eq!(xetex.profile(), ProfileId("xetex"));
        assert!(!tex.has_patch(EnginePatch::Etex));
        assert!(!tex.has_patch(EnginePatch::Xetex));
        assert!(xetex.has_patch(EnginePatch::Etex));
        assert!(xetex.has_patch(EnginePatch::Xetex));
        assert!(xetex.primitives().len() > tex.primitives().len());
        for primitive in tex.primitives() {
            assert!(
                xetex
                    .primitives()
                    .iter()
                    .any(|candidate| candidate.name == primitive.name
                        && candidate.opcode == primitive.opcode
                        && candidate.kind == primitive.kind),
                "xetex should retain TeX core primitive {primitive:?}"
            );
        }
    }

    #[test]
    fn xetex_patch_adds_unicode_and_font_semantics_conditionally() {
        let core = TexCore::tex(ProfileId("custom-xetex")).with_patch(EnginePatch::Xetex);

        assert_eq!(core.kind(), EngineKind::Xetex);
        assert!(core.catcode_defaults().unicode_scalars);
        assert!(core.mathcode_defaults().unicode_math);
        assert!(core.font_semantics().unicode_fonts);
        assert!(core.font_semantics().shaped_text);
        assert!(!core.font_semantics().host_native_fonts);
        assert!(core.extension_policy().etex);
        assert!(core.extension_policy().xetex);
        assert!(core
            .primitives()
            .iter()
            .any(|primitive| primitive.name == "XeTeXrevision"));
    }
}
