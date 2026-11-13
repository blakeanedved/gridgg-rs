//! Result types of the prebuilt Series State queries.
//!
//! Each level of the state tree (series, games, segments, and the teams and
//! players at each of them) is a struct holding the fields of the schema's
//! common interface, plus a `details` enum with the title-specific fields.
//! Fields marked "(from version X)" are `Option`; see the
//! [module documentation](super#data-model-versions).

use cynic::Id;
use jiff::{Span, Timestamp};

use crate::scalars::Version;

// ---------------------------------------------------------------------------
// Series

/// The state of a series (the `SeriesState` type).
///
/// Selects the series' own fields, its title, draft actions, teams (with
/// their players) and games (with their teams, players and segments). See
/// the [module documentation](super) for the selected depth.
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct Series {
    /// The GRID ID of the series.
    pub id: Id,
    /// The version of the data model this state was produced with.
    pub version: Version,
    /// The esports title of the series (from version 1.1).
    #[directives(include(if: $v1_1))]
    pub title: Option<Title>,
    /// The format of the series, such as best of three.
    pub format: String,
    /// Whether the series has started.
    pub started: bool,
    /// Whether the series has finished.
    pub finished: bool,
    /// Whether the series was forfeited (from version 3.48).
    #[directives(include(if: $v3_48))]
    pub forfeited: Option<bool>,
    /// Whether the series data is considered accurate.
    pub valid: bool,
    /// Why the series data is not considered accurate; `None` while it is
    /// valid (from version 3.49).
    #[directives(include(if: $v3_49))]
    pub invalidation_reason: Option<SeriesInvalidationReason>,
    /// When the series started (from version 2.1).
    #[directives(include(if: $v2_1))]
    pub started_at: Option<Timestamp>,
    /// When the series data was last updated.
    pub updated_at: Timestamp,
    /// The duration of the whole series (from version 3.14).
    #[directives(include(if: $v3_14))]
    pub duration: Option<Span>,
    /// The draft actions (picks and bans) of the series.
    pub draft_actions: Vec<DraftAction>,
    /// The participating teams, aggregated over the series.
    pub teams: Vec<SeriesTeam>,
    /// The games of the series.
    pub games: Vec<Game>,
}

/// A team aggregated over a series (the `SeriesTeamState` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesTeamState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SeriesTeam {
    /// The GRID ID of the team.
    pub id: Id,
    /// The name of the team.
    pub name: String,
    /// The team's score in the series.
    pub score: i32,
    /// Whether the team won the series.
    pub won: bool,
    /// Enemy kills.
    pub kills: i32,
    /// Deaths.
    pub deaths: i32,
    /// Enemy kill assists given.
    pub kill_assists_given: i32,
    /// Enemy kill assists received.
    pub kill_assists_received: i32,
    /// Suicides.
    pub selfkills: i32,
    /// Kills of teammates.
    pub teamkills: i32,
    /// Structures destroyed.
    pub structures_destroyed: i32,
    /// Structures captured.
    pub structures_captured: i32,
    /// Whether the team got the first kill (from version 3.10).
    #[directives(include(if: $v3_10))]
    pub first_kill: Option<bool>,
    /// The objectives the team fulfilled.
    pub objectives: Vec<Objective>,
    /// The team's players, aggregated over the series.
    pub players: Vec<SeriesPlayer>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: SeriesTeamDetails,
}

/// Title-specific fields of a [`SeriesTeam`], chosen by the GraphQL
/// `__typename`.
///
/// The schema has no League of Legends series team type, so there is no
/// `Lol` variant here.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesTeamState"
)]
#[non_exhaustive]
pub enum SeriesTeamDetails {
    /// Valorant (`SeriesTeamStateValorant`).
    Valorant(SeriesTeamValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`SeriesTeam`].
    #[cynic(fallback)]
    Other,
}

/// Valorant fields of a [`SeriesTeam`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesTeamStateValorant"
)]
#[non_exhaustive]
pub struct SeriesTeamValorant {
    /// Enemy headshots.
    pub headshots: i32,
    /// Headshots on teammates.
    pub team_headshots: i32,
}

