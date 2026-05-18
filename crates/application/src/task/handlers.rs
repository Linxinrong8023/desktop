use crate::task::mapper::map_task;
use crate::task::ports::{
    CreateTaskWorktreeRequest, DeleteTaskWorktreeRequest, TaskIdGenerator, TaskRepository,
    TaskWorktreeDeletionMode, TaskWorktreeProvisioner,
};
use crate::worktree::{WorktreeIdGenerator, WorktreeRepository};
use crate::{ApplicationError, Clock};
use ora_contracts::{
    CreateTaskRequest, CreateTaskResponse, DeleteTaskRequest, DeleteTaskResponse, GetTaskRequest,
    GetTaskResponse, ListTasksRequest, ListTasksResponse, TaskStatus, UpdateTaskRequest,
    UpdateTaskResponse,
};
use ora_domain::{
    AuditFields, ProjectId, Task as DomainTask, TaskId, TaskStatus as DomainTaskStatus,
    Worktree as DomainWorktree, WorktreeActivity as DomainWorktreeActivity, WorktreeId,
};
use ora_logging::{ora_error, ora_info};
use std::path::{Path, PathBuf};

/// Handles task creation without depending on transport-specific concerns.
pub struct CreateTaskHandler<
    TaskRepositoryPort,
    WorktreeRepositoryPort,
    TaskIdGeneratorPort,
    WorktreeIdGeneratorPort,
    WorktreeProvisioner,
    ClockSource,
> {
    task_repository: TaskRepositoryPort,
    worktree_repository: WorktreeRepositoryPort,
    task_id_generator: TaskIdGeneratorPort,
    worktree_id_generator: WorktreeIdGeneratorPort,
    worktree_provisioner: WorktreeProvisioner,
    work_dir: PathBuf,
    clock: ClockSource,
}

impl<
    TaskRepositoryPort,
    WorktreeRepositoryPort,
    TaskIdGeneratorPort,
    WorktreeIdGeneratorPort,
    WorktreeProvisioner,
    ClockSource,
>
    CreateTaskHandler<
        TaskRepositoryPort,
        WorktreeRepositoryPort,
        TaskIdGeneratorPort,
        WorktreeIdGeneratorPort,
        WorktreeProvisioner,
        ClockSource,
    >
{
    pub fn new(
        task_repository: TaskRepositoryPort,
        worktree_repository: WorktreeRepositoryPort,
        task_id_generator: TaskIdGeneratorPort,
        worktree_id_generator: WorktreeIdGeneratorPort,
        worktree_provisioner: WorktreeProvisioner,
        work_dir: PathBuf,
        clock: ClockSource,
    ) -> Self {
        Self {
            task_repository,
            worktree_repository,
            task_id_generator,
            worktree_id_generator,
            worktree_provisioner,
            work_dir,
            clock,
        }
    }
}

impl<
    TaskRepositoryPort,
    WorktreeRepositoryPort,
    TaskIdGeneratorPort,
    WorktreeIdGeneratorPort,
    WorktreeProvisioner,
    ClockSource,
>
    CreateTaskHandler<
        TaskRepositoryPort,
        WorktreeRepositoryPort,
        TaskIdGeneratorPort,
        WorktreeIdGeneratorPort,
        WorktreeProvisioner,
        ClockSource,
    >
