//! Result types of the prebuilt Central Data queries.
//!
//! Each entity type selects its scalar fields plus small "ref" types for the
//! entities it points to, so no selection recurses. Deprecated fields are not
//! selected. The selection of every type is listed in its documentation.

use cynic::Id;
use jiff::Timestamp;
use jiff::civil::Date;
use rust_decimal::Decimal;
use url::Url;

use crate::scalars::HexColor;

/// A game title, such as League of Legends or Valorant.
///
/// Selects every field of the `Title` type except `logoUrl`; see the
/// [module documentation](super#fields-not-selected).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Title {
    /// The ID of the title.
    pub id: Id,
    /// The name of the title.
    pub name: String,
    /// The shortened name of the title (at most 20 characters).
    pub name_shortened: String,
    /// Whether the title is private.
    pub private: bool,
}

/// A tournament.
///
/// Selects the tournament's own fields, its titles, its external links, and
/// its parent and children as [`TournamentRef`]s. The `teams` field is not
/// selected; query it with [`Central::run`](super::Central::run) if needed.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Tournament {
    /// The ID of the tournament.
    pub id: Id,
    /// The name of the tournament.
    pub name: String,
    /// The shortened name of the tournament (at most 30 characters).
    pub name_shortened: String,
    /// The tournament logo URL.
    pub logo_url: Url,
    /// The start date of the tournament.
    pub start_date: Option<Date>,
    /// The end date of the tournament.
    pub end_date: Option<Date>,
    /// Whether the tournament is private.
    pub private: bool,
    /// The tournament prize pool in USD.
    pub prize_pool: Option<Money>,
    /// The type of venue where the tournament is held.
    pub venue_type: TournamentVenueType,
    /// The titles linked to the tournament.
    pub titles: Vec<Title>,
    /// The parent tournament.
    pub parent: Option<TournamentRef>,
    /// The child tournaments.
    pub children: Vec<TournamentRef>,
    /// The external tournament IDs and their providers.
    pub external_links: Vec<ExternalLink>,
    /// The date and time of the last update.
    pub updated_at: Timestamp,
}

/// A reference to a [`Tournament`]: its ID and names.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Tournament"
)]
#[non_exhaustive]
pub struct TournamentRef {
    /// The ID of the tournament.
    pub id: Id,
    /// The name of the tournament.
    pub name: String,
    /// The shortened name of the tournament (at most 30 characters).
    pub name_shortened: String,
}

/// A monetary value.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Money {
    /// The amount in USD.
    pub amount: Decimal,
}

/// A team.
///
/// Selects the team's own fields, its title, its organization as an
/// [`OrganizationRef`], its logo assets and its external links. The
/// deprecated `titles` field and `rating` are not selected; see the
/// [module documentation](super#fields-not-selected).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Team {
    /// The ID of the team.
    pub id: Id,
    /// The name of the team.
    pub name: String,
    /// The shortened name of the team.
    pub name_shortened: Option<String>,
    /// The team logo URL.
    pub logo_url: Url,
    /// The primary color of the team.
    pub color_primary: HexColor,
    /// The secondary color of the team.
    pub color_secondary: HexColor,
    /// Whether the team is private.
    pub private: bool,
    /// The title the team plays.
    pub title: Title,
    /// The organization the team plays for.
    pub organization: Option<OrganizationRef>,
    /// All logo variants of the team.
    pub assets: Vec<Asset>,
    /// The external team IDs and their providers.
    pub external_links: Vec<ExternalLink>,
    /// The workflow status of the team.
    pub workflow_status: TeamWorkflowStatus,
    /// The date and time of the last update.
    pub updated_at: Timestamp,
}

/// A reference to a [`Team`]: its ID, names and logo.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Team"
)]
#[non_exhaustive]
pub struct TeamRef {
    /// The ID of the team.
    pub id: Id,
    /// The name of the team.
    pub name: String,
    /// The shortened name of the team.
    pub name_shortened: Option<String>,
    /// The team logo URL.
    pub logo_url: Url,
}

