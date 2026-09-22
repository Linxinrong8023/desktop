//! Desktop workflow operations.

use ora_contracts::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

backend_command!(
    analyze_workflow,
    AnalyzeWorkflowRequest,
    AnalyzeWorkflowResponse,
    workflows.analyze,
    "Analyzes execution membership without preparing a run or changing its document."
);

/// Carries a user-selected destination and serialized workflow definition for export.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteWorkflowExportRequest {
    path: PathBuf,
    content: String,
}

/// Writes a workflow export after the desktop save dialog has selected its exact destination.
#[tauri::command]
pub async fn write_workflow_export(request: WriteWorkflowExportRequest) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || std::fs::write(request.path, request.content))
        .await
        .map_err(|error| format!("workflow export task failed: {error}"))?
        .map_err(|error| format!("workflow export write failed: {error}"))
}

/// Carries a user-dropped filesystem path for the import picker.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadWorkflowImportRequest {
    path: PathBuf,
}

/// Returns the dropped file's name and UTF-8 contents for the import preview.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadWorkflowImportResponse {
    name: String,
    size: u64,
    content: String,
}

/// Matches the editor's import size gate so a huge drop is rejected before reading.
const MAX_WORKFLOW_IMPORT_BYTES: u64 = 5 * 1024 * 1024;

/// Reads a dropped workflow file after the OS drag-drop event supplied its path.
///
/// WebView2 does not populate HTML5 `dataTransfer.files` for Explorer drops, so the
/// desktop host must load the path from Tauri's native drag-drop event instead.
#[tauri::command]
pub async fn read_workflow_import(
    request: ReadWorkflowImportRequest,
) -> Result<ReadWorkflowImportResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let metadata = std::fs::metadata(&request.path)
            .map_err(|error| format!("workflow import read failed: {error}"))?;
        if !metadata.is_file() {
            return Err("workflow import path is not a file".to_string());
        }
        let size = metadata.len();
        if size > MAX_WORKFLOW_IMPORT_BYTES {
            return Err("workflow import file is too large".to_string());
        }
        let name = request
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "workflow import file name is not utf-8".to_string())?
            .to_string();
        let content = std::fs::read_to_string(&request.path)
            .map_err(|error| format!("workflow import read failed: {error}"))?;
        Ok(ReadWorkflowImportResponse {
            name,
            size,
            content,
        })
    })
    .await
    .map_err(|error| format!("workflow import task failed: {error}"))?
}

backend_command!(
    create_workflow,
    CreateWorkflowRequest,
    CreateWorkflowResponse,
    workflows.create,
    "Creates one workflow through the shared Backend."
);
backend_command!(
    get_workflow,
    GetWorkflowRequest,
    GetWorkflowResponse,
    workflows.get,
    "Gets one workflow through the shared Backend."
);
backend_command!(
    list_workflows,
    ListWorkflowsRequest,
    ListWorkflowsResponse,
    workflows.list,
    "Lists workflows through the shared Backend."
);
backend_command!(
    update_workflow,
    UpdateWorkflowRequest,
    UpdateWorkflowResponse,
    workflows.update,
    "Updates one workflow through the shared Backend."
);
backend_command!(
    delete_workflow,
    DeleteWorkflowRequest,
    DeleteWorkflowResponse,
    workflows.delete,
    "Deletes one workflow through the shared Backend."
);
backend_command!(
    get_workflow_draft,
    GetDraftRequest,
    GetDraftResponse,
    workflows.get_draft,
    "Gets one workflow's draft snapshot through the shared Backend."
);
backend_command!(
    update_workflow_draft,
    UpdateDraftRequest,
    UpdateDraftResponse,
    workflows.update_draft,
    "Updates one workflow's draft graph through the shared Backend."
);
backend_command!(
    publish_workflow,
    PublishWorkflowRequest,
    PublishWorkflowResponse,
    workflows.publish,
    "Publishes one workflow draft through the shared Backend."
);
backend_command!(
    rollback_workflow,
    RollbackWorkflowRequest,
    RollbackWorkflowResponse,
    workflows.rollback,
    "Rolls back one workflow draft through the shared Backend."
);
backend_command!(
    activate_workflow,
    ActivateWorkflowRequest,
    ActivateWorkflowResponse,
    workflows.activate,
    "Activates one workflow version through the shared Backend."
);
backend_command!(
    list_workflow_versions,
    ListVersionsRequest,
    ListVersionsResponse,
    workflows.list_versions,
    "Lists one workflow's published versions through the shared Backend."
);
backend_command!(
    get_workflow_version,
    GetVersionRequest,
    GetVersionResponse,
    workflows.get_version,
    "Gets one workflow version snapshot through the shared Backend."
);
backend_command!(
    delete_workflow_snapshot,
    DeleteSnapshotRequest,
    DeleteSnapshotResponse,
    workflows.delete_snapshot,
    "Deletes one workflow snapshot through the shared Backend."
);
backend_command!(
    get_workflow_snapshot,
    GetWorkflowSnapshotRequest,
    GetWorkflowSnapshotResponse,
    workflows.get_snapshot,
    "Gets one snapshot by id through the shared Backend."
);
