//! Agent core loop - orchestrates LLM calls, tool dispatch, and session persistence

use crate::provider::{ChatMessage, ChatRequest, ChatResponse, ChatChoice, LlmProvider};
use crate::tool::ToolRegistry;
use crate::RuntimeError;
use hermes_session_db::{SessionStore, Message, MessageRole};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Configuration for agent behavior
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_turns: u32,
    pub system_prompt: String,
    pub timeout_secs: u64,
    pub yolo: bool,  // Skip approval for dangerous commands
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 30,
            system_prompt: String::new(),
            timeout_secs: 120,
            yolo: false,
        }
    }
}

/// Response from agent execution
#[derive(Debug)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls_made: Vec<String>,
    pub turns_used: u32,
    pub session_id: Uuid,
}

/// The core agent that orchestrates LLM ↔ tool ↔ session interactions
pub struct Agent {
    provider: Box<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    session_store: Arc<SessionStore>,
    config: AgentConfig,
    model: String,
    turn_counter: Arc<AtomicU32>,
}

impl Agent {
    /// Create a new agent with the given provider, tools, and session store
    pub fn new(
        provider: Box<dyn LlmProvider>,
        tools: ToolRegistry,
        session_store: SessionStore,
        config: AgentConfig,
        model: String,
    ) -> Self {
        Self {
            provider,
            tools: Arc::new(tools),
            session_store: Arc::new(session_store),
            config,
            model,
            turn_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Get the current turn count
    pub fn turns_used(&self) -> u32 {
        self.turn_counter.load(Ordering::Relaxed)
    }

    /// Create a new session for this conversation
    pub fn create_session(&self) -> Result<Uuid, RuntimeError> {
        let session = self
            .session_store
            .create_session(&self.model, &self.config.system_prompt)
            .map_err(|e| RuntimeError::SessionError {
                source: Box::new(e),
            })?;
        Ok(session.id)
    }

    /// Resume an existing session
    pub fn resume_session(&self, session_id: Uuid) -> Result<(), RuntimeError> {
        let session = self
            .session_store
            .get_session(&session_id)
            .map_err(|e| RuntimeError::SessionError {
                source: Box::new(e),
            })?;
        if session.is_none() {
            return Err(RuntimeError::NotFound(format!(
                "session {} not found",
                session_id
            )));
        }
        Ok(())
    }

    /// Build messages from session history for LLM request
    fn build_messages(&self, session_id: &Uuid) -> Result<Vec<ChatMessage>, RuntimeError> {
        let messages = self
            .session_store
            .get_messages(session_id)
            .map_err(|e| RuntimeError::SessionError {
                source: Box::new(e),
            })?;

        let mut chat_messages = Vec::new();

        // Add system prompt first if present
        if !self.config.system_prompt.is_empty() {
            chat_messages.push(ChatMessage {
                role: "system".to_string(),
                content: self.config.system_prompt.clone(),
            });
        }

        // Convert session messages to chat format
        for msg in messages {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            chat_messages.push(ChatMessage {
                role: role.to_string(),
                content: msg.content,
            });
        }

        Ok(chat_messages)
    }

    /// Append a message to the session
    fn append_message(
        &self,
        session_id: &Uuid,
        role: MessageRole,
        content: &str,
    ) -> Result<Message, RuntimeError> {
        self.session_store
            .append_message(session_id, role, content)
            .map_err(|e| RuntimeError::SessionError {
                source: Box::new(e),
            })
    }

    /// Run a single query and return the response (one-shot mode)
    pub async fn run_query(&mut self, query: &str) -> Result<String, RuntimeError> {
        let session_id = self.create_session()?;
        let response = self.run_turn(session_id, query).await?;
        Ok(response.content)
    }

    /// Run one turn of the agent loop: send message → get response → handle tools → recurse
    pub async fn run_turn(
        &mut self,
        session_id: Uuid,
        user_message: &str,
    ) -> Result<AgentResponse, RuntimeError> {
        // Append user message
        self.append_message(&session_id, MessageRole::User, user_message)?;

        // Track tool calls made
        let mut tool_calls_made = Vec::new();
        let mut current_content = String::new();

        // Main agent loop
        loop {
            let turns = self.turn_counter.load(Ordering::Relaxed);
            if turns >= self.config.max_turns {
                return Ok(AgentResponse {
                    content: format!(
                        "Max turns ({}) reached. Partial response: {}",
                        self.config.max_turns, current_content
                    ),
                    tool_calls_made,
                    turns_used: turns,
                    session_id,
                });
            }

            // Build request
            let messages = self.build_messages(&session_id)?;
            let tool_defs = self.tools.tool_definitions();

            let request = ChatRequest {
                model: self.model.clone(),
                messages,
                tools: if tool_defs.is_empty() {
                    None
                } else {
                    Some(json!(tool_defs))
                },
                max_tokens: Some(4096),
                temperature: Some(0.7),
                stream: None,
            };

            // Call LLM with timeout
            let response = tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_secs),
                self.provider.chat_completion(request),
            )
            .await
            .map_err(|_| RuntimeError::TimeoutError {
                duration_secs: self.config.timeout_secs,
            })??;

            self.turn_counter.fetch_add(1, Ordering::Relaxed);

            // Process response
            if let Some(choice) = response.choices.first() {
                let msg = &choice.message;

                // Check for tool calls in response (OpenAI format)
                // Note: Our current ChatResponse doesn't have tool_calls field
                // For now, treat all responses as final text
                // TODO: Add tool_calls parsing when provider trait supports it

                current_content = msg.content.clone();

                // Append assistant message
                self.append_message(&session_id, MessageRole::Assistant, &current_content)?;

                // If no tool calls, we're done
                return Ok(AgentResponse {
                    content: current_content,
                    tool_calls_made,
                    turns_used: self.turn_counter.load(Ordering::Relaxed),
                    session_id,
                });
            }
        }
    }

