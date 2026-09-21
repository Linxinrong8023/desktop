#![cfg(target_os = "linux")]
mod http;
use ora_controller::{ControllerRuntime, RuntimeConfig};
use ora_node_protocol::NodeId;
use serde::{Deserialize, Serialize};
use std::{
    future::{Future, IntoFuture},
    io,
    net::SocketAddr,
    time::Duration,
};
use tokio::{net::TcpListener, sync::watch};

/// Development-only deployment; no business commands or credentials are accepted in this file.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub listen: SocketAddr,
    pub node_id: NodeId,
    pub controller: RuntimeConfig,
}

/// A loopback listener and exactly one embedded Controller owner.
pub struct Server {
    listener: TcpListener,
    runtime: ControllerRuntime,
    node_id: NodeId,
}

impl Server {
    /// Rejects nonlocal deployment and unknown targets before opening Controller state.
    pub async fn bind(config: ServerConfig) -> io::Result<Self> {
        if !config.listen.ip().is_loopback()
            || !config
                .controller
                .nodes
                .iter()
                .any(|node| node.node_id == config.node_id)
        {
            return Err(io::Error::other(
                "minicloud requires loopback and a configured Node",
            ));
        }
        let listener = TcpListener::bind(config.listen).await?;
        let runtime = ControllerRuntime::open(config.controller).map_err(io::Error::other)?;
        Ok(Self {
            listener,
            runtime,
            node_id: config.node_id,
        })
    }

    /// Reports the actual bound address, including an ephemeral test port.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Stops HTTP admission and reconnect loops together; accepted Node executions remain independent.
    pub async fn run(self, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        let (stop, mut http_stop) = watch::channel(false);
        let mut controller_stop = stop.subscribe();
        let router = http::router(self.runtime.handle(), self.node_id);
        let http = axum::serve(self.listener, router)
            .with_graceful_shutdown(async move {
                let _ = http_stop.changed().await;
            })
            .into_future();
        let coordinator = self.runtime.run(async move {
            let _ = controller_stop.changed().await;
        });
        tokio::pin!(http, coordinator);
        tokio::select! {
            _ = shutdown => {
                let _ = stop.send(true);
                let (http, coordinator) = tokio::join!(tokio::time::timeout(Duration::from_secs(/*secs*/ 5), &mut http), &mut coordinator);
                http.map_err(io::Error::other)??;
                coordinator
            }
            result = &mut http => { let _ = stop.send(true); coordinator.await?; result }
            result = &mut coordinator => {
                let _ = stop.send(true);
                tokio::time::timeout(Duration::from_secs(/*secs*/ 5), &mut http).await.map_err(io::Error::other)??;
                result
            }
        }
    }
}
