//! Local durable clone coordination; no Desktop/Backend writer or Cloud authority is installed.
#[cfg(target_os = "linux")]
mod api;
#[cfg(target_os = "linux")]
mod deployment;
mod operations;
#[cfg(target_os = "linux")]
mod runtime;
#[cfg(target_os = "linux")]
mod service;
#[cfg(target_os = "linux")]
mod session;
#[cfg(target_os = "linux")]
mod single_node;
mod storage;
mod takeover;
#[cfg(target_os = "linux")]
mod transport;
#[cfg(target_os = "linux")]
pub use deployment::{ApiConfig, DeploymentConfig, NodeHosting, SingleNodeConfig};
pub use operations::CloneOperation;
use ora_node_protocol::*;
use ora_utils::fs::{ExclusiveFileLock, ExclusiveLockError};
#[cfg(target_os = "linux")]
pub use runtime::{ControllerHandle, ControllerRuntime, RuntimeConfig};
use rusqlite::Connection;
#[cfg(target_os = "linux")]
pub use service::Service;
#[cfg(target_os = "linux")]
pub use session::{NodeEndpoint, SessionConfig, run_session};
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
pub use transport::{DEFAULT_PORT, Listener, Transport};

/// Local persistence failures never authorize dispatch or acknowledgement.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("controller I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("controller storage: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("controller encoding: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("invalid message: {0}")]
    Validation(#[from] MessageValidationError),
    #[error("another runtime owns the Controller database")]
    AlreadyRunning,
    #[error("unrecognized Controller schema or identity")]
    InvalidStorage,
    #[error("input, result or dispatch ownership conflict")]
    Conflict,
    #[error("injected persistence failure")]
    Injected,
    #[error("invalid deployment composition: {0}")]
    Configuration(String),
}

/// Test seams refuse writes before transactions commit, using the same real SQLite and reconciliation.
pub trait WriteGuard {
    /// Prevents a durable boundary; callers must not dispatch or acknowledge on failure.
    fn before_write(&self, point: WritePoint) -> Result<(), Error>;
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WritePoint {
    Accept,
    Takeover,
    Receipt,
    /// Final takeover boundary inside the open transaction, before SQLite commit or any Ack.
    Commit,
}
pub struct DurableWrites;
impl WriteGuard for DurableWrites {
    /// Production delegates all durability failures to SQLite.
    fn before_write(&self, _point: WritePoint) -> Result<(), Error> {
        Ok(())
    }
}

/// The database lease and transaction owner retain original dispatches, results and event receipts.
pub struct Controller<W = DurableWrites> {
    connection: Connection,
    id: ControllerId,
    home: PathBuf,
    writes: W,
    // Held beside the database rather than on it so SQLite's own locks never collide with ours.
    _lease: ExclusiveFileLock,
}

impl Controller {
    /// Opens explicitly injected local state, preserving unknown files instead of reinitializing them.
    pub fn open(home: &Path, id: ControllerId) -> Result<Self, Error> {
        Self::open_with_guard(home, id, DurableWrites)
    }
}

impl<W: WriteGuard> Controller<W> {
    /// Returns the persistent coordinator identity, never a process or connection identity.
    pub fn id(&self) -> &ControllerId {
        &self.id
    }
    /// Exposes the injected root for deployment overlap checks, not Node-scoped checkout resolution.
    pub fn home_directory(&self) -> &Path {
        &self.home
    }
}
