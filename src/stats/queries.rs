//! Prebuilt operations for the five statistics root fields of the Stats Feed
//! API and the [`Stats`] methods that send them.

use cynic::{Id, QueryBuilder};

use super::Stats;
use super::filters::{
    GameSelection, GameStatisticsFilter, PlayerStatisticsFilter, SeriesStatisticsFilter,
    TeamStatisticsFilter,
};
use super::types::{
    GameStatistics, PlayerStatistics, SeriesStatistics, TeamGameStatistics, TeamStatistics,
};
use crate::Result;

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::stats::schema")]
pub(crate) struct TeamStatisticsVariables {
    pub(crate) team_id: Id,
    pub(crate) filter: TeamStatisticsFilter,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Query",
    variables = "TeamStatisticsVariables"
)]
pub(crate) struct TeamStatisticsQuery {
    #[arguments(teamId: $team_id, filter: $filter)]
    team_statistics: TeamStatistics,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::stats::schema")]
pub(crate) struct TeamGameStatisticsVariables {
    pub(crate) team_id: Id,
    #[cynic(skip_serializing_if = "Option::is_none")]
    pub(crate) selection: Option<GameSelection>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Query",
    variables = "TeamGameStatisticsVariables"
)]
pub(crate) struct TeamGameStatisticsQuery {
    #[arguments(teamId: $team_id, selection: $selection)]
    team_game_statistics: TeamGameStatistics,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::stats::schema")]
pub(crate) struct PlayerStatisticsVariables {
    pub(crate) player_id: Id,
    pub(crate) filter: PlayerStatisticsFilter,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Query",
    variables = "PlayerStatisticsVariables"
)]
pub(crate) struct PlayerStatisticsQuery {
    #[arguments(playerId: $player_id, filter: $filter)]
    player_statistics: PlayerStatistics,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::stats::schema")]
pub(crate) struct GameStatisticsVariables {
    pub(crate) title_id: Id,
    pub(crate) filter: GameStatisticsFilter,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Query",
    variables = "GameStatisticsVariables"
)]
pub(crate) struct GameStatisticsQuery {
    #[arguments(titleId: $title_id, filter: $filter)]
    game_statistics: GameStatistics,
}

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::stats::schema")]
pub(crate) struct SeriesStatisticsVariables {
    pub(crate) title_id: Id,
    pub(crate) filter: SeriesStatisticsFilter,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "stats",
    schema_module = "crate::stats::schema",
    graphql_type = "Query",
    variables = "SeriesStatisticsVariables"
)]
pub(crate) struct SeriesStatisticsQuery {
    #[arguments(titleId: $title_id, filter: $filter)]
    series_statistics: SeriesStatistics,
}

impl Stats {
    /// Fetches a team's statistics aggregated over the series matching
    /// `filter` (the `teamStatistics` field).
    ///
    /// The API rejects an empty filter with a "Missing statistics filter"
    /// [`BadRequest`](crate::ErrorType::BadRequest) error; set at least
    /// `started_at` or `tournament`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn team_statistics(
        &self,
        team_id: impl Into<Id>,
        filter: TeamStatisticsFilter,
    ) -> Result<TeamStatistics> {
        let operation = TeamStatisticsQuery::build(TeamStatisticsVariables {
            team_id: team_id.into(),
            filter,
        });
        Ok(self.run(&operation).await?.team_statistics)
    }

    /// Fetches a team's statistics aggregated over the games in `selection`
    /// (the `teamGameStatistics` field). An empty selection
    /// ([`GameSelection::default`]) is left out of the request, so the server
    /// picks the games.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn team_game_statistics(
        &self,
        team_id: impl Into<Id>,
        selection: GameSelection,
    ) -> Result<TeamGameStatistics> {
        let selection = (selection != GameSelection::default()).then_some(selection);
        let operation = TeamGameStatisticsQuery::build(TeamGameStatisticsVariables {
            team_id: team_id.into(),
            selection,
        });
        Ok(self.run(&operation).await?.team_game_statistics)
    }

    /// Fetches a player's statistics aggregated over the series matching
    /// `filter` (the `playerStatistics` field).
    ///
    /// The API rejects an empty filter with a "Missing statistics filter"
    /// [`BadRequest`](crate::ErrorType::BadRequest) error; set at least
    /// `started_at` or `tournament`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn player_statistics(
        &self,
        player_id: impl Into<Id>,
        filter: PlayerStatisticsFilter,
    ) -> Result<PlayerStatistics> {
        let operation = PlayerStatisticsQuery::build(PlayerStatisticsVariables {
            player_id: player_id.into(),
            filter,
        });
        Ok(self.run(&operation).await?.player_statistics)
    }

    /// Fetches statistics aggregated over a title's games matching `filter`
    /// (the `gameStatistics` field).
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn game_statistics(
        &self,
        title_id: impl Into<Id>,
        filter: GameStatisticsFilter,
    ) -> Result<GameStatistics> {
        let operation = GameStatisticsQuery::build(GameStatisticsVariables {
            title_id: title_id.into(),
            filter,
        });
        Ok(self.run(&operation).await?.game_statistics)
    }

    /// Fetches statistics aggregated over a title's series matching `filter`
    /// (the `seriesStatistics` field).
    ///
    /// The API rejects an empty filter with a "Missing statistics filter"
    /// [`BadRequest`](crate::ErrorType::BadRequest) error; set at least
    /// `started_at` or `tournament`.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run).
    pub async fn series_statistics(
        &self,
        title_id: impl Into<Id>,
        filter: SeriesStatisticsFilter,
    ) -> Result<SeriesStatistics> {
        let operation = SeriesStatisticsQuery::build(SeriesStatisticsVariables {
            title_id: title_id.into(),
            filter,
        });
        Ok(self.run(&operation).await?.series_statistics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_prebuilt_query_parses() {
        let id = || Id::new("1");
        let queries = [
            TeamStatisticsQuery::build(TeamStatisticsVariables {
                team_id: id(),
                filter: TeamStatisticsFilter::default(),
            })
            .query,
            TeamGameStatisticsQuery::build(TeamGameStatisticsVariables {
                team_id: id(),
                selection: None,
            })
            .query,
            PlayerStatisticsQuery::build(PlayerStatisticsVariables {
                player_id: id(),
                filter: PlayerStatisticsFilter::default(),
            })
            .query,
            GameStatisticsQuery::build(GameStatisticsVariables {
                title_id: id(),
                filter: GameStatisticsFilter::default(),
            })
            .query,
            SeriesStatisticsQuery::build(SeriesStatisticsVariables {
                title_id: id(),
                filter: SeriesStatisticsFilter::default(),
            })
            .query,
        ];
        for query in queries {
            assert!(
                crate::graphql::selection::parse(&query).is_some(),
                "selection parser rejects {query}"
            );
        }
    }
}
