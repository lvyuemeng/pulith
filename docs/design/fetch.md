# pulith-fetch: Comprehensive Redesign Plan

## Executive Summary

This document outlines a complete redesign of `pulith-fetch` crate to address architectural issues, align with ecosystem patterns, and implement planned features from the roadmap. The redesign follows the pulith design philosophy (F1-F5 principles) and consistency patterns established across all other pulith crates.

---

## Current State Analysis

### ✅ Implemented Features (v0.2.0)
- Single-source HTTP downloads with exponential backoff
- SHA-256 verification via zero-copy streaming
- Atomic file placement using `pulith-fs::Workspace`
- Progress callbacks for UI integration
- Configurable retries with smart error detection
- HTTP client abstraction for testability

### ❌ Critical Issues Identified

#### 1. **Consistency Violation: Missing Result<T> Type Alias**
**Severity**: CRITICAL

**Problem**: pulith-fetch is the **only** crate in the pulith ecosystem without a `Result<T>` type alias.

**Evidence**:
- `pulith-fs`: `pub use error::{Error, Result};`
- `pulith-verify`: `pub use self::error::{Result, VerifyError};`
- `pulith-archive`: `pub use error::{Error, Result};`
- `pulith-shim`: `pub use error::{Error, Result};`
- **pulith-fetch**: Only exports `FetchError`, no `Result<T>` alias

**Impact**: Breaks ecosystem API consistency, worse ergonomics for users

**Fix**: Add `Result<T>` type alias in `error.rs` and re-export in `lib.rs`

#### 2. **Monolithic source.rs (1700+ lines)**
**Severity**: HIGH

**Problem**: Single file contains:
- Type definitions for 10+ future features
- Stub implementations
- 140+ test functions
- Helper functions
- Mixed concerns (types + implementations + tests)

**Impact**: Unmaintainable, violates single responsibility principle

**Fix**: Split into modular structure following pulith philosophy

#### 3. **Effects Module Disorganization**
**Severity**: HIGH

**Problem**: `effects.rs` contains multiple fetcher implementations:
- `MultiSourceFetcher`
- `ResumableFetcher`
- `BandwidthLimiter` and `ThrottledStream`
- `SegmentedFetcher`
- `BatchFetcher`
- `CachedFetcher`
- All in one 800+ line file

**Impact**: No clear separation, difficult to navigate

**Fix**: Split into separate modules by feature

#### 4. **Unsafe unwrap() Calls**
**Severity**: MEDIUM

**Problem**: 10 instances of `unwrap()` found in production code:

**source.rs:**
- Line 1285: `segments.last().unwrap()` - Could panic if empty
- Line 1369: `.to_std().unwrap()` - Time conversion could fail
- Line 1549: `opts.cache.unwrap().cache_dir` - Unnecessary unwrap

**effects.rs:**
- Line 517: `sem.acquire().await.unwrap()` - Semaphore acquisition could fail
- Line 622, 653: `.unwrap()` - Result handling missing
- Line 690, 711: `ReqwestClient::new().unwrap()` - Test code only
- Line 917: `opts.burst_size.unwrap() > opts.max_bytes_per_second.unwrap()` - Test code

**Impact**: Potential panics in production

**Fix**: Replace with proper error handling using `?` operator or context

#### 5. **Missing Feature Implementations**
**Severity**: MEDIUM

**Problem**: Many features have type definitions but no implementations:
- Compression support (types defined, no transform implementation)
- Protocol extensions (S3, FTP, Custom - types only)
- Integrity verification (signatures - types only)
- Conditional downloads (types only)

**Impact**: Incomplete roadmap, confusing API surface

**Fix**: Either implement or mark as future work with proper visibility

#### 6. **Builder Pattern Inconsistency**
**Severity**: LOW

