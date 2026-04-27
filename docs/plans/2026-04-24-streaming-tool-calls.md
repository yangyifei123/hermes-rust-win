# Streaming + Tool Calls Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable streaming mode to correctly parse, accumulate, and execute tool calls from SSE chunks across OpenAI and Anthropic providers, supporting multi-step agent loops.

**Architecture:** Extend `DeltaMessage` with `tool_calls` field; introduce `StreamEvent` enum to distinguish content deltas from tool call lifecycle events; refactor `Agent::stream_turn()` to yield `StreamEvent` instead of raw strings; rewrite `ChatRepl::run_turn_streaming()` as a multi-step loop that streams → detects tool calls → executes → streams follow-up.

**Tech Stack:** Rust, Tokio, futures, serde, reqwest

---

## Overview

Currently, streaming mode returns `tool_calls_made: vec![]` because:
1. `DeltaMessage` only has `content: Option<String>`, no `tool_calls` field
2. OpenAI SSE parsing ignores `delta.tool_calls` arrays in chunks
3. Anthropic SSE parsing ignores `content_block_start` with `type: "tool_use"`
4. `Agent::stream_turn()` yields only `String` content deltas
5. `ChatRepl::run_turn_streaming()` has no tool call handling at all

This plan fixes all five issues with minimal, testable steps.

---

## Task 1: Extend Provider Types in `provider/mod.rs`

**Files:**
- Modify: `crates/runtime/src/provider/mod.rs:107-110`

**Step 1: Add `ToolCallDelta` and `FunctionCallDelta` structs**

Add after `FunctionCall` (around line 57):

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCallDelta {
    pub index: u32,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub tool_type: Option<String>,
    pub function: Option<FunctionCallDelta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionCallDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}
```

**Step 2: Extend `DeltaMessage`**

Change lines 107-110 from:
```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeltaMessage {
    pub content: Option<String>,
}
```

To:
```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeltaMessage {
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}
```

**Step 3: Add `StreamEvent` enum**

Add after the `LlmProvider` trait (after line 122):

```rust
/// Events yielded by the agent streaming loop.
/// Distinguishes content tokens from tool call lifecycle events.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A content token delta from the LLM.
    ContentDelta(String),
    /// A tool call has started (we know its index, id, and name).
    ToolCallStart { index: u32, id: String, name: String },
    /// Partial arguments for a tool call have arrived.
    ToolCallDelta { index: u32, arguments: String },
    /// A tool call has finished streaming (arguments are complete).
    ToolCallComplete { index: u32 },
    /// The stream finished (with optional finish_reason).
    Done { finish_reason: Option<String> },
}
```

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: PASS (no breaking changes yet — new fields are optional)

**Step 5: Commit**

```bash
git add crates/runtime/src/provider/mod.rs
git commit -m "feat(provider): extend DeltaMessage with tool_calls and add StreamEvent enum"
```

---

## Task 2: Update OpenAI SSE Parsing for Tool Call Deltas

**Files:**
- Modify: `crates/runtime/src/provider/openai.rs:55-69`, `28-52`

**Step 1: Extend `OpenAiStreamDelta`**

Change lines 65-69 from:
```rust
#[derive(serde::Deserialize)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: Option<String>,
}
```

To:
```rust
#[derive(serde::Deserialize)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<crate::provider::ToolCallDelta>>,
    #[serde(default)]
    finish_reason: Option<String>,
}
```

**Step 2: Update `parse_openai_sse_line`**

Change lines 35-52 from:
```rust
    match serde_json::from_str::<OpenAiStreamPayload>(data) {
        Ok(payload) => {
            let choices: Vec<StreamChoice> = payload
                .choices
                .into_iter()
                .map(|c| StreamChoice {
                    delta: DeltaMessage {
                        content: c.delta.content,
                    },
                })
                .collect();
            Some(Ok(StreamChunk { choices }))
        }
```

To:
```rust
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
                    finish_reason: c.delta.finish_reason,
                })
                .collect();
            Some(Ok(StreamChunk { choices }))
        }
```

**Step 3: Extend `StreamChoice`**

Add `finish_reason: Option<String>` to `StreamChoice` in `provider/mod.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct StreamChoice {
    pub delta: DeltaMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}
