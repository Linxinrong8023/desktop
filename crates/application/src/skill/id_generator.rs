use crate::skill::ports::SkillIdGenerator;
use ora_domain::SkillId;
use uuid::Uuid;

/// Generates UUID-backed skill identifiers.
#[derive(Clone, Copy, Debug, Default)]
pub struct UuidSkillIdGenerator;

impl UuidSkillIdGenerator {
    /// Builds the UUID-backed skill identifier generator.
    pub fn new() -> Self {
        Self
    }
}

impl SkillIdGenerator for UuidSkillIdGenerator {
    fn generate_skill_id(&self) -> SkillId {
        SkillId::new(Uuid::new_v4().to_string())
    }
}
