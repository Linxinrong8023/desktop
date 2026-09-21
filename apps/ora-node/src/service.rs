//! One blocking execution owner and an independently responsive, bounded local control session.
mod session;
mod worker;
use crate::{CloneConfig, NodeConfig, ProcessConfig, Shutdown};
use ora_node_protocol::*;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use tokio::sync::oneshot;

/// Local transport is explicitly configured; it neither authenticates peers nor changes ownership.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpcConfig {
    pub controller_id: ControllerId,
    pub endpoint: PathBuf,
    pub heartbeat_ms: u64,
    pub frame_timeout_ms: u64,
}

/// Composition keeps deployment paths separate from business requests and supports recovery-only startup.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    pub node: NodeConfig,
    pub process: ProcessConfig,
    #[serde(default)]
    pub clone: Option<CloneConfig>,
    #[serde(default)]
    pub ipc: Option<IpcConfig>,
    pub recovery_interval_ms: u64,
    pub timezone: String,
}

// The queue holds at most 16 entries; inline envelopes avoid another allocation per control message.
#[allow(clippy::large_enum_variant)]
enum Request {
    Message(ControllerToNodeMessage),
    Replay,
}
struct Work {
    active: Arc<Mutex<bool>>,
    request: Request,
    reply: oneshot::Sender<Result<Vec<NodeToControllerMessage>, String>>,
}
#[derive(Clone)]
struct SessionInfo {
    identity: NodeRuntimeIdentity,
    controller: ControllerId,
    capabilities: Vec<NodeCapability>,
}

struct StopOnDrop(Shutdown);
impl Drop for StopOnDrop {
    /// Canceling the service future must also release its blocking owner and managed process scopes.
    fn drop(&mut self) {
        self.0.request();
    }
}

/// Runs independent IPC and execution lifecycles while retaining the Node lease until cleanup finishes.
pub async fn serve(config: ServiceConfig, shutdown: Shutdown) -> io::Result<()> {
    let _stop = StopOnDrop(shutdown.clone());
    if config.recovery_interval_ms == 0 {
        return Err(io::Error::other("recovery interval must be positive"));
    }
    if let Some(ipc) = &config.ipc
        && (ipc.heartbeat_ms == 0
            || ipc.frame_timeout_ms <= ipc.heartbeat_ms
            || !ipc.endpoint.is_absolute()
            || ipc.endpoint.parent() != Some(config.node.home_directory.as_path())
            || config.clone.is_none())
    {
        return Err(io::Error::other(
            "IPC needs clone configuration, positive bounded timing and an endpoint directly under Node home",
        ));
    }
    let ipc = config.ipc.clone();
    let (sender, receiver) = mpsc::sync_channel(/*bound*/ 16);
    let (ready, started) = oneshot::channel();
    let worker_shutdown = shutdown.clone();
    let mut worker =
        tokio::task::spawn_blocking(move || worker::run(config, receiver, ready, worker_shutdown));
    let startup = started.await.map_err(io::Error::other)?;
    let result = match startup {
        Ok(info) => match ipc {
            Some(ipc) => tokio::select! {
                result = session::listen(ipc, info, sender, shutdown.clone()) => result,
                result = &mut worker => return result.map_err(io::Error::other)?.map_err(io::Error::other),
            },
            None => {
                while !shutdown.requested() && !worker.is_finished() {
                    tokio::time::sleep(Duration::from_millis(/*millis*/ 25)).await;
                }
                Ok(())
            }
        },
        Err(error) => Err(io::Error::other(error)),
    };
    shutdown.request();
    worker
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    result
}
