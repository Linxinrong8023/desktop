//! Adopts the unpublished Loop branch's colliding migration without rolling back user history.

use rusqlite::{Connection, OptionalExtension, params};

use crate::{DatabaseError, MigrationCatalog, TimestampSource};

/// Recognizes the exact old Loop snapshot, then installs upstream migrations and renumbers it.
pub(super) fn adopt_legacy_loop<T: TimestampSource>(
    connection: &mut Connection,
    catalog: &MigrationCatalog,
    clock: &T,
) -> Result<(), DatabaseError> {
    if !catalog.target_versions().contains(&"0013") {
        return Ok(());
    }
    let Some(scopes) = catalog.migration("0013") else {
        return Ok(());
    };
    let Some(mcp) = catalog.migration("0011") else {
        return Ok(());
    };
    let Some(iteration) = catalog.migration("0012") else {
        return Ok(());
    };
    let legacy_sql = scopes.up_sql().replace(
        "ON workflow_node_runs(scope_id, node_id, COALESCE(iteration, -1))",
        "ON workflow_node_runs(scope_id, node_id)",
    );
    let applied: Option<(String, String)> = connection
        .query_row(
            "SELECT up_sql, down_sql FROM migrations WHERE version = '0011'
         AND NOT EXISTS (SELECT 1 FROM migrations WHERE version > '0011')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if applied != Some((legacy_sql, scopes.down_sql())) {
        return Ok(());
    }

    // One transaction keeps both schema and history intact if any upgrade step fails.
    let transaction = connection.transaction()?;
    for migration in [mcp, iteration] {
        transaction.execute_batch(&migration.up_sql())?;
    }
    transaction.execute_batch(
        "DROP INDEX workflow_node_runs_scope_node;
         CREATE UNIQUE INDEX workflow_node_runs_scope_node
         ON workflow_node_runs(scope_id, node_id, COALESCE(iteration, -1)) WHERE is_deleted = 0;",
    )?;
    transaction.execute(
        "UPDATE migrations SET version = '0013', up_sql = ?1, down_sql = ?2 WHERE version = '0011'",
        params![scopes.up_sql(), scopes.down_sql()],
    )?;
    for migration in [mcp, iteration] {
        transaction.execute(
            "INSERT INTO migrations (version, up_sql, down_sql, executed_at) VALUES (?1, ?2, ?3, ?4)",
            params![migration.version(), migration.up_sql(), migration.down_sql(), clock.current_timestamp_millis()],
        )?;
    }
    transaction.commit()?;
    Ok(())
}
