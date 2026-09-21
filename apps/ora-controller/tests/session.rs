#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use ora_controller::*;
use ora_node_protocol::*;
use pretty_assertions::assert_eq;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::UnixListener, time::timeout};

/// A reachable socket is not sufficient authority to dispatch a previously accepted command.
#[test]
fn mismatched_node_or_missing_clone_capability_rejects_before_dispatch() {
    ora_logging::with_trace_logging(|| {
        for (node_id, capabilities) in [
            (
                NodeId::new("other-node"),
                vec![NodeCapability::RepositoryClone],
            ),
            (NodeId::new("node"), vec![NodeCapability::WorktreeExecution]),
        ] {
            let root = tempfile::Builder::new()
                .permissions(fs::Permissions::from_mode(/*mode*/ 0o700))
                .tempdir_in(std::env::var_os("HOME").unwrap())
                .unwrap();
            let mut controller =
                Controller::open(&root.path().join("controller"), ControllerId::new("owner"))
                    .unwrap();
            let command = controller
                .accept_clone(
                    RequestId::new("request"),
                    CloneExecutionSpec {
                        node_id: NodeId::new("node"),
                        repository: CloneRepositoryUrl::parse("https://example.com/repo").unwrap(),
                        branch: BranchName::new("main"),
                    },
                )
                .unwrap();
            fs::create_dir(root.path().join("node")).unwrap();
            let target = NodeEndpoint {
                node_id: NodeId::new("node"),
                endpoint: root.path().join("node").join("control.sock"),
            };
            let owner = Arc::new(Mutex::new(controller));
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let listener = UnixListener::bind(&target.endpoint).unwrap();
                    let settings = SessionConfig {
                        io_timeout_ms: 1000,
                        query_interval_ms: 20,
                    };
                    let peer = async {
                        let (mut stream, _) = listener.accept().await.unwrap();
                        assert!(matches!(
                            read_controller_message(&mut stream).await.unwrap(),
                            Some(ControllerToNodeMessage::Hello(_))
                        ));
                        write_node_message(
                            &mut stream,
                            &NodeToControllerMessage::HelloAccepted(HelloAcceptedMessage {
                                protocol_version: CURRENT_PROTOCOL_VERSION,
                                payload: HelloAccepted {
                                    selected_version: CURRENT_PROTOCOL_VERSION,
                                    node: NodeRuntimeIdentity {
                                        node_id,
                                        incarnation_id: NodeIncarnationId::new("current"),
                                    },
                                    capabilities,
                                },
                            }),
                        )
                        .await
                        .unwrap();
                        // EOF, not a status query or clone command, proves rejection preceded dispatch.
                        assert_eq!(
                            timeout(
                                Duration::from_secs(/*secs*/ 2),
                                read_controller_message(&mut stream)
                            )
                            .await
                            .unwrap()
                            .unwrap(),
                            None
                        );
                    };
                    let (result, ()) = tokio::join!(run_session(&owner, &target, &settings), peer);
                    assert!(result.is_err());
                });
            let owner = owner.lock().unwrap();
            assert_eq!(
                owner.commands(&target.node_id).unwrap(),
                vec![command.clone()]
            );
            assert_eq!(owner.result(&command.execution_id).unwrap(), None);
        }
    });
}

/// Repeated Unknown replies retain responsibility without an unbounded immediate retransmission loop.
#[test]
fn uncertain_execution_retransmits_at_most_once_per_connection() {
    ora_logging::with_trace_logging(|| {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(/*mode*/ 0o700))
            .tempdir_in(std::env::var_os("HOME").unwrap())
            .unwrap();
        let mut controller =
            Controller::open(&root.path().join("controller"), ControllerId::new("owner")).unwrap();
        let command = controller
            .accept_clone(
                RequestId::new("request"),
                CloneExecutionSpec {
                    node_id: NodeId::new("node"),
                    repository: CloneRepositoryUrl::parse("https://example.com/repo").unwrap(),
                    branch: BranchName::new("main"),
                },
            )
            .unwrap();
        fs::create_dir(root.path().join("node")).unwrap();
        let endpoint = NodeEndpoint {
            node_id: NodeId::new("node"),
            endpoint: root.path().join("node").join("control.sock"),
        };
        let owner = Arc::new(Mutex::new(controller));
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let listener = UnixListener::bind(&endpoint.endpoint).unwrap();
            let settings = SessionConfig { io_timeout_ms: 1000, query_interval_ms: 20 };
            let session = run_session(&owner, &endpoint, &settings);
            let peer = async {
                let (mut stream, _) = listener.accept().await.unwrap();
                assert!(matches!(read_controller_message(&mut stream).await.unwrap(), Some(ControllerToNodeMessage::Hello(_))));
                let identity = NodeRuntimeIdentity { node_id: endpoint.node_id.clone(), incarnation_id: NodeIncarnationId::new("current") };
                write_node_message(&mut stream, &NodeToControllerMessage::HelloAccepted(HelloAcceptedMessage { protocol_version: CURRENT_PROTOCOL_VERSION, payload: HelloAccepted { selected_version: CURRENT_PROTOCOL_VERSION, node: identity.clone(), capabilities: vec![NodeCapability::RepositoryClone] } })).await.unwrap();
                let mut retries = 0;
                let mut queries = 0;
                while queries < 4 {
                    let message = timeout(Duration::from_secs(/*secs*/ 2), read_controller_message(&mut stream)).await.unwrap().unwrap().unwrap();
                    match message {
                        ControllerToNodeMessage::CloneRepository(retry) => { assert_eq!(retry, command); retries += 1; assert!(retries <= 1); }
                        ControllerToNodeMessage::GetExecutionStatus(_) => { queries += 1; }
                        message => panic!("unexpected {message:?}"),
                    }
                    write_node_message(&mut stream, &NodeToControllerMessage::ExecutionStatus(ExecutionStatusMessage { protocol_version: CURRENT_PROTOCOL_VERSION, operation_id: command.operation_id.clone(), execution_id: command.execution_id.clone(), payload: ExecutionStatus { node: identity.clone(), state: ExecutionState::Unknown } })).await.unwrap();
                }
                assert_eq!(retries, 1);
            };
            tokio::select! { _ = session => panic!("session ended before peer checks"), _ = peer => {} }
        });
        assert_eq!(
            owner.lock().unwrap().result(&command.execution_id).unwrap(),
            None
        );
    });
}
