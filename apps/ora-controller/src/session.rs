use super::*;
use serde::{Deserialize, Serialize};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    net::UnixStream,
    time::{interval, timeout},
};

/// Deployment maps one persistent Node identity to a local endpoint, never a checkout path.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeEndpoint {
    pub node_id: NodeId,
    pub endpoint: PathBuf,
}

/// Finite I/O deadlines and periodic queries keep sessions live independently of clone duration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionConfig {
    pub io_timeout_ms: u64,
    pub query_interval_ms: u64,
}

/// Coordinates one connection; the caller reconnects using the same durable Controller, never new IDs.
pub async fn run_session<W: WriteGuard>(
    controller: &Arc<Mutex<Controller<W>>>,
    target: &NodeEndpoint,
    config: &SessionConfig,
) -> io::Result<()> {
    if config.io_timeout_ms == 0 || config.query_interval_ms == 0 || !target.endpoint.is_absolute()
    {
        return Err(io::Error::other("invalid local session configuration"));
    }
    let id = {
        let controller = controller
            .lock()
            .map_err(|_| io::Error::other("Controller lock poisoned"))?;
        let node_home = target
            .endpoint
            .parent()
            .ok_or_else(|| io::Error::other("endpoint has no parent"))?;
        let home =
            ora_utils::path::canonicalize_longest_existing_prefix(controller.home_directory());
        let node_home = ora_utils::path::canonicalize_longest_existing_prefix(node_home);
        if home.starts_with(&node_home) || node_home.starts_with(home) {
            return Err(io::Error::other("Controller and Node state roots overlap"));
        }
        controller.id().clone()
    };
    let deadline = Duration::from_millis(config.io_timeout_ms);
    let mut stream = timeout(deadline, UnixStream::connect(&target.endpoint))
        .await
        .map_err(io::Error::other)??;
    let hello = ControllerToNodeMessage::Hello(HelloMessage {
        protocol_version: CURRENT_PROTOCOL_VERSION,
        payload: Hello {
            controller_id: id,
            supported_versions: vec![CURRENT_PROTOCOL_VERSION],
        },
    });
    timeout(deadline, write_controller_message(&mut stream, &hello))
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    let greeting = timeout(deadline, read_node_message(&mut stream))
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)?;
    let Some(NodeToControllerMessage::HelloAccepted(hello)) = greeting else {
        return Err(io::Error::other("Node did not accept Hello"));
    };
    if hello.payload.node.node_id != target.node_id
        || !hello
            .payload
            .capabilities
            .contains(&NodeCapability::RepositoryClone)
    {
        return Err(io::Error::other("Node identity or capability mismatch"));
    }
    let identity = hello.payload.node;
    let (mut reader, mut writer) = stream.into_split();
    let mut tick = interval(Duration::from_millis(config.query_interval_ms));
    let mut cursor = 0usize;
    // Unknown can also be a retained uncertain attempt. Repeated Unknown replies must not form
    // an immediate query/command feedback loop; one exact retransmission per connection is enough.
    let mut retransmitted = std::collections::HashSet::new();
    loop {
        // Pin the whole frame across query ticks: canceling a partial read would corrupt framing.
        let read = timeout(deadline, read_node_message(&mut reader));
        tokio::pin!(read);
        let message = loop {
            tokio::select! {
                message = &mut read => break message.map_err(io::Error::other)?.map_err(io::Error::other)?.ok_or_else(|| io::Error::other("Node disconnected"))?,
                _ = tick.tick() => {
                    let command = {
                        let owner = controller.lock().map_err(|_| io::Error::other("Controller lock poisoned"))?;
                        let commands = owner.commands(&target.node_id).map_err(io::Error::other)?;
                        if commands.is_empty() { None } else { let command = commands[cursor % commands.len()].clone(); cursor = cursor.wrapping_add(1); Some(command) }
                    };
                    if let Some(command) = command {
                        let query = ControllerToNodeMessage::GetExecutionStatus(GetExecutionStatusMessage { protocol_version: CURRENT_PROTOCOL_VERSION, operation_id: command.operation_id, execution_id: command.execution_id, payload: GetExecutionStatus { node_id: target.node_id.clone() } });
                        timeout(deadline, write_controller_message(&mut writer, &query)).await.map_err(io::Error::other)?.map_err(io::Error::other)?;
                    }
                }
            }
        };
        let reply = {
            let mut owner = controller
                .lock()
                .map_err(|_| io::Error::other("Controller lock poisoned"))?;
            let ack = owner
                .take_over(&identity, &message)
                .map_err(io::Error::other)?;
            if let Some(ack) = ack {
                Some(ControllerToNodeMessage::EventAck(ack))
            } else if let NodeToControllerMessage::ExecutionStatus(status) = &message
                && status.payload.state == ExecutionState::Unknown
                && owner
                    .result(&status.execution_id)
                    .map_err(io::Error::other)?
                    .is_none()
                && retransmitted.insert(status.execution_id.clone())
            {
                Some(ControllerToNodeMessage::CloneRepository(
                    owner
                        .original(&identity, &status.operation_id, &status.execution_id)
                        .map_err(io::Error::other)?,
                ))
            } else {
                None
            }
        };
        if let Some(reply) = reply {
            timeout(deadline, write_controller_message(&mut writer, &reply))
                .await
                .map_err(io::Error::other)?
                .map_err(io::Error::other)?;
        }
    }
}
