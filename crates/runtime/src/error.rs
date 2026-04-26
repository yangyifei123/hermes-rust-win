use std::error::Error as StdError;

#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("provider error: {message}")]
    ProviderError { message: String },
    #[error("tool error [{name}]: {message}")]
    ToolError { name: String, message: String },
    #[error("agent error: {message}")]
    AgentError { message: String },
    #[error("session error: {source}")]
    SessionError { source: Box<dyn StdError + Send + Sync> },
    #[error("timeout after {duration_secs}s")]
    TimeoutError { duration_secs: u64 },
    #[error("rate limited (retry after: {retry_after:?}s)")]
    RateLimitError { retry_after: Option<u64> },
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("retry exhausted after {attempts} attempts: {last_error}")]
    RetryExhausted { attempts: u32, last_error: String },
}
