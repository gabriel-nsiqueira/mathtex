use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::resource::{Resource, ResourceError, ResourceKind, ResourceProvider, ResourceRequest};

/// Environment variable naming a `texmf-dist` root, read by [`TexmfResources::discover`].
pub const TEXMF_ROOT_ENV: &str = "MATHTEX_TEXMF_ROOT";

/// Search tree of a file, each with its `texmf.cnf` path prefixes in priority order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchPath {
    Tex,
    Tfm,
    OpenType,
    Enc,
    Map,
}

impl SearchPath {
    /// Relative directory prefixes, highest priority first, each matching its whole subtree.
    fn prefixes(self) -> &'static [&'static str] {
        match self {
            // XeLaTeX's TEXINPUTS order, the whole tex tree comes after the narrower roots.
            Self::Tex => &[
                "tex/xelatex",
                "tex/latex",
                "tex/xetex",
                "tex/generic",
                "tex",
            ],
            Self::Tfm => &["fonts/tfm"],
            Self::OpenType => &["fonts/opentype", "fonts/truetype"],
            Self::Enc => &["fonts/enc"],
            Self::Map => &["fonts/map"],
        }
    }

    /// The tree a request searches, font kinds by extension and other kinds refined by extension.
    fn for_request(kind: ResourceKind, filename: &str) -> Self {
        let lower = filename.to_ascii_lowercase();
        let is_font_program = [".otf", ".ttf", ".otc"]
            .iter()
            .any(|ext| lower.ends_with(ext));
        match kind {
            ResourceKind::Encoding => Self::Enc,
            ResourceKind::Map => Self::Map,
            ResourceKind::Font if is_font_program => Self::OpenType,
            ResourceKind::Font => Self::Tfm,
            _ if lower.ends_with(".enc") => Self::Enc,
            _ if lower.ends_with(".map") => Self::Map,
            _ if lower.ends_with(".tfm") => Self::Tfm,
            _ if is_font_program => Self::OpenType,
            _ => Self::Tex,
        }
    }
}

/// Resource provider over a TeX Live `texmf-dist` tree by exact file name, through its `ls-R` index.
#[derive(Clone, Debug)]
pub struct TexmfResources {
    root: PathBuf,
    /// Basename to the relative directories holding it, in `ls-R` order.
    index: Arc<HashMap<String, Vec<String>>>,
}

impl TexmfResources {
    /// Reads the `ls-R` index of `root`, a `texmf-dist` directory.
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self, TexmfError> {
        let root = root.into();
        let index_path = root.join("ls-R");
        let bytes = std::fs::read(&index_path).map_err(|error| TexmfError::Index {
            path: index_path.clone(),
            message: error.to_string(),
        })?;
        let text = String::from_utf8_lossy(&bytes);
        let mut index: HashMap<String, Vec<String>> = HashMap::new();
        let mut dir = String::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('%') {
                continue;
            }
            if let Some(header) = line.strip_suffix(':') {
                dir = header.strip_prefix("./").unwrap_or(header).to_string();
                continue;
            }
            index.entry(line.to_string()).or_default().push(dir.clone());
        }
        if index.is_empty() {
            return Err(TexmfError::EmptyIndex { path: index_path });
        }
        Ok(Self {
            root,
            index: Arc::new(index),
        })
    }

    /// Finds the tree through `MATHTEX_TEXMF_ROOT`, else `kpsewhich -var-value TEXMFDIST`.
    pub fn discover() -> Result<Self, TexmfError> {
        if let Some(root) = std::env::var_os(TEXMF_ROOT_ENV) {
            return Self::from_root(root);
        }
        let output = std::process::Command::new("kpsewhich")
            .args(["-var-value", "TEXMFDIST"])
            .output()
            .map_err(|error| TexmfError::NoRoot {
                message: format!("{TEXMF_ROOT_ENV} is unset and kpsewhich did not run: {error}"),
            })?;
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !output.status.success() || root.is_empty() {
            return Err(TexmfError::NoRoot {
                message: format!("{TEXMF_ROOT_ENV} is unset and kpsewhich knows no TEXMFDIST"),
            });
        }
        Self::from_root(root)
    }

    /// The tree's root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of distinct file names in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the index holds no file names, never true for a provider that was built.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The file a request names, found in the highest priority directory of its search path.
    #[must_use]
    pub fn resolve(&self, request: &ResourceRequest) -> Option<PathBuf> {
        let name = basename(&request.canonical_name());
        let dirs = self.index.get(&name)?;
        for prefix in SearchPath::for_request(request.kind, &name).prefixes() {
            for dir in dirs {
                let under = dir
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
                if under {
                    return Some(self.root.join(dir).join(&name));
                }
            }
        }
        None
    }
}

