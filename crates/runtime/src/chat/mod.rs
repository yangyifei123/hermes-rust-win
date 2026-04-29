//! Chat REPL - interactive and single-shot modes

use crate::agent::{Agent, AgentResponse, StreamEvent};
use crate::display::MarkdownRenderer;
use crate::RuntimeError;
use futures::StreamExt;
use hermes_session_db::MessageRole;
use std::io::Write;
use uuid::Uuid;

/// Interactive chat REPL
pub struct ChatRepl {
    agent: Agent,
    session_id: Uuid,
}

impl ChatRepl {
    /// Create a new REPL with a fresh session
    pub fn new(agent: Agent) -> Result<Self, RuntimeError> {
        let session_id = agent.create_session()?;
        Ok(Self { agent, session_id })
    }

    /// Resume an existing session
    pub fn resume(agent: Agent, session_id: Uuid) -> Result<Self, RuntimeError> {
        agent.resume_session(session_id)?;
        Ok(Self { agent, session_id })
    }

    /// Run a single-shot query and return the response
    pub async fn run_query(mut agent: Agent, query: &str) -> Result<String, RuntimeError> {
        let response = agent.run_query(query).await?;
        Ok(response)
    }

    /// Run one turn (send message, get response)
    pub async fn run_turn(&mut self, input: &str) -> Result<AgentResponse, RuntimeError> {
        // Handle slash commands
        let trimmed = input.trim();
        if let Some(cmd) = trimmed.strip_prefix('/') {
            return self.handle_command(cmd).await;
        }

        // Use streaming mode if enabled
        if self.agent.streaming_enabled() {
            self.run_turn_streaming(input).await
        } else {
            self.agent.run_turn(self.session_id, input).await
        }
    }

