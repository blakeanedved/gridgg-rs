//! Result types of the prebuilt Stats Feed queries.
//!
//! Each statistics level is a struct holding the fields of the schema's
//! common interface. Where the schema has League of Legends or Valorant
//! implementations with extra fields, the struct also has a `details` enum
//! with those fields; see the [module documentation](super#titles).
//!
//! Fields the schema marks as deprecated (such as `wins`, replaced by `won`)
//! are not selected.

use cynic::Id;
use jiff::Span;

// ---------------------------------------------------------------------------
// Aggregates

/// Aggregated integer values (the `AggregateIntStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "AggregateIntStatistic"
)]
#[non_exhaustive]
pub struct AggregateIntStatistic {
    /// The sum of all values.
    pub sum: i32,
    /// The smallest value.
    pub min: i32,
    /// The largest value.
    pub max: i32,
    /// The average value.
    pub avg: f64,
    /// The values per minute of game time.
    pub rate_per_minute: RateStatistic,
}

/// Aggregated floating-point values (the `AggregateFloatStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "AggregateFloatStatistic"
)]
#[non_exhaustive]
pub struct AggregateFloatStatistic {
    /// The sum of all values.
    pub sum: f64,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
    /// The average value.
    pub avg: f64,
    /// The values per minute of game time.
    pub rate_per_minute: RateStatistic,
}

/// Aggregated rates per minute (the `RateStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "RateStatistic"
)]
#[non_exhaustive]
pub struct RateStatistic {
    /// The smallest rate.
    pub min: f64,
    /// The largest rate.
    pub max: f64,
    /// The average rate.
    pub avg: f64,
}

/// Aggregated durations (the `DurationStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "DurationStatistic"
)]
#[non_exhaustive]
pub struct DurationStatistic {
    /// The total duration.
    pub sum: Span,
    /// The shortest duration.
    pub min: Span,
    /// The longest duration.
    pub max: Span,
    /// The average duration.
    pub avg: Span,
}

/// How often a boolean data point had a value (the
/// `BooleanOccurrenceStatistic` type), for example how often a team got the
/// first kill (`value: true`) or did not (`value: false`).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "BooleanOccurrenceStatistic"
)]
#[non_exhaustive]
pub struct BooleanOccurrenceStatistic {
    /// The value counted.
    pub value: bool,
    /// The number of occurrences of the value.
    pub count: i32,
    /// The share of occurrences with the value, as a percentage.
    pub percentage: f64,
    /// Streaks of consecutive occurrences of the value.
    pub streak: OccurrenceStreakStatistic,
}

/// Streaks of consecutive occurrences (the `OccurrenceStreakStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "OccurrenceStreakStatistic"
)]
#[non_exhaustive]
pub struct OccurrenceStreakStatistic {
    /// The current streak.
    pub current: i32,
    /// The longest streak.
    pub max: i32,
    /// The shortest streak.
    pub min: i32,
}

/// How often a string data point had a value (the
/// `StringOccurrenceStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "StringOccurrenceStatistic"
)]
#[non_exhaustive]
pub struct StringOccurrenceStatistic {
    /// The value counted.
    pub value: String,
    /// The number of occurrences of the value.
    pub count: i32,
    /// The share of occurrences with the value, as a percentage.
    pub percentage: f64,
}

/// Aggregated objectives of one type (the `ObjectiveStatistics` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "ObjectiveStatistics"
)]
#[non_exhaustive]
pub struct ObjectiveStatistics {
    /// The type of objective, such as `destroyTower`.
    #[cynic(rename = "type")]
    pub objective_type: String,
    /// How often the objective was completed.
    pub completion_count: AggregateIntStatistic,
    /// How often the objective was completed first, if reported.
    pub completed_first: Option<Vec<BooleanOccurrenceStatistic>>,
}

/// Aggregated kills of one unit type (the `UnitKillStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "UnitKillStatistic"
)]
#[non_exhaustive]
pub struct UnitKillStatistic {
    /// The name of the unit.
    pub unit_name: String,
    /// The number of units killed.
    pub count: AggregateIntStatistic,
}

