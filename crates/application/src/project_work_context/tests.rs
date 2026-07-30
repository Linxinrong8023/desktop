use crate::{
    ApplicationError, Clock, OpenProjectWorkContextHandler, ProjectRepository,
    ProjectWorkContextIdGenerator, ProjectWorkContextRepository, RenewProjectWorkContextHandler,
    RepositoryError,
};
use ora_contracts::{
    OpenProjectWorkContextRequest, OpenProjectWorkContextResponse,
    ProjectWorkContext as ContractProjectWorkContext,
    ProjectWorkContextSurface as ContractProjectWorkContextSurface, RenewProjectWorkContextRequest,
    RenewProjectWorkContextResponse,
};
use ora_domain::{
    AuditFields, Project, ProjectId, ProjectWorkContext, ProjectWorkContextId,
    ProjectWorkContextSurface,
};
use ora_logging::{with_recorded_trace_logging, with_trace_logging};
use pretty_assertions::assert_eq;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

/// Verifies open requests create or switch one window-scoped context using backend lease timing.
#[test]
fn opens_and_switches_project_work_contexts() {
    with_trace_logging(|| {
        let project_repository = Rc::new(FakeProjectRepository::with_projects(vec![
            Project::new(
                ProjectId::new("project-1"),
                "Ora",
                "/workspace/ora",
                AuditFields::new(1, 1, false),
            ),
            Project::new(
                ProjectId::new("project-2"),
                "Ora Docs",
                "/workspace/ora-docs",
                AuditFields::new(2, 2, false),
            ),
        ]));
        let context_repository = Rc::new(FakeProjectWorkContextRepository::default());
        let open_handler = OpenProjectWorkContextHandler::new(
            project_repository.clone(),
            context_repository.clone(),
            FixedProjectWorkContextIdGenerator::new("context-1"),
            FixedClock::new(10),
        );

        let created_response = open_handler
            .handle(OpenProjectWorkContextRequest {
                surface: ContractProjectWorkContextSurface::Tauri,
                window_id: "window-1".to_string(),
                project_id: "project-1".to_string(),
            })
            .unwrap_or_else(|error| panic!("open handler failed: {error}"));

        assert_eq!(
            created_response,
            OpenProjectWorkContextResponse {
                context: ContractProjectWorkContext {
                    id: "context-1".to_string(),
                    surface: ContractProjectWorkContextSurface::Tauri,
                    window_id: "window-1".to_string(),
                    project_id: "project-1".to_string(),
                    lease_expires_at: 120_010,
                },
            }
        );

        let switch_handler = OpenProjectWorkContextHandler::new(
            project_repository,
            context_repository.clone(),
            FixedProjectWorkContextIdGenerator::new("unused"),
            FixedClock::new(40),
        );
        let switched_response = switch_handler
            .handle(OpenProjectWorkContextRequest {
                surface: ContractProjectWorkContextSurface::Tauri,
                window_id: "window-1".to_string(),
                project_id: "project-2".to_string(),
            })
            .unwrap_or_else(|error| panic!("switch handler failed: {error}"));

        assert_eq!(
            switched_response,
            OpenProjectWorkContextResponse {
                context: ContractProjectWorkContext {
                    id: "context-1".to_string(),
                    surface: ContractProjectWorkContextSurface::Tauri,
                    window_id: "window-1".to_string(),
                    project_id: "project-2".to_string(),
                    lease_expires_at: 120_040,
                },
            }
        );
        assert_eq!(
            context_repository.contexts(),
            vec![ProjectWorkContext::new(
                ProjectWorkContextId::new("context-1"),
                ProjectWorkContextSurface::Tauri,
                "window-1",
                ProjectId::new("project-2"),
                120_040,
                10,
                40,
            )]
        );
    });
}

