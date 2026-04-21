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
            Box::new(crate::provider::anthropic::AnthropicProvider::new(
                api_key.to_string(), base_url, None,
            ))
        }
        _ => {
            Box::new(crate::provider::openai::OpenAiProvider::new(
                api_key.to_string(), base_url, None,
            ))
        }
    }
}

pub mod openai;
pub mod anthropic;
pub mod caching;

pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
