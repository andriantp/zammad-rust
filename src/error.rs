use reqwest::StatusCode;
use serde::Deserialize;

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The client was built with missing or invalid settings.
    #[error("zammad: invalid configuration: {0}")]
    Config(String),

    /// A method was called with an invalid argument; no request was sent.
    #[error("zammad: invalid argument: {0}")]
    InvalidArgument(String),

    /// Zammad answered with a non-2xx status.
    #[error("zammad API error: status {status}: {message}")]
    Api {
        status: StatusCode,
        message: String,
        /// Extra detail from the response, or the raw body if it was not JSON.
        detail: Option<String>,
    },

    /// The request could not be sent or the response could not be read
    /// (connection refused, timeout, TLS failure, ...).
    #[error("execute HTTP request: {0}")]
    Http(#[from] reqwest::Error),

    /// The response body was not the JSON this crate expected.
    #[error("decode response body: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Shorthand for `Result<T, zammad::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Builds an [`Error::Api`] from a non-2xx response.
    pub(crate) fn from_response(status: StatusCode, body: &[u8]) -> Self {
        #[derive(Deserialize)]
        struct Payload {
            error: Option<String>,
            message: Option<String>,
            detail: Option<String>,
        }

        let fallback = status
            .canonical_reason()
            .unwrap_or("unknown status")
            .to_owned();

        if body.is_empty() {
            return Error::Api {
                status,
                message: fallback,
                detail: None,
            };
        }

        match serde_json::from_slice::<Payload>(body) {
            Ok(p) => Error::Api {
                status,
                message: p.message.or(p.error).unwrap_or(fallback),
                detail: p.detail,
            },
            Err(_) => Error::Api {
                status,
                message: fallback,
                detail: Some(String::from_utf8_lossy(body).into_owned()),
            },
        }
    }

    /// Returns the HTTP status if this is an [`Error::Api`].
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Error::Api { status, .. } => Some(*status),
            _ => None,
        }
    }
}
