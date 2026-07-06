use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

use crate::profile::{PrimitiveKind, PrimitiveOpcode, PrimitiveSpec};

/// Maps primitive control sequence names to their dispatch opcodes and semantic kinds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrimitiveRegistry {
    entries: BTreeMap<String, PrimitiveEntry>,
}

impl PrimitiveRegistry {
    /// Builds a registry from a slice of specs, returning an error on the first duplicate name.
    pub fn from_specs(specs: &[PrimitiveSpec]) -> Result<Self, PrimitiveRegistryError> {
        let mut registry = Self::default();
        for spec in specs {
            registry.insert(spec)?;
        }
        Ok(registry)
    }

    /// Inserts a single primitive spec, returning an error if the name is already registered.
    pub fn insert(&mut self, spec: &PrimitiveSpec) -> Result<(), PrimitiveRegistryError> {
        let entry = PrimitiveEntry {
            name: spec.name.to_string(),
            opcode: spec.opcode,
            kind: spec.kind,
        };

        if self.entries.contains_key(&entry.name) {
            return Err(PrimitiveRegistryError::DuplicateName { name: entry.name });
        }

        self.entries.insert(entry.name.clone(), entry);
        Ok(())
    }

    /// Look up a primitive by control sequence name without leading backslash.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PrimitiveEntry> {
        self.entries.get(name)
    }

    /// Returns the number of registered primitives.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true when no primitives have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Resolved information for a single registered primitive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveEntry {
    /// Primitive control sequence name without leading backslash.
    pub name: String,
    /// Numeric dispatch key for translated/native engine code.
    pub opcode: PrimitiveOpcode,
    /// Semantic category of the primitive.
    pub kind: PrimitiveKind,
}

/// Error returned when building a `PrimitiveRegistry`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrimitiveRegistryError {
    /// Two primitives were registered under the same name.
    DuplicateName {
        /// The duplicated control sequence name.
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;

    use super::*;

    #[test]
    fn registry_rejects_duplicate_names() {
        let error = PrimitiveRegistry::from_specs(&[
            PrimitiveSpec {
                name: Cow::Borrowed("input"),
                opcode: PrimitiveOpcode(1),
                kind: PrimitiveKind::Resource,
            },
            PrimitiveSpec {
                name: Cow::Borrowed("input"),
                opcode: PrimitiveOpcode(2),
                kind: PrimitiveKind::Resource,
            },
        ])
        .expect_err("duplicate names should fail");

        assert_eq!(
            error,
            PrimitiveRegistryError::DuplicateName {
                name: "input".to_string(),
            }
        );
    }
}
