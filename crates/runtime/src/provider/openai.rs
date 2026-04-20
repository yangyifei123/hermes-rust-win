use crate::provider::{ChatRequest, ChatResponse, LlmProvider, StreamChunk};
use crate::RuntimeError;
use futures::Stream;
use reqwest::Client;
use std::pin::Pin;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or("https://api.openai.com/v1").to_string(),
            model: model.unwrap_or("gpt-4o").to_string(),
        }
    }
}

impl LlmProvider for OpenAiProvider {
    fn chat_completion(&self, mut request: ChatRequest) -> Pin<Box<dyn std::future::Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }
            let url = format!("{}/chat/completions", self.base_url);
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })?;

            if resp.status().is_success() {
                resp.json::<ChatResponse>()
                    .await
                    .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })
            } else {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                Err(RuntimeError::ProviderError {
                    message: format!("API error {}: {}", status, body),
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
                message: "Streaming not yet implemented for OpenAI".to_string(),
            })
        })
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        "gpt-4o"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAiProvider::new("test-key".to_string(), None, None);
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "gpt-4o");
    }

    #[test]
    fn test_openai_custom_base_url() {
        let provider = OpenAiProvider::new(
            "test-key".to_string(),
            Some("https://custom.api.com/v1"),
            Some("gpt-3.5-turbo"),
        );
        assert_eq!(provider.base_url, "https://custom.api.com/v1");
        assert_eq!(provider.model, "gpt-3.5-turbo");
    }
}