/// A team as listed on an [`Organization`] (the `TeamRelation` type): its ID,
/// names and logo.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "TeamRelation"
)]
#[non_exhaustive]
pub struct OrganizationTeam {
    /// The ID of the team.
    pub id: Id,
    /// The name of the team.
    pub name: String,
    /// The shortened name of the team.
    pub name_shortened: Option<String>,
    /// The team logo URL.
    pub logo_url: Url,
}

/// A logo or other image of a team.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Asset {
    /// The kind of asset.
    pub asset_type: AssetType,
    /// The asset URL.
    pub url: Url,
}

/// An organization.
///
/// Selects the organization's own fields and its teams as
/// [`OrganizationTeam`]s.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Organization {
    /// The ID of the organization.
    pub id: Id,
    /// The name of the organization.
    pub name: String,
    /// Whether the organization is private.
    pub private: bool,
    /// The teams that belong to the organization (empty if the server sends
    /// none).
    #[cynic(flatten)]
    pub teams: Vec<OrganizationTeam>,
    /// The date and time of the last update.
    pub updated_at: Timestamp,
}

/// A reference to an organization (the `OrganizationRelation` type): its ID
/// and name.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "OrganizationRelation"
)]
#[non_exhaustive]
pub struct OrganizationRef {
    /// The ID of the organization.
    pub id: Id,
    /// The name of the organization.
    pub name: String,
}

/// A player.
///
/// Selects the player's own fields, roles, title, current team as a
/// [`TeamRef`] and external links. `fullName`, `age`, `imageUrl` and
/// `nationality` are not selected; see the
/// [module documentation](super#fields-not-selected).
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Player {
    /// The ID of the player.
    pub id: Id,
    /// The player's in-game name.
    pub nickname: String,
    /// The player's roles (for example MID or SUPPORT).
    pub roles: Vec<PlayerRole>,
    /// Whether the player is private.
    pub private: bool,
    /// The title the player currently plays.
    pub title: Title,
    /// The team the player currently plays for.
    pub team: Option<TeamRef>,
    /// The external player IDs and their providers.
    pub external_links: Vec<ExternalLink>,
    /// The date and time of the last update.
    pub updated_at: Timestamp,
}

/// A reference to a [`Player`]: their ID and nickname.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Player"
)]
#[non_exhaustive]
pub struct PlayerRef {
    /// The ID of the player.
    pub id: Id,
    /// The player's in-game name.
    pub nickname: String,
}

/// A player's nationality.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Nationality {
    /// The ISO 3166-1 alpha-3 country code, for example `USA`.
    pub code: String,
    /// The full country name, for example `United States`.
    pub name: String,
}

/// A player role, such as MID or SUPPORT.
///
/// Selects every field of the `PlayerRole` type.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct PlayerRole {
    /// The ID of the role.
    pub id: Id,
    /// The name of the role.
    pub name: String,
    /// Whether the role is private.
    pub private: bool,
    /// The title the role belongs to.
    pub title: Title,
}

/// A series (a match of one or more games).
///
/// Selects the series' own fields, format, title, tournament as a
/// [`TournamentRef`], teams (each as a [`TeamRef`] with its score
/// advantage), players as [`PlayerRef`]s, streams, product service levels
/// and external links.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct Series {
    /// The ID of the series.
    pub id: Id,
    /// The type of the series.
    #[cynic(rename = "type")]
    pub series_type: SeriesType,
    /// The scheduled start time.
    pub start_time_scheduled: Timestamp,
    /// The format of the series, such as best of three.
    pub format: SeriesFormat,
    /// The title being played.
    pub title: Title,
    /// The tournament the series belongs to.
    pub tournament: TournamentRef,
    /// The participating teams.
    pub teams: Vec<TeamParticipant>,
    /// The participating players.
    pub players: Vec<PlayerRef>,
    /// The available video streams.
    pub streams: Vec<VideoStream>,
    /// The availability of each GRID product for this series.
    pub product_service_levels: Vec<ProductServiceLevel>,
    /// Whether the series is private.
    pub private: bool,
    /// The external series IDs and their providers.
    pub external_links: Vec<ExternalLink>,
    /// The workflow status of the series.
    pub workflow_status: SeriesWorkflowStatus,
    /// The date and time of the last update.
    pub updated_at: Timestamp,
}

