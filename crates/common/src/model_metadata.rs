use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub name: String,
    pub provider: String,
    pub context_length: u32,
    pub max_output_tokens: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub input_price_per_1k: f64,
    pub output_price_per_1k: f64,
}

pub struct ModelMetadataRegistry {
    models: HashMap<String, ModelMetadata>,
}

impl Default for ModelMetadataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelMetadataRegistry {
    pub fn new() -> Self {
        let mut m = HashMap::new();
        m.insert(
            "gpt-4o".into(),
            ModelMetadata {
                name: "gpt-4o".into(),
                provider: "openai".into(),
                context_length: 128000,
                max_output_tokens: 16384,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.0025,
                output_price_per_1k: 0.01,
            },
        );
        m.insert(
            "gpt-4o-mini".into(),
            ModelMetadata {
                name: "gpt-4o-mini".into(),
                provider: "openai".into(),
                context_length: 128000,
                max_output_tokens: 16384,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.00015,
                output_price_per_1k: 0.0006,
            },
        );
        m.insert(
            "o1".into(),
            ModelMetadata {
                name: "o1".into(),
                provider: "openai".into(),
                context_length: 200000,
                max_output_tokens: 100000,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.015,
                output_price_per_1k: 0.06,
            },
        );
        m.insert(
            "o1-mini".into(),
            ModelMetadata {
                name: "o1-mini".into(),
                provider: "openai".into(),
                context_length: 128000,
                max_output_tokens: 65536,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.003,
                output_price_per_1k: 0.012,
            },
        );
        m.insert(
            "gpt-3.5-turbo".into(),
            ModelMetadata {
                name: "gpt-3.5-turbo".into(),
                provider: "openai".into(),
                context_length: 16385,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.0005,
                output_price_per_1k: 0.0015,
            },
        );
        m.insert(
            "claude-sonnet-4-20250514".into(),
            ModelMetadata {
                name: "claude-sonnet-4-20250514".into(),
                provider: "anthropic".into(),
                context_length: 200000,
                max_output_tokens: 16384,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.003,
                output_price_per_1k: 0.015,
            },
        );
        m.insert(
            "claude-3-5-haiku-20241022".into(),
            ModelMetadata {
                name: "claude-3-5-haiku-20241022".into(),
                provider: "anthropic".into(),
                context_length: 200000,
                max_output_tokens: 8192,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.001,
                output_price_per_1k: 0.005,
            },
        );
        m.insert(
            "claude-opus-4-20250514".into(),
            ModelMetadata {
                name: "claude-opus-4-20250514".into(),
                provider: "anthropic".into(),
                context_length: 200000,
                max_output_tokens: 16384,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.015,
                output_price_per_1k: 0.075,
            },
        );
        m.insert(
            "gemini-2.0-flash".into(),
            ModelMetadata {
                name: "gemini-2.0-flash".into(),
                provider: "google".into(),
                context_length: 1048576,
                max_output_tokens: 8192,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.0,
                output_price_per_1k: 0.0,
            },
        );
        m.insert(
            "deepseek-chat".into(),
            ModelMetadata {
                name: "deepseek-chat".into(),
                provider: "deepseek".into(),
                context_length: 64000,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.00014,
                output_price_per_1k: 0.00028,
            },
        );
        m.insert(
            "deepseek-reasoner".into(),
            ModelMetadata {
                name: "deepseek-reasoner".into(),
                provider: "deepseek".into(),
                context_length: 64000,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.00055,
                output_price_per_1k: 0.00219,
            },
        );
        m.insert(
            "llama-3.1-70b-versatile".into(),
            ModelMetadata {
                name: "llama-3.1-70b-versatile".into(),
                provider: "groq".into(),
                context_length: 131072,
                max_output_tokens: 32768,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.0,
                output_price_per_1k: 0.0,
            },
        );
        m.insert(
            "mixtral-8x7b-32768".into(),
            ModelMetadata {
                name: "mixtral-8x7b-32768".into(),
                provider: "groq".into(),
                context_length: 32768,
                max_output_tokens: 32768,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.0,
                output_price_per_1k: 0.0,
            },
        );
        m.insert(
            "llama3".into(),
            ModelMetadata {
                name: "llama3".into(),
                provider: "ollama".into(),
                context_length: 8192,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_tools: false,
                supports_streaming: true,
                input_price_per_1k: 0.0,
                output_price_per_1k: 0.0,
            },
        );
        m.insert(
            "mistral".into(),
            ModelMetadata {
                name: "mistral".into(),
                provider: "ollama".into(),
                context_length: 32768,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_tools: false,
                supports_streaming: true,
                input_price_per_1k: 0.0,
                output_price_per_1k: 0.0,
            },
        );
        m.insert(
            "codellama".into(),
            ModelMetadata {
                name: "codellama".into(),
                provider: "ollama".into(),
                context_length: 16384,
                max_output_tokens: 4096,
                supports_vision: false,
                supports_tools: false,
                supports_streaming: true,
                input_price_per_1k: 0.0,
                output_price_per_1k: 0.0,
            },
        );
        m.insert(
            "glm-4".into(),
            ModelMetadata {
                name: "glm-4".into(),
                provider: "zhipu".into(),
                context_length: 128000,
                max_output_tokens: 4096,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.001,
                output_price_per_1k: 0.001,
            },
        );
        m.insert(
            "glm-5".into(),
            ModelMetadata {
                name: "glm-5".into(),
                provider: "zhipu".into(),
                context_length: 128000,
                max_output_tokens: 4096,
                supports_vision: true,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.001,
                output_price_per_1k: 0.001,
            },
        );
        m.insert(
            "kimi-k2.5".into(),
            ModelMetadata {
                name: "kimi-k2.5".into(),
                provider: "moonshot".into(),
                context_length: 131072,
                max_output_tokens: 8192,
                supports_vision: false,
                supports_tools: true,
                supports_streaming: true,
                input_price_per_1k: 0.002,
                output_price_per_1k: 0.002,
            },
        );
        Self { models: m }
    }

    pub fn get(&self, name: &str) -> Option<&ModelMetadata> {
        self.models.get(name)
    }

    pub fn estimate_cost(&self, input_tokens: u32, output_tokens: u32, model: &str) -> f64 {
        if let Some(m) = self.models.get(model) {
            (input_tokens as f64 / 1000.0 * m.input_price_per_1k)
                + (output_tokens as f64 / 1000.0 * m.output_price_per_1k)
        } else {
            0.0
        }
    }

    pub fn get_context_length(&self, model: &str) -> u32 {
        self.models
            .get(model)
            .map(|m| m.context_length)
            .unwrap_or(128000)
    }

    pub fn list(&self) -> Vec<&ModelMetadata> {
        let mut v: Vec<_> = self.models.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }

    pub fn list_by_provider(&self, provider: &str) -> Vec<&ModelMetadata> {
        let mut v: Vec<_> = self
            .models
            .values()
            .filter(|m| m.provider == provider)
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}
