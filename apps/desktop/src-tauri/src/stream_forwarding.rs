//! Bridges backend event streams onto Tauri IPC Channels.
//!
//! Every stream started at the command seam is deferred: the command returns before any event
//! exists, so the forwarding task owns the request's `RequestLifecycle` and is solely responsible
//! for emitting its single completion event. Each way a stream can end — caller cancellation, a
//! terminal backend error, natural exhaustion, or the frontend Channel disappearing — must claim
//! that completion, otherwise the request leaves no closing record in the logs.
//!
//! Terminal frames claim their completion from the backend outcome *before* being sent, so a
//! Channel that dies while the last frame is in flight keeps the real backend result in the log
//! rather than rewriting it as a cancellation. The recorded outcome therefore describes how the
//! backend stream ended, not whether the webview observed its final frame.

use crate::stream_registry::StreamRegistration;
use crate::workspace_files::workspace_file_backend_error;
use ora_backend::{BackendError, RequestLifecycle};
use ora_contracts::{WorkspaceFileChange, WorkspaceFileEventBatch};
use serde::Serialize;
use std::time::Duration;
use tauri::ipc::Channel;

/// Forwards ordered data/error/end frames and drops the backend stream on channel failure.
pub(crate) async fn forward_contract_stream<Event>(
    mut stream: ora_backend::SessionEventStream<Event>,
    registration: StreamRegistration,
    on_event: Channel<serde_json::Value>,
    lifecycle: RequestLifecycle,
) where
    Event: Serialize + Send + 'static,
{
    let cancellation = registration.cancellation().clone();
    let outcome = loop {
        tokio::select! {
            () = cancellation.cancelled() => break StreamEnd::Cancelled,
            event = stream.recv() => match event {
                Some(Ok(data)) => {
                    if on_event.send(serde_json::json!({ "type": "data", "data": data })).is_err() {
                        break StreamEnd::Cancelled;
                    }
                }
                Some(Err(error)) => break StreamEnd::Failed(error),
                None => break StreamEnd::Finished,
            }
        }
    };
    let cleanup = stream.cancel_and_wait().await;
    finish_contract_stream(outcome, cleanup, registration, on_event, lifecycle);
}

/// Settles the business outcome and the independent cleanup receipt at the transport boundary.
fn finish_contract_stream(
    outcome: StreamEnd,
    cleanup: Result<(), BackendError>,
    registration: StreamRegistration,
    on_event: Channel<serde_json::Value>,
    lifecycle: RequestLifecycle,
) {
    let outcome = match (outcome, &cleanup) {
        (StreamEnd::Failed(error), _) => StreamEnd::Failed(error),
        (outcome, Ok(())) => outcome,
        (StreamEnd::Cancelled | StreamEnd::Finished, Err(error)) => {
            StreamEnd::Failed(error.clone())
        }
    };
    match outcome {
        StreamEnd::Cancelled => lifecycle.complete_cancellation(),
        StreamEnd::Finished => {
            lifecycle.complete_success();
            let _ = on_event.send(serde_json::json!({ "type": "end" }));
        }
        StreamEnd::Failed(error) => {
            lifecycle.complete_failure(&error);
            let _ = on_event.send(serde_json::json!({ "type": "error", "error": error.contract_error(lifecycle.request_id()) }));
        }
    }
    registration.finish(cleanup);
}

/// Defers lifecycle completion until owner cleanup has also settled.
enum StreamEnd {
    Cancelled,
    Finished,
    Failed(BackendError),
}

