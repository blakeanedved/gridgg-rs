//! Series Events tests against a local WebSocket server.

#![cfg(feature = "events")]
// tungstenite's handshake callback must return its large `ErrorResponse`.
#![allow(clippy::result_large_err)]

use std::fmt::Write as _;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use gridgg::events::{EventStream, EventType, Transaction};
use gridgg::{Client, Error, RetryConfig};
use serde_json::json;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

const KEY: &str = "test-key-0123456789";
const TIMEOUT: Duration = Duration::from_secs(10);

type Server = WebSocketStream<TcpStream>;

/// Serves WebSocket connections one at a time; `handler` gets the 1-based
/// connection number, the socket and the request URI.
async fn serve<F, Fut>(handler: F) -> (String, JoinHandle<()>)
where
    F: Fn(u32, Server, String) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut connection = 0;
        loop {
            connection += 1;
            let (tcp, _) = listener.accept().await.unwrap();
            let mut uri = String::new();
            let callback = |request: &Request, response: Response| {
                uri = request.uri().to_string();
                Ok(response)
            };
            let socket = tokio_tungstenite::accept_hdr_async(tcp, callback)
                .await
                .unwrap();
            handler(connection, socket, uri).await;
        }
    });
    (format!("ws://{addr}/live-data-feed/series"), task)
}

fn client(events_url: &str) -> Client {
    Client::builder()
        .api_key(KEY)
        .events_url(events_url)
        .build()
        .unwrap()
}

fn transaction(sequence: u64, event_type: &str) -> Message {
    Message::text(
        json!({
            "id": format!("tx-{sequence}"),
            "occurredAt": "2026-11-12T10:00:00Z",
            "publishedAt": "2026-11-12T10:00:00.5Z",
            "seriesId": "42",
            "sequenceNumber": sequence,
            "events": [{
                "id": format!("ev-{sequence}"),
                "type": event_type,
                "actor": { "type": "player", "id": "p1" },
                "action": "killed",
                "target": { "type": "player", "id": "p2" }
            }]
        })
        .to_string(),
    )
}

async fn close(socket: &mut Server, code: CloseCode) {
    socket
        .close(Some(CloseFrame {
            code,
            reason: "bye".into(),
        }))
        .await
        .unwrap();
}

async fn collect(stream: EventStream) -> Vec<gridgg::Result<Transaction>> {
    tokio::time::timeout(TIMEOUT, stream.collect())
        .await
        .expect("stream did not end")
}

fn sequences(items: &[gridgg::Result<Transaction>]) -> Vec<u64> {
    items
        .iter()
        .map(|item| item.as_ref().unwrap().sequence_number)
        .collect()
}

#[tokio::test]
async fn decodes_transactions_in_order_and_ends_on_normal_close() {
    let (url, _server) = serve(|_, mut socket, uri| async move {
        assert_eq!(uri, format!("/live-data-feed/series/42?key={KEY}"));
        socket
            .send(transaction(1, "player-killed-player"))
            .await
            .unwrap();
        socket
            .send(transaction(2, "player-!killed-player"))
            .await
            .unwrap();
        socket
            .send(Message::binary(
                transaction(3, "drone-zapped-player").into_data(),
            ))
            .await
            .unwrap();
        close(&mut socket, CloseCode::Normal).await;
    })
    .await;

    let items = collect(client(&url).events().connect("42").await.unwrap()).await;
    assert_eq!(sequences(&items), [1, 2, 3]);
    let types: Vec<&EventType> = items
        .iter()
        .map(|item| &item.as_ref().unwrap().events[0].event_type)
        .collect();
    assert_eq!(
        types,
        [
            &EventType::PlayerKilledPlayer,
            &EventType::Reverted(Box::new(EventType::PlayerKilledPlayer)),
            &EventType::Unknown("drone-zapped-player".into()),
        ]
    );
}

#[tokio::test]
async fn sends_keepalive_pings() {
    let (url, _server) = serve(|_, mut socket, _| async move {
        // Ends the stream once the client's first ping arrives.
        while let Some(message) = socket.next().await {
            if matches!(message.unwrap(), Message::Ping(_)) {
                close(&mut socket, CloseCode::Normal).await;
                return;
            }
        }
    })
    .await;

    let stream = client(&url)
        .events()
        .series("42")
        .keepalive(Duration::from_millis(20))
        .connect()
        .await
        .unwrap();
    assert!(collect(stream).await.is_empty());
}

