//! Owns stream registrations from startup through forwarding and application shutdown.

use ora_backend::{BackendError, ErrorClassification};
use ora_contracts::{EmptyErrorParams, PublicError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// Shares cancellation with the command, its startup work, and the eventual forwarding task.
#[derive(Clone, Default)]
pub(crate) struct StreamRegistry {
    inner: Arc<RegistryState>,
}

type CleanupReceipt = watch::Receiver<Option<Result<(), BackendError>>>;

#[derive(Default)]
struct RegistryState {
    registrations: Mutex<HashMap<String, (CancellationToken, CleanupReceipt)>>,
    shutdown: CancellationToken,
}

/// Holds exclusive ownership of a stream id until startup or forwarding releases it.
pub(crate) struct StreamRegistration {
    cancellation: CancellationToken,
    completion: watch::Sender<Option<Result<(), BackendError>>>,
}

impl StreamRegistry {
    /// Claims an id before startup so duplicate or cancelled calls cannot create untracked work.
    pub(crate) fn register(&self, id: String) -> Result<StreamRegistration, BackendError> {
        let mut registrations = self.inner.registrations.lock().map_err(|_poisoned| {
            BackendError::internal(
                "stream registry is unavailable",
                std::io::Error::other("registry lock poisoned"),
            )
        })?;
        if self.inner.shutdown.is_cancelled() {
            return Err(BackendError::new(
                ErrorClassification::InvalidRequest,
                PublicError::InvalidRequest(EmptyErrorParams {}),
                "Desktop streams are shutting down",
            ));
        }
        if registrations
            .get(&id)
            .is_some_and(|(_, result)| result.borrow().is_none())
        {
            // The id names a live registration, so the caller must retry under a fresh one; the
            // public code has to say that rather than claim the request itself was malformed.
            return Err(BackendError::new(
                ErrorClassification::Conflict,
                PublicError::ResourceInUse(EmptyErrorParams {}),
                "stream call id is already registered",
            ));
        }
        let cancellation = self.inner.shutdown.child_token();
        // Keep a bounded receipt cache. Evicted and unknown ids cannot claim cleanup success.
        if registrations.len() >= 256 {
            registrations.retain(|_, (_, result)| result.borrow().is_none());
        }
        let (completion, result) = watch::channel(None);
        registrations.insert(id.clone(), (cancellation.clone(), result));
        Ok(StreamRegistration {
            cancellation,
            completion,
        })
    }

    /// Signals cancellation without releasing the id while older startup or forwarding still owns it.
    #[cfg(test)]
    pub(crate) fn cancel(&self, id: &str) -> Result<(), BackendError> {
        let registrations = self.inner.registrations.lock().map_err(|_poisoned| {
            BackendError::internal(
                "stream registry is unavailable",
                std::io::Error::other("registry lock poisoned"),
            )
        })?;
        if let Some((cancellation, _)) = registrations.get(id) {
            cancellation.cancel();
        }
        Ok(())
    }

    /// Waits at most thirty seconds for the same registration's explicit cleanup receipt.
    /// Timeout stops only this waiter; cleanup and other waiters remain active.
    pub(crate) async fn cancel_and_wait(&self, id: &str) -> Result<(), BackendError> {
        let mut completion = {
            let registrations = self.inner.registrations.lock().map_err(|_| {
                BackendError::internal(
                    "stream registry unavailable",
                    std::io::Error::other("poisoned"),
                )
            })?;
            let (cancellation, completion) = registrations.get(id).ok_or_else(|| {
                BackendError::internal(
                    "stream cleanup cannot be confirmed",
                    std::io::Error::other("unknown or expired stream id"),
                )
            })?;
            cancellation.cancel();
            completion.clone()
        };
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                if let Some(result) = completion.borrow().clone() {
                    return result;
                }
                completion.changed().await.map_err(|error| {
                    BackendError::internal("stream cleanup owner disappeared", error)
                })?;
            }
        })
        .await
        .map_err(|error| {
            BackendError::internal(
                "stream cleanup wait timed out; cleanup remains active",
                error,
            )
        })?
    }

    /// Cancels both starting and running streams and rejects every later registration.
    pub(crate) fn shutdown(&self) {
        self.inner.shutdown.cancel();
    }
}

impl StreamRegistration {
    /// Records the owner's result only after its resources and restoration have settled.
    pub(crate) fn finish(self, result: Result<(), BackendError>) {
        self.completion.send_replace(Some(result));
    }

    /// Lets startup and forwarding observe the same cancellation, including application shutdown.
    pub(crate) fn cancellation(&self) -> &CancellationToken {
        &self.cancellation
    }
}