/// A game character (the `Character` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Character"
)]
#[non_exhaustive]
pub struct Character {
    /// The character ID.
    pub id: Id,
    /// The character name.
    pub name: String,
}

/// How often a character was played (the `CharacterOccurrenceStatistic`
/// type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "CharacterOccurrenceStatistic"
)]
#[non_exhaustive]
pub struct CharacterOccurrenceStatistic {
    /// The character.
    pub character: Character,
    /// The number of times the character was played.
    pub count: i32,
    /// The share of occurrences with this character, as a percentage.
    pub percentage: f64,
}

/// A game map (the `Map` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Map"
)]
#[non_exhaustive]
pub struct Map {
    /// The map ID.
    pub id: Id,
    /// The map name.
    pub name: String,
}

/// How often a map was played (the `MapOccurrenceStatistic` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "MapOccurrenceStatistic"
)]
#[non_exhaustive]
pub struct MapOccurrenceStatistic {
    /// The map.
    pub map: Map,
    /// The number of times the map was played.
    pub count: i32,
    /// The share of occurrences with this map, as a percentage.
    pub percentage: f64,
}

/// An entity that can be drafted, such as a champion or agent (the
/// `Draftable` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Draftable"
)]
#[non_exhaustive]
pub struct Draftable {
    /// The GRID ID of the entity.
    pub id: Id,
    /// The name of the entity.
    pub name: String,
    /// The type of the entity.
    #[cynic(rename = "type")]
    pub draftable_type: String,
}

/// Aggregated draft actions on one entity (the `DraftActionStatistics`
/// type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "DraftActionStatistics"
)]
#[non_exhaustive]
pub struct DraftActionStatistics {
    /// The drafted entity.
    pub draftable: Draftable,
    /// How often each kind of draft action (such as `pick` or `ban`) was
    /// taken on the entity.
    #[cynic(rename = "type")]
    pub types: Vec<StringOccurrenceStatistic>,
}

// ---------------------------------------------------------------------------
// Team statistics

/// Aggregated statistics of a team (the `TeamStatistics` type), returned by
/// [`Stats::team_statistics`](super::Stats::team_statistics).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamStatistics"
)]
#[non_exhaustive]
pub struct TeamStatistics {
    /// The GRID ID of the team.
    pub id: Id,
    /// The IDs of the series the statistics were aggregated from.
    pub aggregation_series_ids: Vec<Id>,
    /// Statistics aggregated per series.
    pub series: TeamSeriesStatistics,
    /// Statistics aggregated per game.
    pub game: TeamGameStatistics,
    /// Statistics aggregated per segment type (such as rounds).
    #[cynic(rename = "segment")]
    pub segments: Vec<TeamSegmentStatistics>,
}

/// A team's statistics aggregated per series (the `TeamSeriesStatistics`
/// interface).
///
/// The schema has no League of Legends or Valorant implementation of this
/// interface, so there is no `details` enum.
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamSeriesStatistics"
)]
#[non_exhaustive]
pub struct TeamSeriesStatistics {
    /// The number of series aggregated.
    pub count: i32,
    /// Series won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// The team's score.
    pub score: AggregateIntStatistic,
    /// The duration of the series' games.
    pub duration: DurationStatistic,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
}

/// A team's statistics aggregated per game (the `TeamGameStatistics`
/// interface), also returned by
/// [`Stats::team_game_statistics`](super::Stats::team_game_statistics).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamGameStatistics"
)]
#[non_exhaustive]
pub struct TeamGameStatistics {
    /// The number of games aggregated.
    pub count: i32,
    /// Games won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// The team's score.
    pub score: AggregateIntStatistic,
    /// The duration of the games.
    pub duration: DurationStatistic,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
    /// Money held.
    pub money: AggregateIntStatistic,
    /// Value of the inventory.
    pub inventory_value: AggregateIntStatistic,
    /// Net worth.
    pub net_worth: AggregateIntStatistic,
    /// Kills of non-player units, per unit.
    pub unit_kills: Vec<UnitKillStatistic>,
    /// Statistics of the team's players.
    pub players: PlayerGameStatisticsByTeam,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: TeamGameStatisticsDetails,
}

