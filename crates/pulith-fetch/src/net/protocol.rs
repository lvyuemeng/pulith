//! Protocol abstraction layer.
//!
//! This module provides an extensible protocol abstraction
//! for supporting multiple transfer protocols beyond HTTP.

use crate::error::{Error, Result};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncRead;

/// Protocol identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// HTTP/HTTPS protocol
    Http,
    /// FTP protocol (future)
    Ftp,
    /// S3 protocol (future)
    S3,
    /// SSH/SFTP protocol (future)
    Sftp,
    /// Custom protocol
    Custom(&'static str),
}

impl Protocol {
    /// Get the string representation of the protocol.
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Http => "http",
            Protocol::Ftp => "ftp",
            Protocol::S3 => "s3",
            Protocol::Sftp => "sftp",
            Protocol::Custom(name) => name,
        }
    }
    
    /// Parse a protocol from a URL scheme.
    pub fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme.to_lowercase().as_str() {
            "http" | "https" => Some(Protocol::Http),
            "ftp" => Some(Protocol::Ftp),
            "s3" => Some(Protocol::S3),
            "sftp" | "ssh" => Some(Protocol::Sftp),
            _ => None,
        }
    }
}

/// Transfer direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Download (remote to local)
    Download,
    /// Upload (local to remote)
    Upload,
}

/// Transfer metadata.
#[derive(Debug, Clone)]
pub struct TransferMetadata {
    /// Size of the content in bytes
    pub size: Option<u64>,
    /// Last modified timestamp
    pub last_modified: Option<u64>,
    /// ETag or checksum
    pub etag: Option<String>,
    /// Content type
    pub content_type: Option<String>,
    /// Additional metadata
    pub extra: HashMap<String, String>,
}

impl TransferMetadata {
    /// Create new metadata.
    pub fn new() -> Self {
        Self {
            size: None,
            last_modified: None,
            etag: None,
            content_type: None,
            extra: HashMap::new(),
        }
    }
    
    /// Set the size.
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
    
    /// Set the last modified timestamp.
    pub fn with_last_modified(mut self, timestamp: u64) -> Self {
        self.last_modified = Some(timestamp);
        self
    }
    
    /// Set the ETag.
    pub fn with_etag(mut self, etag: String) -> Self {
        self.etag = Some(etag);
        self
    }
    
    /// Set the content type.
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }
    
    /// Add extra metadata.
    pub fn with_extra(mut self, key: String, value: String) -> Self {
        self.extra.insert(key, value);
        self
    }
}

impl Default for TransferMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Transfer options.
#[derive(Debug, Clone)]
pub struct TransferOptions {
    /// Direction of transfer
    pub direction: Direction,
    /// Whether to resume partial transfers
    pub resume: bool,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    /// Maximum number of retries
    pub max_retries: Option<u32>,
    /// Custom headers (for HTTP)
    pub headers: HashMap<String, String>,
    /// Additional protocol-specific options
    pub protocol_options: HashMap<String, String>,
}

impl TransferOptions {
    /// Create new options for downloading.
    pub fn download() -> Self {
        Self {
            direction: Direction::Download,
            resume: false,
            timeout: None,
            max_retries: None,
            headers: HashMap::new(),
            protocol_options: HashMap::new(),
        }
    }
    
    /// Create new options for uploading.
    pub fn upload() -> Self {
        Self {
            direction: Direction::Upload,
            resume: false,
            timeout: None,
            max_retries: None,
            headers: HashMap::new(),
            protocol_options: HashMap::new(),
        }
    }
    
    /// Set resume option.
    pub fn with_resume(mut self, resume: bool) -> Self {
        self.resume = resume;
        self
    }
    
    /// Set timeout.
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    /// Set max retries.
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }
    
    /// Add a header.
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }
    
    /// Add a protocol option.
    pub fn with_protocol_option(mut self, key: String, value: String) -> Self {
        self.protocol_options.insert(key, value);
        self
    }
}

