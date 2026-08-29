# Ora Desktop

Ora Desktop is an AI-agent IDE that keeps a Workspace's declared intent converged with the
external runtimes and resources used to perform work.

## Effect language

**Effect Scope**:
The isolation root for one complete Desired State, its generation, targets, resources, and
ownership. The first scope shape is a Workspace.
_Avoid_: Workspace Effect aggregate

**Desired State**:
The complete set of Effect intent selected for one Effect Scope at one generation.
_Avoid_: Installation list, incremental Effect events

**Desired Effect**:
One stable item of intent in Desired State, referring to an immutable Effect Revision and an
audience of eligible Targets.
_Avoid_: Managed item, installed Effect

**Consumer**:
An external runtime that consumes Effect results and can acknowledge coordination and readiness.
_Avoid_: Subscriber, observer, Agent when referring to all consumer kinds

**Effect Target**:
One Consumer's complete convergence instance inside one Effect Scope and the unit of scheduling
and readiness.
_Avoid_: Surface, Consumer

**Effect Resource**:
An external object that can be independently located, observed, locked, mutated, and recovered.
_Avoid_: Target, Consumer

**Managed Item**:
An external item for which Ora holds a durable, stable ownership identity inside one Effect
Resource.
_Avoid_: Observed item, matching file

**Observed State**:
A Resource adapter's current factual view of external state; it never establishes ownership.
_Avoid_: Managed State

**Preserved Item**:
An observed external item without exact Managed Item evidence and therefore outside Ora's mutation
authority.
_Avoid_: Unmanaged item, orphan

**Generation**:
The monotonic version of an Effect Scope's entire Desired State. Runtime progress and observation
do not advance it.
_Avoid_: Resource version, Consumer revision

**Digest**:
The content identity of immutable input or a normalized projection.
_Avoid_: Fingerprint, ownership proof

**Fingerprint**:
Evidence of an Effect Resource or external item's observed state.
_Avoid_: Digest, ownership proof