```

**Step 4: Add tests for tool call SSE parsing**

Add to `openai.rs` tests:

```rust
    #[test]
    fn test_parse_openai_sse_tool_call_delta() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"get_weather"}}]},"finish_reason":null}]}"#;
        let result = parse_openai_sse_line(line).unwrap().unwrap();
        assert_eq!(result.choices.len(), 1);
        let delta = &result.choices[0].delta;
        assert!(delta.content.is_none());
        let tool_calls = delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].index, 0);
        assert_eq!(tool_calls[0].id.as_deref(), Some("call_abc"));
        assert_eq!(tool_calls[0].tool_type.as_deref(), Some("function"));
        let func = tool_calls[0].function.as_ref().unwrap();
        assert_eq!(func.name.as_deref(), Some("get_weather"));
    }

    #[test]
    fn test_parse_openai_sse_tool_call_arguments_delta() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"city\":\"Bei"}}]},"finish_reason":null}]}"#;
        let result = parse_openai_sse_line(line).unwrap().unwrap();
        let tool_calls = result.choices[0].delta.tool_calls.as_ref().unwrap();
        let func = tool_calls[0].function.as_ref().unwrap();
        assert_eq!(func.arguments.as_deref(), Some("{\"city\":\"Bei"));
    }

    #[test]
    fn test_parse_openai_sse_finish_reason_stop() {
        let line = r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        let result = parse_openai_sse_line(line).unwrap().unwrap();
        assert_eq!(result.choices[0].finish_reason.as_deref(), Some("stop"));
    }
```

**Step 5: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/runtime/src/provider/openai.rs crates/runtime/src/provider/mod.rs
git commit -m "feat(openai): parse tool_call deltas from SSE streaming chunks"
```

---

## Task 3: Update Anthropic SSE Parsing for Tool Use Blocks

**Files:**
- Modify: `crates/runtime/src/provider/anthropic.rs:64-104`, `27-55`

**Step 1: Add Anthropic tool use structs**

Add after `AnthropicTextDelta` (around line 75):

```rust
/// Intermediate deserialization for Anthropic content_block_start (tool_use).
#[derive(serde::Deserialize)]
struct AnthropicContentBlockStart {
    index: u32,
    content_block: AnthropicContentBlock,
}

#[derive(serde::Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

/// Intermediate deserialization for Anthropic content_block_delta (input_json_delta).
#[derive(serde::Deserialize)]
struct AnthropicInputJsonDelta {
    delta: AnthropicJsonDelta,
}

#[derive(serde::Deserialize)]
struct AnthropicJsonDelta {
    #[serde(rename = "type")]
    _type: String,
    partial_json: String,
}
```

**Step 2: Update `parse_anthropic_sse_event`**

Change the function (lines 79-104) to handle `ContentBlockStart` and `ContentBlockDelta` for both text and tool_use:

```rust
fn parse_anthropic_sse_event(event: &AnthropicSseEvent) -> Option<Result<StreamChunk, RuntimeError>> {
    match event.event_type {
        AnthropicEventType::ContentBlockStart => {
            match serde_json::from_str::<AnthropicContentBlockStart>(&event.data) {
                Ok(payload) => {
                    if payload.content_block.block_type == "tool_use" {
                        // Emit a tool call start delta
                        let tool_calls = vec![crate::provider::ToolCallDelta {
                            index: payload.index,
                            id: payload.content_block.id.clone(),
                            tool_type: Some("function".to_string()),
                            function: Some(crate::provider::FunctionCallDelta {
                                name: payload.content_block.name.clone(),
                                arguments: None,
                            }),
                        }];
                        Some(Ok(StreamChunk {
                            choices: vec![StreamChoice {
                                delta: DeltaMessage {
                                    content: None,
                                    tool_calls: Some(tool_calls),
                                },
                                finish_reason: None,
                            }],
                        }))
                    } else {
                        // Text block start — no delta to emit
                        None
                    }
                }
                Err(e) => Some(Err(RuntimeError::ProviderError {
                    message: format!("Failed to parse Anthropic content_block_start: {e}"),
                })),
            }
        }
        AnthropicEventType::ContentBlockDelta => {
            // Try text delta first
            if let Ok(payload) = serde_json::from_str::<AnthropicContentBlockDelta>(&event.data) {
                Some(Ok(StreamChunk {
                    choices: vec![StreamChoice {
                        delta: DeltaMessage {
                            content: Some(payload.delta.text),
                            tool_calls: None,
                        },
                        finish_reason: None,
                    }],
                }))
            } else if let Ok(payload) = serde_json::from_str::<AnthropicInputJsonDelta>(&event.data) {
                // input_json_delta for tool_use
                let tool_calls = vec![crate::provider::ToolCallDelta {
                    index: 0, // Anthropic sends one block at a time; index inferred from context
                    id: None,
                    tool_type: None,
                    function: Some(crate::provider::FunctionCallDelta {
                        name: None,
                        arguments: Some(payload.delta.partial_json),
                    }),
                }];
                Some(Ok(StreamChunk {
                    choices: vec![StreamChoice {
                        delta: DeltaMessage {
                            content: None,
                            tool_calls: Some(tool_calls),
                        },
                        finish_reason: None,
                    }],
                }))
            } else {
                Some(Err(RuntimeError::ProviderError {
                    message: format!("Failed to parse Anthropic content_block_delta: {}", event.data),
                }))
            }
        }
        AnthropicEventType::Error => {
            Some(Err(RuntimeError::ProviderError {
                message: format!("Anthropic stream error: {}", event.data),
            }))
        }
        _ => None,
    }
}
```

