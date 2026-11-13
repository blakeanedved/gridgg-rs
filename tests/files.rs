//! File Download tests against a local mock server.

#![cfg(feature = "files")]

use std::path::PathBuf;
use std::time::Duration;

use gridgg::files::{FileStatus, InfoType};
use gridgg::{Client, Error, RetryConfig};
use reqwest::StatusCode;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "test-key-0123456789";

fn client(base: &str, retry: RetryConfig) -> Client {
    Client::builder()
        .api_key(KEY)
        .files_url(format!("{base}/file-download"))
        .retry(retry)
        .build()
        .unwrap()
}

fn fast_retry() -> RetryConfig {
    RetryConfig::new()
        .max_attempts(3)
        .base_delay(Duration::from_millis(10))
}

/// A fresh, empty directory for one test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("files-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn dir_entries(dir: &PathBuf) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn list_decodes_file_info() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/file-download/list/2618418"))
        .and(header("x-api-key", KEY))
        .and(query_param_is_missing("key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "files": [
                {
                    "id": "events-grid-compressed",
                    "description": "Grid Series Events (.jsonl)",
                    "status": "ready",
                    "fileName": "events_grid_2618418.jsonl.zip",
                    "fullURL": "https://api.grid.gg/file-download/events/grid/series/2618418"
                },
                { "id": "replay-riot-game-1", "status": "archived" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let files = client(&server.uri(), RetryConfig::disabled())
        .files()
        .list("2618418")
        .await
        .unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].id, "events-grid-compressed");
    assert_eq!(files[0].status, FileStatus::Ready);
    assert_eq!(
        files[0].file_name.as_deref(),
        Some("events_grid_2618418.jsonl.zip")
    );
    assert_eq!(
        files[0].full_url.as_deref(),
        Some("https://api.grid.gg/file-download/events/grid/series/2618418")
    );
    assert_eq!(files[1].status, FileStatus::Other("archived".into()));
    assert_eq!(files[1].description, None);
}

#[tokio::test]
async fn download_methods_hit_their_paths() {
    let server = MockServer::start().await;
    let routes = [
        "/file-download/events/grid/series/7",
        "/file-download/end-state/grid/series/7",
        "/file-download/events/riot/series/7",
        "/file-download/events/riot/series/7/games/2",
        "/file-download/end-state/riot/series/7",
        "/file-download/end-state/riot/series/7/games/2/summary",
        "/file-download/end-state/riot/series/7/games/3/details",
        "/file-download/replay/riot/series/7/games/2",
    ];
    for route in routes {
        Mock::given(method("GET"))
            .and(path(route))
            .and(header("x-api-key", KEY))
            .and(query_param_is_missing("key"))
            .respond_with(ResponseTemplate::new(200).set_body_string(route))
            .expect(1)
            .mount(&server)
            .await;
    }

    let files = client(&server.uri(), RetryConfig::disabled()).files();
    let downloads = [
        files.grid_events("7").await.unwrap(),
        files.grid_end_state("7").await.unwrap(),
        files.riot_events("7").await.unwrap(),
        files.riot_game_events("7", 2).await.unwrap(),
        files.riot_end_state("7").await.unwrap(),
        files
            .riot_game_state("7", 2, InfoType::Summary)
            .await
            .unwrap(),
        files
            .riot_game_state("7", 3, InfoType::Details)
            .await
            .unwrap(),
        files.riot_game_replay("7", 2).await.unwrap(),
    ];
    let names: Vec<String> = downloads.iter().map(|d| d.file_name().to_owned()).collect();
    assert_eq!(
        names,
        [
            "7-events-grid.zip",
            "7-state-grid.json",
            "7-events-riot.zip",
            "7-events-riot-game-2.jsonl",
            "7-state-riot.zip",
            "7-state-summary-riot-game-2.json",
            "7-state-details-riot-game-3.json",
            "7-replay-riot-game-2",
        ]
    );
    for (download, route) in downloads.into_iter().zip(routes) {
        assert_eq!(download.bytes().await.unwrap(), route.as_bytes());
    }
}

