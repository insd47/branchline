//! Generates a typed resource-chain client from an OpenAPI 3.1 document, for use in `build.rs`.
//!
//! The client follows the path tree instead of `operationId`s: every segment is a node, `{param}` is `id(value)`, and
//! every HTTP method is an async method on its node. Types come from [typify](https://crates.io/crates/typify).
//!
//! ```ignore
//! // build.rs
//! branchline_build::configure().compile("openapi.json")?;
//!
//! // src/lib.rs
//! branchline::include!();
//! let api = Api::new(branchline::Http::new("https://api.example.com")?);
//! let notices = api.contests().id(7).notices().get().await?;
//! ```
//!
//! The generated code refers to `branchline` (the runtime), `serde` and `serde_json`, so the crate that includes it
//! depends on all three. Only JSON bodies and query parameters are generated; header and cookie parameters are ignored.

pub use error::Error;
pub use generator::{Generator, configure};

mod builders;
mod error;
mod generator;
mod tree;
mod types;