**Step 3: Add tests for Anthropic tool use parsing**

Add to `anthropic.rs` tests:

```rust
    #[test]
    fn test_parse_anthropic_tool_use_start() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::ContentBlockStart,
            data: r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_123","name":"get_weather","input":{}}}"#.to_string(),
        };
        let result = parse_anthropic_sse_event(&event).unwrap().unwrap();
        let delta = &result.choices[0].delta;
        assert!(delta.content.is_none());
        let tool_calls = delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls[0].index, 0);
        assert_eq!(tool_calls[0].id.as_deref(), Some("toolu_123"));
        let func = tool_calls[0].function.as_ref().unwrap();
        assert_eq!(func.name.as_deref(), Some("get_weather"));
    }

    #[test]
    fn test_parse_anthropic_input_json_delta() {
        let event = AnthropicSseEvent {
            event_type: AnthropicEventType::ContentBlockDelta,
            data: r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"Beijing\"}"}}"#.to_string(),
        };
        let result = parse_anthropic_sse_event(&event).unwrap().unwrap();
        let tool_calls = result.choices[0].delta.tool_calls.as_ref().unwrap();
        let func = tool_calls[0].function.as_ref().unwrap();
        assert_eq!(func.arguments.as_deref(), Some("{\"city\":\"Beijing\"}"));
    }
```

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/runtime/src/provider/anthropic.rs
git commit -m "feat(anthropic): parse tool_use content blocks from SSE streaming"
```

---

## Task 4: Refactor `Agent::stream_turn()` to Yield `StreamEvent`

**Files:**
- Modify: `crates/runtime/src/agent/mod.rs:415-486`

**Step 1: Change return type and imports**

Add `StreamEvent` to the import at line 3:
```rust
use crate::provider::{ChatMessage, ChatRequest, ChatResponse, LlmProvider, StreamEvent};
```

Change the function signature (line 415-418) from:
```rust
    pub fn stream_turn(
        &self,
        session_id: Uuid,
    ) -> Pin<Box<dyn Stream<Item = Result<String, RuntimeError>> + Send>> {
```

To:
```rust
    pub fn stream_turn(
        &self,
        session_id: Uuid,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, RuntimeError>> + Send>> {
```

**Step 2: Rewrite the stream mapping logic**

Replace the `.map()` closure (lines 461-475) from:
```rust
        .map(|chunk_result| {
            // Extract the content delta from each StreamChunk
            match chunk_result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(ref content) = choice.delta.content {
                            return Ok(content.clone());
                        }
                    }
                    // Empty delta (e.g. role-only chunk) — skip by yielding empty
                    Ok(String::new())
                }
                Err(e) => Err(e),
            }
        })
        .filter(|result| {
            // Filter out empty deltas so the caller only gets real content
            let keep = match result {
                Ok(s) => !s.is_empty(),
                Err(_) => true,
            };
            std::future::ready(keep)
        });
