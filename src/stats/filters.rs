//! Filter inputs of the Stats Feed queries.
//!
//! Every filter implements [`Default`] (no constraint) and has one chainable
//! setter per field, named like the field:
//!
//! ```
//! use gridgg::stats::{DateTimeFilter, TeamStatisticsFilter, TimeRangeFilter, TeamStatisticsTournamentFilter};
//!
//! let filter = TeamStatisticsFilter::default()
//!     .started_at(DateTimeFilter::period(TimeRangeFilter::Last3Months))
//!     .tournament(TeamStatisticsTournamentFilter::new(["123"]).include_children(false));
//! # let _ = filter;
//! ```
//!
//! Unset fields are left out of the request, so the server applies its own
//! defaults.
//!
//! Filter fields the schema documents as deprecated (`timeWindow`, replaced
//! by `startedAt`, and `tournamentIds`, replaced by `tournament.id`) are not
//! exposed.

use cynic::Id;

/// Adds one chainable setter per optional field, named like the field.
macro_rules! setters {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        impl $name {
            $(
                #[doc = concat!(
                    "Sets [`", stringify!($field), "`](#structfield.", stringify!($field), ")."
                )]
                #[must_use]
                pub fn $field(mut self, value: impl Into<$ty>) -> Self {
                    self.$field = Some(value.into());
                    self
                }
            )*
        }
    };
}

// ---------------------------------------------------------------------------
// Scalar filters

/// Matches IDs against a list.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct IdFilter {
    /// The IDs to look for (the `in` field).
    #[cynic(rename = "in", skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<Id>>,
}

impl IdFilter {
    /// Matches any of `ids`.
    #[must_use]
    pub fn new<I>(ids: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Id>,
    {
        Self {
            ids: Some(ids.into_iter().map(Into::into).collect()),
        }
    }
}

impl<T: Into<Id>, const N: usize> From<[T; N]> for IdFilter {
    fn from(ids: [T; N]) -> Self {
        Self::new(ids)
    }
}

impl<T: Into<Id>> From<Vec<T>> for IdFilter {
    fn from(ids: Vec<T>) -> Self {
        Self::new(ids)
    }
}

/// Matches a string field, case-insensitively.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct StringFilter {
    /// Matches values containing this string.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    /// Matches values equal to this string.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub equals: Option<String>,
}

impl StringFilter {
    /// Matches values containing `value`, case-insensitively.
    #[must_use]
    pub fn contains(value: impl Into<String>) -> Self {
        Self {
            contains: Some(value.into()),
            equals: None,
        }
    }

    /// Matches values equal to `value`, case-insensitively.
    #[must_use]
    pub fn equals(value: impl Into<String>) -> Self {
        Self {
            contains: None,
            equals: Some(value.into()),
        }
    }
}

/// Matches a boolean field. Converts from [`bool`].
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct BooleanFilter {
    /// The value to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub equals: Option<bool>,
}

setters!(BooleanFilter { equals: bool });

impl From<bool> for BooleanFilter {
    fn from(value: bool) -> Self {
        Self::default().equals(value)
    }
}

