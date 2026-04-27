# Hermes Rust Windows — Production-Ready v1 Plan

> **30-Day Roadmap** | Single Developer | Windows-First | Cross-Platform Compile
> 
> **Goal**: Transform hermes-rust-win from a functional prototype into a production-ready, user-lovable AI Agent CLI.

---

## Executive Summary

### Current State
- **47 Rust files**, **12,283 lines**, **305 tests**, **0 clippy warnings**
- Working: CLI parsing (40+ commands), basic agent loop, OpenAI/Anthropic providers, session DB (SQLite), chat REPL
- Binary: 21.1MB debug build

### Critical Issues (12)
1. Streaming + Tool Calls don't work together (streaming returns empty `tool_calls`)
2. No tool execution visual feedback (user sees nothing while tools run)
3. No token counting (chars/4 heuristic, ~30% inaccurate)
4. No API retry/backoff (429/500/503 crash the app)
5. No signal handling (Ctrl+C kills dirty)
6. `/model` command returns "not yet supported"
7. `/compact` command returns "not yet implemented"
8. No markdown rendering in streaming output
9. Only 2 LLM providers with real implementations (need 10+)
10. Context compression is a stub (truncation only, no summarization)
11. System prompt is hardcoded empty
12. Tool arguments silently fail on JSON parse errors

### Python Features to Port (10)
1. KawaiiSpinner + tool execution feedback (`display.py`)
2. Markdown rendering in streaming (`display.py`)
3. Token usage + cost tracking (`insights.py`, `usage_pricing.py`)
4. Context compression with LLM summarization (`context_compressor.py`)
5. Multi-credential pool with failover (`credential_pool.py`)
6. Smart model routing (`smart_model_routing.py`)
7. Model metadata + pricing (`model_metadata.py`)
8. Skills system (`skill_utils.py`, `skill_commands.py`)
9. System prompt builder (`prompt_builder.py`)
10. Prompt caching optimization (`prompt_caching.py`)

### Additional Providers Needed
Ollama, Google Gemini, DeepSeek, Groq, Mistral, Cohere, OpenRouter, Azure OpenAI, local servers (LM Studio/vLLM)

---

## Architecture Overview

```
hermes-cli/
├── crates/
│   ├── cli/              # Binary entry point (main.rs)
│   ├── cli-core/         # CLI parsing, commands, config, auth
│   │   ├── lib.rs        # 30+ subcommands, handle_chat() wiring
│   │   ├── commands.rs   # Command handlers (stubs + real impls)
│   │   ├── config.rs     # Config YAML loading
│   │   ├── auth.rs       # AuthStore (YAML-based credentials)
│   │   └── ...
│   ├── common/           # Shared types
│   │   ├── types.rs      # Provider enum (20+), Model, Credentials
│   │   └── provider.rs   # URL detection, API mode
│   ├── runtime/          # Core agent runtime
│   │   ├── agent/mod.rs  # Agent loop, run_turn(), stream_turn()
│   │   ├── chat/mod.rs   # ChatRepl, slash commands
│   │   ├── provider/     # LLM providers (openai.rs, anthropic.rs)
│   │   ├── tool/         # Tool trait + registry + implementations
│   │   ├── context/      # Token estimation, truncation
│   │   └── gateway/      # Platform adapters (stubs)
│   └── session-db/       # SQLite session persistence
│       ├── store.rs      # SessionStore CRUD
│       └── models.rs     # Session, Message structs
```

### Key Files for Each Issue

| Issue | Primary File(s) | Lines |
|-------|-----------------|-------|
| 1. Streaming + Tool Calls | `runtime/src/chat/mod.rs:93`, `runtime/src/agent/mod.rs:415-486` | 93, 415-486 |
| 2. Tool Visual Feedback | `runtime/src/chat/mod.rs`, `runtime/src/agent/mod.rs:298-329` | New module |
| 3. Token Counting | `runtime/src/context/token_est.rs` | 1-291 |
| 4. API Retry/Backoff | `runtime/src/provider/openai.rs`, `runtime/src/provider/anthropic.rs` | 105-203, 155-439 |
| 5. Signal Handling | `cli-core/src/lib.rs:1041-1177` | 1041-1177 |
| 6. /model Command | `runtime/src/chat/mod.rs:150-166` | 150-166 |
| 7. /compact Command | `runtime/src/chat/mod.rs:178-196` | 178-196 |
| 8. Markdown Rendering | `runtime/src/chat/mod.rs:55-97` | 55-97 |
| 9. Provider Expansion | `runtime/src/provider/mod.rs:124-144` | 124-144 |
| 10. Context Compression | `runtime/src/context/token_est.rs:82-129` | 82-129 |
| 11. System Prompt | `runtime/src/agent/mod.rs:29-39`, `cli-core/src/lib.rs:1115-1122` | 29-39 |
| 12. Tool JSON Errors | `runtime/src/agent/mod.rs:302-303` | 302-303 |

---

## Wave 1: Fix Critical Bugs + Core Improvements (Days 1-7)

> **Theme**: Stability, reliability, and user trust. Every bug fix must include tests.

---

### Day 1 (Monday) — API Resilience + Error Handling

#### Task 1.1: Add Exponential Backoff Retry to Providers
- **Estimated Hours**: 6h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/openai.rs`, `crates/runtime/src/provider/anthropic.rs`, `crates/runtime/src/error.rs`
- **Dependencies**: None

**What to do**:
- Create `crates/runtime/src/provider/retry.rs` with `RetryPolicy` struct:
  - Configurable: max_retries (default 3), base_delay_ms (default 500), max_delay_ms (default 30000), retryable_statuses: [429, 500, 502, 503]
  - Exponential backoff with jitter: `delay = min(base_delay * 2^attempt + random(0-1000ms), max_delay)`
  - Respect `Retry-After` header on 429
- Wrap `chat_completion()` and `chat_completion_stream()` in both OpenAI and Anthropic providers with retry logic
- Add new `RuntimeError` variants: `RetryExhausted { attempts: u32, last_error: String }`, `RateLimitError { retry_after: Option<u64> }`
- Update `StreamChunk` to carry `finish_reason` and `tool_calls` fields for streaming path

**Verification**:
- `cargo test -p hermes-runtime test_retry_policy`
- `cargo test -p hermes-runtime test_retry_exhaustion`
- `cargo test -p hermes-runtime test_rate_limit_header`
- `cargo clippy -p hermes-runtime -- -D warnings`

**Commit**: `feat(runtime): add exponential backoff retry to LLM providers`

---

#### Task 1.2: Fix Tool Argument JSON Parse Errors
- **Estimated Hours**: 3h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/agent/mod.rs:302-303`, `crates/runtime/src/tool/mod.rs`
- **Dependencies**: None

**What to do**:
- In `agent/mod.rs:302-303`, replace `unwrap_or` with explicit error handling:
  - Parse JSON with `serde_json::from_str`, return `RuntimeError::ToolError` with detailed message on failure
  - Include the raw arguments string and the parse error in the error message
  - Store the error as a tool result so the LLM can see what went wrong and retry
- Add `ToolRegistry::validate_args(name, params) -> Result<(), RuntimeError>` that checks args against JSON schema before dispatch
- Add tests for malformed JSON, missing required fields, wrong types

