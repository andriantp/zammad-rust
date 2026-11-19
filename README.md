# zammad-rust

Async Rust client for the [Zammad](https://zammad.org/) helpdesk REST API.

A Rust counterpart of [zammad-sdk](https://github.com/andriantp/zammad-sdk)
(Go), currently covering tickets: get, search, create and delete.

## Installation

```toml
[dependencies]
zammad = { git = "https://github.com/andriantp/zammad-rust" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Usage

```rust
use zammad::Client;

#[tokio::main]
async fn main() -> zammad::Result<()> {
    let client = Client::builder()
        .base_url("https://zammad.example.com")
        .token("your-api-token")
        .build()?;

    let ticket = client.tickets().get(123).await?;
    println!("{} {} [{}]", ticket.number, ticket.title, ticket.tags.join(", "));

    Ok(())
}
```

### Builder options

| Method | Description |
|---|---|
| `base_url(url)` | Zammad instance URL (required). A path prefix is kept. |
| `token(token)` | API access token (required). |
| `timeout(duration)` | Timeout per request, body included. Default 30s. |
| `http_client(client)` | Use a preconfigured `reqwest::Client`. |

### Get a ticket

```rust
let ticket = client.tickets().get(123).await?;
```

Name fields (`group`, `state`, `customer`, ...) are filled because tickets are
requested with `expand=true`. Zammad does not include tags in ticket
responses, so `get` reads them with a second request.

### Search tickets

Queries use Zammad's search syntax, combined with `AND`, `OR` and `NOT`.

```rust
// Every match, following all pages.
let tickets = client.tickets().search_all("state.name:new AND tags:bug").await?;

// A single page (pages start at 1).
let page = client.tickets().search_page("state.name:new", 1, 50).await?;
```

Zammad returns only 50 results per search by default; `search_all` keeps
requesting pages of 100 until an empty one and removes duplicates. Search
results do not include tags; filter with `tags:...` in the query or call
`tickets().tags(id)`.

Search goes through Zammad's search index, which is updated in the
background: a ticket you just created or changed may take a few seconds to
show up in search results (about 6 seconds on a test instance). Use `get`
when you already know the ID.

### Create a ticket

```rust
use zammad::models::{ArticleInput, TicketInput};

let ticket = client
    .tickets()
    .create(&TicketInput {
        title: "Cannot log in".into(),
        group: Some("Users".into()),
        customer: Some("customer@example.com".into()),
        tags: vec!["login".into(), "urgent".into()],
        article: ArticleInput {
            subject: "Cannot log in".into(),
            body: "The login page shows an error.".into(),
            ..Default::default() // type "note", sender "Customer"
        },
        ..Default::default()
    })
    .await?;
```

The returned ticket has its name fields filled; `tags` is copied from the
input rather than read with a second request, so a failure after creation
cannot invite a retry that duplicates the ticket.

### Delete a ticket

```rust
client.tickets().delete(123).await?; // requires admin permission
```

## Errors

Every method returns `zammad::Result<T>`:

| Variant | When |
|---|---|
| `Error::Api { status, message, detail }` | Zammad answered with a non-2xx status |
| `Error::Http(reqwest::Error)` | Connection failure, timeout, TLS error |
| `Error::Decode(serde_json::Error)` | Unexpected response JSON |
| `Error::InvalidArgument(String)` | Invalid input; no request was sent |
| `Error::Config(String)` | Invalid builder settings |

```rust
match client.tickets().get(999).await {
    Err(e) if e.status() == Some(zammad::StatusCode::NOT_FOUND) => println!("no such ticket"),
    Err(e) => return Err(e),
    Ok(ticket) => println!("{}", ticket.title),
}
```

## Examples

```bash
export URL="https://zammad.example.com"
export TOKEN="your-api-token"

cargo run --example get -- 123
cargo run --example search -- 'state.name:new AND tags:bug'
cargo run --example create -- Users customer@example.com Cannot log in
cargo run --example delete -- 123
```

`create` makes a real ticket tagged `zammad-rust`; remove it with `delete`.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Tests run against a local mock server ([wiremock](https://crates.io/crates/wiremock)),
so they need no Zammad instance.

## License

MIT
