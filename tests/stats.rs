//! Prebuilt Stats Feed queries against a local mock server, plus ignored
//! live checks against the GRID API.

use gridgg::stats::{
    DateTimeFilter, GameOrder, GameSelection, GameStateFilter, GameStatisticsFilter,
    GameTeamStateFilter, GameTeamsStatisticsByGameDetails, MapFilter, PlayerGameStatisticsDetails,
    PlayerSeriesStatisticsDetails, PlayerStatisticsFilter, SeriesStatisticsFilter, StringFilter,
    TeamGameStatisticsDetails, TeamStatistics, TeamStatisticsFilter,
    TeamStatisticsTournamentFilter, TimeRangeFilter,
};
use gridgg::{Client, RateLimit, RetryConfig};
use serde_json::{Value, json};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-key")
        .stats_url(format!("{}/stats", server.uri()))
        .retry(RetryConfig::disabled())
        .rate_limit(RateLimit::Off)
        .build()
        .unwrap()
}

async fn mock(data: Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/stats"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": data })))
        .mount(&server)
        .await;
    server
}

/// The JSON body of the first request the server received.
async fn body(server: &MockServer) -> Value {
    server.received_requests().await.unwrap()[0]
        .body_json()
        .unwrap()
}

/// The query string followed by the pretty-printed variables, for snapshots.
fn request_snapshot(body: &Value) -> String {
    format!(
        "{}\nvariables: {}",
        body["query"].as_str().unwrap(),
        serde_json::to_string_pretty(&body["variables"]).unwrap()
    )
}

/// `base` with the fields of `extra` added (or replaced).
fn merge(mut base: Value, extra: Value) -> Value {
    let (Value::Object(base_map), Value::Object(extra_map)) = (&mut base, extra) else {
        panic!("merge expects two objects");
    };
    base_map.extend(extra_map);
    base
}

// ---------------------------------------------------------------------------
// Fixtures

fn agg_int(sum: i32) -> Value {
    json!({
        "sum": sum, "min": 1, "max": 9, "avg": 4.5,
        "ratePerMinute": { "min": 0.1, "max": 0.5, "avg": 0.25 }
    })
}

fn agg_float(sum: f64) -> Value {
    json!({
        "sum": sum, "min": 0.5, "max": 2.5, "avg": 1.5,
        "ratePerMinute": { "min": 0.01, "max": 0.05, "avg": 0.02 }
    })
}

fn duration() -> Value {
    json!({ "sum": "PT5H", "min": "PT25M", "max": "PT45M", "avg": "PT30M10S" })
}

fn occurrence(value: bool, count: i32) -> Value {
    json!({
        "value": value, "count": count, "percentage": 50.0,
        "streak": { "current": 1, "max": 3, "min": 0 }
    })
}

/// `won` with a null entry, which the SDK drops.
fn won() -> Value {
    json!([occurrence(true, 6), null, occurrence(false, 4)])
}

fn character_occurrence() -> Value {
    json!({ "character": { "id": "c1", "name": "Ahri" }, "count": 3, "percentage": 30.0 })
}

fn draft_action() -> Value {
    json!({
        "draftable": { "id": "d1", "name": "Ahri", "type": "champion" },
        "type": [{ "value": "pick", "count": 4, "percentage": 80.0 }]
    })
}

fn map_occurrence() -> Value {
    json!({ "map": { "id": "m1", "name": "Ascent" }, "count": 2, "percentage": 40.0 })
}

/// Counters shared by every team and player statistics level.
fn counters() -> Value {
    json!({
        "kills": agg_int(120),
        "deaths": agg_int(80),
        "killAssistsGiven": agg_int(200),
        "killAssistsReceived": agg_int(150),
        "selfkills": agg_int(0),
        "teamkills": agg_int(0),
        "teamkillAssistsGiven": agg_int(0),
        "teamkillAssistsReceived": agg_int(0),
        "structuresDestroyed": agg_int(30),
        "structuresCaptured": agg_int(0),
        "firstKill": [occurrence(true, 7), occurrence(false, 3)],
        "objectives": [
            {
                "type": "slayBaron",
                "completionCount": agg_int(8),
                "completedFirst": [occurrence(true, 5)]
            },
            { "type": "destroyTower", "completionCount": agg_int(30), "completedFirst": null }
        ]
    })
}

