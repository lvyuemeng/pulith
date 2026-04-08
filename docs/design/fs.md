# pulith-fs

Cross-platform atomic filesystem primitives. Mechanism-only.

## Purpose

`pulith-fs` provides the low-level filesystem building blocks used by the rest of Pulith:

- atomic file replacement
- directory replacement
- hardlink-or-copy fallback behavior
- transactional staging workspaces
- locked read-modify-write file transactions

It does not know what a package, tool, or resource is. It only knows how to move bytes safely.

## Public Surface

```rust
use pulith_fs::{
    atomic_read, atomic_write, copy_dir_all, hardlink_or_copy, replace_dir,
    AtomicWriteOptions, HardlinkOrCopyOptions, ReplaceDirOptions,
    Transaction, Workspace, WorkspaceReport,
};
```

## Primitives

### Atomic file I/O

```rust
atomic_write(path, bytes, AtomicWriteOptions::new())?;
let bytes = atomic_read(path)?;
```

`atomic_write` writes to a temporary file in the destination directory and renames it into place.

### Directory operations

```rust
copy_dir_all(src, dest)?;
hardlink_or_copy(src, dest, HardlinkOrCopyOptions::new())?;
replace_dir(src, dest, ReplaceDirOptions::new())?;
```

- `copy_dir_all` recursively copies a directory tree
- `hardlink_or_copy` prefers hardlinks and falls back according to options
- `replace_dir` performs an atomic-ish directory swap, with Windows retry handling

## Workflow Layer

### Workspace

`Workspace` is the staging primitive for resource preparation.

```rust
let ws = Workspace::new(staging_dir, final_destination)?;

ws.create_dir("bin")?;
ws.create_dir_all("lib/nested")?;
ws.write("bin/tool", bytes)?;
ws.copy_file(source_path, "share/tool.txt")?;

let report: WorkspaceReport = ws.report()?;
ws.commit()?;
```

Current behavior:

- staging root is created eagerly
- writes create parent directories automatically
- relative paths are sanitized and may not escape the staging root
- dropping an uncommitted workspace removes the staging directory
- `commit()` replaces the destination directory with the staged tree

`WorkspaceReport` currently exposes:

- `staging_root`
- `destination_root`
- `file_count`
- `directory_count`
- `total_bytes`

### Transaction

`Transaction` is the locked read-modify-write primitive for persistent files.

```rust
let tx = Transaction::open("registry.json")?;

tx.execute(|bytes| {
    let mut next = bytes.to_vec();
    next.extend_from_slice(b"\nupdated=true");
    Ok(next)
})?;
```

Current behavior:

- opens or creates the file
- holds an exclusive lock for the transaction lifetime
- supports `read()`, `write()`, and `execute()`
- uses atomic replacement for writes

## Current Maturity

Stable and intended for use:

- atomic file writes and reads
- directory replacement
- hardlink-or-copy fallback
- basic staging workspaces
- basic locked transactions

Still intentionally small:

- workspace manifests are summary-only rather than a rich install plan
- transaction schema migration is out of scope
- state-file semantics remain opaque-byte oriented

## Platform Behavior

| OS | Replace strategy | Link behavior | Notes |
|----|------------------|---------------|-------|
| Windows | retrying remove + rename | junction / symlink primitives elsewhere | locked files are a first-class concern |
| Unix | rename-based replace | native hardlink / symlink support | simpler swap semantics |

## Relationship

```text
pulith-fs
  primitives/   atomic I/O and replacement helpers
  workflow/     Workspace and Transaction
  resource/     file-backed resource helpers
```

Used by:

- `pulith-archive`
- `pulith-fetch`
- future higher-level crates such as `pulith-store` and `pulith-install`
