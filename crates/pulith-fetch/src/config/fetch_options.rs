use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::progress::Progress;

pub type ProgressCallback = Arc<dyn Fn(&Progress) + Send + Sync>;

/// Explicit retry behavior for transient transfer failures.
///
/// Total attempts are `1 + max_retries`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Number of retries after the initial attempt.
    pub max_retries: u32,
    /// Base exponential backoff duration.
    pub base_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_backoff: Duration::from_millis(100),
        }
    }
}

/// Phases of a download operation.
///
/// Downloads progress through these phases in order:
/// Connecting → Downloading → Verifying → Committing → Completed
///
/// Retries return to the Connecting phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FetchPhase {
    /// Initial state, connection in progress.
    ///
    /// This phase is active while establishing the HTTP connection
    /// and waiting for the first response bytes.
    #[default]
    Connecting,

    /// Actively streaming data to disk.
    ///
    /// This phase is active while downloading chunks from the server
    /// and writing them to the staging file.
    Downloading,

    /// Computing and verifying checksum.
    ///
    /// This phase is active after all bytes are downloaded and the
    /// checksum is being finalized and compared (if configured).
    Verifying,

    /// Performing atomic commit of the downloaded file.
    ///
    /// This phase is active while moving the staging file to its
    /// final destination path.
    Committing,

    /// Download completed successfully.
    ///
    /// This is the terminal state for successful downloads.
    Completed,
}

impl std::fmt::Display for FetchPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchPhase::Connecting => write!(f, "Connecting"),
            FetchPhase::Downloading => write!(f, "Downloading"),
            FetchPhase::Verifying => write!(f, "Verifying"),
            FetchPhase::Committing => write!(f, "Committing"),
            FetchPhase::Completed => write!(f, "Completed"),
        }
    }
}

/// Configuration for HTTP fetching operations.
///
/// # Examples
///
/// ```
/// use pulith_fetch::FetchOptions;
/// use std::time::Duration;
///
/// let options = FetchOptions::default()
///     .max_retries(5)
///     .retry_backoff(Duration::from_millis(200))
///     .header("Authorization", "Bearer token");
/// ```
#[derive(Clone)]
pub struct FetchOptions {
    /// Expected SHA-256 checksum for verification (optional).
    /// If provided, the download will be verified and will fail on mismatch.
    pub checksum: Option<[u8; 32]>,

    /// Retry execution policy for transient transfer failures.
    pub retry_policy: RetryPolicy,

    /// Expected total bytes for this transfer, when known by caller.
    pub expected_bytes: Option<u64>,

    /// Resume offset in bytes. When set, fetcher will request `Range: bytes=<offset>-`.
    pub resume_offset: Option<u64>,

    /// Custom HTTP headers to include with requests.
    ///
    /// Headers are sent with every request, including retries.
    ///
    /// Default: empty
    pub headers: Arc<[(String, String)]>,

    /// Progress callback invoked on state transitions and chunk writes.
    ///
    /// The callback is invoked:
    /// - On phase transitions (Connecting → Downloading → Verifying → Committing → Completed)
    /// - After each chunk write (during Downloading phase, typically every ~8KB)
    /// - After each retry attempt (back to Connecting phase)
    ///
    /// The callback receives a reference to avoid cloning on every invocation.
    ///
    /// Default: None
    pub on_progress: Option<ProgressCallback>,
}

impl fmt::Debug for FetchOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FetchOptions")
            .field("checksum", &self.checksum)
            .field("retry_policy", &self.retry_policy)
            .field("expected_bytes", &self.expected_bytes)
            .field("resume_offset", &self.resume_offset)
            .field("headers", &self.headers)
            .field("on_progress", &"{ ... }")
            .finish()
    }
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            checksum: None,
            retry_policy: RetryPolicy::default(),
            expected_bytes: None,
            resume_offset: None,
            headers: Arc::new([]),
            on_progress: None,
        }
    }
}

impl FetchOptions {
    /// Set the expected checksum.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::FetchOptions;
    ///
    /// let hash = [0u8; 32]; // Your expected SHA-256
    /// let options = FetchOptions::default().checksum(Some(hash));
    /// ```
    #[must_use]
    pub fn checksum(mut self, checksum: Option<[u8; 32]>) -> Self {
        self.checksum = checksum;
        self
    }

