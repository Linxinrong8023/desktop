use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Carries the `fs/read_text_file` parameters an agent sends to the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/file.ts")]
pub struct ReadTextFileRequest {
    pub session_id: String,
    pub path: String,
    /// Starts the returned slice at this 1-based line instead of the first line.
    pub line: Option<u32>,
    /// Limits the returned slice to this many lines.
    pub limit: Option<u32>,
}

/// Returns the requested text file contents to the calling agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/file.ts")]
pub struct ReadTextFileResponse {
    pub content: String,
}

/// Carries the `fs/write_text_file` parameters an agent sends to the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/file.ts")]
pub struct WriteTextFileRequest {
    pub session_id: String,
    pub path: String,
    pub content: String,
}

/// Acknowledges a completed text file write, which carries no result fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "acp/file.ts")]
pub struct WriteTextFileResponse {}

#[cfg(test)]
mod tests {
    use super::{
        ReadTextFileRequest, ReadTextFileResponse, WriteTextFileRequest, WriteTextFileResponse,
    };
    use pretty_assertions::assert_eq;
    use serde_json::json;

    /// Verifies read requests keep optional line windowing separate from the required target.
    #[test]
    fn serializes_read_text_file_contracts() {
        assert_serialized_json(
            &ReadTextFileRequest {
                session_id: "sess-1".to_string(),
                path: "/home/user/project/src/main.rs".to_string(),
                line: None,
                limit: None,
            },
            json!({
                "sessionId": "sess-1",
                "path": "/home/user/project/src/main.rs",
                "line": null,
                "limit": null,
            }),
        );
        assert_serialized_json(
            &ReadTextFileRequest {
                session_id: "sess-1".to_string(),
                path: "/home/user/project/src/main.rs".to_string(),
                line: Some(10),
                limit: Some(50),
            },
            json!({
                "sessionId": "sess-1",
                "path": "/home/user/project/src/main.rs",
                "line": 10,
                "limit": 50,
            }),
        );
        assert_serialized_json(
            &ReadTextFileResponse {
                content: "fn main() {}\n".to_string(),
            },
            json!({ "content": "fn main() {}\n" }),
        );
    }

    /// Verifies write requests carry the full replacement content and acknowledge without fields.
    #[test]
    fn serializes_write_text_file_contracts() {
        assert_serialized_json(
            &WriteTextFileRequest {
                session_id: "sess-1".to_string(),
                path: "/home/user/project/src/main.rs".to_string(),
                content: "fn main() {}\n".to_string(),
            },
            json!({
                "sessionId": "sess-1",
                "path": "/home/user/project/src/main.rs",
                "content": "fn main() {}\n",
            }),
        );
        assert_serialized_json(&WriteTextFileResponse {}, json!({}));
    }

    fn assert_serialized_json(value: &impl serde::Serialize, expected: serde_json::Value) {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
}
