use super::support::*;
use ora_node_protocol::*;
use serde_json::json;

#[path = "session/fixtures.rs"]
mod fixtures;

/// Returns the Node identity attached to every result fixture.
pub(super) fn node() -> NodeRuntimeIdentity {
    NodeRuntimeIdentity {
        node_id: NodeId::new("node-1"),
        incarnation_id: NodeIncarnationId::new("incarnation-1"),
    }
}

/// Rejects inconsistent handshake declarations through both public codec paths.
#[tokio::test]
async fn rejects_inconsistent_handshakes() -> Result<(), TestError> {
    for (case, path, value, expected) in [
        (
            fixtures::hello(),
            "/payload/supported_versions",
            json!([]),
            MessageValidationError::NoSupportedProtocolVersions,
        ),
        (
            fixtures::hello(),
            "/payload/supported_versions",
            json!([1, 1]),
            MessageValidationError::DuplicateProtocolVersion { version: 1 },
        ),
        (
            fixtures::hello(),
            "/payload/supported_versions",
            json!([2]),
            MessageValidationError::EnvelopeVersionNotAdvertised { version: 1 },
        ),
        (
            fixtures::hello_accepted(),
            "/payload/selected_version",
            json!(2),
            MessageValidationError::SelectedVersionMismatch {
                selected: 2,
                envelope: 1,
            },
        ),
        (
            fixtures::hello_accepted(),
            "/payload/capabilities",
            json!([]),
            MessageValidationError::WorktreeCapabilityMissing,
        ),
        (
            fixtures::hello_accepted(),
            "/payload/capabilities",
            json!(["worktree_execution", "worktree_execution"]),
            MessageValidationError::DuplicateCapability,
        ),
    ] {
        let peer = case.peer();
        let mut wire = case.wire;
        receive(peer, &wire).await?;
        replace(&mut wire, path, value);
        reject_semantics(peer, &wire, path, expected).await?;
    }
    Ok(())
}

/// Proves fragmented I/O preserves every legal message in this business slice.
#[tokio::test]
async fn round_trips_messages() -> Result<(), TestError> {
    for case in fixtures::cases() {
        case.assert_round_trip().await?;
    }
    Ok(())
}

/// Locks independent wire shapes, optional output fields, and extension acceptance.
#[tokio::test]
async fn preserves_wire_shapes() -> Result<(), TestError> {
    for case in fixtures::cases() {
        case.assert_wire().await?;
        case.assert_extensions().await?;
    }
    Ok(())
}

/// Applies version and direction constraints to every legal shape in this slice.
#[tokio::test]
async fn rejects_versions_directions_and_payloads() -> Result<(), TestError> {
    for case in fixtures::cases() {
        case.assert_envelope_rejections().await?;
    }
    Ok(())
}

/// Checks required structure separately from present-but-empty opaque values.
#[tokio::test]
async fn rejects_missing_and_empty_fields() -> Result<(), TestError> {
    fixtures::hello()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/supported_versions",
                "/payload/controller_id",
            ],
            &[("/payload/controller_id", "controller_id")],
        )
        .await?;
    fixtures::hello_accepted()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/selected_version",
                "/payload/capabilities",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
            ],
            &[
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
            ],
        )
        .await?;
    fixtures::heartbeat()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
            ],
            &[
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
            ],
        )
        .await?;
    Ok(())
}
