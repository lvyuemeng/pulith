# pulith-install

Composable installation workflow primitives.

## Purpose

`pulith-install` composes the lower-level Pulith crates into reusable installation flows.

It does not define one global package model. It defines typed workflow steps that callers can use to build install, upgrade, and activation flows with compile-time ordering.

## Design Rules

- type-state for workflow ordering where misuse would be expensive
- semantic handles from `pulith-store`, not raw paths guessed from conventions
- persistent lifecycle updates through `pulith-state`
- activation remains a trait, not a hard-coded backend policy

## Core Types

- `InstallReady`
- `InstallSpec`
- `InstallInput`
- `PlannedInstall`
- `StagedInstall`
- `InstalledInstall`
- `ActivatedInstall`
- `InstallMode`
- `RollbackReceipt`
- `Activator`
- `SymlinkActivator`
- `InstalledShimResolver`
- `ShimCommand`
- `ShimLinkActivator`

## Workflow Shape

```rust
let installed = PlannedInstall::new(ready, spec)
    .stage()?
    .commit()?;

let receipt = installed.activate(&SymlinkActivator)?.finish();
```

Compile-time states:

- planned
- staged
- installed
- activated

This keeps ordering explicit without turning persistence models into compile-time state machines.

## Activation Boundary

Activation is expressed as a trait:

- callers may provide their own activator
- the crate ships a basic `SymlinkActivator`
- shim-oriented activators can compose on the same interface through `pulith-shim::TargetResolver`

## Current Scope

- stage from stored artifact or extracted directory handle
- stage from fetch receipts and direct archive extraction outputs
- commit into install root atomically through `pulith-fs::Workspace`
- support create-only, replace, and upgrade install modes
- support rollback to the previous install snapshot after replacement
- update persistent state records on install and activation
- record activation history
- provide shim-oriented activation adapters without embedding resolver policy into `pulith-install`
- provide optional backup/restore helpers for install roots and matching state facts

## Deferred

- richer rollback journals beyond single previous-install snapshots
- richer backup retention and pruning policy
- source resolution orchestration
- deeper shim-specific activation policies
