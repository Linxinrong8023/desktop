use super::*;

/// Pairs the ensure_worktree typed fixture with an independent wire contract.
pub(super) fn ensure_worktree() -> Case {
    Case {
        message: Message::Controller(ControllerToNodeMessage::EnsureWorktree(
            EnsureWorktreeMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-1")),
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: EnsureWorktree { spec: spec() },
            },
        )),
        wire: json!({
            "message_type": "ensure_worktree",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "request_id": "request-1",
            "payload": {
                "spec": {
                    "node_id": "node-1",
                    "workspace_id": "workspace-task",
                    "worktree_id": "worktree-1",
                    "repository": "repository-1",
                    "main_workspace": {
                        "workspace_id": "workspace-main",
                        "path": "/node/repos/ora"
                    },
                    "base_ref": "refs/heads/main",
                    "expected_branch": "ora/12345678",
                    "path_policy": {
                        "kind": "node_managed",
                        "directory_name": "workspace-task"
                    }
                }
            }
        }),
    }
}

/// Pairs the ensure_worktree_without_request typed fixture with an independent wire contract.
pub(super) fn ensure_worktree_without_request() -> Case {
    Case {
        message: Message::Controller(ControllerToNodeMessage::EnsureWorktree(
            EnsureWorktreeMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: None,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: EnsureWorktree { spec: spec() },
            },
        )),
        wire: json!({
            "message_type": "ensure_worktree",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "spec": {
                    "node_id": "node-1",
                    "workspace_id": "workspace-task",
                    "worktree_id": "worktree-1",
                    "repository": "repository-1",
                    "main_workspace": {
                        "workspace_id": "workspace-main",
                        "path": "/node/repos/ora"
                    },
                    "base_ref": "refs/heads/main",
                    "expected_branch": "ora/12345678",
                    "path_policy": {
                        "kind": "node_managed",
                        "directory_name": "workspace-task"
                    }
                }
            }
        }),
    }
}

/// Pairs the remove_worktree typed fixture with an independent wire contract.
pub(super) fn remove_worktree() -> Case {
    Case {
        message: Message::Controller(ControllerToNodeMessage::RemoveWorktree(
            RemoveWorktreeMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-2")),
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                payload: RemoveWorktree { spec: spec() },
            },
        )),
        wire: json!({
            "message_type": "remove_worktree",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "request_id": "request-2",
            "payload": {
                "spec": {
                    "node_id": "node-1",
                    "workspace_id": "workspace-task",
                    "worktree_id": "worktree-1",
                    "repository": "repository-1",
                    "main_workspace": {
                        "workspace_id": "workspace-main",
                        "path": "/node/repos/ora"
                    },
                    "base_ref": "refs/heads/main",
                    "expected_branch": "ora/12345678",
                    "path_policy": {
                        "kind": "node_managed",
                        "directory_name": "workspace-task"
                    }
                }
            }
        }),
    }
}

/// Pairs the remove_worktree_without_request typed fixture with an independent wire contract.
pub(super) fn remove_worktree_without_request() -> Case {
    Case {
        message: Message::Controller(ControllerToNodeMessage::RemoveWorktree(
            RemoveWorktreeMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: None,
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                payload: RemoveWorktree { spec: spec() },
            },
        )),
        wire: json!({
            "message_type": "remove_worktree",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "payload": {
                "spec": {
                    "node_id": "node-1",
                    "workspace_id": "workspace-task",
                    "worktree_id": "worktree-1",
                    "repository": "repository-1",
                    "main_workspace": {
                        "workspace_id": "workspace-main",
                        "path": "/node/repos/ora"
                    },
                    "base_ref": "refs/heads/main",
                    "expected_branch": "ora/12345678",
                    "path_policy": {
                        "kind": "node_managed",
                        "directory_name": "workspace-task"
                    }
                }
            }
        }),
    }
}

/// Pairs the worktree_ready typed fixture with an independent wire contract.
pub(super) fn worktree_ready() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeReady(
            WorktreeReadyMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-1")),
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                sequence: Sequence::new(/*value*/ 1),
                payload: ready_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_ready",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "request_id": "request-1",
            "sequence": 1,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "facts": {
                    "path": "/node/worktrees/workspace-task",
                    "branch": "ora/12345678",
                    "base_commit": "0123456789abcdef"
                }
            }
        }),
    }
}