**Problem**: Some structs have builders, some don't:
- `FetchOptions`: Builder with `#[must_use]` ✅
- `DownloadSource`: Builder methods, no `#[must_use]` ⚠️
- `BatchOptions`: No builder pattern ❌
- `CacheOptions`: No builder pattern ❌

**Impact**: Inconsistent API experience

**Fix**: Add builder pattern and `#[must_use]` to all option structs

---

## Target Architecture

### Design Principles

Following `AGENT.md` specifications:

**F1 — Functions First**
- All behavior expressed as `output = f(input)`
- No hidden state or magic side effects
- Pure transformations over objects with behavior

**F2 — Immutability by Default**
- Core data immutable (config types, progress structs)
- Mutation only at system boundaries (I/O, caches, buffers)

**F3 — Pure Core, Impure Edge**
- Pure core: Data transformations, retry logic, segment calculation
- Impure edge: HTTP requests, file I/O, state persistence

**F4 — Explicit Effects**
- Async functions clearly indicate effects via Future return
- HTTP client trait encapsulates all side effects

**F5 — Composition Over Orchestration**
- Traits for composability (HttpClient, Hasher, StreamTransform)
- Users control orchestration (no built-in retry loops in public API)

### Module Structure

```
crates/pulith-fetch/src/
├── lib.rs              # Public API exports
├── error.rs            # Error types and Result<T> alias
├── data/               # Immutable data types
│   ├── mod.rs          # Module re-exports
│   ├── options.rs       # FetchOptions, all builder types
│   ├── progress.rs      # Progress, FetchPhase
│   └── sources.rs      # DownloadSource, MultiSourceOptions
├── core/               # Pure transformations
│   ├── mod.rs          # Module re-exports
│   ├── retry.rs        # Retry logic, backoff calculation
│   ├── bandwidth.rs     # Bandwidth limiting algorithms
│   ├── segment.rs       # Segment calculation for parallel downloads
│   └── validation.rs    # URL validation, redirect detection
├── effects/            # I/O operations
│   ├── mod.rs          # Module re-exports
│   ├── http.rs         # HttpClient trait, ReqwestClient
│   ├── fetcher.rs      # Base Fetcher implementation
│   ├── multi_source.rs # MultiSourceFetcher
│   ├── resumable.rs   # ResumableFetcher
│   ├── segmented.rs    # SegmentedFetcher
│   ├── batch.rs        # BatchFetcher
│   └── cache.rs        # CachedFetcher
└── transform/          # Stream transformations
    ├── mod.rs          # Module re-exports
    ├── decompress.rs    # Compression support
    └── verify.rs       # Integrity verification
```

### Public API Surface

```rust
// From lib.rs
pub use error::{Error, Result};

pub use data::{
    FetchOptions,
    FetchPhase,
    Progress,
    DownloadSource,
    SourceType,
    SourceSelectionStrategy,
};

pub use effects::{
    HttpClient,
    BoxStream,
    Fetcher,
};

pub use transform::{
    StreamTransform,
};

// Feature-gated exports
#[cfg(feature = "reqwest")]
pub use effects::ReqwestClient;
```

---

## Implementation Phases

### Phase 1: Foundation & Consistency (v0.3.0)

**Goal**: Fix critical consistency issues and establish proper architecture

#### 1.1 Add Result<T> Type Alias
**File**: `src/error.rs`

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    // ... existing variants
}
```

**File**: `src/lib.rs`

```rust
pub use error::{Error, Result};
```

#### 1.2 Fix unwrap() Calls

**source.rs**:
- Replace `segments.last().unwrap()` with `.ok_or_else(|| Error::InvalidState("no segments".into()))?`
- Replace `.to_std().unwrap()` with `.map_err(|e| Error::InvalidTimestamp(e.to_string()))?`
- Replace `opts.cache.unwrap()` with `.ok_or_else(|| Error::MissingCacheConfig)?`

**effects.rs**:
- Replace `sem.acquire().await.unwrap()` with `.map_err(|e| Error::Internal("semaphore acquire failed".into()))?`
- Replace other unwraps with proper `?` propagation

#### 1.3 Refactor Module Structure

**Create new directories**:
```
src/data/
src/core/
src/effects/
src/transform/
```

**Move existing code**:
- `data.rs` → `data/mod.rs`, `data/options.rs`, `data/progress.rs`, `data/sources.rs`
- `effects.rs` → `effects/mod.rs`, split into individual modules
- Extract pure functions from `source.rs` → `core/` modules

#### 1.4 Builder Pattern Standardization

Add `#[must_use]` to all builder methods:

