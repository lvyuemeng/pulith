use std::fmt;

use crate::data::options::FetchPhase;

/// Represents the current state of a download operation.
///
/// This struct is passed to progress callbacks and provides information about
/// the current phase, bytes downloaded, and retry status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// Current phase of the download.
    pub phase: FetchPhase,

    /// Number of bytes written to the staging file.
    pub bytes_downloaded: u64,

    /// Total expected bytes, if known from Content-Length header.
    ///
    /// This may be `None` if the server doesn't provide Content-Length
    /// (e.g., when using chunked transfer encoding).
    pub total_bytes: Option<u64>,

    /// Current retry attempt (0 = no retries yet, first attempt).
    pub retry_count: u32,
}

impl Progress {
    /// Calculate the percentage of completion.
    ///
    /// Returns `None` if `total_bytes` is unknown.
    #[must_use]
    pub fn percentage(&self) -> Option<f64> {
        self.total_bytes.map(|total| {
            if total == 0 {
                // For empty files, report 100% when completed, 0% otherwise
                if self.is_completed() {
                    100.0
                } else {
                    0.0
                }
            } else {
                (self.bytes_downloaded as f64 / total as f64) * 100.0
            }
        })
    }

    /// Returns `true` if the download has completed successfully.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.phase == FetchPhase::Completed
    }

    /// Returns `true` if a retry is in progress.
    #[must_use]
    pub fn is_retrying(&self) -> bool {
        self.retry_count > 0
    }
}
