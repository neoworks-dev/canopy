use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Bumped only for breaking wire changes. Clients and daemons exchange it during
/// the handshake and refuse to talk across a mismatch.
pub const PROTOCOL_VERSION: i64 = 1;

/// A single client call. Ids are client-assigned and only need to be unique for
/// the lifetime of one connection.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Carries either `result` or `error`, never both.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Response {
    #[serde(default)]
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,
}

/// A machine-readable failure. `code` is stable; `message` is for logs and
/// developer-facing surfaces, not for end users.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
}

impl ProtocolError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        ProtocolError { code: code.to_string(), message: message.into() }
    }
}

pub const CODE_UNKNOWN_METHOD: &str = "unknownMethod";
pub const CODE_INVALID_PARAMS: &str = "invalidParams";
pub const CODE_PROTOCOL_MISMATCH: &str = "protocolMismatch";
pub const CODE_NOT_FOUND: &str = "notFound";
pub const CODE_INTERNAL: &str = "internal";

pub const METHOD_HANDSHAKE: &str = "handshake";
pub const METHOD_ADD_ROOT: &str = "index.addRoot";
pub const METHOD_REMOVE_ROOT: &str = "index.removeRoot";
pub const METHOD_ROOTS: &str = "index.roots";
pub const METHOD_STATUS: &str = "index.status";
pub const METHOD_RESCAN: &str = "index.rescan";
pub const METHOD_SEARCH_QUERY: &str = "search.query";
pub const METHOD_SHUTDOWN: &str = "daemon.shutdown";