```

To:
```rust
        .map(|chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first() {
                        // Yield content delta if present
                        if let Some(ref content) = choice.delta.content {
                            if !content.is_empty() {
                                return Ok(StreamEvent::ContentDelta(content.clone()));
                            }
                        }
                        // Yield tool call deltas if present
                        if let Some(ref tool_call_deltas) = choice.delta.tool_calls {
                            let mut events = Vec::new();
                            for tc in tool_call_deltas {
                                if tc.id.is_some() && tc.function.as_ref().and_then(|f| f.name.as_ref()).is_some() {
                                    // This is a tool call start
                                    events.push(StreamEvent::ToolCallStart {
                                        index: tc.index,
                                        id: tc.id.clone().unwrap_or_default(),
                                        name: tc.function.as_ref().unwrap().name.clone().unwrap_or_default(),
                                    });
                                }
                                if tc.function.as_ref().and_then(|f| f.arguments.as_ref()).is_some() {
                                    // This is a tool call arguments delta
                                    events.push(StreamEvent::ToolCallDelta {
                                        index: tc.index,
                                        arguments: tc.function.as_ref().unwrap().arguments.clone().unwrap_or_default(),
                                    });
                                }
                            }
                            // Return first event; additional events need flattening
                            if let Some(first) = events.into_iter().next() {
                                return Ok(first);
                            }
                        }
                        // Check for finish_reason
                        if let Some(ref reason) = choice.finish_reason {
                            return Ok(StreamEvent::Done {
                                finish_reason: Some(reason.clone()),
                            });
                        }
                    }
                    // Empty/heartbeat chunk — skip
                    Ok(StreamEvent::ContentDelta(String::new()))
                }
                Err(e) => Err(e),
            }
        })
        .filter(|result| {
            let keep = match result {
                Ok(StreamEvent::ContentDelta(s)) => !s.is_empty(),
                Ok(_) => true,
                Err(_) => true,
            };
            std::future::ready(keep)
        });
```

**Note:** The above `.map()` only yields one event per chunk. For chunks that contain both a `ToolCallStart` and `ToolCallDelta`, we need a `.map()` that yields `Vec<StreamEvent>` followed by `.map(futures::stream::iter).flatten()`. The plan below uses the flattened approach.

**Corrected Step 2 (flattened):**

```rust
        .map(|chunk_result| {
            let events: Vec<Result<StreamEvent, RuntimeError>> = match chunk_result {
                Ok(chunk) => {
                    let mut evs = Vec::new();
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(ref content) = choice.delta.content {
                            if !content.is_empty() {
                                evs.push(Ok(StreamEvent::ContentDelta(content.clone())));
                            }
                        }
                        if let Some(ref tool_call_deltas) = choice.delta.tool_calls {
                            for tc in tool_call_deltas {
                                if tc.id.is_some() && tc.function.as_ref().and_then(|f| f.name.as_ref()).is_some() {
                                    evs.push(Ok(StreamEvent::ToolCallStart {
                                        index: tc.index,
                                        id: tc.id.clone().unwrap_or_default(),
                                        name: tc.function.as_ref().unwrap().name.clone().unwrap_or_default(),
                                    }));
                                }
                                if tc.function.as_ref().and_then(|f| f.arguments.as_ref()).is_some() {
                                    evs.push(Ok(StreamEvent::ToolCallDelta {
                                        index: tc.index,
                                        arguments: tc.function.as_ref().unwrap().arguments.clone().unwrap_or_default(),
                                    }));
                                }
                            }
                        }
                        if let Some(ref reason) = choice.finish_reason {
                            evs.push(Ok(StreamEvent::Done {
                                finish_reason: Some(reason.clone()),
                            }));
                        }
                    }
                    if evs.is_empty() {
                        evs.push(Ok(StreamEvent::ContentDelta(String::new())));
                    }
                    evs
                }
                Err(e) => vec![Err(e)],
            };
            events
        })
        .map(futures::stream::iter)
        .flatten()
        .filter(|result| {
            let keep = match result {
                Ok(StreamEvent::ContentDelta(s)) => !s.is_empty(),
                Ok(_) => true,
                Err(_) => true,
            };
            std::future::ready(keep)
        });
```

**Step 3: Update doc comment**

Update the doc comment (lines 407-414) to reflect the new behavior:
```rust
    /// Stream one turn of the agent loop, yielding `StreamEvent`s as they arrive.
    ///
    /// The caller (ChatRepl) iterates the returned stream to handle content deltas,
    /// tool call starts, tool call argument deltas, and completion events in real-time.
    /// After the stream completes, the caller should check for tool calls and loop back
    /// if necessary.
    ///
    /// Returns a stream of `Result<StreamEvent, RuntimeError>`.
