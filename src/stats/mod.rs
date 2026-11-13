//! Stats Feed API: aggregated team, player, series and game statistics.
//!
//! [`Stats`] has one prebuilt method per statistics root field:
//!
//! | Method | Root field | Aggregates | Returns |
//! |---|---|---|---|
//! | [`Stats::team_statistics`] | `teamStatistics` | a team's series | [`TeamStatistics`] |
//! | [`Stats::team_game_statistics`] | `teamGameStatistics` | a selection of a team's games | [`TeamGameStatistics`] |
//! | [`Stats::player_statistics`] | `playerStatistics` | a player's series | [`PlayerStatistics`] |
//! | [`Stats::game_statistics`] | `gameStatistics` | a title's games | [`GameStatistics`] |
//! | [`Stats::series_statistics`] | `seriesStatistics` | a title's series | [`SeriesStatistics`] |
//!
//! ```no_run
//! # async fn example(client: gridgg::Client) -> gridgg::Result<()> {
//! use gridgg::stats::{DateTimeFilter, TeamGameStatisticsDetails, TeamStatisticsFilter, TimeRangeFilter};
//!
//! let filter = TeamStatisticsFilter::default()
//!     .started_at(DateTimeFilter::period(TimeRangeFilter::Last3Months));
//! let stats = client.stats().team_statistics("47351", filter).await?;
//! println!(
//!     "{} series, {:.1} kills per game",
//!     stats.series.count, stats.game.kills.avg
//! );
//! if let TeamGameStatisticsDetails::Lol(lol) = &stats.game.details {
//!     println!("vision score per game {:.1}", lol.vision_score.avg);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Numbers are aggregated into [`AggregateIntStatistic`]s and
//! [`AggregateFloatStatistic`]s (sum, min, max, average and rate per minute),
//! durations into [`DurationStatistic`]s, and boolean outcomes such as wins
//! into lists of [`BooleanOccurrenceStatistic`]s (one per value, with count,
//! percentage and streaks).
//!
//! For anything not selected, send your own query with [`Stats::run`] (typed
//! [cynic] operations built against [`schema`]) or [`Stats::raw`] (query
//! string and JSON variables).
//!
//! # Titles
//!
//! Most statistics levels are GraphQL interfaces with one implementation per
//! title. Each Rust type holds the interface's common fields; where the
//! schema has League of Legends or Valorant implementations, it also has a
//! `#[non_exhaustive]` `details` enum selected by `__typename`:
//!
//! | Type | `details` variants |
//! |---|---|
//! | [`TeamGameStatistics`] | [`Lol`](TeamGameStatisticsDetails::Lol), [`Valorant`](TeamGameStatisticsDetails::Valorant), [`Other`](TeamGameStatisticsDetails::Other) |
//! | [`PlayerGameStatistics`] | [`Lol`](PlayerGameStatisticsDetails::Lol), [`Valorant`](PlayerGameStatisticsDetails::Valorant), [`Other`](PlayerGameStatisticsDetails::Other) |
//! | [`GameTeamsStatisticsByGame`] | [`Lol`](GameTeamsStatisticsByGameDetails::Lol), [`Valorant`](GameTeamsStatisticsByGameDetails::Valorant), [`Other`](GameTeamsStatisticsByGameDetails::Other) |
//! | [`PlayerSeriesStatistics`] | [`Valorant`](PlayerSeriesStatisticsDetails::Valorant), [`Other`](PlayerSeriesStatisticsDetails::Other) |
//!
//! The schema has no League of Legends player series type, so
//! [`PlayerSeriesStatisticsDetails`] has no `Lol` variant. It has neither
//! League of Legends nor Valorant types for team series statistics
//! ([`TeamSeriesStatistics`]) or segment statistics
//! ([`TeamSegmentStatistics`], [`PlayerSegmentStatistics`]), so those carry
//! only the common fields and no `details`; so does
//! [`GameTeamsStatisticsBySeries`], for the reasons given on it. `Other` covers every other title
//! and carries no fields: the common fields are always on the containing
//! struct.
//!
//! The schema's `implements` clauses were inferred from GRID's
//! documentation, so the `__typename` values are checked by the ignored live
//! tests in `tests/stats.rs`. On the Open Access API (checked 2026-11-12),
//! League of Legends returns the `Lol` types at every game level. Valorant
//! returns [`PlayerSeriesStatisticsDetails::Valorant`] and
//! [`GameTeamsStatisticsByGameDetails::Valorant`], but team and player game
//! statistics came back as the generic `Default*` types, so they decode to
//! `Other` and their Valorant damage fields are not available.

mod filters;
mod queries;
mod types;

pub use filters::{
    BooleanFilter, CharacterFilter, DateTimeFilter, GameOrder, GameOrderField,
    GamePlayerStateFilter, GameSelection, GameStateFilter, GameStatisticsFilter,
    GameStatisticsTournamentFilter, GameStatisticsVersionFilter, GameTeamStateFilter, IdFilter,
    MapFilter, ObjectiveFilter, OrderDirection, PlayerStatisticsFilter,
    PlayerStatisticsTournamentFilter, SegmentStateFilter, SeriesStatisticsFilter,
    SeriesStatisticsTournamentFilter, StringFilter, TeamGameStatisticsTournamentFilter,
    TeamStatisticsFilter, TeamStatisticsTournamentFilter, TimeRangeFilter, TitleVersionFilter,
};
pub use types::{
    AggregateFloatStatistic, AggregateIntStatistic, BooleanOccurrenceStatistic, Character,
    CharacterOccurrenceStatistic, DraftActionStatistics, Draftable, DurationStatistic,
    GameStatistics, GameStatisticsByGame, GameStatisticsBySeries, GameTeamsStatisticsByGame,
    GameTeamsStatisticsByGameDetails, GameTeamsStatisticsByGameLol,
    GameTeamsStatisticsByGameValorant, GameTeamsStatisticsBySeries, Map, MapOccurrenceStatistic,
    ObjectiveStatistics, OccurrenceStreakStatistic, PlayerGameStatistics,
    PlayerGameStatisticsByTeam, PlayerGameStatisticsDetails, PlayerGameStatisticsLol,
    PlayerGameStatisticsValorant, PlayerSegmentStatistics, PlayerSeriesStatistics,
    PlayerSeriesStatisticsDetails, PlayerSeriesStatisticsValorant, PlayerStatistics, RateStatistic,
    SegmentStatisticsByGame, SegmentStatisticsBySeries, SeriesStatistics, SeriesStatisticsBySeries,
    StringOccurrenceStatistic, TeamGameStatistics, TeamGameStatisticsDetails,
    TeamGameStatisticsLol, TeamGameStatisticsValorant, TeamPlayersStatisticsByGame,
    TeamSegmentStatistics, TeamSeriesStatistics, TeamStatistics, UnitKillStatistic,
};

/// The cynic schema module generated from `specs/stats-feed.graphql`.
///
/// Use it as `schema_module = "gridgg::stats::schema"` with
/// `schema = "stats"` in your own cynic derives; see the
/// [crate documentation](crate#custom-queries).
#[cynic::schema("stats")]
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod schema {}

crate::graphql::graphql_handle!(
    /// Handle for the Stats Feed API; see [`Client::stats`](crate::Client::stats)
    /// and the [module documentation](self).
    Stats,
    crate::graphql::Api::Stats
);
