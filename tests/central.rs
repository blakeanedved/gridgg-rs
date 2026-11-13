//! Prebuilt Central Data queries against a local mock server.

use futures_util::TryStreamExt;
use gridgg::central::{
    DateTimeFilter, NullableIdFilter, OrderDirection, PlayerFilter, SeriesFilter, SeriesOrderBy,
    SeriesType, StringFilter, TeamFilter, TournamentVenueType,
};
use gridgg::pagination::PageRequest;
use gridgg::{Client, RateLimit, RetryConfig};
use serde_json::{Value, json};
use wiremock::matchers::{body_partial_json, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-key")
        .central_url(format!("{}/central", server.uri()))
        .retry(RetryConfig::disabled())
        .rate_limit(RateLimit::Off)
        .build()
        .unwrap()
}

fn ok(data: &Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
}

/// The JSON bodies of every request the server received, in order.
async fn bodies(server: &MockServer) -> Vec<Value> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| request.body_json().unwrap())
        .collect()
}

fn title() -> Value {
    json!({
        "id": "3",
        "name": "League of Legends",
        "nameShortened": "lol",
        "private": false
    })
}

fn series(id: &str, series_type: &str) -> Value {
    json!({
        "id": id,
        "type": series_type,
        "startTimeScheduled": "2024-05-01T12:00:00Z",
        "format": { "id": "2", "name": "Best of 3", "nameShortened": "BO3" },
        "title": title(),
        "tournament": { "id": "77", "name": "LEC Spring", "nameShortened": "LEC" },
        "teams": [{
            "baseInfo": {
                "id": "10",
                "name": "G2 Esports",
                "nameShortened": "G2",
                "logoUrl": "https://cdn.grid.gg/g2.png"
            },
            "scoreAdvantage": 0
        }],
        "players": [{ "id": "100", "nickname": "Caps" }],
        "streams": [{ "url": "https://twitch.tv/lec" }],
        "productServiceLevels": [{ "productName": "liveDataFeed", "serviceLevel": "FULL" }],
        "private": false,
        "externalLinks": [{
            "dataProvider": { "name": "RIOT", "description": null },
            "externalEntity": { "id": "ESPORTSTMNT01" }
        }],
        "workflowStatus": "PUBLISHED",
        "updatedAt": "2024-05-02T00:00:00Z"
    })
}

fn series_page(ids: &[&str], has_next: bool, end_cursor: &str) -> Value {
    json!({
        "allSeries": {
            "edges": ids.iter().map(|id| json!({ "node": series(id, "ESPORTS") })).collect::<Vec<_>>(),
            "pageInfo": {
                "hasNextPage": has_next,
                "hasPreviousPage": false,
                "startCursor": null,
                "endCursor": end_cursor
            },
            "totalCount": 3
        }
    })
}

#[tokio::test]
async fn series_by_id_decodes() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .and(body_partial_json(json!({ "variables": { "id": "2" } })))
        .respond_with(ok(&json!({ "series": series("2", "ESPORTS") })))
        .expect(1)
        .mount(&server)
        .await;

    let series = client(&server)
        .central()
        .series("2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(series.id.inner(), "2");
    assert_eq!(series.series_type, SeriesType::Esports);
    assert_eq!(series.start_time_scheduled.as_second(), 1_714_564_800);
    assert_eq!(series.format.name_shortened, "BO3");
    assert_eq!(series.title.name_shortened, "lol");
    assert_eq!(series.tournament.name, "LEC Spring");
    assert_eq!(
        series.teams[0].base_info.name_shortened.as_deref(),
        Some("G2")
    );
    assert_eq!(series.players[0].nickname, "Caps");
    assert_eq!(series.external_links[0].provider(), "RIOT");
    assert_eq!(
        series.external_links[0].external_id().inner(),
        "ESPORTSTMNT01"
    );
}

#[tokio::test]
async fn single_object_null_is_none() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({ "team": null })))
        .expect(1)
        .mount(&server)
        .await;

    assert_eq!(client(&server).central().team("404").await.unwrap(), None);
}

