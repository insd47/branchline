use crate::{HttpError, Transport, request};
use bytes::Bytes;
use http::Method;
use url::Url;

/// A transport that sends JSON requests with reqwest to a base URL.
///
/// Non-2xx responses become [`HttpError::Status`]. Wrap it in your own transport to map errors or add checks.
#[derive(Debug, Clone)]
pub struct Http {
    client: reqwest::Client,
    base: Url,
}

impl Http {
    /// Sends to `base` (`https://api.example.com/v1`) with a default reqwest client.
    pub fn new(base: &str) -> Result<Self, HttpError> {
        Ok(Self {
            client: reqwest::Client::new(),
            base: request::base(base)?,
        })
    }

    /// Uses your reqwest client, for timeouts, proxies, or default headers.
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }
}

impl Transport for Http {
    type Error = HttpError;

    async fn send(&self, method: Method, path: &str, query: Option<&str>, body: Vec<u8>) -> Result<Bytes, HttpError> {
        let url = request::url(&self.base, path, query)?;
        request::send(&self.client, request::build(method, &url, body)?).await
    }
}
