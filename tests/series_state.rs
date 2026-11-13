//! Prebuilt Series State queries against a local mock server, plus ignored
//! live checks against the GRID API.

use gridgg::series_state::{
    GameInvalidationReason, GamePlayerDetails, GameTeamDetails, GameType, ParticipationStatus,
    SegmentPlayerDetails, SegmentTeamDetails, Series, SeriesPlayerDetails, SeriesTeamDetails,
};
use gridgg::{Client, RateLimit, RetryConfig};
use serde_json::{Value, json};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-key")
        .series_state_url(format!("{}/series-state", server.uri()))
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

/// `base` with the fields of `extra` added (or replaced).
fn merge(mut base: Value, extra: Value) -> Value {
    let (Value::Object(base_map), Value::Object(extra_map)) = (&mut base, extra) else {
        panic!("merge expects two objects");
    };
    base_map.extend(extra_map);
    base
}

fn objective() -> Value {
    json!({ "id": "o1", "type": "destroyTower", "completionCount": 2, "completedFirst": true })
}

/// Counters shared by every team and player level.
fn counters(kills: i32, deaths: i32) -> Value {
    json!({
        "kills": kills,
        "deaths": deaths,
        "killAssistsGiven": 3,
        "killAssistsReceived": 4,
        "selfkills": 0,
        "teamkills": 0,
        "firstKill": null,
        "objectives": [objective()]
    })
}

fn series_player(typename: &str, extra: Value) -> Value {
    let base = merge(
        counters(5, 2),
        json!({
            "__typename": typename,
            "id": "p1",
            "name": "Caps",
            "participationStatus": "active",
            "structuresDestroyed": 1,
            "structuresCaptured": 0
        }),
    );
    merge(base, extra)
}

fn series_team(typename: &str, players: &[Value], extra: Value) -> Value {
    let base = merge(
        counters(20, 10),
        json!({
            "__typename": typename,
            "id": "t1",
            "name": "G2 Esports",
            "score": 2,
            "won": true,
            "structuresDestroyed": 11,
            "structuresCaptured": 0,
            "players": players
        }),
    );
    merge(base, extra)
}

fn game_player(typename: &str, extra: Value) -> Value {
    let base = merge(
        counters(5, 2),
        json!({
            "__typename": typename,
            "id": "p1",
            "name": "Caps",
            "participationStatus": "active",
            "character": { "id": "c1", "name": "Ahri" },
            "roles": null,
            "money": 500,
            "loadoutValue": 9000,
            "netWorth": 9500,
            "structuresDestroyed": 1,
            "structuresCaptured": 0,
            "position": { "x": 1.5, "y": -2.0 },
            "inventory": { "items": [{
                "id": "i1", "name": "Luden's Echo", "quantity": 1, "equipped": 1, "stashed": 0
            }] }
        }),
    );
    merge(base, extra)
}

fn game_team(typename: &str, players: &[Value], extra: Value) -> Value {
    let base = merge(
        counters(20, 10),
        json!({
            "__typename": typename,
            "id": "t1",
            "name": "G2 Esports",
            "side": "blue",
            "score": 1,
            "won": true,
            "money": 2500,
            "loadoutValue": 45000,
            "netWorth": 47500,
            "structuresDestroyed": 11,
            "structuresCaptured": 0,
            "players": players
        }),
    );
    merge(base, extra)
}

fn segment_player(typename: &str, extra: Value) -> Value {
    let base = merge(
        counters(1, 0),
        json!({
            "__typename": typename,
            "id": "p1",
            "name": "Derke",
            "participationStatus": "active"
        }),
    );
    merge(base, extra)
}

fn segment_team(typename: &str, players: &[Value], extra: Value) -> Value {
    let base = merge(
        counters(5, 3),
        json!({
            "__typename": typename,
            "id": "t1",
            "name": "Fnatic",
            "side": "attacker",
            "won": true,
            "players": players
        }),
    );
    merge(base, extra)
}

