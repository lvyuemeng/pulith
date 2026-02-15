# Draft: pulith-fetch Refactoring Analysis

## Current Structure Overview

### Modules by Category

**core/** - Pure transformations
- `retry.rs` → exports `retry_delay` - exponential backoff calculation
- `bandwidth.rs` → exports `TokenBucket` - rate limiting algorithm
- `segment.rs` → exports `calculate_segments`, `Segment` - parallel download segmentation
- `validation.rs` → exports `is_redirect` - HTTP status code detection

**data/** - Immutable types/config
- `options.rs` → `FetchOptions`, `FetchPhase` - main configuration
- `progress.rs` → `Progress`, `PerformanceMetrics`, `PhaseTimings` - progress reporting
- `sources.rs` → `DownloadSource`, `MultiSourceOptions`, `SourceSelectionStrategy` - multi-source config

**effects/** - I/O operations
- `http.rs` → `HttpClient` trait, `BoxStream`, `ReqwestClient` - HTTP abstraction
- `fetcher.rs` → `Fetcher` - main single-source download
- `segmented.rs` → `SegmentedFetcher` - parallel segmented download
- `multi_source.rs` - multi-source download logic
- `batch.rs` - batch download operations
- `resumable.rs` - resumable downloads
- `cache.rs` → **FILE-BASED content caching** (stores downloaded files)
- `throttled.rs` → `ThrottledStream`, `AsyncThrottledStream` - bandwidth limiting stream
- `conditional.rs` - conditional fetching with metadata
- `protocol.rs` - protocol abstraction layer

**transform/** - Stream transformations
- `cache.rs` → **HTTP caching with headers** (ETag, Last-Modified, Cache-Control) - for conditional requests
- `decompress.rs` - decompression transforms
- `signature.rs` - signature verification
- `verify.rs` - checksum verification

## Issues Identified

1. **Duplicate "cache" modules with unclear purposes:**
   - `effects/cache.rs` = file-based content cache (stores actual file content)
   - `transform/cache.rs` = HTTP caching (ETag/Last-Modified headers for conditional requests)

2. **Naming inconsistencies:**
   - `core/retry.rs` exports `retry_delay` - module name doesn't match primary export
   - Should be `core/backoff.rs`

3. **Grouping could be clearer:**
   - All fetch strategies (segmented, multi-source, batch, resumable) could be grouped
   - Rate limiting related: `bandwidth.rs` (core) + `throttled.rs` (effects)

## Proposed Refactoring

### 1. Rename modules for clarity
- `core/retry.rs` → `core/backoff.rs`
- `transform/cache.rs` → `transform/http_cache.rs` 
- `effects/cache.rs` → `effects/file_cache.rs`

### 2. Re-export with clearer names
Keep backward compatibility with type aliases if needed

### 3. Consider reorganizing by capability
Could potentially flatten or group related modules
