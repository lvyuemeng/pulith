# pulith-archive

Archive extraction with path sanitization and transactional staging. Mechanism-only.

## Architecture

```text
pulith-archive/
├── lib.rs                    # Public API exports
├── format.rs                 # Format detection and compression codecs
├── extract.rs                # Main extraction pipeline and workspace support
├── sanitize.rs               # Path sanitization and security validation
├── options.rs                # Extraction options and permission/hash strategies
├── entry.rs                  # Archive entry types and report structures
├── error.rs                  # Error types and Result type alias
├── extract/                  # Per-format implementations
│   ├── mod.rs                # EntrySource trait and PendingEntry
│   ├── tar.rs                # TarSource implementation
│   └── zip.rs                # ZipSource implementation
└── workspace.rs              # Transactional extraction wrapper
```

## Core Types

### ArchiveFormat and Compression

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    Tar(TarCompress),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TarCompress {
    None,
    Gzip,
    Xz,
    Zstd,
}
```

### EntrySource Trait

```rust
pub trait EntrySource {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;
    fn format(&self) -> format::ArchiveFormat;
}
```

### Archive Entry

```rust
#[derive(Clone, Debug)]
pub struct Entry {
    pub original_path: PathBuf,
    pub target_path: Option<PathBuf>,
    pub size: u64,
    pub mode: Option<u32>,
    pub kind: EntryKind,
    pub hash: Option<String>,
}

#[derive(Clone, Debug)]
pub enum EntryKind {
    File,
    Directory,
    Symlink { target: PathBuf },
}
```

## Format Detection

```rust
// Detect format from byte header
pub fn detect_format(data: &[u8]) -> Option<ArchiveFormat>;

// Detect format from reader (rewinds after reading)
pub fn detect_from_reader<R: Read + Seek>(reader: &mut R) -> io::Result<Option<ArchiveFormat>>;

// Supported formats
ArchiveFormat::Zip;                    // ZIP files
ArchiveFormat::Tar(TarCompress::None); // Plain tar
ArchiveFormat::Tar(TarCompress::Gzip); // Gzip-compressed tar
ArchiveFormat::Tar(TarCompress::Xz);   // XZ-compressed tar
ArchiveFormat::Tar(TarCompress::Zstd); // Zstd-compressed tar
```

## Path Sanitization

```rust
// Sanitize entry path with options (main API)
pub fn sanitize_path_with_options<P: AsRef<Path>, B: AsRef<Path>>(
    entry_path: P,
    base: B,
    options: &ExtractOptions,
) -> Result<SanitizedPath>;

// Sanitize symlink target with options
pub fn sanitize_symlink_target_with_options<P: AsRef<Path>, L: AsRef<Path>, B: AsRef<Path>>(
    target: P,
    symlink_location: L,
    base: B,
    options: &ExtractOptions,
) -> Result<PathBuf>;

// Result of sanitization
#[derive(Clone, Debug)]
pub struct SanitizedPath {
    pub original: PathBuf,
    pub resolved: PathBuf,
}
```

### Security Features

- **Zip-slip prevention**: Rejects absolute paths and ensures resolved paths stay within base directory
- **Symlink validation**: Prevents absolute symlink targets and ensures symlinks don't escape base
- **Path normalization**: Handles `.`, `..`, mixed separators, and double slashes
- **Component stripping**: Supports `--strip-components` functionality

## Extraction Options

```rust
#[derive(Clone, Default)]
pub struct ExtractOptions {
    pub perm_strategy: PermissionStrategy,
    pub hash_strategy: HashStrategy,
    pub strip_components: usize,
    pub expected_total_bytes: Option<u64>,
    pub on_progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
}

