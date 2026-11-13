//! Prebuilt operations for the 22 root fields of the Central Data API and the
//! [`Central`] methods that send them.

use cynic::{Id, QueryBuilder};
use futures_util::Stream;

use super::Central;
use super::filters::{
    OrderDirection, OrganizationFilter, PlayerFilter, PlayerRoleFilter, SeriesFilter,
    SeriesOrderBy, TeamFilter, TitleFilter, TournamentFilter,
};
use super::types::{
    DataProvider, Organization, Player, PlayerRole, Series, SeriesFormat, Team, Title, Tournament,
};
use crate::graphql::{self, Api};
use crate::pagination::{self, Page, PageInfo, PageRequest};
use crate::{Client, Result};

// ---------------------------------------------------------------------------
// Variables

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct IdVariables {
    pub(crate) id: Id,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct TitlesVariables {
    pub(crate) filter: TitleFilter,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct PlayerRolesVariables {
    pub(crate) filter: PlayerRoleFilter,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct ExternalIdVariables {
    pub(crate) data_provider_name: String,
    pub(crate) external_id: Id,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct PlayerIdByExternalIdVariables {
    pub(crate) data_provider_name: String,
    pub(crate) external_id: Id,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) title_id: Option<Id>,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct TeamIdByExternalIdVariables {
    pub(crate) data_provider_name: String,
    pub(crate) external_id: Id,
    pub(crate) title_id: Id,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct SeriesByExternalSeriesVariables {
    pub(crate) data_provider_name: String,
    pub(crate) external_series_id: String,
}

/// Sets the four paging variables of a connection query from a
/// [`PageRequest`].
macro_rules! paging_variables {
    ($name:ident { $($field:ident : $ty:ident),* $(,)? }) => {
        impl $name {
            fn new($($field: $ty),*) -> Self {
                Self { $($field,)* first: None, after: None, last: None, before: None }
            }

            fn with_page(mut self, page: &PageRequest) -> Self {
                let size = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
                self.first = page.first().map(size);
                self.after = page.after().map(str::to_owned);
                self.last = page.last().map(size);
                self.before = page.before().map(str::to_owned);
                self
            }
        }
    };
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct AllSeriesVariables {
    pub(crate) filter: SeriesFilter,
    pub(crate) order_by: SeriesOrderBy,
    pub(crate) order_direction: OrderDirection,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) last: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<String>,
}

paging_variables!(AllSeriesVariables {
    filter: SeriesFilter,
    order_by: SeriesOrderBy,
    order_direction: OrderDirection,
});

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct OrganizationsVariables {
    pub(crate) filter: OrganizationFilter,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) last: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<String>,
}

paging_variables!(OrganizationsVariables {
    filter: OrganizationFilter,
});

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct PlayersVariables {
    pub(crate) filter: PlayerFilter,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) last: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<String>,
}

paging_variables!(PlayersVariables {
    filter: PlayerFilter,
});

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct TeamsVariables {
    pub(crate) filter: TeamFilter,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) last: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<String>,
}

paging_variables!(TeamsVariables { filter: TeamFilter });

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::central::schema")]
pub(crate) struct TournamentsVariables {
    pub(crate) filter: TournamentFilter,
    pub(crate) order_direction: OrderDirection,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) after: Option<String>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) last: Option<i32>,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) before: Option<String>,
}

paging_variables!(TournamentsVariables {
    filter: TournamentFilter,
    order_direction: OrderDirection,
});

// ---------------------------------------------------------------------------
// Connections

