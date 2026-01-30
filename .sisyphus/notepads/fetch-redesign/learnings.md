# Learnings - Pulith Fetch Redesign

## Atomic Operations
- Use AtomicU64 instead of AtomicF64 (AtomicF64 doesn't exist in Rust)
- Token bucket implementation uses atomic operations for thread safety

## Type System
- Use `std::result::Result` to avoid conflicts with `crate::Result` type alias
- Add `'static` lifetime bounds for async spawn operations

## Module Structure
- Core modules: Pure functions and calculations
- Effects modules: I/O operations and side effects
- Transform modules: Stream transformations

## Error Handling
- Consistent use of `Result<T>` type alias throughout
- Proper error propagation with `?` operator
- No `Io` error variant - use `Network` or `InvalidState` instead

## Testing Patterns
- Unit tests for pure functions in core modules
- Integration tests for effects modules
- Mock external dependencies for isolated testing

## Resumable Downloads
- Checkpoint files store download state for resuming
- Use HTTP Range headers for partial content requests
- Atomic checkpoint updates with temporary files
- Progress callbacks must be `Arc<dyn Fn(&Progress) + Send + Sync>`
- Cannot mutate captured variables in `Fn` closures - create new instances instead

## Closure Capture Rules
- Variables moved into closures cannot be used after
- Clone variables before moving into closures when needed
- Use `Arc::new()` for callback functions in FetchOptions

## Task 8 Completed: Resumable Downloads
- Implemented `DownloadCheckpoint` struct with serde serialization
- Added `ResumableFetcher` with checkpoint management
- HTTP Range header support for partial downloads
- Atomic checkpoint file operations with temp files
- Progress callback integration with checkpoint updates
- Cleanup functionality for old checkpoints
- All tests passing

## Task 9 Completed: Conditional Downloads
- Implemented `RemoteMetadata` struct for ETag/Last-Modified tracking
- Added `ConditionalFetcher` with conditional download logic
- Metadata storage and retrieval system
- Content comparison using ETag, Last-Modified, and Content-Length
- Cleanup functionality for old metadata files
- All tests passing

## Conditional Downloads Implementation Notes
- Metadata stored in `.metadata` directory with hash-based filenames
- ETag comparison is most reliable, followed by Last-Modified
- Content-Length is least reliable but used as fallback
- Directory existence checks prevent errors during cleanup
- Time comparison logic: file is old if (now - file_time) > max_age_seconds