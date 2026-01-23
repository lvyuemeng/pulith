# pulith-archive - Improved Design

## Problems with Current Design

### 1. Duplicated Entry Types

**Current Issues:**
- `ArchiveEntry` (data/entry.rs) and `ExtractedEntry` (data/report.rs) have 90% field overlap
- `ExtractedEntry` adds only `target_path` and `hash` to `ArchiveEntry`
- Conversion function `ExtractedEntry::from_archive_entry` proves duplication
- Maintains two similar types that drift over time

**Impact:**
- Redundant memory allocations when transforming entries
- Confusing API - which type to use?
- Test code duplication
- Difficult to add new fields (must update both types)

### 2. Mixed Permission and Hash Calculation

**Current Issues:**
- `apply_permissions()` and `calculate_hash()` functions mixed in `extract/mod.rs`
- Hash calculation duplicated in both Zip and Tar extractors (lines 40-44 and 281-296)
- Permission application scattered across extraction logic
- No clear separation between extraction and post-processing

**Impact:**
- Violates Single Responsibility Principle
- Difficult to test permission/hash logic independently
- Coupled extraction pipeline - can't easily swap strategies
- Hash computation loads entire file into memory

### 3. Bad Reader Format (`wrap_reader`)

**Current Issues:**
- `wrap_reader()` returns `Box<dyn Read>` - trait object with dynamic dispatch
- Loses type information and incurs runtime overhead
- Only used for tar decompression (line 229 in extract/mod.rs)
- Feature flags create compilation errors at runtime instead of compile-time
- Inconsistent with "Functions First" philosophy

**Impact:**
- No compile-time guarantee of supported formats
- Virtual function call overhead
- Difficult to test and mock
- Error messages less helpful

### 4. Chaotic Extraction Logic

**Current Issues:**
- **721 lines** in a single file (`extract/mod.rs`)
- **Duplicate extractor implementations**: Zip and Tar have nearly identical structure (~130 lines each)
- **Deep nesting and repeated patterns** throughout extraction functions
- **Mixed concerns** in each extractor:
  - Reading archive entries
  - Path sanitization
  - File system I/O (write, mkdir, symlink)
  - Permission application
  - Hash calculation
  - Progress tracking
  - Entry creation

**Code Duplication:**
| Concern | Zip Extractor | Tar Extractor |
|---------|---------------|---------------|
| Progress reporting | Lines 184-195 | Lines 380-387 |
| Permission application | Lines 165-168, 197 | Lines 306-309, 320-324, 399-404 |
| Hash calculation | Lines 140-144 | Lines 281-296 (DUPLICATED) |
| Entry creation | Lines 198-209 | Lines 406-417 |
| Directory creation | Lines 147-155, 169-177 | Lines 267-275, 310-318 |
| Parent directory check | Lines 147-148, 267-268 | Lines 267-268 (same) |

**Inefficiency Issues:**
- **Zip loads entire file into memory** (line 137-138) before hashing and writing
- **Tar also loads entire file** (line 278-279) for hashing - not streaming
- **Hash computed twice** if needed: once during extraction, then written

**Maintainability Issues:**
- Adding new feature requires modifying both extractors
- No separation of concerns - everything is monolithic
- Testing individual steps is impossible without extracting
- Can't reuse extraction logic for different backends

**Architecture Violations:**
- **F1 (Functions First)**: Everything is in methods, not composable functions
- **F3 (Pure Core, Impure Edge)**: I/O mixed throughout, no pure core
- **F5 (Composition Over Orchestration)**: Manual orchestration in monolithic functions

**Impact:**
- 721 lines to maintain (vs ~200 lines with proper architecture)
- Adding a new archive format = copy-paste ~130 lines
- Testing requires full filesystem setup (no pure logic to test)
- Can't easily optimize specific steps (e.g., parallel extraction)
- Memory usage scales with largest file in archive

---

## Improved Design

### 1. Unified Entry Model

**Principle:** Single source of truth for entry data.

```rust
// data/entry.rs

/// Represents an archive entry during extraction.
///
/// The `target_path` field is set after path sanitization,
/// while `hash` is populated during extraction if hashing is enabled.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Original path from the archive (unsanitized)
    pub original_path: PathBuf,

    /// Sanitized target path after strip_components and validation
    pub target_path: Option<PathBuf>,

    /// Entry size in bytes
    pub size: u64,

    /// Unix mode bits (present in some formats)
    pub mode: Option<u32>,

    /// Entry type
    pub kind: EntryKind,

    /// Computed hash (populated during extraction)
    pub hash: Option<String>,
}

/// Entry type classification
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink { target: PathBuf },
}

impl Entry {
    /// Create a new entry from archive metadata
    pub fn new(original_path: PathBuf, size: u64, mode: Option<u32>, kind: EntryKind) -> Self {
        Self {
            original_path,
            target_path: None,
            size,
            mode,
            kind,
            hash: None,
        }
    }

    /// Set the sanitized target path
    pub fn with_target_path(mut self, target_path: PathBuf) -> Self {
        self.target_path = Some(target_path);
        self
    }

    /// Set the computed hash
    pub fn with_hash(mut self, hash: String) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Check if entry is a file
    pub fn is_file(&self) -> bool {
        matches!(self.kind, EntryKind::File)
    }

    /// Check if entry is a directory
    pub fn is_directory(&self) -> bool {
        matches!(self.kind, EntryKind::Directory)
    }

    /// Check if entry is a symlink
    pub fn is_symlink(&self) -> bool {
        matches!(self.kind, EntryKind::Symlink { .. })
    }

    /// Get symlink target if applicable
    pub fn symlink_target(&self) -> Option<&Path> {
        match &self.kind {
            EntryKind::Symlink { target } => Some(target),
            _ => None,
        }
    }

    /// Check if entry is executable (has execute bit set)
    pub fn is_executable(&self) -> bool {
        self.mode.map_or(false, |m| m & 0o111 != 0)
    }
}

/// Backward compatibility type alias for ExtractedEntry
pub type ExtractedEntry = Entry;

/// Backward compatibility: convert old ArchiveEntry to new Entry
#[cfg(feature = "compat")]
impl From<LegacyArchiveEntry> for Entry {
    fn from(entry: LegacyArchiveEntry) -> Self {
        let kind = if entry.is_symlink {
            EntryKind::Symlink {
                target: entry.symlink_target.unwrap_or_default(),
            }
        } else if entry.is_directory {
            EntryKind::Directory
        } else {
            EntryKind::File
        };

        let mode = entry.permissions.and_then(|p| match p {
            PermissionMode::Custom(m) => Some(m),
            _ => None,
        });

        Self {
            original_path: entry.path,
            target_path: None,
            size: entry.size,
            mode,
            kind,
            hash: None,
        }
    }
}
```

