# pulith-fs

Atomic filesystem and workspace primitives.

## Main APIs

- `atomic_read`, `atomic_write`
- `atomic_symlink`
- `hardlink_or_copy`
- `replace_dir`
- `Transaction`
- `Workspace`

## Basic Usage

```rust
use pulith_fs::{atomic_write, AtomicWriteOptions};

atomic_write("state.json", br#"{}"#, AtomicWriteOptions::default())?;
# Ok::<(), pulith_fs::Error>(())
```

See `docs/design/fs.md`.
