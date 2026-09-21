use super::Migration;

const UP_STATEMENTS: &[&str] = &[r#"
CREATE TABLE IF NOT EXISTS workflow_scope_downgrade_archive (
    id TEXT PRIMARY KEY,
    scope_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    record TEXT NOT NULL CHECK (json_valid(record))
);
CREATE INDEX IF NOT EXISTS workflow_scope_downgrade_archive_run
ON workflow_scope_downgrade_archive(run_id);

CREATE TABLE workflow_execution_scopes (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES workflow_runs(id),
    parent_loop_node_run_id TEXT REFERENCES workflow_node_runs(id),
    round_index INTEGER,
    status INTEGER,
    state TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    CHECK (
        (parent_loop_node_run_id IS NULL AND round_index IS NULL AND status IS NULL AND state IS NULL)
        OR (parent_loop_node_run_id IS NOT NULL AND round_index IS NOT NULL
            AND round_index BETWEEN 1 AND 100 AND status IS NOT NULL AND status BETWEEN 0 AND 4
            AND state IS NOT NULL AND json_valid(state))
    ),
    UNIQUE (parent_loop_node_run_id, round_index)
);
CREATE INDEX workflow_execution_scopes_run ON workflow_execution_scopes(run_id);
CREATE UNIQUE INDEX workflow_loop_one_active_round
ON workflow_execution_scopes(parent_loop_node_run_id)
WHERE parent_loop_node_run_id IS NOT NULL AND status IN (0, 1);

CREATE TABLE workflow_run_root_scopes (
    run_id TEXT PRIMARY KEY REFERENCES workflow_runs(id),
    scope_id TEXT NOT NULL UNIQUE REFERENCES workflow_execution_scopes(id)
);

INSERT INTO workflow_execution_scopes (id, run_id, created_at, updated_at)
SELECT 'root:' || id, id, created_at, updated_at FROM workflow_runs;
INSERT INTO workflow_run_root_scopes (run_id, scope_id)
SELECT id, 'root:' || id FROM workflow_runs;

ALTER TABLE workflow_node_runs ADD COLUMN scope_id TEXT REFERENCES workflow_execution_scopes(id);
UPDATE workflow_node_runs SET scope_id = 'root:' || run_id;
CREATE UNIQUE INDEX workflow_node_runs_scope_node
ON workflow_node_runs(scope_id, node_id, COALESCE(iteration, -1)) WHERE is_deleted = 0;

CREATE TRIGGER workflow_scope_validate_parent
BEFORE INSERT ON workflow_execution_scopes
WHEN NEW.parent_loop_node_run_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM workflow_node_runs node
    JOIN workflow_execution_scopes parent ON parent.id = node.scope_id
    JOIN workflow_run_root_scopes current ON current.scope_id = parent.id
    JOIN workflow_runs run ON run.id = node.run_id
    WHERE node.id = NEW.parent_loop_node_run_id AND node.run_id = NEW.run_id
      AND node.node_type = 'loop' AND node.is_deleted = 0 AND node.status = 1
      AND parent.parent_loop_node_run_id IS NULL
      AND run.run_status = 1 AND run.is_deleted = 0
)
BEGIN
    SELECT RAISE(ABORT, 'Loop round requires an active parent in the current root scope');
END;

CREATE TRIGGER workflow_scope_identity_immutable
BEFORE UPDATE OF id, run_id, parent_loop_node_run_id, round_index ON workflow_execution_scopes
BEGIN
    SELECT RAISE(ABORT, 'workflow scope identity is immutable');
END;

CREATE TRIGGER workflow_root_scope_validate_insert
BEFORE INSERT ON workflow_run_root_scopes
WHEN NOT EXISTS (
    SELECT 1 FROM workflow_execution_scopes scope WHERE scope.id = NEW.scope_id
      AND scope.run_id = NEW.run_id AND scope.parent_loop_node_run_id IS NULL
)
BEGIN
    SELECT RAISE(ABORT, 'invalid workflow root scope');
END;
CREATE TRIGGER workflow_root_scope_validate_update
BEFORE UPDATE ON workflow_run_root_scopes
WHEN NEW.run_id != OLD.run_id OR NOT EXISTS (
    SELECT 1 FROM workflow_execution_scopes scope WHERE scope.id = NEW.scope_id
      AND scope.run_id = NEW.run_id AND scope.parent_loop_node_run_id IS NULL
)
BEGIN
    SELECT RAISE(ABORT, 'invalid workflow root scope');
END;

CREATE TRIGGER workflow_run_create_root_scope
AFTER INSERT ON workflow_runs
BEGIN
    INSERT INTO workflow_execution_scopes (id, run_id, created_at, updated_at)
    VALUES ('root:' || NEW.id, NEW.id, NEW.created_at, NEW.updated_at);
    INSERT INTO workflow_run_root_scopes (run_id, scope_id) VALUES (NEW.id, 'root:' || NEW.id);
