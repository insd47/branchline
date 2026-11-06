use super::name::Name;
use crate::generator::Paths;
use crate::types::Types;
use proc_macro2::TokenStream;
use quote::quote;
use serde_json::{Map, Value, json};

/// One HTTP method on one path. It becomes the node's `get(..)`, `post(..)`, ... method.
pub struct Operation {
    method: String,
    doc: Option<String>,
    query: Option<TokenStream>,
    body: Option<TokenStream>,
    response: TokenStream,
}

impl Operation {
    /// `hint` prefixes the names of inline schemas (`ContestsIdNotices`).
    pub fn new(method: &str, operation: &Value, item: &Value, hint: &str, types: &mut Types) -> Result<Self, String> {
        let hint = format!("{hint}{}", Name(method).pascal());
        let query = Self::parameters(operation, item);

        let query = match query.is_empty() {
            true => None,
            false => Some(types.path(&Self::schema(&query), &format!("{hint}Query"))?),
        };

        let body = match operation.pointer("/requestBody/content/application~1json/schema") {
            Some(schema) => Some(types.path(schema, &format!("{hint}Body"))?),
            None => None,
        };

        let response = match Self::success(operation) {
            Some(schema) => types.path(schema, &format!("{hint}Response"))?,
            None => quote! { () },
        };

        let doc = operation
            .get("summary")
            .or_else(|| operation.get("description"))
            .and_then(Value::as_str)
            .map(String::from);

        Ok(Self {
            method: method.into(),
            doc,
            query,
            body,
            response,
        })
    }

    /// Generates the node's method. Without a summary in the document, the doc comment is `GET /contests`.
    pub fn stream(&self, path: &str, paths: &Paths) -> TokenStream {
        let name = Name(&self.method).ident();
        let verb = syn::Ident::new(&self.method.to_ascii_uppercase(), proc_macro2::Span::call_site());
        let doc = self.doc.clone().unwrap_or_else(|| format!("`{verb} /{path}`"));
        let response = &self.response;
        let (runtime, error) = (&paths.runtime, &paths.error);

        let (query_arg, query_value) = match &self.query {
            Some(path) => (quote! { query: &#path, }, quote! { Some(query) }),
            None => (quote! {}, quote! { None::<&()> }),
        };

        let (body_arg, body_value) = match &self.body {
            Some(path) => (quote! { body: &#path, }, quote! { Some(body) }),
            None => (quote! {}, quote! { None::<&()> }),
        };

        quote! {
            #[doc = #doc]
            pub async fn #name(&self, #query_arg #body_arg) -> ::std::result::Result<#response, #error> {
                self.0
                    .send(#runtime::http::Method::#verb, #query_value, #body_value)
                    .await
            }
        }
    }

    /// Merges path-level and operation-level query parameters; the operation wins on the same name.
    fn parameters(operation: &Value, item: &Value) -> Vec<Value> {
        let list = |value: &Value| {
            value
                .get("parameters")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        };

        let mut merged = list(item);

        for parameter in list(operation) {
            merged.retain(|existing| existing["name"] != parameter["name"] || existing["in"] != parameter["in"]);
            merged.push(parameter);
        }

        merged.retain(|parameter| parameter["in"] == "query");
        merged
    }

    /// The JSON schema of the lowest 2xx response.
    fn success(operation: &Value) -> Option<&Value> {
        let responses = operation.get("responses")?.as_object()?;
        let code = responses.keys().filter(|code| code.starts_with('2')).min()?;

        responses[code].pointer("/content/application~1json/schema")
    }

    /// Folds the query parameters into one object schema.
    fn schema(query: &[Value]) -> Value {
        let properties: Map<String, Value> = query
            .iter()
            .filter_map(|parameter| {
                Some((
                    parameter["name"].as_str()?.to_string(),
                    parameter.get("schema").cloned().unwrap_or(json!({})),
                ))
            })
            .collect();

        let required: Vec<&Value> = query
            .iter()
            .filter(|parameter| parameter["required"] == true)
            .map(|parameter| &parameter["name"])
            .collect();

        json!({ "type": "object", "properties": properties, "required": required })
    }
}