/// A predefined period ending now, for [`DateTimeFilter::period`].
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub enum TimeRangeFilter {
    /// The last week.
    LastWeek,
    /// The last month.
    LastMonth,
    /// The last three months.
    #[cynic(rename = "LAST_3_MONTHS")]
    Last3Months,
    /// The last six months.
    #[cynic(rename = "LAST_6_MONTHS")]
    Last6Months,
    /// The last year.
    LastYear,
    /// A value this version of the SDK does not know. The string is sent
    /// unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// Matches a start date and time by a predefined period. Converts from a
/// [`TimeRangeFilter`].
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct DateTimeFilter {
    /// The period to match.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub period: Option<TimeRangeFilter>,
}

impl DateTimeFilter {
    /// Matches the given period.
    #[must_use]
    pub fn period(period: TimeRangeFilter) -> Self {
        Self {
            period: Some(period),
        }
    }
}

impl From<TimeRangeFilter> for DateTimeFilter {
    fn from(period: TimeRangeFilter) -> Self {
        Self::period(period)
    }
}

/// Adds the constructor, setters and conversions of a tournament filter.
macro_rules! tournament_filter {
    ($name:ident) => {
        setters!($name {
            id: IdFilter,
            include_children: bool,
        });

        impl $name {
            /// Matches the given tournaments and their child tournaments.
            #[must_use]
            pub fn new(ids: impl Into<IdFilter>) -> Self {
                Self::default().id(ids)
            }
        }

        impl From<IdFilter> for $name {
            fn from(ids: IdFilter) -> Self {
                Self::new(ids)
            }
        }

        impl<T: Into<Id>, const N: usize> From<[T; N]> for $name {
            fn from(ids: [T; N]) -> Self {
                Self::new(ids)
            }
        }
    };
}

/// Matches tournaments by ID for [`TeamStatisticsFilter::tournament`](TeamStatisticsFilter::tournament), by
/// default including their child tournaments. Converts from an [`IdFilter`]
/// or an array of tournament IDs.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct TeamStatisticsTournamentFilter {
    /// The tournament IDs to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Whether child tournaments match too; the server's default is `true`.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub include_children: Option<bool>,
}

tournament_filter!(TeamStatisticsTournamentFilter);

/// Matches tournaments by ID for [`PlayerStatisticsFilter::tournament`](PlayerStatisticsFilter::tournament), by
/// default including their child tournaments. Converts from an [`IdFilter`]
/// or an array of tournament IDs.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct PlayerStatisticsTournamentFilter {
    /// The tournament IDs to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Whether child tournaments match too; the server's default is `true`.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub include_children: Option<bool>,
}

tournament_filter!(PlayerStatisticsTournamentFilter);

/// Matches tournaments by ID for [`SeriesStatisticsFilter::tournament`](SeriesStatisticsFilter::tournament), by
/// default including their child tournaments. Converts from an [`IdFilter`]
/// or an array of tournament IDs.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct SeriesStatisticsTournamentFilter {
    /// The tournament IDs to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Whether child tournaments match too; the server's default is `true`.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub include_children: Option<bool>,
}

tournament_filter!(SeriesStatisticsTournamentFilter);

/// Matches tournaments by ID for [`GameStatisticsFilter::tournament`](GameStatisticsFilter::tournament), by
/// default including their child tournaments. Converts from an [`IdFilter`]
/// or an array of tournament IDs.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameStatisticsTournamentFilter {
    /// The tournament IDs to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Whether child tournaments match too; the server's default is `true`.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub include_children: Option<bool>,
}

tournament_filter!(GameStatisticsTournamentFilter);

/// Matches tournaments by ID for [`GameStateFilter::tournament`](GameStateFilter::tournament), by
/// default including their child tournaments. Converts from an [`IdFilter`]
/// or an array of tournament IDs.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct TeamGameStatisticsTournamentFilter {
    /// The tournament IDs to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Whether child tournaments match too; the server's default is `true`.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub include_children: Option<bool>,
}

tournament_filter!(TeamGameStatisticsTournamentFilter);

// ---------------------------------------------------------------------------
// Root filters

/// Filter for [`Stats::team_statistics`](super::Stats::team_statistics).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct TeamStatisticsFilter {
    /// Matches series by start date and time.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTimeFilter>,
    /// Matches series by tournament.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<TeamStatisticsTournamentFilter>,
}

setters!(TeamStatisticsFilter {
    started_at: DateTimeFilter,
    tournament: TeamStatisticsTournamentFilter,
});

/// Filter for [`Stats::player_statistics`](super::Stats::player_statistics).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct PlayerStatisticsFilter {
    /// Matches series by start date and time.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTimeFilter>,
    /// Matches series by tournament.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<PlayerStatisticsTournamentFilter>,
}

setters!(PlayerStatisticsFilter {
    started_at: DateTimeFilter,
    tournament: PlayerStatisticsTournamentFilter,
});

