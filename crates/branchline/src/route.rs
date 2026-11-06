use crate::path::Path;
use crate::{Error, Transport};
use http::Method;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Display;

/// One path in a service. Generated nodes wrap it; you rarely build one by hand.
///
/// - The root route is made from a transport (`Route::new(transport)`), and each node adds a segment or a parameter.
/// - Cloning is as cheap as cloning the transport and the path.
#[derive(Debug, Clone)]
pub struct Route<T> {
    transport: T,
    path: Path,
}

impl<T: Transport> Route<T> {
    /// The root of a service.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            path: Path::default(),
        }
    }

    /// Appends a fixed segment (`contests`).
    pub fn join(&self, segment: &str) -> Self {
        Self {
            transport: self.transport.clone(),
            path: self.path.join(segment),
        }
    }

    /// Appends a path parameter, percent-encoding everything but RFC 3986 unreserved characters.
    pub fn param(&self, value: impl Display) -> Self {
        Self {
            transport: self.transport.clone(),
            path: self.path.param(value),
        }
    }

    /// Sends a request to this path. An empty response body is read as `null`, so `()` accepts it.
    pub async fn send<Q, B, R>(&self, method: Method, query: Option<&Q>, body: Option<&B>) -> Result<R, T::Error>
    where
        Q: Serialize + ?Sized,
        B: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let query = query
            .map(serde_html_form::to_string)
            .transpose()
            .map_err(|error| Error::Encode(error.to_string()))?;

        let body = body
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| Error::Encode(error.to_string()))?
            .unwrap_or_default();

        let bytes = self
            .transport
            .send(method, self.path.as_str(), query.as_deref(), body)
            .await?;

        let bytes: &[u8] = if bytes.is_empty() { b"null" } else { &bytes };
        Ok(serde_json::from_slice(bytes).map_err(Error::Decode)?)
    }
}
