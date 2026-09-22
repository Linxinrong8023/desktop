use super::*;
use single_node::ManagedNode;
use std::{future::Future, io, time::Duration};
use tokio::{sync::watch, task::JoinHandle};

/// One process hosting the API listener, the sole Controller owner and optionally its Node.
pub struct Service {
    listener: Listener,
    runtime: ControllerRuntime,
    node_id: NodeId,
    managed: Option<ManagedNode>,
}

impl Service {
    /// Validates composition, opens the exclusive owner, hosts the Node when requested, then binds the API.
    pub async fn start(
        config: DeploymentConfig,
        transport: Transport,
        hosting: NodeHosting,
    ) -> Result<Self, Error> {
        let single = config.validate(hosting)?.cloned();
        transport.validate(&config.controller.home_directory)?;
        // Every composition check precedes the database lease so a refusal leaves no state behind.
        let launch = match &single {
            Some(single) => Some(ManagedNode::prepare(single, &config.controller).await?),
            None => None,
        };
        let runtime = ControllerRuntime::open(config.controller.clone())?;
        let managed = match launch {
            Some(launch) => Some(launch.start().await?),
            None => None,
        };
        let listener = match transport.bind(&config.controller.home_directory).await {
            Ok(listener) => listener,
            Err(error) => {
                // A hosted Node must not outlive a composition that never accepted work.
                if let Some(managed) = managed {
                    let _ = managed.stop().await;
                }
                return Err(error.into());
            }
        };
        Ok(Self {
            listener,
            runtime,
            node_id: config.api.node_id,
            managed,
        })
    }

    /// Reports the actual bound endpoint, including an ephemeral test port.
    pub fn endpoint(&self) -> io::Result<Transport> {
        self.listener.endpoint()
    }

    /// Runs until shutdown or an unexpected component stop, then stops in order: API admission,
    /// Node sessions, the hosted Node, and finally the database lease when the runtime drops.
    pub async fn run(self, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        let (stop_api, api_stopping) = watch::channel(false);
        let (stop_sessions, sessions_stopping) = watch::channel(false);
        let router = api::router(self.runtime.handle(), self.node_id);
        let mut api = serve(self.listener, router, api_stopping);
        let runtime = self.runtime;
        let mut sessions: JoinHandle<(ControllerRuntime, io::Result<()>)> =
            tokio::spawn(async move {
                let mut stopping = sessions_stopping;
                let result = runtime
                    .run(async move {
                        let _ = stopping.changed().await;
                    })
                    .await;
                (runtime, result)
            });
        let mut managed = self.managed;
        let (mut api_done, mut sessions_done, mut node_gone) = (false, false, false);
        let mut runtime = None;
        tokio::pin!(shutdown);
        let outcome = tokio::select! {
            _ = &mut shutdown => Ok(()),
            result = &mut api => {
                api_done = true;
                Err(io::Error::other(format!("API listener stopped: {result:?}")))
            }
            result = &mut sessions => {
                sessions_done = true;
                // A panicked session task loses the runtime handle; ordered stop still proceeds.
                let detail = match result {
                    Ok((stopped, result)) => {
                        runtime = Some(stopped);
                        format!("{result:?}")
                    }
                    Err(join) => join.to_string(),
                };
                Err(io::Error::other(format!("Controller sessions stopped: {detail}")))
            }
            status = exited(&mut managed), if managed.is_some() => {
                node_gone = true;
                Err(io::Error::other(format!("managed Node exited unexpectedly: {status:?}")))
            }
        };
        // Each phase logs its completion so operators and tests can verify the stop order.
        let _ = stop_api.send(true);
        let api = if api_done {
            Ok(())
        } else {
            match tokio::time::timeout(Duration::from_secs(/*secs*/ 5), &mut api).await {
                Ok(joined) => joined.map_err(io::Error::other).and_then(|served| served),
                Err(elapsed) => {
                    // Detaching the listener would keep handle clones, and with them the database
                    // lease, alive past this shutdown; aborting drops them with the router.
                    api.abort();
                    Err(io::Error::other(format!(
                        "API shutdown timed out: {elapsed}"
                    )))
                }
            }
        };
        ora_logging::ora_info!("API admission stopped");
        let _ = stop_sessions.send(true);
        let sessions = if sessions_done {
            Ok(())
        } else {
            let (stopped, result) = sessions.await.map_err(io::Error::other)?;
            runtime = Some(stopped);
            result
        };
        ora_logging::ora_info!("Node sessions stopped");
        let node = match managed {
            Some(node) if !node_gone => {
                let stopped = node.stop().await;
                ora_logging::ora_info!("managed Node stopped");
                stopped
            }
            Some(_) | None => Ok(()),
        };
        // Release the lease only after the hosted Node has been asked to stop.
        drop(runtime);
        outcome.and(api).and(sessions).and(node)
    }
}

/// Serves the transitional JSON surface on whichever listener was bound; both stop on the same signal.
fn serve(
    listener: Listener,
    router: axum::Router,
    mut stopping: watch::Receiver<bool>,
) -> JoinHandle<io::Result<()>> {
    let stop = async move {
        let _ = stopping.changed().await;
    };
    match listener {
        Listener::Tcp(listener) => tokio::spawn(
            axum::serve(listener, router)
                .with_graceful_shutdown(stop)
                .into_future(),
        ),
        Listener::Unix(listener) => tokio::spawn(
            axum::serve(listener, router)
                .with_graceful_shutdown(stop)
                .into_future(),
        ),
    }
}

/// Observes a hosted Node's own exit; callers guard the branch so an external Node never resolves it.
async fn exited(managed: &mut Option<ManagedNode>) -> io::Result<std::process::ExitStatus> {
    match managed {
        Some(node) => node.exited().await,
        None => std::future::pending().await,
    }
}
