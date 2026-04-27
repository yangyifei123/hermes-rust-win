use crate::provider::openai_compatible::OpenAiCompatibleProvider;
use crate::provider::{ChatRequest, ChatResponse, LlmProvider, StreamChunk};
use crate::RuntimeError;
use futures::Stream;
use std::pin::Pin;

pub struct OpenAiProvider(OpenAiCompatibleProvider);

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<&str>, model: Option<&str>) -> Self {
        Self(OpenAiCompatibleProvider::new(
            api_key,
            base_url,
            model,
            "openai",
            None,
            None,
        ))
    }

    /// Expose the internal `base_url` for tests that check it directly.
    #[cfg(test)]
    pub fn base_url(&self) -> &str {
        self.0.base_url()
    }
}

impl LlmProvider for OpenAiProvider {
    fn chat_completion(
        &self,
        request: ChatRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>>
    {
        self.0.chat_completion(request)
    }

    #[allow(clippy::type_complexity)]
    fn chat_completion_stream(
        &self,
        request: ChatRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>,
                        RuntimeError,
                    >,
                > + Send
                + '_,
        >,
    > {
        self.0.chat_completion_stream(request)
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn default_model(&self) -> &str {
        self.0.default_model()
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
        assert_eq!(provider.base_url(), "https://custom.api.com/v1");
        assert_eq!(provider.default_model(), "gpt-3.5-turbo");
    }
}