```rust
impl DownloadSource {
    #[must_use]
    pub fn priority(mut self, priority: u32) -> Self { ... }

    #[must_use]
    pub fn checksum(mut self, checksum: [u8; 32]) -> Self { ... }
}
```

**Add builders to missing structs**:

```rust
// src/data/options.rs
impl BatchOptions {
    pub fn new() -> Self {
        Self {
            max_concurrent: 4,
            fail_fast: false,
            retry_policy: BatchRetryPolicy::RetryCount(3),
        }
    }

    #[must_use]
    pub fn max_concurrent(mut self, limit: usize) -> Self { ... }

    #[must_use]
    pub fn fail_fast(mut self, fail_fast: bool) -> Self { ... }
}
```

**Success Criteria**:
- ✅ `Result<T>` type alias present and re-exported
- ✅ All unwrap() calls replaced with proper error handling
- ✅ No `unwrap()` in production code (test code excluded)
- ✅ Module structure follows data/core/effects pattern
- ✅ All public structs have `#[must_use]` on builder methods

---

### Phase 2: Core Features (v0.4.0)

**Goal**: Implement high-priority features from roadmap

#### 2.1 Retry Logic Improvements

**File**: `src/core/retry.rs`

Implement:
- Exponential backoff with jitter
- Retryable error classification (transient vs permanent)
- Configurable retry policy

```rust
pub struct RetryPolicy {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    jitter: bool,
}

impl RetryPolicy {
    pub fn should_retry(&self, error: &Error, attempt: u32) -> bool {
        // Classify error as transient or permanent
        match error {
            Error::Timeout { .. } | Error::Network { .. } => attempt < self.max_attempts,
            _ => false,
        }
    }

    pub fn next_delay(&self, attempt: u32) -> Duration {
        let base = self.base_delay.as_millis() * 2u64.pow(attempt.min(6));
        let delay = Duration::from_millis(base.min(self.max_delay.as_millis()));

        if self.jitter {
            // Add random jitter (0-100ms)
            let jitter = rand::thread_rng().gen_range(0..100);
            delay + Duration::from_millis(jitter)
        } else {
            delay
        }
    }
}
```

#### 2.2 Multi-Source Downloads

**File**: `src/effects/multi_source.rs`

Complete implementation:
- Priority-based source selection
- Geographic routing
- Race mode (parallel attempts)
- Source verification

```rust
pub struct MultiSourceFetcher<C: HttpClient> {
    client: Arc<C>,
    workspace_root: PathBuf,
}

impl<C: HttpClient> MultiSourceFetcher<C> {
    pub async fn fetch_multi_source(
        &self,
        sources: Vec<DownloadSource>,
        destination: &Path,
        options: FetchOptions,
    ) -> Result<FetchResult> {
        // Sort sources by priority
        // Try each source until success
        // Race mode: spawn all, use first success
        // Verify checksums match if multiple sources succeed
    }
}
```

#### 2.3 Bandwidth Limiting

**File**: `src/core/bandwidth.rs`

Complete token bucket implementation:

```rust
pub struct TokenBucket {
    tokens: AtomicF64,
    capacity: f64,
    refill_rate: f64,
    last_refill: AtomicInstant,
}

impl TokenBucket {
    pub async fn acquire(&self, bytes: usize) {
        // Refill tokens based on elapsed time
        // Wait if insufficient tokens
        // Deduct acquired tokens
    }
}
```

