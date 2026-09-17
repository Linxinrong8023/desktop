use super::*;

/// Checks required structure separately from present-but-empty opaque values.
#[tokio::test]
async fn rejects_missing_and_empty_fields() -> Result<(), TestError> {
    fixtures::get_execution_status()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/operation_id",
                "/execution_id",
                "/payload/node_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node_id", "node_id"),
            ],
        )
        .await?;
    fixtures::event_ack()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node_id", "node_id"),
            ],
        )
        .await?;
    fixtures::execution_status()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/payload/state/result",
                "/payload/state/result/kind",
                "/payload/state/result/result",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/state/result/result/node/node_id",
                "/payload/state/result/result/node/incarnation_id",
                "/payload/state/result/result/workspace_id",
                "/payload/state/result/result/worktree_id",
                "/payload/state/result/result/facts/path",
                "/payload/state/result/result/facts/branch",
                "/payload/state/result/result/facts/base_commit",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/state/result/result/node/node_id", "node.node_id"),
                (
                    "/payload/state/result/result/node/incarnation_id",
                    "node.incarnation_id",
                ),
                ("/payload/state/result/result/workspace_id", "workspace_id"),
                ("/payload/state/result/result/worktree_id", "worktree_id"),
                ("/payload/state/result/result/facts/path", "facts.path"),
                ("/payload/state/result/result/facts/branch", "facts.branch"),
                (
                    "/payload/state/result/result/facts/base_commit",
                    "facts.base_commit",
                ),
            ],
        )
        .await?;
    fixtures::execution_status_unknown()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
            ],
        )
        .await?;
    fixtures::execution_status_accepted()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
            ],
        )
        .await?;
    fixtures::execution_status_running()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
            ],
        )
        .await?;
    fixtures::execution_status_failed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/payload/state/result",
                "/payload/state/result/kind",
                "/payload/state/result/result",
                "/payload/state/result/result/failure/code",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/state/result/result/node/node_id",
                "/payload/state/result/result/node/incarnation_id",
                "/payload/state/result/result/workspace_id",
                "/payload/state/result/result/worktree_id",
                "/payload/state/result/result/failure/message",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/state/result/result/node/node_id", "node.node_id"),
                (
                    "/payload/state/result/result/node/incarnation_id",
                    "node.incarnation_id",
                ),
                ("/payload/state/result/result/workspace_id", "workspace_id"),
                ("/payload/state/result/result/worktree_id", "worktree_id"),
                (
                    "/payload/state/result/result/failure/message",
                    "failure.message",
                ),
            ],
        )
        .await?;
    fixtures::execution_status_removed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/payload/state/result",
                "/payload/state/result/kind",
                "/payload/state/result/result",
                "/payload/state/result/result/outcome",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/state/result/result/node/node_id",
                "/payload/state/result/result/node/incarnation_id",
                "/payload/state/result/result/workspace_id",
                "/payload/state/result/result/worktree_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/state/result/result/node/node_id", "node.node_id"),
                (
                    "/payload/state/result/result/node/incarnation_id",
                    "node.incarnation_id",
                ),
                ("/payload/state/result/result/workspace_id", "workspace_id"),
                ("/payload/state/result/result/worktree_id", "worktree_id"),
            ],
        )
        .await?;
    fixtures::execution_status_removal_failed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/state",
                "/payload/state/state",
                "/payload/state/result",
                "/payload/state/result/kind",
                "/payload/state/result/result",
                "/payload/state/result/result/failure/code",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/state/result/result/node/node_id",
                "/payload/state/result/result/node/incarnation_id",
                "/payload/state/result/result/workspace_id",
                "/payload/state/result/result/worktree_id",
                "/payload/state/result/result/failure/message",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/state/result/result/node/node_id", "node.node_id"),
                (
                    "/payload/state/result/result/node/incarnation_id",
                    "node.incarnation_id",
                ),
                ("/payload/state/result/result/workspace_id", "workspace_id"),
                ("/payload/state/result/result/worktree_id", "worktree_id"),
                (
                    "/payload/state/result/result/failure/message",
                    "failure.message",
                ),
            ],
        )
        .await?;
    Ok(())
}