**Verification**:
- `cargo test -p hermes-runtime test_tool_json_parse_error`
- `cargo test -p hermes-runtime test_tool_missing_required_field`
- `cargo test -p hermes-runtime test_tool_wrong_type`
- `cargo clippy -p hermes-runtime -- -D warnings`

**Commit**: `fix(runtime): handle tool argument JSON parse errors gracefully`

---

### Day 2 (Tuesday) — Signal Handling + Session Safety

#### Task 1.3: Add Ctrl+C Signal Handling
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/cli-core/src/lib.rs:1041-1177`, `crates/runtime/src/chat/mod.rs`
- **Dependencies**: None

**What to do**:
- Add `tokio::signal` dependency to `cli-core/Cargo.toml`
- In `handle_chat()`, wrap the REPL loop with `tokio::select!` listening for `ctrl_c()`
- On Ctrl+C:
  1. Print "\nReceived interrupt, saving session..."
  2. Save current session state (messages already persisted per-turn)
  3. Print "Session saved. Goodbye!"
  4. Exit cleanly with code 0
- Add `ChatRepl::graceful_shutdown()` method for explicit cleanup
- Handle Ctrl+C during tool execution: cancel the tool future, save partial state

**Verification**:
- `cargo test -p hermes-cli-core test_ctrl_c_handling`
- `cargo test -p hermes-runtime test_graceful_shutdown`
- Manual test: run `hermes chat`, press Ctrl+C, verify clean exit
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(cli-core): add graceful Ctrl+C signal handling`

---

#### Task 1.4: Implement /model Command
- **Estimated Hours**: 3h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/chat/mod.rs:150-166`, `crates/runtime/src/agent/mod.rs`
- **Dependencies**: None

**What to do**:
- Add `Agent::set_model(&mut self, model: String)` method
- Update `/model` command handler:
  - `/model` (no args) → show current model + provider + base_url
  - `/model <name>` → validate model exists in known models list, update agent model, print confirmation
  - `/model --list` → list all available models with provider and pricing hint
- Add `ModelRegistry` in `common/src/types.rs` or new `common/src/models.rs`:
  - Static list of known models with metadata: name, provider, context_length, supports_vision, supports_tools
  - Validate model name against registry

**Verification**:
- `cargo test -p hermes-runtime test_model_command_show`
- `cargo test -p hermes-runtime test_model_command_change`
- `cargo test -p hermes-runtime test_model_command_list`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): implement /model command with model registry`

---

### Day 3 (Wednesday) — Streaming + Tool Calls Integration

#### Task 1.5: Fix Streaming + Tool Calls
- **Estimated Hours**: 8h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/chat/mod.rs:55-97`, `crates/runtime/src/agent/mod.rs:415-486`, `crates/runtime/src/provider/mod.rs`
- **Dependencies**: Task 1.1 (retry logic, as streaming needs robust error handling)

**What to do**:
- Extend `StreamChunk` and `DeltaMessage` to carry `tool_calls` field (OpenAI streaming format):
  ```rust
  #[derive(Debug, Clone, Deserialize, Default)]
  pub struct DeltaMessage {
      pub content: Option<String>,
      pub tool_calls: Option<Vec<ToolCallDelta>>,
  }
  
  #[derive(Debug, Clone, Deserialize)]
  pub struct ToolCallDelta {
      pub index: u32,
      pub id: Option<String>,
      pub tool_type: Option<String>,
      pub function: Option<FunctionCallDelta>,
  }
  
  #[derive(Debug, Clone, Deserialize)]
  pub struct FunctionCallDelta {
      pub name: Option<String>,
      pub arguments: Option<String>,
  }
  ```
- Update `OpenAiProvider::chat_completion_stream()` to parse `tool_calls` deltas from SSE
- Update `AnthropicProvider` to handle `content_block_start` with `tool_use` type
- Rewrite `Agent::stream_turn()` to:
  1. Stream content deltas to caller (ChatRepl prints them)
  2. Accumulate full response including tool_calls
  3. If tool_calls detected, yield a special `StreamEvent::ToolCalls(Vec<ToolCall>)` variant
  4. Execute tools, then stream the final response
- Update `ChatRepl::run_turn_streaming()` to handle the new stream events:
  - Content deltas → print immediately
  - Tool calls → print "Running tools: ..." feedback, execute, print results
  - Final response → stream as normal

**Verification**:
- `cargo test -p hermes-runtime test_streaming_tool_calls`
- `cargo test -p hermes-runtime test_streaming_content_only`
- `cargo test -p hermes-runtime test_streaming_mixed_content_and_tools`
- Mock provider test that simulates streaming tool call sequence
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): integrate streaming with tool calls`

---

### Day 4 (Thursday) — Display Polish

#### Task 1.6: Add Tool Execution Visual Feedback
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `rust-patterns`, `frontend-ui-ux`
- **Files**: New `crates/runtime/src/display/mod.rs`, `crates/runtime/src/chat/mod.rs`
- **Dependencies**: Task 1.5 (streaming tool calls)

**What to do**:
- Create `crates/runtime/src/display/mod.rs` with `DisplayEngine`:
  - `print_tool_start(name: &str, args: &serde_json::Value)` → "⚙️  Running `tool_name`..."
  - `print_tool_result(name: &str, result: &ToolOutput, duration_ms: u64)` → "✅ `tool_name` completed (1.2s)" or "❌ `tool_name` failed"
  - `print_tool_progress(name: &str, message: &str)` for long-running tools
  - `print_token_usage(input: u32, output: u32, cost: Option<f64>)`
  - Support `--quiet` flag (no output)
  - Support `--verbose` flag (full tool output preview)
- Add Windows-compatible spinner using `crossterm` or simple ASCII animation:
  - `\ | / -` cycle printed with `\r` carriage return
  - Use `tokio::time::interval` for animation
  - Stop spinner when tool completes
- Integrate into `Agent::run_turn()` and `ChatRepl::run_turn_streaming()`

**Verification**:
- `cargo test -p hermes-runtime test_tool_feedback_display`
- `cargo test -p hermes-runtime test_spinner_animation`
- `cargo test -p hermes-runtime test_quiet_mode`
- Manual test: run tool, verify visual feedback
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add tool execution visual feedback and spinner`

---

#### Task 1.7: Add Markdown Rendering in Streaming Output
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/display/mod.rs`, `crates/runtime/src/chat/mod.rs`
- **Dependencies**: Task 1.6

**What to do**:
- Add `pulldown-cmark` dependency to `runtime/Cargo.toml` for markdown parsing
- Create `MarkdownRenderer` in `display/mod.rs`:
  - Parse markdown tokens from accumulated content
  - Apply terminal formatting using `crossterm`:
    - Headers: bold + color
    - Code blocks: dimmed background hint
    - Inline code: cyan
    - Bold: bright white
    - Links: blue + underline
    - Lists: bullet points
  - Stream-friendly: render incrementally as tokens arrive
  - Fallback to plain text if terminal doesn't support colors (detect via `crossterm::terminal::supports_color`)
- Update `ChatRepl::run_turn_streaming()` to use `MarkdownRenderer` when `DisplayConfig.skin != "plain"`

**Verification**:
- `cargo test -p hermes-runtime test_markdown_rendering`
- `cargo test -p hermes-runtime test_markdown_code_blocks`
- `cargo test -p hermes-runtime test_plain_text_fallback`
- Manual test: ask LLM to output markdown, verify rendering
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add markdown rendering in streaming output`

---

### Day 5 (Friday) — Token Counting + System Prompts

#### Task 1.8: Implement Accurate Token Counting
- **Estimated Hours**: 6h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/context/token_est.rs`, `crates/runtime/src/agent/mod.rs`
- **Dependencies**: None

