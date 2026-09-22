use super::*;
use serde::{Deserialize, Serialize};

/// Deployment state for the executable: the shared runtime configuration, the API's dispatch target
/// and the optional Node this process may host. Listener and hosting choices stay on the command line.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentConfig {
    pub controller: RuntimeConfig,
    pub api: ApiConfig,
    pub single_node: Option<SingleNodeConfig>,
}

/// Callers never choose a Node; deployment fixes the single dispatch target of the transitional API.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiConfig {
    pub node_id: NodeId,
}

/// How to start the one configured Node when `--single-node` is given; host and guardian are prerequisites.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SingleNodeConfig {
    pub node_executable: PathBuf,
    pub node_config: PathBuf,
    pub ready_timeout_ms: u64,
    pub stop_timeout_ms: u64,
}

/// Whether this process starts the configured Node or only connects to an externally managed one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeHosting {
    Managed,
    External,
}

impl DeploymentConfig {
    /// Rejects an unknown API target or an incomplete hosting request before any state is opened.
    pub fn validate(&self, hosting: NodeHosting) -> Result<Option<&SingleNodeConfig>, Error> {
        if !self
            .controller
            .nodes
            .iter()
            .any(|node| node.node_id == self.api.node_id)
        {
            return Err(Error::Configuration(
                "api.node_id must name a configured Node".into(),
            ));
        }
        match hosting {
            NodeHosting::External => Ok(None),
            NodeHosting::Managed => {
                let single = self.single_node.as_ref().ok_or_else(|| {
                    Error::Configuration("--single-node requires a single_node section".into())
                })?;
                // Hosting is only meaningful for exactly the Node the API dispatches to.
                if self.controller.nodes.len() != 1
                    || single.ready_timeout_ms == 0
                    || single.stop_timeout_ms == 0
                    || !single.node_executable.is_absolute()
                    || !single.node_config.is_absolute()
                {
                    return Err(Error::Configuration(
                        "single_node requires exactly one configured Node, absolute paths and nonzero timeouts".into(),
                    ));
                }
                Ok(Some(single))
            }
        }
    }
}
