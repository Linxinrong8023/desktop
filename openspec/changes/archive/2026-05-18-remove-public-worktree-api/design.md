## Context

The repository has already shifted task worktrees into a backend-owned lifecycle: task creation provisions a linked Git worktree and task deletion removes it. Even so, the public surface still reflected an older model in which worktrees were caller-managed resources and task payloads exposed backend-owned linkage identifiers. `ora-contracts` exported standalone worktree DTOs and endpoint metadata, `packages/contracts` generated public worktree client operations, and task contracts still leaked internal worktree linkage that callers could not meaningfully control.

That split creates two problems. First, it exposes implementation detail as product surface, which invites callers to create, mutate, or delete state that should now be owned by task lifecycle orchestration. Second, it forces every layer to keep carrying public worktree shapes that no longer match the intended model, increasing maintenance work and making future API reasoning less clear.

## Goals / Non-Goals

**Goals:**
- Remove standalone worktree CRUD from the public HTTP and generated SDK surface.
- Keep task-owned worktree provisioning and cleanup behavior intact.
- Reduce adapter and contract exports so the external API matches the intended backend-owned worktree model.
- Keep backend-owned worktree linkage entirely internal while preserving task lifecycle behavior.
- Update documentation and tests so public API descriptions no longer drift from runtime behavior.

**Non-Goals:**
- Changing how task worktrees are provisioned, named, persisted, or cleaned up internally.
- Removing internal worktree persistence records or repository ports needed by task lifecycle handlers.
- Introducing a replacement public endpoint for direct worktree inspection.

## Decisions

### Remove the public worktree route family from the web runtime

`apps/web/server` will stop registering `/api/worktrees` routes and will remove the transport adapter pieces that exist only to serve them, including `handlers/worktrees.rs`, `service/worktree.rs`, and the `WorktreeApi` slot on `AppState` and bootstrap wiring.

Why this approach:
- It removes the most visible misuse path first: callers can no longer manipulate internal task worktrees over HTTP.
- It simplifies runtime composition by aligning the router and shared state with the supported model families.

Alternatives considered:
- Leaving the routes in place but documenting them as internal. Rejected because they would still be callable, exported, and test-maintained as supported API.
- Returning `404` from existing routes without removing contracts. Rejected because SDK and docs would still advertise operations that are intentionally unsupported.

### Narrow `ora-contracts` and generated SDK exports to backend-neutral task payloads

The change will remove standalone worktree DTO exports, endpoint metadata, and generated TypeScript artifacts that model worktree CRUD as public operations. Task contracts will also stop exposing the backend-owned `worktree_id`, so callers only receive task fields they can actually use.

Why this approach:
- It makes the generated frontend package reflect the real supported API instead of legacy surface area.
- It prevents backend persistence details from leaking into otherwise transport-neutral task payloads.

Alternatives considered:
- Keeping worktree DTOs for read-only use. Rejected because the current exported shapes imply a first-class public resource and would likely cause route or SDK re-expansion later.
- Replacing worktree CRUD with a dedicated read-only endpoint in the same change. Rejected because there is no current requirement for a supported external inspection surface.

### Keep internal persistence and task orchestration, but stop exporting standalone application entry points or public linkage fields

The application layer will keep the repository abstractions and internal worktree model needed for task provisioning and cleanup, but it should stop presenting standalone worktree CRUD handlers and ID generators as part of the supported external flow. The implementation can either remove those transport-oriented handlers entirely or narrow crate exports so adapters cannot treat them as public orchestration entry points.

Why this approach:
- Task lifecycle still depends on persisted worktree records, so removing the internal model would be unnecessary churn.
- Shrinking the external entry points prevents future adapters from accidentally reviving the caller-managed worktree model.

Alternatives considered:
- Removing all worktree code from `ora-application` and `ora-db`. Rejected because task creation and deletion still need the worktree repository and persisted linkage.
- Keeping standalone handlers exported for “future internal use.” Rejected because the current problem is precisely that public-facing layers keep interpreting them as supported API.

## Risks / Trade-offs

- [Existing callers may still invoke `/api/worktrees`] → Mitigation: remove generated SDK operations in the same change and update runtime docs to make the breaking removal explicit.
- [Tests may overfit the old CRUD surface] → Mitigation: replace worktree-route coverage with task-lifecycle coverage that proves internal provisioning still works through supported task endpoints.
- [Partially removing contracts but not exports can leave stale generated artifacts] → Mitigation: make contract export regeneration part of the implementation tasks and verify `packages/contracts` no longer contains worktree operations.
- [Internal code may still depend on transport-oriented worktree handlers] → Mitigation: audit `ora-application` and `apps/web/server` references before deleting or narrowing exports, and keep repository ports intact for task handlers.

## Migration Plan

1. Remove public worktree endpoint metadata, DTO exports, and task worktree linkage fields from `ora-contracts`, then regenerate `packages/contracts`.
2. Remove web-server route registration, handler wiring, and adapter state dedicated to public worktree CRUD.
3. Narrow or delete application-layer worktree CRUD entry points that were only used by the removed adapter surface.
4. Update route tests, contract tests, and runtime documentation to reflect the reduced public API.
5. Treat the rollout as a breaking API cleanup with no compatibility shim; callers must transition to task-only worktree visibility.

## Open Questions

None.
