use super::*;
use tokio::{
    net::{UnixListener, UnixStream},
    sync::mpsc as async_queue,
    time::{interval, timeout},
};

/// Rejects concurrent connections while a session owns admission, including its handshake window.
pub(super) async fn listen(
    config: IpcConfig,
    info: SessionInfo,
    sender: mpsc::SyncSender<Work>,
    shutdown: Shutdown,
) -> io::Result<()> {
    // SAFETY: this reads identity only; the worker already holds the stable Node database lease.
    let uid = unsafe { libc::geteuid() };
    let listener = ora_utils::local_ipc::bind_private_endpoint(
        &config.endpoint,
        uid,
        Duration::from_millis(config.frame_timeout_ms),
    )
    .await?;
    ora_logging::ora_info!("Node IPC listening");
    let mut tick = interval(Duration::from_millis(/*millis*/ 25));
    while !shutdown.requested() {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                run_exclusive(&listener, stream, &config, &info, &sender, &shutdown).await?;
            }
            _ = tick.tick() => {}
        }
    }
    Ok(())
}

/// The rejection loop stays live during long Git operations and never replaces the existing session.
async fn run_exclusive(
    listener: &UnixListener,
    stream: UnixStream,
    config: &IpcConfig,
    info: &SessionInfo,
    sender: &mpsc::SyncSender<Work>,
    shutdown: &Shutdown,
) -> io::Result<()> {
    let active = Arc::new(Mutex::new(true));
    let session = connected(stream, config, info, sender, active.clone());
    tokio::pin!(session);
    let mut tick = interval(Duration::from_millis(/*millis*/ 25));
    let result = loop {
        tokio::select! {
            result = &mut session => break result,
            accepted = listener.accept() => { drop(accepted?); }
            _ = tick.tick() => { if shutdown.requested() { break Ok(()); } }
        }
    };
    *active
        .lock()
        .map_err(|_| io::Error::other("session admission poisoned"))? = false;
    if result.is_err() {
        ora_logging::ora_warn!(
            "Node control session closed; durable execution responsibility retained"
        );
    }
    Ok(())
}

/// Queues bounded requests with a revocable admission identity; closing a connection does not cancel accepted work.
async fn request(
    sender: &mpsc::SyncSender<Work>,
    active: &Arc<Mutex<bool>>,
    request: Request,
) -> io::Result<Vec<NodeToControllerMessage>> {
    let (reply, response) = oneshot::channel();
    sender
        .try_send(Work {
            active: active.clone(),
            request,
            reply,
        })
        .map_err(|_| io::Error::other("Node admission queue unavailable"))?;
    response
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)
}

/// Keeps one reader future alive per frame and gives heartbeat/output independent execution from Git.
async fn connected(
    mut stream: UnixStream,
    config: &IpcConfig,
    info: &SessionInfo,
    sender: &mpsc::SyncSender<Work>,
    active: Arc<Mutex<bool>>,
) -> io::Result<()> {
    let deadline = Duration::from_millis(config.frame_timeout_ms);
    let greeting = timeout(deadline, read_controller_message(&mut stream))
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    let Some(ControllerToNodeMessage::Hello(hello)) = greeting else {
        return Err(io::Error::other("expected Hello"));
    };
    if hello.payload.controller_id != info.controller {
        return Err(io::Error::other("Controller does not own this Node"));
    }
    let response = NodeToControllerMessage::HelloAccepted(HelloAcceptedMessage {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        payload: HelloAccepted {
            selected_version: CURRENT_PROTOCOL_VERSION,
            node: info.identity.clone(),
            capabilities: info.capabilities.clone(),
        },
    });
    timeout(deadline, write_node_message(&mut stream, &response))
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    let (mut reader, mut writer) = stream.into_split();
    let (outgoing, mut messages) = async_queue::channel(/*buffer*/ 16);
    let read = async {
        loop {
            let message = timeout(deadline, read_controller_message(&mut reader))
                .await
                .map_err(io::Error::other)?
                .map_err(io::Error::other)?;
            let Some(message) = message else {
                return Ok::<(), io::Error>(());
            };
            // Git may occupy the worker. Bound admission waiting independently of heartbeats so
            // revocation invalidates queued work; already durable executions are not canceled.
            let replies = timeout(
                deadline,
                request(sender, &active, Request::Message(message)),
            )
            .await
            .map_err(io::Error::other)??;
            for reply in replies {
                outgoing.send(reply).await.map_err(io::Error::other)?;
            }
        }
    };
    let replay = async {
        let mut tick = interval(Duration::from_millis(config.heartbeat_ms));
        loop {
            tick.tick().await;
            for event in request(sender, &active, Request::Replay).await? {
                outgoing.send(event).await.map_err(io::Error::other)?;
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), io::Error>(())
    };
    let write = async {
        let mut tick = interval(Duration::from_millis(config.heartbeat_ms));
        loop {
            let message = tokio::select! {
                message = messages.recv() => { let Some(message) = message else { return Ok::<(), io::Error>(()); }; message }
                _ = tick.tick() => NodeToControllerMessage::Heartbeat(HeartbeatMessage { protocol_version: CURRENT_PROTOCOL_VERSION, payload: Heartbeat { node: info.identity.clone() } }),
            };
            timeout(deadline, write_node_message(&mut writer, &message))
                .await
                .map_err(io::Error::other)?
                .map_err(io::Error::other)?;
        }
    };
    tokio::select! { result = read => result, result = write => result, result = replay => result }
}
