//! Prebuilt operations for the two root fields of the Series State API and
//! the [`SeriesState`] methods that send them.

use cynic::{Id, QueryBuilder};

use super::SeriesState;
use super::types::Series;
use crate::Result;
use crate::error::{Error, ErrorType, GraphQlError};
use crate::scalars::Version;

#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::series_state::schema")]
pub(crate) struct IdVariables {
    pub(crate) id: Id,
}

/// The variables of the full prebuilt queries: the root field's `id` plus
/// one `@include` gate per data-model version that introduced a selected
/// field. `vX_Y` is true when the series' version is at least `X.Y`.
///
/// GRID rejects a whole request that selects a field newer than the
/// series' version, so the fields of [`Series`] and its nested types carry
/// `@include(if: $vX_Y)`. Re-exported so custom queries can embed those
/// fragment types.
#[derive(cynic::QueryVariables, Debug, Clone)]
#[cynic(schema_module = "crate::series_state::schema", rename_all = "None")]
#[allow(clippy::struct_excessive_bools)]
pub struct SeriesStateVariables {
    id: Id,
    v1_1: bool,
    v1_2: bool,
    v1_3: bool,
    v1_4: bool,
    v1_6: bool,
    v1_9: bool,
    v2_1: bool,
    v3_2: bool,
    v3_3: bool,
    v3_7: bool,
    v3_8: bool,
    v3_10: bool,
    v3_11: bool,
    v3_12: bool,
    v3_13: bool,
    v3_14: bool,
    v3_15: bool,
    v3_16: bool,
    v3_20: bool,
    v3_21: bool,
    v3_23: bool,
    v3_24: bool,
    v3_25: bool,
    v3_26: bool,
    v3_27: bool,
    v3_28: bool,
    v3_29: bool,
    v3_30: bool,
    v3_33: bool,
    v3_34: bool,
    v3_35: bool,
    v3_36: bool,
    v3_37: bool,
    v3_38: bool,
    v3_40: bool,
    v3_41: bool,
    v3_42: bool,
    v3_43: bool,
    v3_48: bool,
    v3_49: bool,
    v3_50: bool,
}

impl SeriesStateVariables {
    /// Gates for data of `version` (`(major, minor)`, as returned by
    /// [`Version::major_minor`]); `None` includes every field.
    ///
    /// Use this when embedding [`Series`] or its nested types in your own
    /// cynic query (declare `variables = "gridgg::series_state::SeriesStateVariables"`).
    #[must_use]
    pub fn new(id: Id, version: Option<(u32, u32)>) -> Self {
        let from = |major: u32, minor: u32| version.is_none_or(|v| v >= (major, minor));
        Self {
            id,
            v1_1: from(1, 1),
            v1_2: from(1, 2),
            v1_3: from(1, 3),
            v1_4: from(1, 4),
            v1_6: from(1, 6),
            v1_9: from(1, 9),
            v2_1: from(2, 1),
            v3_2: from(3, 2),
            v3_3: from(3, 3),
            v3_7: from(3, 7),
            v3_8: from(3, 8),
            v3_10: from(3, 10),
            v3_11: from(3, 11),
            v3_12: from(3, 12),
            v3_13: from(3, 13),
            v3_14: from(3, 14),
            v3_15: from(3, 15),
            v3_16: from(3, 16),
            v3_20: from(3, 20),
            v3_21: from(3, 21),
            v3_23: from(3, 23),
            v3_24: from(3, 24),
            v3_25: from(3, 25),
            v3_26: from(3, 26),
            v3_27: from(3, 27),
            v3_28: from(3, 28),
            v3_29: from(3, 29),
            v3_30: from(3, 30),
            v3_33: from(3, 33),
            v3_34: from(3, 34),
            v3_35: from(3, 35),
            v3_36: from(3, 36),
            v3_37: from(3, 37),
            v3_38: from(3, 38),
            v3_40: from(3, 40),
            v3_41: from(3, 41),
            v3_42: from(3, 42),
            v3_43: from(3, 43),
            v3_48: from(3, 48),
            v3_49: from(3, 49),
            v3_50: from(3, 50),
        }
    }
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "Query",
    variables = "SeriesStateVariables"
)]
pub(crate) struct SeriesStateQuery {
    #[arguments(id: $id)]
    series_state: Option<Series>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "Query",
    variables = "SeriesStateVariables"
)]
pub(crate) struct LatestSeriesStateByPlayerIdQuery {
    #[arguments(id: $id)]
    latest_series_state_by_player_id: Option<Series>,
}

/// The ID and data-model version of a series state.
#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "SeriesState"
)]
pub(crate) struct SeriesVersion {
    id: Id,
    version: Version,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct SeriesStateVersionQuery {
    #[arguments(id: $id)]
    series_state: Option<SeriesVersion>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema = "series_state",
    schema_module = "crate::series_state::schema",
    graphql_type = "Query",
    variables = "IdVariables"
)]
pub(crate) struct LatestSeriesStateVersionQuery {
    #[arguments(id: $id)]
    latest_series_state_by_player_id: Option<SeriesVersion>,
}