/// A player aggregated over a series (the `SeriesPlayerState` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesPlayerState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SeriesPlayer {
    /// The GRID ID of the player.
    pub id: Id,
    /// The name of the player.
    pub name: String,
    /// Whether the player is still actively participating.
    pub participation_status: ParticipationStatus,
    /// Enemy kills.
    pub kills: i32,
    /// Deaths.
    pub deaths: i32,
    /// Enemy kill assists given.
    pub kill_assists_given: i32,
    /// Enemy kill assists received.
    pub kill_assists_received: i32,
    /// Suicides.
    pub selfkills: i32,
    /// Kills of teammates.
    pub teamkills: i32,
    /// Structures destroyed.
    pub structures_destroyed: i32,
    /// Structures captured.
    pub structures_captured: i32,
    /// Whether the player got the first kill (from version 3.10).
    #[directives(include(if: $v3_10))]
    pub first_kill: Option<bool>,
    /// The objectives the player fulfilled.
    pub objectives: Vec<Objective>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: SeriesPlayerDetails,
}

/// Title-specific fields of a [`SeriesPlayer`], chosen by the GraphQL
/// `__typename`.
///
/// The schema has no League of Legends series player type, so there is no
/// `Lol` variant here.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesPlayerState"
)]
#[non_exhaustive]
pub enum SeriesPlayerDetails {
    /// Valorant (`SeriesPlayerStateValorant`).
    Valorant(SeriesPlayerValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`SeriesPlayer`].
    #[cynic(fallback)]
    Other,
}

/// Valorant fields of a [`SeriesPlayer`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesPlayerStateValorant"
)]
#[non_exhaustive]
pub struct SeriesPlayerValorant {
    /// Enemy headshots.
    pub headshots: i32,
    /// Headshots on teammates.
    pub team_headshots: i32,
}

// ---------------------------------------------------------------------------
// Games

/// The state of a game (the `GameState` type).
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GameState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct Game {
    /// The GRID ID of the game.
    pub id: Id,
    /// The position of the game in the series, starting at 1.
    pub sequence_number: i32,
    /// The type of the game (from version 3.21).
    #[cynic(rename = "type")]
    #[directives(include(if: $v3_21))]
    pub game_type: Option<GameType>,
    /// Whether the game has started.
    pub started: bool,
    /// Whether the game has finished.
    pub finished: bool,
    /// Whether the game is paused.
    pub paused: bool,
    /// Whether the game was forfeited (from version 3.48).
    #[directives(include(if: $v3_48))]
    pub forfeited: Option<bool>,
    /// Whether the game data is considered accurate (from version 3.50).
    #[directives(include(if: $v3_50))]
    pub valid: Option<bool>,
    /// Why the game data is not considered accurate; `None` while it is
    /// valid (from version 3.50).
    #[directives(include(if: $v3_50))]
    pub invalidation_reason: Option<GameInvalidationReason>,
    /// When the game started (from version 3.7).
    #[directives(include(if: $v3_7))]
    pub started_at: Option<Timestamp>,
    /// The duration of the game (from version 3.15).
    #[directives(include(if: $v3_15))]
    pub duration: Option<Span>,
    /// The game clock (from version 1.3).
    #[directives(include(if: $v1_3))]
    pub clock: Option<Clock>,
    /// The map the game is played on.
    pub map: Map,
    /// The game client version (from version 3.12).
    #[directives(include(if: $v3_12))]
    pub title_version: Option<TitleVersion>,
    /// IDs of the game at external data providers (from version 1.2).
    #[directives(include(if: $v1_2))]
    pub external_links: Option<Vec<ExternalLink>>,
    /// The draft actions (picks and bans) of the game.
    pub draft_actions: Vec<DraftAction>,
    /// The structures on the map, such as towers.
    pub structures: Vec<Structure>,
    /// The non-player characters, such as neutral objectives.
    pub non_player_characters: Vec<NonPlayerCharacter>,
    /// The participating teams.
    pub teams: Vec<GameTeam>,
    /// The segments (such as rounds) of the game (from version 1.4).
    #[directives(include(if: $v1_4))]
    pub segments: Option<Vec<Segment>>,
}

