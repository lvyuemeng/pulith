# Fix Benchmark Files in pulith-fetch

## Issue Description
The benchmark files in `crates/pulith-fetch/benches/` have compilation errors:

### memory_usage.rs errors:
1. `SegmentedDownloader` doesn't exist in `effects::segmented` module
2. Module `segment` is private (should use `core::calculate_segments` instead of `core::segment::calculate_segments`)
3. Module `segmented` is private (not exported in effects module)
4. Type mismatches: expected `usize`, found `u64`

### stream_processing.rs errors:
1. Type mismatch: `<MockStream as Stream>::Item` expected `Result<Bytes, Box<dyn Error + Send>>` but found `Result<_, &'static str>`
2. Dereference errors: trying to dereference integer types
3. Stream trait bounds not satisfied for `ThrottledStream`
4. Type annotations needed

## Work Plan

### Task 1: Fix memory_usage.rs
- [x] Remove import of non-existent `SegmentedDownloader`
- [x] Fix import to use `core::calculate_segments` instead of `core::segment::calculate_segments`
- [x] Fix type mismatches (convert u64 to usize where needed)
- [x] Replace any reference to SegmentedDownloader with appropriate struct

### Task 2: Fix stream_processing.rs  
- [x] Fix MockStream implementation to return proper error type `Box<dyn std::error::Error + Send>`
- [x] Fix dereference errors in throttle_stream benchmark
- [x] Ensure ThrottledStream implements proper Stream traits
- [x] Add type annotations where needed

### Task 3: Verify fixes
- [x] Run `cargo check` on benchmark files to ensure no compilation errors
- [x] Run `cargo bench` to ensure benchmarks execute properly

## Technical Approach
1. For memory_usage.rs: Replace the import and fix the type conversions
2. For stream_processing.rs: Update MockStream to return proper error types and fix trait implementations

## Definition of Done
- [x] All benchmark files compile without errors
- [x] Benchmarks run successfully with `cargo bench`
- [x] No regressions in existing functionality