/// Converts a connection (edges, `pageInfo`, `totalCount`) into a [`Page`].
macro_rules! connection_into_page {
    ($connection:ident, $node:ident) => {
        impl From<$connection> for Page<$node> {
            fn from(connection: $connection) -> Self {
                Page {
                    items: connection.edges.into_iter().map(|edge| edge.node).collect(),
                    page_info: connection.page_info,
                    total_count: u64::try_from(connection.total_count).ok(),
                }
            }
        }
    };
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct SeriesConnection {
    edges: Vec<SeriesEdge>,
    page_info: PageInfo,
    total_count: i32,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct SeriesEdge {
    node: Series,
}

connection_into_page!(SeriesConnection, Series);

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct OrganizationConnection {
    edges: Vec<OrganizationEdge>,
    page_info: PageInfo,
    total_count: i32,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct OrganizationEdge {
    node: Organization,
}

connection_into_page!(OrganizationConnection, Organization);

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct PlayerConnection {
    edges: Vec<PlayerEdge>,
    page_info: PageInfo,
    total_count: i32,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct PlayerEdge {
    node: Player,
}

connection_into_page!(PlayerConnection, Player);

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct TeamConnection {
    edges: Vec<TeamEdge>,
    page_info: PageInfo,
    total_count: i32,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct TeamEdge {
    node: Team,
}

connection_into_page!(TeamConnection, Team);

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct TournamentConnection {
    edges: Vec<TournamentEdge>,
    page_info: PageInfo,
    total_count: i32,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema = "central", schema_module = "crate::central::schema")]
pub(crate) struct TournamentEdge {
    node: Tournament,
}

connection_into_page!(TournamentConnection, Tournament);

// ---------------------------------------------------------------------------
// Root queries

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "AllSeriesVariables"
)]
pub(crate) struct AllSeriesQuery {
    #[arguments(
        filter: $filter,
        orderBy: $order_by,
        orderDirection: $order_direction,
        first: $first,
        after: $after,
        last: $last,
        before: $before
    )]
    all_series: SeriesConnection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query"
)]
pub(crate) struct DataProvidersQuery {
    data_providers: Vec<DataProvider>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "ExternalIdVariables"
)]
pub(crate) struct GameIdByExternalIdQuery {
    #[arguments(dataProviderName: $data_provider_name, externalGameId: $external_id)]
    game_id_by_external_id: Option<Id>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct OrganizationQuery {
    #[arguments(id: $id)]
    organization: Option<Organization>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "OrganizationsVariables"
)]
pub(crate) struct OrganizationsQuery {
    #[arguments(filter: $filter, first: $first, after: $after, last: $last, before: $before)]
    organizations: OrganizationConnection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct PlayerQuery {
    #[arguments(id: $id)]
    player: Option<Player>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "PlayerIdByExternalIdVariables"
)]
pub(crate) struct PlayerIdByExternalIdQuery {
    #[arguments(
        dataProviderName: $data_provider_name,
        externalPlayerId: $external_id,
        titleId: $title_id
    )]
    player_id_by_external_id: Option<Id>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct PlayerRoleQuery {
    #[arguments(id: $id)]
    player_role: Option<PlayerRole>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "PlayerRolesVariables"
)]
pub(crate) struct PlayerRolesQuery {
    #[arguments(filter: $filter)]
    player_roles: Vec<PlayerRole>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "PlayersVariables"
)]
pub(crate) struct PlayersQuery {
    #[arguments(filter: $filter, first: $first, after: $after, last: $last, before: $before)]
    players: PlayerConnection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct SeriesQuery {
    #[arguments(id: $id)]
    series: Option<Series>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "SeriesByExternalSeriesVariables"
)]
pub(crate) struct SeriesByExternalSeriesQuery {
    #[arguments(dataProviderName: $data_provider_name, externalSeriesId: $external_series_id)]
    series_by_external_series: Option<Series>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query"
)]
pub(crate) struct SeriesFormatsQuery {
    series_formats: Vec<SeriesFormat>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "ExternalIdVariables"
)]
pub(crate) struct SeriesIdByExternalIdQuery {
    #[arguments(dataProviderName: $data_provider_name, externalSeriesId: $external_id)]
    series_id_by_external_id: Option<Id>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct TeamQuery {
    #[arguments(id: $id)]
    team: Option<Team>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "TeamIdByExternalIdVariables"
)]
pub(crate) struct TeamIdByExternalIdQuery {
    #[arguments(
        dataProviderName: $data_provider_name,
        externalTeamId: $external_id,
        titleId: $title_id
    )]
    team_id_by_external_id: Option<Id>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "TeamsVariables"
)]
pub(crate) struct TeamsQuery {
    #[arguments(filter: $filter, first: $first, after: $after, last: $last, before: $before)]
    teams: TeamConnection,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct TitleQuery {
    #[arguments(id: $id)]
    title: Option<Title>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "TitlesVariables"
)]
pub(crate) struct TitlesQuery {
    #[arguments(filter: $filter)]
    titles: Vec<Title>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct TournamentQuery {
    #[arguments(id: $id)]
    tournament: Option<Tournament>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "ExternalIdVariables"
)]
pub(crate) struct TournamentIdByExternalIdQuery {
    #[arguments(dataProviderName: $data_provider_name, externalTournamentId: $external_id)]
    tournament_id_by_external_id: Option<Id>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "central",
    schema_module = "crate::central::schema",
    graphql_type = "Query",
    variables = "TournamentsVariables"
)]
pub(crate) struct TournamentsQuery {
    #[arguments(
        filter: $filter,
        orderDirection: $order_direction,
        first: $first,
        after: $after,
        last: $last,
        before: $before
    )]
    tournaments: TournamentConnection,
}

