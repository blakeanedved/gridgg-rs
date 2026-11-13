//! File Download API: post-series files (feature `files`).
//!
//! [`Files::list`] reports which files exist for a series; the download
//! methods, one per GRID operation, return a [`Download`] whose body can be
//! buffered, streamed or saved to disk.
//!
//! Requests authenticate with the `x-api-key` header, so the key never
//! appears in a URL. Failed requests are retried per
//! [`RetryConfig`](crate::RetryConfig) until a successful status arrives;
//! the body itself is never retried. The GraphQL rate limit does not apply.
//!
//! A redirect to another origin (for example a pre-signed CDN URL) is
//! followed once, **without** the `x-api-key` header. Same-origin redirects
//! are followed by the HTTP client. With a custom
//! [`http_client`](crate::ClientBuilder::http_client), redirects follow that
//! client's policy instead.
//!
//! With the `zip` feature, [`Download::events_archive`] and
//! [`parse_events_archive`] hold the whole archive and every decoded value
//! in memory, with no size limit; use [`Download::save_to`] or
//! [`Download::bytes_stream`] for files you cannot afford to buffer.
//!
//! **Unverified live:** an Open Access key is refused by this API
//! (`403 request is forbidden`), so this module is tested only against a
//! mock server built from GRID's published spec.

use cynic::Id;
use reqwest::StatusCode;
use reqwest::header::LOCATION;
use serde::Deserialize;
use url::Url;

use crate::error::{Error, Result};
use crate::retry;

mod download;

pub use download::Download;
#[cfg(feature = "zip")]
pub use download::parse_events_archive;

/// Handle for the File Download API; see [`Client::files`](crate::Client::files).
#[derive(Debug, Clone)]
pub struct Files {
    client: crate::Client,
}

impl Files {
    pub(crate) fn new(client: crate::Client) -> Self {
        Self { client }
    }

    /// The base URL file requests are sent to.
    #[must_use]
    pub fn base_url(&self) -> &Url {
        &self.client.inner.files_url
    }

    /// Lists the files available for a series (`GET /list/{series_id}`).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Api`] or [`Error::Http`] for an unsuccessful status,
    /// [`Error::RateLimited`] if `429` persists through the retries,
    /// [`Error::Decode`] if the body is not the documented JSON,
    /// [`Error::Transport`] for network failures, and [`Error::Config`] for
    /// an empty series ID.
    pub async fn list(&self, series_id: impl Into<Id>) -> Result<Vec<FileInfo>> {
        #[derive(Deserialize)]
        struct FileList {
            #[serde(default)]
            files: Vec<FileInfo>,
        }

        let series_id = series_id.into();
        let response = self.send(&["list", segment(&series_id)?]).await?;
        let body = response.bytes().await.map_err(Error::transport)?;
        let list: FileList = serde_json::from_slice(&body).map_err(Error::Decode)?;
        Ok(list.files)
    }

