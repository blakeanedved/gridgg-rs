//! The [`Client`], its builder and configuration types.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use secrecy::SecretString;
use url::Url;

use crate::error::{Error, Result, redact_url};
use crate::rate_limit::RateLimiter;
use crate::retry::RetryConfig;
use crate::{central, series_state, stats};

/// Which GRID platform a key belongs to. Selects the default hosts and
/// rate limit.
///
/// | API | [`OpenAccess`](Self::OpenAccess) | [`Commercial`](Self::Commercial) |
/// |---|---|---|
/// | Central Data | `https://api-op.grid.gg/central-data/graphql` | `https://api.grid.gg/central-data/graphql` |
/// | Series State | `https://api-op.grid.gg/live-data-feed/series-state/graphql` | `https://api.grid.gg/live-data-feed/series-state/graphql` |
/// | Stats Feed | `https://api-op.grid.gg/statistics-feed/graphql` | `https://api.grid.gg/statistics-feed/graphql` |
/// | File Download | `https://api.grid.gg/file-download` | `https://api.grid.gg/file-download` |
/// | Series Events | `wss://api.grid.gg/live-data-feed/series` | `wss://api.grid.gg/live-data-feed/series` |
/// | GraphQL rate limit | 20 requests/minute | 40 requests/minute |
/// | Stats Feed rate limit | 10 requests/minute | none |
///
/// The commercial GraphQL hosts are **unverified**: GRID documents only the
/// Open Access GraphQL endpoints. Override them with the per-API URL methods
/// on [`ClientBuilder`] if your account uses different ones.
///
/// The Open Access Stats Feed limit was observed live (GRID reported
/// `rateLimitPerMinute: 10`); whether commercial keys have a Stats Feed
/// limit below the GraphQL one is unverified. See
/// [`ClientBuilder::stats_rate_limit`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Environment {
    /// GRID Open Access (Open Platform) keys.
    #[default]
    OpenAccess,
    /// Commercial (Closed Platform) keys. The GraphQL hosts are unverified.
    Commercial,
}

impl Environment {
    fn graphql_host(self) -> &'static str {
        match self {
            Self::OpenAccess => "https://api-op.grid.gg",
            Self::Commercial => "https://api.grid.gg",
        }
    }

    fn rate_limit(self) -> RateLimit {
        match self {
            Self::OpenAccess => RateLimit::PerMinute(20),
            Self::Commercial => RateLimit::PerMinute(40),
        }
    }

    fn stats_rate_limit(self) -> RateLimit {
        match self {
            Self::OpenAccess => RateLimit::PerMinute(10),
            Self::Commercial => RateLimit::Off,
        }
    }
}

/// Client-side limit on GraphQL requests.
///
/// GRID rejects requests above the per-key limit, so the client spaces its
/// GraphQL requests evenly instead of sending bursts. One limiter is shared
/// by the Central Data, Series State and Stats Feed APIs and by every clone
/// of a [`Client`]; separately built clients do not share it. Retries count
/// against the limit.
///
/// The Stats Feed has its own, lower limit on top of the shared one; see
/// [`ClientBuilder::stats_rate_limit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RateLimit {
    /// At most this many GraphQL requests per minute. Must be non-zero.
    PerMinute(u32),
    /// No client-side limiting.
    Off,
}

