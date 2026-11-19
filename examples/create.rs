//! cargo run --example create -- <group> <customer-email> <title...>
//!
//! Creates a real ticket; remove it with `cargo run --example delete -- <id>`.

mod common;

use zammad::models::{ArticleInput, TicketInput};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err(
            "usage: cargo run --example create -- <group> <customer-email> <title...>".into(),
        );
    }

    let title = args[2..].join(" ");

    let input = TicketInput {
        title: title.clone(),
        group: Some(args[0].clone()),
        customer: Some(args[1].clone()),
        tags: vec!["zammad-rust".into()],
        article: ArticleInput {
            subject: title,
            body: "Created by the zammad-rust create example.".into(),
            ..Default::default()
        },
        ..Default::default()
    };

    let client = common::client()?;
    let ticket = client.tickets().create(&input).await?;

    println!("Ticket created");
    println!("ID:     {}", ticket.id);
    println!("Number: {}", ticket.number);
    println!("Group:  {}", ticket.group.as_deref().unwrap_or("-"));
    println!("State:  {}", ticket.state.as_deref().unwrap_or("-"));
    println!("Tags:   {}", ticket.tags.join(", "));

    Ok(())
}
