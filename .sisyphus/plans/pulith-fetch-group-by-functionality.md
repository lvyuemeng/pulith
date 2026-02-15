# Plan: Group pulith-fetch Modules by Functionality

## TL;DR

> **Quick Summary**: Reorganize `pulith-fetch` from 25 flat files into 7 functional groups using split module style (module.rs + module/ subdirectory).
> 
> **Deliverables**: 
> - 7 functional modules: utils, types, net, fetch, cache, stream, verify
> - Split module structure (e.g., `utils.rs` + `utils/` subdir)
> - All 175+ tests pass
> - Benchmarks verified
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: NO - sequential (file moves)
> **Critical Path**: Group files → Update lib.rs → Fix imports → Test

---

## Context

### Current Structure (25 flat files)
```
src/
  lib.rs
  error.rs
  backoff.rs, bandwidth.rs, segment.rs, validation.rs
  fetch_options.rs, progress.rs, sources.rs, extended_progress.rs
  http.rs, protocol.rs
  fetcher.rs, segmented.rs, multi_source.rs, batch.rs, resumable.rs, conditional.rs
  file_cache.rs, http_cache.rs
  throttled.rs, decompress.rs
  signature.rs, verify.rs
  perf.rs
```

### Proposed Structure (7 functional groups)
```
src/
  lib.rs
  error.rs
  
  utils.rs              # Re-exports: backoff, bandwidth, segment, validation
  utils/
    backoff.rs
    bandwidth.rs
    segment.rs
    validation.rs
  
  types.rs              # Re-exports: fetch_options, progress, sources, extended_progress
  types/
    fetch_options.rs
    progress.rs
    sources.rs
    extended_progress.rs
  
  net.rs                # Re-exports: http, protocol
  net/
    http.rs
    protocol.rs
  
  fetch.rs              # Re-exports: fetcher, segmented, multi_source, batch, resumable, conditional
  fetch/
    fetcher.rs
    segmented.rs
    multi_source.rs
    batch.rs
    resumable.rs
    conditional.rs
  
  cache.rs              # Re-exports: file_cache, http_cache
  cache/
    file_cache.rs
    http_cache.rs
  
  stream.rs             # Re-exports: throttled, decompress
  stream/
    throttled.rs
    decompress.rs
  
  verify.rs             # Re-exports: signature, verify
  verify/
    signature.rs
    verify.rs
  
  perf.rs
```

---

## Work Objectives

### Concrete Deliverables
- [ ] Create 7 group module files (utils.rs, types.rs, net.rs, fetch.rs, cache.rs, stream.rs, verify.rs)
- [ ] Create 7 subdirectories with module files
- [ ] Move existing .rs files into appropriate subdirectories
- [ ] Update lib.rs to use new grouped structure
- [ ] Update all internal import paths
- [ ] Verify benchmarks compile and run
- [ ] Verify all tests pass

### Must Have
- No breaking changes to public API
- All existing tests pass
- Benchmarks work

### Must NOT Have
- Do not delete functionality
- Do not change module logic

---

## File Movements

### Group 1: utils (pure algorithms)
| Source | Destination |
|--------|-------------|
| backoff.rs | utils/backoff.rs |
| bandwidth.rs | utils/bandwidth.rs |
| segment.rs | utils/segment.rs |
| validation.rs | utils/validation.rs |

### Group 2: types (data structures)
| Source | Destination |
|--------|-------------|
| fetch_options.rs | types/fetch_options.rs |
| progress.rs | types/progress.rs |
| sources.rs | types/sources.rs |
| extended_progress.rs | types/extended_progress.rs |

### Group 3: net (networking)
| Source | Destination |
|--------|-------------|
| http.rs | net/http.rs |
| protocol.rs | net/protocol.rs |

### Group 4: fetch (download strategies)
| Source | Destination |
|--------|-------------|
| fetcher.rs | fetch/fetcher.rs |
| segmented.rs | fetch/segmented.rs |
| multi_source.rs | fetch/multi_source.rs |
| batch.rs | fetch/batch.rs |
| resumable.rs | fetch/resumable.rs |
| conditional.rs | fetch/conditional.rs |

### Group 5: cache (caching)
| Source | Destination |
|--------|-------------|
| file_cache.rs | cache/file_cache.rs |
| http_cache.rs | cache/http_cache.rs |

### Group 6: stream (stream processing)
| Source | Destination |
|--------|-------------|
| throttled.rs | stream/throttled.rs |
| decompress.rs | stream/decompress.rs |

### Group 7: verify (verification)
| Source | Destination |
|--------|-------------|
| signature.rs | verify/signature.rs |
| verify.rs | verify/verify.rs |

---

## Module File Templates

