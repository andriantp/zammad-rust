use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};
use zammad::models::{ArticleInput, TicketInput};
use zammad::{Client, Error};

async fn setup() -> (MockServer, Client) {
    let server = MockServer::start().await;
    let client = Client::builder()
        .base_url(server.uri())
        .token("test-token")
        .build()
        .unwrap();
    (server, client)
}

fn tags_mock(id: &str, tags: serde_json::Value) -> Mock {
    Mock::given(method("GET"))
        .and(path("/api/v1/tags"))
        .and(query_param("object", "Ticket"))
        .and(query_param("o_id", id))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "tags": tags })))
}

#[tokio::test]
async fn get_requests_expanded_ticket_and_tags() {
    let (server, client) = setup().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/58"))
        .and(query_param("expand", "true"))
        .and(header("authorization", "Token token=test-token"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": 58,
            "number": "89058",
            "title": "Dashboard image issue",
            "group_id": 1,
            "group": "Users",
            "state": "new",
            "priority": "2 normal",
            "owner": "-",
            "customer": "andi.customer@example.com",
            "organization": null,
            "unknown_future_field": {"ignored": true}
        })))
        .expect(1)
        .mount(&server)
        .await;

    tags_mock("58", json!(["sdk-zammad", "urgent"]))
        .expect(1)
        .mount(&server)
        .await;

    let ticket = client.tickets().get(58).await.unwrap();

    assert_eq!(ticket.id, 58);
    assert_eq!(ticket.number, "89058");
    assert_eq!(ticket.group.as_deref(), Some("Users"));
    assert_eq!(ticket.state.as_deref(), Some("new"));
    assert_eq!(ticket.owner.as_deref(), Some("-"));
    assert_eq!(ticket.organization, None);
    assert_eq!(ticket.tags, ["sdk-zammad", "urgent"]);
}

#[tokio::test]
async fn get_not_found_is_api_error() {
    let (server, client) = setup().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": "Couldn't find Ticket with 'id'=\"999\"",
            "error_human": "Couldn't find Ticket with 'id'=\"999\""
        })))
        .mount(&server)
        .await;

    let err = client.tickets().get(999).await.unwrap_err();

    match &err {
        Error::Api {
            status, message, ..
        } => {
            assert_eq!(status.as_u16(), 404);
            assert_eq!(message, "Couldn't find Ticket with 'id'=\"999\"");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
    assert_eq!(err.status().map(|s| s.as_u16()), Some(404));
}

#[tokio::test]
async fn get_fails_when_tags_fail() {
    let (server, client) = setup().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 1})))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tags"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({"error": "Not authorized"})))
        .mount(&server)
        .await;

    let err = client.tickets().get(1).await.unwrap_err();
    assert_eq!(err.status().map(|s| s.as_u16()), Some(403));
}

#[tokio::test]
async fn invalid_arguments_send_no_request() {
    let (server, client) = setup().await;

    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;

    let tickets = client.tickets();
    assert!(matches!(
        tickets.get(0).await,
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        tickets.delete(-1).await,
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        tickets.search_page("  ", 1, 10).await,
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        tickets.search_page("x", 0, 10).await,
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        tickets.search_all("").await,
        Err(Error::InvalidArgument(_))
    ));
    assert!(matches!(
        tickets.create(&TicketInput::default()).await,
        Err(Error::InvalidArgument(_))
    ));
}

