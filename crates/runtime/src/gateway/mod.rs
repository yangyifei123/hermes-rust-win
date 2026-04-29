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
    fn send_message(
        &self,
        chat_id: &str,
        message: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RuntimeError>> + Send + '_>>;
    fn name(&self) -> &str;
}

pub mod qq;
pub mod wechat;

/// Returns the number of UTF-16 code units in a string.
/// Used for Telegram/WeChat message length limits which count UTF-16 code units.
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Truncates a string to at most `max_len` UTF-16 code units.
/// Returns the original string unchanged if it fits within the limit.
pub fn truncate_utf16(s: &str, max_len: usize) -> String {
    let encoded: Vec<u16> = s.encode_utf16().collect();
    if encoded.len() <= max_len {
        return s.to_string();
    }
    String::from_utf16_lossy(&encoded[..max_len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf16_len_ascii() {
        assert_eq!(utf16_len("hello"), 5);
    }

    #[test]
    fn test_utf16_len_cjk() {
        // CJK characters are 1 UTF-16 code unit each
        assert_eq!(utf16_len("你好"), 2);
    }

    #[test]
    fn test_utf16_len_emoji() {
        // Basic emoji like 😀 is 2 UTF-16 code units (surrogate pair)
        assert_eq!(utf16_len("😀"), 2);
    }

    #[test]
    fn test_utf16_len_mixed() {
        // "hi😀" = 2 ASCII + 2 surrogate = 4
        assert_eq!(utf16_len("hi😀"), 4);
    }

    #[test]
    fn test_truncate_utf16_no_truncation_needed() {
        assert_eq!(truncate_utf16("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_utf16_exact_fit() {
        assert_eq!(truncate_utf16("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_utf16_truncates() {
        assert_eq!(truncate_utf16("hello", 3), "hel");
    }

    #[test]
    fn test_truncate_utf16_cjk() {
        assert_eq!(truncate_utf16("你好世界", 2), "你好");
    }

    #[test]
    fn test_truncate_utf16_preserves_when_fits() {
        let s = "你好";
        assert_eq!(truncate_utf16(s, 2), s);
    }

    #[test]
    fn test_truncate_utf16_empty() {
        assert_eq!(truncate_utf16("", 0), "");
    }
}
