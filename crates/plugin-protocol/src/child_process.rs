use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// Host method that spawns a process owned by one plugin generation.
pub const CHILDPROCESS_SPAWN_METHOD: &str = "ora/childprocess/spawn";
/// Host method that writes bytes to a spawned process.
pub const CHILDPROCESS_WRITE_METHOD: &str = "ora/childprocess/write";
/// Host method that closes a spawned process's stdin.
pub const CHILDPROCESS_CLOSE_STDIN_METHOD: &str = "ora/childprocess/close_stdin";
/// Host method that terminates a spawned process tree.
pub const CHILDPROCESS_KILL_METHOD: &str = "ora/childprocess/kill";
/// Host notification carrying a spawned process's stdout bytes.
pub const CHILDPROCESS_STDOUT_METHOD: &str = "ora/childprocess/stdout";
/// Host notification carrying a spawned process's stderr bytes.
pub const CHILDPROCESS_STDERR_METHOD: &str = "ora/childprocess/stderr";
/// Host notification reporting a spawned process's exit status.
pub const CHILDPROCESS_EXIT_METHOD: &str = "ora/childprocess/exit";

/// Wire payload for spawning a host-managed process.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "child_process.ts")]
pub struct ChildProcessSpawnParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub package_command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

/// Result identifying one spawned process inside its plugin generation.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "child_process.ts")]
pub struct ChildProcessSpawnResult {
    pub process_id: String,
    pub pid: Option<u32>,
}

/// Parameters shared by operations that address one spawned process.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "child_process.ts")]
pub struct ChildProcessIdParams {
    pub process_id: String,
}

/// Parameters for writing one base64-encoded byte chunk to process stdin.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "child_process.ts")]
pub struct ChildProcessWriteParams {
    pub process_id: String,
    pub bytes_base64: String,
}

/// Notification carrying one base64-encoded stdout or stderr chunk.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "child_process.ts")]
pub struct ChildProcessOutput {
    pub process_id: String,
    pub bytes_base64: String,
}

/// Notification describing how one host-managed process ended.
#[derive(Debug, Clone, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "child_process.ts")]
pub struct ChildProcessExit {
    pub process_id: String,
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

/// Stable child-process failure classification carried in JSON-RPC error data.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "child_process.ts")]
pub enum ChildProcessErrorKind {
    InvalidParams,
    InvalidCommand,
    PackageCommandMissing,
    InvalidPackageCommand,
    NotFound,
    ProgramNotFound,
    Io,
}

impl ChildProcessErrorKind {
    /// Returns the stable snake_case spelling placed in JSON-RPC error data.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidParams => "invalid_params",
            Self::InvalidCommand => "invalid_command",
            Self::PackageCommandMissing => "package_command_missing",
            Self::InvalidPackageCommand => "invalid_package_command",
            Self::NotFound => "not_found",
            Self::ProgramNotFound => "program_not_found",
            Self::Io => "io",
        }
    }

    /// Returns the JSON-RPC error code paired with this stable classification.
    pub const fn code(self) -> i64 {
        match self {
            Self::InvalidParams
            | Self::InvalidCommand
            | Self::PackageCommandMissing
            | Self::InvalidPackageCommand => super::INVALID_PARAMS_CODE,
            Self::NotFound => -32004,
            Self::ProgramNotFound | Self::Io => -32000,
        }
    }
}

/// Exports every host-managed process DTO into one TypeScript module.
pub(crate) fn export(config: &ts_rs::Config) -> Result<(), ts_rs::ExportError> {
    ChildProcessSpawnParams::export(config)?;
    ChildProcessSpawnResult::export(config)?;
    ChildProcessIdParams::export(config)?;
    ChildProcessWriteParams::export(config)?;
    ChildProcessOutput::export(config)?;
    ChildProcessExit::export(config)?;
    ChildProcessErrorKind::export(config)?;
    Ok(())
}