/// Forwards debounced native workspace changes until the Desktop stream is cancelled.
pub(crate) async fn forward_workspace_watch(
    watcher: ora_fs::WorkspaceWatcher,
    registration: StreamRegistration,
    on_event: Channel<serde_json::Value>,
    lifecycle: RequestLifecycle,
) {
    let cancellation = registration.cancellation().clone();
    let watch_cancellation = cancellation.clone();
    let terminal_channel = on_event.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        // Return the native owner on errors and closed channels too, so every exit awaits close.
        let outcome = (|| {
            while !watch_cancellation.is_cancelled() {
                match watcher.receive_batch(Duration::from_millis(100)) {
                    Ok(Some(changes)) if !changes.is_empty() => {
                        let data = WorkspaceFileEventBatch {
                            changes: changes.into_iter().map(to_contract_change).collect(),
                        };
                        if on_event
                            .send(serde_json::json!({ "type": "data", "data": data }))
                            .is_err()
                        {
                            return Err(WatchStop::ChannelClosed);
                        }
                    }
                    Ok(Some(_)) | Ok(None) => {}
                    Err(error) => return Err(WatchStop::Watcher(error)),
                }
            }
            Ok(())
        })();
        (watcher, outcome)
    })
    .await;

    let (result, cleanup) = match result {
        Ok((watcher, result)) => (
            Ok(result),
            watcher.close().await.map_err(workspace_file_backend_error),
        ),
        Err(error) => {
            let cleanup = BackendError::internal(
                "workspace watcher cleanup was not confirmed",
                std::io::Error::other(error.to_string()),
            );
            (Err(error), Err(cleanup))
        }
    };
    let outcome = match result {
        Ok(Ok(())) if cancellation.is_cancelled() => StreamEnd::Cancelled,
        Ok(Ok(())) => StreamEnd::Finished,
        Ok(Err(WatchStop::ChannelClosed)) => StreamEnd::Cancelled,
        Ok(Err(WatchStop::Watcher(error))) => {
            StreamEnd::Failed(workspace_file_backend_error(error))
        }
        Err(error) => StreamEnd::Failed(BackendError::internal(
            "Desktop workspace watcher failed",
            error,
        )),
    };
    finish_contract_stream(outcome, cleanup, registration, terminal_channel, lifecycle);
}

/// Distinguishes the two reasons the blocking watch loop stops before cancellation.
enum WatchStop {
    ChannelClosed,
    Watcher(ora_fs::WorkspaceFileSystemError),
}

/// Converts native watcher events to the shared file-change contract.
fn to_contract_change(change: ora_fs::WorkspaceChange) -> WorkspaceFileChange {
    match change.kind {
        ora_fs::WorkspaceChangeKind::Created => WorkspaceFileChange::Created { path: change.path },
        ora_fs::WorkspaceChangeKind::Modified => {
            WorkspaceFileChange::Modified { path: change.path }
        }
        ora_fs::WorkspaceChangeKind::Removed => WorkspaceFileChange::Removed { path: change.path },
        ora_fs::WorkspaceChangeKind::Renamed { from } => WorkspaceFileChange::Renamed {
            from,
            path: change.path,
        },
        ora_fs::WorkspaceChangeKind::RescanRequired => WorkspaceFileChange::RescanRequired,
    }
}

#[cfg(test)]
mod tests {
    use super::{forward_contract_stream, forward_workspace_watch};
    use crate::stream_registry::StreamRegistry;
    use ora_backend::{AppEventHub, RequestLifecycle, UuidRequestIdGenerator};
    use ora_logging::with_recorded_trace_logging;
    use pretty_assertions::assert_eq;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::ipc::Channel;
    use tracing::field::{Field, Visit};
    use tracing_subscriber::layer::{Context, Layer};

    const STREAM_CALL_ID: &str = "test-stream-call-id";

    /// Bounds the wait for a native filesystem event so a silent platform fails instead of hanging.
    const NATIVE_EVENT_TIMEOUT: Duration = Duration::from_secs(30);

    /// A stream whose frontend Channel disconnects must still record one completion event.
    ///
    /// The disconnect happens on a non-terminal data frame, which is the case that previously
    /// broke out of the forwarding loop without claiming the lifecycle at all.
    #[test]
    fn channel_disconnect_on_a_data_frame_completes_the_request_as_cancelled() {
        let recorder = OutcomeRecorder::default();
        let registry = StreamRegistry::default();
        let registration = registry
            .register(STREAM_CALL_ID.to_string())
            .expect("register stream");
        let send_attempts = Arc::new(AtomicUsize::new(0));

        with_recorded_trace_logging(recorder.layer(), || {
            runtime().block_on(async {
                // `subscribe` seeds the stream with `AppEvent::Ready`, so the first frame the
                // loop forwards is a non-terminal data frame.
                let stream = AppEventHub::new().subscribe();
                forward_contract_stream(
                    stream,
                    registration,
                    disconnected_channel(send_attempts.clone()),
                    RequestLifecycle::start("test_stream", &UuidRequestIdGenerator),
                )
                .await;
            });
        });

        assert_eq!(send_attempts.load(Ordering::SeqCst), 1);
        assert_eq!(recorder.outcomes(), vec!["cancelled".to_string()]);
        assert!(registry.register(STREAM_CALL_ID.to_string()).is_ok());
    }

