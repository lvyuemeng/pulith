//! Resumable download functionality.
//!
//! This module provides the ability to resume interrupted downloads
//! using HTTP Range requests and state persistence.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::config::{FetchOptions, FetchPhase};
use crate::progress::Progress;
use crate::error::{Error, Result};
use crate::fetch::fetcher::Fetcher;
use crate::net::http::HttpClient;

/// Checkpoint data for resumable downloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadCheckpoint {
    /// URL being downloaded
    pub url: String,
    /// Destination path
    pub destination: PathBuf,
    /// Total bytes expected (from Content-Length)
    pub total_bytes: Option<u64>,
    /// Bytes already downloaded
    pub downloaded_bytes: u64,
    /// Checksum of downloaded data (if available)
    pub partial_checksum: Option<String>,
    /// Timestamp of last progress
    pub last_update: u64,
}

impl DownloadCheckpoint {
    /// Create a new checkpoint.
    pub fn new(url: String, destination: PathBuf, total_bytes: Option<u64>) -> Self {
        Self {
            url,
            destination,
            total_bytes,
            downloaded_bytes: 0,
            partial_checksum: None,
            last_update: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Update checkpoint with new progress.
    pub fn update_progress(&mut self, downloaded_bytes: u64) {
        self.downloaded_bytes = downloaded_bytes;
        self.last_update = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if download can be resumed.
    pub fn can_resume(&self) -> bool {
        self.downloaded_bytes > 0
    }

    /// Get the Range header value for resuming.
    pub fn range_header(&self) -> String {
        format!("bytes={}-", self.downloaded_bytes)
    }
}

/// Resumable fetcher implementation.
pub struct ResumableFetcher<C: HttpClient> {
    base_fetcher: Fetcher<C>,
    checkpoint_dir: PathBuf,
}

impl<C: HttpClient + 'static> ResumableFetcher<C> {
    /// Create a new resumable fetcher.
    pub fn new(client: C, workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            base_fetcher: Fetcher::new(client, workspace_root.clone()),
            checkpoint_dir: workspace_root.join(".checkpoints"),
        }
    }

    /// Fetch a file with resumable support.
    pub async fn fetch_resumable(
        &self,
        url: &str,
        destination: &Path,
        options: FetchOptions,
    ) -> Result<PathBuf> {
        // Ensure checkpoint directory exists
        fs::create_dir_all(&self.checkpoint_dir).await
            .map_err(|e| Error::Network(e.to_string()))?;

        let checkpoint_path = self.checkpoint_path(url, destination);
        
        // Try to load existing checkpoint
        if let Ok(checkpoint) = self.load_checkpoint(&checkpoint_path).await
            && checkpoint.can_resume() {
                return self.resume_download(&checkpoint, &checkpoint_path, options).await;
            }

        // Start new download
        self.start_new_download(url, destination, &checkpoint_path, options).await
    }

    /// Start a new download with checkpoint tracking.
    async fn start_new_download(
        &self,
        url: &str,
        destination: &Path,
        checkpoint_path: &Path,
        options: FetchOptions,
    ) -> Result<PathBuf> {
        // Get total bytes from HEAD request
        let total_bytes = self.base_fetcher.head(url).await
            .map_err(|e| Error::Network(e.to_string()))?;

        // Create initial checkpoint
        let checkpoint = DownloadCheckpoint::new(
            url.to_string(),
            destination.to_path_buf(),
            total_bytes,
        );

        // Save initial checkpoint
        self.save_checkpoint(&checkpoint, checkpoint_path).await?;

        // Set up progress callback to update checkpoint
        let checkpoint_path_clone = checkpoint_path.to_path_buf();
        let checkpoint_dir = self.checkpoint_dir.clone();
        let url_clone = url.to_string();
        let destination_clone = destination.to_path_buf();
        
        let mut options_with_checkpoint = options.clone();
        let original_callback = options_with_checkpoint.on_progress.clone();
        
        options_with_checkpoint.on_progress = Some(Arc::new(move |progress: &Progress| {
            // Update checkpoint on progress
            if progress.phase == FetchPhase::Downloading {
                // Create new checkpoint with updated progress
                let mut checkpoint = DownloadCheckpoint::new(
                    url_clone.clone(),
                    destination_clone.clone(),
                    total_bytes,
                );
                checkpoint.update_progress(progress.bytes_downloaded);
                
                // Save checkpoint asynchronously (fire and forget)
                let checkpoint_path = checkpoint_path_clone.clone();
                let checkpoint_dir = checkpoint_dir.clone();
                tokio::spawn(async move {
                    let _ = Self::save_checkpoint_static(&checkpoint, &checkpoint_path, &checkpoint_dir).await;
                });
            }
            
            // Call original callback if present
            if let Some(ref callback) = original_callback {
                callback(progress);
            }
        }));

        // Perform the download
        let result = self.base_fetcher.fetch(url, destination, options_with_checkpoint).await;

        match result {
            Ok(path) => {
                // Download successful, remove checkpoint
                let _ = fs::remove_file(checkpoint_path).await;
                Ok(path)
            }
            Err(e) => {
                // Download failed, checkpoint remains for resuming
                Err(e)
            }
        }
    }

    /// Resume an interrupted download.
    async fn resume_download(
        &self,
        checkpoint: &DownloadCheckpoint,
        checkpoint_path: &Path,
        options: FetchOptions,
    ) -> Result<PathBuf> {
        // Check if destination file exists and has expected size
        if checkpoint.destination.exists() {
            let metadata = fs::metadata(&checkpoint.destination).await
                .map_err(|e| Error::Network(e.to_string()))?;
            let current_size = metadata.len();

            if current_size != checkpoint.downloaded_bytes {
                // File size mismatch, start over
                let _ = fs::remove_file(&checkpoint.destination).await;
                let _ = fs::remove_file(checkpoint_path).await;
                return self.start_new_download(
                    &checkpoint.url,
                    &checkpoint.destination,
                    checkpoint_path,
                    options,
                ).await;
            }
        }

        // Create fetch options with Range header
        let mut resume_options = options.clone();
        let mut headers: Vec<_> = resume_options.headers.iter().cloned().collect();
        headers.push(("Range".to_string(), checkpoint.range_header()));
        resume_options.headers = Arc::from(headers);

        // Set up progress callback for resumed download
        let checkpoint_path_clone = checkpoint_path.to_path_buf();
        let checkpoint_dir = self.checkpoint_dir.clone();
        let initial_bytes = checkpoint.downloaded_bytes;
        let original_total_bytes = checkpoint.total_bytes;
        let checkpoint_url = checkpoint.url.clone();
        let checkpoint_destination = checkpoint.destination.clone();
        
        let original_callback = resume_options.on_progress.clone();
        resume_options.on_progress = Some(Arc::new(move |progress: &Progress| {
            // Update checkpoint with total progress (initial + new)
            if progress.phase == FetchPhase::Downloading {
                let total_downloaded = initial_bytes + progress.bytes_downloaded;
                
                // Create new checkpoint with updated progress
                let mut new_checkpoint = DownloadCheckpoint::new(
                    checkpoint_url.clone(),
                    checkpoint_destination.clone(),
                    original_total_bytes,
                );
                new_checkpoint.update_progress(total_downloaded);
                
                // Save checkpoint asynchronously
                let checkpoint_path = checkpoint_path_clone.clone();
                let checkpoint_dir = checkpoint_dir.clone();
                tokio::spawn(async move {
                    let _ = Self::save_checkpoint_static(&new_checkpoint, &checkpoint_path, &checkpoint_dir).await;
                });
            }
            
            // Call original callback if present
            if let Some(ref callback) = original_callback {
                callback(progress);
            }
        }));

        // Resume the download
        let result = self.base_fetcher.fetch(
            &checkpoint.url,
            &checkpoint.destination,
            resume_options,
        ).await;

        match result {
            Ok(path) => {
                // Download successful, remove checkpoint
                let _ = fs::remove_file(checkpoint_path).await;
                Ok(path)
            }
            Err(e) => {
                // Download failed, checkpoint remains
                Err(e)
            }
        }
    }

    /// Get the checkpoint file path for a download.
    fn checkpoint_path(&self, url: &str, destination: &Path) -> PathBuf {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Create a unique filename from URL and destination
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        destination.hash(&mut hasher);
        let hash = hasher.finish();
        
        self.checkpoint_dir.join(format!("checkpoint_{:016x}.json", hash))
    }

    /// Load a checkpoint from file.
    async fn load_checkpoint(&self, path: &Path) -> Result<DownloadCheckpoint> {
        let content = fs::read_to_string(path).await
            .map_err(|e| Error::Network(e.to_string()))?;
        
        serde_json::from_str(&content)
            .map_err(|e| Error::InvalidState(format!("Invalid checkpoint: {}", e)))
    }

    /// Save a checkpoint to file.
    async fn save_checkpoint(&self, checkpoint: &DownloadCheckpoint, path: &Path) -> Result<()> {
        Self::save_checkpoint_static(checkpoint, path, &self.checkpoint_dir).await
    }

    /// Static version of save_checkpoint for use in closures.
    async fn save_checkpoint_static(
        checkpoint: &DownloadCheckpoint,
        path: &Path,
        checkpoint_dir: &Path,
    ) -> Result<()> {
        // Ensure directory exists
        fs::create_dir_all(checkpoint_dir).await
            .map_err(|e| Error::Network(e.to_string()))?;

        // Serialize checkpoint
        let content = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| Error::InvalidState(format!("Failed to serialize checkpoint: {}", e)))?;

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        {
            let mut file: tokio::fs::File = fs::File::create(&temp_path).await
                .map_err(|e| Error::Network(e.to_string()))?;
            file.write_all(content.as_bytes()).await
                .map_err(|e| Error::Network(e.to_string()))?;
            file.sync_all().await
                .map_err(|e| Error::Network(e.to_string()))?;
        }

        // Atomic rename
        fs::rename(&temp_path, path).await
            .map_err(|e| Error::Network(e.to_string()))?;

        Ok(())
    }

