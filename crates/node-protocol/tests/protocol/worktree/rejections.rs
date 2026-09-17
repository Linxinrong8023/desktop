use super::*;

/// Checks required structure separately from present-but-empty opaque values.
#[tokio::test]
async fn rejects_missing_and_empty_fields() -> Result<(), TestError> {
    fixtures::ensure_worktree()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/spec",
                "/payload/spec/path_policy/kind",
                "/operation_id",
                "/execution_id",
                "/payload/spec/node_id",
                "/payload/spec/workspace_id",
                "/payload/spec/worktree_id",
                "/payload/spec/repository",
                "/payload/spec/main_workspace/workspace_id",
                "/payload/spec/main_workspace/path",
                "/payload/spec/base_ref",
                "/payload/spec/expected_branch",
                "/payload/spec/path_policy/directory_name",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/spec/node_id", "node_id"),
                ("/payload/spec/workspace_id", "workspace_id"),
                ("/payload/spec/worktree_id", "worktree_id"),
                ("/payload/spec/repository", "repository"),
                (
                    "/payload/spec/main_workspace/workspace_id",
                    "main_workspace.workspace_id",
                ),
                ("/payload/spec/main_workspace/path", "main_workspace.path"),
                ("/payload/spec/base_ref", "base_ref"),
                ("/payload/spec/expected_branch", "expected_branch"),
                (
                    "/payload/spec/path_policy/directory_name",
                    "path_policy.directory_name",
                ),
            ],
        )
        .await?;
    fixtures::ensure_worktree_without_request()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/spec",
                "/payload/spec/path_policy/kind",
                "/operation_id",
                "/execution_id",
                "/payload/spec/node_id",
                "/payload/spec/workspace_id",
                "/payload/spec/worktree_id",
                "/payload/spec/repository",
                "/payload/spec/main_workspace/workspace_id",
                "/payload/spec/main_workspace/path",
                "/payload/spec/base_ref",
                "/payload/spec/expected_branch",
                "/payload/spec/path_policy/directory_name",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/spec/node_id", "node_id"),
                ("/payload/spec/workspace_id", "workspace_id"),
                ("/payload/spec/worktree_id", "worktree_id"),
                ("/payload/spec/repository", "repository"),
                (
                    "/payload/spec/main_workspace/workspace_id",
                    "main_workspace.workspace_id",
                ),
                ("/payload/spec/main_workspace/path", "main_workspace.path"),
                ("/payload/spec/base_ref", "base_ref"),
                ("/payload/spec/expected_branch", "expected_branch"),
                (
                    "/payload/spec/path_policy/directory_name",
                    "path_policy.directory_name",
                ),
            ],
        )
        .await?;
    fixtures::remove_worktree()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/spec",
                "/payload/spec/path_policy/kind",
                "/operation_id",
                "/execution_id",
                "/payload/spec/node_id",
                "/payload/spec/workspace_id",
                "/payload/spec/worktree_id",
                "/payload/spec/repository",
                "/payload/spec/main_workspace/workspace_id",
                "/payload/spec/main_workspace/path",
                "/payload/spec/base_ref",
                "/payload/spec/expected_branch",
                "/payload/spec/path_policy/directory_name",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/spec/node_id", "node_id"),
                ("/payload/spec/workspace_id", "workspace_id"),
                ("/payload/spec/worktree_id", "worktree_id"),
                ("/payload/spec/repository", "repository"),
                (
                    "/payload/spec/main_workspace/workspace_id",
                    "main_workspace.workspace_id",
                ),
                ("/payload/spec/main_workspace/path", "main_workspace.path"),
                ("/payload/spec/base_ref", "base_ref"),
                ("/payload/spec/expected_branch", "expected_branch"),
                (
                    "/payload/spec/path_policy/directory_name",
                    "path_policy.directory_name",
                ),
            ],
        )
        .await?;
    fixtures::remove_worktree_without_request()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/spec",
                "/payload/spec/path_policy/kind",
                "/operation_id",
                "/execution_id",
                "/payload/spec/node_id",
                "/payload/spec/workspace_id",
                "/payload/spec/worktree_id",
                "/payload/spec/repository",
                "/payload/spec/main_workspace/workspace_id",
                "/payload/spec/main_workspace/path",
                "/payload/spec/base_ref",
                "/payload/spec/expected_branch",
                "/payload/spec/path_policy/directory_name",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/spec/node_id", "node_id"),
                ("/payload/spec/workspace_id", "workspace_id"),
                ("/payload/spec/worktree_id", "worktree_id"),
                ("/payload/spec/repository", "repository"),
                (
                    "/payload/spec/main_workspace/workspace_id",
                    "main_workspace.workspace_id",
                ),
                ("/payload/spec/main_workspace/path", "main_workspace.path"),
                ("/payload/spec/base_ref", "base_ref"),
                ("/payload/spec/expected_branch", "expected_branch"),
                (
                    "/payload/spec/path_policy/directory_name",
                    "path_policy.directory_name",
                ),
            ],
        )
        .await?;
    fixtures::worktree_ready()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
                "/payload/facts/path",
                "/payload/facts/branch",
                "/payload/facts/base_commit",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
                ("/payload/facts/path", "facts.path"),
                ("/payload/facts/branch", "facts.branch"),
                ("/payload/facts/base_commit", "facts.base_commit"),
            ],
        )
        .await?;
    fixtures::worktree_ready_without_request()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
                "/payload/facts/path",
                "/payload/facts/branch",
                "/payload/facts/base_commit",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
                ("/payload/facts/path", "facts.path"),
                ("/payload/facts/branch", "facts.branch"),
                ("/payload/facts/base_commit", "facts.base_commit"),
            ],
        )
        .await?;
    fixtures::worktree_failed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/failure/code",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
                "/payload/failure/message",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
                ("/payload/failure/message", "failure.message"),
            ],
        )
        .await?;
    fixtures::worktree_failed_without_request()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/failure/code",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
                "/payload/failure/message",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
                ("/payload/failure/message", "failure.message"),
            ],
        )
        .await?;
    fixtures::worktree_removed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/outcome",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
            ],
        )
        .await?;
    fixtures::worktree_removed_without_request()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/outcome",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
            ],
        )
        .await?;
    fixtures::worktree_removed_performed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/outcome",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
            ],
        )
        .await?;
    fixtures::worktree_removal_failed()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/failure/code",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
                "/payload/failure/message",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/request_id", "request_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
                ("/payload/failure/message", "failure.message"),
            ],
        )
        .await?;
    fixtures::worktree_removal_failed_without_request()
        .assert_fields(
            &[
                "/message_type",
                "/protocol_version",
                "/payload",
                "/payload/failure/code",
                "/sequence",
                "/operation_id",
                "/execution_id",
                "/payload/node/node_id",
                "/payload/node/incarnation_id",
                "/payload/workspace_id",
                "/payload/worktree_id",
                "/payload/failure/message",
            ],
            &[
                ("/operation_id", "operation_id"),
                ("/execution_id", "execution_id"),
                ("/payload/node/node_id", "node.node_id"),
                ("/payload/node/incarnation_id", "node.incarnation_id"),
                ("/payload/workspace_id", "workspace_id"),
                ("/payload/worktree_id", "worktree_id"),
                ("/payload/failure/message", "failure.message"),
            ],
        )
        .await?;
    Ok(())
}
