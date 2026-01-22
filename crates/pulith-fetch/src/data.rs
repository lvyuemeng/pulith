use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct FetchOptions {
    pub checksum: Option<Vec<u8>>,
    pub max_retries: u32,
    pub retry_backoff: Duration,
    pub timeouts: Timeouts,
    pub headers: HashMap<String, String>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub on_progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            checksum: None,
            max_retries: 3,
            retry_backoff: Duration::from_millis(100),
            timeouts: Timeouts::default(),
            headers: HashMap::new(),
            follow_redirects: true,
            max_redirects: 10,
            on_progress: None,
        }
    }
}

impl FetchOptions {
    pub fn checksum(mut self, checksum: Option<Vec<u8>>) -> Self {
        self.checksum = checksum;
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn retry_backoff(mut self, retry_backoff: Duration) -> Self {
        self.retry_backoff = retry_backoff;
        self
    }

    pub fn timeouts(mut self, timeouts: Timeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn follow_redirects(mut self, follow_redirects: bool) -> Self {
        self.follow_redirects = follow_redirects;
        self
    }

    pub fn max_redirects(mut self, max_redirects: u32) -> Self {
        self.max_redirects = max_redirects;
        self
    }

    pub fn on_progress(mut self, on_progress: Arc<dyn Fn(Progress) + Send + Sync>) -> Self {
        self.on_progress = Some(on_progress);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeouts {
    pub connect: Duration,
    pub read: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(30),
            read: Duration::from_secs(300),
        }
    }
}

impl Timeouts {
    pub fn connect(mut self, connect: Duration) -> Self {
        self.connect = connect;
        self
    }

    pub fn read(mut self, read: Duration) -> Self {
        self.read = read;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchPhase {
    Connecting,
    Downloading,
    Verifying,
    Committing,
    Completed,
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub phase: FetchPhase,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub retry_count: u32,
}

impl Progress {
    pub fn percentage(&self) -> Option<f32> {
        self.total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (self.bytes_downloaded as f32 / total as f32) * 100.0
            }
        })
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.phase, FetchPhase::Completed)
    }
}
