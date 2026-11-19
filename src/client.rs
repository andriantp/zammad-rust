use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION};
use reqwest::{Method, RequestBuilder, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::tickets::Tickets;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Client for the Zammad REST API (`/api/v1`).
///
/// Cheap to clone: clones share the underlying connection pool.
///
/// ```no_run
/// # async fn run() -> zammad::Result<()> {
/// let client = zammad::Client::builder()
///     .base_url("https://zammad.example.com")
///     .token("your-api-token")
///     .build()?;
///
/// let ticket = client.tickets().get(123).await?;
/// println!("{} {}", ticket.number, ticket.title);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    api_url: Url,
    token: String,
}

/// Builder for [`Client`]. Create one with [`Client::builder`].
#[derive(Debug, Default)]
pub struct ClientBuilder {
    base_url: Option<String>,
    token: Option<String>,
    timeout: Option<Duration>,
    http: Option<reqwest::Client>,
}

impl Client {
    /// Starts building a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Ticket operations.
    pub fn tickets(&self) -> Tickets<'_> {
        Tickets::new(self)
    }

    fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self
            .api_url
            .join(path)
            .map_err(|e| Error::InvalidArgument(format!("invalid path {path:?}: {e}")))?;

        Ok(self
            .http
            .request(method, url)
            .header(AUTHORIZATION, format!("Token token={}", self.token))
            .header(ACCEPT, "application/json"))
    }

    /// Sends the request and returns the body of a 2xx response.
    async fn execute(&self, request: RequestBuilder) -> Result<Vec<u8>> {
        let response = request.send().await?;
        let status = response.status();
        let body = response.bytes().await?;

        if !status.is_success() {
            return Err(Error::from_response(status, &body));
        }

        Ok(body.to_vec())
    }

    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let body = self
            .execute(self.request(Method::GET, path)?.query(query))
            .await?;
        Ok(serde_json::from_slice(&body)?)
    }

    pub(crate) async fn post<B, T>(&self, path: &str, query: &[(&str, &str)], body: &B) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let request = self.request(Method::POST, path)?.query(query).json(body);
        let body = self.execute(request).await?;
        Ok(serde_json::from_slice(&body)?)
    }

    pub(crate) async fn delete(&self, path: &str) -> Result<()> {
        self.execute(self.request(Method::DELETE, path)?).await?;
        Ok(())
    }
}

impl ClientBuilder {
    /// Zammad instance URL, e.g. `https://zammad.example.com`. Required.
    /// A path prefix (`https://example.com/zammad`) is kept.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// API access token. Required.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Timeout for each whole request, response body included. Defaults to
    /// 30 seconds. Ignored when [`http_client`](Self::http_client) is set.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Uses a preconfigured `reqwest::Client` (proxy, TLS, timeouts, ...).
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Validates the settings and builds the client.
    pub fn build(self) -> Result<Client> {
        let raw = self
            .base_url
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Config("base URL is required".into()))?;

        let mut url =
            Url::parse(&raw).map_err(|e| Error::Config(format!("invalid base URL: {e}")))?;

        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(Error::Config("base URL must use HTTP or HTTPS".into()));
        }

        if url.host_str().is_none() {
            return Err(Error::Config("base URL has no host".into()));
        }

        // A trailing slash makes Url::join append to the path instead of
        // replacing its last segment.
        if !url.path().ends_with('/') {
            let path = format!("{}/", url.path());
            url.set_path(&path);
        }

        let api_url = url
            .join("api/v1/")
            .map_err(|e| Error::Config(format!("invalid base URL: {e}")))?;

        let token = self
            .token
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| Error::Config("API token is required".into()))?;

        let http = match self.http {
            Some(client) => client,
            None => reqwest::Client::builder()
                .timeout(self.timeout.unwrap_or(DEFAULT_TIMEOUT))
                .build()?,
        };

        Ok(Client {
            http,
            api_url,
            token,
        })
    }
}
