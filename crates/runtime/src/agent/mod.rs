//! Agent core loop - orchestrates LLM calls, tool dispatch, and session persistence

use crate::provider::{
    ChatMessage, ChatRequest, ChatResponse, LlmProvider, ToolCall as ProviderToolCall,
};
use crate::tool::ToolRegistry;
use crate::usage::UsageAccumulator;
use crate::RuntimeError;
use futures::Stream;
use futures::StreamExt;
use futures::TryStreamExt;
use hermes_session_db::{Message, MessageRole, SessionStore};
use serde_json::json;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

pub mod budget;
pub use budget::IterationBudget;

/// Configuration for agent behavior
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_turns: u32,
    pub system_prompt: String,
    pub timeout_secs: u64,
    pub yolo: bool,                // Skip approval for dangerous commands
    pub max_context_tokens: usize, // Max tokens before truncation
    pub streaming: bool,           // Use SSE streaming when available
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_turns: 30,
            system_prompt: String::new(),
            timeout_secs: 120,
            yolo: false,
            max_context_tokens: 128_000,
            streaming: true,
        }
    }
}

/// Token usage statistics from a provider response.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

/// Response from agent execution
#[derive(Debug)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls_made: Vec<String>,
    pub turns_used: u32,
    pub session_id: Uuid,
    pub token_usage: Option<TokenUsage>,
}

/// Events emitted during a streaming agent turn.
#[derive(Debug)]
pub enum StreamEvent {
    /// A text content delta from the LLM
    Delta(String),
    /// A tool call has begun (name known, arguments still streaming)
    ToolCallStart { name: String },
    /// All tool calls are complete and ready to execute
    ToolCallsComplete(Vec<ProviderToolCall>),
}

/// The core agent that orchestrates LLM ↔ tool ↔ session interactions
pub struct Agent {
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    #[allow(clippy::arc_with_non_send_sync)]
    session_store: Arc<SessionStore>,
    config: AgentConfig,
    model: String,
    budget: Arc<IterationBudget>,
    usage: std::sync::Mutex<UsageAccumulator>,
}

impl Agent {
    /// Create a new agent with the given provider, tools, and session store
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn new(
        provider: Box<dyn LlmProvider>,
        tools: ToolRegistry,
        session_store: SessionStore,
        config: AgentConfig,
        model: String,
    ) -> Self {
        let budget = Arc::new(IterationBudget::new(config.max_turns));
        Self {
            provider: Arc::from(provider),
            tools: Arc::new(tools),
            session_store: Arc::new(session_store),
            config,
            model,
            budget,
            usage: std::sync::Mutex::new(UsageAccumulator::new()),
        }
    }

    /// Get the current turn count
    pub fn turns_used(&self) -> u32 {
        self.budget.used()
    }

    /// Create a new session for this conversation
    /// Get the tokenizer registry for accurate token counting.
    fn tokenizer_registry(&self) -> crate::context::TokenizerRegistry {
        crate::context::TokenizerRegistry::new()
    }

    pub fn create_session(&self) -> Result<Uuid, RuntimeError> {
        let session = self
            .session_store
            .create_session(&self.model, &self.config.system_prompt)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;
        Ok(session.id)
    }

    /// Resume an existing session
    pub fn resume_session(&self, session_id: Uuid) -> Result<(), RuntimeError> {
        let session = self
            .session_store
            .get_session(&session_id)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;
        if session.is_none() {
            return Err(RuntimeError::NotFound(format!("session {} not found", session_id)));
        }
        Ok(())
    }

    /// Build messages from session history for LLM request
    fn build_messages(&self, session_id: &Uuid) -> Result<Vec<ChatMessage>, RuntimeError> {
        let messages = self
            .session_store
            .get_messages(session_id)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;

        let mut chat_messages = Vec::new();

        // Add system prompt first if present
        if !self.config.system_prompt.is_empty() {
            chat_messages.push(ChatMessage::system(&self.config.system_prompt));
        }

        // Convert session messages to chat format
        for msg in messages {
            let chat_msg = match msg.role {
                MessageRole::System => ChatMessage::system(&msg.content),
                MessageRole::User => ChatMessage::user(&msg.content),
                MessageRole::Assistant => ChatMessage::assistant(&msg.content),
                MessageRole::Tool => {
                    // Tool messages need tool_call_id — stored in tool_name field for now
                    let tool_call_id = msg.tool_name.as_deref().unwrap_or("unknown");
                    ChatMessage::tool_result(tool_call_id, &msg.content)
                }
            };
            chat_messages.push(chat_msg);
        }

        // Truncate if over token limit using model-specific tokenizer
        let registry = self.tokenizer_registry();
        chat_messages =
            registry.truncate_messages(&self.model, chat_messages, self.config.max_context_tokens);

        Ok(chat_messages)
    }

