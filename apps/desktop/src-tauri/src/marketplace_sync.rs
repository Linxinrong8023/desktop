//! Automatic marketplace index refreshes and the status they report to the main webview.
//!
//! The cached registry index is otherwise only refreshed when a user presses Sync, so an
//! installation nobody presses it in never sees a marketplace listing newer than its first run.
//! This module drives that refresh on its own: once shortly after startup, and every six hours
//! afterwards.
//!
//! A refresh only rebuilds the cached listing; it never installs or updates an installed plugin,
//! so it needs no user consent and is safe to run unattended in every build.

#[path = "../bindings/marketplace_sync.rs"]
mod binding;

use binding::AUTO_SYNC_EVENT;
use ora_backend::Plugins;
use ora_contracts::MarketplaceAutoSyncEvent;
use ora_logging::{ora_info, ora_warn};
use ora_scheduler::{BoxFuture, CronHandle, DelayHandle, Job, Scheduler, SchedulerError};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// How long after startup the first refresh runs.
///
/// Late enough to stay clear of the work a launch already owes the user - opening the backend,
/// scanning installed plugins, restoring the workspace - and early enough that the marketplace is
/// current by the time anyone opens it.
const STARTUP_SYNC_DELAY: Duration = Duration::from_secs(15);

/// Refreshes the marketplace index every six hours in the host's local time.
const MARKETPLACE_SYNC_CRON: &str = "0 0 */6 * * *";

/// Distinguishes the two automatic triggers in logs so a failed refresh can be traced back to the
/// run that produced it. Both drive an identical rebuild.
#[derive(Clone, Copy, Debug)]
enum AutoSyncTrigger {
    Startup,
    Periodic,
}

impl AutoSyncTrigger {
    /// Identifies the scheduling trigger in refresh diagnostics.
    fn as_str(self) -> &'static str {
        match self {
            AutoSyncTrigger::Startup => "startup",
            AutoSyncTrigger::Periodic => "periodic",
        }
    }
}

/// Owns the scheduler registrations that keep the marketplace index current.
///
/// Both handles are retained for the life of the application: dropping either one cancels the
/// work it represents.
pub struct MarketplaceSyncService {
    _scheduler: Scheduler,
    _startup_sync: DelayHandle,
    _periodic_sync: CronHandle,
}

impl MarketplaceSyncService {
    /// Registers the delayed startup refresh and the recurring one.
    pub fn start(
        app: AppHandle,
        plugins: Plugins,
        timezone: chrono_tz::Tz,
    ) -> Result<Self, SchedulerError> {
        let scheduler = Scheduler::new(timezone);
        let startup_app = app.clone();
        let startup_plugins = plugins.clone();
        let startup_sync = scheduler.schedule_after(STARTUP_SYNC_DELAY, async move {
            run_auto_sync(startup_app, startup_plugins, AutoSyncTrigger::Startup).await;
        })?;
        let periodic_sync = scheduler.schedule_cron(MarketplaceSyncJob { app, plugins })?;
        Ok(Self {
            _scheduler: scheduler,
            _startup_sync: startup_sync,
            _periodic_sync: periodic_sync,
        })
    }
}

/// Drives the recurring refresh on the Desktop scheduler.
struct MarketplaceSyncJob {
    app: AppHandle,
    plugins: Plugins,
}

impl Job for MarketplaceSyncJob {
    /// Returns the stable scheduler name used in logs.
    fn name(&self) -> &str {
        "marketplace-sync"
    }

    /// Runs the refresh at the local six-hour schedule.
    fn schedule(&self) -> &str {
        MARKETPLACE_SYNC_CRON
    }

    /// Executes one non-overlapping refresh.
    fn run(&self) -> BoxFuture<'_> {
        Box::pin(run_auto_sync(
            self.app.clone(),
            self.plugins.clone(),
            AutoSyncTrigger::Periodic,
        ))
    }
}

/// Claims the rebuild slot and, if it is granted, rebuilds the index between two status events.
///
/// Admission is claimed before `Started` is emitted so the shell is never told a refresh began
/// that was in fact discarded. A refresh that loses the slot needs no retry of its own: the
/// rebuild that won it produces exactly the same index.
async fn run_auto_sync(app: AppHandle, plugins: Plugins, trigger: AutoSyncTrigger) {
    // The rebuild drives the Git CLI and rewrites the index on disk, so it must never occupy a
    // scheduler worker.
    let blocking = tauri::async_runtime::spawn_blocking(move || {
        let Some(admitted) = plugins.admit_auto_sync() else {
            ora_info!(
                trigger = trigger.as_str(),
                "marketplace sync already in flight; discarded"
            );
            return;
        };
        emit_status(&app, MarketplaceAutoSyncEvent::Started);
        if let Err(error) = admitted.run() {
            ora_warn!(
                message = "automatic marketplace sync failed",
                trigger = trigger.as_str(),
                error = %error,
            );
        }
        emit_status(&app, MarketplaceAutoSyncEvent::Finished);
    });
    if let Err(error) = blocking.await {
        ora_warn!(
            message = "automatic marketplace sync did not complete",
            trigger = trigger.as_str(),
            error = %error,
        );
    }
}

/// Publishes one refresh status change to the main webview.
///
/// A webview that never receives it only loses the disabled state on its Sync button, so a failed
/// emit is logged rather than allowed to abort the refresh around it.
fn emit_status<R: tauri::Runtime>(app: &AppHandle<R>, event: MarketplaceAutoSyncEvent) {
    if let Err(error) = app.emit(AUTO_SYNC_EVENT, event) {
        ora_warn!(
            message = "failed to emit marketplace sync status",
            error = %error,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{AUTO_SYNC_EVENT, MarketplaceAutoSyncEvent, emit_status};
    use pretty_assertions::assert_eq;
    use std::sync::{Arc, Mutex};
    use tauri::Listener;

    /// Exercises Tauri serialization on the route from the shared Desktop binding.
    #[test]
    fn emits_both_refresh_states_on_the_declared_native_route() {
        let app = tauri::test::mock_app();
        let received = Arc::new(Mutex::new(Vec::new()));
        let captured = received.clone();
        let subscription = app.listen(AUTO_SYNC_EVENT, move |event| {
            captured
                .lock()
                .unwrap()
                .push(serde_json::from_str::<serde_json::Value>(event.payload()).unwrap());
        });
        emit_status(app.handle(), MarketplaceAutoSyncEvent::Started);
        emit_status(app.handle(), MarketplaceAutoSyncEvent::Finished);
        app.unlisten(subscription);
        emit_status(app.handle(), MarketplaceAutoSyncEvent::Started);
        assert_eq!(
            *received.lock().unwrap(),
            vec![
                serde_json::json!({ "kind": "started" }),
                serde_json::json!({ "kind": "finished" }),
            ],
        );
    }
}
