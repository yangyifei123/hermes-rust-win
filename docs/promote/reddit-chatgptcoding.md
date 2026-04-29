# Reddit r/ChatGPTCoding

## Title
Built a terminal AI assistant that works with 22+ providers — not just OpenAI

## Body

I got tired of being locked into one provider, so I built Hermes — a CLI that lets you chat with GPT-4o, Claude, Gemini, DeepSeek, Groq, Ollama (local), and 15+ more from the same tool.

**What it can do:**

- Chat with any LLM in your terminal
- Execute shell commands, read/write files, search the web — the agent decides when
- Switch models mid-conversation with `/model`
- Tracks token usage and cost per turn
- Sessions auto-save to SQLite — Ctrl+C won't lose anything
- Multi-key support with automatic failover

**Quick start:**

    # Install
    cargo install hermes-agent-cli

    # Set key
    hermes auth add openai sk-...

    # Go
    hermes chat

    # Or use Claude
    hermes chat --provider anthropic --model claude-sonnet-4-20250514

    # Or run locally with Ollama (no API key)
    hermes chat --provider ollama

It's written in Rust, ~5MB, starts in under 50ms. MIT licensed.

Repo: https://github.com/yangyifei123/hermes-rust-win

What provider or feature would make you switch from your current setup?
