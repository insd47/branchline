use schemars::schema::Schema;
use serde_json::Value;
use typify::{TypeId, TypeSpace, TypeSpaceSettings};

/// The type space of a document. `components.schemas` and inline schemas all land in one `types` module.
pub struct Types {
    space: TypeSpace,
}

impl Types {
    /// Registers `components.schemas` first, since operations refer to them by `$ref`.
    ///
    /// typify's own builders are off; [`Builders`](crate::builders::Builders) adds `bon` builders instead.
    pub fn new(document: &Value) -> Result<Self, String> {
        let mut settings = TypeSpaceSettings::default();
        settings.with_type_mod("types").with_struct_builder(false);

        let mut space = TypeSpace::new(&settings);

        let schemas = document
            .pointer("/components/schemas")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let definitions = schemas
            .into_iter()
            .map(|(name, schema)| Ok((name, Self::parse(&schema)?)))
            .collect::<Result<Vec<_>, String>>()?;

        space.add_ref_types(definitions).map_err(|error| error.to_string())?;

        Ok(Self { space })
    }

    /// Registers one schema and returns how to refer to it (`types::X`, `Vec<types::X>`, ...).
    pub fn path(&mut self, schema: &Value, hint: &str) -> Result<proc_macro2::TokenStream, String> {
        let id = self.id(schema, hint)?;

        Ok(self.space.get_type(&id).map_err(|error| error.to_string())?.ident())
    }

    pub fn stream(&self) -> proc_macro2::TokenStream {
        self.space.to_stream()
    }

    fn id(&mut self, schema: &Value, hint: &str) -> Result<TypeId, String> {
        self.space
            .add_type_with_name(&Self::parse(schema)?, Some(hint.into()))
            .map_err(|error| error.to_string())
    }

    fn parse(schema: &Value) -> Result<Schema, String> {
        serde_json::from_value(schema.clone()).map_err(|error| error.to_string())
    }
}
