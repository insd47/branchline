use crate::{Error, Transport, request};
use aws_credential_types::provider::error::CredentialsError;
use aws_credential_types::provider::{ProvideCredentials, SharedCredentialsProvider};
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;
use aws_types::SdkConfig;
use aws_types::region::Region;
use bytes::Bytes;
use http::Method;
use std::time::SystemTime;
use url::Url;

/// [`Http`](crate::Http) plus AWS SigV4 signing, for IAM-authenticated endpoints such as Lambda Function URLs.
///
/// - Credentials and region come from an AWS SDK config. Credentials are refreshed by the provider on every request.
/// - The exact body bytes are signed, so requests cannot be streamed.
#[derive(Debug, Clone)]
pub struct SigV4 {
    client: reqwest::Client,
    base: Url,
    credentials: SharedCredentialsProvider,
    region: Region,
    service: &'static str,
}

impl SigV4 {
    /// Signs for `service` (`lambda` for Function URLs) with the credentials and region of `config`.
    pub fn new(base: &str, config: &SdkConfig, service: &'static str) -> Result<Self, Error> {
        let credentials = config
            .credentials_provider()
            .ok_or_else(|| CredentialsError::not_loaded("no credentials provider in the SDK config"))?;
        let region = config
            .region()
            .cloned()
            .ok_or_else(|| CredentialsError::not_loaded("no region in the SDK config"))?;

        Ok(Self {
            client: reqwest::Client::new(),
            base: crate::request::base(base)?,
            credentials,
            region,
            service,
        })
    }

    /// Uses your reqwest client, for timeouts, proxies, or default headers.
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// Signs the request in place.
    async fn sign(&self, request: &mut http::Request<Vec<u8>>) -> Result<(), Error> {
        let identity = self.credentials.provide_credentials().await?.into();

        let params = v4::SigningParams::builder()
            .identity(&identity)
            .region(self.region.as_ref())
            .name(self.service)
            .time(SystemTime::now())
            .settings(SigningSettings::default())
            .build()
            .expect("every required signing parameter is set")
            .into();

        let uri = request.uri().to_string();
        let headers = request
            .headers()
            .iter()
            .filter_map(|(name, value)| Some((name.as_str(), value.to_str().ok()?)));

        let signable = SignableRequest::new(
            request.method().as_str(),
            &uri,
            headers,
            SignableBody::Bytes(request.body()),
        )?;

        let (instructions, _) = sign(signable, &params)?.into_parts();

        instructions.apply_to_request_http1x(request);
        Ok(())
    }
}

impl Transport for SigV4 {
    type Error = Error;

    async fn send(&self, method: Method, path: &str, query: Option<&str>, body: Vec<u8>) -> Result<Bytes, Error> {
        let url = request::url(&self.base, path, query)?;
        let mut request = request::build(method, &url, body)?;

        self.sign(&mut request).await?;
        request::send(&self.client, request).await
    }
}
