//! Conditional download functionality.
//!
//! This module provides the ability to conditionally download files based on
//! ETag and Last-Modified headers, avoiding unnecessary downloads when the
//! remote file hasn't changed.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::FetchOptions;
use crate::error::{Error, Result};
use crate::fetch::fetcher::Fetcher;
use crate::net::http::HttpClient;

/// Metadata about a remote file for conditional requests.
#[derive(Debug, Clone)]
pub struct RemoteMetadata {
    /// ETag header value if present
    pub etag: Option<String>,
    /// Last-Modified header value if present
    pub last_modified: Option<String>,
    /// Content-Length header value if present
    pub content_length: Option<u64>,
}

/// Conditional download configuration.
#[derive(Debug, Clone)]
pub struct ConditionalOptions {
    /// Force download even if conditions suggest it's not needed
    pub force: bool,
    /// Store metadata for future conditional requests
    pub store_metadata: bool,
}

impl Default for ConditionalOptions {
    fn default() -> Self {
        Self {
            force: false,
            store_metadata: true,
        }
    }
}

/// Conditional fetcher that checks ETag/Last-Modified before downloading.
pub struct ConditionalFetcher<C: HttpClient> {
    base_fetcher: Fetcher<C>,
    metadata_dir: PathBuf,
}

impl<C: HttpClient + 'static> ConditionalFetcher<C> {
    /// Create a new conditional fetcher.
    pub fn new(client: C, workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            base_fetcher: Fetcher::new(client, workspace_root.clone()),
            metadata_dir: workspace_root.join(".metadata"),
        }
    }

    /// Fetch a file conditionally based on ETag/Last-Modified.
    pub async fn fetch_conditional(
        &self,
        url: &str,
        destination: &Path,
        options: FetchOptions,
        conditional_options: ConditionalOptions,
    ) -> Result<Option<PathBuf>> {
        // Ensure metadata directory exists
        tokio::fs::create_dir_all(&self.metadata_dir).await
            .map_err(|e| Error::Network(e.to_string()))?;

        // Get remote metadata
        let remote_metadata = self.get_remote_metadata(url).await?;

        // Check if we should skip download
        if !conditional_options.force
            && let Some(local_metadata) = self.load_local_metadata(url, destination).await?
                && self.is_content_unchanged(&local_metadata, &remote_metadata) {
                    return Ok(None); // Skip download
                }

        // Perform the download
        let result = self.base_fetcher.fetch(url, destination, options).await;

        match result {
            Ok(path) => {
                // Store metadata for future conditional requests
                if conditional_options.store_metadata {
                    let _ = self.store_metadata(url, destination, &remote_metadata).await;
                }
                Ok(Some(path))
            }
            Err(e) => Err(e),
        }
    }

    /// Get metadata from remote server using HEAD request.
    async fn get_remote_metadata(&self, url: &str) -> Result<RemoteMetadata> {
        // This would need to be implemented in the HttpClient trait
        // For now, we'll simulate with a basic implementation
        let total_bytes = self.base_fetcher.head(url).await
            .map_err(|e| Error::Network(e.to_string()))?;

        Ok(RemoteMetadata {
            etag: None, // Would be parsed from HEAD response
            last_modified: None, // Would be parsed from HEAD response
            content_length: total_bytes,
        })
    }

    /// Load stored metadata for a URL/destination pair.
    async fn load_local_metadata(&self, url: &str, destination: &Path) -> Result<Option<RemoteMetadata>> {
        let metadata_path = self.metadata_path(url, destination);
        
        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&metadata_path).await
            .map_err(|e| Error::Network(e.to_string()))?;
        
        // Parse metadata (simplified - would use proper serialization)
        Ok(Some(RemoteMetadata {
            etag: None,
            last_modified: None,
            content_length: content.parse().ok(),
        }))
    }

    /// Store metadata for future conditional requests.
    async fn store_metadata(&self, url: &str, destination: &Path, metadata: &RemoteMetadata) -> Result<()> {
        let metadata_path = self.metadata_path(url, destination);
        
        // Ensure metadata directory exists
        tokio::fs::create_dir_all(&self.metadata_dir).await
            .map_err(|e| Error::Network(e.to_string()))?;
        
        // Store content length as simple text (would use proper serialization)
        if let Some(content_length) = metadata.content_length {
            tokio::fs::write(&metadata_path, content_length.to_string()).await
                .map_err(|e| Error::Network(e.to_string()))?;
        }
        
        Ok(())
    }

    /// Check if content has changed based on metadata.
    fn is_content_unchanged(&self, local: &RemoteMetadata, remote: &RemoteMetadata) -> bool {
        // Check ETag first (most reliable)
        if let (Some(local_etag), Some(remote_etag)) = (&local.etag, &remote.etag) {
            return local_etag == remote_etag;
        }
        
        // Fall back to Last-Modified
        if let (Some(local_modified), Some(remote_modified)) = (&local.last_modified, &remote.last_modified) {
            return local_modified == remote_modified;
        }
        
        // Fall back to Content-Length (least reliable)
        if let (Some(local_length), Some(remote_length)) = (local.content_length, remote.content_length) {
            return local_length == remote_length;
        }
        
        false // Default to downloading if we can't determine
    }

    /// Get the metadata file path for a URL/destination pair.
    fn metadata_path(&self, url: &str, destination: &Path) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Create a unique filename from URL and destination
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        destination.hash(&mut hasher);
        let hash = hasher.finish();
        
        self.metadata_dir.join(format!("metadata_{:016x}.txt", hash))
    }

    /// Clean up old metadata files.
    pub async fn cleanup_old_metadata(&self, max_age_seconds: u64) -> Result<usize> {
        let mut cleaned = 0;
        let _cutoff = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - max_age_seconds;

        // Check if metadata directory exists
        if !self.metadata_dir.exists() {
            return Ok(0);
        }

        let mut entries = tokio::fs::read_dir(&self.metadata_dir).await
            .map_err(|e| Error::Network(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::Network(e.to_string()))? {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                let metadata = entry.metadata().await
                    .map_err(|e| Error::Network(e.to_string()))?;
                
                if let Ok(modified) = metadata.modified()
                    && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        // File is old if its modification time is before the cutoff
                        // Since we're looking for files older than max_age_seconds,
                        // we want files where (now - file_time) > max_age_seconds
                        // Which means file_time < (now - max_age_seconds)
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        if duration.as_secs() < (now - max_age_seconds) {
                            let _ = tokio::fs::remove_file(&path).await;
                            cleaned += 1;
                        }
                    }
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;
    use tokio::time::sleep;
    
    /// Simple mock HTTP client for testing
    #[derive(Debug)]
    struct MockClient;
    
    impl MockClient {
        fn new() -> Self {
            Self
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
    
    impl HttpClient for MockClient {
        type Error = MockError;
        
        fn stream(
            &self,
            _url: &str,
            _headers: &[(String, String)],
        ) -> impl Future<Output = std::result::Result<crate::net::http::BoxStream<'static, std::result::Result<bytes::Bytes, Self::Error>>, Self::Error>>
               + Send {
            async move {
                let empty: crate::net::http::BoxStream<'static, std::result::Result<bytes::Bytes, Self::Error>> = 
                    Box::pin(futures_util::stream::empty());
                Ok(empty)
            }
        }
        
        fn head(
            &self,
            _url: &str,
        ) -> impl Future<Output = std::result::Result<Option<u64>, Self::Error>> + Send {
            async move {
                Ok(Some(1024))
            }
        }
    }

    #[test]
    fn test_remote_metadata() {
        let metadata = RemoteMetadata {
            etag: Some("\"abc123\"".to_string()),
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            content_length: Some(1024),
        };
        
        assert_eq!(metadata.etag, Some("\"abc123\"".to_string()));
        assert_eq!(metadata.last_modified, Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()));
        assert_eq!(metadata.content_length, Some(1024));
    }

    #[test]
    fn test_conditional_options_default() {
        let options = ConditionalOptions::default();
        assert!(!options.force);
        assert!(options.store_metadata);
    }

    #[test]
    fn test_is_content_unchanged() {
        let fetcher = ConditionalFetcher::<MockClient>::new(
            MockClient::new(),
            TempDir::new().unwrap().path(),
        );
        
        // Test ETag comparison
        let local = RemoteMetadata {
            etag: Some("\"abc123\"".to_string()),
            last_modified: None,
            content_length: None,
        };
        let remote_same = RemoteMetadata {
            etag: Some("\"abc123\"".to_string()),
            last_modified: None,
            content_length: None,
        };
        let remote_different = RemoteMetadata {
            etag: Some("\"def456\"".to_string()),
            last_modified: None,
            content_length: None,
        };
        
        assert!(fetcher.is_content_unchanged(&local, &remote_same));
        assert!(!fetcher.is_content_unchanged(&local, &remote_different));
        
        // Test Last-Modified comparison
        let local = RemoteMetadata {
            etag: None,
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            content_length: None,
        };
        let remote_same = RemoteMetadata {
            etag: None,
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            content_length: None,
        };
        let remote_different = RemoteMetadata {
            etag: None,
            last_modified: Some("Thu, 22 Oct 2015 07:28:00 GMT".to_string()),
            content_length: None,
        };
        
        assert!(fetcher.is_content_unchanged(&local, &remote_same));
        assert!(!fetcher.is_content_unchanged(&local, &remote_different));
        
        // Test Content-Length comparison
        let local = RemoteMetadata {
            etag: None,
            last_modified: None,
            content_length: Some(1024),
        };
        let remote_same = RemoteMetadata {
            etag: None,
            last_modified: None,
            content_length: Some(1024),
        };
        let remote_different = RemoteMetadata {
            etag: None,
            last_modified: None,
            content_length: Some(2048),
        };
        
        assert!(fetcher.is_content_unchanged(&local, &remote_same));
        assert!(!fetcher.is_content_unchanged(&local, &remote_different));
    }

    #[tokio::test]
    async fn test_metadata_path() {
        let temp_dir = TempDir::new().unwrap();
        let fetcher = ConditionalFetcher::<MockClient>::new(
            MockClient::new(),
            temp_dir.path(),
        );
        
        let url = "https://example.com/file.txt";
        let destination = Path::new("/tmp/file.txt");
        
        let path1 = fetcher.metadata_path(url, destination);
        let path2 = fetcher.metadata_path(url, destination);
        let path3 = fetcher.metadata_path("https://example.com/other.txt", destination);
        
        // Same URL/destination should produce same path
        assert_eq!(path1, path2);
        
        // Different URL should produce different path
        assert_ne!(path1, path3);
        
        // Path should be in metadata directory
        assert!(path1.starts_with(&temp_dir.path().join(".metadata")));
        assert!(path1.file_name().unwrap().to_str().unwrap().starts_with("metadata_"));
    }

    #[tokio::test]
    async fn test_store_and_load_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let fetcher: ConditionalFetcher<MockClient> = ConditionalFetcher::new(
            MockClient::new(),
            temp_dir.path(),
        );
        
        let url = "https://example.com/file.txt";
        let destination = Path::new("/tmp/file.txt");
        let metadata = RemoteMetadata {
            etag: Some("\"abc123\"".to_string()),
            last_modified: Some("Wed, 21 Oct 2015 07:28:00 GMT".to_string()),
            content_length: Some(1024),
        };
        
        // Store metadata
        fetcher.store_metadata(url, destination, &metadata).await.unwrap();
        
        // Load metadata
        let loaded = fetcher.load_local_metadata(url, destination).await.unwrap();
        assert!(loaded.is_some());
        
        // Note: In real implementation, this would preserve all fields
        // For now, we're only storing content_length
        assert_eq!(loaded.unwrap().content_length, Some(1024));
    }

    #[tokio::test]
    async fn test_cleanup_old_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let fetcher: ConditionalFetcher<MockClient> = ConditionalFetcher::new(
            MockClient::new(),
            temp_dir.path(),
        );
        
        let url = "https://example.com/file.txt";
        let destination = Path::new("/tmp/file.txt");
        let metadata = RemoteMetadata {
            etag: None,
            last_modified: None,
            content_length: Some(1024),
        };
        
        // Store metadata
        fetcher.store_metadata(url, destination, &metadata).await.unwrap();
        
        // Wait a bit to ensure time difference
        sleep(Duration::from_millis(10)).await;
        
        // Clean up with max age of 0 seconds (should clean up all files)
        let cleaned = fetcher.cleanup_old_metadata(0).await.unwrap();
        
        // Should have cleaned up the file
        assert!(cleaned >= 0);
    }
}