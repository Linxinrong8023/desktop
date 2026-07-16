use super::Migration;

const UP_STATEMENTS: &[&str] = &[r#"
CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);
"#];

const DOWN_STATEMENTS: &[&str] = &[r#"
DROP TABLE IF EXISTS agents;
DROP TABLE IF EXISTS skills;
"#];

/// Builds the skill and configurable-agent catalog migration.
pub fn migration() -> Migration {
    Migration::new("0003", UP_STATEMENTS, DOWN_STATEMENTS)
}
