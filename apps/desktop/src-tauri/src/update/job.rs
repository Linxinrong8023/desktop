//! Adapts the update service to the scheduler's recurring job contract.

use super::service::UpdateService;
use ora_scheduler::{BoxFuture, Job};

/// Checks for a new release every six hours in the host's local time.
const UPDATE_CRON: &str = "0 0 */6 * * *";

/// Runs the recurring release check on the Desktop scheduler.
pub(super) struct UpdateJob {
    service: UpdateService,
}

impl UpdateJob {
    /// Binds the job to the service that owns the update state machine.
    pub(super) fn new(service: UpdateService) -> Self {
        Self { service }
    }
}

impl Job for UpdateJob {
    /// Returns the stable scheduler name used in logs.
    fn name(&self) -> &str {
        "desktop-update"
    }

    /// Runs the check at the local six-hour schedule.
    fn schedule(&self) -> &str {
        UPDATE_CRON
    }

    /// Executes one non-overlapping update check.
    fn run(&self) -> BoxFuture<'_> {
        Box::pin(self.service.check_and_download())
    }
}
