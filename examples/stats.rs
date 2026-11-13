//! Prints a team's statistics over the series it played in the last year.
//!
//! ```sh
//! GRID_API_KEY=... cargo run --example stats -- 3416
//! ```

use std::error::Error;

use gridgg::stats::{TeamGameStatisticsDetails, TeamStatisticsFilter, TimeRangeFilter};
use gridgg::{Client, Environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let Ok(key) = std::env::var("GRID_API_KEY") else {
        eprintln!("GRID_API_KEY is not set; export your GRID API key first");
        std::process::exit(1);
    };
    let team_id = std::env::args().nth(1).unwrap_or_else(|| "3416".into());
    let client = Client::builder()
        .api_key(key)
        .environment(Environment::OpenAccess)
        .build()?;

    // The API rejects an empty filter; restrict to the last year.
    let filter = TeamStatisticsFilter::default().started_at(TimeRangeFilter::LastYear);
    let stats = client
        .stats()
        .team_statistics(team_id.as_str(), filter)
        .await?;

    let series = &stats.series;
    let wins = series.won.iter().find(|w| w.value).map_or(0, |w| w.count);
    println!(
        "team {}: {} series ({wins} won), {:.1} kills per series",
        stats.id.inner(),
        series.count,
        series.kills.avg,
    );

    let game = &stats.game;
    println!(
        "{} games, {:.1} kills and {:.1} deaths per game",
        game.count, game.kills.avg, game.deaths.avg,
    );
    match &game.details {
        TeamGameStatisticsDetails::Lol(lol) => {
            println!(
                "League of Legends: {:.1} vision score per game",
                lol.vision_score.avg
            );
        }
        TeamGameStatisticsDetails::Valorant(_) => println!("Valorant game details available"),
        _ => println!("no title-specific game details"),
    }
    Ok(())
}
