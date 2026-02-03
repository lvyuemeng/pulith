# pulith-fetch Benchmarks Update Plan

## TL;DR

> **Quick Summary**: Fix critical compilation errors in existing benchmarks caused by API changes and expand coverage to include new TokenBucket adaptive features, AsyncThrottledStream, and improved memory tracking.
>
> **Deliverables**:
> - Fixed `memory_usage.rs` with corrected `calculate_segments` API usage
> - Fixed `stream_processing.rs` with proper error type handling
> - Expanded `token_bucket.rs` to benchmark adaptive rate limiting features
> - New `segment_bench.rs` for Segment struct operations
> - Modernized memory tracking approach
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - Update files independently
> **Critical Path**: Fix compilation errors first → Add new feature benchmarks

---

## Context

### Original Request
Read `crates/pulith-fetch/benches/` and propose a plan to update it.

### Interview Summary
**Key Discussions**:
- User requested comprehensive update (not minimal fix)
- Critical: calculate_segments API changed from segment_size to num_segments
- Critical: MockStream has type mismatch with ThrottledStream error types
- TokenBucket has evolved with adaptive rate limiting (new_adaptive, set_rate, metrics)
- New AsyncThrottledStream not benchmarked
- Custom GlobalAlloc memory tracking is fragile and uses deprecated APIs

### Research Findings
- `segment.rs:26-29`: calculate_segments(file_size, num_segments) -> Result<Vec<Segment>, Error>
- `bandwidth.rs:172-410`: TokenBucket with AdaptiveConfig, RateMetrics, adaptive rate limiting
- `throttled.rs:16-61`: ThrottledStream and new AsyncThrottledStream with with_bucket constructor
- Memory tracking uses std::alloc::System which is deprecated behavior

### Metis Review
**Identified Gaps** (addressed):
- Custom allocator conflicts: Modern Rust may usejemalloc/mimalloc on some platforms
- MockStream error types: Must match ThrottledStream's expected `Box<dyn Error + Send>`
- Scope creep risk: Focus on core benchmarks, don't add benchmarks for HTTP/effects layer (separate concern)
- Edge cases: Zero-sized segments, maximum concurrent tasks, adaptive rate adjustment timing

---

## Work Objectives

### Core Objective
Update all pulith-fetch benchmarks to:
1. Fix compilation errors caused by API evolution
2. Add benchmarks for new TokenBucket adaptive features
3. Add AsyncThrottledStream benchmarks
4. Modernize memory tracking to be more reliable
5. Add Segment operations benchmarks

### Concrete Deliverables
- `crates/pulith-fetch/benches/memory_usage.rs` - Fixed API usage, improved tracking
- `crates/pulith-fetch/benches/stream_processing.rs` - Fixed type errors, added AsyncThrottledStream
- `crates/pulith-fetch/benches/token_bucket.rs` - Added adaptive feature benchmarks
- `crates/pulith-fetch/benches/segment_bench.rs` - NEW file for Segment operations

### Definition of Done
- [x] `cargo bench --bench memory_usage` compiles and runs
- [x] `cargo bench --bench stream_processing` compiles and runs
- [x] `cargo bench --bench token_bucket` compiles and runs
- [x] New segment_bench.rs benchmark file exists and runs
- [x] All benchmarks pass with no type errors or warnings

### Must Have
- All existing benchmarks compile without errors
- calculate_segments uses correct API (num_segments, not segment_size)
- MockStream error type matches ThrottledStream expectations
- Adaptive TokenBucket features are benchmarked

### Must NOT Have (Guardrails)
- NO changes to library source code (only benchmarks)
- NO benchmarks for HTTP layer or network operations (separate concern)
- NO breaking changes to benchmark file names (preserved for cargo bench)
- NO changes to Cargo.toml dev-dependencies

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES - criterion 0.5 with async_tokio
- **User wants tests**: Tests-after (existing benchmarks, update to fix/add)
- **Framework**: criterion with async_tokio feature
- **QA approach**: Manual verification via cargo bench

### Automated Verification (All Tasks)
Each task includes verification via cargo bench commands:

**By Deliverable Type:**

| Type | Verification Tool | Automated Procedure |
|------|------------------|---------------------|
| **Benchmark compilation** | Bash cargo | `cargo check --bench <name>` → Exit code 0 |
| **Benchmark execution** | Bash cargo | `cargo bench --bench <name>` → 0 failures |
| **No warnings** | Bash cargo | `cargo check --bench <name> 2>&1` → No warnings |

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately):
├── Task 1: Fix memory_usage.rs - calculate_segments API
└── Task 2: Fix stream_processing.rs - MockStream type mismatch

Wave 2 (After Wave 1):
├── Task 3: Expand token_bucket.rs - adaptive features
└── Task 4: Create segment_bench.rs - Segment operations

