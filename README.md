# Hermes

The blazing-fast, multi-provider AI agent CLI. Written in Rust.

```
$ hermes chat --model gpt-4o
> Write a Python HTTP server

Here's a minimal HTTP server using only the standard library:

    from http.server import HTTPServer, SimpleHTTPRequestHandler

    server = HTTPServer(("0.0.0.0", 8080), SimpleHTTPRequestHandler)
    server.serve_forever()

Run it with `python server.py` — serves files from the current directory on port 8080.
```

## Why Hermes?

| | Hermes | aider | opencode |
|---|---|---|---|
| **Providers** | 22+ out of the box | OpenAI/Anthropic only | Limited |
| **Chinese LLMs** | Zhipu, Kimi, MiniMax | No | No |
| **Local models** | Ollama (built-in) | Experimental | No |
| **Install size** | ~5 MB | ~50 MB (Python) | ~100 MB (Node) |
| **Startup time** | < 50ms | ~2s | ~3s |
| **Session resume** | SQLite, Ctrl+C safe | No | Limited |
| **Cost tracking** | Per-turn, per-session | No | No |

## Install

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/yangyifei123/hermes-rust-win/master/install.sh | sh

# Windows PowerShell
irm https://raw.githubusercontent.com/yangyifei123/hermes-rust-win/master/install.ps1 | iex

# Or build from source
cargo install hermes-agent-cli
```

## Quick Start

```bash
# 1. Set your API key
hermes auth add openai sk-...

# 2. Start chatting
hermes chat

# That's it. Done.
```

```bash
# Use any provider
hermes chat --provider anthropic --model claude-sonnet-4-20250514
hermes chat --provider deepseek
hermes chat --provider ollama     # local models, no API key needed

# Non-interactive: pipe a single query
hermes chat --query "explain this error: $?" --quiet

# List all supported models
hermes models
hermes models --provider openai --pricing
```

## Features

**22+ LLM Providers** — OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama, Azure, OpenRouter, Zhipu (GLM), Kimi (Moonshot), MiniMax, Mistral, Cohere, HuggingFace, and more. Switch freely.

**Tool System** — Execute shell commands, read/write files, search the web, call MCP servers. The agent decides when to use them.

**Session Persistence** — SQLite-backed. Ctrl+C won't lose your conversation. Resume anytime with `hermes sessions`.

**Context Management** — Tiktoken-based token counting, auto-truncation when context fills up, `/compact` to summarize and free space.

**Cost Tracking** — See token usage and estimated cost after every turn. Full session summary on exit.

**Credential Pool** — Add multiple API keys per provider. Hermes load-balances and fails over automatically.

**Skills System** — Save and load reusable prompt templates with `/skill`.

**Streaming** — Real-time token streaming with tool call support.

**Markdown Rendering** — Headers, code blocks, bold, links — all rendered in your terminal.

## Chat Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/model <name>` | Switch model mid-conversation |
| `/compact` | Summarize and compress context |
| `/tools` | List available tools |
| `/skill list` | List skill templates |
| `/skill <name>` | Load a skill |
| `/history` | Show session history |
| `/save` | Save current session |
| `/quit` | Exit (session auto-saved) |

## Configuration

API keys in `~/.hermes/credentials.yaml`:

```yaml
openai:
  api_key: sk-...
anthropic:
  api_key: sk-ant-...
ollama: {}  # no key needed
```

Config in `~/.hermes/config.yaml`:

```yaml
model:
  provider: openai
  name: gpt-4o
```

## Architecture

```
hermes/
├── crates/
│   ├── cli/           # Binary entry point
│   ├── cli-core/      # CLI parsing, commands, auth, config
│   ├── common/        # Shared types, model metadata, provider detection
│   ├── runtime/       # Agent loop, LLM providers, tool system, display
│   └── session-db/    # SQLite persistence (WAL mode)
```

## Build from Source

```bash
git clone https://github.com/yangyifei123/hermes-rust-win.git
cd hermes-rust-win
cargo build --release
```

Requires Rust 1.75+.

## Stats

- 63 Rust source files, 20K lines
- 467 tests, 0 clippy warnings
- 22+ LLM providers, 50+ model presets
- ~5 MB release binary

## License

MIT
