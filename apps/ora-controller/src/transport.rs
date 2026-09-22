use std::{
    fmt, io,
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::net::{TcpListener, UnixListener};

/// Default TCP port; avoids the OTLP defaults (4317/4318) and the minicloud Vite port.
pub const DEFAULT_PORT: u16 = 4820;

/// Where the executable accepts API connections; chosen per process and never stored with deployment state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transport {
    Tcp(SocketAddr),
    Unix(PathBuf),
}

/// A bound listener whose actual endpoint may differ from the request, such as an ephemeral TCP port.
pub enum Listener {
    Tcp(TcpListener),
    Unix(UnixListener),
}

impl Transport {
    /// The loopback default that keeps an unauthenticated deployment local unless explicitly widened.
    pub fn loopback(port: u16) -> Self {
        Self::Tcp(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))
    }

    /// Checks everything that can be known before any state opens: a Unix socket must be an
    /// absolute path directly inside the Controller home, so a refusal never costs a lease.
    pub fn validate(&self, controller_home: &Path) -> io::Result<()> {
        match self {
            Self::Tcp(_) => Ok(()),
            Self::Unix(path) if path.is_absolute() && path.parent() == Some(controller_home) => {
                Ok(())
            }
            Self::Unix(_) => Err(io::Error::other(
                "Unix API socket must be an absolute path directly inside the Controller home directory",
            )),
        }
    }

    /// Binds a validated transport; the Unix branch re-checks so it cannot be misused on its own.
    pub async fn bind(&self, controller_home: &Path) -> io::Result<Listener> {
        self.validate(controller_home)?;
        match self {
            Self::Tcp(address) => {
                if !address.ip().is_loopback() {
                    ora_logging::ora_warn!(
                        address = %address,
                        "API listens on a non-loopback address without authentication"
                    );
                }
                TcpListener::bind(address).await.map(Listener::Tcp)
            }
            Self::Unix(path) => {
                // SAFETY: reads identity only; the caller already holds the Controller database lease.
                let uid = unsafe { libc::geteuid() };
                ora_utils::local_ipc::bind_private_endpoint(
                    path,
                    uid,
                    Duration::from_millis(/*millis*/ 1000),
                )
                .await
                .map(Listener::Unix)
            }
        }
    }
}

impl Listener {
    /// Reports the bound endpoint for logs and tests, including an ephemeral port.
    pub fn endpoint(&self) -> io::Result<Transport> {
        match self {
            Self::Tcp(listener) => listener.local_addr().map(Transport::Tcp),
            Self::Unix(listener) => listener
                .local_addr()?
                .as_pathname()
                .map(Path::to_path_buf)
                .map(Transport::Unix)
                .ok_or_else(|| io::Error::other("Unix API socket has no path")),
        }
    }
}

impl fmt::Display for Transport {
    /// URL-like form so logs and launcher probes can parse the bound endpoint back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tcp(address) => write!(f, "tcp://{address}"),
            Self::Unix(path) => write!(f, "unix://{}", path.display()),
        }
    }
}
