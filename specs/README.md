# GRID Esports Data APIs — machine-readable specs

Source: GRID Data Portal API Reference (portal.grid.gg, login required), compiled 2026-11-12.

| File | API | Protocol | Format |
|---|---|---|---|
| `central-data.graphql` | Central Data Feed (titles, tournaments, teams, players, series; queries + mutations) | GraphQL over HTTPS POST | GraphQL SDL |
| `series-state.graphql` | Live Data Feed — Series State (live + final state of a series) | GraphQL over HTTPS POST | GraphQL SDL |
| `stats-feed.graphql` | Stats Feed (aggregated team/player/series/game statistics) | GraphQL over HTTPS POST | GraphQL SDL |
| `series-events.asyncapi.yaml` | Live Data Feed — Series Events (push stream) | WebSocket (wss) | AsyncAPI 3.0 |
| `file-download.openapi.yaml` | Live Data Feed — File Download (post-series files) | REST over HTTPS | OpenAPI 3.0.3 (GRID's own spec, verbatim) |

## Endpoints

| API | URL |
|---|---|
| Central Data | `https://api-op.grid.gg/central-data/graphql` |
| Series State | `https://api-op.grid.gg/live-data-feed/series-state/graphql` |
| Stats Feed | `https://api-op.grid.gg/statistics-feed/graphql` |
| Series Events | `wss://api.grid.gg/live-data-feed/series/{seriesId}?key={API_KEY}` |
| File Download | `https://api.grid.gg/file-download` |

The three GraphQL URLs are the "Open Access" endpoints from the portal's GQL Playground; the reference pages don't list GraphQL hosts. Closed Platform accounts may use a different host (likely `api.grid.gg`). Make the base URLs configurable.

## Authentication

- HTTP (GraphQL + REST): header `x-api-key: {API_KEY}`. File Download also accepts query param `key`.
- WebSocket: query param `key={API_KEY}` on the connect URL.
- HTTPS/WSS only.

## Conventions not expressible in the specs

- **Rate limits (GraphQL):** 40 req/min on Closed Platform, 20 req/min on Open Platform; excess requests are rejected.
- **No bulk requests:** one request per object.
- **Pagination (Relay connections):** `first`/`after` or `last`/`before` (never mix). Max page size 50, default 10. Loop on `pageInfo.endCursor` while `pageInfo.hasNextPage`. Connections expose `totalCount`, `pageInfo`, `edges { cursor node }`.
- **Dates:** ISO 8601 strings (`DateTime`, `Date`, `Duration` scalars; `DateTimeFilter.gte/lte` in Central Data are `String`s in ISO 8601, inclusive).
- **Series State versioning:** `SeriesState.version` returns the data-model version (`MAJOR.MINOR`). Field descriptions carry annotations like `(available from version "3.17")`.
- **Inferred parts of the SDL:** `implements` clauses were inferred, because the docs list interfaces but not which objects implement them. Fields described as "Use X instead" are marked `@deprecated`.
- **WebSocket limits (per key):** connection-attempt and concurrent-connection caps (defaults not published). Idle connections close 24 h after the last frame, so send Ping frames to keep them alive.

## GraphQL error shape

Errors come back in the standard `errors` array, with GRID details in `extensions`:

```json
{"message": "...", "extensions": {"code": "DOWNSTREAM_SERVICE_ERROR", "serviceName": "...",
  "errorType": "<ErrorType>", "errorDetail": "<ErrorDetail>", "exception": {"message": "...", "locations": [], "path": ["..."]}}}
```

`errorType` / `errorDetail` values are the `ErrorType` / `ErrorDetail` enums in `central-data.graphql`; each value's description gives its HTTP mapping. Common cases:
- `UNAUTHENTICATED`: unknown key.
- `PERMISSION_DENIED`: key lacks access to the product/content.
- `BAD_REQUEST`/`INVALID_ARGUMENT`: mixed `first`/`last`, `from` >= `to`, non-ISO date, invalid cursor, or unknown title/tournament/series/team ID.

## Known quirks in GRID's spec

- File Download: `/end-state/riot/series/{series_id}/games/{sequence_number}/{info_type}` lists `ApiKeyAuth` twice under `security`. It's probably a typo for `ApiKeyAuth` + `ApiKeyQueryParamAuth`.
- Series Events: `player-revived-player` appears both as a cross-title event and as a Rainbow Six DBNO event. The enum lists it once, and `x-grid-event-types` keeps both entries.