fn sub_segment(id: &str, teams: &[Value]) -> Value {
    json!({
        "id": id,
        "type": "phase",
        "sequenceNumber": 1,
        "started": true,
        "finished": true,
        "startedAt": null,
        "duration": null,
        "teams": teams
    })
}

fn segment(id: &str, teams: &[Value], segments: &[Value]) -> Value {
    merge(
        sub_segment(id, teams),
        json!({
            "type": "round",
            "startedAt": "2024-05-01T12:05:00Z",
            "duration": "PT1M40S",
            "segments": segments
        }),
    )
}

fn game(teams: &[Value], segments: &Value) -> Value {
    json!({
        "id": "g1",
        "sequenceNumber": 1,
        "type": "ESPORTS",
        "started": true,
        "finished": true,
        "paused": false,
        "forfeited": false,
        "valid": true,
        "invalidationReason": null,
        "startedAt": "2024-05-01T12:00:00Z",
        "duration": "PT31M5S",
        "clock": {
            "id": "gameClock",
            "type": "gameClock",
            "ticking": false,
            "ticksBackwards": false,
            "currentSeconds": 1865
        },
        "map": { "id": "summonersRift", "name": "Summoner's Rift" },
        "titleVersion": { "name": "14.9" },
        "externalLinks": [{
            "dataProvider": { "name": "RIOT" },
            "externalEntity": { "id": "ESPORTSTMNT01_1" }
        }],
        "draftActions": [],
        "structures": [{
            "id": "s1",
            "type": "tower",
            "side": "red",
            "teamId": "t2",
            "destroyed": true,
            "currentHealth": 0,
            "maxHealth": 5000
        }],
        "nonPlayerCharacters": [{
            "id": "n1",
            "type": "baron",
            "side": "neutral",
            "alive": false,
            "respawnClock": null
        }],
        "teams": teams,
        "segments": segments
    })
}

fn series(version: &str, title: &str, teams: &[Value], games: &[Value]) -> Value {
    json!({
        "id": "2",
        "version": version,
        "title": { "nameShortened": title },
        "format": "Bo3",
        "started": true,
        "finished": true,
        "forfeited": false,
        "valid": true,
        "invalidationReason": null,
        "startedAt": "2024-05-01T11:55:00Z",
        "updatedAt": "2024-05-01T14:00:00Z",
        "duration": "PT2H5M",
        "draftActions": [{
            "id": "d1",
            "type": "pick",
            "sequenceNumber": "1",
            "drafter": { "id": "t1", "type": "team" },
            "draftable": { "id": "c1", "type": "character", "name": "Ahri" }
        }],
        "teams": teams,
        "games": games
    })
}

async fn get_series(data: &Value) -> Option<Series> {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(ok(data))
        .mount(&server)
        .await;
    client(&server).series_state().get("2").await.unwrap()
}

#[tokio::test]
async fn query_snapshot() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(ok(&json!({ "seriesState": null })))
        .mount(&server)
        .await;

    client(&server).series_state().get("2").await.unwrap();

    let mut bodies = bodies(&server).await;
    assert_eq!(bodies.len(), 1, "a successful first attempt is one request");
    let body = bodies.remove(0);
    assert_eq!(body["variables"]["id"], "2");
    assert_gates(&body, None);
    insta::assert_snapshot!(body["query"].as_str().unwrap());
}