/// Whether a failed request may have selected fields newer than the
/// series' data-model version: every error is typed `BAD_REQUEST`.
fn is_version_rejection(err: &Error) -> bool {
    let Error::GraphQl(errors) = err else {
        return false;
    };
    !errors.is_empty()
        && errors.iter().all(|error: &GraphQlError| {
            error
                .extensions
                .as_ref()
                .is_some_and(|ext| ext.error_type == Some(ErrorType::BadRequest))
        })
}

impl SeriesState {
    /// Fetches the state of a series by its GRID series ID (the
    /// `seriesState` field). Returns `None` if the series has no state.
    ///
    /// GRID rejects a whole request that selects a field newer than the
    /// series' [data-model version](super#data-model-versions). The query
    /// first selects every field, which costs one request for data of the
    /// newest version. If GRID rejects it with `BAD_REQUEST` errors, the
    /// method fetches the series' version and resends the query without
    /// the fields that version lacks: three requests in total.
    ///
    /// # Errors
    ///
    /// As for [`run`](Self::run). If the version lookup fails, its error is
    /// returned; if it returns no state or an unparsable version, the first
    /// request's error is.
    pub async fn get(&self, id: impl Into<Id>) -> Result<Option<Series>> {
        let id = id.into();
        let err = match self.series_state(id.clone(), None).await {
            Err(err) if is_version_rejection(&err) => err,
            result => return result,
        };
        let operation = SeriesStateVersionQuery::build(IdVariables { id: id.clone() });
        let found = self.run(&operation).await?.series_state;
        let Some(version) = found.and_then(|s| s.version.major_minor()) else {
            return Err(err);
        };
        self.series_state(id, Some(version)).await
    }

    /// Fetches the state of the latest series a player, given by GRID player
    /// ID, took part in (the `latestSeriesStateByPlayerId` field).
    ///
    /// Costs one request for data of the newest data-model version and three
    /// for older data, as for [`get`](Self::get). After the version lookup,
    /// the full state is fetched by the series ID that lookup returned.
    ///
    /// # Errors
    ///
    /// As for [`get`](Self::get).
    pub async fn latest_by_player_id(&self, player_id: impl Into<Id>) -> Result<Option<Series>> {
        let player_id = player_id.into();
        let operation = LatestSeriesStateByPlayerIdQuery::build(SeriesStateVariables::new(
            player_id.clone(),
            None,
        ));
        let err = match self.run(&operation).await {
            Ok(data) => return Ok(data.latest_series_state_by_player_id),
            Err(err) if is_version_rejection(&err) => err,
            Err(err) => return Err(err),
        };
        let operation = LatestSeriesStateVersionQuery::build(IdVariables { id: player_id });
        let found = self.run(&operation).await?.latest_series_state_by_player_id;
        let Some((id, version)) =
            found.and_then(|s| s.version.major_minor().map(|version| (s.id, version)))
        else {
            return Err(err);
        };
        self.series_state(id, Some(version)).await
    }

    /// Sends the full `seriesState` query gated for `version`.
    async fn series_state(&self, id: Id, version: Option<(u32, u32)>) -> Result<Option<Series>> {
        let operation = SeriesStateQuery::build(SeriesStateVariables::new(id, version));
        Ok(self.run(&operation).await?.series_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_gates() {
        let vars = |version| {
            serde_json::to_value(SeriesStateVariables::new(Id::new("1"), version)).unwrap()
        };
        let all = vars(None);
        let old = vars(Some((3, 1)));
        assert_eq!(all["id"], "1");
        assert_eq!(all.as_object().unwrap().len(), 42);
        let gates = all.as_object().unwrap().iter().filter(|(k, _)| *k != "id");
        assert!(gates.into_iter().all(|(_, v)| v == true));
        assert_eq!(old["v2_1"], true);
        assert_eq!(old["v3_2"], false);
        assert_eq!(vars(Some((1, 7)))["v1_6"], true);
        assert_eq!(vars(Some((1, 7)))["v1_9"], false);
        assert_eq!(vars(Some((3, 50)))["v3_50"], true);
    }

    #[test]
    fn every_prebuilt_query_parses() {
        let id = || IdVariables { id: Id::new("1") };
        let full = || SeriesStateVariables::new(Id::new("1"), None);
        let queries = [
            SeriesStateQuery::build(full()).query,
            LatestSeriesStateByPlayerIdQuery::build(full()).query,
            SeriesStateVersionQuery::build(id()).query,
            LatestSeriesStateVersionQuery::build(id()).query,
        ];
        for query in queries {
            assert!(
                crate::graphql::selection::parse(&query).is_some(),
                "selection parser rejects {query}"
            );
        }
    }

    #[test]
    fn only_bad_request_errors_trigger_the_version_lookup() {
        let errors = |types: &[&str]| {
            let errors = types
                .iter()
                .map(|ty| serde_json::json!({ "message": "m", "extensions": { "errorType": ty } }))
                .collect::<Vec<_>>();
            Error::GraphQl(serde_json::from_value(serde_json::Value::Array(errors)).unwrap())
        };
        assert!(is_version_rejection(&errors(&[
            "BAD_REQUEST",
            "BAD_REQUEST"
        ])));
        assert!(!is_version_rejection(&errors(&["BAD_REQUEST", "INTERNAL"])));
        assert!(!is_version_rejection(&errors(&[])));
        assert!(!is_version_rejection(&Error::Config("x".into())));
    }
}