where
    TaskRepositoryPort: TaskRepository,
    WorktreeRepositoryPort: WorktreeRepository,
    TaskIdGeneratorPort: TaskIdGenerator,
    WorktreeIdGeneratorPort: WorktreeIdGenerator,
    WorktreeProvisioner: TaskWorktreeProvisioner,
    ClockSource: Clock,
{
    /// Creates a task together with its owned linked worktree and returns the public response payload.
    pub fn handle(
        &self,
        request: CreateTaskRequest,
    ) -> Result<CreateTaskResponse, ApplicationError> {
        let task_id = self.task_id_generator.generate_task_id();
        let branch_name = branch_name_for_task(&task_id);
        let worktree_path = worktree_path_for_task(&self.work_dir, &task_id);
        self.worktree_provisioner
            .create_task_worktree(CreateTaskWorktreeRequest {
                branch_name: branch_name.clone(),
                worktree_path: worktree_path.clone(),
            })
            .map_err(|error| {
                let error = ApplicationError::from_task_worktree_provisioner_error(error);
                log_task_failure("create_task", Some(&task_id), &error);
                error
            })?;

        let now = self.clock.now_timestamp_millis();
        let worktree_id = self.worktree_id_generator.generate_worktree_id();
        let worktree = DomainWorktree::new(
            worktree_id,
            task_id.clone(),
            Some(branch_name),
            DomainWorktreeActivity::Active,
            AuditFields::new(now, now, false),
        );
        let worktree = match self.worktree_repository.create_worktree(worktree) {
            Ok(worktree) => worktree,
            Err(error) => {
                return Err(self.handle_create_failure_after_provisioning(
                    "create_task",
                    &task_id,
                    &worktree_path,
                    ApplicationError::from_worktree_repository_error(error),
                ));
            }
        };
        let task = DomainTask::new(
            task_id.clone(),
            ProjectId::new(request.project_id),
            request.title,
            map_contract_task_status(request.status),
            Some(worktree.id.clone()),
            AuditFields::new(now, now, false),
        );
        let task = match self.task_repository.create_task(task) {
            Ok(task) => task,
            Err(error) => {
                return Err(self.handle_task_persistence_failure_after_worktree_create(
                    &task_id,
                    &worktree.id,
                    &worktree_path,
                    ApplicationError::from_task_repository_error(error),
                ));
            }
        };

        log_task_success("create_task", Some(&task.id));

        Ok(CreateTaskResponse {
            task: map_task(task),
        })
    }

    /// Attempts compensating worktree cleanup after persistence fails and returns the stable application error.
    fn handle_create_failure_after_provisioning(
        &self,
        operation: &'static str,
        task_id: &TaskId,
        worktree_path: &Path,
        original_error: ApplicationError,
    ) -> ApplicationError {
        let cleanup_result =
            self.worktree_provisioner
                .delete_task_worktree(DeleteTaskWorktreeRequest {
                    worktree_path: worktree_path.to_path_buf(),
                    mode: TaskWorktreeDeletionMode::Force,
                });

        match cleanup_result {
            Ok(()) => {
                log_task_failure(operation, Some(task_id), &original_error);
                original_error
            }
            Err(cleanup_error) => {
                let cleanup_error =
                    ApplicationError::from_task_worktree_provisioner_error(cleanup_error);
                log_task_failure(operation, Some(task_id), &cleanup_error);
                cleanup_error
            }
        }
    }

    /// Soft-deletes the persisted worktree row, then removes the created checkout, before returning a stable failure.
    fn handle_task_persistence_failure_after_worktree_create(
        &self,
        task_id: &TaskId,
        worktree_id: &WorktreeId,
        worktree_path: &Path,
        original_error: ApplicationError,
    ) -> ApplicationError {
        let worktree_cleanup = self
            .worktree_repository
            .soft_delete_worktree(worktree_id, self.clock.now_timestamp_millis())
            .map_err(ApplicationError::from_worktree_repository_error);
        let filesystem_cleanup = self.handle_create_failure_after_provisioning(
            "create_task",
            task_id,
            worktree_path,
            original_error,
        );

        match worktree_cleanup {
            Ok(_) => filesystem_cleanup,
            Err(error) => {
                log_task_failure("create_task", Some(task_id), &error);
                error
            }
        }
    }
}

/// Handles one task lookup without depending on transport-specific concerns.
pub struct GetTaskHandler<Repository> {
    repository: Repository,
}

impl<Repository> GetTaskHandler<Repository> {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }
}

impl<Repository> GetTaskHandler<Repository>
where
    Repository: TaskRepository,
{
    /// Loads one visible task or returns a stable not-found application error.
    pub fn handle(&self, request: GetTaskRequest) -> Result<GetTaskResponse, ApplicationError> {
        let task_id = TaskId::new(request.task_id);
        let task = self.repository.find_task(&task_id).map_err(|error| {
            let error = ApplicationError::from_task_repository_error(error);
            log_task_failure("get_task", Some(&task_id), &error);
            error
        })?;

        match task {
            Some(task) => {
                log_task_success("get_task", Some(&task_id));

                Ok(GetTaskResponse {
                    task: map_task(task),
                })
            }
            None => {
                let error = ApplicationError::TaskNotFound {
                    task_id: task_id.to_string(),
                };
                log_task_failure("get_task", Some(&task_id), &error);
                Err(error)
            }
        }
    }
}

/// Handles task listing without depending on transport-specific concerns.
pub struct ListTasksHandler<Repository> {
    repository: Repository,
}

impl<Repository> ListTasksHandler<Repository> {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }
}

impl<Repository> ListTasksHandler<Repository>
where
    Repository: TaskRepository,
{
    /// Lists every visible task and maps each one into the shared contract view.
    pub fn handle(
        &self,
        _request: ListTasksRequest,
    ) -> Result<ListTasksResponse, ApplicationError> {
        let tasks = self.repository.list_tasks().map_err(|error| {
            let error = ApplicationError::from_task_repository_error(error);
            log_task_failure("list_tasks", None, &error);
            error
        })?;

        ora_info!(
            message = "listed tasks",
            operation = "list_tasks",
            task_count = tasks.len()
        );

        Ok(ListTasksResponse {
            tasks: tasks.into_iter().map(map_task).collect(),
        })
    }
}

