use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::path::{Component, Path, PathBuf};

use mathtex_ir::ByteSpan;

/// Resolves TeX inputs, packages, fonts, encodings, maps, and config assets.
pub trait ResourceProvider {
    /// Resolve an arbitrary resource described by `request`.
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError>;

    /// Resolve `name` as a resource of the given `kind`.
    fn read(&self, name: &str, kind: ResourceKind) -> Result<Resource, ResourceError> {
        self.read_request(&ResourceRequest::new(name, kind))
    }

    /// Resolve a TeX input file by name.
    fn read_tex_input(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::TexInput)
    }

    /// Resolve a LaTeX package file by name.
    fn read_package(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::Package)
    }

    /// Resolve a LaTeX class file by name.
    fn read_class(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::Class)
    }

    /// Resolve a font definition file by name.
    fn read_font_definition(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::FontDefinition)
    }

    /// Resolve a package support file by name.
    fn read_package_support(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::PackageSupport)
    }

    /// Resolve a font program or collection by name.
    fn read_font(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::Font)
    }

    /// Resolve a font encoding file by name.
    fn read_encoding(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::Encoding)
    }

    /// Resolve a font map file by name.
    fn read_map(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::Map)
    }

    /// Resolve a configuration file by name.
    fn read_config(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::Config)
    }

    /// Resolve a precompiled format image by name.
    fn read_format_image(&self, name: &str) -> Result<Resource, ResourceError> {
        self.read(name, ResourceKind::FormatImage)
    }

    /// Resolve a package-owned asset by package and asset name.
    fn read_asset(&self, package: &str, name: &str) -> Result<Resource, ResourceError> {
        self.read_request(&ResourceRequest::asset(package, name))
    }
}

impl<T> ResourceProvider for &T
where
    T: ResourceProvider,
{
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        (*self).read_request(request)
    }
}

/// Resource provider backed by a caller supplied resolver function.
#[derive(Clone, Debug)]
pub struct ResolverResourceProvider<F> {
    resolver: F,
}

impl<F> ResolverResourceProvider<F> {
    /// Create a provider that delegates to the given resolver function.
    #[must_use]
    pub fn new(resolver: F) -> Self {
        Self { resolver }
    }

    /// Return a reference to the underlying resolver function.
    #[must_use]
    pub fn resolver(&self) -> &F {
        &self.resolver
    }
}

impl<F> ResourceProvider for ResolverResourceProvider<F>
where
    F: Fn(&ResourceRequest) -> Result<Resource, ResourceError>,
{
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        (self.resolver)(request)
    }
}

/// Resource provider that holds resources in a keyed in-memory map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InMemoryResourceProvider {
    resources: BTreeMap<ResourceKey, Resource>,
}

impl InMemoryResourceProvider {
    /// Create an empty in-memory provider.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a resource by name, kind, and bytes, returning the updated provider.
    #[must_use]
    pub fn with_resource(
        mut self,
        name: impl Into<String>,
        kind: ResourceKind,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        self.insert(name, kind, bytes);
        self
    }

    /// Insert a resource by name, kind, and bytes.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        kind: ResourceKind,
        bytes: impl Into<Vec<u8>>,
    ) {
        self.insert_request(ResourceRequest::new(name, kind), bytes);
    }

    /// Insert a resource from a prepared request and bytes.
    pub fn insert_request(&mut self, request: ResourceRequest, bytes: impl Into<Vec<u8>>) {
        let key = ResourceKey::from_request(&request);
        let resource = Resource {
            canonical_name: request.canonical_name(),
            kind: request.kind,
            bytes: bytes.into(),
        };
        self.resources.insert(key, resource);
    }

    /// Return the number of resources stored in this provider.
    #[must_use]
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Return true if this provider contains no resources.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

impl ResourceProvider for InMemoryResourceProvider {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let key = ResourceKey::from_request(request);

        self.resources
            .get(&key)
            .cloned()
            .ok_or_else(|| ResourceError::NotFound {
                name: request.canonical_name(),
                kind: request.kind,
            })
    }
}

