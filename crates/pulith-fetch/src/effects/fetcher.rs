use std::path::{Path, PathBuf};

use bytes::Bytes;
use futures_util::StreamExt;
use pulith_fs::workflow::Workspace;
use pulith_verify::{Hasher, Sha256Hasher};

use crate::data::{FetchOptions, FetchPhase, Progress};
use crate::data::progress::{PerformanceMetrics, PhaseTimings};
use crate::error::{Error, Result};
use crate::effects::http::HttpClient;

/// The main fetcher implementation that handles downloading files with verification.
pub struct Fetcher<C: HttpClient> {
    pub(crate) client: C,
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

    /// Get the total bytes from a HEAD request.
    pub async fn head(&self, url: &str) -> Result<Option<u64>> {
        self.client.head(url).await.map_err(|e| Error::Network(e.to_string()))
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
        let start_time = std::time::Instant::now();
        let mut performance_metrics = PerformanceMetrics::default();
        
        // Connecting phase
        let connecting_start = std::time::Instant::now();
        self.report_progress(&options, Progress {
            phase: FetchPhase::Connecting,
            bytes_downloaded: 0,
            total_bytes: None,
            retry_count: 0,
            performance_metrics: Some(performance_metrics.clone()),
        });

        let total_bytes = self.client.head(url).await.map_err(|e| Error::Network(e.to_string()))?;
        let connecting_duration = connecting_start.elapsed();
        performance_metrics.phase_timings.connecting_ms = connecting_duration.as_millis() as u64;
        performance_metrics.connection_time_ms = Some(connecting_duration.as_millis() as u64);
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Connecting,
            bytes_downloaded: 0,
            total_bytes,
            retry_count: 0,
            performance_metrics: Some(performance_metrics.clone()),
        });

        let staging_dir = self.workspace_root.join("staging");
        let dest_dir = destination.parent().unwrap_or_else(|| Path::new("."));
        let workspace = Workspace::new(&staging_dir, dest_dir)?;
        let staging_file_path = workspace.path().join(destination.file_name().unwrap_or_else(|| std::ffi::OsStr::new("download")));
        let mut stream = self.client.stream(url, &options.headers).await.map_err(|e| Error::Network(e.to_string()))?;
        let mut hasher = Sha256Hasher::new();
        
        // Downloading phase
        let downloading_start = std::time::Instant::now();
        self.report_progress(&options, Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 0,
            total_bytes,
            retry_count: 0,
            performance_metrics: Some(performance_metrics.clone()),
        });
        
        let mut bytes_downloaded = 0u64;
        let mut last_progress_time = std::time::Instant::now();
        let mut last_bytes_downloaded = 0u64;
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::File::create(&staging_file_path).await.map_err(|e| Error::Network(e.to_string()))?;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| Error::Network(e.to_string()))?;
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| Error::Network(e.to_string()))?;
            bytes_downloaded += chunk.len() as u64;
            
            // Calculate current rate every 100ms
            let now = std::time::Instant::now();
            if now.duration_since(last_progress_time).as_millis() >= 100 {
                let time_diff = now.duration_since(last_progress_time).as_secs_f64();
                let bytes_diff = bytes_downloaded - last_bytes_downloaded;
                if time_diff > 0.0 {
                    performance_metrics.current_rate_bps = Some(bytes_diff as f64 / time_diff);
                }
                last_progress_time = now;
                last_bytes_downloaded = bytes_downloaded;
            }
            
            // Calculate average rate
            let total_time = start_time.elapsed().as_secs_f64();
            if total_time > 0.0 {
                performance_metrics.average_rate_bps = Some(bytes_downloaded as f64 / total_time);
            }
            
            self.report_progress(&options, Progress {
                phase: FetchPhase::Downloading,
                bytes_downloaded,
                total_bytes,
                retry_count: 0,
                performance_metrics: Some(performance_metrics.clone()),
            });
        }
        
        let downloading_duration = downloading_start.elapsed();
        performance_metrics.phase_timings.downloading_ms = downloading_duration.as_millis() as u64;
        
        // Verifying phase
        let verifying_start = std::time::Instant::now();
        self.report_progress(&options, Progress {
            phase: FetchPhase::Verifying,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
            performance_metrics: Some(performance_metrics.clone()),
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
        
        let verifying_duration = verifying_start.elapsed();
        performance_metrics.phase_timings.verifying_ms = verifying_duration.as_millis() as u64;
        
        // Committing phase
        let committing_start = std::time::Instant::now();
        self.report_progress(&options, Progress {
            phase: FetchPhase::Committing,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
            performance_metrics: Some(performance_metrics.clone()),
        });
        
        // Move the file to the final destination
        tokio::fs::rename(&staging_file_path, destination).await.map_err(|e| Error::Network(e.to_string()))?;
        workspace.commit().map_err(|e| Error::Network(e.to_string()))?;
        
        let committing_duration = committing_start.elapsed();
        performance_metrics.phase_timings.committing_ms = committing_duration.as_millis() as u64;
        
        self.report_progress(&options, Progress {
            phase: FetchPhase::Completed,
            bytes_downloaded,
            total_bytes,
            retry_count: 0,
            performance_metrics: Some(performance_metrics),
        });
        
        Ok(destination.to_path_buf())
    }
    
    /// Report progress if callback is configured.
    fn report_progress(&self, options: &FetchOptions, progress: Progress) {
        if let Some(ref callback) = options.on_progress {
            callback(&progress);
        }
    }
    
    /// Try to fetch from a single source with verification.
    pub async fn try_source(
        &self,
        source: &crate::data::DownloadSource,
        destination: &Path,
        options: &FetchOptions,
    ) -> Result<PathBuf> {
        // Create fetch options for this source
        let mut fetch_options = options.clone();
        fetch_options.checksum = source.checksum;

        // Fetch using the base fetcher
        self.fetch(&source.url, destination, fetch_options).await
    }
}