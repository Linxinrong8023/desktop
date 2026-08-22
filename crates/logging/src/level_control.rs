use tracing_subscriber::Registry;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::reload;

use crate::{LogLevel, LogLevelReadError, LogLevelReloadError};

type LevelReloadHandle = reload::Handle<LevelFilter, Registry>;

/// Controls the process-wide logging filter without exposing subscriber layer types to runtimes.
#[derive(Clone, Debug)]
pub struct LogLevelControl {
    handle: LevelReloadHandle,
    _retained_dispatch: Option<tracing::Dispatch>,
}

impl LogLevelControl {
    /// Wraps the reload handle created alongside the process subscriber.
    pub(crate) fn new(handle: LevelReloadHandle) -> Self {
        Self {
            handle,
            _retained_dispatch: None,
        }
    }

    /// Creates an isolated control whose subscriber stays alive without global installation.
    pub(crate) fn scoped_for_tests(level: LogLevel) -> Self {
        let (layer, handle) = reload::Layer::new(level_filter(level));
        let dispatch = tracing::Dispatch::new(tracing_subscriber::registry().with(layer));

        Self {
            handle,
            _retained_dispatch: Some(dispatch),
        }
    }

    /// Replaces the effective filter for events emitted after this call succeeds.
    pub fn set_level(&self, level: LogLevel) -> Result<(), LogLevelReloadError> {
        self.handle
            .reload(level_filter(level))
            .map_err(LogLevelReloadError::new)
    }

    /// Reads the level currently installed in the process-wide reload layer.
    pub fn current_level(&self) -> Result<LogLevel, LogLevelReadError> {
        self.handle
            .with_current(|filter| log_level(*filter))
            .map_err(LogLevelReadError::new)
    }
}

/// Maps the public level enum into the tracing filter used by every active sink.
pub(crate) const fn level_filter(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Trace => LevelFilter::TRACE,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Error => LevelFilter::ERROR,
    }
}

/// Maps a tracing filter back into the closed public runtime vocabulary.
fn log_level(filter: LevelFilter) -> LogLevel {
    match filter {
        LevelFilter::OFF => LogLevel::Error,
        LevelFilter::ERROR => LogLevel::Error,
        LevelFilter::WARN => LogLevel::Warn,
        LevelFilter::INFO => LogLevel::Info,
        LevelFilter::DEBUG => LogLevel::Debug,
        LevelFilter::TRACE => LogLevel::Trace,
    }
}
