//! Hermes Common Utilities
//!
//! Shared types, errors, and utilities for Hermes crates.

pub mod error;
pub mod types;

pub use error::{HermesError, Result};
pub use types::*;
