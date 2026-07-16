use crate::bootstrap::SystemClock;
use ora_application::{
    ApplicationError, CreateSkillHandler, DeleteSkillHandler, GetSkillHandler, ListSkillsHandler,
    UpdateSkillHandler, UuidSkillIdGenerator,
};
use ora_contracts::{
    CreateSkillRequest, CreateSkillResponse, DeleteSkillRequest, DeleteSkillResponse,
    GetSkillRequest, GetSkillResponse, ListSkillsRequest, ListSkillsResponse, UpdateSkillRequest,
    UpdateSkillResponse,
};
use ora_db::{RepositoryPool, SqliteSkillRepository};

/// Groups HTTP-facing use cases for the reusable skill catalog.
pub struct SkillApi {
    create_skill: CreateSkillHandler<SqliteSkillRepository, UuidSkillIdGenerator, SystemClock>,
    get_skill: GetSkillHandler<SqliteSkillRepository>,
    list_skills: ListSkillsHandler<SqliteSkillRepository>,
    update_skill: UpdateSkillHandler<SqliteSkillRepository, SystemClock>,
    delete_skill: DeleteSkillHandler<SqliteSkillRepository, SystemClock>,
}

impl SkillApi {
    /// Builds the skill API from shared SQLite infrastructure.
    pub fn new(pool: RepositoryPool, clock: SystemClock) -> Self {
        let repository = SqliteSkillRepository::new(pool);

        Self {
            create_skill: CreateSkillHandler::new(
                repository.clone(),
                UuidSkillIdGenerator::new(),
                clock,
            ),
            get_skill: GetSkillHandler::new(repository.clone()),
            list_skills: ListSkillsHandler::new(repository.clone()),
            update_skill: UpdateSkillHandler::new(repository.clone(), clock),
            delete_skill: DeleteSkillHandler::new(repository, clock),
        }
    }

    /// Delegates skill creation to the application layer.
    pub fn create_skill(
        &self,
        request: CreateSkillRequest,
    ) -> Result<CreateSkillResponse, ApplicationError> {
        self.create_skill.handle(request)
    }

    /// Delegates one skill lookup to the application layer.
    pub fn get_skill(
        &self,
        request: GetSkillRequest,
    ) -> Result<GetSkillResponse, ApplicationError> {
        self.get_skill.handle(request)
    }

    /// Delegates skill listing to the application layer.
    pub fn list_skills(
        &self,
        request: ListSkillsRequest,
    ) -> Result<ListSkillsResponse, ApplicationError> {
        self.list_skills.handle(request)
    }

    /// Delegates skill replacement to the application layer.
    pub fn update_skill(
        &self,
        request: UpdateSkillRequest,
    ) -> Result<UpdateSkillResponse, ApplicationError> {
        self.update_skill.handle(request)
    }

    /// Delegates skill deletion to the application layer.
    pub fn delete_skill(
        &self,
        request: DeleteSkillRequest,
    ) -> Result<DeleteSkillResponse, ApplicationError> {
        self.delete_skill.handle(request)
    }
}