/// Fields of game-level team and player statistics on top of [`counters`].
fn game_counters() -> Value {
    merge(
        counters(),
        json!({
            "money": agg_int(500),
            "inventoryValue": agg_int(400),
            "netWorth": agg_int(900),
            "unitKills": [{ "unitName": "minion", "count": agg_int(2000) }]
        }),
    )
}

fn lol_team_extra() -> Value {
    json!({
        "damageDealt": agg_int(50_000),
        "damageTaken": agg_int(45_000),
        "experiencePoints": agg_int(60_000),
        "totalMoneyEarned": agg_int(55_000),
        "visionScore": agg_float(300.5),
        "forwardPercentage": agg_float(12.5),
        "majorMoneyLead": agg_float(1.0),
        "majorMoneyDeficit": agg_float(0.5),
        "baronPowerPlay": agg_int(3000)
    })
}

fn valorant_damage() -> Value {
    json!({ "damageDealt": agg_int(30_000), "damageTaken": agg_int(28_000) })
}

fn team_series() -> Value {
    merge(
        counters(),
        json!({ "count": 10, "won": won(), "score": agg_int(15), "duration": duration() }),
    )
}

fn team_game(typename: &str, extra: Value) -> Value {
    let base = merge(
        game_counters(),
        json!({
            "__typename": typename,
            "count": 25,
            "won": won(),
            "score": agg_int(15),
            "duration": duration(),
            "players": { "characters": [character_occurrence()] }
        }),
    );
    merge(base, extra)
}

fn team_segment() -> Value {
    merge(
        counters(),
        json!({
            "type": "round",
            "count": 300,
            "won": won(),
            "wonFirst": [occurrence(true, 12)]
        }),
    )
}

fn team_statistics(game: &Value) -> Value {
    json!({
        "teamStatistics": {
            "id": "47351",
            "aggregationSeriesIds": ["s1", "s2"],
            "series": team_series(),
            "game": game,
            "segment": [team_segment()]
        }
    })
}

fn player_series(typename: &str, extra: Value) -> Value {
    let base = merge(
        counters(),
        json!({ "__typename": typename, "count": 10, "won": won() }),
    );
    merge(base, extra)
}

fn player_game(typename: &str, extra: Value) -> Value {
    let base = merge(
        game_counters(),
        json!({ "__typename": typename, "count": 25, "won": won() }),
    );
    merge(base, extra)
}

fn player_statistics(series: &Value, game: &Value) -> Value {
    let segment = merge(
        counters(),
        json!({ "type": "round", "count": 300, "won": won() }),
    );
    json!({
        "playerStatistics": {
            "id": "p1",
            "aggregationSeriesIds": ["s1"],
            "series": series,
            "game": game,
            "segment": [segment]
        }
    })
}

fn game_teams(typename: &str, extra: Value) -> Value {
    let mut base = merge(
        game_counters(),
        json!({
            "__typename": typename,
            "score": agg_int(15),
            "players": { "character": [character_occurrence()] }
        }),
    );
    base.as_object_mut().unwrap().remove("firstKill");
    merge(base, extra)
}

fn game_statistics(teams: &Value) -> Value {
    json!({
        "gameStatistics": {
            "aggregationGameIds": ["g1", "g2"],
            "count": 2,
            "games": {
                "duration": duration(),
                "map": [map_occurrence()],
                "draftActions": [draft_action()],
                "teams": teams
            },
            "segments": [{ "type": "round", "count": agg_int(48) }]
        }
    })
}