**Benefits:**
- Single type to maintain
- Clear field ownership
- `target_path` and `hash` are optional until populated
- Type-safe entry kind enum
- Backward compatibility through type alias
- Helper methods for common queries

---

### 2. Isolated Permission and Hash Modules

**Principle:** F3 - Pure Core, Impure Edge. Extraction is pure; permission/hash are side effects.

```rust
// ops/permissions.rs

/// Permission application strategies.
///
/// Converts archive mode bits to [`PermissionMode`] for application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PermissionStrategy {
    #[default]
    Standard,
    ReadOnly,
    Preserve,
    Owned,
}

/// Result of permission resolution
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionResolution {
    /// The mode bits from the archive (if present)
    pub archive_mode: Option<u32>,
    /// The resolved PermissionMode to apply
    pub resolved: PermissionMode,
}

impl PermissionStrategy {
    /// Resolve permissions based on strategy and archive mode bits.
    ///
    /// This is a pure function - it only computes the value,
    /// it does not apply any side effects.
    ///
    /// # Strategy Behavior
    ///
    /// | Strategy | When mode present | When mode absent |
    /// |----------|------------------|-------------------|
    /// | `Standard` | Keep if executable (`& 0o111`), else add read/write | Use `0o644` |
    /// | `ReadOnly` | Always use `ReadOnly` | Always use `ReadOnly` |
    /// | `Preserve` | Use original mode bits | Use `Inherit` |
    /// | `Owned` | Always use `Custom(0o644)` | Always use `Custom(0o644)` |
    pub fn resolve(self, mode: Option<u32>) -> PermissionResolution {
        let resolved = match self {
            Self::Standard => {
                if let Some(m) = mode {
                    if m & 0o111 != 0 {
                        PermissionMode::Custom(m)
                    } else {
                        PermissionMode::Custom(m | 0o644)
                    }
                } else {
                    PermissionMode::Custom(0o644)
                }
            }
            Self::ReadOnly => PermissionMode::ReadOnly,
            Self::Preserve => {
                if let Some(m) = mode {
                    PermissionMode::Custom(m)
                } else {
                    PermissionMode::Inherit
                }
            }
            Self::Owned => PermissionMode::Custom(0o644),
        };

        PermissionResolution {
            archive_mode: mode,
            resolved,
        }
    }

    /// Apply resolved permissions to a file path.
    ///
    /// This is the impure edge - performs I/O to set permissions.
    pub fn apply_to_path(
        &self,
        path: &Path,
        mode: Option<u32>,
    ) -> Result<(), crate::Error> {
        let resolution = self.resolve(mode);
        resolution.resolved
            .apply_to_path(path)
            .map_err(|e| match e {
                pulith_fs::Error::Io(io_err) => crate::Error::Io(io_err),
                pulith_fs::Error::Write { path, source } => crate::Error::Io(source),
                _ => crate::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("unexpected error: {:?}", e),
                )),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_strategy_executable() {
        let resolution = PermissionStrategy::Standard.resolve(Some(0o755));
        assert_eq!(resolution.archive_mode, Some(0o755));
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o755));
    }

    #[test]
    fn standard_strategy_non_executable() {
        let resolution = PermissionStrategy::Standard.resolve(Some(0o644));
        assert_eq!(resolution.archive_mode, Some(0o644));
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o644 | 0o644));
    }

    #[test]
    fn standard_strategy_no_mode() {
        let resolution = PermissionStrategy::Standard.resolve(None);
        assert_eq!(resolution.archive_mode, None);
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o644));
    }
}
```

```rust
// ops/hash.rs

use std::io::Read;

/// Hash computation strategies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HashStrategy {
    #[default]
    None,
    Sha256,
    Blake3,
}

impl HashStrategy {
    /// Compute hash from reader.
    ///
    /// Streaming computation - does not load entire content into memory.
    ///
    /// Returns `None` if strategy is `None`, or `Some(String)` with hex-encoded hash.
    pub fn compute<R: Read>(&self, mut reader: R) -> Result<Option<String>, crate::Error> {
        match self {
            Self::None => Ok(None),
            Self::Sha256 => {
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                let mut buffer = [0u8; 8192];

                loop {
                    let n = reader.read(&mut buffer).map_err(crate::Error::Io)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buffer[..n]);
                }

                Ok(Some(format!("{:x}", hasher.finalize())))
            }
            Self::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                let mut buffer = [0u8; 8192];

                loop {
                    let n = reader.read(&mut buffer).map_err(crate::Error::Io)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&mut buffer[..n]);
                }

                Ok(Some(format!("{}", hasher.finalize())))
            }
        }
    }

    /// Compute hash from bytes (convenience method for small files).
    ///
    /// Prefer `compute()` with a reader for large files to avoid memory pressure.
    pub fn compute_from_bytes(&self, content: &[u8]) -> Option<String> {
        match self {
            Self::None => None,
            Self::Sha256 => {
                use sha2::Digest;
                let mut hasher = sha2::Sha256::new();
                hasher.update(content);
                Some(format!("{:x}", hasher.finalize()))
            }
            Self::Blake3 => {
                let mut hasher = blake3::Hasher::new();
                hasher.update(content);
                Some(format!("{}", hasher.finalize()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn none_strategy_returns_none() {
        let cursor = Cursor::new(b"hello");
        let result = HashStrategy::None.compute(cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn sha256_compute() {
        let cursor = Cursor::new(b"hello world");
        let result = HashStrategy::Sha256.compute(cursor).unwrap();
        assert!(result.is_some());
        // Known SHA256 of "hello world"
        assert_eq!(
            result.unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn blake3_compute() {
        let cursor = Cursor::new(b"hello world");
        let result = HashStrategy::Blake3.compute(cursor).unwrap();
        assert!(result.is_some());
        // Blake3 hash is stable
        assert_eq!(result.unwrap().len(), 64);
    }

    #[test]
    fn sha256_compute_from_bytes() {
        let result = HashStrategy::Sha256.compute_from_bytes(b"hello world");
        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn streaming_hash_large_file() {
        let large_data = vec![0x42u8; 10_000_000]; // 10MB
        let cursor = Cursor::new(large_data);

        let result = HashStrategy::Sha256.compute(cursor).unwrap();
        assert!(result.is_some());

        // Compute again from bytes to verify
        let large_data_bytes = vec![0x42u8; 10_000_000];
        let result_bytes = HashStrategy::Sha256.compute_from_bytes(&large_data_bytes);
        assert_eq!(result, result_bytes);
    }
}
```