**What to do**:
- Add `tiktoken-rs` or `tokenizers` crate dependency for accurate token counting
- Create `Tokenizer` trait in `context/mod.rs`:
  - `count_tokens(text: &str) -> usize`
  - `count_messages(messages: &[ChatMessage]) -> usize`
  - `model() -> &str`
- Implement `TiktokenTokenizer` for OpenAI models (gpt-4, gpt-3.5)
- Implement `CharHeuristicTokenizer` as fallback for unknown models (current behavior)
- Add `TokenizerRegistry` that maps model names to tokenizers
- Update `AgentConfig` to include `tokenizer: String`
- Update `Agent::build_messages()` to use accurate token counting for truncation
- Add token usage tracking to `AgentResponse`:
  ```rust
  pub struct TokenUsage {
      pub input_tokens: u32,
      pub output_tokens: u32,
      pub total_tokens: u32,
  }
  ```
- Parse `usage` field from OpenAI/Anthropic responses

**Verification**:
- `cargo test -p hermes-runtime test_tokenizer_accuracy`
- `cargo test -p hermes-runtime test_token_usage_tracking`
- `cargo test -p hermes-runtime test_truncation_with_real_tokens`
- Compare token counts with known values from Python tiktoken
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): implement accurate token counting with tiktoken`

---

#### Task 1.9: Add System Prompt Builder
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/context/prompt_builder.rs`, `crates/runtime/src/agent/mod.rs`, `crates/cli-core/src/config.rs`
- **Dependencies**: None

**What to do**:
- Create `crates/runtime/src/context/prompt_builder.rs` with `SystemPromptBuilder`:
  - `new() -> Self`
  - `with_identity(name: &str, version: &str)` → "You are Hermes, an AI agent CLI..."
  - `with_capabilities(tools: &[&str])` → list available tools
  - `with_date()` → include current date/time
  - `with_os_info()` → "Running on Windows 11"
  - `with_cwd()` → current working directory
  - `with_custom(text: &str)` → user-defined prompt
  - `build() -> String`
- Update `AgentConfig::default()` to use the builder instead of empty string
- Update `cli-core/src/config.rs` `AgentConfig` to support `system_prompt_template: String`
- Add `--system-prompt` CLI flag to override
- Add `/system` REPL command to view/update system prompt

**Verification**:
- `cargo test -p hermes-runtime test_prompt_builder_default`
- `cargo test -p hermes-runtime test_prompt_builder_custom`
- `cargo test -p hermes-runtime test_system_prompt_in_request`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add system prompt builder with context awareness`

---

### Day 6 (Saturday) — Buffer / Polish Day

#### Task 1.10: Implement /compact Command
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/chat/mod.rs:178-196`, `crates/runtime/src/context/mod.rs`
- **Dependencies**: Task 1.8 (token counting)

**What to do**:
- Implement basic context compaction in `/compact` command:
  - Count total tokens in session
  - If under threshold (e.g., 50% of max), print "No compaction needed"
  - If over threshold:
    1. Keep system prompt
    2. Keep last N messages (where N = max_turns / 2)
    3. Summarize middle messages into a single "context summary" message
    4. For now, use simple truncation (summarization comes in Wave 3)
- Add `SessionStore::truncate_messages(session_id, keep_count)` method
- Add `SessionStore::get_token_count(session_id) -> usize` using the tokenizer
- Print before/after token counts to show savings

**Verification**:
- `cargo test -p hermes-runtime test_compact_noop`
- `cargo test -p hermes-runtime test_compact_truncation`
- `cargo test -p hermes-runtime test_compact_preserves_system`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): implement /compact command with token-aware truncation`

---

#### Task 1.11: Code Review + Test Fixes
- **Estimated Hours**: 4h
- **Agent Category**: `oracle`
- **Skills**: `review-work`
- **Files**: All changed files
- **Dependencies**: All Wave 1 tasks

**What to do**:
- Run full test suite: `cargo test --workspace`
- Run clippy: `cargo clippy --workspace -- -D warnings`
- Fix any regressions
- Ensure all new code has tests
- Update documentation comments
- Review for `unwrap()` in production code

**Verification**:
- `cargo test --workspace` → all 305+ tests pass
- `cargo clippy --workspace -- -D warnings` → zero warnings
- `cargo build --release` → success

**Commit**: `chore: wave 1 cleanup, test fixes, clippy compliance`

---

### Day 7 (Sunday) — Rest / Documentation

- Write wave 1 retrospective
- Document new features in README
- Update CHANGELOG

---

## Wave 2: Provider Expansion + UI Polish (Days 8-14)

> **Theme**: Make Hermes work with any LLM provider. Polish the user experience.

---

### Day 8 (Monday) — OpenAI-Compatible Provider Framework

#### Task 2.1: Refactor Provider Architecture for Multi-Provider Support
- **Estimated Hours**: 6h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`, `architecture-designer`
- **Files**: `crates/runtime/src/provider/mod.rs`, `crates/runtime/src/provider/openai.rs`
- **Dependencies**: None

**What to do**:
- Create `crates/runtime/src/provider/openai_compatible.rs`:
  - Generic `OpenAiCompatibleProvider` that works with any OpenAI-compatible API
  - Configurable: base_url, auth_header_name, auth_header_prefix, model_param_name
  - Handles SSE streaming, tool calls, error responses
- Refactor `OpenAiProvider` to use the compatible provider internally
- Create `ProviderConfig` struct:
  ```rust
  pub struct ProviderConfig {
      pub name: String,
      pub base_url: String,
      pub auth_type: AuthType,
      pub default_model: String,
      pub supports_streaming: bool,
      pub supports_tools: bool,
      pub headers: HashMap<String, String>,
  }
  ```
- Update `create_provider()` to use a registry pattern:
  ```rust
  pub fn create_provider(config: &ProviderConfig, api_key: &str) -> Box<dyn LlmProvider>
  ```
- Add `ProviderRegistry` with static configs for all known providers

**Verification**:
- `cargo test -p hermes-runtime test_openai_compatible_provider`
- `cargo test -p hermes-runtime test_provider_registry`
- `cargo test -p hermes-runtime test_all_providers_construct`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `refactor(runtime): extract OpenAI-compatible provider framework`

---

### Day 9 (Tuesday) — New Providers: Ollama, DeepSeek, Groq

#### Task 2.2: Implement Ollama Provider
- **Estimated Hours**: 3h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/ollama.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `OllamaProvider` using `OpenAiCompatibleProvider`:
  - Base URL: `http://localhost:11434/v1`
  - No auth header needed
  - Model list endpoint: `/api/tags`
  - Streaming: OpenAI-compatible SSE
- Add `Provider::Ollama` to registry
- Add auto-detection: try localhost:11434 on startup, suggest Ollama if responding

**Verification**:
- `cargo test -p hermes-runtime test_ollama_provider_creation`
- `cargo test -p hermes-runtime test_ollama_no_auth`
- Mock server test for Ollama API
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add Ollama provider support`

---

#### Task 2.3: Implement DeepSeek Provider
- **Estimated Hours**: 2h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/deepseek.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `DeepSeekProvider` using `OpenAiCompatibleProvider`:
  - Base URL: `https://api.deepseek.com/v1`
  - Auth: `Authorization: Bearer {key}`
  - Default model: `deepseek-chat`
  - Supports streaming and tool calls
