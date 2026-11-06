//! Building and sending requests, shared by the built-in transports.

use crate::HttpError;
use bytes::Bytes;
use http::Method;
use url::Url;

/// Headers every request carries. The body is always JSON.
pub const HEADERS: [(&str, &str); 2] = [("content-type", "application/json"), ("accept", "application/json")];

/// Joins `path` and `query` onto `base`, which must end with `/` to keep its own path.
pub fn url(base: &Url, path: &str, query: Option<&str>) -> Result<Url, HttpError> {
    let mut url = base.join(path).map_err(|error| HttpError::Url(error.to_string()))?;
    url.set_query(query.filter(|query| !query.is_empty()));
    Ok(url)
}

/// Parses a base URL and makes sure it ends with `/`.
pub fn base(url: &str) -> Result<Url, HttpError> {
    let url = match url.ends_with('/') {
        true => url.to_owned(),
        false => format!("{url}/"),
    };

    Url::parse(&url).map_err(|error| HttpError::Url(error.to_string()))
}

/// Builds the request with the shared headers.
pub fn build(method: Method, url: &Url, body: Vec<u8>) -> Result<http::Request<Vec<u8>>, HttpError> {
    HEADERS
        .iter()
        .fold(
            http::Request::builder().method(method).uri(url.as_str()),
            |request, (name, value)| request.header(*name, *value),
        )
        .body(body)
        .map_err(|error| HttpError::Url(error.to_string()))
}

/// Sends the request and returns the body of a 2xx response.
pub async fn send(client: &reqwest::Client, request: http::Request<Vec<u8>>) -> Result<Bytes, HttpError> {
    let response = client.execute(reqwest::Request::try_from(request)?).await?;
    let status = response.status();
    let bytes = response.bytes().await?;

    match status.is_success() {
        true => Ok(bytes),
        false => Err(HttpError::Status {
            status,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{base, url};

    #[test]
    fn keeps_the_base_path() {
        let base = base("https://example.com/api").unwrap();
        assert_eq!(
            url(&base, "contests/7", Some("a=1")).unwrap().as_str(),
            "https://example.com/api/contests/7?a=1"
        );
        assert_eq!(
            url(&base, "contests", Some("")).unwrap().as_str(),
            "https://example.com/api/contests"
        );
    }
}
