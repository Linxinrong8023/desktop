use crate::{
    ApplicationError, Clock, CreateProjectHandler, GetProjectHandler, ListProjectsHandler,
    ProjectIdGenerator, ProjectRepository, RepositoryError, UpdateProjectHandler,
};
use ora_contracts::{
    CreateProjectRequest, CreateProjectResponse, GetProjectRequest, GetProjectResponse,
    ListProjectsRequest, ListProjectsResponse, Project as ContractProject, UpdateProjectRequest,
    UpdateProjectResponse,
};
use ora_domain::{AuditFields, Project, ProjectId};
use ora_logging::{with_recorded_trace_logging, with_trace_logging};
use pretty_assertions::assert_eq;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Verifies create handlers build domain projects and return the shared contract response.
#[test]
fn creates_projects_with_generated_identity_and_clock_values() {
    with_trace_logging(|| {
        let repository = Rc::new(FakeProjectRepository::default());
        let handler = CreateProjectHandler::new(
            repository.clone(),
            FixedProjectIdGenerator::new("project-1"),
            FixedClock::new(1_700_000_000_000),
        );

        let response = match handler.handle(CreateProjectRequest {
            name: "Ora".to_string(),
            root_path: "/workspace/ora".to_string(),
        }) {
            Ok(response) => response,
            Err(error) => panic!("create handler failed: {error}"),
        };

        assert_eq!(
            response,
            CreateProjectResponse {
                project: ContractProject {
                    id: "project-1".to_string(),
                    name: "Ora".to_string(),
                    root_path: "/workspace/ora".to_string(),
                },
            }
        );
        assert_eq!(
            repository.visible_projects(),
            vec![Project::new(
                ProjectId::new("project-1"),
                "Ora",
                "/workspace/ora",
                AuditFields::new(1_700_000_000_000, 1_700_000_000_000, false),
            )]
        );
    });
}

/// Verifies get handlers return the shared contract projection for existing projects.
#[test]
fn gets_projects_by_identifier() {
    with_trace_logging(|| {
        let repository = Rc::new(FakeProjectRepository::with_projects(vec![Project::new(
            ProjectId::new("project-1"),
            "Ora",
            "/workspace/ora",
            AuditFields::new(1, 2, false),
        )]));
        let handler = GetProjectHandler::new(repository);

        let response = match handler.handle(GetProjectRequest {
            project_id: "project-1".to_string(),
        }) {
            Ok(response) => response,
            Err(error) => panic!("get handler failed: {error}"),
        };

        assert_eq!(
            response,
            GetProjectResponse {
                project: ContractProject {
                    id: "project-1".to_string(),
                    name: "Ora".to_string(),
                    root_path: "/workspace/ora".to_string(),
                },
            }
        );
    });
}

/// Verifies list handlers map every stored project into the shared contract payload.
#[test]
fn lists_visible_projects() {
    with_trace_logging(|| {
        let repository = Rc::new(FakeProjectRepository::with_projects(vec![
            Project::new(
                ProjectId::new("project-1"),
                "Ora",
                "/workspace/ora",
                AuditFields::new(1, 2, false),
            ),
            Project::new(
                ProjectId::new("project-2"),
                "Ora Docs",
                "/workspace/ora-docs",
                AuditFields::new(3, 4, false),
            ),
        ]));
        let handler = ListProjectsHandler::new(repository);

        let response = match handler.handle(ListProjectsRequest {}) {
            Ok(response) => response,
            Err(error) => panic!("list handler failed: {error}"),
        };

        assert_eq!(
            response,
            ListProjectsResponse {
                projects: vec![
                    ContractProject {
                        id: "project-1".to_string(),
                        name: "Ora".to_string(),
                        root_path: "/workspace/ora".to_string(),
                    },
                    ContractProject {
                        id: "project-2".to_string(),
                        name: "Ora Docs".to_string(),
                        root_path: "/workspace/ora-docs".to_string(),
                    },
                ],
            }
        );
    });
}

/// Verifies update handlers preserve created timestamps while refreshing mutable fields.
#[test]
fn updates_projects_with_refreshed_timestamps() {
    with_trace_logging(|| {
        let repository = Rc::new(FakeProjectRepository::with_projects(vec![Project::new(
            ProjectId::new("project-1"),
            "Ora",
            "/workspace/ora",
            AuditFields::new(10, 20, false),
        )]));
        let handler = UpdateProjectHandler::new(repository.clone(), FixedClock::new(30));

        let response = match handler.handle(UpdateProjectRequest {
            project_id: "project-1".to_string(),
            name: "Ora Updated".to_string(),
        }) {
            Ok(response) => response,
            Err(error) => panic!("update handler failed: {error}"),
        };

        assert_eq!(
            response,
            UpdateProjectResponse {
                project: ContractProject {
                    id: "project-1".to_string(),
                    name: "Ora Updated".to_string(),
                    root_path: "/workspace/ora".to_string(),
                },
            }
        );
        assert_eq!(
            repository.visible_projects(),
            vec![Project::new(
                ProjectId::new("project-1"),
                "Ora Updated",
                "/workspace/ora",
                AuditFields::new(10, 30, false),
            )]
        );
    });
}

