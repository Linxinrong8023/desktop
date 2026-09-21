use super::Migration;

const UP_STATEMENTS: &[&str] = &[r#"
CREATE TABLE pack_installation (
    pack_id    TEXT PRIMARY KEY NOT NULL,
    source_url TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE pack_installation_member (
    pack_id            TEXT NOT NULL REFERENCES pack_installation(pack_id) ON DELETE CASCADE,
    member_id          TEXT NOT NULL,
    version_at_install TEXT NOT NULL,
    ownership          TEXT NOT NULL CHECK (ownership IN ('managed_by_pack', 'pre_existing')),
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL,
    PRIMARY KEY (pack_id, member_id)
);
"#];

const DOWN_STATEMENTS: &[&str] = &[r#"
DROP TABLE IF EXISTS pack_installation_member;
DROP TABLE IF EXISTS pack_installation;
"#];

/// Persists which members one pack installation created versus found already installed
/// (extension-pack decision "pack installation ownership": ownership records the install
/// action, not membership alone).
///
/// Originally shipped as `0013` in the extension-pack PR; renumbered to `0014` because
/// upstream's workflow-loop support landed `0013` first, and versioned snapshots are
/// append-only once merged.
pub fn migration() -> Migration {
    Migration::new("0014", UP_STATEMENTS, DOWN_STATEMENTS)
}

#[cfg(test)]
mod tests {
    use super::{DOWN_STATEMENTS, UP_STATEMENTS};
    use pretty_assertions::assert_eq;
    use rusqlite::Connection;

    /// A fresh database accepts pack and member rows, and the down migration removes both
    /// tables together with their rows.
    #[test]
    fn creates_the_pack_installation_tables_and_rolls_back() {
        let connection = Connection::open_in_memory().unwrap();

        for statement in UP_STATEMENTS {
            connection.execute_batch(statement).unwrap();
        }
        connection
            .execute(
                "INSERT INTO pack_installation (pack_id, source_url, created_at, updated_at)
                 VALUES ('official/ora-space.pack', 'https://github.com/ora-space/marketplace', 1, 1)",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO pack_installation_member (
                     pack_id, member_id, version_at_install, ownership, created_at, updated_at
                 ) VALUES (
                     'official/ora-space.pack', 'official/ora-space.member',
                     '1.0.0', 'managed_by_pack', 1, 1
                 )",
                [],
            )
            .unwrap();

        let members: Vec<String> = connection
            .prepare("SELECT member_id FROM pack_installation_member")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(members, vec!["official/ora-space.member"]);

        for statement in DOWN_STATEMENTS {
            connection.execute_batch(statement).unwrap();
        }
        let tables: Vec<String> = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'pack_installation%'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(tables, Vec::<String>::new());
    }
}