/// A team in a game (the `GameTeamState` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GameTeamState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct GameTeam {
    /// The GRID ID of the team.
    pub id: Id,
    /// The name of the team.
    pub name: String,
    /// The side the team plays, such as red or blue.
    pub side: String,
    /// The team's score in the game.
    pub score: i32,
    /// Whether the team won the game.
    pub won: bool,
    /// Enemy kills.
    pub kills: i32,
    /// Deaths.
    pub deaths: i32,
    /// Enemy kill assists given.
    pub kill_assists_given: i32,
    /// Enemy kill assists received.
    pub kill_assists_received: i32,
    /// Suicides.
    pub selfkills: i32,
    /// Kills of teammates.
    pub teamkills: i32,
    /// Money held in cash by the team's players (gold, for League of
    /// Legends).
    pub money: i32,
    /// Total value of the players' loadouts.
    pub loadout_value: i32,
    /// Money plus inventory value.
    pub net_worth: i32,
    /// Structures destroyed.
    pub structures_destroyed: i32,
    /// Structures captured.
    pub structures_captured: i32,
    /// Whether the team got the first kill (from version 3.10).
    #[directives(include(if: $v3_10))]
    pub first_kill: Option<bool>,
    /// The objectives the team fulfilled.
    pub objectives: Vec<Objective>,
    /// The team's players.
    pub players: Vec<GamePlayer>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: GameTeamDetails,
}

/// Title-specific fields of a [`GameTeam`], chosen by the GraphQL
/// `__typename`.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GameTeamState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub enum GameTeamDetails {
    /// League of Legends (`GameTeamStateLol`).
    Lol(GameTeamLol),
    /// Valorant (`GameTeamStateValorant`).
    Valorant(GameTeamValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`GameTeam`].
    #[cynic(fallback)]
    Other,
}

/// League of Legends fields of a [`GameTeam`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GameTeamStateLol",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct GameTeamLol {
    /// Experience points gathered.
    pub experience_points: i32,
    /// Total gold earned (from version 3.2).
    #[directives(include(if: $v3_2))]
    pub total_money_earned: Option<i32>,
    /// Difference in gold earned to the opponent (from version 3.28).
    #[directives(include(if: $v3_28))]
    pub money_difference: Option<i32>,
    /// Gold earned per minute (from version 3.36).
    #[directives(include(if: $v3_36))]
    pub money_per_minute: Option<f64>,
    /// Percentage of game time with 51.5% or more of the total gold (from
    /// version 3.40).
    #[directives(include(if: $v3_40))]
    pub major_money_lead: Option<f64>,
    /// Percentage of game time with 48.5% or less of the total gold (from
    /// version 3.41).
    #[directives(include(if: $v3_41))]
    pub major_money_deficit: Option<f64>,
    /// Damage dealt (from version 3.23).
    #[directives(include(if: $v3_23))]
    pub damage_dealt: Option<i32>,
    /// Damage taken (from version 3.23).
    #[directives(include(if: $v3_23))]
    pub damage_taken: Option<i32>,
    /// Damage dealt per minute (from version 3.24).
    #[directives(include(if: $v3_24))]
    pub damage_per_minute: Option<f64>,
    /// Damage dealt per gold earned (from version 3.26).
    #[directives(include(if: $v3_26))]
    pub damage_per_money: Option<f64>,
    /// Kills plus assists per death (from version 3.27).
    #[directives(include(if: $v3_27))]
    pub kda_ratio: Option<f64>,
    /// Kills plus assists (from version 3.34).
    #[directives(include(if: $v3_34))]
    pub kills_and_assists: Option<f64>,
    /// Vision score (from version 3.30).
    #[directives(include(if: $v3_30))]
    pub vision_score: Option<f64>,
    /// Vision score per minute (from version 3.33).
    #[directives(include(if: $v3_33))]
    pub vision_score_per_minute: Option<f64>,
    /// Average percentage of game time the players spent on the opponent's
    /// side of the map (from version 3.42).
    #[directives(include(if: $v3_42))]
    pub forward_percentage: Option<f64>,
    /// The completed Baron power plays (from version 3.38).
    #[directives(include(if: $v3_38))]
    pub baron_power_plays: Option<Vec<BaronPowerPlay>>,
}

