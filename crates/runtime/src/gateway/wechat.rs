//! WeChat gateway adapter stub

use crate::gateway::PlatformAdapter;
use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WechatConfig {
    pub app_id: String,
    pub app_secret: String,
    pub webhook_url: String,
    pub token: String,
}

pub struct WechatAdapter {
    #[allow(dead_code)]
    config: WechatConfig,
}

impl WechatAdapter {
    pub fn new(config: WechatConfig) -> Self {
        Self { config }
    }
}

impl PlatformAdapter for WechatAdapter {
    fn start(&mut self) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::AgentError {
                message: "WeChat adapter not yet implemented".to_string(),
            })
        })
    }

    fn stop(&mut self) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::AgentError {
                message: "WeChat adapter not yet implemented".to_string(),
            })
        })
    }

    fn send_message(&self, _chat_id: &str, _message: &str) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::AgentError {
                message: "WeChat adapter not yet implemented".to_string(),
            })
        })
    }

    fn name(&self) -> &str {
        "wechat"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wechat_adapter_creation() {
        let config = WechatConfig {
            app_id: "test".to_string(),
            app_secret: "secret".to_string(),
            webhook_url: "https://example.com".to_string(),
            token: "token".to_string(),
        };
        let adapter = WechatAdapter::new(config);
        assert_eq!(adapter.name(), "wechat");
    }

    #[tokio::test]
    async fn test_wechat_stub_returns_error() {
        let config = WechatConfig {
            app_id: "test".to_string(),
            app_secret: "secret".to_string(),
            webhook_url: "https://example.com".to_string(),
            token: "token".to_string(),
        };
        let mut adapter = WechatAdapter::new(config);
        let result = adapter.start().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not yet implemented"));
    }
}
