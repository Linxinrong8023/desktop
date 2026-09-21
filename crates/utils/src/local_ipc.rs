//! Private Unix endpoint creation; callers must hold their stable exclusive deployment lease.
use std::{
    fs, io,
    os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
    path::Path,
    time::Duration,
};
use tokio::net::{UnixListener, UnixStream};

/// Replaces only a same-owner refused socket under a private parent; never erases arbitrary files.
/// The caller's exclusive lease must cover this endpoint until the returned listener is dropped.
pub async fn bind_private_endpoint(
    path: &Path,
    owner: u32,
    probe_timeout: Duration,
) -> io::Result<UnixListener> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("endpoint requires a parent"))?;
    let directory =
        crate::path::open_private_path(parent, owner, crate::path::TrustedPathKind::Directory)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_socket()
                || metadata.uid() != owner
                || metadata.mode() & 0o077 != 0
            {
                return Err(io::Error::other("endpoint is not an owned private socket"));
            }
            match tokio::time::timeout(probe_timeout, UnixStream::connect(path)).await {
                Ok(Err(error)) if error.kind() == io::ErrorKind::ConnectionRefused => {}
                _ => return Err(io::Error::other("endpoint may still be active")),
            }
            let current = fs::symlink_metadata(path)?;
            if (current.dev(), current.ino()) != (metadata.dev(), metadata.ino()) {
                return Err(io::Error::other("endpoint replaced during probe"));
            }
            fs::remove_file(path)?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(/*mode*/ 0o600))?;
    directory.sync_all()?;
    Ok(listener)
}
