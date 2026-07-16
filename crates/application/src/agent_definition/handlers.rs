use crate::agent_definition::mapper::map_agent_definition;
use crate::agent_definition::ports::{AgentDefinitionIdGenerator, AgentDefinitionRepository};
use crate::{ApplicationError, Clock};
use ora_contracts::{
    CreateAgentRequest, CreateAgentResponse, DeleteAgentRequest, DeleteAgentResponse,
    GetAgentRequest, GetAgentResponse, ListAgentsRequest, ListAgentsResponse, UpdateAgentRequest,
    UpdateAgentResponse,
};
use ora_domain::{AgentDefinition, AgentDefinitionId, AuditFields};

/// Handles creation of configurable agent types.
pub struct CreateAgentDefinitionHandler<Repository, IdGenerator, ClockSource> {
    repository: Repository,
    id_generator: IdGenerator,
    clock: ClockSource,
}

impl<Repository, IdGenerator, ClockSource>
    CreateAgentDefinitionHandler<Repository, IdGenerator, ClockSource>
{
    pub fn new(repository: Repository, id_generator: IdGenerator, clock: ClockSource) -> Self {
        Self {
            repository,
            id_generator,
            clock,
        }
    }
}

impl<Repository, IdGenerator, ClockSource>
    CreateAgentDefinitionHandler<Repository, IdGenerator, ClockSource>
where
    Repository: AgentDefinitionRepository,
    IdGenerator: AgentDefinitionIdGenerator,
    ClockSource: Clock,
{
    /// Creates a normalized configurable agent type and returns its public projection.
    pub fn handle(
        &self,
        request: CreateAgentRequest,
    ) -> Result<CreateAgentResponse, ApplicationError> {
        let now = self.clock.now_timestamp_millis();
        let agent_definition = AgentDefinition::new(
            self.id_generator.generate_agent_definition_id(),
            request.name,
            request.description,
            AuditFields::new(now, now, false),
        )
        .map_err(ApplicationError::from_agent_definition_domain_error)?;
        let agent_definition = self
            .repository
            .create_agent_definition(agent_definition)
            .map_err(ApplicationError::from_agent_definition_repository_error)?;

        Ok(CreateAgentResponse {
            agent: map_agent_definition(agent_definition),
        })
    }
}

/// Handles lookup of configurable agent types.
pub struct GetAgentDefinitionHandler<Repository> {
    repository: Repository,
}

impl<Repository> GetAgentDefinitionHandler<Repository> {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }
}

impl<Repository> GetAgentDefinitionHandler<Repository>
where
    Repository: AgentDefinitionRepository,
{
    /// Loads one visible configurable agent type or reports not found.
    pub fn handle(&self, request: GetAgentRequest) -> Result<GetAgentResponse, ApplicationError> {
        let agent_id = AgentDefinitionId::new(request.agent_id);
        let agent_definition = self
            .repository
            .find_agent_definition(&agent_id)
            .map_err(ApplicationError::from_agent_definition_repository_error)?
            .ok_or_else(|| ApplicationError::AgentDefinitionNotFound {
                agent_id: agent_id.to_string(),
            })?;

        Ok(GetAgentResponse {
            agent: map_agent_definition(agent_definition),
        })
    }
}

/// Handles listing configurable agent types.
pub struct ListAgentDefinitionsHandler<Repository> {
    repository: Repository,
}

impl<Repository> ListAgentDefinitionsHandler<Repository> {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }
}

impl<Repository> ListAgentDefinitionsHandler<Repository>
where
    Repository: AgentDefinitionRepository,
{
    /// Lists every visible configurable agent type in deterministic order.
    pub fn handle(
        &self,
        _request: ListAgentsRequest,
    ) -> Result<ListAgentsResponse, ApplicationError> {
        let agents = self
            .repository
            .list_agent_definitions()
            .map_err(ApplicationError::from_agent_definition_repository_error)?;
        Ok(ListAgentsResponse {
            agents: agents.into_iter().map(map_agent_definition).collect(),
        })
    }
}

/// Handles replacement of configurable agent types.
pub struct UpdateAgentDefinitionHandler<Repository, ClockSource> {
    repository: Repository,
    clock: ClockSource,
}

impl<Repository, ClockSource> UpdateAgentDefinitionHandler<Repository, ClockSource> {
    pub fn new(repository: Repository, clock: ClockSource) -> Self {
        Self { repository, clock }
    }
}

impl<Repository, ClockSource> UpdateAgentDefinitionHandler<Repository, ClockSource>
where
    Repository: AgentDefinitionRepository,
    ClockSource: Clock,
{
    /// Replaces editable fields while preserving the agent identifier and creation timestamp.
    pub fn handle(
        &self,
        request: UpdateAgentRequest,
    ) -> Result<UpdateAgentResponse, ApplicationError> {
        let agent_id = AgentDefinitionId::new(request.agent_id);
        let existing = self
            .repository
            .find_agent_definition(&agent_id)
            .map_err(ApplicationError::from_agent_definition_repository_error)?
            .ok_or_else(|| ApplicationError::AgentDefinitionNotFound {
                agent_id: agent_id.to_string(),
            })?;
        let agent_definition = AgentDefinition::new(
            agent_id,
            request.name,
            request.description,
            AuditFields::new(
                existing.audit_fields.created_at,
                self.clock.now_timestamp_millis(),
                false,
            ),
        )
        .map_err(ApplicationError::from_agent_definition_domain_error)?;
        let agent_definition = self
            .repository
            .update_agent_definition(agent_definition)
            .map_err(ApplicationError::from_agent_definition_repository_error)?;

        Ok(UpdateAgentResponse {
            agent: map_agent_definition(agent_definition),
        })
    }
}

/// Handles soft deletion of configurable agent types.
pub struct DeleteAgentDefinitionHandler<Repository, ClockSource> {
    repository: Repository,
    clock: ClockSource,
}

impl<Repository, ClockSource> DeleteAgentDefinitionHandler<Repository, ClockSource> {
    pub fn new(repository: Repository, clock: ClockSource) -> Self {
        Self { repository, clock }
    }
}

impl<Repository, ClockSource> DeleteAgentDefinitionHandler<Repository, ClockSource>
where
    Repository: AgentDefinitionRepository,
    ClockSource: Clock,
{
    /// Soft-deletes one visible configurable agent type and returns its identifier.
    pub fn handle(
        &self,
        request: DeleteAgentRequest,
    ) -> Result<DeleteAgentResponse, ApplicationError> {
        let agent_id = AgentDefinitionId::new(request.agent_id);
        let deleted = self
            .repository
            .soft_delete_agent_definition(&agent_id, self.clock.now_timestamp_millis())
            .map_err(ApplicationError::from_agent_definition_repository_error)?;
        if !deleted {
            return Err(ApplicationError::AgentDefinitionNotFound {
                agent_id: agent_id.to_string(),
            });
        }

        Ok(DeleteAgentResponse {
            agent_id: agent_id.to_string(),
        })
    }
}
