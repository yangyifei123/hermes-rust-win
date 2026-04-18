# Hermes Rust Core Runtime — Work Plan

## TL;DR

> **Quick Summary**: Build the core runtime engine for hermes-rust-win — LLM client, agent loop, tool registry, session DB, chat REPL, and WeChat/QQ gateway adapters. This transforms the CLI from a stub shell into a working agent.
> 
> **Deliverables**:
> - `hermes-runtime` crate (agent loop + LLM client + tool registry)
> - `hermes-session-db` crate (SQLite session persistence)
> - Working `hermes chat -q "hello"` with LLM response
> - Working `hermes chat` interactive REPL
> - Terminal tool + file tools (functional)
> - Web search tool + browser/MCP (stubs)
> - WeChat + QQ gateway adapter stubs
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 0 → Task 2 → Task 5 → Task 8 → Task 9 → Task 11

---

## Context

### Original Request
Continue the hermes-rust-win Rust rewrite. Previous session crashed before completion. Build core runtime (LLM client + agent loop + tool registry + chat mode) with WeChat + QQ gateway adapters.

### Interview Summary
**Key Discussions**:
- CLI parsing is done (40+ commands, 84 tests). Runtime is zero.
- User wants core runtime first, not full parity with Python
- Target platforms: WeChat + QQ (not all 18)
- TUI: ratatui + crossterm, REPL first then TUI
- Tools: Top 5 (terminal, file, web_search, browser stub, MCP stub)
- Tests: Rust #[test] + tokio::test

**Research Findings**:
- Python run_agent.py: sync while loop, max_iterations budget, parallel tool exec
- Python SessionDB: SQLite WAL + FTS5, sessions + messages tables
- Python has 40+ tools across 20+ toolsets — we implement 3, stub 2
- Rust 1.75+ supports async fn in traits natively (no async-trait needed)

### Metis Review
**Identified Gaps** (addressed):
- BLOCKING BUG: lib.rs:1023-1024 empty test body → compilation broken (Task 0)
- Circular dep risk: runtime must NOT depend on cli-core → extract shared types to common
- Anthropic API ≠ OpenAI format → need 2 request/response mappers
- SSE streaming needs explicit parsing (eventsource-stream crate)
- MCP + Browser = stubs only, not real implementations
- Tool approval mechanism needed (yolo flag in CLI)

---

## Work Objectives

### Core Objective
Build a working agent runtime in Rust that can: connect to LLM providers, execute tool calls, persist sessions, and respond to user input — both via CLI REPL and WeChat/QQ gateway.

### Concrete Deliverables
- `crates/runtime/` — agent loop, LLM client, provider trait, tool trait
- `crates/session-db/` — SQLite session store
- Working `hermes chat` command with LLM responses
- Terminal tool + file tools (functional)
- WeChat + QQ gateway adapter stubs

### Definition of Done
- [ ] `cargo build --workspace` succeeds with zero errors
- [ ] `cargo test --workspace` passes all tests
- [ ] `hermes chat -q "Say exactly: TEST_PASSED"` prints "TEST_PASSED"
- [ ] `hermes chat` enters interactive REPL, responds to input
- [ ] Session persists across restarts (create → quit → resume → verify)

### Must Have
- OpenAI-compatible LLM client with streaming SSE support
- Anthropic API adapter (different request/response format)
- Agent loop with max_turns budget and tool dispatch
- SQLite session DB (create, read, append messages)
- Terminal tool (PowerShell on Windows)
- File read/write tools
- Interactive chat REPL
- Tool approval mechanism (--yolo bypass)

### Must NOT Have (Guardrails)
- NO Python subprocess calls — pure Rust binary
- NO runtime → cli-core dependency (one-way: cli-core → runtime)
- NO real MCP client (stub returning "not implemented")
- NO real browser tool (stub returning "not implemented")
- NO Docker/SSH/Modal terminal backends (PowerShell only)
- NO TUI in this phase (REPL only, TUI is separate plan)
- NO markdown rendering in chat output (plain text)
- NO session search/FTS5 (CRUD only)
- NO `unwrap()` in production code paths
- NO `async-trait` crate (use Rust 1.75+ native async fn in traits)
- NO circular dependencies between crates

---

## Verification Strategy

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed.

### Test Decision
- **Infrastructure exists**: YES (cargo test)
- **Automated tests**: YES (Rust unit tests)
- **Framework**: cargo test + tokio::test

### QA Policy
Every task includes agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Build verification**: Bash (cargo build/test/clippy)
- **Runtime verification**: Bash (hermes chat commands)
- **DB verification**: Bash (cargo test session-db crate)

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Immediate — unblock everything):
└── Task 0: Fix compilation bug + extract shared types [quick]

Wave 1 (Foundation — all parallel after Task 0):
├── Task 1: Runtime crate scaffolding + error types [quick]
├── Task 2: LLM provider trait + OpenAI client [deep]
├── Task 3: Session DB crate (SQLite) [unspecified-high]
├── Task 4: Tool trait + registry [quick]

Wave 2 (Core engine — after Wave 1):
├── Task 5: Agent core loop (depends: 1, 2, 3, 4) [deep]
├── Task 6: Terminal tool impl (depends: 4) [unspecified-high]
├── Task 7: File tools impl (depends: 4) [unspecified-high]
├── Task 8: Web search tool stub (depends: 4) [quick]
├── Task 9: MCP + Browser tool stubs (depends: 4) [quick]

Wave 3 (Integration — after Wave 2):
├── Task 10: Chat REPL (depends: 5, 6, 7) [deep]
├── Task 11: Wire CLI chat to runtime (depends: 5, 10) [quick]
├── Task 12: WeChat gateway adapter stub (depends: 5) [unspecified-high]
├── Task 13: QQ gateway adapter stub (depends: 5) [unspecified-high]

