//! Generates a typed resource-chain client from an OpenAPI 3.1 document, for use in `build.rs`.
//!
//! The client follows the path tree instead of `operationId`s: every segment is a node, `{param}` is `id(value)`, and
//! every HTTP method is an async method on its node. Types come from [typify](https://crates.io/crates/typify).
//!
//! ```ignore
//! // build.rs
//! let document: serde_json::Value = serde_json::from_slice(&std::fs::read("openapi.json")?)?;
//! let tokens = branchline_build::Generator::new("crate::Transport", "crate::Error")?.generate(&document)?;
//! std::fs::write(out.join("api.rs"), prettyplease::unparse(&syn::parse2(tokens)?))?;
//!
//! // src/lib.rs
//! include!(concat!(env!("OUT_DIR"), "/api.rs"));
//! let api = Api::from(branchline::Route::new(Transport::new(base)));
//! let notices = api.contests().id(7).notices().get().await?;
//! ```
//!
//! The generated code refers to `branchline` (the runtime), `serde` and `serde_json`, so the crate that includes it
//! depends on all three. Only JSON bodies and query parameters are generated; header and cookie parameters are ignored.

pub use generator::Generator;

mod builders;
mod generator;
mod tree;
mod types;
