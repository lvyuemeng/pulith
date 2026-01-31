use std::fmt;

use crate::data::options::FetchPhase;

/// Represents the current state of a download operation.
///
/// This struct is passed to progress callbacks and provides information about
/// the current phase, bytes downloaded, and retry status.
#[derive(Debug, Clone, PartialEq)]
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

    /// Performance metrics for this operation.
    pub performance_metrics: Option<PerformanceMetrics>,
}

/// Performance metrics for download operations.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct PerformanceMetrics {
    /// Current download rate in bytes per second
    pub current_rate_bps: Option<f64>,

    /// Average download rate since start in bytes per second
    pub average_rate_bps: Option<f64>,

    /// Current bandwidth limit in bytes per second (if throttled)
    pub bandwidth_limit_bps: Option<u64>,

    /// Bandwidth utilization as a percentage (0.0 to 1.0)
    pub bandwidth_utilization: Option<f64>,

    /// Time spent in each phase (in milliseconds)
    pub phase_timings: PhaseTimings,

    /// Number of times rate was adjusted by adaptive algorithm
    pub rate_adjustments: u32,

    /// Network latency in milliseconds
    pub network_latency_ms: Option<u64>,

    /// Time to establish connection in milliseconds
    pub connection_time_ms: Option<u64>,
}

/// Timing information for different phases of a download operation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PhaseTimings {
    /// Time spent connecting to the server (in milliseconds)
    pub connecting_ms: u64,

    /// Time spent downloading data (in milliseconds)
    pub downloading_ms: u64,

    /// Time spent verifying checksums (in milliseconds)
    pub verifying_ms: u64,

    /// Time spent committing the final file (in milliseconds)
    pub committing_ms: u64,
}

impl PhaseTimings {
    /// Returns the total time spent across all phases (in milliseconds).
    #[must_use]
    pub fn total_ms(&self) -> u64 {
        self.connecting_ms + self.downloading_ms + self.verifying_ms + self.committing_ms
    }
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

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.percentage() {
            Some(pct) => write!(
                f,
                "{}: {:.1}% ({}/{} bytes, retry {})",
                self.phase,
                pct,
                self.bytes_downloaded,
                self.total_bytes.unwrap_or(0),
                self.retry_count
            ),
            None => write!(
                f,
                "{}: {}/{} bytes (retry {})",
                self.phase,
                self.bytes_downloaded,
                self.total_bytes.unwrap_or(0),
                self.retry_count
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_percentage() {
        let progress = Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 50,
            total_bytes: Some(100),
            retry_count: 0,
            performance_metrics: None,
        };
        assert_eq!(progress.percentage(), Some(50.0));

        // Test with zero total bytes
        let progress = Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 0,
            total_bytes: Some(0),
            retry_count: 0,
            performance_metrics: None,
        };
        assert_eq!(progress.percentage(), Some(0.0));

        // Test completed empty file
        let progress = Progress {
            phase: FetchPhase::Completed,
            bytes_downloaded: 0,
            total_bytes: Some(0),
            retry_count: 0,
            performance_metrics: None,
        };
        assert_eq!(progress.percentage(), Some(100.0));

        // Test with unknown total
        let progress = Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 50,
            total_bytes: None,
            retry_count: 0,
            performance_metrics: None,
        };
        assert_eq!(progress.percentage(), None);
    }

    #[test]
    fn test_is_completed() {
        let progress = Progress {
            phase: FetchPhase::Completed,
            bytes_downloaded: 100,
            total_bytes: Some(100),
            retry_count: 0,
            performance_metrics: None,
        };
        assert!(progress.is_completed());

        let progress = Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 100,
            total_bytes: Some(100),
            retry_count: 0,
            performance_metrics: None,
        };
        assert!(!progress.is_completed());
    }

    #[test]
    fn test_is_retrying() {
        let progress = Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 50,
            total_bytes: Some(100),
            retry_count: 1,
            performance_metrics: None,
        };
        assert!(progress.is_retrying());

        let progress = Progress {
            phase: FetchPhase::Downloading,
            bytes_downloaded: 50,
            total_bytes: Some(100),
            retry_count: 0,
            performance_metrics: None,
        };
        assert!(!progress.is_retrying());
    }

    #[test]
    fn test_performance_metrics_default() {
        let metrics = PerformanceMetrics::default();
        assert!(metrics.current_rate_bps.is_none());
        assert!(metrics.average_rate_bps.is_none());
        assert!(metrics.bandwidth_limit_bps.is_none());
        assert!(metrics.bandwidth_utilization.is_none());
        assert_eq!(metrics.rate_adjustments, 0);
        assert!(metrics.network_latency_ms.is_none());
        assert!(metrics.connection_time_ms.is_none());
    }
}
