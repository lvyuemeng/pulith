use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::{Stream, StreamExt};

use crate::error::Result;

/// A boxed stream type for HTTP response bodies.
///
/// This type alias simplifies the complex stream type used throughout the crate.
/// The stream yields `Result<Bytes, E>` where E is the error type from the HTTP client.
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + Send + 'a>>;

/// Asynchronous HTTP client abstraction.
///
/// This trait provides the minimal interface needed for fetching operations.
/// Implementations handle their own redirect following, timeout configuration,
/// and error mapping.
///
/// # Implementations
///
/// - [`ReqwestClient`]: Production implementation using `reqwest`
/// - Mock implementations for testing
pub trait HttpClient: Send + Sync {
    /// Error type for HTTP operations.
    type Error: std::error::Error + Send + 'static;

    /// Open a streaming HTTP connection and return the response body as a stream.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch
    /// * `headers` - Custom headers to include with the request
    ///
    /// # Returns
    ///
    /// A stream of bytes from the response body.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails (DNS failure, connection error,
    /// HTTP error status, etc.). Implementations should map HTTP errors to
    /// a suitable error type.
    fn stream(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> impl Future<Output = std::result::Result<BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error>>
           + Send;

    /// Query the Content-Length header without downloading the body.
    ///
    /// This is used for progress reporting when the total file size is known.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to query
    ///
    /// # Returns
    ///
    /// `Ok(Some(n))` if Content-Length is present,
    /// `Ok(None)` if absent or using chunked encoding,
    /// `Err(...)` if the request fails.
    fn head(
        &self,
        url: &str,
    ) -> impl Future<Output = std::result::Result<Option<u64>, Self::Error>> + Send;
}

#[cfg(feature = "reqwest")]
mod reqwest_impl {
    use super::*;
    use reqwest;

    /// Production HTTP client implementation using reqwest.
    pub struct ReqwestClient {
        client: reqwest::Client,
    }

    impl ReqwestClient {
        /// Create a new ReqwestClient with default configuration.
        pub fn new() -> Result<Self> {
            let client = reqwest::Client::new();
            Ok(Self { client })
        }
    }

    impl HttpClient for ReqwestClient {
        type Error = reqwest::Error;

        async fn stream(
            &self,
            url: &str,
            headers: &[(String, String)],
        ) -> std::result::Result<BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error> {
            let mut request = self.client.get(url);
            
            for (key, value) in headers {
                request = request.header(key, value);
            }
            
            let response = request.send().await?;
            let stream = response.bytes_stream().map(|result| result.map(Bytes::from));
            
            Ok(Box::pin(stream))
        }

        async fn head(
            &self,
            url: &str,
        ) -> std::result::Result<Option<u64>, Self::Error> {
            let response = self.client.head(url).send().await?;
            let content_length = response.headers().get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());
            
            Ok(content_length)
        }
    }
}

#[cfg(feature = "reqwest")]
pub use reqwest_impl::ReqwestClient;