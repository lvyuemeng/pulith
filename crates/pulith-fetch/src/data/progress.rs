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

    /// Network latency measurements (in milliseconds)
    pub network_latency_ms: Option<f64>,

    /// Connection time (time to first byte, in milliseconds)
    pub connection_time_ms: Option<u64>,
}

/// Timing information for each download phase.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PhaseTimings {
    /// Time spent connecting (ms)
    pub connecting_ms: u64,

    /// Time spent downloading (ms)
    pub downloading_ms: u64,

    /// Time spent verifying (ms)
    pub verifying_ms: u64,

    /// Time spent committing (ms)
    pub committing_ms: u64,
}

impl PhaseTimings {
    /// Get total time across all phases.
    pub fn total_ms(&self) -> u64 {
        self.connecting_ms + self.downloading_ms + self.verifying_ms + self.committing_ms
    }

    /// Get timing for a specific phase.
    pub fn get_phase_timing(&self, phase: &FetchPhase) -> u64 {
        match phase {
            FetchPhase::Connecting => self.connecting_ms,
            FetchPhase::Downloading => self.downloading_ms,
            FetchPhase::Verifying => self.verifying_ms,
            FetchPhase::Committing => self.committing_ms,
            FetchPhase::Completed => self.total_ms(),
        }
    }

    /// Set timing for a specific phase.
    pub fn set_phase_timing(&mut self, phase: &FetchPhase, duration_ms: u64) {
        match phase {
            FetchPhase::Connecting => self.connecting_ms = duration_ms,
            FetchPhase::Downloading => self.downloading_ms = duration_ms,
            FetchPhase::Verifying => self.verifying_ms = duration_ms,
            FetchPhase::Committing => self.committing_ms = duration_ms,
            FetchPhase::Completed => {} // No-op for completed
        }
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            current_rate_bps: None,
            average_rate_bps: None,
            bandwidth_limit_bps: None,
            bandwidth_utilization: None,
            phase_timings: PhaseTimings::default(),
            rate_adjustments: 0,
            network_latency_ms: None,
            connection_time_ms: None,
        }
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