/// Verifies Tauri contexts reject occupied projects without exposing owner details in the error.
#[test]
fn rejects_conflicting_tauri_project_opens() {
    with_trace_logging(|| {
        let project_repository = Rc::new(FakeProjectRepository::with_projects(vec![Project::new(
            ProjectId::new("project-1"),
            "Ora",
            "/workspace/ora",
            AuditFields::new(1, 1, false),
        )]));
        let context_repository = Rc::new(FakeProjectWorkContextRepository::with_contexts(vec![
            ProjectWorkContext::new(
                ProjectWorkContextId::new("context-1"),
                ProjectWorkContextSurface::Tauri,
                "window-a",
                ProjectId::new("project-1"),
                200,
                10,
                10,
            ),
        ]));
        let handler = OpenProjectWorkContextHandler::new(
            project_repository,
            context_repository,
            FixedProjectWorkContextIdGenerator::new("context-2"),
            FixedClock::new(100),
        );

        let error = handler
            .handle(OpenProjectWorkContextRequest {
                surface: ContractProjectWorkContextSurface::Tauri,
                window_id: "window-b".to_string(),
                project_id: "project-1".to_string(),
            })
            .unwrap_err();

        assert_eq!(
            error,
            ApplicationError::ProjectOccupied {
                project_id: "project-1".to_string(),
            }
        );
    });
}

/// Verifies renew requests refresh the lease using backend time instead of caller-provided expiry.
#[test]
fn renews_project_work_context_leases() {
    with_trace_logging(|| {
        let context_repository = Rc::new(FakeProjectWorkContextRepository::with_contexts(vec![
            ProjectWorkContext::new(
                ProjectWorkContextId::new("context-1"),
                ProjectWorkContextSurface::Web,
                "main",
                ProjectId::new("project-1"),
                50,
                10,
                10,
            ),
        ]));
        let handler =
            RenewProjectWorkContextHandler::new(context_repository.clone(), FixedClock::new(40));

        let response = handler
            .handle(RenewProjectWorkContextRequest {
                surface: ContractProjectWorkContextSurface::Web,
                window_id: "main".to_string(),
            })
            .unwrap_or_else(|error| panic!("renew handler failed: {error}"));

        assert_eq!(
            response,
            RenewProjectWorkContextResponse {
                context: ContractProjectWorkContext {
                    id: "context-1".to_string(),
                    surface: ContractProjectWorkContextSurface::Web,
                    window_id: "main".to_string(),
                    project_id: "project-1".to_string(),
                    lease_expires_at: 120_040,
                },
            }
        );
    });
}

/// Verifies conflict logging includes the owning surface and window id for operators.
#[test]
fn emits_owner_details_in_conflict_logs() {
    let recorder = EventRecorder::default();
    with_recorded_trace_logging(recorder.layer(), || {
        let project_repository = Rc::new(FakeProjectRepository::with_projects(vec![Project::new(
            ProjectId::new("project-1"),
            "Ora",
            "/workspace/ora",
            AuditFields::new(1, 1, false),
        )]));
        let context_repository = Rc::new(FakeProjectWorkContextRepository::with_contexts(vec![
            ProjectWorkContext::new(
                ProjectWorkContextId::new("context-1"),
                ProjectWorkContextSurface::Tauri,
                "window-a",
                ProjectId::new("project-1"),
                200,
                10,
                10,
            ),
        ]));
        let handler = OpenProjectWorkContextHandler::new(
            project_repository,
            context_repository,
            FixedProjectWorkContextIdGenerator::new("context-2"),
            FixedClock::new(100),
        );

        assert_eq!(
            handler
                .handle(OpenProjectWorkContextRequest {
                    surface: ContractProjectWorkContextSurface::Tauri,
                    window_id: "window-b".to_string(),
                    project_id: "project-1".to_string(),
                })
                .unwrap_err(),
            ApplicationError::ProjectOccupied {
                project_id: "project-1".to_string(),
            }
        );
    });

    assert_eq!(
        recorder.events(),
        vec![LoggedEvent {
            level: "ERROR".to_string(),
            target: "ora_application::project_work_context::handlers".to_string(),
            fields: BTreeMap::from([
                ("error.kind".to_string(), "project_occupied".to_string()),
                (
                    "error.message".to_string(),
                    "project is already occupied: project-1".to_string(),
                ),
                (
                    "message".to_string(),
                    "project work context operation failed".to_string(),
                ),
                (
                    "method".to_string(),
                    "log_project_work_context_failure".to_string(),
                ),
                (
                    "operation".to_string(),
                    "open_project_work_context".to_string(),
                ),
                ("owner.surface".to_string(), "tauri".to_string()),
                ("owner.window_id".to_string(), "window-a".to_string()),
                ("project_id".to_string(), "project-1".to_string()),
                ("surface".to_string(), "tauri".to_string()),
                ("window_id".to_string(), "window-b".to_string()),
            ]),
        }]
    );
}