/// Valorant fields of a [`GameTeam`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GameTeamStateValorant",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct GameTeamValorant {
    /// Enemy headshots.
    pub headshots: i32,
    /// Headshots on teammates.
    pub team_headshots: i32,
    /// Damage dealt (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_dealt: Option<i32>,
    /// Damage taken (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_taken: Option<i32>,
    /// Damage dealt to self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_dealt: Option<i32>,
    /// Damage taken from self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_taken: Option<i32>,
    /// Damage dealt to teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_dealt: Option<i32>,
    /// Damage taken from teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_taken: Option<i32>,
}

/// A player in a game (the `GamePlayerState` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GamePlayerState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct GamePlayer {
    /// The GRID ID of the player.
    pub id: Id,
    /// The name of the player.
    pub name: String,
    /// Whether the player is still actively participating.
    pub participation_status: ParticipationStatus,
    /// The in-game character (champion, agent) played.
    pub character: Option<Character>,
    /// The player's roles in the game (from version 3.43).
    #[directives(include(if: $v3_43))]
    pub roles: Option<Vec<PlayerRole>>,
    /// Enemy kills.
    pub kills: i32,
    /// Deaths.
    pub deaths: i32,
    /// Enemy kill assists given.
    pub kill_assists_given: i32,
    /// Enemy kill assists received.
    pub kill_assists_received: i32,
    /// Suicides.
    pub selfkills: i32,
    /// Kills of teammates.
    pub teamkills: i32,
    /// Money held in cash (gold, for League of Legends).
    pub money: i32,
    /// Total value of the player's loadout.
    pub loadout_value: i32,
    /// Money plus inventory value.
    pub net_worth: i32,
    /// Structures destroyed.
    pub structures_destroyed: i32,
    /// Structures captured.
    pub structures_captured: i32,
    /// Whether the player got the first kill (from version 3.10).
    #[directives(include(if: $v3_10))]
    pub first_kill: Option<bool>,
    /// The player's position on the map.
    pub position: Option<Coordinates>,
    /// The player's inventory.
    pub inventory: Inventory,
    /// The objectives the player fulfilled.
    pub objectives: Vec<Objective>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: GamePlayerDetails,
}

/// Title-specific fields of a [`GamePlayer`], chosen by the GraphQL
/// `__typename`.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GamePlayerState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub enum GamePlayerDetails {
    /// League of Legends (`GamePlayerStateLol`).
    Lol(GamePlayerLol),
    /// Valorant (`GamePlayerStateValorant`).
    Valorant(GamePlayerValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`GamePlayer`].
    #[cynic(fallback)]
    Other,
}

/// League of Legends fields of a [`GamePlayer`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GamePlayerStateLol",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct GamePlayerLol {
    /// Whether the player is alive.
    pub alive: bool,
    /// Current health.
    pub current_health: i32,
    /// Maximum health.
    pub max_health: i32,
    /// Current armor.
    pub current_armor: i32,
    /// Experience points gathered.
    pub experience_points: i32,
    /// When the player respawns (from version 3.3).
    #[directives(include(if: $v3_3))]
    pub respawn_clock: Option<Clock>,
    /// Total gold earned (from version 3.2).
    #[directives(include(if: $v3_2))]
    pub total_money_earned: Option<i32>,
    /// Gold earned per minute (from version 3.36).
    #[directives(include(if: $v3_36))]
    pub money_per_minute: Option<f64>,
    /// Share of the team's gold earned, in percent (from version 3.37).
    #[directives(include(if: $v3_37))]
    pub money_percentage: Option<f64>,
    /// Damage dealt (from version 3.23).
    #[directives(include(if: $v3_23))]
    pub damage_dealt: Option<i32>,
    /// Damage taken (from version 3.23).
    #[directives(include(if: $v3_23))]
    pub damage_taken: Option<i32>,
    /// Damage dealt per minute (from version 3.24).
    #[directives(include(if: $v3_24))]
    pub damage_per_minute: Option<f64>,
    /// Share of the team's damage dealt, in percent (from version 3.25).
    #[directives(include(if: $v3_25))]
    pub damage_percentage: Option<f64>,
    /// Damage dealt per gold earned (from version 3.26).
    #[directives(include(if: $v3_26))]
    pub damage_per_money: Option<f64>,
    /// Kills plus assists per death (from version 3.27).
    #[directives(include(if: $v3_27))]
    pub kda_ratio: Option<f64>,
    /// Kills plus assists (from version 3.34).
    #[directives(include(if: $v3_34))]
    pub kills_and_assists: Option<f64>,
    /// Share of the team's kills the player took part in, in percent (from
    /// version 3.35).
    #[directives(include(if: $v3_35))]
    pub kill_participation: Option<f64>,
    /// Vision score (from version 3.30).
    #[directives(include(if: $v3_30))]
    pub vision_score: Option<f64>,
    /// Vision score per minute (from version 3.33).
    #[directives(include(if: $v3_33))]
    pub vision_score_per_minute: Option<f64>,
    /// Percentage of game time spent on the opponent's side of the map (from
    /// version 3.42).
    #[directives(include(if: $v3_42))]
    pub forward_percentage: Option<f64>,
}

