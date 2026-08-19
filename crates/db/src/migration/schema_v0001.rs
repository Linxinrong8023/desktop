use super::Migration;

const UP_STATEMENTS: &[&str] = &[r#"
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root_path TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    title TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    worktree_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS worktrees (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    branch_name TEXT,
    base_commit_id TEXT CHECK (base_commit_id IS NULL OR base_commit_id <> ''),
    checkout_root TEXT,
    is_active INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    title TEXT,
    agent_cli TEXT NOT NULL,
    agent_session_id TEXT NOT NULL,
    history_degraded_reason TEXT,
    status INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS migrations (
    version TEXT PRIMARY KEY,
    executed_at INTEGER NOT NULL
);
"#];

const DOWN_STATEMENTS: &[&str] = &[r#"
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS worktrees;
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS projects;
"#];

/// Builds the initial migration and seeds migration bookkeeping.
pub fn migration() -> Migration {
    Migration::new("0001", UP_STATEMENTS, DOWN_STATEMENTS)
}
