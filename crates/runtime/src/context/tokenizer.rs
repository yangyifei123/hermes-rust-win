//! Tokenizer implementations for accurate token counting.
//!
//! Provides model-specific tokenizers (via tiktoken-rs for OpenAI models)
//! and heuristic-based fallbacks for other models.

use crate::provider::ChatMessage;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Tokenizer trait
// ---------------------------------------------------------------------------

/// Trait for token counting implementations.
pub trait Tokenizer: Send + Sync {
    /// Count tokens in a plain text string.
    fn count_tokens(&self, text: &str) -> usize;

    /// Count tokens in a slice of chat messages, including overhead.
    fn count_messages(&self, messages: &[ChatMessage]) -> usize;

    /// Return the model name this tokenizer is for.
    fn model_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// TiktokenTokenizer (OpenAI models)
// ---------------------------------------------------------------------------

/// Tokenizer backed by tiktoken-rs for accurate OpenAI-compatible counts.
pub struct TiktokenTokenizer {
    encoder: tiktoken_rs::CoreBPE,
    model: String,
}

impl TiktokenTokenizer {
    /// Create a new tokenizer for the given OpenAI model name.
    ///
    /// Falls back to the "gpt-4" encoding if the model name is not recognised.
    pub fn new(model: impl Into<String>) -> Self {
        let model = model.into();
        let encoder = tiktoken_rs::get_bpe_from_model(&model).unwrap_or_else(|_| {
            tiktoken_rs::get_bpe_from_model("gpt-4").expect("gpt-4 encoding must exist")
        });
        Self { encoder, model }
    }
}

impl Tokenizer for TiktokenTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.encoder.encode_ordinary(text).len()
    }

    fn count_messages(&self, messages: &[ChatMessage]) -> usize {
        messages
            .iter()
            .map(|msg| {
                let content_tokens = self.count_tokens(msg.text());
                let tool_tokens: usize = msg
                    .tool_calls
                    .as_ref()
                    .map(|tcs| {
                        tcs.iter()
                            .map(|tc| {
                                self.count_tokens(&tc.function.arguments)
                                    + self.count_tokens(&tc.function.name)
                            })
                            .sum()
                    })
                    .unwrap_or(0);
                // OpenAI-style message overhead: ~4 tokens per message + role tokens
                content_tokens + tool_tokens + 4
            })
            .sum()
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

// ---------------------------------------------------------------------------
// HeuristicTokenizer (fallback for unknown / non-OpenAI models)
// ---------------------------------------------------------------------------

/// Language-aware heuristic tokenizer.
pub struct HeuristicTokenizer {
    chars_per_token: f64, // ~4 for English, ~2 for Chinese
}

impl HeuristicTokenizer {
    /// Create a heuristic tokenizer with the given ratio.
    pub fn new(chars_per_token: f64) -> Self {
        Self { chars_per_token }
    }

    /// Detect whether text is predominantly CJK.
    fn is_cjk_heavy(text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let cjk_count = text.chars().filter(|c| is_cjk(*c)).count();
        // Consider CJK-heavy if >30% of characters are CJK
        cjk_count as f64 / text.chars().count() as f64 > 0.3
    }
}

impl Default for HeuristicTokenizer {
    fn default() -> Self {
        // Default ~4 chars per token (English average)
        Self::new(4.0)
    }
}

impl Tokenizer for HeuristicTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        let ratio = if Self::is_cjk_heavy(text) {
            2.0
        } else {
            self.chars_per_token
        };
        (text.len() as f64 / ratio).ceil() as usize
    }

    fn count_messages(&self, messages: &[ChatMessage]) -> usize {
        messages
            .iter()
            .map(|msg| {
                let content_tokens = self.count_tokens(msg.text());
                let tool_tokens: usize = msg
                    .tool_calls
                    .as_ref()
                    .map(|tcs| {
                        tcs.iter()
                            .map(|tc| {
                                self.count_tokens(&tc.function.arguments)
                                    + self.count_tokens(&tc.function.name)
                            })
                            .sum()
                    })
                    .unwrap_or(0);
                content_tokens + tool_tokens + 4
            })
            .sum()
    }

    fn model_name(&self) -> &str {
        "heuristic"
    }
}

/// Returns true if the character is a CJK (Chinese, Japanese, Korean) character.
fn is_cjk(c: char) -> bool {
    matches!(
        c,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{2E80}'..='\u{2EFF}' // CJK Radicals Supplement
        | '\u{3000}'..='\u{303F}' // CJK Symbols and Punctuation
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{FF00}'..='\u{FFEF}' // Fullwidth forms
    )
}

// ---------------------------------------------------------------------------
// TokenizerRegistry
// ---------------------------------------------------------------------------

/// Registry that maps model names to their appropriate tokenizers.
pub struct TokenizerRegistry {
    tokenizers: HashMap<String, Box<dyn Tokenizer>>,
    fallback: Box<dyn Tokenizer>,
}

