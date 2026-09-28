//! Owns `plugins/cache/`, which holds downloaded release archives and nothing durable.
//!
//! A `.orax` file below the cache is a transfer artifact: it exists only between a verified
//! download and the commit or failure of the install that requested it. The one durable resident
//! of this directory is the derived marketplace index (`registry_index.json`), which the registry
//! crate rewrites atomically and every marketplace listing read depends on. Every rule here
//! therefore matches release archives by extension and never deletes anything else.
//!
//! Each archive is addressed as `<cache>/<namespace>/<name>-<version>.orax`. The namespace is a
//! directory level, not a filename prefix, because both a namespace and a plugin name may contain
//! hyphens: `foo` + `bar-baz` and `foo-bar` + `baz` would otherwise share one cache path and two
//! concurrent installs from different sources could unpack each other's bytes.

use super::InstallError;
use ora_domain::PluginNamespace;
use ora_logging::ora_warn;
use std::path::{Path, PathBuf};

/// Sub-directory of a plugin data directory that holds downloaded release archives.
const CACHE_ROOT: &str = "cache";
/// Extension of a downloaded release archive, which is the only file kind the cache owns.
const RELEASE_EXTENSION: &str = "orax";

/// Returns `<data-dir>/plugins/cache`, the directory every transfer artifact lands in.
fn cache_root(data_dir: &Path) -> PathBuf {
    data_dir.join("plugins").join(CACHE_ROOT)
}

/// Returns whether `entry` is a downloaded release archive this module may delete.
///
/// The check is the extension plus a regular-file check, so an in-progress download staged as
/// `<destination>.tmp`, the durable `registry_index.json`, and a directory that merely happens to
/// end in `.orax` are all outside the cache's cleanup authority by construction.
fn is_release_archive(entry: &std::fs::DirEntry) -> bool {
    entry
        .path()
        .extension()
        .is_some_and(|extension| extension == RELEASE_EXTENSION)
        && entry.file_type().is_ok_and(|file_type| file_type.is_file())
}

/// Builds the cache path one release archive is downloaded into.
///
/// The namespace comes from the installing source rather than the package, so it is always a
/// validated single path segment and cannot escape the cache directory.
pub(super) fn release_archive_path(
    data_dir: &Path,
    namespace: &PluginNamespace,
    name: &str,
    version: &str,
) -> PathBuf {
    cache_root(data_dir)
        .join(namespace.as_str())
        .join(format!("{name}-{version}.{RELEASE_EXTENSION}"))
}

/// Deletes every leftover release archive before a new transfer starts.
///
/// The sweep is the crash-recovery half of the cache contract: a process that dies between its
/// download and its cleanup leaves an archive nothing will ever read, so each install starts from
/// a cache that holds only the derived registry index. Both the namespace directories written by
/// the current layout and archives written directly into the cache root by the pre-namespace
/// layout are covered, so an upgraded installation cleans up its old residue instead of keeping it
/// forever. An archive that cannot be removed is reported and skipped: it is stale by definition,
/// and refusing an install over it would trade a hygiene failure for a usability failure.
pub(super) fn sweep_leftovers(data_dir: &Path) -> Result<(), InstallError> {
    let cache_dir = cache_root(data_dir);
    let entries = match std::fs::read_dir(&cache_dir) {
        Ok(entries) => entries,
        // No cache directory means this profile has never downloaded a release.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(InstallError::Io {
                path: cache_dir,
                source,
            });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|source| InstallError::Io {
            path: cache_dir.clone(),
            source,
        })?;
        let path = entry.path();
        if is_release_archive(&entry) {
            remove_archive_file(&path);
            continue;
        }
        if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
            sweep_namespace_directory(&path);
        }
    }
    Ok(())
}

/// Deletes every release archive below one namespace directory, then the directory itself when it
/// is left empty, so a finished install leaves the cache holding only the derived index.
fn sweep_namespace_directory(namespace_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(namespace_dir) else {
        return;
    };
    for entry in entries.flatten() {
        if is_release_archive(&entry) {
            remove_archive_file(&entry.path());
        }
    }
    remove_empty_directory(namespace_dir);
}

/// Deletes one downloaded archive together with the cache namespace directory it leaves empty.
///
/// Both halves belong to the same promise — the cache holds only the derived marketplace index
/// once an install ends — and keeping them in one call site means no exit path can do one without
/// the other.
pub(super) fn discard_downloaded_archive(archive_path: &Path) {
    remove_archive_file(archive_path);
    if let Some(namespace_dir) = archive_path.parent() {
        remove_empty_directory(namespace_dir);
    }
}

/// Removes one plugin directory an install emptied, leaving it in place when anything remains.
///
/// Callers pass either a cache namespace directory produced by [`release_archive_path`] or the
/// installed `<namespace>/<name>` parent of a failed install, so the directory removed is never
/// the cache root that holds the durable marketplace index. A directory that still holds anything
/// is the normal case — another archive, installed versions, or a concurrent install
/// materializing one — so `remove_dir`'s refusal is the expected outcome and only an unexpected
/// failure is worth reporting.
pub(super) fn remove_empty_directory(directory: &Path) {
    match std::fs::remove_dir(directory) {
        Ok(()) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            ) => {}
        Err(error) => ora_warn!(
            path = %directory.display(),
            %error,
            "failed to remove an empty plugin directory"
        ),
    }
}

/// Removes one downloaded archive file, reporting a failure without failing the install.
///
/// Unlike [`discard_downloaded_archive`], this leaves the containing directory alone. The bytes
/// have already served their purpose whether the install committed or failed, so an unremovable
/// file is a hygiene problem to surface in logs rather than a reason to tell a user whose package
/// is installed and working that the install failed.
fn remove_archive_file(archive_path: &Path) {
    match std::fs::remove_file(archive_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => ora_warn!(
            path = %archive_path.display(),
            %error,
            "failed to remove a downloaded plugin release archive"
        ),
    }
}
