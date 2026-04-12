# pulith-store

Composable artifact/extract storage with provenance persistence.

## What This Crate Owns

`pulith-store` provides durable local material storage and lookup.

It owns:

- artifact byte registration
- extracted tree registration
- semantic store keys
- provenance persistence
- metadata inspection and prune planning support

It does not own:

- install policy
- fetch retry policy
- state repair policy

## Main Types

- `StoreReady`
- `StoreRoots`
- `StoreKey`
- `StoredArtifact`
- `ExtractedArtifact`
- `StoreProvenance`
- `StoreMetadataRecord`

## Basic Usage

```rust
use pulith_store::{StoreKey, StoreReady, StoreRoots};
use std::path::PathBuf;

let store = StoreReady::initialize(StoreRoots::new(
    PathBuf::from("artifacts"),
    PathBuf::from("extracts"),
    PathBuf::from("metadata"),
))?;

let artifact = store.put_artifact_bytes(&StoreKey::logical("runtime")?, b"hello")?;
# let _ = artifact;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Register With Provenance

`pulith-store` absorbs fetch/archive evidence and shapes provenance at the store boundary.

```rust
# use pulith_store::{StoreKey, StoreReady, StoreRoots};
# use std::path::PathBuf;
# let store = StoreReady::initialize(StoreRoots::new(PathBuf::from("a"), PathBuf::from("b"), PathBuf::from("c")))?;
# let key = StoreKey::logical("runtime")?;
# let path = PathBuf::from("runtime.tar.zst");
# let provenance = todo!();
// store.register_artifact(&key, (&path, &provenance))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Schema and Codec Boundary

Metadata persistence now routes through `pulith-serde-backend`, with explicit schema-version validation during decode.

## See Also

- `docs/design/store.md`
- `crates/pulith-state/README.md`
