# Plan: Group pulith-fetch by Clear Functional Names

## TL;DR

> **Quick Summary**: Reorganize `pulith-fetch` into functional groups with clear, elegant, readable names.
> 
> **New Groups**:
> - `rate/` - Rate limiting, backoff, throttling
> - `segment/` - File segmentation and byte ranges
> - `fetch/` - Download strategies (basic, segmented, multi-source, batch, resumable, conditional)
> - `config/` - Configuration types (options, sources)
> - `progress/` - Progress tracking and reporting
> - `cache/` - Caching implementations
> - `codec/` - Stream processing (decompress, verify, signature)
> - `net/` - HTTP and protocol abstractions
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: NO
> **Critical Path**: Create dirs → Move files → Update imports → Test

---

## Current vs Proposed

### Current (25 flat files)
```
src/
  backoff.rs, bandwidth.rs, validation.rs, segment.rs
  fetch_options.rs, sources.rs, progress.rs, extended_progress.rs
  http.rs, protocol.rs
  fetcher.rs, segmented.rs, multi_source.rs, batch.rs, resumable.rs, conditional.rs, throttled.rs
  file_cache.rs, http_cache.rs
  decompress.rs, signature.rs, verify.rs
  perf.rs
```

### Proposed (8 functional groups)
```
src/
  lib.rs
  error.rs
  perf.rs
  
  rate.rs               # Re-exports: backoff, bandwidth, throttled
  rate/
    backoff.rs          # Retry delay calculation
    bandwidth.rs        # Token bucket rate limiting
    throttled.rs        # Throttled streams
  
  segment.rs            # Re-exports: segment, validation
  segment/
    segment.rs          # File segmentation logic
    validation.rs       # HTTP validation helpers
  
  fetch.rs              # Re-exports: all fetch strategies
  fetch/
    fetcher.rs          # Basic file fetcher
    segmented.rs        # Parallel segmented downloads
    multi_source.rs     # Multi-source with failover
    batch.rs            # Batch downloads with dependencies
    resumable.rs        # Resumable downloads
    conditional.rs      # Conditional downloads (ETag)
  
  config.rs             # Re-exports: fetch_options, sources
  config/
    fetch_options.rs    # Fetch configuration
    sources.rs          # Download source types
  
  progress.rs           # Re-exports: progress, extended_progress
  progress/
    progress.rs         # Basic progress tracking
    extended_progress.rs # Advanced progress metrics
  
  cache.rs              # Re-exports: file_cache, http_cache
  cache/
    file_cache.rs       # File-based content cache
    http_cache.rs       # HTTP conditional cache
  
  codec.rs              # Re-exports: decompress, verify, signature
  codec/
    decompress.rs       # Stream decompression
    verify.rs           # Checksum verification
    signature.rs        # Signature verification
  
  net.rs                # Re-exports: http, protocol
  net/
    http.rs             # HTTP client abstraction
    protocol.rs         # Protocol abstraction layer
```

---

## Rationale for Names

| Group | Contains | Why this name |
|-------|----------|---------------|
| `rate` | backoff, bandwidth, throttled | **Rate control** - everything related to limiting speed, retry delays, throttling |
| `segment` | segment, validation | **Segmentation** - file splitting into segments + validation helpers |
| `fetch` | All fetcher types | **Fetch strategies** - different ways to download files |
| `config` | fetch_options, sources | **Configuration** - setup and source definitions |
| `progress` | progress, extended_progress | **Progress tracking** - monitoring download progress |
| `cache` | file_cache, http_cache | **Caching** - storing and reusing downloaded content |
| `codec` | decompress, verify, signature | **Codec (Coder-Decoder)** - transform and verify data streams |
| `net` | http, protocol | **Network** - low-level networking abstractions |

---

## Module File Templates

### rate.rs
```rust
//! Rate control: limiting, backoff, and throttling.
//!
//! This module provides tools for controlling the rate of operations:
//! - Exponential backoff for retries
//! - Token bucket for bandwidth limiting
//! - Stream throttling

pub mod backoff;
pub mod bandwidth;
pub mod throttled;

pub use backoff::retry_delay;
pub use bandwidth::{TokenBucket, AdaptiveConfig, RateMetrics};
pub use throttled::{ThrottledStream, AsyncThrottledStream};
```

### segment.rs
```rust
//! File segmentation for parallel downloads.
//!
//! Split files into segments for concurrent downloading,
//! plus HTTP validation utilities.

pub mod segment;
pub mod validation;

pub use segment::{calculate_segments, Segment};
pub use validation::is_redirect;
```

### fetch.rs
```rust
//! Download strategies and fetch implementations.
//!
//! Various approaches to downloading files:
//! - Basic single-source fetch
//! - Segmented parallel download
//! - Multi-source with failover
//! - Batch with dependencies
//! - Resumable downloads
//! - Conditional (ETag-based)

pub mod fetcher;
pub mod segmented;
pub mod multi_source;
pub mod batch;
pub mod resumable;
pub mod conditional;

pub use fetcher::Fetcher;
pub use segmented::{SegmentedFetcher, SegmentedOptions};
pub use multi_source::MultiSourceFetcher;
pub use batch::{BatchFetcher, BatchOptions, BatchDownloadJob};
pub use resumable::{ResumableFetcher, DownloadCheckpoint};
pub use conditional::{ConditionalFetcher, RemoteMetadata, ConditionalOptions};
```

### config.rs
```rust
//! Configuration types for fetch operations.

pub mod fetch_options;
pub mod sources;

pub use fetch_options::{FetchOptions, FetchPhase};
pub use sources::{DownloadSource, MultiSourceOptions, SourceSelectionStrategy, SourceType};
```

