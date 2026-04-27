use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use futures::Stream;
use crate::RuntimeError;

// =============================================================================
// Request types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Anthropic/OpenRouter prompt caching control block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self { role: "user".to_string(), content: Some(content.to_string()), tool_calls: None, tool_call_id: None, cache_control: None }
    }
    pub fn assistant(content: &str) -> Self {
        Self { role: "assistant".to_string(), content: Some(content.to_string()), tool_calls: None, tool_call_id: None, cache_control: None }
    }
    pub fn assistant_with_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self { role: "assistant".to_string(), content: None, tool_calls: Some(tool_calls), tool_call_id: None, cache_control: None }
    }
    pub fn tool_result(tool_call_id: &str, content: &str) -> Self {
        Self { role: "tool".to_string(), content: Some(content.to_string()), tool_calls: None, tool_call_id: Some(tool_call_id.to_string()), cache_control: None }
    }
    pub fn system(content: &str) -> Self {
        Self { role: "system".to_string(), content: Some(content.to_string()), tool_calls: None, tool_call_id: None, cache_control: None }
    }

    /// Get text content, defaulting to empty string
    pub fn text(&self) -> &str {
        self.content.as_deref().unwrap_or("")
    }

    /// Check if this message has tool calls
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

// =============================================================================
// Response types
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenUsage {
    #[serde(rename = "prompt_tokens")]
    pub input_tokens: u32,
    #[serde(rename = "completion_tokens")]
    pub output_tokens: u32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamChunk {
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
    pub delta: DeltaMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FunctionCallDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ToolCallDelta {
    pub index: u32,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub tool_type: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionCallDelta>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeltaMessage {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

// =============================================================================
// Provider trait
// =============================================================================

#[allow(clippy::type_complexity)]
pub trait LlmProvider: Send + Sync {
    fn chat_completion(&self, request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>>;
    fn chat_completion_stream(&self, request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>, RuntimeError>> + Send + '_>>;
    fn name(&self) -> &str;
    fn default_model(&self) -> &str;
}

// =============================================================================
// ProviderConfig — static configuration for a provider endpoint
// =============================================================================

/// Static configuration describing how to talk to a provider's API.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub base_url: String,
    pub auth_header: String,
    pub auth_prefix: String,
    pub default_model: String,
    pub supports_streaming: bool,
    pub supports_tools: bool,
}

// =============================================================================
// ProviderRegistry — maps Provider enum variants to ProviderConfig
// =============================================================================

/// Zero-sized type providing a static registry of provider configurations.
pub struct ProviderRegistry;

impl ProviderRegistry {
    /// Return the [`ProviderConfig`] for the given provider.
    ///
    /// For providers that require a user-supplied endpoint (e.g. Azure, Custom)
    /// the `base_url` will be an empty string.
    pub fn config(provider: &hermes_common::Provider) -> ProviderConfig {
        use hermes_common::Provider::*;

        match provider {
            // --- OpenAI-compatible ---
            OpenAI => ProviderConfig {
                base_url: "https://api.openai.com/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "gpt-4o".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            OpenRouter => ProviderConfig {
                base_url: "https://openrouter.ai/api/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "openai/gpt-4o".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Ollama => ProviderConfig {
                base_url: "http://localhost:11434".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "llama3".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Azure => ProviderConfig {
                base_url: String::new(),
                auth_header: "api-key".into(),
                auth_prefix: String::new(),
                default_model: "gpt-4o".into(),
                supports_streaming: true,
                supports_tools: true,
            },

            // --- Anthropic (own API format) ---
            Anthropic => ProviderConfig {
                base_url: "https://api.anthropic.com/v1".into(),
                auth_header: "x-api-key".into(),
                auth_prefix: String::new(),
                default_model: "claude-sonnet-4-20250514".into(),
                supports_streaming: true,
                supports_tools: true,
            },

            // --- Google ---
            Gemini => ProviderConfig {
                base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "gemini-2.5-pro".into(),
                supports_streaming: true,
                supports_tools: true,
            },

            // --- Chinese providers ---
            Zai => ProviderConfig {
                base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "glm-5".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Kimi => ProviderConfig {
                base_url: "https://api.moonshot.cn/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "kimi-k2.5".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            KimiCn => ProviderConfig {
                base_url: "https://api.moonshot.cn/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "kimi-k2.5".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            MiniMax => ProviderConfig {
                base_url: "https://api.minimax.chat/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "MiniMax-M2.7".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            MiniMaxCn => ProviderConfig {
                base_url: "https://api.minimax.chat/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "MiniMax-M2.7".into(),
                supports_streaming: true,
                supports_tools: true,
            },

            // --- Other providers ---
            Arcee => ProviderConfig {
                base_url: "https://api.arcee.ai/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "trinity-large-thinking".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            AiGateway => ProviderConfig {
                base_url: "https://aigateway.nousresearch.com/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "anthropic/claude-sonnet-4-6".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Kilocode => ProviderConfig {
                base_url: "https://api.kilocode.ai/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "anthropic/claude-sonnet-4-6".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            OpenCodeZen => ProviderConfig {
                base_url: "https://api.opencode.ai/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "gpt-5.4".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            OpenCodeGo => ProviderConfig {
                base_url: "https://api.opencode.ai/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "glm-5".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Copilot => ProviderConfig {
                base_url: String::new(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "gpt-5.4".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            CopilotAcp => ProviderConfig {
                base_url: String::new(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "copilot-acp".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            HuggingFace => ProviderConfig {
                base_url: "https://api-inference.huggingface.co/models".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "Qwen/Qwen3.5-397B-A17B".into(),
                supports_streaming: true,
                supports_tools: false,
            },
            DeepSeek => ProviderConfig {
                base_url: "https://api.deepseek.com/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "deepseek-chat".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Groq => ProviderConfig {
                base_url: "https://api.groq.com/openai/v1".into(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "llama-3.1-70b-versatile".into(),
                supports_streaming: true,
                supports_tools: true,
            },
            Custom => ProviderConfig {
                base_url: String::new(),
                auth_header: "Authorization".into(),
                auth_prefix: "Bearer ".into(),
                default_model: "custom".into(),
                supports_streaming: true,
                supports_tools: true,
            },
        }
    }

    /// Return every known provider alongside its static config.
    pub fn all_providers() -> Vec<(&'static str, ProviderConfig)> {
        hermes_common::Provider::all_providers()
            .iter()
            .map(|p| (p.as_str(), Self::config(p)))
            .collect()
    }
}

// =============================================================================
// Provider factory
// =============================================================================

pub fn create_provider(provider_type: &hermes_common::Provider, api_key: &str, base_url: Option<&str>)
    -> Box<dyn LlmProvider>
{
    let cfg = ProviderRegistry::config(provider_type);

    // Prefer caller-supplied base_url, then the registry config, then a
    // sensible fallback for truly dynamic providers (Azure, Custom).
    let url = base_url.map(str::to_string).unwrap_or_else(|| {
        if cfg.base_url.is_empty() {
            match provider_type {
                hermes_common::Provider::Anthropic => "https://api.anthropic.com/v1".to_string(),
                _ => "https://api.openai.com/v1".to_string(),
            }
        } else {
            cfg.base_url.clone()
        }
    });

    match provider_type {
        hermes_common::Provider::Anthropic => {
            Box::new(crate::provider::anthropic::AnthropicProvider::new(
                api_key.to_string(),
                Some(&url),
                Some(&cfg.default_model),
            ))
        }
        hermes_common::Provider::Gemini => {
            Box::new(crate::provider::gemini::GeminiProvider::new(
                api_key.to_string(),
                Some(&url),
                Some(&cfg.default_model),
            ))
        }
        hermes_common::Provider::DeepSeek => {
            Box::new(crate::provider::providers::create_deepseek_provider(
                api_key.to_string(), Some(&url), Some(&cfg.default_model),
            ))
        }
        hermes_common::Provider::Ollama => {
            Box::new(crate::provider::providers::create_ollama_provider(
                Some(&url), Some(&cfg.default_model),
            ))
        }
        hermes_common::Provider::Azure => {
            Box::new(crate::provider::providers::create_azure_provider(
                api_key.to_string(), Some(&url), Some(&cfg.default_model),
            ))
        }
        hermes_common::Provider::OpenRouter => {
            Box::new(crate::provider::providers::create_openrouter_provider(
                api_key.to_string(), Some(&url), Some(&cfg.default_model),
            ))
        }
        _ => {
            Box::new(crate::provider::openai::OpenAiProvider::new(
                api_key.to_string(),
                Some(&url),
                Some(&cfg.default_model),
            ))
        }
    }
}

pub mod openai;
pub mod openai_compatible;
pub mod anthropic;
pub mod caching;
pub mod gemini;
pub mod groq;
pub mod providers;
pub mod retry;

pub use openai::OpenAiProvider;
pub use openai_compatible::OpenAiCompatibleProvider;
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_common::Provider;

    #[test]
    fn test_create_provider_openai() {
        let provider = create_provider(&Provider::OpenAI, "test-key", None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "gpt-4o");
    }

    #[test]
    fn test_create_provider_anthropic() {
        let provider = create_provider(&Provider::Anthropic, "test-key", None);
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.default_model(), "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_create_provider_deepseek() {
        let provider = create_provider(&Provider::DeepSeek, "test-key", None);
        assert_eq!(provider.name(), "deepseek");
        assert_eq!(provider.default_model(), "deepseek-chat");
    }

    #[test]
    fn test_create_provider_custom_base_url() {
        let provider = create_provider(&Provider::OpenAI, "test-key", Some("https://custom.api.com/v1"));
        // Provider name is still "openai" but base_url is custom
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_create_provider_kimi() {
        // Kimi uses OpenAI-compatible API
        let provider = create_provider(&Provider::Kimi, "test-key", None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "kimi-k2.5");
    }

    #[test]
    fn test_create_provider_minimax() {
        let provider = create_provider(&Provider::MiniMax, "test-key", None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "MiniMax-M2.7");
    }

    #[test]
    fn test_create_provider_zai() {
        let provider = create_provider(&Provider::Zai, "test-key", None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "glm-5");
    }

    #[test]
    fn test_create_provider_openrouter() {
        let provider = create_provider(&Provider::OpenRouter, "test-key", None);
        assert_eq!(provider.name(), "openrouter");
        assert_eq!(provider.default_model(), "openai/gpt-4o");
    }

    #[test]
    fn test_create_provider_groq() {
        let provider = create_provider(&Provider::Groq, "test-key", None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "llama-3.1-70b-versatile");
    }

    #[test]
    fn test_create_provider_gemini() {
        let provider = create_provider(&Provider::Gemini, "test-key", None);
        assert_eq!(provider.name(), "gemini");
        assert_eq!(provider.default_model(), "gemini-2.5-pro");
    }

    // =========================================================================
    // ProviderConfig / ProviderRegistry tests
    // =========================================================================

    #[test]
    fn test_registry_config_openai() {
        let cfg = ProviderRegistry::config(&Provider::OpenAI);
        assert_eq!(cfg.base_url, "https://api.openai.com/v1");
        assert_eq!(cfg.auth_header, "Authorization");
        assert_eq!(cfg.auth_prefix, "Bearer ");
        assert_eq!(cfg.default_model, "gpt-4o");
        assert!(cfg.supports_streaming);
        assert!(cfg.supports_tools);
    }

    #[test]
    fn test_registry_config_anthropic() {
        let cfg = ProviderRegistry::config(&Provider::Anthropic);
        assert_eq!(cfg.base_url, "https://api.anthropic.com/v1");
        assert_eq!(cfg.auth_header, "x-api-key");
        assert!(cfg.auth_prefix.is_empty());
        assert_eq!(cfg.default_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_registry_config_azure() {
        let cfg = ProviderRegistry::config(&Provider::Azure);
        assert!(cfg.base_url.is_empty());
        assert_eq!(cfg.auth_header, "api-key");
        assert!(cfg.auth_prefix.is_empty());
    }

    #[test]
    fn test_registry_config_groq() {
        let cfg = ProviderRegistry::config(&Provider::Groq);
        assert_eq!(cfg.base_url, "https://api.groq.com/openai/v1");
        assert_eq!(cfg.default_model, "llama-3.1-70b-versatile");
    }

    #[test]
    fn test_registry_config_deepseek() {
        let cfg = ProviderRegistry::config(&Provider::DeepSeek);
        assert_eq!(cfg.base_url, "https://api.deepseek.com/v1");
        assert_eq!(cfg.default_model, "deepseek-chat");
    }

    #[test]
    fn test_registry_config_ollama() {
        let cfg = ProviderRegistry::config(&Provider::Ollama);
        assert_eq!(cfg.base_url, "http://localhost:11434");
        assert_eq!(cfg.default_model, "llama3");
    }

    #[test]
    fn test_registry_config_huggingface_no_tools() {
        let cfg = ProviderRegistry::config(&Provider::HuggingFace);
        assert!(!cfg.supports_tools);
    }

    #[test]
    fn test_registry_all_providers_completeness() {
        let all = ProviderRegistry::all_providers();
        // Must cover every variant in Provider::all_providers()
        let enum_count = Provider::all_providers().len();
        assert_eq!(all.len(), enum_count);
        // Spot-check a few entries
        assert!(all.iter().any(|(name, _)| *name == "openai"));
        assert!(all.iter().any(|(name, _)| *name == "anthropic"));
        assert!(all.iter().any(|(name, _)| *name == "groq"));
        assert!(all.iter().any(|(name, _)| *name == "deepseek"));
    }

    #[test]
    fn test_registry_config_matches_default_model() {
        // Every ProviderConfig.default_model must match Provider::default_model()
        for p in Provider::all_providers() {
            let cfg = ProviderRegistry::config(p);
            assert_eq!(
                cfg.default_model, p.default_model(),
                "model mismatch for provider {:?}",
                p
            );
        }
    }

    #[test]
    fn test_registry_config_matches_default_base_url() {
        // Every ProviderConfig.base_url must match Provider::default_base_url()
        for p in Provider::all_providers() {
            let cfg = ProviderRegistry::config(p);
            assert_eq!(
                cfg.base_url, p.default_base_url(),
                "base_url mismatch for provider {:?}",
                p
            );
        }
    }
}
