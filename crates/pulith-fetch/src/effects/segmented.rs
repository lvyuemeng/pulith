//! Segmented download functionality.
//!
//! This module provides the ability to download files in parallel
//! segments for improved performance.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::StreamExt;
use pulith_fs::workflow::Workspace;
use pulith_verify::{Hasher, Sha256Hasher};
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::Semaphore;
use futures_util::stream::FuturesUnordered;

use crate::core::{Segment, calculate_segments};
use crate::data::{FetchOptions, FetchPhase, Progress};
use crate::error::{Error, Result};
use crate::effects::http::HttpClient;

/// Configuration for segmented downloads.
#[derive(Debug, Clone)]
pub struct SegmentedOptions {
    /// Number of segments to download in parallel
    pub num_segments: u32,
    /// Maximum concurrent downloads
    pub max_concurrent: usize,
}

impl Default for SegmentedOptions {
    fn default() -> Self {
        Self {
            num_segments: 4,
            max_concurrent: 4,
        }
    }
}

/// Segmented fetcher implementation.
pub struct SegmentedFetcher<C: HttpClient> {
    client: Arc<C>,
    workspace_root: PathBuf,
}

impl<C: HttpClient + 'static> SegmentedFetcher<C> {
    /// Create a new segmented fetcher.
    pub fn new(client: C, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            client: Arc::new(client),
            workspace_root: workspace_root.into(),
        }
    }

    /// Fetch a file using segmented downloads.
    pub async fn fetch_segmented(
        &self,
        url: &str,
        destination: &Path,
        options: SegmentedOptions,
        fetch_options: FetchOptions,
    ) -> Result<PathBuf> {
        // Get file size first
        let total_bytes = self.client.head(url).await.map_err(|e| Error::Network(e.to_string()))?;
        
        // Calculate segments
        let segments = calculate_segments(total_bytes.unwrap_or(0), options.num_segments)?;
        
        // Create workspace for staging
        let staging_dir = self.workspace_root.join("staging");
        let workspace = Workspace::new(&staging_dir, destination.parent().unwrap_or_else(|| Path::new(".")))?;
        
        // Download segments in parallel
        let segment_files = self.download_segments(url, &segments, &workspace, &fetch_options, options.max_concurrent).await?;
        
        // Reassemble segments and commit workspace
        self.reassemble_segments(&segment_files, destination, workspace, &fetch_options, total_bytes).await?;
        
        Ok(destination.to_path_buf())
    }

    /// Download all segments in parallel.
    async fn download_segments(
        &self,
        url: &str,
        segments: &[Segment],
        workspace: &Workspace,
        options: &FetchOptions,
        max_concurrent: usize,
    ) -> Result<Vec<PathBuf>> {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut futures = FuturesUnordered::new();
        
        for segment in segments {
            let permit = semaphore.clone().acquire_owned().await.map_err(|e| Error::Network(e.to_string()))?;
            let client = self.client.clone();
            let url = url.to_string();
            let workspace_path = workspace.path().to_path_buf();
            let segment_clone = segment.clone();
            let options_clone = options.clone();
            
            let future = tokio::spawn(async move {
                let _permit = permit;
                let segment_path = workspace_path.join(format!("segment_{}", segment_clone.index));
                
                // Create Range header for this segment
                let range_header = format!("bytes={}-{}", segment_clone.start, segment_clone.end - 1);
                let mut segment_options = options_clone;
                let mut headers: Vec<_> = segment_options.headers.iter().cloned().collect();
                headers.push(("Range".to_string(), range_header));
                segment_options.headers = Arc::from(headers);
                
                // Download the segment
                let mut stream = client.stream(&url, &segment_options.headers).await.map_err(|e| Error::Network(e.to_string()))?;
                let mut file = tokio::fs::File::create(&segment_path).await.map_err(|e| Error::Network(e.to_string()))?;
                
                while let Some(chunk_result) = stream.next().await {
                    let chunk = chunk_result.map_err(|e| Error::Network(e.to_string()))?;
                    file.write_all(&chunk).await.map_err(|e| Error::Network(e.to_string()))?;
                }
                
                Ok::<PathBuf, Error>(segment_path)
            });
            
            futures.push(future);
        }
        
        // Wait for all downloads to complete
        let mut segment_files = Vec::with_capacity(segments.len());
        while let Some(result) = futures.next().await {
            match result {
                Ok(segment_result) => match segment_result {
                    Ok(path) => segment_files.push(path),
                    Err(e) => return Err(e),
                },
                Err(e) => return Err(Error::Network(e.to_string())),
            }
        }
        
        // Sort by segment index to ensure correct order
        segment_files.sort_by_key(|path| {
            let filename = path.file_name().unwrap().to_str().unwrap();
            filename.split('_').last().unwrap().parse::<u32>().unwrap()
        });
        
        Ok(segment_files)
    }

    /// Reassemble segments into the final file.
    async fn reassemble_segments(
        &self,
        segment_files: &[PathBuf],
        destination: &Path,
        workspace: Workspace,
        options: &FetchOptions,
        total_bytes: Option<u64>,
    ) -> Result<()> {
        let staging_file_path = workspace.path().join(destination.file_name().unwrap_or_else(|| std::ffi::OsStr::new("download")));
        let mut output_file = tokio::fs::File::create(&staging_file_path).await.map_err(|e| Error::Network(e.to_string()))?;
        let mut hasher = Sha256Hasher::new();
        let mut bytes_downloaded = 0u64;
        
        // Report initial progress
        self.report_progress(options, Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 0,
            total_bytes,
            retry_count: 0,
            performance_metrics: None,
        });
        
        // Copy segments in order
        for segment_path in segment_files {
            let mut segment_file = tokio::fs::File::open(segment_path).await.map_err(|e| Error::Network(e.to_string()))?;
            
            let mut buffer = vec![0u8; 8192];
            loop {
                let n = segment_file.read(&mut buffer).await.map_err(|e| Error::Network(e.to_string()))?;
                if n == 0 {
                    break;
                }
                
                hasher.update(&buffer[..n]);
                output_file.write_all(&buffer[..n]).await.map_err(|e| Error::Network(e.to_string()))?;
                bytes_downloaded += n as u64;
                
                // Report progress
                self.report_progress(options, Progress {
                    phase: FetchPhase::Downloading,
                    bytes_downloaded,
                    total_bytes,
                    retry_count: 0,
                    performance_metrics: None,
                });
            }
            
            // Clean up segment file
            tokio::fs::remove_file(segment_path).await.map_err(|e| Error::Network(e.to_string()))?;
        }
        
        // Verify checksum if provided
        if let Some(expected_checksum) = options.checksum {
            self.report_progress(options, Progress {
                phase: FetchPhase::Verifying,
                bytes_downloaded,
                total_bytes,
                retry_count: 0,
                performance_metrics: None,
            });
            
            let actual_checksum = hasher.finalize();
            if actual_checksum != expected_checksum {
                return Err(Error::ChecksumMismatch {
                    expected: hex::encode(expected_checksum),
                    actual: hex::encode(actual_checksum),
                });
            }
        }
        
        // Move to final destination
