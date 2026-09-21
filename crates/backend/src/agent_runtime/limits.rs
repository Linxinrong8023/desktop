use crate::session_setup::SessionMcpSnapshot;
use std::time::Duration;

pub(super) const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(15);
pub(super) const SESSION_SETUP_TIMEOUT: Duration = Duration::from_secs(30);
/// Inactivity window for one ACP setup request that delivers MCP servers.
///
/// The ACP session-setup sequence has the agent connect the delivered servers before it answers,
/// and real MCP initialization — cold stdio package boots, remote HTTP handshakes — routinely
/// takes tens of seconds, so a setup carrying servers waits far longer than a bare one before
/// `agent_timed_out` fires. The value matches the widest single prompt inactivity window, the
/// longest the runtime already tolerates waiting on external work.
pub(super) const SESSION_SETUP_MCP_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const CANCELLATION_GRACE: Duration = Duration::from_secs(5);
/// Meaningful-activity windows for one prompt: the first send, then each automatic
/// retry after the previous window expired without progress.
///
/// Every retry re-sends the prompt as a fresh LLM turn, so the windows widen rather
/// than repeat: a stall that outlived 45 s is more likely an upstream slowdown than a
/// glitch, and a wider window avoids cancelling a slow-but-alive retry while limiting
/// how many duplicate turns a wedged agent is asked for.
pub(super) const PROMPT_INACTIVITY_WINDOWS: [Duration; 4] = [
    Duration::from_secs(45),
    Duration::from_secs(60),
    Duration::from_secs(90),
    Duration::from_secs(120),
];
pub(super) const CONTRACT_QUEUE_CAPACITY: usize = 256;
pub(super) const MAX_PROMPT_BYTES: usize = 16 * 1024 * 1024;

/// Picks the inactivity window for one ACP session setup or refresh request.
///
/// A request that carries no servers is bare session-context work and keeps the plain window; one
/// that delivers a snapshot with servers gets the wider MCP window, because the agent is expected
/// to connect those servers before responding.
pub(super) fn session_setup_window(mcp: &SessionMcpSnapshot) -> Duration {
    if mcp.servers().is_empty() {
        SESSION_SETUP_TIMEOUT
    } else {
        SESSION_SETUP_MCP_TIMEOUT
    }
}

#[cfg(test)]
mod tests {
    use super::{SESSION_SETUP_MCP_TIMEOUT, SESSION_SETUP_TIMEOUT, session_setup_window};
    use crate::session_setup::{
        SessionMcpMemberRevision, SessionMcpRevision, SessionMcpSnapshot, SessionMcpTransportKind,
    };
    use agent_client_protocol_schema::v1::{McpServer, McpServerStdio};
    use ora_domain::PluginId;
    use pretty_assertions::assert_eq;
    use semver::Version;

    /// Builds one coherent snapshot that delivers a single stdio server.
    fn delivering_snapshot() -> SessionMcpSnapshot {
        let plugin_id = PluginId::parse("official/tavily").expect("plugin id");
        SessionMcpSnapshot::new(
            vec![McpServer::Stdio(McpServerStdio::new(
                plugin_id.canonical(),
                "tavily-server.cmd",
            ))],
            SessionMcpRevision::new(vec![SessionMcpMemberRevision {
                plugin_id,
                package_version: Version::new(1, 0, 0),
                configuration_revision: 1,
                transport: SessionMcpTransportKind::Stdio,
            }]),
        )
    }

    /// A setup with no servers keeps the plain window: nothing obliges the agent to connect.
    #[test]
    fn setup_without_servers_keeps_the_plain_window() {
        let empty = SessionMcpSnapshot::new(Vec::new(), SessionMcpRevision::default());
        assert_eq!(session_setup_window(&empty), SESSION_SETUP_TIMEOUT);
    }

    /// A setup that delivers servers gets the wider MCP window.
    #[test]
    fn setup_delivering_servers_gets_the_mcp_window() {
        assert_eq!(
            session_setup_window(&delivering_snapshot()),
            SESSION_SETUP_MCP_TIMEOUT,
        );
    }
}
