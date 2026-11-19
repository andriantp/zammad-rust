//! cargo run --example delete -- <ticket-id>
//!
//! Deleting tickets requires admin permission in Zammad.

mod common;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id: i64 = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example delete -- <ticket-id>")?
        .parse()?;

    common::client()?.tickets().delete(id).await?;
    println!("Ticket {id} deleted");

    Ok(())
}
