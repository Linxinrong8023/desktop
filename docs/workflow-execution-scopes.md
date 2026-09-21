# Workflow Execution Scope Storage

English | [中文](workflow-execution-scopes.zh.md)

Migration `0013` adds execution identity for the [Loop implementation](workflow-loop-plan.md). It does not enable Loop scheduling.

`workflow_execution_scopes` stores roots and individual rounds. A root has no parent, round index, status, or state blob; its current execution state continues to belong to the existing run payload. A round has a parent Loop NodeRun, a one-based index from 1 to 100, a lifecycle status, and JSON state. `workflow_run_root_scopes` identifies the current root for each run. Historical roots remain after restart.

`workflow_node_runs.scope_id` is backfilled for old nodes. Inserts without an explicit scope attach to the current root within the same SQL statement. Explicit scope assignments must belong to the same run and current execution. Node scope identity cannot change after assignment. A unique partial index prevents duplicate live definition nodes within a scope; distinct rounds can execute the same definition. A Loop can own only one pending/running round, and each round index is unique for its parent NodeRun.

Restart creates a new root in the same transaction that resets the run and soft-deletes previous node instances. Old nodes retain their original scope IDs. Loop-round parents must be running Loop instances in the current root, preventing nested loops and creation through an obsolete root.

Downgrade uses the production startup reconciler's persisted `down_sql`. Before removing scope tables and the node scope column, it appends complete scope/node snapshots to `workflow_scope_downgrade_archive`, terminalizes active runs with Loop instances, and soft-deletes child node instances from legacy readers. It leaves flat runs, Sessions, Workspaces, and user files intact. The archive deliberately survives downgrade and re-upgrade; it is historical evidence, not resumable execution state. Archive UI delivery remains part of the pending history work.

Tests exercise upgrade/backfill, default root attachment, uniqueness and ownership constraints, production repository restart, the older startup path, and repeated downgrade/re-upgrade. Atomic feedback advancement and runtime scope projections remain to be implemented before Loop execution is enabled.
