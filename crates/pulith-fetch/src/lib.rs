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

pub mod rate;
pub mod segment;
pub mod fetch;
pub mod config;
pub mod progress;
pub mod cache;
pub mod codec;
pub mod net;
pub mod perf;

pub use error::{Error, Result};

pub use rate::{retry_delay, TokenBucket, ThrottledStream, AsyncThrottledStream};
pub use segment::{calculate_segments, Segment, is_redirect};
pub use config::{FetchOptions, FetchPhase, DownloadSource, MultiSourceOptions, SourceSelectionStrategy, SourceType};
pub use progress::{Progress, ExtendedProgress, PerformanceMetrics, PhaseTimings, ProgressReporter};
pub use net::{HttpClient, BoxStream, ReqwestClient, Protocol};
pub use fetch::{Fetcher, SegmentedFetcher, SegmentedOptions, MultiSourceFetcher, ConditionalFetcher, RemoteMetadata, ConditionalOptions, BatchFetcher, BatchOptions, BatchDownloadJob, ResumableFetcher, DownloadCheckpoint};
pub use cache::{Cache, HttpCache, CacheControl, CacheEntry, CacheError, CacheStats};
pub use codec::{StreamTransform, TransformError, verify_checksum, verify_signature, ChecksumConfig, SignatureVerifier, MultiVerifier, StreamVerifier};