    /// Clean up old checkpoints.
    pub async fn cleanup_old_checkpoints(&self, max_age_seconds: u64) -> Result<usize> {
        let mut cleaned = 0;
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - max_age_seconds;

        let mut entries = fs::read_dir(&self.checkpoint_dir).await
            .map_err(|e| Error::Network(e.to_string()))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::Network(e.to_string()))? {
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_checkpoint(&path).await {
                    Ok(checkpoint) => {
                        if checkpoint.last_update < cutoff {
                            let _ = fs::remove_file(&path).await;
                            cleaned += 1;
                        }
                    }
                    Err(_) => {
                        // Invalid checkpoint, remove it
                        let _ = fs::remove_file(&path).await;
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
    use bytes::Bytes;
    use crate::net::http::BoxStream;
    
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
        ) -> impl Future<Output = std::result::Result<BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error>>
               + Send {
            async move {
                let empty: BoxStream<'static, std::result::Result<Bytes, Self::Error>> = 
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
    fn test_download_checkpoint() {
        let mut checkpoint = DownloadCheckpoint::new(
            "https://example.com/file.txt".to_string(),
            PathBuf::from("/tmp/file.txt"),
            Some(1024),
        );

        assert_eq!(checkpoint.downloaded_bytes, 0);
        assert!(!checkpoint.can_resume());
        assert_eq!(checkpoint.range_header(), "bytes=0-");

        checkpoint.update_progress(512);
        assert_eq!(checkpoint.downloaded_bytes, 512);
        assert!(checkpoint.can_resume());
        assert_eq!(checkpoint.range_header(), "bytes=512-");
    }

    #[tokio::test]
    async fn test_checkpoint_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_path = temp_dir.path().join("checkpoint.json");
        
let mut original = DownloadCheckpoint::new(
            "https://example.com/file.txt".to_string(),
            PathBuf::from("/tmp/file.txt"),
            Some(1024),
        );

        assert_eq!(original.downloaded_bytes, 0);
        assert!(!original.can_resume());
        assert_eq!(original.range_header(), "bytes=0-");

        original.update_progress(512);

        // Save checkpoint
        let fetcher: ResumableFetcher<MockClient> = ResumableFetcher::new(
            MockClient::new(),
            temp_dir.path(),
        );
        fetcher.save_checkpoint(&original, &checkpoint_path).await.unwrap();

        // Load checkpoint
        let loaded: DownloadCheckpoint = fetcher.load_checkpoint(&checkpoint_path).await.unwrap();
        
        assert_eq!(loaded.url, original.url);
        assert_eq!(loaded.destination, original.destination);
        assert_eq!(loaded.total_bytes, original.total_bytes);
        assert_eq!(loaded.downloaded_bytes, original.downloaded_bytes);
    }

#[tokio::test]
    async fn test_cleanup_old_checkpoints() {
        let temp_dir = TempDir::new().unwrap();
        let fetcher = ResumableFetcher::<MockClient>::new(
            MockClient::new(),
            temp_dir.path(),
        );

        // Create some checkpoints with old timestamps
        let mut checkpoint1 = DownloadCheckpoint::new(
            "https://example.com/file1.txt".to_string(),
            PathBuf::from("/tmp/file1.txt"),
            Some(1024),
        );
        let mut checkpoint2 = DownloadCheckpoint::new(
            "https://example.com/file2.txt".to_string(),
            PathBuf::from("/tmp/file2.txt"),
            Some(1024),
        );
        
        // Manually set old timestamps (10 seconds ago)
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - 10;
        checkpoint1.last_update = old_timestamp;
        checkpoint2.last_update = old_timestamp;

        let path1 = fetcher.checkpoint_path("https://example.com/file1.txt", Path::new("/tmp/file1.txt"));
        let path2 = fetcher.checkpoint_path("https://example.com/file2.txt", Path::new("/tmp/file2.txt"));

        fetcher.save_checkpoint(&checkpoint1, &path1).await.unwrap();
        fetcher.save_checkpoint(&checkpoint2, &path2).await.unwrap();

        // Clean up with max age of 5 seconds (should clean up 10-second-old checkpoints)
        let cleaned = fetcher.cleanup_old_checkpoints(5).await.unwrap();
        assert_eq!(cleaned, 2);

        // Checkpoints should be gone
        assert!(!path1.exists());
        assert!(!path2.exists());
    }
}