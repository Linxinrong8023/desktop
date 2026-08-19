use std::sync::Arc;

use ora_application::{DeveloperMode, UserConfigService};
use ora_db::{RepositoryPool, SqliteUserConfigRepository};
use ora_logging::LogLevel;
use ora_runtime_settings::PreferredLogLevelStore;

use crate::BackendError;
use crate::bootstrap::spawn_repository_work;

/// Owns Backend's concrete typed user-configuration composition.
#[derive(Clone)]
pub(crate) struct UserConfigApi {
    service: UserConfigService<SqliteUserConfigRepository>,
}

/// Gives runtime logging only the Backend-owned preferred-level capability it requires.
#[derive(Clone)]
pub struct BackendPreferredLogLevelStore {
    user_config: Arc<UserConfigApi>,
}

impl BackendPreferredLogLevelStore {
    /// Restricts construction to the Backend composition root that owns the repository.
    pub(crate) fn new(user_config: Arc<UserConfigApi>) -> Self {
        Self { user_config }
    }
}

impl PreferredLogLevelStore for BackendPreferredLogLevelStore {
    type Error = BackendError;

    /// Loads the preferred level through Backend's non-blocking repository boundary.
    async fn load_preferred_level(&self) -> Result<LogLevel, Self::Error> {
        self.user_config.preferred_log_level().await
    }

    /// Persists the preferred level through Backend's non-blocking repository boundary.
    async fn save_preferred_level(&self, level: LogLevel) -> Result<(), Self::Error> {
        self.user_config.set_preferred_log_level(level).await?;
        Ok(())
    }
}

impl UserConfigApi {
    /// Builds the shared configuration use cases over SQLite.
    pub(crate) fn new(pool: RepositoryPool) -> Self {
        Self {
            service: UserConfigService::new(SqliteUserConfigRepository::new(pool)),
        }
    }

    /// Loads developer mode off the asynchronous runtime's worker threads.
    pub(crate) async fn developer_mode(&self) -> Result<DeveloperMode, BackendError> {
        let service = self.service.clone();
        spawn_repository_work(move || service.developer_mode().map_err(BackendError::from)).await
    }

    /// Persists developer mode off the asynchronous runtime's worker threads.
    pub(crate) async fn set_developer_mode(
        &self,
        mode: DeveloperMode,
    ) -> Result<DeveloperMode, BackendError> {
        let service = self.service.clone();
        spawn_repository_work(move || service.set_developer_mode(mode).map_err(BackendError::from))
            .await
    }

    /// Loads the preferred runtime log level off the asynchronous runtime's worker threads.
    pub(crate) async fn preferred_log_level(&self) -> Result<LogLevel, BackendError> {
        let service = self.service.clone();
        spawn_repository_work(move || service.preferred_log_level().map_err(BackendError::from))
            .await
    }

    /// Persists the preferred runtime log level off the asynchronous runtime's worker threads.
    pub(crate) async fn set_preferred_log_level(
        &self,
        level: LogLevel,
    ) -> Result<LogLevel, BackendError> {
        let service = self.service.clone();
        spawn_repository_work(move || {
            service
                .set_preferred_log_level(level)
                .map_err(BackendError::from)
        })
        .await
    }
}
