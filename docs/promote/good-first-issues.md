# Good First Issues for Hermes

Create these issues on GitHub with the "good first issue" and "help wanted" labels.

---

## Issue 1: Add unit tests for WebSearchTool

**Title:** Add unit tests for web search tool (WebSearchTool)

**Body:**
The `WebSearchTool` in `crates/runtime/src/tool/web.rs` has no dedicated unit tests.

**What to do:**
- Add tests for URL construction, HTML parsing, and result extraction
- Use `mockito` or similar to mock HTTP responses
- Place tests in the same file under `#[cfg(test)] mod tests`

**Files to look at:**
- `crates/runtime/src/tool/web.rs`
- `crates/runtime/src/tool/mod.rs` for the `Tool` trait

**Helpful context:**
- The tool uses DuckDuckGo HTML search, no API key needed
- See `crates/runtime/tests/openai_mock.rs` for an example of mock HTTP testing in this project

**Labels:** good first issue, help wanted

---

## Issue 2: Add tests for file tools (FileReadTool, FileWriteTool, FileSearchTool)

**Title:** Add integration tests for file tools

**Body:**
The file tools in `crates/runtime/src/tool/file.rs` lack test coverage.

**What to do:**
- Test `FileReadTool` reads files correctly and handles missing files
- Test `FileWriteTool` creates/overwrites files
- Test `FileSearchTool` (grep) finds patterns and returns results
- Use `tempfile` crate for test fixtures (already a dev-dependency in cli-core)

**Files to look at:**
- `crates/runtime/src/tool/file.rs`

**Labels:** good first issue, help wanted

---

## Issue 3: Add Mistral provider factory

**Title:** Add Mistral provider factory in runtime

**Body:**
Mistral is listed in the `Provider` enum but needs a dedicated factory in the runtime.

**What to do:**
- Add a Mistral case to `create_provider()` in `crates/runtime/src/provider/mod.rs`
- Mistral uses an OpenAI-compatible API at `https://api.mistral.ai/v1`
- Test that `create_provider("mistral")` returns a provider with the correct name and default model

**Files to look at:**
- `crates/runtime/src/provider/mod.rs` — the factory function
- `crates/runtime/src/provider/providers.rs` — existing factory examples
- `crates/common/src/types.rs` — Provider enum (Mistral already exists)

**Labels:** good first issue, help wanted

---

## Issue 4: Add Cohere provider factory

**Title:** Add Cohere provider factory in runtime

**Body:**
Same as Mistral — Cohere is in the enum but needs a runtime factory.

**What to do:**
- Add a Cohere case to `create_provider()` in `crates/runtime/src/provider/mod.rs`
- Cohere uses an OpenAI-compatible API at `https://api.cohere.ai/v1`
- Add a test

**Files to look at:**
- `crates/runtime/src/provider/mod.rs`
- `crates/runtime/src/provider/providers.rs`

**Labels:** good first issue, help wanted

---

## Issue 5: Improve CLI error messages with suggestions

**Title:** Improve error messages when provider or model is not found

**Body:**
When a user types an invalid provider name, the error message is not helpful:
```
Error: unknown provider: openaai
```

**What to do:**
- Use string similarity (e.g. `strsim` crate) to suggest the closest match
- Example: "unknown provider: openaai. Did you mean `openai`?"
- Apply to both provider and model name resolution

**Files to look at:**
- `crates/common/src/types.rs` — `FromStr` implementation for `Provider`
- `crates/common/src/model_metadata.rs` — `get_model_metadata()`

**Labels:** good first issue, help wanted, enhancement

---

## Issue 6: Add shell completions for bash/zsh/fish

**Title:** Generate shell completion scripts

**Body:**
Hermes uses `clap` which supports shell completion generation via `clap_complete`.

**What to do:**
- Add a `hermes completions <shell>` command that outputs completion scripts
- Support bash, zsh, fish, and PowerShell
- Use `clap_complete::generate()` (already in workspace dependencies)

**Files to look at:**
- `crates/cli-core/src/lib.rs` — add `Completions` command to `Commands` enum
- `clap_complete` is already a dependency in cli-core

**Labels:** good first issue, help wanted, enhancement

---

## Issue 7: Add a `hermes config show` command

**Title:** Add command to display current configuration

**Body:**
Users need a way to see their current config without hunting for the yaml file.

**What to do:**
- Add `hermes config show` that loads and prints the current config
- Show: default provider, model, base_url, and credential paths
- Pretty-print as YAML or a formatted table

**Files to look at:**
- `crates/cli-core/src/config.rs` — Config struct
- `crates/cli-core/src/commands.rs` — existing `handle_config()`

**Labels:** good first issue, help wanted

---

## Issue 8: Add a man page

**Title:** Generate a man page for hermes(1)

**Body:**
Package managers expect a man page for CLI tools.

**What to do:**
- Use `clap_mangen` to generate a man page from the clap `Cli` struct
- Add it as a build step or generate it as a standalone file in `docs/`
- Alternatively, write a manual `hermes.1` file

**Files to look at:**
- `crates/cli-core/src/lib.rs` — `Cli` struct with clap derive

**Labels:** good first issue, help wanted, enhancement