/// Asserts that `body`'s variables hold one gate `vX_Y` per `$vX_Y` the
/// query declares, each true iff `version` (`None`: newest) is at least
/// `X.Y`.
fn assert_gates(body: &Value, version: Option<(u32, u32)>) {
    let query = body["query"].as_str().unwrap();
    let vars = body["variables"].as_object().unwrap();
    let mut gates = 0;
    for (name, value) in vars {
        if name == "id" {
            continue;
        }
        gates += 1;
        let (major, minor) = name[1..].split_once('_').unwrap();
        let gate = (major.parse::<u32>().unwrap(), minor.parse::<u32>().unwrap());
        let expected = version.is_none_or(|v| v >= gate);
        assert_eq!(value, &json!(expected), "{name} for version {version:?}");
        assert!(query.contains(&format!("${name}: Boolean!")), "{name}");
        assert!(query.contains(&format!("@include(if: ${name})")), "{name}");
    }
    let declared = query.matches(": Boolean!").count();
    assert_eq!(gates, declared, "every declared gate has a value");
    assert!(gates > 0);
}

fn bad_request() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "data": {},
        "errors": [{
            "message": "Requested field is only available from version 3.48",
            "extensions": { "errorType": "BAD_REQUEST" }
        }]
    }))
}

/// Answers like GRID for a series of data-model `version`: full queries
/// selecting newer fields are rejected, the version lookup succeeds.
fn versioned(root: &'static str, version: &'static str) -> impl Fn(&Request) -> ResponseTemplate {
    move |request: &Request| {
        let body: Value = request.body_json().unwrap();
        let vars = &body["variables"];
        if vars.get("v3_2").is_none() {
            ok(&json!({ root: { "id": "9", "version": version } }))
        } else if vars["v3_50"] == true {
            bad_request()
        } else {
            // Excluded fields are absent, including on spread types.
            let player = series_player("SeriesPlayerStateDefault", json!({}));
            let team = series_team("SeriesTeamStateDefault", &[player], json!({}));
            let mut data = series(version, "lol", &[team], &[]);
            strip(
                &mut data,
                &["firstKill", "completedFirst", "forfeited", "startedAt"],
            );
            ok(&json!({ "seriesState": data }))
        }
    }
}

/// Removes the `keys` from every object in `value`.
fn strip(value: &mut Value, keys: &[&str]) {
    match value {
        Value::Object(map) => {
            map.retain(|key, _| !keys.contains(&key.as_str()));
            map.values_mut().for_each(|value| strip(value, keys));
        }
        Value::Array(items) => items.iter_mut().for_each(|value| strip(value, keys)),
        _ => {}
    }
}

#[tokio::test]
async fn old_version_is_refetched_with_gates() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(versioned("seriesState", "3.1"))
        .mount(&server)
        .await;

    let series = client(&server)
        .series_state()
        .get("2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(series.version.as_str(), "3.1");
    assert_eq!(series.forfeited, None);
    assert_eq!(series.teams[0].first_kill, None);
    assert_eq!(
        series.teams[0].players[0].objectives[0].completed_first,
        None
    );

    let bodies = bodies(&server).await;
    assert_eq!(bodies.len(), 3);
    assert_gates(&bodies[0], None);
    assert_eq!(bodies[1]["variables"], json!({ "id": "2" }));
    let query = bodies[1]["query"].as_str().unwrap();
    assert!(query.contains("seriesState(id: $id)"), "{query}");
    assert!(!query.contains("games"), "{query}");
    assert_eq!(bodies[2]["variables"]["id"], "2");
    assert_gates(&bodies[2], Some((3, 1)));
}

#[tokio::test]
async fn latest_by_player_id_refetches_by_series_id() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(versioned("latestSeriesStateByPlayerId", "1.7"))
        .mount(&server)
        .await;

    let state = client(&server).series_state();
    let series = state.latest_by_player_id("7").await.unwrap();
    assert_eq!(series.unwrap().version.as_str(), "1.7");

    let bodies = bodies(&server).await;
    assert_eq!(bodies.len(), 3);
    assert_gates(&bodies[0], None);
    assert_eq!(bodies[1]["variables"], json!({ "id": "7" }));
    let query = bodies[1]["query"].as_str().unwrap();
    assert!(
        query.contains("latestSeriesStateByPlayerId(id: $id)"),
        "{query}"
    );
    let query = bodies[2]["query"].as_str().unwrap();
    assert!(query.contains("seriesState(id: $id)"), "{query}");
    assert_eq!(bodies[2]["variables"]["id"], "9");
    assert_gates(&bodies[2], Some((1, 7)));
}

