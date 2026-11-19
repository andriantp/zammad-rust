//! cargo run --example search -- <query>
//! e.g. cargo run --example search -- 'state.name:new AND tags:bug'

mod common;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if query.trim().is_empty() {
        return Err("usage: cargo run --example search -- <query>".into());
    }

    let client = common::client()?;
    let tickets = client.tickets().search_all(&query).await?;

    println!("Query: {query}");
    println!("Total: {}", tickets.len());

    for ticket in &tickets {
        println!(
            "#{:<8} {:<10} {:<8} {}",
            ticket.number,
            ticket.state.as_deref().unwrap_or("-"),
            ticket.group.as_deref().unwrap_or("-"),
            ticket.title,
        );
    }

    Ok(())
}
