//! Provider Detection from URL Patterns
//!
//! Auto-detects provider and API mode from base URLs,
//! matching Python's auto-detection logic.

use crate::types::Provider;

use std::collections::HashMap;

/// API mode determining the request/response format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMode {
    /// Standard OpenAI-compatible chat completions endpoint.
    ChatCompletions,
    /// Anthropic Messages API format.
    AnthropicMessages,
    /// GPT-5.x Responses API (Codex).
    CodexResponses,
}

/// Lazy-initialized static mapping of URL hostnames to providers.
fn get_url_to_provider() -> &'static HashMap<&'static str, Provider> {
    // Safety: we only write once and never drop the allocation.
    // Use a leaked Box to get a &'static reference.
    static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    static mut CACHE: Option<&'static HashMap<&'static str, Provider>> = None;

    if !ONCE.load(std::sync::atomic::Ordering::Acquire) {
        let map: HashMap<&'static str, Provider> = HashMap::from([
            ("api.openai.com", Provider::OpenAI),
            ("api.anthropic.com", Provider::Anthropic),
            ("openrouter.ai", Provider::OpenRouter),
            ("api.deepseek.com", Provider::DeepSeek),
            ("api.minimax.chat", Provider::MiniMax),
            ("open.bigmodel.cn", Provider::Zai),
            ("api.moonshot.cn", Provider::Kimi),
            ("generativelanguage.googleapis.com", Provider::Gemini),
            ("api.arcee.ai", Provider::Arcee),
            ("localhost:11434", Provider::Ollama),
        ]);
        let boxed = Box::new(map);
        // SAFETY: single-threaded init, written once before any read
        #[allow(static_mut_refs)]
        unsafe {
            CACHE = Some(Box::leak(boxed));
        }
        ONCE.store(true, std::sync::atomic::Ordering::Release);
    }
    #[allow(static_mut_refs)]
    // SAFETY: written once before this read via the atomic guard above
    unsafe {
        CACHE.unwrap()
    }
}

/// Detect a provider from a base URL by matching known hostnames.
///
/// Returns `None` if no known provider hostname is found in the URL.
pub fn detect_provider_from_url(base_url: &str) -> Option<Provider> {
    let url_lower = base_url.to_lowercase();
    for (host, provider) in get_url_to_provider().iter() {
        if url_lower.contains(host) {
            return Some(*provider);
        }
    }
    None
}

/// Detect the API mode based on the base URL and known provider.
///
/// - Anthropic providers (or URLs containing "anthropic") use `AnthropicMessages`.
/// - URLs ending with `/anthropic` also use `AnthropicMessages`.
/// - All others default to `ChatCompletions`.
pub fn detect_api_mode(base_url: &str, provider: Provider) -> ApiMode {
    let url_lower = base_url.to_lowercase();
    if provider == Provider::Anthropic
        || url_lower.contains("api.anthropic.com")
        || url_lower.ends_with("/anthropic")
    {
        ApiMode::AnthropicMessages
    } else {
        ApiMode::ChatCompletions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_openai() {
        assert_eq!(
            detect_provider_from_url("https://api.openai.com/v1"),
            Some(Provider::OpenAI)
        );
    }

    #[test]
    fn test_detect_anthropic() {
        assert_eq!(
            detect_provider_from_url("https://api.anthropic.com/v1"),
            Some(Provider::Anthropic)
        );
    }

    #[test]
    fn test_detect_openrouter() {
        assert_eq!(
            detect_provider_from_url("https://openrouter.ai/api/v1"),
            Some(Provider::OpenRouter)
        );
    }

    #[test]
    fn test_detect_deepseek() {
        assert_eq!(
            detect_provider_from_url("https://api.deepseek.com/v1"),
            Some(Provider::DeepSeek)
        );
    }

    #[test]
    fn test_detect_ollama() {
        assert_eq!(
            detect_provider_from_url("http://localhost:11434"),
            Some(Provider::Ollama)
        );
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_provider_from_url("https://example.com/api"), None);
    }

    #[test]
    fn test_detect_case_insensitive() {
        assert_eq!(
            detect_provider_from_url("HTTPS://API.OPENAI.COM/V1"),
            Some(Provider::OpenAI)
        );
    }

    #[test]
    fn test_api_mode_anthropic_provider() {
        assert_eq!(
            detect_api_mode("https://api.anthropic.com/v1", Provider::Anthropic),
            ApiMode::AnthropicMessages
        );
    }

    #[test]
    fn test_api_mode_anthropic_url_suffix() {
        assert_eq!(
            detect_api_mode(
                "https://openrouter.ai/api/v1/anthropic",
                Provider::OpenRouter
            ),
            ApiMode::AnthropicMessages
        );
    }

    #[test]
    fn test_api_mode_openai() {
        assert_eq!(
            detect_api_mode("https://api.openai.com/v1", Provider::OpenAI),
            ApiMode::ChatCompletions
        );
    }

    #[test]
    fn test_api_mode_anthropic_url_contains() {
        assert_eq!(
            detect_api_mode("https://api.anthropic.com/v1", Provider::OpenAI),
            ApiMode::AnthropicMessages
        );
    }

    #[test]
    fn test_api_mode_deepseek() {
        assert_eq!(
            detect_api_mode("https://api.deepseek.com/v1", Provider::DeepSeek),
            ApiMode::ChatCompletions
        );
    }
}
