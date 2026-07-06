//! Native filesystem [`ResourceProvider`] over a TeXLive tree using `ls-R` and texmf.cnf priority.

#![cfg(feature = "std")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::resource::{Resource, ResourceError, ResourceKind, ResourceProvider, ResourceRequest};

/// search format, mapping each resource kind to its ordered `texmf.cnf` prefixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fmt {
    /// TeX inputs: `.tex`/`.sty`/`.cls`/`.def`/`.cfg`/`.fd`/`.ltx`/`.clo`.
    Tex,
    /// TeX font metrics: `.tfm`.
    Tfm,
    /// OpenType/TrueType font programs.
    OpenType,
    /// Font encoding files: `.enc`.
    Enc,
    /// Font map files: `.map`.
    Map,
}

impl Fmt {
    /// Ordered relative directory prefixes, highest priority first, matching each prefix subtree.
    fn prefixes(self) -> &'static [&'static str] {
        match self {
            // Search entries for TeX input files.
            Fmt::Tex => &["tex/xelatex", "tex/latex", "tex/xetex", "tex/generic", "tex"],
            // Search entries for TeX font metrics.
            Fmt::Tfm => &["fonts/tfm"],
            // Search entries for font programs.
            Fmt::OpenType => &["fonts/opentype", "fonts/truetype"],
            // Search entries for font encodings.
            Fmt::Enc => &["fonts/enc"],
            // Search entries for font maps.
            Fmt::Map => &["fonts/map"],
        }
    }
}

/// Resource provider backed by a TeXMF tree root and its `ls-R` filename index.
#[derive(Debug)]
pub struct TexmfResources {
    root: PathBuf,
    /// Basename to relative dirs containing it, in `ls-R` order.
    index: HashMap<String, Vec<String>>,
}

impl TexmfResources {
    /// Returns `None` when `ls-R` is absent or empty, run `mktexlsr` to generate it.
    #[must_use]
    pub fn from_root(root: impl Into<PathBuf>) -> Option<Self> {
        let root = root.into();
        let bytes = std::fs::read(root.join("ls-R")).ok()?;
        let text = String::from_utf8_lossy(&bytes);

        let mut index: HashMap<String, Vec<String>> = HashMap::new();
        let mut cur_dir = String::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() || line.starts_with('%') {
                continue;
            }
            if let Some(dir) = line.strip_suffix(':') {
                cur_dir = dir.strip_prefix("./").unwrap_or(dir).to_string();
                continue;
            }
            index
                .entry(line.to_string())
                .or_default()
                .push(cur_dir.clone());
        }

        if index.is_empty() {
            return None;
        }
        Some(Self { root, index })
    }

    /// Returns the filesystem root of the TeXMF tree.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the number of basename entries in the `ls-R` index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns true when the `ls-R` index contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Normalizes an engine resource name to the basename used as a lookup key.
    fn normalize(name: &str) -> String {
        let mut n = name.trim();
        loop {
            if let Some(s) = n.strip_prefix("./") {
                n = s;
            } else if let Some(s) = n.strip_prefix("[]") {
                n = s;
            } else if let Some(s) = n.strip_prefix(':') {
                n = s;
            } else {
                break;
            }
        }
        let n = n.trim_matches(|c| c == '[' || c == ']' || c == '"' || c == '\'');
        n.rsplit(['/', '\\']).next().unwrap_or(n).to_string()
    }

    /// Maps a resource kind and filename extension to the texmf.cnf search format.
    fn format_for(kind: ResourceKind, filename: &str) -> Fmt {
        let lower = filename.to_ascii_lowercase();
        match kind {
            ResourceKind::Encoding => Fmt::Enc,
            ResourceKind::Map => Fmt::Map,
            ResourceKind::Font => {
                if lower.ends_with(".otf") || lower.ends_with(".ttf") || lower.ends_with(".otc") {
                    Fmt::OpenType
                } else {
                    Fmt::Tfm
                }
            }
            // Use the tex tree by default, refining by extension for stray font assets.
            _ => {
                if lower.ends_with(".enc") {
                    Fmt::Enc
                } else if lower.ends_with(".map") {
                    Fmt::Map
                } else if lower.ends_with(".tfm") {
                    Fmt::Tfm
                } else if lower.ends_with(".otf") || lower.ends_with(".ttf") {
                    Fmt::OpenType
                } else {
                    Fmt::Tex
                }
            }
        }
    }

    /// Returns the path for the first search prefix subtree containing the filename.
    fn resolve(&self, filename: &str, fmt: Fmt) -> Option<PathBuf> {
        let dirs = self.index.get(filename)?;
        for prefix in fmt.prefixes() {
            for dir in dirs {
                if dir == prefix || dir.strip_prefix(prefix).is_some_and(|r| r.starts_with('/')) {
                    return Some(self.root.join(dir).join(filename));
                }
            }
        }
        None
    }

    /// Candidate filenames for a request, including kind suffixes when no extension is present.
    fn candidates(request: &ResourceRequest) -> Vec<String> {
        let base = Self::normalize(&request.canonical_name());
        let mut out = vec![base.clone()];
        if Path::new(&base).extension().is_none() {
            let exts: &[&str] = match request.kind {
                ResourceKind::Package => &[".sty", ".tex", ".def", ".ltx"],
                ResourceKind::Class => &[".cls"],
                ResourceKind::FontDefinition => &[".fd"],
                ResourceKind::PackageSupport => &[".def", ".cfg", ".ldf", ".clo", ".sty", ".tex"],
                ResourceKind::Config => &[".cfg", ".cnf", ".tex"],
                ResourceKind::Encoding => &[".enc"],
                ResourceKind::Map => &[".map"],
                ResourceKind::Font => &[".tfm", ".otf", ".ttf"],
                ResourceKind::TexInput => &[".tex", ".ltx", ".def", ".sty", ".cfg", ".fd"],
                _ => &[".tex", ".sty", ".def", ".cfg", ".ltx", ".fd", ".cls", ".enc"],
            };
            for e in exts {
                out.push(format!("{base}{e}"));
            }
        }
        out
    }
}

impl ResourceProvider for TexmfResources {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        for cand in Self::candidates(request) {
            let fmt = Self::format_for(request.kind, &cand);
            if let Some(path) = self.resolve(&cand, fmt) {
                if let Ok(bytes) = std::fs::read(&path) {
                    return Ok(Resource::from_request(request, bytes));
                }
            }
        }
        Err(ResourceError::NotFound {
            name: request.canonical_name(),
            kind: request.kind,
        })
    }
}
