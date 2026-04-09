# pulith-state

Transaction-backed persistent lifecycle state.

## Main APIs

- `StateReady`
- `StateSnapshot`
- `ResourceRecord`
- `ResourceRecordPatch`
- `ResourceLifecycle`
- inspection / repair / retention helpers

## Basic Usage

```rust
use pulith_resource::{ResourceId, VersionSelector};
use pulith_state::{ResourceLifecycle, ResourceRecordPatch, StateReady};

let state = StateReady::initialize("state.json")?;
let id = ResourceId::parse("example/runtime")?;
state.ensure_resource_record(id.clone(), VersionSelector::alias("stable")?)?;
state.patch_resource_record(&id, ResourceRecordPatch::lifecycle(ResourceLifecycle::Resolved))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

See `docs/design/state.md`.