impl ExtractOptions {
    pub fn permission_strategy(mut self, strategy: PermissionStrategy) -> Self;
    pub fn hash_strategy(mut self, strategy: HashStrategy) -> Self;
    pub fn strip_components(mut self, n: usize) -> Self;
    pub fn expected_total_bytes(mut self, bytes: u64) -> Self;
    pub fn on_progress(mut self, callback: Arc<dyn Fn(Progress) + Send + Sync>) -> Self;
}
```

### Permission Strategies

| Strategy | Behavior |
|----------|----------|
| `Standard` (default) | Unix 0755→executable, 0644→writable |
| `ReadOnly` | All files read-only (for caches) |
| `Preserve` | Use archive permissions exactly |
| `Owned` | Ignore archive perms, use process umask |

### Hash Strategies

| Strategy | Output |
|----------|--------|
| `None` (default) | No hash calculated |
| `Sha256` | hex-encoded SHA-256 |
| `Blake3` | Blake3 digest |

### Platform Behavior

**Unix**: Full permission support with all `PermissionStrategy` variants functional.
**Windows**: Permission handling is a no-op - `PermissionStrategy` has no effect, but API accepts permission options for compatibility.

## Main Extraction API

### Direct Extraction

```rust
// Extract with automatic format detection
pub fn extract_from_reader<R: Read + Seek>(
    reader: R,
    destination: &Path,
    options: &ExtractOptions,
) -> Result<ArchiveReport>;

// Extract with explicit source
pub fn extract_with_source<S: EntrySource>(
    source: &mut S,
    destination: &Path,
    options: &ExtractOptions,
) -> Result<ArchiveReport>;
```

### Transactional Extraction

```rust
// Extract to workspace for atomic commit
pub fn extract_to_workspace<R: Read + Seek>(
    reader: R,
    destination: &Path,
    options: ExtractOptions,
) -> Result<WorkspaceExtraction>;

pub struct WorkspaceExtraction {
    workspace: Workspace,
    report: ArchiveReport,
}

impl WorkspaceExtraction {
    pub fn commit(self) -> Result<ArchiveReport>;  // Atomic move
    pub fn abort(self);                            // Cleanup
    pub fn report(&self) -> &ArchiveReport;        // Access report
}
```

## Archive Report

```rust
#[derive(Clone, Debug)]
pub struct ArchiveReport {
    pub format: ArchiveFormat,
    pub entry_count: usize,
    pub total_bytes: u64,
    pub entries: Vec<Entry>,
}
```

## Per-Format Implementations

### ZIP Support

```rust
pub struct ZipSource<R: Read + Seek> {
    archive: zip::ZipArchive<R>,
}

impl<R: Read + Seek> EntrySource for ZipSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;
    fn format(&self) -> format::ArchiveFormat;
}
```

**Features:**
- Full ZIP support with path validation
- Symlink detection (Windows .lnk files)
- Streaming extraction with owned bytes
- Seek required for ZIP format

### TAR Support

```rust
pub struct TarSource<R: Read> {
    archive: tar::Archive<Decoder<R>>,
}

impl<R: Read> EntrySource for TarSource<R> {
    fn entries(&mut self) -> Result<Box<dyn Iterator<Item = Result<PendingEntry>> + '_>>;
    fn format(&self) -> format::ArchiveFormat;
}
```

**Features:**
- Streaming extraction (no seek required)
- Compression codec support (Gzip, XZ, Zstd)
- Native symlink support
- Directory and file extraction

## Feature Matrix

| Format | Seek Required | Streaming | Symlinks | Permissions | Hash | Platform |
|--------|---------------|-----------|----------|-------------|------|----------|
| Zip | ✓ | ✗ | ✓ (Windows .lnk) | ✓ | ✓ | Cross-platform |
| Tar.Gz | ✗ | ✓ | ✓ | ✓ | ✓ | Cross-platform |
| Tar.Xz | ✗ | ✓ | ✓ | ✓ | ✓ | Cross-platform |
| Tar.Zstd | ✗ | ✓ | ✓ | ✓ | ✓ | Cross-platform |
| Tar.Plain | ✗ | ✓ | ✓ | ✓ | ✓ | Cross-platform |

## Example Usage

### Basic Extraction

```rust
use pulith_archive::{extract_from_reader, ExtractOptions};

