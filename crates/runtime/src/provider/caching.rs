//! Prompt Caching for Anthropic/OpenRouter Claude Models
//!
//! Applies `cache_control` ephemeral blocks to messages for
//! Anthropic prompt caching support.

use crate::provider::ChatMessage;
use hermes_common::Provider;

/// Apply prompt caching markers to messages for Claude models.
///
/// Caches the system message and the last 3 non-system messages
/// with `{"type": "ephemeral"}` cache control blocks.
///
/// Only applies when the model is a Claude variant and the provider
/// is Anthropic or OpenRouter.
pub fn apply_prompt_caching(messages: &mut [ChatMessage], provider: Provider, model: &str) {
    let is_claude = model.to_lowercase().contains("claude");
    let is_anthropic = provider == Provider::Anthropic;
    let is_openrouter = provider == Provider::OpenRouter;

    if !(is_claude && (is_anthropic || is_openrouter)) {
        return;
    }

    let marker = serde_json::json!({"type": "ephemeral"});

    // Cache system message if present
    if messages.first().map(|m| m.role.as_str()) == Some("system") {
        messages[0].cache_control = Some(marker.clone());
    }

    // Cache last 3 non-system messages
    let non_sys: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role != "system")
        .map(|(i, _)| i)
        .collect();

    for idx in non_sys.iter().rev().take(3) {
        messages[*idx].cache_control = Some(marker.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_caching_for_non_claude() {
        let mut msgs = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hello"),
        ];
        apply_prompt_caching(&mut msgs, Provider::OpenAI, "gpt-4o");
        assert!(msgs[0].cache_control.is_none());
        assert!(msgs[1].cache_control.is_none());
    }

    #[test]
    fn test_caching_for_claude_anthropic() {
        let mut msgs = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("msg1"),
            ChatMessage::assistant("reply1"),
            ChatMessage::user("msg2"),
            ChatMessage::assistant("reply2"),
            ChatMessage::user("msg3"),
        ];
        apply_prompt_caching(&mut msgs, Provider::Anthropic, "claude-sonnet-4-20250514");

        // System message cached
        assert!(msgs[0].cache_control.is_some());
        // Last 3 non-system: msg3 (idx 5), reply2 (idx 4), msg2 (idx 3)
        assert!(msgs[3].cache_control.is_some()); // msg2
        assert!(msgs[4].cache_control.is_some()); // reply2
        assert!(msgs[5].cache_control.is_some()); // msg3
                                                  // Earlier messages not cached
        assert!(msgs[1].cache_control.is_none()); // msg1
        assert!(msgs[2].cache_control.is_none()); // reply1
    }

    #[test]
    fn test_caching_for_claude_openrouter() {
        let mut msgs = vec![ChatMessage::system("sys"), ChatMessage::user("hello")];
        apply_prompt_caching(&mut msgs, Provider::OpenRouter, "anthropic/claude-sonnet-4");
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[1].cache_control.is_some());
    }

    #[test]
    fn test_no_caching_without_system() {
        let mut msgs = vec![ChatMessage::user("hello"), ChatMessage::assistant("hi")];
        apply_prompt_caching(&mut msgs, Provider::Anthropic, "claude-sonnet-4");
        // Both should be cached (last 2 of 3 allowed)
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[1].cache_control.is_some());
    }

    #[test]
    fn test_fewer_than_three_messages() {
        let mut msgs = vec![ChatMessage::system("sys"), ChatMessage::user("hi")];
        apply_prompt_caching(&mut msgs, Provider::Anthropic, "claude-3");
        assert!(msgs[0].cache_control.is_some());
        assert!(msgs[1].cache_control.is_some());
    }
}
