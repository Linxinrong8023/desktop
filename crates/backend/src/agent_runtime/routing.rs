use super::connection::RuntimeConnection;
use crate::BackendError;
use ora_acp::PermissionRequest;
use ora_contracts::acp::notification::SessionNotification;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;

/// Carries low-volume session controls independently from bounded update traffic.
pub(super) enum SessionControl {
    Permission(PermissionRequest),
    ConnectionLost(BackendError),
    UpdateOverflow,
}

/// Owns one session's generation-bound routes on the shared ACP connection.
pub(super) struct SessionChannel {
    pub connection: RuntimeConnection,
    pub updates: mpsc::Receiver<SessionNotification>,
    pub controls: mpsc::UnboundedReceiver<SessionControl>,
    pub(super) _registration: RouteRegistration,
}

#[derive(Default)]
pub(super) struct RouteRegistry {
    entries: RwLock<HashMap<String, RouteEntry>>,
    next_token: AtomicU64,
}

struct RouteEntry {
    generation: u64,
    token: u64,
    updates: mpsc::Sender<SessionNotification>,
    controls: mpsc::UnboundedSender<SessionControl>,
}

impl RouteRegistry {
    /// Installs a route token so a stale actor cannot unregister a newer generation.
    pub(super) fn register(
        self: &Arc<Self>,
        session_id: &str,
        generation: u64,
        updates: mpsc::Sender<SessionNotification>,
        controls: mpsc::UnboundedSender<SessionControl>,
    ) -> RouteRegistration {
        let token = self.next_token.fetch_add(1, Ordering::Relaxed) + 1;
        self.write_entries().insert(
            session_id.to_string(),
            RouteEntry {
                generation,
                token,
                updates,
                controls,
            },
        );
        RouteRegistration {
            session_id: session_id.to_string(),
            token,
            registry: self.clone(),
        }
    }

    /// Routes one high-volume update without allowing a slow session to poison the connection.
    pub(super) fn route_update(&self, update: SessionNotification) {
        let session_id = update.session_id.to_string();
        let delivery = self
            .read_entries()
            .get(&session_id)
            .map(|entry| (entry.token, entry.updates.try_send(update)));
        match delivery {
            Some((token, Err(TrySendError::Full(_)))) => {
                if let Some(entry) = self.remove_route(&session_id, token) {
                    let _ = entry.controls.send(SessionControl::UpdateOverflow);
                }
            }
            Some((token, Err(TrySendError::Closed(_)))) => {
                self.remove_route(&session_id, token);
            }
            Some((_, Ok(()))) | None => {}
        }
    }

    /// Routes a permission request or returns it so the supervisor can reject an orphan safely.
    pub(super) fn route_permission(
        &self,
        permission: PermissionRequest,
    ) -> Result<(), Box<PermissionRequest>> {
        let session_id = permission.request.session_id.to_string();
        match self.read_entries().get(&session_id) {
            Some(entry)
                if entry
                    .controls
                    .send(SessionControl::Permission(permission.clone()))
                    .is_ok() =>
            {
                Ok(())
            }
            Some(_) | None => Err(Box::new(permission)),
        }
    }

    /// Invalidates every route owned by a failed connection generation.
    pub(super) fn fail_generation(&self, generation: u64, error: BackendError) {
        let failed = {
            let mut entries = self.write_entries();
            let all = std::mem::take(&mut *entries);
            let (failed, retained): (HashMap<_, _>, HashMap<_, _>) = all
                .into_iter()
                .partition(|(_, entry)| entry.generation == generation);
            *entries = retained;
            failed
        };
        for entry in failed.into_values() {
            let _ = entry
                .controls
                .send(SessionControl::ConnectionLost(error.clone()));
        }
    }

    /// Removes only the route observed by a delivery attempt, preserving a newer registration.
    fn remove_route(&self, session_id: &str, token: u64) -> Option<RouteEntry> {
        let mut entries = self.write_entries();
        if entries
            .get(session_id)
            .is_some_and(|entry| entry.token == token)
        {
            entries.remove(session_id)
        } else {
            None
        }
    }

