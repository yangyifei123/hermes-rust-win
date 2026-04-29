//! Integration tests for OpenAI-compatible provider against a mock HTTP server.

use hermes_common::Provider;
use hermes_runtime::provider::{create_provider, ChatMessage, ChatRequest};
use hermes_runtime::RuntimeError;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper: build a basic ChatRequest with a single user message.
fn basic_request() -> ChatRequest {
    ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage::user("Hello")],
        tools: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: None,
    }
}

/// Helper: standard OpenAI chat completion response JSON.
fn openai_response_json() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test-123",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello from mock!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    })
}

#[tokio::test]
async fn test_openai_chat_completion_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-key"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_response_json()))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "test-key", Some(&mock_server.uri()));

    let response = provider.chat_completion(basic_request()).await.unwrap();
    assert_eq!(response.choices.len(), 1);
    assert_eq!(response.choices[0].message.text(), "Hello from mock!");
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("stop"));
    assert!(response.usage.is_some());
    let usage = response.usage.unwrap();
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 5);
    assert_eq!(usage.total_tokens, 15);
}

#[tokio::test]
async fn test_openai_error_handling_mock() {
    // 401 Unauthorized is NOT retryable, so the provider returns the error immediately.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {"message": "Invalid API key", "type": "invalid_request_error"}
        })))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "bad-key", Some(&mock_server.uri()));

    let result = provider.chat_completion(basic_request()).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Non-retryable errors surface as ProviderError immediately
    match err {
        RuntimeError::ProviderError { message } => {
            assert!(message.contains("401"), "expected 401 in error: {message}");
        }
        other => panic!("expected ProviderError, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_openai_not_found_mock() {
    // 404 is not retryable.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "test-key", Some(&mock_server.uri()));

    let result = provider.chat_completion(basic_request()).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        RuntimeError::ProviderError { message } => {
            assert!(message.contains("404"), "expected 404 in error: {message}");
        }
        other => panic!("expected ProviderError, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_openai_rate_limit_mock() {
    // 429 triggers retry logic (3 retries). Mount enough responses for all attempts.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": {"message": "Rate limit exceeded", "type": "rate_limit_error"}
        })))
        // Expect up to 4 calls: 1 initial + 3 retries
        .up_to_n_times(4)
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "test-key", Some(&mock_server.uri()));

    let result = provider.chat_completion(basic_request()).await;
    assert!(result.is_err());
    // After exhausting retries on 429, the error is wrapped in RetryExhausted
    match result.unwrap_err() {
        RuntimeError::RetryExhausted { attempts, .. } => {
            assert_eq!(attempts, 3);
        }
        other => panic!("expected RetryExhausted, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_openai_server_error_retry_exhausted_mock() {
    // 500 is retryable. Mount enough 500 responses to exhaust retries.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .up_to_n_times(4)
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "test-key", Some(&mock_server.uri()));

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
async fn test_openai_tool_calls_response_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-tool-test",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\": \"/tmp/test.txt\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
        })))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "test-key", Some(&mock_server.uri()));

    let response = provider.chat_completion(basic_request()).await.unwrap();
    assert_eq!(response.choices.len(), 1);
    let msg = &response.choices[0].message;
    assert!(msg.has_tool_calls());
    let tool_calls = msg.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_abc123");
    assert_eq!(tool_calls[0].function.name, "read_file");
    assert_eq!(tool_calls[0].function.arguments, r#"{"path": "/tmp/test.txt"}"#);
    assert_eq!(response.choices[0].finish_reason.as_deref(), Some("tool_calls"));
}

#[tokio::test]
async fn test_openai_empty_model_uses_default() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_response_json()))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::OpenAI, "test-key", Some(&mock_server.uri()));

    // Empty model string should fall back to the provider's default
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

#[tokio::test]
async fn test_deepseek_provider_mock() {
    // DeepSeek uses OpenAI-compatible API, verify it works through the mock.
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "deepseek-test",
            "choices": [{
                "message": {"role": "assistant", "content": "DeepSeek response"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8}
        })))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::DeepSeek, "test-key", Some(&mock_server.uri()));

    let response = provider.chat_completion(basic_request()).await.unwrap();
    assert_eq!(response.choices[0].message.text(), "DeepSeek response");
}

#[tokio::test]
async fn test_groq_provider_mock() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "groq-test",
            "choices": [{
                "message": {"role": "assistant", "content": "Groq fast response"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
        })))
        .mount(&mock_server)
        .await;

    let provider = create_provider(&Provider::Groq, "test-key", Some(&mock_server.uri()));

    let response = provider.chat_completion(basic_request()).await.unwrap();
    assert_eq!(response.choices[0].message.text(), "Groq fast response");
}