#[tokio::test]
async fn search_page_sends_query_and_paging() {
    let (server, client) = setup().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/search"))
        .and(query_param("query", "state.name:new AND tags:bug"))
        .and(query_param("expand", "true"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": 11}, {"id": 12}])))
        .expect(1)
        .mount(&server)
        .await;

    let tickets = client
        .tickets()
        .search_page("  state.name:new AND tags:bug ", 2, 10)
        .await
        .unwrap();

    let ids: Vec<i64> = tickets.iter().map(|t| t.id).collect();
    assert_eq!(ids, [11, 12]);
}

/// Serves ids 1..=total in pages, like Zammad's search endpoint.
fn paged(total: i64) -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    move |req: &Request| {
        let param = |name: &str| -> i64 {
            req.url
                .query_pairs()
                .find(|(k, _)| k == name)
                .and_then(|(_, v)| v.parse().ok())
                .unwrap()
        };
        let (page, per_page) = (param("page"), param("per_page"));
        let ids: Vec<_> = ((page - 1) * per_page + 1..=(page * per_page).min(total))
            .map(|id| json!({ "id": id }))
            .collect();
        ResponseTemplate::new(200).set_body_json(ids)
    }
}

#[tokio::test]
async fn search_all_follows_every_page() {
    let (server, client) = setup().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/search"))
        .and(query_param("per_page", "100"))
        .respond_with(paged(253))
        .expect(4) // 3 pages with data + 1 empty page
        .mount(&server)
        .await;

    let tickets = client.tickets().search_all("*").await.unwrap();

    assert_eq!(tickets.len(), 253);
    assert!(
        tickets
            .iter()
            .enumerate()
            .all(|(i, t)| t.id == i as i64 + 1)
    );
}

#[tokio::test]
async fn search_all_deduplicates_and_stops_when_page_is_ignored() {
    let (server, client) = setup().await;

    // A server that ignores `page` returns the same tickets every time.
    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id": 1}, {"id": 2}])))
        .expect(2)
        .mount(&server)
        .await;

    let tickets = client.tickets().search_all("*").await.unwrap();
    assert_eq!(tickets.len(), 2);
}

#[tokio::test]
async fn create_posts_input_and_returns_expanded_ticket() {
    let (server, client) = setup().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/tickets"))
        .and(query_param("expand", "true"))
        .and(header("content-type", "application/json"))
        .and(body_partial_json(json!({
            "title": "Cannot log in",
            "group": "Users",
            "customer": "customer@example.com",
            "tags": "login,urgent",
            "article": {
                "subject": "Cannot log in",
                "body": "The login page shows an error.",
                "type": "note",
                "sender": "Customer",
                "internal": false
            }
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({
            "id": 61,
            "number": "89061",
            "title": "Cannot log in",
            "group": "Users",
            "state": "new",
            "article_ids": [82]
        })))
        .expect(1) // tags come from the input, not a second request
        .mount(&server)
        .await;

    let input = TicketInput {
        title: "Cannot log in".into(),
        group: Some("Users".into()),
        customer: Some("customer@example.com".into()),
        tags: vec![" login ".into(), "".into(), "urgent".into()],
        article: ArticleInput {
            subject: "Cannot log in".into(),
            body: "The login page shows an error.".into(),
            ..Default::default()
        },
        ..Default::default()
    };

    let ticket = client.tickets().create(&input).await.unwrap();

    assert_eq!(ticket.id, 61);
    assert_eq!(ticket.state.as_deref(), Some("new"));
    assert_eq!(ticket.article_ids, [82]);
    assert_eq!(ticket.tags, ["login", "urgent"]);
}

#[tokio::test]
async fn create_omits_unset_optional_fields() {
    let (server, client) = setup().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/tickets"))
        .respond_with(|req: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            let obj = body.as_object().unwrap();
            for absent in ["group", "group_id", "customer_id", "state_id", "tags"] {
                assert!(
                    !obj.contains_key(absent),
                    "{absent} should be omitted: {body}"
                );
            }
            ResponseTemplate::new(201).set_body_json(json!({"id": 1}))
        })
        .mount(&server)
        .await;

    let input = TicketInput {
        title: "t".into(),
        article: ArticleInput {
            body: "b".into(),
            ..Default::default()
        },
        ..Default::default()
    };

    let ticket = client.tickets().create(&input).await.unwrap();
    assert!(ticket.tags.is_empty());
}

#[tokio::test]
async fn delete_sends_delete() {
    let (server, client) = setup().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/tickets/61"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    client.tickets().delete(61).await.unwrap();
}

#[tokio::test]
async fn non_json_error_body_is_kept_as_detail() {
    let (server, client) = setup().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/1"))
        .respond_with(ResponseTemplate::new(502).set_body_string("Bad Gateway from proxy"))
        .mount(&server)
        .await;

    match client.tickets().get(1).await.unwrap_err() {
        Error::Api {
            status,
            message,
            detail,
        } => {
            assert_eq!(status.as_u16(), 502);
            assert_eq!(message, "Bad Gateway");
            assert_eq!(detail.as_deref(), Some("Bad Gateway from proxy"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