/// The file name TeX means, without `./`, XeTeX's `[]` and `:` prefixes, brackets, quotes and directories.
fn basename(name: &str) -> String {
    let mut name = name.trim();
    loop {
        if let Some(rest) = name.strip_prefix("./").or_else(|| name.strip_prefix("[]")) {
            name = rest;
        } else if let Some(rest) = name.strip_prefix(':') {
            name = rest;
        } else {
            break;
        }
    }
    let name = name.trim_matches(|c| matches!(c, '[' | ']' | '"' | '\''));
    name.rsplit(['/', '\\']).next().unwrap_or(name).to_string()
}

impl ResourceProvider for TexmfResources {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let path = self
            .resolve(request)
            .ok_or_else(|| ResourceError::not_found(request))?;
        std::fs::read(&path)
            .map(|bytes| Resource::answering(request, bytes))
            .map_err(|error| ResourceError::from_io(request, &error))
    }
}

/// Why a TeX Live tree could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TexmfError {
    /// No root was configured or discoverable.
    NoRoot {
        /// What was tried.
        message: String,
    },
    /// The `ls-R` index could not be read, `mktexlsr` writes it.
    Index {
        /// Path of the index.
        path: PathBuf,
        /// The read failure.
        message: String,
    },
    /// The `ls-R` index lists no files.
    EmptyIndex {
        /// Path of the index.
        path: PathBuf,
    },
}

impl fmt::Display for TexmfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRoot { message } => f.write_str(message),
            Self::Index { path, message } => {
                write!(f, "cannot read {}: {message}", path.display())
            }
            Self::EmptyIndex { path } => write!(f, "{} lists no files", path.display()),
        }
    }
}

impl std::error::Error for TexmfError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> PathBuf {
        let root = std::env::temp_dir().join(format!("mathtex-texmf-{}", std::process::id()));
        for dir in ["tex/latex/base", "tex/plain/base", "fonts/tfm/public/cm"] {
            std::fs::create_dir_all(root.join(dir)).expect("create tree");
        }
        std::fs::write(root.join("tex/latex/base/x.tex"), b"latex").expect("write");
        std::fs::write(root.join("tex/plain/base/x.tex"), b"plain").expect("write");
        std::fs::write(root.join("fonts/tfm/public/cm/cmr10.tfm"), b"tfm").expect("write");
        let index = "% ls-R\n./tex/plain/base:\nx.tex\n\n./tex/latex/base:\nx.tex\n\n./fonts/tfm/public/cm:\ncmr10.tfm\n";
        std::fs::write(root.join("ls-R"), index).expect("write index");
        root
    }

    #[test]
    fn requests_resolve_by_search_path_priority() {
        let root = tree();
        let texmf = TexmfResources::from_root(&root).expect("index");
        assert_eq!(texmf.len(), 2);
        // tex/latex outranks the generic tex tree although ls-R lists the plain copy first.
        let latex = texmf
            .read("./x.tex", ResourceKind::TexInput)
            .expect("x.tex");
        assert_eq!(latex.bytes, b"latex");
        let tfm = texmf.read("cmr10.tfm", ResourceKind::Font).expect("tfm");
        assert_eq!(tfm.bytes, b"tfm");
        // Suffixes are the engine's job, a bare name does not resolve.
        assert!(texmf.read("cmr10", ResourceKind::Font).is_err());
        std::fs::remove_dir_all(root).expect("remove tree");
    }

    #[test]
    fn a_root_without_an_index_says_why() {
        let missing = std::env::temp_dir().join("mathtex-texmf-missing-root");
        assert!(matches!(
            TexmfResources::from_root(&missing),
            Err(TexmfError::Index { .. })
        ));
    }
}