### progress.rs
```rust
//! Progress tracking and reporting.

pub mod progress;
pub mod extended_progress;

pub use progress::{Progress, PerformanceMetrics, PhaseTimings};
pub use extended_progress::{ExtendedProgress, ProgressReporter};
```

### cache.rs
```rust
//! Caching implementations for downloaded content.

pub mod file_cache;
pub mod http_cache;

pub use file_cache::{Cache, CacheConfig, CacheEntry, CacheStats};
pub use http_cache::{CacheControl, CacheEntry as HttpCacheEntry, CacheError, CacheStats as HttpCacheStats, CacheValidation, ConditionalHeaders, HttpCache};
```

### codec.rs
```rust
//! Stream codec: transform and verify data.
//!
//! Encode/decode operations on data streams:
//! - Decompression
//! - Checksum verification
//! - Signature verification

pub mod decompress;
pub mod verify;
pub mod signature;

pub use decompress::{StreamTransform, TransformError};
pub use verify::{parse_multiple_checksums, verify_checksum, verify_multiple_checksums, ChecksumConfig, MultiVerifier, StreamVerifier};
pub use signature::{
    verify_signature, KeyPurpose, KeyUsage, MockVerifier, PublicKey, PublicKeyFormat, Signature,
    SignatureAlgorithm, SignatureConfig, SignatureFormat, SignatureManager, SignatureVerifier,
};
```

### net.rs
```rust
//! Network abstractions: HTTP and protocol traits.

pub mod http;
pub mod protocol;

pub use http::{HttpClient, BoxStream, ReqwestClient};
pub use protocol::{Protocol, Direction, TransferMetadata, TransferOptions, TransferStream, ProtocolClient, ProtocolRegistry, MockHttpClient, MockTransferStream};
```

---

## Updated lib.rs

```rust
//! HTTP downloading with streaming verification and atomic placement.

mod error;

pub mod rate;       // Rate limiting and backoff
pub mod segment;    // File segmentation
pub mod fetch;      // Download strategies
pub mod config;     // Configuration types
pub mod progress;   // Progress tracking
pub mod cache;      // Caching
pub mod codec;      // Stream processing
pub mod net;        // Network abstractions
pub mod perf;       // Performance monitoring

pub use error::{Error, Result};

// Re-exports for convenience
pub use rate::{retry_delay, TokenBucket, ThrottledStream, AsyncThrottledStream};
pub use segment::{calculate_segments, Segment, is_redirect};
pub use config::{FetchOptions, FetchPhase, DownloadSource, MultiSourceOptions};
pub use progress::{Progress, ExtendedProgress};
pub use fetch::{Fetcher, SegmentedFetcher, MultiSourceFetcher};
pub use net::{HttpClient, BoxStream};
```

---

## Import Path Changes

### Before
```rust
use crate::backoff::retry_delay;
use crate::bandwidth::TokenBucket;
use crate::fetcher::Fetcher;
use crate::segmented::SegmentedFetcher;
use crate::file_cache::Cache;
use crate::decompress::StreamTransform;
use crate::verify::verify_checksum;
```

### After
```rust
use crate::rate::{retry_delay, TokenBucket};
use crate::fetch::{Fetcher, SegmentedFetcher};
use crate::cache::Cache;
use crate::codec::{StreamTransform, verify_checksum};
```

---

## File Movements

| Old Location | New Location |
|--------------|--------------|
| backoff.rs | rate/backoff.rs |
| bandwidth.rs | rate/bandwidth.rs |
| throttled.rs | rate/throttled.rs |
| segment.rs | segment/segment.rs |
| validation.rs | segment/validation.rs |
| fetcher.rs | fetch/fetcher.rs |
| segmented.rs | fetch/segmented.rs |
| multi_source.rs | fetch/multi_source.rs |
| batch.rs | fetch/batch.rs |
| resumable.rs | fetch/resumable.rs |
| conditional.rs | fetch/conditional.rs |
| fetch_options.rs | config/fetch_options.rs |
| sources.rs | config/sources.rs |
| progress.rs | progress/progress.rs |
| extended_progress.rs | progress/extended_progress.rs |
| file_cache.rs | cache/file_cache.rs |
| http_cache.rs | cache/http_cache.rs |
| decompress.rs | codec/decompress.rs |
| verify.rs | codec/verify.rs |
| signature.rs | codec/signature.rs |
| http.rs | net/http.rs |
| protocol.rs | net/protocol.rs |
| perf.rs | perf.rs (unchanged) |
| error.rs | error.rs (unchanged) |

---

## TODOs

- [ ] 1. Create 8 group module files (rate.rs, segment.rs, fetch.rs, config.rs, progress.rs, cache.rs, codec.rs, net.rs)
- [ ] 2. Create 8 subdirectories
- [ ] 3. Move existing files into subdirectories
- [ ] 4. Update lib.rs with new module structure
- [ ] 5. Update imports in all moved files (crate::rate, crate::fetch, etc.)
- [ ] 6. Update test imports in tests/
- [ ] 7. Update benchmark imports in benches/
- [ ] 8. Verify cargo build succeeds
- [ ] 9. Run cargo test
- [ ] 10. Verify benchmarks: cargo build --benches && cargo bench
- [ ] 11. Clean up old flat files

---

## Verification Commands

```bash
cd crates/pulith-fetch

# Build
cargo build

# Test
cargo test

# Benchmarks
cargo build --benches
cargo bench
```

---

## Success Criteria

- [ ] All 175+ unit tests pass
- [ ] All 4 integration tests pass
- [ ] All 9 doc tests pass
- [ ] Benchmarks compile and run
- [ ] Clear functional grouping with readable names
- [ ] No breaking changes to public API
