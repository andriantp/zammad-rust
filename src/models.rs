//! Request and response types.

use serde::{Deserialize, Serialize, Serializer};

/// A Zammad ticket.
///
/// Name fields (`group`, `state`, `customer`, ...) are filled because this
/// crate requests tickets with `expand=true`. Zammad never includes tags in
/// ticket responses: [`Tickets::get`](crate::Tickets::get) reads them with a
/// second request and [`Tickets::create`](crate::Tickets::create) copies them
/// from the input, while search results leave `tags` empty.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ticket {
    pub id: i64,
    pub number: String,
    pub title: String,

    pub group_id: Option<i64>,
    pub state_id: Option<i64>,
    pub priority_id: Option<i64>,
    pub customer_id: Option<i64>,
    pub owner_id: Option<i64>,
    pub organization_id: Option<i64>,

    pub group: Option<String>,
    pub state: Option<String>,
    pub priority: Option<String>,
    pub customer: Option<String>,
    /// `"-"` when the ticket is unassigned.
    pub owner: Option<String>,
    pub organization: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,

    pub article_ids: Vec<i64>,
    /// RFC 3339 timestamp, e.g. `2026-11-19T08:53:00.000Z`.
    pub created_at: Option<String>,
    pub updated_at: Option<String>,

    /// Filled by this crate, not by Zammad's ticket response.
    #[serde(skip_deserializing)]
    pub tags: Vec<String>,
}

/// Input for [`Tickets::create`](crate::Tickets::create).
///
/// ```
/// use zammad::models::{ArticleInput, TicketInput};
///
/// let input = TicketInput {
///     title: "Cannot log in".into(),
///     group: Some("Users".into()),
///     customer: Some("customer@example.com".into()),
///     tags: vec!["login".into(), "urgent".into()],
///     article: ArticleInput {
///         subject: "Cannot log in".into(),
///         body: "The login page shows an error.".into(),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct TicketInput {
    pub title: String,

    /// Group name; alternatively set `group_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<i64>,

    /// Customer email or login; alternatively set `customer_id`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<i64>,

    /// Sent to Zammad as one comma-separated string.
    #[serde(
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_tags"
    )]
    pub tags: Vec<String>,

    /// The first article (message) of the ticket.
    pub article: ArticleInput,
}

/// The first article of a new ticket.
#[derive(Debug, Clone, Serialize)]
pub struct ArticleInput {
    pub subject: String,
    pub body: String,
    /// Article type, e.g. `"note"`, `"email"`, `"phone"`. Defaults to `"note"`.
    #[serde(rename = "type")]
    pub kind: String,
    /// `"Customer"`, `"Agent"` or `"System"`. Defaults to `"Customer"`.
    pub sender: String,
    pub internal: bool,
    /// `"text/plain"` or `"text/html"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

impl Default for ArticleInput {
    fn default() -> Self {
        ArticleInput {
            subject: String::new(),
            body: String::new(),
            kind: "note".into(),
            sender: "Customer".into(),
            internal: false,
            content_type: None,
        }
    }
}

/// Trims each tag and drops empty ones.
pub(crate) fn normalize_tags(tags: &[String]) -> Vec<String> {
    tags.iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

fn serialize_tags<S: Serializer>(tags: &[String], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&normalize_tags(tags).join(","))
}