Wave FINAL (Verification — after ALL tasks):
├── F1: Plan compliance audit (oracle)
├── F2: Code quality review (unspecified-high)
├── F3: Real manual QA (unspecified-high)
└── F4: Scope fidelity check (deep)
```

### Dependency Matrix
- **0**: - → 1,2,3,4
- **1**: 0 → 5
- **2**: 0 → 5
- **3**: 0 → 5,10
- **4**: 0 → 5,6,7,8,9
- **5**: 1,2,3,4 → 10,11,12,13
- **6**: 4 → 10
- **7**: 4 → 10
- **8**: 4 → 11
- **9**: 4 → 11
- **10**: 5,6,7 → 11
- **11**: 5,10 → F1-F4
- **12**: 5 → F1-F4
- **13**: 5 → F1-F4

### Agent Dispatch Summary
- **Wave 0**: 1 task — T0 → `quick`
- **Wave 1**: 4 tasks — T1,T4 → `quick`, T2 → `deep`, T3 → `unspecified-high`
- **Wave 2**: 5 tasks — T5 → `deep`, T6,T7,T12,T13 → `unspecified-high`, T8,T9 → `quick`
- **Wave 3**: 4 tasks — T10 → `deep`, T11 → `quick`, T12,T13 → `unspecified-high`
- **FINAL**: 4 tasks — F1 → `oracle`, F2,F3 → `unspecified-high`, F4 → `deep`

---

## TODOs

- [ ] 0. Fix compilation bug + extract shared types to common

  **What to do**:
  - Fix empty test body at `lib.rs:1023-1024` — either add the missing test body or remove the empty declaration
  - Verify `cargo test --workspace` passes (all 84 tests green)
  - Extract from `cli-core` to `hermes-common`: `ModelConfig`, `AgentConfig`, `Provider`, `AuthType`, `Model`, `Credentials`, `SessionId`
  - Update all imports across crates to use `hermes_common::` instead of local definitions
  - Add `uuid` v7 dependency to common for proper session ID generation (replace nanosecond timestamps)

  **Must NOT do**:
  - Don't change any existing test assertions
  - Don't add new features — pure refactoring

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 0 (sequential, unblocks everything)
  - **Blocks**: Tasks 1, 2, 3, 4
  - **Blocked By**: None

  **References**:
  - `crates/cli-core/src/lib.rs:1023-1024` — Empty test body `test_cli_parse_auth_list()` missing function body, immediately followed by next test
  - `crates/common/src/types.rs` — Existing shared types (Provider, AuthType, Model, SessionId, Credentials) — verify what's already here vs cli-core
  - `crates/cli-core/src/config.rs:102-131` — `AgentConfig` and `ModelConfig` structs that runtime will need
  - `crates/cli-core/src/auth.rs` — `ProviderCredentials` and `AuthStore` — runtime needs credential resolution
  - `Cargo.toml` — Workspace dependency section, add `uuid = { version = "1.0", features = ["v7"] }`

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Build succeeds after fix
    Tool: Bash
    Preconditions: Clean workspace
    Steps:
      1. cargo build --workspace
      2. Assert exit code 0
    Expected Result: Build succeeds with zero errors
    Failure Indicators: Compilation errors, undefined symbols
    Evidence: .sisyphus/evidence/task-0-build.txt

  Scenario: All existing tests pass
    Tool: Bash
    Preconditions: Build succeeds
    Steps:
      1. cargo test --workspace
      2. Assert "test result: ok" in output
      3. Count tests ≥ 84
    Expected Result: All tests pass
    Failure Indicators: "test result: FAILED", panic, compilation error
    Evidence: .sisyphus/evidence/task-0-tests.txt

  Scenario: Shared types accessible from common
    Tool: Bash
    Preconditions: Types extracted
    Steps:
      1. cargo test -p hermes-common
      2. Assert Provider, Model, Credentials types are exported
    Expected Result: Common crate exports all shared types
    Failure Indicators: "unresolved import", "not found"
    Evidence: .sisyphus/evidence/task-0-common-export.txt
  ```

  **Commit**: YES
  - Message: `fix(cli-core): fix empty test body, extract shared types to common`
  - Files: `crates/cli-core/src/lib.rs`, `crates/common/src/types.rs`, `Cargo.toml`
  - Pre-commit: `cargo test --workspace`