fn series_statistics(teams: &Value) -> Value {
    json!({
        "seriesStatistics": {
            "aggregationSeriesIds": ["s1"],
            "count": 1,
            "series": { "draftActions": [draft_action()] },
            "games": {
                "count": agg_int(3),
                "duration": duration(),
                "map": [map_occurrence()],
                "draftActions": [draft_action()],
                "teams": teams
            },
            "segments": [{ "type": "round", "count": agg_int(48) }]
        }
    })
}

async fn get_team_statistics(game: &Value) -> TeamStatistics {
    let server = mock(team_statistics(game)).await;
    client(&server)
        .stats()
        .team_statistics("47351", TeamStatisticsFilter::default())
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// teamStatistics

#[tokio::test]
async fn team_statistics_request_snapshot() {
    let server = mock(team_statistics(&team_game(
        "TeamGameStatisticsLol",
        lol_team_extra(),
    )))
    .await;
    let filter = TeamStatisticsFilter::default()
        .started_at(DateTimeFilter::period(TimeRangeFilter::Last3Months))
        .tournament(TeamStatisticsTournamentFilter::new(["7", "8"]).include_children(false));
    client(&server)
        .stats()
        .team_statistics("47351", filter)
        .await
        .unwrap();

    let body = body(&server).await;
    assert_eq!(
        body["variables"],
        json!({
            "teamId": "47351",
            "filter": {
                "startedAt": { "period": "LAST_3_MONTHS" },
                "tournament": { "id": { "in": ["7", "8"] }, "includeChildren": false }
            }
        })
    );
    insta::assert_snapshot!(request_snapshot(&body));
}

#[tokio::test]
async fn team_statistics_decodes_lol() {
    let stats = get_team_statistics(&team_game("TeamGameStatisticsLol", lol_team_extra())).await;

    assert_eq!(stats.id.inner(), "47351");
    assert_eq!(stats.aggregation_series_ids.len(), 2);

    let series = &stats.series;
    assert_eq!(series.count, 10);
    assert_eq!(series.won.len(), 2, "the null `won` entry is dropped");
    assert!(series.won[0].value);
    assert_eq!(series.won[0].streak.max, 3);
    assert_eq!(series.kills.sum, 120);
    assert!((series.kills.rate_per_minute.avg - 0.25).abs() < f64::EPSILON);
    assert_eq!(
        series.duration.avg.fieldwise(),
        jiff::Span::new().minutes(30).seconds(10).fieldwise()
    );
    assert_eq!(series.objectives[0].objective_type, "slayBaron");
    assert_eq!(
        series.objectives[0].completed_first.as_ref().unwrap().len(),
        1
    );
    assert_eq!(series.objectives[1].completed_first, None);

    let game = &stats.game;
    assert_eq!(game.count, 25);
    assert_eq!(game.net_worth.sum, 900);
    assert_eq!(game.unit_kills[0].unit_name, "minion");
    assert_eq!(game.players.characters[0].character.name, "Ahri");
    let TeamGameStatisticsDetails::Lol(lol) = &game.details else {
        panic!("expected LoL details, got {:?}", game.details);
    };
    assert_eq!(lol.baron_power_play.sum, 3000);
    assert!((lol.vision_score.sum - 300.5).abs() < f64::EPSILON);

    let segment = &stats.segments[0];
    assert_eq!(segment.segment_type, "round");
    assert_eq!(segment.won_first[0].count, 12);
}

#[tokio::test]
async fn team_statistics_decodes_valorant() {
    let stats = get_team_statistics(&team_game("TeamGameStatisticsVal", valorant_damage())).await;
    let TeamGameStatisticsDetails::Valorant(valorant) = &stats.game.details else {
        panic!("expected Valorant details, got {:?}", stats.game.details);
    };
    assert_eq!(valorant.damage_dealt.sum, 30_000);
    assert_eq!(valorant.damage_taken.sum, 28_000);
}

#[tokio::test]
async fn team_statistics_other_titles_fall_back() {
    for typename in [
        "TeamGameStatisticsCs2",
        "DefaultTeamGameStatistics",
        "SomeFutureType",
    ] {
        let stats = get_team_statistics(&team_game(typename, json!({}))).await;
        assert!(
            matches!(stats.game.details, TeamGameStatisticsDetails::Other),
            "{typename}: {:?}",
            stats.game.details
        );
        assert_eq!(
            stats.game.kills.sum, 120,
            "{typename}: common fields decode"
        );
    }
}

// ---------------------------------------------------------------------------
// teamGameStatistics

#[tokio::test]
async fn team_game_statistics_request_snapshot_and_decode() {
    let data = json!({
        "teamGameStatistics": team_game("TeamGameStatisticsLol", lol_team_extra())
    });
    let server = mock(data).await;
    let selection = GameSelection::default()
        .filter(
            GameStateFilter::default()
                .started_at(TimeRangeFilter::LastYear)
                .map(MapFilter::name(StringFilter::equals("Summoner's Rift")))
                .teams(GameTeamStateFilter::default().side(StringFilter::equals("blue")))
                .or([
                    GameStateFilter::default().tournament(["7"]),
                    GameStateFilter::default().tournament(["8"]),
                ]),
        )
        .first(20)
        .order_by([GameOrder::started_at_desc()]);
    let stats = client(&server)
        .stats()
        .team_game_statistics("47351", selection)
        .await
        .unwrap();
    assert!(matches!(stats.details, TeamGameStatisticsDetails::Lol(_)));

    let body = body(&server).await;
    assert_eq!(
        body["variables"]["selection"],
        json!({
            "filter": {
                "or": [
                    { "tournament": { "id": { "in": ["7"] } } },
                    { "tournament": { "id": { "in": ["8"] } } }
                ],
                "startedAt": { "period": "LAST_YEAR" },
                "map": { "name": { "equals": "Summoner's Rift" } },
                "teams": { "side": { "equals": "blue" } }
            },
            "first": 20,
            "orderBy": [{ "field": "STARTED_AT", "direction": "DESC" }]
        })
    );
    insta::assert_snapshot!(request_snapshot(&body));
}

#[tokio::test]
async fn team_game_statistics_omits_empty_selection() {
    let data =
        json!({ "teamGameStatistics": team_game("TeamGameStatisticsVal", valorant_damage()) });
    let server = mock(data).await;
    let stats = client(&server)
        .stats()
        .team_game_statistics("47351", GameSelection::default())
        .await
        .unwrap();
    assert!(matches!(
        stats.details,
        TeamGameStatisticsDetails::Valorant(_)
    ));
    assert_eq!(
        body(&server).await["variables"],
        json!({ "teamId": "47351" })
    );
}

// ---------------------------------------------------------------------------
// playerStatistics

#[tokio::test]
async fn player_statistics_request_snapshot_and_decode_lol() {
    let lol_player = json!({
        "damageDealt": agg_int(20_000),
        "damageTaken": agg_int(18_000),
        "experiencePoints": agg_int(15_000),
        "totalMoneyEarned": agg_int(12_000),
        "visionScore": agg_float(40.0),
        "forwardPercentage": agg_float(55.5)
    });
    let data = player_statistics(
        &player_series("DefaultPlayerSeriesStatistics", json!({})),
        &player_game("PlayerGameStatisticsLol", lol_player),
    );
    let server = mock(data).await;
    let filter = PlayerStatisticsFilter::default().started_at(TimeRangeFilter::LastMonth);
    let stats = client(&server)
        .stats()
        .player_statistics("p1", filter)
        .await
        .unwrap();

    assert_eq!(stats.id.inner(), "p1");
    assert_eq!(stats.series.count, 10);
    assert!(matches!(
        stats.series.details,
        PlayerSeriesStatisticsDetails::Other
    ));
    let PlayerGameStatisticsDetails::Lol(lol) = &stats.game.details else {
        panic!("expected LoL details, got {:?}", stats.game.details);
    };
    assert!((lol.forward_percentage.sum - 55.5).abs() < f64::EPSILON);
    assert_eq!(stats.game.money.sum, 500);
    assert_eq!(stats.segments[0].won.len(), 2);

    let body = body(&server).await;
    assert_eq!(
        body["variables"],
        json!({ "playerId": "p1", "filter": { "startedAt": { "period": "LAST_MONTH" } } })
    );
    insta::assert_snapshot!(request_snapshot(&body));
}

#[tokio::test]
async fn player_statistics_decodes_valorant() {
    let data = player_statistics(
        &player_series(
            "ValorantPlayerSeriesStatistics",
            json!({ "headshots": agg_int(90) }),
        ),
        &player_game("PlayerGameStatisticsVal", valorant_damage()),
    );
    let server = mock(data).await;
    let stats = client(&server)
        .stats()
        .player_statistics("p1", PlayerStatisticsFilter::default())
        .await
        .unwrap();

    let PlayerSeriesStatisticsDetails::Valorant(series) = &stats.series.details else {
        panic!("expected Valorant details, got {:?}", stats.series.details);
    };
    assert_eq!(series.headshots.sum, 90);
    let PlayerGameStatisticsDetails::Valorant(game) = &stats.game.details else {
        panic!("expected Valorant details, got {:?}", stats.game.details);
    };
    assert_eq!(game.damage_dealt.sum, 30_000);
    assert_eq!(
        body(&server).await["variables"],
        json!({ "playerId": "p1", "filter": {} })
    );
}

#[tokio::test]
async fn player_statistics_other_titles_fall_back() {
    let data = player_statistics(
        &player_series("Cs2PlayerSeriesStatistics", json!({})),
        &player_game("PlayerGameStatisticsDota", json!({})),
    );
    let server = mock(data).await;
    let stats = client(&server)
        .stats()
        .player_statistics("p1", PlayerStatisticsFilter::default())
        .await
        .unwrap();
    assert!(matches!(
        stats.series.details,
        PlayerSeriesStatisticsDetails::Other
    ));
    assert!(matches!(
        stats.game.details,
        PlayerGameStatisticsDetails::Other
    ));
}

// ---------------------------------------------------------------------------
// gameStatistics

#[tokio::test]
async fn game_statistics_request_snapshot_and_decode() {
    let server = mock(game_statistics(&game_teams(
        "GameTeamsStatisticsByGameLol",
        lol_team_extra(),
    )))
    .await;
    let filter = GameStatisticsFilter::default()
        .started_at(TimeRangeFilter::Last6Months)
        .tournament(["7"])
        .version(["v1"]);
    let stats = client(&server)
        .stats()
        .game_statistics("3", filter)
        .await
        .unwrap();

    assert_eq!(stats.count, 2);
    assert_eq!(stats.aggregation_game_ids.len(), 2);
    assert_eq!(stats.games.maps[0].map.name, "Ascent");
    assert_eq!(
        stats.games.draft_actions[0].draftable.draftable_type,
        "champion"
    );
    assert_eq!(stats.games.draft_actions[0].types[0].value, "pick");
    assert_eq!(stats.games.teams.players.characters[0].count, 3);
    assert_eq!(stats.segments[0].count.sum, 48);
    let GameTeamsStatisticsByGameDetails::Lol(lol) = &stats.games.teams.details else {
        panic!("expected LoL details, got {:?}", stats.games.teams.details);
    };
    assert_eq!(lol.total_money_earned.sum, 55_000);

    let body = body(&server).await;
    assert_eq!(
        body["variables"],
        json!({
            "titleId": "3",
            "filter": {
                "startedAt": { "period": "LAST_6_MONTHS" },
                "tournament": { "id": { "in": ["7"] } },
                "version": { "id": { "in": ["v1"] } }
            }
        })
    );
    insta::assert_snapshot!(request_snapshot(&body));
}

#[tokio::test]
async fn game_statistics_dispatches_valorant_and_other() {
    for (typename, extra) in [
        ("GameTeamsStatisticsByGameVal", valorant_damage()),
        ("GameTeamsStatisticsByGameR6", json!({})),
    ] {
        let server = mock(game_statistics(&game_teams(typename, extra))).await;
        let stats = client(&server)
            .stats()
            .game_statistics("6", GameStatisticsFilter::default())
            .await
            .unwrap();
        let details = &stats.games.teams.details;
        match typename {
            "GameTeamsStatisticsByGameVal" => assert!(
                matches!(details, GameTeamsStatisticsByGameDetails::Valorant(v) if v.damage_taken.sum == 28_000),
                "{details:?}"
            ),
            _ => assert!(
                matches!(details, GameTeamsStatisticsByGameDetails::Other),
                "{details:?}"
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// seriesStatistics

#[tokio::test]
async fn series_statistics_request_snapshot_and_decode() {
    // The prebuilt query selects only the common team fields at this level.
    let mut teams = team_game("TeamGameStatisticsLol", json!({}));
    let teams_map = teams.as_object_mut().unwrap();
    teams_map.remove("players");
    teams_map.remove("__typename");
    let server = mock(series_statistics(&teams)).await;
    let filter = SeriesStatisticsFilter::default()
        .started_at(TimeRangeFilter::LastWeek)
        .tournament(gridgg::stats::SeriesStatisticsTournamentFilter::new(["7"]));
    let stats = client(&server)
        .stats()
        .series_statistics("6", filter)
        .await
        .unwrap();

    assert_eq!(stats.count, 1);
    assert_eq!(stats.series.draft_actions[0].draftable.name, "Ahri");
    assert_eq!(stats.games.count.sum, 3);
    assert_eq!(stats.segments[0].segment_type, "round");
    assert_eq!(stats.games.teams.count, 25);
    assert_eq!(stats.games.teams.won.len(), 2);

    let body = body(&server).await;
    assert_eq!(
        body["variables"],
        json!({
            "titleId": "6",
            "filter": {
                "startedAt": { "period": "LAST_WEEK" },
                "tournament": { "id": { "in": ["7"] } }
            }
        })
    );
    insta::assert_snapshot!(request_snapshot(&body));
}

// ---------------------------------------------------------------------------
// Filters

#[test]
fn time_ranges_serialize_with_schema_names() {
    for (range, name) in [
        (TimeRangeFilter::LastWeek, "LAST_WEEK"),
        (TimeRangeFilter::LastMonth, "LAST_MONTH"),
        (TimeRangeFilter::Last3Months, "LAST_3_MONTHS"),
        (TimeRangeFilter::Last6Months, "LAST_6_MONTHS"),
        (TimeRangeFilter::LastYear, "LAST_YEAR"),
        (TimeRangeFilter::Other("LAST_DECADE".into()), "LAST_DECADE"),
    ] {
        assert_eq!(
            serde_json::to_value(DateTimeFilter::period(range)).unwrap(),
            json!({ "period": name })
        );
    }
}

#[test]
fn empty_filters_serialize_to_empty_objects() {
    assert_eq!(
        serde_json::to_value(TeamStatisticsFilter::default()).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(GameStateFilter::default()).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(TeamStatisticsTournamentFilter::new(["1"])).unwrap(),
        json!({ "id": { "in": ["1"] } })
    );
}

// ---------------------------------------------------------------------------
// Live checks (run with `cargo test -- --ignored`; need `GRID_API_KEY`).

mod live {
    use gridgg::Client;
    use gridgg::stats::{
        GameSelection, GameStatisticsFilter, GameTeamsStatisticsByGameDetails,
        PlayerGameStatisticsDetails, PlayerSeriesStatisticsDetails, PlayerStatisticsFilter,
        SeriesStatisticsFilter, Stats, TeamGameStatisticsDetails, TeamStatisticsFilter,
        TimeRangeFilter,
    };
    use serde_json::json;

    fn live_client() -> Option<Client> {
        let Ok(key) = std::env::var("GRID_API_KEY") else {
            eprintln!("GRID_API_KEY is not set; skipping the live test");
            return None;
        };
        Some(Client::builder().api_key(key).build().unwrap())
    }

    /// The title ID of the title whose name contains `title_name`
    /// (case-insensitive), and a team ID and player ID from its most recent
    /// series in the last 180 days.
    ///
    /// Uses raw Central Data queries selecting only IDs and names: an Open
    /// Access key is refused some fields the prebuilt Central queries
    /// select (such as `Title.logoUrl`).
    async fn recent_ids(client: &Client, title_name: &str) -> (String, String, String) {
        let data = client
            .central()
            .raw("{ titles { id name } }", serde_json::Value::Null)
            .await
            .unwrap();
        let titles = data["titles"].as_array().unwrap();
        let title_id = titles
            .iter()
            .find(|t| {
                t["name"]
                    .as_str()
                    .unwrap()
                    .to_lowercase()
                    .contains(title_name)
            })
            .unwrap_or_else(|| panic!("no title named like {title_name}: {titles:?}"))["id"]
            .as_str()
            .unwrap()
            .to_owned();

        let now = jiff::Timestamp::now();
        let since = now - jiff::SignedDuration::from_hours(180 * 24);
        let query = "query($filter: SeriesFilter) { allSeries(first: 10, filter: $filter, \
            orderBy: StartTimeScheduled, orderDirection: DESC) { edges { node { \
            teams { baseInfo { id } } } } } }";
        let filter = json!({
            "titleIds": { "in": [title_id] },
            "types": ["ESPORTS"],
            "startTimeScheduled": { "gte": since.to_string(), "lte": now.to_string() }
        });
        let data = client
            .central()
            .raw(query, json!({ "filter": filter }))
            .await
            .unwrap();
        // Series do not always list their players, so look them up by team.
        let players_query = "query($teamId: ID!) { players(first: 1, \
            filter: { teamIdFilter: { id: $teamId } }) { edges { node { id } } } }";
        for edge in data["allSeries"]["edges"].as_array().unwrap() {
            for team in edge["node"]["teams"].as_array().unwrap() {
                let team_id = team["baseInfo"]["id"].as_str().unwrap();
                let players = client
                    .central()
                    .raw(players_query, json!({ "teamId": team_id }))
                    .await
                    .unwrap();
                if let Some(player_id) = players["players"]["edges"][0]["node"]["id"].as_str() {
                    return (title_id, team_id.to_owned(), player_id.to_owned());
                }
            }
        }
        panic!("no {title_name} series with teams and players in the last 180 days");
    }

    /// Prints the `__typename` of every statistics level, so dispatch can be
    /// checked against the schema's inferred `implements` clauses.
    async fn print_typenames(client: &Client, title_id: &str, team_id: &str, player_id: &str) {
        let query = "query($titleId: ID!, $teamId: ID!, $playerId: ID!, \
            $filter: TeamStatisticsFilter!, $playerFilter: PlayerStatisticsFilter!, \
            $gameFilter: GameStatisticsFilter!, $seriesFilter: SeriesStatisticsFilter!) { \
            teamStatistics(teamId: $teamId, filter: $filter) { \
            series { __typename } game { __typename players { __typename } } \
            segment { __typename type } } \
            playerStatistics(playerId: $playerId, filter: $playerFilter) { \
            series { __typename } game { __typename } segment { __typename type } } \
            gameStatistics(titleId: $titleId, filter: $gameFilter) { games { teams { __typename } } } \
            seriesStatistics(titleId: $titleId, filter: $seriesFilter) { games { teams { __typename } } } }";
        let filter = json!({ "startedAt": { "period": "LAST_YEAR" } });
        let month = json!({ "startedAt": { "period": "LAST_MONTH" } });
        let data = client
            .stats()
            .raw(
                query,
                json!({
                    "titleId": title_id,
                    "teamId": team_id,
                    "playerId": player_id,
                    "filter": filter,
                    "playerFilter": filter,
                    "gameFilter": month,
                    "seriesFilter": month
                }),
            )
            .await
            .unwrap();
        println!("title {title_id}, team {team_id}, player {player_id} typenames: {data}");
    }

    /// Runs all five prebuilt queries for a title and checks the dispatch
    /// observed on 2026-11-12: League of Legends uses the `*Lol` types at
    /// every game level; Valorant uses `ValorantPlayerSeriesStatistics` and
    /// `GameTeamsStatisticsByGameVal`, but the `Default*` types (so `Other`)
    /// for team and player game statistics, not the schema's `*Val` types.
    async fn check(title_name: &str, lol: bool) {
        let Some(client) = live_client() else { return };
        let (title_id, team_id, player_id) = recent_ids(&client, title_name).await;
        print_typenames(&client, &title_id, &team_id, &player_id).await;
        check_team(&client.stats(), title_name, &team_id, lol).await;
        check_player_and_title(&client.stats(), title_name, &title_id, &player_id, lol).await;
    }

    async fn check_team(stats: &Stats, title_name: &str, team_id: &str, lol: bool) {
        let team = stats
            .team_statistics(
                team_id,
                TeamStatisticsFilter::default().started_at(TimeRangeFilter::LastYear),
            )
            .await
            .unwrap();
        println!(
            "{title_name} team {team_id}: {} series, {} games, {} segment types, details {:?}",
            team.series.count,
            team.game.count,
            team.segments.len(),
            team.game.details
        );
        if lol && team.game.count > 0 {
            assert!(
                matches!(team.game.details, TeamGameStatisticsDetails::Lol(_)),
                "team {team_id}: {:?}",
                team.game.details
            );
        }

        let games = stats
            .team_game_statistics(team_id, GameSelection::default().first(5))
            .await
            .unwrap();
        println!(
            "{title_name} team {team_id}, last {} games: details {:?}",
            games.count, games.details
        );
        if lol && games.count > 0 {
            assert!(
                matches!(games.details, TeamGameStatisticsDetails::Lol(_)),
                "team {team_id}: {:?}",
                games.details
            );
        }
    }

    async fn check_player_and_title(
        stats: &Stats,
        title_name: &str,
        title_id: &str,
        player_id: &str,
        lol: bool,
    ) {
        let player = stats
            .player_statistics(
                player_id,
                PlayerStatisticsFilter::default().started_at(TimeRangeFilter::LastYear),
            )
            .await
            .unwrap();
        println!(
            "{title_name} player {player_id}: {} series, {} games, details {:?} / {:?}",
            player.series.count, player.game.count, player.series.details, player.game.details
        );
        if player.game.count > 0 {
            let typed = if lol {
                matches!(player.game.details, PlayerGameStatisticsDetails::Lol(_))
            } else {
                matches!(
                    player.series.details,
                    PlayerSeriesStatisticsDetails::Valorant(_)
                )
            };
            assert!(
                typed,
                "player {player_id}: {:?} / {:?}",
                player.series.details, player.game.details
            );
        }

        let title_games = stats
            .game_statistics(
                title_id,
                GameStatisticsFilter::default().started_at(TimeRangeFilter::LastMonth),
            )
            .await
            .unwrap();
        println!(
            "{title_name} title {title_id}: {} games last month, team details {:?}",
            title_games.count, title_games.games.teams.details
        );
        if title_games.count > 0 {
            let typed = if lol {
                matches!(
                    title_games.games.teams.details,
                    GameTeamsStatisticsByGameDetails::Lol(_)
                )
            } else {
                matches!(
                    title_games.games.teams.details,
                    GameTeamsStatisticsByGameDetails::Valorant(_)
                )
            };
            assert!(typed, "{:?}", title_games.games.teams.details);
        }

        let title_series = stats
            .series_statistics(
                title_id,
                SeriesStatisticsFilter::default().started_at(TimeRangeFilter::LastMonth),
            )
            .await
            .unwrap();
        println!(
            "{title_name} title {title_id}: {} series last month, {} games, {:.1} team kills per game",
            title_series.count, title_series.games.teams.count, title_series.games.teams.kills.avg
        );
    }

    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn lol_statistics() {
        check("league of legends", true).await;
    }

    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn valorant_statistics() {
        check("valorant", false).await;
    }
}
