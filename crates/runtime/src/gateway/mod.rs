//! Gateway platform adapters for messaging integrations

use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// Gateway platform types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Telegram,
    Discord,
    Slack,
    Whatsapp,
    Wechat,
    Qq,
    Signal,
}

/// Trait for platform-specific messaging adapters
pub trait PlatformAdapter: Send + Sync {
    fn start(&mut self) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>>;
    fn stop(&mut self) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>>;
    fn send_message(&self, chat_id: &str, message: &str) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>>;
    fn name(&self) -> &str;
}

pub mod wechat;
pub mod qq;
