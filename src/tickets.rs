use std::collections::HashSet;

use serde::Deserialize;

use crate::client::Client;
use crate::error::{Error, Result};
use crate::models::{Ticket, TicketInput, normalize_tags};

/// Page size used by [`Tickets::search_all`].
pub const DEFAULT_PER_PAGE: u32 = 100;

/// Ticket operations. Obtain with [`Client::tickets`].
#[derive(Debug, Clone, Copy)]
pub struct Tickets<'a> {
    client: &'a Client,
}

impl<'a> Tickets<'a> {
    pub(crate) fn new(client: &'a Client) -> Self {
        Tickets { client }
    }

    /// Gets a ticket by ID, including name fields and tags.
    ///
    /// Uses two requests: the ticket itself and its tags, which Zammad does
    /// not include in ticket responses. If reading the tags fails the whole
    /// call fails rather than returning a ticket with missing tags.
    pub async fn get(&self, id: i64) -> Result<Ticket> {
        check_id(id)?;

        let mut ticket: Ticket = self
            .client
            .get(&format!("tickets/{id}"), &[("expand", "true")])
            .await?;

        ticket.tags = self.tags(ticket.id).await?;

        Ok(ticket)
    }

    /// Gets the tags of a ticket.
    pub async fn tags(&self, id: i64) -> Result<Vec<String>> {
        check_id(id)?;

        #[derive(Deserialize)]
        struct Response {
            #[serde(default)]
            tags: Vec<String>,
        }

        let id = id.to_string();
        let response: Response = self
            .client
            .get("tags", &[("object", "Ticket"), ("o_id", &id)])
            .await?;

        Ok(response.tags)
    }

    /// Gets one page of tickets matching `query` (Zammad search syntax, e.g.
    /// `state.name:new AND tags:bug`). Pages start at 1.
    ///
    /// Search results do not include tags; use [`tags`](Self::tags) per
    /// ticket if needed, or filter with `tags:...` in the query.
    pub async fn search_page(&self, query: &str, page: u32, per_page: u32) -> Result<Vec<Ticket>> {
        let query = check_query(query)?;

        if page == 0 || per_page == 0 {
            return Err(Error::InvalidArgument(
                "page and per_page start at 1".into(),
            ));
        }

        let (page, per_page) = (page.to_string(), per_page.to_string());

        self.client
            .get(
                "tickets/search",
                &[
                    ("query", query),
                    ("expand", "true"),
                    ("page", &page),
                    ("per_page", &per_page),
                ],
            )
            .await
    }

    /// Gets every ticket matching `query` by requesting pages of
    /// [`DEFAULT_PER_PAGE`] until one adds no new tickets.
    ///
    /// Zammad's search returns only 50 results unless asked for more pages,
    /// so this is the way to get complete results. It stops on an empty page
    /// rather than a short one so a server-side page size cap cannot truncate
    /// the result; the cost is one extra request. Tickets that shift between
    /// pages while paging are deduplicated by ID.
    pub async fn search_all(&self, query: &str) -> Result<Vec<Ticket>> {
        check_query(query)?;

        let mut seen = HashSet::new();
        let mut all = Vec::new();
        let mut page = 1;

        loop {
            let tickets = self.search_page(query, page, DEFAULT_PER_PAGE).await?;
            let before = all.len();

            for ticket in tickets {
                if seen.insert(ticket.id) {
                    all.push(ticket);
                }
            }

            // An empty page, or one with only already-seen tickets (a server
            // that ignores `page`), ends the loop.
            if all.len() == before {
                return Ok(all);
            }

            page += 1;
        }
    }

    /// Creates a ticket with its first article.
    ///
    /// The response is requested with `expand=true` so name fields are
    /// filled. `tags` is copied from the (normalized) input instead of read
    /// with a second request: a failure after the ticket was created would
    /// otherwise invite a retry that creates a duplicate.
    pub async fn create(&self, input: &TicketInput) -> Result<Ticket> {
        if input.title.trim().is_empty() {
            return Err(Error::InvalidArgument("title is required".into()));
        }

        if input.article.body.trim().is_empty() {
            return Err(Error::InvalidArgument("article body is required".into()));
        }

        let mut ticket: Ticket = self
            .client
            .post("tickets", &[("expand", "true")], input)
            .await?;

        if ticket.tags.is_empty() {
            ticket.tags = normalize_tags(&input.tags);
        }

        Ok(ticket)
    }

    /// Deletes a ticket. Requires admin permission in Zammad.
    pub async fn delete(&self, id: i64) -> Result<()> {
        check_id(id)?;
        self.client.delete(&format!("tickets/{id}")).await
    }
}

fn check_id(id: i64) -> Result<()> {
    if id <= 0 {
        return Err(Error::InvalidArgument(format!("invalid ticket ID: {id}")));
    }
    Ok(())
}

fn check_query(query: &str) -> Result<&str> {
    let query = query.trim();
    if query.is_empty() {
        return Err(Error::InvalidArgument("search query is required".into()));
    }
    Ok(query)
}