impl Default for TokenizerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenizerRegistry {
    /// Create a registry pre-populated with known model mappings.
    pub fn new() -> Self {
        let mut tokenizers: HashMap<String, Box<dyn Tokenizer>> = HashMap::new();

        // OpenAI models → TiktokenTokenizer
        for model in &[
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4",
            "gpt-4-turbo",
            "gpt-3.5-turbo",
            "gpt-3.5-turbo-16k",
        ] {
            tokenizers.insert(model.to_string(), Box::new(TiktokenTokenizer::new(*model)));
        }

        // Anthropic models → heuristic (~3.5 chars per token)
        for model in &[
            "claude-sonnet-4-20250514",
            "claude-3-5-sonnet",
            "claude-3-opus",
            "claude-3-haiku",
            "claude-3-sonnet",
        ] {
            tokenizers.insert(model.to_string(), Box::new(HeuristicTokenizer::new(3.5)));
        }

        Self {
            tokenizers,
            fallback: Box::new(HeuristicTokenizer::default()),
        }
    }

    /// Get the tokenizer for a given model name.
    pub fn for_model(&self, model: &str) -> &dyn Tokenizer {
        self.tokenizers
            .get(model)
            .map(|b| b.as_ref())
            .unwrap_or(self.fallback.as_ref())
    }

    /// Register a custom tokenizer for a model name.
    pub fn register(&mut self, model: impl Into<String>, tokenizer: Box<dyn Tokenizer>) {
        self.tokenizers.insert(model.into(), tokenizer);
    }

    /// Count tokens for text using the appropriate tokenizer for the model.
    pub fn count_tokens(&self, model: &str, text: &str) -> usize {
        self.for_model(model).count_tokens(text)
    }

    /// Count tokens for messages using the appropriate tokenizer for the model.
    pub fn count_messages(&self, model: &str, messages: &[ChatMessage]) -> usize {
        self.for_model(model).count_messages(messages)
    }

