# pulith-fetch: Comprehensive Redesign Implementation

## TL;DR

> **Quick Summary**: Implement the complete redesign of pulith-fetch crate following the design document in docs/design/fetch.md, addressing critical consistency issues, implementing missing functionality, and establishing proper architecture with data/core/effects/transform separation.
> 
> **Deliverables**: 
> - Fixed compilation errors in existing files
> - Complete implementation of all planned features across 5 phases
> - Proper module structure with all missing modules created
> - Complete error handling without panics
> 
> **Estimated Effort**: XL
> **Parallel Execution**: YES - 5 waves
> **Critical Path**: Phase 1 Foundation → Phase 2 Core → Phase 3 Advanced → Phase 4 Compression & Caching → Phase 5 Protocol & Testing

---

## Context

### Original Request
Implement the pulith-fetch redesign as specified in docs/design/fetch.md, addressing architectural issues, consistency violations, and implementing planned features from the roadmap.

### Interview Summary
**Key Discussions**:
- Current state analysis: Some infrastructure (like Result<T>) already exists
- Identified compilation errors: duplicate FetchPhase, missing Progress references, BoxStream issues
- Missing modules need to be created: multi_source, resumable, segmented, batch, cache, decompress, verify

**Research Findings**:
- The crate already has good error handling infrastructure with Result<T> type alias
- Module structure follows the intended data/core/effects/transform pattern
- Several modules are referenced but not implemented
- Contains placeholder implementations that need completing

### Metis Review
**Identified Gaps** (addressed):
- Compilation errors in current codebase need fixing before new features
- Missing implementation of core fetch functionality
- Dependencies need to be updated to match design requirements (rand, chrono, etc.)

---

## Work Objectives

### Core Objective
Implement the complete redesign of pulith-fetch crate following the 5-phase plan from docs/design/fetch.md, ensuring consistency with the pulith ecosystem and implementing all planned features.

### Concrete Deliverables
- Fixed pulith-fetch crate with no compilation errors
- Complete implementation of all 5 phases from the design document
- All missing modules created and properly implemented
- All features working with proper error handling

### Definition of Done
- [ ] All compilation errors resolved
- [ ] All 5 phases implemented as per design document
- [ ] All missing modules created and implemented
- [ ] All functionality tested and working
- [ ] No panics in production code
- [ ] All features follow pulith design principles (F1-F5)

### Must Have
- Error handling with Result<T> type alias (already present)
- Proper module structure: data/core/effects/transform
- Complete implementation of all planned features
- No compilation errors after implementation

### Must NOT Have (Guardrails)
- Any panics in production code (replace with proper error handling)
- Duplicate type definitions
- Unimplemented placeholder functionality
- Hardcoded dependencies without trait abstraction

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES - uses workspace dependencies with dev-dependencies for testing
- **User wants tests**: TDD - Each feature should have unit and integration tests
- **Framework**: Rust standard testing with additional proptest for property-based tests

### If TDD Enabled

Each TODO follows RED-GREEN-REFACTOR:

