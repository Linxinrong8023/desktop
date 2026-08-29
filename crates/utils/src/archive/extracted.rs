use super::error::ArchiveError;
use crate::path::StrictRelativePath;
use std::fs;
use std::path::{Path, PathBuf};

/// Whether a materialized file carries the execute bit.
///
/// Extraction records executability rather than the source mode: only the three execute bits of
/// an entry are read, and honoring them writes a fixed `0o755`. An archive can therefore say "this
/// file is runnable" without choosing the identity it runs under, so setuid, setgid, and sticky
/// bits can never survive into an extracted tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExecutability {
    /// The source marks the entry executable and the materialized file keeps that bit.
    Executable,
    /// The entry is ordinary data.
    NotExecutable,
}

impl FileExecutability {
    /// Classifies one Unix mode by its execute bits, ignoring every other permission and flag.
    pub(super) fn from_unix_mode(mode: u32) -> Self {
        if mode & 0o111 == 0 {
            Self::NotExecutable
        } else {
            Self::Executable
        }
    }
}

/// One ordinary file materialized inside a validated tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFile {
    /// Validated relative path of the file below the tree root.
    pub relative_path: StrictRelativePath,
    /// Number of ordinary bytes stored for this file.
    pub size: u64,
    /// Whether the materialized file was made executable.
    pub executability: FileExecutability,
}

/// A validated, materialized tree produced by extraction or folder copy.
#[derive(Debug)]
pub struct ExtractedTree {
    root: PathBuf,
    files: Vec<ExtractedFile>,
}

impl ExtractedTree {
    /// Assembles the tree listing, sorting files deterministically by relative path.
    pub(super) fn new(root: PathBuf, mut files: Vec<ExtractedFile>) -> Self {
        files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Self { root, files }
    }

    /// Returns the absolute destination root that materialized this tree.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns every ordinary file sorted by validated relative path.
    pub fn files(&self) -> &[ExtractedFile] {
        &self.files
    }

    /// Reads one file's bytes from the tree, resolving it under the root.
    pub fn read_file(&self, relative_path: &StrictRelativePath) -> Result<Vec<u8>, ArchiveError> {
        let path = relative_path.to_path(&self.root);
        fs::read(&path).map_err(|error| ArchiveError::Io {
            message: format!("failed to read {relative_path}: {error}"),
        })
    }

    /// Returns the file matching an exact relative path, if present.
    pub fn find_file(&self, relative_path: &StrictRelativePath) -> Option<&ExtractedFile> {
        self.files
            .iter()
            .find(|file| &file.relative_path == relative_path)
    }
}