#[derive(Debug, Default)]
struct FakeProjectRepository {
    projects: RefCell<Vec<Project>>,
    next_error: RefCell<Option<RepositoryError>>,
}

impl FakeProjectRepository {
    /// Builds the fake repository from a deterministic in-memory project list.
    fn with_projects(projects: Vec<Project>) -> Self {
        Self {
            projects: RefCell::new(projects),
            next_error: RefCell::new(None),
        }
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
    /// Persists a new project in memory for create-path tests that need it.
    fn create_project(&self, project: Project) -> Result<Project, RepositoryError> {
        self.take_error()?;
        self.projects.borrow_mut().push(project.clone());
        Ok(project)
    }

    /// Loads one visible project by identifier from the fake in-memory store.
    fn find_project(&self, project_id: &ProjectId) -> Result<Option<Project>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .projects
            .borrow()
            .iter()
            .find(|project| project.id == *project_id && !project.audit_fields.is_deleted)
            .cloned())
    }

    /// Loads one visible project by exact name from the fake in-memory store.
    fn find_project_by_name(&self, project_name: &str) -> Result<Option<Project>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .projects
            .borrow()
            .iter()
            .find(|project| project.name == project_name && !project.audit_fields.is_deleted)
            .cloned())
    }

    /// Lists every visible project from the fake in-memory store.
    fn list_projects(&self) -> Result<Vec<Project>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .projects
            .borrow()
            .iter()
            .filter(|project| !project.audit_fields.is_deleted)
            .cloned()
            .collect())
    }

    /// Replaces one visible project snapshot in the fake in-memory store.
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

    /// Soft-deletes one visible project in the fake in-memory store.
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

#[derive(Debug, Default)]
struct FakeProjectWorkContextRepository {
    contexts: RefCell<Vec<ProjectWorkContext>>,
    next_error: RefCell<Option<RepositoryError>>,
}

impl FakeProjectWorkContextRepository {
    /// Builds the fake repository from a deterministic in-memory context list.
    fn with_contexts(contexts: Vec<ProjectWorkContext>) -> Self {
        Self {
            contexts: RefCell::new(contexts),
            next_error: RefCell::new(None),
        }
    }

    /// Returns every stored context so tests can assert full state transitions.
    fn contexts(&self) -> Vec<ProjectWorkContext> {
        self.contexts.borrow().clone()
    }

    /// Returns a queued error when a test wants to simulate repository failure.
    fn take_error(&self) -> Result<(), RepositoryError> {
        match self.next_error.borrow_mut().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl ProjectWorkContextRepository for Rc<FakeProjectWorkContextRepository> {
    /// Persists a new in-memory work context snapshot.
    fn create_project_work_context(
        &self,
        context: ProjectWorkContext,
    ) -> Result<ProjectWorkContext, RepositoryError> {
        self.take_error()?;
        self.contexts.borrow_mut().push(context.clone());
        Ok(context)
    }

    /// Loads one work context by surface and window identity from the in-memory store.
    fn find_project_work_context(
        &self,
        surface: ProjectWorkContextSurface,
        window_id: &str,
    ) -> Result<Option<ProjectWorkContext>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .contexts
            .borrow()
            .iter()
            .find(|context| context.surface == surface && context.window_id == window_id)
            .cloned())
    }