/// Pairs the worktree_ready_without_request typed fixture with an independent wire contract.
pub(super) fn worktree_ready_without_request() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeReady(
            WorktreeReadyMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: None,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                sequence: Sequence::new(/*value*/ 1),
                payload: ready_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_ready",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "sequence": 1,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "facts": {
                    "path": "/node/worktrees/workspace-task",
                    "branch": "ora/12345678",
                    "base_commit": "0123456789abcdef"
                }
            }
        }),
    }
}

/// Pairs the worktree_failed typed fixture with an independent wire contract.
pub(super) fn worktree_failed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeFailed(
            WorktreeFailedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-1")),
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                sequence: Sequence::new(/*value*/ 1),
                payload: failed_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_failed",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "request_id": "request-1",
            "sequence": 1,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "failure": {
                    "code": "branch_conflict",
                    "message": "branch is already checked out"
                }
            }
        }),
    }
}

/// Pairs the worktree_failed_without_request typed fixture with an independent wire contract.
pub(super) fn worktree_failed_without_request() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeFailed(
            WorktreeFailedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: None,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                sequence: Sequence::new(/*value*/ 1),
                payload: failed_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_failed",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "sequence": 1,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "failure": {
                    "code": "branch_conflict",
                    "message": "branch is already checked out"
                }
            }
        }),
    }
}

/// Pairs the worktree_removed typed fixture with an independent wire contract.
pub(super) fn worktree_removed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeRemoved(
            WorktreeRemovedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-2")),
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                sequence: Sequence::new(/*value*/ 2),
                payload: removed_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_removed",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "request_id": "request-2",
            "sequence": 2,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "outcome": "already_absent"
            }
        }),
    }
}

/// Pairs the worktree_removed_without_request typed fixture with an independent wire contract.
pub(super) fn worktree_removed_without_request() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeRemoved(
            WorktreeRemovedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: None,
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                sequence: Sequence::new(/*value*/ 2),
                payload: removed_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_removed",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "sequence": 2,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "outcome": "already_absent"
            }
        }),
    }
}

/// Pairs the worktree_removed_performed typed fixture with an independent wire contract.
pub(super) fn worktree_removed_performed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeRemoved(
            WorktreeRemovedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-2")),
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                sequence: Sequence::new(/*value*/ 2),
                payload: WorktreeRemoved {
                    outcome: WorktreeRemovalOutcome::Removed,
                    ..removed_result()
                },
            },
        )),
        wire: json!({
            "message_type": "worktree_removed",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "request_id": "request-2",
            "sequence": 2,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "outcome": "removed"
            }
        }),
    }
}

/// Pairs the worktree_removal_failed typed fixture with an independent wire contract.
pub(super) fn worktree_removal_failed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeRemovalFailed(
            WorktreeRemovalFailedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: Some(RequestId::new("request-2")),
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                sequence: Sequence::new(/*value*/ 2),
                payload: removal_failed_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_removal_failed",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "request_id": "request-2",
            "sequence": 2,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "failure": {
                    "code": "operation_failed",
                    "message": "Git refused to remove the worktree"
                }
            }
        }),
    }
}

/// Pairs the worktree_removal_failed_without_request typed fixture with an independent wire contract.
pub(super) fn worktree_removal_failed_without_request() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::WorktreeRemovalFailed(
            WorktreeRemovalFailedMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                request_id: None,
                operation_id: OperationId::new("operation-remove"),
                execution_id: ExecutionId::new("execution-remove"),
                sequence: Sequence::new(/*value*/ 2),
                payload: removal_failed_result(),
            },
        )),
        wire: json!({
            "message_type": "worktree_removal_failed",
            "protocol_version": 1,
            "operation_id": "operation-remove",
            "execution_id": "execution-remove",
            "sequence": 2,
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "workspace_id": "workspace-task",
                "worktree_id": "worktree-1",
                "failure": {
                    "code": "operation_failed",
                    "message": "Git refused to remove the worktree"
                }
            }
        }),
    }
}

/// Lists this business slice’s legal wire shapes.
pub(super) fn cases() -> Vec<Case> {
    vec![
        ensure_worktree(),
        ensure_worktree_without_request(),
        remove_worktree(),
        remove_worktree_without_request(),
        worktree_ready(),
        worktree_ready_without_request(),
        worktree_failed(),
        worktree_failed_without_request(),
        worktree_removed(),
        worktree_removed_without_request(),
        worktree_removed_performed(),
        worktree_removal_failed(),
        worktree_removal_failed_without_request(),
    ]
}
