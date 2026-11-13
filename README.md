# gridgg

An async Rust SDK for the [GRID](https://grid.gg) esports data APIs:

| API | Handle | Protocol |
|---|---|---|
| Central Data | `client.central()` | GraphQL |
| Series State | `client.series_state()` | GraphQL |
| Stats Feed | `client.stats()` | GraphQL |
| File Download | `client.files()` (feature `files`) | REST |
| Series Events | `client.events()` (feature `events`) | WebSocket |

Each GraphQL API has prebuilt typed queries, and accepts your own
[cynic](https://cynic-rs.dev) operations or raw query strings for anything
else. Game-specific data is typed for League of Legends and Valorant; other
titles get the fields common to all titles.

The SDK runs on tokio; there is no blocking client. Its dependencies need
Rust 1.88 or newer (checked in CI).

## Install

```toml
[dependencies]
gridgg = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

| Feature | Default | Enables |
|---|---|---|
| `files` | yes | The File Download API. |
| `events` | yes | The Series Events WebSocket API. |
| `zip` | no | Unpacking GRID's zipped event archives (implies `files`); with `events`, into typed Series Events transactions. |

Use `default-features = false` if you only need the GraphQL APIs.

## Quick start

```rust,no_run
use gridgg::{Client, Environment};

#[tokio::main]
async fn main() -> gridgg::Result<()> {
    let client = Client::builder()
        .api_key(std::env::var("GRID_API_KEY").expect("GRID_API_KEY is not set"))
        .environment(Environment::OpenAccess)
        .build()?;

    // Central Data: the series and its teams.
    if let Some(series) = client.central().series("3018394").await? {
        println!("{} at {}", series.tournament.name, series.start_time_scheduled);
    }

    // Series State: the final score.
    if let Some(state) = client.series_state().get("3018394").await? {
        for team in &state.teams {
            println!("{}: {}", team.name, team.score);
        }
    }
    Ok(())
}
```

`Client` is cheap to clone; clones share the connection pool, rate limiter
and configuration. Every call returns `gridgg::Result`; `429` and `5xx`
responses, connection failures and GraphQL throttling errors are retried
with backoff (see `RetryConfig`). The API key is never logged by the SDK and
is redacted from errors.

## Access tiers

| | Open Access | Commercial |
|---|---|---|
| GraphQL host | `api-op.grid.gg` | `api.grid.gg` (unverified) |
| File Download, Series Events host | `api.grid.gg` | `api.grid.gg` |
| GraphQL rate limit | 20 requests/minute | 40 requests/minute |
| Stats Feed rate limit (within the above) | 10 requests/minute (observed live) | none (unverified) |

Select one with `Environment::OpenAccess` (the default) or
`Environment::Commercial`. The client spaces GraphQL requests to stay under
the limit; one limiter is shared by the three GraphQL APIs and all clones of
a client. GRID documents only the Open Access GraphQL endpoints, so the
commercial GraphQL host is a best guess. Every base URL and limit can be
overridden on the builder:

```rust,no_run
use gridgg::{Client, Environment, RateLimit};

let client = Client::builder()
    .api_key(std::env::var("GRID_API_KEY")?)
    .environment(Environment::Commercial)
    .central_url("https://api.grid.gg/central-data/graphql")
    .stats_rate_limit(RateLimit::PerMinute(20))
    .build()?;
```

The other overrides are `series_state_url`, `stats_url`, `files_url`,
`events_url`, `rate_limit`, `retry`, `timeout` and `http_client`.

## APIs

### Central Data

One method per root field: single objects by ID (`series`, `team`, ...),
lists (`titles`, ...), paged connections with `page()` and `stream()`
(`all_series`, `teams`, ...) and external-ID lookups.

```rust,no_run
use futures_util::TryStreamExt;
use gridgg::central::{SeriesFilter, SeriesType};

let filter = SeriesFilter::default()
    .title_ids(["3"]) // League of Legends
    .types([SeriesType::Esports]);
// Streams are not `Unpin`; pin them to call `try_next`.
let mut all = std::pin::pin!(client.central().all_series(filter).stream());
while let Some(series) = all.try_next().await? {
    println!("{} at {}", series.id.inner(), series.start_time_scheduled);
}
```

### Series State

`get` (by series ID) and `latest_by_player_id` return the series with its
games, teams, players, draft and segments (Valorant rounds). Title-specific
fields are in a `details` enum:

```rust,no_run
use gridgg::series_state::GamePlayerDetails;

if let Some(series) = client.series_state().get("3018394").await? {
    println!("data-model version {}", series.version);
    for game in &series.games {
        for team in &game.teams {
            for player in &team.players {
                if let GamePlayerDetails::Lol(lol) = &player.details {
                    println!("{}: vision score {:?}", player.name, lol.vision_score);
                }
            }
        }
    }
}
```

### Stats Feed

`team_statistics`, `team_game_statistics`, `player_statistics`,
`game_statistics` and `series_statistics`. Three of them reject an empty
filter, so set at least a time range:

```rust,no_run
use gridgg::stats::{TeamStatisticsFilter, TimeRangeFilter};

let filter = TeamStatisticsFilter::default().started_at(TimeRangeFilter::LastYear);
let stats = client.stats().team_statistics("3416", filter).await?;
println!("{} series, {:.1} kills per game", stats.series.count, stats.game.kills.avg);
```

### File Download

```rust,no_run
for file in client.files().list("3018394").await? {
    println!("{} {}", file.id, file.status);
}
let bytes = client.files().grid_events("3018394").await?.save_to("events.zip").await?;
```

The key is sent in the `x-api-key` header. With the built-in HTTP client, a
redirect to another origin is followed once without that header.

### Series Events

```rust,no_run
use futures_util::StreamExt;

let mut stream = client.events().connect("3018394").await?;
while let Some(transaction) = stream.next().await {
    let transaction = transaction?;
    for event in &transaction.events {
        println!("{} {}", transaction.sequence_number, event.event_type);
    }
}
```

`client.events().series(id)` configures keepalive pings (30 s by default)
and reconnection (off by default). The stream skips any transaction whose
sequence number is not greater than the last one it yielded.

## Custom queries

Every GraphQL handle has `raw`, which takes a query string and JSON
variables and returns `data` as JSON:

```rust,no_run
let data = client
    .central()
    .raw("{ titles { id name } }", serde_json::Value::Null)
    .await?;
```

and `run`, which sends a typed cynic operation built against the crate's
schema modules (`gridgg::central::schema`, `gridgg::series_state::schema`,
`gridgg::stats::schema`). cynic also needs the schema registered in your
crate's build script; the SDL is exported as `gridgg::CENTRAL_SDL`,
`gridgg::SERIES_STATE_SDL` and `gridgg::STATS_SDL`:

```toml
[dependencies]
gridgg = "0.2"
cynic = "3"

[build-dependencies]
gridgg = { version = "0.2", default-features = false }
cynic-codegen = "3"
```

```rust,ignore
// build.rs
fn main() {
    cynic_codegen::register_schema("central")
        .from_sdl(gridgg::CENTRAL_SDL)
        .unwrap();
}
```

Then derive with `schema = "central"` and
`schema_module = "gridgg::central::schema"`. The crate documentation has a
complete example.

## Known limitations

- **Open Access field denials.** Open Access keys get `PERMISSION_DENIED` for
  `Title.logoUrl`, `Team.rating` and `Player.fullName`/`age`/`imageUrl`/
  `nationality`; a denied field fails the whole query, so the prebuilt
  queries do not select them. Query them with `run` and a commercial key.
- **File Download needs a commercial key.** Open Access keys get
  `403 request is forbidden`, so the File Download API is tested only
  against a mock server built from GRID's published spec.
- **Series State version gating.** GRID rejects a query that selects a field
  newer than the series' data-model version. The prebuilt queries try all
  fields first (one request) and, for older series, look up the version and
  resend without the newer fields (three requests, all rate limited). Those
  fields are `None` for older data. Data older than version 1.1 is not
  supported. Your own Series State queries must do their own gating.
- **Valorant game statistics.** On the Open Access Stats Feed, Valorant team
  and player game statistics come back as the generic `Default` types, so
  they decode to the `Other` variant without Valorant fields.
- **Event archive layout.** GRID documents the zipped Series Events file
  only as "JSONL"; unpacking it (feature `zip`) assumes one or more JSONL
  entries. The archive is held in memory with no size limit; use `save_to`
  or `bytes_stream` for large files.
- **Series Events replay.** For a finished series GRID closes the connection
  at once and sends nothing. Whether GRID replays transactions after a
  reconnect is unverified; the stream de-duplicates either way.
- **WebSocket key logging.** GRID requires the key in the WebSocket URL. The
  SDK keeps it out of its errors and logs, but `tungstenite` logs the
  handshake request, key included, at `trace` level under target
  `tungstenite::handshake::client`. Keep that target below `trace`.
- **Commercial endpoints and other titles** are unverified: they could not be
  checked with an Open Access key.

## Examples and tests

The [examples](examples) read the key from `GRID_API_KEY` and use Open
Access:

```sh
GRID_API_KEY=... cargo run --example central_series
GRID_API_KEY=... cargo run --example series_state -- 3018394
GRID_API_KEY=... cargo run --example stats -- 3416
GRID_API_KEY=... cargo run --example files -- 3018394
GRID_API_KEY=... cargo run --example events -- 3018394
```

`cargo test --all-features` runs against mock servers. The live checks are
ignored by default; run them with
`GRID_API_KEY=... cargo test --all-features -- --ignored`.

## License

Licensed under the [MIT No Attribution License](LICENSE) (MIT-0).