**File**: `src/effects/throttled.rs`

```rust
pub struct ThrottledStream<S> {
    inner: S,
    limiter: Arc<TokenBucket>,
}

impl<S> Stream for ThrottledStream<S>
where
    S: Stream<Item = Result<Bytes, Box<dyn std::error::Error + Send>>> + Unpin,
{
    // Acquire tokens before yielding each chunk
}
```

#### 2.4 Segmented Downloads

**File**: `src/core/segment.rs`

```rust
pub fn calculate_segments(
    file_size: u64,
    num_segments: u32,
) -> Result<Vec<Segment>> {
    // Validate server supports Range
    // Calculate segment boundaries
    // Ensure coverage without overlap
    // Validate final segment extends to EOF
}

pub struct Segment {
    pub index: u32,
    pub start: u64,
    pub end: u64,
}
```

**File**: `src/effects/segmented.rs`

```rust
pub struct SegmentedFetcher<C: HttpClient> {
    client: Arc<C>,
    workspace_root: PathBuf,
}

impl<C: HttpClient> SegmentedFetcher<C> {
    pub async fn fetch_segmented(
        &self,
        url: &str,
        destination: &Path,
        options: SegmentedOptions,
    ) -> Result<PathBuf> {
        // Get file size via HEAD
        // Calculate segments
        // Download segments in parallel
        // Reassemble with verification
    }
}
```

#### 2.5 Batch Downloads

**File**: `src/effects/batch.rs`

Complete implementation with dependency resolution:

```rust
pub struct BatchFetcher<C: HttpClient> {
    client: Arc<C>,
    workspace_root: PathBuf,
    max_concurrent: usize,
}

impl<C: HttpClient> BatchFetcher<C> {
    pub async fn fetch_batch(
        &self,
        jobs: Vec<BatchDownloadJob>,
        options: BatchOptions,
    ) -> Result<Vec<BatchResult>> {
        // Validate no circular dependencies
        // Topological sort
        // Execute with concurrency limit
        // Fail-fast or continue based on option
    }
}
```

**Success Criteria**:
- ✅ Retry logic with exponential backoff + jitter
- ✅ Multi-source with priority/fallback strategies
- ✅ Bandwidth limiting with token bucket
- ✅ Segmented downloads with parallel execution
- ✅ Batch downloads with dependency resolution
- ✅ All features use `Result<T>` for error handling
- ✅ All features have unit + integration tests

---

### Phase 3: Advanced Features (v0.5.0)

**Goal**: Implement medium-priority features

#### 3.1 Resumable Downloads

**File**: `src/effects/resumable.rs`

Complete implementation:
- HTTP Range request support
- Checksum state persistence
- Automatic resume on failure
- Partial file management

```rust
pub struct ResumeState {
    pub partial_path: PathBuf,
    pub bytes_downloaded: u64,
    pub last_attempt: Instant,
    pub checksum_state: Option<Vec<u8>>,
}

pub struct ResumableFetcher<C: HttpClient> {
    client: Arc<C>,
    workspace_root: PathBuf,
}

impl<C: HttpClient> ResumableFetcher<C> {
    pub async fn fetch_resumable(
        &self,
        url: &str,
        destination: &Path,
        options: ResumableOptions,
    ) -> Result<FetchResult> {
        // Check for existing partial file
        // Validate server supports Range
        // Calculate offset and send Range header
        // Append to existing file
        // Update progress state
    }
}
```

#### 3.2 Conditional Downloads

**File**: `src/data/options.rs` (extend FetchOptions)

```rust
pub struct ConditionalOptions {
    pub if_modified_since: Option<DateTime<Utc>>,
    pub if_none_match: Option<String>,  // ETag
    pub expected_checksum: Option<[u8; 32]>,
}

pub enum ConditionalResult {
    Downloaded(PathBuf),
    NotModified,
    LocalMatch(PathBuf),
}
```