- Add `Provider::DeepSeek` to registry

**Verification**:
- `cargo test -p hermes-runtime test_deepseek_provider`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add DeepSeek provider support`

---

#### Task 2.4: Implement Groq Provider
- **Estimated Hours**: 2h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/groq.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `GroqProvider` using `OpenAiCompatibleProvider`:
  - Base URL: `https://api.groq.com/openai/v1`
  - Auth: `Authorization: Bearer {key}`
  - Default model: `llama-3.1-70b-versatile`
  - Very fast inference, good for tool calls

**Verification**:
- `cargo test -p hermes-runtime test_groq_provider`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add Groq provider support`

---

### Day 10 (Wednesday) — Google Gemini + Azure

#### Task 2.5: Implement Google Gemini Provider
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/gemini.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `GeminiProvider` (NOT OpenAI-compatible, uses Google's API):
  - Base URL: `https://generativelanguage.googleapis.com/v1beta`
  - Auth: `key={api_key}` as query parameter
  - Endpoint: `/models/{model}:generateContent` and `:streamGenerateContent`
  - Request format: `{ contents: [{ role: "user", parts: [{ text: "..." }] }] }`
  - Response format: `{ candidates: [{ content: { parts: [{ text: "..." }] } }] }`
  - Tool format: Google function declarations
  - Streaming: Server-sent events with different format
- Add request/response mappers to convert between internal `ChatMessage` format and Gemini format
- Add tool call mapping (Gemini uses `functionCalls` in parts)

**Verification**:
- `cargo test -p hermes-runtime test_gemini_request_format`
- `cargo test -p hermes-runtime test_gemini_streaming`
- `cargo test -p hermes-runtime test_gemini_tool_calls`
- Mock server test for Gemini API
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add Google Gemini provider support`

---

#### Task 2.6: Implement Azure OpenAI Provider
- **Estimated Hours**: 3h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/azure.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `AzureProvider` using `OpenAiCompatibleProvider`:
  - Base URL: user-provided (e.g., `https://{resource}.openai.azure.com/openai/deployments/{deployment}`)
  - Auth: `api-key: {key}` header (NOT Bearer)
  - Endpoint: `/chat/completions?api-version=2024-02-01`
  - Supports streaming and tool calls
- Add `Provider::Azure` to registry
- Update `AuthStore` to support Azure-specific fields: `resource_name`, `deployment_name`

**Verification**:
- `cargo test -p hermes-runtime test_azure_provider`
- `cargo test -p hermes-runtime test_azure_api_key_header`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add Azure OpenAI provider support`

---

### Day 11 (Thursday) — OpenRouter + Local Servers

#### Task 2.7: Implement OpenRouter Provider
- **Estimated Hours**: 3h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/openrouter.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `OpenRouterProvider` using `OpenAiCompatibleProvider`:
  - Base URL: `https://openrouter.ai/api/v1`
  - Auth: `Authorization: Bearer {key}`
  - Extra headers: `HTTP-Referer`, `X-Title`
  - Default model: `openai/gpt-4o`
  - Supports model routing via `model` parameter
  - Supports `provider` object for routing preferences

**Verification**:
- `cargo test -p hermes-runtime test_openrouter_provider`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add OpenRouter provider support`

---

#### Task 2.8: Implement Local Server Providers (LM Studio, vLLM)
- **Estimated Hours**: 3h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/local.rs`
- **Dependencies**: Task 2.1

**What to do**:
- Implement `LocalProvider` using `OpenAiCompatibleProvider`:
  - Base URL: `http://localhost:1234/v1` (LM Studio) or `http://localhost:8000/v1` (vLLM)
  - No auth required
  - Auto-detect: try common ports on startup
  - Support model listing via `/v1/models`
- Add `Provider::Local` to registry
- Add `hermes doctor` check for local servers

**Verification**:
- `cargo test -p hermes-runtime test_local_provider`
- `cargo test -p hermes-runtime test_local_auto_detect`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add local server provider support (LM Studio, vLLM)`

---

### Day 12 (Friday) — Model Metadata + Pricing

#### Task 2.9: Add Model Metadata and Pricing Database
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `rust-patterns`
- **Files**: `crates/common/src/model_metadata.rs` (new), `crates/runtime/src/context/token_est.rs`
- **Dependencies**: Task 1.8 (token counting)

**What to do**:
- Create `crates/common/src/model_metadata.rs` with `ModelMetadata`:
  ```rust
  pub struct ModelMetadata {
      pub name: String,
      pub provider: Provider,
      pub context_length: u32,
      pub max_output_tokens: u32,
      pub supports_vision: bool,
      pub supports_tools: bool,
      pub supports_streaming: bool,
      pub input_price_per_1k: f64,   // USD
      pub output_price_per_1k: f64,  // USD
      pub description: String,
  }
  ```
- Create static `MODEL_METADATA` map with 50+ models:
  - OpenAI: gpt-4o, gpt-4o-mini, gpt-4-turbo, o1, o1-mini
  - Anthropic: claude-sonnet-4, claude-opus-4, claude-haiku-3
  - Google: gemini-2.5-pro, gemini-2.0-flash
  - DeepSeek: deepseek-chat, deepseek-reasoner
  - Groq: llama-3.1-70b, mixtral-8x7b
  - Ollama: llama3, mistral, codellama
  - Local: various
- Add `ModelMetadataRegistry`:
  - `get(model_name) -> Option<&ModelMetadata>`
  - `list_by_provider(provider) -> Vec<&ModelMetadata>`
  - `estimate_cost(input_tokens, output_tokens, model) -> f64`
  - `get_context_length(model) -> u32`
- Update `AgentConfig.max_context_tokens` to use metadata instead of hardcoded 128k
- Update `Agent` to track and report costs

**Verification**:
- `cargo test -p hermes-common test_model_metadata_lookup`
- `cargo test -p hermes-common test_cost_estimation`
- `cargo test -p hermes-common test_context_length_lookup`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(common): add model metadata and pricing database`

---

#### Task 2.10: Add Token Usage + Cost Tracking
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/agent/mod.rs`, `crates/runtime/src/display/mod.rs`
- **Dependencies**: Task 2.9

**What to do**:
- Extend `AgentResponse` with full usage tracking:
  ```rust
  pub struct UsageReport {
      pub input_tokens: u32,
      pub output_tokens: u32,
      pub total_tokens: u32,
      pub estimated_cost_usd: f64,
      pub model: String,
      pub provider: String,
      pub duration_ms: u64,
  }
  ```
- Track per-session cumulative usage in `SessionStore`:
  - Add `sessions.total_input_tokens`, `total_output_tokens`, `total_cost` columns
  - Update on each turn
- Add `/usage` REPL command to show session usage
- Add `hermes insights` command (stub for now, full impl in Wave 3)
- Display token usage after each response: "(1,234 tokens, ~$0.0042)"

**Verification**:
- `cargo test -p hermes-runtime test_usage_tracking`
- `cargo test -p hermes-runtime test_session_cumulative_usage`
- `cargo test -p hermes-runtime test_usage_command`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add token usage and cost tracking`

---

### Day 13 (Saturday) — UI Polish + Configuration

