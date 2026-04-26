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
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeltaMessage {
    pub content: Option<String>,
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

pub fn create_provider(provider_type: &hermes_common::Provider, api_key: &str, base_url: Option<&str>) 
    -> Box<dyn LlmProvider> {
    match provider_type {
        hermes_common::Provider::Anthropic => {
            let default_base = provider_type.default_base_url();
            let url = base_url.unwrap_or(if default_base.is_empty() { "https://api.anthropic.com/v1" } else { default_base });
            let model = provider_type.default_model();
            Box::new(crate::provider::anthropic::AnthropicProvider::new(
                api_key.to_string(), Some(url), Some(model),
            ))
        }
        hermes_common::Provider::Groq => {
            let default_base = provider_type.default_base_url();
            let url = base_url.unwrap_or(if default_base.is_empty() { "https://api.groq.com/openai/v1" } else { default_base });
            let model = provider_type.default_model();
            Box::new(crate::provider::openai::OpenAiProvider::new(
                api_key.to_string(), Some(url), Some(model),
            ))
        }
        _ => {
            let default_base = provider_type.default_base_url();
            let url = base_url.unwrap_or(if default_base.is_empty() { "https://api.openai.com/v1" } else { default_base });
            let model = provider_type.default_model();
            Box::new(crate::provider::openai::OpenAiProvider::new(
                api_key.to_string(), Some(url), Some(model),
            ))
        }
    }
}

pub mod openai;
pub mod anthropic;
pub mod caching;
pub mod groq;
pub mod retry;

pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;

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
        // DeepSeek uses OpenAI-compatible API
        let provider = create_provider(&Provider::DeepSeek, "test-key", None);
        assert_eq!(provider.name(), "openai"); // Uses OpenAI provider struct
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
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "openai/gpt-4o");
    }

    #[test]
    fn test_create_provider_groq() {
        let provider = create_provider(&Provider::Groq, "test-key", None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "llama-3.1-70b-versatile");
    }
}