/// An async client for the GRID APIs.
///
/// Cloning is cheap: clones share the connection pool, rate limiter and
/// configuration.
///
/// ```no_run
/// # async fn example() -> gridgg::Result<()> {
/// use gridgg::{Client, Environment};
///
/// let client = Client::builder()
///     .api_key(std::env::var("GRID_API_KEY").unwrap_or_default())
///     .environment(Environment::OpenAccess)
///     .build()?;
/// let data = client
///     .central()
///     .raw("{ titles { id name } }", serde_json::Value::Null)
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    pub(crate) inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub(crate) http: reqwest::Client,
    pub(crate) api_key: SecretString,
    pub(crate) central_url: Url,
    pub(crate) series_state_url: Url,
    pub(crate) stats_url: Url,
    #[cfg(feature = "files")]
    pub(crate) files_url: Url,
    #[cfg(feature = "events")]
    pub(crate) events_url: Url,
    pub(crate) limiter: Option<RateLimiter>,
    pub(crate) stats_limiter: Option<RateLimiter>,
    pub(crate) retry: RetryConfig,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = &*self.inner;
        let mut s = f.debug_struct("Client");
        s.field("api_key", &"[REDACTED]")
            .field("central_url", &redacted(&inner.central_url))
            .field("series_state_url", &redacted(&inner.series_state_url))
            .field("stats_url", &redacted(&inner.stats_url));
        #[cfg(feature = "files")]
        s.field("files_url", &redacted(&inner.files_url));
        #[cfg(feature = "events")]
        s.field("events_url", &redacted(&inner.events_url));
        s.field("limiter", &inner.limiter)
            .field("stats_limiter", &inner.stats_limiter)
            .field("retry", &inner.retry)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Starts building a client.
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// The Central Data API (titles, tournaments, teams, players, series).
    #[must_use]
    pub fn central(&self) -> central::Central {
        central::Central::new(self.clone())
    }

    /// The Series State API (live and final state of a series).
    #[must_use]
    pub fn series_state(&self) -> series_state::SeriesState {
        series_state::SeriesState::new(self.clone())
    }

    /// The Stats Feed API (aggregated statistics).
    #[must_use]
    pub fn stats(&self) -> stats::Stats {
        stats::Stats::new(self.clone())
    }

    /// The File Download API (post-series files).
    #[cfg(feature = "files")]
    #[must_use]
    pub fn files(&self) -> crate::files::Files {
        crate::files::Files::new(self.clone())
    }

    /// The Series Events API (live event stream over WebSocket).
    #[cfg(feature = "events")]
    #[must_use]
    pub fn events(&self) -> crate::events::Events {
        crate::events::Events::new(self.clone())
    }
}