#[tokio::test]
async fn unknown_enum_values_decode_to_other() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({ "series": series("2", "EXHIBITION") })))
        .mount(&server)
        .await;

    let series = client(&server)
        .central()
        .series("2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(series.series_type, SeriesType::Other("EXHIBITION".into()));
}

#[tokio::test]
async fn stream_walks_all_pages_with_cursor() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .and(body_partial_json(json!({ "variables": { "after": "c1" } })))
        .respond_with(ok(&series_page(&["3"], false, "c2")))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/central"))
        .respond_with(ok(&series_page(&["1", "2"], true, "c1")))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;

    let ids: Vec<String> = client(&server)
        .central()
        .all_series(SeriesFilter::default())
        .page_request(PageRequest::forward(2, None).unwrap())
        .stream()
        .map_ok(|series| series.id.into_inner())
        .try_collect()
        .await
        .unwrap();
    assert_eq!(ids, ["1", "2", "3"]);

    let bodies = bodies(&server).await;
    assert_eq!(bodies.len(), 2);
    assert_eq!(bodies[0]["variables"]["first"], 2);
    assert!(bodies[0]["variables"].get("after").is_none());
    assert_eq!(bodies[1]["variables"]["first"], 2);
    assert_eq!(bodies[1]["variables"]["after"], "c1");
}

#[tokio::test]
async fn page_returns_items_and_total_count() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&series_page(&["1", "2"], true, "c1")))
        .expect(1)
        .mount(&server)
        .await;

    let page = client(&server)
        .central()
        .all_series(SeriesFilter::default())
        .page()
        .await
        .unwrap();
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total_count, Some(3));
    assert!(page.page_info.has_next_page);
    assert_eq!(page.page_info.end_cursor.as_deref(), Some("c1"));
}

