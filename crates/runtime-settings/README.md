# ora-runtime-settings

`ora-runtime-settings` coordinates process-wide runtime log-level reads and transactional updates without owning a runtime's persistence schema or transport.

## Responsibilities

- `RuntimeLogLevelManager` serializes asynchronous reads and updates shared by cloned Web or Desktop state without blocking runtime workers during persistence.
- Accepted updates run in internal tasks that retain the caller's request span and finish commit or rollback even when the transport request stops waiting.
- Runtime updates reload the live filter before persisting the preferred level.
- A persistence failure keeps the storage error primary and attempts to restore the previous effective level.
- `RuntimeLogLevelState` reports the configured preference, current effective level, and immutable startup environment override.
- `RuntimeLogLevelControl` and asynchronous `PreferredLogLevelStore` define statically dispatched, testable boundaries for live filtering and Backend-owned persistence.

## Boundaries

This crate does not read environment variables, define JSON schemas, initialize logging, expose HTTP or Tauri commands, or map failures into public contracts. Composition roots resolve startup precedence and runtime adapters own their storage formats. Callers must retain the logging writer guard separately for the process lifetime.
