use super::*;

/// Pairs the get_execution_status typed fixture with an independent wire contract.
pub(super) fn get_execution_status() -> Case {
    Case {
        message: Message::Controller(ControllerToNodeMessage::GetExecutionStatus(
            GetExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: GetExecutionStatus {
                    node_id: NodeId::new("node-1"),
                },
            },
        )),
        wire: json!({
            "message_type": "get_execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node_id": "node-1"
            }
        }),
    }
}

/// Pairs the event_ack typed fixture with an independent wire contract.
pub(super) fn event_ack() -> Case {
    Case {
        message: Message::Controller(ControllerToNodeMessage::EventAck(EventAckMessage {
            protocol_version: CURRENT_PROTOCOL_VERSION,
            operation_id: OperationId::new("operation-create"),
            execution_id: ExecutionId::new("execution-create"),
            sequence: Sequence::new(/*value*/ 7),
            payload: EventAck {
                node_id: NodeId::new("node-1"),
            },
        })),
        wire: json!({
            "message_type": "event_ack",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "sequence": 7,
            "payload": {
                "node_id": "node-1"
            }
        }),
    }
}

/// Pairs the execution_status typed fixture with an independent wire contract.
pub(super) fn execution_status() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Completed(
                        WorktreeExecutionResult::Ready(ready_result()),
                    ),
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "completed",
                    "result": {
                        "kind": "ready",
                        "result": {
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
                    }
                }
            }
        }),
    }
}

/// Pairs the execution_status_unknown typed fixture with an independent wire contract.
pub(super) fn execution_status_unknown() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Unknown,
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "unknown"
                }
            }
        }),
    }
}

/// Pairs the execution_status_accepted typed fixture with an independent wire contract.
pub(super) fn execution_status_accepted() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Accepted,
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "accepted"
                }
            }
        }),
    }
}

/// Pairs the execution_status_running typed fixture with an independent wire contract.
pub(super) fn execution_status_running() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Running,
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "running"
                }
            }
        }),
    }
}

/// Pairs the execution_status_failed typed fixture with an independent wire contract.
pub(super) fn execution_status_failed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Completed(WorktreeExecutionResult::Failed(
                        failed_result(),
                    )),
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "completed",
                    "result": {
                        "kind": "failed",
                        "result": {
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
                    }
                }
            }
        }),
    }
}

/// Pairs the execution_status_removed typed fixture with an independent wire contract.
pub(super) fn execution_status_removed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Completed(WorktreeExecutionResult::Removed(
                        removed_result(),
                    )),
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "completed",
                    "result": {
                        "kind": "removed",
                        "result": {
                            "node": {
                                "node_id": "node-1",
                                "incarnation_id": "incarnation-1"
                            },
                            "workspace_id": "workspace-task",
                            "worktree_id": "worktree-1",
                            "outcome": "already_absent"
                        }
                    }
                }
            }
        }),
    }
}

/// Pairs the execution_status_removal_failed typed fixture with an independent wire contract.
pub(super) fn execution_status_removal_failed() -> Case {
    Case {
        message: Message::Node(NodeToControllerMessage::ExecutionStatus(
            ExecutionStatusMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: OperationId::new("operation-create"),
                execution_id: ExecutionId::new("execution-create"),
                payload: ExecutionStatus {
                    node: node(),
                    state: ExecutionState::Completed(WorktreeExecutionResult::RemovalFailed(
                        removal_failed_result(),
                    )),
                },
            },
        )),
        wire: json!({
            "message_type": "execution_status",
            "protocol_version": 1,
            "operation_id": "operation-create",
            "execution_id": "execution-create",
            "payload": {
                "node": {
                    "node_id": "node-1",
                    "incarnation_id": "incarnation-1"
                },
                "state": {
                    "state": "completed",
                    "result": {
                        "kind": "removal_failed",
                        "result": {
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
                    }
                }
            }
        }),
    }
}

/// Lists this business slice’s legal wire shapes.
pub(super) fn cases() -> Vec<Case> {
    vec![
        get_execution_status(),
        event_ack(),
        execution_status(),
        execution_status_unknown(),
        execution_status_accepted(),
        execution_status_running(),
        execution_status_failed(),
        execution_status_removed(),
        execution_status_removal_failed(),
    ]
}
