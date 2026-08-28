use std::error::Error;
use std::future::Future;

use ora_logging::{LogLevel, LogLevelControl, LogLevelReadError, LogLevelReloadError};

/// Abstracts the live process filter used by the runtime settings transaction coordinator.
///
/// Implementations must report the actual effective level and apply replacements synchronously so
/// persistence is never committed before the process filter has accepted the requested level.
pub trait RuntimeLogLevelControl: Clone + Send + Sync + 'static {
    type ReadError: Error + Send + Sync + 'static;
    type ReloadError: Error + Send + Sync + 'static;

    /// Reads the level currently installed in the process-wide filter.
    fn current_level(&self) -> Result<LogLevel, Self::ReadError>;

    /// Replaces the live process filter for subsequently emitted events.
    fn set_level(&self, level: LogLevel) -> Result<(), Self::ReloadError>;
}

impl RuntimeLogLevelControl for LogLevelControl {
    type ReadError = LogLevelReadError;
    type ReloadError = LogLevelReloadError;

    fn current_level(&self) -> Result<LogLevel, Self::ReadError> {
        LogLevelControl::current_level(self)
    }

    fn set_level(&self, level: LogLevel) -> Result<(), Self::ReloadError> {
        LogLevelControl::set_level(self, level)
    }
}

/// Abstracts one runtime's atomic persistence of the preferred process log level.
///
/// Implementations own their persistence adapter. `save_preferred_level` must either replace the
/// configured value atomically or leave the previous value intact on failure. Both operations are
/// asynchronous so database-backed implementations never block an async runtime worker.
pub trait PreferredLogLevelStore: Clone + Send + Sync + 'static {
    type Error: Error + Send + Sync + 'static;

    /// Loads the runtime's preferred level during bootstrap.
    fn load_preferred_level(&self) -> impl Future<Output = Result<LogLevel, Self::Error>> + Send;

    /// Atomically persists the preferred level selected by a successful runtime update.
    fn save_preferred_level(
        &self,
        level: LogLevel,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
