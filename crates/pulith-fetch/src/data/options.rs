use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use super::progress::Progress;

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
/// use pulith_fetch::data::FetchOptions;
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

    /// Maximum number of retry attempts for transient failures.
    ///
    /// - Includes only retries after the initial attempt
    /// - Retries are triggered for network errors and 5xx HTTP errors
    /// - Does not retry for 4xx errors or checksum mismatches
    /// - Total attempts = 1 (initial) + max_retries
    ///
    /// Default: 3
    pub max_retries: u32,

    /// Base delay for exponential backoff between retries.
    ///
    /// The actual delay for retry N is: `retry_backoff * 2^(N-1)`
    ///
    /// Default: 100ms
    pub retry_backoff: Duration,

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
    pub on_progress: Option<Arc<dyn Fn(&Progress) + Send + Sync>>,
}

impl fmt::Debug for FetchOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FetchOptions")
            .field("checksum", &self.checksum)
            .field("max_retries", &self.max_retries)
            .field("retry_backoff", &self.retry_backoff)
            .field("headers", &self.headers)
            .field("on_progress", &"{ ... }")
            .finish()
    }
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            checksum: None,
            max_retries: 3,
            retry_backoff: Duration::from_millis(100),
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
    /// use pulith_fetch::data::FetchOptions;
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
    /// use pulith_fetch::data::FetchOptions;
    ///
    /// let options = FetchOptions::default().max_retries(5);
    /// ```
    #[must_use]
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the base retry backoff duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::data::FetchOptions;
    /// use std::time::Duration;
    ///
    /// let options = FetchOptions::default()
    ///     .retry_backoff(Duration::from_millis(200));
    /// ```
    #[must_use]
    pub fn retry_backoff(mut self, retry_backoff: Duration) -> Self {
        self.retry_backoff = retry_backoff;
        self
    }

    /// Add a single custom HTTP header.
    ///
    /// # Examples
    ///
    /// ```
    /// use pulith_fetch::data::FetchOptions;
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
    /// use pulith_fetch::data::FetchOptions;
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
    /// use pulith_fetch::data::{FetchOptions, FetchPhase};
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
