use std::path::Path;

use ora_db::{DatabaseLocation, default_migration_catalog, reconcile_migration_history};

/// Reconciles the development database against current migration SQL before Desktop starts.
pub fn run_reconcile_migrations(data_directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(data_directory)?;
    let database_path = data_directory.join("ora.sqlite3");
    let catalog = default_migration_catalog()?;
    reconcile_migration_history(&DatabaseLocation::path(database_path), &catalog)?;
    Ok(())
}