    /// Cancelling a live stream records exactly one cancellation and releases its registration.
    ///
    /// The Channel here stays connected, so the loop leaves through the cancellation branch
    /// rather than the disconnect branch even when `select!` forwards the seeded event first.
    #[test]
    fn cancellation_completes_the_request_once_and_releases_the_registration() {
        let recorder = OutcomeRecorder::default();
        let registry = StreamRegistry::default();
        let registration = registry
            .register(STREAM_CALL_ID.to_string())
            .expect("register stream");
        registry.cancel(STREAM_CALL_ID).expect("cancel stream");

        with_recorded_trace_logging(recorder.layer(), || {
            runtime().block_on(async {
                let stream = AppEventHub::new().subscribe();
                forward_contract_stream(
                    stream,
                    registration,
                    connected_channel(),
                    RequestLifecycle::start("test_stream", &UuidRequestIdGenerator),
                )
                .await;
            });
        });

        assert_eq!(recorder.outcomes(), vec!["cancelled".to_string()]);
        assert!(registry.register(STREAM_CALL_ID.to_string()).is_ok());
    }

    /// A watch stream whose frontend Channel disconnects completes as cancelled, not success.
    ///
    /// The blocking watch loop returns `Ok(())` on both a clean stop and a dead Channel, so this
    /// pins the outcome that distinguishes them and would silently regress to `success` if the
    /// disconnect were ever folded back into the normal exit.
    #[test]
    fn watch_channel_disconnect_completes_the_request_as_cancelled() {
        let recorder = OutcomeRecorder::default();
        let registry = StreamRegistry::default();
        let registration = registry
            .register(STREAM_CALL_ID.to_string())
            .expect("register stream");
        let workspace = tempfile::TempDir::new().unwrap();
        let watcher = ora_fs::WorkspaceWatcher::start(workspace.path()).unwrap();
        // The watcher is already running, so this change is queued before forwarding starts and
        // the loop's first batch is the data frame whose send must fail.
        std::fs::write(workspace.path().join("watched.txt"), "changed").unwrap();
        let send_attempts = Arc::new(AtomicUsize::new(0));
        let timeout_guard = registration.cancellation().clone();
        std::thread::spawn(move || {
            std::thread::sleep(NATIVE_EVENT_TIMEOUT);
            timeout_guard.cancel();
        });

        with_recorded_trace_logging(recorder.layer(), || {
            runtime().block_on(forward_workspace_watch(
                watcher,
                registration,
                disconnected_channel(send_attempts.clone()),
                RequestLifecycle::start("test_watch_stream", &UuidRequestIdGenerator),
            ));
        });

        // A zero count means the safety net cancelled the stream before any native event arrived,
        // which would make the expected outcome pass for the wrong reason.
        assert_eq!(send_attempts.load(Ordering::SeqCst), 1);
        assert_eq!(recorder.outcomes(), vec!["cancelled".to_string()]);
        assert!(registry.register(STREAM_CALL_ID.to_string()).is_ok());
    }

