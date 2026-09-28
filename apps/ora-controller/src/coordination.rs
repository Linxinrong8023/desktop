use crate::*;

/// Takes one Node message through the store boundary. Only an actually received event can produce
/// an Ack, and only after the store confirmed the takeover; queries never acknowledge anything.
pub async fn take_over<S: CoordinationStore>(
    store: &S,
    session: &NodeRuntimeIdentity,
    message: &NodeToControllerMessage,
) -> Result<Option<EventAckMessage>, Error> {
    message.validate()?;
    match message {
        NodeToControllerMessage::CloneResult(event) => {
            store.take_over_node_event(session, event).await?;
            Ok(Some(EventAckMessage {
                protocol_version: CURRENT_PROTOCOL_VERSION,
                operation_id: event.operation_id.clone(),
                execution_id: event.execution_id.clone(),
                sequence: event.sequence,
                payload: EventAck {
                    node_id: session.node_id.clone(),
                },
            }))
        }
        NodeToControllerMessage::ExecutionStatus(status) => {
            if status.payload.node != *session {
                return Err(Error::Conflict);
            }
            match &status.payload.state {
                ExecutionState::Completed(ExecutionResult::Clone(result)) => {
                    store
                        .record_queried_result(
                            session,
                            &status.operation_id,
                            &status.execution_id,
                            result,
                        )
                        .await?;
                }
                ExecutionState::Completed(ExecutionResult::Worktree(_)) => {
                    return Err(Error::Conflict);
                }
                // A status for an unknown dispatch is a conflict even when it carries no result.
                ExecutionState::Unknown | ExecutionState::Accepted | ExecutionState::Running => {
                    store
                        .original_dispatch(session, &status.operation_id, &status.execution_id)
                        .await?;
                }
            }
            Ok(None)
        }
        NodeToControllerMessage::Heartbeat(heartbeat) if heartbeat.payload.node == *session => {
            Ok(None)
        }
        NodeToControllerMessage::Heartbeat(_)
        | NodeToControllerMessage::HelloAccepted(_)
        | NodeToControllerMessage::WorktreeReady(_)
        | NodeToControllerMessage::WorktreeFailed(_)
        | NodeToControllerMessage::WorktreeRemoved(_)
        | NodeToControllerMessage::WorktreeRemovalFailed(_) => Err(Error::Conflict),
    }
}