    /// Loads one active work context for the requested project from the in-memory store.
    fn find_active_project_work_context_for_project(
        &self,
        project_id: &ProjectId,
        active_after: i64,
    ) -> Result<Option<ProjectWorkContext>, RepositoryError> {
        self.take_error()?;

        Ok(self
            .contexts
            .borrow()
            .iter()
            .filter(|context| {
                context.project_id == *project_id && context.lease_expires_at > active_after
            })
            .max_by_key(|context| (context.lease_expires_at, context.updated_at))
            .cloned())
    }

    /// Replaces one existing work context snapshot in the in-memory store.
    fn update_project_work_context(
        &self,
        context: ProjectWorkContext,
    ) -> Result<ProjectWorkContext, RepositoryError> {
        self.take_error()?;

        let mut contexts = self.contexts.borrow_mut();
        if let Some(existing_context) = contexts
            .iter_mut()
            .find(|existing_context| existing_context.id == context.id)
        {
            *existing_context = context.clone();
            Ok(context)
        } else {
            Err(RepositoryError::from_message(format!(
                "missing project work context during update: {}",
                context.id
            )))
        }
    }

    /// Deletes one existing work context by surface and window identity.
    fn delete_project_work_context(
        &self,
        surface: ProjectWorkContextSurface,
        window_id: &str,
    ) -> Result<bool, RepositoryError> {
        self.take_error()?;

        let mut contexts = self.contexts.borrow_mut();
        let original_len = contexts.len();
        contexts.retain(|context| !(context.surface == surface && context.window_id == window_id));
        Ok(contexts.len() != original_len)
    }

    /// Deletes expired contexts older than the supplied cutoff from the in-memory store.
    fn delete_expired_project_work_contexts(
        &self,
        expired_before: i64,
    ) -> Result<usize, RepositoryError> {
        self.take_error()?;

        let mut contexts = self.contexts.borrow_mut();
        let original_len = contexts.len();
        contexts.retain(|context| context.lease_expires_at >= expired_before);
        Ok(original_len - contexts.len())
    }
}

struct FixedProjectWorkContextIdGenerator {
    project_work_context_id: ProjectWorkContextId,
}

impl FixedProjectWorkContextIdGenerator {
    /// Builds the deterministic work context id generator used by tests.
    fn new(project_work_context_id: impl Into<String>) -> Self {
        Self {
            project_work_context_id: ProjectWorkContextId::new(project_work_context_id),
        }
    }
}

impl ProjectWorkContextIdGenerator for FixedProjectWorkContextIdGenerator {
    /// Returns the deterministic id configured for the current test.
    fn generate_project_work_context_id(&self) -> ProjectWorkContextId {
        self.project_work_context_id.clone()
    }
}

struct FixedClock {
    timestamp_millis: i64,
}

impl FixedClock {
    /// Builds the deterministic clock used by tests.
    fn new(timestamp_millis: i64) -> Self {
        Self { timestamp_millis }
    }
}

impl Clock for FixedClock {
    /// Returns the deterministic Unix timestamp configured for the current test.
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
    /// Records one event with its level, target, and normalized string fields.
    fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        self.events.lock().unwrap().push(LoggedEvent {
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_string(),
            fields: visitor.fields,
        });
    }
}

/// Collects tracing event fields into a deterministic string map for assertions.
#[derive(Debug, Default)]
struct FieldVisitor {
    fields: BTreeMap<String, String>,
}

impl tracing::field::Visit for FieldVisitor {
    /// Records debug-formatted values for non-string tracing fields.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            format!("{value:?}").trim_matches('"').to_string(),
        );
    }

    /// Records string tracing fields without extra formatting noise.
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
}
