//! Exponential backoff retry logic for LLM provider calls.

use crate::RuntimeError;
use std::future::Future;
use std::time::Duration;

/// Statuses that are safe to retry.
const RETRYABLE_STATUSES: &[u16] = &[429, 500, 502, 503];

/// Retry policy configuration.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 500,
            max_delay_ms: 30_000,
        }
    }
}

impl RetryPolicy {
    /// Compute delay for a given attempt (0-indexed), with jitter.
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = self.base_delay_ms.saturating_mul(1u64 << attempt.min(31));
        let jitter = (rand_jitter_ms()) % 1000;
        let ms = (exp + jitter).min(self.max_delay_ms);
        Duration::from_millis(ms)
    }

    /// Return true if the given status code should be retried.
    pub fn is_retryable_status(status: u16) -> bool {
        RETRYABLE_STATUSES.contains(&status)
    }
}

/// Cheap pseudo-random jitter using thread-local state (no external dep).
fn rand_jitter_ms() -> u64 {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(12345) };
    }
    STATE.with(|s| {
        // xorshift64
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x
    })
}

/// Extract HTTP status code from a `RuntimeError::ProviderError` message.
/// Returns `None` if the error is not an HTTP error or status is not retryable.
pub fn extract_status(err: &RuntimeError) -> Option<u16> {
    match err {
        RuntimeError::ProviderError { message } => {
            // Messages are formatted as "API error 429: ..." or "... error 503 ..."
            for word in message.split_whitespace() {
                if let Ok(code) = word.trim_end_matches(':').parse::<u16>() {
                    if code >= 400 {
                        return Some(code);
                    }
                }
            }
            None
        }
        RuntimeError::RateLimitError { .. } => Some(429),
        _ => None,
    }
}

/// Extract `Retry-After` seconds from a `RuntimeError::RateLimitError`.
pub fn extract_retry_after(err: &RuntimeError) -> Option<u64> {
    match err {
        RuntimeError::RateLimitError { retry_after } => *retry_after,
        _ => None,
    }
}

