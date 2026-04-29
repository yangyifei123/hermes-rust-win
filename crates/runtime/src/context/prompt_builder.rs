//! System prompt builder for constructing LLM system messages.

/// Tool metadata: name and short description.
struct ToolInfo {
    name: &'static str,
    description: &'static str,
}

/// Well-known built-in tool descriptions used when assembling the prompt.
const BUILTIN_TOOLS: &[ToolInfo] = &[
    ToolInfo { name: "terminal", description: "Execute shell commands" },
    ToolInfo { name: "file_read", description: "Read file contents" },
    ToolInfo { name: "file_write", description: "Write or create files" },
    ToolInfo { name: "file_search", description: "Search files by pattern" },
    ToolInfo { name: "web_search", description: "Search the web via DuckDuckGo" },
    ToolInfo { name: "mcp", description: "MCP protocol tools" },
    ToolInfo { name: "browser", description: "Browser automation" },
];

/// Builder that constructs the system prompt sent to the LLM.
pub struct SystemPromptBuilder {
    identity: Option<String>,
    capabilities: Vec<String>,
    date: bool,
    os_info: bool,
    cwd: bool,
    custom: Option<String>,
}

impl SystemPromptBuilder {
    /// Create a new, empty builder.
    pub fn new() -> Self {
        Self {
            identity: None,
            capabilities: Vec::new(),
            date: false,
            os_info: false,
            cwd: false,
            custom: None,
        }
    }

    /// Set the agent identity (name and version).
    pub fn with_identity(&mut self, name: &str, version: &str) -> &mut Self {
        self.identity = Some(format!("You are {} v{}, an AI agent CLI tool.", name, version));
        self
    }

    /// Append tool names to the capabilities list.
    ///
    /// Unrecognised tool names are included without a description.
    pub fn with_capabilities(&mut self, tools: &[&str]) -> &mut Self {
        for &tool in tools {
            let entry = BUILTIN_TOOLS
                .iter()
                .find(|t| t.name == tool)
                .map(|t| format!("- {}: {}", t.name, t.description))
                .unwrap_or_else(|| format!("- {}", tool));
            self.capabilities.push(entry);
        }
        self
    }

    /// Include the current date in the prompt.
    pub fn with_date(&mut self) -> &mut Self {
        self.date = true;
        self
    }

    /// Include OS information in the prompt.
    pub fn with_os_info(&mut self) -> &mut Self {
        self.os_info = true;
        self
    }

    /// Include the current working directory in the prompt.
    pub fn with_cwd(&mut self) -> &mut Self {
        self.cwd = true;
        self
    }

    /// Append a custom text block at the end of the prompt.
    pub fn with_custom(&mut self, text: &str) -> &mut Self {
        self.custom = Some(text.to_owned());
        self
    }

    /// Assemble and return the final system prompt string.
    pub fn build(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Identity line.
        if let Some(ref id) = self.identity {
            parts.push(id.clone());
            parts.push(String::new()); // blank line
        }

        // Date.
        if self.date {
            // Fixed date keeps builds reproducible; callers can override via
            // with_custom if they need real-time dates.
            parts.push(format!("Current date: {}", chrono::Local::now().format("%Y-%m-%d")));
        }

        // OS info.
        if self.os_info {
            parts.push(format!("OS: {}", std::env::consts::OS));
        }

        // Working directory.
        if self.cwd {
            if let Ok(dir) = std::env::current_dir() {
                parts.push(format!("Working directory: {}", dir.display()));
            }
        }

        // Capabilities / tools.
        if !self.capabilities.is_empty() {
            parts.push(String::new()); // blank separator
            parts.push("Available tools:".to_owned());
            parts.extend(self.capabilities.iter().cloned());
        }

        // Custom tail section.
        if let Some(ref custom) = self.custom {
            parts.push(String::new()); // blank separator
            parts.push(custom.clone());
        }

        parts.join("\n")
    }
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}
