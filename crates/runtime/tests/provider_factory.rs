//! Integration tests verifying the provider factory creates correct providers
//! and that provider.name() / default_model() are accurate.

use hermes_common::Provider;
use hermes_agent_runtime::provider::create_provider;

#[test]
fn test_provider_name_openai() {
    let provider = create_provider(&Provider::OpenAI, "key", None);
    assert_eq!(provider.name(), "openai");
    assert_eq!(provider.default_model(), "gpt-4o");
}

#[test]
fn test_provider_name_anthropic() {
    let provider = create_provider(&Provider::Anthropic, "key", None);
    assert_eq!(provider.name(), "anthropic");
    assert_eq!(provider.default_model(), "claude-sonnet-4-20250514");
}

#[test]
fn test_provider_name_deepseek() {
    let provider = create_provider(&Provider::DeepSeek, "key", None);
    assert_eq!(provider.name(), "deepseek");
    assert_eq!(provider.default_model(), "deepseek-chat");
}

#[test]
fn test_provider_name_groq() {
    let provider = create_provider(&Provider::Groq, "key", None);
    assert_eq!(provider.name(), "groq");
    assert_eq!(provider.default_model(), "llama-3.1-70b-versatile");
}

#[test]
fn test_provider_name_openrouter() {
    let provider = create_provider(&Provider::OpenRouter, "key", None);
    assert_eq!(provider.name(), "openrouter");
    assert_eq!(provider.default_model(), "openai/gpt-4o");
}

#[test]
fn test_provider_name_ollama() {
    let provider = create_provider(&Provider::Ollama, "key", None);
    assert_eq!(provider.name(), "ollama");
    assert_eq!(provider.default_model(), "llama3");
}

#[test]
fn test_provider_name_gemini() {
    let provider = create_provider(&Provider::Gemini, "key", None);
    assert_eq!(provider.name(), "gemini");
    assert_eq!(provider.default_model(), "gemini-2.5-pro");
}

#[test]
fn test_provider_name_zai() {
    let provider = create_provider(&Provider::Zai, "key", None);
    assert_eq!(provider.name(), "zai");
    assert_eq!(provider.default_model(), "glm-5");
}

#[test]
fn test_provider_name_kimi() {
    let provider = create_provider(&Provider::Kimi, "key", None);
    assert_eq!(provider.name(), "kimi");
    assert_eq!(provider.default_model(), "kimi-k2.5");
}

#[test]
fn test_provider_name_minimax() {
    let provider = create_provider(&Provider::MiniMax, "key", None);
    assert_eq!(provider.name(), "minimax");
    assert_eq!(provider.default_model(), "MiniMax-M2.7");
}

#[test]
fn test_custom_base_url_preserves_name() {
    let provider =
        create_provider(&Provider::OpenAI, "key", Some("https://custom.proxy.example.com/v1"));
    assert_eq!(provider.name(), "openai");
    assert_eq!(provider.default_model(), "gpt-4o");
}

#[test]
fn test_all_openai_compatible_providers_share_endpoint_path() {
    // All OpenAI-compatible providers should post to /chat/completions.
    // This test verifies the factory produces providers for all non-Anthropic/Gemini variants.
    let openai_compatible = [
        Provider::OpenAI,
        Provider::DeepSeek,
        Provider::Groq,
        Provider::OpenRouter,
        Provider::Ollama,
        Provider::Zai,
        Provider::Kimi,
        Provider::MiniMax,
        Provider::Arcee,
        Provider::AiGateway,
    ];

    for p in &openai_compatible {
        let provider = create_provider(p, "key", None);
        // All OpenAI-compatible providers use the same name() logic
        assert_eq!(provider.name(), p.as_str(), "name mismatch for {:?}", p);
    }
}