#[tokio::test]
async fn other_errors_are_not_refetched() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": null,
            "errors": [{ "message": "boom", "extensions": { "errorType": "INTERNAL" } }]
        })))
        .mount(&server)
        .await;

    let err = client(&server).series_state().get("2").await.unwrap_err();
    assert!(matches!(err, gridgg::Error::GraphQl(_)), "{err:?}");
    assert_eq!(bodies(&server).await.len(), 1);
}

#[tokio::test]
async fn unusable_version_returns_the_first_error() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(versioned("seriesState", "latest"))
        .mount(&server)
        .await;

    let err = client(&server).series_state().get("2").await.unwrap_err();
    let gridgg::Error::GraphQl(errors) = err else {
        panic!("expected the first GraphQL error: {err:?}");
    };
    assert!(errors[0].message.contains("only available"), "{errors:?}");
    assert_eq!(bodies(&server).await.len(), 2);
}

#[tokio::test]
async fn null_is_none() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(ok(&json!({
            "seriesState": null,
            "latestSeriesStateByPlayerId": null
        })))
        .mount(&server)
        .await;

    let state = client(&server).series_state();
    assert!(state.get("404").await.unwrap().is_none());
    assert!(state.latest_by_player_id("7").await.unwrap().is_none());

    let latest = bodies(&server).await.remove(1);
    assert_eq!(latest["variables"]["id"], "7");
    assert_gates(&latest, None);
    let query = latest["query"].as_str().unwrap();
    assert!(
        query.contains("latestSeriesStateByPlayerId(id: $id)"),
        "{query}"
    );
}

