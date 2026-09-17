//! Serves a session's conversation from Ora's own record, with no provider attached.

use super::replay::recorded_replay;
use super::stream::SessionEventStream;
use super::*;
use ora_contracts::LoadSessionEvent;
use ora_history::read_session_history;
use ora_logging::ora_warn;
use std::path::Path;
use tokio::sync::mpsc;

/// Why a detached load could not be served, in the words the recorder would have used.
///
/// The reason travels out rather than only into the log: a record no reader can see must not be
/// appended to either, so the caller degrades the session's history state with this same text.
pub(super) struct UnreadableHistory {
    pub(super) reason: String,
}

/// Streams one session's durable transcript to a client that opened it.
///
/// The conversation belongs to Ora rather than to the agent that produced it, so reading one asks
/// nothing of the agent: a session whose plugin was uninstalled, or whose CLI cannot start, still
/// opens. Nothing is registered and no lifecycle state changes — the provider is attached by the
/// next prompt, which is the first moment one is actually needed.
///
/// Used only for sessions with no live actor. An actor knows its own durable cutoff and the
/// records of a turn still in flight, so it answers its own loads and hands off from disk to live
/// without a gap.
pub(super) fn detached_replay(
    sessions_root: &Path,
    session_id: &str,
) -> Result<SessionEventStream<LoadSessionEvent>, UnreadableHistory> {
    let history = read_session_history(sessions_root, session_id).map_err(|error| {
        // Load is how a user asks to see the conversation, so a history that cannot be read is
        // reported rather than shown as an empty one, which would state that nothing was ever said.
        ora_warn!(
            session_id = %session_id,
            error = %error,
            "session history unreadable during load",
        );
        UnreadableHistory {
            reason: error.to_string(),
        }
    })?;
    let (events, receiver) = mpsc::channel(CONTRACT_QUEUE_CAPACITY);
    // A long history is far larger than the event queue, so the replay is driven by its own task
    // and lets `send` apply backpressure. A client that stops listening drops the receiver, which
    // ends the task at its next send rather than leaving it to finish into nothing.
    let worker = tokio::spawn(async move {
        for event in recorded_replay(history).chain(std::iter::once(LoadSessionEvent::Completed)) {
            if events.send(Ok(event)).await.is_err() {
                return;
            }
        }
    });
    Ok(SessionEventStream::with_cleanup(
        receiver,
        move || async move {
            worker
                .await
                .map_err(|error| crate::BackendError::internal("replay cleanup failed", error))
        },
    ))
}

impl AgentRuntimeManager {
    /// Serves one session conversation from Ora's own record, following a live turn when there is one.
    ///
    /// Opening a conversation never touches ACP. The transcript belongs to Ora rather than to the
    /// agent that produced it, so a session whose plugin was uninstalled — or whose CLI cannot
    /// start — still reads, and no provider session is created for a reader who may never send
    /// anything. The agent is reached by the first prompt instead.
    ///
    /// A live actor answers its own loads: only it knows the durable cutoff and the records of a
    /// turn still streaming, which is what lets a load hand off from disk to live without a gap.
    pub(crate) async fn load_session(
        &self,
        request: LoadSessionRequest,
    ) -> Result<SessionEventStream<LoadSessionEvent>, BackendError> {
        let _lifecycle = self.inner.lifecycle.lock().await;
        let session = self.find_session(&request.session_id)?;
        let Some(handle) = self.lookup_actor(&session.id)? else {
            return load::detached_replay(&self.inner.sessions_root, session.id.as_ref()).map_err(
                |UnreadableHistory { reason }| {
                    // A record no reader can see must not be appended to either, and a load is
                    // usually the first thing to touch it. Degrading here rather than waiting for
                    // a prompt to open its recorder is what stops the composer at the same moment
                    // the transcript stops being readable.
                    self.settle_record(session, RecordOutcome::JustFailed { reason });
                    session_history_unreadable()
                },
            );
        };
        let operation_id = self.inner.next_operation_id.fetch_add(1, Ordering::Relaxed);
        let (cleanup, cleaned) = oneshot::channel();
        let (events_sender, events) = mpsc::channel(CONTRACT_QUEUE_CAPACITY);
        let (accepted_sender, accepted) = oneshot::channel();
        handle
            .commands
            .send(RuntimeCommand::Load {
                cleanup,
                operation_id,
                events: events_sender,
                accepted: accepted_sender,
            })
            .map_err(runtime_unavailable_with)?;
        accepted.await.map_err(runtime_unavailable_with)??;
        Ok(
            SessionEventStream::new(events, handle.commands, operation_id).attach_cleanup(
                move || async move {
                    cleaned.await.map_err(|error| {
                        BackendError::internal("load worker did not confirm cleanup", error)
                    })?
                },
            ),
        )
    }
}
