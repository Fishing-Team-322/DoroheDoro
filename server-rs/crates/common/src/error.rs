use thiserror::Error;

use crate::proto::{
    agent::AgentReplyEnvelope, control::ControlReplyEnvelope, deployment::DeploymentReplyEnvelope,
};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidArgument,
    Unauthenticated,
    NotFound,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "invalid_argument",
            Self::Unauthenticated => "unauthenticated",
            Self::NotFound => "not_found",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    Unauthenticated(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::Unauthenticated(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidArgument(_) => ErrorCode::InvalidArgument,
            Self::Unauthenticated(_) => ErrorCode::Unauthenticated,
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::InvalidArgument(message)
            | Self::Unauthenticated(message)
            | Self::NotFound(message)
            | Self::Internal(message) => message,
        }
    }

    pub fn to_envelope(&self, correlation_id: impl Into<String>) -> AgentReplyEnvelope {
        AgentReplyEnvelope {
            status: "error".to_string(),
            code: self.code().as_str().to_string(),
            message: self.message().to_string(),
            payload: Vec::new(),
            correlation_id: correlation_id.into(),
        }
    }

    pub fn to_control_envelope(&self, correlation_id: impl Into<String>) -> ControlReplyEnvelope {
        ControlReplyEnvelope {
            status: "error".to_string(),
            code: self.code().as_str().to_string(),
            message: self.message().to_string(),
            payload: Vec::new(),
            correlation_id: correlation_id.into(),
        }
    }

    pub fn to_deployment_envelope(
        &self,
        correlation_id: impl Into<String>,
    ) -> DeploymentReplyEnvelope {
        DeploymentReplyEnvelope {
            status: "error".to_string(),
            code: self.code().as_str().to_string(),
            message: self.message().to_string(),
            payload: Vec::new(),
            correlation_id: correlation_id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn maps_error_to_envelope() {
        let envelope = AppError::not_found("agent missing").to_envelope("corr-123");
        assert_eq!(envelope.status, "error");
        assert_eq!(envelope.code, "not_found");
        assert_eq!(envelope.message, "agent missing");
        assert_eq!(envelope.correlation_id, "corr-123");
        assert!(envelope.payload.is_empty());
    }

    #[test]
    fn maps_error_to_control_envelope() {
        let envelope = AppError::internal("boom").to_control_envelope("corr-456");
        assert_eq!(envelope.status, "error");
        assert_eq!(envelope.code, "internal");
        assert_eq!(envelope.message, "boom");
        assert_eq!(envelope.correlation_id, "corr-456");
        assert!(envelope.payload.is_empty());
    }

    #[test]
    fn maps_error_to_deployment_envelope() {
        let envelope = AppError::invalid_argument("bad job").to_deployment_envelope("corr-789");
        assert_eq!(envelope.status, "error");
        assert_eq!(envelope.code, "invalid_argument");
        assert_eq!(envelope.message, "bad job");
        assert_eq!(envelope.correlation_id, "corr-789");
        assert!(envelope.payload.is_empty());
    }
}