- [ ] 1. Runtime crate scaffolding + error types

  **What to do**:
  - Create `crates/runtime/` with `Cargo.toml` depending on `hermes-common`, `reqwest` (with `stream`, `json`, `rustls-tls` features), `tokio`, `serde`, `serde_json`, `thiserror`, `tracing`, `chrono`, `uuid`
  - Create `src/lib.rs` with module declarations
  - Create `src/error.rs` with `RuntimeError` enum using thiserror:
    - `ProviderError { source: Box<dyn StdError> }`
    - `ToolError { name: String, message: String }`
    - `AgentError { message: String }`
    - `SessionError { source: Box<dyn StdError> }`
    - `TimeoutError { duration_secs: u64 }`
    - `RateLimitError { retry_after: Option<u64> }`
  - Create placeholder module files: `src/provider/mod.rs`, `src/tool/mod.rs`, `src/agent/mod.rs`
  - Add `crates/runtime` to workspace members in root `Cargo.toml`

  **Must NOT do**:
  - Don't depend on `cli-core` (runtime is upstream)
  - Don't use `anyhow` — use `thiserror` only for error types
  - Don't use `async-trait` — use Rust 1.75+ native async fn in traits

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (after Task 0)
  - **Parallel Group**: Wave 1 (with Tasks 2, 3, 4)
  - **Blocks**: Task 5
  - **Blocked By**: Task 0

  **References**:
  - `Cargo.toml` — Workspace section, add `crates/runtime` to members
  - `crates/common/src/error.rs` — Follow same `thiserror` derive pattern
  - `crates/common/Cargo.toml` — Reference for crate dependency structure
  - Python `run_agent.py` — Agent loop architecture reference

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Runtime crate compiles
    Tool: Bash
    Preconditions: Task 0 complete
    Steps:
      1. cargo build -p hermes-runtime
      2. Assert exit code 0
    Expected Result: Crate compiles with zero errors
    Failure Indicators: Compilation errors
    Evidence: .sisyphus/evidence/task-1-build.txt

  Scenario: Error types derive correctly
    Tool: Bash
    Preconditions: Crate compiles
    Steps:
      1. cargo test -p hermes-runtime
      2. Assert RuntimeError variants construct and display correctly
    Expected Result: Tests pass
    Failure Indicators: Derive errors, missing Display impl
    Evidence: .sisyphus/evidence/task-1-tests.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add crate scaffolding and error types`
  - Files: `crates/runtime/*`
  - Pre-commit: `cargo build -p hermes-runtime`

- [ ] 2. LLM provider trait + OpenAI client + Anthropic adapter

  **What to do**:
  - Define `Provider` trait in `crates/runtime/src/provider/mod.rs`:
    ```rust
    pub trait LlmProvider: Send + Sync {
        async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse, RuntimeError>;
        async fn chat_completion_stream(&self, request: ChatRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>, RuntimeError>;
        fn name(&self) -> &str;
        fn default_model(&self) -> &str;
    }
    ```
  - Define `ChatRequest` (model, messages, tools, max_tokens, temperature, stream) and `ChatResponse` (choices with message content + tool_calls)
  - Define `StreamChunk` for SSE streaming deltas
  - Implement `OpenAiProvider` using reqwest — POST to `/v1/chat/completions` with Bearer token auth
  - Add SSE stream parsing (byte stream → event lines → delta objects)
  - Implement `AnthropicProvider` — POST to `/v1/messages` with `x-api-key` header, different message format (content blocks, tool_use blocks)
  - Provider factory: `fn create_provider(provider_type: &Provider, api_key: &str, base_url: Option<&str>) -> Box<dyn LlmProvider>`
  - Write unit tests with mock HTTP server (or use `mockito` crate)

  **Must NOT do**:
  - Don't implement providers beyond OpenAI and Anthropic (others use OpenAI-compatible format via config)
  - Don't add prompt caching yet (Phase 2)
  - Don't add retry logic here (belongs in agent loop)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (after Task 0)
  - **Parallel Group**: Wave 1 (with Tasks 1, 3, 4)
  - **Blocks**: Task 5
  - **Blocked By**: Task 0

  **References**:
  - `crates/common/src/types.rs` — `Provider` enum with 20+ providers, `AuthType`, default models/base URLs
  - `crates/cli-core/src/auth.rs:ProviderCredentials` — Credential structure to consume
  - `crates/cli-core/src/config.rs:ModelConfig` — default model, base_url, provider settings
  - Python `agent/auxiliary_client.py` — Provider resolution pattern (OpenAI/Anthropic/Nous clients)
  - Python `agent/anthropic_adapter.py` — Anthropic-specific request/response mapping
  - OpenAI API: `POST /v1/chat/completions` — `Authorization: Bearer {key}`, messages array, tool_choice
  - Anthropic API: `POST /v1/messages` — `x-api-key: {key}`, content blocks format, thinking support

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: OpenAI provider constructs valid request
    Tool: Bash
    Preconditions: Runtime crate compiles
    Steps:
      1. cargo test -p hermes-runtime test_openai_request_format
      2. Assert request JSON has correct structure (model, messages, tools fields)
    Expected Result: Test passes with valid JSON structure
    Failure Indicators: Serialization errors, missing fields
    Evidence: .sisyphus/evidence/task-2-openai-format.txt

  Scenario: Anthropic provider uses different auth header
    Tool: Bash
    Preconditions: OpenAI provider works
    Steps:
      1. cargo test -p hermes-runtime test_anthropic_auth_header
      2. Assert request uses x-api-key header, not Authorization Bearer
    Expected Result: Anthropic uses x-api-key
    Failure Indicators: Wrong header format
    Evidence: .sisyphus/evidence/task-2-anthropic-auth.txt

  Scenario: SSE stream parsing works
    Tool: Bash
    Preconditions: Provider compiles
    Steps:
      1. cargo test -p hermes-runtime test_sse_parsing
      2. Feed mock SSE data ("data: {...}\n\n" lines)
      3. Assert StreamChunk objects parsed correctly
    Expected Result: Chunks parsed with content deltas
    Failure Indicators: Parse errors, missing data
    Evidence: .sisyphus/evidence/task-2-sse.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add LLM provider trait, OpenAI client, Anthropic adapter`
  - Files: `crates/runtime/src/provider/*`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 3. Session DB crate (SQLite)

  **What to do**:
  - Create `crates/session-db/` with `Cargo.toml` depending on `rusqlite` (bundled feature), `hermes-common`, `serde`, `serde_json`, `chrono`, `thiserror`, `uuid`
  - Create `src/lib.rs`, `src/error.rs` (`SessionError`), `src/models.rs`, `src/store.rs`
  - Define `Session` struct: id (UUID v7), source, model, system_prompt, parent_session_id, created_at, updated_at, token_counts
  - Define `Message` struct: id, session_id, role (System/User/Assistant/Tool), content, tool_calls (JSON), tool_name, reasoning, created_at
  - Implement `SessionStore`:
    - `new(path: &Path) -> Result<Self>` — create/open SQLite with WAL mode
    - `create_session(model, system_prompt) -> Session`
    - `get_session(id) -> Option<Session>`
    - `list_sessions(limit, offset) -> Vec<Session>`
    - `append_message(session_id, message) -> Message`
    - `get_messages(session_id) -> Vec<Message>`
    - `delete_session(id) -> Result<()>`
  - Schema migration via `user_version` pragma (start at version 1)
  - Create tables on first run: `sessions`, `messages`
  - Write comprehensive tests: round-trip CRUD, WAL mode verification, concurrent access

  **Must NOT do**:
  - Don't implement FTS5 search (Phase 2)
  - Don't implement token billing/counting (Phase 2)
  - Don't implement session lineage/branching (Phase 2)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (after Task 0)
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 4)
  - **Blocks**: Task 5, Task 10
  - **Blocked By**: Task 0

  **References**:
  - Python `hermes_state.py` — SessionDB implementation: tables schema, CRUD operations, WAL mode
  - Python `hermes_state.py:sessions table` — Columns: id, source, model, system_prompt, parent_session_id, token_counts, billing, cost
  - Python `hermes_state.py:messages table` — Columns: id, session_id, role, content, tool_calls, tool_name, reasoning
  - `crates/common/src/types.rs:SessionId` — Existing session ID wrapper (verify if needs updating for UUID v7)
  - `crates/common/Cargo.toml` — Reference for crate dependency structure

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Session CRUD round-trip
    Tool: Bash
    Preconditions: Session DB crate compiles
    Steps:
      1. cargo test -p hermes-session-db test_session_round_trip
      2. Create session → get by ID → assert fields match
    Expected Result: Session persists and retrieves correctly
    Failure Indicators: Field mismatch, not found error
    Evidence: .sisyphus/evidence/task-3-crud.txt

  Scenario: Message append and retrieval
    Tool: Bash
    Preconditions: Session exists
    Steps:
      1. cargo test -p hermes-session-db test_message_append
      2. Create session → append 3 messages → get_messages → assert order and content
    Expected Result: Messages retrieved in chronological order
    Failure Indicators: Wrong order, missing messages
    Evidence: .sisyphus/evidence/task-3-messages.txt

  Scenario: WAL mode enabled
    Tool: Bash
    Preconditions: DB created
    Steps:
      1. cargo test -p hermes-session-db test_wal_mode
      2. Open DB → query journal_mode → assert "wal"
    Expected Result: Journal mode is WAL
    Failure Indicators: journal_mode returns "delete" or other
    Evidence: .sisyphus/evidence/task-3-wal.txt
  ```

  **Commit**: YES
  - Message: `feat(session-db): add SQLite session store with CRUD operations`
  - Files: `crates/session-db/*`
  - Pre-commit: `cargo test -p hermes-session-db`

- [ ] 4. Tool trait + registry

  **What to do**:
  - Define `Tool` trait in `crates/runtime/src/tool/mod.rs`:
    ```rust
    pub trait Tool: Send + Sync {
        fn name(&self) -> &str;
        fn description(&self) -> &str;
        fn parameters_schema(&self) -> serde_json::Value;  // JSON Schema
        async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput, RuntimeError>;
    }
    ```
  - Define `ToolOutput` struct: content (String), is_error (bool)
  - Implement `ToolRegistry`:
    - `new() -> Self`
    - `register(tool: Box<dyn Tool>)`
    - `get(name: &str) -> Option<&dyn Tool>`
    - `list() -> Vec<(&str, &str)>` — name + description pairs
    - `dispatch(name: &str, params: Value) -> Result<ToolOutput, RuntimeError>`
    - `tool_definitions() -> Vec<Value>` — JSON Schema array for LLM `tools` param
  - Write tests: register 3 mock tools → list → dispatch by name → verify output → dispatch unknown → ToolNotFound error

  **Must NOT do**:
  - Don't implement any concrete tools yet (Tasks 6-9)
  - Don't add approval logic here (belongs in agent loop)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (after Task 0)
  - **Parallel Group**: Wave 1 (with Tasks 1, 2, 3)
  - **Blocks**: Tasks 5, 6, 7, 8, 9
  - **Blocked By**: Task 0

  **References**:
  - Python `tools/registry.py` — ToolRegistry singleton, register/dispatch pattern
  - Python `model_tools.py:get_tool_definitions()` — Returns JSON Schema array for LLM
  - Python `model_tools.py:handle_function_call()` — Dispatch by name, return JSON string
  - `crates/cli-core/src/tools.rs` — Existing `Tool` struct (static metadata only, no execution)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Register and dispatch tools
    Tool: Bash
    Preconditions: Runtime crate compiles
    Steps:
      1. cargo test -p hermes-runtime test_tool_registry
      2. Register 3 mock tools → list → assert 3 entries
      3. Dispatch known tool → assert success
      4. Dispatch unknown → assert ToolNotFound error
    Expected Result: Registry manages tools correctly
    Failure Indicators: Wrong count, dispatch failure, missing error
    Evidence: .sisyphus/evidence/task-4-registry.txt

  Scenario: Tool definitions for LLM
    Tool: Bash
    Preconditions: Tools registered
    Steps:
      1. cargo test -p hermes-runtime test_tool_definitions
      2. Register tools → call tool_definitions() → assert valid JSON Schema array
    Expected Result: Each definition has type:"function", function.name, function.parameters
    Failure Indicators: Invalid JSON, missing schema fields
    Evidence: .sisyphus/evidence/task-4-definitions.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add tool trait and registry`
  - Files: `crates/runtime/src/tool/mod.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 5. Agent core loop

  **What to do**:
  - Create `crates/runtime/src/agent/mod.rs` with `Agent` struct:
    ```rust
    pub struct Agent {
        provider: Box<dyn LlmProvider>,
        tools: ToolRegistry,
        session_store: SessionStore,
        max_turns: u32,
        yolo: bool,
    }
    ```
  - Implement `Agent::run_turn(&mut self, user_message: &str) -> Result<AgentResponse>`:
    1. Append user message to session
    2. Load message history from session DB
    3. Call LLM provider with messages + tool definitions
    4. If response has tool_calls → dispatch each to registry → append tool results → recurse (up to max_turns)
    5. If response is text → append assistant message → return
  - Implement `Agent::run_query(&mut self, query: &str) -> Result<String>` — single-shot query with response
  - Handle token limit: truncate message history when approaching model context window (keep system prompt + last N messages)
  - Handle tool execution timeout: wrap each tool dispatch in `tokio::time::timeout(Duration::from_secs(120))`
  - Handle rate limiting: exponential backoff on 429 (max 3 retries)
  - Handle Ctrl+C graceful shutdown: save partial response to session
  - Write tests with mock provider (returns canned tool_use responses) and mock tools

  **Must NOT do**:
  - Don't implement parallel tool execution yet (sequential for V1)
  - Don't implement context compression/summarization
  - Don't implement provider fallback chain
  - Don't implement smart model routing

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential, depends on all Wave 1)
  - **Blocks**: Tasks 10, 11, 12, 13
  - **Blocked By**: Tasks 1, 2, 3, 4

  **References**:
  - Python `run_agent.py:run_conversation()` — Core loop: while True → API call → tool dispatch → recurse
  - Python `run_agent.py:IterationBudget` — Thread-safe counter for max_turns
  - Python `run_agent.py` lines ~200-400 — Tool call handling: extract function name + args → dispatch → append result
  - `crates/cli-core/src/config.rs:AgentConfig` — max_turns (30), system_prompt, reasoning_effort
  - `crates/runtime/src/provider/mod.rs` — LlmProvider trait (from Task 2)
  - `crates/runtime/src/tool/mod.rs` — ToolRegistry dispatch (from Task 4)
  - `crates/session-db/src/store.rs` — SessionStore append/get (from Task 3)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Agent loop with mock provider (no tools)
    Tool: Bash
    Preconditions: All Wave 1 tasks complete
    Steps:
      1. cargo test -p hermes-runtime test_agent_simple_query
      2. Mock provider returns "Hello from AI" → agent.run_query("hi") → assert response
    Expected Result: Returns "Hello from AI"
    Failure Indicators: Wrong response, loop doesn't terminate
    Evidence: .sisyphus/evidence/task-5-simple.txt

  Scenario: Agent loop with tool calling
    Tool: Bash
    Preconditions: Tool registry with mock tool
    Steps:
      1. cargo test -p hermes-runtime test_agent_tool_call
      2. Mock provider returns tool_use → agent dispatches → mock tool returns "result" → provider returns final answer
    Expected Result: Agent calls tool, gets result, returns final answer
    Failure Indicators: Tool not dispatched, loop exceeds max_turns
    Evidence: .sisyphus/evidence/task-5-tool-call.txt

  Scenario: Max turns enforcement
    Tool: Bash
    Preconditions: Agent configured with max_turns=2
    Steps:
      1. cargo test -p hermes-runtime test_agent_max_turns
      2. Mock provider always returns tool_use (infinite loop) → agent stops at max_turns
    Expected Result: Agent stops and returns budget-exhausted message
    Failure Indicators: Infinite loop, panic, exceeds max_turns
    Evidence: .sisyphus/evidence/task-5-max-turns.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add agent core loop with tool dispatch`
  - Files: `crates/runtime/src/agent/mod.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 6. Terminal tool implementation

  **What to do**:
  - Create `crates/runtime/src/tool/terminal.rs`
  - Implement `TerminalTool` struct implementing `Tool` trait
  - `name()` → `"terminal"`
  - `description()` → `"Execute terminal commands on the local machine"`
  - `parameters_schema()` → JSON Schema with `command` (string, required) and `timeout` (number, optional, default 120)
  - `execute(params)` → spawn `Command::new("powershell")` with `-Command` flag + command arg
  - Capture stdout + stderr, return combined output
  - Enforce timeout via `tokio::time::timeout`
  - Handle Windows-specific encoding (try UTF-8, fallback to lossy conversion)
  - Register terminal tool in tool registry during agent initialization
  - Write tests: run `echo hello` → assert output contains "hello"

  **Must NOT do**:
  - Don't implement Docker/SSH/Modal/Singularity/Daytona backends
  - Don't implement sudo/approval prompts
  - Don't implement background process management

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5 start blocked, but 6 only needs Task 4)
  - **Blocks**: Task 10
  - **Blocked By**: Task 4

  **References**:
  - Python `tools/terminal_tool.py` — Terminal orchestration (6 backends — we implement only local)
  - Python `tools/terminal_tool.py:execute_command()` — Command execution with timeout, stdout/stderr capture
  - `crates/cli-core/src/config.rs:TerminalConfig` — timeout, cwd settings
  - `crates/runtime/src/tool/mod.rs` — Tool trait (from Task 4)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Execute simple command
    Tool: Bash
    Preconditions: Runtime crate compiles
    Steps:
      1. cargo test -p hermes-runtime test_terminal_echo
      2. TerminalTool::execute({"command": "echo HELLO_TOOL"}) → assert output contains "HELLO_TOOL"
    Expected Result: Command output captured
    Failure Indicators: Empty output, spawn error
    Evidence: .sisyphus/evidence/task-6-echo.txt

  Scenario: Command timeout enforcement
    Tool: Bash
    Preconditions: Terminal tool compiles
    Steps:
      1. cargo test -p hermes-runtime test_terminal_timeout
      2. TerminalTool::execute({"command": "Start-Sleep -Seconds 30", "timeout": 2}) → assert timeout error
    Expected Result: Timeout error returned within ~2 seconds
    Failure Indicators: Command hangs, no timeout
    Evidence: .sisyphus/evidence/task-6-timeout.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add terminal tool (PowerShell on Windows)`
  - Files: `crates/runtime/src/tool/terminal.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 7. File tools implementation

  **What to do**:
  - Create `crates/runtime/src/tool/file.rs`
  - Implement three tools in one file:
    - `FileReadTool`: `name() = "file_read"`, params: `path` (string, required), `offset`/`limit` (optional). Reads file content as UTF-8 string.
    - `FileWriteTool`: `name() = "file_write"`, params: `path` (string, required), `content` (string, required). Creates/truncates file.
    - `FileSearchTool`: `name() = "file_search"`, params: `pattern` (string, required), `path` (string, optional). Searches file content using regex.
  - All tools implement `Tool` trait
  - Handle Windows paths (backslash/forward slash normalization)
  - Handle encoding (try UTF-8, fallback to lossy)
  - Restrict file access to CWD and subdirectories (security)
  - Write tests using tempdir: create file → read → search → write → verify

  **Must NOT do**:
  - Don't implement patch/fuzzy matching (Phase 2)
  - Don't implement binary file operations
  - Don't allow reading outside CWD tree

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 8, 9)
  - **Blocks**: Task 10
  - **Blocked By**: Task 4

  **References**:
  - Python `tools/file_tools.py` — File read/write/patch/search implementations
  - Python `tools/file_tools.py:read_file()` — Offset/limit reading, encoding handling
  - `crates/runtime/src/tool/mod.rs` — Tool trait (from Task 4)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: File read/write round-trip
    Tool: Bash
    Preconditions: Runtime crate compiles
    Steps:
      1. cargo test -p hermes-runtime test_file_round_trip
      2. WriteTool("test.txt", "Hello Rust") → ReadTool("test.txt") → assert "Hello Rust"
    Expected Result: Written content matches read content
    Failure Indicators: Content mismatch, file not found
    Evidence: .sisyphus/evidence/task-7-file-rt.txt

  Scenario: File search finds pattern
    Tool: Bash
    Preconditions: File exists with known content
    Steps:
      1. cargo test -p hermes-runtime test_file_search
      2. WriteTool("test.txt", "needle in haystack") → SearchTool("needle") → assert match
    Expected Result: Search returns matching line
    Failure Indicators: No match found, regex error
    Evidence: .sisyphus/evidence/task-7-search.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add file read/write/search tools`
  - Files: `crates/runtime/src/tool/file.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 8. Web search tool stub

  **What to do**:
  - Create `crates/runtime/src/tool/web.rs`
  - Implement `WebSearchTool` with Tool trait
  - `execute()` returns `ToolOutput { content: "Web search not yet implemented. Please use terminal tool with curl.", is_error: true }`
  - `parameters_schema()` returns valid schema: `query` (string, required)
  - Register in registry but mark as stub

  **Must NOT do**:
  - Don't implement actual HTTP search (Phase 2)
  - Don't add exa/firecrawl/parallel dependencies

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 7, 9)
  - **Blocks**: Task 11
  - **Blocked By**: Task 4

  **References**:
  - `crates/runtime/src/tool/mod.rs` — Tool trait (from Task 4)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: Web search stub returns error message
    Tool: Bash
    Preconditions: Runtime crate compiles
    Steps:
      1. cargo test -p hermes-runtime test_web_search_stub
      2. WebSearchTool::execute({"query": "test"}) → assert is_error: true
    Expected Result: Returns "not yet implemented" message
    Failure Indicators: Panic, success response
    Evidence: .sisyphus/evidence/task-8-web-stub.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add web search tool stub`
  - Files: `crates/runtime/src/tool/web.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 9. MCP + Browser tool stubs

  **What to do**:
  - Create `crates/runtime/src/tool/mcp.rs` — `McpTool` stub, returns "MCP not yet implemented"
  - Create `crates/runtime/src/tool/browser.rs` — `BrowserTool` stub, returns "Browser automation not yet implemented"
  - Both implement Tool trait with valid schemas
  - Register in registry

  **Must NOT do**:
  - Don't implement MCP protocol (2264 lines in Python)
  - Don't add browserbase/playwright dependencies
  - Don't implement stdio subprocess launching

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 7, 8)
  - **Blocks**: Task 11
  - **Blocked By**: Task 4

  **References**:
  - `crates/runtime/src/tool/mod.rs` — Tool trait (from Task 4)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: MCP stub returns error message
    Tool: Bash
    Steps:
      1. cargo test -p hermes-runtime test_mcp_stub
      2. McpTool::execute({}) → assert is_error: true, contains "not yet implemented"
    Expected Result: Stub responds with not-implemented message
    Evidence: .sisyphus/evidence/task-9-mcp-stub.txt

  Scenario: Browser stub returns error message
    Tool: Bash
    Steps:
      1. cargo test -p hermes-runtime test_browser_stub
      2. BrowserTool::execute({}) → assert is_error: true, contains "not yet implemented"
    Expected Result: Stub responds with not-implemented message
    Evidence: .sisyphus/evidence/task-9-browser-stub.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add MCP and browser tool stubs`
  - Files: `crates/runtime/src/tool/mcp.rs`, `crates/runtime/src/tool/browser.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 10. Chat REPL implementation

  **What to do**:
  - Create `crates/runtime/src/chat/mod.rs`
  - Implement `ChatRepl` struct:
    ```rust
    pub struct ChatRepl {
        agent: Agent,
        session_id: Uuid,
    }
    ```
  - `ChatRepl::new(agent: Agent) -> Result<Self>` — creates new session
  - `ChatRepl::resume(agent: Agent, session_id: Uuid) -> Result<Self>` — loads existing session
  - `ChatRepl::run(&mut self) -> Result<()>` — main REPL loop:
    1. Print prompt `> `
    2. Read line from stdin
    3. Handle special commands: `/quit`, `/new`, `/history`, `/help`
    4. Send input to `agent.run_turn()`
    5. Print assistant response
    6. Handle Ctrl+C gracefully (save session, print goodbye)
  - `ChatRepl::run_query(&mut self, query: &str) -> Result<String>` — single-shot mode for `-q` flag
  - Display: colored output — user messages in blue, assistant in green, tool calls in yellow, errors in red
  - Write tests: mock stdin → verify output → verify session persisted

  **Must NOT do**:
  - Don't implement TUI (ratatui) — REPL only
  - Don't implement streaming token-by-token display
  - Don't implement slash commands beyond /quit, /new, /history, /help
  - Don't implement markdown rendering

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (depends on Task 5 agent loop)
  - **Blocks**: Task 11
  - **Blocked By**: Tasks 5, 6, 7

  **References**:
  - Python `cli.py:HermesCLI` — REPL loop with readline, slash commands, tool callbacks
  - Python `cli.py` lines ~300-500 — Input handling, /quit, /new, Ctrl+C handling
  - `crates/runtime/src/agent/mod.rs` — Agent::run_turn() and run_query() (from Task 5)
  - `crates/session-db/src/store.rs` — Session create/resume (from Task 3)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: REPL creates session and responds
    Tool: Bash
    Preconditions: Agent loop and tools working
    Steps:
      1. cargo test -p hermes-runtime test_repl_create_session
      2. ChatRepl::new(mock_agent) → assert session created in DB
    Expected Result: Session exists in store with correct model
    Failure Indicators: No session, panic
    Evidence: .sisyphus/evidence/task-10-repl-create.txt

  Scenario: REPL saves messages on quit
    Tool: Bash
    Preconditions: REPL running
    Steps:
      1. cargo test -p hermes-runtime test_repl_persist
      2. ChatRepl → run_turn("hello") → quit → load session → assert messages exist
    Expected Result: User + assistant messages in session DB
    Failure Indicators: Empty message list
    Evidence: .sisyphus/evidence/task-10-repl-persist.txt

  Scenario: Single-shot query mode
    Tool: Bash
    Preconditions: Agent working
    Steps:
      1. cargo test -p hermes-runtime test_repl_query
      2. ChatRepl::run_query("test query") → assert returns string response
    Expected Result: Returns LLM response as string
    Failure Indicators: Empty response, error
    Evidence: .sisyphus/evidence/task-10-repl-query.txt
  ```

  **Commit**: YES
  - Message: `feat(runtime): add chat REPL with session persistence`
  - Files: `crates/runtime/src/chat/mod.rs`
  - Pre-commit: `cargo test -p hermes-runtime`

- [ ] 11. Wire CLI chat command to runtime

  **What to do**:
  - Update `crates/cli-core/src/commands.rs` `handle_chat()` to use real runtime instead of "coming soon" stub
  - In `handle_chat()`:
    1. Load config → get provider + model + API key
    2. Create LlmProvider based on config
    3. Create ToolRegistry → register terminal, file_read, file_write, file_search, web_search (stub), mcp (stub), browser (stub)
    4. Create SessionStore at `{HERMES_HOME}/sessions.db`
    5. Create Agent with provider, registry, session store, config.max_turns
    6. If `-q` flag → `ChatRepl::run_query()` → print response → exit
    7. If interactive → `ChatRepl::run()` REPL loop
  - Add `hermes-runtime` and `hermes-session-db` as dependencies of `hermes-cli-core`
  - Update `crates/cli/Cargo.toml` if needed
  - Handle credential resolution: env var → auth store → config.yaml
  - Handle missing API key: print helpful error "Run `hermes auth add <provider> --api-key <key>`"

  **Must NOT do**:
  - Don't remove existing CLI structure
  - Don't break other commands (auth, config, etc.)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 3 (depends on Task 10 REPL)
  - **Blocks**: F1-F4
  - **Blocked By**: Tasks 5, 8, 9, 10

  **References**:
  - `crates/cli-core/src/commands.rs:handle_chat()` — Current stub, replace with real implementation
  - `crates/cli-core/src/lib.rs` — `Commands::Chat { query, yolo, resume, toolsets, skills, .. }` fields
  - `crates/cli-core/src/config.rs` — Config loading, model settings, agent settings
  - `crates/cli-core/src/auth.rs` — AuthStore credential resolution
  - `crates/runtime/src/chat/mod.rs` — ChatRepl to invoke (from Task 10)

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: hermes chat -q with mock works
    Tool: Bash
    Preconditions: Full stack wired
    Steps:
      1. cargo build --release -p hermes-cli
      2. Set OPENAI_API_KEY=test_key
      3. cargo run -p hermes-cli -- chat -q "hello" (will fail with real API, but should not crash on startup)
      4. Assert no panic, graceful error if API unreachable
    Expected Result: No crash, meaningful error or response
    Failure Indicators: Panic, unwrap failure, missing dependency
    Evidence: .sisyphus/evidence/task-11-cli-wire.txt

  Scenario: Missing API key gives helpful error
    Tool: Bash
    Preconditions: No API key configured
    Steps:
      1. Remove all API keys from env
      2. cargo run -p hermes-cli -- chat -q "test"
      3. Assert output contains "hermes auth add" suggestion
    Expected Result: Helpful error message
    Failure Indicators: Panic, unclear error, unwrap crash
    Evidence: .sisyphus/evidence/task-11-no-key.txt

  Scenario: Full workspace builds
    Tool: Bash
    Preconditions: All crates integrated
    Steps:
      1. cargo build --workspace
      2. cargo test --workspace
      3. cargo clippy --workspace -- -D warnings
    Expected Result: All pass with zero errors
    Failure Indicators: Compilation errors, clippy warnings, test failures
    Evidence: .sisyphus/evidence/task-11-full-build.txt
  ```

  **Commit**: YES
  - Message: `feat(cli-core): wire chat command to runtime engine`
  - Files: `crates/cli-core/src/commands.rs`, `crates/cli-core/Cargo.toml`
  - Pre-commit: `cargo build --workspace`

- [ ] 12. WeChat gateway adapter stub

  **What to do**:
  - Create `crates/runtime/src/gateway/mod.rs` with gateway traits:
    ```rust
    pub trait PlatformAdapter: Send + Sync {
        async fn start(&mut self) -> Result<(), RuntimeError>;
        async fn send_message(&self, chat_id: &str, message: &str) -> Result<(), RuntimeError>;
        fn name(&self) -> &str;
    }
    ```
  - Create `crates/runtime/src/gateway/wechat.rs`
  - Implement `WechatAdapter` struct stub:
    - Configuration: app_id, app_secret, webhook_url, token
    - `start()` → return "WeChat adapter not yet implemented" error
    - `send_message()` → same stub
    - Message types: text only (no media/group/rich)
  - Add Platform::Wechat + Platform::QQ to existing `crates/cli-core/src/gateway.rs` Platform enum
  - Register adapter in gateway module

  **Must NOT do**:
  - Don't implement actual WeChat API calls
  - Don't implement QR login, contact sync, group management
  - Don't implement media messages

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 10, 11, 13)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 5

  **References**:
  - `crates/cli-core/src/gateway.rs` — Existing GatewayState, Platform enum, PID/service management
  - Python `gateway/platforms/wechat.py` — WeChat adapter reference (if exists)
  - Python `gateway/platforms/` — Platform adapter pattern reference

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: WeChat adapter stub compiles and returns error
    Tool: Bash
    Steps:
      1. cargo test -p hermes-runtime test_wechat_stub
      2. WechatAdapter::start() → assert error contains "not yet implemented"
    Expected Result: Stub compiles and returns not-implemented error
    Evidence: .sisyphus/evidence/task-12-wechat-stub.txt

  Scenario: Platform enum includes WeChat
    Tool: Bash
    Steps:
      1. cargo test -p hermes-cli-core test_platform_wechat
      2. Parse "wechat" → Platform::Wechat → assert round-trip
    Expected Result: Platform::Wechat variant exists
    Evidence: .sisyphus/evidence/task-12-platform.txt
  ```

  **Commit**: YES
  - Message: `feat(gateway): add WeChat adapter stub and platform adapter trait`
  - Files: `crates/runtime/src/gateway/mod.rs`, `crates/runtime/src/gateway/wechat.rs`, `crates/cli-core/src/gateway.rs`
  - Pre-commit: `cargo build --workspace`

- [ ] 13. QQ gateway adapter stub

  **What to do**:
  - Create `crates/runtime/src/gateway/qq.rs`
  - Implement `QqAdapter` struct stub:
    - Configuration: app_id, app_secret, token
    - `start()` → return "QQ adapter not yet implemented" error
    - `send_message()` → same stub
  - Register in gateway module alongside WeChat
  - Add Platform::QQ to existing Platform enum (if not added in Task 12)

  **Must NOT do**:
  - Don't implement actual QQ Bot API calls
  - Don't implement OAuth, group management

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: [`rust-patterns`]

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 10, 11, 12)
  - **Blocks**: F1-F4
  - **Blocked By**: Task 5

  **References**:
  - `crates/runtime/src/gateway/mod.rs` — PlatformAdapter trait (from Task 12)
  - Python `gateway/platforms/qqbot.py` — QQ Bot API v2 adapter reference

  **Acceptance Criteria**:

  **QA Scenarios:**

  ```
  Scenario: QQ adapter stub compiles and returns error
    Tool: Bash
    Steps:
      1. cargo test -p hermes-runtime test_qq_stub
      2. QqAdapter::start() → assert error contains "not yet implemented"
    Expected Result: Stub compiles and returns not-implemented error
    Evidence: .sisyphus/evidence/task-13-qq-stub.txt

  Scenario: Platform enum includes QQ
    Tool: Bash
    Steps:
      1. cargo test -p hermes-cli-core test_platform_qq
      2. Parse "qq" → Platform::QQ → assert round-trip
    Expected Result: Platform::QQ variant exists
    Evidence: .sisyphus/evidence/task-13-platform.txt
  ```

  **Commit**: YES
  - Message: `feat(gateway): add QQ adapter stub`
  - Files: `crates/runtime/src/gateway/qq.rs`
  - Pre-commit: `cargo build --workspace`

---

## Final Verification Wave

- [ ] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists (read file, run command). For each "Must NOT Have": search codebase for forbidden patterns — reject with file:line if found. Check evidence files exist in .sisyphus/evidence/.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [ ] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo build --workspace` + `cargo clippy --workspace -- -D warnings` + `cargo test --workspace`. Review all changed files for: `unwrap()` in non-test code, empty catches, `as any`, unused imports, circular deps.
  Output: `Build [PASS/FAIL] | Clippy [PASS/FAIL] | Tests [N pass/N fail] | VERDICT`

- [ ] F3. **Real Manual QA** — `unspecified-high`
  Start from clean state. Execute EVERY QA scenario from EVERY task. Test cross-task integration. Save to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | VERDICT`

- [ ] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — no missing, no creep. Check "Must NOT do" compliance.
  Output: `Tasks [N/N compliant] | VERDICT`

---

## Commit Strategy

- **T0**: `fix(cli-core): fix empty test body in lib.rs` - lib.rs, common/src/types.rs
- **T1**: `feat(runtime): add crate scaffolding and error types` - crates/runtime/*
- **T2**: `feat(runtime): add LLM provider trait and OpenAI client` - crates/runtime/src/provider/*
- **T3**: `feat(session-db): add SQLite session store` - crates/session-db/*
- **T4**: `feat(runtime): add tool trait and registry` - crates/runtime/src/tool/*
- **T5**: `feat(runtime): add agent core loop` - crates/runtime/src/agent/*
- **T6**: `feat(runtime): add terminal tool` - crates/runtime/src/tool/terminal.rs
- **T7**: `feat(runtime): add file tools` - crates/runtime/src/tool/file.rs
- **T8**: `feat(runtime): add web search tool stub` - crates/runtime/src/tool/web.rs
- **T9**: `feat(runtime): add MCP and browser tool stubs` - crates/runtime/src/tool/mcp.rs, browser.rs
- **T10**: `feat(runtime): add chat REPL` - crates/runtime/src/chat.rs
- **T11**: `feat(cli-core): wire chat command to runtime` - crates/cli-core/src/commands.rs
- **T12**: `feat(gateway): add WeChat adapter stub` - crates/runtime/src/gateway/wechat.rs
- **T13**: `feat(gateway): add QQ adapter stub` - crates/runtime/src/gateway/qq.rs

---

## Success Criteria

### Verification Commands
```bash
cargo build --workspace                                    # Expected: success
cargo test --workspace                                     # Expected: all pass
cargo clippy --workspace -- -D warnings                    # Expected: zero warnings
cargo run -p hermes-cli -- chat -q "Say TEST_OK"           # Expected: contains "TEST_OK"
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] `hermes chat -q` works with real LLM provider
- [ ] Session persistence round-trips
- [ ] No circular dependencies between crates
