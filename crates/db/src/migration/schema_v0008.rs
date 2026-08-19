use super::Migration;

const UP_STATEMENTS: &[&str] = &[r#"
CREATE TABLE user_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#];

const DOWN_STATEMENTS: &[&str] = &[r#"
DROP TABLE user_config;
"#];

/// Adds the shared key-value store for non-sensitive user preferences.
pub fn migration() -> Migration {
    Migration::new("0008", UP_STATEMENTS, DOWN_STATEMENTS)
}