Wave 3 (After Wave 2):
└── Task 5: Verify all benchmarks compile and run
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 3, 4, 5 | 2 |
| 2 | None | 3, 4, 5 | 1 |
| 3 | 1, 2 | 5 | 4 |
| 4 | 1, 2 | 5 | 3 |
| 5 | 3, 4 | None | None (final) |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Agents |
|------|-------|-------------------|
| 1 | 1, 2 | Both fixed-target, can run in parallel |
| 2 | 3, 4 | New feature additions |
| 3 | 5 | Verification task |

---

## TODOs

- [x] 1. Fix memory_usage.rs - calculate_segments API and memory tracking

  **What to do**:
  - Change calculate_segments call to use `num_segments` instead of `segment_size`
  - Calculate num_segments as: file_size / segment_size (capped to reasonable max)
  - Update error handling to use Result pattern
  - Keep memory tracking logic but add safety for allocator conflicts

  **Must NOT do**:
  - Don't modify library source code
  - Don't remove memory tracking entirely (useful for relative comparisons)

  **Recommended Agent Profile**:
  > Category: `quick` (targeted, contained fix)
  - Reason: Single file, well-defined scope, no architectural decisions
  - Skills: None required for this straightforward API fix

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Tasks 3, 4, 5
  - **Blocked By**: None (can start immediately)

  **References**:
  - `crates/pulith-fetch/src/core/segment.rs:26-70` - calculate_segments API signature and implementation
  - `crates/pulith-fetch/benches/memory_usage.rs:60-68` - Current incorrect usage to fix

  **Acceptance Criteria**:
  - [x] cargo check --bench memory_usage → 0 errors
  - [x] calculate_segments receives (file_size, num_segments) where num_segments = file_size / (1024*1024)
  - [x] Result pattern properly handled with ? operator
  - [x] Memory tracking continues to work for relative comparisons

  **Evidence to Capture**:
  - [x] Terminal output from `cargo check --bench memory_usage`

  **Commit**: YES
  - Message: `fix(bench): correct calculate_segments API usage in memory benchmarks`
  - Files: `crates/pulith-fetch/benches/memory_usage.rs`

---

- [x] 2. Fix stream_processing.rs - MockStream type mismatch

  **What to do**:
  - Fix MockStream to return `Result<Bytes, Box<dyn std::error::Error + Send>>`
  - Use `Box::new(String::from("error"))` or similar for error type
  - Ensure Stream trait implementation compiles
  - Test compilation after fix

  **Must NOT do**:
  - Don't modify ThrottledStream implementation
  - Don't change benchmark structure unnecessarily

  **Recommended Agent Profile**:
  > Category: `quick` (targeted fix)
  - Reason: Single type mismatch, straightforward fix

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Tasks 3, 4, 5
  - **Blocked By**: None (can start immediately)

  **References**:
  - `crates/pulith-fetch/src/effects/throttled.rs:26-61` - ThrottledStream::new signature
  - `crates/pulith-fetch/benches/stream_processing.rs:11-38` - MockStream implementation to fix

  **Acceptance Criteria**:
  - [x] cargo check --bench stream_processing → 0 errors
  - [x] MockStream::Item is `Result<Bytes, Box<dyn std::error::Error + Send>>`
  - [x] ThrottledStream compiles with MockStream

  **Evidence to Capture**:
  - [x] Terminal output from `cargo check --bench stream_processing`

  **Commit**: YES
  - Message: `fix(bench): correct MockStream error type for ThrottledStream compatibility`
  - Files: `crates/pulith-fetch/benches/stream_processing.rs`

---

- [x] 3. Expand token_bucket.rs - Add adaptive feature benchmarks

  **What to do**:
  - Add benchmarks for `TokenBucket::new_adaptive()` with custom AdaptiveConfig
  - Add benchmarks for `TokenBucket::set_rate()` dynamic rate adjustment
  - Add benchmarks for `TokenBucket::check_and_adjust_rate()` congestion detection
  - Add benchmarks for `RateMetrics` collection and retrieval
  - Keep existing benchmarks for basic acquire/try_acquire

  **Must NOT do**:
  - Don't remove existing benchmarks (they test basic functionality)
  - Don't modify library source code

  **Recommended Agent Profile**:
  > Category: `unspecified-medium` (feature expansion)
  - Reason: Adding new benchmark cases, moderate complexity

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 4)
  - **Blocks**: Task 5
  - **Blocked By**: Tasks 1, 2 (must compile first)

  **References**:
  - `crates/pulith-fetch/src/core/bandwidth.rs:25-42` - AdaptiveConfig struct
  - `crates/pulith-fetch/src/core/bandwidth.rs:58-141` - RateMetrics struct
  - `crates/pulith-fetch/src/core/bandwidth.rs:200-212` - new_adaptive constructor
  - `crates/pulith-fetch/src/core/bandwidth.rs:336-353` - check_and_adjust_rate
  - `crates/pulith-fetch/src/core/bandwidth.rs:405-409` - set_rate

  **Acceptance Criteria**:
  - [x] cargo check --bench token_bucket → 0 errors, 0 warnings
  - [x] New benchmark group for adaptive_rate_limiting
  - [x] New benchmark for set_rate_dynamics
  - [x] New benchmark for metrics_collection
  - [x] All existing benchmarks still pass

  **Evidence to Capture**:
  - [x] Terminal output from `cargo check --bench token_bucket`
  - [x] List of new benchmark functions added

  **Commit**: YES
  - Message: `feat(bench): add TokenBucket adaptive rate limiting benchmarks`
  - Files: `crates/pulith-fetch/benches/token_bucket.rs`