END;

CREATE TRIGGER workflow_node_scope_validate_insert
BEFORE INSERT ON workflow_node_runs
WHEN NEW.scope_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM workflow_execution_scopes scope
    JOIN workflow_run_root_scopes current ON current.run_id = scope.run_id
    WHERE scope.id = NEW.scope_id AND scope.run_id = NEW.run_id
      AND (scope.id = current.scope_id OR (scope.status = 1 AND EXISTS (
        SELECT 1 FROM workflow_node_runs parent WHERE parent.id = scope.parent_loop_node_run_id
          AND parent.scope_id = current.scope_id AND parent.status = 1 AND parent.is_deleted = 0
      )))
)
BEGIN
    SELECT RAISE(ABORT, 'node scope belongs to another workflow run');
END;
CREATE TRIGGER workflow_node_scope_default
AFTER INSERT ON workflow_node_runs
WHEN NEW.scope_id IS NULL
BEGIN
    UPDATE workflow_node_runs
    SET scope_id = (SELECT scope_id FROM workflow_run_root_scopes WHERE run_id = NEW.run_id)
    WHERE id = NEW.id;
END;
CREATE TRIGGER workflow_node_scope_validate_update
BEFORE UPDATE OF scope_id, run_id ON workflow_node_runs
WHEN (OLD.scope_id IS NOT NULL AND (NEW.scope_id IS NOT OLD.scope_id OR NEW.run_id != OLD.run_id))
  OR NEW.scope_id IS NULL OR NOT EXISTS (
    SELECT 1 FROM workflow_execution_scopes WHERE id = NEW.scope_id AND run_id = NEW.run_id
)
BEGIN
    SELECT RAISE(ABORT, 'node scope is missing, foreign, or immutable');
END;
"#];

const DOWN_STATEMENTS: &[&str] = &[r#"
INSERT INTO workflow_scope_downgrade_archive (id, scope_id, run_id, record)
SELECT lower(hex(randomblob(16))), scope.id, scope.run_id, json_object(
    'scope', json_object('id', scope.id, 'runId', scope.run_id,
        'parentLoopNodeRunId', scope.parent_loop_node_run_id, 'roundIndex', scope.round_index,
        'status', scope.status, 'state', scope.state,
        'createdAt', scope.created_at, 'updatedAt', scope.updated_at),
    'nodes', json((SELECT json_group_array(json_object(
        'id', node.id, 'runId', node.run_id, 'nodeId', node.node_id, 'nodeType', node.node_type,
        'sessionId', node.session_id, 'status', node.status, 'input', node.input,
        'output', node.output, 'error', node.error, 'payload', node.payload,
        'startedAt', node.started_at, 'finishedAt', node.finished_at,
        'createdAt', node.created_at, 'updatedAt', node.updated_at, 'isDeleted', node.is_deleted
    )) FROM workflow_node_runs node WHERE node.scope_id = scope.id))
) FROM workflow_execution_scopes scope;

UPDATE workflow_node_runs SET status = 4,
    error = '{"reason":"loop_execution_downgraded"}',
    finished_at = COALESCE(finished_at, updated_at)
WHERE status IN (0, 1) AND run_id IN (
    SELECT run_id FROM workflow_node_runs WHERE node_type = 'loop' AND is_deleted = 0
);
UPDATE workflow_runs SET run_status = 3, state = '{"current_nodes":[]}',
    error = '{"reason":"loop_execution_downgraded"}',
    finished_at = COALESCE(finished_at, updated_at)
WHERE run_status IN (0, 1) AND id IN (
    SELECT run_id FROM workflow_node_runs WHERE node_type = 'loop' AND is_deleted = 0
);
UPDATE workflow_node_runs SET is_deleted = 1 WHERE scope_id IN (
    SELECT id FROM workflow_execution_scopes WHERE parent_loop_node_run_id IS NOT NULL
);

DROP TRIGGER workflow_node_scope_validate_update;
DROP TRIGGER workflow_node_scope_default;
DROP TRIGGER workflow_node_scope_validate_insert;
DROP TRIGGER workflow_run_create_root_scope;
DROP TRIGGER workflow_root_scope_validate_update;
DROP TRIGGER workflow_root_scope_validate_insert;
DROP TRIGGER workflow_scope_identity_immutable;
DROP TRIGGER workflow_scope_validate_parent;
DROP INDEX workflow_node_runs_scope_node;
ALTER TABLE workflow_node_runs DROP COLUMN scope_id;
DROP TABLE workflow_run_root_scopes;
DROP TABLE workflow_execution_scopes;
"#];

/// Gives repeated nodes durable scope identity and archives unsupported history during downgrade.
pub fn migration() -> Migration {
    Migration::new("0013", UP_STATEMENTS, DOWN_STATEMENTS)
}

#[cfg(test)]
mod tests;