#### Task 2.11: Add Display Configuration + Themes
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `frontend-ui-ux`, `rust-patterns`
- **Files**: `crates/cli-core/src/config.rs`, `crates/runtime/src/display/mod.rs`
- **Dependencies**: Task 1.6, Task 1.7

**What to do**:
- Extend `DisplayConfig`:
  ```rust
  pub struct DisplayConfig {
      pub streaming: bool,
      pub compact: bool,
      pub show_reasoning: bool,
      pub skin: String,           // "default", "plain", "minimal", "rich"
      pub show_token_usage: bool,
      pub show_tool_feedback: bool,
      pub markdown_rendering: bool,
      pub spinner_style: String,  // "ascii", "dots", "none"
      pub color_scheme: String,   // "auto", "dark", "light", "none"
  }
  ```
- Implement theme system in `display/mod.rs`:
  - `default`: full colors, markdown, spinner
  - `plain`: no colors, no markdown, minimal output
  - `minimal`: colors but no markdown, simple spinner
  - `rich`: full colors, markdown, animations, syntax highlighting hints
- Load theme from config.yaml
- Add `--theme` CLI flag
- Add `/theme <name>` REPL command

**Verification**:
- `cargo test -p hermes-runtime test_theme_switching`
- `cargo test -p hermes-runtime test_color_scheme`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(cli-core): add display themes and configuration`

---

#### Task 2.12: Buffer / Integration Testing
- **Estimated Hours**: 4h
- **Agent Category**: `oracle`
- **Skills**: `testing-strategies`
- **Files**: All Wave 2 files
- **Dependencies**: All Wave 2 tasks

**What to do**:
- Integration test: create agent with each provider, verify construction
- Integration test: full chat flow with mock provider
- Run full test suite
- Fix any regressions
- Update provider documentation in README

**Verification**:
- `cargo test --workspace` → all pass
- `cargo clippy --workspace -- -D warnings` → zero warnings
- `cargo build --release` → success

**Commit**: `chore: wave 2 integration tests and cleanup`

---

### Day 14 (Sunday) — Rest / Documentation

- Document all new providers
- Write provider setup guide
- Update CHANGELOG

---

## Wave 3: Advanced Features (Days 15-21)

> **Theme**: Intelligence — make Hermes smart about context, routing, and skills.

---

### Day 15 (Monday) — Context Compression

#### Task 3.1: Implement LLM-Based Context Compression
- **Estimated Hours**: 8h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`, `architecture-designer`
- **Files**: `crates/runtime/src/context/compressor.rs` (new), `crates/runtime/src/context/mod.rs`
- **Dependencies**: Task 1.8 (token counting), Task 1.10 (compact command)

**What to do**:
- Create `crates/runtime/src/context/compressor.rs` with `ContextCompressor`:
  ```rust
  pub struct ContextCompressor {
      provider: Arc<dyn LlmProvider>,
      model: String,
      compression_ratio: f32,  // target: 0.5 = compress to 50%
  }
  ```
  - `compress(messages: &[ChatMessage], target_tokens: usize) -> Vec<ChatMessage>`:
    1. If total tokens <= target, return as-is
    2. Keep system prompt + last 4 messages uncompressed
    3. Group middle messages into chunks of ~4 messages each
    4. For each chunk, send to LLM with prompt: "Summarize this conversation concisely, preserving key facts and decisions:"
    5. Replace chunk with summary message
    6. Recurse if still over target
  - `summarize_chunk(chunk: &[ChatMessage]) -> Result<String, RuntimeError>`
  - Add `CompressionStrategy` enum: `Truncate`, `Summarize`, `Hybrid` (default)
- Update `/compact` command to use `CompressionStrategy::Summarize`
- Add `AgentConfig.compression_strategy` and `compression_threshold` (default: 0.8 of max_context)
- Add tests with mock provider that returns summaries

**Verification**:
- `cargo test -p hermes-runtime test_compress_noop`
- `cargo test -p hermes-runtime test_compress_summarize`
- `cargo test -p hermes-runtime test_compress_hybrid`
- `cargo test -p hermes-runtime test_compress_preserves_critical_messages`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add LLM-based context compression`

---

### Day 16 (Tuesday) — Smart Model Routing

#### Task 3.2: Implement Smart Model Routing
- **Estimated Hours**: 6h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`, `architecture-designer`
- **Files**: `crates/runtime/src/provider/routing.rs` (new), `crates/runtime/src/agent/mod.rs`
- **Dependencies**: Task 2.9 (model metadata)

**What to do**:
- Create `crates/runtime/src/provider/routing.rs` with `ModelRouter`:
  ```rust
  pub struct ModelRouter {
      primary: Arc<dyn LlmProvider>,
      fallbacks: Vec<Arc<dyn LlmProvider>>,
      routing_rules: Vec<RoutingRule>,
  }
  
  pub enum RoutingRule {
      ByComplexity { threshold: ComplexityScore, model: String },
      ByCost { max_cost_per_1k: f64, model: String },
      BySpeed { timeout_ms: u64, model: String },
      ByCapability { required: Capability, model: String },
  }
  
  pub enum Capability {
      Vision,
      Tools,
      Reasoning,
      LongContext,
  }
  ```
  - `route(request: &ChatRequest) -> Arc<dyn LlmProvider>`:
    - Analyze request complexity (message count, tool presence, image presence)
    - Match against routing rules
    - Return appropriate provider
  - `execute_with_fallback(request) -> Result<ChatResponse, RuntimeError>`:
    - Try primary provider
    - On failure (timeout, rate limit, error), try fallbacks in order
    - Track which provider succeeded
- Update `Agent` to use `ModelRouter` instead of single provider
- Add `RouterConfig` to `AgentConfig`:
  ```rust
  pub struct RouterConfig {
      pub primary: String,
      pub fallbacks: Vec<String>,
      pub enable_auto_route: bool,
  }
  ```
- Add `/route` REPL command to show current routing config

**Verification**:
- `cargo test -p hermes-runtime test_router_fallback`
- `cargo test -p hermes-runtime test_router_by_complexity`
- `cargo test -p hermes-runtime test_router_by_capability`
- `cargo test -p hermes-runtime test_router_exhaustion`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add smart model routing with fallback chain`

---

### Day 17 (Wednesday) — Multi-Credential Pool

#### Task 3.3: Implement Multi-Credential Pool with Failover
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `rust-patterns`
- **Files**: `crates/cli-core/src/auth.rs`, `crates/runtime/src/provider/pool.rs` (new)
- **Dependencies**: Task 2.1 (provider framework), Task 3.2 (routing)

**What to do**:
- Extend `AuthStore` to support multiple credentials per provider:
  ```rust
  pub struct ProviderCredentials {
      pub provider: String,
      pub entries: Vec<CredentialEntry>,
  }
  
  pub struct CredentialEntry {
      pub id: String,
      pub api_key: String,
      pub base_url: Option<String>,
      pub label: Option<String>,
      pub is_exhausted: bool,
      pub last_used: Option<DateTime<Utc>>,
      pub use_count: u32,
  }
  ```
- Update `AuthCommand::Add` to support `--label` for multiple keys per provider
- Update `AuthCommand::List` to show all entries with labels
- Create `CredentialPool`:
  - `get_available(provider) -> Vec<CredentialEntry>`
  - `mark_exhausted(provider, id)` — mark key as exhausted (rate limited)
  - `mark_restored(provider, id)` — reset exhaustion (after time)
  - `get_least_used(provider) -> Option<CredentialEntry>`
- Integrate with `ModelRouter`: when primary credential is exhausted, try next credential before trying fallback provider
- Add `hermes auth reset <provider>` to clear exhaustion flags

**Verification**:
- `cargo test -p hermes-cli-core test_multi_credential_add`
- `cargo test -p hermes-cli-core test_credential_pool_rotation`
- `cargo test -p hermes-cli-core test_credential_exhaustion`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(cli-core): add multi-credential pool with failover`

