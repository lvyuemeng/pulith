# pulith-install Design

## Overview

Atomic file operations for staged package installation. Works with `StoreLayout` to enable safe install→verify→activate workflows.

## Scope

**Included:**
- `atomic_replace`: Safe file/directory replacement with same-FS optimization
- `StoreLayout`: Directory structure for package storage
- `ensure_layout`: Create store directory structure

**Excluded:**
- Download logic (handled by `pulith-fetch`)
- Shim creation (handled by `pulith-shim`)
- State tracking (handled by `pulith-registry`)
- Version resolution (handled by caller)

## Directory Structure

```
.pulith/                      # root
├── versions/                 # installed package versions
│   ├── 1.0.0/
│   │   ├── bin/              # binaries
│   │   └── lib/              # libraries
│   └── 2.0.0/
│       └── ...
├── current -> versions/1.0.0 # symlink to active version
└── staging/                  # temporary install location
```

## Public API

### atomic_replace

```rust
/// Atomically replace `src` with `dst`.
///
/// Uses `rename()` when same filesystem (atomic).
/// Falls back to copy→rename→delete when cross-filesystem.
///
/// # Errors
///
/// Returns `AtomicReplaceError` on failure.
///
/// # Guarantees
///
/// - On success: `src` is gone, `dst` exists with full content
/// - On failure: `src` remains, `dst` unchanged
/// - Never leaves partial state in destination
pub fn atomic_replace(src: &Path, dst: &Path) -> Result<(), AtomicReplaceError>;
```

### StoreLayout

```rust
pub struct StoreLayout {
    root: PathBuf,
    versions: PathBuf,
    current: PathBuf,
    staging: PathBuf,
}

impl StoreLayout {
    pub fn builder() -> StoreLayoutBuilder;
    pub fn root(&self) -> &Path;
    pub fn versions(&self) -> &Path;
    pub fn current(&self) -> &Path;
    pub fn staging(&self) -> &Path;
    pub fn version(&self, name: &str) -> PathBuf;
    pub fn version_bin(&self, name: &str, bin: &str) -> PathBuf;
}
```

### StoreLayoutBuilder

```rust
pub struct StoreLayoutBuilder {
    root: Option<PathBuf>,
}

impl StoreLayoutBuilder {
    pub fn new() -> Self;
    pub fn root(mut self, path: impl Into<PathBuf>) -> Self;
    pub fn build(self) -> Result<StoreLayout, std::io::Error>;
}
```

### ensure_layout

```rust
/// Ensure the store directory structure exists.
///
/// Creates `versions/` and `staging/` directories if they don't exist.
///
/// # Errors
///
/// Returns `StoreLayoutError` if directory creation fails.
pub fn ensure_layout(layout: &StoreLayout) -> Result<(), StoreLayoutError>;
```

## Error Types

### AtomicReplaceError

```rust
#[derive(Debug, Error)]
pub enum AtomicReplaceError {
    #[error("source path does not exist: {0}")]
    SourceNotFound(PathBuf),

    #[error("destination parent directory does not exist: {0}")]
    DestinationParentNotFound(PathBuf),

    #[error("failed to create parent directory: {0}")]
    CreateParentFailed(#[source] std::io::Error),

    #[error("failed to rename (same-FS): {0}")]
    RenameFailed(#[source] std::io::Error),

    #[error("failed to copy to staging location: {0}")]
    CopyFailed(#[source] std::io::Error),

    #[error("failed to remove stale destination: {0}")]
    RemoveStaleFailed(#[source] std::io::Error),

    #[error("failed to rename from staging: {0}")]
    FinalRenameFailed(#[source] std::io::Error),

    #[error("failed to clean up staging after failure: {0}")]
    CleanupFailed(PathBuf, #[source] std::io::Error),
}
```

### StoreLayoutError

```rust
#[derive(Debug, Error)]
pub enum StoreLayoutError {
    #[error("failed to create directory: {0}")]
    CreateDir(#[source] std::io::Error),

    #[error("failed to create symlink: {0}")]
    CreateSymlink(#[source] std::io::Error),

    #[error("store root is not a directory")]
    RootNotDirectory,

    #[error("version path is not a directory")]
    VersionNotDirectory,
}
```

## Staged Install Pattern

```
1. fetch(url) → staging/temp
2. verify_checksum(staging/temp)
3. atomic_replace(staging/temp, versions/X.X.X)
4. atomic_replace(versions/X.X.X, current)  [optional symlink update]
```

## Error Recovery

| Error | Recovery Strategy |
|-------|-------------------|
| `SourceNotFound` | Retry fetch, abort install |
| `CreateParentFailed` | Check permissions, retry |
| `RenameFailed` | Cross-FS fallback auto-retries |
| `CopyFailed` | Retry, check disk space |
| `RemoveStaleFailed` | Manual cleanup, retry |
| `FinalRenameFailed` | Cleanup staging, retry |
| `CleanupFailed` | Manual cleanup required |

## Composition

### With pulith-fetch

```rust
use pulith_core::{atomic_replace, StoreLayout};

fn install_version(
    layout: &StoreLayout,
    url: &str,
    version: &str,
    checksum: &str
) -> Result<(), InstallError> {
    let staging_path = layout.staging().join(version);
    download_to_file(url, &staging_path).await?;
    verify_checksum(&staging_path, checksum)?;
    let version_path = layout.version(version);
    atomic_replace(&staging_path, &version_path)?;
    Ok(())
}
```

## Design Decisions

### Why Staging Directory?

- Cross-filesystem compatibility
- Verification before activation
- Atomic activation (instant version switch)
- Rollback capability (old version preserved until confirmed)

### Why Not In-Place Update?

- Corrupted installation on failure
- No rollback if update fails mid-way
- Difficult verification (can't verify while files in use)

### Symlink vs Directory for `current/`

Both patterns supported via `StoreLayout`:
- **Symlink**: Simple, atomic switch, easy rollback
- **Directory**: Multiple shims, complex version detection
- Caller chooses pattern based on use case

## Module Structure

```
pulith-install/src/
├── lib.rs              # Public exports
├── install.rs          # atomic_replace
├── store.rs            # StoreLayout, ensure_layout
└── error.rs            # Error types
```
