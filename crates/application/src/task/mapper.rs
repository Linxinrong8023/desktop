use ora_contracts::{Task as ContractTask, TaskStatus as ContractTaskStatus, TaskWorkspaceMode};
use ora_domain::{Task as DomainTask, TaskStatus as DomainTaskStatus};

/// Maps a domain task into the app-facing contract shape.
pub(crate) fn map_task(task: DomainTask) -> ContractTask {
    let workspace_mode = match task.worktree_id {
        Some(_) => TaskWorkspaceMode::Worktree,
        None => TaskWorkspaceMode::ProjectRoot,
    };

    ContractTask {
        id: task.id.to_string(),
        project_id: task.project_id.to_string(),
        title: task.title,
        status: map_task_status(task.status),
        workspace_mode,
    }
}

/// Translates the internal task status into the transport-facing enum.
fn map_task_status(status: DomainTaskStatus) -> ContractTaskStatus {
    match status {
        DomainTaskStatus::Todo => ContractTaskStatus::Todo,
        DomainTaskStatus::Doing => ContractTaskStatus::Doing,
        DomainTaskStatus::Done => ContractTaskStatus::Done,
    }
}
