use crate::generator::Paths;
use crate::types::Types;
pub use name::Name;
use operation::Operation;
use proc_macro2::TokenStream;
use quote::quote;
use serde_json::Value;
use std::collections::BTreeMap;

mod name;
mod operation;

/// One node of the path tree. A segment becomes a module and a type (`contests::Contests`), `{param}` becomes `id::Id`, and
/// each HTTP method becomes a method on the node.
#[derive(Default)]
pub struct Tree {
    segment: String,
    param: bool,
    operations: Vec<Operation>,
    children: BTreeMap<String, Tree>,
}

impl Tree {
    /// Groups the document's `paths` by segment and registers operation schemas in `types`.
    pub fn new(document: &Value, types: &mut Types) -> Result<Self, String> {
        let mut root = Self::default();
        let paths = document
            .get("paths")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        for (path, item) in &paths {
            let mut node = &mut root;
            let mut hint = String::new();

            for segment in path.split('/').filter(|segment| !segment.is_empty()) {
                let param = segment.starts_with('{');
                let module = if param { "id".into() } else { Name(segment).snake() };

                hint.push_str(&Name(&module).pascal());
                node.admit(path, &module, param)?;

                node = node.children.entry(module).or_insert_with(|| Self {
                    segment: segment.into(),
                    param,
                    ..Self::default()
                });
            }

            for method in ["get", "post", "put", "patch", "delete"] {
                if let Some(operation) = item.get(method) {
                    node.operations
                        .push(Operation::new(method, operation, item, &hint, types)?);
                }
            }
        }

        Ok(root)
    }

    /// Rejects segments whose generated names would clash, before generating code that does not compile.
    fn admit(&self, path: &str, module: &str, param: bool) -> Result<(), String> {
        // `t` would become the node `T`, which is also the transport parameter.
        let reserved = ["get", "post", "put", "patch", "delete", "types", "new", "t"];
        let clash = self.children.get(module).is_some_and(|child| child.param != param);

        match reserved.contains(&module) || clash {
            true => Err(format!(
                "`{path}`: segment `{module}` clashes with a generated method or module name"
            )),
            false => Ok(()),
        }
    }

    /// Generates the root node and the tree below it. Generic roots also get `new(transport)`.
    pub fn stream(&self, paths: &Paths) -> TokenStream {
        let tree = self.node(&paths.root, "", paths);
        let (root, runtime) = (&paths.root, &paths.runtime);

        let constructor = paths.generic().then(|| {
            quote! {
                impl<T: #runtime::Transport> #root<T> {
                    /// Starts the chain at the service root with `transport`.
                    pub fn new(transport: T) -> Self {
                        Self(#runtime::Route::new(transport))
                    }
                }
            }
        });

        quote! {
            #tree
            #constructor
        }
    }

    fn node(&self, class: &syn::Ident, path: &str, paths: &Paths) -> TokenStream {
        let doc = format!("`/{path}`");
        let operations = self.operations.iter().map(|operation| operation.stream(path, paths));
        let (route, params, bounds) = (paths.route(), paths.params(), paths.bounds());

        let children: Vec<_> = self
            .children
            .iter()
            .map(|(module, child)| match path.is_empty() {
                true => (module, child, child.segment.clone()),
                false => (module, child, format!("{path}/{}", child.segment)),
            })
            .collect();

        let accessors = children
            .iter()
            .map(|(module, child, path)| child.accessor(module, path, paths));

        let modules = children.iter().map(|(module, child, path)| {
            let class = Name(module).class();
            let body = child.node(&class, path, paths);
            let module = Name(module).ident();

            quote! {
                // Consecutive parameters such as `/update/{target}/{arch}` nest as `id::id`.
                #[allow(clippy::module_inception)]
                pub mod #module {
                    #[allow(unused_imports)]
                    use super::types;
                    #body
                }
            }
        });

        quote! {
            #[doc = #doc]
            #[derive(Debug, Clone)]
            pub struct #class #params (#route);

            impl #bounds ::std::convert::From<#route> for #class #params {
                fn from(route: #route) -> Self {
                    Self(route)
                }
            }

            impl #bounds #class #params {
                #(#operations)*
                #(#accessors)*
            }

            #(#modules)*
        }
    }

    /// Generates the method on the parent that leads to this node. `path` is this node's path.
    fn accessor(&self, module: &str, path: &str, paths: &Paths) -> TokenStream {
        let name = Name(module).ident();
        let class = Name(module).class();
        let doc = format!("`/{path}`");
        let params = paths.params();

        match self.param {
            true => quote! {
                #[doc = #doc]
                pub fn #name(&self, value: impl ::std::fmt::Display) -> #name::#class #params {
                    self.0.param(value).into()
                }
            },
            false => {
                let segment = &self.segment;
                quote! {
                    #[doc = #doc]
                    pub fn #name(&self) -> #name::#class #params {
                        self.0.join(#segment).into()
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tree;
    use crate::types::Types;
    use serde_json::{Value, json};

    fn build(paths: Value) -> Result<Tree, String> {
        let document = json!({ "openapi": "3.1.0", "paths": paths });
        let mut types = Types::new(&document)?;

        Tree::new(&document, &mut types)
    }

    #[test]
    fn rejects_segments_that_clash_with_generated_names() {
        let item = json!({ "get": { "responses": {} } });
        let reserved = json!({ "/types": item });
        let clash = json!({ "/contests/{contest_id}": item, "/contests/id": item });
        let nested = json!({ "/contests/{contest_id}": item, "/contests/{contest_id}/notices": item });

        assert!(build(reserved).is_err());
        assert!(build(clash).is_err());
        assert!(build(nested).is_ok());
    }
}
