use crate::Error;
use bytes::Bytes;
use http::Method;

/// Sends the requests of a generated client.
///
/// - `path` is relative to the service root and has no leading `/` (`contests/7/notices`). Join it onto your base URL.
/// - `query` is already form-encoded and `body` is already JSON (empty when the operation has no body), so you can sign
///   the exact bytes that go on the wire.
/// - Return the body of a successful response. Map everything else, including non-2xx statuses, to `Self::Error`.
pub trait Transport: Clone + Send + Sync {
    /// The error every generated method returns.
    type Error: From<Error>;

    /// Sends one request and returns the body of a successful response.
    fn send(
        &self,
        method: Method,
        path: &str,
        query: Option<&str>,
        body: Vec<u8>,
    ) -> impl Future<Output = Result<Bytes, Self::Error>> + Send;
}
