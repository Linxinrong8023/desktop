# Ora Workflow Module Resources

## Knowledge

- Repository source: `../desktop/docs/workflow.md`
  Primary architecture overview. Use for: definition/snapshot lifecycle, run pinning, table ownership, deletion constraints, and module boundaries.
- Repository source: `../desktop/crates/db/src/migration/schema/schema_v0003.rs`
  Authoritative initial SQLite schema. Use for: the four core tables, foreign-key relationships, indexes, nullable lifecycle fields, and soft deletion.
- Repository source: `../desktop/crates/application/src/workflow_run/engine/README.md`, `graph.rs`, `engine.rs`
  Production DAG model and state machine. Use for: graph validation, topology, ready-set scheduling, start/cancel/restart, and persistence-port boundaries.
- Repository source: `../desktop/crates/backend/src/workflow/run/README.md`, `executor.rs`, `prompt.rs`
  Production Agent adapter. Use for: Session execution, prompt handoff, skills, outputs, file changes, and interactive-node behavior.
- Repository source: `../desktop/packages/app-shell/src/features/workflow-editor/README.md` and `workflow-run/README.md`
  Frontend ownership notes. Use for: separating definition authoring from Theater/Overview and identifying documentation drift against live hooks.
- Repository source: `../desktop/packages/app-shell/src/state/hooks/use-workflow-runs.ts`, `../desktop/apps/desktop/web/tauri-transport.ts`, and `../desktop/apps/desktop/src-tauri/src/commands.rs`
  Production desktop command chain. Use for: tracing a workflow start from React mutation through the generated Contract client and Tauri adapter into the shared Backend.
- Repository source: `../desktop/crates/application/src/workflow_run/engine/node_type.rs` and `../desktop/packages/app-shell/src/features/workflow-editor/workflow-node-catalog.tsx`
  Current node capability boundary. Use for: distinguishing production-executable Start/Agent/Output nodes from known-but-rejected Rust variants and hidden mock/prototype metadata.
- Repository source: `../desktop/packages/workflow-runtime/README.md` and `src/types.ts`
  Transport-neutral frontend model and memory adapter. Use for: understanding the mock/runtime seam without confusing it with Rust persistence.
- [React Flow: `ReactFlowInstance`](https://reactflow.dev/api-reference/types/react-flow-instance)
  Official API reference. Use for: confirming that `toObject()` yields nodes, edges, and viewport as a serializable graph document.
- [petgraph: `toposort`](https://docs.rs/petgraph/latest/petgraph/algo/fn.toposort.html)
  Official crate documentation. Use for: topological ordering, cycle rejection, and complexity of DAG validation.
- [SQLite: Partial Indexes](https://www.sqlite.org/partialindex.html)
  Official SQLite documentation. Use for: understanding Ora's uniqueness rules over only visible (`is_deleted = 0`) workflow rows.
- [SQLite: Foreign Key Support](https://www.sqlite.org/foreignkeys.html)
  Official SQLite documentation. Use for: parent/child table relationships and the runtime requirement to enable foreign-key enforcement.

## Wisdom (Communities)

- Ora pull-request reviews and the responsible workflow maintainer
  Use for: testing explanations against the intent behind lifecycle invariants and challenging whether documentation still matches the live composition.
- [Rust Users Forum](https://users.rust-lang.org/)
  Moderated practitioner community. Use for: pressure-testing Rust ownership, trait-boundary, concurrency, and persistence questions that are not Ora product decisions.

## Gaps

- The frontend currently contains both persisted-contract hooks and a default memory runtime for some live projections. The exact production composition must be revalidated against the live entrypoint before the frontend-integration lesson.
- There is no single canonical failure-injection guide for workflow runs; the debugging lessons will derive one from repository tests and observable database transitions.
