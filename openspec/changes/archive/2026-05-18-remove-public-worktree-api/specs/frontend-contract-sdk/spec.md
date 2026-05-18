## ADDED Requirements

### Requirement: Generated SDK SHALL omit standalone worktree operations
The generated frontend SDK SHALL only expose supported public operations. Because task worktrees are backend-owned internal state, endpoint metadata and generated TypeScript client helpers SHALL omit standalone create, get, list, update, and delete worktree operations.

#### Scenario: Contract export generates endpoint metadata after the cleanup
- **WHEN** the contract export workflow runs after public worktree removal
- **THEN** the generated endpoint manifest excludes `/api/worktrees` and `/api/worktrees/{worktreeId}` operations

#### Scenario: Frontend code imports the generated client
- **WHEN** frontend code consumes `@ora/contracts`
- **THEN** it can call supported project, project-work-context, task, and session operations, and it cannot call generated standalone worktree client methods
