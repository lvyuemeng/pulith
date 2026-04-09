# pulith-store

Composable local artifact and extract storage.

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

See `docs/design/store.md`.
