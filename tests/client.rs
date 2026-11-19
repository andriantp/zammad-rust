use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use zammad::{Client, Error};

fn config_error(builder: zammad::ClientBuilder) -> String {
    match builder.build() {
        Err(Error::Config(msg)) => msg,
        other => panic!("expected Error::Config, got {other:?}"),
    }
}

#[test]
fn builder_validates_settings() {
    assert!(config_error(Client::builder().token("t")).contains("base URL is required"));
    assert!(config_error(Client::builder().base_url("  ").token("t")).contains("required"));
    assert!(config_error(Client::builder().base_url("not a url").token("t")).contains("invalid"));
    assert!(config_error(Client::builder().base_url("ftp://x.com").token("t")).contains("HTTP"));
    assert!(config_error(Client::builder().base_url("https://x.com")).contains("token"));
    assert!(config_error(Client::builder().base_url("https://x.com").token(" ")).contains("token"));
    assert!(
        Client::builder()
            .base_url("https://x.com/")
            .token("t")
            .build()
            .is_ok()
    );
}

#[tokio::test]
async fn base_url_path_prefix_is_kept() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/zammad/api/v1/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"tags": []})))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .base_url(format!("{}/zammad", server.uri()))
        .token("t")
        .build()
        .unwrap();

    assert!(client.tickets().tags(1).await.unwrap().is_empty());
}

#[tokio::test]
async fn connection_failure_is_http_error() {
    // Port 1 is reserved and normally closed, so the connection is refused.
    let client = Client::builder()
        .base_url("http://127.0.0.1:1")
        .token("t")
        .build()
        .unwrap();

    let err = client.tickets().get(1).await.unwrap_err();
    assert!(matches!(err, Error::Http(_)), "got {err:?}");
    assert!(err.to_string().starts_with("execute HTTP request"));
}

#[tokio::test]
async fn timeout_is_http_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5)))
        .mount(&server)
        .await;

    let client = Client::builder()
        .base_url(server.uri())
        .token("t")
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();

    match client.tickets().get(1).await.unwrap_err() {
        Error::Http(e) => assert!(e.is_timeout(), "expected timeout, got {e}"),
        other => panic!("expected Error::Http, got {other:?}"),
    }
}

#[tokio::test]
async fn unexpected_json_is_decode_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/tickets/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"not": "a list"})))
        .mount(&server)
        .await;

    let client = Client::builder()
        .base_url(server.uri())
        .token("t")
        .build()
        .unwrap();

    let err = client.tickets().search_page("*", 1, 10).await.unwrap_err();
    assert!(matches!(err, Error::Decode(_)), "got {err:?}");
}
