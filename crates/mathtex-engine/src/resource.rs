use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Resolves the TeX inputs, packages, fonts and other files a format build reads.
pub trait ResourceProvider {
    /// Resolves the file `request` names, by its exact name, the engine tries suffixed names itself.
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError>;

    /// Resolves `name` as a resource of the given `kind`.
    fn read(&self, name: &str, kind: ResourceKind) -> Result<Resource, ResourceError> {
        self.read_request(&ResourceRequest::new(name, kind))
    }
}

impl<T> ResourceProvider for &T
where
    T: ResourceProvider + ?Sized,
{
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        (**self).read_request(request)
    }
}

/// Resource provider over resources held in memory, keyed by canonical name and kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InMemoryResourceProvider {
    resources: BTreeMap<(String, ResourceKind), Vec<u8>>,
}

impl InMemoryResourceProvider {
    /// Creates an empty provider.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a resource and returns the provider.
    #[must_use]
    pub fn with_resource(
        mut self,
        name: impl Into<String>,
        kind: ResourceKind,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        self.insert(ResourceRequest::new(name, kind), bytes);
        self
    }

    /// Adds or replaces the resource that answers `request`.
    pub fn insert(&mut self, request: ResourceRequest, bytes: impl Into<Vec<u8>>) {
        self.resources
            .insert((request.canonical_name(), request.kind), bytes.into());
    }

    /// Number of resources held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Whether the provider holds no resources.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

impl ResourceProvider for InMemoryResourceProvider {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let name = request.canonical_name();
        match self.resources.get(&(name, request.kind)) {
            Some(bytes) => Ok(Resource::answering(request, bytes.clone())),
            None => Err(ResourceError::not_found(request)),
        }
    }
}

/// Resource provider over a directory, a request's canonical name is a path relative to the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemResourceProvider {
    root: PathBuf,
}

impl FileSystemResourceProvider {
    /// Creates a provider that resolves names under `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory names resolve under.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl ResourceProvider for FileSystemResourceProvider {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let name = request.canonical_name();
        if name.is_empty() {
            return Err(ResourceError::Invalid {
                name,
                message: "resource name is empty".into(),
            });
        }
        let path = Path::new(&name);
        let escapes = path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
        if escapes {
            return Err(ResourceError::Denied {
                name,
                message: "resource path must stay under the provider root".into(),
            });
        }
        match std::fs::read(self.root.join(path)) {
            Ok(bytes) => Ok(Resource::answering(request, bytes)),
            Err(error) => Err(ResourceError::from_io(request, &error)),
        }
    }
}

/// A request for a named resource of a given kind.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResourceRequest {
    /// File name as TeX asked for it, without the owning package.
    pub name: String,
    /// Kind of file TeX asked for.
    pub kind: ResourceKind,
    /// Owning package of an [`ResourceKind::Asset`] request.
    pub package: Option<String>,
}

impl ResourceRequest {
    /// A request for `name` of the given kind.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: ResourceKind) -> Self {
        Self {
            name: name.into(),
            kind,
            package: None,
        }
    }

    /// A request for an asset file owned by `package`.
    #[must_use]
    pub fn asset(package: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ResourceKind::Asset,
            package: Some(package.into()),
        }
    }

    /// The name providers key resources by, `package/name` for an asset and the file name otherwise.
    #[must_use]
    pub fn canonical_name(&self) -> String {
        match (&self.package, self.kind) {
            (Some(package), ResourceKind::Asset) => format!("{package}/{}", self.name),
            _ => self.name.clone(),
        }
    }
}

/// A resolved resource.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Resource {
    /// Canonical name of the request this resource answers, see [`ResourceRequest::canonical_name`].
    pub canonical_name: String,
    /// Kind of the request this resource answers.
    pub kind: ResourceKind,
    /// File contents.
    pub bytes: Vec<u8>,
}

impl Resource {
    /// The resource that answers `request` with `bytes`.
    #[must_use]
    pub fn answering(request: &ResourceRequest, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            canonical_name: request.canonical_name(),
            kind: request.kind,
            bytes: bytes.into(),
        }
    }
}

/// Kind of file a request asks for, which picks the suffixes the engine tries and the search path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ResourceKind {
    /// TeX input file requested by `\input`.
    TexInput,
    /// LaTeX package.
    Package,
    /// LaTeX document class.
    Class,
    /// Font definition file such as `.fd`.
    FontDefinition,
    /// Package support file such as `.clo`, `.def`, `.ldf` or `.cfg`.
    PackageSupport,
    /// Font metric file or font program.
    Font,
    /// Font encoding vector.
    Encoding,
    /// Font map.
    Map,
    /// Engine configuration file.
    Config,
    /// Precompiled format image.
    FormatImage,
    /// Other file owned by a package.
    Asset,
}

