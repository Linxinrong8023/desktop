use crate::domain::paths::RepoRelativePath;
use crate::domain::refs::CommitId;
use crate::domain::worktree::WorktreeHandle;
use crate::error::GitlancerError;
use crate::exec::command::{GitCommand, GitIntent};
use crate::exec::env::GitEnv;
use crate::exec::runner::GitRunner;
use crate::git::Git;
use crate::parse::commit::parse_commit_response;

/// Carries the information needed to stage one or more repo-relative paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddRequest<'a> {
    pub worktree: &'a WorktreeHandle,
    pub paths: Vec<RepoRelativePath>,
}

/// Returns the paths that were requested for staging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddResponse {
    pub staged_paths: Vec<RepoRelativePath>,
}

/// Carries the worktree whose complete change set should be staged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageAllRequest<'a> {
    pub worktree: &'a WorktreeHandle,
}

/// Carries the information needed to unstage one or more repo-relative paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnstageRequest<'a> {
    pub worktree: &'a WorktreeHandle,
    pub paths: Vec<RepoRelativePath>,
}

/// Returns the paths that were requested for unstaging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnstageResponse {
    pub unstaged_paths: Vec<RepoRelativePath>,
}

/// Carries the information needed to create a commit in one worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest<'a> {
    pub worktree: &'a WorktreeHandle,
    pub message: &'a str,
    pub allow_empty: bool,
}

/// Returns the typed metadata upper layers typically need after a successful commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitResponse {
    pub commit_id: CommitId,
    pub summary: String,
}

impl<R: GitRunner> Git<R> {
    /// Stages repo-relative paths so callers never need to build `git add` commands themselves.
    pub fn add(&self, request: AddRequest<'_>) -> Result<AddResponse, GitlancerError> {
        let command = build_add_command(&request);
        let _output = self.runner().run(&command)?;

        Ok(AddResponse {
            staged_paths: request.paths,
        })
    }

    /// Stages tracked changes, deletions, and untracked files for an explicit task commit.
    pub fn stage_all(&self, request: StageAllRequest<'_>) -> Result<(), GitlancerError> {
        self.runner().run(&GitCommand::new(
            request.worktree.worktree_root().as_path().to_path_buf(),
            vec![
                "add".to_string(),
                "--all".to_string(),
                "--".to_string(),
                ".".to_string(),
            ],
            GitEnv::default(),
            GitIntent::Mutating,
        ))?;
        Ok(())
    }

    /// Moves the supplied repo-relative paths out of the index, leaving worktree edits intact.
    ///
    /// An empty path list is a defensive no-op: unstaging nothing should never run Git or error.
    pub fn unstage(&self, request: UnstageRequest<'_>) -> Result<UnstageResponse, GitlancerError> {
        if request.paths.is_empty() {
            return Ok(UnstageResponse {
                unstaged_paths: Vec::new(),
            });
        }
        let command = build_unstage_command(&request);
        let _output = self.runner().run(&command)?;

        Ok(UnstageResponse {
            unstaged_paths: request.paths,
        })
    }

    /// Creates one commit and returns typed metadata once the commit parser is implemented.
    pub fn commit(&self, request: CommitRequest<'_>) -> Result<CommitResponse, GitlancerError> {
        let command = build_commit_command(&request);
        let _output = self.runner().run(&command)?;
        let hash_output = self
            .runner()
            .run(&GitCommand::new(
                request.worktree.worktree_root().as_path().to_path_buf(),
                vec!["rev-parse".to_string(), "HEAD".to_string()],
                GitEnv::default(),
                GitIntent::ReadOnly,
            ))
            .map_err(|source| GitlancerError::CommitMetadataUnavailable {
                source: Box::new(GitlancerError::Exec(source)),
            })?;
        let summary_output = self
            .runner()
            .run(&GitCommand::new(
                request.worktree.worktree_root().as_path().to_path_buf(),
                vec![
                    "log".to_string(),
                    "-1".to_string(),
                    "--pretty=%s".to_string(),
                    "HEAD".to_string(),
                ],
                GitEnv::default(),
                GitIntent::ReadOnly,
            ))
            .map_err(|source| GitlancerError::CommitMetadataUnavailable {
                source: Box::new(GitlancerError::Exec(source)),
            })?;
        // Keep an explicit empty second line so an intentionally empty summary is still a valid response.
        let metadata = format!(
            "{}\n{}\n",
            hash_output.stdout.trim_end(),
            summary_output.stdout.trim_end()
        );

        parse_commit_response(&metadata)
            .map_err(GitlancerError::from)
            .map_err(|source| GitlancerError::CommitMetadataUnavailable {
                source: Box::new(source),
            })
    }
}