#[tokio::test]
async fn reconnects_and_skips_replayed_transactions() {
    let (url, _server) = serve(|connection, mut socket, _| async move {
        let sequences: &[u64] = if connection == 1 { &[1, 2] } else { &[1, 2, 3] };
        for &sequence in sequences {
            socket
                .send(transaction(sequence, "team-won-round"))
                .await
                .unwrap();
        }
        if connection == 1 {
            // Drop the TCP connection without a close frame.
            drop(socket);
        } else {
            close(&mut socket, CloseCode::Normal).await;
        }
    })
    .await;

    let stream = client(&url)
        .events()
        .series("42")
        .reconnect(
            RetryConfig::new()
                .max_attempts(3)
                .base_delay(Duration::from_millis(10)),
        )
        .connect()
        .await
        .unwrap();
    assert_eq!(sequences(&collect(stream).await), [1, 2, 3]);
}

fn reconnecting(url: &str) -> gridgg::events::EventStreamBuilder {
    client(url).events().series("42").reconnect(
        RetryConfig::new()
            .max_attempts(3)
            .base_delay(Duration::from_millis(10)),
    )
}

#[tokio::test]
async fn replayed_transactions_do_not_reset_the_reconnect_budget() {
    let connections = Arc::new(AtomicU32::new(0));
    let seen = Arc::clone(&connections);
    let (url, _server) = serve(move |connection, mut socket, _| {
        seen.store(connection, Ordering::SeqCst);
        async move {
            // Every connection replays the same transactions, then drops.
            for sequence in [1, 2] {
                socket
                    .send(transaction(sequence, "team-won-round"))
                    .await
                    .unwrap();
            }
            drop(socket);
        }
    })
    .await;

    let items = collect(reconnecting(&url).connect().await.unwrap()).await;
    assert_eq!(items.len(), 3, "{items:?}");
    assert_eq!(sequences(&items[..2]), [1, 2]);
    assert!(
        matches!(&items[2], Err(Error::WebSocket(_))),
        "{:?}",
        items[2]
    );
    // The first connection plus `max_attempts` reconnections.
    assert_eq!(connections.load(Ordering::SeqCst), 4);
}

/// Serves the first connection as a WebSocket that sends transaction 1 and
/// drops; rejects every later handshake with `status`. Returns the URL and
/// the number of connections accepted so far.
async fn drop_then_reject(status: u16) -> (String, Arc<AtomicU32>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let connections = Arc::new(AtomicU32::new(0));
    let count = Arc::clone(&connections);
    tokio::spawn(async move {
        loop {
            let (tcp, _) = listener.accept().await.unwrap();
            if count.fetch_add(1, Ordering::SeqCst) == 0 {
                let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
                socket.send(transaction(1, "team-won-round")).await.unwrap();
                drop(socket);
            } else {
                let reject = |_: &Request, _: Response| -> Result<Response, ErrorResponse> {
                    Err(http::Response::builder().status(status).body(None).unwrap())
                };
                let _ = tokio_tungstenite::accept_hdr_async(tcp, reject).await;
            }
        }
    });
    (format!("ws://{addr}/live-data-feed/series"), connections)
}

#[tokio::test]
async fn failed_reconnects_end_the_stream_after_max_attempts() {
    // Retryable rejections: exactly `max_attempts` reconnections.
    let (url, connections) = drop_then_reject(503).await;
    let items = collect(reconnecting(&url).connect().await.unwrap()).await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].as_ref().unwrap().sequence_number, 1);
    let err = items[1].as_ref().unwrap_err();
    assert_eq!(err.status().map(|s| s.as_u16()), Some(503), "{err:?}");
    assert_eq!(connections.load(Ordering::SeqCst), 4);

    // Refused connections: the listener is gone after the first one.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        drop(listener);
        let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
        socket.send(transaction(1, "team-won-round")).await.unwrap();
    });
    let url = format!("ws://{addr}/live-data-feed/series");
    let items = collect(reconnecting(&url).connect().await.unwrap()).await;
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].as_ref().unwrap().sequence_number, 1);
    assert!(
        matches!(&items[1], Err(Error::WebSocket(_))),
        "{:?}",
        items[1]
    );
}

#[tokio::test]
async fn reconnect_rejected_with_a_client_error_is_not_retried() {
    let (url, connections) = drop_then_reject(403).await;
    let items = collect(reconnecting(&url).connect().await.unwrap()).await;
    assert_eq!(items.len(), 2, "{items:?}");
    let err = items[1].as_ref().unwrap_err();
    assert_eq!(err.status().map(|s| s.as_u16()), Some(403), "{err:?}");
    assert_eq!(connections.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn abnormal_disconnects_end_in_errors_without_reconnect() {
    let (url, _server) = serve(|connection, mut socket, _| async move {
        socket.send(transaction(1, "team-won-round")).await.unwrap();
        if connection == 1 {
            drop(socket);
        } else {
            close(&mut socket, CloseCode::Error).await;
        }
    })
    .await;

    let events = client(&url).events();
    let items = collect(events.connect("42").await.unwrap()).await;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].as_ref().unwrap().sequence_number, 1);
    assert!(
        matches!(&items[1], Err(Error::WebSocket(_))),
        "{:?}",
        items[1]
    );

    let items = collect(events.connect("42").await.unwrap()).await;
    assert_eq!(items.len(), 2);
    assert!(
        matches!(&items[1], Err(Error::WebSocketClosed { code: 1011, reason }) if reason == "bye"),
        "{:?}",
        items[1]
    );
}