self.report_progress(options, Progress {
            phase: FetchPhase::Committing,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
            performance_metrics: None,
        });
        
        // Move the file to the final destination
        tokio::fs::rename(&staging_file_path, destination).await.map_err(|e| Error::Network(e.to_string()))?;
        workspace.commit().map_err(|e| Error::Network(e.to_string()))?;
        
        self.report_progress(options, Progress {
            phase: FetchPhase::Completed,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
            performance_metrics: None,
        });
        
        tokio::fs::rename(&staging_file_path, destination).await.map_err(|e| Error::Network(e.to_string()))?;
        
        self.report_progress(options, Progress {
            phase: FetchPhase::Completed,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
            performance_metrics: None,
        });
        
        Ok(())
    }

    /// Report progress if callback is configured.
    fn report_progress(&self, options: &FetchOptions, progress: Progress) {
        if let Some(ref callback) = options.on_progress {
            callback(&progress);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::calculate_segments;

    #[test]
    fn test_segment_calculation() {
        // Test basic segment calculation
        let segments = calculate_segments(100, 4).unwrap();
        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 25);
        assert_eq!(segments[3].start, 75);
        assert_eq!(segments[3].end, 100);
        
        // Test with remainder
        let segments = calculate_segments(10, 3).unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].end, 4); // First segment gets extra byte
        assert_eq!(segments[1].end, 7); // Second segment gets extra byte
        assert_eq!(segments[2].end, 10);
        
        // Test zero file size
        let segments = calculate_segments(0, 4).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start, 0);
        assert_eq!(segments[0].end, 0);
    }

    #[test]
    fn test_segment_calculation_errors() {
        // Test zero segments
        let result = calculate_segments(100, 0);
        assert!(result.is_err());
    }
}