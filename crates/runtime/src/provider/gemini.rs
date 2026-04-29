//! Google Gemini provider using the Generative Language API.
//!
//! Gemini uses its own API format (not OpenAI-compatible). Auth is via query
//! parameter `?key={api_key}`. Streaming uses SSE with `data:` lines.

use crate::provider::retry::{with_retry, RetryPolicy};
use crate::provider::{
    ChatMessage, ChatRequest, ChatResponse, DeltaMessage, LlmProvider, StreamChoice, StreamChunk,
    TokenUsage,
};
use crate::RuntimeError;
use futures::stream::StreamExt;
use futures::Stream;
use reqwest::Client;
use serde_json::json;
use std::pin::Pin;

pub struct GeminiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url
                .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
                .to_string(),
            model: model.unwrap_or("gemini-2.5-pro").to_string(),
        }
    }
}

/// Convert internal [`ChatMessage`] list to Gemini `contents` array, returning
/// the system instruction separately.
fn convert_messages(
    messages: &[ChatMessage],
) -> (Option<serde_json::Value>, Vec<serde_json::Value>) {
    let mut system_instruction: Option<serde_json::Value> = None;
    let mut contents: Vec<serde_json::Value> = Vec::new();

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                system_instruction = Some(json!({
                    "parts": [{ "text": msg.text() }]
                }));
            }
            "assistant" => {
                contents.push(json!({
                    "role": "model",
                    "parts": [{ "text": msg.text() }]
                }));
            }
            "tool" => {
                // Simplified: treat tool results as function-call responses.
                contents.push(json!({
                    "role": "function",
                    "parts": [{ "text": msg.text() }]
                }));
            }
            // "user" and anything else
            _ => {
                contents.push(json!({
                    "role": "user",
                    "parts": [{ "text": msg.text() }]
                }));
            }
        }
    }

    (system_instruction, contents)
}

/// Parse a Gemini `generateContent` JSON response into our [`ChatResponse`].
fn parse_gemini_response(raw: &serde_json::Value) -> Result<ChatResponse, RuntimeError> {
    let candidate = raw["candidates"].as_array().and_then(|arr| arr.first()).ok_or_else(|| {
        RuntimeError::ProviderError {
            message: "Gemini response missing candidates array".to_string(),
        }
    })?;

    let text = candidate["content"]["parts"]
        .as_array()
        .and_then(|parts| parts.first())
        .and_then(|p| p["text"].as_str())
        .unwrap_or("")
        .to_string();

    let finish_reason = candidate["finishReason"].as_str().map(|s| {
        // Map Gemini finish reasons to OpenAI-style names
        match s {
            "STOP" => "stop".to_string(),
            "MAX_TOKENS" => "length".to_string(),
            other => other.to_lowercase(),
        }
    });

    let usage = raw.get("usageMetadata").map(|u| {
        let input = u["promptTokenCount"].as_u64().unwrap_or(0) as u32;
        let output = u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;
        let total = u["totalTokenCount"].as_u64().unwrap_or(input as u64 + output as u64) as u32;
        TokenUsage { input_tokens: input, output_tokens: output, total_tokens: total }
    });

    Ok(ChatResponse {
        choices: vec![crate::provider::ChatChoice {
            message: ChatMessage::assistant(&text),
            finish_reason,
        }],
        usage,
    })
}

/// Parse a single Gemini SSE `data:` payload into a [`StreamChunk`].
/// Returns `None` for non-content chunks.
fn parse_gemini_sse_data(data: &str) -> Option<Result<StreamChunk, RuntimeError>> {
    let raw: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => {
            return Some(Err(RuntimeError::ProviderError {
                message: format!("Failed to parse Gemini SSE chunk: {e}"),
            }));
        }
    };

    let text = raw["candidates"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["content"]["parts"].as_array())
        .and_then(|parts| parts.first())
        .and_then(|p| p["text"].as_str())
        .unwrap_or("")
        .to_string();

    if text.is_empty() {
        // Check for finish reason on the final chunk
        let finish_reason = raw["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["finishReason"].as_str())
            .map(|s| match s {
                "STOP" => "stop".to_string(),
                "MAX_TOKENS" => "length".to_string(),
                other => other.to_lowercase(),
            });

        if let Some(reason) = finish_reason {
            return Some(Ok(StreamChunk {
                choices: vec![StreamChoice {
                    delta: DeltaMessage { content: None, tool_calls: None },
                    finish_reason: Some(reason),
                }],
            }));
        }
        // No content and no finish reason — skip
        return None;
    }

    Some(Ok(StreamChunk {
        choices: vec![StreamChoice {
            delta: DeltaMessage { content: Some(text), tool_calls: None },
            finish_reason: None,
        }],
    }))
}