/// Verifies handlers expose stable application errors for missing projects and repository failures.
#[test]
fn reports_application_errors() {
    with_trace_logging(|| {
        let missing_repository = Rc::new(FakeProjectRepository::default());
        let get_handler = GetProjectHandler::new(missing_repository);
        let failing_repository = Rc::new(FakeProjectRepository::default());
        failing_repository.fail_next(RepositoryError::from_message(
            "storage unavailable".to_string(),
        ));
        let list_handler = ListProjectsHandler::new(failing_repository);

        let missing_error = match get_handler.handle(GetProjectRequest {
            project_id: "missing".to_string(),
        }) {
            Ok(response) => panic!("expected missing error, got response: {response:?}"),
            Err(error) => error,
        };
        let repository_error = match list_handler.handle(ListProjectsRequest {}) {
            Ok(response) => panic!("expected repository error, got response: {response:?}"),
            Err(error) => error,
        };

        assert_eq!(
            missing_error,
            ApplicationError::ProjectNotFound {
                project_id: "missing".to_string(),
            }
        );
        assert_eq!(
            repository_error,
            ApplicationError::ProjectRepository {
                source: RepositoryError::from_message("storage unavailable"),
            }
        );
    });
}

/// Verifies project handlers emit structured success and failure events under a scoped subscriber.
#[test]
fn emits_structured_operational_events() {
    let recorder = EventRecorder::default();
    with_recorded_trace_logging(recorder.layer(), || {
        let create_repository = Rc::new(FakeProjectRepository::default());
        let create_handler = CreateProjectHandler::new(
            create_repository,
            FixedProjectIdGenerator::new("project-42"),
            FixedClock::new(5),
        );
        let get_handler = GetProjectHandler::new(Rc::new(FakeProjectRepository::default()));

        create_handler
            .handle(CreateProjectRequest {
                name: "Ora".to_string(),
                root_path: "/workspace/ora".to_string(),
            })
            .unwrap();
        assert_eq!(
            get_handler
                .handle(GetProjectRequest {
                    project_id: "missing".to_string(),
                })
                .unwrap_err(),
            ApplicationError::ProjectNotFound {
                project_id: "missing".to_string(),
            }
        );
    });

    assert_eq!(
        recorder.events(),
        vec![
            LoggedEvent {
                level: "INFO".to_string(),
                target: "ora_application::project::handlers".to_string(),
                fields: BTreeMap::from([
                    (
                        "message".to_string(),
                        "project operation completed".to_string()
                    ),
                    ("method".to_string(), "log_project_success".to_string()),
                    ("operation".to_string(), "create_project".to_string()),
                    ("project_id".to_string(), "project-42".to_string()),
                ]),
            },
            LoggedEvent {
                level: "ERROR".to_string(),
                target: "ora_application::project::handlers".to_string(),
                fields: BTreeMap::from([
                    ("error.kind".to_string(), "project_not_found".to_string()),
                    (
                        "error.message".to_string(),
                        "project not found: missing".to_string(),
                    ),
                    (
                        "message".to_string(),
                        "project operation failed".to_string()
                    ),
                    ("method".to_string(), "log_project_failure".to_string()),
                    ("operation".to_string(), "get_project".to_string()),
                    ("project_id".to_string(), "missing".to_string()),
                ]),
            },
        ]
    );
}

#[derive(Debug, Default)]
struct FakeProjectRepository {
    projects: RefCell<Vec<Project>>,
    next_error: RefCell<Option<RepositoryError>>,
}

impl FakeProjectRepository {
    fn with_projects(projects: Vec<Project>) -> Self {
        Self {
            projects: RefCell::new(projects),
            next_error: RefCell::new(None),
        }
    }

    /// Configures the next repository call to fail with a deterministic error.
    fn fail_next(&self, error: RepositoryError) {
        self.next_error.replace(Some(error));
    }

    /// Returns every non-deleted project so tests can assert visible repository state.
    fn visible_projects(&self) -> Vec<Project> {
        self.projects
            .borrow()
            .iter()
            .filter(|project| !project.audit_fields.is_deleted)
            .cloned()
            .collect()
    }

