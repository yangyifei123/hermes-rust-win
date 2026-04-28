# Changelog

All notable changes to Hermes CLI (Rust) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.14.19] — 2026-04-29

### Fixed
- P0/P1 evaluation fixes across runtime and CLI core
- Release pipeline configuration for automated builds

### Added
- Community files (CONTRIBUTING, CODE_OF_CONDUCT, etc.)
- CI/CD workflow for continuous integration

## [1.14.0] — 2026-04-28

### Added — Wave 3: LLM Summarization, Display Polish, Model Routing
- LLM-powered conversation summarization for `/compact` command
- Display engine polish: improved tool feedback formatting and ASCII spinner
- Model routing: automatic model selection based on task context
- Integration tests for provider factories and agent loop
- Skills system with skill dispatch and registration
- Usage accumulator wired into provider completions for token tracking
- Provider registry integration: unified lookup across all 22 providers

### Changed
- CredentialPool wired into provider key resolution for transparent failover
- Individual provider factories for DeepSeek, Ollama, Azure, and OpenRouter
  (replacing shared generic factory)

## [1.13.0] — 2026-04-27

### Added — Wave 2: Generic Provider, Gemini, Credential Pool, Usage Tracking
- Generic OpenAI-compatible provider: single implementation serves 15+ providers
- Google Gemini provider support
- Credential pool with failover and round-robin key rotation
- Usage tracking: prompt/completion token counts accumulated per session
- Provider registry with default configs for all 22 providers (base URLs, model lists, rate limits)

### Changed
- README updated with Wave 2 feature documentation

## [1.12.0] — 2026-04-27

### Added — Wave 1: Infrastructure, Display, Streaming Tools
- Tiktoken-based tokenizer with heuristic fallback for fast approximation
- Model metadata registry: 50+ models with pricing, context windows, and capabilities
- Groq provider support
- CLI module stubs: MCP store, pairing store, plugin store, profile store, webhook store
- Display engine with tool feedback, ASCII spinner, and streaming token output
- System prompt builder with identity, available tools, date, OS, and working directory
- `/compact` command for session context compression
- Graceful Ctrl+C signal handling with cleanup
- Streaming tool call support (OpenAI and Anthropic `tool_use` formats)
- Markdown renderer for terminal output (headers, bold, code fences, links, lists)

## [1.11.0] — 2026-04-26

### Added
- Exponential backoff retry for OpenAI and Anthropic providers

### Fixed
- Explicit JSON parse error handling for tool arguments (no more silent panics)

## [1.10.0] — 2026-04-22

### Added
- Web search tool using DuckDuckGo HTML scraping (no API key needed)
- SSE streaming wired into the agent loop for real-time token display

### Fixed
- Read `base_url` from `credentials.yaml` for custom API endpoints
- Wire streaming into agent loop
- C-drive auto-cleanup at 2 GB threshold

## [1.9.0] — 2026-04-22

### Added — Round 7: Context Window Management
- Token-aware context truncation when messages exceed model context window
- Real session commands: `/save`, `/load`, `/sessions`

### Fixed
- Streaming properly connected to agent loop
- Automatic C-drive cleanup when temp files exceed 2 GB

## [1.8.0] — 2026-04-22

### Added — Round 6: SSE Streaming and Provider Factory
- SSE streaming for OpenAI and Anthropic providers
- Provider factory with per-provider default models and base URLs

### Fixed
- Eliminated all clippy warnings: `FromStr` traits, derivable impls, unused variables

## [1.5.0] — 2026-04-22

### Added — Round 5: Expanded Slash Commands
- `/model` command to show current model, switch models, and list available models
- `/tools` command to list registered tools
- `/compact` command for context compression
- `/save` command to persist current session

## [1.4.0] — 2026-04-22

### Added — Round 4: Token Store and Encoding
- UTF-16 length calculation for accurate Windows compatibility
- AES-128-ECB stub for credential encryption
- Token store for managing API keys

## [1.3.0] — 2026-04-22