/// Valorant fields of a [`GamePlayer`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "GamePlayerStateValorant",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct GamePlayerValorant {
    /// Whether the player is alive (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub alive: Option<bool>,
    /// Current health (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub current_health: Option<i32>,
    /// Maximum health (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub max_health: Option<i32>,
    /// Current armor (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub current_armor: Option<i32>,
    /// Ultimate points (from version 3.20).
    #[directives(include(if: $v3_20))]
    pub ultimate_points: Option<i32>,
    /// Enemy headshots.
    pub headshots: i32,
    /// Headshots on teammates.
    pub team_headshots: i32,
    /// Damage dealt (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_dealt: Option<i32>,
    /// Damage taken (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_taken: Option<i32>,
    /// Damage dealt to self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_dealt: Option<i32>,
    /// Damage taken from self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_taken: Option<i32>,
    /// Damage dealt to teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_dealt: Option<i32>,
    /// Damage taken from teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_taken: Option<i32>,
}

// ---------------------------------------------------------------------------
// Segments

/// A segment of a game, such as a Valorant round (the `SegmentState` type).
///
/// Selects one more level of [`SubSegment`]s.
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct Segment {
    /// The GRID ID of the segment.
    pub id: Id,
    /// The type of the segment.
    #[cynic(rename = "type")]
    pub segment_type: String,
    /// The position of the segment in its game, starting at 1.
    pub sequence_number: i32,
    /// Whether the segment has started.
    pub started: bool,
    /// Whether the segment has finished.
    pub finished: bool,
    /// When the segment started (from version 3.12).
    #[directives(include(if: $v3_12))]
    pub started_at: Option<Timestamp>,
    /// The duration of the segment (from version 3.16).
    #[directives(include(if: $v3_16))]
    pub duration: Option<Span>,
    /// The participating teams.
    pub teams: Vec<SegmentTeam>,
    /// The segments nested in this one.
    pub segments: Vec<SubSegment>,
}

/// A segment nested in a [`Segment`] (the `SegmentState` type).
///
/// Selects the same fields as [`Segment`] except its own nested segments.
#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SubSegment {
    /// The GRID ID of the segment.
    pub id: Id,
    /// The type of the segment.
    #[cynic(rename = "type")]
    pub segment_type: String,
    /// The position of the segment in its parent, starting at 1.
    pub sequence_number: i32,
    /// Whether the segment has started.
    pub started: bool,
    /// Whether the segment has finished.
    pub finished: bool,
    /// When the segment started (from version 3.12).
    #[directives(include(if: $v3_12))]
    pub started_at: Option<Timestamp>,
    /// The duration of the segment (from version 3.16).
    #[directives(include(if: $v3_16))]
    pub duration: Option<Span>,
    /// The participating teams.
    pub teams: Vec<SegmentTeam>,
}

/// A team in a segment (the `SegmentTeamState` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentTeamState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SegmentTeam {
    /// The GRID ID of the team.
    pub id: Id,
    /// The name of the team.
    pub name: String,
    /// The side the team plays in the segment.
    pub side: String,
    /// Whether the team won the segment.
    pub won: bool,
    /// Enemy kills.
    pub kills: i32,
    /// Deaths.
    pub deaths: i32,
    /// Enemy kill assists given.
    pub kill_assists_given: i32,
    /// Enemy kill assists received.
    pub kill_assists_received: i32,
    /// Suicides.
    pub selfkills: i32,
    /// Kills of teammates.
    pub teamkills: i32,
    /// Whether the team got the first kill (from version 3.10).
    #[directives(include(if: $v3_10))]
    pub first_kill: Option<bool>,
    /// The objectives the team fulfilled.
    pub objectives: Vec<Objective>,
    /// The team's players.
    pub players: Vec<SegmentPlayer>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: SegmentTeamDetails,
}

