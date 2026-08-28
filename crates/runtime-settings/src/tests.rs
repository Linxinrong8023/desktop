use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use ora_logging::LogLevel;
use pretty_assertions::assert_eq;
use thiserror::Error;
use tokio::sync::Notify;

use crate::{
    PreferredLogLevelStore, RuntimeLogLevelControl, RuntimeLogLevelManager, RuntimeLogLevelState,
    RuntimeLogLevelUpdateError,
};

/// Verifies a successful update reloads, persists, and returns one authoritative snapshot.
#[tokio::test]
async fn updates_effective_and_preferred_levels_together() {
    let control = FakeControl::new(LogLevel::Info);
    let store = FakeStore::new(LogLevel::Info);
    let manager = RuntimeLogLevelManager::new(
        control.clone(),
        store.clone(),
        LogLevel::Info,
        Some(LogLevel::Trace),
    );

    assert_eq!(
        manager.set_level(LogLevel::Warn).await.unwrap(),
        RuntimeLogLevelState {
            configured_level: LogLevel::Warn,
            effective_level: LogLevel::Warn,
            startup_override: Some(LogLevel::Trace),
        }
    );
    assert_eq!(control.level(), LogLevel::Warn);
    assert_eq!(store.level(), LogLevel::Warn);
}

/// Verifies a reload failure prevents any persistence attempt.
#[tokio::test]
async fn leaves_persistence_unchanged_when_reload_fails() {
    let control = FakeControl::new(LogLevel::Info);
    control.fail_next_set();
    let store = FakeStore::new(LogLevel::Info);
    let manager = RuntimeLogLevelManager::new(control.clone(), store.clone(), LogLevel::Info, None);

    let error = manager.set_level(LogLevel::Debug).await.unwrap_err();

    assert!(matches!(error, RuntimeLogLevelUpdateError::Reload(_)));
    assert_eq!(control.level(), LogLevel::Info);
    assert_eq!(store.save_history(), Vec::<LogLevel>::new());
}

/// Verifies storage failure restores the previous live level and remains the primary error.
#[tokio::test]
async fn rolls_back_effective_level_when_persistence_fails() {
    let control = FakeControl::new(LogLevel::Info);
    let store = FakeStore::new(LogLevel::Info);
    store.fail_next_save();
    let manager = RuntimeLogLevelManager::new(control.clone(), store.clone(), LogLevel::Info, None);

    let error = manager.set_level(LogLevel::Debug).await.unwrap_err();

    assert!(matches!(
        error,
        RuntimeLogLevelUpdateError::Persistence {
            rollback_error: None,
            ..
        }
    ));
    assert_eq!(control.level(), LogLevel::Info);
    assert_eq!(store.level(), LogLevel::Info);
}

/// Verifies a failed rollback remains secondary while the storage failure stays primary.
#[tokio::test]
async fn reports_rollback_failure_separately() {
    let control = FakeControl::new(LogLevel::Info);
    control.set_failures([false, true]);
    let store = FakeStore::new(LogLevel::Info);
    store.fail_next_save();
    let manager = RuntimeLogLevelManager::new(control.clone(), store.clone(), LogLevel::Info, None);

    let error = manager.set_level(LogLevel::Trace).await.unwrap_err();

    assert!(matches!(
        error,
        RuntimeLogLevelUpdateError::Persistence {
            rollback_error: Some(_),
            ..
        }
    ));
    assert!(error.rollback_error().is_some());
    assert_eq!(control.level(), LogLevel::Trace);
    assert_eq!(store.level(), LogLevel::Info);
}

/// Verifies cloned managers serialize updates and finish with the last completed successful value.
#[tokio::test(flavor = "multi_thread")]
async fn serializes_concurrent_updates() {
    let control = FakeControl::new(LogLevel::Info);
    let store = FakeStore::new(LogLevel::Info);
    let manager = RuntimeLogLevelManager::new(control.clone(), store.clone(), LogLevel::Info, None);
    let handles = [LogLevel::Debug, LogLevel::Warn, LogLevel::Error]
        .into_iter()
        .map(|level| {
            let manager = manager.clone();
            tokio::spawn(async move { manager.set_level(level).await.unwrap() })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.await.unwrap();
    }
    let save_history = store.save_history();
    let last_level = *save_history.last().unwrap();

    assert_eq!(
        manager.state().await.unwrap(),
        RuntimeLogLevelState {
            configured_level: last_level,
            effective_level: last_level,
            startup_override: None,
        }
    );
    assert_eq!(control.level(), last_level);
    assert_eq!(store.level(), last_level);
    assert_eq!(save_history.len(), 3);
}

/// Verifies cancelling a waiting caller cannot leave a successful transaction's cache stale.
#[tokio::test]
async fn completes_successful_update_after_waiting_caller_is_cancelled() {
    let control = FakeControl::new(LogLevel::Info);
    let store = GatedStore::new(LogLevel::Info, SaveOutcome::Succeed);
    let manager = RuntimeLogLevelManager::new(control.clone(), store.clone(), LogLevel::Info, None);
    let update = tokio::spawn({
        let manager = manager.clone();
        async move { manager.set_level(LogLevel::Debug).await }
    });
    store.wait_until_save_started().await;

    update.abort();
    assert!(update.await.unwrap_err().is_cancelled());
    store.release_save();

    assert_eq!(
        manager.state().await.unwrap(),
        RuntimeLogLevelState {
            configured_level: LogLevel::Debug,
            effective_level: LogLevel::Debug,
            startup_override: None,
        }
    );
    assert_eq!(control.level(), LogLevel::Debug);
    assert_eq!(store.level(), LogLevel::Debug);
}

/// Verifies cancelling a waiting caller cannot prevent rollback after persistence fails.
#[tokio::test]
async fn completes_failed_update_rollback_after_waiting_caller_is_cancelled() {
    let control = FakeControl::new(LogLevel::Info);
    let store = GatedStore::new(LogLevel::Info, SaveOutcome::Fail);
    let manager = RuntimeLogLevelManager::new(control.clone(), store.clone(), LogLevel::Info, None);
    let update = tokio::spawn({
        let manager = manager.clone();
        async move { manager.set_level(LogLevel::Debug).await }
    });
    store.wait_until_save_started().await;

    update.abort();
    assert!(update.await.unwrap_err().is_cancelled());
    store.release_save();

    assert_eq!(
        manager.state().await.unwrap(),
        RuntimeLogLevelState {
            configured_level: LogLevel::Info,
            effective_level: LogLevel::Info,
            startup_override: None,
        }
    );
    assert_eq!(control.level(), LogLevel::Info);
    assert_eq!(store.level(), LogLevel::Info);
}

#[derive(Clone, Debug)]
struct FakeControl {
    state: Arc<Mutex<FakeControlState>>,
}

#[derive(Debug)]
struct FakeControlState {
    level: LogLevel,
    set_failures: VecDeque<bool>,
}

impl FakeControl {
    /// Creates a deterministic live-filter fake at the supplied level.
    fn new(level: LogLevel) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeControlState {
                level,
                set_failures: VecDeque::new(),
            })),
        }
    }

    /// Makes only the next reload fail.
    fn fail_next_set(&self) {
        self.set_failures([true]);
    }

    /// Replaces the deterministic reload failure schedule.
    fn set_failures(&self, failures: impl IntoIterator<Item = bool>) {
        self.state.lock().unwrap().set_failures = failures.into_iter().collect();
    }

    /// Reads the fake's effective level for whole-state assertions.
    fn level(&self) -> LogLevel {
        self.state.lock().unwrap().level
    }
}

