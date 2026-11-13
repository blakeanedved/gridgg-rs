//! Lists the post-series files available for a series.
//!
//! Open Access keys are refused by the File Download API (`403 request is
//! forbidden`); this example prints that error instead of failing.
//!
//! ```sh
//! GRID_API_KEY=... cargo run --example files -- 3018394
//! ```

use std::error::Error;

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

    match client.files().list(series_id.as_str()).await {
        Ok(files) if files.is_empty() => println!("no files for series {series_id}"),
        Ok(files) => {
            for file in files {
                println!(
                    "{:<32} {:<20} {}",
                    file.id,
                    file.status,
                    file.file_name.as_deref().unwrap_or("-"),
                );
            }
        }
        Err(err @ (gridgg::Error::Api { .. } | gridgg::Error::Http { .. })) => {
            eprintln!("the File Download API answered with an error: {err}");
            eprintln!("(Open Access keys get 403: they have no file access)");
        }
        Err(err) => return Err(err.into()),
    }
    Ok(())
}
