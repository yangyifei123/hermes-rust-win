use crate::provider::{ChatRequest, ChatResponse, DeltaMessage, LlmProvider, StreamChunk, StreamChoice};
use crate::provider::retry::{RetryPolicy, with_retry};
use crate::RuntimeError;
use futures::stream::StreamExt;
use futures::Stream;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or("https://api.anthropic.com/v1").to_string(),
            model: model.unwrap_or("claude-sonnet-4-20250514").to_string(),
        }
    }
}

/// Anthropic SSE event types.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AnthropicEventType {
    MessageStart,
    ContentBlockStart,
    ContentBlockDelta,
    ContentBlockStop,
    MessageDelta,
    MessageStop,
    Ping,
    Error,
    Unknown(String),
}

impl AnthropicEventType {
    fn from_str(s: &str) -> Self {
        match s {
            "message_start" => Self::MessageStart,
            "content_block_start" => Self::ContentBlockStart,
            "content_block_delta" => Self::ContentBlockDelta,
            "content_block_stop" => Self::ContentBlockStop,
            "message_delta" => Self::MessageDelta,
            "message_stop" => Self::MessageStop,
            "ping" => Self::Ping,
            "error" => Self::Error,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// A parsed Anthropic SSE event with its type and data payload.
#[derive(Debug)]
struct AnthropicSseEvent {
    event_type: AnthropicEventType,
    data: String,
}

/// Intermediate deserialization for Anthropic content_block_delta.
#[derive(serde::Deserialize)]
struct AnthropicContentBlockDelta {
    delta: AnthropicTextDelta,
}

#[derive(serde::Deserialize)]
struct AnthropicTextDelta {
    #[serde(rename = "type", default)]
    _type: String,
    text: String,
}

/// Parse an Anthropic SSE event into a StreamChunk (if it carries content).
/// Returns `None` for non-content events (ping, message_start, etc.).
fn parse_anthropic_sse_event(event: &AnthropicSseEvent) -> Option<Result<StreamChunk, RuntimeError>> {
    match event.event_type {
        AnthropicEventType::ContentBlockDelta => {
            match serde_json::from_str::<AnthropicContentBlockDelta>(&event.data) {
                Ok(payload) => Some(Ok(StreamChunk {
                    choices: vec![StreamChoice {
                        delta: DeltaMessage {
                            content: Some(payload.delta.text),
                        },
                    }],
                })),
                Err(e) => Some(Err(RuntimeError::ProviderError {
                    message: format!("Failed to parse Anthropic content_block_delta: {e}"),
                })),
            }
        }
        AnthropicEventType::Error => {
            Some(Err(RuntimeError::ProviderError {
                message: format!("Anthropic stream error: {}", event.data),
            }))
        }
        // message_start, content_block_start, content_block_stop, message_delta,
        // message_stop, ping, unknown — skip these
        _ => None,
    }
}

/// SSE buffer state for Anthropic streaming.
/// Tracks the current event type across `event:` and `data:` lines.
struct SseParserState {
    current_event_type: Option<AnthropicEventType>,
    current_data: Option<String>,
}

impl SseParserState {
    fn new() -> Self {
        Self {
            current_event_type: None,
            current_data: None,
        }
    }

    /// Process raw SSE text. Returns complete events and leaves incomplete data in the buffer.
    fn process(&mut self, buffer: &mut String, new_data: &str) -> Vec<AnthropicSseEvent> {
        buffer.push_str(new_data);
        let mut events = Vec::new();

        while let Some(pos) = buffer.find("\n\n") {
            let event_text = buffer[..pos].to_string();
            *buffer = buffer[pos + 2..].to_string();

            // Parse the event block line by line
            for line in event_text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some(event_name) = line.strip_prefix("event: ") {
                    self.current_event_type = Some(AnthropicEventType::from_str(event_name.trim()));
                } else if let Some(data) = line.strip_prefix("data: ") {
                    self.current_data = Some(data.trim().to_string());
                }
                // When we have both event type and data, emit the event
                if self.current_event_type.is_some() && self.current_data.is_some() {
                    events.push(AnthropicSseEvent {
                        event_type: self.current_event_type.take().unwrap(),
                        data: self.current_data.take().unwrap(),
                    });
                }
            }
        }

        events
    }
}

impl LlmProvider for AnthropicProvider {
    fn chat_completion(&self, mut request: ChatRequest) -> Pin<Box<dyn std::future::Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }

            let mut system_prompt = String::new();
            let mut anthropic_messages = Vec::new();
            for msg in &request.messages {
                match msg.role.as_str() {
                    "system" => system_prompt = msg.text().to_string(),
                    _ => anthropic_messages.push(json!({
                        "role": msg.role,
                        "content": msg.text(),
                    })),
                }
            }

            let mut body = json!({
                "model": request.model,
                "messages": anthropic_messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
            });
            if !system_prompt.is_empty() {
                body["system"] = json!(system_prompt);
            }

            let url = format!("{}/messages", self.base_url);
            let policy = RetryPolicy::default();

            with_retry(&policy, || {
                let b = body.clone();
                let url = url.clone();
                async move {
                    let resp = self
                        .client
                        .post(&url)
                        .header("x-api-key", &self.api_key)
                        .header("anthropic-version", "2023-06-01")
                        .header("Content-Type", "application/json")
                        .json(&b)
                        .send()
                        .await
                        .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })?;

                    let code = resp.status().as_u16();
                    if resp.status().is_success() {
                        let raw: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })?;

                        let content = raw["content"]
                            .as_array()
                            .and_then(|arr| arr.first())
                            .and_then(|block| block["text"].as_str())
                            .unwrap_or("")
                            .to_string();

                        // Parse Anthropic usage block:
                        // { "input_tokens": N, "output_tokens": N }
                        let usage = raw.get("usage").map(|u| {
                            let input = u["input_tokens"].as_u64().unwrap_or(0) as u32;
                            let output = u["output_tokens"].as_u64().unwrap_or(0) as u32;
                            crate::provider::TokenUsage {
                                input_tokens: input,
                                output_tokens: output,
                                total_tokens: input + output,
                            }
                        });

                        Ok(ChatResponse {
                            choices: vec![crate::provider::ChatChoice {
                                message: crate::provider::ChatMessage::assistant(&content),
                                finish_reason: raw["stop_reason"].as_str().map(|s| s.to_string()),
                            }],
                            usage,
                        })
                    } else if code == 429 {
                        Err(RuntimeError::RateLimitError { retry_after: None })
                    } else {
                        let body_text = resp.text().await.unwrap_or_default();
                        Err(RuntimeError::ProviderError {
                            message: format!("Anthropic API error {}: {}", code, body_text),
                        })
                    }
                }
            }).await
        })
    }

    fn chat_completion_stream(
        &self,
        mut request: ChatRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>, RuntimeError>> + Send + '_>> {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }

            let mut system_prompt = String::new();
            let mut anthropic_messages = Vec::new();
            for msg in &request.messages {
                match msg.role.as_str() {
                    "system" => system_prompt = msg.text().to_string(),
                    _ => anthropic_messages.push(json!({
                        "role": msg.role,
                        "content": msg.text(),
                    })),
                }
            }

            let mut body = json!({
                "model": request.model,
                "messages": anthropic_messages,
                "max_tokens": request.max_tokens.unwrap_or(4096),
                "stream": true,
            });
            if !system_prompt.is_empty() {
                body["system"] = json!(system_prompt);
            }

            let url = format!("{}/messages", self.base_url);
            let policy = RetryPolicy::default();

            let resp = with_retry(&policy, || {
                let b = body.clone();
                let url = url.clone();
                async move {
                    let r = self
                        .client
                        .post(&url)
                        .header("x-api-key", &self.api_key)
                        .header("anthropic-version", "2023-06-01")
                        .header("Content-Type", "application/json")
                        .json(&b)
                        .send()
                        .await
                        .map_err(|e| RuntimeError::ProviderError { message: e.to_string() })?;

                    let code = r.status().as_u16();
                    if r.status().is_success() {
                        Ok(r)
                    } else if code == 429 {
                        Err(RuntimeError::RateLimitError { retry_after: None })
                    } else {
                        let body_text = r.text().await.unwrap_or_default();
                        Err(RuntimeError::ProviderError {
                            message: format!("Anthropic streaming API error {code}: {body_text}"),
                        })
                    }
                }
            }).await?;

            let parser_state = SseParserState::new();
            let buffer = String::new();

            let stream = resp
                .bytes_stream()
                .scan((parser_state, buffer), |(state, buffer), chunk_result| {
                    let chunk = match chunk_result {
                        Ok(c) => c,
                        Err(e) => {
                            return std::future::ready(Some(vec![Err(RuntimeError::ProviderError {
                                message: format!("Anthropic stream read error: {e}"),
                            })]));
                        }
                    };

                    let text = String::from_utf8_lossy(&chunk);
                    let events = state.process(buffer, &text);

                    let results: Vec<Result<StreamChunk, RuntimeError>> = events
                        .iter()
                        .filter_map(parse_anthropic_sse_event)
                        .collect();

                    std::future::ready(Some(results))
                })
                .map(futures::stream::iter)
                .flatten();

            Ok(Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>)
        })
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new("test-key".to_string(), None, None);
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.default_model(), "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_anthropic_uses_x_api_key() {
        let provider = AnthropicProvider::new("sk-ant-test".to_string(), None, None);
        assert_eq!(provider.api_key, "sk-ant-test");
        assert!(provider.base_url.contains("anthropic.com"));
    }

    #[test]
    fn test_anthropic_event_type_from_str() {
        assert_eq!(AnthropicEventType::from_str("message_start"), AnthropicEventType::MessageStart);
        assert_eq!(AnthropicEventType::from_str("content_block_delta"), AnthropicEventType::ContentBlockDelta);
        assert_eq!(AnthropicEventType::from_str("message_stop"), AnthropicEventType::MessageStop);
        assert_eq!(AnthropicEventType::from_str("ping"), AnthropicEventType::Ping);
        assert_eq!(AnthropicEventType::from_str("error"), AnthropicEventType::Error);
        assert!(matches!(AnthropicEventType::from_str("custom_event"), AnthropicEventType::Unknown(_)));
    }

    #[test]
    fn test_parse_anthropic_content_block_delta() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::ContentBlockDelta,
            data: r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#.to_string(),
        };
        let result = parse_anthropic_sse_event(&event).unwrap().unwrap();
        assert_eq!(result.choices.len(), 1);
        assert_eq!(result.choices[0].delta.content.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_parse_anthropic_ping_ignored() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::Ping,
            data: "{}".to_string(),
        };
        assert!(parse_anthropic_sse_event(&event).is_none());
    }

    #[test]
    fn test_parse_anthropic_message_start_ignored() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::MessageStart,
            data: r#"{"type":"message_start","message":{"id":"msg_123"}}"#.to_string(),
        };
        assert!(parse_anthropic_sse_event(&event).is_none());
    }

    #[test]
    fn test_parse_anthropic_message_stop_ignored() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::MessageStop,
            data: r#"{"type":"message_stop"}"#.to_string(),
        };
        assert!(parse_anthropic_sse_event(&event).is_none());
    }

    #[test]
    fn test_parse_anthropic_error_event() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::Error,
            data: r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#.to_string(),
        };
        let result = parse_anthropic_sse_event(&event).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_anthropic_invalid_json() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::ContentBlockDelta,
            data: "not valid json".to_string(),
        };
        let result = parse_anthropic_sse_event(&event).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn test_sse_parser_state_multiple_events() {
        let mut state = SseParserState::new();
        let mut buffer = String::new();

        let input = "event: message_start\ndata: {\"type\":\"message_start\"}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let events = state.process(&mut buffer, input);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, AnthropicEventType::MessageStart);
        assert_eq!(events[1].event_type, AnthropicEventType::ContentBlockDelta);
        assert_eq!(events[2].event_type, AnthropicEventType::MessageStop);
        assert!(buffer.is_empty());

        // Verify content extraction from the delta event
        let chunk = parse_anthropic_sse_event(&events[1]).unwrap().unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi"));
    }

    #[test]
    fn test_sse_parser_state_partial_event() {
        let mut state = SseParserState::new();
        let mut buffer = String::new();

        // First chunk: incomplete (no \n\n terminator)
        let events1 = state.process(&mut buffer, "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel");
        assert!(events1.is_empty());
        assert!(!buffer.is_empty());

        // Second chunk completes the event
        let events2 = state.process(&mut buffer, "lo\"}}\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].event_type, AnthropicEventType::ContentBlockDelta);

        let chunk = parse_anthropic_sse_event(&events2[0]).unwrap().unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
    }
}
