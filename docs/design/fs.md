# pulith-fs Design Document

## Overview

**Role**: Cross-platform atomic filesystem primitives.

**Philosophy**: Mechanism-only. Provides safe primitives for moving bytes and metadata. Does not understand "tools", "packages", or "installations" - only files, directories, links, and their atomic operations.

**Key Guarantees**:
- All state-changing operations are atomic with rollback on failure
- Consistent behavior across Windows, macOS, Linux
- No temporary or intermediate states visible to observers

## Core API

### Atomic Write

```rust
pub fn atomic_write(
    path: impl AsRef<Path>,
    content: &[u8],
    options: AtomicWriteOptions,
) -> Result<()>
```

Writes to a temp file adjacent to target, fsyncs, then renames over target.

**Options**:
- `permissions`: File permissions (Unix mode, Windows ignored)
- `prefix`: Temp file prefix (default: `.`)
- `suffix`: Temp file suffix (default: `.tmp`)

### Atomic Symlink (Unix)

```rust
#[cfg(unix)]
pub fn atomic_symlink(
    target: impl AsRef<Path>,
    link_path: impl AsRef<Path>,
) -> Result<()>
```

Creates new symlink at `link_path` pointing to `target`, then atomically swaps into place.

### Junction Point (Windows)

```rust
#[cfg(windows)]
pub fn atomic_symlink_junction(
    target: impl AsRef<Path>,
    link_path: impl AsRef<Path>,
) -> Result<()>
```

Creates a junction point (directory symlink equivalent) on Windows.

### Replace Directory (Windows)

```rust
#[cfg(windows)]
pub fn replace_dir(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    options: ReplaceDirOptions,
) -> Result<()>
```

Atomically swaps `src` directory to `dest` on Windows. Handles locked files with configurable retry.

**Options**:
- `retry_count`: Max retry attempts (default: 64)
- `retry_delay`: Base delay between retries (default: 8ms, exponential backoff)

### Hardlink or Copy

```rust
pub fn hardlink_or_copy(
    src: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    options: HardlinkOrCopyOptions,
) -> Result<()>
```

Attempts hardlink first. If cross-device or not supported, falls back to copy based on strategy.

**Options**:
- `fallback`: `Copy` or `Error` (default: `Error`)
- `permissions`: Fallback copy permissions (Unix only)

### Additional Primitives

```rust
pub fn atomic_read(path: &Path) -> Result<Vec<u8>>;

#[cfg(unix)]
pub fn atomic_symlink_file(target, link) -> Result<()>;

#[cfg(unix)]
pub fn atomic_symlink_dir(target, link) -> Result<()>;

// ... other composable primitives
```

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("operation failed")]
    Failed,

    #[error("path not found")]
    NotFound,

    #[error("permission denied")]
    PermissionDenied,

    #[error("already exists")]
    AlreadyExists,

    #[error("retry limit exceeded")]
    RetryLimitExceeded,

    #[error("cross-device hardlink not supported")]
    CrossDeviceHardlink,

    #[error("symlink not supported on this platform")]
    SymlinkNotSupported,

    #[error("path exceeds maximum length")]
    PathTooLong,
}
```

## Platform Behavior

### Windows

- Uses `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`
- Exponential backoff retry for locked files: 8ms → 16ms → 32ms → ...
- Long path support: `\\?\` prefix for paths > 260 chars
- Junction points for directory symlinks (legacy, always works)
- Symbolic links require developer mode or elevated permissions

### Unix (Linux, macOS, BSD)

- Uses `rename(2)` for atomic directory swap
- Uses `symlink(2)` for symlinks
- Uses `link(2)` for hardlinks
- Permissions: respects umask, optional explicit mode

## Dependencies

```toml
[dependencies]
nix = { version = "0.28", features = ["fs"] }
windows-sys = "0.52"
thiserror = "1"

[dev-dependencies]
tempfile = "4"
```

## Example Usage

```rust
use pulith_fs::{atomic_write, hardlink_or_copy, AtomicWriteOptions};

// Atomic write with custom permissions
atomic_write(
    "/etc/mytool/config.toml",
    b"key = \"value\"",
    AtomicWriteOptions::new().permissions(0o644),
)?;

