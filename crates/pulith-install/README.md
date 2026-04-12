# pulith-install

Typed installation, activation, backup/restore, and rollback workflow primitives.

## What This Crate Owns

`pulith-install` takes already-materialized inputs and moves them into an install root.

It owns:

- install staging and commit ordering
- replace/upgrade workflow transitions
- activation through explicit activator traits
- install-root rollback against the previous snapshot
- backup/restore of install content and typed state facts

It does not own:

- fetch policy
- source planning policy
- store retention policy
- non-filesystem side effects outside explicit caller stages

## Main Types

- `InstallReady`
- `InstallSpec`
- `InstallInput`
- `InstallCapabilities`
- `InstallPlanningRequest`
- `InstallPlanReport`
- `PlannedInstall`
- `StagedInstall`
- `InstalledInstall`
- `ActivatedInstall`
- `UninstallOptions`
- `UninstallDisposition`

## Basic Install Flow

```rust
use pulith_install::{InstallReady, InstallSpec, PlannedInstall};

# let ready: InstallReady = todo!();
# let spec: InstallSpec = todo!();
let receipt = PlannedInstall::new(ready, spec)
    .stage()?
    .commit()?
    .finish();
# let _ = receipt;
# Ok::<(), pulith_install::InstallError>(())
```

## Planning Before Mutation

Use `InstallSpec::plan(...)` when you want a typed report before touching the filesystem.

```rust
use pulith_install::{
    ActivationSupport, ConnectivityMode, InstallCapabilities, InstallPlanningRequest,
    InstallWorkflowVariant, InstallWritableScope, RollbackSupport,
};

# use pulith_install::InstallSpec;
# let spec: InstallSpec = todo!();
let plan = spec.plan(InstallPlanningRequest {
    desired_variant: InstallWorkflowVariant::PreStagedStore,
    required_scope: InstallWritableScope::User,
    capabilities: InstallCapabilities {
        connectivity: ConnectivityMode::Online,
        activation: ActivationSupport::Available,
        writable_scope: InstallWritableScope::User,
        rollback: RollbackSupport::Expected,
    },
});

if !plan.can_proceed() {
    for limitation in &plan.limitations {
        println!("limitation: {limitation:?}");
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Inputs

`InstallInput` is transport-agnostic. Common entry points:

- `InstallInput::from_file_path(...)`
- `InstallInput::from_stored_artifact(...)`
- `InstallInput::ExtractedArtifact(...)`
- `InstallInput::from_extracted_tree(...)`

That means fetch/archive receipts do not leak across the install boundary.

## Activation

Activation is explicit and trait-based.

Built-in activators include:

- symlink-style activation
- copy-based file activation
- shim-based activation through `pulith-shim`

## Backup / Restore

Backup and restore use typed `pulith-state::ResourceStateSnapshot` payloads, not a bespoke install-only state shape.

```rust
# use pulith_install::InstallReady;
# use pulith_resource::ResourceId;
# let ready: InstallReady = todo!();
# let id = ResourceId::parse("example/runtime")?;
let backup = ready.create_backup(&id, "installs/runtime", "backups")?;
let restore = ready.restore_backup(&backup)?;
# let _ = restore;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Uninstall

Uninstall scope is typed through dispositions:

```rust
use pulith_install::{UninstallDisposition, UninstallOptions};

let options = UninstallOptions {
    install_root: UninstallDisposition::Remove,
    activation_targets: UninstallDisposition::Keep,
    state_record: UninstallDisposition::Keep,
    activation_records: UninstallDisposition::Keep,
};
# let _ = options;
```

## See Also

- `docs/design/install.md`
- `examples/runtime-manager/README.md`
