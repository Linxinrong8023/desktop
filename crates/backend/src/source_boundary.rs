//! Guards the generic Session Runtime boundary.
//!
//! `agent_runtime` must not reference Workflow domain symbols or backend modules, so it can be
//! replaced by a future Session plugin without stripping Workflow logic out of it. The guard below
//! is a regression test, not a lint framework: it scans the module's production sources for the
//! forbidden symbols and fails CI when any leak in.

#[cfg(test)]
mod tests {
    /// Camel-case Workflow type names and `crate::workflow` module paths that mark domain/module
    /// coupling forbidden inside the generic Session Runtime. Prose that says "workflow" does not
    /// match these, so comments and README text stay unaffected.
    const FORBIDDEN: &[&str] = &[
        "crate::workflow",
        "WorkflowRun",
        "WorkflowNodeRun",
        "WorkflowNodeStatus",
    ];

    /// Fails if any `agent_runtime` source references a Workflow symbol, enforcing the boundary
    /// that keeps the generic Session Runtime replaceable by a future Session plugin.
    #[test]
    fn agent_runtime_does_not_reference_workflow_symbols() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("agent_runtime");
        let mut offenders = Vec::new();
        visit(&root, &mut offenders);
        assert!(
            offenders.is_empty(),
            "agent_runtime must not reference Workflow symbols, found:\n{}",
            offenders.join("\n")
        );
    }

    /// Recursively collects source files that contain a forbidden symbol.
    fn visit(dir: &std::path::Path, offenders: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read agent_runtime directory") {
            let entry = entry.expect("read directory entry");
            let path = entry.path();
            if path.is_dir() {
                visit(&path, offenders);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                let source = std::fs::read_to_string(&path).expect("read source file");
                for forbidden in FORBIDDEN {
                    if source.contains(forbidden) {
                        offenders.push(format!("{} references `{forbidden}`", path.display()));
                    }
                }
            }
        }
    }
}
