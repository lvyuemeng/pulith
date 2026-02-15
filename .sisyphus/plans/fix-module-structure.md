# Plan: Fix pulith-fetch Module Structure

## Current Issues

1. **config.rs** - Empty file (1 line), needs proper re-exports
2. **segment.rs** - Minimal (2 lines), needs proper re-exports  
3. **Missing files** - progress.rs, cache.rs, codec.rs, net.rs don't exist at root

## Required Module Files

### 1. config.rs (FIX - currently empty)
```rust
pub mod fetch_options;
pub mod sources;

pub use fetch_options::{FetchOptions, FetchPhase};
pub use sources::{DownloadSource, MultiSourceOptions, SourceSelectionStrategy, SourceType};
```

### 2. segment.rs (FIX - currently minimal)
```rust
pub mod segment;
pub mod validation;

pub use segment::{calculate_segments, Segment};
pub use validation::is_redirect;
```

### 3. progress.rs (CREATE)
```rust
pub mod progress;
pub mod extended_progress;

pub use progress::{Progress, PerformanceMetrics, PhaseTimings};
pub use extended_progress::{ExtendedProgress, ProgressReporter};
```

### 4. cache.rs (CREATE)
```rust
pub mod file_cache;
pub mod http_cache;

pub use file_cache::{Cache, CacheConfig, CacheEntry, CacheStats};
pub use http_cache::{CacheControl, CacheEntry as HttpCacheEntry, CacheError, CacheStats as HttpCacheStats, CacheValidation, ConditionalHeaders, HttpCache};
```

### 5. codec.rs (CREATE)
```rust
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

### 6. net.rs (CREATE)
```rust
pub mod http;
pub mod protocol;

pub use http::{HttpClient, BoxStream, ReqwestClient};
pub use protocol::{Protocol, Direction, TransferMetadata, TransferOptions, TransferStream, ProtocolClient, ProtocolRegistry, MockHttpClient, MockTransferStream};
```

## Directory Structure After Fix

```
src/
  lib.rs
  error.rs
  perf.rs
  
  rate.rs          # + rate/ subdirectory
  segment.rs       # + segment/ subdirectory  
  fetch.rs         # + fetch/ subdirectory
  config.rs        # + config/ subdirectory
  progress.rs      # + progress/ subdirectory
  cache.rs         # + cache/ subdirectory
  codec.rs         # + codec/ subdirectory
  net.rs           # + net/ subdirectory
```

## TODO

- [ ] Fix config.rs with proper re-exports
- [ ] Fix segment.rs with proper re-exports
- [ ] Create progress.rs
- [ ] Create cache.rs
- [ ] Create codec.rs
- [ ] Create net.rs
- [ ] Verify build passes
- [ ] Run tests