/// Title-specific fields of a [`SegmentTeam`], chosen by the GraphQL
/// `__typename`.
///
/// The schema has no League of Legends segment team type, so there is no
/// `Lol` variant here.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentTeamState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub enum SegmentTeamDetails {
    /// Valorant (`SegmentTeamStateValorant`).
    Valorant(SegmentTeamValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`SegmentTeam`].
    #[cynic(fallback)]
    Other,
}

/// Valorant fields of a [`SegmentTeam`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentTeamStateValorant",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SegmentTeamValorant {
    /// If the team won the round, how it won; otherwise empty (from version
    /// 1.9).
    #[directives(include(if: $v1_9))]
    pub win_type: Option<String>,
    /// Enemy headshots.
    pub headshots: i32,
    /// Headshots on teammates.
    pub team_headshots: i32,
    /// Damage dealt (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_dealt: Option<i32>,
    /// Damage taken (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_taken: Option<i32>,
    /// Damage dealt to self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_dealt: Option<i32>,
    /// Damage taken from self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_taken: Option<i32>,
    /// Damage dealt to teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_dealt: Option<i32>,
    /// Damage taken from teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_taken: Option<i32>,
}

/// A player in a segment (the `SegmentPlayerState` interface).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentPlayerState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SegmentPlayer {
    /// The GRID ID of the player.
    pub id: Id,
    /// The name of the player.
    pub name: String,
    /// Whether the player is actively participating (from version 3.8).
    #[directives(include(if: $v3_8))]
    pub participation_status: Option<ParticipationStatus>,
    /// Enemy kills.
    pub kills: i32,
    /// Deaths.
    pub deaths: i32,
    /// Enemy kill assists given.
    pub kill_assists_given: i32,
    /// Enemy kill assists received.
    pub kill_assists_received: i32,
    /// Suicides.
    pub selfkills: i32,
    /// Kills of teammates.
    pub teamkills: i32,
    /// Whether the player got the first kill (from version 3.10).
    #[directives(include(if: $v3_10))]
    pub first_kill: Option<bool>,
    /// The objectives the player fulfilled.
    pub objectives: Vec<Objective>,
    /// Title-specific fields.
    #[cynic(spread)]
    pub details: SegmentPlayerDetails,
}

/// Title-specific fields of a [`SegmentPlayer`], chosen by the GraphQL
/// `__typename`.
///
/// The schema has no League of Legends segment player type, so there is no
/// `Lol` variant here.
#[derive(cynic::InlineFragments, Debug, Clone, PartialEq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentPlayerState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub enum SegmentPlayerDetails {
    /// Valorant (`SegmentPlayerStateValorant`).
    Valorant(SegmentPlayerValorant),
    /// Any other title, or a title this version has no typed fields for. The
    /// common fields are on the [`SegmentPlayer`].
    #[cynic(fallback)]
    Other,
}

/// Valorant fields of a [`SegmentPlayer`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SegmentPlayerStateValorant",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct SegmentPlayerValorant {
    /// Whether the player is alive (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub alive: Option<bool>,
    /// Current health (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub current_health: Option<i32>,
    /// Maximum health (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub max_health: Option<i32>,
    /// Current armor (from version 1.9).
    #[directives(include(if: $v1_9))]
    pub current_armor: Option<i32>,
    /// Enemy headshots.
    pub headshots: i32,
    /// Headshots on teammates.
    pub team_headshots: i32,
    /// Damage dealt (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_dealt: Option<i32>,
    /// Damage taken (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub damage_taken: Option<i32>,
    /// Damage dealt to self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_dealt: Option<i32>,
    /// Damage taken from self (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub selfdamage_taken: Option<i32>,
    /// Damage dealt to teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_dealt: Option<i32>,
    /// Damage taken from teammates (from version 3.29).
    #[directives(include(if: $v3_29))]
    pub teamdamage_taken: Option<i32>,
}

// ---------------------------------------------------------------------------
// Shared object types

/// The esports title of a series.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct Title {
    /// The unique short name of the title.
    pub name_shortened: String,
}

