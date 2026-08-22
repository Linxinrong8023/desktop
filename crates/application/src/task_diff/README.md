# Task Diff Application Module

This module owns the transport- and storage-independent use cases behind task change review. It keeps Git execution, SQLite rows, filesystem paths, and frontend rendering outside the application layer by defining small ports and composing generic handlers over them.

## Responsibilities

- Expose a Git-backed `TaskDiffReader` that returns a task-scoped unified patch for backend composition.
- Commit and push changes only after the caller's task, worktree, and persisted branch identity have been verified.

## Ports and adapters

`TaskDiffReader` and `TaskGitWriter` are the public seams used by handlers. They use static dispatch so tests can provide in-memory fakes without a Git process. The concrete `GitTaskDiffReader` and `GitTaskGitWriter` adapters are thin translators around `gitlancer`.

The module receives a backend-resolved worktree path and never accepts a frontend filesystem path. Backend composition decides whether the path is the project checkout or an isolated task worktree and supplies the appropriate baseline before invoking a handler.

## Error and logging boundary

Validation failures are semantic `ApplicationError` variants, such as `TaskDiffCommitMessageBlank`. Git and persistence failures retain boxed `Error::source()` chains through the application port and are projected to `internal_error` by `ora-backend`. Handlers do not emit request-completion logs; Web, Tauri, and stream adapters emit one correlated completion event through `RequestLifecycle`, where `ora-logging` bounds and redacts the diagnostic chain.

The diff reader enforces a bounded patch response. Oversized patches become the public `task_diff_too_large` payload, while the discarded byte count stays out of the public contract.

## Invariants

- Commit and push operations require an active task-owned worktree and verify its path and branch against Git metadata immediately before mutation.
- The application module does not choose adapter status, Tauri behavior, public error codes, or log levels.

See [Application and Contracts Boundary](../../../../docs/application-contracts-boundary.md) and [Task Worktrees](../../../../docs/task-worktrees.md).