    /// Truncate messages to fit within `max_tokens` using the model's tokenizer.
    ///
    /// Strategy:
    /// 1. Always keep the **system message** (first message if role is "system").
    /// 2. Keep the **last N messages** from the tail, working backwards,
    ///    until the total would exceed `max_tokens`.
    /// 3. Never drop below `MIN_KEEP_MESSAGES` recent messages.
    pub fn truncate_messages(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        max_tokens: usize,
    ) -> Vec<ChatMessage> {
        if messages.is_empty() {
            return messages;
        }

        let tokenizer = self.for_model(model);
        let total = tokenizer.count_messages(&messages);
        if total <= max_tokens {
            return messages;
        }

        // Separate system message if present.
        let (system, rest) = if messages.first().map(|m| m.role.as_str()) == Some("system") {
            let sys = messages[0].clone();
            (Some(sys), messages[1..].to_vec())
        } else {
            (None, messages)
        };

        let system_tokens = system
            .as_ref()
            .map(|s| tokenizer.count_tokens(s.text()) + 4)
            .unwrap_or(0);

        let budget = max_tokens.saturating_sub(system_tokens);

        // Walk backwards from the end, accumulating messages until budget is exhausted.
        let mut kept: Vec<ChatMessage> = Vec::new();
        let mut used = 0usize;

        for msg in rest.into_iter().rev() {
            let cost = tokenizer.count_tokens(msg.text()) + 4;
            if used + cost > budget && kept.len() >= 2 {
                break;
            }
            used += cost;
            kept.push(msg);
        }

        // Reverse to restore chronological order.
        kept.reverse();

        // Prepend system message.
        if let Some(sys) = system {
            kept.insert(0, sys);
        }

        kept
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> ChatMessage {
        match role {
            "system" => ChatMessage::system(content),
            "user" => ChatMessage::user(content),
            "assistant" => ChatMessage::assistant(content),
            _ => ChatMessage::user(content),
        }
    }

    // ---- TiktokenTokenizer ----

    #[test]
    fn test_tiktoken_counts_english() {
        let t = TiktokenTokenizer::new("gpt-4");
        let tokens = t.count_tokens("Hello world");
        // "Hello world" is 3 tokens with cl100k_base
        assert!(tokens > 0 && tokens < 10, "got {} tokens", tokens);
    }

    #[test]
    fn test_tiktoken_counts_empty() {
        let t = TiktokenTokenizer::new("gpt-4");
        assert_eq!(t.count_tokens(""), 0);
    }

    #[test]
    fn test_tiktoken_model_name() {
        let t = TiktokenTokenizer::new("gpt-4o");
        assert_eq!(t.model_name(), "gpt-4o");
    }

    // ---- HeuristicTokenizer ----

    #[test]
    fn test_heuristic_english() {
        let t = HeuristicTokenizer::new(4.0);
        // 12 chars / 4 = 3 tokens
        assert_eq!(t.count_tokens("Hello world!"), 3);
    }

    #[test]
    fn test_heuristic_cjk() {
        let t = HeuristicTokenizer::default();
        // 4 CJK chars, 3 bytes each in UTF-8 = 12 bytes / 2 = 6 tokens (using byte len)
        assert_eq!(t.count_tokens("你好世界"), 6);
    }

    #[test]
    fn test_heuristic_cjk_mixed() {
        let t = HeuristicTokenizer::default();
        // Mixed text with some CJK but not >30% CJK ratio
        let tokens = t.count_tokens("Hello 你好");
        // 10 chars / 4 = 3 tokens (not CJK-heavy)
        assert_eq!(tokens, 3);
    }

    #[test]
    fn test_heuristic_empty() {
        let t = HeuristicTokenizer::default();
        assert_eq!(t.count_tokens(""), 0);
    }

    #[test]
    fn test_heuristic_model_name() {
        let t = HeuristicTokenizer::default();
        assert_eq!(t.model_name(), "heuristic");
    }

    // ---- TokenizerRegistry ----

    #[test]
    fn test_registry_openai_model() {
        let registry = TokenizerRegistry::new();
        let t = registry.for_model("gpt-4");
        assert_eq!(t.model_name(), "gpt-4");
    }

    #[test]
    fn test_registry_anthropic_model() {
        let registry = TokenizerRegistry::new();
        let t = registry.for_model("claude-sonnet-4-20250514");
        assert_eq!(t.model_name(), "heuristic");
    }

    #[test]
    fn test_registry_unknown_model_fallback() {
        let registry = TokenizerRegistry::new();
        let t = registry.for_model("some-random-model");
        assert_eq!(t.model_name(), "heuristic");
    }

    #[test]
    fn test_registry_count_tokens() {
        let registry = TokenizerRegistry::new();
        let tokens = registry.count_tokens("gpt-4", "Hello world");
        assert!(tokens > 0);
    }

    #[test]
    fn test_registry_truncate_noop_when_under_limit() {
        let registry = TokenizerRegistry::new();
        let messages = vec![msg("user", "hello"), msg("assistant", "hi there")];
        let total = registry.count_messages("gpt-4", &messages);
        let truncated = registry.truncate_messages("gpt-4", messages.clone(), total + 1000);
        assert_eq!(truncated.len(), messages.len());
    }

    #[test]
    fn test_registry_truncate_preserves_system_message() {
        let registry = TokenizerRegistry::new();
        let mut messages = vec![msg("system", "You are a helpful assistant.")];
        for i in 0..50 {
            messages.push(msg(
                "user",
                &format!(
                    "Message number {} with some padding text to increase token count.",
                    i
                ),
            ));
            messages.push(msg(
                "assistant",
                &format!("Response number {} with some padding text.", i),
            ));
        }
        let truncated = registry.truncate_messages("gpt-4", messages, 200);
        assert_eq!(truncated[0].role, "system");
        assert!(truncated.len() < 101);
    }

    #[test]
    fn test_registry_truncate_keeps_recent_messages() {
        let registry = TokenizerRegistry::new();
        let mut messages = vec![];
        for i in 0..20 {
            messages.push(msg(
                "user",
                &format!("User message {} with enough text to consume tokens.", i),
            ));
        }
        let truncated = registry.truncate_messages("gpt-4", messages.clone(), 100);
        assert!(truncated.len() < 20);
        assert_eq!(
            truncated.last().unwrap().text(),
            messages.last().unwrap().text()
        );
    }

    #[test]
    fn test_registry_truncate_empty_input() {
        let registry = TokenizerRegistry::new();
        let truncated = registry.truncate_messages("gpt-4", vec![], 1000);
        assert!(truncated.is_empty());
    }

    #[test]
    fn test_registry_truncate_single_system_message() {
        let registry = TokenizerRegistry::new();
        let messages = vec![msg("system", "Be helpful")];
        let truncated = registry.truncate_messages("gpt-4", messages, 1000);
        assert_eq!(truncated.len(), 1);
        assert_eq!(truncated[0].role, "system");
    }

    #[test]
    fn test_registry_truncate_respects_min_keep() {
        let registry = TokenizerRegistry::new();
        let messages: Vec<ChatMessage> = (0..10)
            .map(|i| msg("user", &format!("msg {}", i)))
            .collect();
        let truncated = registry.truncate_messages("gpt-4", messages, 10);
        assert!(truncated.len() >= 2);
    }

    #[test]
    fn test_registry_register_custom() {
        let mut registry = TokenizerRegistry::new();
        registry.register("custom-model", Box::new(HeuristicTokenizer::new(5.0)));
        let t = registry.for_model("custom-model");
        assert_eq!(t.model_name(), "heuristic");
    }
}