/// Resource provider that checks an override provider before falling back to the base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayResourceProvider<Overrides, Base> {
    overrides: Overrides,
    base: Base,
}

impl<Overrides, Base> OverlayResourceProvider<Overrides, Base> {
    /// Create a provider that checks `overrides` before `base`.
    #[must_use]
    pub fn new(overrides: Overrides, base: Base) -> Self {
        Self { overrides, base }
    }

    /// Return a reference to the override provider.
    #[must_use]
    pub fn overrides(&self) -> &Overrides {
        &self.overrides
    }

    /// Return a reference to the base provider.
    #[must_use]
    pub fn base(&self) -> &Base {
        &self.base
    }
}

impl<Overrides, Base> ResourceProvider for OverlayResourceProvider<Overrides, Base>
where
    Overrides: ResourceProvider,
    Base: ResourceProvider,
{
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        match self.overrides.read_request(request) {
            Ok(resource) => Ok(resource),
            Err(ResourceError::NotFound { .. }) => self.base.read_request(request),
            Err(error) => Err(error),
        }
    }
}

/// Named in memory resource universe suitable for embedded TeX/package bundles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBundle {
    id: String,
    resources: InMemoryResourceProvider,
}

impl ResourceBundle {
    /// Create an empty bundle with the given identifier.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            resources: InMemoryResourceProvider::new(),
        }
    }

    /// Return the identifier of this bundle.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Add a resource by name, kind, and bytes, returning the updated bundle.
    #[must_use]
    pub fn with_resource(
        mut self,
        name: impl Into<String>,
        kind: ResourceKind,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        self.resources.insert(name, kind, bytes);
        self
    }

    /// Add a resource from a prepared request and bytes, returning the updated bundle.
    #[must_use]
    pub fn with_request(mut self, request: ResourceRequest, bytes: impl Into<Vec<u8>>) -> Self {
        self.resources.insert_request(request, bytes);
        self
    }
}

impl ResourceProvider for ResourceBundle {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        self.resources.read_request(request)
    }
}

/// Filesystem backed provider, gated on the `std` feature to keep the engine `no_std` compatible.
#[cfg(feature = "std")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemResourceProvider {
    root: PathBuf,
}

#[cfg(feature = "std")]
impl FileSystemResourceProvider {
    /// Create a provider that resolves resources relative to `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Return the root directory this provider resolves against.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn validate_name(name: &str, kind: ResourceKind) -> Result<&Path, ResourceError> {
        let path = Path::new(name);
        let invalid_component = path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });

        if path.is_absolute() || invalid_component {
            return Err(ResourceError::Denied {
                name: name.to_string(),
                message: "resource path must be relative to the provider root".to_string(),
            });
        }

        if name.is_empty() {
            return Err(ResourceError::Invalid {
                name: name.to_string(),
                message: "resource name cannot be empty".to_string(),
            });
        }

        let _ = kind;
        Ok(path)
    }
}

#[cfg(feature = "std")]
impl ResourceProvider for FileSystemResourceProvider {
    fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
        let name = request.canonical_name();
        let relative_path = Self::validate_name(&name, request.kind)?;
        let path = self.root.join(relative_path);
        let bytes = std::fs::read(&path).map_err(|error| ResourceError::NotFound {
            name: format!("{} ({})", name, error),
            kind: request.kind,
        })?;

        Ok(Resource {
            canonical_name: path.to_string_lossy().into_owned(),
            kind: request.kind,
            bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ResourceKey {
    name: String,
    kind: ResourceKind,
}

impl ResourceKey {
    fn from_request(request: &ResourceRequest) -> Self {
        Self {
            name: request.canonical_name(),
            kind: request.kind,
        }
    }
}

/// A request to resolve a named resource of a given kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRequest {
    /// Base name of the resource being requested.
    pub name: String,
    /// Kind of resource being requested.
    pub kind: ResourceKind,
    /// Owner package, set only for `ResourceKind::Asset` requests.
    pub package: Option<String>,
    /// Source location that triggered this request, if available.
    pub source: Option<ResourceRequestSource>,
}

/// Source location that triggered a resource request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRequestSource {
    /// Name of the source file that issued the request.
    pub name: String,
    /// Byte span within the source file that triggered the request.
    pub span: ByteSpan,
}

impl ResourceRequest {
    /// Create a basic request for a named resource of the given kind.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: ResourceKind) -> Self {
        Self {
            name: name.into(),
            kind,
            package: None,
            source: None,
        }
    }