```

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: PASS (compilation may fail in chat/mod.rs until Task 5)

**Step 5: Commit**

```bash
git add crates/runtime/src/agent/mod.rs
git commit -m "feat(agent): refactor stream_turn to yield StreamEvent enum"
```

---

## Task 5: Rewrite `ChatRepl::run_turn_streaming()` for Multi-Step Tool Loop

**Files:**
- Modify: `crates/runtime/src/chat/mod.rs:55-97`
- Add imports for `StreamEvent`, `MessageRole`, `ToolCall`

**Step 1: Add import for StreamEvent**

At line 3, change:
```rust
use crate::agent::{Agent, AgentResponse};
```
To:
```rust
use crate::agent::{Agent, AgentResponse};
use crate::provider::StreamEvent;
```

**Step 2: Rewrite `run_turn_streaming`**

Replace lines 55-97 with:

```rust
    async fn run_turn_streaming(&mut self, input: &str) -> Result<AgentResponse, RuntimeError> {
        // Append user message first
        self.agent
            .append_message(&self.session_id, MessageRole::User, input)?;

        let mut tool_calls_made = Vec::new();

        // Multi-step streaming loop: stream → detect tools → execute → stream follow-up
        loop {
            // Show typing indicator
            print!("Assistant: ");
            let _ = std::io::stdout().flush();

            let mut stream = self.agent.stream_turn(self.session_id);
            let mut full_content = String::new();

            // Accumulate tool calls across the stream
            let mut pending_tool_calls: std::collections::HashMap<u32, crate::provider::ToolCall> =
                std::collections::HashMap::new();
            let mut stream_done = false;
            let mut finish_reason: Option<String> = None;

            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(StreamEvent::ContentDelta(delta)) => {
                        print!("{}", delta);
                        let _ = std::io::stdout().flush();
                        full_content.push_str(&delta);
                    }
                    Ok(StreamEvent::ToolCallStart { index, id, name }) => {
                        println!("\n⚙ Running {}...", name);
                        let _ = std::io::stdout().flush();
                        pending_tool_calls.insert(
                            index,
                            crate::provider::ToolCall {
                                id,
                                tool_type: "function".to_string(),
                                function: crate::provider::FunctionCall {
                                    name: name.clone(),
                                    arguments: String::new(),
                                },
                            },
                        );
                        tool_calls_made.push(name);
                    }
                    Ok(StreamEvent::ToolCallDelta { index, arguments }) => {
                        if let Some(tc) = pending_tool_calls.get_mut(&index) {
                            tc.function.arguments.push_str(&arguments);
                        }
                    }
                    Ok(StreamEvent::ToolCallComplete { index: _ }) => {
                        // Arguments are complete; nothing special to do here
                    }
                    Ok(StreamEvent::Done { finish_reason: reason }) => {
                        stream_done = true;
                        finish_reason = reason;
                    }
                    Err(e) => {
                        println!();
                        return Err(e);
                    }
                }
            }

            // Final newline after streaming completes
            println!();

            // Persist the assistant's text response (even if empty during tool calls)
            if !full_content.is_empty() {
                self.agent
                    .append_assistant_message(&self.session_id, &full_content)?;
            }

            // If we have pending tool calls, execute them and loop back
            if !pending_tool_calls.is_empty() {
                // Build a list of completed tool calls sorted by index
                let mut completed_calls: Vec<crate::provider::ToolCall> = pending_tool_calls
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|(_, v)| v)
                    .collect();
                completed_calls.sort_by(|a, b| a.id.cmp(&b.id));

                // Append assistant's tool call decision to session
                let calls_summary: Vec<String> = completed_calls
                    .iter()
                    .map(|tc| format!("{}({})", tc.function.name, tc.function.arguments))
                    .collect();
                self.agent.append_message(
                    &self.session_id,
                    MessageRole::Assistant,
                    &format!("[tool_calls: {}]", calls_summary.join(", ")),
                )?;

                // Execute each tool call
                for tc in &completed_calls {
                    let params: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Object(Default::default()));

                    let tool_result = tokio::time::timeout(
                        std::time::Duration::from_secs(120),
                        self.agent.execute_tool(&self.session_id, &tc.function.name, params),
                    )
                    .await
                    .map_err(|_| RuntimeError::TimeoutError { duration_secs: 120 })?;

                    // execute_tool already appends to session; just handle error
                    if let Err(e) = tool_result {
                        eprintln!("Tool error: {}", e);
                    }
                }

                // Loop back to stream the follow-up response from LLM
                continue;
            }

            // No tool calls — final response
            return Ok(AgentResponse {
                content: full_content,
                tool_calls_made,
                turns_used: self.agent.turns_used(),
                session_id: self.session_id,
            });
        }
    }