    /// A running stream cannot confirm cancellation while notify still owns a blocked callback.
    #[test]
    fn running_watch_cancellation_waits_for_native_release() {
        ora_logging::with_trace_logging(|| {
            runtime().block_on(async {
                let workspace = tempfile::TempDir::new().unwrap();
                let (watcher, gate) =
                    ora_fs::watch_test_support::blocked_watcher(workspace.path()).unwrap();
                std::fs::write(workspace.path().join("trigger"), "event").unwrap();
                gate.wait_until_entered().unwrap();
                let registry = StreamRegistry::default();
                let registration = registry.register("blocked-native-run".to_string()).unwrap();
                registration.cancellation().cancel();
                let forwarding = forward_workspace_watch(
                    watcher,
                    registration,
                    connected_channel(),
                    RequestLifecycle::start("blocked_native_run", &UuidRequestIdGenerator),
                );
                tokio::pin!(forwarding);
                let early = tokio::time::timeout(Duration::from_millis(100), &mut forwarding).await;
                assert!(
                    early.is_err(),
                    "running cancellation confirmed while native callback was blocked"
                );
                assert!(
                    tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        registry.cancel_and_wait("blocked-native-run")
                    )
                    .await
                    .is_err()
                );
                gate.release();
                tokio::time::timeout(NATIVE_EVENT_TIMEOUT, &mut forwarding)
                    .await
                    .unwrap();
                registry
                    .cancel_and_wait("blocked-native-run")
                    .await
                    .unwrap();
            })
        });
    }

    /// Cleanup failures belong to the cancellation receipt and must not overwrite a domain failure.
    #[test]
    fn business_failure_survives_a_simultaneous_cleanup_failure() {
        ora_logging::with_trace_logging(|| {
            runtime().block_on(async {
            let registry = StreamRegistry::default();
            let registration = registry.register("both-failed".to_string()).unwrap();
            let frames = Arc::new(Mutex::new(Vec::new()));
            let captured = frames.clone();
            let channel = Channel::new(move |body| {
                if let tauri::ipc::InvokeResponseBody::Json(json) = body {
                    captured.lock().unwrap().push(serde_json::from_str::<serde_json::Value>(&json).unwrap());
                }
                Ok(())
            });
            let lifecycle = RequestLifecycle::start("both_failed", &UuidRequestIdGenerator);
            let request_id = lifecycle.request_id().clone();
            let failure = ora_backend::BackendError::new(ora_backend::ErrorClassification::Conflict, ora_contracts::PublicError::SessionHistoryDegraded(ora_contracts::EmptyErrorParams {}), "history is degraded");
            let cleanup = ora_backend::BackendError::new(ora_backend::ErrorClassification::Internal, ora_contracts::PublicError::AgentRuntimeUnavailable(ora_contracts::EmptyErrorParams {}), "cleanup unconfirmed");
            super::finish_contract_stream(super::StreamEnd::Failed(failure), Err(cleanup), registration, channel, lifecycle);
            assert_eq!(*frames.lock().unwrap(), vec![serde_json::json!({"type":"error", "error":{"code":"session_history_degraded", "params":{}, "requestId":request_id}})]);
            assert_eq!(registry.cancel_and_wait("both-failed").await.unwrap_err().public_error(), &ora_contracts::PublicError::AgentRuntimeUnavailable(ora_contracts::EmptyErrorParams {}));
        })
        });
    }

    /// Builds the current-thread runtime that keeps the scoped subscriber on the test thread.
    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// Builds a Channel that behaves like a webview whose listener has already gone away.
    fn disconnected_channel(send_attempts: Arc<AtomicUsize>) -> Channel<serde_json::Value> {
        Channel::new(move |_body| {
            send_attempts.fetch_add(1, Ordering::SeqCst);
            Err(std::io::Error::other("channel closed").into())
        })
    }

    /// Builds a Channel that accepts every frame, like a webview still listening.
    fn connected_channel() -> Channel<serde_json::Value> {
        Channel::new(|_body| Ok(()))
    }

    /// Captures the `outcome` field of lifecycle completion events without global subscriber state.
    #[derive(Clone, Debug, Default)]
    struct OutcomeRecorder {
        outcomes: Arc<Mutex<Vec<String>>>,
    }

    impl OutcomeRecorder {
        /// Builds the scoped subscriber layer used by one test.
        fn layer(&self) -> OutcomeRecordingLayer {
            OutcomeRecordingLayer {
                outcomes: self.outcomes.clone(),
            }
        }

        /// Returns captured completion outcomes in emission order.
        fn outcomes(&self) -> Vec<String> {
            self.outcomes.lock().unwrap().clone()
        }
    }

    /// Records completion outcomes while leaving production formatting untouched.
    #[derive(Clone, Debug)]
    struct OutcomeRecordingLayer {
        outcomes: Arc<Mutex<Vec<String>>>,
    }

    impl<S> Layer<S> for OutcomeRecordingLayer
    where
        S: tracing::Subscriber,
    {
        /// Collects the `outcome` field, which only lifecycle completion events carry.
        fn on_event(&self, event: &tracing::Event<'_>, _context: Context<'_, S>) {
            let mut visitor = OutcomeVisitor { outcome: None };
            event.record(&mut visitor);
            if let Some(outcome) = visitor.outcome {
                self.outcomes.lock().unwrap().push(outcome);
            }
        }
    }

    /// Extracts the single `outcome` field from one recorded event.
    struct OutcomeVisitor {
        outcome: Option<String>,
    }

    impl Visit for OutcomeVisitor {
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "outcome" {
                self.outcome = Some(value.to_string());
            }
        }

        fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
    }
}
