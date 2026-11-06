/// Why a document could not be turned into a client.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The document file could not be read, or the generated file could not be written.
    #[error("{path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The document is not JSON.
    #[error("the document is not JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// A schema or a path in the document cannot be generated.
    #[error("{0}")]
    Document(String),

    /// A setting is not a Rust path or identifier.
    #[error("`{0}` is not a Rust path")]
    Path(String),

    /// `OUT_DIR` is missing, so `compile` is not running in a build script.
    #[error("OUT_DIR is not set; call compile from build.rs")]
    OutDir,
}