#[tokio::test]
async fn decodes_lol_series() {
    // Version 3.22 data: fields from 3.23 on come back null.
    let lol_player = game_player(
        "GamePlayerStateLol",
        json!({
            "alive": true,
            "currentHealth": 800,
            "maxHealth": 1500,
            "currentArmor": 60,
            "experiencePoints": 14000,
            "respawnClock": {
                "id": null, "type": "respawnClock", "ticking": false,
                "ticksBackwards": true, "currentSeconds": 0
            },
            "totalMoneyEarned": 12000,
            "moneyPerMinute": null,
            "moneyPercentage": null,
            "damageDealt": null,
            "damageTaken": null,
            "damagePerMinute": null,
            "damagePercentage": null,
            "damagePerMoney": null,
            "kdaRatio": null,
            "killsAndAssists": null,
            "killParticipation": null,
            "visionScore": null,
            "visionScorePerMinute": null,
            "forwardPercentage": null
        }),
    );
    let lol_team = game_team(
        "GameTeamStateLol",
        &[lol_player],
        json!({
            "experiencePoints": 70000,
            "totalMoneyEarned": 60000,
            "moneyDifference": null,
            "moneyPerMinute": null,
            "majorMoneyLead": null,
            "majorMoneyDeficit": null,
            "damageDealt": null,
            "damageTaken": null,
            "damagePerMinute": null,
            "damagePerMoney": null,
            "kdaRatio": null,
            "killsAndAssists": null,
            "visionScore": null,
            "visionScorePerMinute": null,
            "forwardPercentage": null,
            "baronPowerPlays": null
        }),
    );
    // The schema has no LoL series team or player type.
    let teams = &[series_team(
        "SeriesTeamStateDefault",
        &[series_player("SeriesPlayerStateDefault", json!({}))],
        json!({}),
    )];
    let data = json!({
        "seriesState": series("3.22", "lol", teams, &[game(&[lol_team], &json!([]))])
    });

    let series = get_series(&data).await.unwrap();
    assert_eq!(series.id.inner(), "2");
    assert_eq!(series.version.major_minor(), Some((3, 22)));
    assert_eq!(series.title.as_ref().unwrap().name_shortened, "lol");
    assert_eq!(series.duration.unwrap().get_hours(), 2);
    assert_eq!(series.draft_actions[0].draftable.name, "Ahri");
    assert_eq!(series.teams[0].details, SeriesTeamDetails::Other);
    assert_eq!(
        series.teams[0].players[0].details,
        SeriesPlayerDetails::Other
    );
    assert_eq!(series.teams[0].score, 2);

    let game = &series.games[0];
    assert_eq!(game.game_type, Some(GameType::Esports));
    assert_eq!(game.map.name, "Summoner's Rift");
    assert_eq!(game.title_version.as_ref().unwrap().name, "14.9");
    assert_eq!(game.structures[0].max_health, Some(5000));
    assert!(!game.non_player_characters[0].alive);
    assert!(game.segments.as_ref().unwrap().is_empty());

    let team = &game.teams[0];
    assert_eq!(team.net_worth, 47500);
    let GameTeamDetails::Lol(lol_team) = &team.details else {
        panic!("expected a LoL team, got {:?}", team.details);
    };
    assert_eq!(lol_team.total_money_earned, Some(60000));
    assert_eq!(lol_team.vision_score, None);

    let player = &team.players[0];
    assert_eq!(player.kills, 5);
    assert_eq!(player.participation_status, ParticipationStatus::Active);
    assert_eq!(player.character.as_ref().unwrap().name, "Ahri");
    assert_eq!(player.inventory.items[0].name, "Luden's Echo");
    assert_eq!(player.first_kill, None);
    assert_eq!(player.roles, None);
    let GamePlayerDetails::Lol(lol) = &player.details else {
        panic!("expected a LoL player, got {:?}", player.details);
    };
    assert_eq!(lol.experience_points, 14000);
    assert_eq!(
        lol.respawn_clock.as_ref().unwrap().ticks_backwards,
        Some(true)
    );
    assert_eq!(lol.damage_dealt, None);
}

fn valorant_damage() -> Value {
    json!({
        "headshots": 7,
        "teamHeadshots": 0,
        "damageDealt": 1500,
        "damageTaken": 900,
        "selfdamageDealt": 0,
        "selfdamageTaken": 0,
        "teamdamageDealt": 10,
        "teamdamageTaken": 0
    })
}