#[tokio::test]
async fn series_ids_are_path_encoded_and_validated() {
    let server = MockServer::start().await;
    Mock::given(path("/file-download/events/grid/series/a%2Fb"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let files = client(&server.uri(), RetryConfig::disabled()).files();
    let download = files.grid_events("a/b").await.unwrap();
    assert_eq!(download.file_name(), "a_b-events-grid.zip");
    for bad in ["", ".", ".."] {
        assert!(matches!(
            files.grid_events(bad).await,
            Err(Error::Config(_))
        ));
    }
}

#[tokio::test]
async fn reads_headers_and_content_disposition() {
    let server = MockServer::start().await;
    Mock::given(path("/file-download/events/grid/series/1"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "content-disposition",
                    "attachment; filename=\"../events_grid_1.jsonl.zip\"",
                )
                .set_body_raw(b"PK-bytes".to_vec(), "application/zip"),
        )
        .mount(&server)
        .await;

    let download = client(&server.uri(), RetryConfig::disabled())
        .files()
        .grid_events("1")
        .await
        .unwrap();
    assert_eq!(download.file_name(), "events_grid_1.jsonl.zip");
    assert_eq!(download.content_type(), Some("application/zip"));
    assert_eq!(download.content_length(), Some(8));

    let stream = download.bytes_stream();
    let mut stream = std::pin::pin!(stream);
    let mut body = Vec::new();
    while let Some(chunk) = futures_util::StreamExt::next(&mut stream).await {
        body.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(body, b"PK-bytes");
}

#[tokio::test]
async fn save_to_writes_the_body_and_leaves_no_temp_file() {
    let server = MockServer::start().await;
    let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    Mock::given(path("/file-download/replay/riot/series/1/games/1"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;

    let dir = temp_dir("save-ok");
    let target = dir.join("replay.bin");
    std::fs::write(&target, b"old contents").unwrap();
    let written = client(&server.uri(), RetryConfig::disabled())
        .files()
        .riot_game_replay("1", 1)
        .await
        .unwrap()
        .save_to(&target)
        .await
        .unwrap();
    assert_eq!(written, body.len() as u64);
    assert_eq!(std::fs::read(&target).unwrap(), body);
    assert_eq!(dir_entries(&dir), ["replay.bin"]);
}

#[tokio::test]
async fn save_to_failure_mid_stream_leaves_no_target() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // A server that promises 100 bytes, sends 10 and hangs up.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = socket.read(&mut buf).await.unwrap();
            if n == 0 {
                return;
            }
            request.extend_from_slice(&buf[..n]);
        }
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n0123456789")
            .await
            .unwrap();
        socket.flush().await.unwrap();
    });

    let dir = temp_dir("save-fail");
    let target = dir.join("events.zip");
    let err = client(&base, RetryConfig::disabled())
        .files()
        .grid_events("1")
        .await
        .unwrap()
        .save_to(&target)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Transport(_)), "{err:?}");
    assert!(!target.exists());
    assert!(dir_entries(&dir).is_empty(), "{:?}", dir_entries(&dir));
}

#[tokio::test]
async fn maps_api_error_messages() {
    let server = MockServer::start().await;
    Mock::given(path("/file-download/end-state/grid/series/404"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({ "message": "File not found" })),
        )
        .mount(&server)
        .await;
    Mock::given(path("/file-download/end-state/grid/series/403"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(json!({ "message": "request is forbidden" })),
        )
        .mount(&server)
        .await;
    Mock::given(path("/file-download/end-state/grid/series/401"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&server)
        .await;

    let files = client(&server.uri(), fast_retry()).files();
    for (id, status, message) in [
        ("404", StatusCode::NOT_FOUND, "File not found"),
        ("403", StatusCode::FORBIDDEN, "request is forbidden"),
    ] {
        let err = files.grid_end_state(id).await.unwrap_err();
        let Error::Api {
            status: got,
            message: ref got_message,
        } = err
        else {
            panic!("expected Error::Api, got {err:?}");
        };
        assert_eq!(got, status);
        assert_eq!(got_message, message);
        assert_eq!(err.status(), Some(status));
        assert!(!format!("{err} {err:?}").contains(KEY));
    }

    let err = files.grid_end_state("401").await.unwrap_err();
    assert!(
        matches!(&err, Error::Http { status, body } if *status == StatusCode::UNAUTHORIZED && body == "Unauthorized"),
        "{err:?}"
    );
    // 4xx responses are not retried.
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn retries_server_errors_before_the_body() {
    let server = MockServer::start().await;
    Mock::given(path("/file-download/list/1"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({ "message": "busy" })))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/file-download/list/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "files": [] })))
        .expect(1)
        .mount(&server)
        .await;

    let files = client(&server.uri(), fast_retry())
        .files()
        .list("1")
        .await
        .unwrap();
    assert_eq!(files, []);
}

#[tokio::test]
async fn persistent_server_errors_end_in_api_error() {
    let server = MockServer::start().await;
    Mock::given(path("/file-download/list/1"))
        .respond_with(ResponseTemplate::new(502).set_body_json(json!({ "message": "bad gateway" })))
        .expect(3)
        .mount(&server)
        .await;

    let err = client(&server.uri(), fast_retry())
        .files()
        .list("1")
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::Api { status, message } if *status == StatusCode::BAD_GATEWAY && message == "bad gateway"),
        "{err:?}"
    );
}