    /// Recovers a poisoned read lock because route loss is safer than crashing the supervisor.
    fn read_entries(&self) -> std::sync::RwLockReadGuard<'_, HashMap<String, RouteEntry>> {
        self.entries.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Recovers a poisoned write lock so connection recovery can still invalidate stale routes.
    fn write_entries(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<String, RouteEntry>> {
        self.entries.write().unwrap_or_else(PoisonError::into_inner)
    }
}

pub(super) struct RouteRegistration {
    session_id: String,
    token: u64,
    registry: Arc<RouteRegistry>,
}

impl Drop for RouteRegistration {
    fn drop(&mut self) {
        self.registry.remove_route(&self.session_id, self.token);
    }
}

#[cfg(test)]
mod tests {
    use super::{RouteRegistry, SessionControl};
    use ora_contracts::acp::notification::SessionNotification;
    use ora_contracts::acp::session::{SessionInfoUpdate, SessionUpdate};
    use pretty_assertions::assert_eq;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Verifies central routing keeps concurrent session update streams isolated.
    #[tokio::test]
    async fn routes_updates_only_to_the_matching_session() {
        let routes = Arc::new(RouteRegistry::default());
        let (first_updates, mut first_receiver) = mpsc::channel(1);
        let (first_controls, _first_controls_receiver) = mpsc::unbounded_channel();
        let (second_updates, mut second_receiver) = mpsc::channel(1);
        let (second_controls, _second_controls_receiver) = mpsc::unbounded_channel();
        let _first = routes.register("session-1", 1, first_updates, first_controls);
        let _second = routes.register("session-2", 1, second_updates, second_controls);
        let update = SessionNotification::new(
            "session-1",
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new().title("First")),
        );

        routes.route_update(update.clone());

        assert_eq!(first_receiver.recv().await, Some(update));
        assert!(second_receiver.try_recv().is_err());
    }

    /// Verifies one slow session is detached without invalidating unrelated routes.
    #[tokio::test]
    async fn isolates_a_session_whose_update_queue_overflows() {
        let routes = Arc::new(RouteRegistry::default());
        let (updates, _updates_receiver) = mpsc::channel(1);
        let (controls, mut controls_receiver) = mpsc::unbounded_channel();
        let _registration = routes.register("session-1", 1, updates, controls);
        let update = SessionNotification::new(
            "session-1",
            SessionUpdate::SessionInfoUpdate(SessionInfoUpdate::new()),
        );

        routes.route_update(update.clone());
        routes.route_update(update);

        assert!(matches!(
            controls_receiver.recv().await,
            Some(SessionControl::UpdateOverflow)
        ));
        assert!(!routes.read_entries().contains_key("session-1"));
    }

    /// Verifies a connection failure invalidates only routes from its generation.
    #[tokio::test]
    async fn invalidates_only_the_failed_connection_generation() {
        let routes = Arc::new(RouteRegistry::default());
        let (old_updates, _old_updates_receiver) = mpsc::channel(1);
        let (old_controls, mut old_controls_receiver) = mpsc::unbounded_channel();
        let (new_updates, _new_updates_receiver) = mpsc::channel(1);
        let (new_controls, mut new_controls_receiver) = mpsc::unbounded_channel();
        let _old = routes.register("old", 1, old_updates, old_controls);
        let _new = routes.register("new", 2, new_updates, new_controls);
        let error = super::super::runtime_internal("agent_runtime_unavailable", "connection lost");

        routes.fail_generation(1, error.clone());

        match old_controls_receiver.recv().await {
            Some(SessionControl::ConnectionLost(received)) => {
                assert_eq!(received.classification(), error.classification());
                assert_eq!(received.public_error(), error.public_error());
            }
            Some(SessionControl::Permission(_)) | Some(SessionControl::UpdateOverflow) | None => {
                panic!("expected connection loss")
            }
        }
        assert!(new_controls_receiver.try_recv().is_err());
        assert!(routes.read_entries().contains_key("new"));
    }
}
