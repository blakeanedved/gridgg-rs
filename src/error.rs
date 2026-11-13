//! Error types returned by the SDK.

use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;

/// Maximum number of bytes of a response body kept in [`Error::Http`].
const BODY_SNIPPET_LIMIT: usize = 1024;

/// A specialized [`Result`](std::result::Result) for this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors returned by the SDK.
///
/// The API key never appears in the [`Display`](fmt::Display) or
/// [`Debug`](fmt::Debug) output of an error: a `key` query parameter in any
/// URL carried by a transport error is redacted.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The HTTP request failed before a response arrived (DNS, connect,
    /// TLS, timeout, or body read errors). The cause is the
    /// [`source`](std::error::Error::source).
    #[error("transport error")]
    Transport(#[source] reqwest::Error),

    /// The server answered with an unsuccessful HTTP status.
    #[error("HTTP status {status}: {body}")]
    Http {
        /// The HTTP status code.
        status: StatusCode,
        /// The start of the response body (at most 1 KiB), for diagnostics.
        body: String,
    },

    /// The File Download API answered with an unsuccessful HTTP status and a
    /// JSON body of the form `{"message": "..."}` (GRID's `ApiErrorMessage`).
    /// Unsuccessful responses with any other body are [`Error::Http`].
    #[error("HTTP status {status}: {message}")]
    Api {
        /// The HTTP status code.
        status: StatusCode,
        /// The `message` GRID sent.
        message: String,
    },

    /// The GraphQL response contained a non-empty `errors` array.
    ///
    /// This is returned even when the response also carried partial `data`.
    #[error("GraphQL error: {}", DisplayErrors(.0))]
    GraphQl(Vec<GraphQlError>),

    /// The response body could not be decoded into the expected type. The
    /// cause is the [`source`](std::error::Error::source).
    #[error("failed to decode response")]
    Decode(#[source] serde_json::Error),

    /// The Series Events WebSocket failed. The cause is the
    /// [`source`](std::error::Error::source).
    #[cfg(feature = "events")]
    #[error("WebSocket error")]
    WebSocket(#[source] Box<tokio_tungstenite::tungstenite::Error>),

    /// The Series Events server closed the WebSocket with a close code
    /// other than `1000` (normal closure).
    #[cfg(feature = "events")]
    #[error("WebSocket closed by the server with code {code}: {reason}")]
    WebSocketClosed {
        /// The close code.
        code: u16,
        /// The reason the server gave (may be empty).
        reason: String,
    },

    /// Writing a download to disk failed. The cause is the
    /// [`source`](std::error::Error::source).
    #[cfg(feature = "files")]
    #[error("I/O error")]
    Io(#[source] std::io::Error),

    /// A downloaded zip archive could not be read. The cause is the
    /// [`source`](std::error::Error::source).
    #[cfg(feature = "zip")]
    #[error("invalid zip archive")]
    Archive(#[source] zip::result::ZipError),

    /// The client or a request was configured incorrectly.
    #[error("invalid configuration: {0}")]
    Config(String),

    /// The server kept answering `429 Too Many Requests` until the retry
    /// budget was exhausted.
    #[error("rate limited by the server after {attempts} attempt(s)")]
    RateLimited {
        /// How many requests were sent.
        attempts: u32,
        /// The last `Retry-After` value the server sent, if any.
        retry_after: Option<Duration>,
    },
}

impl Error {
    /// Wraps a reqwest error, redacting the API key from its URL.
    pub(crate) fn transport(mut err: reqwest::Error) -> Self {
        if let Some(url) = err.url_mut() {
            redact_url(url);
        }
        Self::Transport(err)
    }

    /// Builds an [`Error::Http`] from a status and a raw body.
    pub(crate) fn http(status: StatusCode, body: &[u8]) -> Self {
        let text = String::from_utf8_lossy(body);
        let mut end = text.len().min(BODY_SNIPPET_LIMIT);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Self::Http {
            status,
            body: text[..end].to_owned(),
        }
    }

    /// The HTTP status code, if this error carries one.
    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Http { status, .. } | Self::Api { status, .. } => Some(*status),
            Self::RateLimited { .. } => Some(StatusCode::TOO_MANY_REQUESTS),
            Self::Transport(err) => err.status(),
            #[cfg(feature = "events")]
            Self::WebSocket(err) => match &**err {
                tokio_tungstenite::tungstenite::Error::Http(response) => Some(response.status()),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Replaces the value of every `key` query parameter with `REDACTED`.
pub(crate) fn redact_url(url: &mut url::Url) {
    if !url.query_pairs().any(|(name, _)| name == "key") {
        return;
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(name, value)| {
            let value = if name == "key" {
                "REDACTED".to_owned()
            } else {
                value.into_owned()
            };
            (name.into_owned(), value)
        })
        .collect();
    url.query_pairs_mut().clear().extend_pairs(pairs);
}

struct DisplayErrors<'a>(&'a [GraphQlError]);

impl fmt::Display for DisplayErrors<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            [] => f.write_str("empty errors array"),
            [first] => write!(f, "{first}"),
            [first, rest @ ..] => write!(f, "{first} (and {} more)", rest.len()),
        }
    }
}

/// One entry of a GraphQL response's `errors` array.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct GraphQlError {
    /// Human-readable error message.
    pub message: String,
    /// Path of the response field the error belongs to, if any.
    #[serde(default)]
    pub path: Option<Vec<PathSegment>>,
    /// GRID-specific error details.
    #[serde(default)]
    pub extensions: Option<ErrorExtensions>,
}

impl fmt::Display for GraphQlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(ext) = &self.extensions {
            match (&ext.error_type, &ext.error_detail) {
                (Some(ty), Some(detail)) => write!(f, " [{ty}/{detail}]")?,
                (Some(ty), None) => write!(f, " [{ty}]")?,
                (None, Some(detail)) => write!(f, " [{detail}]")?,
                (None, None) => {}
            }
        }
        Ok(())
    }
}

