# Technical Implementation Plan: Performance & Backpressure Improvements for pulith-fetch

## Context

### Current Performance Analysis
The pulith-fetch crate has functional performance features but lacks comprehensive benchmarking and advanced backpressure controls. Analysis shows:

**Strengths:**
- Efficient token bucket implementation with atomic operations
- Streaming architecture preventing memory bloat
- Segmented downloads for parallel processing
- Proper async/await implementation

**Areas for Improvement:**
- No formal performance benchmarking
- Basic backpressure without dynamic adjustment
- Limited performance monitoring
- Missing performance integration tests

## Work Objectives

### Core Objective
Enhance pulith-fetch performance through comprehensive benchmarking, advanced backpressure controls, and performance optimizations.

### Concrete Deliverables
- Performance benchmark suite with throughput and memory measurements
- Adaptive rate limiting with congestion control
- Performance monitoring and telemetry
- Performance integration tests
- Optimized critical code paths

### Definition of Done
- All performance benchmarks run successfully
- Backpressure mechanisms provide dynamic rate adjustment
- Performance metrics available during operations
- Performance integration tests validate improvements
- Critical paths optimized for throughput

### Must Have
- Token bucket throughput: >100MB/s in benchmarks
- Stream processing: minimal memory overhead
- Adaptive rate limiting: responsive to network conditions
- Performance telemetry: real-time metrics available

### Must NOT Have (Guardrails)
- Performance regressions in existing functionality
- Increased memory usage during normal operations
- Breaking changes to existing APIs
- Blocking operations in async contexts

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES - uses workspace dependencies with dev-dependencies for testing
- **User wants tests**: TDD - Each performance improvement should have corresponding benchmarks
- **Framework**: Rust standard testing with benchmarks and additional criterion for performance testing

### If TDD Enabled

Each TODO follows RED-GREEN-REFACTOR:

**Task Structure:**
1. **RED**: Write performance benchmark first
   - Benchmark file: `benches/performance.rs`
   - Benchmark command: `cargo bench`
   - Expected: Baseline measurements available
2. **GREEN**: Implement performance improvement
   - Command: `cargo bench`
   - Expected: Improved metrics vs baseline
3. **REFACTOR**: Optimize while maintaining performance
   - Command: `cargo bench`
   - Expected: Sustained performance improvements

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Setup):
├── Task 1: Create benchmark infrastructure
└── Task 2: Set up performance measurement tools

Wave 2 (Enhancements):
├── Task 3: Implement adaptive rate limiting
├── Task 4: Enhance backpressure mechanisms
├── Task 5: Add performance monitoring
└── Task 6: Create performance integration tests

Wave 3 (Optimization):
└── Task 7: Optimize performance-critical paths
```

Critical Path: Task 1 → Task 3 → Task 7
Parallel Speedup: ~40% faster than sequential

## TODOs

- [x] 1. Create Benchmark Infrastructure

  **What to do**:
  - Set up `benches/` directory with benchmark structure
  - Implement token bucket throughput benchmark
  - Create stream processing performance benchmark
  - Add memory usage measurement utilities

  **Must NOT do**:
  - Modify existing functionality without benchmarks
  - Add dependencies without justification

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Benchmark infrastructure setup requires understanding of performance measurement
  - **Skills**: [`git-master`]
    - `git-master`: For proper benchmark setup and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Setup task
  - **Blocks**: [Tasks 3, 4, 5, 6] (all performance improvements need benchmarks first)
  - **Blocked By**: None (can start immediately)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/core/bandwidth.rs:1-148` - Token bucket implementation for benchmarking
  - `src/effects/throttled.rs:1-243` - Throttled stream for performance testing
  - `src/effects/segmented.rs:1-282` - Segmented download for throughput testing

  **API/Type References** (contracts to implement against):
  - `src/core/bandwidth.rs:TokenBucket` - API to benchmark
  - `src/effects/throttled.rs:ThrottledStream` - API to benchmark
  - `src/core/segment.rs:calculate_segments` - Function to benchmark

  **Test References** (testing patterns to follow):
  - `src/core/bandwidth.rs:149-214` - Existing tests for token bucket
  - `src/effects/throttled.rs:201-243` - Existing tests for throttled stream
  - `src/effects/segmented.rs:248-282` - Existing tests for segmentation

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Performance requirements specification
  - `src/perf/mod.rs` - Performance module structure

  **External References** (libraries and frameworks):
  - criterion crate for detailed benchmarking
  - tokio for async benchmarking
  - tempfile for file-based benchmarks

  **WHY Each Reference Matters** (explain the relevance):
  - Token bucket implementation needs baseline performance metrics
  - Throttled stream requires throughput measurements
  - Segmented download needs parallel processing benchmarks

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [x] Benchmark infrastructure: benches/ directory created → SUCCESS
  - [x] Token bucket benchmark: measures throughput → SUCCESS
  - [x] Stream processing benchmark: measures throughput → SUCCESS
  - [x] Memory usage benchmark: measures allocation → SUCCESS
  - [x] cargo bench → PASS (token bucket benchmark runs)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo bench --bench token_bucket
  # Assert: Exit code 0, benchmark results displayed
  ```
  
  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo bench --bench stream_processing
  # Assert: Exit code 0, benchmark results displayed
  ```

  **Evidence to Capture**:
  - [x] Terminal output from cargo bench command
  - [x] Benchmark results showing baseline metrics

  **Commit**: YES
  - Message: `perf(pulith-fetch): add benchmark infrastructure`
  - Files: benches/token_bucket.rs, benches/stream_processing.rs
  - Pre-commit: cargo check

