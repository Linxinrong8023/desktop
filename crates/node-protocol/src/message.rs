mod execution;
mod session;
mod validation;
mod worktree;

pub use execution::{
    EventAck, EventAckMessage, ExecutionState, ExecutionStatus, ExecutionStatusMessage,
    GetExecutionStatus, GetExecutionStatusMessage,
};
use serde::{Deserialize, Serialize};
pub use session::{
    Heartbeat, HeartbeatMessage, Hello, HelloAccepted, HelloAcceptedMessage, HelloMessage,
    NodeCapability,
};
pub use validation::MessageValidationError;
pub(crate) use validation::ValidateMessage;
pub use worktree::{
    EnsureWorktree, EnsureWorktreeMessage, RemoveWorktree, RemoveWorktreeMessage,
    WorktreeFailedMessage, WorktreeReadyMessage, WorktreeRemovalFailedMessage,
    WorktreeRemovedMessage,
};

/// Messages sent by the Controller; each business owns its envelope invariants.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum ControllerToNodeMessage {
    Hello(HelloMessage),
    EnsureWorktree(EnsureWorktreeMessage),
    RemoveWorktree(RemoveWorktreeMessage),
    GetExecutionStatus(GetExecutionStatusMessage),
    EventAck(EventAckMessage),
}

impl ValidateMessage for ControllerToNodeMessage {
    /// Dispatches validation without introducing business rules into the codec.
    fn validate(&self) -> Result<(), MessageValidationError> {
        match self {
            Self::Hello(message) => message.validate(),
            Self::EnsureWorktree(message) => message.validate(),
            Self::RemoveWorktree(message) => message.validate(),
            Self::GetExecutionStatus(message) => message.validate(),
            Self::EventAck(message) => message.validate(),
        }
    }
}

/// Messages sent by the Node; each business owns its envelope invariants.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "message_type", rename_all = "snake_case")]
pub enum NodeToControllerMessage {
    HelloAccepted(HelloAcceptedMessage),
    Heartbeat(HeartbeatMessage),
    ExecutionStatus(ExecutionStatusMessage),
    WorktreeReady(WorktreeReadyMessage),
    WorktreeFailed(WorktreeFailedMessage),
    WorktreeRemoved(WorktreeRemovedMessage),
    WorktreeRemovalFailed(WorktreeRemovalFailedMessage),
}

impl ValidateMessage for NodeToControllerMessage {
    /// Dispatches validation without introducing business rules into the codec.
    fn validate(&self) -> Result<(), MessageValidationError> {
        match self {
            Self::HelloAccepted(message) => message.validate(),
            Self::Heartbeat(message) => message.validate(),
            Self::ExecutionStatus(message) => message.validate(),
            Self::WorktreeReady(message) => message.validate(),
            Self::WorktreeFailed(message) => message.validate(),
            Self::WorktreeRemoved(message) => message.validate(),
            Self::WorktreeRemovalFailed(message) => message.validate(),
        }
    }
}
