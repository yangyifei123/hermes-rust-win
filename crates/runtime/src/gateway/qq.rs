//! QQ Bot gateway adapter stub

use crate::gateway::PlatformAdapter;
use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QqConfig {
    pub app_id: String,
    pub app_secret: String,
    pub token: String,
}

pub struct QqAdapter {
    #[allow(dead_code)]
    config: QqConfig,
}

impl QqAdapter {
    pub fn new(config: QqConfig) -> Self {
        Self { config }
    }
}

impl PlatformAdapter for QqAdapter {
    fn start(&mut self) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::AgentError {
                message: "QQ adapter not yet implemented".to_string(),
            })
        })
    }

    fn stop(&mut self) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::AgentError {
                message: "QQ adapter not yet implemented".to_string(),
            })
        })
    }

    fn send_message(&self, _chat_id: &str, _message: &str) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::AgentError {
                message: "QQ adapter not yet implemented".to_string(),
            })
        })
    }

    fn name(&self) -> &str {
        "qq"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qq_adapter_creation() {
        let config = QqConfig {
            app_id: "test".to_string(),
            app_secret: "secret".to_string(),
            token: "token".to_string(),
        };
        let adapter = QqAdapter::new(config);
        assert_eq!(adapter.name(), "qq");
    }

    #[tokio::test]
    async fn test_qq_stub_returns_error() {
        let config = QqConfig {
            app_id: "test".to_string(),
            app_secret: "secret".to_string(),
            token: "token".to_string(),
        };
        let mut adapter = QqAdapter::new(config);
        let result = adapter.start().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not yet implemented"));
    }
}
