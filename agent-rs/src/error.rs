use std::{fmt, io, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportErrorKind {
    TransientNetwork,
    ServerRejected,
    SerializationError,
    Unauthorized,
    Unknown,
}

impl fmt::Display for TransportErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransientNetwork => f.write_str("TransientNetwork"),
            Self::ServerRejected => f.write_str("ServerRejected"),
            Self::SerializationError => f.write_str("SerializationError"),
            Self::Unauthorized => f.write_str("Unauthorized"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("http transport error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("protobuf encode error: {0}")]
    ProstEncode(#[from] prost::EncodeError),
    #[error("protobuf decode error: {0}")]
    ProstDecode(#[from] prost::DecodeError),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("http status {status}: {message}")]
    HttpStatus {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("grpc status {code}: {message}")]
    GrpcStatus { code: i32, message: String },
    #[error("path does not exist: {0}")]
    MissingPath(PathBuf),
}

impl AppError {
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig(message.into())
    }

    pub fn protocol(message: impl Into<String>) -> Self {
        Self::Protocol(message.into())
    }

    pub fn grpc_status(code: i32, message: impl Into<String>) -> Self {
        Self::GrpcStatus {
            code,
            message: message.into(),
        }
    }

    pub fn is_identity_error(&self) -> bool {
        matches!(
            self,
            Self::GrpcStatus { code, .. } if matches!(code, 3 | 5 | 7 | 16)
        )
    }

    pub fn transport_error_kind(&self) -> TransportErrorKind {
        match self {
            Self::Http(error) => {
                if error.is_timeout() || error.is_connect() || error.is_request() {
                    TransportErrorKind::TransientNetwork
                } else if error.is_decode() || error.is_body() {
                    TransportErrorKind::SerializationError
                } else {
                    TransportErrorKind::Unknown
                }
            }
            Self::HttpStatus { status, .. } => match status.as_u16() {
                401 | 403 => TransportErrorKind::Unauthorized,
                408 | 425 | 429 => TransportErrorKind::TransientNetwork,
                500..=599 => TransportErrorKind::TransientNetwork,
                400..=499 => TransportErrorKind::ServerRejected,
                _ => TransportErrorKind::Unknown,
            },
            Self::GrpcStatus { code, .. } => match code {
                16 | 7 => TransportErrorKind::Unauthorized,
                4 | 8 | 13 | 14 => TransportErrorKind::TransientNetwork,
                3 | 5 | 6 | 9 | 10 | 11 | 12 => TransportErrorKind::ServerRejected,
                _ => TransportErrorKind::Unknown,
            },
            Self::Json(_) | Self::Yaml(_) | Self::ProstEncode(_) | Self::ProstDecode(_) => {
                TransportErrorKind::SerializationError
            }
            Self::Protocol(message) => {
                let normalized = message.to_ascii_lowercase();
                if normalized.contains("unauthorized") || normalized.contains("forbidden") {
                    TransportErrorKind::Unauthorized
                } else if normalized.contains("reject")
                    || normalized.contains("invalid")
                    || normalized.contains("denied")
                {
                    TransportErrorKind::ServerRejected
                } else {
                    TransportErrorKind::Unknown
                }
            }
            _ => TransportErrorKind::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, TransportErrorKind};

    #[test]
    fn classifies_protocol_rejections_as_server_rejected() {
        let error = AppError::protocol("edge ingest response rejected the batch");
        assert_eq!(
            error.transport_error_kind(),
            TransportErrorKind::ServerRejected
        );
    }

    #[test]
    fn classifies_protocol_unauthorized_as_unauthorized() {
        let error = AppError::protocol("unauthorized bootstrap token");
        assert_eq!(
            error.transport_error_kind(),
            TransportErrorKind::Unauthorized
        );
    }
}
