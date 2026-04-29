# Twitter/X Thread

## Tweet 1
Built a 5MB Rust binary that replaces your AI CLI tool.

22+ LLM providers. <50ms startup. Sessions auto-saved. Cost tracking built-in.

Meet Hermes 🚀

[attach: terminal screenshot]

## Tweet 2
One tool, every provider:

→ OpenAI / Claude / Gemini / DeepSeek
→ Groq (ultra-fast inference)
→ Ollama (local models, no API key)
→ 智谱 GLM / Kimi / MiniMax
→ 15+ more

Switch mid-conversation with /model

## Tweet 3
Install in one line:

macOS/Linux:
curl -fsSL https://raw.githubusercontent.com/yangyifei123/hermes-rust-win/master/install.sh | sh

Or: cargo install hermes-agent-cli

Set your key, type hermes chat, done.

## Tweet 4
What's under the hood:

• Rust workspace (5 crates, 20K lines)
• SQLite session persistence
• Tiktoken-based token counting
• SSE streaming for all providers
• Tool execution (shell, files, web, MCP)
• 467 tests, 0 warnings

## Tweet 5
MIT licensed. Open source. Ready for contributions.

⭐ https://github.com/yangyifei123/hermes-rust-win

What provider should I add next?