/// A stream of data being transferred.
pub trait TransferStream: AsyncRead + Send + Unpin {
    /// Get metadata about the transfer.
    fn metadata(&self) -> &TransferMetadata;
}

/// Protocol client trait.
#[async_trait]
pub trait ProtocolClient: Send + Sync {
    /// Get the protocol this client handles.
    fn protocol(&self) -> Protocol;
    
    /// Check if this client can handle the given URL.
    fn can_handle(&self, url: &str) -> bool {
        Protocol::from_scheme(url.split("://").next().unwrap_or("")) == Some(self.protocol())
    }
    
    /// Get metadata for a remote resource.
    async fn head(&self, url: &str, options: &TransferOptions) -> Result<TransferMetadata>;
    
    /// Start a transfer.
    async fn transfer(
        &self,
        url: &str,
        options: TransferOptions,
    ) -> Result<Box<dyn TransferStream>>;
    
    /// Check if a partial transfer exists and can be resumed.
    async fn can_resume(&self, url: &str, options: &TransferOptions) -> Result<bool>;
}

/// Protocol registry for managing multiple protocol clients.
pub struct ProtocolRegistry {
    clients: HashMap<Protocol, Box<dyn ProtocolClient>>,
}

impl ProtocolRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }
    
    /// Register a protocol client.
    pub fn register(&mut self, client: Box<dyn ProtocolClient>) {
        self.clients.insert(client.protocol(), client);
    }
    
    /// Get a client for the given protocol.
    pub fn get(&self, protocol: Protocol) -> Option<&dyn ProtocolClient> {
        self.clients.get(&protocol).map(|client| client.as_ref())
    }
    
    /// Find a client that can handle the given URL.
    pub fn find_for_url(&self, url: &str) -> Option<&dyn ProtocolClient> {
        if let Some(protocol) = Protocol::from_scheme(url.split("://").next().unwrap_or("")) {
            self.get(protocol)
        } else {
            None
        }
    }
    
    /// List all registered protocols.
    pub fn protocols(&self) -> Vec<Protocol> {
        self.clients.keys().copied().collect()
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock HTTP client for testing.
pub struct MockHttpClient {
    metadata: HashMap<String, TransferMetadata>,
}

impl MockHttpClient {
    /// Create a new mock HTTP client.
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }
    
    /// Add mock metadata for a URL.
    pub fn add_metadata(&mut self, url: String, metadata: TransferMetadata) {
        self.metadata.insert(url, metadata);
    }
}

impl Default for MockHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolClient for MockHttpClient {
    fn protocol(&self) -> Protocol {
        Protocol::Http
    }
    
    async fn head(&self, url: &str, _options: &TransferOptions) -> Result<TransferMetadata> {
        self.metadata
            .get(url)
            .cloned()
            .ok_or_else(|| Error::InvalidState(format!("No metadata for URL: {}", url)))
    }
    
    async fn transfer(
        &self,
        url: &str,
        _options: TransferOptions,
    ) -> Result<Box<dyn TransferStream>> {
        let metadata = self.head(url, &TransferOptions::download()).await?;
        Ok(Box::new(MockTransferStream::new(metadata)))
    }
    
    async fn can_resume(&self, _url: &str, _options: &TransferOptions) -> Result<bool> {
        Ok(false)
    }
}

/// Mock transfer stream for testing.
pub struct MockTransferStream {
    metadata: TransferMetadata,
    data: Bytes,
    pos: usize,
}

impl MockTransferStream {
    /// Create a new mock transfer stream.
    pub fn new(metadata: TransferMetadata) -> Self {
        let size = metadata.size.unwrap_or(0) as usize;
        Self {
            metadata,
            data: Bytes::from(vec![0u8; size]),
            pos: 0,
        }
    }
    
