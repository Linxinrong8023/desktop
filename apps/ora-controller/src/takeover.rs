use super::*;
use rusqlite::{OptionalExtension, params};

impl<W: WriteGuard> Controller<W> {
    /// Commits query/event facts through one boundary; only an actually received event can produce an Ack.
    pub fn take_over(
        &mut self,
        session: &NodeRuntimeIdentity,
        message: &NodeToControllerMessage,
    ) -> Result<Option<EventAckMessage>, Error> {
        message.validate()?;
        match message {
            NodeToControllerMessage::CloneResult(event) => {
                self.commit_result(
                    session,
                    &event.operation_id,
                    &event.execution_id,
                    &event.payload,
                    Some(event),
                )?;
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
                self.original(session, &status.operation_id, &status.execution_id)?;
                match &status.payload.state {
                    ExecutionState::Completed(ExecutionResult::Clone(result)) => self
                        .commit_result(
                            session,
                            &status.operation_id,
                            &status.execution_id,
                            result,
                            /*event*/ None,
                        )?,
                    ExecutionState::Completed(ExecutionResult::Worktree(_)) => {
                        return Err(Error::Conflict);
                    }
                    ExecutionState::Unknown
                    | ExecutionState::Accepted
                    | ExecutionState::Running => {}
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

    /// Resolves the exact durable dispatch before using any remote fact or authorizing retransmission.
    pub(crate) fn original(
        &self,
        session: &NodeRuntimeIdentity,
        operation: &OperationId,
        execution: &ExecutionId,
    ) -> Result<CloneRepositoryMessage, Error> {
        let input: Option<String> = self.connection.query_row("SELECT input FROM clone_operations WHERE operation=?1 AND execution=?2 AND node=?3",
            params![operation.as_str(), execution.as_str(), session.node_id.as_str()], |r| r.get(/*idx*/ 0)).optional()?;
        Ok(serde_json::from_str(&input.ok_or(Error::Conflict)?)?)
    }

    /// Stores terminal facts and exact event receipt atomically; historical incarnation remains untouched.
    fn commit_result(
        &mut self,
        session: &NodeRuntimeIdentity,
        operation: &OperationId,
        execution: &ExecutionId,
        result: &CloneExecutionResult,
        event: Option<&CloneResultMessage>,
    ) -> Result<(), Error> {
        let command = self.original(session, operation, execution)?;
        let (node, spec) = match result {
            CloneExecutionResult::CloneReady(ready) => (&ready.node, &ready.spec),
            CloneExecutionResult::CloneFailed(failed) => (&failed.node, &failed.spec),
        };
        if node.node_id != session.node_id
            || *spec != command.payload.spec
            || event.is_some_and(|event| event.request_id != command.request_id)
        {
            return Err(Error::Conflict);
        }
        self.writes.before_write(WritePoint::Takeover)?;
        let tx = self.connection.transaction()?;
        let previous: Option<String> = tx.query_row(
            "SELECT result FROM clone_operations WHERE execution=?1",
            [execution.as_str()],
            |r| r.get(/*idx*/ 0),
        )?;
        if let Some(previous) = previous
            && serde_json::from_str::<CloneExecutionResult>(&previous)? != *result
        {
            return Err(Error::Conflict);
        }
        tx.execute(
            "UPDATE clone_operations SET result=?1 WHERE execution=?2 AND result IS NULL",
            params![serde_json::to_string(result)?, execution.as_str()],
        )?;
        if let Some(event) = event {
            self.writes.before_write(WritePoint::Receipt)?;
            let encoded = serde_json::to_string(event)?;
            let sequence = i64::try_from(event.sequence.value()).map_err(|_| Error::Conflict)?;
            let previous: Option<String> = tx
                .query_row(
                    "SELECT event FROM clone_receipts WHERE execution=?1 AND sequence=?2",
                    params![execution.as_str(), sequence],
                    |r| r.get(/*idx*/ 0),
                )
                .optional()?;
            if previous.is_some_and(|previous| previous != encoded) {
                return Err(Error::Conflict);
            }
            tx.execute(
                "INSERT OR IGNORE INTO clone_receipts VALUES (?1,?2,?3)",
                params![execution.as_str(), sequence, encoded],
            )?;
        }
        // Keep the pre-commit fault boundary inside the live transaction, shared by query
        // and event takeover. No acknowledgement can escape while this boundary is pending.
        self.writes.before_write(WritePoint::Commit)?;
        tx.commit()?;
        Ok(())
    }
}