/// The game client version a game was played on.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct TitleVersion {
    /// The version name.
    pub name: String,
}

/// The map of a game (the `MapState` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "MapState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct Map {
    /// The GRID map ID (from version 3.13).
    #[directives(include(if: $v3_13))]
    pub id: Option<String>,
    /// The map name.
    pub name: String,
}

/// A game or respawn clock (the `ClockState` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "ClockState"
)]
#[non_exhaustive]
pub struct Clock {
    /// The ID of the clock.
    pub id: Option<String>,
    /// The type of the clock.
    #[cynic(rename = "type")]
    pub clock_type: Option<String>,
    /// Whether the clock is running.
    pub ticking: Option<bool>,
    /// Whether the clock counts down.
    pub ticks_backwards: Option<bool>,
    /// The clock's current value in seconds.
    pub current_seconds: Option<i32>,
}

/// A position on the map.
#[derive(cynic::QueryFragment, Debug, Clone, Copy, PartialEq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct Coordinates {
    /// The X coordinate.
    pub x: f64,
    /// The Y coordinate.
    pub y: f64,
}

/// An in-game character, such as a champion or agent.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct Character {
    /// The GRID character ID.
    pub id: Id,
    /// The character name.
    pub name: String,
}

/// A role a player has in a game.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct PlayerRole {
    /// The GRID player role ID.
    pub id: Id,
}

/// An objective fulfilled by a team or player.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct Objective {
    /// The GRID objective ID.
    pub id: Id,
    /// The objective type.
    #[cynic(rename = "type")]
    pub objective_type: String,
    /// How many times the objective was completed.
    pub completion_count: i32,
    /// Whether this was the first completion in the current level (from
    /// version 3.11).
    #[directives(include(if: $v3_11))]
    pub completed_first: Option<bool>,
}

/// A player's inventory (the `PlayerInventory` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "PlayerInventory"
)]
#[non_exhaustive]
pub struct Inventory {
    /// The item stacks in the inventory.
    pub items: Vec<ItemStack>,
}

/// A stack of identical items in an [`Inventory`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct ItemStack {
    /// The GRID item ID.
    pub id: Id,
    /// The item name.
    pub name: String,
    /// The number of items, equipped and stashed.
    pub quantity: i32,
    /// The number of equipped items.
    pub equipped: i32,
    /// The number of stashed items.
    pub stashed: i32,
}

/// A draft action, such as a pick or ban.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct DraftAction {
    /// The GRID draft action ID.
    pub id: Id,
    /// The type of the action, such as a pick or a ban.
    #[cynic(rename = "type")]
    pub action_type: String,
    /// The position of the action in the draft.
    pub sequence_number: String,
    /// Who performed the action.
    pub drafter: Drafter,
    /// What was drafted.
    pub draftable: Draftable,
}

/// The entity performing a [`DraftAction`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct Drafter {
    /// The GRID ID of the drafter.
    pub id: Id,
    /// The type of the drafter.
    #[cynic(rename = "type")]
    pub drafter_type: String,
}

/// The entity drafted in a [`DraftAction`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct Draftable {
    /// The GRID ID of the drafted entity.
    pub id: Id,
    /// The type of the drafted entity.
    #[cynic(rename = "type")]
    pub draftable_type: String,
    /// The name of the drafted entity.
    pub name: String,
}

/// A structure on the map, such as a tower (the `StructureState` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "StructureState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct Structure {
    /// The GRID structure ID.
    pub id: Id,
    /// The type of the structure.
    #[cynic(rename = "type")]
    pub structure_type: String,
    /// The side controlling the structure.
    pub side: String,
    /// The GRID ID of the team controlling the structure.
    pub team_id: Id,
    /// Whether the structure is destroyed.
    pub destroyed: bool,
    /// Current health (from version 1.6).
    #[directives(include(if: $v1_6))]
    pub current_health: Option<i32>,
    /// Maximum health (from version 1.6).
    #[directives(include(if: $v1_6))]
    pub max_health: Option<i32>,
}

