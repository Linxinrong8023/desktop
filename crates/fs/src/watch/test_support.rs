//! Native callback gates for testing filesystem cleanup through host adapters.
use crate::{WorkspaceFileSystemError, WorkspaceWatcher};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// Blocks the first native callback until released, including when the watcher is dropped.
pub struct CallbackGate {
    entered: Receiver<()>,
    release: Option<Sender<()>>,
}

impl CallbackGate {
    /// Verifies that the native callback, rather than only the consumer, is blocked.
    pub fn wait_until_entered(&self) -> Result<(), mpsc::RecvTimeoutError> {
        self.entered.recv_timeout(Duration::from_secs(/*secs*/ 10))
    }

    /// Lets the native worker finish the callback and perform its shutdown sequence.
    pub fn release(mut self) {
        self.release.take();
    }
}

/// Starts a real watcher whose first callback is held by the returned gate.
/// Dropping the gate releases the callback even if a test assertion unwinds.
pub fn blocked_watcher(
    root: &Path,
) -> Result<(WorkspaceWatcher, CallbackGate), WorkspaceFileSystemError> {
    let (entered, entering) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let mut first = Some(released);
    let watcher = WorkspaceWatcher::start_with_event_hook(root, move || {
        if let Some(released) = first.take() {
            let _ = entered.send(());
            let _ = released.recv();
        }
    })?;
    Ok((
        watcher,
        CallbackGate {
            entered: entering,
            release: Some(release),
        },
    ))
}
