# pulith-state

Transaction-backed lifecycle persistence, inspection, repair planning, and retention planning.

## What This Crate Owns

`pulith-state` stores lifecycle facts and exposes deterministic read/plan/apply surfaces.

It owns:

- persistent resource and activation records
- typed per-resource snapshots
- inspection reports
- repair plans
- retention/ownership planning helpers
- optional reusable analysis index for repeated report generation

## Main Types

- `StateReady`
- `StateSnapshot`
- `ResourceStateSnapshot`
- `ResourceRecord`
- `ResourceRecordPatch`
- `ResourceInspectionReport`
- `ResourceStateRepairPlan`
- `StateAnalysisIndex`

## Basic Usage

```rust
use pulith_resource::{ResourceId, VersionSelector};
use pulith_state::{ResourceLifecycle, ResourceRecordPatch, StateReady};

let state = StateReady::initialize("state.json")?;
let id = ResourceId::parse("example/runtime")?;

state.ensure_resource_record(id.clone(), VersionSelector::alias("stable")?)?;
state.patch_resource_record(
    &id,
    ResourceRecordPatch::lifecycle(ResourceLifecycle::Resolved),
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Inspect and Repair

```rust
# use pulith_resource::ResourceId;
# use pulith_state::StateReady;
# let state = StateReady::initialize("state.json")?;
# let id = ResourceId::parse("example/runtime")?;
let report = state.inspect_resource(&id, None)?;
if !report.is_clean() {
    let plan = state.plan_resource_state_repair(&id, None)?;
    let _applied = state.apply_resource_state_repair(&plan)?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Repeated Analysis

For repeated ownership/inspection/reference calls, build a reusable analysis index:

```rust
# use pulith_resource::ResourceId;
# use pulith_state::StateReady;
# let state = StateReady::initialize("state.json")?;
# let id = ResourceId::parse("example/runtime")?;
let index = state.build_analysis_index()?;
let report = state.inspect_resource_with_index(&id, None, &index);
let ownership = state.activation_ownership_report_with_index(&index);
# let _ = (report, ownership);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Schema Boundary

State snapshots carry explicit schema versions and validate on decode/load boundaries.

## See Also

- `docs/design/state.md`
- `docs/design/serialization.md`