---

### Day 18 (Thursday) — Prompt Caching

#### Task 3.4: Implement Prompt Caching Optimization
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `rust-patterns`
- **Files**: `crates/runtime/src/provider/caching.rs` (existing), `crates/runtime/src/context/mod.rs`
- **Dependencies**: Task 1.8 (token counting)

**What to do**:
- Extend existing `crates/runtime/src/provider/caching.rs`:
  - `PromptCache` struct with LRU eviction:
    ```rust
    pub struct PromptCache {
      cache: LruCache<String, CacheEntry>,
      max_entries: usize,
    }
    
    struct CacheEntry {
      response: String,
      tool_calls: Vec<ToolCall>,
      tokens_used: u32,
      cached_at: DateTime<Utc>,
    }
    ```
  - `compute_key(messages: &[ChatMessage], model: &str) -> String` — hash of normalized messages + model
  - `get(key) -> Option<CacheEntry>`
  - `put(key, entry)`
- Add cache-aware logic to `Agent::run_turn()`:
  - Before calling LLM, check cache
  - If cache hit and no tools in cached response, return immediately
  - If cache hit but tools present, still execute tools (results may differ)
  - Cache only if `AgentConfig.enable_prompt_caching` is true
- Add Anthropic prompt caching support (beta header `anthropic-beta: prompt-caching-2024-07-31`):
  - Add `cache_control` field to `ChatMessage` (already exists!)
  - Set `cache_control: { type: "ephemeral" }` on system prompt and first user message
- Add cache statistics to `/usage` command: cache hits, misses, hit rate

**Verification**:
- `cargo test -p hermes-runtime test_prompt_cache_hit`
- `cargo test -p hermes-runtime test_prompt_cache_miss`
- `cargo test -p hermes-runtime test_prompt_cache_lru_eviction`
- `cargo test -p hermes-runtime test_anthropic_prompt_caching`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add prompt caching optimization`

---

### Day 19 (Friday) — Skills System Foundation

#### Task 3.5: Implement Skills System
- **Estimated Hours**: 8h
- **Agent Category**: `deep`
- **Skills**: `rust-patterns`, `architecture-designer`
- **Files**: `crates/cli-core/src/skills.rs`, `crates/runtime/src/skills/` (new)
- **Dependencies**: None

**What to do**:
- Create `crates/runtime/src/skills/mod.rs` with `Skill` trait:
  ```rust
  pub trait Skill: Send + Sync {
      fn name(&self) -> &str;
      fn description(&self) -> &str;
      fn version(&self) -> &str;
      fn category(&self) -> SkillCategory;
      fn tools(&self) -> Vec<Box<dyn Tool>>;
      fn system_prompt_addon(&self) -> Option<String>;
      fn on_load(&self) -> Result<(), RuntimeError>;
      fn on_unload(&self) -> Result<(), RuntimeError>;
  }
  
  pub enum SkillCategory {
      Builtin,
      UserInstalled,
      Remote,
  }
  ```
- Create `SkillRegistry`:
  - `register(skill: Box<dyn Skill>)`
  - `unregister(name: &str)`
  - `get(name) -> Option<&dyn Skill>`
  - `list() -> Vec<&dyn Skill>`
  - `list_by_category(category) -> Vec<&dyn Skill>`
  - `get_combined_system_prompt() -> String` — concatenate all loaded skill addons
- Create `BuiltinSkill` struct for built-in skills:
  - `git_skill`: provides git-related tools and knowledge
  - `code_skill`: provides code analysis tools
  - `web_skill`: provides web search and fetch tools
- Update `Agent` to accept `SkillRegistry` and inject skill tools + system prompt addons
- Update `handle_chat()` to:
  - Load skills from `--skills` CLI flag
  - Register skill tools in `ToolRegistry`
  - Append skill system prompts to agent system prompt
- Update `hermes skills` commands:
  - `skills list` → show loaded skills
  - `skills load <name>` → load a skill in REPL
  - `skills unload <name>` → unload a skill

**Verification**:
- `cargo test -p hermes-runtime test_skill_registration`
- `cargo test -p hermes-runtime test_skill_system_prompt`
- `cargo test -p hermes-runtime test_skill_tools`
- `cargo test -p hermes-cli-core test_skills_cli`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): add skills system with builtin skills`

---

### Day 20 (Saturday) — Integration + Testing

#### Task 3.6: Integrate Advanced Features + End-to-End Tests
- **Estimated Hours**: 6h
- **Agent Category**: `oracle`
- **Skills**: `testing-strategies`
- **Files**: All Wave 3 files
- **Dependencies**: All Wave 3 tasks

**What to do**:
- End-to-end test: compression + routing + caching together
- End-to-end test: skills + tools + streaming
- Performance test: measure token savings from compression
- Performance test: measure cache hit rate
- Run full test suite
- Fix any regressions
- Add integration tests to `tests/` directory

**Verification**:
- `cargo test --workspace` → all pass
- `cargo clippy --workspace -- -D warnings` → zero warnings
- `cargo build --release` → success
- New test count: 400+

**Commit**: `test: add wave 3 integration and end-to-end tests`

---

### Day 21 (Sunday) — Rest / Documentation

- Document advanced features
- Write architecture decision records (ADRs) for routing, compression, caching
- Update CHANGELOG

---

## Wave 4: Polish, Documentation, Release Prep (Days 22-30)

> **Theme**: Ship it. Make Hermes reliable, documented, and delightful.

---

### Day 22 (Monday) — Error Handling + Logging

#### Task 4.1: Improve Error Messages and User Guidance
- **Estimated Hours**: 5h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`, `writing-clearly-and-concisely`
- **Files**: `crates/runtime/src/error.rs`, `crates/cli-core/src/error.rs`
- **Dependencies**: None

**What to do**:
- Audit all error messages for user-friendliness:
  - "provider error: connection refused" → "Cannot connect to OpenAI. Check your internet connection and try again."
  - "tool error [terminal]: exit code 1" → "The terminal command failed. Error: {stderr}"
  - "rate limited" → "Rate limited by {provider}. Retrying in {seconds}s..."
- Add `RuntimeError::user_message(&self) -> String` for human-friendly errors
- Add `RuntimeError::suggestion(&self) -> Option<String>` for actionable next steps:
  - "Run `hermes auth add openai --api-key <KEY>`"
  - "Try a different model with `/model gpt-4o-mini`"
  - "Check your base URL with `hermes config get model.base_url`"
- Add structured logging with `tracing`:
  - Log provider calls with timing
  - Log tool executions with args and results
  - Log routing decisions
  - Log cache hits/misses
- Add `hermes logs` command to view structured logs

**Verification**:
- `cargo test -p hermes-runtime test_error_user_messages`
- `cargo test -p hermes-runtime test_error_suggestions`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(runtime): improve error messages with user guidance`

