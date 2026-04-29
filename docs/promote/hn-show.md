# Hacker News — Show HN

## Title (max 80 chars)
Show HN: Hermes – A 5MB Rust CLI that talks to 22+ LLM providers

## Text

I built Hermes because I was frustrated with AI CLI tools that either (a) only support OpenAI/Anthropic, (b) are slow to start, or (c) don't work well on Windows.

Hermes is a single ~5MB binary written in Rust that gives you a chat-based AI agent in your terminal with:

- 22+ LLM providers out of the box (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Ollama for local models, Zhipu/Kimi/MiniMax for Chinese LLMs, and more)
- Tool execution (shell commands, file read/write, web search, MCP protocol)
- SQLite session persistence — Ctrl+C won't lose your conversation
- Per-turn cost tracking
- Multi-key credential pool with automatic failover

Install and start in 30 seconds:

    curl -fsSL https://raw.githubusercontent.com/yangyifei123/hermes-rust-win/master/install.sh | sh
    hermes auth add openai sk-...
    hermes chat

Or: `cargo install hermes-agent-cli`

It's MIT licensed and the entire codebase is ~20K lines across 5 crates with 467 tests.

https://github.com/yangyifei123/hermes-rust-win

I'd love feedback on what providers or features would make this more useful for you.