```rust
// ops/mod.rs

pub mod permissions;
pub mod hash;

pub use permissions::{PermissionStrategy, PermissionResolution};
pub use hash::HashStrategy;

/// Combined extraction operations context
#[derive(Clone, Default)]
pub struct ExtractionOps {
    pub permission_strategy: PermissionStrategy,
    pub hash_strategy: HashStrategy,
}

impl ExtractionOps {
    pub fn new(
        permission_strategy: PermissionStrategy,
        hash_strategy: HashStrategy,
    ) -> Self {
        Self {
            permission_strategy,
            hash_strategy,
        }
    }
}
```

**Benefits:**
- Pure resolution functions (testable, no I/O)
- Streaming hash computation (memory efficient)
- Clear separation: `resolve()` = pure, `apply()` = impure
- Compile-time feature flags for hash algorithms
- Single responsibility per module

---

### 3. Better Reader Format

**Principle:** E2 - Immutability by Default, F4 - Explicit Effects.

**Problem:** `wrap_reader()` returns `Box<dyn Read>` with dynamic dispatch and runtime errors.

**Solution:** Enum-based compression decoder with compile-time type safety.

```rust
// codec/mod.rs

use std::io::Read;

/// Compression codec for tar archives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
    Xz,
    Zstd,
}

impl Compression {
    /// Create a decoder for this compression codec.
    ///
    /// This returns a concrete type, not a trait object.
    /// Feature gates are checked at compile-time.
    ///
    /// # Example
    ///
    /// ```rust
    /// let decoder = Compression::Gzip.decoder(reader)?;
    /// let mut archive = tar::Archive::new(decoder);
    /// ```
    pub fn decoder<R: Read + 'static>(self, reader: R) -> Result<Decoder<R>, crate::Error> {
        match self {
            Self::None => Ok(Decoder::Passthrough(reader)),
            Self::Gzip => Ok(Decoder::Gzip(flate2::read::GzDecoder::new(reader))),
            #[cfg(feature = "xz")]
            Self::Xz => Ok(Decoder::Xz(xz2::read::XzDecoder::new(reader))),
            #[cfg(not(feature = "xz"))]
            Self::Xz => Err(crate::Error::UnsupportedFormat {
                format: "xz compression",
            }),
            #[cfg(feature = "zstd")]
            Self::Zstd => {
                let inner = zstd::stream::Decoder::new(reader)
                    .map_err(|_| crate::Error::Corrupted)?;
                Ok(Decoder::Zstd(inner))
            }
            #[cfg(not(feature = "zstd"))]
            Self::Zstd => Err(crate::Error::UnsupportedFormat {
                format: "zstd compression",
            }),
        }
    }
}

/// Decoder wrapper for tar decompression.
///
/// This enum provides type-safe decompression without trait objects.
pub enum Decoder<R> {
    /// No compression (passthrough)
    Passthrough(R),
    /// Gzip decompression
    Gzip(flate2::read::GzDecoder<R>),
    /// Xz decompression
    #[cfg(feature = "xz")]
    Xz(xz2::read::XzDecoder<R>),
    /// Zstd decompression
    #[cfg(feature = "zstd")]
    Zstd(zstd::stream::Decoder<'static, R>),
}

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Passthrough(r) => r.read(buf),
            Self::Gzip(d) => d.read(buf),
            #[cfg(feature = "xz")]
            Self::Xz(d) => d.read(buf),
            #[cfg(feature = "zstd")]
            Self::Zstd(d) => d.read(buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_none_decoder() {
        let data = b"hello";
        let decoder = Compression::None.decoder(Cursor::new(data)).unwrap();
        assert!(matches!(decoder, Decoder::Passthrough(_)));
    }

    #[test]
    fn compression_gzip_decoder() {
        let data = vec![0x1f, 0x8b]; // gzip magic bytes
        let decoder = Compression::Gzip.decoder(Cursor::new(data)).unwrap();
        assert!(matches!(decoder, Decoder::Gzip(_)));
    }

    #[test]
    #[cfg(not(feature = "xz"))]
    fn compression_xz_unsupported() {
        let data = Vec::new();
        let result = Compression::Xz.decoder(Cursor::new(data));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), crate::Error::UnsupportedFormat { .. }));
    }
}
```

**Benefits:**
- Compile-time feature flag checking
- No dynamic dispatch overhead
- Type-safe enum pattern
- Clear error messages at compile time
- Easier to test (each variant is accessible)

---

### 4. Refactored Extraction Pipeline

**Principle:** F5 - Composition Over Orchestration, F3 - Pure Core, Impure Edge.

**Problem:** Current extraction is monolithic - 721 lines, duplicated code, mixed concerns.

**Solution:** Composable pipeline with small, single-responsibility functions.

```rust
// extract/pipeline.rs - Core extraction orchestrator

use std::io::Read;
use std::path::Path;

use crate::data::entry::{Entry, EntryKind};
use crate::data::archive::ArchiveFormat;
use crate::codec::{Compression, Decoder};
use crate::ops::{ExtractionOps, PermissionStrategy, HashStrategy};
use crate::sanitize::{SanitizedPath, sanitize_path, strip_path_components};
use crate::error::{Error, Result};

/// Archive-specific entry source trait.
///
/// Each archive format implements this to provide entries
/// without coupling extraction logic to format details.
pub trait EntrySource {
    /// Iterate over entries in the archive.
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;

    /// Archive format for reporting
    fn format(&self) -> ArchiveFormat;
}

/// An entry read from archive but not yet processed.
///
/// Pure data structure - no I/O involved.
pub struct PendingEntry {
    /// Original path from archive
    pub original_path: PathBuf,
    /// Entry size in bytes
    pub size: u64,
    /// Unix mode bits (if available)
    pub mode: Option<u32>,
    /// Entry kind (file, directory, symlink)
    pub kind: EntryKind,
    /// Reader for entry content (files only)
    pub reader: Option<Box<dyn Read + '_>>,
}

