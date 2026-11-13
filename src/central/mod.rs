//! Central Data API: titles, tournaments, teams, players, organizations and
//! series.
//!
//! [`Central`] has one prebuilt method per root field of the schema:
//!
//! - single objects by ID, such as [`Central::series`], returning
//!   `Result<Option<T>>`;
//! - lists, such as [`Central::titles`], returning `Result<Vec<T>>`;
//! - paged connections, such as [`Central::all_series`], returning a request
//!   whose [`page`](AllSeriesRequest::page) fetches one [`Page`](crate::pagination::Page)
//!   and whose [`stream`](AllSeriesRequest::stream) walks every page;
//! - external-ID lookups, such as [`Central::series_id_by_external_id`],
//!   returning the GRID ID for a provider's ID. Entities also carry their
//!   [`ExternalLink`]s.
//!
//! ```no_run
//! # async fn example(client: gridgg::Client) -> gridgg::Result<()> {
//! use futures_util::TryStreamExt;
//! use gridgg::central::{SeriesFilter, SeriesType};
//!
//! let series = client.central().series("2").await?;
//!
//! let filter = SeriesFilter::default()
//!     .title_ids(["3"])
//!     .types([SeriesType::Esports]);
//! // Streams are not `Unpin`; pin them to call `try_next`.
//! let mut all = std::pin::pin!(client.central().all_series(filter).stream());
//! while let Some(series) = all.try_next().await? {
//!     println!("{} at {}", series.id.inner(), series.start_time_scheduled);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The fields each result type selects are listed on the type. For anything
//! else, send your own query with [`Central::run`] (typed [cynic] operations
//! built against [`schema`]) or [`Central::raw`] (query string and JSON
//! variables).
//!
//! Enums received from the server are `#[non_exhaustive]` and decode values
//! this version does not know into an `Other(String)` variant.
//!
//! # Fields not selected
//!
//! A denied field adds an error to the response, which fails the whole call
//! with [`Error::GraphQl`](crate::Error::GraphQl). These fields are not
//! selected because Open Access keys get `PERMISSION_DENIED`; use
//! [`Central::run`] with a commercial key:
//!
//! - `Title.logoUrl`
//! - `Team.rating`
//! - `Player.fullName`, `Player.age`, `Player.imageUrl` and
//!   `Player.nationality` (so [`Nationality`] is only for your own queries)

mod filters;
mod queries;
mod types;

pub use filters::{
    BooleanFilter, DateTimeFilter, GameFilter, IdFilter, IntFilter, MapFilter, NationalityFilter,
    NullableDateFilter, NullableIdFilter, OrderDirection, OrganizationFilter, PlayerFilter,
    PlayerPlayerRoleFilter, PlayerRoleFilter, PlayerRoleTitleFilter, ProductServiceLevelFilter,
    SeriesFilter, SeriesLiveFilter, SeriesOrderBy, SeriesPlayerFilter, SeriesTournamentFilter,
    SeriesTournamentTitleFilter, StringFilter, TeamFilter, TitleFilter, TournamentFilter,
    TournamentTitleFilter,
};
pub use queries::{
    AllSeriesRequest, OrganizationsRequest, PlayersRequest, TeamsRequest, TournamentsRequest,
};
pub use types::{
    Asset, AssetType, DataProvider, ExternalEntity, ExternalLink, Money, Nationality, Organization,
    OrganizationRef, OrganizationTeam, Player, PlayerRef, PlayerRole, ProductServiceLevel, Series,
    SeriesFormat, SeriesType, SeriesWorkflowStatus, ServiceLevel, Team, TeamParticipant, TeamRef,
    TeamWorkflowStatus, Title, Tournament, TournamentRef, TournamentVenueType, VideoStream,
};

/// The cynic schema module generated from `specs/central-data.graphql`.
///
/// Use it as `schema_module = "gridgg::central::schema"` with
/// `schema = "central"` in your own cynic derives; see the
/// [crate documentation](crate#custom-queries).
#[cynic::schema("central")]
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod schema {}

crate::graphql::graphql_handle!(
    /// Handle for the Central Data API; see [`Client::central`](crate::Client::central)
    /// and the [module documentation](self).
    Central,
    crate::graphql::Api::Central
);