/// Handles task updates without depending on transport-specific concerns.
pub struct UpdateTaskHandler<Repository, ClockSource> {
    repository: Repository,
    clock: ClockSource,
}

impl<Repository, ClockSource> UpdateTaskHandler<Repository, ClockSource> {
    pub fn new(repository: Repository, clock: ClockSource) -> Self {
        Self { repository, clock }
    }
}

impl<Repository, ClockSource> UpdateTaskHandler<Repository, ClockSource>
where
    Repository: TaskRepository,
    ClockSource: Clock,
{
    /// Replaces the public task fields while preserving persistence-managed audit state.
    pub fn handle(
        &self,
        request: UpdateTaskRequest,
    ) -> Result<UpdateTaskResponse, ApplicationError> {
        let task_id = TaskId::new(request.task_id);
        let existing_task = self.repository.find_task(&task_id).map_err(|error| {
            let error = ApplicationError::from_task_repository_error(error);
            log_task_failure("update_task", Some(&task_id), &error);
            error
        })?;

        let existing_task = match existing_task {
            Some(existing_task) => existing_task,
            None => {
                let error = ApplicationError::TaskNotFound {
                    task_id: task_id.to_string(),
                };
                log_task_failure("update_task", Some(&task_id), &error);
                return Err(error);
            }
        };

        let task = DomainTask::new(
            task_id.clone(),
            ProjectId::new(request.project_id),
            request.title,
            map_contract_task_status(request.status),
            existing_task.worktree_id,
            AuditFields::new(
                existing_task.audit_fields.created_at,
                self.clock.now_timestamp_millis(),
                existing_task.audit_fields.is_deleted,
            ),
        );
        let task = self.repository.update_task(task).map_err(|error| {
            let error = ApplicationError::from_task_repository_error(error);
            log_task_failure("update_task", Some(&task_id), &error);
            error
        })?;

        log_task_success("update_task", Some(&task_id));

        Ok(UpdateTaskResponse {
            task: map_task(task),
        })
    }
}

/// Handles task deletion without exposing transport-specific cleanup details.
pub struct DeleteTaskHandler<
    TaskRepositoryPort,
    WorktreeRepositoryPort,
    WorktreeProvisioner,
    ClockSource,
> {
    task_repository: TaskRepositoryPort,
    worktree_repository: WorktreeRepositoryPort,
    worktree_provisioner: WorktreeProvisioner,
    work_dir: PathBuf,
    clock: ClockSource,
}

impl<TaskRepositoryPort, WorktreeRepositoryPort, WorktreeProvisioner, ClockSource>
    DeleteTaskHandler<TaskRepositoryPort, WorktreeRepositoryPort, WorktreeProvisioner, ClockSource>
{
    pub fn new(
        task_repository: TaskRepositoryPort,
        worktree_repository: WorktreeRepositoryPort,
        worktree_provisioner: WorktreeProvisioner,
        work_dir: PathBuf,
        clock: ClockSource,
    ) -> Self {
        Self {
            task_repository,
            worktree_repository,
            worktree_provisioner,
            work_dir,
            clock,
        }
    }
}

impl<TaskRepositoryPort, WorktreeRepositoryPort, WorktreeProvisioner, ClockSource>
    DeleteTaskHandler<TaskRepositoryPort, WorktreeRepositoryPort, WorktreeProvisioner, ClockSource>
