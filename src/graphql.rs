//! Shared GraphQL-over-HTTPS transport.

use reqwest::StatusCode;
use reqwest::header::HeaderValue;
use secrecy::ExposeSecret;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::client::{Client, Inner};
use crate::error::{Error, GraphQlError, Result};
use crate::{rate_limit, retry};

pub(crate) mod selection;

/// The GraphQL APIs.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Api {
    Central,
    SeriesState,
    Stats,
}

impl Api {
    fn name(self) -> &'static str {
        match self {
            Self::Central => "central-data",
            Self::SeriesState => "series-state",
            Self::Stats => "statistics-feed",
        }
    }

    fn url(self, inner: &Inner) -> &url::Url {
        match self {
            Self::Central => &inner.central_url,
            Self::SeriesState => &inner.series_state_url,
            Self::Stats => &inner.stats_url,
        }
    }
}

#[derive(Serialize)]
struct RawRequest<'a> {
    query: &'a str,
    variables: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct RawResponse {
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    errors: Option<Vec<GraphQlError>>,
}

/// Builds the `x-api-key` header value.
pub(crate) fn api_key_header(inner: &Inner) -> Result<HeaderValue> {
    let mut value = HeaderValue::from_str(inner.api_key.expose_secret())
        .map_err(|_| Error::Config("the API key contains invalid header characters".into()))?;
    value.set_sensitive(true);
    Ok(value)
}

/// Sends a cynic operation and decodes its `data`.
///
/// Selected fields missing from the response are decoded as `null` first:
/// GRID omits null fields (seen on Central Data), and fields excluded by
/// `@include`/`@skip` are absent, but cynic's derives reject missing keys
/// on some types.
pub(crate) async fn run<Q, V>(
    client: &Client,
    api: Api,
    operation: &cynic::Operation<Q, V>,
) -> Result<Q>
where
    Q: DeserializeOwned + 'static,
    V: Serialize,
{
    let mut data: serde_json::Value = execute(client, api, operation).await?;
    if let Some(selections) = selection::parse(&operation.query) {
        selection::fill_missing(&mut data, &selections);
    } else {
        tracing::debug!(
            api = api.name(),
            query = %operation.query,
            "could not parse the operation's selection set; missing fields are not filled with null"
        );
    }
    Q::deserialize(data).map_err(Error::Decode)
}

/// Sends a query string with JSON variables and returns `data` as JSON.
pub(crate) async fn raw(
    client: &Client,
    api: Api,
    query: &str,
    variables: serde_json::Value,
) -> Result<serde_json::Value> {
    execute(client, api, &RawRequest { query, variables }).await
}

async fn execute<T, B>(client: &Client, api: Api, body: &B) -> Result<T>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let inner = &*client.inner;
    let url = api.url(inner);
    let attempts = inner.retry.attempts();
    let mut attempt = 0;
    loop {
        attempt += 1;
        acquire(inner, api).await;
        tracing::debug!(api = api.name(), attempt, "sending GraphQL request");
        let sent = inner
            .http
            .post(url.clone())
            .header("x-api-key", api_key_header(inner)?)
            .json(body)
            .send()
            .await;
        let response = match sent {
            Ok(response) => response,
            Err(err) if attempt < attempts && retry::is_retryable_error(&err) => {
                let delay = inner.retry.delay(attempt, None);
                tracing::warn!(
                    api = api.name(),
                    attempt,
                    delay_ms = delay.as_millis(),
                    error = ?Error::transport(err),
                    "GraphQL request failed; retrying"
                );
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(err) => return Err(Error::transport(err)),
        };

        let status = response.status();
        let retry_after = retry::retry_after(response.headers());
        if status.is_success() {
            let bytes = response.bytes().await.map_err(Error::transport)?;
            match decode(&bytes) {
                Err(Error::GraphQl(errors))
                    if attempt < attempts && retry::is_retryable_graphql(&errors) =>
                {
                    retry_graphql_errors(inner, api, attempt, retry_after, &errors).await;
                    continue;
                }
                result => return result,
            }
        }

        if attempt < attempts && retry::is_retryable_status(status) {
            let delay = inner.retry.delay(attempt, retry_after);
            tracing::warn!(
                api = api.name(),
                attempt,
                status = status.as_u16(),
                delay_ms = delay.as_millis(),
                "GraphQL request rejected; retrying"
            );
            drop(response);
            tokio::time::sleep(delay).await;
            continue;
        }

        let body = response.bytes().await.unwrap_or_default();
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(Error::RateLimited {
                attempts: attempt,
                retry_after,
            });
        }
        if status.is_client_error()
            && let Ok(RawResponse {
                errors: Some(errors),
                ..
            }) = serde_json::from_slice(&body)
            && !errors.is_empty()
        {
            if attempt < attempts && retry::is_retryable_graphql(&errors) {
                retry_graphql_errors(inner, api, attempt, retry_after, &errors).await;
                continue;
            }
            return Err(Error::GraphQl(errors));
        }
        return Err(Error::http(status, &body));
    }
}

