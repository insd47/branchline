use quote::format_ident;

/// Turns names from the document (path segments, parameters, methods) into Rust identifiers.
pub struct Name<'a>(pub &'a str);

impl Name<'_> {
    /// `contestId`, `problem-files` → `contest_id`, `problem_files`.
    pub fn snake(&self) -> String {
        let mut out = String::new();

        for (index, character) in self.0.chars().enumerate() {
            if character.is_ascii_uppercase() {
                if index > 0 && !out.ends_with('_') {
                    out.push('_');
                }
                out.push(character.to_ascii_lowercase());
            } else if character.is_ascii_alphanumeric() {
                out.push(character);
            } else if !out.ends_with('_') {
                out.push('_');
            }
        }

        out.trim_matches('_').to_string()
    }

    /// `problem-files` → `ProblemFiles`.
    pub fn pascal(&self) -> String {
        self.snake()
            .split('_')
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                chars
                    .next()
                    .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Module and method names. Keywords (`type`, `match`, ...) become raw identifiers.
    pub fn ident(&self) -> syn::Ident {
        let snake = self.snake();
        syn::parse_str::<syn::Ident>(&snake).unwrap_or_else(|_| format_ident!("r#{}", snake))
    }

    /// Type names.
    pub fn class(&self) -> syn::Ident {
        format_ident!("{}", self.pascal())
    }
}

#[cfg(test)]
mod tests {
    use super::Name;

    #[test]
    fn names_become_valid_rust_identifiers() {
        assert_eq!(Name("contestId").snake(), "contest_id");
        assert_eq!(Name("problem-files").snake(), "problem_files");
        assert_eq!(Name("get").pascal(), "Get");
        assert_eq!(Name("problem-files").pascal(), "ProblemFiles");
        assert_eq!(Name("type").ident().to_string(), "r#type");
    }
}
