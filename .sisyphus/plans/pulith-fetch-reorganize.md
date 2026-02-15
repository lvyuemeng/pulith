# Plan: pulith-fetch Reorganization

## TL;DR

> **Quick Summary**: Reorganize `pulith-fetch` from nested `mod.rs` structure to flat `.rs` files, group by functionality, and rename modules for clarity.
> 
> **Deliverables**: 
> - Flatten directory structure (no more `mod.rs` files in subdirectories)
> - Rename modules to match functionality
> - Update all imports across the crate
> - All tests pass
> 
> **Estimated Effort**: Medium
> **Parallel Execution**: NO - sequential (directory changes)
> **Critical Path**: Analyze → Rename files → Update imports → Test

---

## Context

### Original Request
User wants to:
1. Clean and distill functionality
2. Regroup by functionalites
3. Use clearer, readable names
4. Make it consistent
5. Use flat structure: `[file name].rs` instead of `[dirname]/mod.rs`

### Current Problems
| Issue | Current | Proposed |
|-------|---------|----------|
| Duplicate "cache" names | `effects/cache.rs` (file cache) vs `transform/cache.rs` (HTTP cache) | `file_cache.rs` vs `http_cache.rs` |
| Misleading module name | `core/retry.rs` exports `retry_delay` | `backoff.rs` |
| Nested structure | `data/mod.rs`, `core/mod.rs`, etc. | Flat `.rs` files |
| Inconsistent naming | Some modules use full names, others abbreviations | Consistent: descriptive but concise |

---

## Work Objectives

### Concrete Deliverables
- [ ] Flatten directory structure: move from nested `mod.rs` pattern to single `.rs` files
- [ ] Rename modules for clarity
- [ ] Update all `use` statements throughout the crate
- [ ] Verify `cargo build` and `cargo test` pass

### Must Have
- No breaking changes to public API (keep re-exports for backward compatibility)
- All tests pass after refactoring

### Must NOT Have
- Do not change any public API signatures
- Do not remove functionality, only reorganize

---

## Current → Proposed Mapping

### Core Utilities (Pure Functions)
| Current | Proposed | Notes |
|---------|----------|-------|
| `core/retry.rs` | `backoff.rs` | Renamed - exports `retry_delay` |
| `core/bandwidth.rs` | `bandwidth.rs` | Keep name - TokenBucket rate limiting |
| `core/segment.rs` | `segment.rs` | Keep - segment calculation |
| `core/validation.rs` | `validation.rs` | Keep - HTTP validation (is_redirect) |

### Data Types (Configuration & Types)
| Current | Proposed | Notes |
|---------|----------|-------|
| `data/options.rs` | `fetch_options.rs` | Renamed for clarity |
| `data/progress.rs` | `progress.rs` | Keep |
| `data/sources.rs` | `sources.rs` | Keep |
| `data/extended_progress.rs` | `extended_progress.rs` | Keep |

### HTTP & Networking
| Current | Proposed | Notes |
|---------|----------|-------|
| `effects/http.rs` | `http.rs` | Keep |
| `effects/protocol.rs` | `protocol.rs` | Keep |
| `effects/fetcher.rs` | `fetcher.rs` | Keep |

### Fetch Strategies
| Current | Proposed | Notes |
|---------|----------|-------|
| `effects/segmented.rs` | `segmented.rs` | Keep |
| `effects/multi_source.rs` | `multi_source.rs` | Keep |
| `effects/batch.rs` | `batch.rs` | Keep |
| `effects/resumable.rs` | `resumable.rs` | Keep |
| `effects/conditional.rs` | `conditional.rs` | Keep |

### Caching (Different purposes!)
| Current | Proposed | Notes |
|---------|----------|-------|
| `effects/cache.rs` | `file_cache.rs` | Renamed - stores actual file content |
| `transform/cache.rs` | `http_cache.rs` | Renamed - HTTP conditional requests |

### Rate Limiting
| Current | Proposed | Notes |
|---------|----------|-------|
| `effects/throttled.rs` | `throttled.rs` | Keep |

