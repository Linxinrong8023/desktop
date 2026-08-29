# ora-effect

`ora-effect` owns generic, Workspace-scoped Desired Effect convergence. Skill directory
materialization is its first Effect kind and filesystem Resources are its first Resource adapter;
neither concept defines the Core model.

## Responsibilities

- Strong identities and immutable revisions for Scopes, Sources, Desired Effects, Consumers,
  Targets, Resources, projections, ownership, Attempts, Operations, and Artifacts.
- Pure Target projection and shared-Resource merge planning with structured Conditions.
- Independent Target readiness and Resource materialization watermarks.
- Level-triggered requests, fenced Target/Resource claims, retry schedules, and journal-backed
  recovery states.
- Static-dispatch repository, Consumer adapter, and Resource adapter seams.
- Filesystem observation, exact ownership markers, journal-first mutation, verification, and
  Artifact cleanup.

## Boundaries and invariants

`EffectTarget` is the Consumer scheduling/readiness boundary; `EffectResource` is the independent
observation, locking, mutation, ownership, and recovery boundary. They are many-to-many and must not
share identity or status.

Desired, Managed, Observed, and Preserved state remain distinct. Only a matching ledger or durable
Operation/Artifact authorizes mutation. Target claims never authorize shared Resource writes, and
the reconciler reloads and replans after Resource claims close the race with other Targets.

The crate does not depend on SQLite, Tauri, or a concrete Agent runtime. Consumer- and
Resource-specific contracts are versioned adapter payloads; generic phases never contain plugin,
session, filesystem-path, or transport vocabulary.
