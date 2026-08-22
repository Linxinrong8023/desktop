use crate::BoxRepositorySource;
use std::path::PathBuf;
use thiserror::Error;

/// Supplies task-scoped Git differences while hiding Git and filesystem implementation details.
///
/// Implementations must restrict execution to the backend-resolved worktree in each request.
pub trait TaskDiffReader {
    /// Computes all task changes against the baseline selected by the composition root.
    fn read_task_diff(
        &self,
        request: ReadTaskDiffRequest,
    ) -> Result<TaskDiffSnapshot, TaskDiffReaderError>;
}

/// Selects the Git layer represented by a task diff snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadTaskDiffScope {
    Branch,
    Unstaged,
    Staged,
    Committed,
}

/// Carries the backend-owned worktree path and immutable comparison baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadTaskDiffRequest {
    pub worktree_path: PathBuf,
    pub base_commit_id: String,
    pub scope: ReadTaskDiffScope,
}

/// Returns the Git revisions and unified patch used by frontend review components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDiffSnapshot {
    pub head_commit_id: String,
    pub patch: String,
}

/// Captures Git-backed diff failures converted into stable application errors by handlers.
#[derive(Debug, Error)]
pub enum TaskDiffReaderError {
    #[error("task diff operation failed")]
    OperationFailed(#[source] BoxRepositorySource),
    /// Indicates that the diff exceeded the bounded response budget.
    #[error("task diff is too large: {byte_count} bytes exceeds {max_byte_count} bytes")]
    TooLarge {
        byte_count: usize,
        max_byte_count: usize,
    },
}

impl TaskDiffReaderError {
    /// Wraps an infrastructure failure without flattening its `Error::source()` chain.
    pub fn operation_failed(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::OperationFailed(Box::new(error))
    }
}

/// Supplies task-scoped Git writes while keeping command execution outside handlers.
pub trait TaskGitWriter {
    /// Stages and commits every current worktree change.
    fn commit_changes(
        &self,
        request: CommitTaskGitRequest,
    ) -> Result<TaskGitCommit, TaskGitWriterError>;

    /// Pushes the verified task branch to its default remote.
    fn push_branch(&self, request: PushTaskGitRequest) -> Result<TaskGitPush, TaskGitWriterError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitTaskGitRequest {
    pub worktree_path: PathBuf,
    pub expected_branch_name: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushTaskGitRequest {
    pub worktree_path: PathBuf,
    pub expected_branch_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGitCommit {
    pub commit_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGitPush {
    pub branch_name: String,
    pub remote_name: String,
}

#[derive(Debug, Error)]
pub enum TaskGitWriterError {
    /// Indicates that a Git write could not be completed.
    #[error("task Git write failed")]
    OperationFailed(#[source] BoxRepositorySource),
}

impl TaskGitWriterError {
    /// Wraps a Git failure without flattening its `Error::source()` chain.
    pub fn operation_failed(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::OperationFailed(Box::new(error))
    }
}
