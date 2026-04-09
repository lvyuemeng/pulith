# pulith-store

Composable local artifact and extracted-tree storage.

## Role

`pulith-store` owns durable local materialization and provenance-aware lookup.

It should not absorb install policy or state policy.

## Main APIs

- `StoreReady`
- `StoreRoots`
- `StoreKey`
- `StoredArtifact`
- `ExtractedArtifact`
- `StoreProvenance`

## Basic Usage

```rust
use pulith_store::{StoreKey, StoreReady, StoreRoots};
use std::path::PathBuf;

let store = StoreReady::initialize(StoreRoots::new(
    PathBuf::from("artifacts"),
    PathBuf::from("extracts"),
    PathBuf::from("metadata"),
))?;
let _artifact = store.put_artifact_bytes(&StoreKey::logical("runtime")?, b"hello")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## How To Use It

Use `pulith-store` when you want:

- durable artifact/extract registration
- provenance persistence
- semantic lookup by `StoreKey` or derived resource identity
- prune planning instead of blind cleanup

See `docs/design/store.md`.
