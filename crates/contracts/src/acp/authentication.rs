use super::{AuthMethodId, Meta};
use serde::{Deserialize, Serialize};

/// Describes the selectable authentication methods returned during initialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthMethod {
    pub id: AuthMethodId,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub method_type: Option<AuthMethodType>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
}

/// Identifies the authentication flow represented by an advertised method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethodType {
    /// Identifies the standard agent-owned authentication flow.
    Agent,
}

/// Selects one authentication method that an agent advertised during initialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateRequest {
    pub method_id: AuthMethodId,
}

/// Represents the empty successful result of an authentication operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticateResponse {}

/// Represents the empty payload accepted by a logout operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutRequest {}

/// Represents the empty successful result of a logout operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutResponse {}