---

- [x] 4. Create segment_bench.rs - Segment operations benchmarks

  **What to do**:
  - Create new benchmark file for Segment struct operations
  - Benchmark Segment::new() creation overhead
  - Benchmark calculate_segments function with various file sizes and segment counts
  - Benchmark Segment iteration and processing
  - Include memory tracking for segment allocation

  **Must NOT do**:
  - Don't duplicate benchmarks from memory_usage.rs (different focus)
  - Don't include benchmarks for unrelated functionality

  **Recommended Agent Profile**:
  > Category: `unspecified-medium` (new file creation)
  - Reason: New benchmark file, needs proper structure

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Task 3)
  - **Blocks**: Task 5
  - **Blocked By**: Tasks 1, 2 (must compile first)

  **References**:
  - `crates/pulith-fetch/src/core/segment.rs:1-70` - Segment struct and calculate_segments
  - `crates/pulith-fetch/benches/memory_usage.rs:1-50` - Benchmark structure reference

  **Acceptance Criteria**:
  - [x] New file created: `crates/pulith-fetch/benches/segment_bench.rs`
  - [x] cargo bench --bench segment_bench runs successfully
  - [x] Benchmarks cover: creation, calculation, iteration
  - [x] Includes Throughput tracking for various file sizes

  **Evidence to Capture**:
  - [x] New file listing via glob
  - [x] Terminal output from `cargo bench --bench segment_bench`

  **Commit**: YES
  - Message: `feat(bench): add Segment operations benchmarks`
  - Files: `crates/pulith-fetch/benches/segment_bench.rs` (new)

---

- [x] 5. Verify all benchmarks compile and run

  **What to do**:
  - Run cargo check for all benchmark files
  - Run cargo bench for all benchmark files
  - Verify no warnings across all benchmarks
  - Document any remaining issues

  **Must NOT do**:
  - Don't make further changes unless critical issues found

  **Recommended Agent Profile**:
  > Category: `quick` (verification task)
  - Reason: Verification-focused, no implementation changes

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (final task)
  - **Blocks**: None (final task)
  - **Blocked By**: Tasks 1, 2, 3, 4

  **References**:
  - `crates/pulith-fetch/Cargo.toml:40-50` - Benchmark registrations

  **Acceptance Criteria**:
  - [x] cargo check --bench memory_usage → 0 errors, 0 warnings
  - [x] cargo check --bench stream_processing → 0 errors, 0 warnings
  - [x] cargo check --bench token_bucket → 0 errors, 0 warnings
  - [x] cargo check --bench segment_bench → 0 errors, 0 warnings
  - [x] cargo bench --bench memory_usage runs without failure
  - [x] cargo bench --bench stream_processing runs without failure
  - [x] cargo bench --bench token_bucket runs without failure
  - [x] cargo bench --bench segment_bench runs without failure

  **Evidence to Capture**:
  - [x] All benchmark check outputs (4 files)
  - [x] All benchmark run outputs (4 files)

  **Commit**: NO (verification only)

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `fix(bench): correct calculate_segments API usage` | memory_usage.rs | cargo check --bench memory_usage |
| 2 | `fix(bench): correct MockStream error type` | stream_processing.rs | cargo check --bench stream_processing |
| 3 | `feat(bench): add TokenBucket adaptive rate limiting benchmarks` | token_bucket.rs | cargo check --bench token_bucket |
| 4 | `feat(bench): add Segment operations benchmarks` | segment_bench.rs (new) | cargo check --bench segment_bench |

---

## Success Criteria

### Verification Commands
```bash
# Check all benchmarks compile
cargo check --bench memory_usage
cargo check --bench stream_processing
cargo check --bench token_bucket
cargo check --bench segment_bench

# Run all benchmarks
cargo bench --bench memory_usage
cargo bench --bench stream_processing
cargo bench --bench token_bucket
cargo bench --bench segment_bench
```

### Final Checklist
- [x] All "Must Have" present (fixes applied, new features benchmarked)
- [x] All "Must NOT Have" absent (no source code changes, preserved benchmark names)
- [x] All benchmarks compile without errors or warnings
- [x] All benchmarks execute successfully
- [x] No new compilation warnings introduced