let file = std::fs::File::open("release.tar.zst")?;
let options = ExtractOptions::default()
    .strip_components(1)
    .hash_strategy(HashStrategy::Blake3)
    .on_progress(|p| eprintln!("{}%", p.percentage.unwrap_or(0.0)));

let report = extract_from_reader(file, "/opt/mytool", &options)?;

for entry in &report.entries {
    println!("Extracted: {} ({})", entry.target_path.display(), entry.size);
    if let Some(hash) = &entry.hash {
        println!("  Hash: {}", hash);
    }
}
```

### Transactional Extraction

```rust
use pulith_archive::{extract_to_workspace, ExtractOptions};

let file = std::fs::File::open("release.tar.zst")?;
let options = ExtractOptions::default()
    .permission_strategy(PermissionStrategy::Standard)
    .hash_strategy(HashStrategy::Sha256);

let extraction = extract_to_workspace(file, "/opt/mytool", options)?;
let report = extraction.commit()?;  // Atomic move

println!("Successfully extracted {} entries", report.entry_count);
```

### Custom Progress Tracking

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

let counter = Arc::new(AtomicU64::new(0));
let counter_clone = counter.clone();

let options = ExtractOptions::default().on_progress(Arc::new(move |progress| {
    if let Some(percentage) = progress.percentage {
        println!("Progress: {:.1}%", percentage);
    }
}));

let report = extract_from_reader(file, "/opt/mytool", &options)?;
```

## Dependencies

```toml
[dependencies]
pulith-fs = { path = "../pulith-fs" }
thiserror.workspace = true
tempfile.workspace = true

sha2 = { version = "0.10", optional = true }
hex = { version = "0.4", optional = true }
blake3 = { version = "1", optional = true }
tar = { version = "0.4.44", optional = true }
xz2 = { version = "0.1.7", optional = true }
zstd = { version = "0.13.3", optional = true }
flate2 = { version = "1.1.8", optional = true }
zip = { version = "7.2.0", optional = true }

[features]
default = ["zip", "tar", "sha256", "blake3"]
zip = ["dep:zip"]
tar = ["dep:tar", "dep:flate2"]
xz = ["dep:xz2"]
zstd = ["dep:zstd"]
sha256 = ["dep:sha2", "dep:hex"]
blake3 = ["dep:blake3"]
```

## Relationship to pulith-fs

```text
pulith-archive
├── extract.rs (extract_to_workspace)
│   └── uses pulith-fs::workflow::Workspace
├── options.rs (PermissionStrategy)
│   └── uses pulith-fs::PermissionMode
└── workspace.rs (WorkspaceExtraction)
    └── wraps pulith-fs::workflow::Workspace
```

## Security Features

- **Zip-slip prevention**: All paths are sanitized against base directory
- **Symlink validation**: Symlink targets are resolved and checked against base
- **Path normalization**: Handles `.`, `..`, mixed separators, double slashes
- **Transactional**: Uses Workspace for atomic commits (or cleanup on abort)
- **Component stripping**: Prevents path traversal via `--strip-components`
- **Format validation**: Rejects unknown or corrupted archive formats

## Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unsupported archive format")]
    UnsupportedFormat,
    #[error("Corrupted archive")]
    Corrupted,
    #[error("Zip-slip attempt: {entry:?} -> {resolved:?}")]
    ZipSlip { entry: PathBuf, resolved: PathBuf },
    #[error("Absolute symlink target: {target:?} in {symlink:?}")]
    AbsoluteSymlinkTarget { target: PathBuf, symlink: PathBuf },
    #[error("Symlink escape: {target:?} -> {resolved:?}")]
    SymlinkEscape { target: PathBuf, resolved: PathBuf },
    #[error("No components remaining after stripping {count} from {original:?}")]
    NoComponentsRemaining { original: PathBuf, count: usize },
    // ... other errors
}

pub type Result<T> = std::result::Result<T, Error>;
```
