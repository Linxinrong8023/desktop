use std::path::{Path, PathBuf};

/// Canonicalizes a path in the spelling external tools accept.
///
/// `std::fs::canonicalize` on Windows returns verbatim paths (`\\?\C:\...`). Rust's own filesystem
/// APIs accept those, but programs the result is handed to, Git in particular, do not: Git treats
/// `\\?\C:\dir` as a different (nonexistent) working tree from `C:\dir`. Callers that pass a
/// canonical path to another process, or compare one against a path another process printed, must
/// use this instead of `std::fs::canonicalize`. Compare results only against other results of the
/// same function; mixing them with `std::fs::canonicalize` output silently fails on Windows.
pub fn canonicalize(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path).map(|canonical| strip_verbatim_prefix(&canonical))
}

/// Rewrites a Windows verbatim path (`\\?\C:\dir`, `\\?\UNC\host\share\dir`) to its plain form.
///
/// Only the disk and UNC verbatim forms have a plain equivalent; other prefixes are returned
/// unchanged. Verbatim paths bypass the legacy `MAX_PATH` limit, which the plain form does not;
/// Rust re-adds the prefix internally when it opens a plain path, and Git honors `core.longpaths`,
/// so interoperability with other processes is the priority here. On non-Windows platforms the
/// path is returned unchanged.
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::path::{Component, Prefix};

        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return path.to_path_buf();
        };
        let mut plain = match prefix.kind() {
            Prefix::VerbatimDisk(letter) => PathBuf::from(format!("{}:", char::from(letter))),
            Prefix::VerbatimUNC(server, share) => {
                // A UNC root has no component-wise construction: `\\` followed by the host is one
                // prefix component, so it is assembled as a string.
                let mut root = OsString::from(r"\\");
                root.push(server);
                root.push(std::path::MAIN_SEPARATOR_STR);
                root.push(share);
                PathBuf::from(root)
            }
            Prefix::Verbatim(_) | Prefix::DeviceNS(_) | Prefix::UNC(..) | Prefix::Disk(_) => {
                return path.to_path_buf();
            }
        };
        // The remaining components (root separator and normal segments) are prefix-independent.
        for component in components {
            plain.push(component.as_os_str());
        }
        plain
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::{canonicalize, strip_verbatim_prefix};
    use pretty_assertions::assert_eq;
    use std::path::{Component, Path};
    use tempfile::TempDir;

    #[test]
    fn canonical_paths_carry_no_verbatim_prefix() {
        let root = TempDir::new().unwrap_or_else(|error| panic!("create root: {error}"));
        let canonical =
            canonicalize(root.path()).unwrap_or_else(|error| panic!("canonicalize root: {error}"));

        let verbatim = matches!(
            canonical.components().next(),
            Some(Component::Prefix(prefix)) if prefix.kind().is_verbatim()
        );
        assert_eq!((verbatim, canonical.is_absolute()), (false, true));
        // Stripping is idempotent and never changes an already plain path.
        assert_eq!(strip_verbatim_prefix(&canonical), canonical);
    }

    #[cfg(windows)]
    #[test]
    fn strips_disk_and_unc_verbatim_prefixes() {
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\C:\Users\node\worktrees")),
            Path::new(r"C:\Users\node\worktrees")
        );
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\UNC\server\share\dir")),
            Path::new(r"\\server\share\dir")
        );
        // Device namespace paths have no plain spelling and stay untouched.
        assert_eq!(
            strip_verbatim_prefix(Path::new(r"\\?\pipe\name")),
            Path::new(r"\\?\pipe\name")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn leaves_unix_paths_unchanged() {
        assert_eq!(
            strip_verbatim_prefix(Path::new("/tmp/dir")),
            Path::new("/tmp/dir")
        );
    }
}
