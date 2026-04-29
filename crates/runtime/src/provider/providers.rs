//! Individual provider factory functions.
//!
//! Each function creates an `OpenAiCompatibleProvider` with provider-specific
//! defaults (base URL, default model, auth header). All use the standard
//! OpenAI chat completions API.

use crate::provider::openai_compatible::OpenAiCompatibleProvider;

/// Create a DeepSeek provider.
/// API: https://api.deepseek.com/v1 (OpenAI-compatible)
pub fn create_deepseek_provider(
    api_key: String,
    base_url: Option<&str>,
    model: Option<&str>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        api_key,
        base_url.or(Some("https://api.deepseek.com/v1")),
        model.or(Some("deepseek-chat")),
        "deepseek",
        None,
        None,
    )
}

/// Create an Ollama provider (local models).
/// API: http://localhost:11434/v1 (OpenAI-compatible, no auth)
pub fn create_ollama_provider(
    base_url: Option<&str>,
    model: Option<&str>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        "ollama".to_string(), // Ollama doesn't require an API key
        base_url.or(Some("http://localhost:11434/v1")),
        model.or(Some("llama3")),
        "ollama",
        None,
        None,
    )
}

/// Create an Azure OpenAI provider.
/// Auth: `api-key` header (not Bearer)
pub fn create_azure_provider(
    api_key: String,
    base_url: Option<&str>,
    model: Option<&str>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        api_key,
        base_url,
        model.or(Some("gpt-4o")),
        "azure",
        Some("api-key"),
        Some(""), // No prefix for Azure
    )
}

/// Create an OpenRouter provider.
/// API: https://openrouter.ai/api/v1 (OpenAI-compatible)
pub fn create_openrouter_provider(
    api_key: String,
    base_url: Option<&str>,
    model: Option<&str>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        api_key,
        base_url.or(Some("https://openrouter.ai/api/v1")),
        model.or(Some("openai/gpt-4o")),
        "openrouter",
        None,
        None,
    )
}

/// Create a Mistral provider.
/// API: https://api.mistral.ai/v1 (OpenAI-compatible)
pub fn create_mistral_provider(
    api_key: String,
    base_url: Option<&str>,
    model: Option<&str>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        api_key,
        base_url.or(Some("https://api.mistral.ai/v1")),
        model.or(Some("mistral-large-latest")),
        "mistral",
        None,
        None,
    )
}

/// Create a Cohere provider.
/// API: https://api.cohere.ai/v1 (OpenAI-compatible)
pub fn create_cohere_provider(
    api_key: String,
    base_url: Option<&str>,
    model: Option<&str>,
) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new(
        api_key,
        base_url.or(Some("https://api.cohere.ai/v1")),
        model.or(Some("command-r-plus")),
        "cohere",
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::LlmProvider;

    #[test]
    fn test_deepseek_provider() {
        let p = create_deepseek_provider("key".to_string(), None, None);
        assert_eq!(p.name(), "deepseek");
        assert_eq!(p.default_model(), "deepseek-chat");
        assert!(p.base_url().contains("deepseek.com"));
    }

    #[test]
    fn test_ollama_provider() {
        let p = create_ollama_provider(None, None);
        assert_eq!(p.name(), "ollama");
        assert_eq!(p.default_model(), "llama3");
        assert!(p.base_url().contains("localhost:11434"));
    }

    #[test]
    fn test_azure_provider() {
        let p = create_azure_provider(
            "key".to_string(),
            Some("https://myresource.openai.azure.com/openai/deployments/gpt-4o"),
            None,
        );
        assert_eq!(p.name(), "azure");
        assert_eq!(p.default_model(), "gpt-4o");
    }

    #[test]
    fn test_openrouter_provider() {
        let p = create_openrouter_provider("key".to_string(), None, None);
        assert_eq!(p.name(), "openrouter");
        assert_eq!(p.default_model(), "openai/gpt-4o");
        assert!(p.base_url().contains("openrouter.ai"));
    }

    #[test]
    fn test_mistral_provider() {
        let p = create_mistral_provider("key".to_string(), None, None);
        assert_eq!(p.name(), "mistral");
        assert_eq!(p.default_model(), "mistral-large-latest");
        assert!(p.base_url().contains("mistral.ai"));
    }

    #[test]
    fn test_cohere_provider() {
        let p = create_cohere_provider("key".to_string(), None, None);
        assert_eq!(p.name(), "cohere");
        assert_eq!(p.default_model(), "command-r-plus");
    }
}
