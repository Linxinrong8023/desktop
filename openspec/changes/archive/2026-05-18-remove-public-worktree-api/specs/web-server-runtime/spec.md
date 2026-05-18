## MODIFIED Requirements

### Requirement: Web server runtime SHALL expose HTTP CRUD routes for supported persisted models
The system SHALL expose create, get, list, update, and delete HTTP routes for `project`, `task`, and `session`, and it SHALL expose the backend-managed project-work-context routes required for web runtime ownership. The runtime SHALL NOT expose standalone public worktree CRUD routes. Each supported route SHALL translate transport input into the matching `ora-contracts` request DTO, SHALL delegate to the corresponding `ora-application` handler, and SHALL serialize the returned `ora-contracts` response DTO without adding adapter-local response shapes. Task-create runtime wiring SHALL provide the configured project repository and worktree root needed for backend-owned linked-worktree provisioning.

#### Scenario: Client performs task CRUD over HTTP
- **WHEN** a caller creates, fetches, lists, updates, or deletes a task through the web server
- **THEN** the server delegates to the matching task application handler and returns the matching `ora-contracts` task response payload

#### Scenario: Task creation provisions an internal worktree through runtime wiring
- **WHEN** a caller creates a task through the web server
- **THEN** the runtime-owned task-create dependencies use the configured project repository and the effective worktree root to provision the task's linked worktree before the created task response is returned

#### Scenario: Client attempts standalone worktree CRUD over HTTP
- **WHEN** a caller targets `/api/worktrees` or `/api/worktrees/{worktree_id}`
- **THEN** the runtime does not provide those routes as part of the supported public HTTP API

#### Scenario: Client performs session CRUD over HTTP
- **WHEN** a caller creates, fetches, lists, updates, or deletes a session through the web server
- **THEN** the server delegates to the matching session application handler and returns the matching `ora-contracts` session response payload

#### Scenario: Existing project CRUD uses persisted storage
- **WHEN** a caller creates a project and later fetches or lists projects through the same SQLite-backed runtime
- **THEN** the project routes read and write through the persistent repository-backed application handlers instead of an in-memory bootstrap store

### Requirement: Web server runtime SHALL keep HTTP readiness and error semantics stable across supported model routes
The system SHALL return readiness success only after the database-backed runtime state is fully initialized, and it SHALL map application-layer not-found and repository failures for `project`, `task`, and `session` into stable structured HTTP error responses across the supported route families.

#### Scenario: Resource route requests a missing entity
- **WHEN** any project, task, or session get, update, or delete route delegates to an application handler that returns a not-found outcome
- **THEN** the server responds with an HTTP not-found status and a structured error payload that identifies the missing entity family

#### Scenario: Resource route encounters a repository failure
- **WHEN** any project, task, or session route delegates to an application handler that returns a repository failure
- **THEN** the server responds with an HTTP server-error status and a structured error payload instead of leaking raw infrastructure error formatting
