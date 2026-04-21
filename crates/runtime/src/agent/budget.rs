//! Thread-safe iteration budget with consume/refund semantics.

use std::sync::Mutex;

/// Tracks how many iterations have been used against a fixed budget.
///
/// Replaces raw `AtomicU32` turn counters with explicit budget semantics:
/// callers `consume()` before work and `refund()` on rollback.
pub struct IterationBudget {
    max_total: u32,
    used: Mutex<u32>,
}

impl IterationBudget {
    /// Create a new budget allowing up to `max_total` iterations.
    pub fn new(max_total: u32) -> Self {
        Self {
            max_total,
            used: Mutex::new(0),
        }
    }

    /// Attempt to consume one iteration slot.
    ///
    /// Returns `true` if the slot was reserved, `false` if budget is exhausted.
    pub fn consume(&self) -> bool {
        let mut used = self.used.lock().expect("budget lock poisoned");
        if *used < self.max_total {
            *used += 1;
            true
        } else {
            false
        }
    }

    /// Return one iteration slot to the budget (e.g. on rollback).
    ///
    /// No-op when already at zero.
    pub fn refund(&self) {
        let mut used = self.used.lock().expect("budget lock poisoned");
        if *used > 0 {
            *used -= 1;
        }
    }

    /// Number of iterations still available.
    pub fn remaining(&self) -> u32 {
        let used = self.used.lock().expect("budget lock poisoned");
        self.max_total.saturating_sub(*used)
    }

    /// Number of iterations consumed so far.
    pub fn used(&self) -> u32 {
        *self.used.lock().expect("budget lock poisoned")
    }

    /// Reset the budget to zero used iterations.
    pub fn reset(&self) {
        let mut used = self.used.lock().expect("budget lock poisoned");
        *used = 0;
    }

    /// The maximum allowed iterations.
    pub fn max_total(&self) -> u32 {
        self.max_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consume_success() {
        let budget = IterationBudget::new(5);
        assert!(budget.consume());
        assert_eq!(budget.used(), 1);
        assert!(budget.consume());
        assert_eq!(budget.used(), 2);
    }

    #[test]
    fn consume_max_reached() {
        let budget = IterationBudget::new(2);
        assert!(budget.consume());
        assert!(budget.consume());
        assert!(!budget.consume()); // third one should fail
        assert_eq!(budget.used(), 2);
    }

    #[test]
    fn refund() {
        let budget = IterationBudget::new(5);
        budget.consume();
        budget.consume();
        assert_eq!(budget.used(), 2);
        budget.refund();
        assert_eq!(budget.used(), 1);
        // refund below zero is a no-op
        budget.refund();
        assert_eq!(budget.used(), 0);
        budget.refund();
        assert_eq!(budget.used(), 0);
    }

    #[test]
    fn remaining() {
        let budget = IterationBudget::new(10);
        assert_eq!(budget.remaining(), 10);
        budget.consume();
        assert_eq!(budget.remaining(), 9);
        budget.consume();
        budget.consume();
        assert_eq!(budget.remaining(), 7);
    }
}
