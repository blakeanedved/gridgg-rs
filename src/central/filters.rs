//! Filter and ordering inputs of the Central Data queries.
//!
//! Every filter implements [`Default`] (no constraint) and has one chainable
//! setter per field, named like the field:
//!
//! ```
//! use gridgg::central::{SeriesFilter, SeriesType, StringFilter, TeamFilter};
//!
//! let series = SeriesFilter::default()
//!     .title_ids(["3"])
//!     .types([SeriesType::Esports])
//!     .start_time_scheduled(
//!         gridgg::central::DateTimeFilter::default().gte("2024-01-01T00:00:00Z"),
//!     );
//! let teams = TeamFilter::default().name(StringFilter::contains("G2"));
//! # let _ = (series, teams);
//! ```
//!
//! Unset fields are left out of the request, so the server applies its own
//! defaults (for example, only `PUBLISHED` series and teams). The exceptions
//! are [`NullableIdFilter`] and [`NullableDateFilter`], whose `null` values
//! are meaningful and always sent.
//!
//! Filter fields the schema marks as deprecated (such as `SeriesFilter.titleId`
//! or `SeriesFilter.type`) are not exposed; use their replacements.

use cynic::Id;
use jiff::civil::Date;

use super::types::{
    SeriesType, SeriesWorkflowStatus, ServiceLevel, TeamWorkflowStatus, TournamentVenueType,
};

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

/// Matches IDs against a list.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
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
#[cynic(schema = "central", schema_module = "crate::central::schema")]
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
#[cynic(schema = "central", schema_module = "crate::central::schema")]
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

/// Matches an integer field by value or inclusive range.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct IntFilter {
    /// Matches values equal to this one.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub equals: Option<i32>,
    /// Matches values greater than or equal to this one.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub gte: Option<i32>,
    /// Matches values less than or equal to this one.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub lte: Option<i32>,
}

setters!(IntFilter {
    equals: i32,
    gte: i32,
    lte: i32,
});

/// Matches a date-time field by an inclusive range of ISO 8601 date-time
/// strings. A [`jiff::Timestamp`]'s `to_string()` is in this format.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct DateTimeFilter {
    /// Start of the range (inclusive), as an ISO 8601 date-time string.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub gte: Option<String>,
    /// End of the range (inclusive), as an ISO 8601 date-time string.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub lte: Option<String>,
}

setters!(DateTimeFilter {
    gte: String,
    lte: String,
});

/// Matches a date field by an inclusive range.
///
/// Both bounds are always sent: if both are `None` (the [`Default`]), the
/// filter matches entities whose date is null.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct NullableDateFilter {
    /// Start of the range (inclusive).
    pub gte: Option<Date>,
    /// End of the range (inclusive).
    pub lte: Option<Date>,
}

impl NullableDateFilter {
    /// Sets the start of the range (inclusive).
    #[must_use]
    pub fn gte(mut self, date: Date) -> Self {
        self.gte = Some(date);
        self
    }

    /// Sets the end of the range (inclusive).
    #[must_use]
    pub fn lte(mut self, date: Date) -> Self {
        self.lte = Some(date);
        self
    }
}

/// Matches a nullable ID field.
///
/// The ID is always sent: `None` (the [`Default`]) matches entities without
/// one, for example players without a team.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct NullableIdFilter {
    /// The ID to look for; `None` matches a null ID.
    pub id: Option<Id>,
}

impl NullableIdFilter {
    /// Matches entities with this ID.
    #[must_use]
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: Some(id.into()),
        }
    }
}

/// Matches series or tournaments by the availability of a GRID product.
#[derive(cynic::InputObject, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct ProductServiceLevelFilter {
    /// The name of the product.
    pub product_name: String,
    /// The availability level.
    pub service_level: ServiceLevel,
}

impl ProductServiceLevelFilter {
    /// Matches `product_name` at `service_level`.
    #[must_use]
    pub fn new(product_name: impl Into<String>, service_level: ServiceLevel) -> Self {
        Self {
            product_name: product_name.into(),
            service_level,
        }
    }
}

/// Matches player roles by title. Converts from an [`IdFilter`] or an array
/// of title IDs.
#[derive(cynic::InputObject, Debug, Clone, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct PlayerRoleTitleFilter {
    /// The title IDs to look for.
    pub id: IdFilter,
}

impl PlayerRoleTitleFilter {
    /// Matches roles of any of the given titles.
    #[must_use]
    pub fn new(title_ids: impl Into<IdFilter>) -> Self {
        Self {
            id: title_ids.into(),
        }
    }
}

impl From<IdFilter> for PlayerRoleTitleFilter {
    fn from(title_ids: IdFilter) -> Self {
        Self::new(title_ids)
    }
}