/// Title-specific fields of a [`TeamGameStatistics`], chosen by the GraphQL
/// `__typename`.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamGameStatistics"
)]
#[non_exhaustive]
pub enum TeamGameStatisticsDetails {
    /// League of Legends (`TeamGameStatisticsLol`).
    Lol(Box<TeamGameStatisticsLol>),
    /// Valorant (`TeamGameStatisticsVal`).
    Valorant(TeamGameStatisticsValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`TeamGameStatistics`].
    #[cynic(fallback)]
    Other,
}

/// League of Legends fields of a [`TeamGameStatistics`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamGameStatisticsLol"
)]
#[non_exhaustive]
pub struct TeamGameStatisticsLol {
    /// Damage dealt.
    pub damage_dealt: AggregateIntStatistic,
    /// Damage taken.
    pub damage_taken: AggregateIntStatistic,
    /// Experience points earned.
    pub experience_points: AggregateIntStatistic,
    /// Total gold earned.
    pub total_money_earned: AggregateIntStatistic,
    /// Vision score.
    pub vision_score: AggregateFloatStatistic,
    /// Forward percentage (`forwardPercentage`).
    pub forward_percentage: AggregateFloatStatistic,
    /// Major gold lead (`majorMoneyLead`).
    pub major_money_lead: AggregateFloatStatistic,
    /// Major gold deficit (`majorMoneyDeficit`).
    pub major_money_deficit: AggregateFloatStatistic,
    /// Baron power play (`baronPowerPlay`).
    pub baron_power_play: AggregateIntStatistic,
}

/// Valorant fields of a [`TeamGameStatistics`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamGameStatisticsVal"
)]
#[non_exhaustive]
pub struct TeamGameStatisticsValorant {
    /// Damage dealt.
    pub damage_dealt: AggregateIntStatistic,
    /// Damage taken.
    pub damage_taken: AggregateIntStatistic,
}

/// The characters a team's players played, aggregated per game (the
/// `PlayerGameStatisticsByTeam` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerGameStatisticsByTeam"
)]
#[non_exhaustive]
pub struct PlayerGameStatisticsByTeam {
    /// How often each character was played.
    pub characters: Vec<CharacterOccurrenceStatistic>,
}

/// A team's statistics aggregated per segment type, such as rounds (the
/// `TeamSegmentStatistics` interface).
///
/// The schema has no League of Legends or Valorant implementation of this
/// interface, so there is no `details` enum.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamSegmentStatistics"
)]
#[non_exhaustive]
pub struct TeamSegmentStatistics {
    /// The segment type, such as `round`.
    #[cynic(rename = "type")]
    pub segment_type: String,
    /// The number of segments aggregated.
    pub count: i32,
    /// Segments won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// First segments won and lost.
    pub won_first: Vec<BooleanOccurrenceStatistic>,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
}

// ---------------------------------------------------------------------------
// Player statistics

/// Aggregated statistics of a player (the `PlayerStatistics` type), returned
/// by [`Stats::player_statistics`](super::Stats::player_statistics).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerStatistics"
)]
#[non_exhaustive]
pub struct PlayerStatistics {
    /// The GRID ID of the player.
    pub id: Id,
    /// The IDs of the series the statistics were aggregated from.
    pub aggregation_series_ids: Vec<Id>,
    /// Statistics aggregated per series.
    pub series: PlayerSeriesStatistics,
    /// Statistics aggregated per game.
    pub game: PlayerGameStatistics,
    /// Statistics aggregated per segment type (such as rounds).
    #[cynic(rename = "segment")]
    pub segments: Vec<PlayerSegmentStatistics>,
}

/// A player's statistics aggregated per series (the `PlayerSeriesStatistics`
/// interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerSeriesStatistics"
)]
#[non_exhaustive]
pub struct PlayerSeriesStatistics {
    /// The number of series aggregated.
    pub count: i32,
    /// Series won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: PlayerSeriesStatisticsDetails,
}

/// Title-specific fields of a [`PlayerSeriesStatistics`], chosen by the
/// GraphQL `__typename`.
///
/// The schema has no League of Legends implementation of this interface, so
/// there is no `Lol` variant here.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerSeriesStatistics"
)]
#[non_exhaustive]
pub enum PlayerSeriesStatisticsDetails {
    /// Valorant (`ValorantPlayerSeriesStatistics`).
    Valorant(PlayerSeriesStatisticsValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`PlayerSeriesStatistics`].
    #[cynic(fallback)]
    Other,
}

