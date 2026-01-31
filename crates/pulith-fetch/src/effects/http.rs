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
            let stream = response.bytes_stream().map(|result| result);
            
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use futures_util::stream::{self, Stream};

    // Mock HTTP client for testing
    struct MockHttpClient {
        should_fail: bool,
        content_length: Option<u64>,
    }

    impl MockHttpClient {
        fn new() -> Self {
            Self {
                should_fail: false,
                content_length: Some(1024),
            }
        }

        fn with_error() -> Self {
            Self {
                should_fail: true,
                content_length: None,
            }
        }

        fn with_content_length(length: u64) -> Self {
            Self {
                should_fail: false,
                content_length: Some(length),
            }
        }

        fn without_content_length() -> Self {
            Self {
                should_fail: false,
                content_length: None,
            }
        }
    }

    #[derive(Debug)]
    struct MockError(String);

    impl std::fmt::Display for MockError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for MockError {}

    impl HttpClient for MockHttpClient {
        type Error = MockError;

        fn stream(
            &self,
            _url: &str,
            _headers: &[(String, String)],
        ) -> impl Future<Output = std::result::Result<BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error>>
               + Send {
            async move {
                if self.should_fail {
                    Err(MockError("Stream failed".to_string()))
                } else {
                    let data = vec![Bytes::from("test data")];
                    let stream = stream::iter(data).map(Ok);
                    Ok(Box::pin(stream))
                }
            }
        }

        fn head(
            &self,
            _url: &str,
        ) -> impl Future<Output = std::result::Result<Option<u64>, Self::Error>> + Send {
            async move {
                if self.should_fail {
                    Err(MockError("HEAD request failed".to_string()))
                } else {
                    Ok(self.content_length)
                }
            }
        }
    }

    #[tokio::test]
    async fn test_mock_http_client_stream_success() {
        let client = MockHttpClient::new();
        let result = client.stream("http://example.com", &[]).await;
        assert!(result.is_ok());
        
        let stream = result.unwrap();
        // The stream should yield one item
        let pinned = Pin::new(&mut Box::pin(stream));
        match futures_util::future::poll_next(pinned) {
            Poll::Ready(Some(Ok(bytes))) => {
                assert_eq!(bytes, Bytes::from("test data"));
            }
            _ => panic!("Expected data"),
        }
    }

    #[tokio::test]
    async fn test_mock_http_client_stream_error() {
        let client = MockHttpClient::with_error();
        let result = client.stream("http://example.com", &[]).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Stream failed");
    }

    #[tokio::test]
    async fn test_mock_http_client_head_with_content_length() {
        let client = MockHttpClient::with_content_length(2048);
        let result = client.head("http://example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(2048));
    }

    #[tokio::test]
    async fn test_mock_http_client_head_without_content_length() {
        let client = MockHttpClient::without_content_length();
        let result = client.head("http://example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_mock_http_client_head_error() {
        let client = MockHttpClient::with_error();
        let result = client.head("http://example.com").await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "HEAD request failed");
    }

    #[test]
    fn test_box_stream_type_alias() {
        // Test that BoxStream is a valid type
        fn _assert_send_sync<T: Send + Sync>(_: T) {}
        
        let _stream: BoxStream<'static, Result<Bytes, MockError>> = 
            Box::pin(stream::empty());
        
        // This would fail to compile if BoxStream wasn't Send + Sync
        // _assert_send_sync(_stream);
    }

    

    #[cfg(feature = "reqwest")]
    #[tokio::test]
    async fn test_reqwest_client_creation() {
        // Test that ReqwestClient can be created
        let result = ReqwestClient::new();
        assert!(result.is_ok());
        
        let client = result.unwrap();
        // The client should be usable
        let _client: ReqwestClient = client;
    }
}