/// Waits for a slot free on both the API's own limiter (Stats Feed only) and
/// the shared one. The slot is reserved on both at once, so both advance
/// from the actual send time.
async fn acquire(inner: &Inner, api: Api) {
    let stats = match api {
        Api::Stats => inner.stats_limiter.as_ref(),
        Api::Central | Api::SeriesState => None,
    };
    match (stats, &inner.limiter) {
        (Some(stats), Some(shared)) => rate_limit::acquire_all(&[stats, shared]).await,
        (Some(limiter), None) | (None, Some(limiter)) => limiter.acquire().await,
        (None, None) => {}
    }
}

/// Logs and waits before retrying a response whose GraphQL errors signal
/// throttling or a transient outage.
async fn retry_graphql_errors(
    inner: &Inner,
    api: Api,
    attempt: u32,
    retry_after: Option<std::time::Duration>,
    errors: &[GraphQlError],
) {
    let delay = inner.retry.delay(attempt, retry_after);
    tracing::warn!(
        api = api.name(),
        attempt,
        delay_ms = delay.as_millis(),
        error = %errors[0],
        "GraphQL request throttled or unavailable; retrying"
    );
    tokio::time::sleep(delay).await;
}

/// Decodes a GraphQL response body: a non-empty `errors` array wins over
/// any (partial) `data`.
fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
    let response: RawResponse = serde_json::from_slice(body).map_err(Error::Decode)?;
    if let Some(errors) = response.errors
        && !errors.is_empty()
    {
        return Err(Error::GraphQl(errors));
    }
    T::deserialize(response.data.unwrap_or(serde_json::Value::Null)).map_err(Error::Decode)
}

/// Defines a lightweight handle for one GraphQL API with `run` and `raw`.
macro_rules! graphql_handle {
    ($(#[$meta:meta])* $name:ident, $api:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $name {
            client: $crate::Client,
        }

        impl $name {
            pub(crate) fn new(client: $crate::Client) -> Self {
                Self { client }
            }

            /// Runs a [`cynic::Operation`] built against this API's
            /// [`schema`](self::schema) module and decodes its `data`.
            ///
            /// Selected fields missing from the response (GRID omits some
            /// null fields; `@include`/`@skip` remove others) decode as
            /// `null`.
            ///
            /// # Errors
            ///
            /// Returns [`Error::GraphQl`](crate::Error::GraphQl) if the
            /// response has a non-empty `errors` array (even alongside
            /// partial data), and the other [`Error`](crate::Error) variants
            /// for transport, HTTP and decoding failures.
            pub async fn run<Q, V>(
                &self,
                operation: &cynic::Operation<Q, V>,
            ) -> $crate::Result<Q>
            where
                Q: serde::de::DeserializeOwned + 'static,
                V: serde::Serialize,
            {
                $crate::graphql::run(&self.client, $api, operation).await
            }

            /// Runs a GraphQL query string with JSON `variables` (use
            /// [`serde_json::Value::Null`] for none) and returns `data`.
            ///
            /// # Errors
            ///
            /// As for [`run`](Self::run).
            pub async fn raw(
                &self,
                query: &str,
                variables: serde_json::Value,
            ) -> $crate::Result<serde_json::Value> {
                $crate::graphql::raw(&self.client, $api, query, variables).await
            }
        }
    };
}