    /// Returns a queued error when a test wants to simulate repository failure.
    fn take_error(&self) -> Result<(), RepositoryError> {
        match self.next_error.borrow_mut().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl ProjectRepository for Rc<FakeProjectRepository> {
    fn create_project(&self, project: Project) -> Result<Project, RepositoryError> {
        self.take_error()?;

        self.projects.borrow_mut().push(project.clone());
        Ok(project)
    }

    fn find_project(&self, project_id: &ProjectId) -> Result<Option<Project>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .projects
            .borrow()
            .iter()
            .find(|project| project.id == *project_id && !project.audit_fields.is_deleted)
            .cloned())
    }

    fn find_project_by_name(&self, project_name: &str) -> Result<Option<Project>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .projects
            .borrow()
            .iter()
            .find(|project| project.name == project_name && !project.audit_fields.is_deleted)
            .cloned())
    }

    fn list_projects(&self) -> Result<Vec<Project>, RepositoryError> {
        self.take_error()?;

        Ok(self.visible_projects())
    }

    fn update_project(&self, project: Project) -> Result<Project, RepositoryError> {
        self.take_error()?;

        let mut projects = self.projects.borrow_mut();
        if let Some(existing_project) = projects.iter_mut().find(|existing_project| {
            existing_project.id == project.id && !existing_project.audit_fields.is_deleted
        }) {
            *existing_project = project.clone();
            Ok(project)
        } else {
            Err(RepositoryError::from_message(format!(
                "missing project during update: {}",
                project.id
            )))
        }
    }

    fn soft_delete_project(
        &self,
        project_id: &ProjectId,
        deleted_at: i64,
    ) -> Result<bool, RepositoryError> {
        self.take_error()?;

        let mut projects = self.projects.borrow_mut();
        if let Some(project) = projects
            .iter_mut()
            .find(|project| project.id == *project_id && !project.audit_fields.is_deleted)
        {
            project.audit_fields.updated_at = deleted_at;
            project.audit_fields.is_deleted = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

struct FixedProjectIdGenerator {
    project_id: ProjectId,
}

impl FixedProjectIdGenerator {
    fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: ProjectId::new(project_id),
        }
    }
}

impl ProjectIdGenerator for FixedProjectIdGenerator {
    fn generate_project_id(&self) -> ProjectId {
        self.project_id.clone()
    }
}

struct FixedClock {
    timestamp_millis: i64,
}

impl FixedClock {
    fn new(timestamp_millis: i64) -> Self {
        Self { timestamp_millis }
    }
}

impl Clock for FixedClock {
    fn now_timestamp_millis(&self) -> i64 {
        self.timestamp_millis
    }
}

/// Captures one emitted event in a comparison-friendly structure for logging assertions.
#[derive(Clone, Debug, Eq, PartialEq)]
struct LoggedEvent {
    level: String,
    target: String,
    fields: BTreeMap<String, String>,
}

/// Records tracing events into shared memory so tests can assert full structured outcomes.
#[derive(Clone, Debug, Default)]
struct EventRecorder {
    events: Arc<Mutex<Vec<LoggedEvent>>>,
}

impl EventRecorder {
    /// Builds the recording layer attached to one scoped test subscriber.
    fn layer(&self) -> RecordingLayer {
        RecordingLayer {
            events: self.events.clone(),
        }
    }

    /// Returns every captured event in emission order.
    fn events(&self) -> Vec<LoggedEvent> {
        self.events.lock().unwrap().clone()
    }
}

/// Pushes each tracing event into the shared recorder without relying on global subscriber state.
#[derive(Clone, Debug)]
struct RecordingLayer {
    events: Arc<Mutex<Vec<LoggedEvent>>>,
}

impl<S> Layer<S> for RecordingLayer
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    /// Converts each event into a stable, fully comparable structure for test assertions.
    fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, S>) {
        let mut visitor = EventFieldVisitor::default();
        event.record(&mut visitor);
        self.events.lock().unwrap().push(LoggedEvent {
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_string(),
            fields: visitor.fields,
        });
    }
}

/// Records tracing fields as strings because these tests care about semantic content, not JSON formatting.
#[derive(Debug, Default)]
struct EventFieldVisitor {
    fields: BTreeMap<String, String>,
}

impl tracing::field::Visit for EventFieldVisitor {
    /// Preserves string fields exactly as handler logs emitted them.
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    /// Preserves signed integers in decimal form for stable assertions.
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    /// Preserves unsigned integers in decimal form for stable assertions.
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    /// Falls back to debug formatting for field types without a more specific visitor hook.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            format!("{value:?}").trim_matches('"').to_string(),
        );
    }
}