/// One segment of a [`GraphQlError::path`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum PathSegment {
    /// A field name.
    Field(String),
    /// A list index.
    Index(u64),
}

/// The `extensions` object GRID attaches to GraphQL errors.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ErrorExtensions {
    /// Gateway error code, for example `DOWNSTREAM_SERVICE_ERROR`.
    #[serde(default)]
    pub code: Option<String>,
    /// Name of the downstream service that produced the error.
    #[serde(default)]
    pub service_name: Option<String>,
    /// Error classification.
    #[serde(default)]
    pub error_type: Option<ErrorType>,
    /// Finer-grained error detail.
    #[serde(default)]
    pub error_detail: Option<ErrorDetail>,
    /// Any other extension fields (for example `exception`).
    #[serde(flatten)]
    pub other: serde_json::Map<String, serde_json::Value>,
}

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $wire:literal, )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, ::serde::Deserialize)]
        #[serde(from = "String")]
        #[non_exhaustive]
        pub enum $name {
            $( $(#[$vmeta])* $variant, )*
            /// A value this version of the SDK does not know.
            Other(String),
        }

        impl $name {
            /// The value as sent on the wire.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$variant => $wire, )*
                    Self::Other(value) => value,
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                match value.as_str() {
                    $( $wire => Self::$variant, )*
                    _ => Self::Other(value),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

#[cfg(feature = "files")]
pub(crate) use string_enum;

string_enum! {
    /// GRID's error classification (the `ErrorType` enum of the Central Data
    /// schema). Each value's HTTP mapping is noted.
    ErrorType {
        /// The request is malformed; retrying it unchanged will not succeed
        /// (HTTP 400).
        BadRequest = "BAD_REQUEST",
        /// The system is not in a state required for the operation
        /// (HTTP 400 or 500).
        FailedPrecondition = "FAILED_PRECONDITION",
        /// An unexpected internal error (HTTP 500).
        Internal = "INTERNAL",
        /// The requested entity was not found (HTTP 404).
        NotFound = "NOT_FOUND",
        /// The caller lacks permission for the operation (HTTP 403).
        PermissionDenied = "PERMISSION_DENIED",
        /// The request has no valid authentication credentials (HTTP 401).
        Unauthenticated = "UNAUTHENTICATED",
        /// The service is currently unavailable; retrying may succeed
        /// (HTTP 503).
        Unavailable = "UNAVAILABLE",
        /// Unknown error (HTTP 500).
        Unknown = "UNKNOWN",
    }
}

string_enum! {
    /// GRID's finer-grained error detail (the `ErrorDetail` enum of the
    /// Central Data schema).
    ErrorDetail {
        /// The deadline expired before the operation could complete.
        DeadlineExceeded = "DEADLINE_EXCEEDED",
        /// The client is being throttled.
        EnhanceYourCalm = "ENHANCE_YOUR_CALM",
        /// The requested field is not found in the schema.
        FieldNotFound = "FIELD_NOT_FOUND",
        /// An argument is invalid.
        InvalidArgument = "INVALID_ARGUMENT",
        /// A pagination cursor is invalid.
        InvalidCursor = "INVALID_CURSOR",
        /// A required resource is missing.
        MissingResource = "MISSING_RESOURCE",
        /// A downstream service failed.
        ServiceError = "SERVICE_ERROR",
        /// A TCP failure occurred.
        TcpFailure = "TCP_FAILURE",
        /// Too many concurrent requests.
        ThrottledConcurrency = "THROTTLED_CONCURRENCY",
        /// CPU throttling.
        ThrottledCpu = "THROTTLED_CPU",
        /// Too many requests (rate limited).
        TooManyRequests = "TOO_MANY_REQUESTS",
        /// The operation is not implemented.
        Unimplemented = "UNIMPLEMENTED",
        /// Unknown error detail.
        Unknown = "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_key_query_param() {
        let mut url: url::Url = "https://api.grid.gg/x?a=1&key=s3cr3t".parse().unwrap();
        redact_url(&mut url);
        assert_eq!(url.as_str(), "https://api.grid.gg/x?a=1&key=REDACTED");
    }

    #[test]
    fn leaves_urls_without_key_untouched() {
        let mut url: url::Url = "https://api.grid.gg/x?a=b%20c".parse().unwrap();
        redact_url(&mut url);
        assert_eq!(url.as_str(), "https://api.grid.gg/x?a=b%20c");
    }

    #[test]
    fn decodes_grid_error_shape() {
        let err: GraphQlError = serde_json::from_value(serde_json::json!({
            "message": "Series not found",
            "path": ["series", 0, "id"],
            "extensions": {
                "code": "DOWNSTREAM_SERVICE_ERROR",
                "serviceName": "central-data",
                "errorType": "NOT_FOUND",
                "errorDetail": "SOMETHING_NEW",
                "exception": {"message": "boom"}
            }
        }))
        .unwrap();
        assert_eq!(
            err.path,
            Some(vec![
                PathSegment::Field("series".into()),
                PathSegment::Index(0),
                PathSegment::Field("id".into()),
            ])
        );
        let ext = err.extensions.as_ref().unwrap();
        assert_eq!(ext.code.as_deref(), Some("DOWNSTREAM_SERVICE_ERROR"));
        assert_eq!(ext.service_name.as_deref(), Some("central-data"));
        assert_eq!(ext.error_type, Some(ErrorType::NotFound));
        assert_eq!(
            ext.error_detail,
            Some(ErrorDetail::Other("SOMETHING_NEW".into()))
        );
        assert!(ext.other.contains_key("exception"));
        assert_eq!(
            err.to_string(),
            "Series not found [NOT_FOUND/SOMETHING_NEW]"
        );
    }

    #[test]
    fn display_does_not_repeat_the_source() {
        use std::error::Error as _;

        let source = serde_json::from_str::<u8>("x").unwrap_err();
        let source_text = source.to_string();
        let err = Error::Decode(source);
        assert_eq!(err.to_string(), "failed to decode response");
        assert_eq!(err.source().unwrap().to_string(), source_text);
    }

    #[test]
    fn body_snippet_is_truncated_on_char_boundary() {
        let body = "é".repeat(BODY_SNIPPET_LIMIT);
        let Error::Http { body, .. } = Error::http(StatusCode::BAD_GATEWAY, body.as_bytes()) else {
            unreachable!()
        };
        assert!(body.len() <= BODY_SNIPPET_LIMIT);
        assert!(body.chars().all(|c| c == 'é'));
    }
}
