use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use futures::Stream;
use crate::RuntimeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
