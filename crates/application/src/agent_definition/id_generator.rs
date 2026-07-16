use crate::agent_definition::ports::AgentDefinitionIdGenerator;
use ora_domain::AgentDefinitionId;
use uuid::Uuid;

/// Generates UUID-backed configurable-agent identifiers.
#[derive(Clone, Copy, Debug, Default)]
pub struct UuidAgentDefinitionIdGenerator;

impl UuidAgentDefinitionIdGenerator {
    /// Builds the UUID-backed configurable-agent identifier generator.
    pub fn new() -> Self {
        Self
    }
}

impl AgentDefinitionIdGenerator for UuidAgentDefinitionIdGenerator {
    fn generate_agent_definition_id(&self) -> AgentDefinitionId {
        AgentDefinitionId::new(Uuid::new_v4().to_string())
    }
}