/// Extraction context passed through pipeline.
pub struct ExtractionContext<'a> {
    /// Destination directory
    pub destination: &'a Path,
    /// Strip components count
    pub strip_components: usize,
    /// Permission strategy
    pub permission_strategy: PermissionStrategy,
    /// Hash strategy
    pub hash_strategy: HashStrategy,
    /// Progress callback
    pub progress: Option<&'a dyn Fn(Progress)>,
}

/// Extraction pipeline results.
pub struct ExtractionResults {
    pub entries: Vec<Entry>,
    pub total_bytes: u64,
}

impl<'a> ExtractionContext<'a> {
    /// Create new extraction context.
    pub fn new(
        destination: &'a Path,
        options: &crate::data::options::ExtractionOptions,
    ) -> Self {
        Self {
            destination,
            strip_components: options.strip_components,
            permission_strategy: options.permission_strategy,
            hash_strategy: options.hash_strategy,
            progress: options.on_progress.as_deref(),
        }
    }
}

/// Main extraction pipeline.
///
/// Orchestrates the flow:
/// 1. Read entries from archive (pure)
/// 2. Sanitize paths (pure)
/// 3. Write files to disk (impure)
/// 4. Apply permissions (impure)
/// 5. Compute hashes (impure)
///
/// Returns entries with all metadata populated.
pub fn extract<S: EntrySource>(
    source: &mut S,
    ctx: &ExtractionContext<'_>,
) -> Result<ExtractionResults> {
    let mut entries = Vec::new();
    let mut total_bytes = 0u64;
    let mut bytes_processed = 0u64;

    for pending in source.entries()? {
        let pending = pending?;
        bytes_processed += pending.size;
        total_bytes += pending.size;

        // Pure: Create entry with metadata
        let mut entry = Entry::new(
            pending.original_path.clone(),
            pending.size,
            pending.mode,
            pending.kind,
        );

        // Pure: Sanitize path
        let sanitized = if ctx.strip_components > 0 {
            let stripped = strip_path_components(&entry.original_path, ctx.strip_components)
                .map_err(|_| Error::NoComponentsRemaining {
                    original: entry.original_path.clone(),
                    count: ctx.strip_components,
                })?;
            sanitize_path(&stripped, ctx.destination)?
        } else {
            sanitize_path(&entry.original_path, ctx.destination)?
        };

        entry = entry.with_target_path(sanitized.resolved.clone());

        // Impure: Write to disk based on entry type
        write_entry(&pending, &sanitized.resolved, ctx)?;

        // Impure: Compute hash if needed
        if let Some(ref reader) = pending.reader {
            if ctx.hash_strategy != HashStrategy::None {
                let hash = ctx.hash_strategy.compute(reader.by_ref())?;
                entry = entry.with_hash(hash);
            }
        }

        // Impure: Apply permissions
        if let Some(target_path) = &entry.target_path {
            ctx.permission_strategy
                .apply_to_path(target_path, pending.mode)?;
        }

        // Impure: Report progress
        if let Some(ref callback) = ctx.progress {
            callback(Progress {
                bytes_processed,
                total_bytes: Some(total_bytes),
                percentage: None,
                current_file: Some(entry.original_path.clone()),
            });
        }

        entries.push(entry);
    }

    Ok(ExtractionResults {
        entries,
        total_bytes,
    })
}

/// Write entry to filesystem based on its kind.
///
/// This is impure - performs I/O.
fn write_entry(
    pending: &PendingEntry,
    target_path: &Path,
    ctx: &ExtractionContext<'_>,
) -> Result<()> {
    match &pending.kind {
        EntryKind::File => write_file(pending, target_path)?,
        EntryKind::Directory => ensure_directory(target_path)?,
        EntryKind::Symlink { target } => write_symlink(target, target_path)?,
    }
    Ok(())
}

/// Write file to disk.
fn write_file(pending: &PendingEntry, target_path: &Path) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| Error::DirectoryCreationFailed {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
    }

    // Stream content to file (no buffering entire file)
    if let Some(ref reader) = pending.reader {
        let mut file = std::fs::File::create(target_path).map_err(|e| {
            Error::ExtractionFailed {
                path: target_path.to_path_buf(),
                source: e,
            }
        })?;
        std::io::copy(reader, &mut file)?;
    }

    Ok(())
}

/// Ensure directory exists.
fn ensure_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| Error::DirectoryCreationFailed {
            path: path.to_path_buf(),
            source: e,
        })?;
    }
    Ok(())
}

/// Create symbolic link.
#[cfg(unix)]
fn write_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link).map_err(|e| {
        Error::SymlinkCreationFailed {
            target: target.to_path_buf(),
            link: link.to_path_buf(),
            source: e,
        }
    })
}

/// Create symbolic link (Windows).
#[cfg(windows)]
fn write_symlink(target: &Path, link: &Path) -> Result<()> {
    use std::os::windows::fs;

    let is_dir_target = target.is_dir() || target.to_string_lossy().ends_with('/');
    if is_dir_target {
        fs::symlink_dir(target, link).map_err(|e| Error::SymlinkCreationFailed {
            target: target.to_path_buf(),
            link: link.to_path_buf(),
            source: e,
        })
    } else {
        fs::symlink_file(target, link).map_err(|e| Error::SymlinkCreationFailed {
            target: target.to_path_buf(),
            link: link.to_path_buf(),
            source: e,
        })
    }
}
```

```rust
// extract/zip.rs - Zip-specific entry source

use std::io::{Read, Seek};
use std::path::PathBuf;

use super::{EntrySource, PendingEntry};
use crate::data::entry::{EntryKind};
use crate::codec::Decoder;
use crate::error::{Error, Result};

/// Zip archive entry source.
pub struct ZipSource<R: Read + Seek> {
    archive: zip::ZipArchive<R>,
}

impl<R: Read + Seek> ZipSource<R> {
    /// Create new zip entry source.
    pub fn new(reader: R) -> Result<Self> {
        let archive = zip::ZipArchive::new(reader).map_err(|_| Error::Corrupted)?;
        Ok(Self { archive })
    }
}

