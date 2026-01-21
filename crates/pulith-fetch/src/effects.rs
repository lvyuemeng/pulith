//! Effect layer: I/O operations with trait abstraction.

use std::path::{Path, PathBuf};

use sha2::Digest;
use sha2::Sha256;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::core::verify_checksum;
use crate::data::{DownloadOptions, ProgressTracker};
use crate::error::FetchError;

pub struct Downloader<F, N>
where
    F: pulith_core::fs::AsyncFileSystem,
    N: pulith_core::fs::Network,
{
    fs:      F,
    network: N,
    options: DownloadOptions,
}

impl<F, N> Downloader<F, N>
where
    F: pulith_core::fs::AsyncFileSystem,
    N: pulith_core::fs::Network,
{
    pub fn new(fs: F, network: N, options: DownloadOptions) -> Self {
        Self {
            fs,
            network,
            options,
        }
    }

    pub async fn download_to_temp(&self) -> Result<PathBuf, FetchError> {
        let temp_dir = std::env::temp_dir();
        let filename = format!("pulith_fetch_{}", std::process::id());
        let temp_path = temp_dir.join(filename);

        self.download_to_path(&temp_path).await?;
        Ok(temp_path)
    }

    pub async fn download_to(&self, path: &Path) -> Result<(), FetchError> {
        self.download_to_path(path).await.map(|_| ())
    }

    pub async fn download_to_staging(
        &self,
        staging_dir: &Path,
        filename: &str,
    ) -> Result<PathBuf, FetchError> {
        if staging_dir.is_file() {
            return Err(FetchError::DestinationIsDirectory);
        }

        if let Some(parent) = staging_dir.parent() {
            if !parent.exists() {
                self.fs.create_dir_all(parent).await?;
            }
        }

        let path = staging_dir.join(filename);
        self.download_to_path(&path).await?;
        Ok(path)
    }

    async fn download_to_path(&self, path: &Path) -> Result<PathBuf, FetchError> {
        if path.is_dir() {
            return Err(FetchError::DestinationIsDirectory);
        }

        let mut attempt = 0u32;

        while attempt < self.options.max_retries {
            attempt += 1;

            match self.do_download(path, attempt).await {
                Ok(()) => {
                    return Ok(path.to_path_buf());
                }
                Err(_) if attempt < self.options.max_retries => {
                    let delay = self.options.retry_backoff * attempt;
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Err(FetchError::MaxRetriesExceeded {
            count: self.options.max_retries,
        })
    }

    async fn do_download(&self, path: &Path, attempt: u32) -> Result<(), FetchError> {
        let bytes = self.network.get(&self.options.url).await?;

        let total_size = Some(bytes.len() as u64);
        let mut progress = ProgressTracker::new(self.options.on_progress, total_size);
        progress.set_retry_count(attempt.saturating_sub(1));

        progress.set_downloading();

        let mut file = BufWriter::new(self.fs.create(path).await?);

        let mut hasher = Sha256::new();
        hasher.update(&bytes);

        file.write_all(&bytes).await?;
        file.flush().await?;

        progress.set_verifying();

        if let Some(expected) = &self.options.checksum {
            verify_checksum(&bytes, expected).map_err(|_| FetchError::ChecksumMismatch {
                expected: expected.as_str().to_string(),
                actual:   format!("{:x}", hasher.finalize()),
            })?;
        }

        progress.set_completed();

        Ok(())
    }
}
