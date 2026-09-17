//! Exercises the Settings adapter with real transactions and injected control/store failures.

use super::update_runtime_log_level;
use crate::commands::execution_tests::Recorder;
use ora_contracts::{
    ContractError, EmptyErrorParams, PublicError, RuntimeLogLevel, SetRuntimeLogLevelRequest,
};
use ora_logging::{LogLevel, with_recorded_trace_logging};
use ora_runtime_settings::{
    PreferredLogLevelStore, RuntimeLogLevelControl, RuntimeLogLevelManager,
};
use pretty_assertions::assert_eq;

#[derive(Clone)]
struct RollbackFailingControl;

impl RuntimeLogLevelControl for RollbackFailingControl {
    type ReadError = std::io::Error;
    type ReloadError = std::io::Error;

    /// Supplies the level that the real transaction will attempt to restore.
    fn current_level(&self) -> Result<LogLevel, Self::ReadError> {
        Ok(LogLevel::Info)
    }

    /// Accepts the requested level but rejects compensation.
    fn set_level(&self, level: LogLevel) -> Result<(), Self::ReloadError> {
        match level {
            LogLevel::Info => Err(std::io::Error::other("rollback probe")),
            LogLevel::Trace | LogLevel::Debug | LogLevel::Warn | LogLevel::Error => Ok(()),
        }
    }
}

#[derive(Clone)]
struct FailingStore;

impl PreferredLogLevelStore for FailingStore {
    type Error = std::io::Error;

    /// Supplies the persisted default without touching process state.
    async fn load_preferred_level(&self) -> Result<LogLevel, Self::Error> {
        Ok(LogLevel::Info)
    }

    /// Triggers the transaction's real rollback path.
    async fn save_preferred_level(&self, _level: LogLevel) -> Result<(), Self::Error> {
        Err(std::io::Error::other("persistence probe"))
    }
}

/// Settings retains rollback diagnostics while the executor completes the primary failure once.
#[test]
fn rollback_failure_shares_the_public_error_request_id_and_one_completion() {
    let recorder = Recorder::default();
    let error = with_recorded_trace_logging(recorder.clone(), || {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                let manager = RuntimeLogLevelManager::new(
                    RollbackFailingControl,
                    FailingStore,
                    LogLevel::Info,
                );
                update_runtime_log_level(
                    manager,
                    SetRuntimeLogLevelRequest {
                        level: RuntimeLogLevel::Debug,
                    },
                )
                .await
                .unwrap_err()
            })
    });
    let public: ContractError =
        serde_json::from_value(serde_json::to_value(error).unwrap()).unwrap();
    assert_eq!(
        public,
        ContractError {
            error: PublicError::InternalError(EmptyErrorParams {}),
            request_id: public.request_id,
        }
    );

    let events = recorder.0.lock().unwrap();
    let relevant: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(
                event.fields.0.get("message").map(String::as_str),
                Some("request completed" | "secondary cleanup failed")
            )
        })
        .collect();
    let request_id = public.request_id.to_string();
    assert_eq!(
        relevant
            .iter()
            .map(|event| {
                ["message", "operation", "outcome", "request_id"]
                    .map(|key| event.fields.0.get(key).unwrap().as_str())
            })
            .collect::<Vec<_>>(),
        vec![
            [
                "secondary cleanup failed",
                "set_runtime_log_level.rollback",
                "secondary_failure",
                request_id.as_str()
            ],
            [
                "request completed",
                "set_runtime_log_level",
                "failure",
                request_id.as_str()
            ],
        ]
    );
    assert_eq!(relevant[0].scope, relevant[1].scope);
    assert_eq!(relevant[0].scope.len(), 1);
    assert_eq!(relevant[0].scope[0].0["request_id"], request_id);
    assert_eq!(relevant[0].fields.0["error.message"], "rollback probe");
    assert!(relevant[1].fields.0["error.chain"].contains("persistence probe"));
    assert!(!relevant[1].fields.0["error.chain"].contains("rollback probe"));
}
