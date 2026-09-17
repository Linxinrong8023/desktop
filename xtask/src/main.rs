use std::process::ExitCode;

/// Runs the requested xtask command from the workspace root.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Parses the xtask command line and dispatches to the matching workflow.
fn run() -> Result<(), String> {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "failed to determine workspace root".to_string())?;
    run_with_arguments(std::env::args().skip(1), workspace_root)
}

/// Accepts CLI inputs explicitly so dispatch can be tested without modifying the checkout.
fn run_with_arguments(
    mut arguments: impl Iterator<Item = String>,
    workspace_root: &std::path::Path,
) -> Result<(), String> {
    let Some(command) = arguments.next() else {
        return Err(
            "usage: cargo xtask <export-contracts|check-contracts|check-rust-size|report-rust-size|reconcile-migrations DATA_DIRECTORY>".to_string(),
        );
    };

    match command.as_str() {
        "check-rust-size" | "report-rust-size" => {
            if let Some(unexpected) = arguments.next() {
                return Err(format!("unexpected argument `{unexpected}`"));
            }
            let operation = if command == "check-rust-size" {
                xtask::check_rust_architecture
            } else {
                xtask::report_rust_architecture
            };
            operation(workspace_root)
                .map_err(|error| format!("Rust architecture check failed: {error}"))
        }
        "export-contracts" | "check-contracts" => {
            if let Some(unexpected) = arguments.next() {
                return Err(format!("unexpected argument `{unexpected}`"));
            }
            if command == "check-contracts" {
                xtask::check_export_contracts(workspace_root)
                    .map_err(|error| format!("failed to check contracts: {error}"))
            } else {
                xtask::run_export_contracts(workspace_root)
                    .map_err(|error| format!("failed to export contracts: {error}"))
            }
        }
        "reconcile-migrations" => {
            let data_directory = arguments.next().ok_or_else(|| {
                "usage: cargo xtask reconcile-migrations DATA_DIRECTORY".to_string()
            })?;
            if let Some(unexpected) = arguments.next() {
                return Err(format!("unexpected argument `{unexpected}`"));
            }
            xtask::run_reconcile_migrations(std::path::Path::new(&data_directory))
                .map_err(|error| format!("failed to reconcile migrations: {error}"))
        }
        _ => Err(format!("unknown xtask command `{command}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::run_with_arguments;
    use pretty_assertions::assert_eq;

    /// Missing generated artifacts must be reported as a check failure by the CLI.
    #[test]
    fn check_contracts_reports_check_failure() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;

        let error = run_with_arguments(
            ["check-contracts".to_string()].into_iter(),
            workspace.path(),
        )
        .err()
        .ok_or("missing artifacts must fail the check")?;

        assert_eq!(
            error.lines().next(),
            Some(
                "failed to check contracts: generated artifacts differ; run task export-contracts:"
            )
        );
        Ok(())
    }

    /// An unowned output collision must retain the export failure context and its cause.
    #[test]
    fn export_contracts_reports_export_failure() -> Result<(), Box<dyn std::error::Error>> {
        let workspace = tempfile::tempdir()?;
        let directory = workspace
            .path()
            .join("packages")
            .join("contracts")
            .join("src");
        std::fs::create_dir_all(&directory)?;
        let output = directory.join("endpoints.ts");
        std::fs::write(&output, "// user-owned file\n")?;

        assert_eq!(
            run_with_arguments(
                ["export-contracts".to_string()].into_iter(),
                workspace.path(),
            ),
            Err(format!(
                "failed to export contracts: generated output collides with an unowned file or link: {}",
                output.canonicalize()?.display()
            ))
        );
        Ok(())
    }
}