    /// Create a `ResourceKind::Asset` request owned by a package.
    #[must_use]
    pub fn asset(package: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: ResourceKind::Asset,
            package: Some(package.into()),
            source: None,
        }
    }

    /// Attach a source location to this request.
    #[must_use]
    pub fn with_source(mut self, name: impl Into<String>, span: ByteSpan) -> Self {
        self.source = Some(ResourceRequestSource {
            name: name.into(),
            span,
        });
        self
    }

    /// Canonical request name used by simple providers.
    #[must_use]
    pub fn canonical_name(&self) -> String {
        match (&self.package, self.kind) {
            (Some(package), ResourceKind::Asset) => {
                let mut name = String::with_capacity(package.len() + 1 + self.name.len());
                name.push_str(package);
                name.push('/');
                name.push_str(&self.name);
                name
            }
            _ => self.name.clone(),
        }
    }
}

/// A resolved resource consisting of its canonical name, kind, and raw bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resource {
    /// Canonical path or name under which this resource was resolved.
    pub canonical_name: String,
    /// Kind of this resource.
    pub kind: ResourceKind,
    /// Raw bytes of the resource content.
    pub bytes: Vec<u8>,
}

impl Resource {
    /// Create a resource from its canonical name, kind, and bytes.
    #[must_use]
    pub fn new(
        canonical_name: impl Into<String>,
        kind: ResourceKind,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            canonical_name: canonical_name.into(),
            kind,
            bytes: bytes.into(),
        }
    }

    /// Build a resource from an existing request and the resolved bytes.
    #[must_use]
    pub fn from_request(request: &ResourceRequest, bytes: impl Into<Vec<u8>>) -> Self {
        Self::new(request.canonical_name(), request.kind, bytes)
    }
}

/// Discriminant that classifies the kind of a resource request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ResourceKind {
    /// TeX input file requested by `\input`.
    TexInput,
    /// LaTeX package requested by `\usepackage`.
    Package,
    /// LaTeX class requested by `\documentclass`.
    Class,
    /// TeX font definition file such as `.fd`.
    FontDefinition,
    /// Package support input such as `.clo`, `.def`, `.ldf`, or `.cfg`.
    PackageSupport,
    /// Font program or collection.
    Font,
    /// Font encoding vector file.
    Encoding,
    /// Font map file.
    Map,
    /// Engine configuration file such as `texmf.cnf`.
    Config,
    /// Precompiled format image.
    FormatImage,
    /// Other package owned asset.
    Asset,
}

