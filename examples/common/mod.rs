// Shared helper for the examples (included with `mod common;`).

use zammad::Client;

/// Builds a client from the URL and TOKEN environment variables.
pub fn client() -> Result<Client, Box<dyn std::error::Error>> {
    let url = std::env::var("URL").map_err(|_| "set the URL environment variable")?;
    let token = std::env::var("TOKEN").map_err(|_| "set the TOKEN environment variable")?;

    Ok(Client::builder().base_url(url).token(token).build()?)
}
