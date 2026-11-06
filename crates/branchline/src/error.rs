/// Why a generated method failed.
///
/// - Errors of the crates underneath (serde, reqwest, the AWS SDK) are kept as they are, with their sources.
/// - Transport variants exist only with the feature that adds the transport.
/// - Your own transport's error must convert from this, so generated methods return a single error type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The query could not be form-encoded.
    #[error("could not encode the query: {0}")]
    Query(#[from] serde_html_form::ser::Error),

    /// The body could not be encoded as JSON.
    #[error("could not encode the body: {0}")]
    Body(#[source] serde_json::Error),

    /// The response body did not match the type in the contract.
    #[error("could not decode the response: {0}")]
    Decode(#[source] serde_json::Error),

    /// The base URL or the path does not make a valid URL.
    #[cfg(feature = "reqwest")]
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// The request could not be built from the URL and headers.
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Request(#[from] http::Error),

    /// The request never reached the server, or the response was cut off.
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Send(#[from] reqwest::Error),

    /// A non-2xx response. `body` is the response body as text.
    #[cfg(feature = "reqwest")]
    #[error("{status} {body}")]
    Status { status: http::StatusCode, body: String },

    /// No credentials or region to sign with, or the credentials provider failed (`SigV4`).
    #[cfg(feature = "sigv4")]
    #[error(transparent)]
    Credentials(#[from] aws_credential_types::provider::error::CredentialsError),

    /// The request could not be signed (`SigV4`).
    #[cfg(feature = "sigv4")]
    #[error(transparent)]
    Sign(#[from] aws_sigv4::http_request::SigningError),
}