```

**Step 3: Add `FunctionCall` to provider imports if not already public**

Ensure `FunctionCall` is pub in `provider/mod.rs` (it already is at line 53).

**Step 4: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/runtime/src/chat/mod.rs
git commit -m "feat(chat): multi-step streaming loop with tool call execution"
```

---

## Task 6: Add `ToolCallComplete` Detection in `Agent::stream_turn()`

**Files:**
- Modify: `crates/runtime/src/agent/mod.rs`

**Step 1: Detect when a tool call's arguments are complete**

In the `.map()` closure of `stream_turn`, we need to detect when a chunk carries a `finish_reason: "tool_calls"` or when the stream ends. OpenAI sends `finish_reason: "tool_calls"` when all tool calls are complete. Anthropic sends `message_stop` which maps to `Done`.

Update the `Done` event handling in `chat/mod.rs` to also emit `ToolCallComplete` for all pending indices when the stream ends with `finish_reason == Some("tool_calls")`.

Actually, a simpler approach: in `chat/mod.rs`, when we receive `StreamEvent::Done { finish_reason: Some("tool_calls") }`, we can emit `ToolCallComplete` for all known indices. But since the stream is ending, we can just treat the stream end as "all tool calls complete" and execute them.

The current Task 5 code already handles this: it collects all `ToolCallStart` and `ToolCallDelta` events, and when the stream ends with pending tool calls, it executes them. We don't strictly need `ToolCallComplete` events in the consumer if we just execute at stream end.

However, for completeness, add `ToolCallComplete` emission when `finish_reason == "tool_calls"`:

In `agent/mod.rs` inside the `.map()` closure, when handling `finish_reason`:
```rust
                        if let Some(ref reason) = choice.finish_reason {
                            if reason == "tool_calls" {
                                // Emit ToolCallComplete for each tool call we saw.
                                // Since we don't track indices here, we emit a generic one.
                                evs.push(Ok(StreamEvent::ToolCallComplete { index: 0 }));
                            }
                            evs.push(Ok(StreamEvent::Done {
                                finish_reason: Some(reason.clone()),
                            }));
                        }
```

**Step 2: Run tests**

Run: `cargo test --workspace`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/runtime/src/agent/mod.rs
git commit -m "feat(agent): emit ToolCallComplete on finish_reason=tool_calls"
```

---

## Task 7: Clippy and Final Verification

**Step 1: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings, no errors

**Step 2: Run full test suite**

Run: `cargo test --workspace`
Expected: All tests pass

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat(runtime): integrate streaming with tool calls

- Extend DeltaMessage with tool_calls field and add ToolCallDelta, FunctionCallDelta
- Add StreamEvent enum: ContentDelta, ToolCallStart, ToolCallDelta, ToolCallComplete, Done
- Update OpenAI SSE parser to extract tool_call deltas from chunks
- Update Anthropic SSE parser to extract tool_use content blocks
- Refactor Agent::stream_turn() to yield StreamEvent instead of raw strings
- Rewrite ChatRepl::run_turn_streaming() as multi-step loop:
  stream → accumulate tool calls → execute → stream follow-up
- Streaming path now correctly returns tool_calls_made Vec"
```

---

## Testing Strategy Summary

| Component | Tests Added |
|-----------|-------------|
| `provider/mod.rs` | Existing tests for provider creation still pass |
| `provider/openai.rs` | `test_parse_openai_sse_tool_call_delta`, `test_parse_openai_sse_tool_call_arguments_delta`, `test_parse_openai_sse_finish_reason_stop` |
| `provider/anthropic.rs` | `test_parse_anthropic_tool_use_start`, `test_parse_anthropic_input_json_delta` |
| `agent/mod.rs` | Existing mock tests use non-streaming; no new tests needed for stream_turn signature change |
| `chat/mod.rs` | Existing REPL tests use non-streaming; manual integration test recommended |

## Rollback Plan

If issues arise:
1. Revert `chat/mod.rs` to old `run_turn_streaming()` (returns empty `tool_calls_made`)
2. Revert `agent/mod.rs` `stream_turn()` to yield `String`
3. Revert `provider/mod.rs` to remove `tool_calls` from `DeltaMessage`
4. Revert `provider/openai.rs` and `provider/anthropic.rs` SSE parsers

Each task is independently commit-able for easy bisect.
