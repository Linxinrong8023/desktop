use super::Migration;

const UP_STATEMENTS: &[&str] = &[r#"
CREATE TABLE effect_scopes (
    id TEXT PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    workspace_id TEXT NOT NULL UNIQUE REFERENCES workspaces(id),
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('active', 'retiring')),
    generation INTEGER NOT NULL DEFAULT 0 CHECK (generation >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    CHECK (scope_kind = 'workspace')
);

CREATE TABLE effect_sources (
    id TEXT PRIMARY KEY,
    effect_kind TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    namespace TEXT NOT NULL COLLATE NOCASE,
    identifier TEXT NOT NULL COLLATE NOCASE,
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('active', 'retired')),
    publication_state TEXT NOT NULL CHECK (publication_state IN ('unpublished', 'published')),
    published_revision_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (effect_kind, source_kind, namespace, identifier),
    CHECK ((publication_state = 'published') = (published_revision_id IS NOT NULL)),
    FOREIGN KEY (published_revision_id) REFERENCES effect_revisions(id)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE effect_revisions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES effect_sources(id),
    revision_key TEXT NOT NULL,
    definition_kind TEXT NOT NULL,
    definition_version INTEGER NOT NULL CHECK (definition_version > 0),
    definition_json TEXT NOT NULL,
    digest TEXT NOT NULL,
    availability TEXT NOT NULL CHECK (availability IN ('available', 'unavailable')),
    unavailable_reason TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (source_id, revision_key),
    UNIQUE (source_id, id),
    CHECK ((availability = 'unavailable') = (unavailable_reason IS NOT NULL))
);

CREATE TRIGGER effect_revisions_immutable
BEFORE UPDATE OF source_id, revision_key, definition_kind, definition_version, definition_json, digest
ON effect_revisions
BEGIN
    SELECT RAISE(ABORT, 'Effect revisions are immutable');
END;

CREATE TABLE effect_desired_effects (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL REFERENCES effect_scopes(id) ON DELETE CASCADE,
    revision_id TEXT NOT NULL REFERENCES effect_revisions(id),
    parameters_kind TEXT NOT NULL,
    parameters_version INTEGER NOT NULL CHECK (parameters_version > 0),
    parameters_json TEXT NOT NULL,
    selector_version INTEGER NOT NULL CHECK (selector_version > 0),
    selector_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (scope_id, id)
);
CREATE INDEX effect_desired_effects_revision_scope
    ON effect_desired_effects(revision_id, scope_id);

