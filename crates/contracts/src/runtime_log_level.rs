use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Enumerates the only log-level values accepted across runtime transport boundaries.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "runtimeLogLevel.ts")]
pub enum RuntimeLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Requests the process-wide runtime log-level state without additional parameters.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "runtimeLogLevel.ts")]
pub struct GetRuntimeLogLevelRequest {}

/// Requests a new effective and persisted preferred process-wide log level.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "runtimeLogLevel.ts")]
pub struct SetRuntimeLogLevelRequest {
    pub level: RuntimeLogLevel,
}

/// Returns the runtime-authoritative preference, live filter, and startup explanation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "runtimeLogLevel.ts")]
pub struct RuntimeLogLevelStateResponse {
    pub configured_level: RuntimeLogLevel,
    pub effective_level: RuntimeLogLevel,
    pub startup_override: Option<RuntimeLogLevel>,
}

/// Exports the complete runtime log-level DTO family into one TypeScript module.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    RuntimeLogLevel::export(config)?;
    GetRuntimeLogLevelRequest::export(config)?;
    SetRuntimeLogLevelRequest::export(config)?;
    RuntimeLogLevelStateResponse::export(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::{
        GetRuntimeLogLevelRequest, RuntimeLogLevel, RuntimeLogLevelStateResponse,
        SetRuntimeLogLevelRequest,
    };

    /// Verifies request and response fields use the stable lower-camel wire schema.
    #[test]
    fn serializes_runtime_log_level_contracts() {
        assert_eq!(
            serde_json::to_value(GetRuntimeLogLevelRequest::default()).unwrap(),
            json!({})
        );
        assert_eq!(
            serde_json::to_value(SetRuntimeLogLevelRequest {
                level: RuntimeLogLevel::Debug,
            })
            .unwrap(),
            json!({ "level": "debug" })
        );
        assert_eq!(
            serde_json::to_value(RuntimeLogLevelStateResponse {
                configured_level: RuntimeLogLevel::Warn,
                effective_level: RuntimeLogLevel::Trace,
                startup_override: Some(RuntimeLogLevel::Trace),
            })
            .unwrap(),
            json!({
                "configuredLevel": "warn",
                "effectiveLevel": "trace",
                "startupOverride": "trace",
            })
        );
    }

    /// Verifies unsupported wire values are rejected by the closed enum deserializer.
    #[test]
    fn rejects_unsupported_runtime_log_level() {
        assert!(
            serde_json::from_value::<SetRuntimeLogLevelRequest>(json!({ "level": "verbose" }))
                .is_err()
        );
    }
}