impl RuntimeLogLevelControl for FakeControl {
    type ReadError = FakeControlError;
    type ReloadError = FakeControlError;

    fn current_level(&self) -> Result<LogLevel, Self::ReadError> {
        Ok(self.level())
    }

    fn set_level(&self, level: LogLevel) -> Result<(), Self::ReloadError> {
        let mut state = self.state.lock().unwrap();
        if state.set_failures.pop_front().unwrap_or(false) {
            return Err(FakeControlError);
        }
        state.level = level;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error)]
#[error("fake control failure")]
struct FakeControlError;

#[derive(Clone, Debug)]
struct FakeStore {
    state: Arc<Mutex<FakeStoreState>>,
}

#[derive(Debug)]
struct FakeStoreState {
    level: LogLevel,
    fail_next_save: bool,
    save_history: Vec<LogLevel>,
}

impl FakeStore {
    /// Creates an in-memory preferred-level store.
    fn new(level: LogLevel) -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeStoreState {
                level,
                fail_next_save: false,
                save_history: Vec::new(),
            })),
        }
    }

    /// Makes only the next persistence attempt fail atomically.
    fn fail_next_save(&self) {
        self.state.lock().unwrap().fail_next_save = true;
    }

    /// Reads the persisted level represented by the fake.
    fn level(&self) -> LogLevel {
        self.state.lock().unwrap().level
    }

    /// Returns the successful save order used by concurrency assertions.
    fn save_history(&self) -> Vec<LogLevel> {
        self.state.lock().unwrap().save_history.clone()
    }
}

impl PreferredLogLevelStore for FakeStore {
    type Error = FakeStoreError;

    async fn load_preferred_level(&self) -> Result<LogLevel, Self::Error> {
        Ok(self.level())
    }

    async fn save_preferred_level(&self, level: LogLevel) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        if state.fail_next_save {
            state.fail_next_save = false;
            return Err(FakeStoreError);
        }
        state.level = level;
        state.save_history.push(level);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error)]
#[error("fake store failure")]
struct FakeStoreError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SaveOutcome {
    Succeed,
    Fail,
}

#[derive(Clone)]
struct GatedStore {
    state: Arc<Mutex<GatedStoreState>>,
    save_started: Arc<Notify>,
    release_save: Arc<Notify>,
}

struct GatedStoreState {
    level: LogLevel,
    outcome: SaveOutcome,
}

impl GatedStore {
    /// Creates a store whose save result remains blocked until the test releases it.
    fn new(level: LogLevel, outcome: SaveOutcome) -> Self {
        Self {
            state: Arc::new(Mutex::new(GatedStoreState { level, outcome })),
            save_started: Arc::new(Notify::new()),
            release_save: Arc::new(Notify::new()),
        }
    }

    /// Waits until the transaction has changed the live filter and entered persistence.
    async fn wait_until_save_started(&self) {
        self.save_started.notified().await;
    }

    /// Allows the pending persistence operation to commit or fail deterministically.
    fn release_save(&self) {
        self.release_save.notify_one();
    }

    /// Reads the fake's persisted level after the transaction finishes.
    fn level(&self) -> LogLevel {
        self.state.lock().unwrap().level
    }
}

impl PreferredLogLevelStore for GatedStore {
    type Error = FakeStoreError;

    async fn load_preferred_level(&self) -> Result<LogLevel, Self::Error> {
        Ok(self.level())
    }

    async fn save_preferred_level(&self, level: LogLevel) -> Result<(), Self::Error> {
        self.save_started.notify_one();
        self.release_save.notified().await;
        let mut state = self.state.lock().unwrap();
        match state.outcome {
            SaveOutcome::Succeed => {
                state.level = level;
                Ok(())
            }
            SaveOutcome::Fail => Err(FakeStoreError),
        }
    }
}
