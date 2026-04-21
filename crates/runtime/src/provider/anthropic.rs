use crate::provider::{ChatRequest, ChatResponse, LlmProvider, StreamChunk};
use crate::RuntimeError;
use futures::Stream;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or("https://api.anthropic.com/v1").to_string(),
            model: model.unwrap_or("claude-sonnet-4-20250514").to_string(),
        }
    }
}

impl LlmProvider for AnthropicProvider {
    fn chat_completion(&self, mut request: ChatRequest) -> Pin<Box<dyn std::future::Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }

            let mut system_prompt = String::new();
            let mut anthropic_messages = Vec::new();
            for msg in &request.messages {
                match msg.role.as_str() {
                    "system" => system_prompt = msg.text().to_string(),
                    _ => anthropic_messages.push(json!({
                        "role": msg.role,
                        "content": msg.text(),
                    })),
                }
            }

            let mut body = json!({
                "model": request.model,
                "messages": anthropic_messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
            });
            if !system_prompt.is_empty() {
                body["system"] = json!(system_prompt);
            }

            let url = format!("{}/messages", self.base_url);
            let resp = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })?;

            if resp.status().is_success() {
                let raw: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })?;

                let content = raw["content"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|block| block["text"].as_str())
                    .unwrap_or("")
                    .to_string();

                Ok(ChatResponse {
                    choices: vec![crate::provider::ChatChoice {
                        message: crate::provider::ChatMessage::assistant(&content),
                        finish_reason: raw["stop_reason"].as_str().map(|s| s.to_string()),
                    }],
                })
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(RuntimeError::ProviderError {
                    message: format!("Anthropic API error {}: {}", status, body),
                })
            }
        })
    }

    fn chat_completion_stream(
        &self,
        _request: ChatRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            Err(RuntimeError::ProviderError {
                message: "Streaming not yet implemented for Anthropic".to_string(),
            })
        })
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        "claude-sonnet-4-20250514"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new("test-key".to_string(), None, None);
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.default_model(), "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_anthropic_uses_x_api_key() {
        let provider = AnthropicProvider::new("sk-ant-test".to_string(), None, None);
        assert_eq!(provider.api_key, "sk-ant-test");
        assert!(provider.base_url.contains("anthropic.com"));
    }
}
