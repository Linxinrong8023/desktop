# Interactive Workflow Nodes

This module coordinates human interaction with workflow nodes that pause for input.

## Responsibilities

- `session.rs` validates follow-up turns and moves an awaiting node between pending and running.
- `completion.rs` claims a node, reconstructs its result and file changes, and prepares the durable
  engine transition.
- `mod.rs` owns the transient completion gate shared by both paths.

## Invariants

A terminal node cannot accept another prompt, a node being completed cannot begin a concurrent
turn, and completion is revalidated after session shutdown before the engine transition commits.
