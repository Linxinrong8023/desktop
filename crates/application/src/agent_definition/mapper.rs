use ora_contracts::Agent as ContractAgent;
use ora_domain::AgentDefinition;

/// Projects a domain configurable agent type into its audit-free public contract form.
pub(crate) fn map_agent_definition(agent_definition: AgentDefinition) -> ContractAgent {
    ContractAgent {
        id: agent_definition.id.to_string(),
        name: agent_definition.name,
        description: agent_definition.description,
    }
}
