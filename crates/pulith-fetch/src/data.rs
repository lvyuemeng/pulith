//! Data layer: immutable types for download configuration and progress tracking.

use std::time::Duration;

use crate::ParseSha256HashError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Hash(pub String);

impl std::str::FromStr for Sha256Hash {
    type Err = ParseSha256HashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() != 64 {
            return Err(ParseSha256HashError(s.to_string()));
        }
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ParseSha256HashError(s.to_string()));
        }
        Ok(Self(s.to_string()))
    }
}

impl Sha256Hash {
    pub fn from_hex(s: &str) -> Result<Self, ParseSha256HashError> { s.parse() }

    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub url:             String,
    pub checksum:        Option<Sha256Hash>,
    pub max_retries:     u32,
    pub retry_backoff:   Duration,
    pub connect_timeout: Duration,
    pub read_timeout:    Duration,
    pub on_progress:     ProgressCallback,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            url:             String::new(),
            checksum:        None,
            max_retries:     3,
            retry_backoff:   Duration::from_millis(100),
            connect_timeout: Duration::from_secs(30),
            read_timeout:    Duration::from_secs(30),
            on_progress:     noop_progress,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPhase {
    Connecting,
    Downloading,
    Verifying,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub bytes_downloaded: u64,
    pub total_bytes:      Option<u64>,
    pub phase:            DownloadPhase,
    pub retry_count:      u32,
}

impl Progress {
    pub fn new(
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
        phase: DownloadPhase,
        retry_count: u32,
    ) -> Self {
        Self {
            bytes_downloaded,
            total_bytes,
            phase,
            retry_count,
        }
    }

    pub fn percentage(&self) -> Option<f32> {
        self.total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (self.bytes_downloaded as f32 / total as f32) * 100.0
            }
        })
    }
}

pub type ProgressCallback = fn(Progress);

pub fn noop_progress(_: Progress) {}

pub struct ProgressTracker {
    callback:         ProgressCallback,
    total_bytes:      Option<u64>,
    bytes_downloaded: u64,
    retry_count:      u32,
}

impl ProgressTracker {
    pub fn new(callback: ProgressCallback, total_bytes: Option<u64>) -> Self {
        let tracker = Self {
            callback,
            total_bytes,
            bytes_downloaded: 0,
            retry_count: 0,
        };
        tracker.emit(DownloadPhase::Connecting);
        tracker
    }

    pub fn set_retry_count(&mut self, count: u32) { self.retry_count = count; }

    pub fn add_bytes(&mut self, bytes: u64) {
        self.bytes_downloaded += bytes;
        self.emit(DownloadPhase::Downloading);
    }

    pub fn set_downloading(&mut self) { self.emit(DownloadPhase::Downloading); }

    pub fn set_verifying(&mut self) { self.emit(DownloadPhase::Verifying); }

    pub fn set_completed(&mut self) { self.emit(DownloadPhase::Completed); }

    fn emit(&self, phase: DownloadPhase) {
        (self.callback)(Progress::new(
            self.bytes_downloaded,
            self.total_bytes,
            phase,
            self.retry_count,
        ));
    }
}