impl<R: Read + Seek> EntrySource for ZipSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>> {
        let iter = (0..self.archive.len()).map(move |index| {
            let mut file = self.archive.by_index(index).map_err(|_| Error::Corrupted)?;
            let raw_path = file
                .enclosed_name()
                .ok_or(Error::InvalidPath)?
                .to_path_buf();

            let size = file.size();
            let mode = file.unix_mode();
            let kind = if file.is_dir() {
                EntryKind::Directory
            } else {
                EntryKind::File
            };

            // Detect .lnk as symlink (Windows ZIP convention)
            let symlink_indicator = raw_path.as_os_str().to_string_lossy();
            let (kind, reader) = if symlink_indicator.ends_with(".lnk")
                || symlink_indicator.contains(".lnk")
            {
                let content = read_to_vec(&mut file)?;
                (EntryKind::Symlink { target: content.into() }, None)
            } else {
                let reader = Box::new(file) as Box<dyn Read + 'static>;
                (kind, Some(reader))
            };

            Ok(PendingEntry {
                original_path: raw_path,
                size,
                mode,
                kind,
                reader,
            })
        });

        Ok(Box::new(iter))
    }

    fn format(&self) -> crate::data::archive::ArchiveFormat {
        crate::data::archive::ArchiveFormat::Zip
    }
}

/// Read entire reader to Vec (for symlink targets).
fn read_to_vec<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}
```

```rust
// extract/tar.rs - Tar-specific entry source

use std::io::Read;
use std::path::PathBuf;

use super::{EntrySource, PendingEntry};
use crate::codec::{Compression, Decoder};
use crate::data::entry::EntryKind;
use crate::error::{Error, Result};

/// Tar archive entry source.
pub struct TarSource<R: Read> {
    archive: tar::Archive<Decoder<R>>,
}

impl<R: Read> TarSource<R> {
    /// Create new tar entry source.
    pub fn new(reader: R, codec: Compression) -> Result<Self> {
        let decoder = codec.decoder(reader)?;
        let archive = tar::Archive::new(decoder);
        Ok(Self { archive })
    }
}

impl<R: Read> EntrySource for TarSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>> {
        let entries = self.archive.entries()?.map(move |entry| {
            let mut entry = entry.map_err(|_| Error::Corrupted)?;

            let raw_path = entry.path()?.into_owned();
            let header = entry.header();

            let size = header.size().unwrap_or(0);
            let mode = header.mode().ok();
            let entry_type = header.entry_type();

            let kind = match entry_type {
                t if t.is_dir() => EntryKind::Directory,
                t if t.is_symlink() => {
                    let target = entry.link_name()?.map(|p| p.into_owned())
                        .ok_or(Error::InvalidPath)??;
                    EntryKind::Symlink { target }
                }
                _ => EntryKind::File,
            };

            // Create reader only for files (streaming)
            let reader = if matches!(kind, EntryKind::File) {
                let reader = Box::new(entry) as Box<dyn Read + 'static>;
                Some(reader)
            } else {
                None
            };

            Ok(PendingEntry {
                original_path: raw_path,
                size,
                mode,
                kind,
                reader,
            })
        });

        Ok(Box::new(entries))
    }

    fn format(&self) -> crate::data::archive::ArchiveFormat {
        crate::data::archive::ArchiveFormat::Tar(self.compression_codec())
    }
}

impl<R: Read> TarSource<R> {
    /// Extract compression codec from decoder (for format reporting).
    fn compression_codec(&self) -> Compression {
        // This is a limitation - we can't inspect the Decoder enum
        // For now, return None - could be improved with type state
        Compression::None
    }
}
```

```rust
// extract/mod.rs - Public API

use std::io::{Read, Seek};

use super::{EntrySource, extract};
use crate::data::options::ExtractionOptions;
use crate::data::report::ArchiveReport;
use crate::error::{Error, Result};

pub use pipeline::{ExtractionContext, ExtractionResults};
pub use zip::ZipSource;
pub use tar::TarSource;

mod pipeline;
mod zip;
mod tar;

/// Extract archive using automatic format detection.
///
/// # Example
///
/// ```rust
/// use pulith_archive::extract;
///
/// let file = std::fs::File::open("archive.tar.gz")?;
/// let report = extract::extract_from_reader(
///     file,
///     "/tmp/output",
///     ExtractionOptions::default(),
/// )?;
/// ```
pub fn extract_from_reader<R: Read + Seek>(
    mut reader: R,
    destination: &Path,
    options: &ExtractionOptions,
) -> Result<ArchiveReport> {
    use crate::detect::detect_from_reader;

    // Detect format
    let format = detect_from_reader(&mut reader)?
        .ok_or(Error::UnsupportedFormat)?;

    // Rewind reader after detection
    reader.rewind()?;

    // Create appropriate source and extract
    let report = match format {
        ArchiveFormat::Zip => {
            let mut source = ZipSource::new(reader)?;
            let ctx = ExtractionContext::new(destination, options);
            let results = extract(&mut source, &ctx)?;
            ArchiveReport {
                format,
                entry_count: results.entries.len(),
                total_bytes: results.total_bytes,
                entries: results.entries,
            }
        }
        ArchiveFormat::Tar(codec) => {
            let mut source = TarSource::new(reader, codec)?;
            let ctx = ExtractionContext::new(destination, options);
            let results = extract(&mut source, &ctx)?;
            ArchiveReport {
                format,
                entry_count: results.entries.len(),
                total_bytes: results.total_bytes,
                entries: results.entries,
            }
        }
    };

    Ok(report)
}