// Optimize storage with hardlinks, fall back to copy
hardlink_or_copy(
    "/cache/node-v20.0.0/bin/node",
    "/usr/local/bin/node",
    HardlinkOrCopyOptions::new().fallback(FallbackStrategy::Copy),
)?;
```

## Testing

```rust
#[cfg(unix)]
mod unix_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("test.txt");
        atomic_write(&path, b"data", AtomicWriteOptions::new())?;
        assert_eq!(std::fs::read(&path)?, b"data");
        Ok(())
    }

    #[test]
    fn test_symlink_is_atomic() -> Result<()> {
        let dir = tempdir()?;
        let target = dir.path().join("target");
        let link = dir.path().join("link");

        std::fs::write(&target, "data")?;
        atomic_symlink(&target, &link)?;

        assert!(link.is_symlink());
        assert_eq!(std::fs::read_to_string(link)?, "data");
        Ok(())
    }
}

#[cfg(windows)]
mod windows_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_junction_point() -> Result<()> {
        let dir = tempdir()?;
        let junction = dir.path().join("link");
        atomic_symlink_junction(dir.path(), &junction)?;
        assert!(junction.is_dir());
        Ok(())
    }

    #[test]
    fn test_locked_file_retry() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("src");
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&src)?;

        let mut options = ReplaceDirOptions::new();
        options.retry_count(10).retry_delay_ms(16);

        replace_dir(&src, &dest, options)?;
        assert!(dest.exists());
        Ok(())
    }
}
```

## Workspace

A transactional directory for preparing resources before atomic commit. Mechanism-only: provides file/directory operations in an isolated staging area, with atomic commit to final destination.

### API

```rust
pub struct Workspace(Cow<'static, Path>);

impl Workspace {
    /// Create workspace at root directory.
    pub fn new(root: impl Into<Cow<'static, Path>>) -> Result<Self>;

    /// Write bytes to a path relative to workspace root.
    pub fn write(&self, path: &Path, content: &[u8]) -> Result<()>;

    /// Write all bytes (convenience wrapper).
    pub fn write_all(&self, path: &Path, content: &[u8]) -> Result<()> {
        self.write(path, content)
    }

    /// Create a single directory.
    pub fn create_dir(&self, path: &Path) -> Result<()>;

    /// Create directory and all parent directories.
    pub fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Atomically commit workspace to destination. Consumes self.
    pub fn commit(self, destination: impl AsRef<Path>) -> Result<()>;
}
```

### Guarantees

- All writes go to paths within the workspace root.
- `commit()` atomically swaps workspace to destination using `rename(2)` or `MoveFileEx`.
- Dropping without commit removes the workspace (automatic cleanup).

### Example

```rust
let workspace = Workspace::new("/tmp/staging.abc123")?;

workspace.write("bin/tool", &executable_bytes)?;
workspace.create_dir_all("lib/nested")?;
workspace.write("lib/native.so", &lib_bytes)?;

workspace.commit("/usr/local/bin/tool")?;
```

## Transaction

Concurrent-safe read-modify-write on a persistent file. Mechanism-only: handles locking and atomic write-back; user manages format and serialization.

### API

```rust
pub struct Transaction(Cow<'static, Path>);

impl Transaction {
    /// Open file for transactional access.
    pub fn open(path: impl Into<Cow<'static, Path>>) -> Result<Self>;

    /// Execute f(current_bytes) → new_bytes atomically.
    pub fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Option<&[u8]>) -> Result<Vec<u8>>,
        T: serde::de::DeserializeOwned;
}
```

### Guarantees

- Exclusive file lock held during transaction.
- Atomic write-back on success.
- File unchanged on failure (automatic rollback).
- Opaque bytes: user handles serialization/deserialization.

### Example

```rust
let tx = Transaction::open("registry.json")?;

let version: Version = tx.execute(|bytes| {
    let data: Option<Registry> = bytes.map(|b| serde_json::from_slice(b)).transpose()?;
    let mut registry = data.unwrap_or_default();
    let old_version = registry.version;

    registry.version = new_version.to_string();
    serde_json::to_vec(&registry)
})?;

println!("Upgraded from {} to {}", old_version, version);
```

## Relationship to Other Crates

```
pulith-fs (this crate)
    │
    ├── primitives/          # atomic_write, symlink, hardlink...
    ├── workspace/           # Staging directory for resource preparation
    └── transaction/         # File-based state with locking
```

## Future Considerations

- `atomic_append()` for log files
- `atomic_metadata()` for permission-only changes
- Async variants (consider `tokio` or `async-std` support)
