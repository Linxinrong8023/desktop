# Workflow unused nodes implementation plan

English | [中文](workflow-unused-nodes-plan.zh.md)

Status: implemented. This document retains the design and acceptance scope; see [Workflow](workflow.md) for product behavior.

## Goal and scope

Allow authors to retain individual nodes or connected groups outside the execution path for debugging.
Preserve the complete canvas in drafts and published snapshots, while executing only the subgraph reachable from its entry.
This proposal does not introduce manual disable switches or standalone node execution.

## Behavior and constraints before this change

- Draft-save and publish handlers in `crates/application/src/workflow/handlers.rs` store graphs or create snapshots without checking reachability there.
- Startup in `crates/application/src/workflow_run/engine/engine.rs` rejects nodes unreachable from Start through `UnreachableNodes`.
- `crates/application/src/workflow_run/engine/loop_graph.rs` requires nodes in each scope to be reachable from that scope's Start.
- `crates/application/src/workflow_run/engine/iteration.rs` requires every Iteration member to be reachable through the container's entry edges.
- `crates/application/src/workflow_run/engine/branch_projection.rs` treats nodes without incoming edges as `Ready`. Removing reachability checks alone could execute isolated nodes.
- `crates/backend/src/workflow/run/prerequisites.rs` resolves roles and Skills for all Agents across scopes. Changing scheduling alone would still let unused dependencies block a run.

## Behavioral definition

An unused node is unreachable along directed edges from its scope's effective entry. It need not be completely disconnected.

```text
Start → A → Output       Executes normally
B → C                    Preserved, not executed
D                        Preserved, not executed
```

1. Compute root-scope reachability from Start.
2. A Loop participates only when reachable in its outer scope; compute its internal reachability from its own Start.
3. An Iteration participates only when reachable in its outer scope; compute internal reachability from its entry edges. Existing execution constraints still apply, including valid entry members for an active Iteration.
4. All members of an unused container are unused, regardless of local connectivity inside it.
5. Include every Condition outlet in static reachability. Keep existing runtime branch selection distinct from unused-node classification.
6. Unused nodes create no NodeRun or Session, require no runtime dependencies, wait for no interaction, and do not affect completion.
7. Reconnecting a node to the execution path restores normal execution validation automatically.
8. Do not persist `enabled` or `unused` flags; derive participation from topology.

Handle this case explicitly:

```text
Start → A → Output
B ─────→ A
```

B is unreachable from Start. Exclude B and its edge to A from the execution subgraph so A never waits for B.
If A references B's output, report an explicit execution validation error rather than silently substituting an empty value.
Variable references do not implicitly activate unused nodes.

## Backend design

Add a separate module within the `workflow_run` domain to derive the execution graph from the complete document, avoiding further growth of the large `graph.rs` module.
Choose concrete module and type names during implementation, while preserving these responsibilities and ordering:

```text
Complete workflow document
    ↓
Document structure checks and scope partitioning
    ↓
Compute participating nodes and edges
    ↓
Parse executable node configurations and validate the execution subgraph
    ↓
Prerequisite preparation, scheduling, and recovery
```

Determine execution membership before parsing participating nodes' complete configurations.
The current full parser already validates configuration and container membership; filtering after parsing would still let incomplete unused nodes block runs.
Initial document parsing reads only fields needed for topology and scope derivation, retaining raw configuration for later processing.

### Validation boundaries

| Scope              | Rules                                                                                                                                                                                                |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Complete document  | Valid JSON and required structure, unique node IDs, existing edge endpoints, valid container ownership, no illegal cross-scope edges, unique entries                                                 |
| Execution subgraph | Supported node types, complete configuration, DAG constraints, branch ports, variable references, Output constraints, container execution constraints, roles, Skills, and other runtime dependencies |

Configuration errors on unused nodes may appear as editor diagnostics but do not block the main flow. Reconnecting those nodes makes the errors blocking again.
Global variables remain workflow-level declarations with existing validation; unused nodes do not relax that validation.
An active scope without an entry remains invalid.

### Shared consumers

These consumers must use the same execution-membership derivation rather than maintain separate filters:

- Role, Skill, and other prerequisite preparation at run creation, plus variable-pool initialization.
- Startup, subsequent scheduling, completion detection, and variable resolution.
- Loop and Iteration internal scheduling.
- Restart, recovery, interactive continuation, and prompt construction that needs the execution graph.