    /// Downloads the zipped GRID Series Events JSONL file
    /// (`GET /events/grid/series/{series_id}`, `application/zip`).
    ///
    /// With the `zip` feature, [`Download::events_archive`] unpacks it.
    ///
    /// # Errors
    ///
    /// As for [`list`](Self::list), except that the body is not decoded.
    pub async fn grid_events(&self, series_id: impl Into<Id>) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        self.download(
            &["events", "grid", "series", id],
            &format!("{id}-events-grid.zip"),
        )
        .await
    }

    /// Downloads the GRID Series End State JSON file
    /// (`GET /end-state/grid/series/{series_id}`).
    ///
    /// # Errors
    ///
    /// As for [`grid_events`](Self::grid_events).
    pub async fn grid_end_state(&self, series_id: impl Into<Id>) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        self.download(
            &["end-state", "grid", "series", id],
            &format!("{id}-state-grid.json"),
        )
        .await
    }

    /// Downloads the zipped Riot Series Events JSONL file
    /// (`GET /events/riot/series/{series_id}`, `application/zip`).
    ///
    /// # Errors
    ///
    /// As for [`grid_events`](Self::grid_events).
    pub async fn riot_events(&self, series_id: impl Into<Id>) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        self.download(
            &["events", "riot", "series", id],
            &format!("{id}-events-riot.zip"),
        )
        .await
    }

    /// Downloads the Riot events JSONL file of one game
    /// (`GET /events/riot/series/{series_id}/games/{sequence_number}`,
    /// `application/json-lines`).
    ///
    /// # Errors
    ///
    /// As for [`grid_events`](Self::grid_events).
    pub async fn riot_game_events(
        &self,
        series_id: impl Into<Id>,
        sequence_number: u32,
    ) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        let game = sequence_number.to_string();
        self.download(
            &["events", "riot", "series", id, "games", &game],
            &format!("{id}-events-riot-game-{game}.jsonl"),
        )
        .await
    }

    /// Downloads the zipped Riot Series End State JSON file
    /// (`GET /end-state/riot/series/{series_id}`, `application/zip`).
    ///
    /// # Errors
    ///
    /// As for [`grid_events`](Self::grid_events).
    pub async fn riot_end_state(&self, series_id: impl Into<Id>) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        self.download(
            &["end-state", "riot", "series", id],
            &format!("{id}-state-riot.zip"),
        )
        .await
    }

    /// Downloads the Riot end state JSON file of one game
    /// (`GET /end-state/riot/series/{series_id}/games/{sequence_number}/{info_type}`).
    ///
    /// # Errors
    ///
    /// As for [`grid_events`](Self::grid_events).
    pub async fn riot_game_state(
        &self,
        series_id: impl Into<Id>,
        sequence_number: u32,
        info_type: InfoType,
    ) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        let game = sequence_number.to_string();
        let info = info_type.as_str();
        self.download(
            &["end-state", "riot", "series", id, "games", &game, info],
            &format!("{id}-state-{info}-riot-game-{game}.json"),
        )
        .await
    }

    /// Downloads the Riot replay file of one game
    /// (`GET /replay/riot/series/{series_id}/games/{sequence_number}`,
    /// `application/octet-stream`).
    ///
    /// # Errors
    ///
    /// As for [`grid_events`](Self::grid_events).
    pub async fn riot_game_replay(
        &self,
        series_id: impl Into<Id>,
        sequence_number: u32,
    ) -> Result<Download> {
        let id = series_id.into();
        let id = segment(&id)?;
        let game = sequence_number.to_string();
        self.download(
            &["replay", "riot", "series", id, "games", &game],
            &format!("{id}-replay-riot-game-{game}"),
        )
        .await
    }

    async fn download(&self, segments: &[&str], fallback_name: &str) -> Result<Download> {
        let response = self.send(segments).await?;
        Ok(Download::new(response, fallback_name))
    }

    /// Sends a `GET` to the base URL plus `segments` (percent-encoded),
    /// retrying until a successful status, and returns the unread response.
    async fn send(&self, segments: &[&str]) -> Result<reqwest::Response> {
        let inner = &*self.client.inner;
        let mut url = inner.files_url.clone();
        url.path_segments_mut()
            .map_err(|()| Error::Config("files_url cannot be a base URL".into()))?
            .pop_if_empty()
            .extend(segments);

        let attempts = inner.retry.attempts();
        let mut attempt = 0;
        loop {
            attempt += 1;
            tracing::debug!(path = url.path(), attempt, "sending File Download request");
            let sent = inner
                .http
                .get(url.clone())
                .header("x-api-key", crate::graphql::api_key_header(inner)?)
                .send()
                .await;
            let response = match sent {
                Ok(response) => response,
                Err(err) if attempt < attempts && retry::is_retryable_error(&err) => {
                    let delay = inner.retry.delay(attempt, None);
                    tracing::warn!(
                        attempt,
                        delay_ms = delay.as_millis(),
                        error = ?Error::transport(err),
                        "File Download request failed; retrying"
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                Err(err) => return Err(Error::transport(err)),
            };

            let status = response.status();
            if status.is_success() {
                return Ok(response);
            }
            if status.is_redirection()
                && let Some(target) = cross_origin_location(&response)
            {
                drop(response);
                return self.follow_without_key(target).await;
            }
            let retry_after = retry::retry_after(response.headers());
            if attempt < attempts && retry::is_retryable_status(status) {
                let delay = inner.retry.delay(attempt, retry_after);
                tracing::warn!(
                    attempt,
                    status = status.as_u16(),
                    delay_ms = delay.as_millis(),
                    "File Download request rejected; retrying"
                );
                drop(response);
                tokio::time::sleep(delay).await;
                continue;
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                return Err(Error::RateLimited {
                    attempts: attempt,
                    retry_after,
                });
            }
            let body = response.bytes().await.unwrap_or_default();
            return Err(api_error(status, &body));
        }
    }
}

impl Files {
    /// Fetches a cross-origin redirect target once, without the API key.
    async fn follow_without_key(&self, url: Url) -> Result<reqwest::Response> {
        tracing::debug!(
            host = url.host_str(),
            "following cross-origin File Download redirect without the API key"
        );
        let response = self
            .client
            .inner
            .http
            .get(url)
            .send()
            .await
            .map_err(Error::transport)?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let body = response.bytes().await.unwrap_or_default();
        Err(api_error(status, &body))
    }
}

/// The `Location` of a redirect response, resolved against the response URL,
/// if it is an `http`/`https` URL on a different origin.
fn cross_origin_location(response: &reqwest::Response) -> Option<Url> {
    let location = response.headers().get(LOCATION)?.to_str().ok()?;
    let target = response.url().join(location).ok()?;
    let web = matches!(target.scheme(), "http" | "https");
    (web && target.origin() != response.url().origin()).then_some(target)
}

/// Maps an unsuccessful response to [`Error::Api`] if its body is GRID's
/// `ApiErrorMessage`, else to [`Error::Http`].
fn api_error(status: StatusCode, body: &[u8]) -> Error {
    #[derive(Deserialize)]
    struct ApiErrorMessage {
        message: String,
    }

    match serde_json::from_slice::<ApiErrorMessage>(body) {
        Ok(error) => Error::Api {
            status,
            message: error.message,
        },
        Err(_) => Error::http(status, body),
    }
}

/// The series ID as a path segment; empty, `.` and `..` are rejected because
/// they would change the request path.
fn segment(id: &Id) -> Result<&str> {
    match id.inner() {
        "" | "." | ".." => Err(Error::Config(format!("invalid series ID {:?}", id.inner()))),
        id => Ok(id),
    }
}

/// One entry of [`Files::list`].
///
/// The spec marks no field as required; `id` and `status` are required here
/// because an entry is unusable without them, the rest are optional.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FileInfo {
    /// The file's kind, for example `events-grid-compressed`, `state-grid`,
    /// `events-riot-compressed`, `state-riot-compressed`,
    /// `events-riot-game-{n}`, `state-summary-riot-game-{n}`,
    /// `state-details-riot-game-{n}` or `replay-riot-game-{n}`.
    pub id: String,
    /// A short description of the file.
    #[serde(default)]
    pub description: Option<String>,
    /// The state of the series the file belongs to.
    pub status: FileStatus,
    /// The file's name.
    #[serde(default)]
    pub file_name: Option<String>,
    /// The file's download URL.
    #[serde(default, rename = "fullURL")]
    pub full_url: Option<String>,
}

crate::error::string_enum! {
    /// The [`status`](FileInfo::status) of a listed file: the state of the
    /// series it refers to.
    FileStatus {
        /// The match has not started.
        MatchNotStarted = "match-not-started",
        /// The match is in progress.
        MatchInProgress = "match-in-progress",
        /// The file is being produced.
        Processing = "processing",
        /// The file can be downloaded.
        Ready = "ready",
        /// The file is not available.
        FileNotAvailable = "file-not-available",
    }
}

/// Which Riot game end state file [`Files::riot_game_state`] downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InfoType {
    /// The summary file (`summary`).
    Summary,
    /// The details file (`details`).
    Details,
}

impl InfoType {
    /// The value as sent in the request path.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Details => "details",
        }
    }
}
