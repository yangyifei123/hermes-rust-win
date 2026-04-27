//! Hermes Common Types
//!
//! Shared types for providers, models, credentials, and session management.

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Provider
// =============================================================================

/// AI provider enum matching all Hermes-supported providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    // OpenAI-compatible providers
    OpenAI,
    Anthropic,
    OpenRouter,
    Ollama,
    Azure,
    // Google
    Gemini,
    // Chinese providers
    Zai,    // Zhipu AI (GLM)
    Kimi,   // Moonshot AI
    KimiCn, // Moonshot AI (China endpoint)
    MiniMax,
    MiniMaxCn,
    // Other providers
    Arcee,
    AiGateway,
    Kilocode,
    OpenCodeZen,
    OpenCodeGo,
    Copilot,
    CopilotAcp,
    HuggingFace,
    DeepSeek,
    // Custom
    Custom,
    // Groq - Ultra-fast inference
    Groq,
}

impl Provider {
    /// Parse a provider name string into a Provider enum.
    /// Case-insensitive, supports both kebab-case and snake_case.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Provider::OpenAI),
            "anthropic" => Some(Provider::Anthropic),
            "openrouter" => Some(Provider::OpenRouter),
            "ollama" => Some(Provider::Ollama),
            "azure" | "azure-openai" => Some(Provider::Azure),
            "gemini" | "google" => Some(Provider::Gemini),
            "zai" | "zhipu" | "glm" => Some(Provider::Zai),
            "kimi" | "moonshot" => Some(Provider::Kimi),
            "kimi-cn" | "moonshot-cn" => Some(Provider::KimiCn),
            "minimax" => Some(Provider::MiniMax),
            "minimax-cn" => Some(Provider::MiniMaxCn),
            "arcee" => Some(Provider::Arcee),
            "ai-gateway" | "aigateway" => Some(Provider::AiGateway),
            "kilocode" => Some(Provider::Kilocode),
            "opencode-zen" | "opencodezen" => Some(Provider::OpenCodeZen),
            "opencode-go" | "opencodego" => Some(Provider::OpenCodeGo),
            "copilot" => Some(Provider::Copilot),
            "copilot-acp" | "copilotacp" => Some(Provider::CopilotAcp),
            "huggingface" | "hf" => Some(Provider::HuggingFace),
            "deepseek" => Some(Provider::DeepSeek),
            "groq" => Some(Provider::Groq),
            "custom" => Some(Provider::Custom),
            _ => None,
        }
    }

    /// Get the canonical kebab-case name for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::OpenAI => "openai",
            Provider::Anthropic => "anthropic",
            Provider::OpenRouter => "openrouter",
            Provider::Ollama => "ollama",
            Provider::Azure => "azure",
            Provider::Gemini => "gemini",
            Provider::Zai => "zai",
            Provider::Kimi => "kimi",
            Provider::KimiCn => "kimi-cn",
            Provider::MiniMax => "minimax",
            Provider::MiniMaxCn => "minimax-cn",
            Provider::Arcee => "arcee",
            Provider::AiGateway => "ai-gateway",
            Provider::Kilocode => "kilocode",
            Provider::OpenCodeZen => "opencode-zen",
            Provider::OpenCodeGo => "opencode-go",
            Provider::Copilot => "copilot",
            Provider::CopilotAcp => "copilot-acp",
            Provider::HuggingFace => "huggingface",
            Provider::DeepSeek => "deepseek",
            Provider::Groq => "groq",
            Provider::Custom => "custom",
        }
    }

    /// Get the default model for this provider.
    pub fn default_model(&self) -> &'static str {
        match self {
            Provider::OpenAI => "gpt-4o",
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::OpenRouter => "openai/gpt-4o",
            Provider::Ollama => "llama3",
            Provider::Azure => "gpt-4o",
            Provider::Gemini => "gemini-2.5-pro",
            Provider::Zai => "glm-5",
            Provider::Kimi => "kimi-k2.5",
            Provider::KimiCn => "kimi-k2.5",
            Provider::MiniMax => "MiniMax-M2.7",
            Provider::MiniMaxCn => "MiniMax-M2.7",
            Provider::Arcee => "trinity-large-thinking",
            Provider::AiGateway => "anthropic/claude-sonnet-4-6",
            Provider::Kilocode => "anthropic/claude-sonnet-4-6",
            Provider::OpenCodeZen => "gpt-5.4",
            Provider::OpenCodeGo => "glm-5",
            Provider::Copilot => "gpt-5.4",
            Provider::CopilotAcp => "copilot-acp",
            Provider::HuggingFace => "Qwen/Qwen3.5-397B-A17B",
            Provider::DeepSeek => "deepseek-chat",
            Provider::Groq => "llama-3.1-70b-versatile",
            Provider::Custom => "custom",
        }
    }

    /// Get the default base URL for this provider's API.
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Provider::OpenAI => "https://api.openai.com/v1",
            Provider::Anthropic => "https://api.anthropic.com/v1",
            Provider::OpenRouter => "https://openrouter.ai/api/v1",
            Provider::Ollama => "http://localhost:11434",
            Provider::Azure => "", // Requires custom endpoint
            Provider::Gemini => "https://generativelanguage.googleapis.com/v1beta",
            Provider::Zai => "https://open.bigmodel.cn/api/paas/v4",
            Provider::Kimi => "https://api.moonshot.cn/v1",
            Provider::KimiCn => "https://api.moonshot.cn/v1",
            Provider::MiniMax => "https://api.minimax.chat/v1",
            Provider::MiniMaxCn => "https://api.minimax.chat/v1",
            Provider::Arcee => "https://api.arcee.ai/v1",
            Provider::AiGateway => "https://aigateway.nousresearch.com/v1",
            Provider::Kilocode => "https://api.kilocode.ai/v1",
            Provider::OpenCodeZen => "https://api.opencode.ai/v1",
            Provider::OpenCodeGo => "https://api.opencode.ai/v1",
            Provider::Copilot => "",    // Copilot uses its own protocol
            Provider::CopilotAcp => "", // ACP uses its own protocol
            Provider::HuggingFace => "https://api-inference.huggingface.co/models",
            Provider::DeepSeek => "https://api.deepseek.com/v1",
            Provider::Groq => "https://api.groq.com/openai/v1",
            Provider::Custom => "",
        }
    }

    /// Get the environment variable name for this provider's API key.
    pub fn env_key(&self) -> &'static str {
        match self {
            Provider::OpenAI => "OPENAI_API_KEY",
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::OpenRouter => "OPENROUTER_API_KEY",
            Provider::Ollama => "", // No key needed
            Provider::Azure => "AZURE_OPENAI_API_KEY",
            Provider::Gemini => "GEMINI_API_KEY",
            Provider::Zai => "GLM_API_KEY",
            Provider::Kimi | Provider::KimiCn => "KIMI_API_KEY",
            Provider::MiniMax => "MINIMAX_API_KEY",
            Provider::MiniMaxCn => "MINIMAX_CN_API_KEY",
            Provider::Arcee => "ARCEE_API_KEY",
            Provider::AiGateway => "AI_GATEWAY_API_KEY",
            Provider::Kilocode => "KILOCODE_API_KEY",
            Provider::OpenCodeZen => "OPENCODE_ZEN_API_KEY",
            Provider::OpenCodeGo => "OPENCODE_GO_API_KEY",
            Provider::Copilot => "",    // Uses GitHub auth
            Provider::CopilotAcp => "", // Uses ACP protocol
            Provider::HuggingFace => "HF_TOKEN",
            Provider::DeepSeek => "DEEPSEEK_API_KEY",
            Provider::Groq => "GROQ_API_KEY",
            Provider::Custom => "",
        }
    }

    /// Authentication type required for this provider.
    pub fn auth_type(&self) -> AuthType {
        match self {
            Provider::Ollama => AuthType::None,
            Provider::Copilot | Provider::CopilotAcp => AuthType::OAuth,
            Provider::OpenRouter
            | Provider::AiGateway
            | Provider::Kilocode
            | Provider::OpenCodeZen
            | Provider::OpenCodeGo => AuthType::ApiKeyOrOAuth,
            _ => AuthType::ApiKey,
        }
    }

    /// Get all known provider names.
    pub fn all_providers() -> &'static [Provider] {
        &[
            Provider::OpenAI,
            Provider::Anthropic,
            Provider::OpenRouter,
            Provider::Ollama,
            Provider::Azure,
            Provider::Gemini,
            Provider::Zai,
            Provider::Kimi,
            Provider::MiniMax,
            Provider::Arcee,
            Provider::AiGateway,
            Provider::Kilocode,
            Provider::OpenCodeZen,
            Provider::OpenCodeGo,
            Provider::Copilot,
            Provider::CopilotAcp,
            Provider::HuggingFace,
            Provider::DeepSeek,
            Provider::Groq,
            Provider::Custom,
        ]
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// Auth Type
// =============================================================================