Separate the original snapshot from its derived execution graph. Execution consumers use the derived graph so reparsing a complete snapshot cannot accidentally restore unused nodes.
Derivation must be deterministic for a frozen snapshot; restart and recovery must not use the current editor draft.
Also tighten the scheduler's no-incoming-edge rule so only legitimate scope entries can become initial ready nodes.

## Persistence, publishing, and compatibility

Drafts and published snapshots preserve all nodes, configurations, positions, and edges. The derived execution graph must never overwrite the authoring document.
Saving, publishing, rollback, import/export, and reopening must retain unused content.
Existing fully connected workflows retain their behavior. No database or local filesystem layout migration should be necessary.
Editing a draft during a run does not alter that run's frozen execution membership.

## Editor and run views

- Show an “Excluded from execution” badge on unused nodes, with reduced visual emphasis while keeping editing available.
- Explain: “This node is not connected from the start node and will not run.” For container members, explain the relevant container or entry reason.
- Show a summary such as “N nodes excluded from execution” without blocking save, publish, or run.
- Refresh after connecting, disconnecting, undo, and redo. Asynchronous analysis responses must match the current document revision; stale results must not replace newer state.
- Run details may show the complete snapshot, but unused nodes must appear as excluded, not waiting to execute or as an unselected branch.
- Do not create synthetic NodeRun records for display.

Provide frontend participation sets and diagnostics through a backend domain analysis interface rather than duplicating container reachability rules.
Expose only the sets and diagnostics needed by actual consumers, without introducing a generic graph framework.
Declare contracts, logical operations, and Desktop bindings in their respective owners and generate clients and related artifacts according to the feature change guide.
Reuse existing request lifecycle helpers. Clean up analysis requests on workflow switches and unmount to prevent results leaking across workflows.
Keep translations with the workflow-editor and relevant run-view domain owners.

## Implementation sequence

1. Add execution-graph derivation, separate document checks from execution validation, and cover root and container semantics.
2. Unify graph sources for prerequisites, scheduling, variable resolution, restart, and recovery; tighten ready-node rules.
3. Add the minimal editor analysis interface, generate contracts and bindings, and integrate node badges and summary diagnostics.
4. Update run views and verify complete snapshot preservation and excluded-node display.
5. Complete behavioral tests and update both `docs/workflow.md` and `docs/workflow.zh.md` to describe the implemented behavior.

## Verification and acceptance

| Scenario                                                                        | Expected result                                                                         |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Isolated Agent or spare chain beside the main flow                              | Main flow succeeds; unused nodes create no NodeRun, Session, or prerequisite calls      |
| Missing role or Skill, or incomplete configuration on an unused node            | Main flow proceeds; reconnecting restores normal errors                                 |
| Unreachable node points to a reachable node                                     | No accidental execution or blocked join                                                 |
| Reachable node references an unused node's output                               | Explicit validation error                                                               |
| Spare nodes inside Loop or Iteration                                            | Membership follows the appropriate entry; spare members do not execute                  |
| Entire container is unreachable                                                 | Neither container nor members execute or prepare dependencies                           |
| Active subgraph has a cycle, invalid branch, invalid entry, or cross-scope edge | Relevant errors remain rejected                                                         |
| Condition branch selection and joins                                            | Existing runtime semantics remain intact                                                |
| Cancellation, restart, recovery, interactive continuation                       | No unused-node activation, resource leaks, or state crossover between runs or workflows |
| Save, publish, rollback, import/export, reopen                                  | Spare nodes, configurations, and edges remain intact                                    |
| Connect, disconnect, undo/redo, rapid document switching                        | Diagnostics match the latest document; stale analysis responses have no effect          |

Backend tests exercise production interfaces for execution and host prerequisite preparation. Frontend tests use typed operation handlers, await React updates correctly, and pass the clean-stderr gate.
During implementation, run the smallest relevant tests first, followed by required contract, frontend, and Rust checks.
Desktop binding changes require `task test:tauri`; run `task test` before completing the cross-layer implementation.
The implementation passed `task test`, including frontend clean-stderr, the Rust workspace, Tauri, and Desktop E2E, with additional focused coverage for execution membership, persistence, request cancellation, and run views.
