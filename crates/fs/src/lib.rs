mod create;
mod delete;
mod entry_dest;
mod error;
mod relocate;
mod search;
mod watch;
#[cfg(any(test, feature = "test-support"))]
#[path = "watch/test_support.rs"]
pub mod watch_test_support;
mod workspace;

pub use error::WorkspaceFileSystemError;
pub use search::{SearchKind, SearchMatch, SearchResult, SearchResults};
pub use watch::{WorkspaceChange, WorkspaceChangeKind, WorkspaceWatcher};
pub use workspace::{
    DirectoryEntry, DirectoryEntryKind, DirectoryListing, ReadFile, WorkspaceFileSystem,
};
