//! Shared stream startup, cancellation, and forwarding; domain routing is generated.

use super::stream_routes::{self, StreamOperation};
use crate::stream_forwarding::{forward_contract_stream, forward_workspace_watch};
use crate::stream_registry::StreamRegistration;
use crate::{error::CommandError, state::DesktopState};
use ora_backend::{
    BackendError, ErrorClassification, RequestLifecycle, SessionEventStream, UuidRequestIdGenerator,
};
use ora_contracts::{EmptyErrorParams, PublicError};
use std::future::Future;
use tauri::{State, ipc::Channel};
use tokio_util::sync::CancellationToken;

/// Owns a request and registration until startup transfers them to a forwarding task.
pub(super) struct StreamStart {
    registration: StreamRegistration,
    channel: Channel<serde_json::Value>,
    lifecycle: RequestLifecycle,
}

/// Distinguishes cancelled startup from a resource ready to transfer to its forwarding task.
enum Startup<T> {
    Cancelled,
    Ready(T),
}

impl StreamStart {
    /// Starts an ordered backend event source and transfers exactly one lifecycle to forwarding.
    pub(super) async fn events<T: serde::Serialize + Send + 'static>(
        self,
        source: impl Future<Output = Result<SessionEventStream<T>, BackendError>>,
    ) -> Result<(), CommandError> {
        match settle_startup(
            source,
            self.registration.cancellation(),
            &self.lifecycle,
            |mut stream: SessionEventStream<T>| async move { stream.cancel_and_wait().await },
        )
        .await
        {
            Ok(Startup::Cancelled) => self.registration.finish(Ok(())),
            Err(error) => {
                self.registration.finish(Err(error.clone()));
                return Err(CommandError::from_backend_with_lifecycle(
                    error,
                    &self.lifecycle,
                ));
            }
            Ok(Startup::Ready(stream)) => {
                tauri::async_runtime::spawn(forward_contract_stream(
                    stream,
                    self.registration,
                    self.channel,
                    self.lifecycle,
                ));
            }
        }
        Ok(())
    }

    /// Starts a native event source while using the same cancellation and request ownership rules.
    pub(super) async fn watch(
        self,
        source: impl Future<Output = Result<ora_fs::WorkspaceWatcher, BackendError>>,
    ) -> Result<(), CommandError> {
        match settle_startup(
            source,
            self.registration.cancellation(),
            &self.lifecycle,
            |watcher| async move {
                watcher
                    .close()
                    .await
                    .map_err(crate::workspace_files::workspace_file_backend_error)
            },
        )
        .await
        {
            Ok(Startup::Cancelled) => self.registration.finish(Ok(())),
            Err(error) => {
                self.registration.finish(Err(error.clone()));
                return Err(CommandError::from_backend_with_lifecycle(
                    error,
                    &self.lifecycle,
                ));
            }
            Ok(Startup::Ready(watcher)) => {
                tauri::async_runtime::spawn(forward_workspace_watch(
                    watcher,
                    self.registration,
                    self.channel,
                    self.lifecycle,
                ));
            }
        }
        Ok(())
    }
}

/// Lets started domain work settle before dropping its resource; arbitrary startup futures may
/// already have committed actor side effects and are not safe to abandon midway through creation.
async fn settle_startup<T, F: Future<Output = Result<(), BackendError>>>(
    source: impl Future<Output = Result<T, BackendError>>,
    cancellation: &CancellationToken,
    lifecycle: &RequestLifecycle,
    cleanup: impl FnOnce(T) -> F,
) -> Result<Startup<T>, BackendError> {
    if cancellation.is_cancelled() {
        lifecycle.complete_cancellation();
        return Ok(Startup::Cancelled);
    }
    let resource = source.await?;
    if cancellation.is_cancelled() {
        cleanup(resource).await?;
        lifecycle.complete_cancellation();
        Ok(Startup::Cancelled)
    } else {
        Ok(Startup::Ready(resource))
    }
}

