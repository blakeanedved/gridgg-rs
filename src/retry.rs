//! Retry policy for transient failures.

use std::hash::{BuildHasher, RandomState};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::{HeaderMap, RETRY_AFTER};

use crate::error::{ErrorDetail, ErrorType, GraphQlError};

/// How requests are retried after transient failures.
///
/// A request is retried when the server answers `429 Too Many Requests` or
/// any `5xx` status, when connecting fails or times out, or when a GraphQL
/// `errors` array (in a successful or `4xx` response) contains an error whose
/// [`ErrorDetail`] is `TOO_MANY_REQUESTS` or `ENHANCE_YOUR_CALM`, or whose
/// [`ErrorType`] is `UNAVAILABLE`. Other errors, including other `4xx`
/// statuses and GraphQL errors (also those typed `INTERNAL`), are returned
/// at once.
///
/// When the attempts run out, a `429` status ends in
/// [`Error::RateLimited`](crate::Error::RateLimited), and retryable GraphQL
/// errors end in [`Error::GraphQl`](crate::Error::GraphQl) so their messages
/// and typed details are kept.
///
/// The delay before retry `n` (1-based) is `base_delay * 2^(n - 1)`, capped at
/// `max_delay`, then jittered to a random value between half and all of it.
/// A `Retry-After` header (seconds or HTTP date) replaces the computed delay,
/// also capped at `max_delay`.
///
/// The default is 4 attempts, a 500 ms base delay and a 60 s maximum delay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryConfig {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
        }
    }
}

impl RetryConfig {
    /// The default policy; see [`RetryConfig`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A policy that sends every request exactly once.
    #[must_use]
    pub fn disabled() -> Self {
        Self::default().max_attempts(1)
    }

    /// Sets the total number of attempts, including the first one. Values
    /// below 1 are treated as 1.
    #[must_use]
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts.max(1);
        self
    }

    /// Sets the delay before the first retry.
    #[must_use]
    pub fn base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    /// Sets the upper bound for any single delay, including one requested by
    /// `Retry-After`.
    #[must_use]
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// The total number of attempts, including the first one.
    pub(crate) fn attempts(&self) -> u32 {
        self.max_attempts
    }

    /// The delay before retry number `retry` (1-based), given the server's
    /// `Retry-After` value if any.
    pub(crate) fn delay(&self, retry: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(after) = retry_after {
            return after.min(self.max_delay);
        }
        let factor = 2u32.saturating_pow(retry.saturating_sub(1));
        let full = self.base_delay.saturating_mul(factor).min(self.max_delay);
        jitter(full)
    }
}

/// Whether an HTTP status is worth retrying.
pub(crate) fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Whether a transport error is worth retrying.
pub(crate) fn is_retryable_error(err: &reqwest::Error) -> bool {
    err.is_connect() || err.is_timeout()
}

/// Whether a GraphQL `errors` array signals throttling or a transient outage:
/// any error with detail `TOO_MANY_REQUESTS` or `ENHANCE_YOUR_CALM`, or with
/// type `UNAVAILABLE`.
pub(crate) fn is_retryable_graphql(errors: &[GraphQlError]) -> bool {
    errors.iter().any(|error| {
        error.extensions.as_ref().is_some_and(|ext| {
            matches!(
                ext.error_detail,
                Some(ErrorDetail::TooManyRequests | ErrorDetail::EnhanceYourCalm)
            ) || ext.error_type == Some(ErrorType::Unavailable)
        })
    })
}

/// Parses a `Retry-After` header given as delay-seconds or an HTTP date.
pub(crate) fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(RETRY_AFTER)?.to_str().ok()?.trim();
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let at = jiff::fmt::rfc2822::DateTimeParser::new()
        .parse_timestamp(value)
        .ok()?;
    let wait = at.duration_since(jiff::Timestamp::now());
    Some(Duration::try_from(wait).unwrap_or(Duration::ZERO))
}

/// Returns a pseudo-random duration in `[delay / 2, delay]`.
fn jitter(delay: Duration) -> Duration {
    let half = delay / 2;
    let span = u64::try_from(delay.saturating_sub(half).as_nanos()).unwrap_or(u64::MAX);
    if span == 0 {
        return delay;
    }
    // Each `RandomState` is freshly seeded, so this is a cheap random u64.
    let random = RandomState::new().hash_one(0u8);
    half + Duration::from_nanos(random % (span + 1))
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    #[test]
    fn backoff_grows_and_is_capped() {
        let config = RetryConfig::new()
            .base_delay(Duration::from_secs(1))
            .max_delay(Duration::from_secs(5));
        for (retry, full) in [(1, 1), (2, 2), (3, 4), (4, 5), (40, 5)] {
            let full = Duration::from_secs(full);
            let delay = config.delay(retry, None);
            assert!(delay >= full / 2 && delay <= full, "{retry}: {delay:?}");
        }
    }

    #[test]
    fn retry_after_overrides_backoff_up_to_max() {
        let config = RetryConfig::new().max_delay(Duration::from_secs(10));
        assert_eq!(
            config.delay(1, Some(Duration::from_secs(7))),
            Duration::from_secs(7)
        );
        assert_eq!(
            config.delay(1, Some(Duration::from_secs(70))),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn parses_retry_after() {
        let mut headers = HeaderMap::new();
        assert_eq!(retry_after(&headers), None);
        headers.insert(RETRY_AFTER, HeaderValue::from_static("12"));
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(12)));
        headers.insert(
            RETRY_AFTER,
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        assert_eq!(retry_after(&headers), Some(Duration::ZERO));
        headers.insert(RETRY_AFTER, HeaderValue::from_static("soon"));
        assert_eq!(retry_after(&headers), None);
    }

    #[test]
    fn retryable_statuses() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn retryable_graphql_errors() {
        let error = |extensions: serde_json::Value| -> GraphQlError {
            serde_json::from_value(serde_json::json!({
                "message": "m",
                "extensions": extensions,
            }))
            .unwrap()
        };
        let retryable = [
            error(serde_json::json!({ "errorDetail": "TOO_MANY_REQUESTS" })),
            error(serde_json::json!({ "errorDetail": "ENHANCE_YOUR_CALM" })),
            error(serde_json::json!({ "errorType": "UNAVAILABLE" })),
        ];
        for e in retryable {
            assert!(is_retryable_graphql(std::slice::from_ref(&e)), "{e}");
        }
        let not_found = error(serde_json::json!({ "errorType": "NOT_FOUND" }));
        assert!(!is_retryable_graphql(std::slice::from_ref(&not_found)));
        assert!(!is_retryable_graphql(&[]));
        let throttled = error(serde_json::json!({ "errorDetail": "TOO_MANY_REQUESTS" }));
        assert!(is_retryable_graphql(&[not_found, throttled]));
    }

    #[test]
    fn attempts_are_at_least_one() {
        assert_eq!(RetryConfig::new().max_attempts(0).attempts(), 1);
        assert_eq!(RetryConfig::disabled().attempts(), 1);
    }
}
