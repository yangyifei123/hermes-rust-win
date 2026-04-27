# Hermes CLI (Rust)

A fast, native AI agent CLI for Windows. Chat with any LLM, execute tools, manage sessions — all from your terminal.

> Rust rewrite of [hermes-agent](https://github.com/user/hermes-agent) for first-class Windows support.

## Features

- **Multi-Provider**: OpenAI, Anthropic, Groq, DeepSeek, Ollama, and 15+ more via OpenAI-compatible API
- **Streaming**: Real-time token streaming with tool call support
- **Tool System**: Terminal, file I/O, web search — extensible via trait
- **Session Persistence**: SQLite-backed conversation history with resume
- **Context Management**: Tiktoken-based token counting, auto-truncation, `/compact` command
- **Markdown Rendering**: Terminal-formatted output (headers, code, bold, links)
- **Ctrl+C Safety**: Graceful shutdown preserves your session

## Quick Start

```bash
# Build
cargo build --release

# Set up API key
hermes auth add openai sk-...

# Start chatting
hermes chat

# Use a specific model/provider
hermes chat --model gpt-4o --provider openai
```

## Chat Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/model` | Show current model |
| `/model <name>` | Switch model |
| `/model list` | List known models |
| `/compact` | Compress context (truncate old messages) |
| `/history` | Show session history |
| `/tools` | List available tools |
| `/new` | Start new session |
| `/save` | Save current session |
| `/quit` | Exit |

## Architecture

```
hermes-cli/
├── crates/
│   ├── cli/           # Binary entry point (main.rs)
│   ├── cli-core/      # CLI parsing, commands, auth store, config
│   ├── common/        # Shared types: Provider enum, Credentials, Model metadata
│   ├── runtime/       # Agent loop, tool registry, LLM providers, display engine
│   └── session-db/    # SQLite session persistence
```

### Key Components

- **Agent** — Core loop: send messages → LLM → parse tool calls → execute tools → loop
- **LlmProvider** — Trait for provider implementations (OpenAI, Anthropic, Groq...)
- **ToolRegistry** — Dispatches tool calls to registered tools
- **TokenizerRegistry** — Model-specific token counting (tiktoken + heuristic fallback)
- **DisplayEngine** — Tool feedback, spinner, markdown rendering
- **SessionStore** — SQLite WAL-mode storage for messages and sessions

## Provider Support

| Provider | Streaming | Tool Calls | Notes |
|----------|-----------|------------|-------|
| OpenAI | SSE | Yes | GPT-4o, GPT-4, GPT-3.5, o1 |
| Anthropic | SSE | Yes | Claude 4 Opus/Sonnet/Haiku |
| Groq | SSE | Yes | Llama 3.1, Mixtral |
| DeepSeek | SSE | Yes | DeepSeek Chat/Reasoner |
| Ollama | SSE | Yes | Local models |
| OpenRouter | SSE | Yes | Multi-provider routing |
| 15+ more | SSE | Yes | OpenAI-compatible |

## Build Commands

```bash
cargo build --release     # Release build
cargo build               # Dev build
cargo test                # Run all tests
cargo test <test_name>    # Run specific test
cargo clippy              # Lint
cargo fmt                 # Format
```

## Configuration

API keys stored in `~/.hermes/credentials.yaml`:

```yaml
openai:
  api_key: sk-...
anthropic:
  api_key: sk-ant-...
groq:
  api_key: gsk_...
```

Config in `~/.hermes/config.yaml`:

```yaml
model:
  provider: openai
  name: gpt-4o
```

## Stats

- **53 Rust source files**, **17K lines**
- **355 tests**, **0 clippy warnings**
- Supports **20+ LLM providers**

## Requirements

- Rust 1.75+ (stable toolchain, Windows x86_64 target)
- SQLite 3 (bundled via rusqlite)

## License

MIT
