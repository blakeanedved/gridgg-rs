//! Series Events API: live event stream over WebSocket (feature `events`).
//!
//! [`Events::connect`] opens `wss://api.grid.gg/live-data-feed/series/{id}`
//! and returns an [`EventStream`] of [`Transaction`]s. Use
//! [`Events::series`] to configure keepalive and reconnection first.
//!
//! The API key travels in the `key` query parameter, as GRID requires. The
//! SDK never puts that URL in its errors or logs, but the `tungstenite`
//! crate logs the raw handshake request (including the key) at the `log`
//! crate's `trace` level under target `tungstenite::handshake::client`; keep
//! that target below `trace` in production.
//!
//! **Live behaviour:** an Open Access key may connect. For a finished series
//! GRID closes the connection at once with code `1000` and sends no data, so
//! the stream simply ends. Whether GRID replays earlier transactions after a
//! reconnect is unverified; the stream de-duplicates either way.
//!
//! ```no_run
//! # async fn example(client: gridgg::Client) -> gridgg::Result<()> {
//! use futures_util::StreamExt;
//!
//! let mut stream = client.events().connect("2618418").await?;
//! while let Some(transaction) = stream.next().await {
//!     let transaction = transaction?;
//!     for event in &transaction.events {
//!         println!("{} {}", transaction.sequence_number, event.event_type);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use cynic::Id;
use futures_util::future::{self, Either};
use futures_util::{SinkExt, Stream, StreamExt};
use secrecy::ExposeSecret;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::error::{ProtocolError, UrlError};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::{self, Message};
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use url::Url;

use crate::RetryConfig;
use crate::error::{Error, Result, redact_url};

mod types;

pub use types::{Entity, Event, EventType, Transaction};

/// The default interval between keepalive Ping frames.
pub const DEFAULT_KEEPALIVE: Duration = Duration::from_secs(30);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Handle for the Series Events API; see [`Client::events`](crate::Client::events).
#[derive(Debug, Clone)]
pub struct Events {
    client: crate::Client,
}

impl Events {
    pub(crate) fn new(client: crate::Client) -> Self {
        Self { client }
    }

    /// The base URL WebSocket connections are opened against; the series ID
    /// is appended to it.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.client.inner.events_url
    }

    /// Starts configuring a connection to one series' event stream.
    pub fn series(&self, series_id: impl Into<Id>) -> EventStreamBuilder {
        EventStreamBuilder {
            client: self.client.clone(),
            series_id: series_id.into(),
            keepalive: Some(DEFAULT_KEEPALIVE),
            reconnect: None,
        }
    }

    /// Connects to one series' event stream with the default settings:
    /// a Ping every 30 seconds and no reconnection. Shorthand for
    /// `self.series(series_id).connect()`.
    ///
    /// # Errors
    ///
    /// As for [`EventStreamBuilder::connect`].
    pub async fn connect(&self, series_id: impl Into<Id>) -> Result<EventStream> {
        self.series(series_id).connect().await
    }
}

/// Configures an [`EventStream`]; see [`Events::series`].
#[derive(Debug, Clone)]
#[must_use]
pub struct EventStreamBuilder {
    client: crate::Client,
    series_id: Id,
    keepalive: Option<Duration>,
    reconnect: Option<RetryConfig>,
}

impl EventStreamBuilder {
    /// Sets the interval between WebSocket Ping frames, which keep an idle
    /// connection open (GRID closes connections 24 hours after the last
    /// frame). `None` or a zero interval disables them. Defaults to
    /// [`DEFAULT_KEEPALIVE`] (30 seconds).
    pub fn keepalive(mut self, interval: impl Into<Option<Duration>>) -> Self {
        self.keepalive = interval.into().filter(|interval| !interval.is_zero());
        self
    }

    /// Reconnects after an abnormal disconnection, using `policy` for the
    /// backoff. Off by default: then an abnormal disconnection ends the
    /// stream with an error.
    ///
    /// A disconnection is abnormal when the connection fails or drops
    /// without a close frame, or the server closes it with a code other than
    /// `1000`. Each reconnection waits as a retry would (see
    /// [`RetryConfig`]); after `max_attempts` consecutive reconnection
    /// attempts without a new transaction being yielded in between (replayed
    /// transactions and undecodable messages do not count), the stream
    /// yields the last error and ends. A handshake rejected with a `4xx`
    /// status other than `429` is not retried.
    ///
    /// Transactions the server replays after a reconnect are skipped (see
    /// [`EventStream`]); transactions published while disconnected are lost
    /// if the server does not replay them.
    pub fn reconnect(mut self, policy: RetryConfig) -> Self {
        self.reconnect = Some(policy);
        self
    }

