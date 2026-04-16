use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
    OpenRouter,
    Ollama,
    Azure,
    Custom,
}

impl Provider {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Provider::OpenAI),
            "anthropic" => Some(Provider::Anthropic),
            "openrouter" => Some(Provider::OpenRouter),
            "ollama" => Some(Provider::Ollama),
            "azure" => Some(Provider::Azure),
            "custom" => Some(Provider::Custom),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub provider: Provider,
    pub base_url: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(format!("session-{}", uuid_simple()))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub provider: String,
    pub api_key: String,
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
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}