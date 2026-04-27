//! Model Metadata Database
//!
//! Static registry of known AI models with context lengths, pricing,
//! and capability flags. Used for cost estimation, context management,
//! and model selection.

use once_cell::sync::Lazy;

use crate::types::Provider;

// =============================================================================
// ModelMetadata
// =============================================================================

/// Metadata for a known AI model.
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub name: &'static str,
    pub provider: Provider,
    pub context_length: u32,
    pub max_output_tokens: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    /// USD per 1 million input tokens
    pub input_price_per_million: f64,
    /// USD per 1 million output tokens
    pub output_price_per_million: f64,
}

// =============================================================================
// Static Registry
// =============================================================================

/// Global model registry containing metadata for 50+ models.
#[allow(dead_code)]
static MODEL_REGISTRY: Lazy<Vec<ModelMetadata>> = Lazy::new(|| {
    vec![
        // ----- OpenAI -----
        ModelMetadata {
            name: "gpt-4o",
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 2.50,
            output_price_per_million: 10.0,
        },
        ModelMetadata {
            name: "gpt-4o-mini",
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.15,
            output_price_per_million: 0.60,
        },
        ModelMetadata {
            name: "gpt-4-turbo",
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 4_096,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 10.0,
            output_price_per_million: 30.0,
        },
        ModelMetadata {
            name: "o1",
            provider: Provider::OpenAI,
            context_length: 200_000,
            max_output_tokens: 100_000,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 15.0,
            output_price_per_million: 60.0,
        },
        ModelMetadata {
            name: "o1-mini",
            provider: Provider::OpenAI,
            context_length: 128_000,
            max_output_tokens: 65_536,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 3.0,
            output_price_per_million: 12.0,
        },
        ModelMetadata {
            name: "o3-mini",
            provider: Provider::OpenAI,
            context_length: 200_000,
            max_output_tokens: 100_000,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.10,
            output_price_per_million: 4.40,
        },
        ModelMetadata {
            name: "gpt-3.5-turbo",
            provider: Provider::OpenAI,
            context_length: 16_385,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.50,
            output_price_per_million: 1.50,
        },
        // ----- Anthropic -----
        ModelMetadata {
            name: "claude-sonnet-4-20250514",
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 3.0,
            output_price_per_million: 15.0,
        },
        ModelMetadata {
            name: "claude-opus-4-20250514",
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 15.0,
            output_price_per_million: 75.0,
        },
        ModelMetadata {
            name: "claude-haiku-3-20240307",
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 4_096,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.25,
            output_price_per_million: 1.25,
        },
        ModelMetadata {
            name: "claude-3.5-sonnet-20241022",
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 3.0,
            output_price_per_million: 15.0,
        },
        ModelMetadata {
            name: "claude-3-5-haiku-20241022",
            provider: Provider::Anthropic,
            context_length: 200_000,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.0,
            output_price_per_million: 5.0,
        },
        // ----- Google Gemini -----
        ModelMetadata {
            name: "gemini-2.5-pro",
            provider: Provider::Gemini,
            context_length: 1_048_576,
            max_output_tokens: 65_536,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.25,
            output_price_per_million: 10.0,
        },
        ModelMetadata {
            name: "gemini-2.0-flash",
            provider: Provider::Gemini,
            context_length: 1_048_576,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.10,
            output_price_per_million: 0.40,
        },
        ModelMetadata {
            name: "gemini-1.5-pro",
            provider: Provider::Gemini,
            context_length: 2_097_152,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.25,
            output_price_per_million: 5.0,
        },
        ModelMetadata {
            name: "gemini-1.5-flash",
            provider: Provider::Gemini,
            context_length: 1_048_576,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.075,
            output_price_per_million: 0.30,
        },
        // ----- DeepSeek -----
        ModelMetadata {
            name: "deepseek-chat",
            provider: Provider::DeepSeek,
            context_length: 64_000,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.14,
            output_price_per_million: 0.28,
        },
        ModelMetadata {
            name: "deepseek-reasoner",
            provider: Provider::DeepSeek,
            context_length: 64_000,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.55,
            output_price_per_million: 2.19,
        },
        // ----- Groq -----
        ModelMetadata {
            name: "llama-3.1-70b-versatile",
            provider: Provider::Groq,
            context_length: 131_072,
            max_output_tokens: 32_768,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.59,
            output_price_per_million: 0.79,
        },
        ModelMetadata {
            name: "llama-3.1-8b-instant",
            provider: Provider::Groq,
            context_length: 131_072,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.05,
            output_price_per_million: 0.08,
        },
        ModelMetadata {
            name: "mixtral-8x7b-32768",
            provider: Provider::Groq,
            context_length: 32_768,
            max_output_tokens: 32_768,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.24,
            output_price_per_million: 0.24,
        },
        ModelMetadata {
            name: "llama-3.3-70b-versatile",
            provider: Provider::Groq,
            context_length: 131_072,
            max_output_tokens: 32_768,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.59,
            output_price_per_million: 0.79,
        },
        ModelMetadata {
            name: "gemma2-9b-it",
            provider: Provider::Groq,
            context_length: 8_192,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.20,
            output_price_per_million: 0.20,
        },
        // ----- Ollama / Local -----
        ModelMetadata {
            name: "llama3",
            provider: Provider::Ollama,
            context_length: 8_192,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: false,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "llama3.1",
            provider: Provider::Ollama,
            context_length: 131_072,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "llama3.2",
            provider: Provider::Ollama,
            context_length: 131_072,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "mistral",
            provider: Provider::Ollama,
            context_length: 32_768,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: false,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "codellama",
            provider: Provider::Ollama,
            context_length: 16_384,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: false,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "phi-3",
            provider: Provider::Ollama,
            context_length: 128_000,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: false,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "qwen2",
            provider: Provider::Ollama,
            context_length: 32_768,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "gemma2",
            provider: Provider::Ollama,
            context_length: 8_192,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: false,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "deepseek-coder-v2",
            provider: Provider::Ollama,
            context_length: 131_072,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "nomic-embed-text",
            provider: Provider::Ollama,
            context_length: 8_192,
            max_output_tokens: 0,
            supports_vision: false,
            supports_tools: false,
            supports_streaming: false,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        // ----- OpenRouter -----
        ModelMetadata {
            name: "openai/gpt-4o",
            provider: Provider::OpenRouter,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 2.50,
            output_price_per_million: 10.0,
        },
        ModelMetadata {
            name: "openai/gpt-4o-mini",
            provider: Provider::OpenRouter,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.15,
            output_price_per_million: 0.60,
        },
        ModelMetadata {
            name: "anthropic/claude-sonnet-4",
            provider: Provider::OpenRouter,
            context_length: 200_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 3.0,
            output_price_per_million: 15.0,
        },
        ModelMetadata {
            name: "anthropic/claude-opus-4",
            provider: Provider::OpenRouter,
            context_length: 200_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 15.0,
            output_price_per_million: 75.0,
        },
        ModelMetadata {
            name: "meta-llama/llama-3.1-70b",
            provider: Provider::OpenRouter,
            context_length: 131_072,
            max_output_tokens: 32_768,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.59,
            output_price_per_million: 0.79,
        },
        ModelMetadata {
            name: "meta-llama/llama-3.1-8b",
            provider: Provider::OpenRouter,
            context_length: 131_072,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.05,
            output_price_per_million: 0.08,
        },
        ModelMetadata {
            name: "google/gemini-2.0-flash",
            provider: Provider::OpenRouter,
            context_length: 1_048_576,
            max_output_tokens: 8_192,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.10,
            output_price_per_million: 0.40,
        },
        ModelMetadata {
            name: "deepseek/deepseek-chat",
            provider: Provider::OpenRouter,
            context_length: 64_000,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.14,
            output_price_per_million: 0.28,
        },
        // ----- Zai (Zhipu AI) -----
        ModelMetadata {
            name: "glm-4",
            provider: Provider::Zai,
            context_length: 128_000,
            max_output_tokens: 4_096,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.0,
            output_price_per_million: 1.0,
        },
        ModelMetadata {
            name: "glm-5",
            provider: Provider::Zai,
            context_length: 128_000,
            max_output_tokens: 4_096,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.0,
            output_price_per_million: 1.0,
        },
        // ----- Kimi (Moonshot AI) -----
        ModelMetadata {
            name: "kimi-k2.5",
            provider: Provider::Kimi,
            context_length: 131_072,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 2.0,
            output_price_per_million: 2.0,
        },
        ModelMetadata {
            name: "moonshot-v1-128k",
            provider: Provider::Kimi,
            context_length: 131_072,
            max_output_tokens: 4_096,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 8.0,
            output_price_per_million: 8.0,
        },
        // ----- MiniMax -----
        ModelMetadata {
            name: "MiniMax-M2.7",
            provider: Provider::MiniMax,
            context_length: 245_760,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 1.10,
            output_price_per_million: 4.40,
        },
        ModelMetadata {
            name: "abab6.5s-chat",
            provider: Provider::MiniMax,
            context_length: 245_760,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.50,
            output_price_per_million: 1.0,
        },
        // ----- Azure -----
        ModelMetadata {
            name: "azure/gpt-4o",
            provider: Provider::Azure,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 2.50,
            output_price_per_million: 10.0,
        },
        ModelMetadata {
            name: "azure/gpt-4o-mini",
            provider: Provider::Azure,
            context_length: 128_000,
            max_output_tokens: 16_384,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.15,
            output_price_per_million: 0.60,
        },
        // ----- HuggingFace -----
        ModelMetadata {
            name: "Qwen/Qwen3.5-397B-A17B",
            provider: Provider::HuggingFace,
            context_length: 131_072,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
        ModelMetadata {
            name: "meta-llama/Llama-3.1-70B-Instruct",
            provider: Provider::HuggingFace,
            context_length: 131_072,
            max_output_tokens: 8_192,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_price_per_million: 0.0,
            output_price_per_million: 0.0,
        },
    ]
});

// =============================================================================
// Lookup helpers
// =============================================================================

/// Look up model metadata by name. Case-insensitive. Also matches partial
/// prefixes (e.g. "gpt-4o" matches the "gpt-4o" entry but not "gpt-4o-mini").
pub fn get_model_metadata(name: &str) -> Option<&'static ModelMetadata> {
    let lower = name.to_lowercase();

    // Exact match (case-insensitive)
    if let Some(m) = MODEL_REGISTRY
        .iter()
        .find(|m| m.name.to_lowercase() == lower)
    {
        return Some(m);
    }

    // Partial prefix match: query must match start of model name and the
    // next char (if any) must not be alphanumeric, preventing "gpt-4o"
    // from matching "gpt-4o-mini".
    MODEL_REGISTRY.iter().find(|m| {
        let mn = m.name.to_lowercase();
        if mn.starts_with(&lower) {
            mn.len() == lower.len()
                || !mn.as_bytes()[lower.len()].is_ascii_alphanumeric()
        } else {
            false
        }
    })
}

/// List all models for a given provider.
pub fn list_models_by_provider(provider: &Provider) -> Vec<&'static ModelMetadata> {
    MODEL_REGISTRY
        .iter()
        .filter(|m| &m.provider == provider)
        .collect()
}