### Stream Transforms
| Current | Proposed | Notes |
|---------|----------|-------|
| `transform/decompress.rs` | `decompress.rs` | Keep |
| `transform/signature.rs` | `signature.rs` | Keep |
| `transform/verify.rs` | `verify.rs` | Keep |

### Performance
| Current | Proposed | Notes |
|---------|----------|-------|
| `perf/mod.rs` | `perf.rs` | Flattened |

---

## Directory Structure Changes

### Before (Nested with mod.rs)
```
src/
  lib.rs
  error.rs
  core/
    mod.rs
    retry.rs
    bandwidth.rs
    segment.rs
    validation.rs
  data/
    mod.rs
    options.rs
    progress.rs
    sources.rs
    extended_progress.rs
  effects/
    mod.rs
    http.rs
    fetcher.rs
    ...
  transform/
    mod.rs
    cache.rs
    decompress.rs
    ...
  perf/
    mod.rs
```

### After (Flat .rs files)
```
src/
  lib.rs
  error.rs
  backoff.rs           # was core/retry.rs
  bandwidth.rs         # was core/bandwidth.rs
  segment.rs           # was core/segment.rs
  validation.rs        # was core/validation.rs
  fetch_options.rs    # was data/options.rs
  progress.rs         # was data/progress.rs
  sources.rs           # was data/sources.rs
  extended_progress.rs # was data/extended_progress.rs
  http.rs             # was effects/http.rs
  protocol.rs         # was effects/protocol.rs
  fetcher.rs          # was effects/fetcher.rs
  segmented.rs        # was effects/segmented.rs
  multi_source.rs     # was effects/multi_source.rs
  batch.rs            # was effects/batch.rs
  resumable.rs        # was effects/resumable.rs
  conditional.rs      # was effects/conditional.rs
  file_cache.rs      # was effects/cache.rs (renamed)
  throttled.rs        # was effects/throttled.rs
  http_cache.rs       # was transform/cache.rs (renamed)
  decompress.rs       # was transform/decompress.rs
  signature.rs        # was transform/signature.rs
  verify.rs           # was transform/verify.rs
  perf.rs             # was perf/mod.rs
```

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (Rust with tokio, criterion for benches)
- **Automated tests**: YES (tests embedded in each .rs file)
- **Framework**: cargo test

### Verification Commands
```bash
cargo build --package pulith-fetch
cargo test --package pulith-fetch
```

---

## TODOs

- [ ] 1. Create new flat directory structure with all .rs files
  - Create all new files with content from old locations
  - Ensure directory structure is flat (no mod.rs files)

- [ ] 2. Rename modules in file contents
  - Update `mod module_name` declarations
  - Update `pub use module_name::...` statements

- [ ] 3. Update lib.rs to use new module structure
  - Change from `pub mod core;` to `pub mod backoff;`
  - Update all public exports

- [ ] 4. Update internal imports across all files
  - Change `use crate::core::...` to `use crate::backoff::...`
  - Change `use crate::data::...` to `use crate::fetch_options::...`

- [ ] 5. Build and fix any errors
  - Run `cargo build --package pulith-fetch`
  - Fix import path issues

- [ ] 6. Run tests to verify correctness
  - Run `cargo test --package pulith-fetch`
  - Fix any test failures

- [ ] 7. Remove old directory structure
  - Delete old core/, data/, effects/, transform/, perf/ directories

---

## Commit Strategy

| After Task | Message | Files |
|------------|---------|-------|
| All | `refactor(pulith-fetch): reorganize crate structure` | All files |

---

## Success Criteria

### Verification Commands
```bash
cargo build --package pulith-fetch    # Expected: success
cargo test --package pulith-fetch     # Expected: all tests pass
```

### Final Checklist
- [ ] No more `mod.rs` files in src/
- [ ] All modules are flat `.rs` files
- [ ] Module names reflect functionality
- [ ] No duplicate names (cache → file_cache + http_cache)
- [ ] All imports updated
- [ ] Build passes
- [ ] Tests pass