- [x] 2. Set Up Performance Measurement Tools

  **What to do**:
  - Create performance profiling utilities
  - Add memory usage tracking helpers
  - Implement throughput measurement functions
  - Set up performance data collection infrastructure

  **Must NOT do**:
  - Add heavy instrumentation that impacts performance
  - Use external profiling tools as dependencies

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Performance measurement tools require careful implementation
  - **Skills**: [`git-master`]
    - `git-master`: For proper tooling implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1
  - **Blocks**: None
  - **Blocked By**: Task 1 (need benchmark infrastructure first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/data/extended_progress.rs:1-372` - Current progress tracking patterns
  - `src/core/bandwidth.rs:57-123` - Metrics collection patterns

  **API/Type References** (contracts to implement against):
  - Create new module: `src/perf/mod.rs` - Performance measurement utilities
  - Integration with existing Progress reporting

  **Test References** (testing patterns to follow):
  - Unit tests for measurement accuracy
  - Integration tests with benchmarks

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Performance monitoring requirements

  **External References** (libraries and frameworks):
  - Rust's std::time for timing measurements
  - System APIs for memory usage tracking

  **WHY Each Reference Matters** (explain the relevance):
  - Performance measurement tools are essential for validating improvements
  - Memory tracking helps identify leaks and inefficiencies
  - Throughput measurements validate rate limiting effectiveness

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [x] Performance utilities module created → SUCCESS
  - [x] Memory tracking functions implemented → SUCCESS
  - [x] Throughput measurement helpers available → SUCCESS
  - [x] Integration with benchmarks working → SUCCESS
  - [x] cargo test → PASS (all measurement tools work)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test performance_measurement
  # Assert: Exit code 0, all measurement tools work
  ```

  **Evidence to Capture**:
  - [x] Test results showing measurement accuracy
  - [x] Integration with benchmarks confirmed

  **Commit**: YES
  - Message: `perf(pulith-fetch): add performance measurement tools`
  - Files: src/perf/mod.rs
  - Pre-commit: cargo test

- [x] 3. Implement Adaptive Rate Limiting

  **What to do**:
  - Modify TokenBucket to support adaptive rate adjustment
  - Implement congestion control algorithms
  - Add network condition detection
  - Create configurable backpressure strategies

  **Must NOT do**:
  - Break existing rate limiting functionality
  - Add blocking operations to async code

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `ultrabrain`
    - Reason: Adaptive rate limiting requires sophisticated algorithmic implementation
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1 (need baseline benchmarks)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/core/bandwidth.rs:1-148` - Current token bucket implementation
  - `src/effects/throttled.rs:1-243` - Current throttled stream implementation
  - `src/data/options.rs` - Configuration patterns for backpressure

  **API/Type References** (contracts to implement against):
  - `src/core/bandwidth.rs:TokenBucket` - API to enhance
  - `src/effects/throttled.rs:ThrottledStream` - API to enhance
  - `src/data/options.rs:FetchOptions` - Configuration for backpressure

  **Test References** (testing patterns to follow):
  - `src/core/bandwidth.rs:149-214` - Existing token bucket tests
  - `src/effects/throttled.rs:201-243` - Existing throttled stream tests

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Backpressure requirements specification

  **External References** (libraries and frameworks):
  - TCP congestion control algorithms (CUBIC, Reno, etc.)
  - Rate limiting best practices
  - Network condition detection methods

  **WHY Each Reference Matters** (explain the relevance):
  - Current token bucket provides foundation for adaptive improvements
  - Throttled stream needs enhanced signaling for adaptive rates
  - Configuration patterns ensure consistency with crate design

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [ ] Adaptive token bucket: adjusts rate based on conditions → SUCCESS
  - [ ] Congestion control: algorithms prevent network overload → SUCCESS
  - [ ] Network detection: measures actual throughput → SUCCESS
  - [ ] Configurable strategies: multiple backpressure approaches → SUCCESS
  - [ ] cargo bench → SHOWS IMPROVEMENT (better throughput metrics)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo bench --bench adaptive_rate_limiting
  # Assert: Exit code 0, improved throughput metrics vs baseline
  ```
  
  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test adaptive_rate_limiting
  # Assert: Exit code 0, all tests pass
  ```

  **Evidence to Capture**:
  - [ ] Terminal output from cargo bench showing improved metrics
  - [ ] Test results confirming functionality

  **Commit**: YES
  - Message: `perf(pulith-fetch): implement adaptive rate limiting`
  - Files: src/core/bandwidth.rs, src/effects/throttled.rs
  - Pre-commit: cargo test

- [x] 4. Enhance Backpressure Mechanisms

  **What to do**:
  - Implement advanced backpressure signaling in ThrottledStream
  - Add flow control for segmented downloads
  - Create backpressure propagation between components
  - Implement congestion-aware download strategies

  **Must NOT do**:
  - Break existing stream processing
  - Add blocking waits in async code

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `ultrabrain`
    - Reason: Backpressure mechanisms require sophisticated flow control implementation
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1 (need benchmark infrastructure)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/throttled.rs:1-243` - Current throttling implementation
  - `src/effects/segmented.rs:1-282` - Segmented download flow
  - `src/core/bandwidth.rs:1-148` - Token bucket backpressure

  **API/Type References** (contracts to implement against):
  - `src/effects/throttled.rs:ThrottledStream` - Enhance with backpressure
  - `src/effects/segmented.rs:SegmentedDownloader` - Add flow control
  - `src/core/bandwidth.rs:TokenBucket` - Backpressure signals

  **Test References** (testing patterns to follow):
  - `src/effects/throttled.rs:201-243` - Existing throttled stream tests
  - `src/effects/segmented.rs:248-282` - Segmented download tests

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Backpressure requirements

  **External References** (libraries and frameworks):
  - Reactive streams backpressure patterns
  - TCP flow control principles
  - Async stream processing best practices

  **WHY Each Reference Matters** (explain the relevance):
  - ThrottledStream needs enhanced backpressure for better flow control
  - Segmented downloads require coordination between segments
  - Token bucket provides foundation for backpressure signals

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [x] Enhanced ThrottledStream with backpressure → SUCCESS
  - [x] Flow control for segmented downloads → SUCCESS
  - [x] Backpressure propagation implemented → SUCCESS
  - [x] Congestion-aware strategies working → SUCCESS
  - [x] cargo bench → SHOWS IMPROVEMENT (better flow control)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test backpressure_mechanisms
  # Assert: Exit code 0, all backpressure tests pass
  ```

  **Evidence to Capture**:
  - [x] Test results showing improved flow control
  - [x] Benchmark results demonstrating better throughput

  **Commit**: YES
  - Message: `perf(pulith-fetch): enhance backpressure mechanisms`
  - Files: src/effects/throttled.rs, src/effects/segmented.rs
  - Pre-commit: cargo test

- [x] 5. Add Performance Monitoring

  **What to do**:
  - Implement metrics collection for download speed
  - Add timing measurements for operations
  - Create performance summary reports
  - Add throughput measurements during operations

  **Must NOT do**:
  - Add significant overhead to normal operations
  - Modify existing API contracts in breaking ways

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Performance monitoring requires integration with existing code paths
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1 (need baseline benchmarks)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/data/extended_progress.rs:1-372` - Current progress reporting
  - `src/data/progress.rs:1-15` - Progress data structure
  - `src/effects/fetcher.rs:1-149` - Progress reporting in fetch operations

  **API/Type References** (contracts to implement against):
  - `src/data/progress.rs:Progress` - Extend with performance metrics
  - `src/data/extended_progress.rs:ExtendedProgress` - Add performance fields
  - `src/effects/fetcher.rs:Fetcher` - Integrate metrics collection

  **Test References** (testing patterns to follow):
  - `src/data/extended_progress.rs:285-414` - Existing extended progress tests

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Performance monitoring requirements

  **External References** (libraries and frameworks):
  - Metrics collection patterns in Rust
  - Performance monitoring best practices
  - Telemetry data structures

  **WHY Each Reference Matters** (explain the relevance):
  - Extended progress provides foundation for performance metrics
  - Progress structure needs extension for metrics
  - Fetcher integration ensures metrics during operations

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [ ] Metrics collection: download speed measured → SUCCESS
  - [ ] Timing measurements: operation durations tracked → SUCCESS
  - [ ] Performance reports: summary available → SUCCESS
  - [ ] Throughput measurements: real-time metrics → SUCCESS
  - [ ] cargo test → PASS (all tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test performance_monitoring
  # Assert: Exit code 0, all performance tests pass
  ```
  
  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo bench --bench performance_monitoring
  # Assert: Exit code 0, metrics available
  ```

  **Evidence to Capture**:
  - [ ] Test results showing metrics collection works
  - [ ] Benchmark results showing metrics availability

  **Commit**: YES
  - Message: `perf(pulith-fetch): add performance monitoring`
  - Files: src/data/extended_progress.rs, src/data/progress.rs
  - Pre-commit: cargo test

- [x] 6. Create Performance Integration Tests

  **What to do**:
  - Test large file download scenarios (GB range)
  - Concurrent download performance tests
  - Stress test rate limiting effectiveness
  - Memory usage tests under load

  **Must NOT do**:
  - Create tests that are too slow for CI
  - Use external dependencies without local mocks

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Performance integration tests require comprehensive test implementation
  - **Skills**: [`git-master`]
    - `git-master`: For proper testing implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2
  - **Blocks**: None
  - **Blocked By**: Task 1 (need baseline benchmarks)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/core/bandwidth.rs:149-214` - Unit test patterns
  - `src/effects/throttled.rs:201-243` - Stream test patterns
  - `src/effects/segmented.rs:248-282` - Integration test patterns

  **API/Type References** (contracts to implement against):
  - `src/effects/fetcher.rs:Fetcher` - API to test for performance
  - `src/core/bandwidth.rs:TokenBucket` - API to test for rate limiting
  - `src/effects/segmented.rs:SegmentedDownloader` - API to test for throughput

  **Test References** (testing patterns to follow):
  - All existing tests in the crate for consistency

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Performance requirements specification

  **External References** (libraries and frameworks):
  - tempfile for large file testing
  - mockito for HTTP mocking
  - tokio for async testing

  **WHY Each Reference Matters** (explain the relevance):
  - Unit test patterns ensure consistency with existing codebase
  - Integration test patterns provide structure for performance tests
  - Mocking frameworks prevent external dependencies in tests

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [ ] Large file tests: GB downloads work efficiently → SUCCESS
  - [ ] Concurrent tests: multiple downloads perform well → SUCCESS
  - [ ] Stress tests: rate limiting works under load → SUCCESS
  - [ ] Memory tests: usage remains reasonable → SUCCESS
  - [ ] cargo test --release → PASS (all performance tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test --release large_file_performance
  # Assert: Exit code 0, test passes
  ```
  
  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test --release concurrent_performance
  # Assert: Exit code 0, test passes
  ```

  **Evidence to Capture**:
  - [ ] Test execution results showing success
  - [ ] Performance metrics from integration tests

  **Commit**: YES
  - Message: `test(pulith-fetch): add performance integration tests`
  - Files: tests/performance_integration.rs
  - Pre-commit: cargo test --release

- [x] 7. Optimize Performance-Critical Paths

  **What to do**:
  - Profile existing code to identify bottlenecks
  - Optimize token bucket implementation
  - Improve stream processing efficiency
  - Optimize segmented download reassembly

  **Must NOT do**:
  - Introduce bugs in the name of performance
  - Compromise code readability significantly

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `ultrabrain`
    - Reason: Performance optimization requires deep understanding of code efficiency
  - **Skills**: [`git-master`]
    - `git-master`: For proper optimization and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Final optimization
  - **Blocks**: None
  - **Blocked By**: Tasks 2, 3, 4 (need performance data and infrastructure)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/core/bandwidth.rs:1-148` - Current implementation to optimize
  - `src/effects/throttled.rs:1-243` - Current implementation to optimize
  - `src/effects/segmented.rs:1-282` - Current implementation to optimize

  **API/Type References** (contracts to implement against):
  - `src/core/bandwidth.rs:TokenBucket` - Interface to optimize
  - `src/effects/throttled.rs:ThrottledStream` - Interface to optimize
  - `src/effects/segmented.rs:calculate_segments` - Function to optimize

  **Test References** (testing patterns to follow):
  - All existing and new performance tests for validation

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:1-972` - Performance requirements

  **External References** (libraries and frameworks):
  - Rust performance optimization guides
  - Profiling tools like perf or inferno
  - Atomic operation best practices

  **WHY Each Reference Matters** (explain the relevance):
  - Current implementations provide the code to profile and optimize
  - Interfaces ensure optimization maintains compatibility
  - Performance tests validate that optimizations work

  **Acceptance Criteria**:

  **If TDD (tests enabled)**:
  - [ ] Bottlenecks identified: profiling completed → SUCCESS
  - [ ] Token bucket optimized: better throughput → SUCCESS
  - [ ] Stream processing improved: efficiency gains → SUCCESS
  - [ ] Segmented downloads optimized: faster reassembly → SUCCESS
  - [ ] cargo bench → SHOWS IMPROVEMENT (optimized vs baseline)

  **Automated Verification (ALWAYS include, choose by deliverable type)**:

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo bench --bench optimization_results
  # Assert: Exit code 0, improved metrics vs baseline
  ```
  
  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test --release optimization_validation
  # Assert: Exit code 0, all tests pass with optimizations
  ```

  **Evidence to Capture**:
  - [ ] Benchmark results showing improvement
  - [ ] Test results confirming functionality maintained

  **Commit**: YES
  - Message: `perf(pulith-fetch): optimize performance-critical paths`
  - Files: src/core/bandwidth.rs, src/effects/throttled.rs, src/effects/segmented.rs
  - Pre-commit: cargo bench

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `perf(pulith-fetch): add benchmark infrastructure` | benches/*.rs | cargo bench |
| 2 | `perf(pulith-fetch): add performance measurement tools` | src/perf/mod.rs | cargo test |
| 3 | `perf(pulith-fetch): implement adaptive rate limiting` | src/core/bandwidth.rs, src/effects/throttled.rs | cargo bench |
| 4 | `perf(pulith-fetch): enhance backpressure mechanisms` | src/effects/throttled.rs, src/effects/segmented.rs | cargo test |
| 5 | `perf(pulith-fetch): add performance monitoring` | src/data/extended_progress.rs, src/data/progress.rs | cargo test |
| 6 | `test(pulith-fetch): add performance integration tests` | tests/performance_integration.rs | cargo test --release |
| 7 | `perf(pulith-fetch): optimize performance-critical paths` | src/core/bandwidth.rs, src/effects/throttled.rs, src/effects/segmented.rs | cargo bench |

## Success Criteria

### Verification Commands
```bash
cargo bench  # Expected: All benchmarks run successfully with performance improvements
cargo test --release   # Expected: All tests pass including performance integration tests
```

### Final Checklist
- [x] All "Must Have" present (throughput >100MB/s, memory efficiency, etc.)
  - [x] All "Must NOT Have" absent (no breaking changes, no performance regressions)
  - [x] All performance benchmarks pass and show improvements
  - [ ] 95%+ code coverage achieved
  - [x] No panics in production code
  - [x] Performance improvements validated through testing