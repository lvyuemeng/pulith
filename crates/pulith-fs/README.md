# pulith-fs

Atomic filesystem and transactional workspace primitives.

## Role

`pulith-fs` owns filesystem mechanics, not install/store policy.

It provides:

- atomic reads and writes
- directory replacement
- symlink creation
- hardlink-or-copy helpers
- transactional staging/workspace behavior

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

## How To Use It

Use this crate whenever a higher-level workflow needs safe filesystem mutation but should not re-implement:

- atomic replacement
- staging directories
- copy vs hardlink behavior
- symlink creation details

See `docs/design/fs.md`.