    /// Run one turn with real-time streaming token display.
    ///
    /// Handles both text deltas and streaming tool calls.
    async fn run_turn_streaming(&mut self, input: &str) -> Result<AgentResponse, RuntimeError> {
        // Append user message first
        self.agent.append_message(&self.session_id, MessageRole::User, input)?;

        // Show typing indicator
        print!("Assistant: ");
        let _ = std::io::stdout().flush();

        // Stream events from agent
        let mut stream = self.agent.stream_turn(self.session_id);
        let mut full_content = String::new();
        let mut tool_calls_made = Vec::new();
        let mut md = MarkdownRenderer::new();

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(StreamEvent::Delta(text)) => {
                    print!("{}", md.render(&text));
                    let _ = std::io::stdout().flush();
                    full_content.push_str(&text);
                }
                Ok(StreamEvent::ToolCallStart { name }) => {
                    eprintln!("\n  Running `{}`...", name);
                    tool_calls_made.push(name);
                }
                Ok(StreamEvent::ToolCallsComplete(_tool_calls)) => {
                    // Tool calls are executed inside the agent loop (via run_turn),
                    // but streaming path currently yields them for caller handling.
                    // For now, log and continue — full tool execution in streaming
                    // requires the caller to dispatch tools and re-enter the stream.
                    eprintln!("  Tool calls complete, awaiting follow-up...");
                }
                Err(e) => {
                    println!();
                    return Err(e);
                }
            }
        }

        // Final newline after streaming completes
        println!();

        // Persist the full assistant response to session
        if !full_content.is_empty() {
            self.agent.append_assistant_message(&self.session_id, &full_content)?;
        }

        Ok(AgentResponse {
            content: full_content,
            tool_calls_made,
            turns_used: self.agent.turns_used(),
            session_id: self.session_id,
            token_usage: None,
        })
    }

    /// Handle slash commands
    async fn handle_command(&mut self, cmd: &str) -> Result<AgentResponse, RuntimeError> {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let command = parts[0];
        let _args = parts.get(1).copied().unwrap_or("");

        match command {
            "quit" | "exit" | "q" => Err(RuntimeError::AgentError {
                message: "REPL exited by user".to_string(),
            }),
            "new" | "reset" => {
                self.session_id = self.agent.create_session()?;
                self.agent.reset_turns();
                Ok(AgentResponse {
                    content: "New session started.".to_string(),
                    tool_calls_made: vec![],
                    turns_used: 0,
                    session_id: self.session_id,
                    token_usage: None,
                })
            }
            "history" => {
                let messages = self.agent.get_history(&self.session_id)?;
                let history: Vec<String> = messages
                    .iter()
                    .map(|m| {
                        let role = match m.role {
                            MessageRole::System => "system",
                            MessageRole::User => "you",
                            MessageRole::Assistant => "hermes",
                            MessageRole::Tool => "tool",
                        };
                        format!("[{}] {}", role, m.content)
                    })
                    .collect();
                Ok(AgentResponse {
                    content: if history.is_empty() {
                        "No messages yet.".to_string()
                    } else {
                        history.join("\n")
                    },
                    tool_calls_made: vec![],
                    turns_used: self.agent.turns_used(),
                    session_id: self.session_id,
                    token_usage: None,
                })
            }
            "help" => Ok(AgentResponse {
                    content: "Commands:\n  /quit, /exit, /q  — Exit REPL\n  /new, /reset      — Start new session\n  /history           — Show message history\n  /model             — Show current model\n  /model <name>      — Change model\n  /model list        — List known models\n  /system            — Show current system prompt\n  /tools              — List available tools\n  /compact            — Compact context (truncate old messages)\n  /save [name]        — Save current session\n  /help               — Show this help".to_string(),
                    tool_calls_made: vec![],
                    turns_used: self.agent.turns_used(),
                    session_id: self.session_id,
                    token_usage: None,
                }),
            "model" => {
                if _args.is_empty() {
                    // /model — show current model
                    let current = self.agent.model_name();
                    Ok(AgentResponse {
                        content: format!("Current model: {}", current),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                } else if _args == "list" || _args == "--list" {
                    // /model list — show known models grouped by provider
                    let groups = Agent::known_models();
                    let mut output = String::from("Known models:\n");
                    for (provider, models) in &groups {
                        output.push_str(&format!("\n  [{}]\n", provider));
                        for m in models {
                            output.push_str(&format!("    {}\n", m));
                        }
                    }
                    Ok(AgentResponse {
                        content: output,
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                } else {
                    // /model <name> — change model
                    let new_model = _args.trim().to_string();
                    self.agent.set_model(new_model.clone());
                    Ok(AgentResponse {
                        content: format!("Model changed to: {}", new_model),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                }
            }
            "tools" => {
                let tools = self.agent.list_tools();
                let list: Vec<String> = tools.iter().map(|(n, d)| format!("  {} — {}", n, d)).collect();
                Ok(AgentResponse {
                    content: format!("Available tools:\n{}", list.join("\n")),
                    tool_calls_made: vec![],
                    turns_used: self.agent.turns_used(),
                    session_id: self.session_id,
                    token_usage: None,
                })
            }
            "system" => {
                if _args.is_empty() {
                    // /system — show current system prompt
                    let prompt = self.agent.system_prompt().to_string();
                    let display = if prompt.is_empty() { "(empty)".to_string() } else { prompt };
                    Ok(AgentResponse {
                        content: display,
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                } else if _args.trim() == "reset" {
                    self.agent.set_system_prompt(String::new());
                    Ok(AgentResponse {
                        content: "System prompt cleared.".to_string(),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                } else {
                    // /system <text> — set system prompt
                    let new_prompt = _args.trim().to_string();
                    self.agent.set_system_prompt(new_prompt.clone());
                    Ok(AgentResponse {
                        content: format!("System prompt set ({} chars).", new_prompt.len()),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                }
            }
            "compact" => {
                let (before, after, tokens_saved) = self.agent.compact_session(&self.session_id, 10)?;
                if before == after {
                    Ok(AgentResponse {
                        content: format!("Context has {} messages. No compaction needed.", before),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                } else {
                    Ok(AgentResponse {
                        content: format!(
                            "Compacted: {} → {} messages (~{} tokens saved). Kept system prompt + 10 recent messages.",
                            before, after, tokens_saved
                        ),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                        token_usage: None,
                    })
                }
            }
            "save" => {
                Ok(AgentResponse {
                    content: format!("Session {} saved.", self.session_id),
                    tool_calls_made: vec![],
                    turns_used: self.agent.turns_used(),
                    session_id: self.session_id,
                    token_usage: None,
                })
            }
            _ => Ok(AgentResponse {
                content: format!("Unknown command: /{}. Type /help for commands.", command),
                tool_calls_made: vec![],
                turns_used: self.agent.turns_used(),
                session_id: self.session_id,
                token_usage: None,
            }),
        }
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get a reference to the underlying agent (for usage stats, model info, etc.)
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// Get a mutable reference to the underlying agent (for skill loading, etc.)
    pub fn agent_mut(&mut self) -> &mut Agent {
        &mut self.agent
    }

    /// Get message history
    pub fn get_history(&self) -> Result<Vec<hermes_session_db::Message>, RuntimeError> {
        self.agent.get_history(&self.session_id)
    }

    /// Graceful shutdown: save session state and print farewell.
    ///
    /// Called when the user presses Ctrl+C or the REPL exits cleanly.
    /// Returns the session ID for reference.
    pub fn graceful_shutdown(&self) -> Uuid {
        // Session messages are already persisted on each turn via append_message,
        // so there's nothing extra to flush. This method exists as a hook for
        // future cleanup (temp files, MCP connections, etc.).
        self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{
        ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmProvider, StreamChunk,
    };
    use crate::tool::ToolRegistry;
    use crate::{AgentConfig, RuntimeError};
    use futures::Stream;
    use hermes_session_db::SessionStore;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::AtomicU32;

    struct MockProvider {
        response: String,
        call_count: AtomicU32,
    }

    impl MockProvider {
        fn new(response: &str) -> Self {
            Self { response: response.to_string(), call_count: AtomicU32::new(0) }
        }
    }

    impl LlmProvider for MockProvider {
        fn chat_completion(
            &self,
            _request: ChatRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
            let content = self.response.clone();
            Box::pin(async move {
                Ok(ChatResponse {
                    choices: vec![ChatChoice {
                        message: ChatMessage::assistant(&content),
                        finish_reason: Some("stop".to_string()),
                    }],
                    usage: None,
                })
            })
        }

        fn chat_completion_stream(
            &self,
            _request: ChatRequest,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>,
                            RuntimeError,
                        >,
                    > + Send
                    + '_,
            >,
        > {
            Box::pin(async { Err(RuntimeError::ProviderError { message: "no stream".into() }) })
        }

        fn name(&self) -> &str {
            "mock"
        }
        fn default_model(&self) -> &str {
            "mock"
        }
    }

    fn make_agent() -> Agent {
        let provider = Box::new(MockProvider::new("Mock response"));
        let tools = ToolRegistry::new();
        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig { streaming: false, ..AgentConfig::default() };
        Agent::new(provider, tools, store, config, "test-model".to_string())
    }

    #[tokio::test]
    async fn test_repl_create_session() {
        let agent = make_agent();
        let repl = ChatRepl::new(agent).unwrap();
        assert!(!repl.session_id().is_nil());
    }

    #[tokio::test]
    async fn test_repl_query() {
        let agent = make_agent();
        let response = ChatRepl::run_query(agent, "hello").await.unwrap();
        assert_eq!(response, "Mock response");
    }

    #[tokio::test]
    async fn test_repl_persist() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        repl.run_turn("hello").await.unwrap();
        let messages = repl.get_history().unwrap();
        assert_eq!(messages.len(), 2); // user + assistant
    }

    #[tokio::test]
    async fn test_repl_new_command() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let old_session = repl.session_id();
        let result = repl.run_turn("/new").await.unwrap();
        assert_ne!(result.session_id, old_session);
    }

    #[tokio::test]
    async fn test_model_show_current() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let result = repl.run_turn("/model").await.unwrap();
        assert!(result.content.contains("test-model"));
    }

    #[tokio::test]
    async fn test_model_change() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let result = repl.run_turn("/model gpt-4o").await.unwrap();
        assert_eq!(result.content, "Model changed to: gpt-4o");
        // Verify it stuck
        let result2 = repl.run_turn("/model").await.unwrap();
        assert!(result2.content.contains("gpt-4o"));
    }

    #[tokio::test]
    async fn test_model_list() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let result = repl.run_turn("/model list").await.unwrap();
        assert!(result.content.contains("Known models:"));
        assert!(result.content.contains("[openai]"));
        assert!(result.content.contains("gpt-4o"));
        assert!(result.content.contains("[anthropic]"));
    }

    #[tokio::test]
    async fn test_model_list_flag() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let result = repl.run_turn("/model --list").await.unwrap();
        assert!(result.content.contains("Known models:"));
    }

    #[tokio::test]
    async fn test_known_models_not_empty() {
        let models = Agent::known_models();
        assert!(!models.is_empty());
        for (provider, model_list) in &models {
            assert!(!provider.is_empty());
            assert!(!model_list.is_empty());
        }
    }

    #[tokio::test]
    async fn test_system_show_empty() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let result = repl.run_turn("/system").await.unwrap();
        assert_eq!(result.content, "(empty)");
    }

    #[tokio::test]
    async fn test_system_set_and_show() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        let result = repl.run_turn("/system You are a pirate").await.unwrap();
        assert!(result.content.contains("16 chars"));
        let result2 = repl.run_turn("/system").await.unwrap();
        assert_eq!(result2.content, "You are a pirate");
    }

    #[tokio::test]
    async fn test_system_reset() {
        let agent = make_agent();
        let mut repl = ChatRepl::new(agent).unwrap();
        repl.run_turn("/system test prompt").await.unwrap();
        let result = repl.run_turn("/system reset").await.unwrap();
        assert_eq!(result.content, "System prompt cleared.");
        let result2 = repl.run_turn("/system").await.unwrap();
        assert_eq!(result2.content, "(empty)");
    }
}