where
    TaskRepositoryPort: TaskRepository,
    WorktreeRepositoryPort: WorktreeRepository,
    WorktreeProvisioner: TaskWorktreeProvisioner,
    ClockSource: Clock,
{
    /// Deletes one task and removes its owned linked worktree before returning the CRUD-shaped response.
    pub fn handle(
        &self,
        request: DeleteTaskRequest,
    ) -> Result<DeleteTaskResponse, ApplicationError> {
        let task_id = TaskId::new(request.task_id);
        let existing_task = self.task_repository.find_task(&task_id).map_err(|error| {
            let error = ApplicationError::from_task_repository_error(error);
            log_task_failure("delete_task", Some(&task_id), &error);
            error
        })?;
        let existing_task = match existing_task {
            Some(task) => task,
            None => {
                let error = ApplicationError::TaskNotFound {
                    task_id: task_id.to_string(),
                };
                log_task_failure("delete_task", Some(&task_id), &error);
                return Err(error);
            }
        };
        if let Some(worktree_id) = existing_task.worktree_id {
            let existing_worktree = self
                .worktree_repository
                .find_worktree(&worktree_id)
                .map_err(|error| {
                    let error = ApplicationError::from_worktree_repository_error(error);
                    log_task_failure("delete_task", Some(&task_id), &error);
                    error
                })?;
            let existing_worktree = match existing_worktree {
                Some(worktree) => worktree,
                None => {
                    let error = ApplicationError::WorktreeNotFound {
                        worktree_id: worktree_id.to_string(),
                    };
                    log_task_failure("delete_task", Some(&task_id), &error);
                    return Err(error);
                }
            };
            let worktree_path = worktree_path_for_task(&self.work_dir, &task_id);
            self.worktree_provisioner
                .delete_task_worktree(DeleteTaskWorktreeRequest {
                    worktree_path,
                    mode: TaskWorktreeDeletionMode::Force,
                })
                .map_err(|error| {
                    let error = ApplicationError::from_task_worktree_provisioner_error(error);
                    log_task_failure("delete_task", Some(&task_id), &error);
                    error
                })?;
            self.worktree_repository
                .soft_delete_worktree(&existing_worktree.id, self.clock.now_timestamp_millis())
                .map_err(|error| {
                    let error = ApplicationError::from_worktree_repository_error(error);
                    log_task_failure("delete_task", Some(&task_id), &error);
                    error
                })?;
        }

        let deleted = self
            .task_repository
            .soft_delete_task(&task_id, self.clock.now_timestamp_millis())
            .map_err(|error| {
                let error = ApplicationError::from_task_repository_error(error);
                log_task_failure("delete_task", Some(&task_id), &error);
                error
            })?;

        if deleted {
            log_task_success("delete_task", Some(&task_id));

            Ok(DeleteTaskResponse {
                task_id: task_id.to_string(),
            })
        } else {
            let error = ApplicationError::TaskNotFound {
                task_id: task_id.to_string(),
            };
            log_task_failure("delete_task", Some(&task_id), &error);
            Err(error)
        }
    }
}

/// Emits the shared informational event shape for successful task CRUD operations.
fn log_task_success(operation: &'static str, task_id: Option<&TaskId>) {
    match task_id {
        Some(task_id) => {
            ora_info!(
                message = "task operation completed",
                operation,
                task_id = task_id.to_string()
            );
        }
        None => {
            ora_info!(message = "task operation completed", operation);
        }
    }
}

/// Emits the shared error event shape for failed task CRUD operations.
fn log_task_failure(operation: &'static str, task_id: Option<&TaskId>, error: &ApplicationError) {
    match (task_id, error) {
        (Some(task_id), ApplicationError::TaskNotFound { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                task_id = task_id.to_string(),
                error.kind = "task_not_found",
                error.message = error.to_string()
            );
        }
        (Some(task_id), ApplicationError::TaskRepository { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                task_id = task_id.to_string(),
                error.kind = "task_repository",
                error.message = error.to_string()
            );
        }
        (Some(task_id), ApplicationError::TaskWorktree { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                task_id = task_id.to_string(),
                error.kind = "task_worktree",
                error.message = error.to_string()
            );
        }
        (Some(task_id), ApplicationError::WorktreeNotFound { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                task_id = task_id.to_string(),
                error.kind = "worktree_not_found",
                error.message = error.to_string()
            );
        }
        (Some(task_id), ApplicationError::WorktreeRepository { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                task_id = task_id.to_string(),
                error.kind = "worktree_repository",
                error.message = error.to_string()
            );
        }
        (None, ApplicationError::TaskRepository { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                error.kind = "task_repository",
                error.message = error.to_string()
            );
        }
        (None, ApplicationError::TaskNotFound { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                error.kind = "task_not_found",
                error.message = error.to_string()
            );
        }
        (None, ApplicationError::TaskWorktree { .. }) => {
            ora_error!(
                message = "task operation failed",
                operation,
                error.kind = "task_worktree",
                error.message = error.to_string()
            );
        }
        _ => {}
    }
}

/// Translates the transport-facing task status into the domain enum.
fn map_contract_task_status(status: TaskStatus) -> DomainTaskStatus {
    match status {
        TaskStatus::Todo => DomainTaskStatus::Todo,
        TaskStatus::Doing => DomainTaskStatus::Doing,
        TaskStatus::Done => DomainTaskStatus::Done,
    }
}

/// Derives the stable task branch name from the first eight characters of the generated task id.
fn branch_name_for_task(task_id: &TaskId) -> String {
    format!(
        "ora/{}",
        task_id.to_string().chars().take(8).collect::<String>()
    )
}

/// Derives the owned linked-worktree path from the configured worktree root and full task id.
fn worktree_path_for_task(work_dir: &Path, task_id: &TaskId) -> PathBuf {
    work_dir.join(task_id.to_string())
}