/// Filter for [`Stats::series_statistics`](super::Stats::series_statistics).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct SeriesStatisticsFilter {
    /// Matches series by start date.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTimeFilter>,
    /// Matches series by tournament.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<SeriesStatisticsTournamentFilter>,
}

setters!(SeriesStatisticsFilter {
    started_at: DateTimeFilter,
    tournament: SeriesStatisticsTournamentFilter,
});

/// Filter for [`Stats::game_statistics`](super::Stats::game_statistics).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameStatisticsFilter {
    /// Matches games by start date.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTimeFilter>,
    /// Matches games by tournament.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<GameStatisticsTournamentFilter>,
    /// Matches games by game version ID.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub version: Option<GameStatisticsVersionFilter>,
}

setters!(GameStatisticsFilter {
    started_at: DateTimeFilter,
    tournament: GameStatisticsTournamentFilter,
    version: GameStatisticsVersionFilter,
});

/// Matches games by game version ID. Converts from an [`IdFilter`] or an
/// array of version IDs.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameStatisticsVersionFilter {
    /// The version IDs to look for.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
}

setters!(GameStatisticsVersionFilter { id: IdFilter });

impl From<IdFilter> for GameStatisticsVersionFilter {
    fn from(ids: IdFilter) -> Self {
        Self::default().id(ids)
    }
}

impl<T: Into<Id>, const N: usize> From<[T; N]> for GameStatisticsVersionFilter {
    fn from(ids: [T; N]) -> Self {
        Self::default().id(ids)
    }
}

// ---------------------------------------------------------------------------
// Team game selection

/// The games of a team to aggregate, for
/// [`Stats::team_game_statistics`](super::Stats::team_game_statistics).
///
/// ```
/// use gridgg::stats::{GameOrder, GameSelection, GameStateFilter, MapFilter, StringFilter};
///
/// // The last 20 games on Ascent.
/// let selection = GameSelection::default()
///     .filter(GameStateFilter::default().map(MapFilter::name(StringFilter::equals("Ascent"))))
///     .first(20)
///     .order_by([GameOrder::started_at_desc()]);
/// # let _ = selection;
/// ```
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameSelection {
    /// Matches games by their data points.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub filter: Option<GameStateFilter>,
    /// The number of games to select.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub first: Option<i32>,
    /// The order to select games in.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<Vec<GameOrder>>,
}

impl GameSelection {
    /// Sets [`filter`](#structfield.filter).
    #[must_use]
    pub fn filter(mut self, filter: GameStateFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Sets [`first`](#structfield.first).
    #[must_use]
    pub fn first(mut self, first: i32) -> Self {
        self.first = Some(first);
        self
    }

    /// Sets [`order_by`](#structfield.order_by).
    #[must_use]
    pub fn order_by(mut self, order: impl IntoIterator<Item = GameOrder>) -> Self {
        self.order_by = Some(order.into_iter().collect());
        self
    }
}

/// A sort order for [`GameSelection::order_by`].
#[derive(cynic::InputObject, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameOrder {
    /// The field to sort by.
    pub field: GameOrderField,
    /// The sort direction.
    pub direction: OrderDirection,
}

impl GameOrder {
    /// Sorts by `field` in `direction`.
    #[must_use]
    pub fn new(field: GameOrderField, direction: OrderDirection) -> Self {
        Self { field, direction }
    }

    /// Most recently started games first.
    #[must_use]
    pub fn started_at_desc() -> Self {
        Self::new(GameOrderField::StartedAt, OrderDirection::Desc)
    }
}

/// A field to sort games by.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub enum GameOrderField {
    /// The game's start time.
    StartedAt,
    /// A value this version of the SDK does not know. The string is sent
    /// unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// A sort direction.
#[derive(cynic::Enum, Debug, Clone, PartialEq, Eq, Hash)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub enum OrderDirection {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
    /// A value this version of the SDK does not know. The string is sent
    /// unchanged.
    #[cynic(fallback)]
    Other(String),
}

