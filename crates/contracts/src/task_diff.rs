use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Selects which Git layer should be rendered in the task review surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub enum TaskDiffScope {
    Branch,
    Unstaged,
    Staged,
    Committed,
}

/// Identifies which task diff should be computed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub struct GetTaskDiffRequest {
    pub task_id: String,
    pub scope: TaskDiffScope,
}

/// Returns one standard unified patch and the revisions needed to render it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub struct GetTaskDiffResponse {
    pub base_commit_id: String,
    pub head_commit_id: String,
    pub patch: String,
}

/// Commits every current change in one task-owned worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub struct CommitTaskChangesRequest {
    pub task_id: String,
    pub message: String,
}

/// Returns the commit created from the task worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub struct CommitTaskChangesResponse {
    pub commit_id: String,
    pub summary: String,
}

/// Pushes the current task branch to its default remote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub struct PushTaskBranchRequest {
    pub task_id: String,
}

/// Returns the branch and remote updated by a successful push.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "task_diff.ts")]
pub struct PushTaskBranchResponse {
    pub branch_name: String,
    pub remote_name: String,
}

/// Exports every TypeScript binding owned by this module so the aggregate exporter can keep one call site per family.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    TaskDiffScope::export(config)?;
    GetTaskDiffRequest::export(config)?;
    GetTaskDiffResponse::export(config)?;
    CommitTaskChangesRequest::export(config)?;
    CommitTaskChangesResponse::export(config)?;
    PushTaskBranchRequest::export(config)?;
    PushTaskBranchResponse::export(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::GetTaskDiffResponse;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies task diff payloads use the camel-case shape consumed by generated clients.
    #[test]
    fn serializes_task_diff_contracts() {
        let response = GetTaskDiffResponse {
            base_commit_id: "base".to_string(),
            head_commit_id: "head".to_string(),
            patch: "patch".to_string(),
        };

        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({
                "baseCommitId": "base",
                "headCommitId": "head",
                "patch": "patch",
            })
        );
    }
}
