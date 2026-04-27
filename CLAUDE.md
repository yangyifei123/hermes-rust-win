# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build --release     # Release build
cargo build               # Dev build
cargo test                # Run all tests
cargo test <test_name>    # Run specific test
cargo clippy              # Lint
cargo fmt                 # Format
```

## Architecture

```
hermes-rust/
├── crates/
│   ├── cli/           # Binary entry point
│   ├── cli-core/      # CLI parsing, command dispatch, auth store, config
│   ├── common/        # Shared types: Provider enum, Credentials, Model, SessionId
│   ├── runtime/       # Core agent loop, tool registry, LLM providers
│   └── session-db/    # SQLite session persistence (messages, sessions)
```

### Key Components

**Runtime (`crates/runtime`)**
- `Agent` — Core loop: sends messages → LLM → parses tool calls → executes tools → loops
- `AgentConfig` — max_turns, system_prompt, streaming, yolo mode
- `IterationBudget` — tracks turns per conversation
- `ToolRegistry` — dispatches tool calls to registered tools
- `Tool` trait — implement this to add new tools

**Providers (`crates/runtime/src/provider/`)**
- `LlmProvider` trait — `chat_completion()` and `chat_completion_stream()` 
- `create_provider()` factory — creates OpenAI, Anthropic, or Groq providers
- Provider selection: CLI flag > config > default

**Session Store (`crates/session-db`)**
- SQLite database storing messages and sessions
- `Message` with role (System/User/Assistant/Tool) and content
- Session has model, system prompt, timestamps

**CLI Core (`crates/cli-core`)**
- Clap derive for CLI parsing — all commands in `lib.rs`
- `handle_chat()` wires CLI to runtime Agent
- AuthStore manages API keys from `~/.hermes/credentials.yaml`
- Config loads from `~/.hermes/config.yaml`

### Tool System

Tools implement the `Tool` trait with `name`, `description`, `parameters_schema`, and `execute()`. Built-in tools:
- `TerminalTool` — execute shell commands
- `FileReadTool`, `FileWriteTool`, `FileSearchTool` — filesystem operations
- `WebSearchTool` — DuckDuckGo HTML search
- `McpTool` — MCP protocol tools
- `BrowserTool` — browser automation

### Context Management

`crates/runtime/src/context/` handles token counting:
- `TiktokenTokenizer` — tiktoken-based accurate counting
- `HeuristicTokenizer` — fast approximation
- Messages are truncated if they exceed `max_context_tokens`

### Provider Resolution Priority

1. CLI flag (`--provider`)
2. Config `model.provider`
3. Default: `openai`

API key resolution:
1. Auth store credential
2. `{PROVIDER}_API_KEY` env var
3. `OPENAI_API_KEY` fallback

Base URL resolution:
1. Auth store `base_url`
2. Config `model.base_url`
3. Provider default

## Provider Support

Providers use OpenAI-compatible API except Anthropic which uses its own Messages API. The `Provider` enum in `crates/common/src/types.rs` defines 20+ providers including OpenAI, Anthropic, Groq, DeepSeek, Ollama, Gemini, Zai, Kimi, MiniMax, OpenRouter, etc.

## Rust Version

Requires Rust 1.75+ (stable toolchain, Windows x86_64 target).