/// Authentication type for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthType {
    /// No authentication needed
    None,
    /// Simple API key
    ApiKey,
    /// OAuth device code flow
    OAuth,
    /// Either API key or OAuth
    ApiKeyOrOAuth,
}

impl fmt::Display for AuthType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthType::None => write!(f, "none"),
            AuthType::ApiKey => write!(f, "api_key"),
            AuthType::OAuth => write!(f, "oauth"),
            AuthType::ApiKeyOrOAuth => write!(f, "api_key_or_oauth"),
        }
    }
}

// =============================================================================
// Model
// =============================================================================

/// AI model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub provider: Provider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u32>,
}

impl Model {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            provider: Provider::OpenAI,
            base_url: None,
            context_length: None,
        }
    }

    pub fn with_provider(name: &str, provider: Provider) -> Self {
        Self {
            name: name.to_string(),
            provider,
            base_url: None,
            context_length: None,
        }
    }
}

// =============================================================================
// Session
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        use uuid::Timestamp;
        let ts = Timestamp::now(uuid::NoContext);
        Self(format!("session-{}", uuid::Uuid::new_v7(ts)))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Credentials
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub provider: String,
    pub api_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl Credentials {
    pub fn new(provider: &str, api_key: &str) -> Self {
        Self {
            provider: provider.to_string(),
            api_key: api_key.to_string(),
            base_url: None,
        }
    }

    /// Mask the API key for display, showing only last 4 characters.
    pub fn masked_api_key(&self) -> String {
        if self.api_key.len() <= 8 {
            return "*".repeat(self.api_key.len());
        }
        let visible = &self.api_key[self.api_key.len() - 4..];
        format!("{}{}", "*".repeat(self.api_key.len() - 4), visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(Provider::from_str("openai"), Some(Provider::OpenAI));
        assert_eq!(Provider::from_str("OpenAI"), Some(Provider::OpenAI));
        assert_eq!(Provider::from_str("OPENAI"), Some(Provider::OpenAI));
        assert_eq!(Provider::from_str("anthropic"), Some(Provider::Anthropic));
        assert_eq!(Provider::from_str("gemini"), Some(Provider::Gemini));
        assert_eq!(Provider::from_str("zai"), Some(Provider::Zai));
        assert_eq!(Provider::from_str("zhipu"), Some(Provider::Zai));
        assert_eq!(Provider::from_str("glm"), Some(Provider::Zai));
        assert_eq!(Provider::from_str("kimi"), Some(Provider::Kimi));
        assert_eq!(Provider::from_str("moonshot"), Some(Provider::Kimi));
        assert_eq!(Provider::from_str("deepseek"), Some(Provider::DeepSeek));
        assert_eq!(Provider::from_str("unknown"), None);
    }

    #[test]
    fn test_provider_as_str() {
        assert_eq!(Provider::OpenAI.as_str(), "openai");
        assert_eq!(Provider::Anthropic.as_str(), "anthropic");
        assert_eq!(Provider::OpenRouter.as_str(), "openrouter");
        assert_eq!(Provider::Gemini.as_str(), "gemini");
        assert_eq!(Provider::Zai.as_str(), "zai");
        assert_eq!(Provider::AiGateway.as_str(), "ai-gateway");
    }

    #[test]
    fn test_provider_default_model() {
        assert_eq!(Provider::OpenAI.default_model(), "gpt-4o");
        assert_eq!(
            Provider::Anthropic.default_model(),
            "claude-sonnet-4-20250514"
        );
        assert_eq!(Provider::Gemini.default_model(), "gemini-2.5-pro");
        assert_eq!(Provider::Zai.default_model(), "glm-5");
    }

    #[test]
    fn test_provider_default_base_url() {
        assert_eq!(
            Provider::OpenAI.default_base_url(),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            Provider::Anthropic.default_base_url(),
            "https://api.anthropic.com/v1"
        );
        assert_eq!(
            Provider::Ollama.default_base_url(),
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_provider_env_key() {
        assert_eq!(Provider::OpenAI.env_key(), "OPENAI_API_KEY");
        assert_eq!(Provider::Anthropic.env_key(), "ANTHROPIC_API_KEY");
        assert_eq!(Provider::Ollama.env_key(), "");
        assert_eq!(Provider::HuggingFace.env_key(), "HF_TOKEN");
    }

    #[test]
    fn test_provider_auth_type() {
        assert_eq!(Provider::OpenAI.auth_type(), AuthType::ApiKey);
        assert_eq!(Provider::Ollama.auth_type(), AuthType::None);
        assert_eq!(Provider::Copilot.auth_type(), AuthType::OAuth);
        assert_eq!(Provider::OpenRouter.auth_type(), AuthType::ApiKeyOrOAuth);
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(format!("{}", Provider::OpenAI), "openai");
        assert_eq!(format!("{}", Provider::Gemini), "gemini");
    }

    #[test]
    fn test_all_providers() {
        let providers = Provider::all_providers();
        assert!(providers.len() >= 15);
        assert!(providers.contains(&Provider::OpenAI));
        assert!(providers.contains(&Provider::Anthropic));
    }

    #[test]
    fn test_auth_type_display() {
        assert_eq!(format!("{}", AuthType::ApiKey), "api_key");
        assert_eq!(format!("{}", AuthType::None), "none");
        assert_eq!(format!("{}", AuthType::OAuth), "oauth");
    }

    #[test]
    fn test_model_new() {
        let m = Model::new("gpt-4");
        assert_eq!(m.name, "gpt-4");
        assert_eq!(m.provider, Provider::OpenAI);
    }

    #[test]
    fn test_model_with_provider() {
        let m = Model::with_provider("claude-3", Provider::Anthropic);
        assert_eq!(m.name, "claude-3");
        assert_eq!(m.provider, Provider::Anthropic);
    }

    #[test]
    fn test_credentials_masked() {
        // "sk-1234567890abcdef" = 19 chars, show last 4, mask = 15 stars + "cdef" = 19
        let c = Credentials::new("openai", "sk-1234567890abcdef");
        let masked = c.masked_api_key();
        assert!(masked.ends_with("cdef"));
        assert!(masked.starts_with('*'));
        assert_eq!(masked.len(), 19);
    }

    #[test]
    fn test_credentials_masked_short() {
        let c = Credentials::new("test", "abc");
        assert_eq!(c.masked_api_key(), "***");
    }

    #[test]
    fn test_session_id_new() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert!(id1.0.starts_with("session-"));
        assert_ne!(id1.0, id2.0);
    }
}