### Added — Round 3: Database Resilience
- SQLite write retry with exponential backoff and jitter
- FTS5 full-text query sanitization to prevent injection

## [1.2.0] — 2026-04-22

### Added — Round 2: API Modes and Prompt Caching
- `ApiMode` detection (chat vs. tool-use vs. streaming)
- Prompt caching support for Anthropic-style caching
- Tool argument coercion (string-to-JSON automatic conversion)

## [1.1.0] — 2026-04-22

### Fixed — Round 1: Core Bug Fixes
- Tool calls parsing: handle malformed tool call responses gracefully
- Path security validation for filesystem tools
- Config loading: robust `HERMES_HOME` resolution
- `GH_TOKEN` environment variable for GitHub Actions comment posting

### Added
- AI code review workflow (GitHub Actions)
- Rust format workflow (GitHub Actions)

## [1.0.0] — 2026-04-21

### Added — Core Runtime
- Agent loop: sends messages to LLM, parses tool calls, executes tools, loops
- Chat REPL with multi-line input and slash command dispatch
- Gateway command stubs (placeholder for future HTTP gateway)
- CLI chat command wired to runtime engine
- Session database (SQLite with WAL mode) for message and session persistence
- Tool and provider trait stubs for extensibility

## [0.5.0] — 2026-04-19

### Fixed
- Empty test body compilation error in `cli-core`
- Extracted shared types (`Provider`, `Credentials`, `Model`, `SessionId`) to `common` crate
- Added `uuid v7` dependency for time-ordered session IDs

## [0.4.0] — 2026-04-17

### Added
- Windows service management for gateway process
- PID JSON metadata for process tracking
- 84 tests passing across all crates

### Changed
- Provider registry expanded with additional test coverage
- Config defaults refined for first-run experience

## [0.3.0] — 2026-04-17

### Added
- Gateway command (start/stop/status)
- Cron command for scheduled tasks
- Setup command for first-run configuration
- Doctor command for environment diagnostics
- Update command for self-updating
- Uninstall command for clean removal

## [0.2.0] — 2026-04-17

### Added
- Authentication store with permissions model
- Tools and skills command stubs
- Error handling with user-friendly messages

### Fixed
- Unwrap-or-error patterns replaced with proper error propagation
- Session and model initialization edge cases

## [0.1.0] — 2026-04-17

### Added
- Initial workspace scaffold: `cli`, `cli-core`, `common`, `runtime`, `session-db`
- Multi-provider support: OpenAI, Anthropic, Groq, DeepSeek, Ollama, and 15+ more
- SQLite session store with WAL mode
- Clap-based CLI parsing with command dispatch
- `~/.hermes/credentials.yaml` auth store for API keys
- `~/.hermes/config.yaml` configuration loading

[1.14.19]: https://github.com/user/hermes-rust/compare/v1.14.0...v1.14.19
[1.14.0]: https://github.com/user/hermes-rust/compare/v1.13.0...v1.14.0
[1.13.0]: https://github.com/user/hermes-rust/compare/v1.12.0...v1.13.0
[1.12.0]: https://github.com/user/hermes-rust/compare/v1.11.0...v1.12.0
[1.11.0]: https://github.com/user/hermes-rust/compare/v1.10.0...v1.11.0
[1.10.0]: https://github.com/user/hermes-rust/compare/v1.9.0...v1.10.0
[1.9.0]: https://github.com/user/hermes-rust/compare/v1.8.0...v1.9.0
[1.8.0]: https://github.com/user/hermes-rust/compare/v1.5.0...v1.8.0
[1.5.0]: https://github.com/user/hermes-rust/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/user/hermes-rust/compare/v1.2.0...v1.4.0
[1.3.0]: https://github.com/user/hermes-rust/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/user/hermes-rust/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/user/hermes-rust/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/user/hermes-rust/compare/v0.5.0...v1.0.0
[0.5.0]: https://github.com/user/hermes-rust/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/user/hermes-rust/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/user/hermes-rust/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/user/hermes-rust/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/user/hermes-rust/releases/tag/v0.1.0