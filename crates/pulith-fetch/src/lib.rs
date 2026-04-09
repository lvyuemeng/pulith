//! HTTP transfer primitives for Pulith.
//!
//! Keep planning in `pulith-source`, verification in `pulith-verify`,
//! and filesystem safety in `pulith-fs`. This crate owns transfer execution.

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
    DownloadCheckpoint, FetchReceipt, FetchSource, Fetcher, MultiSourceFetcher, RemoteMetadata,
    ResumableFetcher, SegmentedFetcher, SegmentedOptions,
};
pub use net::{BoxStream, HttpClient, ReqwestClient};
pub use progress::{
    ExtendedProgress, PerformanceMetrics, PhaseTimings, Progress, ProgressReporter,
};
pub use rate::{AsyncThrottledStream, ThrottledStream, TokenBucket, retry_delay};
pub use segment::{Segment, calculate_segments, is_redirect};
