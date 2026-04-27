use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serde_json::Value;

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