#[tokio::test]
async fn cross_origin_redirects_are_followed_without_the_key() {
    let api = MockServer::start().await;
    let cdn = MockServer::start().await;
    Mock::given(path("/file-download/end-state/grid/series/7"))
        .and(header("x-api-key", KEY))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("location", format!("{}/signed/7.json?sig=abc", cdn.uri())),
        )
        .expect(1)
        .mount(&api)
        .await;
    Mock::given(path("/signed/7.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"state\":1}"))
        .expect(1)
        .mount(&cdn)
        .await;

    let body = client(&api.uri(), RetryConfig::disabled())
        .files()
        .grid_end_state("7")
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(&body[..], b"{\"state\":1}");
    let requests = cdn.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key("x-api-key"));
}

#[tokio::test]
async fn cross_origin_redirect_errors_are_mapped() {
    let api = MockServer::start().await;
    let cdn = MockServer::start().await;
    Mock::given(path("/file-download/end-state/grid/series/7"))
        .respond_with(
            ResponseTemplate::new(307).insert_header("location", format!("{}/expired", cdn.uri())),
        )
        .mount(&api)
        .await;
    Mock::given(path("/expired"))
        .respond_with(ResponseTemplate::new(403).set_body_string("<Error>expired</Error>"))
        .mount(&cdn)
        .await;

    let err = client(&api.uri(), RetryConfig::disabled())
        .files()
        .grid_end_state("7")
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(StatusCode::FORBIDDEN), "{err:?}");
}

#[tokio::test]
async fn same_origin_redirects_keep_the_key() {
    let server = MockServer::start().await;
    Mock::given(path("/file-download/end-state/grid/series/7"))
        .respond_with(
            ResponseTemplate::new(301).insert_header("location", "/file-download/moved/7"),
        )
        .mount(&server)
        .await;
    Mock::given(path("/file-download/moved/7"))
        .and(header("x-api-key", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .expect(1)
        .mount(&server)
        .await;

    let body = client(&server.uri(), RetryConfig::disabled())
        .files()
        .grid_end_state("7")
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    assert_eq!(&body[..], b"ok");
}

#[cfg(feature = "zip")]
mod archive {
    use std::io::Write;

    use serde::Deserialize;
    use zip::write::SimpleFileOptions;

    use super::*;

    fn archive() -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        writer.add_directory("logs/", options).unwrap();
        writer.start_file("logs/part-1.jsonl", options).unwrap();
        writer
            .write_all(
                b"{\"id\":\"a\",\"sequenceNumber\":1}\n\n{\"id\":\"b\",\"sequenceNumber\":2}\n",
            )
            .unwrap();
        writer.start_file("logs/part-2.jsonl", options).unwrap();
        writer
            .write_all(b"{\"id\":\"c\",\"sequenceNumber\":3}")
            .unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Line {
        id: String,
        sequence_number: u32,
    }

    #[test]
    fn parses_jsonl_entries_in_order() {
        let lines: Vec<Line> = gridgg::files::parse_events_archive(&archive()).unwrap();
        let ids: Vec<(&str, u32)> = lines
            .iter()
            .map(|l| (l.id.as_str(), l.sequence_number))
            .collect();
        assert_eq!(ids, [("a", 1), ("b", 2), ("c", 3)]);
    }

    #[test]
    fn rejects_bad_archives_and_lines() {
        let err = gridgg::files::parse_events_archive::<Line>(b"not a zip").unwrap_err();
        assert!(matches!(err, Error::Archive(_)), "{err:?}");

        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file("x.jsonl", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{\"id\":1}\n").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let err = gridgg::files::parse_events_archive::<Line>(&bytes).unwrap_err();
        assert!(matches!(err, Error::Decode(_)), "{err:?}");
    }

    #[tokio::test]
    async fn download_events_archive() {
        let server = MockServer::start().await;
        Mock::given(path("/file-download/events/grid/series/9"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(archive(), "application/zip"))
            .mount(&server)
            .await;

        let values: Vec<serde_json::Value> = client(&server.uri(), RetryConfig::disabled())
            .files()
            .grid_events("9")
            .await
            .unwrap()
            .events_archive()
            .await
            .unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(values[2], json!({ "id": "c", "sequenceNumber": 3 }));
    }

    #[cfg(feature = "events")]
    #[tokio::test]
    async fn download_grid_events_transactions() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/series_events_transaction.json")).unwrap();
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file("events.jsonl", SimpleFileOptions::default())
            .unwrap();
        writeln!(writer, "{fixture}").unwrap();
        let bytes = writer.finish().unwrap().into_inner();

        let server = MockServer::start().await;
        Mock::given(path("/file-download/events/grid/series/2618418"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(bytes, "application/zip"))
            .mount(&server)
            .await;

        let transactions = client(&server.uri(), RetryConfig::disabled())
            .files()
            .grid_events("2618418")
            .await
            .unwrap()
            .grid_events_transactions()
            .await
            .unwrap();
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].sequence_number, 1342);
        assert_eq!(transactions[0].events.len(), 3);
    }
}
