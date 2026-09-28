use super::*;
use single_node::ManagedNode;
use std::{future::Future, io, time::Duration};
use tokio::{sync::watch, task::JoinHandle};

/// One process hosting the sole Controller owner, optionally its Node, and the JSON surface when
/// the persistence adapter accepts requests locally.
pub struct Service<S: CoordinationStore> {
    surface: Option<Surface>,
    runtime: ControllerRuntime<S>,
    managed: Option<ManagedNode>,
}

/// The bound transitional JSON surface: composed only by adapters that implement [`CloneIntake`],
/// so a deployment without local intake has no listener at all rather than one answering errors.
struct Surface {
    listener: Listener,
    router: axum::Router,
}

impl Service<SqliteStore> {
    /// Validates composition, opens the exclusive local owner, hosts the Node when requested, then binds the API.
    pub async fn start(
        config: DeploymentConfig,
        transport: Transport,
        hosting: NodeHosting,
    ) -> Result<Self, Error> {
        let single = config.validate(hosting)?.cloned();
        transport.validate(&config.controller.home_directory)?;
        let api = config.api.clone().ok_or_else(|| {
            Error::Configuration(
                "sqlite persistence serves the JSON surface; add an api section".into(),
            )
        })?;
        let (runtime, managed) = compose(
            &config,
            single.as_ref(),
            ControllerRuntime::<SqliteStore>::open,
        )
        .await?;
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
        let router = api::router(runtime.handle(), api.node_id);
        Ok(Self {
            surface: Some(Surface { listener, router }),
            runtime,
            managed,
        })
    }
}

impl Service<CloudStore> {
    /// Composes a cloud deployment: the Controller bound to Cloud and optionally its Node. There is
    /// no JSON surface, because acceptance and queries belong to Cloud's public API.
    pub async fn start(config: DeploymentConfig, hosting: NodeHosting) -> Result<Self, Error> {
        let single = config.validate(hosting)?.cloned();
        let (runtime, managed) = compose(
            &config,
            single.as_ref(),
            ControllerRuntime::<CloudStore>::open,
        )
        .await?;
        Ok(Self {
            surface: None,
            runtime,
            managed,
        })
    }
}

/// The order every composition shares: all checks, then the owner, then the hosted Node, so a
/// refusal leaves no state behind and a Node never runs without its Controller.
async fn compose<S: CoordinationStore>(
    config: &DeploymentConfig,
    single: Option<&SingleNodeConfig>,
    open: impl FnOnce(RuntimeConfig) -> Result<ControllerRuntime<S>, Error>,
) -> Result<(ControllerRuntime<S>, Option<ManagedNode>), Error> {
    let launch = match single {
        Some(single) => Some(ManagedNode::prepare(single, &config.controller).await?),
        None => None,
    };
    let runtime = open(config.controller.clone())?;
    let managed = match launch {
        Some(launch) => Some(launch.start().await?),
        None => None,
    };
    Ok((runtime, managed))
}

impl<S: CoordinationStore> Service<S> {
    /// Reports the actual bound endpoint, including an ephemeral test port; a composition without
    /// local intake has none.
    pub fn endpoint(&self) -> io::Result<Transport> {
        self.surface
            .as_ref()
            .ok_or_else(|| io::Error::other("this deployment serves no JSON surface"))?
            .listener
            .endpoint()
    }

    /// Runs until shutdown or an unexpected component stop, then stops in order: API admission,
    /// Node sessions, the hosted Node, and finally the database lease when the runtime drops.
    pub async fn run(self, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        let (stop_api, api_stopping) = watch::channel(false);
        let (stop_sessions, sessions_stopping) = watch::channel(false);
        let mut api = match self.surface {
            Some(Surface { listener, router }) => serve(listener, router, api_stopping),
            // Nothing to admit: the task resolves only when told to stop, like an idle listener.
            None => tokio::spawn(async move {
                let mut stopping = api_stopping;
                let _ = stopping.changed().await;
                Ok(())
            }),
        };
        let runtime = self.runtime;
        let mut sessions: JoinHandle<(ControllerRuntime<S>, io::Result<()>)> =
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
