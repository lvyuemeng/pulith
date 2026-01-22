# pulith-archive

Archive extraction with path sanitization and transactional staging. Mechanism-only.

## Architecture

```
pulith-archive/
├── lib.rs
├── detect.rs              # Format detection
├── sanitize.rs            # Path sanitization
├── workspace.rs           # Transactional extraction
├── extract/               # Per-format implementations
│   ├── mod.rs             # ZipExtractor, TarExtractor, ArchiveExtractor enum
│   └── tar_codecs.rs      # Gzip/Xz/Zstd wrapping
├── data/
│   ├── mod.rs
│   ├── archive.rs         # ArchiveFormat, Compression
│   ├── entry.rs           # ArchiveEntry
│   ├── options.rs         # ExtractionOptions
│   └── report.rs          # ArchiveReport, ExtractedEntry
├── progress.rs
└── error.rs
```

## Format Detection

```rust
detect_format(data: &[u8]) -> Option<ArchiveFormat>;
detect_from_reader<R: Read + Seek>(reader: &mut R) -> Result<Option<ArchiveFormat>>;

ArchiveFormat::Zip;
ArchiveFormat::Tar(Compression::Gzip | Zstd | Xz | None);
```

## Path Sanitization

```rust
// Returns sanitized path or error if path escapes base (zip-slip)
sanitize_path(entry_path: &Path, base: &Path) -> Result<SanitizedEntry>;

// Symlink target validation (critical security)
sanitize_symlink_target(target: &Path, symlink_location: &Path, base: &Path) -> Result<PathBuf>;

// Strip leading path components (like tar --strip-components)
strip_path_components(path: &Path, count: usize) -> Result<PathBuf>;
```

## Transactional Extraction

```rust
extract_to_workspace<R: Read + Seek>(
    reader: R,
    destination: &Path,
    options: ExtractionOptions,
) -> Result<WorkspaceExtraction>;

let extraction = extract_to_workspace(file, "/opt/mytool", options)?;
let report = extraction.commit()?;  // Atomic move
```

## Extraction Options

```rust
ExtractionOptions::default()
    .permission_strategy(PermissionStrategy::Standard)  // Unix mode mapping
    .hash_strategy(HashStrategy::None)                  // Optional SHA-256/Blake3
    .strip_components(0)                                // tar --strip-components
    .expected_total_bytes(None)                         // For progress calculation
    .on_progress(|p| println!("{}%", p.percentage))
```

## Permission Strategy

| Strategy | Behavior |
|----------|----------|
| `Standard` | Unix 0755→executable, 0644→writable |
| `ReadOnly` | All files read-only (for caches) |
| `Preserve` | Use archive permissions exactly |
| `Owned` | Ignore archive perms, use process umask |

## Hash Strategy

| Strategy | Output |
|----------|--------|
| `None` | No hash calculated |
| `Sha256` | hex-encoded SHA-256 |
| `Blake3` | Blake3 digest |

## Output: ArchiveReport

```rust
ArchiveReport {
    format: ArchiveFormat,
    entry_count: usize,
    total_bytes: u64,
    entries: Vec<ExtractedEntry>,
}

ExtractedEntry {
    original_path: PathBuf,
    target_path: PathBuf,
    size: u64,
    permissions: Option<u32>,
    is_directory: bool,
    is_symlink: bool,
    symlink_target: Option<PathBuf>,
    hash: Option<String>,  // If HashStrategy is enabled
}
```

## Extractors

```rust
// Enum-based dispatch (avoids lifetime issues with trait objects)
pub enum ArchiveExtractor {
    Zip(ZipExtractor),
    Tar(TarExtractor),
}

impl ArchiveExtractor {
    pub fn extract<R: Read + Seek + 'static>(
        &self,
        reader: R,
        destination: &Path,
        options: &ExtractionOptions,
        workspace: Option<&Workspace>,
    ) -> Result<ArchiveReport>;
}

// Factory
extractor_for(format: ArchiveFormat) -> Option<ArchiveExtractor>;
```

## Feature Matrix

| Format | Seek Required | Streaming | Symlinks | Permissions | Hash |
|--------|---------------|-----------|----------|-------------|------|
| Zip | ✓ | ✗ | ✓ | ✓ | ✓ |
| Tar.Gz | ✗ | ✓ | ✓ | ✓ | ✓ |
| Tar.Xz | ✗ | ✓ | ✓ | ✓ | ✓ |
| Tar.Zstd | ✗ | ✓ | ✓ | ✓ | ✓ |
| Tar.Plain | ✗ | ✓ | ✓ | ✓ | ✓ |

## Example

```rust
use pulith_archive::{detect_from_reader, extract_to_workspace, ExtractionOptions};

let file = std::fs::File::open("release.tar.zst")?;
let options = ExtractionOptions::default()
    .strip_components(1)
    .hash_strategy(HashStrategy::Blake3)
    .on_progress(|p| eprintln!("{}%", p.percentage.unwrap_or(0.0)));

let extraction = extract_to_workspace(file, "/opt/mytool", options)?;
let report = extraction.commit()?;  // Atomic move

for entry in &report.entries {
    println!("Extracted: {} ({})", entry.target_path.display(), entry.size);
    if let Some(hash) = &entry.hash {
        println!("  Hash: {}", hash);
    }
}
```

## Dependencies

```
thiserror
tempfile
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

## Relationship

```
pulith-archive
    ├── detect.rs
    ├── sanitize.rs
    ├── workspace.rs
    ├── extract/
    │   ├── mod.rs
    │   └── tar_codecs.rs
    └── data/
        ├── mod.rs
        ├── archive.rs
        ├── entry.rs
        ├── options.rs
        └── report.rs

Uses: pulith-fs (Workspace)
```

## Security Features

- **Zip-slip prevention**: All paths are sanitized against base directory
- **Symlink validation**: Symlink targets are resolved and checked against base
- **Path normalization**: Handles `.`, `..`, mixed separators, double slashes
- **Transactional**: Uses Workspace for atomic commits (or cleanup on abort)