impl<T: Into<Id>, const N: usize> From<[T; N]> for PlayerRoleTitleFilter {
    fn from(title_ids: [T; N]) -> Self {
        Self::new(title_ids)
    }
}

/// Filter for [`Central::titles`](super::Central::titles).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct TitleFilter {
    /// Filter by visibility.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub private: Option<BooleanFilter>,
}

setters!(TitleFilter {
    private: BooleanFilter,
});

/// Filter for [`Central::player_roles`](super::Central::player_roles).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct PlayerRoleFilter {
    /// Filter by role name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
    /// Filter by role title.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title: Option<PlayerRoleTitleFilter>,
}

setters!(PlayerRoleFilter {
    name: StringFilter,
    title: PlayerRoleTitleFilter,
});

/// Filter for [`Central::organizations`](super::Central::organizations).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct OrganizationFilter {
    /// Filter by organization name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
    /// Filter by visibility.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub private: Option<BooleanFilter>,
    /// Filter by last update.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTimeFilter>,
}

setters!(OrganizationFilter {
    name: StringFilter,
    private: BooleanFilter,
    updated_at: DateTimeFilter,
});

/// Filter for [`Central::teams`](super::Central::teams).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct TeamFilter {
    /// Filter by team name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
    /// Filter by shortened team name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name_shortened: Option<StringFilter>,
    /// Filter by organization ID.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Id>,
    /// Filter by visibility.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub private: Option<BooleanFilter>,
    /// Filter by title ID.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title_id: Option<Id>,
    /// Filter by last update.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTimeFilter>,
    /// Filter by workflow status (the server's default is `PUBLISHED` only).
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub workflow_statuses: Option<Vec<TeamWorkflowStatus>>,
}

setters!(TeamFilter {
    name: StringFilter,
    name_shortened: StringFilter,
    organization_id: Id,
    private: BooleanFilter,
    title_id: Id,
    updated_at: DateTimeFilter,
    workflow_statuses: Vec<TeamWorkflowStatus>,
});

/// Filter for [`Central::players`](super::Central::players).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct PlayerFilter {
    /// Filter by age.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub age: Option<IntFilter>,
    /// Filter by full name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<StringFilter>,
    /// Filter by nationality.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub nationality: Option<NationalityFilter>,
    /// Filter by nickname.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<StringFilter>,
    /// Filter by visibility.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub private: Option<BooleanFilter>,
    /// Filter by role.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub roles: Option<PlayerPlayerRoleFilter>,
    /// Filter by current team ID; a [`NullableIdFilter`] without an ID
    /// matches players without a team.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub team_id_filter: Option<NullableIdFilter>,
    /// Filter by title ID.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title_id: Option<Id>,
    /// Filter by last update.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTimeFilter>,
}

setters!(PlayerFilter {
    age: IntFilter,
    full_name: StringFilter,
    nationality: NationalityFilter,
    nickname: StringFilter,
    private: BooleanFilter,
    roles: PlayerPlayerRoleFilter,
    team_id_filter: NullableIdFilter,
    title_id: Id,
    updated_at: DateTimeFilter,
});

/// Matches players by nationality.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct NationalityFilter {
    /// Filter by ISO 3166-1 alpha-3 country code.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub code: Option<StringFilter>,
    /// Filter by country name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
}

setters!(NationalityFilter {
    code: StringFilter,
    name: StringFilter,
});

/// Matches players by role.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct PlayerPlayerRoleFilter {
    /// Filter by role name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
    /// Filter by role title.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title: Option<PlayerRoleTitleFilter>,
}

setters!(PlayerPlayerRoleFilter {
    name: StringFilter,
    title: PlayerRoleTitleFilter,
});

/// Filter for [`Central::tournaments`](super::Central::tournaments).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct TournamentFilter {
    /// Filter by end date.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NullableDateFilter>,
    /// Filter tournaments that have children; `false` returns only the
    /// bottom level of the hierarchy.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub has_children: Option<BooleanFilter>,
    /// Filter tournaments that have a parent; `false` returns only the
    /// top level of the hierarchy.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub has_parent: Option<BooleanFilter>,
    /// Filter by name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
    /// Filter by shortened name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name_shortened: Option<StringFilter>,
    /// Filter by visibility.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub private: Option<BooleanFilter>,
    /// Filter tournaments with at least one series at this product
    /// service level.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub product_service_levels: Option<ProductServiceLevelFilter>,
    /// Filter by start date.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NullableDateFilter>,
    /// Filter by title.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title: Option<TournamentTitleFilter>,
    /// Filter by last update.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTimeFilter>,
    /// Filter by venue type.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub venue_type: Option<Vec<TournamentVenueType>>,
}

