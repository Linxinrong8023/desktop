mod retirement;

use crate::WorkspaceFileSystemError;
use crate::workspace::{canonical_root, relative_string};
use notify::event::ModifyKind;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use ora_utils::path::CanonicalPathRoot;
use retirement::NativeEvents;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// Identifies the cache invalidation implied by one native filesystem event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceChangeKind {
    Created,
    Modified,
    Removed,
    Renamed { from: String },
    RescanRequired,
}

/// Describes one workspace-relative file change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceChange {
    pub path: String,
    pub kind: WorkspaceChangeKind,
}

/// Owns one native recursive watcher and batches its platform-specific callback events.
pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    events: Receiver<notify::Result<Event>>,
    root: CanonicalPathRoot,
    retired: oneshot::Receiver<Result<(), &'static str>>,
    shutdown_requested: Arc<AtomicBool>,
}

impl WorkspaceWatcher {
    /// Starts watching one canonical workspace root recursively.
    pub fn start(root: &Path) -> Result<Self, WorkspaceFileSystemError> {
        Self::start_with_event_hook(root, || {})
    }

    /// Keeps the native callback boundary injectable for blocked-release verification.
    pub(crate) fn start_with_event_hook(
        root: &Path,
        mut before_event: impl FnMut() + Send + 'static,
    ) -> Result<Self, WorkspaceFileSystemError> {
        let root = canonical_root(root)?;
        let (sender, events) = mpsc::channel();
        let (retirement, retired) = oneshot::channel();
        let shutdown_requested = Arc::new(AtomicBool::new(/*v*/ false));
        let handler = NativeEvents::new(
            move |event| {
                before_event();
                let _ = sender.send(event);
            },
            retirement,
            shutdown_requested.clone(),
        );
        let mut watcher = notify::recommended_watcher(handler).map_err(|error| {
            WorkspaceFileSystemError::WatchFailed {
                path: root.as_path().to_path_buf(),
                message: error.to_string(),
            }
        })?;
        watcher
            .watch(root.as_path(), RecursiveMode::Recursive)
            .map_err(|error| WorkspaceFileSystemError::WatchFailed {
                path: root.as_path().to_path_buf(),
                message: error.to_string(),
            })?;
        Ok(Self {
            _watcher: watcher,
            events,
            root,
            retired,
            shutdown_requested,
        })
    }

    /// Waits for native handles and the event callback to be released, not just Drop to return.
    /// The caller may time out this wait; the blocking shutdown continues independently.
    pub async fn close(self) -> Result<(), WorkspaceFileSystemError> {
        let Self {
            _watcher,
            events,
            root,
            retired,
            shutdown_requested,
        } = self;
        let failure = |message| WorkspaceFileSystemError::WatchFailed {
            path: root.as_path().to_path_buf(),
            message,
        };
        tokio::task::spawn_blocking(move || {
            shutdown_requested.store(/*val*/ true, Ordering::SeqCst);
            drop(_watcher);
            drop(events);
        })
        .await
        .map_err(|error| failure(format!("native watcher shutdown failed: {error}")))?;
        retired
            .await
            .map_err(|error| failure(format!("native release was not confirmed: {error}")))?
            .map_err(|message| failure(message.to_string()))
    }

    /// Waits for one event and coalesces follow-up events arriving within the debounce window.
    pub fn receive_batch(
        &self,
        debounce: Duration,
    ) -> Result<Option<Vec<WorkspaceChange>>, WorkspaceFileSystemError> {
        let first = match self.events.recv_timeout(debounce) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => return Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                return Err(WorkspaceFileSystemError::WatchFailed {
                    path: self.root.as_path().to_path_buf(),
                    message: "native watcher disconnected".to_string(),
                });
            }
        };
        let deadline = Instant::now() + debounce;
        let mut changes = self.map_event(first)?;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            match self.events.recv_timeout(remaining) {
                Ok(event) => changes.extend(self.map_event(event)?),
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        changes.sort_by(|left, right| left.path.cmp(&right.path));
        changes.dedup();
        Ok(Some(changes))
    }

    /// Converts native event shapes into stable workspace-relative invalidation events.
    fn map_event(
        &self,
        event: notify::Result<Event>,
    ) -> Result<Vec<WorkspaceChange>, WorkspaceFileSystemError> {
        let event = event.map_err(|error| WorkspaceFileSystemError::WatchFailed {
            path: self.root.as_path().to_path_buf(),
            message: error.to_string(),
        })?;
        if matches!(event.kind, EventKind::Other | EventKind::Any) {
            return Ok(vec![WorkspaceChange {
                path: String::new(),
                kind: WorkspaceChangeKind::RescanRequired,
            }]);
        }
        if matches!(event.kind, EventKind::Modify(ModifyKind::Name(_))) && event.paths.len() == 2 {
            let from = self.relative_event_path(&event.paths[0])?;
            let path = self.relative_event_path(&event.paths[1])?;
            return Ok(vec![WorkspaceChange {
                path,
                kind: WorkspaceChangeKind::Renamed { from },
            }]);
        }

        let kind = match event.kind {
            EventKind::Create(_) => WorkspaceChangeKind::Created,
            EventKind::Modify(_) | EventKind::Access(_) => WorkspaceChangeKind::Modified,
            EventKind::Remove(_) => WorkspaceChangeKind::Removed,
            EventKind::Other | EventKind::Any => WorkspaceChangeKind::RescanRequired,
        };
        event
            .paths
            .iter()
            .map(|path| {
                Ok(WorkspaceChange {
                    path: self.relative_event_path(path)?,
                    kind: kind.clone(),
                })
            })
            .collect()
    }

    /// Preserves removed paths by relativizing their event spelling without requiring existence.
    fn relative_event_path(&self, path: &Path) -> Result<String, WorkspaceFileSystemError> {
        relative_string(&self.root, path)
    }
}

#[cfg(test)]
mod tests;
