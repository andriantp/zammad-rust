//! cargo run --example get -- <ticket-id>

mod common;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id: i64 = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example get -- <ticket-id>")?
        .parse()?;

    let client = common::client()?;
    let ticket = client.tickets().get(id).await?;

    println!("ID:           {}", ticket.id);
    println!("Number:       {}", ticket.number);
    println!("Title:        {}", ticket.title);
    println!("Group:        {}", ticket.group.as_deref().unwrap_or("-"));
    println!("State:        {}", ticket.state.as_deref().unwrap_or("-"));
    println!(
        "Priority:     {}",
        ticket.priority.as_deref().unwrap_or("-")
    );
    println!(
        "Customer:     {}",
        ticket.customer.as_deref().unwrap_or("-")
    );
    println!(
        "Organization: {}",
        ticket.organization.as_deref().unwrap_or("-")
    );
    println!("Tags:         {}", ticket.tags.join(", "));

    Ok(())
}
