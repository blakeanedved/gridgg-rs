//! Series State API: the live and final state of a series.
//!
//! [`SeriesState::get`] fetches a series' state by series ID and
//! [`SeriesState::latest_by_player_id`] the state of a player's latest
//! series. Both return a [`Series`]:
//!
//! ```no_run
//! # async fn example(client: gridgg::Client) -> gridgg::Result<()> {
//! use gridgg::series_state::GamePlayerDetails;
//!
//! if let Some(series) = client.series_state().get("2").await? {
//!     println!("data model version {}", series.version);
//!     for game in &series.games {
//!         for team in &game.teams {
//!             for player in &team.players {
//!                 print!("{}: {}/{}", player.name, player.kills, player.deaths);
//!                 if let GamePlayerDetails::Lol(lol) = &player.details {
//!                     print!(", vision score {:?}", lol.vision_score);
//!                 }
//!                 println!();
//!             }
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! For anything not selected, send your own query with [`SeriesState::run`]
//! (typed [cynic] operations built against [`schema`]) or
//! [`SeriesState::raw`] (query string and JSON variables).
//!
//! # Titles
//!
//! Teams and players are GraphQL interfaces with one implementation per
//! title. Each Rust type holds the interface's common fields, plus a
//! `#[non_exhaustive]` `details` enum selected by `__typename`:
//!
//! | Type | `details` variants |
//! |---|---|
//! | [`GameTeam`] | [`Lol`](GameTeamDetails::Lol), [`Valorant`](GameTeamDetails::Valorant), [`Other`](GameTeamDetails::Other) |
//! | [`GamePlayer`] | [`Lol`](GamePlayerDetails::Lol), [`Valorant`](GamePlayerDetails::Valorant), [`Other`](GamePlayerDetails::Other) |
//! | [`SeriesTeam`] | [`Valorant`](SeriesTeamDetails::Valorant), [`Other`](SeriesTeamDetails::Other) |
//! | [`SeriesPlayer`] | [`Valorant`](SeriesPlayerDetails::Valorant), [`Other`](SeriesPlayerDetails::Other) |
//! | [`SegmentTeam`] | [`Valorant`](SegmentTeamDetails::Valorant), [`Other`](SegmentTeamDetails::Other) |
//! | [`SegmentPlayer`] | [`Valorant`](SegmentPlayerDetails::Valorant), [`Other`](SegmentPlayerDetails::Other) |
//!
//! The schema has no League of Legends type for series and segment teams and
//! players, so those enums have no `Lol` variant. `Other` covers every other
//! title and carries no fields: the common fields are always on the
//! containing struct. The schema's `implements` clauses were inferred from
//! GRID's documentation, so the `__typename` values are checked by the
//! ignored live tests in `tests/series_state.rs`.
//!
//! # Segments
//!
//! Segments (the `SegmentState` type, "e.g. a round") nest recursively in
//! the schema. The prebuilt query selects a fixed depth: each [`Game`]'s
//! [`segments`](Game::segments) (Valorant rounds sit at this level), and in
//! each [`Segment`] one more level of [`SubSegment`]s for titles that
//! nest segments further. Deeper levels are not selected; query them with
//! [`SeriesState::run`].
//!
//! # Data-model versions
//!
//! [`Series::version`] is the data-model version the series was recorded
//! with. Fields documented "(from version X)" only exist in data of that
//! version or newer and are `Option` here, even where the schema marks them
//! non-null; they are `None` for older data.
//!
//! GRID rejects the whole request (errors `Requested field is only
//! available from version X`, typed `BAD_REQUEST`) when it selects a field
//! newer than the series' version. The prebuilt queries therefore wrap each
//! such field in `@include(if: $vX_Y)`: [`SeriesState::get`] and
//! [`SeriesState::latest_by_player_id`] first include every field (one
//! request for current data) and, on `BAD_REQUEST`, look up the version and
//! resend without the newer fields (three requests). Queries sent with
//! [`SeriesState::run`] or [`SeriesState::raw`] must do their own gating.
//!
//! The schema also gates some types (several title-specific
//! implementations, from version 1.1); inline fragments on them are not
//! gated, so data older than version 1.1 is unsupported. Fields
//! "available until" a version are not selected.

mod queries;
mod types;

pub use queries::SeriesStateVariables;
pub use types::{
    BaronPowerPlay, Character, Clock, Coordinates, DataProvider, DraftAction, Draftable, Drafter,
    ExternalEntity, ExternalLink, Game, GameInvalidationReason, GamePlayer, GamePlayerDetails,
    GamePlayerLol, GamePlayerValorant, GameTeam, GameTeamDetails, GameTeamLol, GameTeamValorant,
    GameType, Inventory, ItemStack, Map, NonPlayerCharacter, Objective, ParticipationStatus,
    PlayerRole, Segment, SegmentPlayer, SegmentPlayerDetails, SegmentPlayerValorant, SegmentTeam,
    SegmentTeamDetails, SegmentTeamValorant, Series, SeriesInvalidationReason, SeriesPlayer,
    SeriesPlayerDetails, SeriesPlayerValorant, SeriesTeam, SeriesTeamDetails, SeriesTeamValorant,
    Structure, SubSegment, Title, TitleVersion,
};

/// The cynic schema module generated from `specs/series-state.graphql`.
///
/// Use it as `schema_module = "gridgg::series_state::schema"` with
/// `schema = "series_state"` in your own cynic derives; see the
/// [crate documentation](crate#custom-queries).
#[cynic::schema("series_state")]
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod schema {}

crate::graphql::graphql_handle!(
    /// Handle for the Series State API; see
    /// [`Client::series_state`](crate::Client::series_state) and the
    /// [module documentation](self).
    SeriesState,
    crate::graphql::Api::SeriesState
);