#[tokio::test]
async fn invalid_messages_are_errors_and_the_stream_continues() {
    let (url, _server) = serve(|_, mut socket, _| async move {
        socket.send(Message::text("not json")).await.unwrap();
        socket.send(transaction(5, "team-won-round")).await.unwrap();
        close(&mut socket, CloseCode::Normal).await;
    })
    .await;

    let items = collect(client(&url).events().connect("42").await.unwrap()).await;
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[0], Err(Error::Decode(_))), "{:?}", items[0]);
    assert_eq!(items[1].as_ref().unwrap().sequence_number, 5);
}

#[tokio::test]
async fn errors_never_contain_the_key() {
    let check = |err: &Error| {
        let mut text = format!("{err} {err:?}");
        let mut source = std::error::Error::source(err);
        while let Some(cause) = source {
            write!(text, " {cause} {cause:?}").unwrap();
            source = cause.source();
        }
        assert!(!text.contains(KEY), "{text}");
    };

    // Handshake rejected with 403.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let reject = |_: &Request, _: Response| -> Result<Response, ErrorResponse> {
            Err(http::Response::builder()
                .status(403)
                .body(Some("request is forbidden".into()))
                .unwrap())
        };
        let _ = tokio_tungstenite::accept_hdr_async(tcp, reject).await;
    });
    let err = client(&format!("ws://{addr}/live-data-feed/series"))
        .events()
        .connect("42")
        .await
        .unwrap_err();
    assert_eq!(err.status().map(|s| s.as_u16()), Some(403));
    check(&err);

    // Connection refused.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let err = client(&format!("ws://{addr}/live-data-feed/series"))
        .events()
        .connect("42")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::WebSocket(_)), "{err:?}");
    check(&err);

    // Debug output of the handles.
    let client = client("ws://127.0.0.1:1/x");
    let builder = client.events().series("42");
    assert!(!format!("{builder:?}").contains(KEY));
}

#[test]
fn rejects_invalid_series_ids() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    for id in ["", ".", ".."] {
        let err = runtime
            .block_on(client("ws://127.0.0.1:1/x").events().connect(id))
            .unwrap_err();
        assert!(matches!(err, Error::Config(_)), "{err:?}");
    }
}

#[test]
fn decodes_the_spec_fixture() {
    let raw = include_str!("fixtures/series_events_transaction.json");
    let transaction: Transaction = serde_json::from_str(raw).unwrap();
    assert_eq!(transaction.series_id, "2618418");
    assert_eq!(transaction.sequence_number, 1342);
    assert_eq!(transaction.session_sequence_number, Some(17));
    assert_eq!(
        transaction.occurred_at,
        "2026-11-12T18:04:31.25Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
    );

    let [kill, revert, unknown] = &transaction.events[..] else {
        panic!("expected 3 events");
    };
    assert_eq!(kill.event_type, EventType::PlayerKilledPlayer);
    assert!(!kill.event_type.is_reverted());
    assert_eq!(kill.actor.entity_type, "player");
    assert_eq!(kill.actor.state_delta, Some(json!({ "kills": 1 })));
    assert_eq!(kill.target.state, None);
    assert!(kill.series_state_delta.is_some());

    assert!(revert.event_type.is_reverted());
    assert_eq!(revert.event_type.base(), &EventType::PlayerKilledPlayer);
    assert_eq!(revert.action, "!killed");
    assert_eq!(revert.reverted_event_id.as_deref(), Some("e-1"));

    assert_eq!(
        unknown.event_type,
        EventType::Unknown("drone-zapped-player".into())
    );
    assert_eq!(unknown.includes_full_state, Some(true));

    // Serializing gives back the same JSON.
    let original: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert_eq!(serde_json::to_value(&transaction).unwrap(), original);
}

#[tokio::test]
#[ignore = "needs GRID_API_KEY and network access"]
async fn live_finished_series_closes_cleanly() {
    let key = std::env::var("GRID_API_KEY").expect("GRID_API_KEY must be set");
    let client = Client::builder().api_key(key).build().unwrap();
    let stream = client.events().connect("3018394").await.unwrap();
    let items = tokio::time::timeout(Duration::from_secs(60), stream.collect::<Vec<_>>())
        .await
        .expect("stream did not end");
    println!("finished series 3018394: {} item(s)", items.len());
    for item in &items {
        assert!(item.is_ok(), "{item:?}");
    }
}