// ---------------------------------------------------------------------------
// Connection requests

/// Defines the request type of a connection query, with `page_request`,
/// `page` and `stream`.
macro_rules! connection_request {
    (
        $(#[$meta:meta])*
        $name:ident {
            query: $query:ident,
            variables: $variables:ident,
            field: $field:ident,
            node: $node:ident,
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        #[must_use = "a request does nothing until `page` or `stream` is called"]
        pub struct $name {
            client: Client,
            variables: $variables,
            page: PageRequest,
        }

        impl $name {
            fn new(client: Client, variables: $variables) -> Self {
                Self {
                    client,
                    variables,
                    page: PageRequest::default(),
                }
            }

            /// Sets the page [`page`](Self::page) fetches and
            /// [`stream`](Self::stream) starts from. The default is
            /// [`PageRequest::default`]: the first 50 items.
            pub fn page_request(mut self, page: PageRequest) -> Self {
                self.page = page;
                self
            }

            /// Fetches the page selected by
            /// [`page_request`](Self::page_request), with the connection's
            /// `totalCount`.
            ///
            /// # Errors
            ///
            /// Returns [`Error::Config`](crate::Error::Config) for an
            /// out-of-range page size, and the errors of
            /// [`Central::run`](super::Central::run).
            pub async fn page(&self) -> Result<Page<$node>> {
                self.page.validate()?;
                self.fetch(self.page.clone()).await
            }

            /// Streams every item from the page selected by
            /// [`page_request`](Self::page_request) onwards (in its
            /// direction), one request per page.
            ///
            /// The stream ends after the last page, or after yielding the
            /// first error.
            pub fn stream(&self) -> impl Stream<Item = Result<$node>> + Send + 'static {
                let request = self.clone();
                pagination::paginate(self.page.clone(), move |page| {
                    let request = request.clone();
                    async move { request.fetch(page).await }
                })
            }

            async fn fetch(&self, page: PageRequest) -> Result<Page<$node>> {
                let operation = $query::build(self.variables.clone().with_page(&page));
                let data = graphql::run(&self.client, Api::Central, &operation).await?;
                Ok(data.$field.into())
            }
        }
    };
}

connection_request! {
    /// A paged [`Central::all_series`] query; see
    /// [`page`](Self::page) and [`stream`](Self::stream).
    AllSeriesRequest {
        query: AllSeriesQuery,
        variables: AllSeriesVariables,
        field: all_series,
        node: Series,
    }
}

impl AllSeriesRequest {
    /// Sets the field to order by (default [`SeriesOrderBy::Id`]).
    pub fn order_by(mut self, order_by: SeriesOrderBy) -> Self {
        self.variables.order_by = order_by;
        self
    }

    /// Sets the sort direction (default [`OrderDirection::Asc`]).
    pub fn order_direction(mut self, direction: OrderDirection) -> Self {
        self.variables.order_direction = direction;
        self
    }
}

connection_request! {
    /// A paged [`Central::organizations`] query; see
    /// [`page`](Self::page) and [`stream`](Self::stream).
    OrganizationsRequest {
        query: OrganizationsQuery,
        variables: OrganizationsVariables,
        field: organizations,
        node: Organization,
    }
}

connection_request! {
    /// A paged [`Central::players`] query; see
    /// [`page`](Self::page) and [`stream`](Self::stream).
    PlayersRequest {
        query: PlayersQuery,
        variables: PlayersVariables,
        field: players,
        node: Player,
    }
}

connection_request! {
    /// A paged [`Central::teams`] query; see
    /// [`page`](Self::page) and [`stream`](Self::stream).
    TeamsRequest {
        query: TeamsQuery,
        variables: TeamsVariables,
        field: teams,
        node: Team,
    }
}

connection_request! {
    /// A paged [`Central::tournaments`] query, ordered by ID; see
    /// [`page`](Self::page) and [`stream`](Self::stream).
    TournamentsRequest {
        query: TournamentsQuery,
        variables: TournamentsVariables,
        field: tournaments,
        node: Tournament,
    }
}

impl TournamentsRequest {
    /// Sets the direction of the ID ordering (default
    /// [`OrderDirection::Asc`]).
    pub fn order_direction(mut self, direction: OrderDirection) -> Self {
        self.variables.order_direction = direction;
        self
    }
}

// ---------------------------------------------------------------------------
// Central methods

fn external_id_variables(
    data_provider: impl Into<String>,
    external_id: impl Into<Id>,
) -> ExternalIdVariables {
    ExternalIdVariables {
        data_provider_name: data_provider.into(),
        external_id: external_id.into(),
    }
}

/// Prebuilt queries, one per root field of the Central Data schema.
///
/// Single-object queries return `Ok(None)` when the server answers `null`.
/// GRID may instead reject an unknown ID with
/// [`Error::GraphQl`](crate::Error::GraphQl) (`NOT_FOUND` or `BAD_REQUEST`).
impl Central {
    /// Lists series matching `filter` (the `allSeries` field), ordered by ID
    /// ascending unless changed on the returned request.
    ///
    /// ```no_run
    /// # async fn example(client: gridgg::Client) -> gridgg::Result<()> {
    /// use futures_util::TryStreamExt;
    /// use gridgg::central::{SeriesFilter, SeriesOrderBy};
    ///
    /// let filter = SeriesFilter::default().title_ids(["3"]);
    /// let series: Vec<_> = client
    ///     .central()
    ///     .all_series(filter)
    ///     .order_by(SeriesOrderBy::StartTimeScheduled)
    ///     .stream()
    ///     .try_collect()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_series(&self, filter: SeriesFilter) -> AllSeriesRequest {
        AllSeriesRequest::new(
            self.client.clone(),
            AllSeriesVariables::new(filter, SeriesOrderBy::default(), OrderDirection::default()),
        )
    }

    /// Lists the supported external data providers (the `dataProviders`
    /// field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn data_providers(&self) -> Result<Vec<DataProvider>> {
        Ok(self
            .run(&DataProvidersQuery::build(()))
            .await?
            .data_providers)
    }

    /// Looks up the GRID game ID for an external provider's game ID (the
    /// `gameIdByExternalId` field). `data_provider` is a
    /// [`DataProvider::name`], for example `"STEAM"`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn game_id_by_external_id(
        &self,
        data_provider: impl Into<String>,
        external_id: impl Into<Id>,
    ) -> Result<Option<Id>> {
        let operation =
            GameIdByExternalIdQuery::build(external_id_variables(data_provider, external_id));
        Ok(self.run(&operation).await?.game_id_by_external_id)
    }

    /// Fetches an organization by ID (the `organization` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn organization(&self, id: impl Into<Id>) -> Result<Option<Organization>> {
        let operation = OrganizationQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.organization)
    }

    /// Lists organizations matching `filter` (the `organizations` field).
    pub fn organizations(&self, filter: OrganizationFilter) -> OrganizationsRequest {
        OrganizationsRequest::new(self.client.clone(), OrganizationsVariables::new(filter))
    }

    /// Fetches a player by ID (the `player` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn player(&self, id: impl Into<Id>) -> Result<Option<Player>> {
        let operation = PlayerQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.player)
    }

    /// Looks up the GRID player ID for an external provider's player ID (the
    /// `playerIdByExternalId` field), optionally restricted to one title.
    /// `data_provider` is a [`DataProvider::name`], for example `"STEAM"`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn player_id_by_external_id(
        &self,
        data_provider: impl Into<String>,
        external_id: impl Into<Id>,
        title_id: Option<Id>,
    ) -> Result<Option<Id>> {
        let operation = PlayerIdByExternalIdQuery::build(PlayerIdByExternalIdVariables {
            data_provider_name: data_provider.into(),
            external_id: external_id.into(),
            title_id,
        });
        Ok(self.run(&operation).await?.player_id_by_external_id)
    }

    /// Fetches a player role by ID (the `playerRole` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn player_role(&self, id: impl Into<Id>) -> Result<Option<PlayerRole>> {
        let operation = PlayerRoleQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.player_role)
    }

    /// Lists player roles matching `filter` (the `playerRoles` field; not
    /// paged).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn player_roles(&self, filter: PlayerRoleFilter) -> Result<Vec<PlayerRole>> {
        let operation = PlayerRolesQuery::build(PlayerRolesVariables { filter });
        Ok(self.run(&operation).await?.player_roles)
    }

    /// Lists players matching `filter` (the `players` field).
    pub fn players(&self, filter: PlayerFilter) -> PlayersRequest {
        PlayersRequest::new(self.client.clone(), PlayersVariables::new(filter))
    }

    /// Fetches a series by ID (the `series` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn series(&self, id: impl Into<Id>) -> Result<Option<Series>> {
        let operation = SeriesQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.series)
    }

    /// Fetches a series by an external provider's series ID (the deprecated
    /// `seriesByExternalSeries` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    #[deprecated(note = "GRID deprecated `seriesByExternalSeries`; use \
                         `series_id_by_external_id` and then `series`")]
    pub async fn series_by_external_series(
        &self,
        data_provider: impl Into<String>,
        external_series_id: impl Into<String>,
    ) -> Result<Option<Series>> {
        let operation = SeriesByExternalSeriesQuery::build(SeriesByExternalSeriesVariables {
            data_provider_name: data_provider.into(),
            external_series_id: external_series_id.into(),
        });
        Ok(self.run(&operation).await?.series_by_external_series)
    }

    /// Lists the supported series formats (the `seriesFormats` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn series_formats(&self) -> Result<Vec<SeriesFormat>> {
        Ok(self
            .run(&SeriesFormatsQuery::build(()))
            .await?
            .series_formats)
    }

    /// Looks up the GRID series ID for an external provider's series ID (the
    /// `seriesIdByExternalId` field). `data_provider` is a
    /// [`DataProvider::name`], for example `"STEAM"`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn series_id_by_external_id(
        &self,
        data_provider: impl Into<String>,
        external_id: impl Into<Id>,
    ) -> Result<Option<Id>> {
        let operation =
            SeriesIdByExternalIdQuery::build(external_id_variables(data_provider, external_id));
        Ok(self.run(&operation).await?.series_id_by_external_id)
    }

    /// Fetches a team by ID (the `team` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn team(&self, id: impl Into<Id>) -> Result<Option<Team>> {
        let operation = TeamQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.team)
    }

    /// Looks up the GRID team ID for an external provider's team ID within a
    /// title (the `teamIdByExternalId` field). `data_provider` is a
    /// [`DataProvider::name`], for example `"STEAM"`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn team_id_by_external_id(
        &self,
        data_provider: impl Into<String>,
        external_id: impl Into<Id>,
        title_id: impl Into<Id>,
    ) -> Result<Option<Id>> {
        let operation = TeamIdByExternalIdQuery::build(TeamIdByExternalIdVariables {
            data_provider_name: data_provider.into(),
            external_id: external_id.into(),
            title_id: title_id.into(),
        });
        Ok(self.run(&operation).await?.team_id_by_external_id)
    }

    /// Lists teams matching `filter` (the `teams` field).
    pub fn teams(&self, filter: TeamFilter) -> TeamsRequest {
        TeamsRequest::new(self.client.clone(), TeamsVariables::new(filter))
    }

    /// Fetches a title by ID (the `title` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn title(&self, id: impl Into<Id>) -> Result<Option<Title>> {
        let operation = TitleQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.title)
    }

    /// Lists titles matching `filter` (the `titles` field; not paged).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn titles(&self, filter: TitleFilter) -> Result<Vec<Title>> {
        let operation = TitlesQuery::build(TitlesVariables { filter });
        Ok(self.run(&operation).await?.titles)
    }

    /// Fetches a tournament by ID (the `tournament` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn tournament(&self, id: impl Into<Id>) -> Result<Option<Tournament>> {
        let operation = TournamentQuery::build(IdVariables { id: id.into() });
        Ok(self.run(&operation).await?.tournament)
    }

    /// Looks up the GRID tournament ID for an external provider's tournament
    /// ID (the `tournamentIdByExternalId` field). `data_provider` is a
    /// [`DataProvider::name`], for example `"STEAM"`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn tournament_id_by_external_id(
        &self,
        data_provider: impl Into<String>,
        external_id: impl Into<Id>,
    ) -> Result<Option<Id>> {
        let operation =
            TournamentIdByExternalIdQuery::build(external_id_variables(data_provider, external_id));
        Ok(self.run(&operation).await?.tournament_id_by_external_id)
    }

    /// Lists tournaments matching `filter` (the `tournaments` field), ordered
    /// by ID ascending unless changed on the returned request.
    pub fn tournaments(&self, filter: TournamentFilter) -> TournamentsRequest {
        TournamentsRequest::new(
            self.client.clone(),
            TournamentsVariables::new(filter, OrderDirection::default()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Also checks that the null-filling parser understands the query.
    fn assert_selects<Q, V>(operation: &cynic::Operation<Q, V>, field: &str) {
        assert!(
            operation.query.contains(field),
            "{field} missing from {}",
            operation.query
        );
        assert!(
            crate::graphql::selection::parse(&operation.query).is_some(),
            "selection parser rejects {}",
            operation.query
        );
    }

    #[test]
    fn every_prebuilt_query_builds() {
        let id = || IdVariables { id: Id::new("1") };
        let external = || external_id_variables("STEAM", "9");
        assert_selects(
            &AllSeriesQuery::build(AllSeriesVariables::new(
                SeriesFilter::default(),
                SeriesOrderBy::default(),
                OrderDirection::default(),
            )),
            "allSeries(",
        );
        assert_selects(&DataProvidersQuery::build(()), "dataProviders");
        assert_selects(
            &GameIdByExternalIdQuery::build(external()),
            "gameIdByExternalId(",
        );
        assert_selects(&OrganizationQuery::build(id()), "organization(");
        assert_selects(
            &OrganizationsQuery::build(OrganizationsVariables::new(OrganizationFilter::default())),
            "organizations(",
        );
        assert_selects(&PlayerQuery::build(id()), "player(");
        assert_selects(
            &PlayerIdByExternalIdQuery::build(PlayerIdByExternalIdVariables {
                data_provider_name: "STEAM".into(),
                external_id: Id::new("9"),
                title_id: None,
            }),
            "playerIdByExternalId(",
        );
        assert_selects(&PlayerRoleQuery::build(id()), "playerRole(");
        assert_selects(
            &PlayerRolesQuery::build(PlayerRolesVariables {
                filter: PlayerRoleFilter::default(),
            }),
            "playerRoles(",
        );
        assert_selects(
            &PlayersQuery::build(PlayersVariables::new(PlayerFilter::default())),
            "players(",
        );
        assert_selects(&SeriesQuery::build(id()), "series(");
        assert_selects(
            &SeriesByExternalSeriesQuery::build(SeriesByExternalSeriesVariables {
                data_provider_name: "STEAM".into(),
                external_series_id: "9".into(),
            }),
            "seriesByExternalSeries(",
        );
        assert_selects(&SeriesFormatsQuery::build(()), "seriesFormats");
        assert_selects(
            &SeriesIdByExternalIdQuery::build(external()),
            "seriesIdByExternalId(",
        );
        assert_selects(&TeamQuery::build(id()), "team(");
        assert_selects(
            &TeamIdByExternalIdQuery::build(TeamIdByExternalIdVariables {
                data_provider_name: "STEAM".into(),
                external_id: Id::new("9"),
                title_id: Id::new("3"),
            }),
            "teamIdByExternalId(",
        );
        assert_selects(
            &TeamsQuery::build(TeamsVariables::new(TeamFilter::default())),
            "teams(",
        );
        assert_selects(&TitleQuery::build(id()), "title(");
        assert_selects(
            &TitlesQuery::build(TitlesVariables {
                filter: TitleFilter::default(),
            }),
            "titles(",
        );
        assert_selects(&TournamentQuery::build(id()), "tournament(");
        assert_selects(
            &TournamentIdByExternalIdQuery::build(external()),
            "tournamentIdByExternalId(",
        );
        assert_selects(
            &TournamentsQuery::build(TournamentsVariables::new(
                TournamentFilter::default(),
                OrderDirection::default(),
            )),
            "tournaments(",
        );
    }

    #[test]
    fn paging_variables_follow_the_request() {
        let forward = TeamsVariables::new(TeamFilter::default())
            .with_page(&PageRequest::forward(5, Some("c".into())).unwrap());
        assert_eq!(
            (
                forward.first,
                forward.after.as_deref(),
                forward.last,
                forward.before.as_deref()
            ),
            (Some(5), Some("c"), None, None)
        );
        let backward = TeamsVariables::new(TeamFilter::default())
            .with_page(&PageRequest::backward(7, None).unwrap());
        assert_eq!(
            (
                backward.first,
                backward.after.as_deref(),
                backward.last,
                backward.before.as_deref()
            ),
            (None, None, Some(7), None)
        );
    }
}
