#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]
use ora_contracts::minicloud::*;
use ora_controller::{NodeEndpoint, RuntimeConfig, SessionConfig};
use ora_minicloud_server::{Server, ServerConfig};
use ora_node_protocol::{BranchName, CloneExecutionSpec, CloneRepositoryUrl, ControllerId, NodeId};
use pretty_assertions::assert_eq;
use std::{fs, os::unix::fs::PermissionsExt};

/// Real HTTP acceptance, conflict and restart use the same durable owner, even when Node is offline.
#[test]
fn http_acceptance_is_idempotent_and_survives_server_restart() {
    ora_logging::with_trace_logging(|| {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(/*mode*/ 0o700))
            .tempdir_in(std::env::var_os("HOME").unwrap())
            .unwrap();
        let config = ServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            node_id: NodeId::new("node"),
            controller: RuntimeConfig {
                home_directory: root.path().join("controller"),
                protected_state_directories: vec![root.path().join("process")],
                controller_id: ControllerId::new("owner"),
                nodes: vec![NodeEndpoint {
                    node_id: NodeId::new("node"),
                    endpoint: root.path().join("node").join("control.sock"),
                }],
                session: SessionConfig {
                    io_timeout_ms: 100,
                    query_interval_ms: 50,
                },
                reconnect_ms: 50,
                timezone: "Asia/Shanghai".into(),
            },
        };
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut exposed = config.clone();
                exposed.listen = "0.0.0.0:0".parse().unwrap();
                assert!(Server::bind(exposed).await.is_err());
                assert!(!config.controller.home_directory.exists());
                let mut overlap = config.clone();
                overlap.controller.home_directory = root.path().join("process").join("nested");
                assert!(Server::bind(overlap).await.is_err());
                assert!(!root.path().join("process").exists());
                fs::create_dir(&config.controller.home_directory).unwrap();
                fs::set_permissions(
                    &config.controller.home_directory,
                    fs::Permissions::from_mode(/*mode*/ 0o700),
                )
                .unwrap();
                let unknown = config
                    .controller
                    .home_directory
                    .join("ora-controller.sqlite3");
                fs::write(&unknown, b"user-owned unknown file").unwrap();
                assert!(Server::bind(config.clone()).await.is_err());
                assert_eq!(fs::read(&unknown).unwrap(), b"user-owned unknown file");
                // Retain the rejected fixture file; the legitimate owner starts in a fresh root.
                let mut config = config;
                config.controller.home_directory = root.path().join("valid-controller");
                let mut standalone = ora_controller::Controller::open(
                    &config.controller.home_directory,
                    config.controller.controller_id.clone(),
                )
                .unwrap();
                let original = standalone
                    .accept_clone(
                        ora_node_protocol::RequestId::new("original"),
                        CloneExecutionSpec {
                            node_id: config.node_id.clone(),
                            repository: CloneRepositoryUrl::parse("https://example.com/repo.git")
                                .unwrap(),
                            branch: BranchName::new("main"),
                        },
                    )
                    .unwrap();
                assert!(Server::bind(config.clone()).await.is_err());
                drop(standalone);
                let server = Server::bind(config.clone()).await.unwrap();
                assert!(Server::bind(config.clone()).await.is_err());
                let base = format!("http://{}/api/clones", server.local_addr().unwrap());
                let (stop, stopped) = tokio::sync::oneshot::channel();
                let task = tokio::spawn(server.run(async {
                    let _ = stopped.await;
                }));
                let client = reqwest::Client::new();
                let input = MiniCloneRequest {
                    request_id: "original".into(),
                    repository: "https://example.com/repo.git".into(),
                    branch: "main".into(),
                };
                let response = client.post(&base).json(&input).send().await.unwrap();
                assert_eq!(response.status().as_u16(), 202);
                let accepted: MiniCloneAccepted = response.json().await.unwrap();
                assert_eq!(
                    accepted,
                    MiniCloneAccepted {
                        request_id: "original".into(),
                        operation_id: original.operation_id.as_str().into(),
                        execution_id: original.execution_id.as_str().into(),
                    }
                );
                assert_eq!(
                    client
                        .post(&base)
                        .json(&input)
                        .send()
                        .await
                        .unwrap()
                        .json::<MiniCloneAccepted>()
                        .await
                        .unwrap(),
                    accepted
                );
                let mut changed = input.clone();
                changed.branch = "other".into();
                assert_eq!(
                    client
                        .post(&base)
                        .json(&changed)
                        .send()
                        .await
                        .unwrap()
                        .status()
                        .as_u16(),
                    409
                );
                changed.request_id = "bad".into();
                changed.repository = "file:///tmp/repository".into();
                assert_eq!(
                    client
                        .post(&base)
                        .json(&changed)
                        .send()
                        .await
                        .unwrap()
                        .status()
                        .as_u16(),
                    400
                );
                assert_eq!(
                    client
                        .get(format!("{base}/missing"))
                        .send()
                        .await
                        .unwrap()
                        .status()
                        .as_u16(),
                    404
                );
                let expected = MiniCloneOperation {
                    operation_id: accepted.operation_id,
                    execution_id: accepted.execution_id.clone(),
                    node_id: "node".into(),
                    repository: input.repository,
                    branch: input.branch,
                    state: MiniCloneState::Pending,
                };
                assert_eq!(
                    client
                        .get(&base)
                        .send()
                        .await
                        .unwrap()
                        .json::<Vec<MiniCloneOperation>>()
                        .await
                        .unwrap(),
                    vec![expected.clone()]
                );
                stop.send(()).unwrap();
                task.await.unwrap().unwrap();
                let server = Server::bind(config).await.unwrap();
                let base = format!("http://{}/api/clones", server.local_addr().unwrap());
                let (stop, stopped) = tokio::sync::oneshot::channel();
                let task = tokio::spawn(server.run(async {
                    let _ = stopped.await;
                }));
                assert_eq!(
                    client
                        .get(format!("{base}/{}", accepted.execution_id))
                        .send()
                        .await
                        .unwrap()
                        .json::<MiniCloneOperation>()
                        .await
                        .unwrap(),
                    expected
                );
                stop.send(()).unwrap();
                task.await.unwrap().unwrap();
            });
    });
}
