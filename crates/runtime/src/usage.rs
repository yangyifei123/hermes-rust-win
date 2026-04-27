//! Token usage and cost tracking for agent sessions.

use hermes_common::model_metadata;
use std::fmt;

/// Accumulated token usage for a session.
#[derive(Debug, Clone, Default)]
pub struct UsageAccumulator {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub turn_count: u32,
}

impl UsageAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record token usage from a single LLM turn.
    pub fn record(&mut self, model: &str, input: u32, output: u32) {
        self.input_tokens += input as u64;
        self.output_tokens += output as u64;
        self.total_tokens += (input + output) as u64;
        self.turn_count += 1;

        if let Some(cost) = model_metadata::estimate_cost(model, input, output) {
            self.total_cost_usd += cost;
        }
    }

    /// Get a formatted summary string.
    pub fn summary(&self) -> String {
        format!(
            "Tokens: {} in / {} out ({} total) across {} turns. Cost: ${:.4}",
            self.input_tokens,
            self.output_tokens,
            self.total_tokens,
            self.turn_count,
            self.total_cost_usd
        )
    }
}

impl fmt::Display for UsageAccumulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulator_single_turn() {
        let mut acc = UsageAccumulator::new();
        acc.record("gpt-4o", 100, 50);
        assert_eq!(acc.input_tokens, 100);
        assert_eq!(acc.output_tokens, 50);
        assert_eq!(acc.total_tokens, 150);
        assert_eq!(acc.turn_count, 1);
        // gpt-4o: $2.50/M input, $10.00/M output
        let expected = (100.0 / 1_000_000.0) * 2.5 + (50.0 / 1_000_000.0) * 10.0;
        assert!((acc.total_cost_usd - expected).abs() < 1e-10);
    }

    #[test]
    fn test_accumulator_multiple_turns() {
        let mut acc = UsageAccumulator::new();
        acc.record("gpt-4o", 100, 50);
        acc.record("gpt-4o", 200, 100);
        assert_eq!(acc.input_tokens, 300);
        assert_eq!(acc.output_tokens, 150);
        assert_eq!(acc.turn_count, 2);
    }

    #[test]
    fn test_accumulator_unknown_model() {
        let mut acc = UsageAccumulator::new();
        acc.record("nonexistent-model", 100, 50);
        assert_eq!(acc.input_tokens, 100);
        assert_eq!(acc.total_cost_usd, 0.0); // no cost for unknown model
    }

    #[test]
    fn test_accumulator_summary_format() {
        let mut acc = UsageAccumulator::new();
        acc.record("gpt-4o", 1000, 500);
        let summary = acc.summary();
        assert!(summary.contains("1000 in"));
        assert!(summary.contains("500 out"));
        assert!(summary.contains("1 turns"));
        assert!(summary.contains("$"));
    }
}