impl Drop for StreamRegistration {
    /// Releases only this owner's id; cancellation deliberately cannot make it reusable earlier.
    fn drop(&mut self) {
        self.cancellation.cancel();
        if self.completion.borrow().is_none() {
            self.completion
                .send_replace(Some(Err(BackendError::internal(
                    "stream cleanup owner exited without confirmation",
                    std::io::Error::other("cleanup was not acknowledged"),
                ))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StreamRegistry;
    use ora_backend::ErrorClassification;
    use ora_logging::with_trace_logging;
    use pretty_assertions::assert_eq;

    /// A duplicate id is contended state, so its classification and public code must agree.
    #[test]
    fn duplicate_registration_reports_a_conflicting_resource() {
        with_trace_logging(|| {
            let registry = StreamRegistry::default();
            let _registration = registry
                .register("fixture".to_string())
                .expect("register stream");

            let Err(error) = registry.register("fixture".to_string()) else {
                panic!("duplicate id must be rejected");
            };

            assert_eq!(
                (error.classification(), error.public_error().code()),
                (ErrorClassification::Conflict, "resource_in_use")
            );
        });
    }

    /// A cancelled creator retains its id until its resources have actually been released.
    #[test]
    fn cancellation_during_startup_cannot_release_another_generation() {
        with_trace_logging(|| {
            let registry = StreamRegistry::default();
            let registration = registry
                .register("fixture".to_string())
                .expect("register stream");
            assert!(registry.register("fixture".to_string()).is_err());
            registry.cancel("fixture").expect("cancel during startup");
            assert!(registration.cancellation().is_cancelled());
            assert!(registry.register("fixture".to_string()).is_err());
            drop(registration);
            let replacement = registry
                .register("fixture".to_string())
                .expect("reuse released id");
            assert!(!replacement.cancellation().is_cancelled());
        });
    }

    /// Shutdown reaches every phase and a cancelled runtime cannot accept fresh subscriptions.
    #[test]
    fn shutdown_cancels_all_registrations_and_prevents_reopening() {
        with_trace_logging(|| {
            let registry = StreamRegistry::default();
            let starting = registry
                .register("starting".to_string())
                .expect("starting stream");
            let running = registry
                .register("running".to_string())
                .expect("running stream");
            registry.shutdown();
            assert!(starting.cancellation().is_cancelled());
            assert!(running.cancellation().is_cancelled());
            assert!(registry.register("later".to_string()).is_err());
            registry
                .cancel("already-finished")
                .expect("late cancellation is idempotent");
        });
    }
    /// Concurrent and repeated cancellation wait for the owner's receipt, including failures.
    #[tokio::test]
    async fn cancellation_receipt_waits_and_repeats() {
        for succeeds in [true, false] {
            let registry = StreamRegistry::default();
            let registration = registry.register("receipt".to_string()).expect("register");
            let first = registry.cancel_and_wait("receipt");
            let second = registry.cancel_and_wait("receipt");
            tokio::pin!(first, second);
            tokio::select! { biased; _ = &mut first => panic!("premature receipt"), () = std::future::ready(()) => {} }
            tokio::select! { biased; _ = &mut second => panic!("premature repeated receipt"), () = std::future::ready(()) => {} }
            assert!(registration.cancellation().is_cancelled());
            registration.finish(if succeeds {
                Ok(())
            } else {
                Err(ora_backend::BackendError::internal(
                    "cleanup failed",
                    std::io::Error::other("fixture"),
                ))
            });
            assert_eq!(
                (
                    first.await.is_ok(),
                    second.await.is_ok(),
                    registry.cancel_and_wait("receipt").await.is_ok()
                ),
                (succeeds, succeeds, succeeds)
            );
        }
    }

    /// Unwinding the owner or querying an unknown id cannot synthesize cleanup success.
    #[tokio::test]
    async fn missing_confirmation_is_an_error() {
        let registry = StreamRegistry::default();
        let registration = registry.register("lost".to_string()).expect("register");
        drop(registration);
        assert!(registry.cancel_and_wait("lost").await.is_err());
        assert!(registry.cancel_and_wait("unknown").await.is_err());
    }
    /// A deadline reports failure while keeping the registration and its eventual receipt alive.
    #[tokio::test(start_paused = true)]
    async fn timeout_does_not_abort_cleanup_or_release_the_id() {
        let registry = StreamRegistry::default();
        let registration = registry.register("blocked".to_string()).expect("register");
        let error = registry
            .cancel_and_wait("blocked")
            .await
            .expect_err("deadline expires");
        assert!(error.to_string().contains("timed out"));
        assert!(registration.cancellation().is_cancelled());
        assert!(registry.register("blocked".to_string()).is_err());
        registration.finish(Ok(()));
        registry
            .cancel_and_wait("blocked")
            .await
            .expect("later confirmation");
    }
}
