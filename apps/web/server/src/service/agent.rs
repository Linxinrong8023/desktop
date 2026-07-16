use crate::bootstrap::SystemClock;
use ora_application::{
    ApplicationError, CreateAgentDefinitionHandler, DeleteAgentDefinitionHandler,
    GetAgentDefinitionHandler, ListAgentDefinitionsHandler, UpdateAgentDefinitionHandler,
    UuidAgentDefinitionIdGenerator,
};
use ora_contracts::{
    CreateAgentRequest, CreateAgentResponse, DeleteAgentRequest, DeleteAgentResponse,
    GetAgentRequest, GetAgentResponse, ListAgentsRequest, ListAgentsResponse, UpdateAgentRequest,
    UpdateAgentResponse,
};
use ora_db::{RepositoryPool, SqliteAgentDefinitionRepository};

/// Groups HTTP-facing use cases for configurable agent types.
pub struct AgentApi {
    create_agent: CreateAgentDefinitionHandler<
        SqliteAgentDefinitionRepository,
        UuidAgentDefinitionIdGenerator,
        SystemClock,
    >,
    get_agent: GetAgentDefinitionHandler<SqliteAgentDefinitionRepository>,
    list_agents: ListAgentDefinitionsHandler<SqliteAgentDefinitionRepository>,
    update_agent: UpdateAgentDefinitionHandler<SqliteAgentDefinitionRepository, SystemClock>,
    delete_agent: DeleteAgentDefinitionHandler<SqliteAgentDefinitionRepository, SystemClock>,
}

impl AgentApi {
    /// Builds the agent API from shared SQLite infrastructure.
    pub fn new(pool: RepositoryPool, clock: SystemClock) -> Self {
        let repository = SqliteAgentDefinitionRepository::new(pool);

        Self {
            create_agent: CreateAgentDefinitionHandler::new(
                repository.clone(),
                UuidAgentDefinitionIdGenerator::new(),
                clock,
            ),
            get_agent: GetAgentDefinitionHandler::new(repository.clone()),
            list_agents: ListAgentDefinitionsHandler::new(repository.clone()),
            update_agent: UpdateAgentDefinitionHandler::new(repository.clone(), clock),
            delete_agent: DeleteAgentDefinitionHandler::new(repository, clock),
        }
    }

    /// Delegates configurable-agent creation to the application layer.
    pub fn create_agent(
        &self,
        request: CreateAgentRequest,
    ) -> Result<CreateAgentResponse, ApplicationError> {
        self.create_agent.handle(request)
    }

    /// Delegates one configurable-agent lookup to the application layer.
    pub fn get_agent(
        &self,
        request: GetAgentRequest,
    ) -> Result<GetAgentResponse, ApplicationError> {
        self.get_agent.handle(request)
    }

    /// Delegates configurable-agent listing to the application layer.
    pub fn list_agents(
        &self,
        request: ListAgentsRequest,
    ) -> Result<ListAgentsResponse, ApplicationError> {
        self.list_agents.handle(request)
    }

    /// Delegates configurable-agent replacement to the application layer.
    pub fn update_agent(
        &self,
        request: UpdateAgentRequest,
    ) -> Result<UpdateAgentResponse, ApplicationError> {
        self.update_agent.handle(request)
    }

    /// Delegates configurable-agent deletion to the application layer.
    pub fn delete_agent(
        &self,
        request: DeleteAgentRequest,
    ) -> Result<DeleteAgentResponse, ApplicationError> {
        self.delete_agent.handle(request)
    }
}
