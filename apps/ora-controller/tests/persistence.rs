#![allow(clippy::unwrap_used)]
use ora_controller::*;
use ora_node_protocol::*;
use pretty_assertions::assert_eq;
use std::{cell::Cell, rc::Rc};

struct Fault(Rc<Cell<Option<WritePoint>>>);
impl WriteGuard for Fault {
    /// Refuses one transaction boundary while preserving the production database behavior.
    fn before_write(&self, point: WritePoint) -> Result<(), Error> {
        if self.0.get() == Some(point) {
            Err(Error::Injected)
        } else {
            Ok(())
        }
    }
}

/// Uses an owner-private deployment location rather than weakening production path policy for tests.
fn directory() -> tempfile::TempDir {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(/*mode*/ 0o700))
            .tempdir_in(std::env::var_os("HOME").unwrap())
            .unwrap()
    }
    #[cfg(not(unix))]
    {
        tempfile::tempdir().unwrap()
    }
}

/// Supplies immutable business intent, never a local checkout path or credential.
fn spec() -> CloneExecutionSpec {
    CloneExecutionSpec {
        node_id: NodeId::new("node"),
        repository: CloneRepositoryUrl::parse("https://example.com/repo.git").unwrap(),
        branch: BranchName::new("main"),
    }
}

/// Builds retained historical facts with a different incarnation from the current session.
fn event(command: &CloneRepositoryMessage) -> CloneResultMessage {
    CloneResultMessage {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        request_id: command.request_id.clone(),
        operation_id: command.operation_id.clone(),
        execution_id: command.execution_id.clone(),
        sequence: Sequence::new(/*value*/ 1),
        payload: CloneExecutionResult::CloneReady(CloneReady {
            node: NodeRuntimeIdentity {
                node_id: NodeId::new("node"),
                incarnation_id: NodeIncarnationId::new("historical"),
            },
            spec: command.payload.spec.clone(),
            repository_id: RepositoryId::new("repository"),
            path: NodePath::new("/node/checkout"),
            commit: CommitId::new("0123456789abcdef0123456789abcdef01234567"),
        }),
    }
}

/// Acceptance survives restart, rejects changed input and excludes a second live database owner.
#[test]
fn acceptance_is_durable_idempotent_and_exclusive() {
    let directory = directory();
    let mut owner = Controller::open(directory.path(), ControllerId::new("owner")).unwrap();
    let accepted = owner
        .accept_clone(RequestId::new("request"), spec())
        .unwrap();
    assert!(matches!(
        Controller::open(directory.path(), ControllerId::new("owner")),
        Err(Error::AlreadyRunning)
    ));
    assert_eq!(
        owner
            .accept_clone(RequestId::new("request"), spec())
            .unwrap(),
        accepted
    );
    let mut changed = spec();
    changed.branch = BranchName::new("different");
    assert!(matches!(
        owner.accept_clone(RequestId::new("request"), changed),
        Err(Error::Conflict)
    ));
    drop(owner);
    let owner = Controller::open(directory.path(), ControllerId::new("owner")).unwrap();
    assert_eq!(
        owner.commands(&NodeId::new("node")).unwrap(),
        vec![accepted]
    );
    drop(owner);
    assert!(matches!(
        Controller::open(directory.path(), ControllerId::new("other")),
        Err(Error::InvalidStorage)
    ));
}

/// Query/event order and lost Ack share one immutable result; receipt failure rolls back first takeover.
#[test]
fn takeover_is_atomic_in_both_delivery_orders_and_conflicts_never_ack() {
    for query_first in [true, false] {
        let directory = directory();
        let fault = Rc::new(Cell::new(None));
        let mut owner = Controller::open_with_guard(
            directory.path(),
            ControllerId::new("owner"),
            Fault(fault.clone()),
        )
        .unwrap();
        let command = owner
            .accept_clone(RequestId::new("request"), spec())
            .unwrap();
        let event = event(&command);
        let session = NodeRuntimeIdentity {
            node_id: NodeId::new("node"),
            incarnation_id: NodeIncarnationId::new("current"),
        };
        let query = NodeToControllerMessage::ExecutionStatus(ExecutionStatusMessage {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            operation_id: command.operation_id.clone(),
            execution_id: command.execution_id.clone(),
            payload: ExecutionStatus {
                node: session.clone(),
                state: ExecutionState::Completed(ExecutionResult::Clone(event.payload.clone())),
            },
        });
        fault.set(Some(WritePoint::Receipt));
        assert!(matches!(
            owner.take_over(
                &session,
                &NodeToControllerMessage::CloneResult(event.clone())
            ),
            Err(Error::Injected)
        ));
        assert_eq!(owner.result(&command.execution_id).unwrap(), None);
        fault.set(None);
        if query_first {
            assert_eq!(owner.take_over(&session, &query).unwrap(), None);
        }
        let expected = Some(EventAckMessage {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            operation_id: command.operation_id.clone(),
            execution_id: command.execution_id.clone(),
            sequence: event.sequence,
            payload: EventAck {
                node_id: session.node_id.clone(),
            },
        });
        assert_eq!(
            owner
                .take_over(
                    &session,
                    &NodeToControllerMessage::CloneResult(event.clone())
                )
                .unwrap(),
            expected
        );
        assert_eq!(owner.take_over(&session, &query).unwrap(), None);
        drop(owner);
        let mut owner = Controller::open(directory.path(), ControllerId::new("owner")).unwrap();
        assert_eq!(
            owner
                .take_over(
                    &session,
                    &NodeToControllerMessage::CloneResult(event.clone())
                )
                .unwrap(),
            expected
        );
        let mut wrong = event.clone();
        wrong.request_id = Some(RequestId::new("other"));
        assert!(matches!(
            owner.take_over(&session, &NodeToControllerMessage::CloneResult(wrong)),
            Err(Error::Conflict)
        ));
        let mut wrong = event.clone();
        if let CloneExecutionResult::CloneReady(ready) = &mut wrong.payload {
            ready.commit = CommitId::new("1123456789abcdef0123456789abcdef01234567");
        }
        assert!(matches!(
            owner.take_over(&session, &NodeToControllerMessage::CloneResult(wrong)),
            Err(Error::Conflict)
        ));
        assert_eq!(
            owner.result(&command.execution_id).unwrap(),
            Some(event.payload)
        );
        let inspect =
            rusqlite::Connection::open(directory.path().join("ora-controller.sqlite3")).unwrap();
        assert_eq!(
            inspect
                .query_row("SELECT count(*) FROM clone_receipts", [], |r| r
                    .get::<_, i64>(/*idx*/ 0))
                .unwrap(),
            1
        );
    }
}

/// Persistence failure cannot manufacture accepted intent, and foreign files are not reinitialized.
#[test]
fn failed_acceptance_and_unknown_files_remain_untouched() {
    let directory = directory();
    let mut owner = Controller::open_with_guard(
        directory.path(),
        ControllerId::new("owner"),
        Fault(Rc::new(Cell::new(Some(WritePoint::Accept)))),
    )
    .unwrap();
    assert!(matches!(
        owner.accept_clone(RequestId::new("request"), spec()),
        Err(Error::Injected)
    ));
    assert_eq!(owner.commands(&NodeId::new("node")).unwrap(), vec![]);
    drop(owner);
    let foreign = directory.path().join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&foreign, std::fs::Permissions::from_mode(/*mode*/ 0o700))
            .unwrap();
    }
    let path = foreign.join("ora-controller.sqlite3");
    std::fs::write(&path, "user content").unwrap();
    assert!(Controller::open(&foreign, ControllerId::new("owner")).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "user content");
}
