//! Heuristic token estimation and context truncation.
//!
//! Provides a lightweight, no-dependency token counter based on the
//! observation that GPT-style tokenizers produce ~1 token per 4 characters
//! for English text.  This is deliberately approximate — a production
//! system would use tiktoken or a model-specific tokenizer — but avoids
//! adding a heavy native dependency.

use crate::provider::ChatMessage;

/// Characters per token (GPT-style heuristic for English text).
const CHARS_PER_TOKEN: usize = 4;

/// Multiplier applied to the word-count estimate to account for
/// sub-word fragmentation in BPE tokenizers.
const WORD_TOKEN_FACTOR: f64 = 1.3;

/// Overhead tokens per message (role tags, separator, etc.).
/// OpenAI-style messages add ~4 tokens of framing per message.
const MESSAGE_OVERHEAD_TOKENS: usize = 4;

/// Minimum number of recent messages to always keep during truncation.
const MIN_KEEP_MESSAGES: usize = 2;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Estimate the number of tokens in `text` using a heuristic.
///
/// The heuristic blends two approaches:
/// 1. **Character ratio**: `len / 4` — good for prose / English.
/// 2. **Word count × 1.3** — better for code / mixed content.
///
/// We take the maximum of the two to avoid under-counting.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let char_estimate = text.len() / CHARS_PER_TOKEN;

    let word_count = text.split_whitespace().count();
    let word_estimate = (word_count as f64 * WORD_TOKEN_FACTOR).ceil() as usize;

    char_estimate.max(word_estimate)
}

/// Estimate total tokens consumed by a slice of messages,
/// including per-message overhead.
pub fn estimate_messages_tokens(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .map(|msg| {
            let content_tokens = estimate_tokens(msg.text());
            let tool_tokens: usize = msg
                .tool_calls
                .as_ref()
                .map(|tcs| {
                    tcs.iter()
                        .map(|tc| {
                            estimate_tokens(&tc.function.arguments)
                                + estimate_tokens(&tc.function.name)
                        })
                        .sum()
                })
                .unwrap_or(0);
            content_tokens + tool_tokens + MESSAGE_OVERHEAD_TOKENS
        })
        .sum()
}

/// Truncate a list of messages to fit within `max_tokens`.
///
/// Strategy:
/// 1. Always keep the **system message** (first message if role is "system").
/// 2. Keep the **last N messages** from the tail, working backwards,
///    until the total would exceed `max_tokens`.
/// 3. Never drop below `MIN_KEEP_MESSAGES` recent messages.
///
/// Returns the truncated vector.  The original order is preserved.
pub fn truncate_messages(messages: Vec<ChatMessage>, max_tokens: usize) -> Vec<ChatMessage> {
    if messages.is_empty() {
        return messages;
    }

    let total = estimate_messages_tokens(&messages);
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

    let system_tokens =
        system.as_ref().map(|s| estimate_tokens(s.text()) + MESSAGE_OVERHEAD_TOKENS).unwrap_or(0);

    let budget = max_tokens.saturating_sub(system_tokens);

    // Walk backwards from the end, accumulating messages until budget is exhausted.
    let mut kept: Vec<ChatMessage> = Vec::new();
    let mut used = 0usize;

    for msg in rest.into_iter().rev() {
        let cost = estimate_tokens(msg.text()) + MESSAGE_OVERHEAD_TOKENS;
        if used + cost > budget && kept.len() >= MIN_KEEP_MESSAGES {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a message quickly.
    fn msg(role: &str, content: &str) -> ChatMessage {
        match role {
            "system" => ChatMessage::system(content),
            "user" => ChatMessage::user(content),
            "assistant" => ChatMessage::assistant(content),
            _ => ChatMessage::user(content),
        }
    }

    // ---- estimate_tokens ----

    #[test]
    fn test_estimate_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_short_text() {
        let tokens = estimate_tokens("Hello world");
        assert!(tokens >= 2, "expected at least 2 tokens, got {}", tokens);
    }

    #[test]
    fn test_estimate_long_text() {
        let text = "a ".repeat(1000);
        let tokens = estimate_tokens(&text);
        assert!(tokens >= 500, "expected at least 500 tokens, got {}", tokens);
    }

    #[test]
    fn test_estimate_code_heavy() {
        let code = "fn main() { println!(\"hello\"); }";
        let tokens = estimate_tokens(code);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_unicode() {
        let text = "你好世界";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
    }

    // ---- estimate_messages_tokens ----

    #[test]
    fn test_estimate_messages_empty() {
        assert_eq!(estimate_messages_tokens(&[]), 0);
    }

    #[test]
    fn test_estimate_messages_basic() {
        let messages = vec![ChatMessage::user("hello"), ChatMessage::assistant("hi there")];
        let tokens = estimate_messages_tokens(&messages);
        assert!(tokens > 0);
    }

    // ---- truncate_messages ----

    #[test]
    fn test_truncate_noop_when_under_limit() {
        let messages = vec![msg("user", "hello"), msg("assistant", "hi there")];
        let total = estimate_messages_tokens(&messages);
        let truncated = truncate_messages(messages.clone(), total + 1000);
        assert_eq!(truncated.len(), messages.len());
    }

    #[test]
    fn test_truncate_preserves_system_message() {
        let mut messages = vec![msg("system", "You are a helpful assistant.")];
        for i in 0..50 {
            messages.push(msg(
                "user",
                &format!("Message number {} with some padding text to increase token count.", i),
            ));
            messages
                .push(msg("assistant", &format!("Response number {} with some padding text.", i)));
        }
        let truncated = truncate_messages(messages, 200);
        assert_eq!(truncated[0].role, "system");
        assert!(truncated.len() < 101);
    }

    #[test]
    fn test_truncate_keeps_recent_messages() {
        let mut messages = vec![];
        for i in 0..20 {
            messages.push(msg(
                "user",
                &format!("User message {} with enough text to consume tokens.", i),
            ));
        }
        let truncated = truncate_messages(messages.clone(), 100);
        assert!(truncated.len() < 20);
        assert_eq!(truncated.last().unwrap().text(), messages.last().unwrap().text());
    }

    #[test]
    fn test_truncate_empty_input() {
        let truncated = truncate_messages(vec![], 1000);
        assert!(truncated.is_empty());
    }

    #[test]
    fn test_truncate_single_system_message() {
        let messages = vec![msg("system", "Be helpful")];
        let truncated = truncate_messages(messages, 1000);
        assert_eq!(truncated.len(), 1);
        assert_eq!(truncated[0].role, "system");
    }

    #[test]
    fn test_truncate_respects_min_keep() {
        let messages: Vec<ChatMessage> =
            (0..10).map(|i| msg("user", &format!("msg {}", i))).collect();
        let truncated = truncate_messages(messages, 10);
        assert!(truncated.len() >= MIN_KEEP_MESSAGES);
    }

    #[test]
    fn test_truncate_drops_oldest_non_system() {
        let mut messages = vec![msg("system", "sys")];
        for i in 0..30 {
            messages.push(msg("user", &format!(
                "This is message number {} and it has some extra text to pad the token count out.", i
            )));
        }
        let truncated = truncate_messages(messages.clone(), 300);
        assert_eq!(truncated[0].role, "system");
        let oldest_content = messages[1].text();
        assert!(
            truncated.iter().all(|m| m.text() != oldest_content),
            "oldest non-system message should have been dropped"
        );
    }
}
