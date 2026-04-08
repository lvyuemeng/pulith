//! HTTP downloading with streaming verification and atomic placement.
//!
//! # Architecture
//!
//! This crate follows a functional group structure:
//! - `rate` - Rate limiting, backoff, throttling
//! - `segment` - File segmentation and validation
//! - `fetch` - Download strategies
//! - `config` - Configuration types
//! - `progress` - Progress tracking
//! - `cache` - Caching implementations
//! - `codec` - Stream processing (decompress, verify, signature)
//! - `net` - Network abstractions
//!
//! # Key Features
//!
//! - **Single-Pass**: Tee-Reader pattern hashes while streaming to avoid memory bloat
//! - **Atomic Placement**: Uses `pulith-fs::Workspace` for guaranteed cleanup on error
//! - **Streaming Verification**: Uses `pulith-verify::Hasher` for incremental hashing
//! - **Mechanism-Only**: No policy; caller handles progress UI and retry orchestration

mod error;

pub mod cache;
pub mod codec;
pub mod config;
pub mod fetch;
pub mod net;
pub mod perf;
pub mod progress;
pub mod rate;
pub mod segment;

pub use error::{Error, Result};

pub use cache::{Cache, CacheControl, CacheEntry, CacheError, CacheStats, HttpCache};
pub use codec::{
    ChecksumConfig, MultiVerifier, SignatureVerifier, StreamTransform, StreamVerifier,
    TransformError, verify_checksum, verify_signature,
};
pub use config::{
    DownloadSource, FetchOptions, FetchPhase, MultiSourceOptions, SourceSelectionStrategy,
    SourceType,
};
pub use fetch::{
    BatchDownloadJob, BatchFetcher, BatchOptions, ConditionalFetcher, ConditionalOptions,
    DownloadCheckpoint, Fetcher, MultiSourceFetcher, RemoteMetadata, ResumableFetcher,
    SegmentedFetcher, SegmentedOptions,
};
pub use net::{BoxStream, HttpClient, Protocol, ReqwestClient};
pub use progress::{
    ExtendedProgress, PerformanceMetrics, PhaseTimings, Progress, ProgressReporter,
};
pub use rate::{AsyncThrottledStream, ThrottledStream, TokenBucket, retry_delay};
pub use segment::{Segment, calculate_segments, is_redirect};
