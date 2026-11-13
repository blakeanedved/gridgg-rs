//! # gridgg
//!
//! An async Rust SDK for the [GRID](https://grid.gg) esports data APIs.
//!
//! | API | Handle | Protocol |
//! |---|---|---|
//! | Central Data | [`Client::central`] | GraphQL |
//! | Series State | [`Client::series_state`] | GraphQL |
//! | Stats Feed | [`Client::stats`] | GraphQL |
//! | File Download | `Client::files` (feature `files`) | REST |
//! | Series Events | `Client::events` (feature `events`) | WebSocket |
//!
//! Each GraphQL API has prebuilt typed queries (see the [`central`],
//! [`series_state`] and [`stats`] modules) and accepts your own [cynic]
//! operations or raw query strings for anything else; see
//! [Custom queries](#custom-queries). Game-specific data is typed for League
//! of Legends and Valorant; other titles get the fields common to all titles.
//! The SDK runs on tokio; there is no blocking client.
//!
//! ## Quick start
//!
//! ```no_run
//! use gridgg::{Client, Environment};
//!
//! #[tokio::main]
//! async fn main() -> gridgg::Result<()> {
//!     let client = Client::builder()
//!         .api_key(std::env::var("GRID_API_KEY").expect("GRID_API_KEY is not set"))
//!         .environment(Environment::OpenAccess)
//!         .build()?;
//!
//!     // Central Data: the series and its teams.
//!     if let Some(series) = client.central().series("3018394").await? {
//!         println!("{} at {}", series.tournament.name, series.start_time_scheduled);
//!     }
//!
//!     // Series State: the final score.
//!     if let Some(state) = client.series_state().get("3018394").await? {
//!         for team in &state.teams {
//!             println!("{}: {}", team.name, team.score);
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! [`Client`] is cheap to clone; clones share the connection pool, rate
//! limiter and configuration.
//!
//! ## Access tiers
//!
//! [`Environment::OpenAccess`] (the default) and [`Environment::Commercial`]
//! select the hosts and the client-side rate limit (20 or 40 GraphQL
//! requests per minute, see [`RateLimit`]; Open Access Stats Feed requests
//! are further limited to 10 per minute, see
//! [`ClientBuilder::stats_rate_limit`]). The commercial GraphQL hosts are
//! unverified; every base URL can be overridden on [`ClientBuilder`].
//!
//! ## Errors and retries
//!
//! All fallible calls return [`Error`]. A GraphQL response with a non-empty
//! `errors` array becomes [`Error::GraphQl`], with GRID's typed
//! [`ErrorType`] and [`ErrorDetail`] in each error's
//! [`extensions`](GraphQlError::extensions). `429` and `5xx` responses,
//! connection failures, and GraphQL errors that signal throttling or a
//! transient outage are retried according to [`RetryConfig`]. The SDK
//! never logs the API key and redacts it from errors; the `events` module
//! documents one exception in a dependency's `trace` log.
//!
//! ## Custom queries
//!
//! Every GraphQL handle has `raw`, which sends a query string with JSON
//! variables and returns `data` as JSON (see [`Central::raw`](central::Central::raw)),
//! and `run`, which sends a typed [cynic] operation.
//!
//! The schema modules [`central::schema`], [`series_state::schema`] and
//! [`stats::schema`] let you write your own [cynic] queries. cynic also needs
//! the schema registered in *your* crate's build script; the SDL is exported
//! as [`CENTRAL_SDL`], [`SERIES_STATE_SDL`] and [`STATS_SDL`] for that:
//!
//! ```toml
//! [dependencies]
//! gridgg = "0.2"
//! cynic = "3"
//! jiff = "0.2" # for the `Timestamp` scalar below
//!
//! [build-dependencies]
//! gridgg = { version = "0.2", default-features = false }
//! cynic-codegen = "3"
//! ```
//!
//! ```rust,ignore
//! // build.rs
//! fn main() {
//!     cynic_codegen::register_schema("central")
//!         .from_sdl(gridgg::CENTRAL_SDL)
//!         .unwrap();
//! }
//! ```
//!
//! ```no_run
//! #[derive(cynic::QueryVariables)]
//! #[cynic(schema_module = "gridgg::central::schema")]
//! struct Vars {
//!     id: cynic::Id,
//! }
//!
//! #[derive(cynic::QueryFragment, Debug)]
//! #[cynic(
//!     schema = "central",
//!     schema_module = "gridgg::central::schema",
//!     graphql_type = "Query",
//!     variables = "Vars"
//! )]
//! struct SeriesQuery {
//!     #[arguments(id: $id)]
//!     series: Option<Series>,
//! }
//!
//! #[derive(cynic::QueryFragment, Debug)]
//! #[cynic(schema = "central", schema_module = "gridgg::central::schema")]
//! struct Series {
//!     id: cynic::Id,
//!     updated_at: jiff::Timestamp,
//! }
//!
//! # async fn example(client: gridgg::Client) -> gridgg::Result<()> {
//! use cynic::QueryBuilder;
//!
//! let op = SeriesQuery::build(Vars { id: cynic::Id::new("2") });
//! let series = client.central().run(&op).await?.series;
//! # Ok(())
//! # }
//! ```
//!
//! Custom scalars map to the types listed in [`scalars`].
//!
//! ## Known limitations
//!
//! - Open Access keys are denied some Central Data fields, which the
//!   prebuilt queries leave out (see [`central`](central#fields-not-selected)),
//!   and the whole File Download API (`403`).
//! - Series State fields are gated by the series' data-model version, which
//!   can cost two extra requests; data older than version 1.1 is unsupported
//!   (see [`series_state`](series_state#data-model-versions)).
//! - Valorant team and player game statistics came back from the Open Access
//!   Stats Feed as generic types and decode to `Other` (see
//!   [`stats`](stats#titles)).
//! - The commercial endpoints and titles other than League of Legends and
//!   Valorant are unverified.
//!
//! ## Features
//!
//! - `files` (default): the File Download API.
//! - `events` (default): the Series Events WebSocket API.
//! - `zip`: unpacking GRID's zipped event archives (implies `files`); together
//!   with `events`, also into typed Series Events transactions.

pub mod central;
mod client;
mod error;
#[cfg(feature = "events")]
pub mod events;
#[cfg(feature = "files")]
pub mod files;
mod graphql;
pub mod pagination;
mod rate_limit;
mod retry;
pub mod scalars;
pub mod series_state;
pub mod stats;

pub use client::{Client, ClientBuilder, Environment, RateLimit};
pub use error::{
    Error, ErrorDetail, ErrorExtensions, ErrorType, GraphQlError, PathSegment, Result,
};
pub use retry::RetryConfig;

/// GraphQL SDL of the Central Data API (`specs/central-data.graphql`).
///
/// Register it in your build script as schema `"central"` to write custom
/// queries against [`central::schema`]; see [Custom queries](crate#custom-queries).
pub const CENTRAL_SDL: &str = include_str!("../specs/central-data.graphql");

/// GraphQL SDL of the Series State API (`specs/series-state.graphql`).
///
/// Register it in your build script as schema `"series_state"` to write
/// custom queries against [`series_state::schema`]; see
/// [Custom queries](crate#custom-queries).
pub const SERIES_STATE_SDL: &str = include_str!("../specs/series-state.graphql");

/// GraphQL SDL of the Stats Feed API (`specs/stats-feed.graphql`).
///
/// Register it in your build script as schema `"stats"` to write custom
/// queries against [`stats::schema`]; see [Custom queries](crate#custom-queries).
pub const STATS_SDL: &str = include_str!("../specs/stats-feed.graphql");