**File**: `src/effects/fetcher.rs` (extend Fetcher)

```rust
impl Fetcher {
    pub async fn fetch_conditional(
        &self,
        url: &str,
        destination: &Path,
        options: FetchOptions,
        conditional: ConditionalOptions,
    ) -> Result<ConditionalResult> {
        // Add If-Modified-Since header
        // Add If-None-Match header
        // Check local file checksum if provided
        // Handle 304 Not Modified
    }
}
```

#### 3.3 Extended Progress Reporting

**File**: `src/data/progress.rs`

```rust
pub struct ExtendedProgress {
    pub base: Progress,
    pub speed: Option<u64>,          // bytes/sec
    pub eta: Option<Duration>,
    pub current_source: Option<String>,
    pub segments: Vec<SegmentProgress>,
    pub connection_stats: ConnectionStats,
}

pub struct ConnectionStats {
    pub latency: Duration,
    pub reconnection_count: u32,
    pub peak_speed: u64,
}
```

Implement speed calculator with exponential moving average:

```rust
pub struct SpeedCalculator {
    alpha: f64,
    current_speed: f64,
    last_update: Instant,
    last_bytes: u64,
}

impl SpeedCalculator {
    pub fn update(&mut self, bytes_downloaded: u64) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        if elapsed > 0.0 {
            let delta = bytes_downloaded - self.last_bytes;
            let instant_speed = delta as f64 / elapsed;

            // EMA: S_t = α * x_t + (1-α) * S_{t-1}
            self.current_speed = self.alpha * instant_speed
                               + (1.0 - self.alpha) * self.current_speed;

            self.last_update = now;
            self.last_bytes = bytes_downloaded;
        }

        self.current_speed as u64
    }
}
```

**Success Criteria**:
- ✅ Resumable downloads with Range header support
- ✅ Conditional downloads with ETag/If-Modified-Since
- ✅ Extended progress with speed and ETA
- ✅ All features use `Result<T>` for error handling

---

### Phase 4: Compression & Caching (v0.6.0)

**Goal**: Implement remaining high-value features

#### 4.1 Compression Support

**File**: `src/transform/decompress.rs`

```rust
pub trait StreamTransform: Send + Sync {
    fn transform(&mut self, chunk: &[u8]) -> Result<Vec<u8>, TransformError>;
    fn finalize(&mut self) -> Result<Vec<u8>, TransformError>;
}

pub struct GzipDecoder {
    decoder: flate2::Decoder<Vec<u8>>,
}

impl StreamTransform for GzipDecoder {
    fn transform(&mut self, chunk: &[u8]) -> Result<Vec<u8>, TransformError> {
        self.decoder.write_all(chunk)?;
        Ok(self.decoder.get_mut().to_vec())
    }

    fn finalize(&mut self) -> Result<Vec<u8>, TransformError> {
        self.decoder.try_finish()?;
        Ok(self.decoder.get_mut().to_vec())
    }
}
```

**File**: `src/effects/fetcher.rs` (add decompression option)

```rust
pub struct DecompressOptions {
    pub format: CompressionFormat,
    pub decompress_on_fly: bool,
    pub checksum_target: ChecksumTarget,
}
```

#### 4.2 HTTP Caching

**File**: `src/effects/cache.rs`

Complete implementation:
- HTTP cache semantics (RFC 7234)
- ETag and Last-Modified validation
- LRU eviction
- Cache metadata persistence