#[tokio::test]
async fn all_series_request_snapshot() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&series_page(&[], false, "c")))
        .mount(&server)
        .await;

    let filter = SeriesFilter::default()
        .title_ids(["3", "6"])
        .types([SeriesType::Esports])
        .start_time_scheduled(DateTimeFilter::default().gte("2024-01-01T00:00:00Z"));
    client(&server)
        .central()
        .all_series(filter)
        .order_by(SeriesOrderBy::StartTimeScheduled)
        .order_direction(OrderDirection::Desc)
        .page_request(PageRequest::forward(10, None).unwrap())
        .page()
        .await
        .unwrap();

    let body = bodies(&server).await.remove(0);
    // Unset filter fields and paging arguments are omitted.
    assert_eq!(
        body["variables"]["filter"],
        json!({
            "startTimeScheduled": { "gte": "2024-01-01T00:00:00Z" },
            "titleIds": { "in": ["3", "6"] },
            "types": ["ESPORTS"]
        })
    );
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn players_request_snapshot() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({
            "players": {
                "edges": [],
                "pageInfo": {
                    "hasNextPage": false,
                    "hasPreviousPage": false,
                    "startCursor": null,
                    "endCursor": null
                },
                "totalCount": 0
            }
        })))
        .mount(&server)
        .await;

    // A null team ID is meaningful (players without a team) and is sent.
    let filter = PlayerFilter::default()
        .nickname(StringFilter::contains("caps"))
        .team_id_filter(NullableIdFilter::default());
    let page = client(&server)
        .central()
        .players(filter)
        .page_request(PageRequest::backward(5, Some("z".into())).unwrap())
        .page()
        .await
        .unwrap();
    assert_eq!(page.items.len(), 0);
    assert_eq!(page.total_count, Some(0));

    let body = bodies(&server).await.remove(0);
    assert_eq!(
        body["variables"]["filter"],
        json!({ "nickname": { "contains": "caps" }, "teamIdFilter": { "id": null } })
    );
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn teams_stream_snapshot_and_decode() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({
            "teams": {
                "edges": [{ "node": {
                    "id": "10",
                    "name": "G2 Esports",
                    "logoUrl": "https://cdn.grid.gg/g2.png",
                    "colorPrimary": "#000000",
                    "colorSecondary": "#ffffff",
                    "private": false,
                    "title": title(),
                    "organization": { "id": "5", "name": "G2" },
                    "assets": [{ "assetType": "NEW_LOGO_KIND", "url": "https://cdn.grid.gg/a.png" }],
                    "externalLinks": [],
                    "workflowStatus": "PUBLISHED",
                    "updatedAt": "2024-05-02T00:00:00Z"
                }}],
                "pageInfo": {
                    "hasNextPage": false,
                    "hasPreviousPage": false,
                    "startCursor": "a",
                    "endCursor": "a"
                },
                "totalCount": 1
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let teams: Vec<_> = client(&server)
        .central()
        .teams(TeamFilter::default().title_id("3"))
        .stream()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(teams.len(), 1);
    // GRID omits null fields such as `nameShortened` from the response.
    assert_eq!(teams[0].name_shortened, None);
    assert_eq!(teams[0].color_primary.as_str(), "#000000");
    assert_eq!(teams[0].organization.as_ref().unwrap().name, "G2");
    assert_eq!(
        teams[0].assets[0].asset_type,
        gridgg::central::AssetType::Other("NEW_LOGO_KIND".into())
    );

    insta::assert_json_snapshot!(bodies(&server).await.remove(0));
}

#[tokio::test]
async fn tournament_decodes_venue_and_dates() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({ "tournament": {
            "id": "77",
            "name": "LEC Spring",
            "nameShortened": "LEC",
            "logoUrl": "https://cdn.grid.gg/lec.png",
            "startDate": "2024-01-13",
            "endDate": null,
            "private": false,
            "prizePool": { "amount": "200000.00" },
            "venueType": "LAN",
            "titles": [title()],
            "parent": { "id": "70", "name": "LEC 2024", "nameShortened": "LEC24" },
            "children": [],
            "externalLinks": [],
            "updatedAt": "2024-05-02T00:00:00Z"
        }})))
        .mount(&server)
        .await;

    let tournament = client(&server)
        .central()
        .tournament("77")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tournament.venue_type, TournamentVenueType::Lan);
    assert_eq!(tournament.start_date, Some(jiff::civil::date(2024, 1, 13)));
    assert_eq!(tournament.end_date, None);
    assert_eq!(
        tournament.prize_pool.unwrap().amount,
        rust_decimal::Decimal::new(20_000_000, 2)
    );
    assert_eq!(tournament.parent.unwrap().id.inner(), "70");
}

#[tokio::test]
async fn external_id_lookups() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .and(body_partial_json(json!({
            "variables": { "dataProviderName": "RIOT", "externalId": "t-1", "titleId": "3" }
        })))
        .respond_with(ok(&json!({ "teamIdByExternalId": "10" })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/central"))
        .and(body_partial_json(json!({
            "variables": { "dataProviderName": "RIOT", "externalId": "s-404" }
        })))
        .respond_with(ok(&json!({ "seriesIdByExternalId": null })))
        .expect(1)
        .mount(&server)
        .await;

    let central = client(&server).central();
    let team = central
        .team_id_by_external_id("RIOT", "t-1", "3")
        .await
        .unwrap();
    assert_eq!(team, Some(cynic::Id::new("10")));
    let series = central
        .series_id_by_external_id("RIOT", "s-404")
        .await
        .unwrap();
    assert_eq!(series, None);

    let body = bodies(&server).await.remove(0);
    let query = body["query"].as_str().unwrap();
    assert!(query.contains("teamIdByExternalId("), "{query}");
    insta::assert_json_snapshot!(body);
}

#[tokio::test]
async fn player_id_lookup_omits_unset_title() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({ "playerIdByExternalId": "100" })))
        .expect(1)
        .mount(&server)
        .await;

    let id = client(&server)
        .central()
        .player_id_by_external_id("STEAM", "765", None)
        .await
        .unwrap();
    assert_eq!(id, Some(cynic::Id::new("100")));
    let body = bodies(&server).await.remove(0);
    assert!(body["variables"].get("titleId").is_none(), "{body}");
}