    /// Opens the WebSocket and returns the stream of transactions.
    ///
    /// # Errors
    ///
    /// Returns [`Error::WebSocket`] if the connection or handshake fails
    /// (including an HTTP rejection, whose status is available through
    /// [`Error::status`]), and [`Error::Config`] for an invalid series ID or
    /// base URL. The first connection is not retried.
    pub async fn connect(self) -> Result<EventStream> {
        let url = self.url()?;
        let tls = tls_config()?;
        let socket = open(&url, &tls).await?;
        tracing::debug!(
            series_id = self.series_id.inner(),
            "Series Events connected"
        );
        let session = Session {
            url,
            tls,
            series_id: self.series_id.into_inner(),
            keepalive: self.keepalive,
            reconnect: self.reconnect,
        };
        Ok(EventStream {
            inner: Box::pin(session.run(socket)),
        })
    }

    /// The connection URL, including the API key. Must never be logged.
    fn url(&self) -> Result<Url> {
        let id = match self.series_id.inner() {
            "" | "." | ".." => {
                return Err(Error::Config(format!(
                    "invalid series ID {:?}",
                    self.series_id.inner()
                )));
            }
            id => id,
        };
        let inner = &*self.client.inner;
        let mut url = inner.events_url.clone();
        url.path_segments_mut()
            .map_err(|()| Error::Config("events_url cannot be a base URL".into()))?
            .pop_if_empty()
            .push(id);
        url.query_pairs_mut()
            .append_pair("key", inner.api_key.expose_secret());
        Ok(url)
    }
}

/// A stream of [`Transaction`]s from the Series Events API; see
/// [`Events::connect`].
///
/// - Transactions are yielded in order, and their
///   [`sequence_number`](Transaction::sequence_number)s strictly increase: a
///   transaction whose number is not greater than the last one yielded (for
///   example one replayed after a reconnect) is skipped.
/// - A message that is not a valid transaction is yielded as
///   [`Error::Decode`]; the stream continues.
/// - When the server closes the connection normally (code `1000`, or no
///   code), the stream ends. GRID does this at once for a finished series.
/// - Any other disconnection is reconnected if
///   [`reconnect`](EventStreamBuilder::reconnect) is set; otherwise, or when
///   reconnection gives up, the stream yields [`Error::WebSocket`] or
///   [`Error::WebSocketClosed`] and ends.
///
/// Ping frames are sent on the [`keepalive`](EventStreamBuilder::keepalive)
/// interval while the stream is polled; the server's Pings are answered
/// automatically. Dropping the stream drops the connection.
pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = Result<Transaction>> + Send>>,
}

impl fmt::Debug for EventStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventStream").finish_non_exhaustive()
    }
}

