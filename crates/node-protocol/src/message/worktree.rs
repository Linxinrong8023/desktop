use super::validation::{
    MessageValidationError, ValidateMessage, validate_execution_ids, validate_protocol_version,
};
use crate::{
    ExecutionId, OperationId, ProtocolVersion, RequestId, Sequence, WorktreeExecutionSpec,
};
use serde::{Deserialize, Serialize};

/// Command asking a Node to durably ensure one task worktree exists.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EnsureWorktree {
    pub spec: WorktreeExecutionSpec,
}

/// Command asking a Node to durably remove one task worktree and its owned branch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RemoveWorktree {
    pub spec: WorktreeExecutionSpec,
}

/// Complete EnsureWorktree envelope, including only metadata valid for this message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EnsureWorktreeMessage {
    pub protocol_version: ProtocolVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub operation_id: OperationId,
    pub execution_id: ExecutionId,
    pub payload: EnsureWorktree,
}

/// Complete RemoveWorktree envelope, including only metadata valid for this message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RemoveWorktreeMessage {
    pub protocol_version: ProtocolVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub operation_id: OperationId,
    pub execution_id: ExecutionId,
    pub payload: RemoveWorktree,
}

impl ValidateMessage for EnsureWorktreeMessage {
    /// Enforces this message’s semantic rules on both send and receive.
    fn validate(&self) -> Result<(), MessageValidationError> {
        let Self {
            protocol_version,
            request_id,
            operation_id,
            execution_id,
            payload,
        } = self;

        validate_protocol_version(*protocol_version)?;
        if request_id.as_ref().is_some_and(RequestId::is_empty) {
            return Err(MessageValidationError::EmptyField {
                field: "request_id",
            });
        }
        validate_execution_ids(operation_id, execution_id)?;
        payload
            .spec
            .validate()
            .map_err(|field| MessageValidationError::EmptyField { field })
    }
}

impl ValidateMessage for RemoveWorktreeMessage {
    /// Enforces this message’s semantic rules on both send and receive.
    fn validate(&self) -> Result<(), MessageValidationError> {
        let Self {
            protocol_version,
            request_id,
            operation_id,
            execution_id,
            payload,
        } = self;

        validate_protocol_version(*protocol_version)?;
        if request_id.as_ref().is_some_and(RequestId::is_empty) {
            return Err(MessageValidationError::EmptyField {
                field: "request_id",
            });
        }
        validate_execution_ids(operation_id, execution_id)?;
        payload
            .spec
            .validate()
            .map_err(|field| MessageValidationError::EmptyField { field })
    }
}

/// Complete WorktreeReady envelope, including only metadata valid for this message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorktreeReadyMessage {
    pub protocol_version: ProtocolVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub operation_id: OperationId,
    pub execution_id: ExecutionId,
    pub sequence: Sequence,
    pub payload: crate::WorktreeReady,
}

/// Complete WorktreeFailed envelope, including only metadata valid for this message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorktreeFailedMessage {
    pub protocol_version: ProtocolVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub operation_id: OperationId,
    pub execution_id: ExecutionId,
    pub sequence: Sequence,
    pub payload: crate::WorktreeFailed,
}

/// Complete WorktreeRemoved envelope, including only metadata valid for this message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorktreeRemovedMessage {
    pub protocol_version: ProtocolVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub operation_id: OperationId,
    pub execution_id: ExecutionId,
    pub sequence: Sequence,
    pub payload: crate::WorktreeRemoved,
}

/// Complete WorktreeRemovalFailed envelope, including only metadata valid for this message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorktreeRemovalFailedMessage {
    pub protocol_version: ProtocolVersion,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub operation_id: OperationId,
    pub execution_id: ExecutionId,
    pub sequence: Sequence,
    pub payload: crate::WorktreeRemovalFailed,
}

impl ValidateMessage for WorktreeReadyMessage {
    /// Enforces this message’s semantic rules on both send and receive.
    fn validate(&self) -> Result<(), MessageValidationError> {
        let Self {
            protocol_version,
            request_id,
            operation_id,
            execution_id,
            payload,
            ..
        } = self;

        validate_protocol_version(*protocol_version)?;
        if request_id.as_ref().is_some_and(RequestId::is_empty) {
            return Err(MessageValidationError::EmptyField {
                field: "request_id",
            });
        }
        validate_execution_ids(operation_id, execution_id)?;
        payload
            .validate()
            .map_err(|field| MessageValidationError::EmptyField { field })
    }
}

impl ValidateMessage for WorktreeFailedMessage {
    /// Enforces this message’s semantic rules on both send and receive.
    fn validate(&self) -> Result<(), MessageValidationError> {
        let Self {
            protocol_version,
            request_id,
            operation_id,
            execution_id,
            payload,
            ..
        } = self;

        validate_protocol_version(*protocol_version)?;
        if request_id.as_ref().is_some_and(RequestId::is_empty) {
            return Err(MessageValidationError::EmptyField {
                field: "request_id",
            });
        }
        validate_execution_ids(operation_id, execution_id)?;
        payload
            .validate()
            .map_err(|field| MessageValidationError::EmptyField { field })
    }
}

impl ValidateMessage for WorktreeRemovedMessage {
    /// Enforces this message’s semantic rules on both send and receive.
    fn validate(&self) -> Result<(), MessageValidationError> {
        let Self {
            protocol_version,
            request_id,
            operation_id,
            execution_id,
            payload,
            ..
        } = self;

        validate_protocol_version(*protocol_version)?;
        if request_id.as_ref().is_some_and(RequestId::is_empty) {
            return Err(MessageValidationError::EmptyField {
                field: "request_id",
            });
        }
        validate_execution_ids(operation_id, execution_id)?;
        payload
            .validate()
            .map_err(|field| MessageValidationError::EmptyField { field })
    }
}

impl ValidateMessage for WorktreeRemovalFailedMessage {
    /// Enforces this message’s semantic rules on both send and receive.
    fn validate(&self) -> Result<(), MessageValidationError> {
        let Self {
            protocol_version,
            request_id,
            operation_id,
            execution_id,
            payload,
            ..
        } = self;

        validate_protocol_version(*protocol_version)?;
        if request_id.as_ref().is_some_and(RequestId::is_empty) {
            return Err(MessageValidationError::EmptyField {
                field: "request_id",
            });
        }
        validate_execution_ids(operation_id, execution_id)?;
        payload
            .validate()
            .map_err(|field| MessageValidationError::EmptyField { field })
    }
}
