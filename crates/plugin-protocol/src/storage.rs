use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Host method that lists entries below a logical plugin data path.
pub const STORAGE_LIST_METHOD: &str = "ora/storage/list";
/// Host method that reads one plugin data file as base64.
pub const STORAGE_READ_METHOD: &str = "ora/storage/read";
/// Host method that atomically replaces one plugin data file.
pub const STORAGE_WRITE_METHOD: &str = "ora/storage/write";
/// Host method that removes one plugin data file or directory tree.
pub const STORAGE_REMOVE_METHOD: &str = "ora/storage/remove";
/// Largest raw file accepted by storage read and write operations.
pub const MAX_STORAGE_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// Parameters for a storage operation addressed by logical path.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[ts(export_to = "storage.ts")]
pub struct StoragePathParams {
    pub path: String,
}

/// Parameters for replacing one storage file from base64-encoded bytes.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[ts(export_to = "storage.ts")]
pub struct StorageWriteParams {
    pub path: String,
    pub bytes_base64: String,
}

/// Result of reading one storage file.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[ts(export_to = "storage.ts")]
pub struct StorageReadResult {
    pub bytes_base64: String,
}

/// Result of listing one storage directory.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[ts(export_to = "storage.ts")]
pub struct StorageListResult {
    pub entries: Vec<StorageListEntry>,
}

/// One regular file or directory returned by storage listing.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[ts(export_to = "storage.ts")]
pub struct StorageListEntry {
    pub name: String,
    pub kind: StorageEntryKind,
    #[ts(type = "number")]
    pub size_bytes: u64,
}

/// Closed set of entry kinds visible through plugin storage.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "storage.ts")]
pub enum StorageEntryKind {
    File,
    Directory,
}

/// Stable storage failure classification carried in JSON-RPC error data.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "storage.ts")]
pub enum StorageErrorKind {
    InvalidParams,
    InvalidPath,
    NotFound,
    TooLarge,
    Io,
}

impl StorageErrorKind {
    /// Returns the stable snake_case spelling placed in JSON-RPC error data.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidParams => "invalid_params",
            Self::InvalidPath => "invalid_path",
            Self::NotFound => "not_found",
            Self::TooLarge => "too_large",
            Self::Io => "io",
        }
    }

    /// Returns the JSON-RPC error code paired with this stable classification.
    pub const fn code(self) -> i64 {
        match self {
            Self::InvalidParams | Self::InvalidPath => super::INVALID_PARAMS_CODE,
            Self::NotFound => -32004,
            Self::TooLarge => -32005,
            Self::Io => -32000,
        }
    }
}

/// Exports every plugin storage DTO into one TypeScript module.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    StoragePathParams::export(config)?;
    StorageWriteParams::export(config)?;
    StorageReadResult::export(config)?;
    StorageListResult::export(config)?;
    StorageListEntry::export(config)?;
    StorageEntryKind::export(config)?;
    StorageErrorKind::export(config)?;
    Ok(())
}