impl ResourceKind {
    /// Suffixes tried in order after the bare name when a request has no extension.
    #[must_use]
    pub fn suffixes(self) -> &'static [&'static str] {
        match self {
            Self::TexInput => &[".tex", ".ltx", ".def", ".sty", ".cfg", ".fd"],
            Self::Package => &[".sty", ".tex", ".def", ".ltx"],
            Self::Class => &[".cls"],
            Self::FontDefinition => &[".fd"],
            Self::PackageSupport => &[".def", ".cfg", ".ldf", ".clo", ".sty", ".tex"],
            Self::Font => &[".tfm", ".otf", ".ttf"],
            Self::Encoding => &[".enc"],
            Self::Map => &[".map"],
            Self::Config => &[".cfg", ".cnf", ".tex"],
            Self::FormatImage | Self::Asset => &[],
        }
    }
}

/// Why a resource could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceError {
    /// No resource answers the request.
    NotFound {
        /// Canonical name of the request.
        name: String,
        /// Kind of the request.
        kind: ResourceKind,
    },
    /// The resource exists but cannot serve the request.
    Invalid {
        /// Canonical name of the request.
        name: String,
        /// Why it cannot serve.
        message: String,
    },
    /// The provider's policy or the file system's permissions refuse the request.
    Denied {
        /// Canonical name of the request.
        name: String,
        /// Why it was refused.
        message: String,
    },
    /// Reading the resource failed for another reason.
    Io {
        /// Canonical name of the request.
        name: String,
        /// The failure's kind.
        error: std::io::ErrorKind,
        /// The failure's message.
        message: String,
    },
}

impl ResourceError {
    /// The error for a request no resource answers.
    #[must_use]
    pub fn not_found(request: &ResourceRequest) -> Self {
        Self::NotFound {
            name: request.canonical_name(),
            kind: request.kind,
        }
    }

    /// The error for a failed read, keeping the kind of the I/O failure.
    #[must_use]
    pub fn from_io(request: &ResourceRequest, error: &std::io::Error) -> Self {
        let name = request.canonical_name();
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::not_found(request),
            std::io::ErrorKind::PermissionDenied => Self::Denied {
                name,
                message: error.to_string(),
            },
            kind => Self::Io {
                name,
                error: kind,
                message: error.to_string(),
            },
        }
    }
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { name, kind } => write!(f, "{kind:?} resource not found: {name}"),
            Self::Invalid { name, message } => write!(f, "resource {name} is invalid: {message}"),
            Self::Denied { name, message } => write!(f, "resource {name} was refused: {message}"),
            Self::Io { name, message, .. } => {
                write!(f, "resource {name} could not be read: {message}")
            }
        }
    }
}

impl std::error::Error for ResourceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_provider_keeps_resource_kinds_separate() {
        let provider =
            InMemoryResourceProvider::new().with_resource("cmr10", ResourceKind::Font, b"font");

        assert_eq!(
            provider.read("cmr10", ResourceKind::Font).map(|r| r.bytes),
            Ok(b"font".to_vec())
        );
        assert_eq!(
            provider.read("cmr10", ResourceKind::Package),
            Err(ResourceError::NotFound {
                name: "cmr10".into(),
                kind: ResourceKind::Package,
            })
        );
    }

    #[test]
    fn asset_requests_are_keyed_by_their_package() {
        let mut provider = InMemoryResourceProvider::new();
        provider.insert(ResourceRequest::asset("mhchem", "arrows.dat"), b"asset");

        let resource = provider
            .read_request(&ResourceRequest::asset("mhchem", "arrows.dat"))
            .expect("asset resolves");
        assert_eq!(resource.canonical_name, "mhchem/arrows.dat");
        assert!(provider
            .read_request(&ResourceRequest::asset("other", "arrows.dat"))
            .is_err());
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("mathtex-resource-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create test root");
        dir
    }

    #[test]
    fn filesystem_provider_reads_relative_names_and_refuses_escapes() {
        let root = scratch_dir("read");
        std::fs::write(root.join("plain.tex"), b"\\relax").expect("write resource");
        let provider = FileSystemResourceProvider::new(&root);

        let resource = provider
            .read("plain.tex", ResourceKind::TexInput)
            .expect("relative resource loads");
        assert_eq!(resource.canonical_name, "plain.tex");
        assert_eq!(resource.bytes, b"\\relax");
        assert!(matches!(
            provider.read("../plain.tex", ResourceKind::TexInput),
            Err(ResourceError::Denied { .. })
        ));
        assert!(matches!(
            provider.read("missing.tex", ResourceKind::TexInput),
            Err(ResourceError::NotFound { .. })
        ));
        std::fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn filesystem_read_failures_keep_their_kind() {
        let root = scratch_dir("kind");
        std::fs::create_dir_all(root.join("dir.tex")).expect("create directory");
        let provider = FileSystemResourceProvider::new(&root);

        // Reading a directory fails with an I/O error that is neither missing nor refused.
        let error = provider
            .read("dir.tex", ResourceKind::TexInput)
            .expect_err("a directory is not a file");
        assert!(matches!(error, ResourceError::Io { .. }), "{error:?}");
        let denied = ResourceError::from_io(
            &ResourceRequest::new("x.tex", ResourceKind::TexInput),
            &std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );
        assert!(matches!(denied, ResourceError::Denied { .. }));
        std::fs::remove_dir_all(root).expect("remove test root");
    }
}