/// Validates a typed request and claims its id before any domain startup work can run.
#[tauri::command]
pub async fn stream_contract(
    state: State<'_, DesktopState>,
    operation_name: String,
    request: serde_json::Value,
    stream_call_id: String,
    on_event: Channel<serde_json::Value>,
) -> Result<(), CommandError> {
    let lifecycle = RequestLifecycle::start(
        format!("stream_contract:{operation_name}"),
        &UuidRequestIdGenerator,
    );
    let operation = serde_json::from_value::<StreamOperation>(serde_json::json!({
        "operationName": operation_name,
        "request": request,
    }))
    .map_err(|error| {
        CommandError::from_backend_with_lifecycle(
            BackendError::new(
                ErrorClassification::InvalidRequest,
                PublicError::InvalidRequest(EmptyErrorParams {}),
                format!("invalid stream request: {error}"),
            ),
            &lifecycle,
        )
    })?;
    let registration = state
        .streams
        .register(stream_call_id)
        .map_err(|error| CommandError::from_backend_with_lifecycle(error, &lifecycle))?;
    stream_routes::start(
        state,
        operation,
        StreamStart {
            registration,
            channel: on_event,
            lifecycle,
        },
    )
    .await
}

/// Cancels starting or running streams without prematurely releasing their private ids.
#[tauri::command]
pub async fn cancel_contract_stream(
    state: State<'_, DesktopState>,
    stream_call_id: String,
) -> Result<(), CommandError> {
    let lifecycle = RequestLifecycle::start("cancel_contract_stream", &UuidRequestIdGenerator);
    state
        .streams
        .cancel_and_wait(&stream_call_id)
        .await
        .map_err(|error| CommandError::from_backend_with_lifecycle(error, &lifecycle))?;
    lifecycle.complete_success();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::stream_routes::StreamOperation;
    use super::{Startup, settle_startup};
    use ora_backend::{BackendError, RequestLifecycle, UuidRequestIdGenerator};
    use ora_logging::with_trace_logging;
    use pretty_assertions::assert_eq;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio_util::sync::CancellationToken;

    struct Resource(Arc<AtomicUsize>);

    impl Drop for Resource {
        /// Records release of a resource that completed creation after its caller cancelled.
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Pre-cancelled work is never polled; cancellation during creation waits for safe cleanup.
    #[test]
    fn cancellation_before_and_during_creation_releases_resources() {
        with_trace_logging(|| {
            tauri::async_runtime::block_on(async {
                let token = CancellationToken::new();
                token.cancel();
                let lifecycle = RequestLifecycle::start("pre_cancel", &UuidRequestIdGenerator);
                let polls = AtomicUsize::new(0);
                let result = settle_startup(
                    async {
                        polls.fetch_add(1, Ordering::SeqCst);
                        Ok::<(), BackendError>(())
                    },
                    &token,
                    &lifecycle,
                    |_resource| async move { Ok(()) },
                )
                .await
                .expect("pre-cancel succeeds");
                assert!(matches!(result, Startup::Cancelled));
                assert_eq!(polls.load(Ordering::SeqCst), 0);

                let token = CancellationToken::new();
                let drops = Arc::new(AtomicUsize::new(0));
                let lifecycle =
                    RequestLifecycle::start("cancel_during_creation", &UuidRequestIdGenerator);
                let result = settle_startup(
                    async {
                        token.cancel();
                        Ok(Resource(drops.clone()))
                    },
                    &token,
                    &lifecycle,
                    |resource| async move {
                        drop(resource);
                        Ok(())
                    },
                )
                .await
                .expect("creation settles");
                assert!(matches!(result, Startup::Cancelled));
                assert_eq!(drops.load(Ordering::SeqCst), 1);
            });
        });
    }

    /// Creation and asynchronous cleanup each hold cancellation open until their own gate releases.
    #[test]
    fn startup_cancellation_waits_for_creation_and_cleanup() {
        with_trace_logging(|| {
            tauri::async_runtime::block_on(async {
                for succeeds in [true, false] {
                    let token = CancellationToken::new();
                    let lifecycle =
                        RequestLifecycle::start("blocked_startup", &UuidRequestIdGenerator);
                    let (created, creating) = tokio::sync::oneshot::channel();
                    let (cleaned, cleaning) = tokio::sync::oneshot::channel();
                    let result = settle_startup(
                        async {
                            creating.await.expect("creation released");
                            Ok(())
                        },
                        &token,
                        &lifecycle,
                        |()| async {
                            cleaning.await.expect("cleanup released");
                            if succeeds {
                                Ok(())
                            } else {
                                Err(BackendError::internal(
                                    "cleanup failed",
                                    std::io::Error::other("fixture"),
                                ))
                            }
                        },
                    );
                    tokio::pin!(result);
                    tokio::select! { biased; _ = &mut result => panic!("creation returned early"), () = std::future::ready(()) => {} }
                    token.cancel();
                    tokio::select! { biased; _ = &mut result => panic!("cancel abandoned creation"), () = std::future::ready(()) => {} }
                    created.send(()).expect("release creation");
                    tokio::select! { biased; _ = &mut result => panic!("cleanup returned early"), () = std::future::ready(()) => {} }
                    cleaned.send(()).expect("release cleanup");
                    assert_eq!(matches!(result.await, Ok(Startup::Cancelled)), succeeds);
                }
            })
        });
    }

    /// Startup cancellation must wait for the native callback and watcher resources to retire.
    #[test]
    fn startup_watch_cancellation_waits_for_native_release() {
        with_trace_logging(|| {
            tauri::async_runtime::block_on(async {
                let workspace = tempfile::TempDir::new().unwrap();
                let (watcher, gate) =
                    ora_fs::watch_test_support::blocked_watcher(workspace.path()).unwrap();
                std::fs::write(workspace.path().join("trigger"), "event").unwrap();
                gate.wait_until_entered().unwrap();
                let registry = crate::stream_registry::StreamRegistry::default();
                let registration = registry
                    .register("blocked-native-start".to_string())
                    .unwrap();
                let token = registration.cancellation().clone();
                let started = super::StreamStart {
                    registration,
                    channel: tauri::ipc::Channel::new(|_| Ok(())),
                    lifecycle: RequestLifecycle::start(
                        "blocked_native_start",
                        &UuidRequestIdGenerator,
                    ),
                }
                .watch(async {
                    token.cancel();
                    Ok(watcher)
                });
                tokio::pin!(started);
                let early =
                    tokio::time::timeout(std::time::Duration::from_millis(100), &mut started).await;
                assert!(
                    early.is_err(),
                    "startup confirmed while native callback was still blocked"
                );
                assert!(
                    tokio::time::timeout(
                        std::time::Duration::from_millis(100),
                        registry.cancel_and_wait("blocked-native-start")
                    )
                    .await
                    .is_err()
                );
                gate.release();
                tokio::time::timeout(std::time::Duration::from_secs(10), &mut started)
                    .await
                    .unwrap()
                    .unwrap();
                registry
                    .cancel_and_wait("blocked-native-start")
                    .await
                    .unwrap();
            })
        });
    }

    /// A creation failure remains a correlated failure even when cancellation races with it.
    #[test]
    fn startup_failure_preserves_the_request_id() {
        with_trace_logging(|| {
            tauri::async_runtime::block_on(async {
                let token = CancellationToken::new();
                let lifecycle = RequestLifecycle::start("failed_start", &UuidRequestIdGenerator);
                let result = settle_startup(
                    async {
                        token.cancel();
                        Err::<(), _>(BackendError::internal(
                            "fixture startup",
                            std::io::Error::other("failed"),
                        ))
                    },
                    &token,
                    &lifecycle,
                    |_resource| async move { Ok(()) },
                )
                .await;
                let Err(error) = result else {
                    panic!("startup must fail");
                };
                assert_eq!(
                    serde_json::to_value(super::CommandError::from_backend_with_lifecycle(
                        error, &lifecycle
                    ))
                    .expect("serialize public error"),
                    serde_json::json!({
                        "code": "internal_error",
                        "params": {},
                        "requestId": lifecycle.request_id(),
                    })
                );
            });
        });
    }

    /// The generated decoder accepts only declared stream operations and their actual DTO shape.
    #[test]
    fn generated_stream_requests_reject_unknown_operations_and_bad_payloads() {
        assert!(matches!(
            serde_json::from_value::<StreamOperation>(
                serde_json::json!({"operationName": "watchProject", "request": {"projectId": "fixture"}})
            ),
            Ok(StreamOperation::WatchProject(_))
        ));
        assert!(
            serde_json::from_value::<StreamOperation>(
                serde_json::json!({"operationName": "notDeclared", "request": {}})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<StreamOperation>(
                serde_json::json!({"operationName": "watchProject", "request": {}})
            )
            .is_err()
        );
    }
}
