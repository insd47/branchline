/// A request that could not be encoded, or a response body that did not match the contract.
///
/// The transport's error type must convert from this, so generated methods return a single error type.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The query or body could not be encoded.
    #[error("could not encode the request: {0}")]
    Encode(String),

    /// The response body did not match the type in the contract.
    #[error("could not decode the response: {0}")]
    Decode(#[from] serde_json::Error),
}
