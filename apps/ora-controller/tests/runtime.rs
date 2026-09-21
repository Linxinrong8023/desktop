#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use ora_controller::*;
use ora_node_protocol::*;
use pretty_assertions::assert_eq;
use std::{fs, os::unix::fs::PermissionsExt};

/// The embedding interface preserves durable intent across runtime shutdown and excludes duplicate owners.
#[test]
fn embedded_owner_reopens_original_operations_and_rejects_overlap() {
    ora_logging::with_trace_logging(|| {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(/*mode*/ 0o700))
            .tempdir_in(std::env::var_os("HOME").unwrap())
            .unwrap();
        let config = RuntimeConfig {
            home_directory: root.path().join("controller"),
            protected_state_directories: vec![root.path().join("process")],
            controller_id: ControllerId::new("owner"),
            nodes: vec![NodeEndpoint {
                node_id: NodeId::new("node"),
                endpoint: root.path().join("node").join("control.sock"),
            }],
            session: SessionConfig {
                io_timeout_ms: 100,
                query_interval_ms: 10,
            },
            reconnect_ms: 10,
            timezone: "Asia/Shanghai".into(),
        };
        let mut overlap = config.clone();
        overlap.home_directory = root.path().join("process").join("nested");
        assert!(ControllerRuntime::open(overlap).is_err());
        assert!(!root.path().join("process").exists());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let runtime = ControllerRuntime::open(config.clone()).unwrap();
                assert!(matches!(
                    ControllerRuntime::open(config.clone()),
                    Err(Error::AlreadyRunning)
                ));
                let handle = runtime.handle();
                let spec = CloneExecutionSpec {
                    node_id: NodeId::new("node"),
                    repository: CloneRepositoryUrl::parse("https://example.com/repo.git").unwrap(),
                    branch: BranchName::new("main"),
                };
                let command = handle
                    .accept_clone(RequestId::new("request"), spec.clone())
                    .await
                    .unwrap();
                assert_eq!(
                    handle
                        .accept_clone(RequestId::new("request"), spec)
                        .await
                        .unwrap(),
                    command
                );
                let expected = CloneOperation {
                    command: command.clone(),
                    result: None,
                };
                assert_eq!(handle.operations().await.unwrap(), vec![expected.clone()]);
                assert_eq!(
                    handle.operation(ExecutionId::new("absent")).await.unwrap(),
                    None
                );
                runtime.run(async {}).await.unwrap();
                drop(handle);
                drop(runtime);
                let replacement = ControllerRuntime::open(config).unwrap();
                assert_eq!(
                    replacement
                        .handle()
                        .operation(command.execution_id)
                        .await
                        .unwrap(),
                    Some(expected)
                );
            });
    });
}