```rust
pub struct Cache {
    dir: PathBuf,
    max_size: Option<u64>,
    max_age: Option<Duration>,
}

pub struct CacheEntry {
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<DateTime<Utc>>,
    pub cached_at: DateTime<Utc>,
    pub size: u64,
    pub checksum: [u8; 32],
    pub access_count: u64,
}

impl Cache {
    pub async fn get(&self, url: &str) -> Result<Option<CacheEntry>> {
        // Calculate cache key from URL hash
        // Load metadata from cache.db
        // Check age and max_age
        // Return None if expired
    }

    pub async fn put(&self, url: &str, content: &[u8], metadata: &Metadata) -> Result<()> {
        // Check if exceeds max_size
        // Evict if needed (LRU)
        // Write cache.db metadata
        // Write cached file
    }

    pub async fn validate(&self, url: &str, server_meta: &Metadata) -> Result<bool> {
        // Send HEAD request with If-None-Match
        // Send HEAD request with If-Modified-Since
        // Return true if cache is valid (304)
    }
}
```

**Success Criteria**:
- ✅ Transparent decompression during download
- ✅ Multiple compression formats (gzip, brotli, zstd)
- ✅ HTTP caching with ETag/Last-Modified
- ✅ LRU eviction with size limits
- ✅ Persistent cache metadata

---

### Phase 5: Protocol Extensions & Testing (v0.7.0)

**Goal**: Extensible protocol support and comprehensive testing

#### 5.1 Protocol Abstraction

**File**: `src/effects/protocol.rs`

```rust
pub enum ProtocolOptions {
    Http(HttpOptions),
    Ftp(FtpOptions),
    S3(S3Options),
}

pub trait Protocol: Send + Sync {
    async fn fetch(&self, url: &str, dest: &Path) -> Result<()>;
}
```

**Note**: Implementation of FTP/S3 is **deferred** to v1.0+ as per roadmap

#### 5.2 Comprehensive Testing

**Unit Tests**:
- Core logic (retry delay, segment calculation)
- Data structures (builders, validation)
- Error handling

**Integration Tests**:
- Multi-source fallback scenarios
- Resumable download resume
- Segmented download reassembly
- Batch dependency resolution
- Cache hit/miss scenarios

**Property-Based Tests** (using proptest):
- Segment coverage completeness (union = [0, file_size))
- Bandwidth limit compliance (rate < limit)
- Progress percentage bounds (0.0 <= pct <= 100.0)

**Stress Tests**:
- 1000 small files (1KB each)
- Single 10GB file
- 100 concurrent downloads
- Rapid cancel/resume cycles

**Fault Injection Tests**:
- Random connection drops
- Corrupted chunk injection
- Slow server responses
- Simulated server changes during segmented download

**Success Criteria**:
- ✅ 95%+ code coverage
- ✅ All tests pass (including stress tests)
- ✅ Property tests cover invariants
- ✅ Fault injection tests handle edge cases

---

## Migration Guide

### v0.2.0 → v0.3.0 (Breaking Changes)

#### 1. Result<T> Type Alias Added

**Before**:
```rust
use pulith_fetch::FetchError;

async fn download() -> std::result::Result<PathBuf, FetchError> {
    fetcher.fetch(url, dest, options).await
}
```

**After**:
```rust
use pulith_fetch::{Result, Error};

async fn download() -> Result<PathBuf> {
    fetcher.fetch(url, dest, options).await
}
```

#### 2. Module Structure Changes

**Before**:
```rust
use pulith_fetch::FetchOptions;
use pulith_fetch::DownloadSource;
```

**After**:
```rust
use pulith_fetch::{FetchOptions, DownloadSource};
// No changes - re-exports maintain compatibility
```

#### 3. Builder Method Must-Use

**Before**:
```rust
let source = DownloadSource::new(url)
    .priority(0);  // Compiler warning: unused result
```

**After**:
```rust
let source = DownloadSource::new(url)
    .priority(0);  // Now emits must_use warning
```

---

## Dependencies

### New Dependencies Required

