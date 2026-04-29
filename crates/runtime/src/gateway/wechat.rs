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

    fn send_message(
        &self,
        _chat_id: &str,
        _message: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>> {
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

/// AES-128-ECB encryption stub for WeChat message encryption.
/// Actual implementation requires the 'aes' crate dependency.
pub fn aes128_ecb_encrypt(_plaintext: &[u8], _key: &[u8]) -> Result<Vec<u8>, RuntimeError> {
    Err(RuntimeError::AgentError {
        message: "AES encryption not yet implemented - add 'aes' crate dependency".into(),
    })
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

    #[test]
    fn test_aes128_ecb_encrypt_stub_returns_error() {
        let result = aes128_ecb_encrypt(b"plaintext", b"0123456789abcdef");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("AES encryption not yet implemented"));
        assert!(msg.contains("'aes' crate dependency"));
    }
}
