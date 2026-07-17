//! ACP v1 message payload contracts, excluding the JSON-RPC transport envelope.

mod authentication;
mod common;
mod initialization;
mod mcp;
mod session;

pub use authentication::{
    AuthMethod, AuthMethodType, AuthenticateRequest, AuthenticateResponse, LogoutRequest,
    LogoutResponse,
};
pub use common::{
    AuthMethodId, Cursor, EmptyObject, ImplementationInfo, MessageId, Meta, ProtocolVersion,
    SessionId,
};
pub use initialization::{
    AgentCapabilities, AuthenticationCapabilities, ClientCapabilities, FileSystemCapabilities,
    InitializeRequest, InitializeResponse, McpCapabilities, PromptCapabilities,
    SessionCapabilities,
};
pub use mcp::{
    EnvironmentVariable, HttpHeader, HttpMcpServer, McpServer, McpTransport, SseMcpServer,
    StdioMcpServer,
};
pub use session::{
    CancelSessionNotification, CloseSessionRequest, CloseSessionResponse, DeleteSessionRequest,
    DeleteSessionResponse, ListSessionsRequest, ListSessionsResponse, LoadSessionRequest,
    LoadSessionResponse, NewSessionRequest, NewSessionResponse, PatchField, ResumeSessionRequest,
    ResumeSessionResponse, SessionEnvironment, SessionInfo, SessionInfoUpdate, SessionUpdate,
    SessionUpdateNotification, SessionUpdateType,
};