#[tokio::test]
async fn decodes_valorant_series() {
    let vitals =
        json!({ "alive": true, "currentHealth": 100, "maxHealth": 100, "currentArmor": 50 });
    let seg_player = segment_player(
        "SegmentPlayerStateValorant",
        merge(valorant_damage(), vitals.clone()),
    );
    let seg_team = segment_team(
        "SegmentTeamStateValorant",
        &[seg_player],
        merge(valorant_damage(), json!({ "winType": "eliminated" })),
    );
    let sub = sub_segment("r1-p1", std::slice::from_ref(&seg_team));
    let round = segment("r1", &[seg_team], &[sub]);

    let player = game_player(
        "GamePlayerStateValorant",
        merge(
            merge(valorant_damage(), vitals),
            json!({ "ultimatePoints": 3, "character": { "id": "jett", "name": "Jett" } }),
        ),
    );
    let team = game_team("GameTeamStateValorant", &[player], valorant_damage());
    let mut game = game(&[team], &json!([round]));
    game["map"] = json!({ "id": "ascent", "name": "Ascent" });
    game["invalidationReason"] = json!("AGENT_SWITCH");

    let teams = &[series_team(
        "SeriesTeamStateValorant",
        &[series_player(
            "SeriesPlayerStateValorant",
            json!({ "headshots": 30, "teamHeadshots": 0 }),
        )],
        json!({ "headshots": 90, "teamHeadshots": 1 }),
    )];
    let data = json!({ "seriesState": series("3.50", "val", teams, &[game]) });

    let series = get_series(&data).await.unwrap();
    let SeriesTeamDetails::Valorant(team) = &series.teams[0].details else {
        panic!("expected a Valorant series team");
    };
    assert_eq!(team.headshots, 90);
    let SeriesPlayerDetails::Valorant(player) = &series.teams[0].players[0].details else {
        panic!("expected a Valorant series player");
    };
    assert_eq!(player.headshots, 30);

    let game = &series.games[0];
    assert_eq!(
        game.invalidation_reason,
        Some(GameInvalidationReason::AgentSwitch)
    );
    let GameTeamDetails::Valorant(team) = &game.teams[0].details else {
        panic!("expected a Valorant game team");
    };
    assert_eq!(team.damage_dealt, Some(1500));
    let GamePlayerDetails::Valorant(player) = &game.teams[0].players[0].details else {
        panic!("expected a Valorant game player");
    };
    assert_eq!(player.ultimate_points, Some(3));
    assert_eq!(player.current_armor, Some(50));
    assert_eq!(
        game.teams[0].players[0].character.as_ref().unwrap().name,
        "Jett"
    );

    let round = &game.segments.as_ref().unwrap()[0];
    assert_eq!(round.segment_type, "round");
    assert_eq!(round.duration.unwrap().get_seconds(), 40);
    let SegmentTeamDetails::Valorant(team) = &round.teams[0].details else {
        panic!("expected a Valorant segment team");
    };
    assert_eq!(team.win_type.as_deref(), Some("eliminated"));
    let SegmentPlayerDetails::Valorant(player) = &round.teams[0].players[0].details else {
        panic!("expected a Valorant segment player");
    };
    assert_eq!(player.alive, Some(true));
    assert_eq!(round.segments[0].id.inner(), "r1-p1");
    assert!(matches!(
        round.segments[0].teams[0].details,
        SegmentTeamDetails::Valorant(_)
    ));
}

#[tokio::test]
async fn unknown_title_falls_back_to_other() {
    let seg_team = segment_team(
        "SegmentTeamStateCs2",
        &[segment_player("SegmentPlayerStateCs2", json!({}))],
        json!({}),
    );
    let player = game_player("GamePlayerStateNewTitle", json!({ "character": null }));
    let team = game_team("GameTeamStateCs2", &[player], json!({}));
    let teams = &[series_team(
        "SeriesTeamStateCs2",
        &[series_player("SeriesPlayerStateCs2", json!({}))],
        json!({}),
    )];
    let mut game = game(&[team], &json!([segment("r1", &[seg_team], &[])]));
    // Old data: fields from newer versions are null.
    for field in [
        "type",
        "valid",
        "startedAt",
        "duration",
        "titleVersion",
        "externalLinks",
    ] {
        game[field] = Value::Null;
    }
    game["map"] = json!({ "id": null, "name": "de_mirage" });
    let mut series = series("1.0", "cs2", teams, &[game]);
    for field in ["title", "forfeited", "startedAt", "duration"] {
        series[field] = Value::Null;
    }

    let series = get_series(&json!({ "seriesState": series })).await.unwrap();
    assert!(series.title.is_none());
    assert!(series.duration.is_none());
    let series_team = &series.teams[0];
    assert_eq!(series_team.details, SeriesTeamDetails::Other);
    assert_eq!(series_team.players[0].details, SeriesPlayerDetails::Other);
    assert_eq!(series_team.kills, 20);

    let game = &series.games[0];
    assert_eq!(game.game_type, None);
    assert_eq!(game.map.id, None);
    assert!(game.external_links.is_none());
    let team = &game.teams[0];
    assert_eq!(team.details, GameTeamDetails::Other);
    assert_eq!(team.players[0].details, GamePlayerDetails::Other);
    assert_eq!(team.players[0].net_worth, 9500);
    assert!(team.players[0].character.is_none());

    let segment_team = &game.segments.as_ref().unwrap()[0].teams[0];
    assert_eq!(segment_team.details, SegmentTeamDetails::Other);
    assert_eq!(segment_team.players[0].details, SegmentPlayerDetails::Other);
    assert_eq!(segment_team.players[0].kills, 1);
}

