use thiserror::Error;

pub type Result<T> = std::result::Result<T, HermesError>;

#[derive(Error, Debug)]
pub enum HermesError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("internal error: {0}")]
    Internal(String),
}