use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde_json::Value;

// ANSI escape codes for terminal formatting
mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BLUE: &str = "\x1b[34m";
    pub const UNDERLINE: &str = "\x1b[4m";
    pub const BRIGHT_WHITE: &str = "\x1b[97m";
}

/// Streaming-friendly markdown renderer for terminal output.
///
/// Applies ANSI formatting to markdown text as it streams in:
/// - `# Headers` → bold + bright white
/// - `**bold**` → bold
/// - `` `code` `` → cyan
/// - ```code blocks``` → dim
/// - `[links](url)` → blue + underline
/// - `- lists` → bullet points with indent
pub struct MarkdownRenderer {
    in_code_block: bool,
    code_fence: String,
    supports_color: bool,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            in_code_block: false,
            code_fence: String::new(),
            supports_color: true,
        }
    }

    pub fn with_color(mut self, supports: bool) -> Self {
        self.supports_color = supports;
        self
    }

    /// Render a chunk of markdown text for terminal display.
    ///
    /// Designed for streaming: maintains state across calls (e.g., inside code blocks).
    /// Returns the formatted string ready to print.
    pub fn render(&mut self, text: &str) -> String {
        let mut output = String::with_capacity(text.len() + 64);
        let mut remaining = text;

        while !remaining.is_empty() {
            if self.in_code_block {
                // Look for closing fence
                if let Some(pos) = remaining.find(&self.code_fence) {
                    output.push_str(&remaining[..pos]);
                    if self.supports_color {
                        output.push_str(ansi::RESET);
                    }
                    output.push_str(&self.code_fence);
                    remaining = &remaining[pos + self.code_fence.len()..];
                    self.in_code_block = false;
                    self.code_fence.clear();
                } else {
                    output.push_str(remaining);
                    break;
                }
            } else if let Some(pos) = remaining.find("```") {
                // Opening code fence
                let after = &remaining[pos + 3..];
                let newline = after.find('\n').unwrap_or(after.len());
                self.code_fence = remaining[pos..pos + 3 + newline].to_string();

                // Render pre-fence text with inline formatting
                output.push_str(&self.format_inline(&remaining[..pos]));

                if self.supports_color {
                    output.push_str(ansi::DIM);
                }
                output.push_str(&self.code_fence);
                remaining = &remaining[pos + 3 + newline..];
                self.in_code_block = true;
            } else {
                output.push_str(&self.format_inline(remaining));
                break;
            }
        }

        output
    }

    /// Format inline markdown elements (bold, code, headers, links).
    fn format_inline(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + 32);
        let mut chars = text.char_indices().peekable();
        let bytes = text.as_bytes();

        while let Some((i, ch)) = chars.next() {
            match ch {
                '#' if i == 0 || text[..i].ends_with('\n') => {
                    // Count hash level
                    let mut level = 1;
                    while chars.peek().map(|(_, c)| *c) == Some('#') {
                        chars.next();
                        level += 1;
                    }
                    // Skip space after hashes
                    if chars.peek().map(|(_, c)| *c) == Some(' ') {
                        chars.next();
                    }
                    if self.supports_color {
                        out.push_str(ansi::BOLD);
                        out.push_str(ansi::BRIGHT_WHITE);
                    }
                    // Collect rest of line
                    let start = chars.peek().map(|(i, _)| *i).unwrap_or(i + level);
                    let end = text[start..].find('\n').map(|p| start + p).unwrap_or(text.len());
                    out.push_str(&text[start..end]);
                    if self.supports_color {
                        out.push_str(ansi::RESET);
                    }
                    out.push('\n');
                    // Skip past what we consumed
                    for _ in start..end {
                        chars.next();
                    }
                }
                '*' if bytes.get(i + 1) == Some(&b'*') => {
                    // Bold **text**
                    chars.next(); // skip second *
                    if let Some(end) = text[i + 2..].find("**") {
                        if self.supports_color {
                            out.push_str(ansi::BOLD);
                        }
                        out.push_str(&text[i + 2..i + 2 + end]);
                        if self.supports_color {
                            out.push_str(ansi::RESET);
                        }
                        // Skip past closing **
                        for _ in 0..end + 2 {
                            chars.next();
                        }
                    } else {
                        out.push_str("**");
                    }
                }
                '`' => {
                    // Inline code `text`
                    if let Some(end) = text[i + 1..].find('`') {
                        if self.supports_color {
                            out.push_str(ansi::CYAN);
                        }
                        out.push_str(&text[i + 1..i + 1 + end]);
                        if self.supports_color {
                            out.push_str(ansi::RESET);
                        }
                        for _ in 0..end + 1 {
                            chars.next();
                        }
                    } else {
                        out.push('`');
                    }
                }
                '[' => {
                    // Try to match [text](url)
                    if let Some(bracket_end) = text[i + 1..].find(']') {
                        let after_bracket = i + 1 + bracket_end + 1;
                        if bytes.get(after_bracket) == Some(&b'(') {
                            if let Some(paren_end) = text[after_bracket + 1..].find(')') {
                                let link_text = &text[i + 1..i + 1 + bracket_end];
                                if self.supports_color {
                                    out.push_str(ansi::BLUE);
                                    out.push_str(ansi::UNDERLINE);
                                }
                                out.push_str(link_text);
                                if self.supports_color {
                                    out.push_str(ansi::RESET);
                                }
                                // Skip entire [text](url)
                                let total_len = 1 + bracket_end + 1 + 1 + paren_end + 1;
                                for _ in 1..total_len {
                                    chars.next();
                                }
                                continue;
                            }
                        }
                    }
                    out.push('[');
                }
                '-' if (i == 0 || text[..i].ends_with('\n'))
                    && bytes.get(i + 1) == Some(&b' ') =>
                {
                    out.push_str("  • ");
                    chars.next(); // skip space
                }
                _ => {
                    out.push(ch);
                }
            }
        }

        out
    }

    /// Check if we're currently inside a code block (for streaming state).
    pub fn is_in_code_block(&self) -> bool {
        self.in_code_block
    }
}