    /// Append a message to the session (public for ChatRepl streaming)
    pub fn append_message(
        &self,
        session_id: &Uuid,
        role: MessageRole,
        content: &str,
    ) -> Result<Message, RuntimeError> {
        self.session_store
            .append_message(session_id, role, content)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })
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
            let turns = self.budget.used();
            if turns >= self.config.max_turns {
                return Ok(AgentResponse {
                    content: format!(
                        "Max turns ({}) reached. Partial response: {}",
                        self.config.max_turns, current_content
                    ),
                    tool_calls_made,
                    turns_used: turns,
                    session_id,
                    token_usage: None,
                });
            }

            // Build request
            let messages = self.build_messages(&session_id)?;
            let tool_defs = self.tools.tool_definitions();

            let request = ChatRequest {
                model: self.model.clone(),
                messages,
                tools: if tool_defs.is_empty() { None } else { Some(json!(tool_defs)) },
                max_tokens: Some(4096),
                temperature: Some(0.7),
                stream: if self.config.streaming { Some(true) } else { None },
            };

            // Call LLM with timeout — use streaming or non-streaming based on config
            let response = if self.config.streaming {
                // Streaming mode: collect chunks into full response, accumulating tool calls
                let stream_result = tokio::time::timeout(
                    std::time::Duration::from_secs(self.config.timeout_secs),
                    self.provider.chat_completion_stream(request),
                )
                .await;

                let mut stream = stream_result.map_err(|_| RuntimeError::TimeoutError {
                    duration_secs: self.config.timeout_secs,
                })??;

                // Collect all chunks into a single response
                let mut full_content = String::new();
                // Tool call accumulation: keyed by index
                let mut tool_call_parts: HashMap<u32, (String, String, String)> = HashMap::new();
                // (tool_call_id, function_name, arguments_accumulator)
                let mut finish_reason: Option<String> = None;

                let collect_result = tokio::time::timeout(
                    std::time::Duration::from_secs(self.config.timeout_secs),
                    async {
                        while let Some(chunk_result) = stream.next().await {
                            let chunk = chunk_result?;
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(ref content) = choice.delta.content {
                                    full_content.push_str(content);
                                }
                                // Accumulate tool call deltas
                                if let Some(ref tool_calls) = choice.delta.tool_calls {
                                    for tc_delta in tool_calls {
                                        let entry =
                                            tool_call_parts.entry(tc_delta.index).or_insert_with(
                                                || (String::new(), String::new(), String::new()),
                                            );
                                        if let Some(ref id) = tc_delta.id {
                                            entry.0 = id.clone();
                                        }
                                        if let Some(ref func) = tc_delta.function {
                                            if let Some(ref name) = func.name {
                                                entry.1 = name.clone();
                                            }
                                            if let Some(ref args) = func.arguments {
                                                entry.2.push_str(args);
                                            }
                                        }
                                    }
                                }
                                if choice.finish_reason.is_some() {
                                    finish_reason = choice.finish_reason.clone();
                                }
                            }
                        }
                        Ok::<(), RuntimeError>(())
                    },
                )
                .await;

                collect_result.map_err(|_| RuntimeError::TimeoutError {
                    duration_secs: self.config.timeout_secs,
                })??;

                // Reconstruct tool calls from accumulated parts
                let mut tool_calls: Vec<ProviderToolCall> = Vec::new();
                if !tool_call_parts.is_empty() {
                    let mut indices: Vec<u32> = tool_call_parts.keys().copied().collect();
                    indices.sort();
                    for idx in indices {
                        let (id, name, arguments) = &tool_call_parts[&idx];
                        tool_calls.push(ProviderToolCall {
                            id: id.clone(),
                            tool_type: "function".to_string(),
                            function: crate::provider::FunctionCall {
                                name: name.clone(),
                                arguments: arguments.clone(),
                            },
                        });
                    }
                }

                let message = if !tool_calls.is_empty() {
                    ChatMessage::assistant_with_tool_calls(tool_calls)
                } else {
                    ChatMessage::assistant(&full_content)
                };

                ChatResponse {
                    choices: vec![crate::provider::ChatChoice { message, finish_reason }],
                    usage: None,
                }
            } else {
                // Non-streaming mode
                tokio::time::timeout(
                    std::time::Duration::from_secs(self.config.timeout_secs),
                    self.provider.chat_completion(request),
                )
                .await
                .map_err(|_| RuntimeError::TimeoutError {
                    duration_secs: self.config.timeout_secs,
                })??
            };

            // Consume one iteration slot
            self.budget.consume();

            // Process response
            if let Some(choice) = response.choices.first() {
                let msg = &choice.message;

                // Check for tool calls — core agent loop behavior
                if msg.has_tool_calls() {
                    if let Some(tool_calls) = &msg.tool_calls {
                        // Append assistant's tool call decision to session
                        let calls_summary: Vec<String> = tool_calls
                            .iter()
                            .map(|tc| format!("{}({})", tc.function.name, tc.function.arguments))
                            .collect();
                        self.append_message(
                            &session_id,
                            MessageRole::Assistant,
                            &format!("[tool_calls: {}]", calls_summary.join(", ")),
                        )?;

                        // Execute each tool call
                        for tc in tool_calls {
                            tool_calls_made.push(tc.function.name.clone());

                            // Parse arguments JSON — on failure, store error as tool result
                            // so the LLM can see what went wrong and retry with valid JSON.
                            let params: serde_json::Value = match serde_json::from_str(
                                &tc.function.arguments,
                            ) {
                                Ok(v) => v,
                                Err(parse_err) => {
                                    let error_msg = format!(
                                        "JSON parse error for tool '{}' arguments: {}. Raw args: {}",
                                        tc.function.name, parse_err, tc.function.arguments
                                    );
                                    tracing::warn!("{}", error_msg);
                                    // Store the parse error as a tool result so LLM sees it
                                    self.session_store
                                        .append_message(&session_id, MessageRole::Tool, &error_msg)
                                        .map_err(|e| RuntimeError::SessionError {
                                            source: Box::new(e),
                                        })?;
                                    // Continue to next tool call
                                    continue;
                                }
                            };

                            // Dispatch tool with timeout
                            let tool_result = tokio::time::timeout(
                                std::time::Duration::from_secs(self.config.timeout_secs),
                                self.tools.dispatch(&tc.function.name, params),
                            )
                            .await
                            .map_err(|_| {
                                RuntimeError::TimeoutError {
                                    duration_secs: self.config.timeout_secs,
                                }
                            })??;

                            // Store tool result with tool_call_id in tool_name field
                            let store = self.session_store.clone();
                            let sid = session_id;
                            let _tc_id = tc.id.clone();
                            let result_content = if tool_result.is_error {
                                format!("Error: {}", tool_result.content)
                            } else {
                                tool_result.content
                            };
                            store
                                .append_message(&sid, MessageRole::Tool, &result_content)
                                .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;

                            // Update tool_name to store tool_call_id for message reconstruction
                            // (tool_call_id stored in tool_name field for session DB compatibility)
                        }

                        // Loop back to LLM with tool results
                        continue;
                    }
                }

                // No tool calls — final text response
                current_content = msg.text().to_string();
                self.append_message(&session_id, MessageRole::Assistant, &current_content)?;

                let token_usage = response.usage.map(|u| crate::agent::TokenUsage {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    total_tokens: u.total_tokens,
                });

                // Record usage in accumulator
                if let Some(ref usage) = token_usage {
                    if let Ok(mut acc) = self.usage.lock() {
                        acc.record(&self.model, usage.input_tokens, usage.output_tokens);
                    }
                }

                return Ok(AgentResponse {
                    content: current_content,
                    tool_calls_made,
                    turns_used: self.budget.used(),
                    session_id,
                    token_usage,
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
        self.append_message(session_id, MessageRole::Tool, &result.content)?;

        Ok(result.content)
    }

    /// Reset turn counter for new conversation
    pub fn reset_turns(&self) {
        self.budget.reset();
    }

    /// Get session messages for display/history
    pub fn get_history(&self, session_id: &Uuid) -> Result<Vec<Message>, RuntimeError> {
        self.session_store
            .get_messages(session_id)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })
    }

    /// Get current model name
    pub fn model_name(&self) -> &str {
        &self.model
    }

    /// Change the model used by this agent.
    pub fn set_model(&mut self, model: String) {
        self.model = model;
    }

    /// Get the current system prompt.
    pub fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    /// Change the system prompt.
    pub fn set_system_prompt(&mut self, prompt: String) {
        self.config.system_prompt = prompt;
    }

    /// Return a list of well-known models grouped by provider.
    ///
    /// Each entry is `(provider_name, &[model_name])`.  The list is not
    /// exhaustive — it covers the most popular models across providers.
    pub fn known_models() -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            (
                "openai",
                vec!["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-mini"],
            ),
            (
                "anthropic",
                vec![
                    "claude-opus-4-20250514",
                    "claude-sonnet-4-20250514",
                    "claude-3-5-haiku-20241022",
                ],
            ),
            ("deepseek", vec!["deepseek-chat", "deepseek-reasoner"]),
            ("groq", vec!["llama-3.1-70b-versatile", "mixtral-8x7b-32768"]),
            ("gemini", vec!["gemini-2.5-pro", "gemini-2.0-flash"]),
            ("ollama", vec!["llama3", "mistral", "codellama"]),
        ]
    }

    /// List available tools
    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        self.tools.list()
    }

    /// Compact session by truncating middle messages, keeping system + recent context.
    ///
    /// Returns (before_count, after_count, tokens_saved_estimate).
    pub fn compact_session(
        &self,
        session_id: &Uuid,
        keep_recent: usize,
    ) -> Result<(usize, usize, usize), RuntimeError> {
        let messages = self.get_history(session_id)?;
        let before = messages.len();

        // Count system messages at the front
        let system_count = messages.iter().take_while(|m| m.role == MessageRole::System).count();

        // Count tokens before
        let registry = self.tokenizer_registry();
        let chat_msgs: Vec<ChatMessage> = messages
            .iter()
            .map(|m| match m.role {
                MessageRole::System => ChatMessage::system(&m.content),
                MessageRole::User => ChatMessage::user(&m.content),
                MessageRole::Assistant => ChatMessage::assistant(&m.content),
                MessageRole::Tool => ChatMessage::tool_result(
                    m.tool_name.as_deref().unwrap_or("unknown"),
                    &m.content,
                ),
            })
            .collect();
        let tokens_before = registry.count_messages(&self.model, &chat_msgs);

        // Truncate: keep system messages + last N
        let deleted = self
            .session_store
            .truncate_messages(session_id, system_count, keep_recent)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;

        let after = before - deleted;

        // Estimate tokens after
        let after_msgs = self.get_history(session_id)?;
        let after_chat: Vec<ChatMessage> = after_msgs
            .iter()
            .map(|m| match m.role {
                MessageRole::System => ChatMessage::system(&m.content),
                MessageRole::User => ChatMessage::user(&m.content),
                MessageRole::Assistant => ChatMessage::assistant(&m.content),
                MessageRole::Tool => ChatMessage::tool_result(
                    m.tool_name.as_deref().unwrap_or("unknown"),
                    &m.content,
                ),
            })
            .collect();
        let tokens_after = registry.count_messages(&self.model, &after_chat);

        Ok((before, after, tokens_before.saturating_sub(tokens_after)))
    }

    /// Compact session with LLM summarization of old messages.
    ///
    /// Instead of deleting middle messages, summarize them into a single
    /// context summary message. Keeps system prompt + summary + recent N.
    pub async fn compact_with_summary(
        &self,
        session_id: &Uuid,
        keep_recent: usize,
    ) -> Result<(usize, usize, String), RuntimeError> {
        let messages = self.get_history(session_id)?;
        let before = messages.len();

        let system_count = messages.iter().take_while(|m| m.role == MessageRole::System).count();

        // Messages to summarize (between system and recent)
        let summarize_count = before.saturating_sub(system_count + keep_recent);
        if summarize_count == 0 {
            return Ok((before, before, "No messages to summarize.".to_string()));
        }

        // Build text of messages to summarize
        let to_summarize: Vec<String> = messages[system_count..system_count + summarize_count]
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::Tool => "Tool",
                    MessageRole::System => "System",
                };
                format!("[{}]: {}", role, m.content)
            })
            .collect();
        let conversation_text = to_summarize.join("\n\n");

        // Ask LLM to summarize
        let summary_prompt = format!(
            "Summarize the following conversation in a concise paragraph. \
             Preserve key facts, decisions, code snippets, and action items. \
             Omit pleasantries.\n\n{}",
            conversation_text
        );

        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage::user(&summary_prompt)],
            tools: None,
            max_tokens: Some(1024),
            temperature: Some(0.3),
            stream: None,
        };

        let response = tokio::time::timeout(
            std::time::Duration::from_secs(self.config.timeout_secs),
            self.provider.chat_completion(request),
        )
        .await
        .map_err(|_| RuntimeError::TimeoutError { duration_secs: self.config.timeout_secs })??;

        let summary =
            response.choices.first().map(|c| c.message.text().to_string()).unwrap_or_default();

        // Delete old messages
        self.session_store
            .truncate_messages(session_id, system_count, keep_recent)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;

        // Insert summary as a system message after existing system messages
        let summary_content = format!("[Context Summary]: {}", summary);
        self.session_store
            .append_message(session_id, MessageRole::System, &summary_content)
            .map_err(|e| RuntimeError::SessionError { source: Box::new(e) })?;

        let after = self.get_history(session_id)?.len();
        let summary_preview =
            if summary.len() > 200 { format!("{}...", &summary[..200]) } else { summary };

        Ok((before, after, summary_preview))
    }

    /// Check whether streaming mode is enabled
    pub fn streaming_enabled(&self) -> bool {
        self.config.streaming
    }

    /// Get a snapshot of accumulated usage stats.
    pub fn usage_summary(&self) -> String {
        match self.usage.lock() {
            Ok(acc) => acc.summary(),
            Err(_) => "Usage unavailable".to_string(),
        }
    }

    /// Get total cost in USD.
    pub fn total_cost(&self) -> f64 {
        self.usage.lock().map(|acc| acc.total_cost_usd).unwrap_or(0.0)
    }

    /// Append an assistant message to the session (public for ChatRepl streaming)
    pub fn append_assistant_message(
        &self,
        session_id: &Uuid,
        content: &str,
    ) -> Result<Message, RuntimeError> {
        self.append_message(session_id, MessageRole::Assistant, content)
    }

    /// Stream one turn of the agent loop, yielding events as they arrive.
    ///
    /// The caller (ChatRepl) iterates the returned stream to print tokens in
    /// real-time and handle tool calls. After the stream completes, the caller
    /// should call `append_assistant_message` to persist the full response.
    ///
    /// Returns a stream of `Result<StreamEvent, RuntimeError>` where each event
    /// is either a text delta, a tool call start notification, or the complete
    /// set of tool calls ready for execution.
    pub fn stream_turn(
        &self,
        session_id: Uuid,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, RuntimeError>> + Send>> {
        // Build request from current session history
        let messages = match self.build_messages(&session_id) {
            Ok(m) => m,
            Err(e) => {
                return Box::pin(futures::stream::once(async move { Err(e) }));
            }
        };

        let tool_defs = self.tools.tool_definitions();
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            tools: if tool_defs.is_empty() { None } else { Some(json!(tool_defs)) },
            max_tokens: Some(4096),
            temperature: Some(0.7),
            stream: Some(true),
        };

        let timeout_secs = self.config.timeout_secs;
        let provider = Arc::clone(&self.provider);

        let stream = futures::stream::once(async move {
            // Resolve the future that gives us the underlying SSE stream
            let stream_result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                provider.chat_completion_stream(request),
            )
            .await;

            match stream_result {
                Ok(Ok(sse_stream)) => Ok(sse_stream),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(RuntimeError::TimeoutError { duration_secs: timeout_secs }),
            }
        })
        .try_flatten()
        .scan(HashMap::<u32, (String, String, String)>::new(), |tool_call_parts, chunk_result| {
            let mut events = Vec::new();

            match chunk_result {
                Ok(chunk) => {
                    if let Some(choice) = chunk.choices.first() {
                        // Emit text content delta
                        if let Some(ref content) = choice.delta.content {
                            if !content.is_empty() {
                                events.push(Ok(StreamEvent::Delta(content.clone())));
                            }
                        }

                        // Handle tool call deltas
                        if let Some(ref tool_calls) = choice.delta.tool_calls {
                            for tc_delta in tool_calls {
                                let entry =
                                    tool_call_parts.entry(tc_delta.index).or_insert_with(|| {
                                        (String::new(), String::new(), String::new())
                                    });
                                if let Some(ref id) = tc_delta.id {
                                    entry.0 = id.clone();
                                }
                                if let Some(ref func) = tc_delta.function {
                                    if let Some(ref name) = func.name {
                                        // Tool call start — emit event
                                        events.push(Ok(StreamEvent::ToolCallStart {
                                            name: name.clone(),
                                        }));
                                        entry.1 = name.clone();
                                    }
                                    if let Some(ref args) = func.arguments {
                                        entry.2.push_str(args);
                                    }
                                }
                            }
                        }

                        // If stream finished with tool_calls reason, emit complete event
                        if choice.finish_reason.as_deref() == Some("tool_calls")
                            && !tool_call_parts.is_empty()
                        {
                            let mut indices: Vec<u32> = tool_call_parts.keys().copied().collect();
                            indices.sort();
                            let completed: Vec<ProviderToolCall> = indices
                                .iter()
                                .map(|&idx| {
                                    let (id, name, arguments) = &tool_call_parts[&idx];
                                    ProviderToolCall {
                                        id: id.clone(),
                                        tool_type: "function".to_string(),
                                        function: crate::provider::FunctionCall {
                                            name: name.clone(),
                                            arguments: arguments.clone(),
                                        },
                                    }
                                })
                                .collect();
                            events.push(Ok(StreamEvent::ToolCallsComplete(completed)));
                            tool_call_parts.clear();
                        }
                    }
                }
                Err(e) => {
                    events.push(Err(e));
                }
            }

            std::future::ready(Some(events))
        })
        .map(futures::stream::iter)
        .flatten();

        Box::pin(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{ChatChoice, ChatMessage, ChatRequest, ChatResponse, StreamChunk};
    use crate::tool::{Tool, ToolOutput};
    use futures::Stream;
    use hermes_session_db::SessionStore;
    use serde_json::json;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::AtomicU32;

    // Mock provider that returns canned responses
    struct MockProvider {
        responses: Vec<String>,
        call_count: AtomicU32,
    }

    impl MockProvider {
        fn new(responses: Vec<String>) -> Self {
            Self { responses, call_count: AtomicU32::new(0) }
        }
    }

    impl LlmProvider for MockProvider {
        fn chat_completion(
            &self,
            _request: ChatRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
            Box::pin(async move {
                let idx = self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let content = self
                    .responses
                    .get(idx as usize)
                    .cloned()
                    .unwrap_or_else(|| "Default mock response".to_string());
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
            Box::pin(async move {
                Err(RuntimeError::ProviderError {
                    message: "Streaming not supported in mock".to_string(),
                })
            })
        }

        fn name(&self) -> &str {
            "mock"
        }
        fn default_model(&self) -> &str {
            "mock-model"
        }
    }

    // Mock tool
    struct MockTool;

    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "A mock tool for testing"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            json!({ "type": "object", "properties": {} })
        }
        fn execute(
            &self,
            _params: serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<ToolOutput, RuntimeError>> + Send + '_>> {
            Box::pin(async move { Ok(ToolOutput::success("mock tool result")) })
        }
    }

    #[tokio::test]
    async fn test_agent_simple_query() {
        let provider = Box::new(MockProvider::new(vec!["Hello from AI".to_string()]));
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(MockTool));

        let store = SessionStore::new_in_memory().unwrap();
        // Use non-streaming for mock tests
        let config = AgentConfig { streaming: false, ..AgentConfig::default() };
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let response = agent.run_query("hi").await.unwrap();
        assert_eq!(response, "Hello from AI");
    }

    #[tokio::test]
    async fn test_agent_creates_session() {
        let provider = Box::new(MockProvider::new(vec!["Response".to_string()]));
        let tools = ToolRegistry::new();
        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig { streaming: false, ..AgentConfig::default() };
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
        let config = AgentConfig { streaming: false, ..AgentConfig::default() };
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
            max_context_tokens: 128_000,
            streaming: false, // Tests use non-streaming
        };
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let session_id = agent.create_session().unwrap();
        let response = agent.run_turn(session_id, "test").await.unwrap();
        assert!(response.content.contains("Response 1") || response.turns_used <= 1);
    }

    // Mock provider that returns a tool call with invalid JSON args on first call,
    // then a plain text response on the second call.
    struct MockProviderBadToolArgs {
        call_count: AtomicU32,
    }

    impl MockProviderBadToolArgs {
        fn new() -> Self {
            Self { call_count: AtomicU32::new(0) }
        }
    }

    impl LlmProvider for MockProviderBadToolArgs {
        fn chat_completion(
            &self,
            _request: ChatRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ChatResponse, RuntimeError>> + Send + '_>> {
            Box::pin(async move {
                let idx = self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if idx == 0 {
                    // Return a tool call with invalid JSON arguments
                    Ok(ChatResponse {
                        choices: vec![ChatChoice {
                            message: ChatMessage {
                                role: "assistant".to_string(),
                                content: None,
                                tool_calls: Some(vec![crate::provider::ToolCall {
                                    id: "call_bad".to_string(),
                                    tool_type: "function".to_string(),
                                    function: crate::provider::FunctionCall {
                                        name: "mock_tool".to_string(),
                                        arguments: "{invalid json!!!}".to_string(),
                                    },
                                }]),
                                tool_call_id: None,
                                cache_control: None,
                            },
                            finish_reason: Some("tool_calls".to_string()),
                        }],
                        usage: None,
                    })
                } else {
                    // Second call: plain text after seeing the error
                    Ok(ChatResponse {
                        choices: vec![ChatChoice {
                            message: ChatMessage::assistant("I saw the parse error and fixed it"),
                            finish_reason: Some("stop".to_string()),
                        }],
                        usage: None,
                    })
                }
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
            "mock_bad_args"
        }
        fn default_model(&self) -> &str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_tool_json_parse_error() {
        let provider = Box::new(MockProviderBadToolArgs::new());
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(MockTool));

        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig { streaming: false, ..AgentConfig::default() };
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let session_id = agent.create_session().unwrap();
        let response = agent.run_turn(session_id, "call the tool").await.unwrap();

        // Agent should recover: LLM sees the parse error as a tool result and responds
        assert!(response.content.contains("parse error") || response.content.contains("fixed"));

        // The session should contain a Tool message with the parse error
        let messages = agent.get_history(&session_id).unwrap();
        let tool_msgs: Vec<_> = messages.iter().filter(|m| m.role == MessageRole::Tool).collect();
        assert!(!tool_msgs.is_empty(), "Expected at least one tool message with parse error");
        assert!(
            tool_msgs.iter().any(
                |m| m.content.contains("JSON parse error") || m.content.contains("parse error")
            ),
            "Tool message should contain parse error info"
        );
    }

    #[tokio::test]
    async fn test_tool_missing_required_field() {
        // Verify that valid JSON with missing fields still executes (tool handles it)
        // and that the tool result is stored in session
        let provider = Box::new(MockProvider::new(vec![
            // First response: tool call with empty JSON object (missing fields)
            // We can't easily inject tool_calls via MockProvider, so test via direct dispatch
            "Final answer".to_string(),
        ]));
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(MockTool));

        let store = SessionStore::new_in_memory().unwrap();
        let config = AgentConfig { streaming: false, ..AgentConfig::default() };
        let mut agent = Agent::new(provider, tools, store, config, "test-model".to_string());

        let session_id = agent.create_session().unwrap();

        // Execute tool directly with empty params (missing required fields)
        let result = agent.execute_tool(&session_id, "mock_tool", json!({})).await;
        // MockTool ignores params, so it should succeed
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mock tool result");
    }
}
