use thiserror::Error;

#[derive(Error, Debug)]
pub enum SessionError {
    #[error("database error: {0}")]
    DatabaseError(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("message not found: {0}")]
    MessageNotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SessionError>;
