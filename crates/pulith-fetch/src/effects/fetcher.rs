use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::StreamExt;
use pulith_fs::workflow::Workspace;
use pulith_verify::{Hasher, Sha256Hasher};

use crate::data::{FetchOptions, FetchPhase, Progress};
use crate::error::{Error, Result};
use crate::effects::http::HttpClient;

/// The main fetcher implementation that handles downloading files with verification.
pub struct Fetcher<C: HttpClient> {
    client: C,
    workspace_root: PathBuf,
}

impl<C: HttpClient> Fetcher<C> {
    /// Create a new fetcher with the provided HTTP client and workspace root.
    pub fn new(client: C, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            client,
            workspace_root: workspace_root.into(),
        }
    }

    /// Fetch a file from the given URL and save it to the destination.
    ///
    /// This function downloads the file with progress reporting, verification,
    /// and atomic placement using pulith-fs workspace.
    pub async fn fetch(
        &self,
        url: &str,
        destination: &Path,
        options: FetchOptions,
    ) -> Result<PathBuf> {
        self.report_progress(&options, Progress {
            phase: FetchPhase::Connecting,
            bytes_downloaded: 0,
            total_bytes: None,
            retry_count: 0,
        });

        let total_bytes = self.client.head(url).await.map_err(|e| Error::Network(e.to_string()))?;
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Connecting,
            bytes_downloaded: 0,
            total_bytes,
            retry_count: 0,
        });

        let staging_dir = self.workspace_root.join("staging");
        let workspace = Workspace::new(&staging_dir, destination.parent().unwrap_or_else(|| Path::new(".")))?;
        let staging_file_path = workspace.path().join(destination.file_name().unwrap_or_else(|| std::ffi::OsStr::new("download")));
        let mut stream = self.client.stream(url, &options.headers).await.map_err(|e| Error::Network(e.to_string()))?;
        let mut hasher = Sha256Hasher::new();
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 0,
            total_bytes,
            retry_count: 0,
        });
        
        let mut bytes_downloaded = 0u64;
        use tokio::io::AsyncWriteExt;
        let mut file: tokio::fs::File = tokio::fs::File::create(&staging_file_path).await.map_err(|e| Error::Network(e.to_string()))?;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| Error::Network(e.to_string()))?;
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| Error::Network(e.to_string()))?;
            bytes_downloaded += chunk.len() as u64;
            
            self.report_progress(&options, Progress {
                phase: FetchPhase::Downloading,
                bytes_downloaded,
                total_bytes,
                retry_count: 0,
            });
        }
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Verifying,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
        });
        
        if let Some(expected_checksum) = options.checksum {
            let actual_checksum = hasher.finalize();
            if actual_checksum != expected_checksum {
                return Err(Error::ChecksumMismatch {
                    expected: hex::encode(expected_checksum),
                    actual: hex::encode(actual_checksum),
                });
            }
        }
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Committing,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
        });
        
        // Move the file to the final destination
        tokio::fs::rename(&staging_file_path, destination).await.map_err(|e| Error::Network(e.to_string()))?;
        workspace.commit().map_err(|e| Error::Network(e.to_string()))?;
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Completed,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
        });
        
        Ok(destination.to_path_buf())
    }
    
    /// Report progress if callback is configured.
    fn report_progress(&self, options: &FetchOptions, progress: Progress) {
        if let Some(ref callback) = options.on_progress {
            callback(&progress);
        }
    }
}