CREATE TABLE effect_consumers (
    id TEXT PRIMARY KEY,
    consumer_kind TEXT NOT NULL,
    identity_key TEXT NOT NULL,
    adapter_id TEXT NOT NULL,
    current_revision_id TEXT NOT NULL,
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('declared', 'retiring')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (consumer_kind, identity_key),
    UNIQUE (id, current_revision_id),
    FOREIGN KEY (current_revision_id) REFERENCES effect_consumer_revisions(id)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE effect_consumer_revisions (
    id TEXT PRIMARY KEY,
    consumer_id TEXT NOT NULL REFERENCES effect_consumers(id),
    capability_version INTEGER NOT NULL CHECK (capability_version > 0),
    capabilities_json TEXT NOT NULL,
    declaration_digest TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (consumer_id, id)
);

CREATE TRIGGER effect_consumer_revisions_no_update
BEFORE UPDATE ON effect_consumer_revisions
BEGIN
    SELECT RAISE(ABORT, 'Effect consumer revisions are immutable');
END;

CREATE TABLE effect_targets (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL REFERENCES effect_scopes(id),
    consumer_id TEXT NOT NULL REFERENCES effect_consumers(id),
    consumer_revision_id TEXT NOT NULL,
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('active', 'retiring')),
    claim_fence INTEGER NOT NULL DEFAULT 0 CHECK (claim_fence >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (scope_id, id),
    FOREIGN KEY (consumer_id, consumer_revision_id)
        REFERENCES effect_consumer_revisions(consumer_id, id)
);
CREATE UNIQUE INDEX effect_targets_active_scope_consumer
    ON effect_targets(scope_id, consumer_id) WHERE lifecycle = 'active';

CREATE TABLE effect_target_declarations (
    target_id TEXT PRIMARY KEY REFERENCES effect_targets(id) ON DELETE CASCADE,
    consumer_revision_id TEXT NOT NULL REFERENCES effect_consumer_revisions(id),
    declaration_version INTEGER NOT NULL CHECK (declaration_version > 0),
    declaration_json TEXT NOT NULL,
    digest TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE effect_resources (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL REFERENCES effect_scopes(id),
    resource_key TEXT NOT NULL,
    adapter_id TEXT NOT NULL,
    descriptor_version INTEGER NOT NULL CHECK (descriptor_version > 0),
    descriptor_json TEXT NOT NULL,
    materialization_format TEXT NOT NULL,
    lifecycle TEXT NOT NULL CHECK (lifecycle IN ('active', 'retiring')),
    claim_fence INTEGER NOT NULL DEFAULT 0 CHECK (claim_fence >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (scope_id, id)
);
CREATE UNIQUE INDEX effect_resources_active_scope_key
    ON effect_resources(scope_id, resource_key) WHERE lifecycle = 'active';

CREATE TABLE effect_target_resource_bindings (
    scope_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    accepts_version INTEGER NOT NULL CHECK (accepts_version > 0),
    accepts_json TEXT NOT NULL,
    coordination_kind TEXT NOT NULL
        CHECK (coordination_kind IN ('uninterrupted', 'quiesce_before_mutation')),
    coordination_contract_version INTEGER,
    coordination_contract_json TEXT,
    PRIMARY KEY (target_id, resource_id),
    FOREIGN KEY (scope_id, target_id) REFERENCES effect_targets(scope_id, id) ON DELETE CASCADE,
    FOREIGN KEY (scope_id, resource_id) REFERENCES effect_resources(scope_id, id) ON DELETE CASCADE,
    CHECK ((coordination_kind = 'quiesce_before_mutation') =
           (coordination_contract_version IS NOT NULL)),
    CHECK ((coordination_contract_version IS NULL) =
           (coordination_contract_json IS NULL))
);
CREATE INDEX effect_target_resource_bindings_resource
    ON effect_target_resource_bindings(resource_id, target_id);

CREATE TABLE effect_target_projections (
    target_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    consumer_revision_id TEXT NOT NULL REFERENCES effect_consumer_revisions(id),
    digest TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    UNIQUE (target_id, generation, consumer_revision_id)
);

CREATE TABLE effect_target_projection_effects (
    projection_digest TEXT NOT NULL REFERENCES effect_target_projections(digest) ON DELETE CASCADE,
    desired_effect_id TEXT NOT NULL,
    PRIMARY KEY (projection_digest, desired_effect_id)
);

CREATE TABLE effect_resource_requirements (
    digest TEXT PRIMARY KEY,
    target_projection_digest TEXT NOT NULL
        REFERENCES effect_target_projections(digest) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    resource_id TEXT NOT NULL,
    materialization_contract_version INTEGER NOT NULL CHECK (materialization_contract_version > 0),
    materialization_contract_json TEXT NOT NULL,
    UNIQUE (target_projection_digest, resource_id)
);

CREATE TABLE effect_resource_requirement_effects (
    requirement_digest TEXT NOT NULL
        REFERENCES effect_resource_requirements(digest) ON DELETE CASCADE,
    desired_effect_id TEXT NOT NULL,
    PRIMARY KEY (requirement_digest, desired_effect_id)
);

CREATE TABLE effect_resource_projections (
    resource_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    digest TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    UNIQUE (resource_id, generation, digest)
);

CREATE TABLE effect_resource_projection_contributors (
    projection_digest TEXT NOT NULL
        REFERENCES effect_resource_projections(digest) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    requirement_digest TEXT NOT NULL REFERENCES effect_resource_requirements(digest),
    PRIMARY KEY (projection_digest, target_id)
);

CREATE TABLE effect_resolved_materializations (
    projection_digest TEXT NOT NULL
        REFERENCES effect_resource_projections(digest) ON DELETE CASCADE,
    resource_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    managed_identity TEXT NOT NULL,
    desired_effect_id TEXT NOT NULL,
    revision_id TEXT NOT NULL REFERENCES effect_revisions(id),
    native_identity TEXT NOT NULL,
    contract_version INTEGER NOT NULL CHECK (contract_version > 0),
    contract_json TEXT NOT NULL,
    input_digest TEXT NOT NULL,
    input_version INTEGER NOT NULL CHECK (input_version > 0),
    input_json TEXT NOT NULL,
    PRIMARY KEY (projection_digest, managed_identity),
    UNIQUE (projection_digest, native_identity)
);

CREATE TRIGGER effect_target_projections_no_update
BEFORE UPDATE ON effect_target_projections
BEGIN
    SELECT RAISE(ABORT, 'Effect Target projections are immutable');
END;

CREATE TRIGGER effect_target_projection_effects_no_update
BEFORE UPDATE ON effect_target_projection_effects
BEGIN
    SELECT RAISE(ABORT, 'Effect Target projection membership is immutable');
END;

CREATE TRIGGER effect_resource_requirements_no_update
BEFORE UPDATE ON effect_resource_requirements
BEGIN
    SELECT RAISE(ABORT, 'Effect Resource requirements are immutable');
END;

CREATE TRIGGER effect_resource_requirement_effects_no_update
BEFORE UPDATE ON effect_resource_requirement_effects
BEGIN
    SELECT RAISE(ABORT, 'Effect Resource requirement membership is immutable');
END;

CREATE TRIGGER effect_resource_projections_no_update
BEFORE UPDATE ON effect_resource_projections
BEGIN
    SELECT RAISE(ABORT, 'Effect Resource projections are immutable');
END;

CREATE TRIGGER effect_resource_projection_contributors_no_update
BEFORE UPDATE ON effect_resource_projection_contributors
BEGIN
    SELECT RAISE(ABORT, 'Effect Resource projection contributors are immutable');
END;

CREATE TRIGGER effect_resolved_materializations_no_update
BEFORE UPDATE ON effect_resolved_materializations
BEGIN
    SELECT RAISE(ABORT, 'Effect resolved materializations are immutable');
END;

CREATE TABLE effect_managed_items (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    desired_effect_id TEXT NOT NULL,
    applied_revision_id TEXT NOT NULL REFERENCES effect_revisions(id),
    native_identity TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    applied_generation INTEGER NOT NULL CHECK (applied_generation >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (resource_id, native_identity),
    UNIQUE (resource_id, id),
    FOREIGN KEY (scope_id, resource_id) REFERENCES effect_resources(scope_id, id)
);

CREATE TABLE effect_target_status (
    target_id TEXT PRIMARY KEY REFERENCES effect_targets(id) ON DELETE CASCADE,
    desired_generation INTEGER NOT NULL CHECK (desired_generation >= 0),
    observed_generation INTEGER NOT NULL CHECK (observed_generation >= 0),
    applied_generation INTEGER NOT NULL CHECK (applied_generation >= 0),
    ready_generation INTEGER NOT NULL CHECK (ready_generation >= 0),
    phase TEXT NOT NULL CHECK (phase IN (
        'pending', 'planning', 'coordinating', 'applying', 'verifying', 'activating',
        'current', 'current_with_issues', 'retiring', 'recovery_required'
    )),
    recovery_operation_id TEXT REFERENCES effect_operations(id),
    status_version INTEGER NOT NULL CHECK (status_version > 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    CHECK (ready_generation <= applied_generation),
    CHECK (applied_generation <= observed_generation),
    CHECK (observed_generation <= desired_generation),
    CHECK ((phase = 'recovery_required') = (recovery_operation_id IS NOT NULL))
);

CREATE TABLE effect_resource_status (
    resource_id TEXT PRIMARY KEY REFERENCES effect_resources(id) ON DELETE CASCADE,
    desired_generation INTEGER NOT NULL CHECK (desired_generation >= 0),
    observed_generation INTEGER NOT NULL CHECK (observed_generation >= 0),
    applied_generation INTEGER NOT NULL CHECK (applied_generation >= 0),
    phase TEXT NOT NULL CHECK (phase IN (
        'pending', 'reconciling', 'current', 'retiring', 'recovery_required'
    )),
    recovery_operation_id TEXT REFERENCES effect_operations(id),
    status_version INTEGER NOT NULL CHECK (status_version > 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    CHECK (applied_generation <= observed_generation),
    CHECK (observed_generation <= desired_generation),
    CHECK ((phase = 'recovery_required') = (recovery_operation_id IS NOT NULL))
);

CREATE TABLE effect_conditions (
    id TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL CHECK (owner_kind IN ('target', 'resource')),
    owner_id TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    code TEXT NOT NULL,
    impact TEXT NOT NULL CHECK (impact IN ('blocking', 'non_blocking')),
    retry_kind TEXT NOT NULL CHECK (retry_kind IN ('on_change', 'backoff', 'manual')),
    retry_policy_version INTEGER,
    retry_policy_json TEXT,
    generation INTEGER CHECK (generation >= 0),
    safe_details_version INTEGER NOT NULL CHECK (safe_details_version > 0),
    safe_details_json TEXT NOT NULL,
    first_observed_at INTEGER NOT NULL,
    last_observed_at INTEGER NOT NULL CHECK (last_observed_at >= first_observed_at),
    UNIQUE (owner_kind, owner_id, subject_kind, subject_id, code),
    CHECK ((retry_kind = 'backoff') = (retry_policy_version IS NOT NULL)),
    CHECK ((retry_policy_version IS NULL) = (retry_policy_json IS NULL))
);
CREATE INDEX effect_conditions_owner ON effect_conditions(owner_kind, owner_id);

CREATE TABLE effect_reconcile_requests (
    target_id TEXT PRIMARY KEY REFERENCES effect_targets(id) ON DELETE CASCADE,
    requested_generation INTEGER NOT NULL CHECK (requested_generation >= 0),
    state TEXT NOT NULL CHECK (state IN ('pending', 'claimed', 'retry_scheduled', 'blocked')),
    wake_reasons_json TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    claim_token INTEGER,
    claim_worker TEXT,
    lease_until INTEGER,
    retry_attempt INTEGER,
    not_before INTEGER,
    blocked_conditions_json TEXT,
    resume_trigger_version INTEGER,
    resume_trigger_json TEXT,
    requested_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= requested_at),
    CHECK ((state = 'claimed') =
           (claim_token IS NOT NULL AND claim_worker IS NOT NULL AND lease_until IS NOT NULL)),
    CHECK ((state = 'retry_scheduled') =
           (retry_attempt IS NOT NULL AND not_before IS NOT NULL)),
    CHECK ((state = 'blocked') =
           (blocked_conditions_json IS NOT NULL AND resume_trigger_version IS NOT NULL
            AND resume_trigger_json IS NOT NULL))
);
CREATE INDEX effect_reconcile_requests_due
    ON effect_reconcile_requests(state, not_before, requested_at, target_id);
CREATE INDEX effect_reconcile_requests_leases
    ON effect_reconcile_requests(lease_until) WHERE state = 'claimed';

CREATE TABLE effect_resource_claims (
    resource_id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    target_claim_token INTEGER NOT NULL,
    resource_fence INTEGER NOT NULL,
    worker TEXT NOT NULL,
    lease_until INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (scope_id, resource_id)
        REFERENCES effect_resources(scope_id, id) ON DELETE CASCADE,
    FOREIGN KEY (scope_id, target_id) REFERENCES effect_targets(scope_id, id)
);
CREATE INDEX effect_resource_claims_leases ON effect_resource_claims(lease_until);

CREATE TABLE effect_reconcile_attempts (
    id TEXT PRIMARY KEY,
    target_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    consumer_revision_id TEXT NOT NULL REFERENCES effect_consumer_revisions(id),
    target_projection_digest TEXT NOT NULL REFERENCES effect_target_projections(digest),
    coordination_plan_version INTEGER NOT NULL CHECK (coordination_plan_version > 0),
    coordination_plan_json TEXT NOT NULL,
    phase TEXT NOT NULL CHECK (phase IN (
        'prepared', 'coordinated', 'applied', 'verified', 'activated', 'finalized',
        'recovery_required'
    )),
    prepared_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= prepared_at)
);

CREATE TABLE effect_attempt_resource_projections (
    attempt_id TEXT NOT NULL REFERENCES effect_reconcile_attempts(id) ON DELETE CASCADE,
    resource_projection_digest TEXT NOT NULL REFERENCES effect_resource_projections(digest),
    sequence INTEGER NOT NULL CHECK (sequence >= 0),
    PRIMARY KEY (attempt_id, resource_projection_digest),
    UNIQUE (attempt_id, sequence)
);

CREATE TRIGGER effect_attempt_resource_projections_no_update
BEFORE UPDATE ON effect_attempt_resource_projections
BEGIN
    SELECT RAISE(ABORT, 'Effect Attempt Resource projections are immutable');
END;

CREATE TRIGGER effect_reconcile_attempts_inputs_immutable
BEFORE UPDATE OF target_id, generation, consumer_revision_id, target_projection_digest,
                 coordination_plan_version, coordination_plan_json
ON effect_reconcile_attempts
BEGIN
    SELECT RAISE(ABORT, 'Effect reconcile attempt inputs are immutable');
END;

CREATE TABLE effect_coordination_receipts (
    attempt_id TEXT NOT NULL REFERENCES effect_reconcile_attempts(id) ON DELETE CASCADE,
    target_id TEXT NOT NULL,
    contract_version INTEGER NOT NULL CHECK (contract_version > 0),
    contract_json TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('safe_to_mutate', 'reactivated')),
    proof_version INTEGER NOT NULL CHECK (proof_version > 0),
    proof_json TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    PRIMARY KEY (attempt_id, target_id, state)
);

CREATE TRIGGER effect_coordination_receipts_no_update
BEFORE UPDATE ON effect_coordination_receipts
BEGIN
    SELECT RAISE(ABORT, 'Effect coordination receipts are immutable');
END;

CREATE TABLE effect_operations (
    id TEXT PRIMARY KEY,
    attempt_id TEXT NOT NULL REFERENCES effect_reconcile_attempts(id),
    resource_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    sequence INTEGER NOT NULL CHECK (sequence >= 0),
    mutation TEXT NOT NULL CHECK (mutation IN ('create', 'update', 'replace', 'delete')),
    expected_version INTEGER NOT NULL CHECK (expected_version > 0),
    expected_json TEXT NOT NULL,
    planned_version INTEGER NOT NULL CHECK (planned_version > 0),
    planned_json TEXT NOT NULL,
    payload_version INTEGER NOT NULL CHECK (payload_version > 0),
    payload_json TEXT NOT NULL,
    phase TEXT NOT NULL CHECK (phase IN ('prepared', 'applied', 'finalized', 'recovery_required')),
    prepared_at INTEGER NOT NULL,
    applied_at INTEGER,
    finalized_at INTEGER,
    updated_at INTEGER NOT NULL CHECK (updated_at >= prepared_at),
    UNIQUE (attempt_id, sequence),
    CHECK (applied_at IS NULL OR applied_at >= prepared_at),
    CHECK (finalized_at IS NULL OR (applied_at IS NOT NULL AND finalized_at >= applied_at))
);
CREATE INDEX effect_operations_recovery
    ON effect_operations(resource_id, prepared_at, id) WHERE phase <> 'finalized';

CREATE TRIGGER effect_operations_intent_immutable
BEFORE UPDATE OF attempt_id, resource_id, generation, sequence, mutation, expected_version,
                 expected_json, planned_version, planned_json, payload_version, payload_json
ON effect_operations
BEGIN
    SELECT RAISE(ABORT, 'Effect operation intent is immutable');
END;

CREATE TABLE effect_operation_artifacts (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES effect_operations(id),
    role TEXT NOT NULL,
    locator_version INTEGER NOT NULL CHECK (locator_version > 0),
    locator_json TEXT NOT NULL,
    expected_fingerprint TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN (
        'reserved', 'retained', 'pending_cleanup', 'cleanup_failed'
    )),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL CHECK (updated_at >= created_at),
    UNIQUE (operation_id, role, locator_version, locator_json)
);
CREATE INDEX effect_operation_artifacts_cleanup
    ON effect_operation_artifacts(state, updated_at, id)
    WHERE state IN ('pending_cleanup', 'cleanup_failed');

CREATE TRIGGER effect_operation_artifacts_authority_immutable
BEFORE UPDATE OF operation_id, role, locator_version, locator_json, expected_fingerprint
ON effect_operation_artifacts
BEGIN
    SELECT RAISE(ABORT, 'Effect operation artifact authority is immutable');
END;

CREATE TABLE effect_readiness_receipts (
    target_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    consumer_revision_id TEXT NOT NULL REFERENCES effect_consumer_revisions(id),
    projection_digest TEXT NOT NULL REFERENCES effect_target_projections(digest),
    proof_version INTEGER NOT NULL CHECK (proof_version > 0),
    proof_json TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    PRIMARY KEY (target_id, generation, consumer_revision_id, projection_digest)
);

CREATE TRIGGER effect_readiness_receipts_no_update
BEFORE UPDATE ON effect_readiness_receipts
BEGIN
    SELECT RAISE(ABORT, 'Effect readiness receipts are immutable');
END;

CREATE TABLE effect_audit_events (
    id TEXT PRIMARY KEY,
    scope_id TEXT,
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    generation INTEGER CHECK (generation >= 0),
    initiator_kind TEXT NOT NULL CHECK (initiator_kind IN ('user', 'system', 'consumer')),
    initiator_id TEXT,
    payload_version INTEGER NOT NULL CHECK (payload_version > 0),
    payload_json TEXT NOT NULL,
    occurred_at INTEGER NOT NULL,
    CHECK ((initiator_kind = 'consumer') = (initiator_id IS NOT NULL))
);
CREATE INDEX effect_audit_events_scope_time
    ON effect_audit_events(scope_id, occurred_at, id) WHERE scope_id IS NOT NULL;
CREATE INDEX effect_audit_events_subject_time
    ON effect_audit_events(subject_kind, subject_id, occurred_at, id);
CREATE INDEX effect_audit_events_kind_time
    ON effect_audit_events(event_kind, occurred_at, id);

CREATE TRIGGER effect_audit_events_no_update
BEFORE UPDATE ON effect_audit_events
BEGIN
    SELECT RAISE(ABORT, 'Effect audit events are append-only');
END;

-- Workspace is the only legal first-phase scope. The trigger creates the isolation root, while
-- Desired defaults and consumer pairing remain convergence policy in the repository/worker.
CREATE TRIGGER effect_scopes_after_workspace_insert
AFTER INSERT ON workspaces
BEGIN
    INSERT INTO effect_scopes (
        id, scope_kind, workspace_id, lifecycle, generation, created_at, updated_at
    ) VALUES (
        'workspace:' || NEW.id, 'workspace', NEW.id, 'active', 0, NEW.created_at, NEW.updated_at
    );
END;

INSERT INTO effect_scopes (
    id, scope_kind, workspace_id, lifecycle, generation, created_at, updated_at
)
SELECT 'workspace:' || id, 'workspace', id, 'active', 0, created_at, updated_at
FROM workspaces;
"#];

const DOWN_STATEMENTS: &[&str] = &[r#"
DROP TRIGGER IF EXISTS effect_scopes_after_workspace_insert;
DROP TRIGGER IF EXISTS effect_audit_events_no_update;
DROP TRIGGER IF EXISTS effect_readiness_receipts_no_update;
DROP TRIGGER IF EXISTS effect_coordination_receipts_no_update;
DROP TRIGGER IF EXISTS effect_operation_artifacts_authority_immutable;
DROP TRIGGER IF EXISTS effect_operations_intent_immutable;
DROP TRIGGER IF EXISTS effect_attempt_resource_projections_no_update;
DROP TRIGGER IF EXISTS effect_reconcile_attempts_inputs_immutable;
DROP TRIGGER IF EXISTS effect_resolved_materializations_no_update;
DROP TRIGGER IF EXISTS effect_resource_projection_contributors_no_update;
DROP TRIGGER IF EXISTS effect_resource_projections_no_update;
DROP TRIGGER IF EXISTS effect_resource_requirement_effects_no_update;
DROP TRIGGER IF EXISTS effect_resource_requirements_no_update;
DROP TRIGGER IF EXISTS effect_target_projection_effects_no_update;
DROP TRIGGER IF EXISTS effect_target_projections_no_update;
DROP TRIGGER IF EXISTS effect_consumer_revisions_no_update;
DROP TRIGGER IF EXISTS effect_revisions_immutable;
DROP TABLE IF EXISTS effect_audit_events;
DROP TABLE IF EXISTS effect_readiness_receipts;
DROP TABLE IF EXISTS effect_operation_artifacts;
DROP TABLE IF EXISTS effect_resource_status;
DROP TABLE IF EXISTS effect_target_status;
DROP TABLE IF EXISTS effect_operations;
DROP TABLE IF EXISTS effect_coordination_receipts;
DROP TABLE IF EXISTS effect_attempt_resource_projections;
DROP TABLE IF EXISTS effect_reconcile_attempts;
DROP TABLE IF EXISTS effect_resource_claims;
DROP TABLE IF EXISTS effect_reconcile_requests;
DROP TABLE IF EXISTS effect_conditions;
DROP TABLE IF EXISTS effect_managed_items;
DROP TABLE IF EXISTS effect_resolved_materializations;
DROP TABLE IF EXISTS effect_resource_projection_contributors;
DROP TABLE IF EXISTS effect_resource_projections;
DROP TABLE IF EXISTS effect_resource_requirement_effects;
DROP TABLE IF EXISTS effect_resource_requirements;
DROP TABLE IF EXISTS effect_target_projection_effects;
DROP TABLE IF EXISTS effect_target_projections;
DROP TABLE IF EXISTS effect_target_resource_bindings;
DROP TABLE IF EXISTS effect_resources;
DROP TABLE IF EXISTS effect_target_declarations;
DROP TABLE IF EXISTS effect_targets;
DROP TABLE IF EXISTS effect_consumer_revisions;
DROP TABLE IF EXISTS effect_consumers;
DROP TABLE IF EXISTS effect_desired_effects;
DROP TABLE IF EXISTS effect_revisions;
DROP TABLE IF EXISTS effect_sources;
DROP TABLE IF EXISTS effect_scopes;
"#];

/// Builds the Generic Target Effect persistence model.
pub fn migration() -> Migration {
    Migration::new("0006", UP_STATEMENTS, DOWN_STATEMENTS)
}
