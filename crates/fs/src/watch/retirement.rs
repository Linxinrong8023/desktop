//! Native release receipt for the exactly pinned notify 8.2.0 implementation.
//!
//! inotify/kqueue release the kernel watcher before dropping EventLoop.event_handler.
//! Windows stops and closes directory handles, drains their APCs and closes its wake semaphore
//! before ReadDirectoryChangesServer drops its last handler Arc. FSEvents joins its runloop and
//! releases its stream/path storage before dropping the handler. PollWatcher drops its handler
//! after its polling worker releases the shared builder. Reaudit these boundaries on upgrades;
//! a callback return, unwatch response, or drop of the public watcher is not this receipt.
use notify::{Event, EventHandler};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::oneshot;

/// Carries completion in the native worker's owned handler, after its kernel resources retire.
pub(super) struct NativeEvents<F> {
    callback: Option<F>,
    retirement: Option<oneshot::Sender<Result<(), &'static str>>>,
    shutdown_requested: Arc<AtomicBool>,
}

impl<F> NativeEvents<F> {
    pub(super) fn new(
        callback: F,
        retirement: oneshot::Sender<Result<(), &'static str>>,
        shutdown_requested: Arc<AtomicBool>,
    ) -> Self {
        Self {
            callback: Some(callback),
            retirement: Some(retirement),
            shutdown_requested,
        }
    }
}

impl<F: FnMut(notify::Result<Event>) + Send + 'static> EventHandler for NativeEvents<F> {
    /// Preserves notify's ordered callbacks while retaining their release receipt in the owner.
    fn handle_event(&mut self, event: notify::Result<Event>) {
        if let Some(callback) = &mut self.callback {
            callback(event);
        }
    }
}

impl<F> Drop for NativeEvents<F> {
    /// Confirms normal native retirement only after callback captures have also been released.
    fn drop(&mut self) {
        drop(self.callback.take());
        let result = if std::thread::panicking() {
            Err("native watcher panicked before confirming release")
        } else if !self.shutdown_requested.load(Ordering::SeqCst) {
            Err("native watcher exited without a shutdown request")
        } else {
            Ok(())
        };
        if let Some(retirement) = self.retirement.take() {
            let _ = retirement.send(result);
        }
    }
}