---

### Day 23 (Tuesday) — Configuration Management

#### Task 4.2: Add Configuration Validation and Migration
- **Estimated Hours**: 5h
- **Agent Category**: `quick`
- **Skills**: `rust-patterns`
- **Files**: `crates/cli-core/src/config.rs`
- **Dependencies**: None

**What to do**:
- Add config schema version tracking:
  ```rust
  pub struct Config {
      pub version: u32,  // schema version
      // ... existing fields
  }
  ```
- Add config migration system:
  - `migrate_v1_to_v2(config: &mut Config)` — add new fields with defaults
  - `migrate_v2_to_v3(config: &mut Config)` — rename fields, etc.
  - Auto-migrate on load
- Add config validation:
  - Validate provider names against known providers
  - Validate model names against metadata registry
  - Validate API keys format (e.g., OpenAI keys start with "sk-")
  - Validate base URLs are valid URLs
  - Print warnings for invalid config, use defaults
- Add `hermes config validate` command
- Add `hermes config doctor` command (check common issues)

**Verification**:
- `cargo test -p hermes-cli-core test_config_migration`
- `cargo test -p hermes-cli-core test_config_validation`
- `cargo test -p hermes-cli-core test_config_doctor`
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat(cli-core): add config validation and migration`

---

### Day 24 (Wednesday) — Performance Optimization

#### Task 4.3: Optimize Binary Size and Startup Time
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `performance-optimization`, `rust-patterns`
- **Files**: `Cargo.toml`, `crates/*/Cargo.toml`
- **Dependencies**: None

**What to do**:
- Binary size optimization:
  - Enable `strip = true` in release profile (already done)
  - Enable `lto = true` (already done)
  - Enable `codegen-units = 1` (already done)
  - Add `opt-level = "z"` option for size-optimized builds
  - Use `cargo bloat` to identify large dependencies
  - Consider replacing `reqwest` with `ureq` for non-async HTTP (if possible)
  - Feature-gate heavy dependencies: `tiktoken-rs` only for OpenAI models
- Startup time optimization:
  - Lazy-load provider registry (don't initialize all providers on startup)
  - Lazy-load model metadata (load on first use)
  - Use `once_cell::Lazy` for static data
  - Profile startup with `cargo flamegraph`
- Add `hermes doctor --performance` command
- Target: <15MB release binary, <500ms startup time

**Verification**:
- `cargo build --release` → measure binary size
- `hyperfine "target/release/hermes --version"` → measure startup time
- `cargo bloat --release -p hermes-cli` → identify bloat
- `cargo clippy --workspace -- -D warnings`

**Commit**: `perf: optimize binary size and startup time`

---

### Day 25 (Thursday) — Cross-Platform Testing

#### Task 4.4: Ensure Cross-Platform Compatibility
- **Estimated Hours**: 5h
- **Agent Category**: `unspecified-high`
- **Skills**: `rust-patterns`
- **Files**: All platform-specific code
- **Dependencies**: None

**What to do**:
- Audit all `#[cfg(target_os = "windows")]` blocks:
  - `cli-core/src/lib.rs:891-952` — disk space check
  - `cli-core/src/auth.rs:60-65` — hidden file attribute
  - `runtime/src/tool/terminal.rs` — PowerShell usage
- Add `#[cfg(target_os = "linux")]` and `#[cfg(target_os = "macos")]` equivalents:
  - Linux/macOS disk space: `df -h .`
  - Linux/macOS file permissions: `chmod 600`
  - Linux/macOS terminal: `bash` or `zsh`
- Add CI configuration for Linux and macOS builds (GitHub Actions)
- Test compilation on Linux (WSL or VM):
  - `cargo build --workspace`
  - `cargo test --workspace`
- Add platform-specific tests:
  - `test_terminal_unix` — run `echo hello` with bash
  - `test_terminal_windows` — run `echo hello` with PowerShell

**Verification**:
- `cargo test --workspace` on Windows → pass
- `cargo test --workspace` on Linux (WSL) → pass
- `cargo clippy --workspace -- -D warnings`

**Commit**: `feat: add cross-platform support for Linux and macOS`

---

### Day 26 (Friday) — Documentation

#### Task 4.5: Write Comprehensive Documentation
- **Estimated Hours**: 6h
- **Agent Category**: `quick`
- **Skills**: `writing-clearly-and-concisely`, `project-docs`
- **Files**: `README.md`, `docs/` (new)
- **Dependencies**: None

**What to do**:
- Rewrite `README.md`:
  - Hero section with features list
  - Quick start guide
  - Installation instructions (Windows, Linux, macOS)
  - Provider setup guide (all 10+ providers)
  - Configuration reference
  - REPL commands reference
  - Tool usage examples
  - Troubleshooting guide
- Create `docs/` directory:
  - `docs/architecture.md` — crate structure, data flow
  - `docs/providers.md` — provider-specific setup
  - `docs/configuration.md` — config.yaml reference
  - `docs/skills.md` — skills system guide
  - `docs/contributing.md` — development setup
- Add rustdoc comments to all public APIs
- Generate docs: `cargo doc --workspace --no-deps`

**Verification**:
- `cargo doc --workspace --no-deps` → no warnings
- README renders correctly on GitHub
- All links work

**Commit**: `docs: add comprehensive documentation and README`

---

### Day 27 (Saturday) — Release Prep

#### Task 4.6: Add Release Automation + Versioning
- **Estimated Hours**: 5h
- **Agent Category**: `quick`
- **Skills**: `ci-cd-pipelines`, `rust-patterns`
- **Files**: `.github/workflows/release.yml` (new), `Cargo.toml`
- **Dependencies**: None

**What to do**:
- Add `Cargo.toml` workspace version bumping script
- Create `.github/workflows/ci.yml`:
  - Run on PR: `cargo test`, `cargo clippy`, `cargo fmt --check`
  - Run on push to main: same + build release binary
- Create `.github/workflows/release.yml`:
  - Trigger on tag `v*`
  - Build for Windows (x64), Linux (x64), macOS (x64, ARM)
  - Create GitHub release with binaries
  - Generate changelog from commits
- Add `hermes update` command to check for updates
- Add `hermes version --verbose` to show build info
- Create `CHANGELOG.md` with conventional commits format

**Verification**:
- `cargo fmt --check` → no formatting issues
- `cargo test --workspace` → pass
- `cargo build --release` → success

**Commit**: `ci: add GitHub Actions CI/CD and release automation`

---

### Day 28 (Sunday) — Final Testing

#### Task 4.7: Run Full Test Suite + Fix Regressions
- **Estimated Hours**: 6h
- **Agent Category**: `oracle`
- **Skills**: `testing-strategies`
- **Files**: All
- **Dependencies**: All previous tasks

**What to do**:
- Run complete test suite: `cargo test --workspace`
- Run clippy: `cargo clippy --workspace -- -D warnings`
- Run fmt: `cargo fmt --check`
- Run doc tests: `cargo test --doc --workspace`
- Manual QA checklist:
  - [ ] `hermes chat -q "hello"` works with OpenAI
  - [ ] `hermes chat -q "hello"` works with Anthropic
  - [ ] `hermes chat -q "hello"` works with Ollama (if running)
  - [ ] Tool calls work in non-streaming mode
  - [ ] Tool calls work in streaming mode
  - [ ] Ctrl+C exits cleanly
  - [ ] `/model` shows and changes models
  - [ ] `/compact` truncates context
  - [ ] `/usage` shows token counts
  - [ ] Markdown renders in output
  - [ ] Spinner shows during tool execution
  - [ ] Error messages are helpful
  - [ ] Session persists across restarts
  - [ ] Config validates correctly
  - [ ] `hermes doctor` passes all checks
- Fix any issues found

**Verification**:
- Test count: 500+
- Clippy: 0 warnings
- Build: success
- Manual QA: all pass

**Commit**: `test: final QA pass, fix regressions`

---

### Day 29 (Monday) — Release Candidate

#### Task 4.8: Create Release Candidate
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `git-release`
- **Files**: All
- **Dependencies**: Task 4.7

**What to do**:
- Bump version to `1.0.0-rc.1` in all `Cargo.toml` files
- Update `CHANGELOG.md` with all changes
- Create release branch: `git checkout -b release/v1.0.0`
- Build release binary
- Test release binary: `target/release/hermes --version`
- Tag: `git tag v1.0.0-rc.1`
- Push branch and tag
- Create GitHub release draft

**Verification**:
- `target/release/hermes --version` → `hermes 1.0.0-rc.1`
- Binary size < 15MB
- All tests pass

**Commit**: `chore(release): v1.0.0-rc.1`

---

### Day 30 (Tuesday) — Release Day

#### Task 4.9: Final Release
- **Estimated Hours**: 4h
- **Agent Category**: `quick`
- **Skills**: `git-release`
- **Files**: All
- **Dependencies**: Task 4.8

**What to do**:
- If RC testing passes, bump to `1.0.0`
- Finalize CHANGELOG
- Publish GitHub release
- Announce on relevant channels
- Monitor for issues
- Create `hotfix` branch for any critical issues

**Verification**:
- Release published on GitHub
- Binaries available for Windows, Linux, macOS
- Installation instructions work

**Commit**: `chore(release): v1.0.0`

---

## Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 1.1 Retry | None | 1.5 |
| 1.2 JSON Errors | None | None |
| 1.3 Signal | None | None |
| 1.4 /model | None | None |
| 1.5 Streaming+Tools | 1.1 | 1.6, 1.7 |
| 1.6 Tool Feedback | 1.5 | None |
| 1.7 Markdown | 1.6 | None |
| 1.8 Tokens | None | 1.10, 3.1, 3.4 |
| 1.9 System Prompt | None | None |
| 1.10 /compact | 1.8 | None |
| 1.11 Cleanup | All W1 | W2 |
| 2.1 Provider Framework | None | 2.2-2.8 |
| 2.2 Ollama | 2.1 | None |
| 2.3 DeepSeek | 2.1 | None |
| 2.4 Groq | 2.1 | None |
| 2.5 Gemini | 2.1 | None |
| 2.6 Azure | 2.1 | None |
| 2.7 OpenRouter | 2.1 | None |
| 2.8 Local | 2.1 | None |
| 2.9 Metadata | None | 2.10, 3.2 |
| 2.10 Usage | 2.9 | None |
| 2.11 Themes | 1.6, 1.7 | None |
| 2.12 Cleanup | All W2 | W3 |
| 3.1 Compression | 1.8, 1.10 | None |
| 3.2 Routing | 2.9 | None |
| 3.3 Credential Pool | 2.1, 3.2 | None |
| 3.4 Caching | 1.8 | None |
| 3.5 Skills | None | None |
| 3.6 Cleanup | All W3 | W4 |
| 4.1 Errors | None | None |
| 4.2 Config | None | None |
| 4.3 Performance | None | None |
| 4.4 Cross-Platform | None | None |
| 4.5 Docs | None | None |
| 4.6 CI/CD | None | None |
| 4.7 Final QA | All | 4.8 |
| 4.8 RC | 4.7 | 4.9 |
| 4.9 Release | 4.8 | None |

---

## Parallel Execution Strategy

### Max 3 Parallel Agents at a Time

**Week 1**:
- Agent A: Tasks 1.1, 1.2, 1.3 (sequential, deep)
- Agent B: Tasks 1.4, 1.8, 1.9 (sequential, quick)
- Agent C: Tasks 1.5, 1.6, 1.7 (sequential after 1.1, deep)

**Week 2**:
- Agent A: Tasks 2.1, 2.2, 2.3, 2.4 (sequential, deep)
- Agent B: Tasks 2.5, 2.6, 2.7, 2.8 (sequential, quick)
- Agent C: Tasks 2.9, 2.10, 2.11 (sequential, quick)

**Week 3**:
- Agent A: Tasks 3.1, 3.2 (sequential, deep)
- Agent B: Tasks 3.3, 3.4 (sequential, unspecified-high)
- Agent C: Task 3.5 (deep)

**Week 4**:
- Agent A: Tasks 4.1, 4.2, 4.3 (sequential, quick)
- Agent B: Tasks 4.4, 4.5, 4.6 (sequential, quick)
- Agent C: Tasks 4.7, 4.8, 4.9 (sequential, oracle)

---

## Success Criteria

### Functional
- [ ] All 12 critical issues resolved
- [ ] 10+ LLM providers supported
- [ ] Streaming + tool calls work together
- [ ] Tool execution has visual feedback
- [ ] Accurate token counting
- [ ] API retry with exponential backoff
- [ ] Graceful Ctrl+C handling
- [ ] All REPL commands implemented
- [ ] Markdown rendering in output
- [ ] Context compression with summarization
- [ ] Smart model routing
- [ ] Multi-credential pool
- [ ] Prompt caching
- [ ] Skills system

### Quality
- [ ] 500+ tests, all passing
- [ ] 0 clippy warnings
- [ ] `cargo fmt --check` passes
- [ ] `cargo doc` generates without warnings
- [ ] Binary size < 15MB
- [ ] Startup time < 500ms
- [ ] Cross-platform: Windows, Linux, macOS

### Documentation
- [ ] Comprehensive README
- [ ] Architecture documentation
- [ ] Provider setup guide
- [ ] Configuration reference
- [ ] API documentation (rustdoc)
- [ ] CHANGELOG
- [ ] Contributing guide

### Release
- [ ] GitHub release with binaries
- [ ] CI/CD pipeline
- [ ] Version 1.0.0 tagged

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Streaming+Tools complexity | Medium | High | Task 1.5 is 8h, can extend to 2 days if needed |
| Gemini API differences | Medium | Medium | Use mock server tests, fallback to OpenAI-compatible |
| Token counting accuracy | Low | Medium | Use tiktoken-rs, fallback to heuristic |
| Cross-platform issues | Medium | Medium | Test on WSL early (Day 25) |
| Binary size bloat | Low | Low | Profile with cargo-bloat, feature-gate deps |
| Scope creep | High | High | Strict "Must NOT Have" guardrails, daily standup |

---

## Daily Standup Template

Each day, answer:
1. What did I complete yesterday?
2. What am I working on today?
3. Are there any blockers?
4. Do I need to adjust the plan?

---

## Commit Message Convention

```
<type>(<scope>): <description>

<body>

Refs: <task-number>
```

Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `chore`
Scopes: `runtime`, `cli-core`, `common`, `session-db`, `provider`, `tool`, `chat`, `display`

---

*Plan created: 2026-04-24*
*Target release: 2026-05-24*
*Owner: Single Developer*