```toml
# Cargo.toml
[dependencies]
# Existing
pulith-fs = { path = "../pulith-fs" }
pulith-verify = { path = "../pulith-verify" }
reqwest = { version = "0.11", optional = true }
bytes = "1.5"
tokio = { version = "1.35", features = ["fs", "io-util"] }
futures-util = "0.3"

# New dependencies for v0.3.0+
thiserror = "1.0"  # Already present
rand = "0.8"       # For jitter in retry
chrono = "0.4"      # For timestamp handling
proptest = "1.4"     # For property tests

# Optional for advanced features
flate2 = { version = "1.0", optional = true }  # Gzip
brotli = { version = "3.4", optional = true }   # Brotli
zstd = { version = "0.12", optional = true }    # Zstd
rusqlite = { version = "0.29", optional = true } # For cache metadata

[dev-dependencies]
tempfile = "3.8"
mockall = "0.12"  # For testing
```

---

## Verification Criteria

### Phase 1: Foundation & Consistency
- [ ] `Result<T>` type alias present in `error.rs`
- [ ] `Result` re-exported in `lib.rs`
- [ ] All production `unwrap()` calls replaced
- [ ] Module structure: `data/`, `core/`, `effects/`, `transform/`
- [ ] All builder methods have `#[must_use]`
- [ ] `cargo clippy` passes without warnings
- [ ] `cargo test` passes all unit tests

### Phase 2: Core Features
- [ ] Retry logic with exponential backoff + jitter
- [ ] Multi-source downloads working
- [ ] Bandwidth limiting functional
- [ ] Segmented downloads functional
- [ ] Batch downloads with dependency resolution
- [ ] All new features have unit tests
- [ ] All new features have integration tests

### Phase 3: Advanced Features
- [ ] Resumable downloads with Range support
- [ ] Conditional downloads with ETag/If-Modified-Since
- [ ] Extended progress with speed/ETA
- [ ] All advanced features have tests

### Phase 4: Compression & Caching
- [ ] Gzip decompression working
- [ ] HTTP caching with ETag/Last-Modified
- [ ] LRU eviction working
- [ ] Cache metadata persistence
- [ ] All cache features tested

### Phase 5: Protocol & Testing
- [ ] Protocol abstraction in place
- [ ] 95%+ code coverage
- [ ] Property tests passing
- [ ] Stress tests passing
- [ ] Fault injection tests passing

---

## Anti-Patterns to Avoid

1. **Blocking in Async Context**: Never use `std::fs` directly in async functions
2. **Unbounded Buffers**: Always use sized buffers, avoid loading entire responses
3. **Ignoring Timeouts**: All HTTP requests must have reasonable timeouts
4. **Silent Error Swallowing**: Always log or propagate errors appropriately
5. **Hardcoded Dependencies**: No hardcoded reqwest imports, use trait abstraction
6. **Missing Documentation**: All public types must have `///` docs with examples

---

## Success Metrics

### Performance Targets
- **Single file**: 90%+ of network bandwidth utilization
- **Parallel segments**: 95%+ bandwidth utilization
- **Batch downloads**: <5% coordination overhead
- **Cache hit**: <10ms response time
- **Resume**: <100ms to detect and resume

### Quality Targets
- **Zero `unwrap()` in production code**
- **Zero unsafe code without safety justification**
- **95%+ code coverage**
- **Zero clippy warnings**
- **All public API documented**

---

## Conclusion

This redesign addresses critical consistency issues, aligns pulith-fetch with the pulith ecosystem, and implements the high-priority features from the roadmap. The modular structure following data/core/effects/transform separation ensures maintainability and testability. Each phase builds on the previous, allowing incremental releases (v0.3.0, v0.4.0, v0.5.0, v0.6.0, v0.7.0).

The implementation prioritizes:
1. **Consistency** with ecosystem patterns (Result<T>, module structure)
2. **Safety** (elimination of unwrap() panics)
3. **Correctness** (pure core, explicit effects)
4. **Performance** (zero-copy streaming, efficient algorithms)
5. **Testability** (trait abstraction, modular design)

Following this plan will transform pulith-fetch from a basic HTTP downloader into a comprehensive, production-ready file acquisition library.
