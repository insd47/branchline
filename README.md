# branchline

A typed Rust client generator for OpenAPI 3.1 that follows the path tree instead of `operationId`s.

```rust
let notices = api.contests().id(7).notices().get().await?;   // GET  /contests/{contest_id}/notices
api.contests().id(7).notices().post(&body).await?;            // POST /contests/{contest_id}/notices
```

- **Path-first.** Every path segment is a node, `{param}` is `id(value)`, and every HTTP method is an async method on
  its node. The document does not need `operationId`s.
- **OpenAPI 3.1 as is.** Types come from [typify](https://crates.io/crates/typify), including `type: [T, "null"]`,
  `oneOf`, `anyOf`, `allOf`, and `const`.
- **Your transport.** The generated client calls a `Transport`, so authentication and signing, base URLs, retries,
  and error mapping stay in your code. Ready-made ones are behind features: `Http` (reqwest) and `SigV4` (reqwest plus
  AWS SigV4, for IAM-authenticated endpoints such as Lambda Function URLs).
- **Generated in `build.rs`.** Nothing generated is committed; the client tracks the document on every build.

## Usage

```toml
[dependencies]
branchline = { version = "0.1", features = ["sigv4"] }   # or "reqwest", or neither with your own Transport
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[build-dependencies]
branchline-build = "0.1"
prettyplease = "0.3"
serde_json = "1"
syn = "3"
```

```rust
// build.rs
fn main() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let document: serde_json::Value = serde_json::from_slice(&std::fs::read("openapi.json").unwrap()).unwrap();

    let tokens = branchline_build::Generator::new("crate::Transport", "crate::Error")
        .and_then(|generator| generator.root("Api"))
        .and_then(|generator| generator.generate(&document))
        .unwrap();

    let file = syn::parse2(tokens).unwrap();
    std::fs::write(out.join("api.rs"), prettyplease::unparse(&file)).unwrap();
}
```

```rust
// src/lib.rs
include!(concat!(env!("OUT_DIR"), "/api.rs"));

#[derive(Debug, Clone)]
pub struct Transport { /* base URL, HTTP client, credentials */ }

impl branchline::Transport for Transport {
    type Error = Error; // must implement From<branchline::Error>

    async fn send(
        &self,
        method: branchline::http::Method,
        path: &str,
        query: Option<&str>,
        body: Vec<u8>,
    ) -> Result<bytes::Bytes, Error> {
        // Join `path` onto the base URL, sign, send, and map non-2xx responses to `Error`.
    }
}

let api = Api::from(branchline::Route::new(Transport::new(base)));
```

### Request bodies and queries

Bodies and queries are the types typify generates from the document. Fill them as struct literals, or with the
[bon](https://crates.io/crates/bon) builder every struct gets. Optional fields can be left out, and a missing required
field does not compile:

```rust
let body = types::NoticeRequest::builder().title("Notice").build();
api.contests().id(7).notices().post(&body).await?;
```

`String` setters take `impl Into<String>`. Fields typify marks `#[serde(default)]` (optional arrays, schema defaults)
are optional in the builder too. To skip builders, call `Generator::builders(false)` and turn off the runtime's
default `bon` feature together.

### Built-in transports

```rust
let api = Api::from(branchline::Route::new(branchline::Http::new("https://api.example.com/v1")?));

let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
let api = Api::from(branchline::Route::new(branchline::SigV4::new(function_url, &config, "lambda")?));
```

Both send JSON and return `branchline::HttpError`, with `HttpError::Status` for non-2xx responses. Wrap one in your own
transport to add checks or map errors.

### Features

| Feature | Default | Adds |
| --- | --- | --- |
| `bon` | yes | builders for generated types |
| `reqwest` | no | `Http` |
| `sigv4` | no | `SigV4`, includes `reqwest` |

Use the same version of `branchline` and `branchline-build`; they are released together.

## Limits

- Only JSON request and response bodies and query parameters are generated. Header and cookie parameters are ignored.
- The success type is taken from the lowest 2xx response. Error responses are left to the transport.
- Consecutive path parameters nest as `id(..).id(..)`.

## License

MIT