/// Builder for [`Client`]; see [`Client::builder`].
///
/// Only [`api_key`](Self::api_key) is required.
#[derive(Default)]
pub struct ClientBuilder {
    api_key: Option<SecretString>,
    environment: Environment,
    central_url: Option<String>,
    series_state_url: Option<String>,
    stats_url: Option<String>,
    #[cfg(feature = "files")]
    files_url: Option<String>,
    #[cfg(feature = "events")]
    events_url: Option<String>,
    rate_limit: Option<RateLimit>,
    stats_rate_limit: Option<RateLimit>,
    retry: RetryConfig,
    http_client: Option<reqwest::Client>,
    timeout: Option<Duration>,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field("environment", &self.environment)
            .field("rate_limit", &self.rate_limit)
            .field("stats_rate_limit", &self.stats_rate_limit)
            .field("retry", &self.retry)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    /// Sets the GRID API key. It is held as a [`SecretString`] and is never
    /// logged or included in errors.
    #[must_use]
    pub fn api_key(mut self, key: impl Into<SecretString>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Selects the platform presets (hosts and rate limit). Defaults to
    /// [`Environment::OpenAccess`].
    #[must_use]
    pub fn environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// Overrides the Central Data GraphQL endpoint.
    #[must_use]
    pub fn central_url(mut self, url: impl Into<String>) -> Self {
        self.central_url = Some(url.into());
        self
    }

    /// Overrides the Series State GraphQL endpoint.
    #[must_use]
    pub fn series_state_url(mut self, url: impl Into<String>) -> Self {
        self.series_state_url = Some(url.into());
        self
    }

    /// Overrides the Stats Feed GraphQL endpoint.
    #[must_use]
    pub fn stats_url(mut self, url: impl Into<String>) -> Self {
        self.stats_url = Some(url.into());
        self
    }

    /// Overrides the File Download base URL.
    #[cfg(feature = "files")]
    #[must_use]
    pub fn files_url(mut self, url: impl Into<String>) -> Self {
        self.files_url = Some(url.into());
        self
    }

    /// Overrides the Series Events base URL (the series ID is appended).
    #[cfg(feature = "events")]
    #[must_use]
    pub fn events_url(mut self, url: impl Into<String>) -> Self {
        self.events_url = Some(url.into());
        self
    }

    /// Overrides the environment's GraphQL rate limit.
    #[must_use]
    pub fn rate_limit(mut self, rate_limit: RateLimit) -> Self {
        self.rate_limit = Some(rate_limit);
        self
    }

    /// Overrides the environment's Stats Feed rate limit, which applies to
    /// Stats Feed requests in addition to the shared GraphQL
    /// [`rate_limit`](Self::rate_limit).
    ///
    /// Defaults to 10 requests/minute for [`Environment::OpenAccess`] (the
    /// limit GRID reported in a live rate-limit error) and
    /// [`RateLimit::Off`] for [`Environment::Commercial`] (unverified).
    #[must_use]
    pub fn stats_rate_limit(mut self, rate_limit: RateLimit) -> Self {
        self.stats_rate_limit = Some(rate_limit);
        self
    }

    /// Sets the retry policy. Defaults to [`RetryConfig::default`].
    #[must_use]
    pub fn retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// Uses a preconfigured [`reqwest::Client`] instead of building one.
    /// Cannot be combined with [`timeout`](Self::timeout).
    ///
    /// You then own its redirect policy. The built-in client follows only
    /// same-origin redirects (same scheme, host and port), because reqwest's
    /// default policy forwards the `x-api-key` header to any host a server
    /// redirects to. Configure the same with [`reqwest::redirect::Policy`] if
    /// that matters to you.
    #[must_use]
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Sets a per-request timeout for the built-in HTTP client. By default
    /// requests have no timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the client.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`] if no API key was set, a URL override does
    /// not parse, a rate limit is `PerMinute(0)`, or both
    /// [`http_client`](Self::http_client) and [`timeout`](Self::timeout) were
    /// set; [`Error::Transport`] if the HTTP client cannot be built.
    pub fn build(self) -> Result<Client> {
        let api_key = self
            .api_key
            .ok_or_else(|| Error::Config("an API key is required".into()))?;
        let env = self.environment;
        let graphql = |path: &str| format!("{}{path}", env.graphql_host());

        let limiter = build_limiter(self.rate_limit.unwrap_or(env.rate_limit()))?;
        let stats_limiter = build_limiter(self.stats_rate_limit.unwrap_or(env.stats_rate_limit()))?;

        let http = match (self.http_client, self.timeout) {
            (Some(_), Some(_)) => {
                return Err(Error::Config(
                    "timeout cannot be combined with a custom http_client; \
                     configure the timeout on the reqwest::Client instead"
                        .into(),
                ));
            }
            (Some(client), None) => client,
            (None, timeout) => {
                let mut builder = reqwest::Client::builder()
                    .user_agent(concat!("gridgg/", env!("CARGO_PKG_VERSION")))
                    .redirect(reqwest::redirect::Policy::custom(same_origin_redirects));
                if let Some(timeout) = timeout {
                    builder = builder.timeout(timeout);
                }
                builder.build().map_err(Error::transport)?
            }
        };

        let inner = Inner {
            http,
            api_key,
            central_url: parse_url(
                "central_url",
                self.central_url,
                graphql("/central-data/graphql"),
            )?,
            series_state_url: parse_url(
                "series_state_url",
                self.series_state_url,
                graphql("/live-data-feed/series-state/graphql"),
            )?,
            stats_url: parse_url(
                "stats_url",
                self.stats_url,
                graphql("/statistics-feed/graphql"),
            )?,
            #[cfg(feature = "files")]
            files_url: parse_url(
                "files_url",
                self.files_url,
                "https://api.grid.gg/file-download".into(),
            )?,
            #[cfg(feature = "events")]
            events_url: parse_url(
                "events_url",
                self.events_url,
                "wss://api.grid.gg/live-data-feed/series".into(),
            )?,
            limiter,
            stats_limiter,
            retry: self.retry,
        };
        crate::graphql::api_key_header(&inner)?;
        Ok(Client {
            inner: Arc::new(inner),
        })
    }
}

/// Redirect policy of the built-in HTTP client: follow redirects within the
/// same origin (scheme, host and port) only, so the `x-api-key` header never
/// reaches another host. A cross-origin redirect is returned as the `3xx`
/// response itself.
fn same_origin_redirects(attempt: reqwest::redirect::Attempt<'_>) -> reqwest::redirect::Action {
    let previous = attempt.previous();
    if previous.len() > 10 {
        return attempt.error("too many redirects");
    }
    let same_origin = previous
        .last()
        .is_some_and(|from| from.origin() == attempt.url().origin());
    if same_origin {
        attempt.follow()
    } else {
        attempt.stop()
    }
}

/// The URL as a string with any `key` query parameter redacted.
fn redacted(url: &Url) -> String {
    let mut url = url.clone();
    redact_url(&mut url);
    url.into()
}

fn build_limiter(rate_limit: RateLimit) -> Result<Option<RateLimiter>> {
    match rate_limit {
        RateLimit::PerMinute(0) => Err(Error::Config(
            "RateLimit::PerMinute must be non-zero; use RateLimit::Off".into(),
        )),
        RateLimit::PerMinute(n) => Ok(Some(RateLimiter::per_minute(n))),
        RateLimit::Off => Ok(None),
    }
}

fn parse_url(name: &str, value: Option<String>, default: String) -> Result<Url> {
    let value = value.unwrap_or(default);
    Url::parse(&value).map_err(|e| Error::Config(format!("invalid {name}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(builder: ClientBuilder) -> Client {
        builder.api_key("k").build().unwrap()
    }

    #[test]
    fn open_access_presets() {
        let client = build(Client::builder());
        let inner = &client.inner;
        assert_eq!(
            inner.central_url.as_str(),
            "https://api-op.grid.gg/central-data/graphql"
        );
        assert_eq!(
            inner.series_state_url.as_str(),
            "https://api-op.grid.gg/live-data-feed/series-state/graphql"
        );
        assert_eq!(
            inner.stats_url.as_str(),
            "https://api-op.grid.gg/statistics-feed/graphql"
        );
        assert!(inner.limiter.is_some());
        assert!(inner.stats_limiter.is_some());
    }

    #[test]
    fn commercial_presets_and_overrides() {
        let client = build(
            Client::builder()
                .environment(Environment::Commercial)
                .stats_url("http://localhost:1234/stats")
                .rate_limit(RateLimit::Off),
        );
        let inner = &client.inner;
        assert_eq!(
            inner.central_url.as_str(),
            "https://api.grid.gg/central-data/graphql"
        );
        assert_eq!(inner.stats_url.as_str(), "http://localhost:1234/stats");
        assert!(inner.limiter.is_none());
        assert!(inner.stats_limiter.is_none());
    }

    #[test]
    fn rejects_bad_config() {
        let err = |b: ClientBuilder| b.build().unwrap_err().to_string();
        assert!(err(Client::builder()).contains("API key"));
        assert!(err(Client::builder().api_key("k").central_url("nope")).contains("central_url"));
        assert!(
            err(Client::builder()
                .api_key("k")
                .rate_limit(RateLimit::PerMinute(0)))
            .contains("non-zero")
        );
        assert!(
            err(Client::builder()
                .api_key("k")
                .stats_rate_limit(RateLimit::PerMinute(0)))
            .contains("non-zero")
        );
        assert!(
            err(Client::builder()
                .api_key("k")
                .http_client(reqwest::Client::new())
                .timeout(Duration::from_secs(1)))
            .contains("timeout")
        );
    }

    #[test]
    fn debug_hides_api_key() {
        let builder = Client::builder()
            .api_key("super-secret-key")
            .central_url("https://example.com/graphql?key=super-secret-key");
        assert!(!format!("{builder:?}").contains("super-secret-key"));
        let client = builder.build().unwrap();
        assert!(!format!("{client:?}").contains("super-secret-key"));
    }
}