/// Estimate the cost in USD for a given model and token counts.
/// Returns `None` if the model is not found.
pub fn estimate_cost(model: &str, input_tokens: u32, output_tokens: u32) -> Option<f64> {
    let meta = get_model_metadata(model)?;
    let cost = (input_tokens as f64 / 1_000_000.0) * meta.input_price_per_million
        + (output_tokens as f64 / 1_000_000.0) * meta.output_price_per_million;
    Some(cost)
}

/// Get the context length for a model. Returns `None` if unknown.
pub fn get_context_length(model: &str) -> Option<u32> {
    get_model_metadata(model).map(|m| m.context_length)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_50_plus_models() {
        assert!(
            MODEL_REGISTRY.len() >= 50,
            "Expected 50+ models, got {}",
            MODEL_REGISTRY.len()
        );
    }

    #[test]
    fn test_lookup_exact_name() {
        let m = get_model_metadata("gpt-4o").expect("gpt-4o should exist");
        assert_eq!(m.name, "gpt-4o");
        assert_eq!(m.provider, Provider::OpenAI);
        assert_eq!(m.context_length, 128_000);

        let m = get_model_metadata("claude-sonnet-4-20250514")
            .expect("claude-sonnet-4 should exist");
        assert_eq!(m.provider, Provider::Anthropic);
    }

    #[test]
    fn test_lookup_case_insensitive() {
        let m = get_model_metadata("GPT-4O").expect("case-insensitive");
        assert_eq!(m.name, "gpt-4o");

        let m = get_model_metadata("DEEPSEEK-CHAT").expect("case-insensitive");
        assert_eq!(m.name, "deepseek-chat");
    }

    #[test]
    fn test_list_by_provider() {
        let openai = list_models_by_provider(&Provider::OpenAI);
        assert!(openai.len() >= 5);
        assert!(openai.iter().all(|m| m.provider == Provider::OpenAI));

        let ollama = list_models_by_provider(&Provider::Ollama);
        assert!(ollama.len() >= 5);
    }

    #[test]
    fn test_cost_estimation() {
        // gpt-4o: $2.50/1M input, $10/1M output
        let cost = estimate_cost("gpt-4o", 1_000_000, 1_000_000)
            .expect("cost");
        assert!((cost - 12.50).abs() < 0.001);

        // Free model
        let cost = estimate_cost("llama3", 1_000_000, 1_000_000)
            .expect("cost");
        assert!((cost - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_context_length() {
        assert_eq!(get_context_length("gpt-4o"), Some(128_000));
        assert_eq!(get_context_length("o1"), Some(200_000));
        assert_eq!(get_context_length("gemini-2.5-pro"), Some(1_048_576));
        assert_eq!(get_context_length("gemini-1.5-pro"), Some(2_097_152));
        assert_eq!(get_context_length("deepseek-chat"), Some(64_000));
    }

    #[test]
    fn test_unknown_model_returns_none() {
        assert!(get_model_metadata("nonexistent-xyz").is_none());
        assert!(estimate_cost("nonexistent-xyz", 100, 100).is_none());
        assert!(get_context_length("nonexistent-xyz").is_none());
    }

    #[test]
    fn test_partial_name_does_not_match_longer() {
        let m = get_model_metadata("gpt-4o").expect("should match");
        assert_eq!(m.name, "gpt-4o");
    }

    #[test]
    fn test_openrouter_models() {
        let m = get_model_metadata("openai/gpt-4o").expect("openrouter");
        assert_eq!(m.provider, Provider::OpenRouter);

        let m = get_model_metadata("anthropic/claude-sonnet-4")
            .expect("openrouter");
        assert_eq!(m.provider, Provider::OpenRouter);
    }

    #[test]
    fn test_pricing_non_negative() {
        for m in MODEL_REGISTRY.iter() {
            assert!(m.input_price_per_million >= 0.0);
            assert!(m.output_price_per_million >= 0.0);
        }
    }
}
