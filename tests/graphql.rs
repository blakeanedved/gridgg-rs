//! Transport tests for the GraphQL APIs against a local mock server.

use std::time::Duration;

use gridgg::{Client, Error, ErrorDetail, ErrorType, RateLimit, RetryConfig};
use serde_json::json;
use tokio::time::Instant;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "test-key-0123456789";

fn client(server: &MockServer, retry: RetryConfig, rate_limit: RateLimit) -> Client {
    Client::builder()
        .api_key(KEY)
        .central_url(format!("{}/central", server.uri()))
        .series_state_url(format!("{}/series-state", server.uri()))
        .stats_url(format!("{}/stats", server.uri()))
        .retry(retry)
        .rate_limit(rate_limit)
        .build()
        .unwrap()
}

fn fast_retry() -> RetryConfig {
    RetryConfig::new()
        .max_attempts(3)
        .base_delay(Duration::from_millis(100))
}

fn ok(data: &serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
}

#[tokio::test]
async fn sends_api_key_header_and_query() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/central"))
        .and(header("x-api-key", KEY))
        .and(header("content-type", "application/json"))
        .and(body_partial_json(json!({
            "query": "{ titles { id } }",
            "variables": { "a": 1 }
        })))
        .respond_with(ok(&json!({ "titles": [] })))
        .expect(1)
        .mount(&server)
        .await;

    let data = client(&server, RetryConfig::disabled(), RateLimit::Off)
        .central()
        .raw("{ titles { id } }", json!({ "a": 1 }))
        .await
        .unwrap();
    assert_eq!(data, json!({ "titles": [] }));
}

#[tokio::test]
async fn cross_origin_redirects_are_not_followed() {
    let server = MockServer::start().await;
    let other = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", other.uri()))
        .mount(&server)
        .await;

    let err = client(&server, RetryConfig::disabled(), RateLimit::Off)
        .central()
        .raw("{ titles { id } }", json!({}))
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::Http { status, .. } if status.as_u16() == 307),
        "{err:?}"
    );
    assert!(other.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn each_api_uses_its_own_url() {
    let server = MockServer::start().await;
    for (route, value) in [("/central", 1), ("/series-state", 2), ("/stats", 3)] {
        Mock::given(path(route))
            .respond_with(ok(&json!({ "v": value })))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server, RetryConfig::disabled(), RateLimit::Off);
    let query = "{ v }";
    let null = serde_json::Value::Null;
    assert_eq!(
        client.central().raw(query, null.clone()).await.unwrap(),
        json!({"v": 1})
    );
    assert_eq!(
        client
            .series_state()
            .raw(query, null.clone())
            .await
            .unwrap(),
        json!({"v": 2})
    );
    assert_eq!(
        client.stats().raw(query, null).await.unwrap(),
        json!({"v": 3})
    );
}

mod typed {
    #[derive(cynic::QueryVariables, Debug)]
    #[cynic(schema_module = "gridgg::central::schema")]
    pub struct SeriesVariables {
        pub id: cynic::Id,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(
        schema = "central",
        schema_module = "gridgg::central::schema",
        graphql_type = "Query",
        variables = "SeriesVariables"
    )]
    pub struct SeriesQuery {
        #[arguments(id: $id)]
        pub series: Option<Series>,
    }

    #[derive(cynic::QueryFragment, Debug)]
    #[cynic(schema = "central", schema_module = "gridgg::central::schema")]
    pub struct Series {
        pub id: cynic::Id,
        pub start_time_scheduled: jiff::Timestamp,
    }
}

#[tokio::test]
async fn runs_typed_cynic_operation() {
    use cynic::QueryBuilder;

    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .and(body_partial_json(json!({ "variables": { "id": "42" } })))
        .respond_with(ok(&json!({
            "series": { "id": "42", "startTimeScheduled": "2024-05-01T12:00:00Z" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let op = typed::SeriesQuery::build(typed::SeriesVariables {
        id: cynic::Id::new("42"),
    });
    let series = client(&server, RetryConfig::disabled(), RateLimit::Off)
        .central()
        .run(&op)
        .await
        .unwrap()
        .series
        .unwrap();
    assert_eq!(series.id.inner(), "42");
    assert_eq!(series.start_time_scheduled.as_second(), 1_714_564_800);
}

#[tokio::test]
async fn maps_graphql_errors_with_typed_extensions() {
    let server = MockServer::start().await;
    Mock::given(path("/stats"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": null,
            "errors": [{
                "message": "Permission denied",
                "path": ["teamStatistics"],
                "extensions": {
                    "code": "DOWNSTREAM_SERVICE_ERROR",
                    "serviceName": "statistics-feed",
                    "errorType": "PERMISSION_DENIED",
                    "errorDetail": "MISSING_RESOURCE"
                }
            }, {
                "message": "second"
            }]
        })))
        .mount(&server)
        .await;

    let err = client(&server, fast_retry(), RateLimit::Off)
        .stats()
        .raw("{ teamStatistics }", serde_json::Value::Null)
        .await
        .unwrap_err();
    let Error::GraphQl(errors) = &err else {
        panic!("expected GraphQL error, got {err:?}");
    };
    assert_eq!(errors.len(), 2);
    let ext = errors[0].extensions.as_ref().unwrap();
    assert_eq!(ext.error_type, Some(ErrorType::PermissionDenied));
    assert_eq!(ext.error_detail, Some(ErrorDetail::MissingResource));
    assert_eq!(ext.code.as_deref(), Some("DOWNSTREAM_SERVICE_ERROR"));
    assert_eq!(ext.service_name.as_deref(), Some("statistics-feed"));
    assert_eq!(
        err.to_string(),
        "GraphQL error: Permission denied [PERMISSION_DENIED/MISSING_RESOURCE] (and 1 more)"
    );
    // GraphQL errors are not retried.
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn maps_graphql_errors_in_4xx_body() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "errors": [{
                "message": "Cannot query field",
                "extensions": { "errorType": "BAD_REQUEST", "errorDetail": "FIELD_NOT_FOUND" }
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let err = client(&server, fast_retry(), RateLimit::Off)
        .central()
        .raw("{ nope }", serde_json::Value::Null)
        .await
        .unwrap_err();
    let Error::GraphQl(errors) = err else {
        panic!("expected GraphQL error");
    };
    let ext = errors[0].extensions.as_ref().unwrap();
    assert_eq!(ext.error_type, Some(ErrorType::BadRequest));
    assert_eq!(ext.error_detail, Some(ErrorDetail::FieldNotFound));
}

#[tokio::test(start_paused = true)]
async fn retries_429_honouring_retry_after() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "5"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({ "ok": true })))
        .mount(&server)
        .await;

    let start = Instant::now();
    let data = client(&server, fast_retry(), RateLimit::Off)
        .central()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap();
    assert_eq!(data, json!({ "ok": true }));
    assert!(start.elapsed() >= Duration::from_secs(5));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test(start_paused = true)]
async fn retries_5xx_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(path("/series-state"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(path("/series-state"))
        .respond_with(ok(&json!({ "ok": true })))
        .mount(&server)
        .await;

    let data = client(&server, fast_retry(), RateLimit::Off)
        .series_state()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap();
    assert_eq!(data, json!({ "ok": true }));
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test(start_paused = true)]
async fn gives_up_after_max_attempts() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .mount(&server)
        .await;
    Mock::given(path("/stats"))
        .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
        .mount(&server)
        .await;
    let client = client(&server, fast_retry(), RateLimit::Off);

    let err = client
        .central()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::RateLimited {
                attempts: 3,
                retry_after: Some(d)
            } if d == Duration::from_secs(1)
        ),
        "{err:?}"
    );

    let err = client
        .stats()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::Http { status, body } if status.as_u16() == 502 && body == "bad gateway"),
        "{err:?}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 6);
}

