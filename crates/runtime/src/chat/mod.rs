//! Chat REPL - interactive and single-shot modes

use crate::agent::{Agent, AgentResponse};
use crate::RuntimeError;
use hermes_session_db::MessageRole;
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

        self.agent.run_turn(self.session_id, input).await
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
                })
            }
            "help" => Ok(AgentResponse {
                    content: "Commands:\n  /quit, /exit, /q  — Exit REPL\n  /new, /reset      — Start new session\n  /history           — Show message history\n  /model [name]      — Show or change model\n  /tools              — List available tools\n  /compact            — Compact context (truncate old messages)\n  /save [name]        — Save current session\n  /help               — Show this help".to_string(),
                    tool_calls_made: vec![],
                    turns_used: self.agent.turns_used(),
                    session_id: self.session_id,
                }),
            "model" => {
                let current = self.agent.model_name();
                if _args.is_empty() {
                    Ok(AgentResponse {
                        content: format!("Current model: {}", current),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                    })
                } else {
                    Ok(AgentResponse {
                        content: format!("Model change not yet supported in this version. Current: {}", current),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
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
                })
            }
            "compact" => {
                // Truncate old messages keeping last 10
                let messages = self.agent.get_history(&self.session_id)?;
                let count = messages.len();
                if count > 10 {
                    Ok(AgentResponse {
                        content: format!("Context has {} messages. Compact not yet implemented — would keep last 10.", count),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                    })
                } else {
                    Ok(AgentResponse {
                        content: format!("Context has {} messages. No compaction needed.", count),
                        tool_calls_made: vec![],
                        turns_used: self.agent.turns_used(),
                        session_id: self.session_id,
                    })
                }
            }
            "save" => {
                Ok(AgentResponse {
                    content: format!("Session {} saved.", self.session_id),
                    tool_calls_made: vec![],
                    turns_used: self.agent.turns_used(),
                    session_id: self.session_id,
                })
            }
            _ => Ok(AgentResponse {
                content: format!("Unknown command: /{}. Type /help for commands.", command),
                tool_calls_made: vec![],
                turns_used: self.agent.turns_used(),
                session_id: self.session_id,
            }),
        }
    }

    /// Get the current session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get message history
    pub fn get_history(&self) -> Result<Vec<hermes_session_db::Message>, RuntimeError> {
        self.agent.get_history(&self.session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmProvider, StreamChunk};
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
            Self {
                response: response.to_string(),
                call_count: AtomicU32::new(0),
            }
        }
    }

    impl LlmProvider for MockProvider {
        fn chat_completion(&self, _request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
            let content = self.response.clone();
            Box::pin(async move {
                Ok(ChatResponse {
                    choices: vec![ChatChoice {
                        message: ChatMessage::assistant(&content),
                        finish_reason: Some("stop".to_string()),
                    }],
                })
            })
        }

        fn chat_completion_stream(&self, _request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>, RuntimeError>> + Send + '_>> {
            Box::pin(async { Err(RuntimeError::ProviderError { message: "no stream".into() }) })
        }

        fn name(&self) -> &str { "mock" }
        fn default_model(&self) -> &str { "mock" }
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
}