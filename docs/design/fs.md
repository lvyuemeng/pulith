# pulith-fs

Cross-platform atomic filesystem primitives. Mechanism-only.

## Primitives

```rust
// Atomic file operations
atomic_write(path, content, options) -> Result<()>;
atomic_symlink(target, link) -> Result<()>;  // Unix
hardlink_or_copy(src, dest, options) -> Result<()>;
replace_dir(src, dest, options) -> Result<()>;  // Windows
copy_dir_all(src, dest) -> Result<()>;

// Options
AtomicWriteOptions::new().permissions(0o644);
HardlinkOrCopyOptions::new().fallback(FallbackStrategy::Copy);
ReplaceDirOptions::new().retry_count(64).retry_delay(Duration::from_millis(8));
```

## Workflow

```rust
// Workspace: transactional staging
Workspace::new(staging_dir)?;
// write(), create_dir_all(), then commit(dest) or drop for cleanup

// Transaction: locked file access
Transaction::open(path)?;
// execute(|bytes| -> Result<Vec<u8>>)
```

## Resource

```rust
Resource::new(path)?;
// content() -> &mut [u8] (mmap if >= threshold)
// ensure_integrity()?  // Verify file unchanged
```

## Example

```rust
use pulith_fs::{atomic_write, Workspace, HardlinkOrCopyOptions};

// Atomic write
atomic_write("/etc/config.toml", b"data", options)?;

// Transactional install
let ws = Workspace::new("/tmp/staging")?;
ws.write("bin/tool", &bytes)?;
ws.create_dir_all("lib")?;
ws.commit("/opt/mytool")?;
```

## Dependencies

```
thiserror, memmap2, uuid, junction, fs2
```

## Platform Behavior

| OS | Swap | Symlinks | Retry |
|----|------|----------|-------|
| Windows | MoveFileEx | Junctions | 8ms→16ms→... |
| Unix | rename(2) | symlink(2) | - |

## Relationship

```
pulith-fs
    ├── primitives/  # atomic ops
    ├── workflow/    # Workspace, Transaction
    └── resource/    # Lazy mmap

Used by: pulith-fetch, pulith-archive
```