pub(crate) use graphql_handle;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorType;

    #[tokio::test(start_paused = true)]
    async fn stats_requests_also_wait_for_the_stats_limiter() {
        use tokio::time::Instant;

        use crate::{Environment, RateLimit};

        let offsets = |builder: crate::ClientBuilder, apis: &'static [Api]| async move {
            let client = builder.api_key("k").build().unwrap();
            let start = Instant::now();
            let mut offsets = Vec::new();
            for api in apis {
                acquire(&client.inner, *api).await;
                offsets.push(start.elapsed().as_millis());
            }
            offsets
        };
        let mixed = &[Api::Stats, Api::Stats, Api::Central, Api::Stats];
        // Open Access: stats every 6 s, all requests at least 3 s apart.
        assert_eq!(
            offsets(Client::builder(), mixed).await,
            [0, 6000, 9000, 12000]
        );
        let commercial = Client::builder().environment(Environment::Commercial);
        assert_eq!(offsets(commercial, mixed).await, [0, 1500, 3000, 4500]);
        let off = Client::builder().stats_rate_limit(RateLimit::Off);
        assert_eq!(offsets(off, &[Api::Stats, Api::Stats]).await, [0, 3000]);
    }

    #[tokio::test(start_paused = true)]
    async fn stats_spacing_holds_when_shared_limiter_is_booked_ahead() {
        use tokio::time::Instant;

        let client = Client::builder().api_key("k").build().unwrap();
        let start = Instant::now();
        let spawn = |api: Api| {
            let client = client.clone();
            tokio::spawn(async move {
                acquire(&client.inner, api).await;
                start.elapsed().as_millis()
            })
        };
        // Five concurrent Central requests book the shared limiter to 15 s.
        let central: Vec<_> = (0..5).map(|_| spawn(Api::Central)).collect();
        tokio::task::yield_now().await;
        let stats: Vec<_> = (0..3).map(|_| spawn(Api::Stats)).collect();
        let mut sent = Vec::new();
        for task in stats {
            sent.push(task.await.unwrap());
        }
        for task in central {
            task.await.unwrap();
        }
        sent.sort_unstable();
        assert_eq!(sent[0], 15_000, "shared limiter was not booked ahead");
        for pair in sent.windows(2) {
            assert!(pair[1] - pair[0] >= 6000, "stats sent too close: {sent:?}");
        }
    }

    #[test]
    fn errors_win_over_partial_data() {
        let body =
            br#"{"data":{"a":1},"errors":[{"message":"x","extensions":{"errorType":"INTERNAL"}}]}"#;
        let Err(Error::GraphQl(errors)) = decode::<serde_json::Value>(body) else {
            panic!("expected GraphQL error");
        };
        assert_eq!(
            errors[0].extensions.as_ref().unwrap().error_type,
            Some(ErrorType::Internal)
        );
    }

    #[test]
    fn empty_errors_array_is_success() {
        let value: serde_json::Value = decode(br#"{"data":{"a":1},"errors":[]}"#).unwrap();
        assert_eq!(value, serde_json::json!({"a": 1}));
    }

    #[test]
    fn non_json_body_is_decode_error() {
        assert!(matches!(
            decode::<serde_json::Value>(b"<html>"),
            Err(Error::Decode(_))
        ));
    }
}