/// Builds a stable `git add` command so staging behavior can be tested independently from process execution.
pub fn build_add_command(request: &AddRequest<'_>) -> GitCommand {
    let mut args = vec!["add".to_string(), "--".to_string()];
    args.extend(
        request
            .paths
            .iter()
            .map(|path| path.as_path().to_string_lossy().into_owned()),
    );

    GitCommand::new(
        request.worktree.worktree_root().as_path().to_path_buf(),
        args,
        GitEnv::default(),
        GitIntent::Mutating,
    )
}

/// Builds a stable `git restore --staged` command so unstaging can be tested without process execution.
pub fn build_unstage_command(request: &UnstageRequest<'_>) -> GitCommand {
    let mut args = vec![
        "restore".to_string(),
        "--staged".to_string(),
        "--".to_string(),
    ];
    args.extend(
        request
            .paths
            .iter()
            .map(|path| path.as_path().to_string_lossy().into_owned()),
    );

    GitCommand::new(
        request.worktree.worktree_root().as_path().to_path_buf(),
        args,
        GitEnv::default(),
        GitIntent::Mutating,
    )
}

/// Builds a stable `git commit` command so commit policy and options stay centralized.
pub fn build_commit_command(request: &CommitRequest<'_>) -> GitCommand {
    let mut args = vec![
        "commit".to_string(),
        "--no-gpg-sign".to_string(),
        "-m".to_string(),
        request.message.to_string(),
    ];

    if request.allow_empty {
        args.push("--allow-empty".to_string());
    }

    GitCommand::new(
        request.worktree.worktree_root().as_path().to_path_buf(),
        args,
        GitEnv::default(),
        GitIntent::Mutating,
    )
}
#[cfg(test)]
mod tests {
    use super::{UnstageRequest, build_unstage_command};
    use crate::domain::paths::{RepoRelativePath, WorktreeRoot};
    use crate::domain::repo::Repository;
    use crate::domain::worktree::{WorktreeHandle, WorktreeKind};
    use crate::exec::command::{GitCommand, GitIntent};
    use crate::exec::env::GitEnv;
    use pretty_assertions::assert_eq;

    #[test]
    fn builds_unstage_command_for_the_requested_paths() {
        let worktree = worktree_fixture();
        let paths = vec![
            RepoRelativePath::new("a.txt"),
            RepoRelativePath::new("dir/b.txt"),
        ];

        let command = build_unstage_command(&UnstageRequest {
            worktree: &worktree,
            paths,
        });

        assert_eq!(
            command,
            GitCommand::new(
                worktree.worktree_root().as_path().to_path_buf(),
                vec![
                    "restore".to_string(),
                    "--staged".to_string(),
                    "--".to_string(),
                    "a.txt".to_string(),
                    "dir/b.txt".to_string(),
                ],
                GitEnv::default(),
                GitIntent::Mutating,
            )
        );
    }

    /// Builds an isolated linked worktree handle pointing at a stable fixture root.
    fn worktree_fixture() -> WorktreeHandle {
        let repository = Repository::new(crate::RepoRoot::new("/repo"));
        WorktreeHandle::new(
            repository.root().clone(),
            WorktreeRoot::new("/repo/wt"),
            crate::GitDir::new("/repo/.git/worktrees/wt"),
            WorktreeKind::Linked {
                name: "wt".to_string(),
            },
            None,
        )
    }
}
