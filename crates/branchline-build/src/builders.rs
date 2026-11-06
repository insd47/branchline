use syn::visit_mut::VisitMut;

/// Adds a `bon` builder to every struct with named fields, so `X::builder().a(1).build()` fails to compile when a
/// required field is missing instead of failing at runtime.
///
/// - `bon` is re-exported by the runtime (`branchline::bon`), so callers need no extra dependency.
/// - `String` setters take `impl Into<String>`, so `.name("..")` works.
/// - typify writes optional arrays and fields with schema defaults as `#[serde(default)]` instead of `Option`. Those
///   become `#[builder(default)]` (or `default = path()`), so they are optional in the builder too.
pub struct Builders<'a> {
    pub runtime: &'a syn::Path,
}

impl VisitMut for Builders<'_> {
    fn visit_item_struct_mut(&mut self, item: &mut syn::ItemStruct) {
        let syn::Fields::Named(fields) = &mut item.fields else {
            return;
        };

        let runtime = self.runtime;
        item.attrs.push(syn::parse_quote!(#[derive(#runtime::bon::Builder)]));
        item.attrs.push(syn::parse_quote!(
            #[builder(crate = #runtime::bon, on(::std::string::String, into))]
        ));

        for field in &mut fields.named {
            if let Some(default) = Self::default(&field.attrs) {
                field.attrs.push(default);
            }
        }
    }
}

impl Builders<'_> {
    /// The builder default matching a field's `#[serde(default)]` or `#[serde(default = "path")]`.
    fn default(attrs: &[syn::Attribute]) -> Option<syn::Attribute> {
        let mut found = None;

        for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("default") {
                    found = Some(match meta.value() {
                        Ok(value) => {
                            let path: syn::Path = value.parse::<syn::LitStr>()?.parse()?;
                            syn::parse_quote!(#[builder(default = #path())])
                        }
                        Err(_) => syn::parse_quote!(#[builder(default)]),
                    });
                } else if meta.input.peek(syn::Token![=]) {
                    meta.value()?.parse::<syn::Expr>()?;
                }
                Ok(())
            });
        }

        found
    }
}

#[cfg(test)]
mod tests {
    use super::Builders;
    use syn::visit_mut::VisitMut;

    #[test]
    fn defaults_follow_serde() {
        let mut item: syn::ItemStruct = syn::parse_quote! {
            pub struct Notice {
                pub title: ::std::string::String,
                #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
                pub tags: ::std::vec::Vec<::std::string::String>,
                #[serde(default = "defaults::limit")]
                pub limit: i64,
            }
        };

        Builders {
            runtime: &syn::parse_quote!(::branchline),
        }
        .visit_item_struct_mut(&mut item);
        let code = quote::quote!(#item).to_string();

        assert!(code.contains("derive (:: branchline :: bon :: Builder)"), "{code}");
        assert!(code.contains("# [builder (default)] pub tags"), "{code}");
        assert!(
            code.contains("# [builder (default = defaults :: limit ())] pub limit"),
            "{code}"
        );
    }
}
