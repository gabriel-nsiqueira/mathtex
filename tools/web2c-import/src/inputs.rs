//! Location of the web2c and C2Rust inputs the bootstrap script produces, gitignored under `cache/`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::model::PatchError;

/// Cache relative paths of every input the patcher reads, in pin manifest order.
pub(crate) const CACHE_INPUTS: [&str; 3] = [
    "c2rust/xetex/src/xetex0.rs",
    "c2rust/xetex/src/xetexini.rs",
    "web2c/xetex/xetex.pool",
];

/// The XeTeX inputs inside one cache directory.
pub(crate) struct Inputs {
    pub(crate) cache: PathBuf,
}

impl Inputs {
    /// Resolves the cache from the environment and checks every input exists.
    pub(crate) fn locate() -> Result<Self, PatchError> {
        Self::at(resolve_cache_dir(std::env::var_os("MATHTEX_WEB2C_CACHE"), &repo_root()))
    }

    /// Uses `cache` as the cache directory and checks every input exists.
    pub(crate) fn at(cache: PathBuf) -> Result<Self, PatchError> {
        let inputs = Self { cache };
        let missing = CACHE_INPUTS
            .iter()
            .map(|relative| inputs.cache.join(relative))
            .filter(|path| !path.is_file())
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(inputs)
        } else {
            Err(PatchError::MissingInputs { paths: missing })
        }
    }

    /// Translated Rust sources, `xetex0.rs` then `xetexini.rs`.
    pub(crate) fn translated_sources(&self) -> [PathBuf; 2] {
        [self.path(CACHE_INPUTS[0]), self.path(CACHE_INPUTS[1])]
    }

    /// The string pool tangled alongside the XeTeX sources.
    pub(crate) fn pool(&self) -> PathBuf {
        self.path(CACHE_INPUTS[2])
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.cache.join(relative)
    }
}

/// The repository root, found from this crate's own location so scratch output roots do not move the cache.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `MATHTEX_WEB2C_CACHE`, else `cache/web2c` in the repository.
fn resolve_cache_dir(explicit: Option<OsString>, repo_root: &Path) -> PathBuf {
    match explicit.filter(|value| !value.is_empty()) {
        Some(explicit) => PathBuf::from(explicit),
        None => repo_root.join("cache/web2c"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(value: &str) -> Option<OsString> {
        Some(OsString::from(value))
    }

    #[test]
    fn cache_dir_prefers_explicit_then_the_repository() {
        let repo = Path::new("/repo");
        assert_eq!(resolve_cache_dir(os("/explicit"), repo), PathBuf::from("/explicit"));
        assert_eq!(resolve_cache_dir(os(""), repo), PathBuf::from("/repo/cache/web2c"));
        assert_eq!(resolve_cache_dir(None, repo), PathBuf::from("/repo/cache/web2c"));
    }

    #[test]
    fn missing_inputs_are_all_named() {
        let empty = std::env::temp_dir().join("mathtex-web2c-empty-cache");
        let Err(PatchError::MissingInputs { paths }) = Inputs::at(empty.clone()) else {
            panic!("an empty cache must report missing inputs");
        };
        assert_eq!(paths.len(), CACHE_INPUTS.len());
        assert_eq!(paths[2], empty.join("web2c/xetex/xetex.pool"));
    }
}