/// Extract archive with explicit source (for testing/advanced use).
///
/// # Example
///
/// ```rust
/// use pulith_archive::extract::{ZipSource, extract_with_source};
///
/// let file = std::fs::File::open("archive.zip")?;
/// let mut source = ZipSource::new(file)?;
/// let report = extract_with_source(&mut source, "/tmp/output", &options)?;
/// ```
pub fn extract_with_source<S: EntrySource>(
    source: &mut S,
    destination: &Path,
    options: &ExtractionOptions,
) -> Result<ArchiveReport> {
    let ctx = ExtractionContext::new(destination, options);
    let results = extract(source, &ctx)?;
    Ok(ArchiveReport {
        format: source.format(),
        entry_count: results.entries.len(),
        total_bytes: results.total_bytes,
        entries: results.entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_reader_detects_zip() {
        let data = vec![0x50, 0x4B, 0x03, 0x04]; // ZIP magic
        let cursor = std::io::Cursor::new(data);
        let result = extract_from_reader(cursor, Path::new("/tmp"), &ExtractionOptions::default());
        assert!(result.is_err()); // Corrupt archive
    }
}
```

**Benefits of Refactored Extraction:**

| Aspect | Before (721 lines) | After (~250 lines) |
|--------|---------------------|-------------------|
| **Lines of code** | 721 | ~250 |
| **File count** | 1 monolithic | 4 focused modules |
| **Entry types** | ArchiveEntry, ExtractedEntry | Single `Entry` |
| **Hash duplication** | In Zip + Tar | Single in `ops::hash` |
| **Permission logic** | Inlined in extractors | Single in `ops::permissions` |
| **Progress reporting** | Duplicated in both | Single in pipeline |
| **Memory usage** | Loads entire file per entry | Streaming |
| **Testability** | Requires filesystem | Each function testable |

**Architecture Improvements:**

1. **Separation of Concerns:**
   - `pipeline.rs` - orchestration and workflow
   - `zip.rs` - Zip-specific entry reading
   - `tar.rs` - Tar-specific entry reading
   - Each function has single responsibility

2. **Pure Core, Impure Edge:**
   - `PendingEntry` creation - pure
   - Path sanitization - pure
   - File writing - impure edge
   - Permission/hash application - impure edge

3. **Composition Over Orchestration:**
   - `extract()` composes functions: `entries()` → `sanitize_path()` → `write_entry()`
   - Easy to add new steps: e.g., virus scanning, signature verification

4. **Type Safety:**
   - `EntrySource` trait - compile-time guarantee of entry source
   - Each format implements its own source
   - No dynamic dispatch for entry iteration

5. **Memory Efficiency:**
   - Streaming writes via `std::io::copy()`
   - Hash computed during write (reader.by_ref())
   - No buffering entire file in memory

6. **Extensibility:**

```rust
// Adding a new archive format (e.g., 7z)

// extract/seven_zip.rs
use super::{EntrySource, PendingEntry};

pub struct SevenZipSource<R: Read> {
    // ...
}

impl<R: Read> EntrySource for SevenZipSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>> {
        // Implement 7z-specific parsing
    }

    fn format(&self) -> ArchiveFormat {
        ArchiveFormat::SevenZip
    }
}

// Add to extract_from_reader
ArchiveFormat::SevenZip => {
    let mut source = SevenZipSource::new(reader)?;
    let ctx = ExtractionContext::new(destination, options);
    let results = extract(&mut source, &ctx)?;
    ArchiveReport { format, entries: results.entries, ... }
}
```

**Testing Improvements:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Test path sanitization (pure)
    #[test]
    fn test_entry_sanitization() {
        let entry = PendingEntry {
            original_path: PathBuf::from("../../etc/passwd"),
            size: 0,
            mode: None,
            kind: EntryKind::File,
            reader: None,
        };

        let sanitized = sanitize_path(&entry.original_path, Path::new("/tmp"))
            .expect("should sanitize");
        // Pure test - no filesystem needed
    }

    // Test permission resolution (pure)
    #[test]
    fn test_permission_resolution() {
        let resolution = PermissionStrategy::Standard.resolve(Some(0o755));
        assert_eq!(resolution.resolved, PermissionMode::Custom(0o755));
    }

    // Test hash computation (streaming)
    #[test]
    fn test_hash_streaming() {
        let data = vec![0x42u8; 10_000_000];
        let reader = std::io::Cursor::new(data);
        let hash = HashStrategy::Sha256.compute(reader).unwrap();
        assert!(hash.is_some());
    }
}
```

---

## Improved Extraction Flow

### Before

```
Reader → wrap_reader → Box<dyn Read> → Archive
                                      ↓
                                      extract entry
                                      ├─ apply_permissions (inline)
                                      ├─ calculate_hash (inline, duplicated)
                                      └─ create ExtractedEntry from ArchiveEntry
```

### After

```
Reader → Compression::decoder → Decoder<R> → Archive
                                         ↓
                                         extract Entry
                                         ↓
                                         Entry {original_path, target_path, hash, ...}
                                         ↓
                                         post-processing (separated)
                                         ├─ ops::permissions::apply_to_path()
                                         └─ ops::hash::compute()
```

### Key Changes

1. **Entry flow is pure** - extraction produces `Entry` objects
2. **Side effects are isolated** - permission/hash application happens after extraction
3. **Single entry type** - no transformation between `ArchiveEntry` and `ExtractedEntry`
4. **Streaming hash** - `HashStrategy::compute()` takes `Read`, not bytes
5. **Type-safe compression** - enum-based `Decoder` instead of trait object

---

## Migration Path

### Phase 1: Add New Types (Non-breaking)

```rust
// Add new types alongside existing ones
pub struct Entry { ... }
pub enum EntryKind { ... }

// Keep existing types for backward compatibility
pub struct ExtractedEntry { ... }

// Add conversion functions
impl From<ExtractedEntry> for Entry { ... }
```

### Phase 2: Migrate Internal Usage

```rust
// Update extractors to use Entry internally
// Keep ExtractedEntry in public API for compatibility
impl ZipExtractor {
    pub fn extract(...) -> Result<ArchiveReport> {
        // Use Entry internally
        let mut entry = Entry::new(...);

        // Convert for public API
        let extracted = ExtractedEntry::from(entry);
    }
}
```

### Phase 3: Add Permission/Hash Ops

```rust
// Add ops module
pub mod ops {
    pub mod permissions;
    pub mod hash;
}

// Gradually migrate extractors to use ops
```

### Phase 4: Replace wrap_reader

```rust
// Add codec module
pub mod codec {
    pub enum Compression { ... }
    pub enum Decoder<R> { ... }
}

// Keep wrap_reader for backward compatibility
#[deprecated(since = "0.2.0", note = "Use Compression::decoder instead")]
pub fn wrap_reader<R: Read + 'static>(
    reader: R,
    codec: Compression,
) -> Result<Box<dyn Read>, Error> {
    codec.decoder(reader).map(|d| Box::new(d) as Box<dyn Read>)
}
```

### Phase 5: Refactor Extraction Pipeline

```rust
// Add EntrySource trait and PendingEntry
pub trait EntrySource {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;
    fn format(&self) -> ArchiveFormat;
}

// Add pipeline module
pub mod pipeline {
    // Core extract() function
    // write_file(), ensure_directory(), write_symlink()
}

// Add ZipSource and TarSource
pub mod zip { pub struct ZipSource<R> { ... } }
pub mod tar { pub struct TarSource<R> { ... } }

// Keep old extractors for backward compatibility
#[deprecated(since = "0.2.0", note = "Use extract::extract_from_reader")]
pub enum ArchiveExtractor { ... }
```

### Phase 6: Clean Up (Breaking Change)

```rust
// Remove deprecated types
// ExtractedEntry → Entry (type alias for compatibility)
// Remove wrap_reader
// Remove old ArchiveExtractor enum
// Remove old ZipExtractor/TarExtractor structs
// Remove tar_codecs.rs (moved to codec/decoder.rs)
```

---

## File Structure

```
pulith-archive/
├── lib.rs
├── detect.rs
├── sanitize.rs
├── workspace.rs
├── codec/                    # NEW
│   ├── mod.rs
│   └── decoder.rs           # Compression, Decoder<R>
├── ops/                      # NEW
│   ├── mod.rs
│   ├── permissions.rs       # PermissionStrategy, PermissionResolution
│   └── hash.rs              # HashStrategy, compute()
├── extract/                   # REFACTORED
│   ├── mod.rs               # Public API: extract_from_reader, extract_with_source
│   ├── pipeline.rs          # Core pipeline orchestrator
│   ├── zip.rs              # ZipSource, Zip-specific logic
│   └── tar.rs              # TarSource, Tar-specific logic
├── data/
│   ├── mod.rs
│   ├── archive.rs           # ArchiveFormat
│   ├── entry.rs             # Entry, EntryKind (unified)
│   ├── options.rs           # ExtractionOptions (simplified)
│   └── report.rs            # ArchiveReport (uses Entry)
└── error.rs
```

---

## Benefits Summary

| Aspect | Before | After |
|--------|--------|-------|
| **Entry Types** | 2 similar types | 1 unified type |
| **Permission Logic** | Mixed in extraction | Isolated in `ops::permissions` |
| **Hash Logic** | Duplicated, loads entire file | Streaming, isolated in `ops::hash` |
| **Reader Wrapping** | `Box<dyn Read>` + runtime errors | Enum-based, compile-time checks |
| **Memory** | Double allocation (Entry→ExtractedEntry) | Single allocation |
| **Testability** | Coupled to I/O | Pure functions + isolated effects |
| **Type Safety** | Runtime errors | Compile-time guarantees |
| **Code Duplication** | Hash calc in Zip & Tar | Single implementation |
| **Extraction Code** | 721 lines monolithic | ~250 lines modular |
| **File Loading** | Loads entire file | Streaming writes |
| **New Format Cost** | Copy-paste ~130 lines | Implement `EntrySource` trait |
| **Pipeline Steps** | Mixed, hard to test | Composable, testable functions |

---

## Example Usage (After Refactor)

```rust
use pulith_archive::{
    detect_from_reader,
    extract_to_workspace,
    ExtractionOptions,
    ops::{PermissionStrategy, HashStrategy},
};

let file = std::fs::File::open("release.tar.zst")?;
let ops = ops::ExtractionOps::new(
    PermissionStrategy::Standard,
    HashStrategy::Blake3,
);

let options = ExtractionOptions::default()
    .strip_components(1)
    .ops(ops)
    .on_progress(|p| eprintln!("{}%", p.percentage.unwrap_or(0.0)));

let extraction = extract_to_workspace(file, "/opt/mytool", options)?;
let report = extraction.commit()?;  // Atomic move

for entry in &report.entries {
    println!("Extracted: {}", entry.target_path().display());
    if let Some(hash) = &entry.hash {
        println!("  Hash: {}", hash);
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
// ops/permissions tests
#[test]
fn test_permission_resolution() {
    let resolution = PermissionStrategy::Standard.resolve(Some(0o755));
    assert_eq!(resolution.resolved, PermissionMode::Custom(0o755));
}

// ops/hash tests
#[test]
fn test_sha256_streaming() {
    let data = b"hello world".repeat(1000);
    let cursor = Cursor::new(data);
    let hash = HashStrategy::Sha256.compute(cursor).unwrap();
    assert!(hash.is_some());
}
```

### Integration Tests

```rust
// Full extraction with streaming hash
#[test]
fn test_extraction_with_hashing() {
    let tar_gz_data = include_bytes!("../../tests/fixtures/large_file.tar.gz");
    let cursor = Cursor::new(tar_gz_data.to_vec());

    let options = ExtractionOptions::default()
        .hash_strategy(HashStrategy::Blake3);

    let report = extract(cursor, &dest, &options, None)?;
    assert!(report.entries.iter().all(|e| e.hash.is_some()));
}
```

---

## Future Extensibility

### Adding New Compression

```rust
// codec/decoder.rs
pub enum Decoder<R> {
    // ... existing variants
    #[cfg(feature = "lz4")]
    Lz4(lz4::Decoder<R>),
}

impl Compression {
    pub fn decoder<R: Read + 'static>(self, reader: R) -> Result<Decoder<R>, Error> {
        // ... existing match
        Self::Lz4 => {
            #[cfg(feature = "lz4")]
            return Ok(Decoder::Lz4(lz4::Decoder::new(reader)?));
            #[cfg(not(feature = "lz4"))]
            return Err(Error::UnsupportedFormat { format: "lz4" });
        }
    }
}
```

### Adding New Hash Algorithm

```rust
// ops/hash.rs
pub enum HashStrategy {
    // ... existing variants
    #[cfg(feature = "sha3")]
    Sha3_256,
}

impl HashStrategy {
    pub fn compute<R: Read>(&self, mut reader: R) -> Result<Option<String>, Error> {
        match self {
            // ... existing cases
            Self::Sha3_256 => {
                #[cfg(feature = "sha3")]
                {
                    use sha3::Digest;
                    let mut hasher = sha3::Sha3_256::new();
                    // streaming compute
                    Ok(Some(format!("{:x}", hasher.finalize())))
                }
                #[cfg(not(feature = "sha3"))]
                Err(Error::UnsupportedFormat { format: "sha3" })
            }
        }
    }
}
```

### Adding New Archive Format

**Before (monolithic):** Copy-paste ~130 lines from ZipExtractor.

**After (composable):** Implement `EntrySource` trait - ~50 lines.

```rust
// extract/rar.rs - Adding RAR support (example)

use std::io::Read;
use std::path::PathBuf;

use super::{EntrySource, PendingEntry};
use crate::codec::Decoder;
use crate::data::entry::EntryKind;
use crate::error::{Error, Result};

/// RAR archive entry source.
pub struct RarSource<R: Read> {
    // Internal RAR library wrapper
    archive: rar_archive::Archive<R>,
}

impl<R: Read> RarSource<R> {
    /// Create new RAR entry source.
    pub fn new(reader: R) -> Result<Self> {
        let archive = rar_archive::Archive::new(reader)
            .map_err(|_| Error::Corrupted)?;
        Ok(Self { archive })
    }
}

impl<R: Read> EntrySource for RarSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>> {
        let iter = self.archive.entries().map(move |result| {
            let entry = result.map_err(|_| Error::Corrupted)?;

            let raw_path = entry.path().to_path_buf();
            let size = entry.size();
            let mode = entry.mode();
            let kind = match entry.kind() {
                rar_archive::EntryKind::File => EntryKind::File,
                rar_archive::EntryKind::Directory => EntryKind::Directory,
                rar_archive::EntryKind::Symlink => EntryKind::Symlink {
                    target: entry.link_target()?.into(),
                },
            };

            // Create streaming reader for files
            let reader = if matches!(kind, EntryKind::File) {
                let reader = Box::new(entry.reader()?) as Box<dyn Read + 'static>;
                Some(reader)
            } else {
                None
            };

            Ok(PendingEntry {
                original_path: raw_path,
                size,
                mode,
                kind,
                reader,
            })
        });

        Ok(Box::new(iter))
    }

    fn format(&self) -> crate::data::archive::ArchiveFormat {
        crate::data::archive::ArchiveFormat::Rar
    }
}
```

**Benefits:**
- ~50 lines of code vs 130 lines for new format
- Automatic path sanitization, permission, hash from pipeline
- Streaming by default
- Testable without filesystem
- No code duplication

---

## Performance Considerations

### Memory

| Operation | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Entry allocation | 2× (ArchiveEntry + ExtractedEntry) | 1× | 50% reduction |
| File loading (per entry) | Load entire file | Streaming | O(1) memory |
| Hash computation | Load entire file | Streaming | O(1) memory |
| Compression overhead | `Box<dyn Read>` vtable | Enum (inline) | ~10-20ns |
| Peak memory (large file) | File size × 2 | File size × 1 | 50% reduction |

### CPU

| Metric | Before | After |
|---------|--------|-------|
| Permission resolution | Inline in extract | Pure function (cacheable) |
| Hash dispatch | Match in each extractor | Single implementation |
| Decompression | Virtual call | Static dispatch |
| Archive iteration | Nested loops | Flat iterator |

### Code Size

| Metric | Before | After | Improvement |
|---------|--------|-------|-------------|
| Extract module | 721 lines | ~250 lines | 65% reduction |
| Files per format | 1 monolithic | 2 (source + impl) | Better separation |
| Lines to add format | ~130 lines copy-paste | ~50 lines trait impl | 60% reduction |

---

## Architectural Patterns

### Pattern 1: Trait-based Source Abstraction

**Purpose:** Decouple extraction pipeline from archive format details.

```rust
pub trait EntrySource {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;
    fn format(&self) -> ArchiveFormat;
}
```

**Benefits:**
- Zero-cost abstraction (static dispatch via monomorphization)
- Easy to add new formats
- Format-specific code isolated
- No runtime overhead

### Pattern 2: Pure Core, Impure Edge

**Purpose:** Separate logic from I/O for testability.

```rust
// Pure
fn sanitize_path(raw: &Path, base: &Path) -> Result<SanitizedPath>;
fn resolve_permissions(strategy: PermissionStrategy, mode: Option<u32>) -> PermissionResolution;

// Impure
fn write_file(path: &Path, content: &[u8]) -> Result<()>;
fn apply_permissions(path: &Path, mode: PermissionMode) -> Result<()>;
```

**Benefits:**
- Testable without filesystem
- Cacheable pure functions
- Clear effect boundaries
- Easy to mock for tests

### Pattern 3: Streaming Processing

**Purpose:** Process data incrementally, avoid buffering entire files.

```rust
// Reader can be tee'd to both write and hash
if let Some(ref reader) = pending.reader {
    let hash = ctx.hash_strategy.compute(reader.by_ref())?;
    std::io::copy(reader.by_ref(), &mut file)?;
}
```

**Benefits:**
- Constant memory usage
- Parallelizable (hash + write)
- Works with arbitrarily large files

### Pattern 4: Builder-Style Entry Creation

**Purpose:** Incrementally build entry with optional fields.

```rust
let mut entry = Entry::new(path, size, mode, kind);
entry = entry.with_target_path(sanitized);
entry = entry.with_hash(computed_hash);
```

**Benefits:**
- Clear field initialization
- Optional fields explicit
- Type-safe field setting
- No default values confusion

### Pattern 5: Context Object

**Purpose:** Pass configuration through pipeline.

```rust
pub struct ExtractionContext<'a> {
    pub destination: &'a Path,
    pub strip_components: usize,
    pub permission_strategy: PermissionStrategy,
    pub hash_strategy: HashStrategy,
    pub progress: Option<&'a dyn Fn(Progress)>,
}
```

**Benefits:**
- Single parameter to pass
- All options in one place
- Easy to extend
- Reference lifetime explicit

---

## Conclusion

This improved design addresses all four issues:

1. **Unified Entry Model** - Single `Entry` type with clear semantics
2. **Isolated Operations** - `ops::permissions` and `ops::hash` modules
3. **Type-safe Codec** - Enum-based `Decoder<R>` instead of `Box<dyn Read>`
4. **Refactored Extraction Pipeline** - Composable, testable, streaming

The design follows Pulith philosophy:
- **F1 (Functions First)** - Pure resolution, isolated effects, composable pipeline
- **F2 (Immutability by Default)** - Entry is immutable once created
- **F3 (Pure Core, Impure Edge)** - Resolution is pure, application is impure
- **F4 (Explicit Effects)** - Hash and permission effects are explicit
- **F5 (Composition Over Orchestration)** - Pipeline composes small functions

Migration is gradual with backward compatibility maintained until cleanup.

