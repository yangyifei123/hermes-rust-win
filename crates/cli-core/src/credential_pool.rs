//! Credential pool with automatic failover on rate-limit errors.
//!
//! Supports multiple API keys per provider. Rotates keys on 429/rate-limit
//! responses and tracks which keys are temporarily disabled.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A single credential entry with health tracking.
#[derive(Debug, Clone)]
pub struct CredentialEntry {
    pub api_key: String,
    pub base_url: Option<String>,
    /// When this key was rate-limited. `None` if healthy.
    pub rate_limited_at: Option<Instant>,
    /// How long to wait before retrying this key.
    pub cooldown: Duration,
    /// Number of consecutive failures.
    pub failures: u32,
}

impl CredentialEntry {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url,
            rate_limited_at: None,
            cooldown: Duration::from_secs(60),
            failures: 0,
        }
    }

    /// Whether this credential is available (not in cooldown).
    pub fn is_available(&self) -> bool {
        match self.rate_limited_at {
            None => true,
            Some(t) => t.elapsed() >= self.cooldown,
        }
    }

    /// Mark this credential as rate-limited.
    pub fn mark_rate_limited(&mut self, retry_after: Option<u64>) {
        self.rate_limited_at = Some(Instant::now());
        self.cooldown = retry_after
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));
        self.failures += 1;
    }

    /// Mark this credential as successfully used.
    pub fn mark_success(&mut self) {
        self.rate_limited_at = None;
        self.failures = 0;
    }
}

/// Pool of credentials per provider with round-robin selection and failover.
pub struct CredentialPool {
    /// provider_name → list of credentials
    pool: Mutex<HashMap<String, Vec<CredentialEntry>>>,
    /// provider_name → current index for round-robin
    index: Mutex<HashMap<String, usize>>,
}

impl Default for CredentialPool {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialPool {
    pub fn new() -> Self {
        Self {
            pool: Mutex::new(HashMap::new()),
            index: Mutex::new(HashMap::new()),
        }
    }

    /// Create a pool pre-loaded from an AuthStore's credentials.
    pub fn from_auth_store(auth: &crate::auth::AuthStore) -> Self {
        let pool = Self::new();
        for cred in &auth.credentials {
            pool.add(&cred.provider, cred.api_key.clone(), cred.base_url.clone());
        }
        pool
    }

    /// Add a credential for a provider.
    pub fn add(&self, provider: &str, api_key: String, base_url: Option<String>) {
        let mut pool = self.pool.lock().unwrap();
        let entries = pool.entry(provider.to_string()).or_default();

        // Don't add duplicates
        if entries.iter().any(|e| e.api_key == api_key) {
            return;
        }

        entries.push(CredentialEntry::new(api_key, base_url));
    }

    /// Remove all credentials for a provider.
    pub fn remove(&self, provider: &str) {
        let mut pool = self.pool.lock().unwrap();
        pool.remove(provider);
        self.index.lock().unwrap().remove(provider);
    }

    /// Get the next available credential for a provider using round-robin.
    ///
    /// Returns `None` if no credentials are registered or all are in cooldown.
    pub fn get(&self, provider: &str) -> Option<CredentialEntry> {
        let pool = self.pool.lock().unwrap();
        let entries = pool.get(provider)?;

        if entries.is_empty() {
            return None;
        }

        let mut index = self.index.lock().unwrap();
        let start = *index.entry(provider.to_string()).or_insert(0);
        let len = entries.len();

        // Try each credential starting from current index
        for i in 0..len {
            let idx = (start + i) % len;
            if entries[idx].is_available() {
                *index.get_mut(provider).unwrap() = (idx + 1) % len;
                return Some(entries[idx].clone());
            }
        }

        // All in cooldown — return the one closest to expiry as last resort
        let best = entries
            .iter()
            .min_by_key(|e| e.rate_limited_at.map(|t| t.elapsed()))
            .unwrap();
        Some(best.clone())
    }

    /// Report that a credential hit a rate limit.
    pub fn report_rate_limit(&self, provider: &str, api_key: &str, retry_after: Option<u64>) {
        let mut pool = self.pool.lock().unwrap();
        if let Some(entries) = pool.get_mut(provider) {
            if let Some(entry) = entries.iter_mut().find(|e| e.api_key == api_key) {
                entry.mark_rate_limited(retry_after);
            }
        }
    }

    /// Report that a credential was used successfully.
    pub fn report_success(&self, provider: &str, api_key: &str) {
        let mut pool = self.pool.lock().unwrap();
        if let Some(entries) = pool.get_mut(provider) {
            if let Some(entry) = entries.iter_mut().find(|e| e.api_key == api_key) {
                entry.mark_success();
            }
        }
    }

    /// Get the number of credentials for a provider.
    pub fn count(&self, provider: &str) -> usize {
        self.pool
            .lock()
            .unwrap()
            .get(provider)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get the number of available (non-cooldown) credentials for a provider.
    pub fn available_count(&self, provider: &str) -> usize {
        self.pool
            .lock()
            .unwrap()
            .get(provider)
            .map(|v| v.iter().filter(|e| e.is_available()).count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_add_and_get() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.add("openai", "key2".to_string(), None);

        let cred = pool.get("openai").unwrap();
        assert_eq!(cred.api_key, "key1");
    }

    #[test]
    fn test_pool_round_robin() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.add("openai", "key2".to_string(), None);

        let first = pool.get("openai").unwrap();
        let second = pool.get("openai").unwrap();
        assert_ne!(first.api_key, second.api_key);
    }

    #[test]
    fn test_pool_no_duplicates() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.add("openai", "key1".to_string(), None);
        assert_eq!(pool.count("openai"), 1);
    }

    #[test]
    fn test_pool_rate_limit_cooldown() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.add("openai", "key2".to_string(), None);

        pool.report_rate_limit("openai", "key1", None);

        // key1 is in cooldown, should get key2
        let cred = pool.get("openai").unwrap();
        assert_eq!(cred.api_key, "key2");
    }

    #[test]
    fn test_pool_report_success() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.report_rate_limit("openai", "key1", None);
        pool.report_success("openai", "key1");

        let cred = pool.get("openai").unwrap();
        assert_eq!(cred.api_key, "key1");
        assert!(cred.is_available());
    }

    #[test]
    fn test_pool_empty_provider() {
        let pool = CredentialPool::new();
        assert!(pool.get("nonexistent").is_none());
    }

    #[test]
    fn test_pool_remove() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.remove("openai");
        assert!(pool.get("openai").is_none());
    }

    #[test]
    fn test_pool_available_count() {
        let pool = CredentialPool::new();
        pool.add("openai", "key1".to_string(), None);
        pool.add("openai", "key2".to_string(), None);
        pool.report_rate_limit("openai", "key1", None);

        assert_eq!(pool.available_count("openai"), 1);
        assert_eq!(pool.count("openai"), 2);
    }

    #[test]
    fn test_credential_entry_cooldown_expiry() {
        let mut entry = CredentialEntry::new("key".to_string(), None);
        // Manually set a past rate limit with very short cooldown
        entry.rate_limited_at = Some(Instant::now() - Duration::from_secs(120));
        entry.cooldown = Duration::from_secs(60);
        // Should be available since cooldown expired
        assert!(entry.is_available());
    }
}