// ---------------------------------------------------------------------------
// Live checks (run with `cargo test -- --ignored`; need `GRID_API_KEY`).

mod live {
    use gridgg::Client;
    use gridgg::series_state::{GamePlayerDetails, GameTeamDetails, Series};

    /// Finished series of known data-model versions.
    const LOL_RECENT: &str = "3018394"; // 3.50
    const LOL_OLD: &str = "2618418"; // 3.1
    const VALORANT_RECENT: &str = "3003316"; // 3.50
    const VALORANT_OLD: &str = "562389"; // 1.7

    fn live_client() -> Option<Client> {
        let Ok(key) = std::env::var("GRID_API_KEY") else {
            eprintln!("GRID_API_KEY is not set; skipping the live test");
            return None;
        };
        Some(Client::builder().api_key(key).build().unwrap())
    }

    /// Fetches series `id` with `get` and checks that every game team and
    /// player decodes to the title's typed variant.
    async fn check(client: &Client, id: &str, lol: bool) -> Series {
        let series = client
            .series_state()
            .get(id)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("series {id} has no state"));
        println!("series {id}: data model version {}", series.version);
        assert!(!series.games.is_empty(), "series {id} has no games");

        for game in &series.games {
            for team in &game.teams {
                let typed = if lol {
                    matches!(team.details, GameTeamDetails::Lol(_))
                } else {
                    matches!(team.details, GameTeamDetails::Valorant(_))
                };
                assert!(typed, "game team of series {id}: {:?}", team.details);
                for player in &team.players {
                    let typed = if lol {
                        matches!(player.details, GamePlayerDetails::Lol(_))
                    } else {
                        matches!(player.details, GamePlayerDetails::Valorant(_))
                    };
                    assert!(typed, "game player of series {id}: {:?}", player.details);
                }
            }
        }
        series
    }

    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn lol_series_dispatches_to_lol() {
        let Some(client) = live_client() else { return };
        check(&client, LOL_RECENT, true).await;
    }

    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn valorant_series_dispatches_to_valorant() {
        let Some(client) = live_client() else { return };
        check(&client, VALORANT_RECENT, false).await;
    }

    /// Version 3.1 data: the first request is rejected, the gated resend
    /// decodes.
    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn old_lol_series_decodes() {
        let Some(client) = live_client() else { return };
        let series = check(&client, LOL_OLD, true).await;
        assert!(series.forfeited.is_none());
    }

    /// Version 1.7 data: the first request is rejected, the gated resend
    /// decodes.
    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn old_valorant_series_decodes() {
        let Some(client) = live_client() else { return };
        let series = check(&client, VALORANT_OLD, false).await;
        assert!(series.started_at.is_none());
    }

    /// Fetches a player's latest series state, exercising
    /// `latestSeriesStateByPlayerId` and reporting its version.
    #[tokio::test]
    #[ignore = "needs GRID_API_KEY and network access"]
    async fn latest_by_player_id_reports_version() {
        let Some(client) = live_client() else { return };
        let series = check(&client, LOL_RECENT, true).await;
        let Some(player) = series.games[0].teams[0].players.first() else {
            panic!("series {LOL_RECENT} has no players");
        };
        let latest = client
            .series_state()
            .latest_by_player_id(player.id.clone())
            .await
            .unwrap();
        match latest {
            Some(latest) => println!(
                "player {}: latest series {} at data model version {}",
                player.id.inner(),
                latest.id.inner(),
                latest.version
            ),
            None => println!("player {}: no latest series state", player.id.inner()),
        }
    }
}
