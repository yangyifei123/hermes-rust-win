//! Hermes Common Utilities
//!
//! Shared types, errors, and utilities for Hermes crates.

pub mod error;
pub mod string;
pub mod types;

pub use error::{HermesError, Result};
pub use string::sanitize_surrogates;
pub use types::*;
