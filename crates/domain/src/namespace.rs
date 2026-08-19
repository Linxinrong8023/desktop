use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

use crate::DomainModelError;

/// Namespaces catalog resources by their trusted owner, such as local storage or a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Namespace(String);

impl Namespace {
    /// Builds a normalized non-empty namespace identifier.
    pub fn new(value: impl Into<String>) -> Result<Self, DomainModelError> {
        // Plugin identifiers are compared case-insensitively in SQLite, so canonicalizing here
        // keeps in-memory equality aligned with the persisted identity rule.
        let value = value.into().trim().to_ascii_lowercase();
        if value.is_empty() {
            return Err(DomainModelError::EmptyNamespace);
        }

        Ok(Self(value))
    }

    /// Returns the namespace assigned to user-created and locally imported resources.
    pub fn local() -> Self {
        Self("local".to_string())
    }
}

impl TryFrom<String> for Namespace {
    type Error = DomainModelError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Display for Namespace {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
