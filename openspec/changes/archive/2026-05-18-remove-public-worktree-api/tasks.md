## 1. Remove public worktree contracts and SDK exports

- [x] 1.1 Remove standalone worktree DTO exports, frontend endpoint metadata, public re-exports, and public task `worktree_id` fields from `crates/contracts`.
- [x] 1.2 Regenerate `packages/contracts` so the TypeScript DTO set and generated client no longer expose standalone worktree operations.
- [x] 1.3 Update Rust and TypeScript contract tests to assert that supported task payloads omit `worktreeId` and that generated worktree CRUD operations are no longer present.

## 2. Remove public worktree HTTP runtime exposure

- [x] 2.1 Remove `/api/worktrees` route registration plus the web adapter handlers, services, and `AppState` wiring that only support standalone public worktree CRUD.
- [x] 2.2 Narrow or delete `ora-application` worktree CRUD entry points that are no longer needed after the public adapter surface is removed, while keeping repository support required by task lifecycle orchestration.
- [x] 2.3 Update runtime and integration tests so the supported router surface covers project, project-work-context, task, and session flows, and so `/api/worktrees` is no longer treated as a supported route family.

## 3. Align documentation and verification

- [x] 3.1 Update `docs/` and any route-surface references to describe worktrees as backend-managed task state rather than a caller-managed public resource.
- [x] 3.2 Run contract regeneration, formatting, and the relevant test suites needed to verify the reduced public API surface and preserved task-owned worktree lifecycle behavior.