    /// Create a mock stream with custom data.
    pub fn with_data(metadata: TransferMetadata, data: Vec<u8>) -> Self {
        Self {
            metadata,
            data: Bytes::from(data),
            pos: 0,
        }
    }
}

impl AsyncRead for MockTransferStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pos >= self.data.len() {
            return Poll::Ready(Ok(()));
        }
        
        let remaining = self.data.len() - self.pos;
        let to_copy = std::cmp::min(remaining, buf.remaining());
        
        buf.put_slice(&self.data[self.pos..self.pos + to_copy]);
        self.pos += to_copy;
        
        Poll::Ready(Ok(()))
    }
}

impl TransferStream for MockTransferStream {
    fn metadata(&self) -> &TransferMetadata {
        &self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_protocol() {
        assert_eq!(Protocol::Http.as_str(), "http");
        assert_eq!(Protocol::from_scheme("http"), Some(Protocol::Http));
        assert_eq!(Protocol::from_scheme("HTTPS"), Some(Protocol::Http));
        assert_eq!(Protocol::from_scheme("ftp"), Some(Protocol::Ftp));
        assert_eq!(Protocol::from_scheme("unknown"), None);
    }
    
    #[test]
    fn test_transfer_metadata() {
        let metadata = TransferMetadata::new()
            .with_size(1024)
            .with_last_modified(1234567890)
            .with_etag("etag123".to_string())
            .with_content_type("text/plain".to_string())
            .with_extra("key".to_string(), "value".to_string());
        
        assert_eq!(metadata.size, Some(1024));
        assert_eq!(metadata.last_modified, Some(1234567890));
        assert_eq!(metadata.etag, Some("etag123".to_string()));
        assert_eq!(metadata.content_type, Some("text/plain".to_string()));
        assert_eq!(metadata.extra.get("key"), Some(&"value".to_string()));
    }
    
    #[test]
    fn test_transfer_options() {
        let options = TransferOptions::download()
            .with_resume(true)
            .with_timeout(30)
            .with_max_retries(3)
            .with_header("Accept".to_string(), "*/*".to_string())
            .with_protocol_option("follow_redirects".to_string(), "true".to_string());
        
        assert_eq!(options.direction, Direction::Download);
        assert!(options.resume);
        assert_eq!(options.timeout, Some(30));
        assert_eq!(options.max_retries, Some(3));
        assert_eq!(options.headers.get("Accept"), Some(&"*/*".to_string()));
        assert_eq!(
            options.protocol_options.get("follow_redirects"),
            Some(&"true".to_string())
        );
    }
    
    #[tokio::test]
    async fn test_protocol_registry() {
        let mut registry = ProtocolRegistry::new();
        let client = MockHttpClient::new();
        registry.register(Box::new(client));
        
        assert!(registry.get(Protocol::Http).is_some());
        assert!(registry.get(Protocol::Ftp).is_none());
        
        assert!(registry.find_for_url("http://example.com").is_some());
        assert!(registry.find_for_url("ftp://example.com").is_none());
        
        let protocols = registry.protocols();
        assert!(protocols.contains(&Protocol::Http));
    }
    
    #[tokio::test]
    async fn test_mock_http_client() {
        let mut client = MockHttpClient::new();
        let metadata = TransferMetadata::new().with_size(1024);
        client.add_metadata("http://example.com/test.txt".to_string(), metadata.clone());
        
        let options = TransferOptions::download();
        let retrieved = client.head("http://example.com/test.txt", &options).await.unwrap();
        assert_eq!(retrieved.size, Some(1024));
        
        let stream = client.transfer("http://example.com/test.txt", options).await.unwrap();
        assert_eq!(stream.metadata().size, Some(1024));
    }
    
    #[tokio::test]
    async fn test_mock_transfer_stream() {
        let metadata = TransferMetadata::new().with_size(10);
        let stream = MockTransferStream::new(metadata);
        
        assert_eq!(stream.metadata().size, Some(10));
        assert_eq!(stream.data.len(), 10);
    }
}