/// A non-player character, such as a neutral objective (the
/// `NonPlayerCharacterState` type).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "NonPlayerCharacterState",
    variables = "super::queries::SeriesStateVariables"
)]
#[non_exhaustive]
pub struct NonPlayerCharacter {
    /// The GRID ID of the character.
    pub id: Id,
    /// The type of the character.
    #[cynic(rename = "type")]
    pub npc_type: String,
    /// The side controlling the character.
    pub side: String,
    /// Whether the character is alive.
    pub alive: bool,
    /// When the character respawns (from version 1.3).
    #[directives(include(if: $v1_3))]
    pub respawn_clock: Option<Clock>,
}

/// A completed League of Legends Baron power play.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "LolBaronPowerPlay"
)]
#[non_exhaustive]
pub struct BaronPowerPlay {
    /// The GRID ID of the power play.
    pub id: Id,
    /// The value of the power play after it ended.
    pub value: i32,
}

/// An ID of a game at an external data provider.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct ExternalLink {
    /// The data provider.
    pub data_provider: DataProvider,
    /// The entity at the data provider.
    pub external_entity: ExternalEntity,
}

/// A provider of external IDs.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct DataProvider {
    /// The unique name of the data provider, such as `"RIOT"`.
    pub name: String,
}

/// An entity at an external data provider.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub struct ExternalEntity {
    /// The ID used by the data provider.
    pub id: Id,
}

// ---------------------------------------------------------------------------
// Enums

/// Whether a player is still participating.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub enum ParticipationStatus {
    /// The player is actively participating.
    #[cynic(rename = "active")]
    Active,
    /// The player is no longer actively participating.
    #[cynic(rename = "inactive")]
    Inactive,
    /// A value this version of the SDK does not know.
    #[cynic(fallback)]
    Other(String),
}

/// The type of a [`Game`].
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub enum GameType {
    /// A competitive non-esports game.
    Competitive,
    /// An esports game.
    Esports,
    /// A regular non-competitive game.
    Regular,
    /// A practice game.
    Scrim,
    /// A value this version of the SDK does not know.
    #[cynic(fallback)]
    Other(String),
}

/// Why a [`Series`] is not considered accurate.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub enum SeriesInvalidationReason {
    /// A player switched to a different agent.
    AgentSwitch,
    /// The teams could not be mapped.
    FailedTeamMapping,
    /// The winner of a game could not be determined.
    GameWinnerMissing,
    /// A team played with an unexpected number of players.
    IncorrectTeamSize,
    /// The draft was invalid.
    InvalidDraft,
    /// At least one event was lost.
    LostEvent,
    /// The series was invalidated manually.
    ManualAction,
    /// A Roshan kill could not be attributed to a player.
    NoRoshanPlayerKiller,
    /// A Roshan kill could not be attributed to a team.
    NoRoshanTeamKiller,
    /// The series was reverted to an earlier point in time.
    Revert,
    /// The series was invalidated through an SDK command.
    SdkCommand,
    /// A team was still to be determined.
    TbdTeam,
    /// The series exceeded the maximum number of processed transactions.
    TransactionLimitReached,
    /// The series exceeded its maximum lifetime.
    TtlReached,
    /// No reason was given.
    Unspecified,
    /// A value this version of the SDK does not know.
    #[cynic(fallback)]
    Other(String),
}

/// Why a [`Game`] is not considered accurate.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "series_state", schema_module = "crate::series_state::schema")]
#[non_exhaustive]
pub enum GameInvalidationReason {
    /// A player switched to a different agent.
    AgentSwitch,
    /// The winner could not be determined.
    GameWinnerMissing,
    /// A team played with an unexpected number of players.
    IncorrectTeamSize,
    /// The draft was invalid.
    InvalidDraft,
    /// At least one event was lost.
    LostEvent,
    /// The game was invalidated manually.
    ManualAction,
    /// A Roshan kill could not be attributed to a player.
    NoRoshanPlayerKiller,
    /// A Roshan kill could not be attributed to a team.
    NoRoshanTeamKiller,
    /// The game was reverted to an earlier point in time.
    Revert,
    /// The game was invalidated through an SDK command.
    SdkCommand,
    /// The series exceeded the maximum number of processed transactions.
    TransactionLimitReached,
    /// The series exceeded its maximum lifetime.
    TtlReached,
    /// No reason was given.
    Unspecified,
    /// A value this version of the SDK does not know.
    #[cynic(fallback)]
    Other(String),
}
