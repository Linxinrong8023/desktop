use std::error::Error;
use std::sync::Arc;

use ora_logging::LogLevel;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::Instrument;

use crate::{PreferredLogLevelStore, RuntimeLogLevelControl};

/// Describes the authoritative preferred and effective state of one process-wide logging runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeLogLevelState {
    pub configured_level: LogLevel,
    pub effective_level: LogLevel,
    pub startup_override: Option<LogLevel>,
}

/// Names the transactional update result shared by runtime adapters.
pub type RuntimeLogLevelUpdateResult<ReadError, ReloadError, StoreError> =
    Result<RuntimeLogLevelState, RuntimeLogLevelUpdateError<ReadError, ReloadError, StoreError>>;

/// Coordinates clone-shared runtime log-level reads and transactional updates.
pub struct RuntimeLogLevelManager<C, S> {
    coordination: Arc<Mutex<CoordinatedRuntimeLogLevel<C, S>>>,
}

impl<C, S> Clone for RuntimeLogLevelManager<C, S> {
    fn clone(&self) -> Self {
        Self {
            coordination: self.coordination.clone(),
        }
    }
}

impl<C, S> RuntimeLogLevelManager<C, S>
where
    C: RuntimeLogLevelControl,
    S: PreferredLogLevelStore,
{
    /// Creates a manager from the already initialized filter and resolved startup preference.
    pub fn new(
        control: C,
        store: S,
        configured_level: LogLevel,
        startup_override: Option<LogLevel>,
    ) -> Self {
        Self {
            coordination: Arc::new(Mutex::new(CoordinatedRuntimeLogLevel {
                control,
                store,
                configured_level,
                startup_override,
            })),
        }
    }

    /// Reads one coherent snapshot while excluding the reload-before-persist transition window.
    pub async fn state(&self) -> Result<RuntimeLogLevelState, C::ReadError> {
        let coordination = self.coordination.lock().await;
        let effective_level = coordination.control.current_level()?;

        Ok(RuntimeLogLevelState {
            configured_level: coordination.configured_level,
            effective_level,
            startup_override: coordination.startup_override,
        })
    }

    /// Completes a reload-and-persist transaction even if its calling request stops waiting.
    pub async fn set_level(
        &self,
        level: LogLevel,
    ) -> RuntimeLogLevelUpdateResult<C::ReadError, C::ReloadError, S::Error> {
        let coordination = self.coordination.clone();
        tokio::spawn(
            async move {
                let mut coordination = coordination.lock().await;
                let previous_effective = coordination
                    .control
                    .current_level()
                    .map_err(RuntimeLogLevelUpdateError::Read)?;
                coordination
                    .control
                    .set_level(level)
                    .map_err(RuntimeLogLevelUpdateError::Reload)?;

                if let Err(source) = coordination.store.save_preferred_level(level).await {
                    // Storage remains the primary operation because the requested preference was
                    // not committed. The internal task must perform compensation because the
                    // transport request may no longer be waiting for this result.
                    let rollback_error = coordination.control.set_level(previous_effective).err();
                    return Err(RuntimeLogLevelUpdateError::Persistence {
                        source,
                        rollback_error,
                    });
                }

                coordination.configured_level = level;

                Ok(RuntimeLogLevelState {
                    configured_level: level,
                    effective_level: level,
                    startup_override: coordination.startup_override,
                })
            }
            // A detached transaction can outlive its caller, so it must retain the request span
            // rather than relying on task-local span inheritance.
            .instrument(tracing::Span::current()),
        )
        .await
        .map_err(RuntimeLogLevelUpdateError::Worker)?
    }
}

/// Owns the state that must never be observed midway through a runtime update.
struct CoordinatedRuntimeLogLevel<C, S> {
    control: C,
    store: S,
    configured_level: LogLevel,
    startup_override: Option<LogLevel>,
}

/// Preserves the primary stage that failed while exposing compensation failure independently.
#[derive(Debug, Error)]
pub enum RuntimeLogLevelUpdateError<ReadError, ReloadError, StoreError>
where
    ReadError: Error + 'static,
    ReloadError: Error + 'static,
    StoreError: Error + 'static,
{
    #[error("failed to read the current runtime log level")]
    Read(#[source] ReadError),
    #[error("failed to reload the runtime log level")]
    Reload(#[source] ReloadError),
    #[error("failed to persist the preferred runtime log level")]
    Persistence {
        #[source]
        source: StoreError,
        rollback_error: Option<ReloadError>,
    },
    #[error("runtime log-level transaction worker did not complete")]
    Worker(#[source] tokio::task::JoinError),
}

impl<ReadError, ReloadError, StoreError>
    RuntimeLogLevelUpdateError<ReadError, ReloadError, StoreError>
where
    ReadError: Error + 'static,
    ReloadError: Error + 'static,
    StoreError: Error + 'static,
{
    /// Returns a failed compensation separately from the client-visible primary failure.
    pub fn rollback_error(&self) -> Option<&ReloadError> {
        match self {
            Self::Persistence { rollback_error, .. } => rollback_error.as_ref(),
            Self::Read(_) | Self::Reload(_) | Self::Worker(_) => None,
        }
    }
}