/// Valorant fields of a [`PlayerSeriesStatistics`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "ValorantPlayerSeriesStatistics"
)]
#[non_exhaustive]
pub struct PlayerSeriesStatisticsValorant {
    /// Headshot kills.
    pub headshots: AggregateIntStatistic,
}

/// A player's statistics aggregated per game (the `PlayerGameStatistics`
/// interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerGameStatistics"
)]
#[non_exhaustive]
pub struct PlayerGameStatistics {
    /// The number of games aggregated.
    pub count: i32,
    /// Games won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
    /// Money held.
    pub money: AggregateIntStatistic,
    /// Value of the inventory.
    pub inventory_value: AggregateIntStatistic,
    /// Net worth.
    pub net_worth: AggregateIntStatistic,
    /// Kills of non-player units, per unit.
    pub unit_kills: Vec<UnitKillStatistic>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: PlayerGameStatisticsDetails,
}

/// Title-specific fields of a [`PlayerGameStatistics`], chosen by the
/// GraphQL `__typename`.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerGameStatistics"
)]
#[non_exhaustive]
pub enum PlayerGameStatisticsDetails {
    /// League of Legends (`PlayerGameStatisticsLol`).
    Lol(Box<PlayerGameStatisticsLol>),
    /// Valorant (`PlayerGameStatisticsVal`).
    Valorant(PlayerGameStatisticsValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`PlayerGameStatistics`].
    #[cynic(fallback)]
    Other,
}

/// League of Legends fields of a [`PlayerGameStatistics`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerGameStatisticsLol"
)]
#[non_exhaustive]
pub struct PlayerGameStatisticsLol {
    /// Damage dealt.
    pub damage_dealt: AggregateIntStatistic,
    /// Damage taken.
    pub damage_taken: AggregateIntStatistic,
    /// Experience points earned.
    pub experience_points: AggregateIntStatistic,
    /// Total gold earned.
    pub total_money_earned: AggregateIntStatistic,
    /// Vision score.
    pub vision_score: AggregateFloatStatistic,
    /// Forward percentage (`forwardPercentage`).
    pub forward_percentage: AggregateFloatStatistic,
}

/// Valorant fields of a [`PlayerGameStatistics`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerGameStatisticsVal"
)]
#[non_exhaustive]
pub struct PlayerGameStatisticsValorant {
    /// Damage dealt.
    pub damage_dealt: AggregateIntStatistic,
    /// Damage taken.
    pub damage_taken: AggregateIntStatistic,
}

/// A player's statistics aggregated per segment type, such as rounds (the
/// `PlayerSegmentStatistics` interface).
///
/// The schema has no League of Legends or Valorant implementation of this
/// interface, so there is no `details` enum.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "PlayerSegmentStatistics"
)]
#[non_exhaustive]
pub struct PlayerSegmentStatistics {
    /// The segment type, such as `round`.
    #[cynic(rename = "type")]
    pub segment_type: String,
    /// The number of segments aggregated.
    pub count: i32,
    /// Segments won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
}

// ---------------------------------------------------------------------------
// Game statistics

/// Aggregated statistics of a title's games (the `GameStatistics` type),
/// returned by [`Stats::game_statistics`](super::Stats::game_statistics).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameStatistics"
)]
#[non_exhaustive]
pub struct GameStatistics {
    /// The IDs of the games the statistics were aggregated from.
    pub aggregation_game_ids: Vec<Id>,
    /// The number of games aggregated.
    pub count: i32,
    /// Statistics of the games.
    pub games: GameStatisticsByGame,
    /// Statistics per segment type (such as rounds).
    pub segments: Vec<SegmentStatisticsByGame>,
}

/// Statistics of aggregated games (the `GameStatisticsByGame` type).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameStatisticsByGame"
)]
#[non_exhaustive]
pub struct GameStatisticsByGame {
    /// The duration of the games.
    pub duration: DurationStatistic,
    /// How often each map was played.
    #[cynic(rename = "map")]
    pub maps: Vec<MapOccurrenceStatistic>,
    /// Draft actions per drafted entity.
    pub draft_actions: Vec<DraftActionStatistics>,
    /// Statistics of the teams in the games.
    pub teams: GameTeamsStatisticsByGame,
}