#[tokio::test]
async fn does_not_retry_4xx() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .expect(1)
        .mount(&server)
        .await;

    let err = client(&server, fast_retry(), RateLimit::Off)
        .central()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap_err();
    assert_eq!(err.status().map(|s| s.as_u16()), Some(401));
    assert!(matches!(err, Error::Http { .. }), "{err:?}");
}

#[tokio::test(start_paused = true)]
async fn rate_limit_is_shared_across_apis_and_clones() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ok(&json!({})))
        .mount(&server)
        .await;
    let client = client(&server, RetryConfig::disabled(), RateLimit::PerMinute(30));
    let clone = client.clone();
    let null = || serde_json::Value::Null;

    let start = Instant::now();
    client.central().raw("{a}", null()).await.unwrap();
    clone.stats().raw("{a}", null()).await.unwrap();
    client.series_state().raw("{a}", null()).await.unwrap();
    // 30 per minute: slots at 0 s, 2 s and 4 s.
    assert!(start.elapsed() >= Duration::from_secs(4));
}

#[tokio::test]
async fn api_key_is_redacted_from_errors() {
    // A closed local port: connecting fails with a reqwest error carrying the URL.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let client = Client::builder()
        .api_key(KEY)
        .central_url(format!("http://127.0.0.1:{port}/graphql?key={KEY}"))
        .retry(RetryConfig::disabled())
        .rate_limit(RateLimit::Off)
        .build()
        .unwrap();

    let err = client
        .central()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Transport(_)), "{err:?}");
    let display = err.to_string();
    let debug = format!("{err:?}");
    assert!(!display.contains(KEY), "{display}");
    assert!(!debug.contains(KEY), "{debug}");
    assert!(debug.contains("REDACTED"), "{debug}");
    assert!(!format!("{client:?}").contains(KEY));
}

fn graphql_error(extensions: &serde_json::Value) -> serde_json::Value {
    json!({ "data": null, "errors": [{ "message": "slow down", "extensions": extensions }] })
}

#[tokio::test(start_paused = true)]
async fn retries_throttling_graphql_errors_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(200).set_body_json(graphql_error(
            &json!({ "errorType": "UNAVAILABLE", "errorDetail": "TOO_MANY_REQUESTS" }),
        )))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(path("/central"))
        .respond_with(ResponseTemplate::new(420).set_body_json(graphql_error(
            &json!({ "errorDetail": "ENHANCE_YOUR_CALM" }),
        )))
        .up_to_n_times(1)
        .with_priority(2)
        .mount(&server)
        .await;
    Mock::given(path("/central"))
        .respond_with(ok(&json!({ "ok": true })))
        .with_priority(3)
        .mount(&server)
        .await;

    let data = client(&server, fast_retry(), RateLimit::Off)
        .central()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap();
    assert_eq!(data, json!({ "ok": true }));
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test(start_paused = true)]
async fn gives_up_on_retryable_graphql_errors_with_graphql_error() {
    let server = MockServer::start().await;
    Mock::given(path("/central"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(graphql_error(&json!({ "errorType": "UNAVAILABLE" }))),
        )
        .mount(&server)
        .await;

    let err = client(&server, fast_retry(), RateLimit::Off)
        .central()
        .raw("{ ok }", serde_json::Value::Null)
        .await
        .unwrap_err();
    let Error::GraphQl(errors) = &err else {
        panic!("expected GraphQL error, got {err:?}");
    };
    assert_eq!(
        errors[0].extensions.as_ref().unwrap().error_type,
        Some(ErrorType::Unavailable)
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}