#[tokio::test]
async fn invalid_page_size_is_rejected_before_sending() {
    let server = MockServer::start().await;
    let err = client(&server)
        .central()
        .teams(TeamFilter::default())
        .page_request(PageRequest::Forward {
            first: 500,
            after: None,
        })
        .page()
        .await
        .unwrap_err();
    assert!(matches!(err, gridgg::Error::Config(_)), "{err:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn streams_are_send_and_static() {
    fn assert_send_static<T: Send + 'static>(_: &T) {}
    let client = Client::builder().api_key("k").build().unwrap();
    let stream = client
        .central()
        .tournaments(gridgg::central::TournamentFilter::default())
        .stream();
    assert_send_static(&stream);
}

// ---------------------------------------------------------------------------
// Live check (run with `cargo test -- --ignored`; needs `GRID_API_KEY`).

mod live {
    use std::collections::BTreeSet;

    use gridgg::central::{
        ExternalLink, OrderDirection, OrganizationFilter, PlayerFilter, PlayerRoleFilter,
        SeriesFilter, TeamFilter, TitleFilter, TournamentFilter,
    };
    use gridgg::pagination::PageRequest;
    use gridgg::{Client, Error, ErrorType, PathSegment};

    /// Outcomes of the prebuilt queries.
    #[derive(Default)]
    struct Report {
        /// Every `PERMISSION_DENIED` error path, without list indices.
        denied: BTreeSet<String>,
        /// Queries that failed for any other reason.
        failed: Vec<String>,
    }

    impl Report {
        /// Records `result` of query `name`. `NOT_FOUND` is tolerated when
        /// `lookup` is set (external IDs that may not resolve).
        fn record<T>(&mut self, name: &str, lookup: bool, result: gridgg::Result<T>) -> Option<T> {
            let err = match result {
                Ok(value) => {
                    println!("{name}: ok");
                    return Some(value);
                }
                Err(err) => err,
            };
            let mut message = err.to_string();
            let mut source = std::error::Error::source(&err);
            while let Some(cause) = source {
                message = format!("{message}: {cause}");
                source = cause.source();
            }
            println!("{name}: {message}");
            let mut other = false;
            if let Error::GraphQl(errors) = &err {
                for error in errors {
                    let error_type = error.extensions.as_ref().and_then(|e| e.error_type.clone());
                    match error_type {
                        Some(ErrorType::PermissionDenied) => {
                            let path =
                                error
                                    .path
                                    .iter()
                                    .flatten()
                                    .filter_map(|segment| match segment {
                                        PathSegment::Field(field) => Some(field.as_str()),
                                        PathSegment::Index(_) => None,
                                    });
                            let path = path.collect::<Vec<_>>().join(".");
                            self.denied.insert(format!("{name}: {path}"));
                        }
                        Some(ErrorType::NotFound) if lookup => {}
                        _ => other = true,
                    }
                }
            } else {
                other = true;
            }
            if other {
                self.failed.push(format!("{name}: {message}"));
            }
            None
        }
    }

    fn first_link(links: &[ExternalLink]) -> Option<(String, String)> {
        links.first().map(|link| {
            (
                link.data_provider.name.clone(),
                link.external_entity.id.inner().to_owned(),
            )
        })
    }

    /// Runs every prebuilt Central Data query once against real IDs (taken
    /// from the list queries) and reports denied fields.
    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    #[allow(deprecated, clippy::too_many_lines)]
    async fn every_prebuilt_query_is_permitted() {
        let Ok(key) = std::env::var("GRID_API_KEY") else {
            eprintln!("GRID_API_KEY is not set; skipping the live test");
            return;
        };
        let client = Client::builder().api_key(key).build().unwrap();
        let central = client.central();
        let page = || PageRequest::forward(5, None).unwrap();
        let mut report = Report::default();

        let titles = report.record(
            "titles",
            false,
            central.titles(TitleFilter::default()).await,
        );
        let title_id = titles.and_then(|titles| titles.first().map(|t| t.id.clone()));
        if let Some(id) = &title_id {
            report.record("title", false, central.title(id.clone()).await);
        }
        let providers = report.record("dataProviders", false, central.data_providers().await);
        report.record("seriesFormats", false, central.series_formats().await);

        let tournaments = central
            .tournaments(TournamentFilter::default())
            .order_direction(OrderDirection::Desc)
            .page_request(page());
        let tournament = report
            .record("tournaments", false, tournaments.page().await)
            .and_then(|page| page.items.into_iter().next());
        if let Some(tournament) = &tournament {
            let id = tournament.id.clone();
            report.record("tournament", false, central.tournament(id).await);
        }

        let teams = central.teams(TeamFilter::default()).page_request(page());
        let team = report
            .record("teams", false, teams.page().await)
            .and_then(|page| page.items.into_iter().next());
        if let Some(team) = &team {
            report.record("team", false, central.team(team.id.clone()).await);
        }

        let players = central
            .players(PlayerFilter::default())
            .page_request(page());
        let player = report
            .record("players", false, players.page().await)
            .and_then(|page| page.items.into_iter().next());
        if let Some(player) = &player {
            report.record("player", false, central.player(player.id.clone()).await);
        }

        let organizations = central
            .organizations(OrganizationFilter::default())
            .page_request(page());
        let organization = report
            .record("organizations", false, organizations.page().await)
            .and_then(|page| page.items.into_iter().next());
        if let Some(organization) = &organization {
            let id = organization.id.clone();
            report.record("organization", false, central.organization(id).await);
        }

        let roles = central.player_roles(PlayerRoleFilter::default()).await;
        let role = report
            .record("playerRoles", false, roles)
            .and_then(|roles| roles.into_iter().next());
        if let Some(role) = &role {
            report.record(
                "playerRole",
                false,
                central.player_role(role.id.clone()).await,
            );
        }

        let all_series = central
            .all_series(SeriesFilter::default())
            .order_direction(OrderDirection::Desc)
            .page_request(page());
        let series = report
            .record("allSeries", false, all_series.page().await)
            .and_then(|page| page.items.into_iter().next());
        if let Some(series) = &series {
            report.record("series", false, central.series(series.id.clone()).await);
        }

        // External-ID lookups, with real external IDs where the entities
        // above have any.
        let fallback = || {
            let provider = providers
                .as_ref()
                .and_then(|p| p.first().map(|p| p.name.clone()))
                .unwrap_or_else(|| "RIOT".to_owned());
            (provider, "0".to_owned())
        };
        let link =
            |links: Option<&[ExternalLink]>| links.and_then(first_link).unwrap_or_else(fallback);

        let (provider, id) = link(series.as_ref().map(|s| s.external_links.as_slice()));
        let result = central
            .series_id_by_external_id(&provider, id.as_str())
            .await;
        report.record("seriesIdByExternalId", true, result);
        let result = central
            .series_by_external_series(&provider, id.as_str())
            .await;
        report.record("seriesByExternalSeries", true, result);

        let (provider, id) = link(team.as_ref().map(|t| t.external_links.as_slice()));
        let title = team
            .as_ref()
            .map(|t| t.title.id.clone())
            .or(title_id)
            .unwrap_or_else(|| cynic::Id::new("1"));
        let result = central
            .team_id_by_external_id(&provider, id.as_str(), title)
            .await;
        report.record("teamIdByExternalId", true, result);

        let (provider, id) = link(player.as_ref().map(|p| p.external_links.as_slice()));
        let result = central
            .player_id_by_external_id(&provider, id.as_str(), None)
            .await;
        report.record("playerIdByExternalId", true, result);

        let (provider, id) = link(tournament.as_ref().map(|t| t.external_links.as_slice()));
        let result = central
            .tournament_id_by_external_id(&provider, id.as_str())
            .await;
        report.record("tournamentIdByExternalId", true, result);

        let (provider, id) = fallback();
        let result = central.game_id_by_external_id(&provider, id.as_str()).await;
        report.record("gameIdByExternalId", true, result);

        println!("denied: {:#?}", report.denied);
        println!("failed: {:#?}", report.failed);
        assert!(report.denied.is_empty(), "denied: {:#?}", report.denied);
        assert!(report.failed.is_empty(), "failed: {:#?}", report.failed);
    }
}