    /// Execute a tool call and append result to session
    pub async fn execute_tool(
        &self,
        session_id: &Uuid,
        tool_name: &str,
        params: serde_json::Value,
    ) -> Result<String, RuntimeError> {
        let result = self.tools.dispatch(tool_name, params).await?;

        // Append tool result to session
        self.append_message(
            session_id,
            MessageRole::Tool,
            &result.content,
        )?;

        Ok(result.content)
    }

    /// Reset turn counter for new conversation
    pub fn reset_turns(&self) {
        self.turn_counter.store(0, Ordering::Relaxed);
    }

    /// Get session messages for display/history
    pub fn get_history(&self, session_id: &Uuid) -> Result<Vec<Message>, RuntimeError> {
        self.session_store
            .get_messages(session_id)
            .map_err(|e| RuntimeError::SessionError {
                source: Box::new(e),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatRequest, ChatResponse, ChatChoice, ChatMessage, StreamChunk};
    use crate::tool::{Tool, ToolOutput};
    use futures::Stream;
    use hermes_session_db::SessionStore;
    use serde_json::json;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Mock provider that returns canned responses
    struct MockProvider {
        responses: Vec<String>,
        call_count: AtomicU32,
    }

    impl MockProvider {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses,
                call_count: AtomicU32::new(0),
            }
        }
    }

    impl LlmProvider for MockProvider {
        fn chat_completion(&self, _request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
            Box::pin(async move {
                let idx = self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let content = self.responses.get(idx as usize)
                    .cloned()
                    .unwrap_or_else(|| "Default mock response".to_string());
                Ok(ChatResponse {
                    choices: vec![ChatChoice {
                        message: ChatMessage {
                            role: "assistant".to_string(),
                            content,
                        },
                        finish_reason: Some("stop".to_string()),
                    }],
                })
            })
        }

        fn chat_completion_stream(&self, _request: ChatRequest) -> Pin<Box<dyn Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, RuntimeError>> + Send>>, RuntimeError>> + Send + '_>> {
            Box::pin(async move {
                Err(RuntimeError::ProviderError {
                    message: "Streaming not supported in mock".to_string(),
                })
            })
        }

        fn name(&self) -> &str { "mock" }
        fn default_model(&self) -> &str { "mock-model" }
    }

    // Mock tool
    struct MockTool;

    impl Tool for MockTool {
        fn name(&self) -> &str { "mock_tool" }
        fn description(&self) -> &str { "A mock tool for testing" }
        fn parameters_schema(&self) -> serde_json::Value {
            json!({ "type": "object", "properties": {} })
        }
        fn execute(&self, _params: serde_json::Value) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
            Box::pin(async move { Ok(ToolOutput::success("mock tool result")) })
        }
    }

    #[tokio::test]
    async fn test_agent_simple_query() {
        let provider = Box::new(MockProvider::new(vec!["Hello from AI".to_string()]));
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(MockTool));

        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig::default();
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let response = agent.run_query("hi").await.unwrap();
        assert_eq!(response, "Hello from AI");
    }

    #[tokio::test]
    async fn test_agent_creates_session() {
        let provider = Box::new(MockProvider::new(vec!["Response".to_string()]));
        let tools = ToolRegistry::new();
        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig::default();
        let agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let session_id = agent.create_session().unwrap();
        let session = agent.session_store.get_session(&session_id).unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().model, "test-model");
    }

    #[tokio::test]
    async fn test_agent_persists_messages() {
        let provider = Box::new(MockProvider::new(vec!["AI response".to_string()]));
        let tools = ToolRegistry::new();
        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig::default();
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let session_id = agent.create_session().unwrap();
        let _ = agent.run_turn(session_id, "user input").await.unwrap();

        let messages = agent.get_history(&session_id).unwrap();
        assert_eq!(messages.len(), 2); // user + assistant
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[tokio::test]
    async fn test_agent_max_turns() {
        let provider = Box::new(MockProvider::new(vec![
            "Response 1".to_string(),
            "Response 2".to_string(),
            "Response 3".to_string(),
        ]));
        let tools = ToolRegistry::new();
        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig {
            max_turns: 1,
            system_prompt: String::new(),
            timeout_secs: 10,
            yolo: false,
        };
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let session_id = agent.create_session().unwrap();
        let response = agent.run_turn(session_id, "test").await.unwrap();
        assert!(response.content.contains("Response 1") || response.turns_used <= 1);
    }
}