/// A team participating in a [`Series`].
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct TeamParticipant {
    /// The team.
    pub base_info: TeamRef,
    /// The team's score advantage in the series.
    pub score_advantage: i32,
}

/// A series format, such as `BO3`.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct SeriesFormat {
    /// The ID of the format.
    pub id: Option<Id>,
    /// The name of the format.
    pub name: String,
    /// The shortened name of the format (at most 5 characters).
    pub name_shortened: String,
}

/// A live video stream.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct VideoStream {
    /// The stream URL, as sent by GRID (not validated).
    pub url: String,
}

/// The availability of one GRID product for a series.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct ProductServiceLevel {
    /// The name of the product.
    pub product_name: String,
    /// The availability of the product.
    pub service_level: ServiceLevel,
}

/// An external data provider, such as `STEAM`.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct DataProvider {
    /// The provider name, as passed to the `*_id_by_external_id` lookups.
    pub name: String,
    /// A description of the provider.
    pub description: Option<String>,
}

/// The ID an external data provider uses for an entity.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct ExternalLink {
    /// The external data provider.
    pub data_provider: DataProvider,
    /// The provider's information about the entity.
    pub external_entity: ExternalEntity,
}

impl ExternalLink {
    /// The provider name, for example `STEAM`.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.data_provider.name
    }

    /// The provider's ID for the entity.
    #[must_use]
    pub fn external_id(&self) -> &Id {
        &self.external_entity.id
    }
}

/// Information an external data provider holds about an entity.
#[derive(cynic::QueryFragment, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct ExternalEntity {
    /// The ID used by the external data provider.
    pub id: Id,
}

/// The kind of an [`Asset`].
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum AssetType {
    /// An alternative logo.
    AlternativeLogo,
    /// A logo for use on backgrounds.
    BackgroundLogo,
    /// A logo for dark backgrounds.
    DarkLogo,
    /// A logo for light backgrounds.
    LightLogo,
    /// The main logo.
    MainLogo,
    /// A value this version of the SDK does not know.
    #[cynic(fallback)]
    Other(String),
}

/// The type of a [`Series`].
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum SeriesType {
    /// A competitive non-esports series.
    Competitive,
    /// A competitive esports series.
    Esports,
    /// A GRID test game that is played repeatedly in a loop.
    Loopfeed,
    /// A practice series.
    Scrim,
    /// A value this version of the SDK does not know. When sent in a filter,
    /// the string is passed through unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// The workflow status of a [`Series`].
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum SeriesWorkflowStatus {
    /// The series has been deleted.
    Deleted,
    /// The series is published.
    Published,
    /// A value this version of the SDK does not know. When sent in a filter,
    /// the string is passed through unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// The workflow status of a [`Team`].
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum TeamWorkflowStatus {
    /// The team has been deleted.
    Deleted,
    /// The team is published.
    Published,
    /// A value this version of the SDK does not know. When sent in a filter,
    /// the string is passed through unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// The availability of a GRID product for a series.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum ServiceLevel {
    /// The product is fully available.
    Full,
    /// Some functionality of the product may be limited or unavailable.
    Limited,
    /// The product is not available.
    None,
    /// A value this version of the SDK does not know. When sent in a filter,
    /// the string is passed through unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// Where a [`Tournament`] is held.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum TournamentVenueType {
    /// The tournament has both online and LAN phases.
    Hybrid,
    /// The tournament is held entirely at a physical location (LAN).
    Lan,
    /// The tournament is held entirely online.
    Online,
    /// The venue type is not specified.
    Unknown,
    /// A value this version of the SDK does not know. When sent in a filter,
    /// the string is passed through unchanged.
    #[cynic(fallback)]
    Other(String),
}
