//! Lists the five most recent League of Legends series, then fetches the
//! newest one by ID.
//!
//! ```sh
//! GRID_API_KEY=... cargo run --example central_series
//! ```

use std::error::Error;
use std::pin::pin;

use futures_util::{StreamExt, TryStreamExt};
use gridgg::central::{DateTimeFilter, OrderDirection, SeriesFilter, SeriesOrderBy};
use gridgg::pagination::PageRequest;
use gridgg::{Client, Environment};

/// GRID's title ID for League of Legends.
const LOL_TITLE_ID: &str = "3";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let Ok(key) = std::env::var("GRID_API_KEY") else {
        eprintln!("GRID_API_KEY is not set; export your GRID API key first");
        std::process::exit(1);
    };
    let client = Client::builder()
        .api_key(key)
        .environment(Environment::OpenAccess)
        .build()?;

    // Series that were scheduled to start before now, newest first.
    let filter = SeriesFilter::default()
        .title_ids([LOL_TITLE_ID])
        .start_time_scheduled(DateTimeFilter::default().lte(jiff::Timestamp::now().to_string()));
    let request = client
        .central()
        .all_series(filter)
        .order_by(SeriesOrderBy::StartTimeScheduled)
        .order_direction(OrderDirection::Desc)
        .page_request(PageRequest::forward(5, None)?);

    // `take` stops the stream after the first page; streams are not `Unpin`.
    let mut recent = pin!(request.stream().take(5));
    let mut newest = None;
    while let Some(series) = recent.try_next().await? {
        let teams: Vec<_> = series
            .teams
            .iter()
            .map(|t| t.base_info.name.as_str())
            .collect();
        println!(
            "{:>8}  {}  {:<40}  {}",
            series.id.inner(),
            series.start_time_scheduled,
            series.tournament.name,
            teams.join(" vs "),
        );
        newest.get_or_insert(series.id);
    }

    let Some(id) = newest else {
        println!("no series found");
        return Ok(());
    };
    match client.central().series(id.clone()).await? {
        Some(series) => println!(
            "\nseries {}: {} {}, {} stream(s), workflow status {:?}",
            series.id.inner(),
            series.title.name,
            series.format.name_shortened,
            series.streams.len(),
            series.workflow_status,
        ),
        None => println!("\nseries {} not found", id.inner()),
    }
    Ok(())
}
