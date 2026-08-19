use crate::WorkspaceFileEventBatch;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Selects the project checkout or task workspace whose specification documents are managed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export_to = "spec.ts")]
pub enum SpecTarget {
    Project {
        #[serde(rename = "projectId")]
        #[ts(rename = "projectId")]
        project_id: String,
    },
    Task {
        #[serde(rename = "taskId")]
        #[ts(rename = "taskId")]
        task_id: String,
    },
}

/// Identifies the workflow that owns a specification source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export_to = "spec.ts")]
pub enum SpecWorkflow {
    OpenSpec,
    Superpowers,
    Custom { name: String },
}

/// Describes one Markdown document assigned to an automatically detected source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "spec.ts")]
pub struct SpecDocument {
    pub relative_path: String,
    pub source_relative_path: String,
    pub workflow: SpecWorkflow,
    pub byte_size: u32,
}

/// Requests the bounded catalog for one project or task context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "spec.ts")]
pub struct GetSpecCatalogRequest {
    pub target: SpecTarget,
}

/// Returns the bounded Markdown document index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "spec.ts")]
pub struct SpecCatalogResponse {
    pub documents: Vec<SpecDocument>,
    pub truncated: bool,
}

/// Requests one catalog-authorized Markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "spec.ts")]
pub struct ReadSpecRequest {
    pub target: SpecTarget,
    pub relative_path: String,
}

/// Returns the raw, read-only Markdown payload and its exact size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "spec.ts")]
pub struct ReadSpecResponse {
    pub relative_path: String,
    pub content: String,
    pub byte_size: u32,
}

/// Starts specification-aware workspace file monitoring for one target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "spec.ts")]
pub struct WatchSpecsRequest {
    pub target: SpecTarget,
}

/// Gives the stream event type a spec-owned name while retaining the shared wire format.
pub type WatchSpecsEvent = WorkspaceFileEventBatch;

/// Exports every TypeScript binding declared in this module.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    SpecTarget::export(config)?;
    SpecWorkflow::export(config)?;
    SpecDocument::export(config)?;
    GetSpecCatalogRequest::export(config)?;
    SpecCatalogResponse::export(config)?;
    ReadSpecRequest::export(config)?;
    ReadSpecResponse::export(config)?;
    WatchSpecsRequest::export(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies target and custom workflow tagged unions retain the frontend wire shape.
    #[test]
    fn serializes_tagged_spec_contracts() {
        assert_eq!(
            serde_json::to_value(GetSpecCatalogRequest {
                target: SpecTarget::Task {
                    task_id: "task-1".to_string(),
                },
            })
            .unwrap(),
            json!({ "target": { "kind": "task", "taskId": "task-1" } })
        );
        assert_eq!(
            serde_json::to_value(SpecWorkflow::Custom {
                name: "Architecture".to_string(),
            })
            .unwrap(),
            json!({ "kind": "custom", "name": "Architecture" })
        );
    }
}