impl Stream for EventStream {
    type Item = Result<Transaction>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Everything a stream needs to (re)connect.
struct Session {
    url: Url,
    tls: Arc<rustls::ClientConfig>,
    series_id: String,
    keepalive: Option<Duration>,
    reconnect: Option<RetryConfig>,
}

/// Why a connection ended abnormally.
enum Disconnect {
    Error(tungstenite::Error),
    Closed { code: u16, reason: String },
}

impl Disconnect {
    fn into_error(self) -> Error {
        match self {
            Self::Error(err) => ws_error(err),
            Self::Closed { code, reason } => Error::WebSocketClosed { code, reason },
        }
    }
}

impl Session {
    fn run(self, socket: Socket) -> impl Stream<Item = Result<Transaction>> + Send {
        async_stream::stream! {
            let series_id = self.series_id.as_str();
            let mut socket = socket;
            let mut last_sequence: Option<u64> = None;
            // Consecutive reconnection attempts without a new transaction in
            // between.
            let mut attempts: u32 = 0;
            loop {
                let mut ping = self.keepalive.map(|period| {
                    let mut ping =
                        tokio::time::interval_at(tokio::time::Instant::now() + period, period);
                    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    ping
                });

                let disconnect = loop {
                    let message = match ping.as_mut() {
                        Some(ping) => {
                            match future::select(socket.next(), std::pin::pin!(ping.tick())).await {
                                Either::Left((message, _)) => message,
                                Either::Right(_) => {
                                    tracing::trace!(series_id, "sending Series Events keepalive ping");
                                    if let Err(err) = socket.send(Message::Ping(Bytes::new())).await {
                                        break Disconnect::Error(err);
                                    }
                                    continue;
                                }
                            }
                        }
                        None => socket.next().await,
                    };
                    let data = match message {
                        Some(Ok(Message::Text(text))) => Bytes::from(text),
                        Some(Ok(Message::Binary(data))) => data,
                        Some(Ok(Message::Close(frame))) => {
                            let (code, reason) = frame.map_or((CloseCode::Normal, String::new()), |frame| {
                                (frame.code, frame.reason.as_str().to_owned())
                            });
                            if code == CloseCode::Normal {
                                tracing::debug!(series_id, "Series Events closed by the server");
                                // Send the queued close reply before dropping the socket.
                                let _ = socket.flush().await;
                                return;
                            }
                            break Disconnect::Closed { code: code.into(), reason };
                        }
                        Some(Ok(_)) => continue,
                        Some(Err(err)) => break Disconnect::Error(err),
                        None => break Disconnect::Error(tungstenite::Error::Protocol(
                            ProtocolError::ResetWithoutClosingHandshake,
                        )),
                    };
                    match serde_json::from_slice::<Transaction>(&data) {
                        Ok(transaction) => {
                            let sequence = transaction.sequence_number;
                            if last_sequence.is_some_and(|last| sequence <= last) {
                                tracing::debug!(series_id, sequence, "skipping already delivered transaction");
                                continue;
                            }
                            last_sequence = Some(sequence);
                            attempts = 0;
                            yield Ok(transaction);
                        }
                        Err(err) => yield Err(Error::Decode(err)),
                    }
                };

                let mut error = disconnect.into_error();
                let Some(policy) = &self.reconnect else {
                    yield Err(error);
                    return;
                };
                socket = loop {
                    if attempts >= policy.attempts() || is_fatal(&error) {
                        yield Err(error);
                        return;
                    }
                    attempts += 1;
                    let delay = policy.delay(attempts, None);
                    tracing::warn!(
                        series_id,
                        attempt = attempts,
                        delay_ms = delay.as_millis(),
                        error = ?error,
                        "Series Events connection lost; reconnecting"
                    );
                    tokio::time::sleep(delay).await;
                    match open(&self.url, &self.tls).await {
                        Ok(socket) => break socket,
                        Err(err) => error = err,
                    }
                };
                tracing::debug!(series_id, "Series Events reconnected");
            }
        }
    }
}

/// Opens a WebSocket to `url` (which carries the API key).
async fn open(url: &Url, tls: &Arc<rustls::ClientConfig>) -> Result<Socket> {
    let connector = Connector::Rustls(Arc::clone(tls));
    let (socket, _response) = tokio_tungstenite::connect_async_tls_with_config(
        url.as_str(),
        None,
        false,
        Some(connector),
    )
    .await
    .map_err(ws_error)?;
    Ok(socket)
}

/// The rustls configuration for `wss://` connections, built once per
/// process with the aws-lc-rs provider and the platform verifier. An explicit
/// provider avoids rustls' process-wide default, which panics when another
/// crate in the build also enables the `ring` provider.
fn tls_config() -> Result<Arc<rustls::ClientConfig>> {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    if let Some(config) = CONFIG.get() {
        return Ok(Arc::clone(config));
    }
    let tls_error = |err: rustls::Error| ws_error(tungstenite::Error::Tls(err.into()));
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(tls_error)?;
    let config = rustls_platform_verifier::BuilderVerifierExt::with_platform_verifier(config)
        .map_err(tls_error)?
        .with_no_client_auth();
    Ok(Arc::clone(CONFIG.get_or_init(|| Arc::new(config))))
}

/// Wraps a tungstenite error, redacting the API key from any URL it carries.
fn ws_error(mut err: tungstenite::Error) -> Error {
    if let tungstenite::Error::Url(UrlError::UnableToConnect(uri)) = &mut err {
        *uri = match Url::parse(uri) {
            Ok(mut url) => {
                redact_url(&mut url);
                url.into()
            }
            Err(_) => "[REDACTED]".to_owned(),
        };
    }
    Error::WebSocket(Box::new(err))
}

/// Whether a reconnection error should end the stream at once: a handshake
/// rejected with a client error other than `429`.
fn is_fatal(error: &Error) -> bool {
    error
        .status()
        .is_some_and(|status| status.is_client_error() && status.as_u16() != 429)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_the_key_from_connect_errors() {
        let err = ws_error(tungstenite::Error::Url(UrlError::UnableToConnect(
            "wss://api.grid.gg/live-data-feed/series/1?key=s3cr3t".into(),
        )));
        let source = std::error::Error::source(&err).unwrap().to_string();
        assert!(!format!("{err:?}{source}").contains("s3cr3t"));
        assert!(source.contains("key=REDACTED"), "{source}");
    }

    #[test]
    fn builds_the_tls_config() {
        assert!(tls_config().is_ok());
    }
}