/// Statistics of the teams in aggregated games (the
/// `GameTeamsStatisticsByGame` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameTeamsStatisticsByGame"
)]
#[non_exhaustive]
pub struct GameTeamsStatisticsByGame {
    /// The teams' scores.
    pub score: AggregateIntStatistic,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
    /// Money held.
    pub money: AggregateIntStatistic,
    /// Value of the inventory.
    pub inventory_value: AggregateIntStatistic,
    /// Net worth.
    pub net_worth: AggregateIntStatistic,
    /// Kills of non-player units, per unit.
    pub unit_kills: Vec<UnitKillStatistic>,
    /// The characters the teams' players played.
    pub players: TeamPlayersStatisticsByGame,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: GameTeamsStatisticsByGameDetails,
}

/// Title-specific fields of a [`GameTeamsStatisticsByGame`], chosen by the
/// GraphQL `__typename`.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameTeamsStatisticsByGame"
)]
#[non_exhaustive]
pub enum GameTeamsStatisticsByGameDetails {
    /// League of Legends (`GameTeamsStatisticsByGameLol`).
    Lol(Box<GameTeamsStatisticsByGameLol>),
    /// Valorant (`GameTeamsStatisticsByGameVal`).
    Valorant(GameTeamsStatisticsByGameValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`GameTeamsStatisticsByGame`].
    #[cynic(fallback)]
    Other,
}

/// League of Legends fields of a [`GameTeamsStatisticsByGame`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameTeamsStatisticsByGameLol"
)]
#[non_exhaustive]
pub struct GameTeamsStatisticsByGameLol {
    /// Damage dealt.
    pub damage_dealt: AggregateIntStatistic,
    /// Damage taken.
    pub damage_taken: AggregateIntStatistic,
    /// Experience points earned.
    pub experience_points: AggregateIntStatistic,
    /// Total gold earned.
    pub total_money_earned: AggregateIntStatistic,
    /// Vision score.
    pub vision_score: AggregateFloatStatistic,
    /// Forward percentage (`forwardPercentage`).
    pub forward_percentage: AggregateFloatStatistic,
    /// Major gold lead (`majorMoneyLead`).
    pub major_money_lead: AggregateFloatStatistic,
    /// Major gold deficit (`majorMoneyDeficit`).
    pub major_money_deficit: AggregateFloatStatistic,
    /// Baron power play (`baronPowerPlay`).
    pub baron_power_play: AggregateIntStatistic,
}

/// Valorant fields of a [`GameTeamsStatisticsByGame`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameTeamsStatisticsByGameVal"
)]
#[non_exhaustive]
pub struct GameTeamsStatisticsByGameValorant {
    /// Damage dealt.
    pub damage_dealt: AggregateIntStatistic,
    /// Damage taken.
    pub damage_taken: AggregateIntStatistic,
}

/// The characters played in aggregated games (the
/// `TeamPlayersStatisticsByGame` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamPlayersStatisticsByGame"
)]
#[non_exhaustive]
pub struct TeamPlayersStatisticsByGame {
    /// How often each character was played.
    #[cynic(rename = "character")]
    pub characters: Vec<CharacterOccurrenceStatistic>,
}

/// Segments of one type, such as rounds, aggregated per game (the
/// `SegmentStatisticsByGame` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "SegmentStatisticsByGame"
)]
#[non_exhaustive]
pub struct SegmentStatisticsByGame {
    /// The segment type, such as `round`.
    #[cynic(rename = "type")]
    pub segment_type: String,
    /// The number of segments per game.
    pub count: AggregateIntStatistic,
}

// ---------------------------------------------------------------------------
// Series statistics

