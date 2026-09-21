#![cfg(all(unix, feature = "local-ipc"))]
use ora_utils::local_ipc::bind_private_endpoint;
use pretty_assertions::assert_eq;
use std::{fs, io, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

/// Endpoint recovery refuses foreign inode types and live listeners, replacing only refused owned sockets.
#[tokio::test]
async fn private_endpoint_preserves_files_links_and_live_sockets() -> io::Result<()> {
    let parent = PathBuf::from(
        std::env::var_os("HOME")
            .ok_or_else(|| io::Error::other("HOME required for private test deployment"))?,
    )
    .canonicalize()?;
    let directory = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(/*mode*/ 0o700))
        .tempdir_in(parent)?;
    let path = directory.path().join("control.sock");
    // SAFETY: reads the current identity without changing credentials.
    let uid = unsafe { libc::geteuid() };
    let deadline = Duration::from_millis(/*millis*/ 100);
    fs::write(&path, "user file")?;
    assert!(bind_private_endpoint(&path, uid, deadline).await.is_err());
    assert_eq!(fs::read_to_string(&path)?, "user file");
    let link = directory.path().join("link.sock");
    std::os::unix::fs::symlink(&path, &link)?;
    assert!(bind_private_endpoint(&link, uid, deadline).await.is_err());
    let socket = directory.path().join("owned.sock");
    let first = bind_private_endpoint(&socket, uid, deadline).await?;
    assert!(bind_private_endpoint(&socket, uid, deadline).await.is_err());
    drop(first);
    let _replacement = bind_private_endpoint(&socket, uid, deadline).await?;
    assert_eq!(fs::metadata(socket)?.permissions().mode() & 0o777, 0o600);
    Ok(())
}
