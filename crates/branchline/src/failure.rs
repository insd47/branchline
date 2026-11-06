use crate::Error;
use http::StatusCode;

/// The error of the built-in transports ([`Http`](crate::Http) and [`SigV4`](crate::SigV4)).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HttpError {
    /// The request could not be encoded or the response did not match the contract.
    #[error(transparent)]
    Request(#[from] Error),

    /// The request never reached the server, or the response was cut off.
    #[error("could not send the request: {0}")]
    Send(#[from] reqwest::Error),

    /// The base URL or the path does not make a valid URL.
    #[error("invalid URL: {0}")]
    Url(String),

    /// No credentials or no region to sign with (`SigV4`).
    #[error("no AWS credentials: {0}")]
    Credentials(String),

    /// The request could not be signed (`SigV4`).
    #[error("could not sign the request: {0}")]
    Sign(String),

    /// A non-2xx response. `body` is the response body as text.
    #[error("{status} {body}")]
    Status { status: StatusCode, body: String },
}
