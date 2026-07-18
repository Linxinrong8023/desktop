use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Carries one environment variable applied to a terminal command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct EnvVariable {
    pub name: String,
    pub value: String,
}

/// Describes how a terminal command finished once it is no longer running.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct TerminalExitStatus {
    /// Reports the process exit code, absent when a signal terminated the process.
    pub exit_code: Option<u32>,
    /// Names the signal that terminated the process, absent on a normal exit.
    pub signal: Option<String>,
}

/// Requests a new terminal running one command for the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct CreateTerminalRequest {
    pub session_id: String,
    pub command: String,
    /// Passes command arguments verbatim without shell interpretation.
    pub args: Vec<String>,
    /// Adds environment variables on top of the client environment.
    pub env: Vec<EnvVariable>,
    /// Runs the command in this absolute directory instead of the client default.
    pub cwd: Option<String>,
    /// Retains at most this many output bytes, truncating older output first.
    pub output_byte_limit: Option<u64>,
}

/// Returns the identifier of the terminal that was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct CreateTerminalResponse {
    pub terminal_id: String,
}

/// Requests the output captured so far without waiting for the command to finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct TerminalOutputRequest {
    pub session_id: String,
    pub terminal_id: String,
}

/// Returns the captured terminal output and the command status at capture time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct TerminalOutputResponse {
    pub output: String,
    /// Reports whether output was dropped to stay within the byte limit.
    pub truncated: bool,
    /// Describes the exit, absent while the command is still running.
    pub exit_status: Option<TerminalExitStatus>,
}

/// Requests a wait until the terminal command finishes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct WaitForTerminalExitRequest {
    pub session_id: String,
    pub terminal_id: String,
}

/// Returns how the terminal command finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct WaitForTerminalExitResponse {
    /// Reports the process exit code, absent when a signal terminated the process.
    pub exit_code: Option<u32>,
    /// Names the signal that terminated the process, absent on a normal exit.
    pub signal: Option<String>,
}

/// Requests termination of the running command while keeping the terminal readable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct KillTerminalRequest {
    pub session_id: String,
    pub terminal_id: String,
}

/// Acknowledges a completed kill, which carries no result fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct KillTerminalResponse {}

/// Requests release of the terminal and every resource it still holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct ReleaseTerminalRequest {
    pub session_id: String,
    pub terminal_id: String,
}

/// Acknowledges a completed release, which carries no result fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/terminal.ts")]
pub struct ReleaseTerminalResponse {}

#[cfg(test)]
mod tests {
    use super::{
        CreateTerminalRequest, CreateTerminalResponse, EnvVariable, KillTerminalRequest,
        KillTerminalResponse, ReleaseTerminalRequest, ReleaseTerminalResponse, TerminalExitStatus,
        TerminalOutputRequest, TerminalOutputResponse, WaitForTerminalExitRequest,
        WaitForTerminalExitResponse,
    };
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies terminal creation carries argument, environment, and limit options separately.
    #[test]
    fn serializes_create_terminal_contracts() {
        assert_serialized_json(
            &CreateTerminalRequest {
                session_id: "sess-1".to_string(),
                command: "cargo".to_string(),
                args: vec!["test".to_string()],
                env: vec![EnvVariable {
                    name: "RUST_LOG".to_string(),
                    value: "debug".to_string(),
                }],
                cwd: Some("/home/user/project".to_string()),
                output_byte_limit: Some(1_048_576),
            },
            json!({
                "sessionId": "sess-1",
                "command": "cargo",
                "args": ["test"],
                "env": [{ "name": "RUST_LOG", "value": "debug" }],
                "cwd": "/home/user/project",
                "outputByteLimit": 1_048_576,
            }),
        );
        assert_serialized_json(
            &CreateTerminalRequest {
                session_id: "sess-1".to_string(),
                command: "ls".to_string(),
                args: Vec::new(),
                env: Vec::new(),
                cwd: None,
                output_byte_limit: None,
            },
            json!({
                "sessionId": "sess-1",
                "command": "ls",
                "args": [],
                "env": [],
                "cwd": null,
                "outputByteLimit": null,
            }),
        );
        assert_serialized_json(
            &CreateTerminalResponse {
                terminal_id: "term-1".to_string(),
            },
            json!({ "terminalId": "term-1" }),
        );
    }

    /// Verifies output responses omit an exit status while the command is still running.
    #[test]
    fn serializes_terminal_output_contracts() {
        assert_serialized_json(
            &TerminalOutputRequest {
                session_id: "sess-1".to_string(),
                terminal_id: "term-1".to_string(),
            },
            json!({ "sessionId": "sess-1", "terminalId": "term-1" }),
        );
        assert_serialized_json(
            &TerminalOutputResponse {
                output: "running tests\n".to_string(),
                truncated: false,
                exit_status: None,
            },
            json!({ "output": "running tests\n", "truncated": false, "exitStatus": null }),
        );
        assert_serialized_json(
            &TerminalOutputResponse {
                output: "ok\n".to_string(),
                truncated: true,
                exit_status: Some(TerminalExitStatus {
                    exit_code: Some(0),
                    signal: None,
                }),
            },
            json!({
                "output": "ok\n",
                "truncated": true,
                "exitStatus": { "exitCode": 0, "signal": null },
            }),
        );
    }

    /// Verifies exit waiting reports either an exit code or a terminating signal.
    #[test]
    fn serializes_wait_for_terminal_exit_contracts() {
        assert_serialized_json(
            &WaitForTerminalExitRequest {
                session_id: "sess-1".to_string(),
                terminal_id: "term-1".to_string(),
            },
            json!({ "sessionId": "sess-1", "terminalId": "term-1" }),
        );
        assert_serialized_json(
            &WaitForTerminalExitResponse {
                exit_code: Some(1),
                signal: None,
            },
            json!({ "exitCode": 1, "signal": null }),
        );
        assert_serialized_json(
            &WaitForTerminalExitResponse {
                exit_code: None,
                signal: Some("SIGKILL".to_string()),
            },
            json!({ "exitCode": null, "signal": "SIGKILL" }),
        );
    }

    /// Verifies kill and release target one terminal and acknowledge without fields.
    #[test]
    fn serializes_terminal_lifecycle_contracts() {
        assert_serialized_json(
            &KillTerminalRequest {
                session_id: "sess-1".to_string(),
                terminal_id: "term-1".to_string(),
            },
            json!({ "sessionId": "sess-1", "terminalId": "term-1" }),
        );
        assert_serialized_json(&KillTerminalResponse {}, json!({}));
        assert_serialized_json(
            &ReleaseTerminalRequest {
                session_id: "sess-1".to_string(),
                terminal_id: "term-1".to_string(),
            },
            json!({ "sessionId": "sess-1", "terminalId": "term-1" }),
        );
        assert_serialized_json(&ReleaseTerminalResponse {}, json!({}));
    }

    fn assert_serialized_json(value: &impl serde::Serialize, expected: serde_json::Value) {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
}