/// SSE parser state for Gemini streaming.
struct GeminiSseParser {
    buffer: String,
}

impl GeminiSseParser {
    fn new() -> Self {
        Self { buffer: String::new() }
    }

    /// Process raw bytes, returning parsed `StreamChunk`s.
    fn process(&mut self, new_data: &str) -> Vec<Result<StreamChunk, RuntimeError>> {
        self.buffer.push_str(new_data);
        let mut results = Vec::new();

        while let Some(pos) = self.buffer.find("\n\n") {
            let event_text = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            for line in event_text.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Some(chunk) = parse_gemini_sse_data(data) {
                        results.push(chunk);
                    }
                }
            }
        }

        results
    }
}

impl LlmProvider for GeminiProvider {
    fn chat_completion(
        &self,
        mut request: ChatRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>>
    {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }

            let (system_instruction, contents) = convert_messages(&request.messages);

            let mut body = json!({
                "contents": contents,
                "generationConfig": {
                    "maxOutputTokens": request.max_tokens.unwrap_or(4096),
                }
            });

            if let Some(temp) = request.temperature {
                body["generationConfig"]["temperature"] = json!(temp);
            }

            if let Some(si) = system_instruction {
                body["systemInstruction"] = si;
            }

            let url = format!(
                "{}/models/{}:generateContent?key={}",
                self.base_url, request.model, self.api_key
            );

            let policy = RetryPolicy::default();

            with_retry(&policy, || {
                let b = body.clone();
                let url = url.clone();
                async move {
                    let resp = self
                        .client
                        .post(&url)
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

                        parse_gemini_response(&raw)
                    } else if code == 429 {
                        Err(RuntimeError::RateLimitError { retry_after: None })
                    } else {
                        let body_text = resp.text().await.unwrap_or_default();
                        Err(RuntimeError::ProviderError {
                            message: format!("Gemini API error {}: {}", code, body_text),
                        })
                    }
                }
            })
            .await
        })
    }

    #[allow(clippy::type_complexity)]
    fn chat_completion_stream(
        &self,
        mut request: ChatRequest,
    ) -> Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>,
                        RuntimeError,
                    >,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }

            let (system_instruction, contents) = convert_messages(&request.messages);

            let mut body = json!({
                "contents": contents,
                "generationConfig": {
                    "maxOutputTokens": request.max_tokens.unwrap_or(4096),
                }
            });

            if let Some(temp) = request.temperature {
                body["generationConfig"]["temperature"] = json!(temp);
            }

            if let Some(si) = system_instruction {
                body["systemInstruction"] = si;
            }

            let url = format!(
                "{}/models/{}:streamGenerateContent?alt=sse&key={}",
                self.base_url, request.model, self.api_key
            );

            let policy = RetryPolicy::default();

            let resp = with_retry(&policy, || {
                let b = body.clone();
                let url = url.clone();
                async move {
                    let r = self
                        .client
                        .post(&url)
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
                            message: format!("Gemini streaming API error {code}: {body_text}"),
                        })
                    }
                }
            })
            .await?;

            let parser = GeminiSseParser::new();

            let stream = resp
                .bytes_stream()
                .scan(parser, |parser_state, chunk_result| {
                    let chunk = match chunk_result {
                        Ok(c) => c,
                        Err(e) => {
                            return std::future::ready(Some(vec![Err(
                                RuntimeError::ProviderError {
                                    message: format!("Gemini stream read error: {e}"),
                                },
                            )]));
                        }
                    };

                    let text = String::from_utf8_lossy(&chunk);
                    let results = parser_state.process(&text);

                    std::future::ready(Some(results))
                })
                .map(futures::stream::iter)
                .flatten();

            Ok(Box::pin(stream)
                as Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>)
        })
    }

    fn name(&self) -> &str {
        "gemini"
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_provider_creation_with_defaults() {
        let provider = GeminiProvider::new("test-key".to_string(), None, None);
        assert_eq!(provider.name(), "gemini");
        assert_eq!(provider.default_model(), "gemini-2.5-pro");
        assert!(provider.base_url.contains("googleapis.com"));
    }

    #[test]
    fn test_gemini_provider_custom_config() {
        let provider = GeminiProvider::new(
            "my-key".to_string(),
            Some("https://custom.gemini.api"),
            Some("gemini-2.0-flash"),
        );
        assert_eq!(provider.api_key, "my-key");
        assert_eq!(provider.base_url, "https://custom.gemini.api");
        assert_eq!(provider.default_model(), "gemini-2.0-flash");
    }

    #[test]
    fn test_message_format_conversion() {
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there!"),
            ChatMessage::tool_result("call-1", "result text"),
        ];

        let (system_instruction, contents) = convert_messages(&messages);

        // System prompt extracted separately
        assert!(system_instruction.is_some());
        assert_eq!(system_instruction.unwrap()["parts"][0]["text"], "You are helpful.");

        // Three non-system messages
        assert_eq!(contents.len(), 3);

        // User message
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[0]["parts"][0]["text"], "Hello");

        // Assistant mapped to "model"
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[1]["parts"][0]["text"], "Hi there!");

        // Tool result mapped to "function"
        assert_eq!(contents[2]["role"], "function");
        assert_eq!(contents[2]["parts"][0]["text"], "result text");
    }

    #[test]
    fn test_parse_gemini_response() {
        let raw = json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello from Gemini!" }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 20,
                "totalTokenCount": 30
            }
        });

        let resp = parse_gemini_response(&raw).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.text(), "Hello from Gemini!");
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));

        let usage = resp.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn test_parse_gemini_response_no_candidates() {
        let raw = json!({"candidates": []});
        let result = parse_gemini_response(&raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_gemini_response_empty_text() {
        let raw = json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "" }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 0,
                "totalTokenCount": 5
            }
        });

        let resp = parse_gemini_response(&raw).unwrap();
        assert_eq!(resp.choices[0].message.text(), "");
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn test_sse_response_parsing() {
        let mut parser = GeminiSseParser::new();

        let chunk1 = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel\"}],\"role\":\"model\"}}]}\n\n";
        let chunk2 = "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"lo!\"}],\"role\":\"model\"},\"finishReason\":\"STOP\"}]}\n\n";

        let results1 = parser.process(chunk1);
        assert_eq!(results1.len(), 1);
        let r1 = results1[0].as_ref().unwrap();
        assert_eq!(r1.choices[0].delta.content.as_deref(), Some("Hel"));

        let results2 = parser.process(chunk2);
        assert_eq!(results2.len(), 1);
        let r2 = results2[0].as_ref().unwrap();
        assert_eq!(r2.choices[0].delta.content.as_deref(), Some("lo!"));
    }

    #[test]
    fn test_sse_response_partial_chunk() {
        let mut parser = GeminiSseParser::new();

        // Incomplete first chunk
        let results1 =
            parser.process("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"Hel");
        assert!(results1.is_empty());

        // Complete the chunk
        let results2 = parser.process("lo\"}]}}]}\n\n");
        assert_eq!(results2.len(), 1);
        assert_eq!(
            results2[0].as_ref().unwrap().choices[0].delta.content.as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn test_sse_invalid_json() {
        let mut parser = GeminiSseParser::new();
        let results = parser.process("data: not json\n\n");
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn test_sse_done_ignored() {
        let mut parser = GeminiSseParser::new();
        let results = parser.process("data: [DONE]\n\n");
        assert!(results.is_empty());
    }
}
