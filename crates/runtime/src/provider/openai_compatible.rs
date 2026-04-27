use crate::provider::{
    ChatRequest, ChatResponse, DeltaMessage, LlmProvider, StreamChunk, StreamChoice, ToolCallDelta,
};
use crate::provider::retry::{RetryPolicy, with_retry};
use crate::RuntimeError;
use futures::stream::StreamExt;
use futures::Stream;
use reqwest::Client;
use std::pin::Pin;

pub struct OpenAiCompatibleProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    provider_name: String,
    auth_header: String,
    auth_prefix: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        api_key: String,
        base_url: Option<&str>,
        model: Option<&str>,
        provider_name: &str,
        auth_header: Option<&str>,
        auth_prefix: Option<&str>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url.unwrap_or("https://api.openai.com/v1").to_string(),
            model: model.unwrap_or("gpt-4o").to_string(),
            provider_name: provider_name.to_string(),
            auth_header: auth_header.unwrap_or("Authorization").to_string(),
            auth_prefix: auth_prefix.unwrap_or("Bearer ").to_string(),
        }
    }

    /// Returns a reference to the internal `base_url` string.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns a reference to the internal `model` string.
    pub fn model(&self) -> &str {
        &self.model
    }
}

/// Intermediate struct for deserializing OpenAI-compatible stream deltas.
#[derive(serde::Deserialize)]
struct OpenAiStreamPayload {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(serde::Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Parse a single SSE `data:` line into a `StreamChunk`.
/// Returns `None` for `[DONE]` sentinel or non-parseable lines.
fn parse_sse_line(line: &str) -> Option<Result<StreamChunk, RuntimeError>> {
    let data = line.strip_prefix("data:")?;
    let data = data.trim();

    if data == "[DONE]" {
        return None;
    }

    match serde_json::from_str::<OpenAiStreamPayload>(data) {
        Ok(payload) => {
            let choices: Vec<StreamChoice> = payload
                .choices
                .into_iter()
                .map(|c| StreamChoice {
                    delta: DeltaMessage {
                        content: c.delta.content,
                        tool_calls: c.delta.tool_calls,
                    },
                    finish_reason: c.finish_reason,
                })
                .collect();
            Some(Ok(StreamChunk { choices }))
        }
        Err(e) => Some(Err(RuntimeError::ProviderError {
            message: format!("Failed to parse SSE JSON: {e}"),
        })),
    }
}

/// Process a raw SSE buffer into individual `data:` lines.
/// SSE lines are separated by `\n\n`. Each event may have one `data:` line.
fn sse_buffer_to_lines(buffer: &str) -> (Vec<String>, String) {
    let mut complete_lines = Vec::new();
    let mut remaining = String::new();

    for chunk in buffer.split("\n\n") {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Check if this looks like a complete SSE event (starts with "data:")
        // If the last segment doesn't end cleanly, it's incomplete
        if chunk == buffer && !buffer.ends_with("\n\n") && !buffer.contains("\n\n") {
            // Entire buffer is one incomplete event
            remaining = chunk.to_string();
        } else if !buffer.ends_with("\n\n")
            && chunk == buffer.rsplit_once("\n\n").map(|(_, s)| s).unwrap_or("")
        {
            // Last segment after last \n\n is incomplete
            remaining = chunk.to_string();
        } else {
            for line in trimmed.lines() {
                let line = line.trim();
                if line.starts_with("data:") {
                    complete_lines.push(line.to_string());
                }
            }
        }
    }

    (complete_lines, remaining)
}

impl LlmProvider for OpenAiCompatibleProvider {
    fn chat_completion(
        &self,
        mut request: ChatRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>>
    {
        Box::pin(async move {
            if request.model.is_empty() {
                request.model = self.model.clone();
            }
            let url = format!("{}/chat/completions", self.base_url);
            let policy = RetryPolicy::default();
            with_retry(&policy, || {
                let req = request.clone();
                let url = url.clone();
                async move {
                    let resp = self
                        .client
                        .post(&url)
                        .header(
                            &self.auth_header,
                            format!("{}{}", self.auth_prefix, self.api_key),
                        )
                        .header("Content-Type", "application/json")
                        .json(&req)
                        .send()
                        .await
                        .map_err(|e| RuntimeError::ProviderError {
                            message: e.to_string(),
                        })?;

                    let status = resp.status();
                    if status.is_success() {
                        resp.json::<ChatResponse>()
                            .await
                            .map_err(|e| RuntimeError::ProviderError {
                                message: e.to_string(),
                            })
                    } else {
                        let code = status.as_u16();
                        let body = resp.text().await.unwrap_or_default();
                        if code == 429 {
                            Err(RuntimeError::RateLimitError { retry_after: None })
                        } else {
                            Err(RuntimeError::ProviderError {
                                message: format!("API error {}: {}", code, body),
                            })
                        }
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
            request.stream = Some(true);

            let url = format!("{}/chat/completions", self.base_url);
            let policy = RetryPolicy::default();

            let resp = with_retry(&policy, || {
                let req = request.clone();
                let url = url.clone();
                async move {
                    let r = self
                        .client
                        .post(&url)
                        .header(
                            &self.auth_header,
                            format!("{}{}", self.auth_prefix, self.api_key),
                        )
                        .header("Content-Type", "application/json")
                        .json(&req)
                        .send()
                        .await
                        .map_err(|e| RuntimeError::ProviderError {
                            message: e.to_string(),
                        })?;

                    let code = r.status().as_u16();
                    if r.status().is_success() {
                        Ok(r)
                    } else if code == 429 {
                        Err(RuntimeError::RateLimitError { retry_after: None })
                    } else {
                        let body = r.text().await.unwrap_or_default();
                        Err(RuntimeError::ProviderError {
                            message: format!("{} streaming API error {code}: {body}", self.provider_name),
                        })
                    }
                }
            })
            .await?;

            let stream = resp
                .bytes_stream()
                .scan(String::new(), |buffer, chunk_result| {
                    let chunk = match chunk_result {
                        Ok(c) => c,
                        Err(e) => {
                            return std::future::ready(Some(vec![Err(
                                RuntimeError::ProviderError {
                                    message: format!("Stream read error: {e}"),
                                },
                            )]));
                        }
                    };

                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    let (lines, remaining) = sse_buffer_to_lines(buffer);
                    *buffer = remaining;

                    let results: Vec<Result<StreamChunk, RuntimeError>> =
                        lines.iter().filter_map(|line| parse_sse_line(line)).collect();

                    std::future::ready(Some(results))
                })
                .map(futures::stream::iter)
                .flatten();

            Ok(Box::pin(stream)
                as Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>)
        })
    }

    fn name(&self) -> &str {
        &self.provider_name
    }

    fn default_model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatible_provider_defaults() {
        let provider =
            OpenAiCompatibleProvider::new("test-key".to_string(), None, None, "test", None, None);
        assert_eq!(provider.name(), "test");
        assert_eq!(provider.default_model(), "gpt-4o");
        assert_eq!(provider.base_url(), "https://api.openai.com/v1");
        assert_eq!(provider.auth_header, "Authorization");
        assert_eq!(provider.auth_prefix, "Bearer ");
    }

    #[test]
    fn test_compatible_provider_custom() {
        let provider = OpenAiCompatibleProvider::new(
            "key".to_string(),
            Some("https://custom.api.com/v1"),
            Some("my-model"),
            "custom",
            Some("X-Api-Key"),
            Some("Token "),
        );
        assert_eq!(provider.name(), "custom");
        assert_eq!(provider.default_model(), "my-model");
        assert_eq!(provider.base_url(), "https://custom.api.com/v1");
        assert_eq!(provider.auth_header, "X-Api-Key");
        assert_eq!(provider.auth_prefix, "Token ");
    }

    #[test]
    fn test_parse_sse_content_delta() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let result = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(result.choices.len(), 1);
        assert_eq!(
            result.choices[0].delta.content.as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn test_parse_sse_done_sentinel() {
        let line = "data: [DONE]";
        assert!(parse_sse_line(line).is_none());
    }

    #[test]
    fn test_parse_sse_empty_content() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":null}]}"#;
        let result = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(result.choices.len(), 1);
        assert!(result.choices[0].delta.content.is_none());
    }

    #[test]
    fn test_parse_sse_invalid_json() {
        let line = "data: {invalid json}";
        let result = parse_sse_line(line).unwrap();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_sse_non_data_line() {
        assert!(parse_sse_line(": comment").is_none());
        assert!(parse_sse_line("").is_none());
        assert!(parse_sse_line("event: ping").is_none());
    }

    #[test]
    fn test_sse_buffer_to_lines_complete() {
        let buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";
        let (lines, remaining) = sse_buffer_to_lines(buffer);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"Hi\""));
        assert!(lines[1].contains("[DONE]"));
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_sse_buffer_to_lines_incomplete() {
        let buffer = "data: {\"choices\":[{\"delta\":{\"content\":\"Hel";
        let (lines, remaining) = sse_buffer_to_lines(buffer);
        assert!(lines.is_empty());
        assert!(remaining.contains("Hel"));
    }

    #[test]
    fn test_parse_sse_role_delta() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}"#;
        let result = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(result.choices.len(), 1);
        assert!(result.choices[0].delta.content.is_none());
    }

    #[test]
    fn test_parse_sse_finish_reason() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        let result = parse_sse_line(line).unwrap().unwrap();
        assert_eq!(result.choices.len(), 1);
        assert!(result.choices[0].delta.content.is_none());
    }
}
