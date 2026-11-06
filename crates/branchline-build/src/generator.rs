use crate::Error;
use crate::builders::Builders;
use crate::tree::{Name, Tree};
use crate::types::Types;
use proc_macro2::TokenStream;
use quote::quote;
use serde_json::Value;
use std::path::{Path, PathBuf};
use syn::visit_mut::VisitMut;

/// Generates `pub mod types` and the root node (`Api` unless renamed) from one OpenAPI document.
///
/// - By default the nodes are generic over the transport (`Api<T: branchline::Transport>`) and return `T::Error`, and
///   the root has `Api::new(transport)`. [`transport`](Self::transport) writes a concrete transport into the generated
///   code instead, without `new`, so you can add your own constructor.
/// - Every struct gets a `bon` builder (`types::X::builder().a(1).build()`) unless [`builders`](Self::builders) turns
///   them off. The runtime's `bon` feature must be on while they are.
/// - Settings never fail; they are checked once in [`compile`](Self::compile) or [`generate`](Self::generate).
#[derive(Debug, Clone)]
pub struct Generator {
    runtime: String,
    root: String,
    transport: Option<(String, String)>,
    builders: bool,
}

/// Starts a generator with the default settings.
pub fn configure() -> Generator {
    Generator::default()
}

impl Default for Generator {
    fn default() -> Self {
        Self {
            runtime: "::branchline".into(),
            root: "Api".into(),
            transport: None,
            builders: true,
        }
    }
}

impl Generator {
    /// Names the root node, for example after the service (`Contest`). [`compile`](Self::compile) names the file after
    /// it too (`contest.rs`).
    pub fn root(mut self, name: &str) -> Self {
        self.root = name.into();
        self
    }

    /// Writes a concrete transport and its error type (`crate::Transport`, `crate::Error`) into the generated code, so
    /// the nodes are not generic. The root gets no `new`; build it from `branchline::Route::new(transport).into()`.
    pub fn transport(mut self, transport: &str, error: &str) -> Self {
        self.transport = Some((transport.into(), error.into()));
        self
    }

    /// Refers to the runtime by another path, for a renamed or re-exported `branchline`.
    pub fn runtime(mut self, path: &str) -> Self {
        self.runtime = path.into();
        self
    }

    /// Generates a `bon` builder for every struct, so optional fields can be left out and a missing required field
    /// fails to compile (`X::builder().a(1).build()`). On by default; turn it off, together with the runtime's `bon`
    /// feature, to shrink the generated code when callers never build request bodies.
    pub fn builders(mut self, builders: bool) -> Self {
        self.builders = builders;
        self
    }

    /// Reads `document` (an OpenAPI 3.1 JSON file), generates the client, and writes it to `OUT_DIR` for
    /// `branchline::include!`. Call it from `build.rs`; it also asks Cargo to rerun when the document changes.
    pub fn compile(&self, document: impl AsRef<Path>) -> Result<PathBuf, Error> {
        let document = document.as_ref();
        println!("cargo:rerun-if-changed={}", document.display());

        let bytes = std::fs::read(document).map_err(|source| Error::Io {
            path: document.into(),
            source,
        })?;
        let tokens = self.generate(&serde_json::from_slice(&bytes)?)?;
        let file = syn::parse2(tokens).map_err(|error| Error::Document(error.to_string()))?;

        let out = std::env::var_os("OUT_DIR").ok_or(Error::OutDir)?;
        let target = PathBuf::from(out).join(format!("{}.rs", Name(&self.root).snake()));

        std::fs::write(&target, prettyplease::unparse(&file)).map_err(|source| Error::Io {
            path: target.clone(),
            source,
        })?;

        Ok(target)
    }

    /// Generates the types and the path tree of `document` as tokens, for callers that add their own items.
    pub fn generate(&self, document: &Value) -> Result<TokenStream, Error> {
        let paths = Paths::new(self)?;
        let mut types = Types::new(document).map_err(Error::Document)?;
        let tree = Tree::new(document, &mut types).map_err(Error::Document)?.stream(&paths);
        let mut definitions: syn::File =
            syn::parse2(types.stream()).map_err(|error| Error::Document(error.to_string()))?;

        if self.builders {
            Builders {
                runtime: &paths.runtime,
            }
            .visit_file_mut(&mut definitions);
        }

        Ok(quote! {
            // Callers use only part of a document.
            #[allow(clippy::all, dead_code, unused_imports, non_camel_case_types)]
            pub mod types {
                #definitions
            }

            #tree
        })
    }
}

/// How the generated code refers to the runtime, the transport, and the root.
pub struct Paths {
    pub runtime: syn::Path,
    pub root: syn::Ident,
    /// A concrete transport and its error, or `None` for nodes generic over `T`.
    transport: Option<(syn::Type, syn::Type)>,
}

impl Paths {
    fn new(generator: &Generator) -> Result<Self, Error> {
        let transport = match &generator.transport {
            Some((transport, error)) => Some((parse(transport)?, parse(error)?)),
            None => None,
        };

        Ok(Self {
            runtime: parse(&generator.runtime)?,
            root: parse(&generator.root)?,
            transport,
        })
    }

    /// Whether the nodes are generic over the transport.
    pub fn generic(&self) -> bool {
        self.transport.is_none()
    }

    /// The node's type parameters at its definition: `<T>` or nothing.
    pub fn params(&self) -> TokenStream {
        match self.generic() {
            true => quote! { <T> },
            false => quote! {},
        }
    }

