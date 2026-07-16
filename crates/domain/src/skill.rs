use crate::{AuditFields, DomainModelError, SkillId};
use serde::{Deserialize, Serialize};

/// Represents one reusable skill definition available to configurable agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub audit_fields: AuditFields,
}

impl Skill {
    /// Creates a skill while normalizing its user-facing name for stable lookup.
    pub fn new(
        id: SkillId,
        name: impl Into<String>,
        description: impl Into<String>,
        audit_fields: AuditFields,
    ) -> Result<Self, DomainModelError> {
        let name = name.into().trim().to_string();

        if name.is_empty() {
            return Err(DomainModelError::EmptySkillName);
        }

        Ok(Self {
            id,
            name,
            description: description.into(),
            audit_fields,
        })
    }
}
