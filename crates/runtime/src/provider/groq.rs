use crate::provider::openai::OpenAiProvider;
use crate::provider::LlmProvider;

/// Create a Groq provider instance.
///
/// Groq offers ultra-fast inference, ideal for tool-heavy agents.
pub fn create_groq_provider(api_key: &str) -> Box<dyn LlmProvider> {
    Box::new(OpenAiProvider::new(
        api_key.to_string(),
        Some("https://api.groq.com/openai/v1"),
        Some("llama-3.1-70b-versatile"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_provider_creation() {
        let provider = create_groq_provider("test-key");
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model(), "llama-3.1-70b-versatile");
    }
}
