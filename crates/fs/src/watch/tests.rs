use super::WorkspaceWatcher;
use pretty_assertions::assert_eq;
use std::collections::BTreeSet;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

/// Verifies native callbacks are normalized to workspace-relative paths.
#[test]
fn observes_workspace_file_changes() {
    let workspace = TempDir::new().unwrap_or_else(|error| panic!("create temp workspace: {error}"));
    let watcher = WorkspaceWatcher::start(workspace.path())
        .unwrap_or_else(|error| panic!("start workspace watcher: {error}"));

    fs::write(workspace.path().join("watched.txt"), "changed")
        .unwrap_or_else(|error| panic!("write watched fixture: {error}"));
    let changes = watcher
        .receive_batch(Duration::from_secs(2))
        .unwrap_or_else(|error| panic!("receive native file event: {error}"))
        .unwrap_or_else(|| panic!("expected a native file event"));

    assert_eq!(
        changes
            .iter()
            .map(|change| change.path.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["watched.txt"])
    );
}

/// Runs alone so exact descriptor accounting cannot race another test's native watcher.
#[cfg(target_os = "linux")]
#[test]
fn native_close_waits_for_callback_and_releases_inotify() {
    const CHILD: &str = "ORA_NATIVE_RELEASE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "watch::tests::native_close_waits_for_callback_and_releases_inotify",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let initial = inotify_descriptors();
            let workspace = TempDir::new().unwrap();
            let (watcher, gate) =
                crate::watch_test_support::blocked_watcher(workspace.path()).unwrap();
            fs::write(workspace.path().join("trigger"), "event").unwrap();
            gate.wait_until_entered().unwrap();
            let watching = inotify_descriptors();
            assert_eq!(watching.len(), initial.len() + 1);
            let closing = watcher.close();
            tokio::pin!(closing);
            assert!(
                tokio::time::timeout(Duration::from_millis(/*millis*/ 100), &mut closing)
                    .await
                    .is_err()
            );
            assert_eq!(inotify_descriptors(), watching);
            gate.release();
            tokio::time::timeout(Duration::from_secs(/*secs*/ 10), &mut closing)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(inotify_descriptors(), initial);
        });
}

/// Reads kernel-owned descriptors rather than inferring release from a callback or public Drop.
#[cfg(target_os = "linux")]
fn inotify_descriptors() -> BTreeSet<std::path::PathBuf> {
    fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            fs::read_link(path)
                .is_ok_and(|target| target == std::path::Path::new("anon_inode:inotify"))
        })
        .collect()
}

/// A lost receipt is a failure even when the public watcher Drop itself returns normally.
#[tokio::test]
async fn native_close_rejects_a_missing_receipt() {
    let workspace = TempDir::new().unwrap();
    let mut watcher = WorkspaceWatcher::start(workspace.path()).unwrap();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    watcher.retired = receiver;
    drop(sender);
    let error = watcher.close().await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("native release was not confirmed")
    );
}

/// A callback panic cannot be reclassified as a successful shutdown by its destructor.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn native_close_rejects_a_panicked_worker() {
    let workspace = TempDir::new().unwrap();
    let (entered, entering) = std::sync::mpsc::channel();
    let watcher = WorkspaceWatcher::start_with_event_hook(workspace.path(), move || {
        entered.send(()).unwrap();
        panic!("fixture native callback panic");
    })
    .unwrap();
    fs::write(workspace.path().join("trigger"), "event").unwrap();
    entering
        .recv_timeout(Duration::from_secs(/*secs*/ 10))
        .unwrap();
    let error = watcher.close().await.unwrap_err();
    assert!(error.to_string().contains("panicked"));
}
