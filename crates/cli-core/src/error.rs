use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("authentication error: {0}")]
    Auth(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),
}

impl CliError {
    pub fn auth(msg: impl Into<String>) -> Self {
        CliError::Auth(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        CliError::Config(msg.into())
    }

    pub fn invalid_arg(msg: impl Into<String>) -> Self {
        CliError::InvalidArgument(msg.into())
    }
}
