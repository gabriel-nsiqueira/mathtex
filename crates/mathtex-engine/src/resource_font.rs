use std::cell::RefCell;
use std::collections::BTreeMap;

use mathtex_font::{FontData, FontError, FontKey, FontLoader, FontSpec};

use crate::resource::{ResourceError, ResourceKind, ResourceProvider};

/// Loads font files through a [`ResourceProvider`], one key per file in load order.
#[derive(Debug)]
pub struct ResourceFontLoader<R> {
    resources: R,
    loaded_fonts: RefCell<BTreeMap<String, FontData>>,
}

impl<R> ResourceFontLoader<R> {
    /// Creates a loader backed by the given resource provider.
    #[must_use]
    pub fn new(resources: R) -> Self {
        Self {
            resources,
            loaded_fonts: RefCell::new(BTreeMap::new()),
        }
    }

    /// Returns a reference to the underlying resource provider.
    #[must_use]
    pub fn resources(&self) -> &R {
        &self.resources
    }

    /// Number of font files loaded so far.
    #[must_use]
    pub fn cached_font_count(&self) -> usize {
        self.loaded_fonts.borrow().len()
    }

    /// Returns the loaded font with the given key, so hosts can draw what the engine laid out.
    #[must_use]
    pub fn font(&self, key: FontKey) -> Option<FontData> {
        self.loaded_fonts
            .borrow()
            .values()
            .find(|font| font.key == key)
            .cloned()
    }
}

impl<R> FontLoader for ResourceFontLoader<R>
where
    R: ResourceProvider,
{
    fn load(&self, spec: &FontSpec) -> Result<FontData, FontError> {
        let mut last_error = None;
        for name in spec.file_candidates() {
            if let Some(font) = self.loaded_fonts.borrow().get(&name) {
                return Ok(font.clone());
            }
            match self.resources.read(&name, ResourceKind::Font) {
                Ok(resource) => {
                    let mut fonts = self.loaded_fonts.borrow_mut();
                    let key = FontKey(fonts.len() as u64 + 1);
                    let font = FontData::new(key, resource.bytes);
                    fonts.insert(name, font.clone());
                    return Ok(font);
                }
                Err(error) => last_error = Some(error),
            }
        }
        Err(match last_error {
            Some(ResourceError::NotFound { .. }) | None => FontError::NotFound {
                name: spec.name().into(),
            },
            Some(error) => FontError::Invalid {
                name: spec.name().into(),
                message: error.to_string(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryResourceProvider;
    use mathtex_ir::Length;

    #[test]
    fn resource_font_loader_caches_files_and_resolves_extensionless_names() {
        let resources = InMemoryResourceProvider::new()
            .with_resource("latinmodern-math.otf", ResourceKind::Font, b"math")
            .with_resource("lmroman10-regular.otf", ResourceKind::Font, b"text");
        let fonts = ResourceFontLoader::new(resources);
        let spec = |name: &str| FontSpec::parse(name, Length(10 * 65_536));

        let math = fonts
            .load(&spec("[latinmodern-math.otf]:script=math"))
            .expect("file spec");
        assert_eq!(&**math.bytes().expect("owned bytes"), b"math");
        let again = fonts
            .load(&spec("[latinmodern-math.otf]:script=math;+ssty=0"))
            .expect("cached file");
        assert_eq!(again, math);
        let text = fonts
            .load(&spec("lmroman10-regular:mapping=tex-text"))
            .expect("extensionless name");
        assert_ne!(text.key, math.key);
        assert_eq!(fonts.cached_font_count(), 2);
        assert_eq!(fonts.font(text.key), Some(text));

        assert_eq!(
            fonts.load(&spec("[missing.otf]")),
            Err(FontError::NotFound {
                name: "missing.otf".into()
            })
        );
    }
}
