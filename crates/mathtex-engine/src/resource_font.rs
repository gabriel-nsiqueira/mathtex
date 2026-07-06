use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::cell::RefCell;

use mathtex_font::{
    FontData, FontError, FontLoader, FontQuery, FontSystem, ShapeRequest, ShapedText,
};
use mathtex_ir::FontId;

use crate::resource::{ResourceError, ResourceProvider};

/// Loads font bytes through a [`ResourceProvider`], shaping is unsupported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceFontSystem<R> {
    resources: R,
    loaded_fonts: RefCell<BTreeMap<String, FontData>>,
}

impl<R> ResourceFontSystem<R> {
    /// Creates a font system backed by the given resource provider.
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

    /// Unwraps and returns the underlying resource provider.
    #[must_use]
    pub fn into_resources(self) -> R {
        self.resources
    }

    /// Number of font resources cached by resolver facing name.
    #[must_use]
    pub fn cached_font_count(&self) -> usize {
        self.loaded_fonts.borrow().len()
    }
}

impl<R> FontSystem for ResourceFontSystem<R>
where
    R: ResourceProvider,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        FontLoader::load_font(self, query)
    }

    fn shape_text(&self, _request: &ShapeRequest<'_>) -> Result<ShapedText, FontError> {
        Err(FontError::ShapingUnsupported {
            message: "resource font system loads bytes but does not shape text".to_string(),
        })
    }
}

impl<R> FontLoader for ResourceFontSystem<R>
where
    R: ResourceProvider,
{
    fn load_font(&self, query: &FontQuery) -> Result<FontData, FontError> {
        if let Some(font) = self
            .loaded_fonts
            .borrow()
            .get(query.family.as_str())
            .cloned()
        {
            return Ok(font);
        }

        let resource = self
            .resources
            .read_font(query.family.as_str())
            .map_err(|error| resource_error_to_font_error(query, error))?;

        let font = FontData::new(
            font_id_for_name(&resource.canonical_name),
            resource.canonical_name,
            resource.bytes,
        );
        self.loaded_fonts
            .borrow_mut()
            .insert(query.family.clone(), font.clone());
        Ok(font)
    }
}

fn resource_error_to_font_error(query: &FontQuery, error: ResourceError) -> FontError {
    match error {
        ResourceError::NotFound { .. } => FontError::NotFound {
            family: query.family.clone(),
        },
        ResourceError::Invalid { message, .. } | ResourceError::Denied { message, .. } => {
            FontError::Invalid {
                family: query.family.clone(),
                message,
            }
        }
    }
}

fn font_id_for_name(name: &str) -> FontId {
    let mut hash = 2_166_136_261u32;
    for byte in name.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    FontId(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InMemoryResourceProvider, ResourceKind};
    use mathtex_ir::Length;

    #[test]
    fn resource_font_system_loads_fonts_through_resource_provider() {
        let resources = InMemoryResourceProvider::new().with_resource(
            "Latin Modern Math.otf",
            ResourceKind::Font,
            b"font-bytes",
        );
        let fonts = ResourceFontSystem::new(resources);

        let font = FontSystem::load_font(
            &fonts,
            &FontQuery {
                family: "Latin Modern Math.otf".to_string(),
                size: Length::from_scaled_points(655_360),
                math: true,
            },
        )
        .expect("font should resolve through resource provider");

        assert_eq!(font.canonical_name, "Latin Modern Math.otf");
        assert_eq!(&**font.bytes().expect("library owned bytes"), b"font-bytes");
        assert_eq!(font.id, font_id_for_name("Latin Modern Math.otf"));
        assert_eq!(fonts.cached_font_count(), 1);

        let cached = FontSystem::load_font(
            &fonts,
            &FontQuery {
                family: "Latin Modern Math.otf".to_string(),
                size: Length::from_scaled_points(327_680),
                math: true,
            },
        )
        .expect("cached font should resolve without another cache entry");

        assert_eq!(cached, font);
        assert_eq!(fonts.cached_font_count(), 1);
    }

    #[test]
    fn resource_font_system_reports_missing_font_as_font_error() {
        let fonts = ResourceFontSystem::new(InMemoryResourceProvider::new());

        let error = FontSystem::load_font(
            &fonts,
            &FontQuery {
                family: "missing.otf".to_string(),
                size: Length::ZERO,
                math: false,
            },
        )
        .expect_err("missing resource should be a font error");

        assert_eq!(
            error,
            FontError::NotFound {
                family: "missing.otf".to_string(),
            }
        );
    }

    #[test]
    fn resource_font_system_keeps_shaping_explicitly_separate() {
        let fonts = ResourceFontSystem::new(InMemoryResourceProvider::new());

        let error = fonts
            .shape_text(&ShapeRequest {
                font: FontId(0),
                text: "x",
                direction: mathtex_ir::Direction::LeftToRight,
                source: None,
                script: None,
                features: Vec::new(),
            })
            .expect_err("resource adapter should not shape text");

        match error {
            FontError::ShapingUnsupported { message } => {
                assert!(message.contains("does not shape text"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
