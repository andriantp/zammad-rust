//! Async client for the [Zammad](https://zammad.org/) helpdesk REST API.
//!
//! ```no_run
//! use zammad::Client;
//!
//! #[tokio::main]
//! async fn main() -> zammad::Result<()> {
//!     let client = Client::builder()
//!         .base_url("https://zammad.example.com")
//!         .token("your-api-token")
//!         .build()?;
//!
//!     let tickets = client.tickets().search_all("state.name:new").await?;
//!     for ticket in &tickets {
//!         println!("{} {}", ticket.number, ticket.title);
//!     }
//!     Ok(())
//! }
//! ```

mod client;
mod error;
pub mod models;
mod tickets;

pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use tickets::{DEFAULT_PER_PAGE, Tickets};

/// HTTP status code, as found in [`Error::Api`]; re-exported so callers need
/// no direct `reqwest` dependency.
pub use reqwest::StatusCode;