setters!(TournamentFilter {
    end_date: NullableDateFilter,
    has_children: BooleanFilter,
    has_parent: BooleanFilter,
    name: StringFilter,
    name_shortened: StringFilter,
    private: BooleanFilter,
    product_service_levels: ProductServiceLevelFilter,
    start_date: NullableDateFilter,
    title: TournamentTitleFilter,
    updated_at: DateTimeFilter,
    venue_type: Vec<TournamentVenueType>,
});

/// Matches tournaments by title.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct TournamentTitleFilter {
    /// Filter by title IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
}

setters!(TournamentTitleFilter { id: IdFilter });

/// Filter for [`Central::all_series`](super::Central::all_series).
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct SeriesFilter {
    /// Filter by live data.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub live: Option<SeriesLiveFilter>,
    /// Filter by participating player IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub live_player_ids: Option<IdFilter>,
    /// Filter by participating players.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub players: Option<SeriesPlayerFilter>,
    /// Filter by visibility.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub private: Option<BooleanFilter>,
    /// Filter by product service level.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub product_service_levels: Option<ProductServiceLevelFilter>,
    /// Filter by scheduled start time.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub start_time_scheduled: Option<DateTimeFilter>,
    /// Filter by team IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub team_ids: Option<IdFilter>,
    /// Filter by title IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title_ids: Option<IdFilter>,
    /// Filter by tournament.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<SeriesTournamentFilter>,
    /// Filter by series type.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<SeriesType>>,
    /// Filter by last update.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTimeFilter>,
    /// Filter by workflow status (the server's default is `PUBLISHED` only).
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub workflow_statuses: Option<Vec<SeriesWorkflowStatus>>,
}

setters!(SeriesFilter {
    live: SeriesLiveFilter,
    live_player_ids: IdFilter,
    players: SeriesPlayerFilter,
    private: BooleanFilter,
    product_service_levels: ProductServiceLevelFilter,
    start_time_scheduled: DateTimeFilter,
    team_ids: IdFilter,
    title_ids: IdFilter,
    tournament: SeriesTournamentFilter,
    types: Vec<SeriesType>,
    updated_at: DateTimeFilter,
    workflow_statuses: Vec<SeriesWorkflowStatus>,
});

/// Matches series by live data.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct SeriesLiveFilter {
    /// Filter by game.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub games: Option<GameFilter>,
}

setters!(SeriesLiveFilter { games: GameFilter });

/// Matches series by their games.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct GameFilter {
    /// Filter by game IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Filter by map.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub map: Option<MapFilter>,
}

setters!(GameFilter {
    id: IdFilter,
    map: MapFilter,
});

/// Matches games by map.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct MapFilter {
    /// Filter by map name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
}

setters!(MapFilter { name: StringFilter });

/// Matches series by their players.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct SeriesPlayerFilter {
    /// Matches series where at least one participant's age matches.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub age: Option<IntFilter>,
}

setters!(SeriesPlayerFilter { age: IntFilter });

/// Matches series by their tournament.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct SeriesTournamentFilter {
    /// Filter by tournament end date.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NullableDateFilter>,
    /// Filter by tournament IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
    /// Whether child tournaments of the matched tournaments are included.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub include_children: Option<BooleanFilter>,
    /// Filter by tournament name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name: Option<StringFilter>,
    /// Filter by shortened tournament name.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub name_shortened: Option<StringFilter>,
    /// Filter by tournament start date.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NullableDateFilter>,
    /// Filter by tournament title.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub title: Option<SeriesTournamentTitleFilter>,
}

setters!(SeriesTournamentFilter {
    end_date: NullableDateFilter,
    id: IdFilter,
    include_children: BooleanFilter,
    name: StringFilter,
    name_shortened: StringFilter,
    start_date: NullableDateFilter,
    title: SeriesTournamentTitleFilter,
});

/// Matches series by the title of their tournament.
#[derive(cynic::InputObject, Debug, Clone, Default, PartialEq, Eq)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub struct SeriesTournamentTitleFilter {
    /// Filter by title IDs.
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub id: Option<IdFilter>,
}

setters!(SeriesTournamentTitleFilter { id: IdFilter });

/// The field [`Central::all_series`](super::Central::all_series) orders by.
#[derive(cynic::Enum, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum SeriesOrderBy {
    /// Order by ID (the default).
    #[default]
    #[cynic(rename = "ID")]
    Id,
    /// Order by scheduled start time.
    #[cynic(rename = "StartTimeScheduled")]
    StartTimeScheduled,
    /// Order by last update.
    #[cynic(rename = "UpdatedAt")]
    UpdatedAt,
}

/// Sort direction.
#[derive(cynic::Enum, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
#[non_exhaustive]
pub enum OrderDirection {
    /// Ascending (the default).
    #[default]
    Asc,
    /// Descending.
    Desc,
}
