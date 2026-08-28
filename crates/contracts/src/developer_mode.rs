use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Requests the shared developer-mode preference without additional parameters.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "developerMode.ts")]
pub struct GetDeveloperModeRequest {}

/// Requests a new shared developer-mode preference.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "developerMode.ts")]
pub struct SetDeveloperModeRequest {
    pub enabled: bool,
}

/// Returns the authoritative shared developer-mode preference.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "developerMode.ts")]
pub struct DeveloperModeResponse {
    pub enabled: bool,
}

/// Exports the complete developer-mode DTO family into one TypeScript module.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    GetDeveloperModeRequest::export(config)?;
    SetDeveloperModeRequest::export(config)?;
    DeveloperModeResponse::export(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::{DeveloperModeResponse, GetDeveloperModeRequest, SetDeveloperModeRequest};

    /// Verifies developer-mode requests and responses use one authoritative boolean field.
    #[test]
    fn serializes_developer_mode_contracts() {
        assert_eq!(
            serde_json::to_value(GetDeveloperModeRequest::default()).unwrap(),
            json!({})
        );
        assert_eq!(
            serde_json::to_value(SetDeveloperModeRequest { enabled: true }).unwrap(),
            json!({ "enabled": true })
        );
        assert_eq!(
            serde_json::to_value(DeveloperModeResponse { enabled: false }).unwrap(),
            json!({ "enabled": false })
        );
    }
}
