//! Prints the state of one series: its data-model version, the series and
//! game scores, and a title-specific summary per team.
//!
//! ```sh
//! GRID_API_KEY=... cargo run --example series_state -- 3018394
//! ```

use std::error::Error;

use gridgg::series_state::GameTeamDetails;
use gridgg::{Client, Environment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let Ok(key) = std::env::var("GRID_API_KEY") else {
        eprintln!("GRID_API_KEY is not set; export your GRID API key first");
        std::process::exit(1);
    };
    let series_id = std::env::args().nth(1).unwrap_or_else(|| "3018394".into());
    let client = Client::builder()
        .api_key(key)
        .environment(Environment::OpenAccess)
        .build()?;

    let Some(series) = client.series_state().get(series_id.as_str()).await? else {
        println!("series {series_id} has no state");
        return Ok(());
    };
    let title = series
        .title
        .as_ref()
        .map_or("?", |t| t.name_shortened.as_str());
    println!(
        "series {} ({title}, {}), data-model version {}, finished: {}",
        series.id.inner(),
        series.format,
        series.version,
        series.finished,
    );
    for team in &series.teams {
        println!("  {:<30} {}", team.name, team.score);
    }

    for game in &series.games {
        println!("game {} on {}:", game.sequence_number, game.map.name);
        for team in &game.teams {
            let summary = match &team.details {
                // Fields newer than the series' data-model version are `None`.
                GameTeamDetails::Lol(lol) => format!(
                    "gold earned {}, vision score {}",
                    or_dash(lol.total_money_earned),
                    or_dash(lol.vision_score.map(|v| format!("{v:.1}"))),
                ),
                GameTeamDetails::Valorant(val) => format!(
                    "headshots {}, damage dealt {}",
                    val.headshots,
                    or_dash(val.damage_dealt),
                ),
                _ => "no title-specific details".to_owned(),
            };
            println!(
                "  {:<30} score {:>2}, won {:<5}, K/D {}/{}, {summary}",
                team.name, team.score, team.won, team.kills, team.deaths,
            );
        }
        match &game.segments {
            Some(segments) if !segments.is_empty() => {
                println!("  {} segment(s)", segments.len());
            }
            _ => {}
        }
    }
    Ok(())
}

fn or_dash(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| v.to_string())
}
