use std::{io, path::PathBuf};

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

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
}
