# Workflow Run Backend Module

This module adapts workflow-run application use cases to the production backend environment.

## Responsibilities

- `api.rs` composes workflow-run CRUD handlers and worktree provisioning.
- `engine.rs` builds the production run engine, attaches callbacks, and resumes recoverable runs.
- `executor.rs` drives agent nodes through Ora sessions and records their outputs and file changes.
- `prerequisites.rs` resolves roles and materializes required skills in the run worktree.
- `prompt.rs` assembles the localized, topology-aware handoff for an agent node.
- `interactive/` coordinates human turns and manual completion for interactive nodes.

## Boundaries

DAG parsing, scheduling, and durable node-run transitions are owned by `ora-application`. This
module supplies concrete execution and infrastructure adapters without duplicating that state
machine.
