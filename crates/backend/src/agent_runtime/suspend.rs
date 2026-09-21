//! Suspends and resumes agent supervision around package replacement operations.
//!
//! Updating or uninstalling a plugin retires version directories that the plugin process keeps
//! open as its working directory; on Windows those directory handles block the replacement. The
//! suspend/resume pair lets the plugin operations wrappers stop the supervisor's respawn loop for
//! the operation's duration. Process termination helpers live here too, because ending a
//! generation and suspending its supervisor are two halves of the same boundary.

use super::{AgentRuntimeManager, CANCELLATION_GRACE};
use crate::plugin::PluginApi;
use ora_contracts::StopPluginRequest;
use ora_domain::{AgentRef, PluginId};
use ora_logging::ora_warn;
use ora_plugin_runtime::PluginRuntime;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::time::timeout;

/// Flags agent identities whose supervisor must not (re)spawn its process right now.
///
/// Plugin package replacement has to retire version directories that the plugin process holds
/// open as its working directory; on Windows that directory handle blocks the replacement. A
/// suspended supervisor finishes its current generation and exits instead of scheduling the
/// restart it would normally run, so the package replacement sees a process that is gone and
/// stays gone until the flag is lifted and the supervisor is recreated.
pub(super) type SuspendedAgents = Arc<Mutex<BTreeSet<AgentRef>>>;

/// Reports whether ` + [char]96 + gent_ref + [char]96 +  is currently suspended from spawning.
pub(super) fn is_agent_suspended(suspended: &SuspendedAgents, agent_ref: &AgentRef) -> bool {
    suspended
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(agent_ref)
}

/// Ends the plugin process backing one connection generation once that generation is over.
///
/// The process belongs to the plugin lifecycle rather than to this module: a connection only
/// borrowed its ACP stream, so ending the generation means telling the lifecycle to stop it, which
/// keeps the runtime state the settings surface reports honest and leaves the next attach to start
/// a fresh process.
pub(super) struct AgentProcess {
    pub(super) plugin_id: PluginId,
    pub(super) runtime: PluginRuntime,
    pub(super) host: Arc<PluginApi>,
}

impl AgentProcess {
    /// Reaps a failed generation before its replacement so two generations cannot overlap.
    pub(super) async fn terminate_and_reap(&self) {
        // Stopping the agent is the plugin's chance to reap the agent process it owns before the
        // lifecycle ends the plugin process itself.
        super::plugin_agent::stop_agent(&self.runtime, &self.plugin_id.canonical()).await;
        stop_plugin_runtime(&self.host, &self.plugin_id).await;
    }

    /// Bounds application shutdown even when the operating system does not promptly reap a child.
    pub(super) async fn stop_with_grace(&self) {
        let _ = timeout(CANCELLATION_GRACE, self.terminate_and_reap()).await;
    }
}

/// Asks the lifecycle to end one plugin process after its agent generation failed or shut down.
///
/// A stop that itself fails is logged rather than propagated: the caller is already tearing a
/// generation down, and the next attach restarts the plugin regardless of what this left behind.
pub(super) async fn stop_plugin_runtime(host: &PluginApi, plugin_id: &PluginId) {
    if let Err(error) = host
        .stop(StopPluginRequest {
            plugin_id: plugin_id.to_string(),
        })
        .await
    {
        ora_warn!(
            plugin_id = %plugin_id,
            error = %error,
            "plugin runtime could not be stopped after its agent generation ended"
        );
    }
}

impl AgentRuntimeManager {
    /// Bars the plugin agent's supervisor from respawning its process.
    ///
    /// Package replacement retires version directories the plugin process holds open as its
    /// working directory; on Windows those handles block the replacement, so the supervisor is
    /// suspended for the operation's duration.
    pub(crate) fn suspend_plugin_agent(&self, plugin_id: &str) {
        self.inner.connections.suspend_plugin_agent(plugin_id);
    }

    /// Lifts a supervisor suspension and drops the supervisor so the next reconciliation starts
    /// fresh against the package version that is now installed.
    pub(crate) fn resume_plugin_agent(&self, plugin_id: &str) {
        self.inner.connections.resume_plugin_agent(plugin_id);
    }
}
