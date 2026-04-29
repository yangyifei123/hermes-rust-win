# Roadmap

## Vision

Hermes is the blazing-fast, Windows-first AI agent CLI that puts you in control. Built in Rust for instant startup and minimal resource usage, it ships with 22+ LLM providers out of the box, persists everything locally in SQLite, and never locks you into a single vendor. Whether you are on Windows, macOS, or Linux, Hermes gives you a powerful, private, and portable AI companion in your terminal.

---

## Done

- [x] **v0.1 -- Core scaffold** -- workspace layout, `common`, `cli-core`, `runtime`, `session-db` crates
- [x] **v0.2 -- CLI foundation** -- Clap derive, command dispatch, config loading (`~/.hermes/config.yaml`)
- [x] **v0.3 -- Session store** -- SQLite persistence for messages and sessions
- [x] **v0.4 -- Auth store** -- credential management from `~/.hermes/credentials.yaml`, env-var fallback
- [x] **v0.5 -- Provider skeleton** -- `LlmProvider` trait, OpenAI and Anthropic adapters

- [x] **v1.0 -- Agent loop** -- `Agent` core loop, chat REPL, `Tool` trait, tool registry
- [x] **v1.1 -- Streaming** -- real-time streaming output for all OpenAI-compatible providers
- [x] **v1.2 -- Context management** -- `TiktokenTokenizer`, `HeuristicTokenizer`, message truncation
- [x] **v1.3 -- Web search tool** -- DuckDuckGo HTML search integration
- [x] **v1.4 -- File tools** -- read, write, search filesystem tools
- [x] **v1.5 -- Terminal tool** -- shell command execution with output capture

- [x] **v1.8 -- Wave 1: Tokenizer & model registry** -- accurate token counting, structured model catalog
- [x] **v1.10 -- Wave 2: Credential pool & provider factories** -- per-provider key management, DeepSeek/Ollama/Azure/OpenRouter/Gemini/Zai/Kimi/MiniMax/Mistral/Cohere adapters
- [x] **v1.14 -- Wave 3: Summarization, display, routing** -- LLM-powered context summarization, polished streaming output, intelligent model routing, skills system

- [x] **v1.14.19 -- P0/P1 fixes & community** -- CI/CD pipeline, `CONTRIBUTING.md`, `LICENSE`, release workflow, usage accumulator

---

## Next (Q2 2025)

- [ ] **MCP protocol -- stdio transport** -- implement Model Context Protocol server/client over stdio, enabling tool discovery and invocation across processes
- [ ] **Context caching** -- cache tokenizer state and compressed context to reduce redundant API calls and latency on repeated sessions
- [ ] **Crates.io publication** -- publish all workspace crates, set up `cargo install hermes-cli` as a first-class install path
- [ ] `hermes models` command -- list available models across all configured providers with capability metadata
- [ ] **Benchmark suite** -- startup time, memory usage, and token throughput benchmarks; publish results in CI

---

## Future (Q3 2025)

- [ ] **MCP protocol -- HTTP/SSE transport** -- extend MCP support to HTTP with server-sent events for remote tool servers
- [ ] **Multi-agent orchestration** -- run multiple specialized agents in parallel or sequence, share context via SQLite
- [ ] **Plugin system** -- load third-party tools and providers as dynamic Rust plugins with a stable ABI
- [ ] **TUI mode** -- rich terminal UI built on `ratatui` with split-pane chat, tool output, and session browser
- [ ] **Documentation site** -- `mdBook`-powered docs hosted on GitHub Pages with guides, API reference, and architecture deep dives
- [ ] **Package managers** -- Homebrew tap, Scoop bucket, and Winget manifest for one-command install on macOS, Linux, and Windows

---

## Long-term (2026)

- [ ] **WASM plugin sandbox** -- run user-authored plugins in a WebAssembly sandbox for safe, portable extensibility
- [ ] **Voice I/O** -- Whisper-based speech-to-text and text-to-speech for hands-free agent interaction
- [ ] **Desktop app** -- Tauri-based GUI wrapping the core runtime for users who prefer a windowed experience
- [ ] **Enterprise features** -- SSO integration, audit logging, team credential management, and on-premise deployment guides

---

*This roadmap is a living document. Priorities shift based on community feedback and contributions. File an issue or start a discussion to help shape what comes next.*
