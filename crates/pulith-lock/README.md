# pulith-lock

Deterministic lock model and lock diff primitives for Pulith.

## What This Crate Owns

`pulith-lock` provides a stable lock-file shape and deterministic change reporting.

## Main Types

- `LockFile`
- `LockedResource`
- `LockDiff`
- `LockResourceChange`

## Basic Usage

```rust
use pulith_lock::{LockFile, LockedResource};

let mut lock = LockFile::default();
lock.upsert(
    "example/runtime",
    LockedResource::new("1.0.0", "https://example.com/runtime.tgz"),
);

let json = lock.to_json()?;
let parsed = LockFile::from_json(&json)?;
let diff = lock.diff(&parsed);
assert!(diff.is_empty());
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Codec Boundary

Lock serialization routes through `pulith-serde-backend`, and compact/pretty JSON parity is covered by tests.

## See Also

- `docs/design/lock.md`
