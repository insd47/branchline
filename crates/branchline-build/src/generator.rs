use crate::builders::Builders;
use crate::tree::Tree;
use crate::types::Types;
use proc_macro2::TokenStream;
use quote::quote;
use serde_json::Value;
use syn::visit_mut::VisitMut;

/// Generates `pub mod types` and the root node (`Api` unless renamed) from one OpenAPI document.
///
/// - `transport` is the type that implements `branchline::Transport` and `error` is its `Error`. They are written into
///   the generated code as given, so the nodes are not generic: `Api(branchline::Route<crate::Transport>)`.
/// - The root node only implements `From<branchline::Route<transport>>`. Add your own constructor next to it.
/// - Every struct gets a `bon` builder (`types::X::builder().a(1).build()`) unless [`builders`](Self::builders) turns
///   them off. The runtime's `bon` feature must be on while they are.
pub struct Generator {
    paths: Paths,
    builders: bool,
}

/// Paths the generated code refers to.
pub struct Paths {
    /// The runtime crate, `::branchline` unless renamed.
    pub runtime: syn::Path,
    /// The root node's type name.
    pub root: syn::Ident,
    pub transport: syn::Type,
    pub error: syn::Type,
}

impl Paths {
    /// The route every node wraps.
    pub fn route(&self) -> TokenStream {
        let (runtime, transport) = (&self.runtime, &self.transport);
        quote! { #runtime::Route<#transport> }
    }
}

impl Generator {
    /// `transport` and `error` are Rust paths such as `crate::Transport` and `crate::Error`.
    pub fn new(transport: &str, error: &str) -> Result<Self, String> {
        Ok(Self {
            paths: Paths {
                runtime: syn::parse_quote!(::branchline),
                root: syn::parse_quote!(Api),
                transport: parse(transport)?,
                error: parse(error)?,
            },
            builders: true,
        })
    }

    /// Refers to the runtime by another path, for a renamed or re-exported `branchline`.
    pub fn runtime(mut self, path: &str) -> Result<Self, String> {
        self.paths.runtime = parse(path)?;
        Ok(self)
    }

    /// Names the root node, for example after the service (`Contest`).
    pub fn root(mut self, name: &str) -> Result<Self, String> {
        self.paths.root = parse(name)?;
        Ok(self)
    }

    /// Generates a `bon` builder for every struct, so optional fields can be left out and a missing required field
    /// fails to compile (`X::builder().a(1).build()`). On by default; turn it off, together with the runtime's `bon`
    /// feature, to shrink the generated code when callers never build request bodies.
    pub fn builders(mut self, builders: bool) -> Self {
        self.builders = builders;
        self
    }

    /// Generates the types and the path tree of `document`.
    pub fn generate(&self, document: &Value) -> Result<TokenStream, String> {
        let mut types = Types::new(document)?;
        let tree = Tree::new(document, &mut types)?.stream(&self.paths);
        let mut definitions: syn::File = syn::parse2(types.stream()).map_err(|error| error.to_string())?;

        if self.builders {
            Builders {
                runtime: &self.paths.runtime,
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

fn parse<T: syn::parse::Parse>(path: &str) -> Result<T, String> {
    syn::parse_str(path).map_err(|error| format!("`{path}` is not a Rust path: {error}"))
}

#[cfg(test)]
mod tests {
    use super::Generator;
    use serde_json::json;

    #[test]
    fn generates_the_sample_document() {
        let document = json!({
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
        });

        let tokens = Generator::new("crate::Transport", "crate::Error")
            .and_then(|generator| generator.generate(&document))
            .expect("sample generates");
        let code = prettyplease::unparse(&syn::parse2(tokens).expect("generated code parses"));

        for expected in [
            "pub struct Api(::branchline::Route<crate::Transport>);",
            "impl ::std::convert::From<::branchline::Route<crate::Transport>> for Api",
            "pub fn contests(&self) -> contests::Contests",
            "self.0.join(\"contests\").into()",
            "pub fn id(&self, value: impl ::std::fmt::Display) -> id::Id",
            "query: &types::ContestsGetQuery",
            "body: &types::ContestRequest",
            "::std::result::Result<::std::vec::Vec<types::Contest>, crate::Error>",
            "pub async fn delete(&self) -> ::std::result::Result<(), crate::Error>",
            "::branchline::http::Method::GET",
            "pub fn update(&self) -> update::Update",
            "pub hidden: ::std::option::Option<bool>",
        ] {
            assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
        }
    }

    #[test]
    fn names_the_root_node() {
        let document = json!({ "openapi": "3.1.0", "paths": { "/health": { "get": { "responses": {} } } } });
        let tokens = Generator::new("crate::Transport", "crate::Error")
            .and_then(|generator| generator.root("Contest"))
            .and_then(|generator| generator.generate(&document))
            .expect("document generates");

        assert!(tokens.to_string().contains("pub struct Contest"), "{tokens}");
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
        let generate = |builders| {
            Generator::new("crate::Transport", "crate::Error")
                .map(|generator| generator.builders(builders))
                .and_then(|generator| generator.generate(&document))
                .expect("document generates")
                .to_string()
        };

        assert!(generate(true).contains("bon :: Builder"));
        assert!(!generate(false).contains("bon :: Builder"));
        assert!(!generate(true).contains("pub mod builder"), "typify builders stay off");
    }

    #[test]
    fn rejects_paths_that_are_not_rust_paths() {
        assert!(Generator::new("not a path", "crate::Error").is_err());
    }
}