    /// Set the maximum number of retries.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::FetchOptions;
    ///
    /// let options = FetchOptions::default().max_retries(5);
    /// ```
    #[must_use]
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.retry_policy.max_retries = max_retries;
        self
    }

    /// Set the base retry backoff duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::FetchOptions;
    /// use std::time::Duration;
    ///
    /// let options = FetchOptions::default()
    ///     .retry_backoff(Duration::from_millis(200));
    /// ```
    #[must_use]
    pub fn retry_backoff(mut self, retry_backoff: Duration) -> Self {
        self.retry_policy.base_backoff = retry_backoff;
        self
    }

    #[must_use]
    /// Set the full retry policy object directly.
    pub fn retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    #[must_use]
    /// Set expected transfer size for progress/reporting without HEAD lookup.
    pub fn expected_bytes(mut self, expected_bytes: Option<u64>) -> Self {
        self.expected_bytes = expected_bytes;
        self
    }

    #[must_use]
    /// Set resume offset in bytes for ranged fetch.
    pub fn resume_offset(mut self, resume_offset: Option<u64>) -> Self {
        self.resume_offset = resume_offset;
        self
    }

    /// Add a single custom HTTP header.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::FetchOptions;
    ///
    /// let options = FetchOptions::default()
    ///     .header("Authorization", "Bearer token")
    ///     .header("User-Agent", "MyApp/1.0");
    /// ```
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut headers: Vec<_> = self.headers.iter().cloned().collect();
        headers.push((key.into(), value.into()));
        self.headers = Arc::from(headers);
        self
    }

    /// Set multiple custom HTTP headers at once.
    ///
    /// This replaces any existing headers.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::FetchOptions;
    ///
    /// let headers = vec![
    ///     ("Authorization".to_string(), "Bearer token".to_string()),
    ///     ("User-Agent".to_string(), "MyApp/1.0".to_string()),
    /// ];
    /// let options = FetchOptions::default().headers(headers);
    /// ```
    #[must_use]
    pub fn headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = Arc::from(headers);
        self
    }

    /// Set the progress callback.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::{FetchOptions, FetchPhase};
    /// use std::sync::Arc;
    ///
    /// let options = FetchOptions::default()
    ///     .on_progress(Arc::new(|progress| {
    ///         match progress.phase {
    ///             FetchPhase::Downloading => {
    ///                 if let Some(pct) = progress.percentage() {
    ///                     println!("Progress: {:.1}%", pct);
    ///                 }
    ///             }
    ///             FetchPhase::Completed => println!("Done!"),
    ///             _ => {}
    ///         }
    ///     }));
    /// ```
    #[must_use]
    pub fn on_progress(mut self, on_progress: Arc<dyn Fn(&Progress) + Send + Sync>) -> Self {
        self.on_progress = Some(on_progress);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_fetch_phase_display() {
        assert_eq!(FetchPhase::Connecting.to_string(), "Connecting");
        assert_eq!(FetchPhase::Downloading.to_string(), "Downloading");
        assert_eq!(FetchPhase::Verifying.to_string(), "Verifying");
        assert_eq!(FetchPhase::Committing.to_string(), "Committing");
        assert_eq!(FetchPhase::Completed.to_string(), "Completed");
    }

    #[test]
    fn test_fetch_phase_default() {
        assert_eq!(FetchPhase::default(), FetchPhase::Connecting);
    }

    #[test]
    fn test_fetch_options_default() {
        let options = FetchOptions::default();
        assert!(options.checksum.is_none());
        assert_eq!(options.retry_policy.max_retries, 3);
        assert_eq!(
            options.retry_policy.base_backoff,
            Duration::from_millis(100)
        );
        assert_eq!(options.expected_bytes, None);
        assert_eq!(options.resume_offset, None);
        assert!(options.headers.is_empty());
        assert!(options.on_progress.is_none());
    }

    #[test]
    fn test_fetch_options_checksum() {
        let hash = [1u8; 32];
        let options = FetchOptions::default().checksum(Some(hash));
        assert_eq!(options.checksum, Some(hash));

        let options = FetchOptions::default().checksum(None);
        assert!(options.checksum.is_none());
    }

    #[test]
    fn test_fetch_options_max_retries() {
        let options = FetchOptions::default().max_retries(5);
        assert_eq!(options.retry_policy.max_retries, 5);

        let options = FetchOptions::default().max_retries(0);
        assert_eq!(options.retry_policy.max_retries, 0);
    }

    #[test]
    fn test_fetch_options_retry_backoff() {
        let duration = Duration::from_secs(1);
        let options = FetchOptions::default().retry_backoff(duration);
        assert_eq!(options.retry_policy.base_backoff, duration);
    }

    #[test]
    fn test_fetch_options_resume_and_expected_bytes() {
        let options = FetchOptions::default()
            .resume_offset(Some(128))
            .expected_bytes(Some(512));
        assert_eq!(options.resume_offset, Some(128));
        assert_eq!(options.expected_bytes, Some(512));
    }

    #[test]
    fn test_fetch_options_header() {
        let options = FetchOptions::default()
            .header("Authorization", "Bearer token")
            .header("User-Agent", "MyApp/1.0");

        let headers: Vec<_> = options.headers.iter().cloned().collect();
        assert_eq!(headers.len(), 2);
        assert!(headers.contains(&("Authorization".to_string(), "Bearer token".to_string())));
        assert!(headers.contains(&("User-Agent".to_string(), "MyApp/1.0".to_string())));
    }

    #[test]
    fn test_fetch_options_headers() {
        let headers = vec![
            ("Authorization".to_string(), "Bearer token".to_string()),
            ("User-Agent".to_string(), "MyApp/1.0".to_string()),
        ];
        let options = FetchOptions::default().headers(headers.clone());

        let options_headers: Vec<_> = options.headers.iter().cloned().collect();
        assert_eq!(options_headers, headers);
    }

    #[test]
    fn test_fetch_options_headers_replace() {
        let options = FetchOptions::default()
            .header("Old", "value")
            .headers(vec![("New".to_string(), "value".to_string())]);

        let headers: Vec<_> = options.headers.iter().cloned().collect();
        assert_eq!(headers.len(), 1);
        assert!(headers.contains(&("New".to_string(), "value".to_string())));
        assert!(!headers.iter().any(|(k, _)| k == "Old"));
    }

    #[test]
    fn test_fetch_options_on_progress() {
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let options = FetchOptions::default().on_progress(Arc::new(move |_| {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        }));

        assert!(options.on_progress.is_some());

        if let Some(callback) = &options.on_progress {
            let progress = Progress {
                phase: FetchPhase::Downloading,
                bytes_downloaded: 100,
                total_bytes: Some(1000),
                retry_count: 0,
                performance_metrics: None,
            };
            callback(&progress);
            assert_eq!(call_count.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn test_fetch_options_debug() {
        let options = FetchOptions::default()
            .checksum(Some([1u8; 32]))
            .max_retries(5)
            .header("Test", "value");

        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("FetchOptions"));
        assert!(debug_str.contains("checksum: Some(["));
        assert!(debug_str.contains("retry_policy"));
        assert!(debug_str.contains("{ ... }"));
    }

    #[test]
    fn test_fetch_options_builder_pattern() {
        let hash = [2u8; 32];
        let options = FetchOptions::default()
            .checksum(Some(hash))
            .max_retries(10)
            .retry_backoff(Duration::from_millis(500))
            .header("Custom", "header");

        assert_eq!(options.checksum, Some(hash));
        assert_eq!(options.retry_policy.max_retries, 10);
        assert_eq!(
            options.retry_policy.base_backoff,
            Duration::from_millis(500)
        );
        assert_eq!(options.headers.len(), 1);

        // Test with headers() replacing
        let options2 = FetchOptions::default()
            .checksum(Some(hash))
            .max_retries(10)
            .retry_backoff(Duration::from_millis(500))
            .headers(vec![("Another".to_string(), "header".to_string())]);

        assert_eq!(options2.checksum, Some(hash));
        assert_eq!(options2.retry_policy.max_retries, 10);
        assert_eq!(
            options2.retry_policy.base_backoff,
            Duration::from_millis(500)
        );
        assert_eq!(options2.headers.len(), 1);
    }

    #[test]
    fn test_fetch_options_clone() {
        let options = FetchOptions::default()
            .checksum(Some([3u8; 32]))
            .header("Test", "value");

        let cloned = options.clone();
        assert_eq!(cloned.checksum, options.checksum);
        assert_eq!(cloned.retry_policy, options.retry_policy);
        assert_eq!(cloned.headers.as_ptr(), options.headers.as_ptr()); // Same Arc
    }
}
