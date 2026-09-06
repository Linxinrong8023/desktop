use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

/// Identifies one workbench page instance and the plugin generation serving it.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[ts(export_to = "workbench.ts")]
pub struct WorkbenchSurface {
    #[ts(type = "number")]
    pub instance_id: u64,
    #[ts(type = "number")]
    pub generation: u64,
}

/// Host envelope carrying workbench surface identity and page-supplied input.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq)]
#[ts(export_to = "workbench.ts")]
pub struct WorkbenchCallParams {
    pub surface: WorkbenchSurface,
    #[ts(type = "import(\"./json.ts\").JsonValue")]
    pub input: Value,
}

/// Exports the workbench identity DTO shared by the host and SDK.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    WorkbenchSurface::export(config)?;
    WorkbenchCallParams::export(config)?;
    Ok(())
}