**Task Structure:**
1. **RED**: Write failing test first
   - Test file: `[path]`
   - Test command: `cargo test [feature]`
   - Expected: FAIL (test exists, implementation doesn't)
2. **GREEN**: Implement minimum code to pass
   - Command: `cargo test [feature]`
   - Expected: PASS
3. **REFACTOR**: Clean up while keeping green
   - Command: `cargo test [feature]`
   - Expected: PASS (still)

**Test Setup Task (if infrastructure doesn't exist):**
- [ ] 0. Setup Test Infrastructure
  - Install: No additional setup needed (uses workspace dependencies)
  - Config: No additional config needed
  - Verify: `cargo test` → shows test framework available
  - Example: Create test in existing modules
  - Verify: `cargo test` → tests run successfully

### If Automated Verification Only (NO User Intervention)

> **CRITICAL PRINCIPLE: ZERO USER INTERVENTION**
>
> **NEVER** create acceptance criteria that require:
> - "User manually tests..." / "사용자가 직접 테스트..."
> - "User visually confirms..." / "사용자가 눈으로 확인..."
> - "User interacts with..." / "사용자가 직접 조작..."
> - "Ask user to verify..." / "사용자에게 확인 요청..."
> - ANY step that requires a human to perform an action
>
> **ALL verification MUST be automated and executable by the agent.**
> If a verification cannot be automated, find an automated alternative or explicitly note it as a known limitation.

Each TODO includes EXECUTABLE verification procedures that agents can run directly:

**By Deliverable Type:**

| Type | Verification Tool | Automated Procedure |
|------|------------------|---------------------|
| **Rust Library/Module** | cargo test via Bash | Agent runs tests, validates exit codes and output patterns |
| **Configuration/Infra** | Shell commands via Bash | Agent applies config, runs state check, validates output |

**Evidence Requirements (Agent-Executable):**
- Test output captured and compared against expected patterns
- Exit codes checked (0 = success)
- Error messages validated for proper error handling

---

## Execution Strategy

### Parallel Execution Waves

> Maximize throughput by grouping independent tasks into parallel waves.
> Each wave completes before the next begins.

```
Wave 1 (Foundation fixes):
├── Task 1: Fix compilation errors in existing files
├── Task 2: Create missing basic modules
└── Task 3: Implement basic fetch functionality

Wave 2 (Core features):
├── Task 4: Multi-source downloads implementation
├── Task 5: Segmented downloads implementation
├── Task 6: Bandwidth limiting implementation
└── Task 7: Batch downloads implementation

Wave 3 (Advanced features):
├── Task 8: Resumable downloads implementation
├── Task 9: Conditional downloads implementation
└── Task 10: Extended progress reporting

Wave 4 (Compression & caching):
├── Task 11: Compression support implementation
└── Task 12: HTTP caching implementation

Wave 5 (Testing & protocol):
├── Task 13: Protocol abstraction setup
├── Task 14: Comprehensive testing implementation
└── Task 15: Integration testing

Critical Path: Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 8 → Task 14
Parallel Speedup: ~60% faster than sequential
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2, 3 | None |
| 2 | 1 | 3, 4, 5, 6, 7 | None |
| 3 | 1 | 4, 5, 6, 7 | None |
| 4 | 2, 3 | 8, 14 | 5, 6, 7 |
| 5 | 2, 3 | 8, 14 | 4, 6, 7 |
| 6 | 2, 3 | 8, 14 | 4, 5, 7 |
| 7 | 2, 3 | 8, 14 | 4, 5, 6 |
| 8 | 4, 5, 6, 7 | 14 | 9, 10 |
| 9 | 4, 5, 6, 7 | 14 | 8, 10 |
| 10 | 4, 5, 6, 7 | 14 | 8, 9 |
| 11 | 2, 3 | 12 | 13 |
| 12 | 11 | 14 | 13 |
| 13 | 2, 3 | 14 | 11, 12 |
| 14 | 8, 9, 10, 12, 13 | None | None (final) |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Agents |
|------|-------|-------------------|
| 1 | 1, 2, 3 | delegate_task(category="quick", load_skills=[git-master], run_in_background=true) |
| 2 | 4, 5, 6, 7 | dispatch parallel after Wave 1 completes |
| 3 | 8, 9, 10 | dispatch parallel after Wave 2 completes |
| 4 | 11, 12, 13 | dispatch parallel after Wave 2 completes |
| 5 | 14 | final integration task |

---

## TODOs

- [x] 1. Fix compilation errors in existing files

  **What to do**:
  - Fix duplicate FetchPhase definition in options.rs
  - Add missing Progress import in options.rs
  - Fix BoxStream type alias issues in http.rs
  - Replace panic! in segment.rs with proper Result return

  **Must NOT do**:
  - Change the overall architecture from the design document
  - Remove existing functionality without replacement

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `quick`
    - Reason: Simple fixes to existing code with known solutions
  - **Skills**: [`git-master`]
    - `git-master`: For code changes and proper commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Sequential (must be first)
  - **Blocks**: [Tasks 2, 3] 
  - **Blocked By**: None (can start immediately)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/data/progress.rs:12-58` - Progress struct definition and implementation
  - `src/data/options.rs:56-303` - FetchPhase definition (first occurrence)
  - `src/core/segment.rs:26-29` - Current panic! implementation that needs fixing

  **API/Type References** (contracts to implement against):
  - `src/error.rs:3-58` - Error enum and Result type alias definition
  - `src/effects/http.rs:13-13` - BoxStream type alias definition that needs fixing

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:21-36` - Critical issues identified that need fixing
  - `docs/design/fetch.md:117-142` - Design principles (F1-F5) to follow

  **External References** (libraries and frameworks):
  - Rust standard library patterns for Result types
  - futures-util and Stream trait for proper async handling

  **WHY Each Reference Matters** (explain the relevance):
  - Progress struct is needed to fix missing reference in options.rs
  - Error handling patterns should be consistent with existing error.rs
  - Stream trait implementation is needed for BoxStream fixes

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Fix compilation errors: cargo check → PASS (no errors)
  - [ ] Run all tests: cargo test → PASS (all existing tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo check
  # Assert: Exit code 0, no compilation errors
  ```

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test
  # Assert: All existing tests continue to pass
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo check command (no errors)
  - [ ] Terminal output from cargo test command (all tests pass)

  **Commit**: YES
  - Message: `fix(pulith-fetch): resolve compilation errors in existing files`
  - Files: src/data/options.rs, src/effects/http.rs, src/core/segment.rs
  - Pre-commit: cargo check

- [ ] 2. Create missing basic modules

  **What to do**:
  - Create missing modules: multi_source.rs, resumable.rs, segmented.rs, batch.rs, cache.rs in effects/
  - Create missing modules: decompress.rs, verify.rs in transform/
  - Implement basic structure following design document patterns

  **Must NOT do**:
  - Skip any modules mentioned in mod.rs files
  - Implement complex functionality in this task (save for later tasks)

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `quick`
    - Reason: Creating basic module files with initial structure
  - **Skills**: [`git-master`]
    - `git-master`: For proper file creation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3) 
  - **Blocks**: [Tasks 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
  - **Blocked By**: [Task 1] (compilation fixes needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/http.rs:70-124` - Example of conditional module with reqwest feature flag
  - `src/core/retry.rs:1-37` - Example of core module structure with documentation
  - `src/data/options.rs:56-303` - Example of data module with builder patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:144-172` - Module structure specification
  - `docs/design/fetch.md:350-376` - MultiSourceFetcher interface
  - `docs/design/fetch.md:499-541` - ResumableFetcher interface
  - `docs/design/fetch.md:417-460` - SegmentedFetcher interface
  - `docs/design/fetch.md:465-487` - BatchFetcher interface
  - `docs/design/fetch.md:684-730` - Cache implementation
  - `docs/design/fetch.md:649-670` - StreamTransform trait

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:144-172` - Module structure specification

  **External References** (libraries and frameworks):
  - Rust standard library patterns for module organization
  - futures-util for async stream handling

  **WHY Each Reference Matters** (explain the relevance):
  - Module structure from design doc ensures proper organization
  - Existing patterns ensure consistency with the rest of the crate
  - Interface specifications from design doc provide implementation targets

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Create module files: src/effects/multi_source.rs, src/effects/resumable.rs, etc. → PASS
  - [ ] Verify no compilation errors: cargo check → PASS (no new errors)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo check
  # Assert: Exit code 0, no compilation errors from new modules
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo check command (no errors)
  - [ ] List of created files

  **Commit**: YES
  - Message: `feat(pulith-fetch): create missing modules structure`
  - Files: src/effects/multi_source.rs, src/effects/resumable.rs, src/effects/segmented.rs, src/effects/batch.rs, src/effects/cache.rs, src/transform/decompress.rs, src/transform/verify.rs
  - Pre-commit: cargo check

- [ ] 3. Implement basic fetch functionality

  **What to do**:
  - Complete the implementation of the main fetch function in fetcher.rs
  - Implement core download functionality with progress reporting
  - Add proper error handling and verification
  - Ensure atomic placement using pulith-fs::Workspace

  **Must NOT do**:
  - Add advanced features like multi-source or segmented downloads (these are separate tasks)
  - Implement compression or caching at this stage

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Core functionality implementation requiring significant Rust expertise
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 2)
  - **Blocks**: [Tasks 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]
  - **Blocked By**: [Task 1] (compilation fixes needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/http.rs:89-119` - HttpClient trait implementation pattern
  - `src/core/segment.rs:12-65` - Function with proper documentation and error handling
  - `src/data/progress.rs:5-58` - Progress struct usage patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:174-202` - Public API surface specification
  - `src/effects/fetcher.rs:27-36` - Current placeholder function signature
  - `src/error.rs:6-36` - Error types for proper error handling
  - `pulith-fs` workspace crate for atomic file operations

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:174-202` - Public API surface specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - tokio for async file operations
  - futures-util for stream handling
  - bytes for buffer management
  - pulith-fs for atomic operations
  - pulith-verify for checksum operations

  **WHY Each Reference Matters** (explain the relevance):
  - HttpClient trait pattern ensures consistency with HTTP abstraction
  - Error handling patterns ensure proper error propagation
  - pulith-fs integration ensures atomic file placement as specified
  - Progress struct enables proper progress reporting

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Implement basic fetch functionality: src/effects/fetcher.rs → COMPLETE
  - [ ] Verify functionality: cargo test fetcher → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test fetcher
  # Assert: Exit code 0, tests pass for fetcher functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that basic fetch works with a test URL

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement basic fetch functionality`
  - Files: src/effects/fetcher.rs
  - Pre-commit: cargo test fetcher

- [ ] 4. Implement multi-source downloads

  **What to do**:
  - Complete MultiSourceFetcher implementation with priority-based selection
  - Implement geographic routing and race mode
  - Add source verification capabilities
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex async implementation with multiple sources and coordination
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6, 7)
  - **Blocks**: [Tasks 8, 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/fetcher.rs:14-36` - Fetcher struct and implementation pattern
  - `src/core/retry.rs:1-37` - Asynchronous coordination patterns
  - `src/data/sources.rs:1-99` - DownloadSource and MultiSourceOptions definitions

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:347-376` - MultiSourceFetcher interface specification
  - `src/data/sources.rs:69-98` - MultiSourceOptions and SelectionStrategy definitions
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:347-376` - Multi-source download specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - tokio for async coordination and task management
  - futures-util for stream handling and concurrent operations
  - select! macro for race conditions

  **WHY Each Reference Matters** (explain the relevance):
  - Fetcher pattern ensures consistency with the overall architecture
  - MultiSourceOptions provides the configuration interface
  - Async coordination patterns ensure efficient concurrent operations

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Implement MultiSourceFetcher: src/effects/multi_source.rs → COMPLETE
  - [ ] Verify functionality: cargo test multi_source → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test multi_source
  # Assert: Exit code 0, tests pass for multi-source functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that multiple sources can be tried with different strategies

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement multi-source downloads`
  - Files: src/effects/multi_source.rs
  - Pre-commit: cargo test multi_source

- [ ] 5. Implement segmented downloads

  **What to do**:
  - Complete SegmentedFetcher implementation with parallel segment downloads
  - Implement segment calculation and reassembly
  - Add proper verification for segmented downloads
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex async implementation with parallel downloads and file reassembly
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 6, 7)
  - **Blocks**: [Tasks 8, 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/fetcher.rs:14-36` - Fetcher struct and implementation pattern
  - `src/core/segment.rs:12-65` - Segment calculation logic (to be extended)
  - `src/core/retry.rs:1-37` - Asynchronous coordination patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:417-460` - SegmentedFetcher interface specification
  - `src/core/segment.rs:2-65` - Segment struct and calculate_segments function
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:417-460` - Segmented download specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - tokio for async coordination and parallel downloads
  - futures-util for stream handling and concurrent operations
  - async-std for async file operations

  **WHY Each Reference Matters** (explain the relevance):
  - Fetcher pattern ensures consistency with the overall architecture
  - Segment calculation logic provides the foundation for segmented downloads
  - Async coordination patterns ensure efficient parallel operations

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Implement SegmentedFetcher: src/effects/segmented.rs → COMPLETE
  - [ ] Verify functionality: cargo test segmented → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test segmented
  # Assert: Exit code 0, tests pass for segmented functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that file can be downloaded in segments and reassembled correctly

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement segmented downloads`
  - Files: src/effects/segmented.rs
  - Pre-commit: cargo test segmented

- [ ] 6. Implement bandwidth limiting

  **What to do**:
  - Complete TokenBucket implementation in core/bandwidth.rs
  - Implement ThrottledStream in effects/throttled.rs
  - Add bandwidth limiting options to FetchOptions
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex async implementation with rate limiting and stream transformation
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 7)
  - **Blocks**: [Tasks 8, 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/core/bandwidth.rs:5-32` - Current TokenBucket structure
  - `src/effects/fetcher.rs:14-36` - Fetcher struct and implementation pattern
  - `src/data/options.rs:56-303` - Options structure pattern

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:377-415` - Bandwidth limiting specification
  - `src/core/bandwidth.rs:5-32` - Current TokenBucket structure (to be completed)
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:377-415` - Bandwidth limiting specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - tokio for async synchronization
  - futures-util for stream transformation
  - async-std for async operations

  **WHY Each Reference Matters** (explain the relevance):
  - TokenBucket structure provides the foundation for rate limiting
  - Fetcher pattern ensures consistency with the overall architecture
  - Options pattern ensures consistent configuration interface

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Complete TokenBucket implementation: src/core/bandwidth.rs → COMPLETE
  - [ ] Implement bandwidth limiting: src/effects/throttled.rs → COMPLETE
  - [ ] Verify functionality: cargo test bandwidth → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test bandwidth
  # Assert: Exit code 0, tests pass for bandwidth functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that download rate can be limited to specified values

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement bandwidth limiting`
  - Files: src/core/bandwidth.rs, src/effects/throttled.rs (new file)
  - Pre-commit: cargo test bandwidth

- [ ] 7. Implement batch downloads

  **What to do**:
  - Complete BatchFetcher implementation with dependency resolution
  - Implement concurrent download limiting
  - Add fail-fast vs continue options
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex async implementation with dependency resolution and coordination
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 4, 5, 6)
  - **Blocks**: [Tasks 8, 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/fetcher.rs:14-36` - Fetcher struct and implementation pattern
  - `src/core/retry.rs:1-37` - Asynchronous coordination patterns
  - `src/data/options.rs:56-303` - Options structure pattern

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:465-487` - BatchFetcher interface specification
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:465-487` - Batch download specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - tokio for async coordination and task management
  - futures-util for stream handling and concurrent operations
  - petgraph or similar for dependency resolution

  **WHY Each Reference Matters** (explain the relevance):
  - Fetcher pattern ensures consistency with the overall architecture
  - Dependency resolution algorithms ensure proper execution order
  - Async coordination patterns ensure efficient concurrent operations

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Implement BatchFetcher: src/effects/batch.rs → COMPLETE
  - [ ] Verify functionality: cargo test batch → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test batch
  # Assert: Exit code 0, tests pass for batch functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that multiple downloads can run with concurrency limits

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement batch downloads`
  - Files: src/effects/batch.rs
  - Pre-commit: cargo test batch

- [ ] 8. Implement resumable downloads

  **What to do**:
  - Complete ResumableFetcher implementation with HTTP Range support
  - Implement checksum state persistence
  - Add automatic resume on failure
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex implementation with HTTP Range handling and state persistence
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 9, 10)
  - **Blocks**: [Task 14]
  - **Blocked By**: [Tasks 4, 5, 6, 7] (core features needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/fetcher.rs:14-36` - Fetcher struct and implementation pattern
  - `src/data/options.rs:56-303` - Options structure pattern
  - `src/core/retry.rs:1-37` - State management patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:506-541` - ResumableFetcher interface specification
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:506-541` - Resumable download specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - tokio for async file operations
  - reqwest for HTTP Range request support
  - HTTP Range header specifications

  **WHY Each Reference Matters** (explain the relevance):
  - Fetcher pattern ensures consistency with the overall architecture
  - HTTP Range specifications ensure proper resume functionality
  - State management patterns ensure reliable resume capability

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Implement ResumableFetcher: src/effects/resumable.rs → COMPLETE
  - [ ] Verify functionality: cargo test resumable → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test resumable
  # Assert: Exit code 0, tests pass for resumable functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that downloads can be resumed after interruption

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement resumable downloads`
  - Files: src/effects/resumable.rs
  - Pre-commit: cargo test resumable

- [ ] 9. Implement conditional downloads

  **What to do**:
  - Extend FetchOptions with conditional download options
  - Implement If-Modified-Since and If-None-Match support
  - Add proper handling for 304 Not Modified responses
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: HTTP protocol implementation with conditional requests
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 10)
  - **Blocks**: [Task 14]
  - **Blocked By**: [Tasks 4, 5, 6, 7] (core features needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/data/options.rs:56-303` - Options structure pattern
  - `src/effects/http.rs:89-119` - HTTP request implementation patterns
  - `src/error.rs:10-11` - HTTP error handling patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:543-578` - Conditional download specification
  - `src/data/options.rs:70-251` - FetchOptions structure (to be extended)
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:543-578` - Conditional download specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - HTTP/1.1 specification for conditional requests
  - reqwest for If-Modified-Since and If-None-Match headers

  **WHY Each Reference Matters** (explain the relevance):
  - HTTP specification ensures proper conditional request handling
  - Options pattern ensures consistent configuration interface
  - Error handling patterns ensure proper response to HTTP errors

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Extend FetchOptions: src/data/options.rs → UPDATED
  - [ ] Implement conditional downloads: src/effects/fetcher.rs → UPDATED
  - [ ] Verify functionality: cargo test conditional → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test conditional
  # Assert: Exit code 0, tests pass for conditional functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that conditional requests work properly with 304 responses

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement conditional downloads`
  - Files: src/data/options.rs, src/effects/fetcher.rs
  - Pre-commit: cargo test conditional

- [ ] 10. Implement extended progress reporting

  **What to do**:
  - Create ExtendedProgress struct with speed and ETA calculations
  - Implement SpeedCalculator with exponential moving average
  - Add segment progress tracking
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex state management for progress tracking and calculations
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 9)
  - **Blocks**: [Task 14]
  - **Blocked By**: [Tasks 4, 5, 6, 7] (core features needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/data/progress.rs:5-58` - Current Progress struct (to be extended)
  - `src/core/retry.rs:1-37` - State management patterns
  - `src/data/options.rs:56-303` - Data structure patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:580-631` - Extended progress specification
  - `src/data/progress.rs:10-58` - Progress struct (to be extended)
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:580-631` - Extended progress specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - Exponential moving average algorithms
  - Time calculations for ETA

  **WHY Each Reference Matters** (explain the relevance):
  - Progress struct extension maintains compatibility with existing code
  - EMA algorithms ensure smooth speed calculations
  - Time calculations enable accurate ETA predictions

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Extend Progress: src/data/progress.rs → UPDATED
  - [ ] Implement speed calculator: src/core/mod.rs, src/core/speed.rs (new file) → COMPLETE
  - [ ] Verify functionality: cargo test progress → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test progress
  # Assert: Exit code 0, tests pass for progress functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that speed and ETA are calculated correctly

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement extended progress reporting`
  - Files: src/data/progress.rs, src/core/speed.rs (new file)
  - Pre-commit: cargo test progress

- [ ] 11. Implement compression support

  **What to do**:
  - Complete StreamTransform trait in transform/decompress.rs
  - Implement Gzip, Brotli, and Zstd decoders
  - Add decompression options to FetchOptions
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Stream transformation implementation with multiple compression formats
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 12, 13)
  - **Blocks**: [Task 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/transform/mod.rs:7-9` - Current transform module structure
  - `src/data/options.rs:56-303` - Options structure pattern
  - `src/transform/decompress.rs:7-9` - Current StreamTransform trait (to be completed)

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:649-682` - StreamTransform and compression specification
  - `src/transform/decompress.rs:5-10` - StreamTransform trait (to be completed)
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:649-682` - Compression support specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - flate2 for Gzip compression
  - brotli for Brotli compression
  - zstd for Zstd compression
  - Stream processing patterns

  **WHY Each Reference Matters** (explain the relevance):
  - StreamTransform trait ensures consistent interface for compression
  - Compression libraries provide the actual decompression capabilities
  - Options pattern ensures consistent configuration interface

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Complete StreamTransform: src/transform/decompress.rs → COMPLETE
  - [ ] Implement compression support: src/transform/verify.rs → COMPLETE
  - [ ] Verify functionality: cargo test compression → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test compression
  # Assert: Exit code 0, tests pass for compression functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that compressed content can be downloaded and decompressed

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement compression support`
  - Files: src/transform/decompress.rs, src/transform/verify.rs
  - Pre-commit: cargo test compression

- [ ] 12. Implement HTTP caching

  **What to do**:
  - Complete Cache implementation with ETag and Last-Modified support
  - Implement LRU eviction with size limits
  - Add persistent cache metadata storage
  - Follow the design document specifications

  **Must NOT do**:
  - Implement features not specified in the design document
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Complex caching implementation with metadata persistence
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 13)
  - **Blocks**: [Task 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/cache.rs:14-16` - Current cache module reference
  - `src/data/options.rs:56-303` - Options structure pattern
  - `src/error.rs:6-36` - Error handling patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:684-730` - Cache implementation specification
  - `src/effects/cache.rs:14` - Cache module (to be implemented)
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:684-730` - HTTP caching specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - rusqlite for cache metadata storage
  - HTTP/1.1 specification for caching semantics (RFC 7234)
  - LRU cache algorithms

  **WHY Each Reference Matters** (explain the relevance):
  - HTTP caching specification ensures proper RFC 7234 compliance
  - Database storage ensures persistent cache metadata
  - LRU algorithms ensure proper cache eviction

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Implement Cache: src/effects/cache.rs → COMPLETE
  - [ ] Verify functionality: cargo test cache → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test cache
  # Assert: Exit code 0, tests pass for cache functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that HTTP caching works with ETag and Last-Modified

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement HTTP caching`
  - Files: src/effects/cache.rs
  - Pre-commit: cargo test cache

- [ ] 13. Implement protocol abstraction

  **What to do**:
  - Create protocol abstraction layer with trait definitions
  - Implement extensible protocol support
  - Prepare for future protocol extensions (FTP, S3, etc.)
  - Note: Implementation of actual protocols deferred to v1.0+

  **Must NOT do**:
  - Implement actual FTP/S3 protocols (these are deferred per design doc)
  - Add non-essential features that might complicate the implementation

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Protocol abstraction design with extensibility for future protocols
  - **Skills**: [`git-master`]
    - `git-master`: For proper implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Tasks 11, 12)
  - **Blocks**: [Task 14]
  - **Blocked By**: [Tasks 2, 3] (basic structure and fetch needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - `src/effects/http.rs:25-69` - HttpClient trait pattern
  - `src/effects/protocol.rs:50-58` - Current protocol trait (to be created)
  - `src/error.rs:6-36` - Error handling patterns

  **API/Type References** (contracts to implement against):
  - `docs/design/fetch.md:747-762` - Protocol abstraction specification
  - `src/effects/protocol.rs:50-58` - Protocol trait (to be created)
  - `src/error.rs:6-36` - Error types for proper error handling

  **Test References** (testing patterns to follow):
  - Existing test patterns in workspace

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:747-762` - Protocol abstraction specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - Trait-based abstraction patterns
  - Extensibility design patterns

  **WHY Each Reference Matters** (explain the relevance):
  - HttpClient trait pattern ensures consistency with existing abstractions
  - Protocol abstraction enables future extensibility
  - Error handling patterns ensure consistent error propagation

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Create protocol abstraction: src/effects/protocol.rs (new file) → COMPLETE
  - [ ] Verify functionality: cargo test protocol → PASS (new tests pass)

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test protocol
  # Assert: Exit code 0, tests pass for protocol functionality
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (tests pass)
  - [ ] Verification that protocol abstraction is properly set up

  **Commit**: YES
  - Message: `feat(pulith-fetch): implement protocol abstraction`
  - Files: src/effects/protocol.rs (new file)
  - Pre-commit: cargo test protocol

- [ ] 14. Implement comprehensive testing

  **What to do**:
  - Add unit tests for all implemented features
  - Create integration tests for complete workflows
  - Implement property-based tests using proptest
  - Add stress tests and fault injection tests
  - Ensure 95%+ code coverage

  **Must NOT do**:
  - Skip any major functionality in testing
  - Add unnecessary test complexity that doesn't verify requirements

  **Recommended Agent Profile**:
  > Select category + skills based on task domain. Justify each choice.
  - **Category**: `unspecified-high`
    - Reason: Comprehensive testing implementation across all features
  - **Skills**: [`git-master`]
    - `git-master`: For proper testing implementation and commit practices
  - **Skills Evaluated but Omitted**:
    - `frontend-ui-ux`: Not relevant for backend Rust crate

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Final integration task
  - **Blocks**: None (final task)
  - **Blocked By**: [Tasks 8, 9, 10, 12, 13] (all features needed first)

  **References** (CRITICAL - Be Exhaustive):

  **Pattern References** (existing code to follow):
  - Existing test patterns in workspace crates
  - proptest usage patterns in similar crates

  **API/Type References** (contracts to implement against):
  - All implemented features and their interfaces
  - `src/error.rs:6-36` - Error types for testing error conditions

  **Test References** (testing patterns to follow):
  - Unit test patterns in workspace
  - Integration test patterns in workspace
  - Property-based test patterns with proptest

  **Documentation References** (specs and requirements):
  - `docs/design/fetch.md:764-798` - Testing requirements specification
  - `docs/design/fetch.md:1-972` - Complete design specification

  **External References** (libraries and frameworks):
  - proptest for property-based testing
  - tempfile for temporary file testing
  - mockall for mocking dependencies

  **WHY Each Reference Matters** (explain the relevance):
  - Unit tests ensure individual components work correctly
  - Integration tests verify complete workflows
  - Property tests verify invariants across random inputs
  - Stress tests ensure performance under load

  **Acceptance Criteria**:

  **If TDD (tests enabled):**
  - [ ] Add comprehensive tests: All modules → TESTS ADDED
  - [ ] Verify test coverage: cargo test → PASS (all tests pass)
  - [ ] Verify coverage: cargo tarpaulin or similar → 95%+ coverage

  **Automated Verification (ALWAYS include, choose by deliverable type):**

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test
  # Assert: Exit code 0, all tests pass
  ```

  **For Library/Module changes** (using Bash cargo):
  ```bash
  # Agent runs:
  cargo test --release
  # Assert: Exit code 0, all tests pass in release mode too
  ```

  **Evidence to Capture:**
  - [ ] Terminal output from cargo test command (all tests pass)
  - [ ] Coverage report showing 95%+ coverage

  **Commit**: YES
  - Message: `test(pulith-fetch): add comprehensive test coverage`
  - Files: All test files across the crate
  - Pre-commit: cargo test

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 1 | `fix(pulith-fetch): resolve compilation errors in existing files` | src/data/options.rs, src/effects/http.rs, src/core/segment.rs | cargo check |
| 2 | `feat(pulith-fetch): create missing modules structure` | src/effects/multi_source.rs, src/effects/resumable.rs, src/effects/segmented.rs, src/effects/batch.rs, src/effects/cache.rs, src/transform/decompress.rs, src/transform/verify.rs | cargo check |
| 3 | `feat(pulith-fetch): implement basic fetch functionality` | src/effects/fetcher.rs | cargo test fetcher |
| 4 | `feat(pulith-fetch): implement multi-source downloads` | src/effects/multi_source.rs | cargo test multi_source |
| 5 | `feat(pulith-fetch): implement segmented downloads` | src/effects/segmented.rs | cargo test segmented |
| 6 | `feat(pulith-fetch): implement bandwidth limiting` | src/core/bandwidth.rs, src/effects/throttled.rs | cargo test bandwidth |
| 7 | `feat(pulith-fetch): implement batch downloads` | src/effects/batch.rs | cargo test batch |
| 8 | `feat(pulith-fetch): implement resumable downloads` | src/effects/resumable.rs | cargo test resumable |
| 9 | `feat(pulith-fetch): implement conditional downloads` | src/data/options.rs, src/effects/fetcher.rs | cargo test conditional |
| 10 | `feat(pulith-fetch): implement extended progress reporting` | src/data/progress.rs, src/core/speed.rs | cargo test progress |
| 11 | `feat(pulith-fetch): implement compression support` | src/transform/decompress.rs, src/transform/verify.rs | cargo test compression |
| 12 | `feat(pulith-fetch): implement HTTP caching` | src/effects/cache.rs | cargo test cache |
| 13 | `feat(pulith-fetch): implement protocol abstraction` | src/effects/protocol.rs | cargo test protocol |
| 14 | `test(pulith-fetch): add comprehensive test coverage` | All test files | cargo test |

---

## Success Criteria

### Verification Commands
```bash
cargo check  # Expected: exit code 0, no compilation errors
cargo test   # Expected: exit code 0, all tests pass
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass
- [ ] 95%+ code coverage achieved
- [ ] No panics in production code
- [ ] All 5 phases from design document implemented