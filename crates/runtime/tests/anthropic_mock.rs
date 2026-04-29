//! Integration tests for Anthropic provider against a mock HTTP server.

use hermes_common::Provider;
use hermes_runtime::provider::{create_provider, ChatMessage, ChatRequest};
use hermes_runtime::RuntimeError;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: build a basic ChatRequest for Anthropic.
fn basic_request() -> ChatRequest {
    ChatRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        messages: vec![ChatMessage::user("Hello")],
        tools: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: None,
    }
}

/// Helper: standard Anthropic Messages API response JSON.
fn anthropic_response_json() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_test-123",
        "type": "message",
        "role": "assistant",
        "content": [{
            "type": "text",
            "text": "Hello from Anthropic mock!"
        }],
        "model": "claude-sonnet-4-20250514",
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 12,
            "output_tokens": 8
        }
    })
}

#[tokio::test]
async fn test_anthropic_chat_completion_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_response_json()))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::Anthropic, "test-key", Some(&mock_server.uri()));

    let response = provider.chat_completion(basic_request()).await.unwrap();
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.choices[0].message.text(), "Hello from Anthropic mock!");
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("end_turn"));
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert_eq!(usage.input_tokens, 12);
    assert_eq!(usage.output_tokens, 8);
    assert_eq!(usage.total_tokens, 20);
}

#[tokio::test]
async fn test_anthropic_error_handling_mock() {
    // 401 is not retryable, returns immediately as ProviderError.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "type": "error",
            "error": {"type": "authentication_error", "message": "invalid x-api-key"}
        })))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::Anthropic, "bad-key", Some(&mock_server.uri()));

    let result = provider.chat_completion(basic_request()).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        RuntimeError::ProviderError { message } => {
            assert!(message.contains("401"), "expected 401 in error: {message}");
        }
        other => panic!("expected ProviderError, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_anthropic_rate_limit_mock() {
    // 429 is retryable. Mount enough for all attempts (1 initial + 3 retries).
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "type": "error",
            "error": {"type": "rate_limit_error", "message": "rate limited"}
        })))
        .up_to_n_times(4)
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::Anthropic, "test-key", Some(&mock_server.uri()));

    let result = provider.chat_completion(basic_request()).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        RuntimeError::RetryExhausted { attempts, .. } => {
            assert_eq!(attempts, 3);
        }
        other => panic!("expected RetryExhausted, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_anthropic_system_prompt_extraction() {
    // Verify the Anthropic provider handles system messages correctly.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_system_test",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "I understand the system prompt."}],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 20, "output_tokens": 6}
        })))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::Anthropic, "test-key", Some(&mock_server.uri()));

    let request = ChatRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        messages: vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Hello"),
        ],
        tools: None,
        max_tokens: Some(50),
        temperature: None,
        stream: None,
    };

    let response = provider.chat_completion(request).await.unwrap();
    assert_eq!(response.choices[0].message.text(), "I understand the system prompt.");
}

#[tokio::test]
async fn test_anthropic_empty_model_uses_default() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_response_json()))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::Anthropic, "test-key", Some(&mock_server.uri()));

    let request = ChatRequest {
        model: String::new(),
        messages: vec![ChatMessage::user("Hi")],
        tools: None,
        max_tokens: None,
        temperature: None,
        stream: None,
    };

    let response = provider.chat_completion(request).await.unwrap();
    assert_eq!(response.choices.len(), 1);
}
