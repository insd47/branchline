use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use std::fmt::Display;

/// A path relative to the service root, without a leading `/` so that joining it onto a base URL keeps the base path.
#[derive(Debug, Clone, Default)]
pub struct Path(String);

impl Path {
    pub fn join(&self, segment: &str) -> Self {
        match self.0.is_empty() {
            true => Self(segment.into()),
            false => Self(format!("{}/{segment}", self.0)),
        }
    }

    /// Path parameters keep only RFC 3986 unreserved characters and percent-encode the rest.
    pub fn param(&self, value: impl Display) -> Self {
        const UNRESERVED: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');

        self.join(&utf8_percent_encode(&value.to_string(), UNRESERVED).to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn joins_relative_segments() {
        let path = Path::default().join("contests").param(7).join("notices");
        assert_eq!(path.as_str(), "contests/7/notices");
    }

    #[test]
    fn encodes_parameters() {
        assert_eq!(Path::default().param("a b/c?").as_str(), "a%20b%2Fc%3F");
        assert_eq!(Path::default().param("0199-abc_~.").as_str(), "0199-abc_~.");
    }
}
