# pulith-state

Transaction-backed persistent lifecycle state.

## Role

`pulith-state` owns lifecycle facts, inspection, repair planning, and retention planning.

It should persist facts and expose semantic operations, not absorb install orchestration policy.

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

## How To Use It

Use this crate to:

- persist lifecycle state
- inspect drift
- plan or apply state repair
- inspect ownership/conflicts
- derive retention-aware cleanup plans

See `docs/design/state.md`.
