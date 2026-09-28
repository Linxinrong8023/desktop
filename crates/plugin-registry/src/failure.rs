//! Records the marketplace sources one registry-index rebuild could not refresh.
//!
//! A rebuild must not drop a failing source's listings: a repository that is temporarily
//! unreachable would look like one that withdrew its plugins. The rebuild therefore keeps the
//! listings the previous index already carried and records the failure beside them, and that
//! record is persisted with the cache because the marketplace page reads the cache rather than
//! the sync response that produced it.

use gitlancer::{GitExecError, GitlancerError};
use ora_utils::url::canonical_repository_url;
use serde::{Deserialize, Serialize};

use crate::error::RegistryError;

/// Describes one marketplace source whose refresh failed while the index was rebuilt.
///
/// The URL is canonicalized on construction because it is matched against the canonical URL every
/// index entry is attributed to: a source that failed must be recognized no matter which spelling
/// of its URL the caller used.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegistrySourceFailure {
    url: String,
    message: String,
}

impl RegistrySourceFailure {
    /// Records one failed source and the reason its refresh failed.
    pub fn new(url: impl AsRef<str>, message: impl Into<String>) -> Self {
        Self {
            url: canonical_repository_url(url.as_ref()),
            message: message.into(),
        }
    }

    /// Records one source whose [`crate::RegistrySync::sync`] failed, keeping only the part of
    /// `error` a user can act on.
    ///
    /// The message is shown on the marketplace page and persisted in the cache, while the complete
    /// error belongs in the log written where the refresh failed. A failed Git command renders its
    /// whole argument list — the local checkout path included — ahead of Git's own report, so only
    /// Git's `fatal:` / `error:` lines are kept: they name the remote and the reason, and Git
    /// already strips credentials from the remote URLs it prints there.
    pub fn from_sync_error(url: impl AsRef<str>, error: &RegistryError) -> Self {
        let message = match error {
            RegistryError::Git(GitlancerError::Exec(exec)) => match exec {
                GitExecError::NonZeroExit { code, stderr, .. } => {
                    let lines: Vec<&str> = stderr
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect();
                    let diagnostics: Vec<&str> = lines
                        .iter()
                        .copied()
                        .filter(|line| line.starts_with("fatal:") || line.starts_with("error:"))
                        .collect();
                    // Progress chatter such as `Cloning into '…'` precedes the diagnostic, so a
                    // report without a tagged line falls back to its last line rather than its first.
                    if !diagnostics.is_empty() {
                        diagnostics.join(" ")
                    } else if let Some(last) = lines.last() {
                        (*last).to_owned()
                    } else if let Some(code) = code {
                        format!("git exited with code {code}")
                    } else {
                        "git was terminated before it exited".to_owned()
                    }
                }
                GitExecError::SpawnFailed { source, .. } => {
                    format!("failed to start git: {source}")
                }
                GitExecError::GitNotFound
                | GitExecError::OutputTooLarge { .. }
                | GitExecError::OutputReadFailed { .. } => exec.to_string(),
            },
            RegistryError::Git(_)
            | RegistryError::Manifest(_)
            | RegistryError::SourceUrl(_)
            | RegistryError::SourceBranch(_)
            | RegistryError::Io(_)
            | RegistryError::Json(_)
            | RegistryError::UnsupportedIndexVersion { .. }
            | RegistryError::MissingCloneParent(_) => error.to_string(),
        };
        Self::new(url, message)
    }

    /// Returns the canonical URL of the source that could not be refreshed.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the failure description shown to the user and written to the log.
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const SOURCE_URL: &str =
        "https://szv-y.codehub.example.com/AI_Coding_Lab/ora-space-marketplace";

    /// Wraps one Git exit report the way a failed `RegistrySync::sync` surfaces it.
    fn git_exit(code: Option<i32>, stderr: &str) -> RegistryError {
        RegistryError::Git(GitlancerError::Exec(GitExecError::NonZeroExit {
            code,
            args: vec![
                "clone".to_owned(),
                "--branch".to_owned(),
                "master".to_owned(),
                SOURCE_URL.to_owned(),
                r"C:\Users\alice\.ora\plugins\sources\szv-y.codehub.example.com".to_owned(),
            ],
            stdout: String::new(),
            stderr: stderr.to_owned(),
        }))
    }

    /// Verifies a recorded failure carries Git's own diagnosis and never the command line or the
    /// local checkout path it ran against.
    #[test]
    fn keeps_only_the_actionable_part_of_a_sync_error() {
        let cases = [
            git_exit(
                Some(128),
                "Cloning into 'C:\\Users\\alice\\.ora\\plugins\\sources\\szv-y'...\n\
                 fatal: unable to access 'https://szv-y.codehub.example.com/AI_Coding_Lab/ora-space-marketplace/': \
                 schannel: failed to receive handshake, SSL/TLS connection failed\n",
            ),
            git_exit(
                Some(128),
                "error: RPC failed; curl 56 Recv failure\nfatal: early EOF\n",
            ),
            git_exit(Some(1), "hint: something unusual\nremote: access denied\n"),
            git_exit(Some(128), "  \n"),
            git_exit(/*code*/ None, ""),
            RegistryError::Git(GitlancerError::Exec(GitExecError::SpawnFailed {
                args: vec!["fetch".to_owned(), "origin".to_owned()],
                source: std::io::Error::other("access is denied"),
            })),
            RegistryError::Git(GitlancerError::Exec(GitExecError::GitNotFound)),
            RegistryError::Io(std::io::Error::other("disk full")),
        ];

        assert_eq!(
            cases
                .iter()
                .map(|error| RegistrySourceFailure::from_sync_error(SOURCE_URL, error))
                .collect::<Vec<_>>(),
            vec![
                RegistrySourceFailure::new(
                    SOURCE_URL,
                    "fatal: unable to access 'https://szv-y.codehub.example.com/AI_Coding_Lab/ora-space-marketplace/': \
                     schannel: failed to receive handshake, SSL/TLS connection failed",
                ),
                RegistrySourceFailure::new(
                    SOURCE_URL,
                    "error: RPC failed; curl 56 Recv failure fatal: early EOF",
                ),
                RegistrySourceFailure::new(SOURCE_URL, "remote: access denied"),
                RegistrySourceFailure::new(SOURCE_URL, "git exited with code 128"),
                RegistrySourceFailure::new(SOURCE_URL, "git was terminated before it exited"),
                RegistrySourceFailure::new(SOURCE_URL, "failed to start git: access is denied"),
                RegistrySourceFailure::new(SOURCE_URL, "Git executable not found"),
                RegistrySourceFailure::new(SOURCE_URL, "registry file operation failed: disk full",),
            ],
        );
    }
}
