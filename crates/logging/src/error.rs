use std::path::PathBuf;

use thiserror::Error;

/// Reports that the installed runtime filter could not be read.
#[derive(Debug, Error)]
#[error("failed to read the runtime log level")]
pub struct LogLevelReadError {
    #[source]
    source: tracing_subscriber::reload::Error,
}

impl LogLevelReadError {
    /// Preserves the subscriber failure while keeping its concrete handle type private.
    pub(crate) fn new(source: tracing_subscriber::reload::Error) -> Self {
        Self { source }
    }
}

/// Reports that the installed process-wide level filter could not be replaced.
#[derive(Debug, Error)]
#[error("failed to reload the runtime log level")]
pub struct LogLevelReloadError {
    #[source]
    source: tracing_subscriber::reload::Error,
}

impl LogLevelReloadError {
    /// Preserves the subscriber failure while keeping its concrete handle type private.
    pub(crate) fn new(source: tracing_subscriber::reload::Error) -> Self {
        Self { source }
    }
}

/// Reports a runtime log-level name outside the supported closed set.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("unsupported log level `{value}`")]
pub struct ParseLogLevelError {
    value: String,
}

impl ParseLogLevelError {
    /// Preserves the caller-provided value so runtime adapters can project their own errors.
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Returns the original unsupported value without normalizing it for display.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Identifies the filesystem step that failed while preparing file-backed logging.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileSystemAction {
    CreateDirectory,
    ReadDirectory,
    RemoveFile,
}

/// Describes the typed failures that can prevent shared logging from starting.
#[derive(Debug, Error)]
pub enum LoggingInitError {
    #[error("log file path must include a file name: {path}")]
    InvalidFilePath { path: PathBuf },
    #[error("failed to {action:?} at {path}")]
    FileSystem {
        action: FileSystemAction,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to install the global tracing subscriber")]
    SetGlobalSubscriber(#[source] tracing::dispatcher::SetGlobalDefaultError),
    #[error("the process-wide logging clock was already initialized")]
    ClockAlreadyInitialized,
}
