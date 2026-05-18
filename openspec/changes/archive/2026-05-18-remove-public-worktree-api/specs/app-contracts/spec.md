## ADDED Requirements

### Requirement: Task worktrees SHALL remain internal to the public contract surface
The system SHALL treat task worktrees as backend-owned internal state in `ora-contracts`. The public contract surface SHALL NOT expose task worktree identifiers on shared task payloads, and it SHALL NOT export standalone worktree CRUD request or response DTOs, shared worktree view models, or frontend endpoint metadata for direct caller-managed worktree operations.

#### Scenario: Caller inspects task payload linkage
- **WHEN** a caller reads a created, fetched, listed, or updated task through the shared contracts
- **THEN** the task payload excludes backend-owned worktree identifiers and does not require a separate public worktree DTO to express internal linkage

#### Scenario: Frontend contract exports are generated
- **WHEN** `ora-contracts` exports its frontend-facing DTO and endpoint metadata set
- **THEN** the exported surface omits standalone worktree CRUD contract types and task contract fields that reference backend-owned worktrees
