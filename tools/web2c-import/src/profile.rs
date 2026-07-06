//! Engine import profile specifications derived from [`Web2cBootstrapRecipe`].

use crate::{texlive_bootstrap_recipe, EngineKind, Web2cBootstrapRecipe};

/// Capabilities a profile turns on over the shared TeX core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineCapabilities {
    /// Enables eTeX behavior in the shared core.
    pub etex: bool,
    /// Enables Unicode scalar character codes.
    pub unicode_scalars: bool,
    /// Enables Unicode math handling.
    pub unicode_math: bool,
    /// Enables native font support.
    pub native_fonts: bool,
    /// Enables page output handling.
    pub output: bool,
}

/// One formalized engine import profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineImportProfile {
    /// Stable profile id such as `"tex"`, `"etex"`, or `"xetex"`.
    pub id: &'static str,
    /// Engine kind selected by this profile.
    pub kind: EngineKind,
    /// Source chain with shared WEB source first, then change files.
    pub source_chain: Vec<&'static str>,
    /// Pool file generated for this profile.
    pub pool_file: &'static str,
    /// Runtime state bounds for this profile.
    pub state_bounds: StateBounds,
    /// Capabilities enabled by this profile.
    pub capabilities: EngineCapabilities,
    /// Boundary identifiers allowed to remain after selection.
    pub allowed_boundaries: Vec<&'static str>,
}

/// Inclusive runtime `mem` array bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateBounds {
    /// Minimum inclusive index of the runtime `mem` array.
    pub mem_min: i32,
    /// Maximum inclusive index of the runtime `mem` array.
    pub mem_top: i32,
}

/// Default `mem` bounds shared by every profile (`self.memtop = 19_999_999` in runtime.rs).
pub const DEFAULT_STATE_BOUNDS: StateBounds = StateBounds {
    mem_min: 0,
    mem_top: 19_999_999,
};

impl EngineImportProfile {
    #[must_use]
    /// Returns every built in import profile.
    pub fn all_from_recipe() -> Vec<Self> {
        let recipe = texlive_bootstrap_recipe();
        vec![
            Self::tex(&recipe),
            Self::etex(&recipe),
            Self::xetex(&recipe),
        ]
    }

    fn tex(recipe: &Web2cBootstrapRecipe) -> Self {
        Self {
            id: "tex",
            kind: EngineKind::Tex,
            source_chain: shared_source_chain(recipe),
            pool_file: "generated/web2c/tex/tex.pool",
            state_bounds: DEFAULT_STATE_BOUNDS,
            capabilities: EngineCapabilities {
                etex: false,
                unicode_scalars: false,
                unicode_math: false,
                native_fonts: false,
                output: true,
            },
            allowed_boundaries: Vec::new(),
        }
    }

    fn etex(recipe: &Web2cBootstrapRecipe) -> Self {
        Self {
            id: "etex",
            kind: EngineKind::Etex,
            // eTeX behavior is runtime gated over the shared core, so its source chain is identical.
            source_chain: shared_source_chain(recipe),
            pool_file: "generated/web2c/tex/tex.pool",
            state_bounds: DEFAULT_STATE_BOUNDS,
            capabilities: EngineCapabilities {
                etex: true,
                unicode_scalars: false,
                unicode_math: false,
                native_fonts: false,
                output: true,
            },
            allowed_boundaries: Vec::new(),
        }
    }

    fn xetex(recipe: &Web2cBootstrapRecipe) -> Self {
        let mut source_chain = shared_source_chain(recipe);
        for change_file in recipe.xetex_patch_changes {
            source_chain.push(*change_file);
        }
        Self {
            id: "xetex",
            kind: EngineKind::Xetex,
            source_chain,
            pool_file: "generated/web2c/xetex/xetex.pool",
            state_bounds: DEFAULT_STATE_BOUNDS,
            capabilities: EngineCapabilities {
                etex: true,
                unicode_scalars: true,
                unicode_math: true,
                native_fonts: false,
                output: true,
            },
            // Native font and text measurement boundaries are adapted from the boundary registry.
            allowed_boundaries: crate::select::UNSUPPORTED_EXTERNAL_BOUNDARY_IDENTIFIERS.to_vec(),
        }
    }

    #[must_use]
    /// Returns whether this profile allows a boundary identifier.
    pub fn allows_boundary(&self, identifier: &str) -> bool {
        self.allowed_boundaries.contains(&identifier)
    }
}

fn shared_source_chain(recipe: &Web2cBootstrapRecipe) -> Vec<&'static str> {
    let mut chain = vec![recipe.shared_tex_web];
    for change_file in recipe.shared_tex_changes {
        chain.push(*change_file);
    }
    chain
}