/// Commands sent from the public API to the spinner thread.
enum SpinnerCmd {
    /// Start (or restart) spinning with the given label.
    Start(String),
    /// Stop spinning and clear the line.
    Stop,
    /// Shut down the thread (used on Drop).
    Quit,
}

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

/// A simple ASCII spinner that runs on a background `std::thread`.
///
/// Uses only `\r` (carriage return) for in-place updates — no ANSI escape
/// sequences — so it works reliably on stock Windows terminals.
pub struct Spinner {
    tx: Sender<SpinnerCmd>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// Create a **stopped** spinner. Call [`start`](Self::start) to begin.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<SpinnerCmd>();
        let handle = thread::Builder::new()
            .name("hermes-spinner".into())
            .spawn(move || spinner_loop(rx))
            .expect("failed to spawn spinner thread");

        Self {
            tx,
            handle: Some(handle),
        }
    }

    /// Start spinning with `message` as the label.
    pub fn start(&self, message: &str) {
        let _ = self.tx.send(SpinnerCmd::Start(message.to_owned()));
    }

    /// Stop the spinner and clear the line.
    pub fn stop(&self) {
        let _ = self.tx.send(SpinnerCmd::Stop);
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        let _ = self.tx.send(SpinnerCmd::Quit);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// Background loop — blocks on the receiver and draws frames.
fn spinner_loop(rx: Receiver<SpinnerCmd>) {
    const FRAMES: &[u8] = b"\\|/-";
    let mut spinning = false;
    let mut idx: usize = 0;
    let mut label = String::new();

    loop {
        // Check for commands with a small timeout so we can animate.
        match rx.recv_timeout(Duration::from_millis(80)) {
            Ok(SpinnerCmd::Start(msg)) => {
                label = msg;
                spinning = true;
                idx = 0;
            }
            Ok(SpinnerCmd::Stop) => {
                if spinning {
                    // Clear the line: overwrite with spaces then \r.
                    let _ = write!(io::stderr(), "\r{}", " ".repeat(60));
                    let _ = io::stderr().flush();
                }
                spinning = false;
            }
            Ok(SpinnerCmd::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Clean up before exiting.
                if spinning {
                    let _ = write!(io::stderr(), "\r{}", " ".repeat(60));
                    let _ = io::stderr().flush();
                }
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Time to draw the next frame.
            }
        }

        if spinning {
            let ch = FRAMES[idx % FRAMES.len()] as char;
            let _ = write!(io::stderr(), "\r{} {}", ch, label);
            let _ = io::stderr().flush();
            idx = idx.wrapping_add(1);
        }
    }
}

// ---------------------------------------------------------------------------
// DisplayEngine
// ---------------------------------------------------------------------------

/// Terminal display engine for the CLI agent.
///
/// All output goes to **stderr** so it never interferes with piping stdout.
pub struct DisplayEngine {
    quiet: bool,
    verbose: bool,
    spinner: Spinner,
}

impl DisplayEngine {
    pub fn new(quiet: bool, verbose: bool) -> Self {
        Self {
            quiet,
            verbose,
            spinner: Spinner::new(),
        }
    }

    /// Convenience constructor for the default (non-quiet, non-verbose) engine.
    pub fn default_engine() -> Self {
        Self::new(false, false)
    }

    // ----- public API -------------------------------------------------------

    /// Called when a tool begins execution.
    pub fn print_tool_start(&self, name: &str, args: &Value) {
        if self.quiet {
            return;
        }
        let preview = Self::arg_preview(args);
        let msg = if preview.is_empty() {
            format!("  Running `{name}`...")
        } else {
            format!("  Running `{name}` — {preview}")
        };
        let _ = writeln!(io::stderr(), "\r{}", truncate_str(&msg, 120));
        let _ = io::stderr().flush();
    }

    /// Called when a tool finishes execution.
    pub fn print_tool_result(&self, name: &str, success: bool, duration_ms: u64) {
        if self.quiet {
            return;
        }
        let secs = duration_ms as f64 / 1000.0;
        if success {
            let _ = writeln!(io::stderr(), "  `{name}` done ({:.1}s)", secs);
        } else {
            let _ = writeln!(io::stderr(), "  `{name}` failed");
        }
        let _ = io::stderr().flush();
    }

    /// Progress message for long-running tools.
    pub fn print_tool_progress(&self, name: &str, msg: &str) {
        if self.quiet {
            return;
        }
        let _ = writeln!(io::stderr(), "  [{name}] {msg}");
        let _ = io::stderr().flush();
    }

    /// Show token usage and (optionally) estimated cost.
    pub fn print_token_usage(&self, input: u32, output: u32, cost: Option<f64>) {
        if self.quiet {
            return;
        }
        match cost {
            Some(c) => {
                let _ = writeln!(
                    io::stderr(),
                    "  Tokens: {input} in / {output} out  (~${:.4})",
                    c
                );
            }
            None => {
                let _ = writeln!(io::stderr(), "  Tokens: {input} in / {output} out");
            }
        }
        let _ = io::stderr().flush();
    }

    /// Start the background spinner with `message`.
    pub fn start_spinner(&self, message: &str) {
        if !self.quiet {
            self.spinner.start(message);
        }
    }

    /// Stop the background spinner.
    pub fn stop_spinner(&self) {
        self.spinner.stop();
    }

    /// Returns `true` when verbose mode is enabled.
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Returns `true` when quiet mode is enabled.
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    // ----- helpers ----------------------------------------------------------

    /// Build a short, single-line preview of the tool arguments.
    fn arg_preview(args: &Value) -> String {
        if args.is_null() {
            return String::new();
        }
        let s = if args.is_object() {
            // Show each key=value pair, compact.
            let pairs: Vec<String> = args
                .as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| format!("{k}={}", Self::short_val(v)))
                        .collect()
                })
                .unwrap_or_default();
            pairs.join(", ")
        } else {
            // Fallback: compact JSON.
            args.to_string()
        };
        truncate_str(&s, 80).into_owned()
    }

    /// Truncate a JSON value to something short for preview purposes.
    fn short_val(v: &Value) -> String {
        match v {
            Value::String(s) => truncate_str(s, 30).into_owned(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_owned(),
            _ => truncate_str(&v.to_string(), 30).into_owned(),
        }
    }
}

/// Truncate `s` to at most `max_len` characters, appending `…` if truncated.
fn truncate_str(s: &str, max_len: usize) -> std::borrow::Cow<'_, str> {
    if s.len() <= max_len {
        std::borrow::Cow::Borrowed(s)
    } else {
        // Find a safe char boundary.
        let mut end = max_len.saturating_sub(1);
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        std::borrow::Cow::Owned(format!("{}…", &s[..end]))
    }
}