/// Aggregated statistics of a title's series (the `SeriesStatistics` type),
/// returned by [`Stats::series_statistics`](super::Stats::series_statistics).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "SeriesStatistics"
)]
#[non_exhaustive]
pub struct SeriesStatistics {
    /// The IDs of the series the statistics were aggregated from.
    pub aggregation_series_ids: Vec<Id>,
    /// The number of series aggregated.
    pub count: i32,
    /// Statistics of the series themselves.
    pub series: SeriesStatisticsBySeries,
    /// Statistics of the series' games.
    pub games: GameStatisticsBySeries,
    /// Statistics per segment type (such as rounds).
    pub segments: Vec<SegmentStatisticsBySeries>,
}

/// Statistics of aggregated series (the `SeriesStatisticsBySeries` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "SeriesStatisticsBySeries"
)]
#[non_exhaustive]
pub struct SeriesStatisticsBySeries {
    /// Draft actions per drafted entity.
    pub draft_actions: Vec<DraftActionStatistics>,
}

/// Statistics of the games of aggregated series (the
/// `GameStatisticsBySeries` type).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "GameStatisticsBySeries"
)]
#[non_exhaustive]
pub struct GameStatisticsBySeries {
    /// The number of games per series.
    pub count: AggregateIntStatistic,
    /// The duration of the games.
    pub duration: DurationStatistic,
    /// How often each map was played.
    #[cynic(rename = "map")]
    pub maps: Vec<MapOccurrenceStatistic>,
    /// Draft actions per drafted entity.
    pub draft_actions: Vec<DraftActionStatistics>,
    /// Statistics of the teams in the games.
    pub teams: GameTeamsStatisticsBySeries,
}

/// The teams' statistics in the games of aggregated series (the
/// `TeamGameStatistics` interface, as selected under
/// `GameStatisticsBySeries.teams`).
///
/// Holds the common fields of [`TeamGameStatistics`] only. At this level,
/// selecting `players` or most of the League of Legends fields (such as
/// `damageTaken` or `visionScore`) made the API fail with an `INTERNAL`
/// error, and Valorant data came back as the generic
/// `DefaultTeamGameStatistics` type (checked 2026-11-12). Title-specific team
/// statistics of a title's games are on
/// [`GameStatisticsByGame::teams`] instead.
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "TeamGameStatistics"
)]
#[non_exhaustive]
pub struct GameTeamsStatisticsBySeries {
    /// The number of games aggregated.
    pub count: i32,
    /// Games won and lost.
    #[cynic(flatten)]
    pub won: Vec<BooleanOccurrenceStatistic>,
    /// The team's score.
    pub score: AggregateIntStatistic,
    /// The duration of the games.
    pub duration: DurationStatistic,
    /// Enemy kills.
    pub kills: AggregateIntStatistic,
    /// Deaths.
    pub deaths: AggregateIntStatistic,
    /// Enemy kill assists given.
    pub kill_assists_given: AggregateIntStatistic,
    /// Enemy kill assists received.
    pub kill_assists_received: AggregateIntStatistic,
    /// Suicides.
    pub selfkills: AggregateIntStatistic,
    /// Kills of teammates.
    pub teamkills: AggregateIntStatistic,
    /// Assists given on kills of teammates.
    pub teamkill_assists_given: AggregateIntStatistic,
    /// Assists received on kills of teammates.
    pub teamkill_assists_received: AggregateIntStatistic,
    /// Structures destroyed.
    pub structures_destroyed: AggregateIntStatistic,
    /// Structures captured.
    pub structures_captured: AggregateIntStatistic,
    /// First kills.
    pub first_kill: Vec<BooleanOccurrenceStatistic>,
    /// Objectives completed.
    pub objectives: Vec<ObjectiveStatistics>,
    /// Money held.
    pub money: AggregateIntStatistic,
    /// Value of the inventory.
    pub inventory_value: AggregateIntStatistic,
    /// Net worth.
    pub net_worth: AggregateIntStatistic,
    /// Kills of non-player units, per unit.
    pub unit_kills: Vec<UnitKillStatistic>,
}

/// Segments of one type, such as rounds, aggregated per series (the
/// `SegmentStatisticsBySeries` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "SegmentStatisticsBySeries"
)]
#[non_exhaustive]
pub struct SegmentStatisticsBySeries {
    /// The segment type, such as `round`.
    #[cynic(rename = "type")]
    pub segment_type: String,
    /// The number of segments per series.
    pub count: AggregateIntStatistic,
}
