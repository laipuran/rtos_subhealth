//! API error model. Mirrors the error codes in RFC-005 so the external
//! contract is preserved exactly.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidJson,
    InvalidGoal,
    InvalidParam,
    InvalidState,
    NotFound,
    Conflict,
    Unauthorized,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidJson => "INVALID_JSON",
            Self::InvalidGoal => "INVALID_GOAL",
            Self::InvalidParam => "INVALID_PARAM",
            Self::InvalidState => "INVALID_STATE",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Internal => "INTERNAL",
        }
    }

    pub fn status(self) -> u16 {
        match self {
            Self::InvalidJson | Self::InvalidGoal | Self::InvalidParam | Self::InvalidState => 400,
            Self::Unauthorized => 401,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::Internal => 500,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn status(&self) -> u16 {
        self.code.status()
    }

    /// Serialize to the RFC-005 error envelope:
    /// `{"error": {"code", "message", "details"?}}`.
    pub fn to_body(&self) -> Value {
        let mut error = serde_json::Map::new();
        error.insert("code".into(), Value::String(self.code.as_str().into()));
        error.insert("message".into(), Value::String(self.message.clone()));
        if let Some(details) = &self.details {
            error.insert("details".into(), details.clone());
        }
        let mut body = serde_json::Map::new();
        body.insert("error".into(), Value::Object(error));
        Value::Object(body)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, GatewayError>;