/// Matches games by their data points. All set fields must match; combine
/// filters with [`and`](#structfield.and) and [`or`](#structfield.or).
///
/// The schema allows [`started_at`](#structfield.started_at) only once in a
/// filter hierarchy.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameStateFilter {
    /// Filters that must all match too.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub and: Option<Vec<GameStateFilter>>,
    /// Filters of which at least one must match too.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub or: Option<Vec<GameStateFilter>>,
    /// Matches games by start date and time.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTimeFilter>,
    /// Matches games by tournament.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<TeamGameStatisticsTournamentFilter>,
    /// Matches games by map.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub map: Option<MapFilter>,
    /// Matches games by title version.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title_version: Option<TitleVersionFilter>,
    /// Matches games by participating team.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub teams: Option<GameTeamStateFilter>,
    /// Matches games by segment data points.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub segments: Option<SegmentStateFilter>,
}

setters!(GameStateFilter {
    started_at: DateTimeFilter,
    tournament: TeamGameStatisticsTournamentFilter,
    map: MapFilter,
    title_version: TitleVersionFilter,
    teams: GameTeamStateFilter,
    segments: SegmentStateFilter,
});

impl GameStateFilter {
    /// Sets [`and`](#structfield.and).
    #[must_use]
    pub fn and(mut self, filters: impl IntoIterator<Item = GameStateFilter>) -> Self {
        self.and = Some(filters.into_iter().collect());
        self
    }

    /// Sets [`or`](#structfield.or).
    #[must_use]
    pub fn or(mut self, filters: impl IntoIterator<Item = GameStateFilter>) -> Self {
        self.or = Some(filters.into_iter().collect());
        self
    }
}

/// Matches games by map.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct MapFilter {
    /// Matches the map name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
}

impl MapFilter {
    /// Matches maps whose name matches `name`.
    #[must_use]
    pub fn name(name: StringFilter) -> Self {
        Self { name: Some(name) }
    }
}

/// Matches games by title version.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct TitleVersionFilter {
    /// Matches the title version name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
}

impl TitleVersionFilter {
    /// Matches title versions whose name matches `name`.
    #[must_use]
    pub fn name(name: StringFilter) -> Self {
        Self { name: Some(name) }
    }
}

/// Matches games by segment data points.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct SegmentStateFilter {
    /// Matches the segment type.
    #[cynic(rename = "type", skip_serializing_if = "Option::is_none")]
    pub segment_type: Option<StringFilter>,
    /// Matches whether the first segment was won.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub won_first: Option<BooleanFilter>,
}

setters!(SegmentStateFilter {
    segment_type: StringFilter,
    won_first: BooleanFilter,
});

/// Matches games by a participating team's data points.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GameTeamStateFilter {
    /// Matches the team ID.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Matches the team's side.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub side: Option<StringFilter>,
    /// Matches whether the team got the first kill.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub first_kill: Option<BooleanFilter>,
    /// Matches the team's objectives.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub objectives: Option<ObjectiveFilter>,
    /// Matches the team's players.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub players: Option<GamePlayerStateFilter>,
}

setters!(GameTeamStateFilter {
    id: IdFilter,
    side: StringFilter,
    first_kill: BooleanFilter,
    objectives: ObjectiveFilter,
    players: GamePlayerStateFilter,
});

/// Matches games by a team's objectives.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct ObjectiveFilter {
    /// Matches the objective type.
    #[cynic(rename = "type", skip_serializing_if = "Option::is_none")]
    pub objective_type: Option<StringFilter>,
    /// Matches whether the objective was completed first.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub completed_first: Option<BooleanFilter>,
}

setters!(ObjectiveFilter {
    objective_type: StringFilter,
    completed_first: BooleanFilter,
});

/// Matches games by a team's players.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct GamePlayerStateFilter {
    /// Matches the character a player played.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub character: Option<CharacterFilter>,
}

setters!(GamePlayerStateFilter {
    character: CharacterFilter
});

/// Matches a character by ID or name.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "stats", schema_module = "crate::stats::schema")]
#[non_exhaustive]
pub struct CharacterFilter {
    /// Matches the character ID.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Matches the character name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
}

setters!(CharacterFilter {
    id: IdFilter,
    name: StringFilter,
});
