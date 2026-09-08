use crate::domain::worktree::WorktreeHandle;
use crate::error::GitlancerError;
use crate::exec::command::{GitCommand, GitIntent};
use crate::exec::env::GitEnv;
use crate::exec::runner::GitRunner;
use crate::git::Git;
use crate::parse;

/// Carries the information needed to read structured status information from one worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRequest<'a> {
    pub worktree: &'a WorktreeHandle,
}

/// Represents the high-level status view returned to upper layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusResponse {
    pub entries: Vec<StatusEntry>,
}

/// Represents one structured worktree status entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub raw: String,
}

/// Carries the information needed to read structured per-file status for one worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntriesRequest<'a> {
    pub worktree: &'a WorktreeHandle,
}

/// Represents one per-file staging status entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusFileEntry {
    pub path: String,
    pub is_staged: bool,
    pub is_untracked: bool,
}

/// Represents the structured per-file status view returned to upper layers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntriesResponse {
    pub entries: Vec<StatusFileEntry>,
}

impl<R: GitRunner> Git<R> {
    /// Returns worktree status using porcelain v2 so callers can reason about changes without ad-hoc parsing.
    pub fn status(&self, request: StatusRequest<'_>) -> Result<StatusResponse, GitlancerError> {
        let command = GitCommand::new(
            request.worktree.worktree_root().as_path().to_path_buf(),
            vec![
                "status".to_string(),
                "--porcelain=v2".to_string(),
                "-z".to_string(),
            ],
            GitEnv::default(),
            GitIntent::ReadOnly,
        );
        let output = self.runner().run(&command)?;
        let entries = parse::status::parse_status_v2(&output.stdout)?;

        Ok(StatusResponse { entries })
    }

    /// Returns structured per-file staging status so the review panel can render stage toggles.
    pub fn status_entries(
        &self,
        request: StatusEntriesRequest<'_>,
    ) -> Result<StatusEntriesResponse, GitlancerError> {
        let command = GitCommand::new(
            request.worktree.worktree_root().as_path().to_path_buf(),
            vec![
                "status".to_string(),
                "--porcelain=v2".to_string(),
                "-z".to_string(),
            ],
            GitEnv::default(),
            GitIntent::ReadOnly,
        );
        let output = self.runner().run(&command)?;
        let entries = parse::status::parse_status_v2_entries(&output.stdout)?;

        Ok(StatusEntriesResponse { entries })
    }
}