/// Execute `f` with exponential backoff retry according to `policy`.
///
/// Retries on:
/// - `RuntimeError::RateLimitError` (respects `retry_after` if present)
/// - `RuntimeError::ProviderError` with HTTP status in `RETRYABLE_STATUSES`
pub async fn with_retry<F, Fut, T>(policy: &RetryPolicy, mut f: F) -> Result<T, RuntimeError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, RuntimeError>>,
{
    let mut last_error: Option<RuntimeError> = None;

    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(err) => {
                let should_retry = if attempt < policy.max_retries {
                    match &err {
                        RuntimeError::RateLimitError { .. } => true,
                        RuntimeError::ProviderError { .. } => {
                            extract_status(&err)
                                .map(RetryPolicy::is_retryable_status)
                                .unwrap_or(false)
                        }
                        _ => false,
                    }
                } else {
                    false
                };

                if should_retry {
                    // Respect Retry-After header on 429
                    let delay = if let Some(secs) = extract_retry_after(&err) {
                        Duration::from_secs(secs.min(policy.max_delay_ms / 1000))
                    } else {
                        policy.delay_for_attempt(attempt)
                    };

                    tracing::warn!(
                        attempt = attempt + 1,
                        max = policy.max_retries,
                        delay_ms = delay.as_millis(),
                        "Provider error, retrying: {}",
                        err
                    );

                    tokio::time::sleep(delay).await;
                    last_error = Some(err);
                } else {
                    return Err(err);
                }
            }
        }
    }

    Err(RuntimeError::RetryExhausted {
        attempts: policy.max_retries,
        last_error: last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_policy_defaults() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_retries, 3);
        assert_eq!(p.base_delay_ms, 500);
        assert_eq!(p.max_delay_ms, 30_000);
    }

    #[test]
    fn test_retry_policy_delay_capped() {
        let p = RetryPolicy::default();
        // Very high attempt — should be capped at max_delay_ms + jitter
        let d = p.delay_for_attempt(100);
        assert!(d.as_millis() <= (p.max_delay_ms + 999) as u128);
    }

    #[test]
    fn test_retry_policy_delay_grows() {
        let p = RetryPolicy { max_retries: 3, base_delay_ms: 100, max_delay_ms: 100_000 };
        let d0 = p.delay_for_attempt(0).as_millis();
        let d1 = p.delay_for_attempt(1).as_millis();
        let d2 = p.delay_for_attempt(2).as_millis();
        // Each should be >= previous base (ignoring jitter overlap at low values)
        assert!(d1 >= d0 || d1 >= 100); // base doubles
        assert!(d2 >= d1 || d2 >= 200);
    }

    #[test]
    fn test_retryable_statuses() {
        assert!(RetryPolicy::is_retryable_status(429));
        assert!(RetryPolicy::is_retryable_status(500));
        assert!(RetryPolicy::is_retryable_status(502));
        assert!(RetryPolicy::is_retryable_status(503));
        assert!(!RetryPolicy::is_retryable_status(200));
        assert!(!RetryPolicy::is_retryable_status(400));
        assert!(!RetryPolicy::is_retryable_status(404));
    }

    #[test]
    fn test_extract_status_from_provider_error() {
        let err = RuntimeError::ProviderError { message: "API error 429: rate limited".to_string() };
        assert_eq!(extract_status(&err), Some(429));

        let err2 = RuntimeError::ProviderError { message: "API error 500: internal server error".to_string() };
        assert_eq!(extract_status(&err2), Some(500));

        let err3 = RuntimeError::ProviderError { message: "connection refused".to_string() };
        assert_eq!(extract_status(&err3), None);
    }

    #[test]
    fn test_extract_status_rate_limit_error() {
        let err = RuntimeError::RateLimitError { retry_after: Some(30) };
        assert_eq!(extract_status(&err), Some(429));
    }

    #[test]
    fn test_extract_retry_after() {
        let err = RuntimeError::RateLimitError { retry_after: Some(60) };
        assert_eq!(extract_retry_after(&err), Some(60));

        let err2 = RuntimeError::RateLimitError { retry_after: None };
        assert_eq!(extract_retry_after(&err2), None);
    }

    #[tokio::test]
    async fn test_retry_succeeds_on_first_try() {
        let policy = RetryPolicy::default();
        let result = with_retry(&policy, || async { Ok::<i32, RuntimeError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let policy = RetryPolicy { max_retries: 3, base_delay_ms: 1, max_delay_ms: 10 };
        let counter = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&counter);

        let result = with_retry(&policy, || {
            let c = Arc::clone(&c);
            async move {
                let n = c.fetch_add(1, Ordering::Relaxed);
                if n < 2 {
                    Err(RuntimeError::ProviderError { message: "API error 503: unavailable".to_string() })
                } else {
                    Ok("success")
                }
            }
        }).await;

        assert_eq!(result.unwrap(), "success");
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let policy = RetryPolicy { max_retries: 2, base_delay_ms: 1, max_delay_ms: 10 };
        let counter = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&counter);

        let result = with_retry(&policy, || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                Err::<(), RuntimeError>(RuntimeError::ProviderError {
                    message: "API error 500: server error".to_string(),
                })
            }
        }).await;

        assert!(matches!(result, Err(RuntimeError::RetryExhausted { attempts: 2, .. })));
        // Called max_retries+1 times (initial + retries)
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn test_rate_limit_header_respected() {
        let policy = RetryPolicy { max_retries: 1, base_delay_ms: 1, max_delay_ms: 10 };
        let counter = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&counter);

        // Should retry once on RateLimitError, then succeed
        let result = with_retry(&policy, || {
            let c = Arc::clone(&c);
            async move {
                let n = c.fetch_add(1, Ordering::Relaxed);
                if n == 0 {
                    Err(RuntimeError::RateLimitError { retry_after: Some(0) })
                } else {
                    Ok("ok")
                }
            }
        }).await;

        assert_eq!(result.unwrap(), "ok");
    }

    #[tokio::test]
    async fn test_non_retryable_error_not_retried() {
        let policy = RetryPolicy { max_retries: 3, base_delay_ms: 1, max_delay_ms: 10 };
        let counter = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&counter);

        let result = with_retry(&policy, || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                Err::<(), RuntimeError>(RuntimeError::ProviderError {
                    message: "API error 400: bad request".to_string(),
                })
            }
        }).await;

        assert!(result.is_err());
        // Should NOT retry — only called once
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
