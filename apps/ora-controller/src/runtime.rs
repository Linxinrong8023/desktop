use super::*;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

/// Shared deployment configuration for the standalone executable and embedded HTTP composition.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub home_directory: PathBuf,
    pub protected_state_directories: Vec<PathBuf>,
    pub controller_id: ControllerId,
    pub nodes: Vec<NodeEndpoint>,
    pub session: SessionConfig,
    pub reconnect_ms: u64,
    pub timezone: String,
}

/// Owns deployment and the reconnect lifetime; callers supply their own process shutdown signal.
pub struct ControllerRuntime {
    handle: ControllerHandle,
    config: RuntimeConfig,
}

/// Narrow application access to the single Controller owner; SQLite runs on a blocking executor.
#[derive(Clone)]
pub struct ControllerHandle {
    owner: Arc<Mutex<Controller>>,
    nodes: Arc<Vec<NodeId>>,
}

impl ControllerRuntime {
    /// Validates deployment before opening state, preserving protected roots and exclusive ownership.
    pub fn open(config: RuntimeConfig) -> Result<Self, Error> {
        if !config.home_directory.is_absolute()
            || config.reconnect_ms == 0
            || config.session.query_interval_ms == 0
            || config.session.io_timeout_ms == 0
        {
            return Err(Error::InvalidStorage);
        }
        for (index, node) in config.nodes.iter().enumerate() {
            if node.node_id.as_str().trim().is_empty()
                || !node.endpoint.is_absolute()
                || config.nodes[..index]
                    .iter()
                    .any(|other| other.node_id == node.node_id || other.endpoint == node.endpoint)
            {
                return Err(Error::Conflict);
            }
        }
        let home = ora_utils::path::canonicalize_longest_existing_prefix(&config.home_directory);
        for root in config
            .protected_state_directories
            .iter()
            .map(PathBuf::as_path)
            .chain(
                config
                    .nodes
                    .iter()
                    .filter_map(|node| node.endpoint.parent()),
            )
        {
            if !root.is_absolute() {
                return Err(Error::InvalidStorage);
            }
            let root = ora_utils::path::canonicalize_longest_existing_prefix(root);
            if home.starts_with(&root) || root.starts_with(&home) {
                return Err(Error::InvalidStorage);
            }
        }
        let owner = Controller::open(&config.home_directory, config.controller_id.clone())?;
        let nodes = Arc::new(
            config
                .nodes
                .iter()
                .map(|node| node.node_id.clone())
                .collect(),
        );
        Ok(Self {
            handle: ControllerHandle {
                owner: Arc::new(Mutex::new(owner)),
                nodes,
            },
            config,
        })
    }

    /// Supplies application access without exposing the database, mutex or reconnect implementation.
    pub fn handle(&self) -> ControllerHandle {
        self.handle.clone()
    }

    /// Reconnects configured Nodes until shutdown; cancellation drops the JoinSet and aborts every session.
    pub async fn run(&self, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        let mut sessions = tokio::task::JoinSet::new();
        for target in self.config.nodes.clone() {
            let owner = self.handle.owner.clone();
            let settings = self.config.session.clone();
            let delay = Duration::from_millis(self.config.reconnect_ms);
            sessions.spawn(async move {
                loop {
                    if run_session(&owner, &target, &settings).await.is_err() { ora_logging::ora_warn!(node_id = %target.node_id.as_str(), "Controller connection unavailable; original execution responsibility retained"); }
                    tokio::time::sleep(delay).await;
                }
            });
        }
        ora_logging::ora_info!("Controller recovery started");
        let result = tokio::select! {
            _ = shutdown => Ok(()),
            result = sessions.join_next(), if !sessions.is_empty() => Err(io::Error::other(format!("Controller session task stopped: {result:?}"))),
        };
        sessions.abort_all();
        while sessions.join_next().await.is_some() {}
        result
    }
}

impl ControllerHandle {
    /// Executes a short durable operation on a blocking executor while sharing the sole owner.
    async fn access<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Controller) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let owner = self.owner.clone();
        tokio::task::spawn_blocking(move || {
            let mut owner = owner
                .lock()
                .map_err(|_| io::Error::other("Controller lock poisoned"))?;
            action(&mut owner)
        })
        .await
        .map_err(|error| io::Error::other(error.to_string()))?
    }

    /// Accepts only a deployment-configured target before any Node dispatch observes the operation.
    pub async fn accept_clone(
        &self,
        request: RequestId,
        spec: CloneExecutionSpec,
    ) -> Result<CloneRepositoryMessage, Error> {
        if !self.nodes.contains(&spec.node_id) {
            return Err(Error::Conflict);
        }
        self.access(move |owner| owner.accept_clone(request, spec))
            .await
    }

    /// Returns accepted operations, including pending responsibility while Nodes are disconnected.
    pub async fn operations(&self) -> Result<Vec<CloneOperation>, Error> {
        self.access(|owner| owner.operations()).await
    }

    /// Reads one operation without confusing missing identity with an unknown terminal result.
    pub async fn operation(&self, execution: ExecutionId) -> Result<Option<CloneOperation>, Error> {
        self.access(move |owner| owner.operation(&execution)).await
    }
}
