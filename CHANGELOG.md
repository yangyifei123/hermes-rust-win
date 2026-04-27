# Changelog

All notable changes to Hermes CLI (Rust) will be documented in this file.

## [0.1.0] — 2026-04-27

### Added
- Core agent loop with tool dispatch and session persistence
- Multi-provider support: OpenAI, Anthropic, Groq, DeepSeek, Ollama, and 15+ more
- SSE streaming with real-time token display
- Streaming tool call support (OpenAI + Anthropic formats)
- Tool system: Terminal, FileRead, FileWrite, FileSearch, WebSearch, MCP, Browser
- SQLite session store with WAL mode and write retry
- Tiktoken-based tokenizer with heuristic fallback
- Token-aware context truncation
- `/compact` command for session compression
- `/model` command to show/switch/list models
- System prompt builder with identity, tools, date, OS, cwd
- Display engine with tool feedback and ASCII spinner
- Markdown renderer for terminal output (headers, bold, code, links, lists)
- Exponential backoff retry for LLM providers
- Graceful Ctrl+C signal handling
- Model metadata registry (50+ models with pricing/capabilities)
- CLI modules: MCP store, pairing store, plugin store, profile store, webhook store
- DuckDuckGo web search tool (no API key needed)
- Base URL from credentials.yaml for custom API endpoints