    /// The `impl` header's type parameters: `<T: branchline::Transport>` or nothing.
    pub fn bounds(&self) -> TokenStream {
        let runtime = &self.runtime;
        match self.generic() {
            true => quote! { <T: #runtime::Transport> },
            false => quote! {},
        }
    }

    /// The route every node wraps.
    pub fn route(&self) -> TokenStream {
        let runtime = &self.runtime;
        match &self.transport {
            Some((transport, _)) => quote! { #runtime::Route<#transport> },
            None => quote! { #runtime::Route<T> },
        }
    }

    /// The error every operation returns.
    pub fn error(&self) -> TokenStream {
        match &self.transport {
            Some((_, error)) => quote! { #error },
            None => quote! { T::Error },
        }
    }
}

fn parse<T: syn::parse::Parse>(path: &str) -> Result<T, Error> {
    syn::parse_str(path).map_err(|_| Error::Path(path.into()))
}

#[cfg(test)]
mod tests {
    use super::configure;
    use serde_json::{Value, json};

    fn sample() -> Value {
        json!({
            "openapi": "3.1.0",
            "info": { "title": "sample", "version": "1.2.3" },
            "paths": {
                "/contests": {
                    "get": {
                        "parameters": [{ "in": "query", "name": "hidden", "schema": { "type": "boolean" } }],
                        "responses": { "200": { "description": "", "content": { "application/json": {
                            "schema": { "type": "array", "items": { "$ref": "#/components/schemas/Contest" } }
                        } } } }
                    },
                    "post": {
                        "requestBody": { "content": { "application/json": {
                            "schema": { "$ref": "#/components/schemas/ContestRequest" }
                        } } },
                        "responses": { "201": { "description": "", "content": { "application/json": {
                            "schema": { "$ref": "#/components/schemas/Contest" }
                        } } } }
                    }
                },
                "/contests/{contest_id}": {
                    "delete": { "responses": { "204": { "description": "" } } }
                },
                "/update/{target}/{arch}": {
                    "get": {
                        "responses": { "200": { "description": "", "content": { "application/json": {
                            "schema": { "type": "object", "properties": { "version": { "type": "string" } }, "required": ["version"] }
                        } } } }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Contest": {
                        "type": "object",
                        "properties": { "id": { "type": "string" }, "name": { "type": "string" }, "note": { "type": ["string", "null"] } },
                        "required": ["id", "name"]
                    },
                    "ContestRequest": {
                        "type": "object",
                        "properties": { "name": { "type": "string" } },
                        "required": ["name"]
                    }
                }
            }
        })
    }

    fn render(tokens: proc_macro2::TokenStream) -> String {
        prettyplease::unparse(&syn::parse2(tokens).expect("generated code parses"))
    }

    #[test]
    fn generates_generic_nodes_by_default() {
        let code = render(configure().generate(&sample()).expect("sample generates"));

        for expected in [
            "pub struct Api<T>(::branchline::Route<T>);",
            "impl<T: ::branchline::Transport> Api<T>",
            "pub fn new(transport: T) -> Self",
            "pub fn contests(&self) -> contests::Contests<T>",
            "pub fn id(&self, value: impl ::std::fmt::Display) -> id::Id<T>",
            "::std::result::Result<::std::vec::Vec<types::Contest>, T::Error>",
            "query: &types::ContestsGetQuery",
            "body: &types::ContestRequest",
            "::branchline::http::Method::GET",
            "pub hidden: ::std::option::Option<bool>",
        ] {
            assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
        }
    }

    #[test]
    fn writes_a_concrete_transport_without_new() {
        let generator = configure()
            .root("Contest")
            .transport("crate::Transport", "crate::Error");
        let code = render(generator.generate(&sample()).expect("sample generates"));

        for expected in [
            "pub struct Contest(::branchline::Route<crate::Transport>);",
            "impl ::std::convert::From<::branchline::Route<crate::Transport>> for Contest",
            "pub fn contests(&self) -> contests::Contests",
            "pub async fn delete(&self) -> ::std::result::Result<(), crate::Error>",
        ] {
            assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
        }
        assert!(!code.contains("pub fn new(transport"), "{code}");
    }

    #[test]
    fn generates_builders_unless_turned_off() {
        let document = json!({
            "openapi": "3.1.0",
            "paths": {},
            "components": { "schemas": { "Notice": {
                "type": "object",
                "properties": { "title": { "type": "string" }, "pinned": { "type": "boolean" } },
                "required": ["title"]
            } } }
        });
        let generate = |builders| configure().builders(builders).generate(&document).unwrap().to_string();

        assert!(generate(true).contains("bon :: Builder"));
        assert!(!generate(false).contains("bon :: Builder"));
        assert!(!generate(true).contains("pub mod builder"), "typify builders stay off");
    }

    #[test]
    fn rejects_settings_that_are_not_rust_paths() {
        assert!(
            configure()
                .transport("not a path", "crate::Error")
                .generate(&sample())
                .is_err()
        );
        assert!(configure().root("1Api").generate(&sample()).is_err());
    }

    #[test]
    fn compile_needs_a_build_script() {
        let path = std::env::temp_dir().join("branchline-sample.json");
        std::fs::write(&path, sample().to_string()).unwrap();

        // `OUT_DIR` exists only while a build script runs.
        let error = configure().compile(&path).unwrap_err();
        assert!(matches!(error, crate::Error::OutDir), "{error}");
    }
}
