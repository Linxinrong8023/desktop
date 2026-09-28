use clap::{Parser, ValueEnum};
use ora_controller::{DEFAULT_PORT, NodeHosting, Transport};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

/// Composition flags for one process; deployment state stays in the configuration file.
#[derive(Parser, Debug)]
#[command(
    name = "ora-controller",
    about = "Durable local clone coordination with an API listener"
)]
pub struct Cli {
    /// Absolute path to the deployment configuration file.
    #[arg(long)]
    pub config: PathBuf,
    /// Start the configured Node in this process group and stop it on normal shutdown.
    #[arg(long)]
    pub single_node: bool,
    /// How the API listener accepts connections; SQLite persistence only, TCP when omitted.
    #[arg(long, value_enum)]
    pub transport: Option<TransportKind>,
    /// TCP bind address; defaults to loopback because the API has no authentication yet.
    #[arg(long)]
    pub host: Option<IpAddr>,
    /// TCP port.
    #[arg(long)]
    pub port: Option<u16>,
    /// Unix socket path, directly inside the Controller home directory.
    #[arg(long)]
    pub socket: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum TransportKind {
    Tcp,
    Unix,
}

impl Cli {
    /// Whether any listener flag was given; a cloud deployment has no listener to apply them to.
    pub fn listener_requested(&self) -> bool {
        self.transport.is_some()
            || self.host.is_some()
            || self.port.is_some()
            || self.socket.is_some()
    }

    /// Rejects flag combinations that would silently ignore an operator's intent.
    pub fn transport(&self) -> Result<Transport, String> {
        let kind = self.transport.unwrap_or(TransportKind::Tcp);
        match (kind, self.host, self.port, &self.socket) {
            (TransportKind::Tcp, host, port, None) => Ok(Transport::Tcp(SocketAddr::new(
                host.unwrap_or(Ipv4Addr::LOCALHOST.into()),
                port.unwrap_or(DEFAULT_PORT),
            ))),
            (TransportKind::Tcp, _, _, Some(_)) => {
                Err("--socket applies only to --transport unix".into())
            }
            (TransportKind::Unix, None, None, Some(socket)) => Ok(Transport::Unix(socket.clone())),
            (TransportKind::Unix, _, _, _) => {
                Err("--transport unix requires --socket and accepts no --host/--port".into())
            }
        }
    }

    /// Maps the flag to the explicit hosting choice the service validates against configuration.
    pub fn hosting(&self) -> NodeHosting {
        if self.single_node {
            NodeHosting::Managed
        } else {
            NodeHosting::External
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// Flags map to one explicit transport; defaults stay on loopback and mixed flags are refused.
    #[test]
    fn flags_map_to_explicit_transports() {
        let parse = |args: &[&str]| {
            Cli::parse_from(["ora-controller", "--config", "/c.json"].iter().chain(args))
        };
        assert_eq!(
            parse(&[]).transport(),
            Ok(Transport::Tcp("127.0.0.1:4820".parse().unwrap()))
        );
        assert_eq!(
            parse(&["--host", "0.0.0.0", "--port", "1"]).transport(),
            Ok(Transport::Tcp("0.0.0.0:1".parse().unwrap()))
        );
        assert_eq!(
            parse(&["--transport", "unix", "--socket", "/home/api.sock"]).transport(),
            Ok(Transport::Unix("/home/api.sock".into()))
        );
        assert!(parse(&["--socket", "/home/api.sock"]).transport().is_err());
        assert!(parse(&["--transport", "unix"]).transport().is_err());
        assert!(
            parse(&["--transport", "unix", "--socket", "/s", "--port", "1"])
                .transport()
                .is_err()
        );
        assert_eq!(parse(&["--single-node"]).hosting(), NodeHosting::Managed);
        assert_eq!(parse(&[]).hosting(), NodeHosting::External);
        assert!(!parse(&["--single-node"]).listener_requested());
        assert!(parse(&["--port", "0"]).listener_requested());
        assert!(parse(&["--transport", "tcp"]).listener_requested());
    }
}
