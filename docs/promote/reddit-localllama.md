# Reddit r/LocalLLaMA

## Title
Hermes CLI — talk to local models (Ollama) and 22+ cloud providers from one tool

## Body

If you're running local models through Ollama and also using cloud APIs, Hermes lets you use all of them from one CLI.

    # Local model, no API key needed
    hermes chat --provider ollama --model llama3

    # Switch to cloud mid-conversation
    /model gpt-4o

    # Or DeepSeek, Groq, Anthropic, Gemini...
    hermes chat --provider deepseek
    hermes chat --provider groq --model llama-3.1-70b-versatile

It's a ~5MB Rust binary. Sessions are saved to SQLite so you can resume later. Has tool execution (shell, files, web search), cost tracking, and multi-key support.

For the Chinese LLM crowd: first-class support for Zhipu (GLM), Kimi (Moonshot), and MiniMax — set your API key and go.

Install: `cargo install hermes-agent-cli`

Repo with full details: https://github.com/yangyifei123/hermes-rust-win