/// Error returned when a resource request cannot be fulfilled.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceError {
    /// The requested resource does not exist in the configured universe.
    NotFound {
        /// Name of the resource that was not found.
        name: String,
        /// Kind of resource that was requested.
        kind: ResourceKind,
    },
    /// The resource exists but is not valid for the requested purpose.
    Invalid {
        /// Name of the invalid resource.
        name: String,
        /// Description of why the resource is invalid.
        message: String,
    },
    /// The resolver policy denied the request.
    Denied {
        /// Name of the denied resource.
        name: String,
        /// Reason the request was denied.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_provider_loads_package_without_filesystem() {
        let provider = InMemoryResourceProvider::new().with_resource(
            "amsmath.sty",
            ResourceKind::Package,
            br"\ProvidesPackage{amsmath}".to_vec(),
        );

        let resource = provider
            .read("amsmath.sty", ResourceKind::Package)
            .expect("package should resolve");

        assert_eq!(resource.canonical_name, "amsmath.sty");
        assert_eq!(resource.kind, ResourceKind::Package);
        assert_eq!(resource.bytes, br"\ProvidesPackage{amsmath}".to_vec());
    }

    #[test]
    fn in_memory_provider_keeps_resource_kinds_separate() {
        let provider =
            InMemoryResourceProvider::new().with_resource("cmr10", ResourceKind::Font, b"font");

        let error = provider
            .read("cmr10", ResourceKind::Package)
            .expect_err("font must not satisfy package lookup");

        assert_eq!(
            error,
            ResourceError::NotFound {
                name: "cmr10".to_string(),
                kind: ResourceKind::Package,
            }
        );
    }

    #[test]
    fn provider_convenience_methods_cover_tex_resource_kinds() {
        let provider = InMemoryResourceProvider::new()
            .with_resource("plain.tex", ResourceKind::TexInput, b"tex")
            .with_resource("amsmath.sty", ResourceKind::Package, b"package")
            .with_resource("article.cls", ResourceKind::Class, b"class")
            .with_resource("ot1cmr.fd", ResourceKind::FontDefinition, b"fd")
            .with_resource("size10.clo", ResourceKind::PackageSupport, b"support")
            .with_resource("latinmodern-math.otf", ResourceKind::Font, b"font")
            .with_resource("t1.enc", ResourceKind::Encoding, b"encoding")
            .with_resource("pdftex.map", ResourceKind::Map, b"map")
            .with_resource("texmf.cnf", ResourceKind::Config, b"config")
            .with_resource("latex.fmt", ResourceKind::FormatImage, b"format");

        assert_eq!(
            provider.read_tex_input("plain.tex").expect("tex").bytes,
            b"tex"
        );
        assert_eq!(
            provider.read_package("amsmath.sty").expect("package").bytes,
            b"package"
        );
        assert_eq!(
            provider.read_class("article.cls").expect("class").bytes,
            b"class"
        );
        assert_eq!(
            provider
                .read_font_definition("ot1cmr.fd")
                .expect("font definition")
                .bytes,
            b"fd"
        );
        assert_eq!(
            provider
                .read_package_support("size10.clo")
                .expect("package support")
                .bytes,
            b"support"
        );
        assert_eq!(
            provider
                .read_font("latinmodern-math.otf")
                .expect("font")
                .bytes,
            b"font"
        );
        assert_eq!(
            provider.read_encoding("t1.enc").expect("encoding").bytes,
            b"encoding"
        );
        assert_eq!(provider.read_map("pdftex.map").expect("map").bytes, b"map");
        assert_eq!(
            provider.read_config("texmf.cnf").expect("config").bytes,
            b"config"
        );
        assert_eq!(
            provider
                .read_format_image("latex.fmt")
                .expect("format")
                .bytes,
            b"format"
        );
    }

    #[test]
    fn typed_asset_requests_include_package_owner() {
        let mut provider = InMemoryResourceProvider::new();
        provider.insert_request(ResourceRequest::asset("mhchem", "arrows.dat"), b"asset");

        let resource = provider
            .read_asset("mhchem", "arrows.dat")
            .expect("asset should resolve");

        assert_eq!(resource.kind, ResourceKind::Asset);
        assert_eq!(resource.canonical_name, "mhchem/arrows.dat");
        assert_eq!(resource.bytes, b"asset");
    }

    #[test]
    fn overlay_provider_prefers_overrides_before_base_bundle() {
        let base = ResourceBundle::new("latex-base")
            .with_resource("article.cls", ResourceKind::TexInput, b"base")
            .with_resource("amsmath.sty", ResourceKind::Package, b"base-ams");
        let overrides = InMemoryResourceProvider::new().with_resource(
            "amsmath.sty",
            ResourceKind::Package,
            b"override-ams",
        );
        let provider = OverlayResourceProvider::new(overrides, base);

        let package = provider
            .read_package("amsmath.sty")
            .expect("override package should resolve");
        let class = provider
            .read_tex_input("article.cls")
            .expect("base input should resolve");

        assert_eq!(package.bytes, b"override-ams");
        assert_eq!(class.bytes, b"base");
    }

    #[test]
    fn overlay_provider_preserves_denied_errors_from_overrides() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct DenyProvider;

        impl ResourceProvider for DenyProvider {
            fn read_request(&self, request: &ResourceRequest) -> Result<Resource, ResourceError> {
                Err(ResourceError::Denied {
                    name: request.canonical_name(),
                    message: "denied by policy".to_string(),
                })
            }
        }

        let base = InMemoryResourceProvider::new().with_resource(
            "plain.tex",
            ResourceKind::TexInput,
            b"base",
        );
        let provider = OverlayResourceProvider::new(DenyProvider, base);

        let error = provider
            .read_tex_input("plain.tex")
            .expect_err("override denial must not fall through");

        assert_eq!(
            error,
            ResourceError::Denied {
                name: "plain.tex".to_string(),
                message: "denied by policy".to_string(),
            }
        );
    }

    #[test]
    fn resource_bundle_resolves_package_owned_assets() {
        let bundle = ResourceBundle::new("chemistry")
            .with_request(ResourceRequest::asset("mhchem", "arrows.dat"), b"asset");

        let asset = bundle
            .read_asset("mhchem", "arrows.dat")
            .expect("bundle asset should resolve");

        assert_eq!(bundle.id(), "chemistry");
        assert_eq!(asset.canonical_name, "mhchem/arrows.dat");
        assert_eq!(asset.bytes, b"asset");
    }

    #[test]
    fn resolver_provider_delegates_typed_requests_to_host_resolver() {
        let provider = ResolverResourceProvider::new(|request: &ResourceRequest| {
            if request.kind == ResourceKind::Package && request.name == "amsmath.sty" {
                Ok(Resource::from_request(request, b"package"))
            } else if request.kind == ResourceKind::Asset
                && request.package.as_deref() == Some("mhchem")
                && request.name == "arrows.dat"
            {
                Ok(Resource::from_request(request, b"asset"))
            } else {
                Err(ResourceError::NotFound {
                    name: request.canonical_name(),
                    kind: request.kind,
                })
            }
        });

        let package = provider
            .read_package("amsmath.sty")
            .expect("package should resolve through resolver");
        let asset = provider
            .read_asset("mhchem", "arrows.dat")
            .expect("asset should resolve through resolver");
        let error = provider
            .read_tex_input("missing.tex")
            .expect_err("missing input should propagate resolver error");

        assert_eq!(package.canonical_name, "amsmath.sty");
        assert_eq!(package.bytes, b"package");
        assert_eq!(asset.canonical_name, "mhchem/arrows.dat");
        assert_eq!(asset.bytes, b"asset");
        assert_eq!(
            error,
            ResourceError::NotFound {
                name: "missing.tex".to_string(),
                kind: ResourceKind::TexInput,
            }
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn filesystem_provider_loads_relative_resource() {
        let root =
            std::env::temp_dir().join(format!("mathtex-resource-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create test root");
        let path = root.join("plain.tex");
        std::fs::write(&path, b"\\relax").expect("write resource");

        let provider = FileSystemResourceProvider::new(&root);
        let resource = provider
            .read("plain.tex", ResourceKind::TexInput)
            .expect("relative resource should load");

        assert_eq!(resource.kind, ResourceKind::TexInput);
        assert_eq!(resource.bytes, b"\\relax");

        std::fs::remove_file(path).expect("remove resource");
        std::fs::remove_dir(root).expect("remove test root");
    }

    #[cfg(feature = "std")]
    #[test]
    fn filesystem_provider_rejects_parent_directory_escape() {
        let provider = FileSystemResourceProvider::new(std::env::temp_dir());
        let error = provider
            .read("../plain.tex", ResourceKind::TexInput)
            .expect_err("parent path must be denied");

        assert_eq!(
            error,
            ResourceError::Denied {
                name: "../plain.tex".to_string(),
                message: "resource path must be relative to the provider root".to_string(),
            }
        );
    }
}
