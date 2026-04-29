# Reddit r/rust

## Title
Built a multi-provider AI agent CLI in Rust — 22+ LLM providers, 5MB binary, 467 tests

## Body

Hey r/rust,

I've been working on **Hermes** — a terminal-based AI agent CLI written in pure Rust. It's been through 3 major waves of development and I just published it to crates.io.

**What it does:** Gives you a ChatGPT-like experience in your terminal, but works with 22+ LLM providers. The agent can execute tools (shell, files, web search, MCP) and maintains conversation history in SQLite.

**Why Rust:** I wanted <50ms startup and ~5MB binary. Python-based alternatives take 2-3 seconds to start and weigh 50-100MB.

**Architecture:** 5-crate workspace — `common` (types, model metadata), `session-db` (SQLite/WAL), `runtime` (agent loop, providers, tools), `cli-core` (commands, auth), `cli` (thin binary). Each crate has its own error type (thiserror), application layer uses anyhow.

**Some things I'm happy with:**

- `OnceLock`-based lazy provider registry (replaced unsafe static mut)
- Tiktoken-based token counting with CJK-aware heuristic fallback
- Credential pool with round-robin and failover for multi-key setups
- Model router with Cheapest/MostCapable/Balanced strategies
- Full SSE streaming for both OpenAI-compatible and Anthropic APIs

**Install:** `cargo install hermes-agent-cli`

**Repo:** https://github.com/yangyifei123/hermes-rust-win

Happy to answer questions about the architecture or accept PRs!
