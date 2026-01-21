# pulith-fetch Design

## Overview

HTTP downloading with verification, progress tracking, and retry logic. Downloads to temporary files for verification before use.

## Scope

**Included:**
- HTTP/HTTPS downloads to temp files
- SHA256 checksum verification
- Retry with exponential backoff
- Redirect handling (up to 10)
- Progress callbacks
- Configurable timeouts

**Excluded:**
- Authentication (caller handles)
- Streaming downloads (future enhancement)
- Resumable downloads (future enhancement)
- Multiple checksum algorithms (future enhancement)

## Public API

### DownloadOptions

```rust
pub struct DownloadOptions {
    /// URL to download
    pub url: String,

    /// Expected SHA256 checksum (optional)
    pub checksum: Option<Sha256Hash>,

    /// Maximum retry attempts (default: 3)
    pub max_retries: u32,

    /// Base delay between retries (default: 100ms)
    pub retry_backoff: Duration,

    /// Connection timeout (default: 30s)
    pub connect_timeout: Duration,

    /// Read timeout per chunk (default: 30s)
    pub read_timeout: Duration,

    /// Progress callback (optional)
    pub on_progress: Option<ProgressCallback>,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            checksum: None,
            max_retries: 3,
            retry_backoff: Duration::from_millis(100),
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            on_progress: None,
        }
    }
}
```

### Sha256Hash

```rust
/// SHA256 checksum wrapper.
///
/// Parses from hex string format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Hash(String);

impl Sha256Hash {
    /// Parse from hex string.
    pub fn from_hex(s: &str) -> Result<Self, ParseSha256HashError>;

    /// Get as hex string.
    pub fn as_str(&self) -> &str;
}
```

### Downloader

```rust
pub struct Downloader {
    client: reqwest::Client,
    options: DownloadOptions,
}

impl Downloader {
    /// Create new downloader with options.
    pub fn new(options: DownloadOptions) -> Self;

    /// Download URL to a temporary file.
    ///
    /// Returns path to temp file on success.
    /// Caller is responsible for cleanup.
    ///
    /// # Errors
    ///
    /// Returns `FetchError` on failure.
    pub async fn download_to_temp(&self) -> Result<PathBuf, FetchError>;

    /// Download URL to a specific path.
    ///
    /// Creates parent directories if needed.
    ///
    /// # Errors
    ///
    /// Returns `FetchError` on failure.
    pub async fn download_to(&self, path: &Path) -> Result<(), FetchError>;

    /// Download URL directly to staging area.
    ///
    /// Convenience method combining download and staging location.
    /// The staging path is returned for further processing.
    ///
    /// # Errors
    ///
    /// Returns `FetchError` on failure.
    pub async fn download_to_staging(
        &self,
        staging_dir: &Path,
        filename: &str,
    ) -> Result<PathBuf, FetchError>;
}
```

## Error Types

```rust
#[derive(Debug, Error)]
pub enum FetchError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("download failed: {0}")]
    DownloadFailed(#[source] reqwest::Error),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("max retries exceeded ({count} attempts)")]
    MaxRetriesExceeded { count: u32 },

    #[error("redirect loop detected (more than 10 redirects)")]
    TooManyRedirects,

    #[error("file I/O error: {0}")]
    IoError(#[source] std::io::Error),

    #[error("checksum parse error: {0}")]
    ChecksumParseError(#[source] ParseSha256HashError),

    #[error("destination path is a directory")]
    DestinationIsDirectory,
}
```

## Progress Reporting

```rust
/// Download progress information.
#[derive(Debug, Clone)]
pub struct Progress {
    /// Bytes downloaded so far.
    pub bytes_downloaded: u64,

    /// Total bytes expected (None if unknown).
    pub total_bytes: Option<u64>,

    /// Current phase of download.
    pub phase: DownloadPhase,

    /// Number of retries attempted.
    pub retry_count: u32,
}

/// Download phase for UI feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadPhase {
    /// Establishing connection.
    Connecting,

    /// Downloading content.
    Downloading,

    /// Verifying checksum.
    Verifying,

    /// Download completed successfully.
    Completed,
}

/// Progress callback type.
pub type ProgressCallback = fn(Progress);
```

## Verification Flow

```
download → temp_file
      ↓
verify_checksum(temp_file, expected)
      ↓
  success → return temp_file path
  failure → retry or abort
```

## Retry Strategy

- Default: 3 retries with exponential backoff (100ms, 200ms, 400ms...)
- Configurable via `DownloadOptions.max_retries` and `retry_backoff`
- Network errors (5xx, timeouts) retried
- Client errors (4xx) not retried
- Redirects followed automatically (max 10)

## Future Enhancements (Documented)

These features are planned but not in initial scope:

### Streaming Downloads
Direct to staging area for memory efficiency with large files.

```rust
// Future API (not implemented yet)
pub async fn download_stream(
    &self,
    writer: impl AsyncWriteExt,
) -> Result<(), FetchError>;
```

### Resumable Downloads
Continue partial downloads using Range headers.

```rust
// Future API (not implemented yet)
pub async fn download_resume(
    &self,
    path: &Path,
    range: u64,
) -> Result<u64, FetchError>;
```

### Multiple Algorithms
SHA512, BLAKE3 support for verification.

```rust
// Future API (not implemented yet)
pub struct Checksum {
    pub algorithm: ChecksumAlgorithm,
    pub hash: String,
}

pub enum ChecksumAlgorithm {
    Sha256,
    Sha512,
    Blake3,
}
```

## Composition

### With pulith-install

```rust
use pulith_fetch::{Downloader, DownloadOptions};

async fn install_from_url(
    layout: &StoreLayout,
    url: &str,
    version: &str,
    checksum: &str,
) -> Result<(), Error> {
    let downloader = Downloader::new(DownloadOptions {
        url: url.to_string(),
        checksum: Some(checksum.parse()?),
        ..Default::default()
    });

    let staging = layout.staging().join(version);
    downloader.download_to(&staging).await?;

    atomic_replace(&staging, &layout.version(version))?;
    Ok(())
}
```

## Module Structure

```
pulith-fetch/src/
├── lib.rs              # Public exports, prelude
├── downloader.rs       # Downloader, DownloadOptions
├── error.rs            # FetchError, ParseSha256HashError
└── progress.rs         # Progress, DownloadPhase
```

## Dependencies

```toml
[package]
name = "pulith-fetch"
version = "0.1.0"
edition = "2024"

[dependencies]
reqwest = { version = "0.12", features = ["json", "stream", "tls"] }
tokio = { version = "1", features = ["full"] }
thiserror = { workspace = true }
anyhow = { workspace = true }
sha2 = "0.10"
```

## Design Decisions

### Why Temp File First?

- Verification before activation
- Resume capability (future)
- Memory efficient for large files
- Atomic: either fully present or not

### Checksum Timing

- Verify **after** download, **before** install
- Never install unverified content
- SHA256 as default (widely supported, good security)

### Callback Design

- `Fn(Progress)` not `&mut Fn` - caller manages state
- Phase field for UI feedback (connecting vs downloading)
- No ETA - unreliable for network streams
- Retry count included for debugging

### Why reqwest?

- Full HTTP/HTTPS support with TLS
- Async runtime agnostic (works with tokio)
- Handles redirects, timeouts, chunked encoding
- Battle-tested in production
- Avoids reimplementing HTTP client wheel