### utils.rs
```rust
//! Utility functions for rate limiting, retry logic, and validation.

pub mod backoff;
pub mod bandwidth;
pub mod segment;
pub mod validation;

pub use backoff::retry_delay;
pub use bandwidth::{TokenBucket, AdaptiveConfig, RateMetrics};
pub use segment::{calculate_segments, Segment};
pub use validation::is_redirect;
```

### types.rs
```rust
//! Data types for fetch operations.

pub mod fetch_options;
pub mod progress;
pub mod sources;
pub mod extended_progress;

pub use fetch_options::{FetchOptions, FetchPhase};
pub use progress::{Progress, PerformanceMetrics, PhaseTimings};
pub use sources::{DownloadSource, MultiSourceOptions, SourceSelectionStrategy, SourceType};
pub use extended_progress::{ExtendedProgress, ProgressReporter};
```

### net.rs
```rust
//! Network and protocol abstractions.

pub mod http;
pub mod protocol;

pub use http::{HttpClient, BoxStream, ReqwestClient};
pub use protocol::{Protocol, Direction, TransferMetadata, TransferOptions, TransferStream, ProtocolClient, ProtocolRegistry, MockHttpClient, MockTransferStream};
```

### fetch.rs
```rust
//! Fetch strategies and download implementations.

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

### cache.rs
```rust
//! Caching implementations.

pub mod file_cache;
pub mod http_cache;

pub use file_cache::{Cache, CacheConfig, CacheEntry, CacheStats};
pub use http_cache::{CacheControl, CacheEntry as HttpCacheEntry, CacheError, CacheStats as HttpCacheStats, CacheValidation, ConditionalHeaders, HttpCache};
```

### stream.rs
```rust
//! Stream processing and transformations.

pub mod throttled;
pub mod decompress;

pub use throttled::{ThrottledStream, AsyncThrottledStream};
pub use decompress::{StreamTransform, TransformError};
```

### verify.rs
```rust
//! Verification and signature checking.

pub mod signature;
pub mod verify;

pub use signature::{
    verify_signature, KeyPurpose, KeyUsage, MockVerifier, PublicKey, PublicKeyFormat, Signature,
    SignatureAlgorithm, SignatureConfig, SignatureFormat, SignatureManager, SignatureVerifier,
};
pub use verify::{
    parse_multiple_checksums, verify_checksum, verify_multiple_checksums, ChecksumConfig,
    MultiVerifier, StreamVerifier,
};
```

---

## Updated lib.rs Structure

```rust
mod error;

pub mod utils;
pub mod types;
pub mod net;
pub mod fetch;
pub mod cache;
pub mod stream;
pub mod verify;
pub mod perf;

pub use error::{Error, Result};

// Re-export commonly used items at crate root
pub use utils::{retry_delay, TokenBucket, calculate_segments, Segment, is_redirect};
pub use types::{FetchOptions, FetchPhase, Progress, DownloadSource, MultiSourceOptions, SourceSelectionStrategy, SourceType, ExtendedProgress, ProgressReporter};
pub use net::{HttpClient, BoxStream, ReqwestClient, Protocol};
pub use fetch::{Fetcher, SegmentedFetcher, MultiSourceFetcher, ConditionalFetcher};
pub use stream::{ThrottledStream, AsyncThrottledStream};
pub use cache::{HttpCache, Cache};
pub use verify::{verify_checksum, verify_signature, ChecksumConfig, SignatureVerifier};
```

---

## Import Path Changes

### Before
```rust
use crate::backoff::retry_delay;
use crate::data::FetchOptions;
use crate::effects::Fetcher;
use crate::transform::verify_checksum;
```

### After
```rust
use crate::utils::retry_delay;
use crate::types::FetchOptions;
use crate::fetch::Fetcher;
use crate::verify::verify_checksum;
```

---

## TODOs

- [ ] 1. Create 7 group module files (utils.rs, types.rs, net.rs, fetch.rs, cache.rs, stream.rs, verify.rs)
- [ ] 2. Create 7 subdirectories
- [ ] 3. Move existing files into subdirectories
- [ ] 4. Update lib.rs with new module structure
- [ ] 5. Update imports in all moved files
- [ ] 6. Update test imports
- [ ] 7. Verify cargo build succeeds
- [ ] 8. Run cargo test
- [ ] 9. Verify benchmarks compile: cargo build --benches
- [ ] 10. Run benchmarks: cargo bench
- [ ] 11. Clean up old flat files

---

## Verification Commands

```bash
# Build
cargo build --package pulith-fetch

# Test
cargo test --package pulith-fetch

# Benchmarks
cargo build --benches --package pulith-fetch
cargo bench --package pulith-fetch
```

---

## Success Criteria

- [ ] All 175+ unit tests pass
- [ ] All 4 integration tests pass
- [ ] All 9 doc tests pass
- [ ] Benchmarks compile successfully
- [ ] No compiler warnings (or minimal acceptable warnings)
