## Why

Task worktrees are now backend-owned infrastructure created and cleaned up as part of task lifecycle management, but the repository still exposes worktree CRUD as if worktrees were a caller-managed public resource. That mismatch leaks an internal model through contracts, generated SDK output, HTTP routes, and runtime documentation, which makes the external surface harder to understand and easier to misuse.

## What Changes

- Remove standalone public worktree CRUD from the web server runtime, including `/api/worktrees` routes and the adapter wiring behind them.
- Remove frontend-facing worktree DTO exports and endpoint metadata from `ora-contracts` and the generated `packages/contracts` SDK artifacts.
- Remove backend-owned worktree identifiers from public task payloads as part of shrinking the exposed API surface.
- Clean up runtime documentation and route tests so the documented public API matches the backend-owned worktree model.
- Remove or narrow application-layer and web-runtime entry points that only existed to support public worktree CRUD exposure.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `app-contracts`: change the contract surface so task worktrees are fully internal and no longer appear in public task payloads or standalone worktree DTOs.
- `frontend-contract-sdk`: change generated endpoint metadata and TypeScript outputs so the frontend SDK no longer exports standalone worktree operations.
- `web-server-runtime`: change the persisted HTTP runtime so it serves project, task, session, and project-work-context routes, while task worktrees remain backend-managed internal state.

## Impact

- Affected code: `crates/contracts`, `packages/contracts`, `apps/web/server`, and any tests or docs that enumerate the public HTTP surface.
- Affected APIs: removed public worktree DTOs, endpoint metadata, generated SDK operations, and `/api/worktrees` HTTP routes.
- Dependencies: no new external dependencies are expected.
- Systems: task provisioning and cleanup remain intact, but callers must stop depending on both direct worktree CRUD and public task payload worktree identifiers.
