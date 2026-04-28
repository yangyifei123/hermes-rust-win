# Contributing to Hermes

Thank you for your interest in contributing to Hermes! This document provides guidelines and instructions for contributing.

## Prerequisites

- **Rust** 1.75+ (stable toolchain, Windows x86_64 target)
- **Git**
- A supported LLM API key (OpenAI, Anthropic, etc.) for integration testing

## Getting Started

```bash
git clone https://github.com/nousresearch/hermes-rust-win.git
cd hermes-rust-win
cargo build
cargo test
```

## Development Commands

| Command | Description |
|---------|-------------|
| `cargo build` | Debug build |
| `cargo build --release` | Release build (LTO enabled) |
| `cargo test` | Run all tests |
| `cargo test <test_name>` | Run specific test |
| `cargo clippy` | Lint check |
| `cargo fmt` | Format code |
| `cargo fmt -- --check` | Verify formatting |

## Project Architecture

```
hermes-rust/
├── crates/
│   ├── cli/           # Binary entry point (thin wrapper)
│   ├── cli-core/      # CLI parsing, command dispatch, auth store, config
│   ├── common/        # Shared types: Provider enum, Credentials, Model, SessionId
│   ├── runtime/       # Core agent loop, tool registry, LLM providers
│   └── session-db/    # SQLite session persistence (WAL mode)
```

- **cli** — 26-line binary that delegates entirely to `cli-core`
- **cli-core** — Command parsing (Clap), authentication, configuration
- **common** — Shared types with independent error handling (thiserror)
- **runtime** — Agent loop, provider implementations, tool system
- **session-db** — SQLite persistence layer

## Workflow

1. **Fork** the repository
2. **Create a branch**: `git checkout -b feature/your-feature`
3. **Make changes** and ensure:
   - `cargo fmt` passes
   - `cargo clippy` passes with no warnings
   - `cargo test` passes
   - New code has tests where applicable
4. **Commit** with clear, descriptive messages
5. **Push** to your fork
6. **Open a Pull Request**

## PR Checklist

- [ ] Code compiles (`cargo build`)
- [ ] Tests pass (`cargo test`)
- [ ] Lint clean (`cargo clippy`)
- [ ] Formatted (`cargo fmt`)
- [ ] New features include tests
- [ ] Breaking changes documented in PR description

## Code Standards

- Follow `rustfmt` defaults (configured in `rustfmt.toml`)
- Resolve all `clippy` warnings
- Use `thiserror` for library errors, `anyhow` for application errors
- No `unsafe` without a safety comment explaining why it's necessary
- Keep the 5-crate separation: types in `common`, logic in respective crates

## Adding a New Provider

1. Add variant to `Provider` enum in `crates/common/src/types.rs`
2. Add entries to: `as_str()`, `from_str()`, `default_model()`, `default_base_url()`, `env_key()`, `auth_type()`, `all_providers()`
3. Add URL detection in `crates/common/src/provider.rs`
4. Add factory in `crates/runtime/src/provider/mod.rs`
5. Add tests in both `common` and `runtime`

## Reporting Issues

- Use GitHub Issues
- Include: Rust version, OS, command that failed, full error output
- For feature requests, describe the use case

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
