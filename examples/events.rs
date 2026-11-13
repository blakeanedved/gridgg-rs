//! Prints a series' live Series Events transactions until the stream ends.
//!
//! For a finished series GRID closes the connection at once, so the stream
//! ends without any transactions.
//!
//! ```sh
//! GRID_API_KEY=... cargo run --example events -- 3018394
//! ```

use std::error::Error;

use futures_util::StreamExt;
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

    let mut stream = client.events().connect(series_id.as_str()).await?;
    println!("connected to series {series_id}");
    let mut transactions = 0_u64;
    while let Some(transaction) = stream.next().await {
        let transaction = transaction?;
        transactions += 1;
        for event in &transaction.events {
            println!(
                "#{} {} {} {} {} -> {} {}",
                transaction.sequence_number,
                transaction.occurred_at,
                event.actor.entity_type,
                event.actor.id,
                event.event_type,
                event.target.entity_type,
                event.target.id,
            );
        }
    }
    println!("stream ended after {transactions} transaction(s)");
    Ok(())
}
